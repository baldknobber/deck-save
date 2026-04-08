use serde::Serialize;
use std::collections::HashSet;
use std::io::Read;
use std::path::PathBuf;
use tauri::Emitter;

use crate::launchers;
use crate::manifest;
use crate::path_expander::{self, ExpansionContext};
use crate::steam;

#[derive(Debug, Serialize, Clone)]
pub struct Game {
    pub id: i64,
    pub title: String,
    pub steam_id: Option<String>,
    pub save_paths: Vec<String>,
    pub save_path_count: usize,
    pub last_backup: Option<String>,
    pub status: String,
    pub custom_save_paths: Vec<String>,
    pub launcher: String,
}

#[tauri::command]
pub fn get_steam_header_url(
    steam_id: String,
    state: tauri::State<'_, crate::AppState>,
) -> Result<Option<String>, String> {
    // Check in-memory cache first
    {
        let cache = state
            .header_url_cache
            .lock()
            .map_err(|e| format!("Cache lock error: {e}"))?;
        if let Some(cached) = cache.get(&steam_id) {
            return Ok(cached.clone());
        }
    }

    // Validate steam_id is numeric to prevent injection
    if !steam_id.chars().all(|c| c.is_ascii_digit()) {
        return Err("Invalid steam ID".to_string());
    }

    // Fetch from Steam Store API
    let url = format!(
        "https://store.steampowered.com/api/appdetails?appids={}",
        steam_id
    );
    let result = match ureq::get(&url).call() {
        Ok(resp) => {
            let mut body_str = String::new();
            resp.into_reader()
                .take(1_000_000)
                .read_to_string(&mut body_str)
                .map_err(|e| format!("Read error: {e}"))?;
            let body: serde_json::Value = serde_json::from_str(&body_str)
                .map_err(|e| format!("JSON parse error: {e}"))?;
            body.get(&steam_id)
                .and_then(|app: &serde_json::Value| app.get("data"))
                .and_then(|data: &serde_json::Value| data.get("header_image"))
                .and_then(|v: &serde_json::Value| v.as_str())
                .map(|s: &str| s.to_string())
        }
        Err(e) => {
            eprintln!("[DeckSave] Steam API request failed for {}: {}", steam_id, e);
            None
        }
    };

    // Cache the result (even None, to avoid repeated failed requests)
    {
        let mut cache = state
            .header_url_cache
            .lock()
            .map_err(|e| format!("Cache lock error: {e}"))?;
        cache.insert(steam_id, result.clone());
    }

    Ok(result)
}

#[tauri::command]
pub fn scan_games(app: tauri::AppHandle, state: tauri::State<'_, crate::AppState>) -> Result<Vec<Game>, String> {
    let conn = state.db.lock().map_err(|e| format!("DB lock error: {e}"))?;

    // Locate Steam
    let steam_dir = steam::locate_steam()?;
    let steam_root = steam_dir.path().to_path_buf();

    // Download + load Ludusavi manifest
    let manifest_path = manifest::ensure_manifest(&state.app_data_dir)?;
    let manifest_data = manifest::load_manifest(&manifest_path)?;
    let steam_index = manifest::build_steam_index(&manifest_data);

    // Detect Proton usage (Linux only)
    let compat_tools = steam::compat_tool_mapping(&steam_dir);

    // Enumerate installed games
    let installed = steam::installed_games(&steam_dir);

    eprintln!(
        "[DeckSave] Scan: {} installed games, {} manifest entries",
        installed.len(),
        steam_index.len()
    );

    for game in &installed {
        let mut save_paths: Vec<PathBuf> = Vec::new();
        let mut seen = HashSet::new();

        let is_proton = compat_tools.contains_key(&game.app_id);

        // Look up in Ludusavi manifest by Steam ID
        if let Some(game_name) = steam_index.get(&game.app_id) {
            if let Some(manifest_game) = manifest_data.get(game_name) {
                let ctx = ExpansionContext {
                    steam_root: steam_root.clone(),
                    library_path: game.library_path.clone(),
                    install_dir: game.install_dir.clone(),
                    app_id: game.app_id,
                    is_proton,
                };

                for (path_template, file_entry) in &manifest_game.files {
                    // Filter by current OS
                    if !file_entry.when.is_empty() {
                        let os_ok = file_entry.when.iter().any(|c| {
                            c.os.as_deref().is_none_or(|os| {
                                if cfg!(target_os = "windows") {
                                    os == "windows"
                                } else if cfg!(target_os = "linux") {
                                    // On Linux with Proton, also accept windows paths
                                    os == "linux" || is_proton
                                } else {
                                    false
                                }
                            })
                        });
                        if !os_ok {
                            continue;
                        }
                    }

                    let expanded = path_expander::expand_path(path_template, &ctx);
                    for p in expanded {
                        if seen.insert(p.clone()) {
                            save_paths.push(p);
                        }
                    }
                }
            }
        }

        // Always check Steam Cloud userdata
        let userdata = steam_root.join("userdata");
        if userdata.exists() {
            if let Ok(users) = std::fs::read_dir(&userdata) {
                for user in users.flatten() {
                    let remote = user
                        .path()
                        .join(game.app_id.to_string())
                        .join("remote");
                    if remote.exists() && seen.insert(remote.clone()) {
                        save_paths.push(remote);
                    }
                }
            }
        }

        // Heuristic Proton prefix scan: when a game uses Proton but no save
        // paths were found from Ludusavi, scan common Windows save locations
        // inside the Proton prefix for save-like files.
        if is_proton && save_paths.is_empty() {
            let pfx_user = game.library_path
                .join("steamapps/compatdata")
                .join(game.app_id.to_string())
                .join("pfx/drive_c/users/steamuser");

            let heuristic_dirs = [
                pfx_user.join("AppData/Local"),
                pfx_user.join("AppData/Roaming"),
                pfx_user.join("AppData/LocalLow"),
                pfx_user.join("Saved Games"),
                pfx_user.join("Documents/My Games"),
                pfx_user.join("Documents"),
            ];

            let save_extensions = [".sav", ".save", ".dat", ".sl2", ".bak", ".cfg", ".ini", ".json", ".xml"];

            for dir in &heuristic_dirs {
                if !dir.exists() { continue; }
                // Walk max 3 levels deep to avoid scanning huge trees
                let walker = walkdir::WalkDir::new(dir)
                    .max_depth(3)
                    .into_iter()
                    .filter_map(|e| e.ok());

                for entry in walker {
                    if entry.file_type().is_file() {
                        let name = entry.file_name().to_string_lossy().to_lowercase();
                        if save_extensions.iter().any(|ext| name.ends_with(ext)) {
                            if let Some(parent) = entry.path().parent() {
                                let pb = parent.to_path_buf();
                                if seen.insert(pb.clone()) {
                                    save_paths.push(pb);
                                }
                            }
                            // Found a match in this dir tree, no need to keep walking
                            break;
                        }
                    }
                }
            }
        }

        // Upsert into SQLite
        let paths_json = serde_json::to_string(
            &save_paths
                .iter()
                .map(|p| p.to_string_lossy().into_owned())
                .collect::<Vec<_>>(),
        )
        .unwrap_or_else(|_| "[]".to_string());

        let app_id_str = game.app_id.to_string();

        let existing_id: Option<i64> = conn
            .query_row(
                "SELECT id FROM games WHERE steam_id = ?1",
                rusqlite::params![app_id_str],
                |row| row.get(0),
            )
            .ok();

        if let Some(id) = existing_id {
            conn.execute(
                "UPDATE games SET title = ?1, install_dir = ?2, save_paths = ?3, \
                 launcher = 'steam', updated_at = datetime('now') WHERE id = ?4",
                rusqlite::params![game.name, game.install_dir, paths_json, id],
            )
            .map_err(|e| format!("DB update error: {e}"))?;
        } else {
            conn.execute(
                "INSERT INTO games (title, steam_id, install_dir, save_paths, launcher) \
                 VALUES (?1, ?2, ?3, ?4, 'steam')",
                rusqlite::params![game.name, app_id_str, game.install_dir, paths_json],
            )
            .map_err(|e| format!("DB insert error: {e}"))?;
        }
    }

    // ── Non-Steam launcher detection ─────────────────────────────────────
    let non_steam_games = launchers::detect_all();
    let mut known_titles: HashSet<String> = HashSet::new();
    {
        let mut stmt = conn
            .prepare("SELECT LOWER(title) FROM games")
            .map_err(|e| format!("DB query error: {e}"))?;
        let rows = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|e| format!("DB query error: {e}"))?;
        for row in rows {
            if let Ok(t) = row {
                known_titles.insert(t);
            }
        }
    }

    let mut launcher_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

    for detected in non_steam_games {
        let title_lower = detected.title.to_lowercase();
        if known_titles.contains(&title_lower) {
            // Game already exists (from Steam) — merge save paths
            if !detected.save_paths.is_empty() {
                if let Ok((existing_id,)) = conn.query_row(
                    "SELECT id FROM games WHERE LOWER(title) = ?1",
                    rusqlite::params![title_lower],
                    |row| Ok((row.get::<_, i64>(0)?,)),
                ) {
                    let current_json: String = conn
                        .query_row(
                            "SELECT save_paths FROM games WHERE id = ?1",
                            rusqlite::params![existing_id],
                            |row| row.get(0),
                        )
                        .unwrap_or_else(|_| "[]".to_string());
                    let mut current_paths: Vec<String> =
                        serde_json::from_str(&current_json).unwrap_or_default();
                    for sp in &detected.save_paths {
                        if !current_paths.contains(sp) {
                            current_paths.push(sp.clone());
                        }
                    }
                    let merged_json =
                        serde_json::to_string(&current_paths).unwrap_or_else(|_| "[]".to_string());
                    conn.execute(
                        "UPDATE games SET save_paths = ?1, updated_at = datetime('now') WHERE id = ?2",
                        rusqlite::params![merged_json, existing_id],
                    )
                    .ok();
                }
            }
            continue;
        }

        known_titles.insert(title_lower);
        *launcher_counts.entry(detected.launcher.clone()).or_insert(0) += 1;

        let paths_json = serde_json::to_string(&detected.save_paths)
            .unwrap_or_else(|_| "[]".to_string());

        conn.execute(
            "INSERT INTO games (title, steam_id, install_dir, save_paths, launcher) \
             VALUES (?1, NULL, ?2, ?3, ?4)",
            rusqlite::params![
                detected.title,
                detected.install_dir,
                paths_json,
                detected.launcher
            ],
        )
        .map_err(|e| format!("DB insert non-Steam game error: {e}"))?;
    }

    // Emit scan summary event with Steam count and per-launcher breakdown
    let _ = app.emit("scan-summary", serde_json::json!({
        "steam_count": installed.len(),
        "launcher_counts": launcher_counts,
    }));

    load_games_from_db(&conn)
}

#[tauri::command]
pub fn get_cached_games(state: tauri::State<'_, crate::AppState>) -> Result<Vec<Game>, String> {
    let conn = state.db.lock().map_err(|e| format!("DB lock error: {e}"))?;
    load_games_from_db(&conn)
}

fn load_games_from_db(conn: &rusqlite::Connection) -> Result<Vec<Game>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT g.id, g.title, g.steam_id, g.save_paths, g.status, \
             (SELECT MAX(b.timestamp) FROM backups b WHERE b.game_id = g.id), \
             g.custom_save_paths, g.launcher \
             FROM games g ORDER BY g.title COLLATE NOCASE",
        )
        .map_err(|e| format!("DB query error: {e}"))?;

    let games = stmt
        .query_map([], |row| {
            let paths_json: String = row.get(3)?;
            let custom_json: String = row.get::<_, String>(6).unwrap_or_else(|_| "[]".to_string());
            let mut save_paths: Vec<String> = serde_json::from_str(&paths_json).unwrap_or_default();
            let custom_save_paths: Vec<String> = serde_json::from_str(&custom_json).unwrap_or_default();

            // Merge custom paths (prepend, deduplicated)
            for cp in &custom_save_paths {
                if !save_paths.contains(cp) {
                    save_paths.insert(0, cp.clone());
                }
            }

            let save_path_count = save_paths.len();
            Ok(Game {
                id: row.get(0)?,
                title: row.get(1)?,
                steam_id: row.get(2)?,
                save_paths,
                save_path_count,
                last_backup: row.get(5)?,
                status: row.get(4)?,
                custom_save_paths,
                launcher: row.get::<_, String>(7).unwrap_or_else(|_| "steam".to_string()),
            })
        })
        .map_err(|e| format!("DB query error: {e}"))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(games)
}

#[tauri::command]
pub fn add_custom_save_path(
    game_id: i64,
    path: String,
    state: tauri::State<'_, crate::AppState>,
) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| format!("DB lock error: {e}"))?;

    let current_json: String = conn
        .query_row(
            "SELECT custom_save_paths FROM games WHERE id = ?1",
            rusqlite::params![game_id],
            |row| row.get(0),
        )
        .map_err(|e| format!("Game not found: {e}"))?;

    let mut paths: Vec<String> = serde_json::from_str(&current_json).unwrap_or_default();
    if !paths.contains(&path) {
        paths.push(path);
    }

    let new_json = serde_json::to_string(&paths).unwrap_or_else(|_| "[]".to_string());
    conn.execute(
        "UPDATE games SET custom_save_paths = ?1, updated_at = datetime('now') WHERE id = ?2",
        rusqlite::params![new_json, game_id],
    )
    .map_err(|e| format!("DB update error: {e}"))?;

    Ok(())
}

#[tauri::command]
pub fn add_custom_game(
    title: String,
    save_path: String,
    state: tauri::State<'_, crate::AppState>,
) -> Result<Game, String> {
    let conn = state.db.lock().map_err(|e| format!("DB lock error: {e}"))?;

    let paths_json = serde_json::to_string(&vec![&save_path]).unwrap_or_else(|_| "[]".to_string());

    conn.execute(
        "INSERT INTO games (title, steam_id, install_dir, save_paths, launcher) \
         VALUES (?1, NULL, NULL, ?2, 'custom')",
        rusqlite::params![title, paths_json],
    )
    .map_err(|e| format!("DB insert error: {e}"))?;

    let id = conn.last_insert_rowid();

    Ok(Game {
        id,
        title,
        steam_id: None,
        save_paths: vec![save_path],
        save_path_count: 1,
        last_backup: None,
        status: "never_backed_up".to_string(),
        custom_save_paths: Vec::new(),
        launcher: "custom".to_string(),
    })
}

#[tauri::command]
pub fn remove_custom_save_path(
    game_id: i64,
    path: String,
    state: tauri::State<'_, crate::AppState>,
) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| format!("DB lock error: {e}"))?;

    let current_json: String = conn
        .query_row(
            "SELECT custom_save_paths FROM games WHERE id = ?1",
            rusqlite::params![game_id],
            |row| row.get(0),
        )
        .map_err(|e| format!("Game not found: {e}"))?;

    let mut paths: Vec<String> = serde_json::from_str(&current_json).unwrap_or_default();
    paths.retain(|p| p != &path);

    let new_json = serde_json::to_string(&paths).unwrap_or_else(|_| "[]".to_string());
    conn.execute(
        "UPDATE games SET custom_save_paths = ?1, updated_at = datetime('now') WHERE id = ?2",
        rusqlite::params![new_json, game_id],
    )
    .map_err(|e| format!("DB update error: {e}"))?;

    Ok(())
}

use serde::Serialize;
use std::collections::HashSet;
use std::path::PathBuf;

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
}

#[tauri::command]
pub fn scan_games(state: tauri::State<'_, crate::AppState>) -> Result<Vec<Game>, String> {
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
                            c.os.as_deref().map_or(true, |os| {
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
                 updated_at = datetime('now') WHERE id = ?4",
                rusqlite::params![game.name, game.install_dir, paths_json, id],
            )
            .map_err(|e| format!("DB update error: {e}"))?;
        } else {
            conn.execute(
                "INSERT INTO games (title, steam_id, install_dir, save_paths) \
                 VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![game.name, app_id_str, game.install_dir, paths_json],
            )
            .map_err(|e| format!("DB insert error: {e}"))?;
        }
    }

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
             (SELECT MAX(b.timestamp) FROM backups b WHERE b.game_id = g.id) \
             FROM games g ORDER BY g.title COLLATE NOCASE",
        )
        .map_err(|e| format!("DB query error: {e}"))?;

    let games = stmt
        .query_map([], |row| {
            let paths_json: String = row.get(3)?;
            let save_paths: Vec<String> = serde_json::from_str(&paths_json).unwrap_or_default();
            let save_path_count = save_paths.len();
            Ok(Game {
                id: row.get(0)?,
                title: row.get(1)?,
                steam_id: row.get(2)?,
                save_paths,
                save_path_count,
                last_backup: row.get(5)?,
                status: row.get(4)?,
            })
        })
        .map_err(|e| format!("DB query error: {e}"))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(games)
}

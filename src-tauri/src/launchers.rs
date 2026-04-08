//! Non-Steam launcher detection.
//!
//! Each launcher module checks whether its config files/directories exist on
//! the current machine and, when they do, enumerates installed games and their
//! likely save-file paths. Everything is best-effort: a missing or corrupt
//! config is silently skipped — never crashes the scan.

use serde::Serialize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

// ─── Public types ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct DetectedGame {
    pub title: String,
    pub install_dir: Option<String>,
    pub save_paths: Vec<String>,
    pub launcher: String,
    pub launcher_id: Option<String>,
}

// ─── Top-level entry point ───────────────────────────────────────────────────

/// Run every launcher detector and merge the results.
pub fn detect_all() -> Vec<DetectedGame> {
    let mut games: Vec<DetectedGame> = Vec::new();

    type Detector = (&'static str, fn() -> Vec<DetectedGame>);
    let detectors: Vec<Detector> = vec![
        ("Heroic", detect_heroic),
        ("Lutris", detect_lutris),
        ("Bottles", detect_bottles),
        ("EA", detect_ea),
        ("Ubisoft", detect_ubisoft),
        ("Rockstar", detect_rockstar),
        #[cfg(target_os = "windows")]
        ("Epic (Windows)", detect_epic_windows),
        #[cfg(target_os = "windows")]
        ("GOG (Windows)", detect_gog_windows),
        #[cfg(target_os = "windows")]
        ("EA (Windows)", detect_ea_windows),
        #[cfg(target_os = "windows")]
        ("Ubisoft (Windows)", detect_ubisoft_windows),
    ];

    for (name, detector) in detectors {
        match std::panic::catch_unwind(detector) {
            Ok(found) => {
                if !found.is_empty() {
                    eprintln!("[DeckSave] {name}: detected {} games", found.len());
                    games.extend(found);
                }
            }
            Err(_) => {
                eprintln!("[DeckSave] {name}: detector panicked, skipping");
            }
        }
    }

    games
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn home_dir() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var("USERPROFILE").ok().map(PathBuf::from)
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::env::var("HOME").ok().map(PathBuf::from)
    }
}

/// Shared set of extensions that likely indicate a save file.
const SAVE_EXTENSIONS: &[&str] = &[
    ".sav", ".save", ".dat", ".sl2", ".bak", ".cfg", ".ini", ".json", ".xml",
];

/// Scan *common Windows-style save dirs* inside a Wine/Proton prefix for save-
/// like files. Returns any directories that contain at least one match. Walks
/// at most 3 levels deep to keep things fast.
fn scan_prefix_for_saves(prefix: &Path) -> Vec<String> {
    let steamuser = prefix.join("drive_c/users/steamuser");
    if !steamuser.exists() {
        return Vec::new();
    }

    let candidates = [
        steamuser.join("AppData/Local"),
        steamuser.join("AppData/Roaming"),
        steamuser.join("AppData/LocalLow"),
        steamuser.join("Saved Games"),
        steamuser.join("Documents/My Games"),
        steamuser.join("Documents"),
    ];

    let mut found: Vec<String> = Vec::new();
    let mut seen: HashSet<PathBuf> = HashSet::new();

    for dir in &candidates {
        if !dir.exists() {
            continue;
        }
        let walker = walkdir::WalkDir::new(dir)
            .max_depth(3)
            .into_iter()
            .filter_map(|e| e.ok());

        for entry in walker {
            if entry.file_type().is_file() {
                let name = entry.file_name().to_string_lossy().to_lowercase();
                if SAVE_EXTENSIONS.iter().any(|ext| name.ends_with(ext)) {
                    if let Some(parent) = entry.path().parent() {
                        let pb = parent.to_path_buf();
                        if seen.insert(pb.clone()) {
                            found.push(pb.to_string_lossy().into_owned());
                        }
                    }
                    break; // one hit per candidate dir is enough
                }
            }
        }
    }

    found
}

/// Try to read a Wine-format registry file and extract a display name for a
/// given registry key prefix. Wine `.reg` files are plain text.
#[allow(dead_code)]
fn read_reg_display_names(reg_path: &Path, key_prefix: &str) -> Vec<(String, String)> {
    let contents = match std::fs::read_to_string(reg_path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let mut results: Vec<(String, String)> = Vec::new();
    let mut current_key = String::new();

    for line in contents.lines() {
        let trimmed = line.trim();
        // Section header: [Software\\...\\Key]
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            current_key = trimmed[1..trimmed.len() - 1].to_string();
        }
        // If we're in a matching section, look for DisplayName or similar
        if current_key.contains(key_prefix) {
            if let Some(rest) = trimmed.strip_prefix("\"DisplayName\"=") {
                let value = rest.trim_matches('"').to_string();
                if !value.is_empty() {
                    results.push((current_key.clone(), value));
                }
            }
        }
    }

    results
}

// ─── Heroic Games Launcher ───────────────────────────────────────────────────

fn heroic_config_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(home) = home_dir() {
        // Multiple possible locations depending on Heroic version / install method
        dirs.push(home.join(".config/heroic"));
        dirs.push(home.join(".config/Heroic"));
        dirs.push(home.join(".local/share/heroic"));
        dirs.push(home.join(".local/share/Heroic"));
        // Flatpak
        dirs.push(home.join(".var/app/com.heroicgameslauncher.hgl/config/heroic"));
    }
    dirs
}

fn detect_heroic() -> Vec<DetectedGame> {
    let mut games: Vec<DetectedGame> = Vec::new();
    let mut seen_titles: HashSet<String> = HashSet::new();

    for base in heroic_config_dirs() {
        if !base.exists() {
            continue;
        }

        // ── Epic (Legendary) ─────────────────────────────────────────────
        let epic_installed = base.join("legendaryConfig/legendary/installed.json");
        if epic_installed.exists() {
            if let Ok(data) = std::fs::read_to_string(&epic_installed) {
                if let Ok(map) = serde_json::from_str::<serde_json::Value>(&data) {
                    if let Some(obj) = map.as_object() {
                        for (app_name, info) in obj {
                            let title = info
                                .get("title")
                                .and_then(|v| v.as_str())
                                .unwrap_or(app_name)
                                .to_string();

                            if !seen_titles.insert(title.to_lowercase()) {
                                continue;
                            }

                            let install_path = info
                                .get("install_path")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string());

                            // Try to find save locations in the wine prefix
                            let mut save_paths = Vec::new();

                            // Check Heroic default prefix locations
                            if let Some(home) = home_dir() {
                                let prefix_dirs = [
                                    home.join(format!("Games/Heroic/Prefixes/{app_name}")),
                                    home.join("Games/Heroic/Prefixes/default"),
                                ];
                                for pfx in &prefix_dirs {
                                    if pfx.exists() {
                                        save_paths.extend(scan_prefix_for_saves(pfx));
                                        if !save_paths.is_empty() {
                                            break;
                                        }
                                    }
                                }
                            }

                            games.push(DetectedGame {
                                title,
                                install_dir: install_path,
                                save_paths,
                                launcher: "heroic_epic".to_string(),
                                launcher_id: Some(app_name.clone()),
                            });
                        }
                    }
                }
            }
        }

        // ── GOG (Heroic) ─────────────────────────────────────────────────
        let gog_installed = base.join("gog_store/installed.json");
        if gog_installed.exists() {
            if let Ok(data) = std::fs::read_to_string(&gog_installed) {
                if let Ok(arr) = serde_json::from_str::<serde_json::Value>(&data) {
                    let items = arr.as_array().cloned().unwrap_or_default();
                    for info in items {
                        let title = info
                            .get("appName")
                            .or_else(|| info.get("title"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("Unknown GOG Game")
                            .to_string();

                        if !seen_titles.insert(title.to_lowercase()) {
                            continue;
                        }

                        let install_path = info
                            .get("install_path")
                            .or_else(|| info.get("path"))
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());

                        let launcher_id = info
                            .get("appName")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());

                        let mut save_paths = Vec::new();
                        if let Some(home) = home_dir() {
                            if let Some(ref app_name) = launcher_id {
                                let pfx = home.join(format!("Games/Heroic/Prefixes/{app_name}"));
                                if pfx.exists() {
                                    save_paths.extend(scan_prefix_for_saves(&pfx));
                                }
                            }
                        }

                        games.push(DetectedGame {
                            title,
                            install_dir: install_path,
                            save_paths,
                            launcher: "heroic_gog".to_string(),
                            launcher_id,
                        });
                    }
                }
            }
        }

        // ── Amazon (Nile) ────────────────────────────────────────────────
        let nile_installed = base.join("nile_config/nile/installed.json");
        if nile_installed.exists() {
            if let Ok(data) = std::fs::read_to_string(&nile_installed) {
                if let Ok(arr) = serde_json::from_str::<serde_json::Value>(&data) {
                    let items = arr.as_array().cloned().unwrap_or_default();
                    for info in items {
                        let title = info
                            .get("title")
                            .or_else(|| info.get("id"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("Unknown Amazon Game")
                            .to_string();

                        if !seen_titles.insert(title.to_lowercase()) {
                            continue;
                        }

                        let install_path = info
                            .get("path")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());

                        let launcher_id = info
                            .get("id")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());

                        games.push(DetectedGame {
                            title,
                            install_dir: install_path,
                            save_paths: Vec::new(),
                            launcher: "heroic_amazon".to_string(),
                            launcher_id,
                        });
                    }
                }
            }
        }
    }

    games
}

// ─── Lutris ──────────────────────────────────────────────────────────────────

fn detect_lutris() -> Vec<DetectedGame> {
    let home = match home_dir() {
        Some(h) => h,
        None => return Vec::new(),
    };

    let db_path = home.join(".local/share/lutris/pga.db");
    if !db_path.exists() {
        return Vec::new();
    }

    // Open read-only so we don't interfere with Lutris
    let conn = match rusqlite::Connection::open_with_flags(
        &db_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    ) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[DeckSave] Lutris: cannot open pga.db: {e}");
            return Vec::new();
        }
    };

    let mut stmt = match conn.prepare(
        "SELECT name, slug, runner, directory, configpath FROM games WHERE installed = 1",
    ) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[DeckSave] Lutris: query failed: {e}");
            return Vec::new();
        }
    };

    type LutrisRow = (String, String, String, Option<String>, Option<String>);
    let rows: Vec<LutrisRow> = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2).unwrap_or_default(),
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
            ))
        })
        .ok()
        .map(|iter| iter.filter_map(|r| r.ok()).collect())
        .unwrap_or_default();

    let mut games = Vec::new();

    for (name, slug, runner, directory, configpath) in rows {
        let mut save_paths = Vec::new();

        // Try to find wine prefix from game config YAML
        if runner == "wine" || runner == "proton" {
            if let Some(ref cfg) = configpath {
                let cfg_path = home
                    .join(".local/share/lutris/games")
                    .join(format!("{cfg}.yml"));
                if let Ok(yaml_str) = std::fs::read_to_string(&cfg_path) {
                    if let Ok(yaml) = serde_yaml::from_str::<serde_json::Value>(&yaml_str) {
                        // Prefix can be at game.wine.prefix or game.prefix
                        let prefix = yaml
                            .pointer("/game/wine/prefix")
                            .or_else(|| yaml.pointer("/game/prefix"))
                            .and_then(|v| v.as_str());
                        if let Some(pfx_str) = prefix {
                            let pfx = PathBuf::from(pfx_str);
                            if pfx.exists() {
                                save_paths.extend(scan_prefix_for_saves(&pfx));
                            }
                        }
                    }
                }
            }
        }

        games.push(DetectedGame {
            title: name,
            install_dir: directory,
            save_paths,
            launcher: "lutris".to_string(),
            launcher_id: Some(slug),
        });
    }

    games
}

// ─── Bottles ─────────────────────────────────────────────────────────────────

fn detect_bottles() -> Vec<DetectedGame> {
    let home = match home_dir() {
        Some(h) => h,
        None => return Vec::new(),
    };

    let bottles_dir = home.join(".local/share/bottles/bottles");
    if !bottles_dir.exists() {
        return Vec::new();
    }

    let mut games = Vec::new();

    // Check library.yml for explicit game entries
    let library_path = home.join(".local/share/bottles/library.yml");
    if library_path.exists() {
        if let Ok(yaml_str) = std::fs::read_to_string(&library_path) {
            if let Ok(entries) = serde_yaml::from_str::<serde_json::Value>(&yaml_str) {
                if let Some(obj) = entries.as_object() {
                    for (_key, info) in obj {
                        let title = match info.get("name").and_then(|v| v.as_str()) {
                            Some(t) => t,
                            None => continue,
                        };

                        let bottle_name = info
                            .get("bottle")
                            .and_then(|b| b.get("name"))
                            .or_else(|| info.get("bottle_name"))
                            .and_then(|v| v.as_str());

                        let mut save_paths = Vec::new();
                        if let Some(bn) = bottle_name {
                            let pfx = bottles_dir.join(bn);
                            if pfx.exists() {
                                save_paths.extend(scan_prefix_for_saves(&pfx));
                            }
                        }

                        games.push(DetectedGame {
                            title: title.to_string(),
                            install_dir: None,
                            save_paths,
                            launcher: "bottles".to_string(),
                            launcher_id: None,
                        });
                    }
                }
            }
        }
    }

    // Also scan each bottle for installed programs
    if let Ok(entries) = std::fs::read_dir(&bottles_dir) {
        for entry in entries.flatten() {
            let bottle_path = entry.path();
            let bottle_yml = bottle_path.join("bottle.yml");
            if !bottle_yml.exists() {
                continue;
            }

            // Check Program Files for installed games
            for program_dir_name in &["Program Files", "Program Files (x86)"] {
                let pf = bottle_path.join(format!("drive_c/{program_dir_name}"));
                if !pf.exists() {
                    continue;
                }
                if let Ok(dirs) = std::fs::read_dir(&pf) {
                    for dir in dirs.flatten() {
                        if !dir.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                            continue;
                        }
                        let dir_name = dir.file_name().to_string_lossy().into_owned();
                        // Skip common non-game directories
                        if matches!(
                            dir_name.to_lowercase().as_str(),
                            "common files"
                                | "internet explorer"
                                | "windows nt"
                                | "windows media player"
                                | "windows"
                                | "microsoft.net"
                        ) {
                            continue;
                        }

                        // Check if any .exe exists in the directory (it's likely a game)
                        let has_exe = walkdir::WalkDir::new(dir.path())
                            .max_depth(2)
                            .into_iter()
                            .filter_map(|e| e.ok())
                            .any(|e| {
                                e.file_type().is_file()
                                    && e.path()
                                        .extension()
                                        .map(|ext| ext == "exe")
                                        .unwrap_or(false)
                            });

                        if !has_exe {
                            continue;
                        }

                        let save_paths = scan_prefix_for_saves(&bottle_path);

                        // Only add if not already found via library.yml
                        if !games.iter().any(|g| {
                            g.title.to_lowercase() == dir_name.to_lowercase()
                        }) {
                            games.push(DetectedGame {
                                title: dir_name,
                                install_dir: Some(dir.path().to_string_lossy().into_owned()),
                                save_paths,
                                launcher: "bottles".to_string(),
                                launcher_id: None,
                            });
                        }
                    }
                }
            }
        }
    }

    games
}

// ─── EA App (Linux via compatdata) ───────────────────────────────────────────

fn detect_ea() -> Vec<DetectedGame> {
    #[cfg(target_os = "windows")]
    {
        Vec::new() // Handled by detect_ea_windows
    }

    #[cfg(not(target_os = "windows"))]
    {
        let home = match home_dir() {
            Some(h) => h,
            None => return Vec::new(),
        };

        let steam_compatdata = home.join(".local/share/Steam/steamapps/compatdata");
        if !steam_compatdata.exists() {
            return Vec::new();
        }

        let prefix_names = [
            "NonSteamLaunchers",
            "TheEAappLauncher",
            "EALauncher",
        ];

        let mut games = Vec::new();
        let mut seen_titles: HashSet<String> = HashSet::new();

        for pfx_name in &prefix_names {
            let pfx = steam_compatdata.join(pfx_name).join("pfx");
            if !pfx.exists() {
                continue;
            }

            // Scan EA Games directories
            for ea_dir_name in &[
                "Program Files/Electronic Arts/EA Games",
                "Program Files (x86)/EA Games",
                "Program Files/EA Games",
                "Program Files (x86)/Electronic Arts",
            ] {
                let ea_games = pfx.join(format!("drive_c/{ea_dir_name}"));
                if !ea_games.exists() {
                    continue;
                }

                if let Ok(entries) = std::fs::read_dir(&ea_games) {
                    for entry in entries.flatten() {
                        if !entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                            continue;
                        }

                        let dir_name = entry.file_name().to_string_lossy().into_owned();

                        // Try to get title from installerdata.xml
                        let title = parse_ea_installer_xml(&entry.path())
                            .unwrap_or_else(|| dir_name.clone());

                        if !seen_titles.insert(title.to_lowercase()) {
                            continue;
                        }

                        let save_paths = scan_prefix_for_saves(&pfx);

                        games.push(DetectedGame {
                            title,
                            install_dir: Some(entry.path().to_string_lossy().into_owned()),
                            save_paths,
                            launcher: "ea".to_string(),
                            launcher_id: None,
                        });
                    }
                }
            }

            // Fallback: parse system.reg for Origin Games
            let system_reg = pfx.join("system.reg");
            if system_reg.exists() {
                let reg_entries =
                    read_reg_display_names(&system_reg, "Origin Games");
                for (key, display_name) in reg_entries {
                    if !seen_titles.insert(display_name.to_lowercase()) {
                        continue;
                    }
                    let save_paths = scan_prefix_for_saves(&pfx);
                    games.push(DetectedGame {
                        title: display_name,
                        install_dir: None,
                        save_paths,
                        launcher: "ea".to_string(),
                        launcher_id: Some(key),
                    });
                }
            }
        }

        games
    }
}

/// Parse EA `__Installer/installerdata.xml` for a game title.
/// Simple text search — no full XML parser needed.
fn parse_ea_installer_xml(game_dir: &Path) -> Option<String> {
    let xml_path = game_dir.join("__Installer/installerdata.xml");
    if !xml_path.exists() {
        return None;
    }
    let contents = std::fs::read_to_string(&xml_path).ok()?;
    // Look for <gameTitle> or <GameTitle>
    for pattern in &["<gameTitle>", "<GameTitle>", "<game_title>"] {
        if let Some(start) = contents.find(pattern) {
            let rest = &contents[start + pattern.len()..];
            let close_pattern = pattern.replace('<', "</");
            if let Some(end) = rest.find(&close_pattern) {
                let title = rest[..end].trim().to_string();
                if !title.is_empty() {
                    return Some(title);
                }
            }
        }
    }
    None
}

// ─── Ubisoft Connect (Linux via compatdata) ──────────────────────────────────

fn detect_ubisoft() -> Vec<DetectedGame> {
    #[cfg(target_os = "windows")]
    {
        Vec::new() // Handled by detect_ubisoft_windows
    }

    #[cfg(not(target_os = "windows"))]
    {
        let home = match home_dir() {
            Some(h) => h,
            None => return Vec::new(),
        };

        let steam_compatdata = home.join(".local/share/Steam/steamapps/compatdata");
        if !steam_compatdata.exists() {
            return Vec::new();
        }

        let prefix_names = [
            "NonSteamLaunchers",
            "UplayLauncher",
            "UbisoftConnectLauncher",
        ];

        let mut games = Vec::new();
        let mut seen_titles: HashSet<String> = HashSet::new();

        for pfx_name in &prefix_names {
            let pfx = steam_compatdata.join(pfx_name).join("pfx");
            if !pfx.exists() {
                continue;
            }

            // Check Ubisoft data directory for numeric game ID folders
            let ubi_data = pfx.join(
                "drive_c/Program Files (x86)/Ubisoft/Ubisoft Game Launcher/data",
            );
            let has_ubi_data = ubi_data.exists();

            // Parse registry for Uplay game entries
            let system_reg = pfx.join("system.reg");
            if system_reg.exists() {
                let reg_entries =
                    read_reg_display_names(&system_reg, "Uplay Install");
                for (_key, display_name) in reg_entries {
                    if !seen_titles.insert(display_name.to_lowercase()) {
                        continue;
                    }
                    let save_paths = scan_prefix_for_saves(&pfx);
                    games.push(DetectedGame {
                        title: display_name,
                        install_dir: None,
                        save_paths,
                        launcher: "ubisoft".to_string(),
                        launcher_id: None,
                    });
                }
            }

            // If registry gave us nothing, try data directory folders
            if games.is_empty() && has_ubi_data {
                if let Ok(entries) = std::fs::read_dir(&ubi_data) {
                    for entry in entries.flatten() {
                        if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                            let folder_name = entry.file_name().to_string_lossy().into_owned();
                            // Numeric folders are game IDs — use as title placeholder
                            if folder_name.chars().all(|c| c.is_ascii_digit()) {
                                let title = format!("Ubisoft Game {folder_name}");
                                if seen_titles.insert(title.to_lowercase()) {
                                    let save_paths = scan_prefix_for_saves(&pfx);
                                    games.push(DetectedGame {
                                        title,
                                        install_dir: None,
                                        save_paths,
                                        launcher: "ubisoft".to_string(),
                                        launcher_id: Some(folder_name),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        games
    }
}

// ─── Rockstar Launcher (Linux via compatdata) ────────────────────────────────

fn detect_rockstar() -> Vec<DetectedGame> {
    #[cfg(target_os = "windows")]
    {
        Vec::new()
    }

    #[cfg(not(target_os = "windows"))]
    {
        let home = match home_dir() {
            Some(h) => h,
            None => return Vec::new(),
        };

        let steam_compatdata = home.join(".local/share/Steam/steamapps/compatdata");
        if !steam_compatdata.exists() {
            return Vec::new();
        }

        let prefix_names = [
            "NonSteamLaunchers",
            "RockstarGamesLauncher",
        ];

        let mut games = Vec::new();
        let mut seen_titles: HashSet<String> = HashSet::new();

        for pfx_name in &prefix_names {
            let pfx = steam_compatdata.join(pfx_name).join("pfx");
            if !pfx.exists() {
                continue;
            }

            // Scan Rockstar Games program directory
            let rs_dir = pfx.join("drive_c/Program Files/Rockstar Games");
            if rs_dir.exists() {
                if let Ok(entries) = std::fs::read_dir(&rs_dir) {
                    for entry in entries.flatten() {
                        if !entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                            continue;
                        }
                        let dir_name = entry.file_name().to_string_lossy().into_owned();
                        if dir_name == "Launcher" || dir_name == "Social Club" {
                            continue;
                        }
                        if !seen_titles.insert(dir_name.to_lowercase()) {
                            continue;
                        }

                        // Rockstar saves are typically in Documents/Rockstar Games/
                        let mut save_paths = Vec::new();
                        let docs_save = pfx.join(format!(
                            "drive_c/users/steamuser/Documents/Rockstar Games/{dir_name}"
                        ));
                        if docs_save.exists() {
                            save_paths.push(docs_save.to_string_lossy().into_owned());
                        } else {
                            save_paths.extend(scan_prefix_for_saves(&pfx));
                        }

                        games.push(DetectedGame {
                            title: dir_name,
                            install_dir: Some(entry.path().to_string_lossy().into_owned()),
                            save_paths,
                            launcher: "rockstar".to_string(),
                            launcher_id: None,
                        });
                    }
                }
            }

            // Also check registry
            let system_reg = pfx.join("system.reg");
            if system_reg.exists() {
                let reg_entries =
                    read_reg_display_names(&system_reg, "Rockstar Games");
                for (_key, display_name) in reg_entries {
                    if !seen_titles.insert(display_name.to_lowercase()) {
                        continue;
                    }
                    let save_paths = scan_prefix_for_saves(&pfx);
                    games.push(DetectedGame {
                        title: display_name,
                        install_dir: None,
                        save_paths,
                        launcher: "rockstar".to_string(),
                        launcher_id: None,
                    });
                }
            }
        }

        games
    }
}

// ─── Windows-specific launcher detection ─────────────────────────────────────

#[cfg(target_os = "windows")]
fn detect_epic_windows() -> Vec<DetectedGame> {
    // LauncherInstalled.dat is a JSON file listing installed Epic games
    let programdata = std::env::var("ProgramData").unwrap_or_else(|_| "C:\\ProgramData".into());
    let dat_path = PathBuf::from(&programdata)
        .join("Epic/UnrealEngineLauncher/LauncherInstalled.dat");

    if !dat_path.exists() {
        return Vec::new();
    }

    let mut games = Vec::new();

    if let Ok(data) = std::fs::read_to_string(&dat_path) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&data) {
            if let Some(list) = json
                .get("InstallationList")
                .and_then(|v| v.as_array())
            {
                for item in list {
                    let app_name = item
                        .get("AppName")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown");
                    let install_loc = item
                        .get("InstallLocation")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");

                    if install_loc.is_empty() || app_name.is_empty() {
                        continue;
                    }

                    // Use folder name as display title (Epic doesn't store titles here)
                    let title = PathBuf::from(install_loc)
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_else(|| app_name.to_string());

                    games.push(DetectedGame {
                        title,
                        install_dir: Some(install_loc.to_string()),
                        save_paths: Vec::new(), // Will be matched via Ludusavi
                        launcher: "epic".to_string(),
                        launcher_id: Some(app_name.to_string()),
                    });
                }
            }
        }
    }

    games
}

#[cfg(target_os = "windows")]
fn detect_gog_windows() -> Vec<DetectedGame> {
    // GOG Galaxy stores game info in a SQLite DB
    let programdata = std::env::var("ProgramData").unwrap_or_else(|_| "C:\\ProgramData".into());
    let db_path = PathBuf::from(&programdata)
        .join("GOG.com/Galaxy/storage/galaxy-2.0.db");

    if !db_path.exists() {
        return Vec::new();
    }

    let conn = match rusqlite::Connection::open_with_flags(
        &db_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    ) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let mut stmt = match conn.prepare(
        "SELECT productId, title, installationPath FROM InstalledBaseProducts WHERE isInstalled = 1",
    ) {
        Ok(s) => s,
        // Table might not exist or have different schema
        Err(_) => match conn.prepare(
            "SELECT productId, title, installationPath FROM Products WHERE isInstalled = 1",
        ) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        },
    };

    let games: Vec<DetectedGame> = stmt
        .query_map([], |row| {
            let product_id: String = row.get::<_, i64>(0).map(|i| i.to_string()).unwrap_or_default();
            let title: String = row.get(1)?;
            let install_path: Option<String> = row.get(2)?;
            Ok(DetectedGame {
                title,
                install_dir: install_path,
                save_paths: Vec::new(),
                launcher: "gog".to_string(),
                launcher_id: Some(product_id),
            })
        })
        .ok()
        .map(|iter| iter.filter_map(|r| r.ok()).collect())
        .unwrap_or_default();

    games
}

#[cfg(target_os = "windows")]
fn detect_ea_windows() -> Vec<DetectedGame> {
    let mut games = Vec::new();

    // Check common EA install directories
    for base in &[
        "C:\\Program Files\\Electronic Arts",
        "C:\\Program Files (x86)\\Electronic Arts",
        "C:\\Program Files\\EA Games",
        "C:\\Program Files (x86)\\EA Games",
    ] {
        let base_path = PathBuf::from(base);
        if !base_path.exists() {
            continue;
        }
        if let Ok(entries) = std::fs::read_dir(&base_path) {
            for entry in entries.flatten() {
                if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                    let title = parse_ea_installer_xml(&entry.path())
                        .unwrap_or_else(|| entry.file_name().to_string_lossy().into_owned());

                    games.push(DetectedGame {
                        title,
                        install_dir: Some(entry.path().to_string_lossy().into_owned()),
                        save_paths: Vec::new(),
                        launcher: "ea".to_string(),
                        launcher_id: None,
                    });
                }
            }
        }
    }

    games
}

#[cfg(target_os = "windows")]
fn detect_ubisoft_windows() -> Vec<DetectedGame> {
    let mut games = Vec::new();

    // Check Ubisoft install directory
    for base in &[
        "C:\\Program Files\\Ubisoft\\Ubisoft Game Launcher\\games",
        "C:\\Program Files (x86)\\Ubisoft\\Ubisoft Game Launcher\\games",
    ] {
        let base_path = PathBuf::from(base);
        if !base_path.exists() {
            continue;
        }
        if let Ok(entries) = std::fs::read_dir(&base_path) {
            for entry in entries.flatten() {
                if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                    let dir_name = entry.file_name().to_string_lossy().into_owned();
                    games.push(DetectedGame {
                        title: dir_name,
                        install_dir: Some(entry.path().to_string_lossy().into_owned()),
                        save_paths: Vec::new(),
                        launcher: "ubisoft".to_string(),
                        launcher_id: None,
                    });
                }
            }
        }
    }

    games
}

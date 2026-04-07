//! Thin wrapper around the `steamlocate` crate for Steam library/game detection.

use std::collections::HashMap;
use std::path::PathBuf;

use steamlocate::SteamDir;

/// App IDs for Steam tools/runtimes we want to skip.
const SKIP_IDS: [u32; 6] = [228980, 1007, 1070560, 1391110, 1628350, 2180100];

fn is_tool(name: &str) -> bool {
    let lo = name.to_lowercase();
    lo.starts_with("proton ")
        || lo.starts_with("steam linux runtime")
        || lo.starts_with("steamworks common")
        || lo.contains("redistributable")
}

#[derive(Debug, Clone)]
pub struct SteamGame {
    pub app_id: u32,
    pub name: String,
    pub install_dir: String,
    pub library_path: PathBuf,
}

pub fn locate_steam() -> Result<SteamDir, String> {
    SteamDir::locate().map_err(|e| format!("Steam not found: {e}"))
}

pub fn installed_games(steam_dir: &SteamDir) -> Vec<SteamGame> {
    let mut games = Vec::new();

    let libraries = match steam_dir.libraries() {
        Ok(libs) => libs,
        Err(e) => {
            eprintln!("[DeckSave] Failed to enumerate libraries: {e}");
            return games;
        }
    };

    for lib_result in libraries {
        let library = match lib_result {
            Ok(lib) => lib,
            Err(e) => {
                eprintln!("[DeckSave] Skipping library (error): {e}");
                continue;
            }
        };
        let lib_path = library.path().to_path_buf();

        for app_result in library.apps() {
            let app = match app_result {
                Ok(a) => a,
                Err(_) => continue,
            };
            if SKIP_IDS.contains(&app.app_id) {
                continue;
            }
            let name = match &app.name {
                Some(n) => n.clone(),
                None => continue,
            };
            if is_tool(&name) {
                continue;
            }
            games.push(SteamGame {
                app_id: app.app_id,
                name,
                install_dir: app.install_dir.clone(),
                library_path: lib_path.clone(),
            });
        }
    }

    games.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    games
}

/// Returns a map of app_id → compat tool name for Proton detection.
pub fn compat_tool_mapping(steam_dir: &SteamDir) -> HashMap<u32, String> {
    steam_dir
        .compat_tool_mapping()
        .unwrap_or_default()
        .into_iter()
        .map(|(id, tool)| (id, tool.name.unwrap_or_default()))
        .collect()
}

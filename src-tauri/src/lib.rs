mod commands;
mod db;
mod manifest;
mod path_expander;
mod steam;
mod sync;
mod watcher;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

pub struct AppState {
    pub db: Mutex<rusqlite::Connection>,
    pub app_data_dir: PathBuf,
    pub header_url_cache: Mutex<HashMap<String, Option<String>>>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .setup(|app| {
            use tauri::Manager;

            let app_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&app_dir)?;
            let db_path = app_dir.join("deck-save.db");
            let conn = db::init_db(&db_path)?;
            app.manage(AppState {
                db: Mutex::new(conn),
                app_data_dir: app_dir,
                header_url_cache: Mutex::new(HashMap::new()),
            });

            // Start file watcher + auto-backup scheduler in background
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                watcher::start(handle);
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::scanner::scan_games,
            commands::scanner::get_cached_games,
            commands::scanner::get_steam_header_url,
            commands::backup::backup_game,
            commands::backup::backup_all,
            commands::backup::restore_game,
            commands::backup::get_backups,
            commands::backup::get_settings,
            commands::backup::update_setting,
            commands::sync::sync_status,
            commands::sync::sync_list_devices,
            commands::sync::sync_add_device,
            commands::sync::sync_remove_device,
            commands::sync::sync_share_folder,
            commands::sync::sync_remove_folder,
            commands::sync::sync_folder_status,
            commands::sync::sync_detect_conflicts,
            commands::sync::sync_resolve_conflict,
            commands::sync::sync_update_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

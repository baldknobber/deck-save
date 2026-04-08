mod commands;
mod db;
mod gamepad;
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
    pub syncthing_process: Mutex<Option<std::process::Child>>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Work around WebKitGTK EGL crash on Steam Deck / AMD APUs.
    // Without this, the web process aborts with:
    //   "Could not create default EGL display: EGL_BAD_PARAMETER"
    #[cfg(target_os = "linux")]
    {
        if std::env::var("WEBKIT_DISABLE_DMABUF_RENDERER").is_err() {
            std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        }
        eprintln!("[DeckSave] Platform: Linux");
        eprintln!("[DeckSave] WEBKIT_DISABLE_DMABUF_RENDERER={}", std::env::var("WEBKIT_DISABLE_DMABUF_RENDERER").unwrap_or_default());
        eprintln!("[DeckSave] XDG_SESSION_TYPE={}", std::env::var("XDG_SESSION_TYPE").unwrap_or_else(|_| "unset".into()));
        eprintln!("[DeckSave] GDK_BACKEND={}", std::env::var("GDK_BACKEND").unwrap_or_else(|_| "unset".into()));
        if let Ok(contents) = std::fs::read_to_string("/etc/os-release") {
            for line in contents.lines().take(3) {
                eprintln!("[DeckSave] {}", line);
            }
        }
    }

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
                syncthing_process: Mutex::new(None),
            });

            // ── System tray (Windows only — Linux lacks libayatana-appindicator3 in Flatpak) ──
            #[cfg(target_os = "windows")]
            setup_tray(app)?;

            // ── Close-to-tray on Windows ─────────────────────────────
            #[cfg(target_os = "windows")]
            {
                if let Some(win) = app.get_webview_window("main") {
                    let win_hide = win.clone();
                    win.on_window_event(move |event| {
                        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                            api.prevent_close();
                            let _ = win_hide.hide();
                        }
                    });
                }
            }

            // Start file watcher + auto-backup scheduler in background
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                watcher::start(handle);
            });

            // Start native gamepad polling (gilrs) — WebKitGTK lacks Gamepad API
            gamepad::start(app.handle().clone());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::scanner::scan_games,
            commands::scanner::get_cached_games,
            commands::scanner::get_steam_header_url,
            commands::scanner::add_custom_save_path,
            commands::scanner::remove_custom_save_path,
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
            commands::shortcut::check_steam_shortcut,
            commands::shortcut::register_steam_shortcut,
            commands::syncthing_mgr::check_syncthing_installed,
            commands::syncthing_mgr::install_syncthing,
            commands::syncthing_mgr::start_syncthing,
            commands::syncthing_mgr::stop_syncthing,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Build the system tray icon with Show/Quit menu.
/// On click, shows the main window. On "Quit", exits the app.
#[cfg(target_os = "windows")]
fn setup_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    use tauri::menu::{Menu, MenuItem};
    use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
    use tauri::Manager;

    let show_i = MenuItem::with_id(app, "show", "Show DeckSave", true, None::<&str>)?;
    let quit_i = MenuItem::with_id(app, "quit", "Quit DeckSave", true, None::<&str>)?;
    let tray_menu = Menu::with_items(app, &[&show_i, &quit_i])?;

    let tray_icon = tauri::image::Image::from_bytes(include_bytes!("../icons/32x32.png"))?;

    let tray = TrayIconBuilder::new()
        .icon(tray_icon)
        .tooltip("DeckSave")
        .menu(&tray_menu)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => {
                if let Some(win) = app.get_webview_window("main") {
                    let _ = win.show();
                    let _ = win.set_focus();
                }
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(win) = app.get_webview_window("main") {
                    let _ = win.show();
                    let _ = win.set_focus();
                }
            }
        })
        .build(app)?;

    // Keep tray icon alive for the app lifetime
    app.manage(tray);
    Ok(())
}

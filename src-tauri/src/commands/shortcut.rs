use crate::steam;
use serde::Serialize;
use steam_shortcuts_util::{parse_shortcuts, shortcuts_to_bytes, Shortcut};
use std::path::PathBuf;

const APP_NAME: &str = "DeckSave";

#[derive(Serialize)]
pub struct ShortcutResult {
    pub registered: bool,
    pub already_existed: bool,
}

/// Find all `shortcuts.vdf` files across Steam userdata directories.
fn find_shortcut_files() -> Result<Vec<PathBuf>, String> {
    let steam_dir = steam::locate_steam()?;
    let steam_path = steam_dir.path().to_path_buf();

    // userdata/<user_id>/config/shortcuts.vdf
    let userdata = steam_path.join("userdata");
    if !userdata.is_dir() {
        return Err("Steam userdata directory not found".into());
    }

    let mut files = Vec::new();
    let entries = std::fs::read_dir(&userdata)
        .map_err(|e| format!("Cannot read userdata dir: {e}"))?;

    for entry in entries.flatten() {
        if entry.path().is_dir() {
            let vdf = entry.path().join("config").join("shortcuts.vdf");
            // Include even if file doesn't exist yet — we'll create it
            files.push(vdf);
        }
    }

    if files.is_empty() {
        return Err("No Steam user profiles found in userdata".into());
    }
    Ok(files)
}

/// Build the exe string and launch_options for the current platform.
fn shortcut_exe_info() -> (String, String, String) {
    #[cfg(target_os = "linux")]
    {
        let exe = "/usr/bin/flatpak".to_string();
        let launch_options = "run com.baldknobber.decksave".to_string();
        let start_dir = String::new();
        (exe, start_dir, launch_options)
    }
    #[cfg(target_os = "windows")]
    {
        let exe = std::env::current_exe()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        let start_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.to_string_lossy().to_string()))
            .unwrap_or_default();
        let launch_options = String::new();
        (exe, start_dir, launch_options)
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        let exe = std::env::current_exe()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        let start_dir = String::new();
        let launch_options = String::new();
        (exe, start_dir, launch_options)
    }
}

/// Check if DeckSave is already registered by matching app_name or exe path.
fn is_decksave_shortcut(s: &Shortcut<'_>) -> bool {
    if s.app_name.eq_ignore_ascii_case(APP_NAME) {
        return true;
    }
    // On Linux: exe == "/usr/bin/flatpak" and launch_options contains our app id
    if s.launch_options.contains("com.baldknobber.decksave") {
        return true;
    }
    // On Windows: exe path ends with deck-save.exe (case-insensitive)
    if s.exe.to_lowercase().ends_with("deck-save.exe") {
        return true;
    }
    false
}

/// Locate the app icon for Steam shortcut (absolute path).
fn find_icon_path() -> String {
    // Flatpak: icon installed via desktop-file convention
    #[cfg(target_os = "linux")]
    {
        let candidates = [
            "/app/share/icons/hicolor/128x128/apps/com.baldknobber.decksave.png",
            "/app/share/icons/hicolor/256x256/apps/com.baldknobber.decksave.png",
        ];
        for p in &candidates {
            if PathBuf::from(p).exists() {
                return p.to_string();
            }
        }
        // Fall back to any installed icon
        if let Ok(home) = std::env::var("HOME") {
            let local = format!("{home}/.local/share/icons/hicolor/128x128/apps/com.baldknobber.decksave.png");
            if PathBuf::from(&local).exists() {
                return local;
            }
        }
    }
    // Windows: use the exe itself (Steam reads its embedded icon)
    #[cfg(target_os = "windows")]
    {
        if let Ok(exe) = std::env::current_exe() {
            return exe.to_string_lossy().to_string();
        }
    }
    String::new()
}

#[tauri::command]
pub fn check_steam_shortcut() -> Result<bool, String> {
    let vdf_files = find_shortcut_files()?;

    for vdf_path in &vdf_files {
        if !vdf_path.exists() {
            continue;
        }
        let content = std::fs::read(vdf_path)
            .map_err(|e| format!("Cannot read {}: {e}", vdf_path.display()))?;
        if content.is_empty() {
            continue;
        }
        let shortcuts = parse_shortcuts(&content)
            .map_err(|e| format!("Cannot parse {}: {e}", vdf_path.display()))?;
        if shortcuts.iter().any(is_decksave_shortcut) {
            return Ok(true);
        }
    }
    Ok(false)
}

#[tauri::command]
pub fn register_steam_shortcut() -> Result<ShortcutResult, String> {
    let vdf_files = find_shortcut_files()?;
    let (exe, start_dir, launch_options) = shortcut_exe_info();

    let mut any_registered = false;
    let mut any_existed = false;

    for vdf_path in &vdf_files {
        // Ensure config dir exists
        if let Some(parent) = vdf_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        // Read existing shortcuts (empty file / missing = empty vec)
        let content = if vdf_path.exists() {
            std::fs::read(vdf_path)
                .map_err(|e| format!("Cannot read {}: {e}", vdf_path.display()))?
        } else {
            Vec::new()
        };

        let mut shortcuts: Vec<Shortcut<'_>> = if content.is_empty() {
            Vec::new()
        } else {
            parse_shortcuts(&content)
                .map_err(|e| format!("Cannot parse {}: {e}", vdf_path.display()))?
        };

        // Check if already present
        if shortcuts.iter().any(is_decksave_shortcut) {
            any_existed = true;
            continue;
        }

        // Determine next order index
        let order_str = shortcuts.len().to_string();

        let icon = find_icon_path();
        let new_shortcut = Shortcut::new(
            &order_str,
            APP_NAME,
            &exe,
            &start_dir,
            &icon,
            "",            // shortcut_path
            &launch_options,
        );

        shortcuts.push(new_shortcut);

        let bytes = shortcuts_to_bytes(&shortcuts);
        std::fs::write(vdf_path, bytes)
            .map_err(|e| format!("Cannot write {}: {e}", vdf_path.display()))?;

        any_registered = true;
    }

    Ok(ShortcutResult {
        registered: any_registered,
        already_existed: any_existed && !any_registered,
    })
}

use crate::steam;
use serde::Serialize;
use steam_shortcuts_util::{
    calculate_app_id_for_shortcut, parse_shortcuts, shortcuts_to_bytes, Shortcut,
};
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
            // Skip userdata/0 — it's a placeholder, not a real user profile
            if entry.file_name() == "0" {
                continue;
            }
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
/// Steam expects exe and start_dir to be wrapped in double quotes.
fn shortcut_exe_info() -> (String, String, String) {
    #[cfg(target_os = "linux")]
    {
        let in_flatpak = std::path::Path::new("/.flatpak-info").exists();
        if in_flatpak {
            let exe = "\"/usr/bin/flatpak\"".to_string();
            let launch_options = "run com.baldknobber.decksave".to_string();
            let start_dir = String::new();
            (exe, start_dir, launch_options)
        } else {
            let raw = std::env::current_exe()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();
            let exe = format!("\"{raw}\"");
            let start_dir = std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|d| format!("\"{}\"", d.to_string_lossy())))
                .unwrap_or_default();
            let launch_options = String::new();
            (exe, start_dir, launch_options)
        }
    }
    #[cfg(target_os = "windows")]
    {
        let raw = std::env::current_exe()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        let exe = format!("\"{raw}\"");
        let start_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| format!("\"{}\"", d.to_string_lossy())))
            .unwrap_or_default();
        let launch_options = String::new();
        (exe, start_dir, launch_options)
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        let raw = std::env::current_exe()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        let exe = format!("\"{raw}\"");
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
    // Check exe filename — strip surrounding quotes that Steam includes
    let exe = s.exe.trim_matches('"');
    let exe_lower = exe.to_lowercase();
    if exe_lower.ends_with("deck-save.exe") || exe_lower.ends_with("deck-save") {
        return true;
    }
    false
}

/// Locate the app icon for Steam shortcut (absolute path).
/// On Flatpak, copies the icon from the sandbox to a host-visible path
/// since Steam reads shortcuts.vdf from the host filesystem.
fn find_icon_path() -> String {
    #[cfg(target_os = "linux")]
    {
        // Check if running inside Flatpak
        let in_flatpak = std::path::Path::new("/.flatpak-info").exists();

        if in_flatpak {
            // Flatpak: /app/ paths aren't visible to the host.
            // Copy to ~/.local/share/icons/ which IS on the host filesystem.
            if let Ok(home) = std::env::var("HOME") {
                let host_icon = format!(
                    "{home}/.local/share/icons/hicolor/128x128/apps/com.baldknobber.decksave.png"
                );
                // If already copied, use it
                if PathBuf::from(&host_icon).exists() {
                    return host_icon;
                }
                // Try to copy from sandbox /app/ path
                let sandbox_candidates = [
                    "/app/share/icons/hicolor/128x128/apps/com.baldknobber.decksave.png",
                    "/app/share/icons/hicolor/256x256/apps/com.baldknobber.decksave.png",
                ];
                for src in &sandbox_candidates {
                    if PathBuf::from(src).exists() {
                        let dest = PathBuf::from(&host_icon);
                        if let Some(parent) = dest.parent() {
                            let _ = std::fs::create_dir_all(parent);
                        }
                        if std::fs::copy(src, &dest).is_ok() {
                            return host_icon;
                        }
                    }
                }
                // Fallback: return host path even if copy failed
                return host_icon;
            }
        } else {
            // Not Flatpak: check standard icon locations
            let candidates = [
                "/app/share/icons/hicolor/128x128/apps/com.baldknobber.decksave.png",
                "/app/share/icons/hicolor/256x256/apps/com.baldknobber.decksave.png",
            ];
            for p in &candidates {
                if PathBuf::from(p).exists() {
                    return p.to_string();
                }
            }
            if let Ok(home) = std::env::var("HOME") {
                let local = format!(
                    "{home}/.local/share/icons/hicolor/128x128/apps/com.baldknobber.decksave.png"
                );
                if PathBuf::from(&local).exists() {
                    return local;
                }
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

/// Copy the app icon into the Steam grid folder so the shortcut has artwork
/// in Gaming Mode. Uses `calculate_app_id_for_shortcut` from steam_shortcuts_util
/// to derive the correct filename.
fn copy_grid_artwork(vdf_path: &PathBuf, shortcut: &Shortcut<'_>, icon_source: &str) {
    // Grid folder is sibling to shortcuts.vdf: userdata/<id>/config/grid/
    let grid_dir = match vdf_path.parent() {
        Some(config) => config.join("grid"),
        None => return,
    };
    let _ = std::fs::create_dir_all(&grid_dir);

    let app_id = calculate_app_id_for_shortcut(shortcut);

    // If the icon source is a PNG, copy it as the grid image
    if !icon_source.is_empty() && PathBuf::from(icon_source).exists() {
        let grid_file = grid_dir.join(format!("{app_id}.png"));
        if !grid_file.exists() {
            let _ = std::fs::copy(icon_source, &grid_file);
        }
        let logo_file = grid_dir.join(format!("{app_id}_logo.png"));
        if !logo_file.exists() {
            let _ = std::fs::copy(icon_source, &logo_file);
        }
    }
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

        // Remove any existing DeckSave entries so we always write a fresh, correct one
        let had_existing = shortcuts.iter().any(is_decksave_shortcut);
        if had_existing {
            any_existed = true;
        }
        shortcuts.retain(|s| !is_decksave_shortcut(s));

        // Determine next order index
        let order_str = shortcuts.len().to_string();

        let icon = find_icon_path();
        let mut new_shortcut = Shortcut::new(
            &order_str,
            APP_NAME,
            &exe,
            &start_dir,
            &icon,
            "",            // shortcut_path
            &launch_options,
        );
        new_shortcut.tags = vec!["DeckSave"];

        copy_grid_artwork(vdf_path, &new_shortcut, &icon);

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

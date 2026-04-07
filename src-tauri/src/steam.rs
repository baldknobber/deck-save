//! Steam library detection, VDF parsing, and game/save enumeration.
//! Cross-platform: Windows + Linux (Steam Deck).

use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

// ── VDF (Valve KeyValues) Parser ────────────────────────────

#[derive(Debug, Clone)]
pub enum VdfValue {
    Str(String),
    Obj(HashMap<String, VdfValue>),
}

impl VdfValue {
    pub fn as_str(&self) -> Option<&str> {
        if let VdfValue::Str(s) = self {
            Some(s)
        } else {
            None
        }
    }
    pub fn as_obj(&self) -> Option<&HashMap<String, VdfValue>> {
        if let VdfValue::Obj(m) = self {
            Some(m)
        } else {
            None
        }
    }
    pub fn get(&self, key: &str) -> Option<&VdfValue> {
        self.as_obj()?.get(key)
    }
    pub fn get_str(&self, key: &str) -> Option<&str> {
        self.get(key)?.as_str()
    }
}

/// Parse a Valve VDF (KeyValues) text file into a nested structure.
pub fn parse_vdf(content: &str) -> VdfValue {
    let mut chars = content.chars().peekable();
    VdfValue::Obj(parse_obj(&mut chars))
}

fn parse_obj(it: &mut std::iter::Peekable<std::str::Chars>) -> HashMap<String, VdfValue> {
    let mut map = HashMap::new();
    loop {
        eat_ws(it);
        match it.peek() {
            None | Some('}') => {
                it.next();
                break;
            }
            Some('"') => {
                let key = read_str(it);
                eat_ws(it);
                match it.peek() {
                    Some('"') => {
                        map.insert(key, VdfValue::Str(read_str(it)));
                    }
                    Some('{') => {
                        it.next();
                        map.insert(key, VdfValue::Obj(parse_obj(it)));
                    }
                    _ => break,
                }
            }
            Some('/') => {
                it.next();
                if it.peek() == Some(&'/') {
                    for c in it.by_ref() {
                        if c == '\n' {
                            break;
                        }
                    }
                }
            }
            _ => {
                it.next();
            }
        }
    }
    map
}

fn read_str(it: &mut std::iter::Peekable<std::str::Chars>) -> String {
    it.next(); // opening "
    let mut s = String::new();
    loop {
        match it.next() {
            None | Some('"') => break,
            Some('\\') => match it.next() {
                Some('n') => s.push('\n'),
                Some('t') => s.push('\t'),
                Some('\\') => s.push('\\'),
                Some('"') => s.push('"'),
                Some(other) => {
                    s.push('\\');
                    s.push(other);
                }
                None => break,
            },
            Some(c) => s.push(c),
        }
    }
    s
}

fn eat_ws(it: &mut std::iter::Peekable<std::str::Chars>) {
    while it.peek().map_or(false, |c| c.is_whitespace()) {
        it.next();
    }
}

// ── Steam Root Detection ────────────────────────────────────

/// Locate the Steam root directory on the current OS.
pub fn find_steam_root() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        return find_root_win();
    }
    #[cfg(target_os = "linux")]
    {
        return find_root_linux();
    }
    #[allow(unreachable_code)]
    None
}

#[cfg(target_os = "windows")]
fn find_root_win() -> Option<PathBuf> {
    use winreg::enums::*;
    use winreg::RegKey;

    // HKCU is most reliable (Steam writes SteamPath here)
    if let Ok(k) = RegKey::predef(HKEY_CURRENT_USER).open_subkey("SOFTWARE\\Valve\\Steam") {
        if let Ok(p) = k.get_value::<String, _>("SteamPath") {
            let p = PathBuf::from(p);
            if p.join("steamapps").exists() {
                return Some(p);
            }
        }
    }
    // HKLM fallbacks
    for sub in [
        "SOFTWARE\\Valve\\Steam",
        "SOFTWARE\\WOW6432Node\\Valve\\Steam",
    ] {
        if let Ok(k) = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(sub) {
            if let Ok(p) = k.get_value::<String, _>("InstallPath") {
                let p = PathBuf::from(p);
                if p.join("steamapps").exists() {
                    return Some(p);
                }
            }
        }
    }
    // Common default paths
    for fp in [
        r"C:\Program Files (x86)\Steam",
        r"C:\Program Files\Steam",
        r"D:\Steam",
        r"D:\SteamLibrary",
    ] {
        let p = PathBuf::from(fp);
        if p.join("steamapps").exists() {
            return Some(p);
        }
    }
    None
}

#[cfg(target_os = "linux")]
fn find_root_linux() -> Option<PathBuf> {
    let home = PathBuf::from(std::env::var("HOME").ok()?);
    for rel in [
        ".steam/steam",
        ".local/share/Steam",
        "snap/steam/common/.steam/steam",
        ".var/app/com.valvesoftware.Steam/.steam/steam",
        ".var/app/com.valvesoftware.Steam/.local/share/Steam",
    ] {
        let c = home.join(rel);
        let resolved = fs::canonicalize(&c).unwrap_or(c);
        if resolved.join("steamapps").exists() {
            return Some(resolved);
        }
    }
    None
}

// ── Library Folder Enumeration ──────────────────────────────

/// Return all Steam library folders (may span multiple drives).
pub fn library_folders(steam_root: &Path) -> Vec<PathBuf> {
    let vdf_path = steam_root.join("steamapps").join("libraryfolders.vdf");
    let content = match fs::read_to_string(&vdf_path) {
        Ok(c) => c,
        Err(_) => return vec![steam_root.to_path_buf()],
    };
    let root = parse_vdf(&content);
    let mut libs = Vec::new();
    if let Some(lf) = root.get("libraryfolders").and_then(|v| v.as_obj()) {
        for val in lf.values() {
            if let Some(p) = val.get_str("path") {
                let pb = PathBuf::from(p);
                if pb.exists() {
                    libs.push(pb);
                }
            }
        }
    }
    if !libs.iter().any(|p| p == steam_root) {
        libs.push(steam_root.to_path_buf());
    }
    libs
}

// ── Game Enumeration ────────────────────────────────────────

/// App IDs for Steam tools/runtimes (not games).
const SKIP_IDS: &[&str] = &[
    "228980", "1007", "1070560", "1391110", "1628350", "2180100",
];

fn is_tool(name: &str) -> bool {
    let lo = name.to_lowercase();
    lo.starts_with("proton ")
        || lo.starts_with("steam linux runtime")
        || lo.starts_with("steamworks common")
        || lo.contains("redistributable")
}

#[derive(Debug, Clone)]
pub struct SteamGame {
    pub app_id: String,
    pub name: String,
    pub install_dir: String,
    pub library: PathBuf,
}

/// Scan every library folder for installed games via appmanifest ACF files.
pub fn installed_games(libs: &[PathBuf]) -> Vec<SteamGame> {
    let mut games = Vec::new();

    for lib in libs {
        let sa = lib.join("steamapps");
        let entries = match fs::read_dir(&sa) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for ent in entries.flatten() {
            let fname = ent.file_name();
            let fname = fname.to_string_lossy();
            if !(fname.starts_with("appmanifest_") && fname.ends_with(".acf")) {
                continue;
            }
            let text = match fs::read_to_string(ent.path()) {
                Ok(t) => t,
                Err(_) => continue,
            };
            let vdf = parse_vdf(&text);
            let state = vdf.get("AppState").unwrap_or(&vdf);

            let app_id = match state.get_str("appid") {
                Some(v) => v.to_string(),
                None => continue,
            };
            if SKIP_IDS.contains(&app_id.as_str()) {
                continue;
            }

            let name = match state.get_str("name") {
                Some(v) => v.to_string(),
                None => continue,
            };
            if is_tool(&name) {
                continue;
            }

            let install_dir = state
                .get_str("installdir")
                .unwrap_or(&name)
                .to_string();

            // StateFlags bit 2 (value 4) = fully installed
            let flags: u32 = state
                .get_str("StateFlags")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            if flags & 4 == 0 {
                continue;
            }

            games.push(SteamGame {
                app_id,
                name,
                install_dir,
                library: lib.clone(),
            });
        }
    }

    games.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    games
}

// ── Save-Path Detection ─────────────────────────────────────

/// Build name variants for matching save directories to a game.
fn search_names(game: &SteamGame) -> Vec<String> {
    let mut ns: Vec<String> = Vec::new();
    ns.push(game.install_dir.clone());
    ns.push(game.name.clone());

    // Strip subtitle ("Game: Subtitle" → "Game")
    if let Some(i) = game.name.find(':') {
        ns.push(game.name[..i].trim().to_string());
    }
    if let Some(i) = game.name.find(" - ") {
        ns.push(game.name[..i].trim().to_string());
    }
    // Drop leading "The "
    if let Some(rest) = game.name.strip_prefix("The ") {
        ns.push(rest.to_string());
    }
    // Alphanumeric only
    let an: String = game.name.chars().filter(|c| c.is_alphanumeric()).collect();
    if !ns.contains(&an) {
        ns.push(an);
    }
    // CamelCase join
    let cc: String = game.name.split_whitespace().collect();
    if !ns.contains(&cc) {
        ns.push(cc);
    }
    ns
}

/// OS-specific directories that commonly hold per-game save folders.
fn save_roots() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    #[cfg(target_os = "windows")]
    {
        if let Ok(v) = std::env::var("APPDATA") {
            dirs.push(PathBuf::from(v));
        }
        if let Ok(v) = std::env::var("LOCALAPPDATA") {
            dirs.push(PathBuf::from(v));
        }
        if let Ok(v) = std::env::var("USERPROFILE") {
            let u = PathBuf::from(v);
            dirs.push(u.join("AppData").join("LocalLow"));
            dirs.push(u.join("Documents").join("My Games"));
            dirs.push(u.join("Documents"));
            dirs.push(u.join("Saved Games"));
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Ok(h) = std::env::var("HOME") {
            let h = PathBuf::from(h);
            dirs.push(h.join(".local/share"));
            dirs.push(h.join(".config"));
        }
        if let Ok(v) = std::env::var("XDG_DATA_HOME") {
            dirs.push(PathBuf::from(v));
        }
        if let Ok(v) = std::env::var("XDG_CONFIG_HOME") {
            dirs.push(PathBuf::from(v));
        }
    }

    dirs
}

/// Sub-directory names inside a game's install folder that typically hold saves.
const SAVE_SUBDIRS: &[&str] = &[
    "Saves",
    "saves",
    "SaveGames",
    "savegames",
    "Save",
    "save",
    "SaveData",
    "savedata",
    "save_data",
    "Profiles",
    "profiles",
];

/// Locate save-file directories for a given game.
pub fn find_save_paths(game: &SteamGame, steam_root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();

    macro_rules! add {
        ($p:expr) => {
            let p: PathBuf = $p;
            if p.exists() && seen.insert(p.clone()) {
                out.push(p);
            }
        };
    }

    // 1 ── Steam Cloud saves (userdata/<userid>/<appid>/) ────
    let ud = steam_root.join("userdata");
    if let Ok(users) = fs::read_dir(&ud) {
        for u in users.flatten() {
            let app = u.path().join(&game.app_id);
            if app.exists() {
                let remote = app.join("remote");
                add!(if remote.exists() { remote } else { app });
            }
        }
    }

    // 2 ── Name-matched folders in common OS save directories ─
    let names = search_names(game);
    for root in save_roots() {
        if !root.exists() {
            continue;
        }
        if let Ok(entries) = fs::read_dir(&root) {
            for ent in entries.flatten() {
                if !ent.path().is_dir() {
                    continue;
                }
                let ename = ent.file_name().to_string_lossy().to_lowercase();
                if names.iter().any(|n| ename == n.to_lowercase()) {
                    add!(ent.path());
                }
            }
        }
    }

    // 3 ── Known save sub-directories inside game install folder
    let game_dir = game
        .library
        .join("steamapps")
        .join("common")
        .join(&game.install_dir);
    for sub in SAVE_SUBDIRS {
        add!(game_dir.join(sub));
    }

    // 4 ── Proton prefix (Linux / Steam Deck) ────────────────
    #[cfg(target_os = "linux")]
    {
        let pfx = game
            .library
            .join("steamapps/compatdata")
            .join(&game.app_id)
            .join("pfx");
        if pfx.exists() {
            let su = pfx.join("drive_c/users/steamuser");
            for rel in [
                "AppData/Roaming",
                "AppData/Local",
                "AppData/LocalLow",
                "Documents/My Games",
                "Documents",
                "Saved Games",
            ] {
                let dir = su.join(rel);
                if !dir.exists() {
                    continue;
                }
                if let Ok(entries) = fs::read_dir(&dir) {
                    for ent in entries.flatten() {
                        if !ent.path().is_dir() {
                            continue;
                        }
                        let ename = ent.file_name().to_string_lossy().to_lowercase();
                        if names.iter().any(|n| ename == n.to_lowercase()) {
                            add!(ent.path());
                        }
                    }
                }
            }
        }
    }

    out
}

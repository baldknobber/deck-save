//! Expand Ludusavi manifest path placeholders into real filesystem paths.
//! Handles Windows, Linux, and Proton prefix mapping.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Context needed to resolve all Ludusavi placeholders for a single game.
#[allow(dead_code)]
pub struct ExpansionContext {
    pub steam_root: PathBuf,
    pub library_path: PathBuf,
    pub install_dir: String,
    pub app_id: u32,
    pub is_proton: bool,
}

impl ExpansionContext {
    fn placeholders(&self) -> HashMap<&'static str, String> {
        let mut m = HashMap::new();

        let base = self
            .library_path
            .join("steamapps")
            .join("common")
            .join(&self.install_dir);

        m.insert("<root>", self.library_path.to_string_lossy().into_owned());
        m.insert("<game>", self.install_dir.clone());
        m.insert("<base>", base.to_string_lossy().into_owned());
        m.insert("<storeUserId>", "*".to_string());

        // OS username
        #[cfg(target_os = "windows")]
        if let Ok(u) = std::env::var("USERNAME") {
            m.insert("<osUserName>", u);
        }
        #[cfg(target_os = "linux")]
        if let Ok(u) = std::env::var("USER") {
            m.insert("<osUserName>", u);
        }

        // Home directory
        #[cfg(target_os = "windows")]
        {
            if let Ok(h) = std::env::var("USERPROFILE") {
                let home = PathBuf::from(&h);
                m.insert("<home>", h);
                m.insert(
                    "<winAppData>",
                    std::env::var("APPDATA").unwrap_or_else(|_| {
                        home.join("AppData\\Roaming")
                            .to_string_lossy()
                            .into_owned()
                    }),
                );
                m.insert(
                    "<winLocalAppData>",
                    std::env::var("LOCALAPPDATA").unwrap_or_else(|_| {
                        home.join("AppData\\Local")
                            .to_string_lossy()
                            .into_owned()
                    }),
                );
                m.insert(
                    "<winLocalAppDataLow>",
                    home.join("AppData\\LocalLow")
                        .to_string_lossy()
                        .into_owned(),
                );
                m.insert(
                    "<winDocuments>",
                    home.join("Documents").to_string_lossy().into_owned(),
                );
                m.insert("<winPublic>", "C:\\Users\\Public".to_string());
                m.insert(
                    "<winProgramData>",
                    std::env::var("ProgramData")
                        .unwrap_or_else(|_| "C:\\ProgramData".to_string()),
                );
            }
        }

        #[cfg(target_os = "linux")]
        {
            if let Ok(h) = std::env::var("HOME") {
                let home = PathBuf::from(&h);
                m.insert("<home>", h);
                m.insert(
                    "<xdgData>",
                    std::env::var("XDG_DATA_HOME").unwrap_or_else(|_| {
                        home.join(".local/share").to_string_lossy().into_owned()
                    }),
                );
                m.insert(
                    "<xdgConfig>",
                    std::env::var("XDG_CONFIG_HOME").unwrap_or_else(|_| {
                        home.join(".config").to_string_lossy().into_owned()
                    }),
                );
            }

            // Proton prefix: map Windows placeholders to compatdata paths
            if self.is_proton {
                let pfx = self
                    .library_path
                    .join("steamapps/compatdata")
                    .join(self.app_id.to_string())
                    .join("pfx/drive_c/users/steamuser");

                m.insert(
                    "<winAppData>",
                    pfx.join("AppData/Roaming")
                        .to_string_lossy()
                        .into_owned(),
                );
                m.insert(
                    "<winLocalAppData>",
                    pfx.join("AppData/Local").to_string_lossy().into_owned(),
                );
                m.insert(
                    "<winLocalAppDataLow>",
                    pfx.join("AppData/LocalLow")
                        .to_string_lossy()
                        .into_owned(),
                );
                m.insert(
                    "<winDocuments>",
                    pfx.join("Documents").to_string_lossy().into_owned(),
                );

                let pfx_root = self
                    .library_path
                    .join("steamapps/compatdata")
                    .join(self.app_id.to_string())
                    .join("pfx/drive_c");
                m.insert(
                    "<winPublic>",
                    pfx_root
                        .join("users/Public")
                        .to_string_lossy()
                        .into_owned(),
                );
                m.insert(
                    "<winProgramData>",
                    pfx_root.join("ProgramData").to_string_lossy().into_owned(),
                );
            }
        }

        m
    }
}

/// Expand a Ludusavi path template into concrete filesystem paths.
/// Returns only paths that actually exist on disk (after glob resolution).
pub fn expand_path(template: &str, ctx: &ExpansionContext) -> Vec<PathBuf> {
    let placeholders = ctx.placeholders();

    let mut expanded = template.to_string();
    for (placeholder, value) in &placeholders {
        expanded = expanded.replace(placeholder, value);
    }

    // If unresolved placeholders remain, this path isn't relevant for our OS
    if expanded.contains('<') && expanded.contains('>') {
        return vec![];
    }

    // Normalize separators for glob
    let expanded = expanded.replace('\\', "/");

    // Use glob to resolve wildcards (from <storeUserId> etc.)
    match glob::glob(&expanded) {
        Ok(entries) => {
            let mut results = Vec::new();
            for entry in entries.flatten() {
                // For file paths, use parent directory as the save location
                if entry.is_file() {
                    if let Some(parent) = entry.parent() {
                        if !results.contains(&parent.to_path_buf()) {
                            results.push(parent.to_path_buf());
                        }
                    }
                } else if entry.is_dir() && !results.contains(&entry) {
                    results.push(entry);
                }
            }
            results
        }
        Err(_) => {
            // Glob parse failed — try as literal path
            let p = PathBuf::from(&expanded);
            if p.exists() {
                if p.is_file() {
                    p.parent().map(|pp| vec![pp.to_path_buf()]).unwrap_or_default()
                } else {
                    vec![p]
                }
            } else {
                // Check if parent directory exists (path may point to specific save files)
                let parent = p.parent().map(Path::to_path_buf);
                match parent {
                    Some(pp) if pp.exists() && pp.is_dir() => vec![pp],
                    _ => vec![],
                }
            }
        }
    }
}

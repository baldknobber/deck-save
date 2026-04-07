//! Download, cache, and parse the Ludusavi manifest (19K+ game save path database).

use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

const MANIFEST_URL: &str =
    "https://raw.githubusercontent.com/mtkennerly/ludusavi-manifest/master/data/manifest.yaml";
const MANIFEST_FILENAME: &str = "ludusavi-manifest.yaml";
const MAX_AGE: Duration = Duration::from_secs(24 * 60 * 60);

// ── Data structures matching the Ludusavi manifest YAML ─────

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ManifestGame {
    #[serde(default)]
    pub files: BTreeMap<String, FileEntry>,
    #[serde(default)]
    pub steam: Option<SteamMeta>,
    #[serde(rename = "installDir", default)]
    pub install_dir: BTreeMap<String, serde_yaml::Value>,
    #[serde(default)]
    pub registry: BTreeMap<String, serde_yaml::Value>,
}

#[derive(Debug, Deserialize, Default)]
#[allow(dead_code)]
pub struct FileEntry {
    #[serde(default)]
    pub when: Vec<Constraint>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct Constraint {
    pub os: Option<String>,
    pub store: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SteamMeta {
    pub id: Option<u32>,
}

// ── Download + cache ────────────────────────────────────────

/// Ensure a fresh manifest YAML is cached locally. Returns the cache path.
pub fn ensure_manifest(cache_dir: &Path) -> Result<PathBuf, String> {
    let cache_path = cache_dir.join(MANIFEST_FILENAME);

    // Use cached version if it exists and is less than 24h old
    if cache_path.exists() {
        if let Ok(meta) = fs::metadata(&cache_path) {
            if let Ok(modified) = meta.modified() {
                if modified.elapsed().unwrap_or(MAX_AGE) < MAX_AGE {
                    return Ok(cache_path);
                }
            }
        }
    }

    // Download fresh copy
    let resp = ureq::get(MANIFEST_URL)
        .call()
        .map_err(|e| format!("Failed to download Ludusavi manifest: {e}"))?;

    let mut body = Vec::new();
    resp.into_reader()
        .take(20_000_000) // 20MB safety limit
        .read_to_end(&mut body)
        .map_err(|e| format!("Failed to read manifest response: {e}"))?;

    fs::write(&cache_path, &body)
        .map_err(|e| format!("Failed to cache manifest: {e}"))?;

    Ok(cache_path)
}

// ── Parse + index ───────────────────────────────────────────

pub fn load_manifest(path: &Path) -> Result<BTreeMap<String, ManifestGame>, String> {
    let content =
        fs::read_to_string(path).map_err(|e| format!("Failed to read manifest file: {e}"))?;
    serde_yaml::from_str(&content).map_err(|e| format!("Failed to parse manifest YAML: {e}"))
}

/// Build a fast lookup index: steam app_id → game name in the manifest.
pub fn build_steam_index(manifest: &BTreeMap<String, ManifestGame>) -> HashMap<u32, String> {
    let mut index = HashMap::new();
    for (name, game) in manifest {
        if let Some(steam) = &game.steam {
            if let Some(id) = steam.id {
                index.insert(id, name.clone());
            }
        }
    }
    index
}

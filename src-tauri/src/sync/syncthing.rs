//! Syncthing REST API client.
//!
//! Communicates with a locally running Syncthing instance to manage device
//! pairing, folder sharing, sync status, and conflict detection.
//! Docs: https://docs.syncthing.net/dev/rest.html

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Default Syncthing REST API base URL.
const DEFAULT_BASE: &str = "http://127.0.0.1:8384";

/// Client for interacting with the Syncthing REST API.
#[derive(Debug, Clone)]
pub struct SyncthingClient {
    base_url: String,
    api_key: String,
}

// ── API response types ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStatus {
    #[serde(rename = "myID")]
    pub my_id: String,
    pub uptime: Option<u64>,
    #[serde(rename = "startTime")]
    pub start_time: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceConfig {
    #[serde(rename = "deviceID")]
    pub device_id: String,
    pub name: String,
    #[serde(default)]
    pub addresses: Vec<String>,
    #[serde(default)]
    pub paused: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderConfig {
    pub id: String,
    pub label: String,
    pub path: String,
    #[serde(rename = "type")]
    pub folder_type: String,
    #[serde(default)]
    pub devices: Vec<FolderDevice>,
    #[serde(default)]
    pub paused: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderDevice {
    #[serde(rename = "deviceID")]
    pub device_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Completion {
    pub completion: f64,
    #[serde(rename = "globalBytes")]
    pub global_bytes: Option<i64>,
    #[serde(rename = "needBytes")]
    pub need_bytes: Option<i64>,
    #[serde(rename = "globalItems")]
    pub global_items: Option<i64>,
    #[serde(rename = "needItems")]
    pub need_items: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionInfo {
    pub total: ConnectionTotal,
    pub connections: std::collections::HashMap<String, ConnectionDevice>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionTotal {
    #[serde(rename = "inBytesTotal")]
    pub in_bytes_total: i64,
    #[serde(rename = "outBytesTotal")]
    pub out_bytes_total: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionDevice {
    pub connected: bool,
    pub paused: bool,
    #[serde(rename = "clientVersion")]
    pub client_version: Option<String>,
    pub address: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConflictFile {
    pub path: String,
    pub game_folder: String,
}

// ── Public API types returned to frontend ───────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct SyncStatus {
    pub running: bool,
    pub my_device_id: String,
    pub uptime: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeviceInfo {
    pub device_id: String,
    pub name: String,
    pub connected: bool,
    pub paused: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct FolderStatus {
    pub id: String,
    pub label: String,
    pub path: String,
    pub folder_type: String,
    pub paused: bool,
    pub completion: f64,
}

// ── Implementation ──────────────────────────────────────────────────

impl SyncthingClient {
    pub fn new(api_key: &str) -> Self {
        Self {
            base_url: DEFAULT_BASE.to_string(),
            api_key: api_key.to_string(),
        }
    }

    pub fn with_base_url(api_key: &str, base_url: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key: api_key.to_string(),
        }
    }

    // ── Helpers ─────────────────────────────────────────────────────

    fn get(&self, path: &str) -> Result<ureq::Response, String> {
        ureq::get(&format!("{}{}", self.base_url, path))
            .set("X-API-Key", &self.api_key)
            .call()
            .map_err(|e| format!("Syncthing API error: {e}"))
    }

    fn post_json(&self, path: &str, body: &impl Serialize) -> Result<ureq::Response, String> {
        let json = serde_json::to_string(body).map_err(|e| format!("JSON serialize: {e}"))?;
        ureq::post(&format!("{}{}", self.base_url, path))
            .set("X-API-Key", &self.api_key)
            .set("Content-Type", "application/json")
            .send_string(&json)
            .map_err(|e| format!("Syncthing API error: {e}"))
    }

    fn put_json(&self, path: &str, body: &impl Serialize) -> Result<ureq::Response, String> {
        let json = serde_json::to_string(body).map_err(|e| format!("JSON serialize: {e}"))?;
        ureq::put(&format!("{}{}", self.base_url, path))
            .set("X-API-Key", &self.api_key)
            .set("Content-Type", "application/json")
            .send_string(&json)
            .map_err(|e| format!("Syncthing API error: {e}"))
    }

    fn delete(&self, path: &str) -> Result<ureq::Response, String> {
        ureq::delete(&format!("{}{}", self.base_url, path))
            .set("X-API-Key", &self.api_key)
            .call()
            .map_err(|e| format!("Syncthing API error: {e}"))
    }

    // ── System ──────────────────────────────────────────────────────

    /// Get system status including local device ID.
    pub fn system_status(&self) -> Result<SystemStatus, String> {
        let resp = self.get("/rest/system/status")?;
        resp.into_json::<SystemStatus>()
            .map_err(|e| format!("Parse system status: {e}"))
    }

    /// Get combined sync status.
    pub fn sync_status(&self) -> Result<SyncStatus, String> {
        let status = self.system_status()?;
        Ok(SyncStatus {
            running: true,
            my_device_id: status.my_id,
            uptime: status.uptime.unwrap_or(0),
        })
    }

    // ── Devices ─────────────────────────────────────────────────────

    /// List all configured devices (excluding self).
    pub fn list_devices(&self) -> Result<Vec<DeviceConfig>, String> {
        let resp = self.get("/rest/config/devices")?;
        let devices: Vec<DeviceConfig> = resp
            .into_json()
            .map_err(|e| format!("Parse devices: {e}"))?;

        let my_id = self.system_status()?.my_id;
        Ok(devices.into_iter().filter(|d| d.device_id != my_id).collect())
    }

    /// List devices with connection status.
    pub fn list_devices_with_status(&self) -> Result<Vec<DeviceInfo>, String> {
        let devices = self.list_devices()?;
        let connections = self.connections()?;

        Ok(devices
            .into_iter()
            .map(|d| {
                let conn = connections.connections.get(&d.device_id);
                DeviceInfo {
                    device_id: d.device_id.clone(),
                    name: d.name,
                    connected: conn.map_or(false, |c| c.connected),
                    paused: d.paused,
                }
            })
            .collect())
    }

    /// Add a new device.
    pub fn add_device(&self, device_id: &str, name: &str) -> Result<(), String> {
        let device = DeviceConfig {
            device_id: device_id.to_string(),
            name: name.to_string(),
            addresses: vec!["dynamic".to_string()],
            paused: false,
        };
        self.post_json("/rest/config/devices", &device)?;
        Ok(())
    }

    /// Remove a device by ID.
    pub fn remove_device(&self, device_id: &str) -> Result<(), String> {
        // Get current config, filter out the device, put it back
        let resp = self.get("/rest/config/devices")?;
        let mut devices: Vec<DeviceConfig> = resp
            .into_json()
            .map_err(|e| format!("Parse devices: {e}"))?;
        devices.retain(|d| d.device_id != device_id);
        self.put_json("/rest/config/devices", &devices)?;
        Ok(())
    }

    /// Get connection info for all devices.
    pub fn connections(&self) -> Result<ConnectionInfo, String> {
        let resp = self.get("/rest/system/connections")?;
        resp.into_json::<ConnectionInfo>()
            .map_err(|e| format!("Parse connections: {e}"))
    }

    // ── Folders ─────────────────────────────────────────────────────

    /// List all configured folders.
    pub fn list_folders(&self) -> Result<Vec<FolderConfig>, String> {
        let resp = self.get("/rest/config/folders")?;
        resp.into_json()
            .map_err(|e| format!("Parse folders: {e}"))
    }

    /// Share a folder with a device. Creates the folder config if it doesn't exist.
    /// `folder_type`: "sendreceive", "sendonly", "receiveonly"
    pub fn share_folder(
        &self,
        folder_id: &str,
        label: &str,
        path: &str,
        folder_type: &str,
        device_ids: &[String],
    ) -> Result<(), String> {
        let devices: Vec<FolderDevice> = device_ids
            .iter()
            .map(|id| FolderDevice {
                device_id: id.clone(),
            })
            .collect();

        // Check if folder already exists
        let existing = self.list_folders()?;
        if let Some(mut folder) = existing.into_iter().find(|f| f.id == folder_id) {
            // Update existing: merge devices
            for d in &devices {
                if !folder.devices.iter().any(|fd| fd.device_id == d.device_id) {
                    folder.devices.push(d.clone());
                }
            }
            folder.folder_type = folder_type.to_string();
            self.put_json(&format!("/rest/config/folders/{}", folder_id), &folder)?;
        } else {
            // Also include our own device ID
            let my_id = self.system_status()?.my_id;
            let mut all_devices = vec![FolderDevice {
                device_id: my_id,
            }];
            all_devices.extend(devices);

            let folder = FolderConfig {
                id: folder_id.to_string(),
                label: label.to_string(),
                path: path.to_string(),
                folder_type: folder_type.to_string(),
                devices: all_devices,
                paused: false,
            };
            self.post_json("/rest/config/folders", &folder)?;
        }
        Ok(())
    }

    /// Remove a shared folder.
    pub fn remove_folder(&self, folder_id: &str) -> Result<(), String> {
        self.delete(&format!("/rest/config/folders/{}", folder_id))?;
        Ok(())
    }

    /// Get completion percentage for a folder (optionally for a specific device).
    pub fn folder_completion(
        &self,
        folder_id: &str,
        device_id: Option<&str>,
    ) -> Result<Completion, String> {
        let url = match device_id {
            Some(did) => format!(
                "/rest/db/completion?folder={}&device={}",
                folder_id, did
            ),
            None => format!("/rest/db/completion?folder={}", folder_id),
        };
        let resp = self.get(&url)?;
        resp.into_json::<Completion>()
            .map_err(|e| format!("Parse completion: {e}"))
    }

    /// Get folder statuses including completion.
    pub fn folder_statuses(&self) -> Result<Vec<FolderStatus>, String> {
        let folders = self.list_folders()?;
        let mut results = Vec::new();
        for f in folders {
            let completion = self
                .folder_completion(&f.id, None)
                .map(|c| c.completion)
                .unwrap_or(0.0);
            results.push(FolderStatus {
                id: f.id,
                label: f.label,
                path: f.path,
                folder_type: f.folder_type,
                paused: f.paused,
                completion,
            });
        }
        Ok(results)
    }

    // ── Conflicts ───────────────────────────────────────────────────

    /// Scan a folder path for Syncthing conflict files (`.sync-conflict-*`).
    pub fn detect_conflicts(folder_path: &Path) -> Vec<ConflictFile> {
        let folder_str = folder_path.to_string_lossy().to_string();
        WalkDir::new(folder_path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_name()
                    .to_string_lossy()
                    .contains(".sync-conflict-")
            })
            .map(|e| ConflictFile {
                path: e.path().to_string_lossy().to_string(),
                game_folder: folder_str.clone(),
            })
            .collect()
    }

    /// Delete a conflict file.
    pub fn resolve_conflict(conflict_path: &Path) -> Result<(), String> {
        std::fs::remove_file(conflict_path)
            .map_err(|e| format!("Failed to delete conflict file: {e}"))
    }
}

/// Try to auto-detect the Syncthing API key from its config file.
/// Syncthing stores config at:
/// Windows: %LOCALAPPDATA%/Syncthing/config.xml
/// Linux:   ~/.local/state/syncthing/config.xml (or ~/.config/syncthing/config.xml)
pub fn detect_api_key() -> Option<String> {
    let config_path = syncthing_config_path()?;
    let content = std::fs::read_to_string(&config_path).ok()?;

    // Parse out <apikey>...</apikey> without pulling in an XML crate
    let start = content.find("<apikey>")?;
    let end = content.find("</apikey>")?;
    let key = &content[start + 8..end];
    if key.is_empty() {
        None
    } else {
        Some(key.to_string())
    }
}

/// Get the Syncthing config file path for the current OS.
fn syncthing_config_path() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        let local_app_data = std::env::var("LOCALAPPDATA").ok()?;
        Some(PathBuf::from(local_app_data).join("Syncthing").join("config.xml"))
    }
    #[cfg(target_os = "linux")]
    {
        // Try new path first (~/.local/state/syncthing), fall back to legacy
        let home = std::env::var("HOME").ok()?;
        let new_path = PathBuf::from(&home)
            .join(".local/state/syncthing/config.xml");
        if new_path.exists() {
            return Some(new_path);
        }
        let legacy = PathBuf::from(&home).join(".config/syncthing/config.xml");
        if legacy.exists() {
            return Some(legacy);
        }
        None
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        None
    }
}



use serde::Serialize;
use tauri::State;

use crate::sync::syncthing::{self, ConflictFile, SyncthingClient};
use crate::AppState;

// ── Serializable response types ─────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct SyncStatusResponse {
    pub available: bool,
    pub running: bool,
    pub my_device_id: String,
    pub uptime: u64,
    pub api_key: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeviceResponse {
    pub device_id: String,
    pub name: String,
    pub connected: bool,
    pub paused: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct FolderStatusResponse {
    pub id: String,
    pub label: String,
    pub path: String,
    pub folder_type: String,
    pub paused: bool,
    pub completion: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ShareResult {
    pub folder_id: String,
    pub success: bool,
}

// ── Helper ──────────────────────────────────────────────────────────

/// Build a SyncthingClient from the API key stored in settings, or auto-detect.
fn get_client(state: &AppState) -> Result<SyncthingClient, String> {
    let conn = state.db.lock().map_err(|e| format!("DB lock: {e}"))?;

    // Try settings first
    let stored_key: Option<String> = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'syncthing_api_key'",
            [],
            |row| row.get(0),
        )
        .ok()
        .filter(|v: &String| !v.is_empty());

    let api_key = match stored_key {
        Some(k) => k,
        None => syncthing::detect_api_key()
            .ok_or_else(|| "Syncthing API key not found. Is Syncthing installed?".to_string())?,
    };

    let base_url: Option<String> = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'syncthing_url'",
            [],
            |row| row.get(0),
        )
        .ok()
        .filter(|v: &String| !v.is_empty());

    match base_url {
        Some(url) => Ok(SyncthingClient::with_base_url(&api_key, &url)),
        None => Ok(SyncthingClient::new(&api_key)),
    }
}

// ── Commands ────────────────────────────────────────────────────────

/// Check Syncthing status: is it installed, running, what's the device ID?
#[tauri::command]
pub fn sync_status(state: State<'_, AppState>) -> Result<SyncStatusResponse, String> {
    let api_key = {
        let conn = state.db.lock().map_err(|e| format!("DB lock: {e}"))?;
        conn.query_row(
            "SELECT value FROM settings WHERE key = 'syncthing_api_key'",
            [],
            |row| row.get::<_, String>(0),
        )
        .ok()
        .filter(|v| !v.is_empty())
        .or_else(syncthing::detect_api_key)
    };

    let Some(key) = api_key else {
        return Ok(SyncStatusResponse {
            available: false,
            running: false,
            my_device_id: String::new(),
            uptime: 0,
            api_key: String::new(),
        });
    };

    let client = SyncthingClient::new(&key);
    match client.sync_status() {
        Ok(status) => Ok(SyncStatusResponse {
            available: true,
            running: status.running,
            my_device_id: status.my_device_id,
            uptime: status.uptime,
            api_key: key,
        }),
        Err(_) => Ok(SyncStatusResponse {
            available: true, // key found, but Syncthing not responding
            running: false,
            my_device_id: String::new(),
            uptime: 0,
            api_key: key,
        }),
    }
}

/// List paired devices with connection status.
#[tauri::command]
pub fn sync_list_devices(state: State<'_, AppState>) -> Result<Vec<DeviceResponse>, String> {
    let client = get_client(&state)?;
    let devices = client.list_devices_with_status()?;
    Ok(devices
        .into_iter()
        .map(|d| DeviceResponse {
            device_id: d.device_id,
            name: d.name,
            connected: d.connected,
            paused: d.paused,
        })
        .collect())
}

/// Add a device for pairing.
#[tauri::command]
pub fn sync_add_device(
    state: State<'_, AppState>,
    device_id: String,
    name: String,
) -> Result<(), String> {
    // Validate device ID format (basic check: should be like XXXXXXX-XXXXXXX-XXXXXXX-XXXXXXX-XXXXXXX-XXXXXXX-XXXXXXX-XXXXXXX)
    if device_id.len() < 50 || !device_id.contains('-') {
        return Err("Invalid Syncthing device ID format".to_string());
    }

    let client = get_client(&state)?;
    client.add_device(&device_id, &name)?;

    // Also record in our DB
    let conn = state.db.lock().map_err(|e| format!("DB lock: {e}"))?;
    conn.execute(
        "INSERT OR REPLACE INTO sync_devices (name, syncthing_device_id, last_seen) VALUES (?1, ?2, datetime('now'))",
        rusqlite::params![name, device_id],
    )
    .map_err(|e| format!("DB insert: {e}"))?;

    Ok(())
}

/// Remove a paired device.
#[tauri::command]
pub fn sync_remove_device(
    state: State<'_, AppState>,
    device_id: String,
) -> Result<(), String> {
    let client = get_client(&state)?;
    client.remove_device(&device_id)?;

    // Remove from our DB
    let conn = state.db.lock().map_err(|e| format!("DB lock: {e}"))?;
    conn.execute(
        "DELETE FROM sync_devices WHERE syncthing_device_id = ?1",
        rusqlite::params![device_id],
    )
    .map_err(|e| format!("DB delete: {e}"))?;

    Ok(())
}

/// Share the backup folder with paired devices.
/// `sync_mode`: "sendreceive", "sendonly", "receiveonly"
#[tauri::command]
pub fn sync_share_folder(
    state: State<'_, AppState>,
    sync_mode: String,
) -> Result<ShareResult, String> {
    let valid_modes = ["sendreceive", "sendonly", "receiveonly"];
    if !valid_modes.contains(&sync_mode.as_str()) {
        return Err(format!("Invalid sync mode: {sync_mode}. Use: sendreceive, sendonly, receiveonly"));
    }

    let client = get_client(&state)?;

    // Get backup directory
    let conn = state.db.lock().map_err(|e| format!("DB lock: {e}"))?;
    let backup_dir = crate::commands::backup::backup_root(&conn, &state.app_data_dir)?;
    let backup_path = backup_dir.to_string_lossy().to_string();
    drop(conn);

    // Get all paired device IDs
    let devices = client.list_devices()?;
    let device_ids: Vec<String> = devices.iter().map(|d| d.device_id.clone()).collect();

    if device_ids.is_empty() {
        return Err("No paired devices. Add a device first.".to_string());
    }

    let folder_id = "decksave-backups";
    client.share_folder(
        folder_id,
        "DeckSave Backups",
        &backup_path,
        &sync_mode,
        &device_ids,
    )?;

    Ok(ShareResult {
        folder_id: folder_id.to_string(),
        success: true,
    })
}

/// Remove a shared folder from Syncthing.
#[tauri::command]
pub fn sync_remove_folder(
    state: State<'_, AppState>,
    folder_id: String,
) -> Result<(), String> {
    let client = get_client(&state)?;
    client.remove_folder(&folder_id)
}

/// Get sync status for all shared folders.
#[tauri::command]
pub fn sync_folder_status(state: State<'_, AppState>) -> Result<Vec<FolderStatusResponse>, String> {
    let client = get_client(&state)?;
    let folders = client.folder_statuses()?;
    Ok(folders
        .into_iter()
        .map(|f| FolderStatusResponse {
            id: f.id,
            label: f.label,
            path: f.path,
            folder_type: f.folder_type,
            paused: f.paused,
            completion: f.completion,
        })
        .collect())
}

/// Detect conflict files in the backup directory.
#[tauri::command]
pub fn sync_detect_conflicts(state: State<'_, AppState>) -> Result<Vec<ConflictFile>, String> {
    let conn = state.db.lock().map_err(|e| format!("DB lock: {e}"))?;
    let backup_dir = crate::commands::backup::backup_root(&conn, &state.app_data_dir)?;
    drop(conn);
    Ok(SyncthingClient::detect_conflicts(&backup_dir))
}

/// Resolve (delete) a conflict file.
#[tauri::command]
pub fn sync_resolve_conflict(path: String) -> Result<(), String> {
    let p = std::path::Path::new(&path);

    // Safety: only allow deleting files that contain .sync-conflict- in their name
    let file_name = p
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    if !file_name.contains(".sync-conflict-") {
        return Err("Not a Syncthing conflict file".to_string());
    }

    SyncthingClient::resolve_conflict(p)
}

/// Save Syncthing connection settings.
#[tauri::command]
pub fn sync_update_settings(
    state: State<'_, AppState>,
    api_key: Option<String>,
    base_url: Option<String>,
) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| format!("DB lock: {e}"))?;

    if let Some(key) = api_key {
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('syncthing_api_key', ?1)",
            rusqlite::params![key],
        )
        .map_err(|e| format!("DB: {e}"))?;
    }

    if let Some(url) = base_url {
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('syncthing_url', ?1)",
            rusqlite::params![url],
        )
        .map_err(|e| format!("DB: {e}"))?;
    }

    Ok(())
}

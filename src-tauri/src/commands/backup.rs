use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct BackupRecord {
    pub id: i64,
    pub game_id: i64,
    pub timestamp: String,
    pub file_path: String,
    pub size_bytes: i64,
    pub checksum: String,
}

#[tauri::command]
pub fn backup_game(game_id: i64) -> Result<BackupRecord, String> {
    // Phase 2: Will copy save files, compress to zip, record in SQLite
    Err(format!("Backup not yet implemented for game {}", game_id))
}

#[tauri::command]
pub fn backup_all() -> Result<Vec<BackupRecord>, String> {
    // Phase 2: Will iterate all detected games and back up each
    Err("Backup-all not yet implemented".to_string())
}

#[tauri::command]
pub fn restore_game(game_id: i64, backup_id: Option<i64>) -> Result<(), String> {
    // Phase 2: Will extract backup zip to original save paths
    Err(format!(
        "Restore not yet implemented for game {} (backup {:?})",
        game_id, backup_id
    ))
}

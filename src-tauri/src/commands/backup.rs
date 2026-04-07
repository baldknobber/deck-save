use chrono::Utc;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use zip::write::SimpleFileOptions;

#[derive(Debug, Serialize, Clone)]
pub struct BackupRecord {
    pub id: i64,
    pub game_id: i64,
    pub timestamp: String,
    pub file_path: String,
    pub size_bytes: i64,
    pub checksum: String,
}

/// Resolve the backup root directory. Uses the `backup_dir` setting if set,
/// otherwise falls back to `<app_data_dir>/backups`.
pub(crate) fn backup_root(conn: &rusqlite::Connection, app_data_dir: &Path) -> Result<PathBuf, String> {
    let custom: Option<String> = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'backup_dir'",
            [],
            |row| row.get(0),
        )
        .ok();

    let root = match custom {
        Some(ref v) if !v.is_empty() => PathBuf::from(v),
        _ => app_data_dir.join("backups"),
    };
    fs::create_dir_all(&root).map_err(|e| format!("Cannot create backup dir: {e}"))?;
    Ok(root)
}

/// Read the max_versions setting (default 5).
fn max_versions(conn: &rusqlite::Connection) -> i64 {
    conn.query_row(
        "SELECT value FROM settings WHERE key = 'max_versions'",
        [],
        |row| {
            let v: String = row.get(0)?;
            Ok(v.parse::<i64>().unwrap_or(5))
        },
    )
    .unwrap_or(5)
}

/// Fetch a game row from the DB.
pub(crate) fn get_game(conn: &rusqlite::Connection, game_id: i64) -> Result<(String, Vec<String>), String> {
    conn.query_row(
        "SELECT title, save_paths FROM games WHERE id = ?1",
        rusqlite::params![game_id],
        |row| {
            let title: String = row.get(0)?;
            let paths_json: String = row.get(1)?;
            Ok((title, paths_json))
        },
    )
    .map(|(title, paths_json)| {
        let paths: Vec<String> = serde_json::from_str(&paths_json).unwrap_or_default();
        (title, paths)
    })
    .map_err(|e| format!("Game not found (id={game_id}): {e}"))
}

/// Walk every save path for a game and zip them up, returning (zip_path, size, sha256).
pub(crate) fn create_backup_zip(
    game_id: i64,
    title: &str,
    save_paths: &[String],
    backup_root: &Path,
) -> Result<(PathBuf, u64, String), String> {
    let ts = Utc::now().format("%Y%m%d_%H%M%S").to_string();
    // Sanitise title for filesystem
    let safe_title: String = title
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect();
    let game_dir = backup_root.join(format!("{game_id}_{safe_title}"));
    fs::create_dir_all(&game_dir).map_err(|e| format!("Cannot create game backup dir: {e}"))?;

    let zip_name = format!("{safe_title}_{ts}.zip");
    let zip_path = game_dir.join(&zip_name);
    let file =
        fs::File::create(&zip_path).map_err(|e| format!("Cannot create zip file: {e}"))?;
    let mut zip_writer = zip::ZipWriter::new(file);
    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    let mut files_added: usize = 0;

    for save_path_str in save_paths {
        let save_path = PathBuf::from(save_path_str);
        if !save_path.exists() {
            eprintln!("[DeckSave] Save path not found, skipping: {save_path_str}");
            continue;
        }

        if save_path.is_file() {
            let entry_name = save_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            zip_writer
                .start_file(&entry_name, options)
                .map_err(|e| format!("Zip start_file error: {e}"))?;
            let mut f =
                fs::File::open(&save_path).map_err(|e| format!("Cannot open save file: {e}"))?;
            std::io::copy(&mut f, &mut zip_writer)
                .map_err(|e| format!("Zip copy error: {e}"))?;
            files_added += 1;
        } else if save_path.is_dir() {
            for entry in WalkDir::new(&save_path)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
            {
                let rel = entry
                    .path()
                    .strip_prefix(&save_path)
                    .unwrap_or(entry.path());
                let entry_name = rel.to_string_lossy().replace('\\', "/");
                zip_writer
                    .start_file(&entry_name, options)
                    .map_err(|e| format!("Zip start_file error: {e}"))?;
                let mut f = fs::File::open(entry.path())
                    .map_err(|e| format!("Cannot open {}: {e}", entry.path().display()))?;
                std::io::copy(&mut f, &mut zip_writer)
                    .map_err(|e| format!("Zip copy error: {e}"))?;
                files_added += 1;
            }
        }
    }

    if files_added == 0 {
        // Clean up empty zip
        drop(zip_writer);
        let _ = fs::remove_file(&zip_path);
        return Err(format!("No save files found for \"{title}\""));
    }

    zip_writer
        .finish()
        .map_err(|e| format!("Zip finish error: {e}"))?;

    // Compute SHA-256
    let mut hasher = Sha256::new();
    let mut zip_file =
        fs::File::open(&zip_path).map_err(|e| format!("Cannot reopen zip: {e}"))?;
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = zip_file
            .read(&mut buf)
            .map_err(|e| format!("Hash read error: {e}"))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    let hash = format!("{:x}", hasher.finalize());

    let size = fs::metadata(&zip_path)
        .map(|m| m.len())
        .unwrap_or(0);

    Ok((zip_path, size, hash))
}

/// Insert backup record, enforce retention, update game status.
pub(crate) fn record_backup(
    conn: &rusqlite::Connection,
    game_id: i64,
    zip_path: &Path,
    size: u64,
    checksum: &str,
) -> Result<BackupRecord, String> {
    let next_version: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) + 1 FROM backups WHERE game_id = ?1",
            rusqlite::params![game_id],
            |row| row.get(0),
        )
        .unwrap_or(1);

    conn.execute(
        "INSERT INTO backups (game_id, file_path, size_bytes, checksum, version) \
         VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![
            game_id,
            zip_path.to_string_lossy().as_ref(),
            size as i64,
            checksum,
            next_version
        ],
    )
    .map_err(|e| format!("DB insert backup error: {e}"))?;

    let id = conn.last_insert_rowid();

    // Update game status
    conn.execute(
        "UPDATE games SET status = 'backed_up', updated_at = datetime('now') WHERE id = ?1",
        rusqlite::params![game_id],
    )
    .map_err(|e| format!("DB update game status error: {e}"))?;

    // Enforce retention — delete oldest backups beyond max_versions
    let max = max_versions(conn);
    let old_backups: Vec<(i64, String)> = {
        let mut stmt = conn
            .prepare(
                "SELECT id, file_path FROM backups WHERE game_id = ?1 \
                 ORDER BY version DESC LIMIT -1 OFFSET ?2",
            )
            .map_err(|e| format!("DB retention query error: {e}"))?;
        let rows = stmt
            .query_map(rusqlite::params![game_id, max], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .map_err(|e| format!("DB retention map error: {e}"))?
            .filter_map(|r| r.ok())
            .collect();
        rows
    };

    for (old_id, old_path) in old_backups {
        let _ = fs::remove_file(&old_path);
        conn.execute("DELETE FROM backups WHERE id = ?1", rusqlite::params![old_id])
            .ok();
        eprintln!("[DeckSave] Pruned old backup id={old_id}: {old_path}");
    }

    let record = conn
        .query_row(
            "SELECT id, game_id, timestamp, file_path, size_bytes, checksum FROM backups WHERE id = ?1",
            rusqlite::params![id],
            |row| {
                Ok(BackupRecord {
                    id: row.get(0)?,
                    game_id: row.get(1)?,
                    timestamp: row.get(2)?,
                    file_path: row.get(3)?,
                    size_bytes: row.get(4)?,
                    checksum: row.get(5)?,
                })
            },
        )
        .map_err(|e| format!("DB read-back error: {e}"))?;

    Ok(record)
}

// ─── Tauri Commands ──────────────────────────────────────────────────────────

#[tauri::command]
pub fn backup_game(
    game_id: i64,
    state: tauri::State<'_, crate::AppState>,
) -> Result<BackupRecord, String> {
    let conn = state
        .db
        .lock()
        .map_err(|e| format!("DB lock error: {e}"))?;
    let root = backup_root(&conn, &state.app_data_dir)?;
    let (title, save_paths) = get_game(&conn, game_id)?;

    if save_paths.is_empty() {
        return Err(format!("No save paths configured for \"{title}\""));
    }

    let (zip_path, size, checksum) = create_backup_zip(game_id, &title, &save_paths, &root)?;
    eprintln!(
        "[DeckSave] Backed up \"{title}\" → {} ({} bytes)",
        zip_path.display(),
        size
    );

    record_backup(&conn, game_id, &zip_path, size, &checksum)
}

#[tauri::command]
pub fn backup_all(
    state: tauri::State<'_, crate::AppState>,
) -> Result<Vec<BackupRecord>, String> {
    let conn = state
        .db
        .lock()
        .map_err(|e| format!("DB lock error: {e}"))?;
    let root = backup_root(&conn, &state.app_data_dir)?;

    // Find all games that have at least one save path
    let mut stmt = conn
        .prepare("SELECT id, title, save_paths FROM games ORDER BY title COLLATE NOCASE")
        .map_err(|e| format!("DB query error: {e}"))?;

    let games: Vec<(i64, String, Vec<String>)> = stmt
        .query_map([], |row| {
            let id: i64 = row.get(0)?;
            let title: String = row.get(1)?;
            let json: String = row.get(2)?;
            Ok((id, title, json))
        })
        .map_err(|e| format!("DB query_map error: {e}"))?
        .filter_map(|r| r.ok())
        .map(|(id, title, json)| {
            let paths: Vec<String> = serde_json::from_str(&json).unwrap_or_default();
            (id, title, paths)
        })
        .filter(|(_, _, paths)| !paths.is_empty())
        .collect();

    let mut results = Vec::new();
    let mut errors = Vec::new();

    for (game_id, title, save_paths) in &games {
        match create_backup_zip(*game_id, title, save_paths, &root) {
            Ok((zip_path, size, checksum)) => {
                match record_backup(&conn, *game_id, &zip_path, size, &checksum) {
                    Ok(record) => {
                        eprintln!(
                            "[DeckSave] Backed up \"{title}\" → {} ({size} bytes)",
                            zip_path.display()
                        );
                        results.push(record);
                    }
                    Err(e) => errors.push(format!("{title}: {e}")),
                }
            }
            Err(e) => errors.push(format!("{title}: {e}")),
        }
    }

    if results.is_empty() && !errors.is_empty() {
        return Err(format!("All backups failed:\n{}", errors.join("\n")));
    }
    if !errors.is_empty() {
        eprintln!("[DeckSave] Some backups failed:\n{}", errors.join("\n"));
    }

    Ok(results)
}

#[tauri::command]
pub fn restore_game(
    game_id: i64,
    backup_id: Option<i64>,
    state: tauri::State<'_, crate::AppState>,
) -> Result<(), String> {
    let conn = state
        .db
        .lock()
        .map_err(|e| format!("DB lock error: {e}"))?;
    let (title, save_paths) = get_game(&conn, game_id)?;

    if save_paths.is_empty() {
        return Err(format!("No save paths configured for \"{title}\""));
    }

    // Determine which backup to restore
    let (b_id, b_path, b_checksum): (i64, String, String) = if let Some(bid) = backup_id {
        conn.query_row(
            "SELECT id, file_path, checksum FROM backups WHERE id = ?1 AND game_id = ?2",
            rusqlite::params![bid, game_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(|e| format!("Backup not found (id={bid}): {e}"))?
    } else {
        // Latest backup
        conn.query_row(
            "SELECT id, file_path, checksum FROM backups WHERE game_id = ?1 ORDER BY version DESC LIMIT 1",
            rusqlite::params![game_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(|_| format!("No backups found for \"{title}\""))?
    };

    let zip_path = PathBuf::from(&b_path);
    if !zip_path.exists() {
        return Err(format!("Backup file missing: {b_path}"));
    }

    // Verify checksum
    {
        let mut hasher = Sha256::new();
        let mut f =
            fs::File::open(&zip_path).map_err(|e| format!("Cannot open backup zip: {e}"))?;
        let mut buf = vec![0u8; 64 * 1024];
        loop {
            let n = f
                .read(&mut buf)
                .map_err(|e| format!("Hash read error: {e}"))?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
        }
        let hash = format!("{:x}", hasher.finalize());
        if hash != b_checksum {
            return Err(format!(
                "Checksum mismatch for backup {b_id}: expected {b_checksum}, got {hash}"
            ));
        }
    }

    // Safety backup: back up current saves before overwriting
    let root = backup_root(&conn, &state.app_data_dir)?;
    match create_backup_zip(game_id, &title, &save_paths, &root) {
        Ok((pre_zip, pre_size, pre_hash)) => {
            record_backup(&conn, game_id, &pre_zip, pre_size, &pre_hash).ok();
            eprintln!(
                "[DeckSave] Pre-restore safety backup created: {}",
                pre_zip.display()
            );
        }
        Err(e) => {
            eprintln!("[DeckSave] Pre-restore safety backup skipped (no current saves): {e}");
        }
    }

    // Extract the zip into the first save path directory.
    // The first save_path is treated as the primary restore target.
    let target_dir = PathBuf::from(&save_paths[0]);
    let target_dir = if target_dir.extension().is_some() {
        // If the save_path looks like a file, use its parent dir
        target_dir
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or(target_dir)
    } else {
        target_dir
    };
    fs::create_dir_all(&target_dir)
        .map_err(|e| format!("Cannot create restore dir: {e}"))?;

    let zip_file =
        fs::File::open(&zip_path).map_err(|e| format!("Cannot open backup zip: {e}"))?;
    let mut archive =
        zip::ZipArchive::new(zip_file).map_err(|e| format!("Invalid zip archive: {e}"))?;

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| format!("Zip read error: {e}"))?;

        let Some(name) = entry.enclosed_name() else {
            continue; // skip suspicious paths
        };
        let out_path = target_dir.join(name);

        if entry.is_dir() {
            fs::create_dir_all(&out_path)
                .map_err(|e| format!("Cannot create dir {}: {e}", out_path.display()))?;
        } else {
            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("Cannot create parent dir: {e}"))?;
            }
            let mut outfile = fs::File::create(&out_path)
                .map_err(|e| format!("Cannot create {}: {e}", out_path.display()))?;
            std::io::copy(&mut entry, &mut outfile)
                .map_err(|e| format!("Restore write error: {e}"))?;
        }
    }

    eprintln!(
        "[DeckSave] Restored \"{title}\" from backup {b_id} → {}",
        target_dir.display()
    );

    Ok(())
}

#[tauri::command]
pub fn get_backups(
    game_id: i64,
    state: tauri::State<'_, crate::AppState>,
) -> Result<Vec<BackupRecord>, String> {
    let conn = state
        .db
        .lock()
        .map_err(|e| format!("DB lock error: {e}"))?;

    let mut stmt = conn
        .prepare(
            "SELECT id, game_id, timestamp, file_path, size_bytes, checksum \
             FROM backups WHERE game_id = ?1 ORDER BY version DESC",
        )
        .map_err(|e| format!("DB query error: {e}"))?;

    let records = stmt
        .query_map(rusqlite::params![game_id], |row| {
            Ok(BackupRecord {
                id: row.get(0)?,
                game_id: row.get(1)?,
                timestamp: row.get(2)?,
                file_path: row.get(3)?,
                size_bytes: row.get(4)?,
                checksum: row.get(5)?,
            })
        })
        .map_err(|e| format!("DB query_map error: {e}"))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(records)
}

// ─── Settings Commands ───────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct Setting {
    pub key: String,
    pub value: String,
}

#[tauri::command]
pub fn get_settings(
    state: tauri::State<'_, crate::AppState>,
) -> Result<Vec<Setting>, String> {
    let conn = state
        .db
        .lock()
        .map_err(|e| format!("DB lock error: {e}"))?;

    let mut stmt = conn
        .prepare("SELECT key, value FROM settings ORDER BY key")
        .map_err(|e| format!("DB query error: {e}"))?;

    let settings = stmt
        .query_map([], |row| {
            Ok(Setting {
                key: row.get(0)?,
                value: row.get(1)?,
            })
        })
        .map_err(|e| format!("DB query_map error: {e}"))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(settings)
}

#[tauri::command]
pub fn update_setting(
    key: String,
    value: String,
    state: tauri::State<'_, crate::AppState>,
) -> Result<(), String> {
    let conn = state
        .db
        .lock()
        .map_err(|e| format!("DB lock error: {e}"))?;

    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2) \
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        rusqlite::params![key, value],
    )
    .map_err(|e| format!("DB update setting error: {e}"))?;

    Ok(())
}

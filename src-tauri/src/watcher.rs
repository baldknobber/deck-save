//! File watcher + auto-backup scheduler.
//!
//! Watches known save directories for changes (via `notify` crate with debouncing),
//! marks games as "changed" in SQLite, emits Tauri events to the frontend,
//! and optionally triggers auto-backup based on user settings.

use notify_debouncer_mini::{new_debouncer, DebouncedEventKind};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

use crate::commands::backup;
use crate::AppState;

/// Payload sent to the frontend when a save file changes.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SaveChangedEvent {
    pub game_id: i64,
    pub game_title: String,
}

/// Payload sent to the frontend after auto-backup completes.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AutoBackupEvent {
    pub backed_up: usize,
    pub failed: usize,
    pub game_titles: Vec<String>,
}

/// Build a map from watched directory → game_id by reading all games from the DB.
fn build_watch_map(state: &AppState) -> HashMap<PathBuf, (i64, String)> {
    let mut map = HashMap::new();
    let conn = match state.db.lock() {
        Ok(c) => c,
        Err(_) => return map,
    };

    let mut stmt = match conn.prepare("SELECT id, title, save_paths FROM games") {
        Ok(s) => s,
        Err(_) => return map,
    };

    let rows: Vec<(i64, String, String)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .unwrap_or_else(|_| panic!("query_map failed"))
        .filter_map(|r| r.ok())
        .collect();

    for (id, title, json) in rows {
        let paths: Vec<String> = serde_json::from_str(&json).unwrap_or_default();
        for p in paths {
            let path = PathBuf::from(&p);
            // Watch the directory containing saves (parent if it's a file-like path)
            let watch_dir = if path.extension().is_some() {
                path.parent().map(|p| p.to_path_buf()).unwrap_or(path)
            } else {
                path
            };
            if watch_dir.exists() {
                map.insert(watch_dir, (id, title.clone()));
            }
        }
    }

    map
}

/// Given a changed path, find which game_id it belongs to.
fn match_path_to_game(path: &PathBuf, watch_map: &HashMap<PathBuf, (i64, String)>) -> Option<(i64, String)> {
    // Walk up the path hierarchy to find a matching watched directory
    let mut candidate = path.clone();
    loop {
        if let Some((id, title)) = watch_map.get(&candidate) {
            return Some((*id, title.clone()));
        }
        if !candidate.pop() {
            break;
        }
    }
    None
}

/// Mark a game as "changed" in the database.
fn mark_game_changed(state: &AppState, game_id: i64) {
    if let Ok(conn) = state.db.lock() {
        conn.execute(
            "UPDATE games SET status = 'changed', updated_at = datetime('now') WHERE id = ?1 AND status != 'changed'",
            rusqlite::params![game_id],
        )
        .ok();
    }
}

/// Run auto-backup for all games marked as "changed", if auto_backup is enabled.
/// Returns (backed_up_count, failed_count, game_titles).
fn run_auto_backup(state: &AppState) -> (usize, usize, Vec<String>) {
    let conn = match state.db.lock() {
        Ok(c) => c,
        Err(_) => return (0, 0, vec![]),
    };

    // Check if auto-backup is enabled
    let auto_enabled: bool = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'auto_backup'",
            [],
            |row| {
                let v: String = row.get(0)?;
                Ok(v == "true")
            },
        )
        .unwrap_or(false);

    if !auto_enabled {
        return (0, 0, vec![]);
    }

    let root = match backup::backup_root(&conn, &state.app_data_dir) {
        Ok(r) => r,
        Err(_) => return (0, 0, vec![]),
    };

    // Find all "changed" games
    let mut stmt = match conn.prepare(
        "SELECT id, title, save_paths FROM games WHERE status = 'changed'",
    ) {
        Ok(s) => s,
        Err(_) => return (0, 0, vec![]),
    };

    let changed_games: Vec<(i64, String, Vec<String>)> = stmt
        .query_map([], |row| {
            let id: i64 = row.get(0)?;
            let title: String = row.get(1)?;
            let json: String = row.get(2)?;
            Ok((id, title, json))
        })
        .unwrap_or_else(|_| panic!("query_map failed"))
        .filter_map(|r| r.ok())
        .map(|(id, title, json)| {
            let paths: Vec<String> = serde_json::from_str(&json).unwrap_or_default();
            (id, title, paths)
        })
        .filter(|(_, _, paths)| !paths.is_empty())
        .collect();

    drop(stmt);

    let mut backed_up = 0;
    let mut failed = 0;
    let mut titles = Vec::new();

    for (game_id, title, save_paths) in &changed_games {
        match backup::create_backup_zip(*game_id, title, save_paths, &root) {
            Ok((zip_path, size, checksum)) => {
                match backup::record_backup(&conn, *game_id, &zip_path, size, &checksum) {
                    Ok(_) => {
                        eprintln!("[DeckSave] Auto-backup: \"{title}\" OK");
                        backed_up += 1;
                        titles.push(title.clone());
                    }
                    Err(e) => {
                        eprintln!("[DeckSave] Auto-backup record error for \"{title}\": {e}");
                        failed += 1;
                    }
                }
            }
            Err(e) => {
                eprintln!("[DeckSave] Auto-backup zip error for \"{title}\": {e}");
                failed += 1;
            }
        }
    }

    (backed_up, failed, titles)
}

/// Read the backup_interval setting and return the sleep duration.
/// Returns None for "on_change" mode (watcher handles it directly).
fn get_scheduler_interval(state: &AppState) -> Option<Duration> {
    let conn = state.db.lock().ok()?;
    let interval: String = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'backup_interval'",
            [],
            |row| row.get(0),
        )
        .unwrap_or_else(|_| "hourly".to_string());

    match interval.as_str() {
        "on_change" => None, // Watcher triggers backup directly
        "hourly" => Some(Duration::from_secs(3600)),
        "daily" => Some(Duration::from_secs(86400)),
        _ => Some(Duration::from_secs(3600)), // Default to hourly
    }
}

/// Start the file watcher and auto-backup scheduler.
/// This should be called from `setup()` in a background thread.
pub fn start(app_handle: AppHandle) {
    let state = app_handle.state::<AppState>();
    let watch_map = Arc::new(std::sync::Mutex::new(build_watch_map(&state)));

    // ── File Watcher ─────────────────────────────────────────────────────
    let watcher_app = app_handle.clone();
    let watcher_map = Arc::clone(&watch_map);

    let mut debouncer = match new_debouncer(Duration::from_secs(5), move |result: Result<Vec<notify_debouncer_mini::DebouncedEvent>, notify::Error>| {
        let events = match result {
            Ok(events) => events,
            Err(e) => {
                eprintln!("[DeckSave] Watch error: {e}");
                return;
            }
        };

        let state = watcher_app.state::<AppState>();
        let map = watcher_map.lock().unwrap_or_else(|e| e.into_inner());
        let mut notified_games = std::collections::HashSet::new();

        for event in &events {
            if event.kind != DebouncedEventKind::Any {
                continue;
            }

            if let Some((game_id, title)) = match_path_to_game(&event.path, &map) {
                if notified_games.insert(game_id) {
                    mark_game_changed(&state, game_id);
                    eprintln!("[DeckSave] Save changed: \"{title}\" (game_id={game_id})");

                    let _ = watcher_app.emit("save-changed", SaveChangedEvent {
                        game_id,
                        game_title: title,
                    });
                }
            }
        }

        // In "on_change" mode, trigger backup immediately after detecting changes
        if !notified_games.is_empty() {
            if let Some(interval) = get_scheduler_interval(&state) {
                // Not on_change mode — scheduler will handle it
                let _ = interval;
            } else {
                // on_change mode: backup now
                let (backed_up, failed, titles) = run_auto_backup(&state);
                if backed_up > 0 || failed > 0 {
                    let _ = watcher_app.emit("auto-backup-complete", AutoBackupEvent {
                        backed_up,
                        failed,
                        game_titles: titles,
                    });
                }
            }
        }
    }) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("[DeckSave] Failed to create file watcher: {e}");
            return;
        }
    };

    // Watch all known save directories
    {
        let map = watch_map.lock().unwrap_or_else(|e| e.into_inner());
        let mut watch_count = 0;
        for dir in map.keys() {
            if let Err(e) = debouncer.watcher().watch(dir, notify::RecursiveMode::Recursive) {
                eprintln!("[DeckSave] Failed to watch {}: {e}", dir.display());
            } else {
                watch_count += 1;
            }
        }
        eprintln!("[DeckSave] Watching {watch_count} save directories");
    }

    // ── Scheduled Auto-Backup ────────────────────────────────────────────
    let scheduler_app = app_handle.clone();

    std::thread::spawn(move || {
        // Keep the debouncer alive for the lifetime of the app
        let _debouncer = debouncer;

        loop {
            let state = scheduler_app.state::<AppState>();
            let interval = get_scheduler_interval(&state);

            match interval {
                Some(dur) => {
                    std::thread::sleep(dur);

                    let state = scheduler_app.state::<AppState>();
                    let (backed_up, failed, titles) = run_auto_backup(&state);
                    if backed_up > 0 || failed > 0 {
                        eprintln!(
                            "[DeckSave] Scheduled auto-backup: {backed_up} OK, {failed} failed"
                        );
                        let _ = scheduler_app.emit(
                            "auto-backup-complete",
                            AutoBackupEvent {
                                backed_up,
                                failed,
                                game_titles: titles,
                            },
                        );
                    }
                }
                None => {
                    // on_change mode — watcher handles backups directly.
                    // Just sleep a bit and re-check settings in case user changes mode.
                    std::thread::sleep(Duration::from_secs(60));
                }
            }
        }
    });
}

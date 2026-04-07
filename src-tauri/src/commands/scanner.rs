use serde::Serialize;

use crate::steam;

#[derive(Debug, Serialize, Clone)]
pub struct Game {
    pub id: i64,
    pub title: String,
    pub steam_id: Option<String>,
    pub save_paths: Vec<String>,
    pub save_path_count: usize,
    pub last_backup: Option<String>,
    pub status: String,
}

#[tauri::command]
pub fn scan_games(state: tauri::State<'_, crate::AppState>) -> Result<Vec<Game>, String> {
    let conn = state.db.lock().map_err(|e| format!("DB lock error: {e}"))?;

    // Locate Steam installation
    let steam_root = steam::find_steam_root()
        .ok_or_else(|| "Steam installation not found. Make sure Steam is installed.".to_string())?;

    let libs = steam::library_folders(&steam_root);
    let installed = steam::installed_games(&libs);

    // Upsert each discovered game into the database
    for game in &installed {
        let save_paths = steam::find_save_paths(game, &steam_root);
        let paths_json = serde_json::to_string(
            &save_paths
                .iter()
                .map(|p| p.to_string_lossy().into_owned())
                .collect::<Vec<_>>(),
        )
        .unwrap_or_else(|_| "[]".to_string());

        let existing_id: Option<i64> = conn
            .query_row(
                "SELECT id FROM games WHERE steam_id = ?1",
                rusqlite::params![game.app_id],
                |row| row.get(0),
            )
            .ok();

        if let Some(id) = existing_id {
            conn.execute(
                "UPDATE games SET title = ?1, install_dir = ?2, save_paths = ?3, \
                 updated_at = datetime('now') WHERE id = ?4",
                rusqlite::params![game.name, game.install_dir, paths_json, id],
            )
            .map_err(|e| format!("DB update error: {e}"))?;
        } else {
            conn.execute(
                "INSERT INTO games (title, steam_id, install_dir, save_paths) \
                 VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![game.name, game.app_id, game.install_dir, paths_json],
            )
            .map_err(|e| format!("DB insert error: {e}"))?;
        }
    }

    // Return all games with their latest backup timestamp
    let mut stmt = conn
        .prepare(
            "SELECT g.id, g.title, g.steam_id, g.save_paths, g.status, \
             (SELECT MAX(b.timestamp) FROM backups b WHERE b.game_id = g.id) \
             FROM games g ORDER BY g.title COLLATE NOCASE",
        )
        .map_err(|e| format!("DB query error: {e}"))?;

    let games = stmt
        .query_map([], |row| {
            let paths_json: String = row.get(3)?;
            let save_paths: Vec<String> =
                serde_json::from_str(&paths_json).unwrap_or_default();
            let save_path_count = save_paths.len();
            Ok(Game {
                id: row.get(0)?,
                title: row.get(1)?,
                steam_id: row.get(2)?,
                save_paths,
                save_path_count,
                last_backup: row.get(5)?,
                status: row.get(4)?,
            })
        })
        .map_err(|e| format!("DB query error: {e}"))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(games)
}

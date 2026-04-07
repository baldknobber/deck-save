import { invoke } from "@tauri-apps/api/core";

export interface Game {
  id: number;
  title: string;
  steam_id: string | null;
  save_paths: string[];
  save_path_count: number;
  last_backup: string | null;
  status: string;
}

export interface BackupRecord {
  id: number;
  game_id: number;
  timestamp: string;
  file_path: string;
  size_bytes: number;
  checksum: string;
}

export async function scanGames(): Promise<Game[]> {
  return invoke<Game[]>("scan_games");
}

export async function getCachedGames(): Promise<Game[]> {
  return invoke<Game[]>("get_cached_games");
}

export async function backupGame(gameId: number): Promise<BackupRecord> {
  return invoke<BackupRecord>("backup_game", { gameId });
}

export async function backupAll(): Promise<BackupRecord[]> {
  return invoke<BackupRecord[]>("backup_all");
}

export async function restoreGame(
  gameId: number,
  backupId?: number,
): Promise<void> {
  return invoke("restore_game", { gameId, backupId });
}

export async function getBackups(gameId: number): Promise<BackupRecord[]> {
  return invoke<BackupRecord[]>("get_backups", { gameId });
}

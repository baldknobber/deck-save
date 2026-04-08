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

export interface Setting {
  key: string;
  value: string;
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

export async function getSettings(): Promise<Setting[]> {
  return invoke<Setting[]>("get_settings");
}

export async function updateSetting(
  key: string,
  value: string,
): Promise<void> {
  return invoke("update_setting", { key, value });
}

export async function getSteamHeaderUrl(
  steamId: string,
): Promise<string | null> {
  return invoke<string | null>("get_steam_header_url", { steamId });
}

// ── Sync (Syncthing) ─────────────────────────────────────────────

export interface SyncStatus {
  available: boolean;
  running: boolean;
  my_device_id: string;
  uptime: number;
  api_key: string;
}

export interface SyncDevice {
  device_id: string;
  name: string;
  connected: boolean;
  paused: boolean;
}

export interface SyncFolder {
  id: string;
  label: string;
  path: string;
  folder_type: string;
  paused: boolean;
  completion: number;
}

export interface ConflictFile {
  path: string;
  game_folder: string;
}

export async function syncStatus(): Promise<SyncStatus> {
  return invoke<SyncStatus>("sync_status");
}

export async function syncListDevices(): Promise<SyncDevice[]> {
  return invoke<SyncDevice[]>("sync_list_devices");
}

export async function syncAddDevice(
  deviceId: string,
  name: string,
): Promise<void> {
  return invoke("sync_add_device", { deviceId, name });
}

export async function syncRemoveDevice(deviceId: string): Promise<void> {
  return invoke("sync_remove_device", { deviceId });
}

export async function syncShareFolder(
  syncMode: string,
): Promise<{ folder_id: string; success: boolean }> {
  return invoke("sync_share_folder", { syncMode });
}

export async function syncRemoveFolder(folderId: string): Promise<void> {
  return invoke("sync_remove_folder", { folderId });
}

export async function syncFolderStatus(): Promise<SyncFolder[]> {
  return invoke<SyncFolder[]>("sync_folder_status");
}

export async function syncDetectConflicts(): Promise<ConflictFile[]> {
  return invoke<ConflictFile[]>("sync_detect_conflicts");
}

export async function syncResolveConflict(path: string): Promise<void> {
  return invoke("sync_resolve_conflict", { path });
}

export async function syncUpdateSettings(
  apiKey?: string,
  baseUrl?: string,
): Promise<void> {
  return invoke("sync_update_settings", { apiKey, baseUrl });
}

// ── Steam Shortcut Registration ──────────────────────────────────

export interface ShortcutResult {
  registered: boolean;
  already_existed: boolean;
}

export async function checkSteamShortcut(): Promise<boolean> {
  return invoke<boolean>("check_steam_shortcut");
}

export async function registerSteamShortcut(): Promise<ShortcutResult> {
  return invoke<ShortcutResult>("register_steam_shortcut");
}

// ── Syncthing Management ────────────────────────────────────────────

export interface SyncthingInfo {
  installed: boolean;
  path: string | null;
  version: string | null;
  managed: boolean;
}

export interface InstallResult {
  version: string;
  path: string;
}

export async function checkSyncthingInstalled(): Promise<SyncthingInfo> {
  return invoke<SyncthingInfo>("check_syncthing_installed");
}

export async function installSyncthing(): Promise<InstallResult> {
  return invoke<InstallResult>("install_syncthing");
}

export async function startSyncthing(): Promise<void> {
  return invoke("start_syncthing");
}

export async function stopSyncthing(): Promise<void> {
  return invoke("stop_syncthing");
}

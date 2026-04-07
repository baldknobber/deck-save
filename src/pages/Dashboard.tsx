import { useState, useEffect, useCallback } from "react";
import {
  scanGames,
  getCachedGames,
  backupAll,
  backupGame,
  restoreGame,
  getBackups,
  type Game,
  type BackupRecord,
} from "../lib/api";
import { listen } from "@tauri-apps/api/event";

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

export default function Dashboard() {
  const [games, setGames] = useState<Game[]>([]);
  const [scanning, setScanning] = useState(false);
  const [backingUpAll, setBackingUpAll] = useState(false);
  const [busyGameId, setBusyGameId] = useState<number | null>(null);
  const [search, setSearch] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [selectedGame, setSelectedGame] = useState<Game | null>(null);
  const [backups, setBackups] = useState<BackupRecord[]>([]);
  const [toast, setToast] = useState<string | null>(null);

  const refreshGames = useCallback(() => {
    getCachedGames()
      .then((cached) => {
        if (cached.length > 0) setGames(cached);
      })
      .catch(() => {});
  }, []);

  // Load cached games from SQLite on mount (instant, no scanning)
  useEffect(() => {
    refreshGames();
  }, [refreshGames]);

  // Listen for save-changed + auto-backup-complete events from watcher
  useEffect(() => {
    const unlisteners: Array<() => void> = [];

    listen<{ game_id: number; game_title: string }>("save-changed", (event) => {
      // Refresh game list so the "Changed" badge shows up
      refreshGames();
      setToast(`Save changed: ${event.payload.game_title}`);
      setTimeout(() => setToast(null), 3000);
    }).then((fn) => unlisteners.push(fn));

    listen<{ backed_up: number; failed: number; game_titles: string[] }>(
      "auto-backup-complete",
      (event) => {
        refreshGames();
        const { backed_up, game_titles } = event.payload;
        if (backed_up > 0) {
          setToast(`Auto-backed up ${backed_up} game${backed_up > 1 ? "s" : ""}: ${game_titles.join(", ")}`);
          setTimeout(() => setToast(null), 4000);
        }
      },
    ).then((fn) => unlisteners.push(fn));

    return () => {
      unlisteners.forEach((fn) => fn());
    };
  }, [refreshGames]);

  const handleScan = async () => {
    setScanning(true);
    setError(null);
    try {
      const result = await scanGames();
      setGames(result);
    } catch (err) {
      setError(String(err));
    } finally {
      setScanning(false);
    }
  };

  const handleBackupAll = async () => {
    setBackingUpAll(true);
    setError(null);
    try {
      await backupAll();
      // Refresh game list to update statuses
      const refreshed = await getCachedGames();
      setGames(refreshed);
    } catch (err) {
      setError(String(err));
    } finally {
      setBackingUpAll(false);
    }
  };

  const handleBackupGame = async (game: Game) => {
    setBusyGameId(game.id);
    setError(null);
    try {
      await backupGame(game.id);
      const refreshed = await getCachedGames();
      setGames(refreshed);
      // Refresh backup list if this game is selected
      if (selectedGame?.id === game.id) {
        setBackups(await getBackups(game.id));
      }
    } catch (err) {
      setError(String(err));
    } finally {
      setBusyGameId(null);
    }
  };

  const handleSelectGame = async (game: Game) => {
    setSelectedGame(game);
    try {
      setBackups(await getBackups(game.id));
    } catch {
      setBackups([]);
    }
  };

  const handleRestore = async (game: Game, backupId?: number) => {
    setBusyGameId(game.id);
    setError(null);
    try {
      await restoreGame(game.id, backupId);
    } catch (err) {
      setError(String(err));
    } finally {
      setBusyGameId(null);
    }
  };

  const filteredGames = games.filter((g) =>
    g.title.toLowerCase().includes(search.toLowerCase()),
  );

  return (
    <div className="flex gap-6 h-full">
      {/* Game list */}
      <div className={selectedGame ? "w-2/3" : "w-full"}>
        <div className="flex items-center justify-between mb-6">
          <h2 className="text-2xl font-bold">Your Games</h2>
          <div className="flex gap-3">
            <input
              type="text"
              placeholder="Search games..."
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              className="bg-gray-800 border border-gray-600 rounded-lg px-4 py-2 text-sm focus:outline-none focus:border-blue-500"
            />
            <button
              onClick={handleScan}
              disabled={scanning}
              className="bg-blue-600 hover:bg-blue-700 disabled:opacity-50 px-4 py-2 rounded-lg text-sm font-medium transition-colors"
            >
              {scanning ? "Scanning..." : "Scan Games"}
            </button>
            <button
              onClick={handleBackupAll}
              disabled={backingUpAll}
              className="bg-green-600 hover:bg-green-700 disabled:opacity-50 px-4 py-2 rounded-lg text-sm font-medium transition-colors"
            >
              {backingUpAll ? "Backing Up..." : "Back Up All"}
            </button>
          </div>
        </div>

        {error && (
          <div className="mb-4 p-4 bg-red-900/50 border border-red-700 rounded-lg text-red-200 text-sm">
            <span className="font-semibold">Error:</span> {error}
          </div>
        )}

        {toast && (
          <div className="mb-4 p-3 bg-blue-900/50 border border-blue-700 rounded-lg text-blue-200 text-sm animate-pulse">
            {toast}
          </div>
        )}

        {games.length === 0 && !error ? (
          <div className="text-center py-20 text-gray-500">
            <p className="text-lg mb-2">No games detected yet</p>
            <p className="text-sm">
              Click &quot;Scan Games&quot; to detect your installed games and
              their save files.
            </p>
          </div>
        ) : (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            {filteredGames.map((game) => (
              <div
                key={game.id}
                onClick={() => handleSelectGame(game)}
                className={`bg-gray-800 rounded-lg p-4 border transition-colors cursor-pointer ${
                  selectedGame?.id === game.id
                    ? "border-blue-500"
                    : "border-gray-700 hover:border-gray-600"
                }`}
              >
                <h3 className="font-semibold text-white mb-1">{game.title}</h3>
                {game.steam_id && (
                  <p className="text-xs text-gray-500 mb-1">
                    Steam ID: {game.steam_id}
                  </p>
                )}
                {game.save_path_count > 0 && (
                  <p className="text-xs text-green-400 mb-2">
                    {game.save_path_count} save location
                    {game.save_path_count > 1 ? "s" : ""} detected
                  </p>
                )}
                <div className="flex items-center justify-between mb-2">
                  <span
                    className={`text-xs px-2 py-1 rounded ${
                      game.status === "backed_up"
                        ? "bg-green-900 text-green-300"
                        : game.status === "changed"
                          ? "bg-yellow-900 text-yellow-300"
                          : "bg-gray-700 text-gray-400"
                    }`}
                  >
                    {game.status === "backed_up"
                      ? "Backed Up"
                      : game.status === "changed"
                        ? "Changed"
                        : "Never Backed Up"}
                  </span>
                  {game.last_backup && (
                    <span className="text-xs text-gray-500">
                      {game.last_backup}
                    </span>
                  )}
                </div>
                <div className="flex gap-2">
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      handleBackupGame(game);
                    }}
                    disabled={
                      busyGameId === game.id || game.save_path_count === 0
                    }
                    className="flex-1 bg-blue-600 hover:bg-blue-700 disabled:opacity-50 px-3 py-1.5 rounded text-xs font-medium transition-colors"
                  >
                    {busyGameId === game.id ? "..." : "Backup"}
                  </button>
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      handleRestore(game);
                    }}
                    disabled={
                      busyGameId === game.id || !game.last_backup
                    }
                    className="flex-1 bg-amber-600 hover:bg-amber-700 disabled:opacity-50 px-3 py-1.5 rounded text-xs font-medium transition-colors"
                  >
                    Restore
                  </button>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Backup history panel */}
      {selectedGame && (
        <div className="w-1/3 bg-gray-800 rounded-lg border border-gray-700 p-4 h-fit max-h-[calc(100vh-8rem)] overflow-y-auto">
          <div className="flex items-center justify-between mb-4">
            <h3 className="font-semibold text-white">
              {selectedGame.title}
            </h3>
            <button
              onClick={() => setSelectedGame(null)}
              className="text-gray-400 hover:text-white text-sm"
            >
              Close
            </button>
          </div>

          <div className="mb-3">
            <p className="text-xs text-gray-400 mb-1">Save Locations</p>
            {selectedGame.save_paths.map((p, i) => (
              <p key={i} className="text-xs text-gray-300 truncate" title={p}>
                {p}
              </p>
            ))}
          </div>

          <h4 className="text-sm font-medium text-gray-300 mb-2">
            Backup History
          </h4>
          {backups.length === 0 ? (
            <p className="text-xs text-gray-500">No backups yet</p>
          ) : (
            <div className="space-y-2">
              {backups.map((b) => (
                <div
                  key={b.id}
                  className="bg-gray-700 rounded p-3 text-xs"
                >
                  <div className="flex justify-between mb-1">
                    <span className="text-gray-300">{b.timestamp}</span>
                    <span className="text-gray-400">
                      {formatBytes(b.size_bytes)}
                    </span>
                  </div>
                  <p
                    className="text-gray-500 truncate mb-2"
                    title={b.checksum}
                  >
                    SHA-256: {b.checksum.slice(0, 16)}...
                  </p>
                  <button
                    onClick={() => handleRestore(selectedGame, b.id)}
                    disabled={busyGameId === selectedGame.id}
                    className="bg-amber-600 hover:bg-amber-700 disabled:opacity-50 px-3 py-1 rounded text-xs font-medium transition-colors w-full"
                  >
                    Restore This Version
                  </button>
                </div>
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
}

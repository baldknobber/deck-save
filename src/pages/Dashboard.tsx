import { useState, useEffect, useCallback, useRef, useMemo } from "react";
import {
  scanGames,
  getCachedGames,
  backupAll,
  backupGame,
  restoreGame,
  getBackups,
  getSteamHeaderUrl,
  addCustomSavePath,
  removeCustomSavePath,
  type Game,
  type BackupRecord,
} from "../lib/api";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { DeckButton, DeckCard, DeckInput, DeckModal, DeckStatusBadge } from "../components/deck";
import { useGridNav } from "../hooks/useGridNav";

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function steamImageUrls(steamId: string | null): string[] {
  if (!steamId) return [];
  return [
    // Classic CDN (works for most games)
    `https://cdn.akamai.steamstatic.com/steam/apps/${steamId}/header.jpg`,
    // New CDN for newer games (2025+)
    `https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/${steamId}/header.jpg`,
    // Alternate CDN
    `https://steamcdn-a.akamaihd.net/steam/apps/${steamId}/header.jpg`,
    // Capsule fallback (different aspect but better than nothing)
    `https://cdn.akamai.steamstatic.com/steam/apps/${steamId}/capsule_616x353.jpg`,
    `https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/${steamId}/capsule_616x353.jpg`,
  ];
}

function GameImage({ steamId, title }: { steamId: string | null; title: string }) {
  const [urlIndex, setUrlIndex] = useState(0);
  const [apiUrl, setApiUrl] = useState<string | null | undefined>(undefined); // undefined = not fetched yet
  const urls = steamImageUrls(steamId);

  const allCdnFailed = urls.length === 0 || urlIndex >= urls.length;

  useEffect(() => {
    if (allCdnFailed && apiUrl === undefined && steamId) {
      getSteamHeaderUrl(steamId).then(
        (url) => setApiUrl(url),
        () => setApiUrl(null),
      );
    }
  }, [allCdnFailed, apiUrl, steamId]);

  // Still trying CDN URLs
  if (!allCdnFailed) {
    return (
      <img
        src={urls[urlIndex]}
        alt={title}
        loading="lazy"
        onError={() => setUrlIndex((i) => i + 1)}
        className="w-full aspect-[460/215] object-cover rounded-t-xl"
      />
    );
  }

  // CDN failed, try API URL
  if (apiUrl) {
    return (
      <img
        src={apiUrl}
        alt={title}
        loading="lazy"
        onError={() => setApiUrl(null)} // give up on error
        className="w-full aspect-[460/215] object-cover rounded-t-xl"
      />
    );
  }

  // Fallback: gradient with first letter
  return (
    <div className="w-full aspect-[460/215] bg-gradient-to-br from-gray-700 to-gray-800 rounded-t-xl flex items-center justify-center">
      <span className="text-4xl font-bold text-gray-500">{title.charAt(0).toUpperCase()}</span>
    </div>
  );
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

  const pageRef = useRef<HTMLDivElement>(null);
  const gridRef = useRef<HTMLDivElement>(null);
  useGridNav(pageRef, 3);

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
      const refreshed = await getCachedGames();
      setGames(refreshed);
      if (selectedGame?.id === game.id) {
        setBackups(await getBackups(game.id));
      }
    } catch (err) {
      setError(String(err));
    } finally {
      setBusyGameId(null);
    }
  };

  const filteredGames = useMemo(
    () => games.filter((g) => g.title.toLowerCase().includes(search.toLowerCase())),
    [games, search],
  );

  const gamesWithSaves = useMemo(
    () => games.filter((g) => g.save_path_count > 0).length,
    [games],
  );

  return (
    <div ref={pageRef} className="h-full flex flex-col">
      {/* Top bar: search + actions — horizontal nav zone */}
      <div data-nav-zone="topbar" data-nav-type="horizontal" className="flex-shrink-0 flex items-center gap-3 mb-4">
        <DeckInput
          placeholder="Search games..."
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          className="flex-1"
          icon={
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <circle cx="11" cy="11" r="8" />
              <path d="M21 21l-4.35-4.35" />
            </svg>
          }
        />
        <DeckButton onClick={handleScan} loading={scanning} className="flex-shrink-0">
          {scanning ? "Scanning..." : "Scan"}
        </DeckButton>
        <DeckButton
          variant="success"
          onClick={handleBackupAll}
          loading={backingUpAll}
          className="flex-shrink-0"
          icon={
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <path d="M19 21H5a2 2 0 01-2-2V5a2 2 0 012-2h11l5 5v11a2 2 0 01-2 2z" />
              <polyline points="17,21 17,13 7,13 7,21" />
              <polyline points="7,3 7,8 15,8" />
            </svg>
          }
        >
          Back Up All
        </DeckButton>
      </div>

      {/* Stats bar */}
      {games.length > 0 && (
        <div className="flex-shrink-0 flex items-center gap-4 mb-4 text-sm text-gray-400">
          <span>{games.length} games found</span>
          <span className="text-gray-600">|</span>
          <span>{gamesWithSaves} with save files</span>
          {search && (
            <>
              <span className="text-gray-600">|</span>
              <span>{filteredGames.length} matching &ldquo;{search}&rdquo;</span>
            </>
          )}
        </div>
      )}

      {/* Error */}
      {error && (
        <div className="flex-shrink-0 mb-4 p-4 bg-red-900/40 border-2 border-red-800 rounded-xl text-red-200 flex items-center gap-3 animate-fade-in">
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <circle cx="12" cy="12" r="10" />
            <line x1="15" y1="9" x2="9" y2="15" />
            <line x1="9" y1="9" x2="15" y2="15" />
          </svg>
          <span className="flex-1">{error}</span>
          <DeckButton variant="ghost" size="sm" onClick={() => setError(null)}>Dismiss</DeckButton>
        </div>
      )}

      {/* Toast */}
      {toast && (
        <div className="flex-shrink-0 mb-4 p-3 bg-blue-900/40 border-2 border-blue-800 rounded-xl text-blue-200 text-sm animate-fade-in flex items-center gap-2">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <path d="M22 11.08V12a10 10 0 11-5.93-9.14" />
            <polyline points="22,4 12,14.01 9,11.01" />
          </svg>
          {toast}
        </div>
      )}

      {/* Empty state */}
      {games.length === 0 && !error ? (
        <div className="flex-1 flex flex-col items-center justify-center text-center animate-fade-in">
          <svg width="64" height="64" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" className="text-gray-600 mb-4">
            <rect x="2" y="6" width="20" height="12" rx="2" />
            <path d="M6 12h4M8 10v4" />
            <circle cx="15" cy="11" r="1" />
            <circle cx="18" cy="13" r="1" />
          </svg>
          <p className="text-xl font-medium text-gray-400 mb-2">No games detected yet</p>
          <p className="text-gray-500 mb-6 max-w-md">
            DeckSave will scan your Steam library for installed games and locate their save files automatically.
          </p>
          <DeckButton size="lg" onClick={handleScan} loading={scanning}>
            Scan Steam Library
          </DeckButton>
        </div>
      ) : (
        /* Game grid — content zone with dynamic columns */
        <div
          ref={gridRef}
          data-nav-zone="grid"
          data-nav-type="grid"
          className="flex-1 overflow-y-auto grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4 content-start pb-2"
        >
          {filteredGames.map((game) => (
            <DeckCard
              key={game.id}
              selected={selectedGame?.id === game.id}
              onClick={() => handleSelectGame(game)}
              onKeyDown={(e) => {
                if (e.key === "Enter") handleSelectGame(game);
              }}
              className="!p-0 overflow-hidden"
            >
              {/* Game art */}
              <GameImage steamId={game.steam_id} title={game.title} />

              {/* Card body — compact: title, badge, last backup only */}
              <div className="p-3">
                <div className="flex items-center justify-between gap-2">
                  <h3 className="font-semibold text-white text-sm leading-tight line-clamp-1">{game.title}</h3>
                  <DeckStatusBadge status={game.status} />
                </div>

                {game.last_backup && (
                  <p className="text-xs text-gray-500 mt-1">
                    Last: {game.last_backup}
                  </p>
                )}
              </div>
            </DeckCard>
          ))}
        </div>
      )}

      {/* Game detail modal */}
      <DeckModal
        open={!!selectedGame}
        onClose={() => setSelectedGame(null)}
        title={selectedGame?.title ?? ""}
      >
        {selectedGame && (
          <div className="space-y-4">
            {/* Game header image in modal */}
            {selectedGame.steam_id && (
              <GameImage steamId={selectedGame.steam_id} title={selectedGame.title} />
            )}

            {/* Save paths */}
            <div>
              <h4 className="text-sm font-medium text-gray-300 mb-2">Save Locations</h4>
              <div className="space-y-1">
                {selectedGame.save_paths.map((p, i) => (
                  <div key={i} className="flex items-center gap-2">
                    <p className="flex-1 text-sm text-gray-400 break-all bg-gray-900/50 px-3 py-2 rounded-lg">
                      {p}
                    </p>
                    {selectedGame.custom_save_paths?.includes(p) && (
                      <DeckButton
                        size="sm"
                        variant="danger"
                        onClick={async () => {
                          try {
                            await removeCustomSavePath(selectedGame.id, p);
                            const refreshed = await getCachedGames();
                            setGames(refreshed);
                            const updated = refreshed.find((g) => g.id === selectedGame.id);
                            if (updated) {
                              setSelectedGame(updated);
                              setBackups(await getBackups(updated.id));
                            }
                          } catch (err) {
                            setError(String(err));
                          }
                        }}
                      >
                        ✕
                      </DeckButton>
                    )}
                  </div>
                ))}
              </div>
              <DeckButton
                size="sm"
                variant="ghost"
                className="mt-2"
                onClick={async () => {
                  const selected = await open({ directory: true, title: "Select Save Folder" });
                  if (selected) {
                    try {
                      await addCustomSavePath(selectedGame.id, selected);
                      const refreshed = await getCachedGames();
                      setGames(refreshed);
                      const updated = refreshed.find((g) => g.id === selectedGame.id);
                      if (updated) {
                        setSelectedGame(updated);
                      }
                    } catch (err) {
                      setError(String(err));
                    }
                  }
                }}
              >
                + Add Save Path
              </DeckButton>
            </div>

            {/* Quick actions */}
            <div className="flex gap-3">
              <DeckButton
                fullWidth
                onClick={() => handleBackupGame(selectedGame)}
                disabled={busyGameId === selectedGame.id || selectedGame.save_path_count === 0}
                loading={busyGameId === selectedGame.id}
              >
                Backup Now
              </DeckButton>
              <DeckButton
                variant="secondary"
                fullWidth
                onClick={() => handleRestore(selectedGame)}
                disabled={busyGameId === selectedGame.id || !selectedGame.last_backup}
              >
                Restore Latest
              </DeckButton>
            </div>

            {/* Backup history */}
            <div>
              <h4 className="text-sm font-medium text-gray-300 mb-2">
                Backup History {backups.length > 0 && `(${backups.length})`}
              </h4>
              {backups.length === 0 ? (
                <p className="text-sm text-gray-500">No backups yet</p>
              ) : (
                <div className="space-y-2 max-h-64 overflow-y-auto">
                  {backups.map((b) => (
                    <div
                      key={b.id}
                      className="bg-gray-900/50 rounded-xl p-3 flex items-center justify-between gap-3"
                    >
                      <div className="min-w-0">
                        <p className="text-sm text-gray-200">{b.timestamp}</p>
                        <p className="text-xs text-gray-500">
                          {formatBytes(b.size_bytes)} &middot; {b.checksum.slice(0, 12)}...
                        </p>
                      </div>
                      <DeckButton
                        size="sm"
                        variant="secondary"
                        onClick={() => handleRestore(selectedGame, b.id)}
                        disabled={busyGameId === selectedGame.id}
                      >
                        Restore
                      </DeckButton>
                    </div>
                  ))}
                </div>
              )}
            </div>
          </div>
        )}
      </DeckModal>
    </div>
  );
}

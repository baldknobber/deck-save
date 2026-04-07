import { useState } from "react";
import { scanGames, backupAll, type Game } from "../lib/api";

export default function Dashboard() {
  const [games, setGames] = useState<Game[]>([]);
  const [scanning, setScanning] = useState(false);
  const [search, setSearch] = useState("");
  const [error, setError] = useState<string | null>(null);

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
    try {
      await backupAll();
    } catch (err) {
      console.error("Backup failed:", err);
    }
  };

  const filteredGames = games.filter((g) =>
    g.title.toLowerCase().includes(search.toLowerCase()),
  );

  return (
    <div>
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
            className="bg-green-600 hover:bg-green-700 px-4 py-2 rounded-lg text-sm font-medium transition-colors"
          >
            Back Up All
          </button>
        </div>
      </div>

      {error && (
        <div className="mb-4 p-4 bg-red-900/50 border border-red-700 rounded-lg text-red-200 text-sm">
          <span className="font-semibold">Scan error:</span> {error}
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
              className="bg-gray-800 rounded-lg p-4 border border-gray-700 hover:border-gray-600 transition-colors cursor-pointer"
            >
              <h3 className="font-semibold text-white mb-1">{game.title}</h3>
              {game.steam_id && (
                <p className="text-xs text-gray-500 mb-1">
                  Steam ID: {game.steam_id}
                </p>
              )}
              {game.save_path_count > 0 && (
                <p className="text-xs text-green-400 mb-2">
                  {game.save_path_count} save location{game.save_path_count > 1 ? "s" : ""} detected
                </p>
              )}
              <div className="flex items-center justify-between">
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
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

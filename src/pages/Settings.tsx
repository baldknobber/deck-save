import { useState, useEffect } from "react";
import { getSettings, updateSetting, getCachedGames } from "../lib/api";

export default function Settings() {
  const [backupDir, setBackupDir] = useState("");
  const [autoBackup, setAutoBackup] = useState(true);
  const [backupInterval, setBackupInterval] = useState("hourly");
  const [maxVersions, setMaxVersions] = useState(5);
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);
  const [watchedCount, setWatchedCount] = useState(0);

  useEffect(() => {
    getSettings()
      .then((settings) => {
        for (const s of settings) {
          switch (s.key) {
            case "backup_dir":
              setBackupDir(s.value);
              break;
            case "auto_backup":
              setAutoBackup(s.value === "true");
              break;
            case "backup_interval":
              setBackupInterval(s.value);
              break;
            case "max_versions":
              setMaxVersions(Number(s.value) || 5);
              break;
          }
        }
      })
      .catch(() => {});

    // Count games with save paths to show watcher status
    getCachedGames()
      .then((games) => {
        setWatchedCount(games.filter((g) => g.save_path_count > 0).length);
      })
      .catch(() => {});
  }, []);

  const handleSave = async () => {
    setSaving(true);
    setSaved(false);
    try {
      await Promise.all([
        updateSetting("backup_dir", backupDir),
        updateSetting("auto_backup", autoBackup ? "true" : "false"),
        updateSetting("backup_interval", backupInterval),
        updateSetting("max_versions", String(maxVersions)),
      ]);
      setSaved(true);
      setTimeout(() => setSaved(false), 2000);
    } catch (err) {
      console.error("Failed to save settings:", err);
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="max-w-2xl">
      <h2 className="text-2xl font-bold mb-6">Settings</h2>

      {watchedCount > 0 && (
        <div className="mb-6 p-3 bg-green-900/30 border border-green-800 rounded-lg text-green-300 text-sm flex items-center gap-2">
          <span className="w-2 h-2 bg-green-400 rounded-full animate-pulse" />
          Watching {watchedCount} game{watchedCount > 1 ? "s" : ""} for save file changes
        </div>
      )}

      <section className="space-y-6">
        <div>
          <label className="block text-sm font-medium text-gray-300 mb-2">
            Backup Directory
          </label>
          <div className="flex gap-2">
            <input
              type="text"
              value={backupDir}
              onChange={(e) => setBackupDir(e.target.value)}
              placeholder="Default: app data directory"
              className="flex-1 bg-gray-800 border border-gray-600 rounded-lg px-4 py-2 text-sm focus:outline-none focus:border-blue-500"
            />
            <button className="bg-gray-700 hover:bg-gray-600 px-4 py-2 rounded-lg text-sm">
              Browse
            </button>
          </div>
        </div>

        <div className="flex items-center justify-between">
          <div>
            <p className="text-sm font-medium text-gray-300">
              Automatic Backups
            </p>
            <p className="text-xs text-gray-500">
              Automatically back up saves when changes are detected
            </p>
          </div>
          <button
            onClick={() => setAutoBackup(!autoBackup)}
            className={`relative w-12 h-6 rounded-full transition-colors ${autoBackup ? "bg-blue-600" : "bg-gray-600"}`}
          >
            <span
              className={`absolute top-0.5 left-0.5 w-5 h-5 bg-white rounded-full transition-transform ${autoBackup ? "translate-x-6" : ""}`}
            />
          </button>
        </div>

        <div>
          <label className="block text-sm font-medium text-gray-300 mb-2">
            Backup Interval
          </label>
          <select
            value={backupInterval}
            onChange={(e) => setBackupInterval(e.target.value)}
            className="bg-gray-800 border border-gray-600 rounded-lg px-4 py-2 text-sm focus:outline-none focus:border-blue-500"
          >
            <option value="on_change">On Change (5 min debounce)</option>
            <option value="hourly">Hourly</option>
            <option value="daily">Daily</option>
          </select>
        </div>

        <div>
          <label className="block text-sm font-medium text-gray-300 mb-2">
            Max Backup Versions Per Game
          </label>
          <input
            type="number"
            value={maxVersions}
            onChange={(e) => setMaxVersions(Number(e.target.value))}
            min={1}
            max={50}
            className="bg-gray-800 border border-gray-600 rounded-lg px-4 py-2 text-sm w-24 focus:outline-none focus:border-blue-500"
          />
        </div>

        <button
          onClick={handleSave}
          disabled={saving}
          className="bg-blue-600 hover:bg-blue-700 disabled:opacity-50 px-6 py-2 rounded-lg text-sm font-medium transition-colors"
        >
          {saving ? "Saving..." : saved ? "Saved!" : "Save Settings"}
        </button>
      </section>
    </div>
  );
}

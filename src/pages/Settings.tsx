import { useState, useEffect, useRef } from "react";
import { useNavigate } from "react-router-dom";
import { getSettings, updateSetting, getCachedGames } from "../lib/api";
import { DeckButton, DeckInput, DeckToggle, DeckSelect } from "../components/deck";
import { open } from "@tauri-apps/plugin-dialog";
import { useGridNav } from "../hooks/useGridNav";

export default function Settings() {
  const containerRef = useRef<HTMLDivElement>(null);
  useGridNav(containerRef, 1);
  const navigate = useNavigate();

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

    getCachedGames()
      .then((games) => {
        setWatchedCount(games.filter((g) => g.save_path_count > 0).length);
      })
      .catch(() => {});
  }, []);

  const handleBrowseBackupDir = async () => {
    const selected = await open({ directory: true, title: "Select Backup Directory" });
    if (selected) setBackupDir(selected);
  };

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
    <div ref={containerRef} className="max-w-2xl mx-auto">
      <h2 className="text-2xl font-bold mb-5">Settings</h2>

      {/* Watcher status */}
      {watchedCount > 0 && (
        <div className="mb-5 p-4 bg-emerald-900/30 border-2 border-emerald-800 rounded-xl text-emerald-300 flex items-center gap-3">
          <span className="w-3 h-3 bg-emerald-400 rounded-full animate-pulse flex-shrink-0" />
          <span className="text-base">
            Watching {watchedCount} game{watchedCount > 1 ? "s" : ""} for save file changes
          </span>
        </div>
      )}

      <section className="space-y-4">
        {/* Backup directory */}
        <div className="flex gap-3 items-end">
          <DeckInput
            label="Backup Directory"
            value={backupDir}
            onChange={(e) => setBackupDir(e.target.value)}
            placeholder="Default: app data directory"
          />
          <DeckButton variant="secondary" className="flex-shrink-0" onClick={handleBrowseBackupDir}>
            Browse
          </DeckButton>
        </div>

        {/* Auto-backup toggle */}
        <DeckToggle
          checked={autoBackup}
          onChange={setAutoBackup}
          label="Automatic Backups"
          description="Back up saves automatically when changes are detected"
        />

        {/* Backup interval */}
        <DeckSelect
          label="Backup Interval"
          value={backupInterval}
          onChange={setBackupInterval}
          options={[
            { value: "on_change", label: "On Change (5 min debounce)" },
            { value: "hourly", label: "Hourly" },
            { value: "daily", label: "Daily" },
          ]}
        />

        {/* Max versions */}
        <DeckInput
          label="Max Backup Versions Per Game"
          type="number"
          value={String(maxVersions)}
          onChange={(e) => setMaxVersions(Number(e.target.value) || 5)}
          min={1}
          max={50}
          className="!w-32"
        />

        {/* Save button */}
        <div className="pt-2">
          <DeckButton
            size="lg"
            onClick={handleSave}
            loading={saving}
            variant={saved ? "success" : "primary"}
            icon={
              saved ? (
                <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                  <polyline points="20,6 9,17 4,12" />
                </svg>
              ) : undefined
            }
          >
            {saving ? "Saving..." : saved ? "Saved!" : "Save Settings"}
          </DeckButton>
        </div>
      </section>

      {/* Re-run setup */}
      <section className="border-t border-gray-700 pt-6 mt-6">
        <DeckButton
          data-deck-focusable
          variant="ghost"
          onClick={async () => {
            await updateSetting("setup_complete", "false");
            navigate("/setup");
          }}
        >
          Re-run Setup Wizard
        </DeckButton>
      </section>
    </div>
  );
}

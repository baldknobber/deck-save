import { useState, useEffect, useCallback, useRef } from "react";
import {
  syncStatus,
  syncListDevices,
  syncAddDevice,
  syncRemoveDevice,
  syncShareFolder,
  syncFolderStatus,
  syncDetectConflicts,
  syncResolveConflict,
  syncUpdateSettings,
  type SyncStatus,
  type SyncDevice,
  type SyncFolder,
  type ConflictFile,
} from "../lib/api";
import {
  DeckButton,
  DeckCard,
  DeckInput,
  DeckModal,
  DeckSelect,
} from "../components/deck";
import { useGridNav, useTriggerNav } from "../hooks/useGridNav";

type Step = "status" | "devices" | "folder" | "monitor";
const STEPS: Step[] = ["status", "devices", "folder", "monitor"];

export default function SyncWizard() {
  const containerRef = useRef<HTMLDivElement>(null);
  useGridNav(containerRef, 1);

  // L2/R2 for sub-tab switching
  useTriggerNav(
    useCallback(
      (delta: -1 | 1) => {
        setStep((prev) => {
          const idx = STEPS.indexOf(prev);
          const next = (idx + delta + STEPS.length) % STEPS.length;
          return STEPS[next];
        });
      },
      [],
    ),
  );

  const [step, setStep] = useState<Step>("status");
  const [status, setStatus] = useState<SyncStatus | null>(null);
  const [devices, setDevices] = useState<SyncDevice[]>([]);
  const [folders, setFolders] = useState<SyncFolder[]>([]);
  const [conflicts, setConflicts] = useState<ConflictFile[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");

  // Add device form
  const [showAddDevice, setShowAddDevice] = useState(false);
  const [newDeviceId, setNewDeviceId] = useState("");
  const [newDeviceName, setNewDeviceName] = useState("");
  const [addingDevice, setAddingDevice] = useState(false);

  // Settings form
  const [showSettings, setShowSettings] = useState(false);
  const [apiKeyInput, setApiKeyInput] = useState("");
  const [urlInput, setUrlInput] = useState("");

  // Share folder
  const [syncMode, setSyncMode] = useState("sendreceive");
  const [sharing, setSharing] = useState(false);

  const refresh = useCallback(async () => {
    try {
      const s = await syncStatus();
      setStatus(s);
      if (s.api_key) setApiKeyInput(s.api_key);

      if (s.running) {
        const [devs, folds, confs] = await Promise.all([
          syncListDevices().catch(() => [] as SyncDevice[]),
          syncFolderStatus().catch(() => [] as SyncFolder[]),
          syncDetectConflicts().catch(() => [] as ConflictFile[]),
        ]);
        setDevices(devs);
        setFolders(folds);
        setConflicts(confs);
      }
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  // Auto-refresh monitor every 10s
  useEffect(() => {
    if (step !== "monitor") return;
    const interval = setInterval(refresh, 10_000);
    return () => clearInterval(interval);
  }, [step, refresh]);

  const handleAddDevice = async () => {
    if (!newDeviceId.trim() || !newDeviceName.trim()) return;
    setAddingDevice(true);
    setError("");
    try {
      await syncAddDevice(newDeviceId.trim(), newDeviceName.trim());
      setNewDeviceId("");
      setNewDeviceName("");
      setShowAddDevice(false);
      await refresh();
    } catch (err) {
      setError(String(err));
    } finally {
      setAddingDevice(false);
    }
  };

  const handleRemoveDevice = async (deviceId: string) => {
    setError("");
    try {
      await syncRemoveDevice(deviceId);
      await refresh();
    } catch (err) {
      setError(String(err));
    }
  };

  const handleShareFolder = async () => {
    setSharing(true);
    setError("");
    try {
      await syncShareFolder(syncMode);
      await refresh();
      setStep("monitor");
    } catch (err) {
      setError(String(err));
    } finally {
      setSharing(false);
    }
  };

  const handleSaveSettings = async () => {
    setError("");
    try {
      await syncUpdateSettings(
        apiKeyInput || undefined,
        urlInput || undefined,
      );
      setShowSettings(false);
      setLoading(true);
      await refresh();
    } catch (err) {
      setError(String(err));
    }
  };

  const handleResolveConflict = async (path: string) => {
    try {
      await syncResolveConflict(path);
      setConflicts((prev) => prev.filter((c) => c.path !== path));
    } catch (err) {
      setError(String(err));
    }
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center py-20">
        <div className="animate-spin w-8 h-8 border-2 border-blue-500 border-t-transparent rounded-full" />
      </div>
    );
  }

  return (
    <div ref={containerRef} className="max-w-3xl mx-auto">
      {/* Header */}
      <div className="flex items-center justify-between mb-5">
        <h2 className="text-2xl font-bold">Sync</h2>
        <DeckButton
          variant="ghost"
          size="sm"
          onClick={() => setShowSettings(true)}
        >
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <path d="M12.22 2h-.44a2 2 0 00-2 2v.18a2 2 0 01-1 1.73l-.43.25a2 2 0 01-2 0l-.15-.08a2 2 0 00-2.73.73l-.22.38a2 2 0 00.73 2.73l.15.1a2 2 0 011 1.72v.51a2 2 0 01-1 1.74l-.15.09a2 2 0 00-.73 2.73l.22.38a2 2 0 002.73.73l.15-.08a2 2 0 012 0l.43.25a2 2 0 011 1.73V20a2 2 0 002 2h.44a2 2 0 002-2v-.18a2 2 0 011-1.73l.43-.25a2 2 0 012 0l.15.08a2 2 0 002.73-.73l.22-.39a2 2 0 00-.73-2.73l-.15-.08a2 2 0 01-1-1.74v-.5a2 2 0 011-1.74l.15-.09a2 2 0 00.73-2.73l-.22-.38a2 2 0 00-2.73-.73l-.15.08a2 2 0 01-2 0l-.43-.25a2 2 0 01-1-1.73V4a2 2 0 00-2-2z" />
            <circle cx="12" cy="12" r="3" />
          </svg>
          Settings
        </DeckButton>
      </div>

      {/* Error banner */}
      {error && (
        <div className="mb-4 p-3 bg-red-900/40 border-2 border-red-800 rounded-xl text-red-300 text-sm flex items-center justify-between">
          <span>{error}</span>
          <button data-deck-focusable onClick={() => setError("")} className="text-red-400 hover:text-red-300 ml-3">✕</button>
        </div>
      )}

      {/* Step navigation */}
      <div className="flex gap-1 mb-5 bg-gray-800/50 rounded-xl p-1">
        {(
          [
            { id: "status", label: "Status" },
            { id: "devices", label: "Devices" },
            { id: "folder", label: "Share" },
            { id: "monitor", label: "Monitor" },
          ] as { id: Step; label: string }[]
        ).map((s) => (
          <button
            key={s.id}
            data-deck-focusable
            onClick={() => setStep(s.id)}
            className={`flex-1 min-h-[44px] rounded-lg text-sm font-medium transition-colors ${
              step === s.id
                ? "bg-blue-600 text-white"
                : "text-gray-400 hover:text-gray-200 hover:bg-gray-700/50"
            }`}
          >
            {s.label}
          </button>
        ))}
      </div>

      {/* ── Status tab ── */}
      {step === "status" && (
        <div className="space-y-4">
          <DeckCard>
            <div className="flex items-center gap-4">
              <div
                className={`w-4 h-4 rounded-full flex-shrink-0 ${
                  status?.running
                    ? "bg-emerald-400 animate-pulse"
                    : status?.available
                      ? "bg-amber-400"
                      : "bg-red-400"
                }`}
              />
              <div className="flex-1">
                <h3 className="text-lg font-semibold">
                  {status?.running
                    ? "Syncthing Running"
                    : status?.available
                      ? "Syncthing Not Running"
                      : "Syncthing Not Found"}
                </h3>
                <p className="text-sm text-gray-400 mt-1">
                  {status?.running
                    ? `Connected · Uptime: ${formatUptime(status.uptime)}`
                    : status?.available
                      ? "API key found but Syncthing is not responding. Start Syncthing and refresh."
                      : "Install Syncthing to sync saves between devices."}
                </p>
              </div>
              <DeckButton variant="secondary" size="sm" onClick={refresh}>
                Refresh
              </DeckButton>
            </div>
          </DeckCard>

          {status?.running && (
            <>
              <DeckCard>
                <h4 className="text-sm font-medium text-gray-400 mb-2">This Device ID</h4>
                <code className="text-xs bg-gray-900 px-3 py-2 rounded-lg block break-all font-mono text-blue-300 select-all">
                  {status.my_device_id}
                </code>
                <p className="text-xs text-gray-500 mt-2">
                  Share this ID with the other device to pair.
                </p>
              </DeckCard>

              <div className="flex gap-3">
                <DeckButton
                  fullWidth
                  onClick={() => setStep("devices")}
                >
                  {devices.length > 0 ? `Manage Devices (${devices.length})` : "Add a Device"}
                </DeckButton>
                {devices.length > 0 && (
                  <DeckButton
                    variant="success"
                    fullWidth
                    onClick={() => setStep("folder")}
                  >
                    Share Backups
                  </DeckButton>
                )}
              </div>
            </>
          )}

          {!status?.available && (
            <DeckCard>
              <h4 className="font-medium mb-2">Getting Started</h4>
              <ol className="text-sm text-gray-400 space-y-2 list-decimal list-inside">
                <li>Install Syncthing from <span className="text-blue-400">syncthing.net</span></li>
                <li>Run Syncthing on both devices (PC + Steam Deck)</li>
                <li>Come back here and click Refresh</li>
                <li>DeckSave will auto-detect the API key</li>
              </ol>
            </DeckCard>
          )}
        </div>
      )}

      {/* ── Devices tab ── */}
      {step === "devices" && (
        <div className="space-y-4">
          {devices.length === 0 && !showAddDevice && (
            <DeckCard>
              <div className="text-center py-6">
                <svg className="mx-auto mb-3 text-gray-600" width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
                  <rect x="5" y="2" width="14" height="20" rx="2" ry="2" />
                  <path d="M12 18h.01" />
                </svg>
                <p className="text-gray-400 mb-4">No paired devices yet</p>
                <DeckButton onClick={() => setShowAddDevice(true)}>
                  Add Device
                </DeckButton>
              </div>
            </DeckCard>
          )}

          {devices.map((d) => (
            <DeckCard key={d.device_id}>
              <div className="flex items-center gap-4">
                <div
                  className={`w-3 h-3 rounded-full flex-shrink-0 ${
                    d.connected ? "bg-emerald-400" : d.paused ? "bg-gray-500" : "bg-amber-400"
                  }`}
                />
                <div className="flex-1 min-w-0">
                  <h4 className="font-medium truncate">{d.name}</h4>
                  <p className="text-xs text-gray-500 font-mono truncate">{d.device_id}</p>
                  <span className="text-xs text-gray-400">
                    {d.connected ? "Connected" : d.paused ? "Paused" : "Disconnected"}
                  </span>
                </div>
                <DeckButton
                  variant="danger"
                  size="sm"
                  onClick={() => handleRemoveDevice(d.device_id)}
                >
                  Remove
                </DeckButton>
              </div>
            </DeckCard>
          ))}

          {devices.length > 0 && !showAddDevice && (
            <DeckButton
              variant="secondary"
              fullWidth
              onClick={() => setShowAddDevice(true)}
            >
              Add Another Device
            </DeckButton>
          )}

          {/* Add device form */}
          <DeckModal
            open={showAddDevice}
            onClose={() => setShowAddDevice(false)}
            title="Add Device"
          >
            <div className="space-y-4">
              <p className="text-sm text-gray-400">
                Enter the Syncthing Device ID from the other machine. You can find it in Syncthing's web UI under Actions → Show ID.
              </p>
              <DeckInput
                label="Device ID"
                value={newDeviceId}
                onChange={(e) => setNewDeviceId(e.target.value)}
                placeholder="XXXXXXX-XXXXXXX-XXXXXXX-XXXXXXX-..."
              />
              <DeckInput
                label="Device Name"
                value={newDeviceName}
                onChange={(e) => setNewDeviceName(e.target.value)}
                placeholder="e.g. Steam Deck, Gaming PC"
              />
              <div className="flex gap-3 pt-2">
                <DeckButton
                  variant="secondary"
                  fullWidth
                  onClick={() => setShowAddDevice(false)}
                >
                  Cancel
                </DeckButton>
                <DeckButton
                  fullWidth
                  loading={addingDevice}
                  onClick={handleAddDevice}
                  disabled={!newDeviceId.trim() || !newDeviceName.trim()}
                >
                  Add Device
                </DeckButton>
              </div>
            </div>
          </DeckModal>
        </div>
      )}

      {/* ── Share tab ── */}
      {step === "folder" && (
        <div className="space-y-4">
          <DeckCard>
            <h3 className="font-semibold mb-3">Share Backup Folder</h3>
            <p className="text-sm text-gray-400 mb-4">
              This will share your DeckSave backup directory with all paired devices via Syncthing.
            </p>
            <DeckSelect
              label="Sync Mode"
              value={syncMode}
              onChange={setSyncMode}
              options={[
                { value: "sendreceive", label: "Send & Receive (recommended)" },
                { value: "sendonly", label: "Send Only (this device → others)" },
                { value: "receiveonly", label: "Receive Only (others → this device)" },
              ]}
            />
            <div className="mt-4">
              <DeckButton
                fullWidth
                loading={sharing}
                onClick={handleShareFolder}
                disabled={devices.length === 0}
              >
                {devices.length === 0
                  ? "Add a device first"
                  : `Share with ${devices.length} device${devices.length > 1 ? "s" : ""}`}
              </DeckButton>
            </div>
          </DeckCard>

          {folders.length > 0 && (
            <div className="space-y-3">
              <h4 className="text-sm font-medium text-gray-400">Active Shared Folders</h4>
              {folders.map((f) => (
                <DeckCard key={f.id}>
                  <div className="flex items-center justify-between">
                    <div>
                      <h5 className="font-medium">{f.label}</h5>
                      <p className="text-xs text-gray-500 truncate">{f.path}</p>
                      <span className="text-xs text-gray-400">
                        {f.folder_type === "sendreceive"
                          ? "Send & Receive"
                          : f.folder_type === "sendonly"
                            ? "Send Only"
                            : "Receive Only"}
                      </span>
                    </div>
                    <div className="text-right">
                      <div className="text-2xl font-bold text-blue-400">
                        {Math.round(f.completion)}%
                      </div>
                      <span className={`text-xs ${f.paused ? "text-gray-500" : "text-emerald-400"}`}>
                        {f.paused ? "Paused" : "Syncing"}
                      </span>
                    </div>
                  </div>
                  {/* Progress bar */}
                  <div className="mt-3 h-2 bg-gray-700 rounded-full overflow-hidden">
                    <div
                      className="h-full bg-blue-500 rounded-full transition-all duration-500"
                      style={{ width: `${Math.min(100, f.completion)}%` }}
                    />
                  </div>
                </DeckCard>
              ))}
            </div>
          )}
        </div>
      )}

      {/* ── Monitor tab ── */}
      {step === "monitor" && (
        <div className="space-y-4">
          {/* Devices overview */}
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
            {devices.map((d) => (
              <DeckCard key={d.device_id}>
                <div className="flex items-center gap-3">
                  <div
                    className={`w-3 h-3 rounded-full ${
                      d.connected ? "bg-emerald-400 animate-pulse" : "bg-gray-500"
                    }`}
                  />
                  <div className="min-w-0">
                    <p className="font-medium truncate">{d.name}</p>
                    <p className="text-xs text-gray-400">
                      {d.connected ? "Connected" : "Offline"}
                    </p>
                  </div>
                </div>
              </DeckCard>
            ))}
          </div>

          {/* Folder sync progress */}
          {folders.map((f) => (
            <DeckCard key={f.id}>
              <div className="flex items-center justify-between mb-2">
                <h4 className="font-medium">{f.label}</h4>
                <span className="text-lg font-bold text-blue-400">
                  {Math.round(f.completion)}%
                </span>
              </div>
              <div className="h-3 bg-gray-700 rounded-full overflow-hidden">
                <div
                  className={`h-full rounded-full transition-all duration-500 ${
                    f.completion >= 100 ? "bg-emerald-500" : "bg-blue-500"
                  }`}
                  style={{ width: `${Math.min(100, f.completion)}%` }}
                />
              </div>
              <p className="text-xs text-gray-500 mt-2">
                {f.completion >= 100
                  ? "Up to date"
                  : "Syncing..."}
              </p>
            </DeckCard>
          ))}

          {folders.length === 0 && (
            <DeckCard>
              <div className="text-center py-6 text-gray-400">
                <p className="mb-3">No folders are being synced yet.</p>
                <DeckButton onClick={() => setStep("folder")}>
                  Set Up Sharing
                </DeckButton>
              </div>
            </DeckCard>
          )}

          {/* Conflicts */}
          {conflicts.length > 0 && (
            <div className="space-y-3">
              <h4 className="text-sm font-medium text-amber-400 flex items-center gap-2">
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                  <path d="M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0z" />
                  <line x1="12" y1="9" x2="12" y2="13" />
                  <line x1="12" y1="17" x2="12.01" y2="17" />
                </svg>
                {conflicts.length} Conflict{conflicts.length > 1 ? "s" : ""} Detected
              </h4>
              {conflicts.map((c) => (
                <DeckCard key={c.path}>
                  <div className="flex items-center justify-between gap-3">
                    <div className="min-w-0 flex-1">
                      <p className="text-sm font-mono truncate text-amber-300">
                        {c.path.split(/[/\\]/).pop()}
                      </p>
                      <p className="text-xs text-gray-500 truncate">{c.path}</p>
                    </div>
                    <DeckButton
                      variant="danger"
                      size="sm"
                      onClick={() => handleResolveConflict(c.path)}
                    >
                      Delete
                    </DeckButton>
                  </div>
                </DeckCard>
              ))}
            </div>
          )}

          {/* Auto-refresh indicator */}
          <p className="text-xs text-gray-600 text-center">
            Auto-refreshes every 10 seconds
          </p>
        </div>
      )}

      {/* ── Settings modal ── */}
      <DeckModal
        open={showSettings}
        onClose={() => setShowSettings(false)}
        title="Syncthing Settings"
      >
        <div className="space-y-4">
          <p className="text-sm text-gray-400">
            DeckSave auto-detects these from Syncthing's config. Only change if auto-detect fails.
          </p>
          <DeckInput
            label="API Key"
            value={apiKeyInput}
            onChange={(e) => setApiKeyInput(e.target.value)}
            placeholder="Auto-detected from Syncthing config"
          />
          <DeckInput
            label="Syncthing URL"
            value={urlInput}
            onChange={(e) => setUrlInput(e.target.value)}
            placeholder="http://127.0.0.1:8384"
          />
          <div className="flex gap-3 pt-2">
            <DeckButton
              variant="secondary"
              fullWidth
              onClick={() => setShowSettings(false)}
            >
              Cancel
            </DeckButton>
            <DeckButton fullWidth onClick={handleSaveSettings}>
              Save
            </DeckButton>
          </div>
        </div>
      </DeckModal>
    </div>
  );
}

function formatUptime(seconds: number): string {
  if (seconds < 60) return `${seconds}s`;
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m`;
  const hours = Math.floor(seconds / 3600);
  const mins = Math.floor((seconds % 3600) / 60);
  return `${hours}h ${mins}m`;
}

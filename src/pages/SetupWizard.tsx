import { useState, useEffect, useRef } from "react";
import { useNavigate } from "react-router-dom";
import { listen } from "@tauri-apps/api/event";
import { DeckButton, DeckCard } from "../components/deck";
import { useGridNav } from "../hooks/useGridNav";
import {
  checkSteamShortcut,
  registerSteamShortcut,
  checkSyncthingInstalled,
  installSyncthing,
  startSyncthing,
  updateSetting,
} from "../lib/api";

type Step = "gamepad" | "steam" | "syncthing";

const STEPS: Step[] = ["gamepad", "steam", "syncthing"];

export default function SetupWizard() {
  const containerRef = useRef<HTMLDivElement>(null);
  useGridNav(containerRef, 1);
  const navigate = useNavigate();

  const [step, setStep] = useState<Step>("gamepad");
  const [gamepadDetected, setGamepadDetected] = useState<string | null>(null);

  // Steam shortcut
  const [shortcutStatus, setShortcutStatus] = useState<
    "idle" | "checking" | "registering" | "done" | "already" | "error"
  >("idle");
  const [shortcutError, setShortcutError] = useState("");

  // Syncthing
  const [syncInfo, setSyncInfo] = useState<{
    installed: boolean;
    version: string | null;
    managed: boolean;
  } | null>(null);
  const [syncInstalling, setSyncInstalling] = useState(false);
  const [syncStarting, setSyncStarting] = useState(false);
  const [syncError, setSyncError] = useState("");
  const [syncDone, setSyncDone] = useState(false);

  // ── Gamepad detection (via native gilrs events) ─────────────────
  useEffect(() => {
    if (step !== "gamepad") return;
    let unlisten: (() => void) | undefined;

    listen<{ kind: string; name: string; pressed?: boolean }>(
      "gamepad-event",
      (event) => {
        const ev = event.payload;
        // Any event means a gamepad is connected
        setGamepadDetected("Gamepad connected");
        // A button press advances to next step
        if (ev.kind === "button" && ev.name === "A" && ev.pressed) {
          setStep("steam");
        }
      },
    ).then((fn) => {
      unlisten = fn;
    });

    return () => {
      unlisten?.();
    };
  }, [step]);

  // ── Steam shortcut check on step enter ─────────────────────────
  useEffect(() => {
    if (step !== "steam") return;
    setShortcutStatus("checking");
    checkSteamShortcut()
      .then((exists) => {
        setShortcutStatus(exists ? "already" : "idle");
      })
      .catch(() => setShortcutStatus("idle"));
  }, [step]);

  // ── Syncthing check on step enter ──────────────────────────────
  useEffect(() => {
    if (step !== "syncthing") return;
    checkSyncthingInstalled()
      .then((info) => {
        setSyncInfo(info);
        if (info.installed) setSyncDone(true);
      })
      .catch(() => setSyncInfo(null));
  }, [step]);

  const handleRegisterShortcut = async () => {
    setShortcutStatus("registering");
    setShortcutError("");
    try {
      const result = await registerSteamShortcut();
      setShortcutStatus(result.already_existed ? "already" : "done");
    } catch (e) {
      setShortcutError(String(e));
      setShortcutStatus("error");
    }
  };

  const handleInstallSyncthing = async () => {
    setSyncInstalling(true);
    setSyncError("");
    try {
      const result = await installSyncthing();
      setSyncInfo({ installed: true, version: result.version, managed: true });
      // Auto-start so the API key gets generated (needed by Sync tab later)
      try {
        await startSyncthing();
      } catch {
        // Non-fatal — user can start manually from Sync tab
      }
      setSyncDone(true);
    } catch (e) {
      setSyncError(String(e));
    } finally {
      setSyncInstalling(false);
    }
  };

  const handleStartSyncthing = async () => {
    setSyncStarting(true);
    setSyncError("");
    try {
      await startSyncthing();
      setSyncDone(true);
    } catch (e) {
      setSyncError(String(e));
    } finally {
      setSyncStarting(false);
    }
  };

  const handleFinish = async () => {
    await updateSetting("setup_complete", "true");
    navigate("/");
  };

  const handleSkip = () => {
    const idx = STEPS.indexOf(step);
    if (idx < STEPS.length - 1) {
      setStep(STEPS[idx + 1]);
    } else {
      handleFinish();
    }
  };

  const stepIdx = STEPS.indexOf(step) + 1;

  return (
    <div ref={containerRef} className="min-h-screen bg-gray-900 text-gray-100 flex flex-col items-center justify-center p-8">
      <h1 className="text-3xl font-bold mb-2">Welcome to DeckSave</h1>
      <p className="text-gray-400 mb-8">
        Step {stepIdx} of {STEPS.length} — Let's get you set up
      </p>

      <DeckCard className="w-full max-w-lg">
        {/* ── Step 1: Gamepad ─────────────────────────────────── */}
        {step === "gamepad" && (
          <div className="p-6 space-y-4">
            <h2 className="text-xl font-semibold">🎮 Gamepad Check</h2>
            <p className="text-gray-400">
              Connect your controller and press <strong>A</strong> to continue.
            </p>
            {gamepadDetected ? (
              <div className="p-3 bg-emerald-900/30 border border-emerald-700 rounded-lg text-emerald-300">
                ✓ Detected: {gamepadDetected}
              </div>
            ) : (
              <div className="p-3 bg-gray-800 border border-gray-700 rounded-lg text-gray-400 animate-pulse">
                Waiting for gamepad…
              </div>
            )}
            <div className="flex gap-3 pt-2">
              <DeckButton data-deck-focusable onClick={() => setStep("steam")} variant="primary">
                {gamepadDetected ? "Continue" : "Skip — I'm using keyboard"}
              </DeckButton>
            </div>
          </div>
        )}

        {/* ── Step 2: Steam Shortcut ──────────────────────────── */}
        {step === "steam" && (
          <div className="p-6 space-y-4">
            <h2 className="text-xl font-semibold">🎯 Add to Steam</h2>
            <p className="text-gray-400">
              Register DeckSave as a non-Steam game so it appears in Gaming Mode.
            </p>

            {shortcutStatus === "already" && (
              <div className="p-3 bg-emerald-900/30 border border-emerald-700 rounded-lg text-emerald-300">
                ✓ DeckSave is already registered in Steam
              </div>
            )}
            {shortcutStatus === "done" && (
              <div className="p-3 bg-emerald-900/30 border border-emerald-700 rounded-lg text-emerald-300">
                ✓ Added to Steam! Restart Steam to see it in Gaming Mode.
              </div>
            )}
            {shortcutStatus === "error" && (
              <div className="p-3 bg-red-900/30 border border-red-700 rounded-lg text-red-300">
                Error: {shortcutError}
              </div>
            )}
            {shortcutStatus === "checking" && (
              <div className="p-3 bg-gray-800 border border-gray-700 rounded-lg text-gray-400 animate-pulse">
                Checking…
              </div>
            )}

            <div className="flex gap-3 pt-2">
              {shortcutStatus !== "done" && shortcutStatus !== "already" && (
                <DeckButton
                  data-deck-focusable
                  variant="primary"
                  onClick={handleRegisterShortcut}
                  disabled={shortcutStatus === "registering" || shortcutStatus === "checking"}
                >
                  {shortcutStatus === "registering" ? "Registering…" : "Add to Steam"}
                </DeckButton>
              )}
              <DeckButton data-deck-focusable onClick={handleSkip} variant="ghost">
                {shortcutStatus === "done" || shortcutStatus === "already" ? "Continue" : "Skip"}
              </DeckButton>
            </div>
          </div>
        )}

        {/* ── Step 3: Syncthing ───────────────────────────────── */}
        {step === "syncthing" && (
          <div className="p-6 space-y-4">
            <h2 className="text-xl font-semibold">🔄 Syncthing Setup</h2>
            <p className="text-gray-400">
              Syncthing enables peer-to-peer save sync between your devices — no cloud needed.
            </p>

            {syncInfo?.installed && (
              <div className="p-3 bg-emerald-900/30 border border-emerald-700 rounded-lg text-emerald-300">
                ✓ Syncthing {syncInfo.version ?? ""} {syncInfo.managed ? "(managed by DeckSave)" : "(system)"} is ready
              </div>
            )}
            {syncError && (
              <div className="p-3 bg-red-900/30 border border-red-700 rounded-lg text-red-300">
                Error: {syncError}
              </div>
            )}
            {!syncInfo?.installed && !syncInstalling && (
              <div className="p-3 bg-amber-900/30 border border-amber-700 rounded-lg text-amber-300">
                Syncthing is not installed. DeckSave can download and manage it for you.
              </div>
            )}
            {syncInstalling && (
              <div className="p-3 bg-blue-900/30 border border-blue-700 rounded-lg text-blue-300 animate-pulse">
                Downloading Syncthing… This may take a moment.
              </div>
            )}

            <div className="flex gap-3 pt-2">
              {!syncInfo?.installed && !syncDone && (
                <DeckButton
                  data-deck-focusable
                  variant="primary"
                  onClick={handleInstallSyncthing}
                  disabled={syncInstalling}
                >
                  {syncInstalling ? "Installing…" : "Install Syncthing"}
                </DeckButton>
              )}
              {syncInfo?.installed && !syncDone && (
                <DeckButton
                  data-deck-focusable
                  variant="primary"
                  onClick={handleStartSyncthing}
                  disabled={syncStarting}
                >
                  {syncStarting ? "Starting…" : "Start Syncthing"}
                </DeckButton>
              )}
              <DeckButton data-deck-focusable onClick={syncDone ? handleFinish : handleSkip} variant={syncDone ? "primary" : "ghost"}>
                {syncDone ? "Finish Setup" : "Skip"}
              </DeckButton>
            </div>
          </div>
        )}
      </DeckCard>

      {/* Step indicator */}
      <div className="flex gap-2 mt-6">
        {STEPS.map((s, i) => (
          <div
            key={s}
            className={`w-3 h-3 rounded-full transition-colors ${
              i < stepIdx ? "bg-blue-400" : i === stepIdx - 1 ? "bg-blue-400" : "bg-gray-600"
            }`}
          />
        ))}
      </div>
    </div>
  );
}

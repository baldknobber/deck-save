import { useEffect, useRef, useCallback, type RefObject } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

// ── Types for events emitted by the Rust gilrs backend ───────────
interface GamepadButtonEvent {
  kind: "button";
  name: string;
  pressed: boolean;
}

interface GamepadAxisEvent {
  kind: "axis";
  name: string;
  value: number;
}

type GamepadEvent = GamepadButtonEvent | GamepadAxisEvent;

const STICK_DEADZONE = 0.5;
const NAV_REPEAT_MS = 200;

/**
 * Returns the list of focusable elements inside a container.
 */
function getFocusable(container: HTMLElement): HTMLElement[] {
  return Array.from(
    container.querySelectorAll<HTMLElement>("[data-deck-focusable]"),
  );
}

/**
 * Move focus within a flat list of focusable elements by a signed delta.
 * Clamps to bounds and scrolls the newly-focused element into view.
 */
function moveFocus(focusable: HTMLElement[], delta: number): boolean {
  const active = document.activeElement as HTMLElement;
  const idx = focusable.indexOf(active);
  if (idx === -1) return false;
  const next = Math.max(0, Math.min(idx + delta, focusable.length - 1));
  if (next === idx) return false;
  focusable[next].focus();
  focusable[next].scrollIntoView({ block: "nearest", behavior: "smooth" });
  return true;
}

/**
 * Enables arrow-key + native gamepad grid navigation within a container.
 *
 * Keyboard: Arrow keys for navigation (fallback / Desktop Mode).
 *
 * Gamepad: Listens to `gamepad-event` Tauri events emitted by the Rust
 * gilrs backend (reads /dev/input/event* directly via evdev on Linux,
 * WGI on Windows). This bypasses WebKitGTK's missing Gamepad API.
 *
 * D-pad & left-stick for navigation, A to confirm, B to go back.
 * Elements must have the `data-deck-focusable` attribute.
 */
export function useGridNav(
  containerRef: RefObject<HTMLElement | null>,
  columns: number,
) {
  const lastNavTime = useRef(0);

  const navigate = useCallback(
    (dir: "up" | "down" | "left" | "right") => {
      const container = containerRef.current;
      if (!container) return;
      const focusable = getFocusable(container);
      if (focusable.length === 0) return;

      if (!focusable.includes(document.activeElement as HTMLElement)) {
        focusable[0].focus();
        focusable[0].scrollIntoView({ block: "nearest", behavior: "smooth" });
        return;
      }

      const delta =
        dir === "right" ? 1 : dir === "left" ? -1 : dir === "down" ? columns : -columns;
      moveFocus(focusable, delta);
    },
    [containerRef, columns],
  );

  // ── Keyboard handler (fallback) ────────────────────────────────
  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      switch (e.key) {
        case "ArrowRight":
          navigate("right");
          break;
        case "ArrowLeft":
          navigate("left");
          break;
        case "ArrowDown":
          navigate("down");
          break;
        case "ArrowUp":
          navigate("up");
          break;
        default:
          return;
      }
      e.preventDefault();
    };

    container.addEventListener("keydown", handleKeyDown);
    return () => container.removeEventListener("keydown", handleKeyDown);
  }, [containerRef, navigate]);

  // ── Auto-focus first element on mount ──────────────────────────
  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;
    const id = requestAnimationFrame(() => {
      const focusable = getFocusable(container);
      if (
        focusable.length > 0 &&
        !focusable.includes(document.activeElement as HTMLElement)
      ) {
        focusable[0].focus();
      }
    });
    return () => cancelAnimationFrame(id);
  }, [containerRef]);

  // ── Native gamepad events from Rust gilrs backend ──────────────
  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    let unlisten: UnlistenFn | undefined;

    listen<GamepadEvent>("gamepad-event", (event) => {
      const ev = event.payload;
      const now = performance.now();
      const elapsed = now - lastNavTime.current;

      if (ev.kind === "button" && ev.pressed && elapsed >= NAV_REPEAT_MS) {
        let handled = true;
        switch (ev.name) {
          case "DPadUp":
            navigate("up");
            break;
          case "DPadDown":
            navigate("down");
            break;
          case "DPadLeft":
            navigate("left");
            break;
          case "DPadRight":
            navigate("right");
            break;
          case "A": {
            const active = document.activeElement as HTMLElement | null;
            if (active && container.contains(active)) {
              active.click();
            }
            break;
          }
          case "B":
            document.dispatchEvent(
              new KeyboardEvent("keydown", { key: "Escape", bubbles: true }),
            );
            break;
          default:
            handled = false;
        }
        if (handled) lastNavTime.current = now;
      }

      // Left stick navigation
      if (ev.kind === "axis" && elapsed >= NAV_REPEAT_MS) {
        let moved = false;
        if (ev.name === "LeftStickY" && ev.value < -STICK_DEADZONE) {
          navigate("up");
          moved = true;
        } else if (ev.name === "LeftStickY" && ev.value > STICK_DEADZONE) {
          navigate("down");
          moved = true;
        } else if (ev.name === "LeftStickX" && ev.value < -STICK_DEADZONE) {
          navigate("left");
          moved = true;
        } else if (ev.name === "LeftStickX" && ev.value > STICK_DEADZONE) {
          navigate("right");
          moved = true;
        }
        if (moved) lastNavTime.current = now;
      }
    }).then((fn) => {
      unlisten = fn;
    });

    return () => {
      unlisten?.();
    };
  }, [containerRef, navigate]);
}

// ── Shoulder-button hook (L1/R1 tab switching) ───────────────────

const TAB_REPEAT_MS = 300;

/**
 * Listens for L1/R1 gamepad button events and calls `onSwitch(-1 | 1)`.
 * Uses native gilrs events instead of navigator.getGamepads().
 */
export function useShoulderNav(onSwitch: (delta: -1 | 1) => void) {
  const lastTime = useRef(0);
  const cbRef = useRef(onSwitch);
  cbRef.current = onSwitch;

  useEffect(() => {
    let unlisten: UnlistenFn | undefined;

    listen<GamepadEvent>("gamepad-event", (event) => {
      const ev = event.payload;
      if (ev.kind !== "button" || !ev.pressed) return;

      const now = performance.now();
      if (now - lastTime.current < TAB_REPEAT_MS) return;

      if (ev.name === "L1") {
        cbRef.current(-1);
        lastTime.current = now;
      } else if (ev.name === "R1") {
        cbRef.current(1);
        lastTime.current = now;
      }
    }).then((fn) => {
      unlisten = fn;
    });

    return () => {
      unlisten?.();
    };
  }, []);
}

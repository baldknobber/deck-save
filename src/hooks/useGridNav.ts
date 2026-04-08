import { useEffect, useRef, useCallback, type RefObject } from "react";

// ── Gamepad button constants (Standard Gamepad mapping) ──────────
const BTN_A = 0;
const BTN_B = 1;
const DPAD_UP = 12;
const DPAD_DOWN = 13;
const DPAD_LEFT = 14;
const DPAD_RIGHT = 15;

const STICK_DEADZONE = 0.5;
const NAV_REPEAT_MS = 200; // debounce interval for held buttons / stick

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
 * Returns true if focus actually moved.
 */
function moveFocus(
  focusable: HTMLElement[],
  delta: number,
): boolean {
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
 * Enables arrow-key + Gamepad API grid navigation within a container.
 *
 * Keyboard: Arrow keys for navigation (works on desktop / when Steam Input
 * injects key events).
 *
 * Gamepad: D-pad & left-stick for navigation, A to confirm (Enter),
 * B to go back (Escape). Uses requestAnimationFrame polling — the only
 * reliable method inside WebKitGTK / Flatpak where Steam Input keyboard
 * injection doesn't reach the webview.
 *
 * Elements must have the `data-deck-focusable` attribute to be navigable.
 */
export function useGridNav(
  containerRef: RefObject<HTMLElement | null>,
  columns: number,
) {
  const lastNavTime = useRef(0);
  const rafId = useRef(0);

  // ── Shared navigation helper (used by both keyboard + gamepad) ─
  const navigate = useCallback(
    (dir: "up" | "down" | "left" | "right") => {
      const container = containerRef.current;
      if (!container) return;
      const focusable = getFocusable(container);
      if (focusable.length === 0) return;

      // Auto-focus first element if nothing in the container is focused
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

  // ── Keyboard handler (existing behaviour, kept as fallback) ────
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
    // Delay slightly so the DOM has rendered focusable children
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

  // ── Gamepad polling loop ───────────────────────────────────────
  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    const poll = () => {
      const gamepads = navigator.getGamepads?.();
      if (!gamepads) {
        rafId.current = requestAnimationFrame(poll);
        return;
      }

      const gp = gamepads[0] ?? gamepads[1] ?? gamepads[2] ?? gamepads[3];
      if (!gp) {
        rafId.current = requestAnimationFrame(poll);
        return;
      }

      const now = performance.now();
      const elapsed = now - lastNavTime.current;

      // ── D-pad / left-stick navigation (debounced) ──────────
      if (elapsed >= NAV_REPEAT_MS) {
        let moved = false;

        // D-pad buttons
        if (gp.buttons[DPAD_UP]?.pressed) {
          navigate("up");
          moved = true;
        } else if (gp.buttons[DPAD_DOWN]?.pressed) {
          navigate("down");
          moved = true;
        } else if (gp.buttons[DPAD_LEFT]?.pressed) {
          navigate("left");
          moved = true;
        } else if (gp.buttons[DPAD_RIGHT]?.pressed) {
          navigate("right");
          moved = true;
        }

        // Left stick (axes 0 = X, 1 = Y) — only if D-pad didn't fire
        if (!moved) {
          const lx = gp.axes[0] ?? 0;
          const ly = gp.axes[1] ?? 0;
          if (ly < -STICK_DEADZONE) {
            navigate("up");
            moved = true;
          } else if (ly > STICK_DEADZONE) {
            navigate("down");
            moved = true;
          } else if (lx < -STICK_DEADZONE) {
            navigate("left");
            moved = true;
          } else if (lx > STICK_DEADZONE) {
            navigate("right");
            moved = true;
          }
        }

        if (moved) lastNavTime.current = now;
      }

      // ── A button → Enter (confirm / activate) ─────────────
      if (gp.buttons[BTN_A]?.pressed && elapsed >= NAV_REPEAT_MS) {
        const active = document.activeElement as HTMLElement | null;
        if (active && container.contains(active)) {
          active.click();
          lastNavTime.current = now;
        }
      }

      // ── B button → Escape (close modal / go back) ─────────
      if (gp.buttons[BTN_B]?.pressed && elapsed >= NAV_REPEAT_MS) {
        document.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
        lastNavTime.current = now;
      }

      rafId.current = requestAnimationFrame(poll);
    };

    rafId.current = requestAnimationFrame(poll);
    return () => cancelAnimationFrame(rafId.current);
  }, [containerRef, navigate]);
}

// ── Shoulder-button hook (L1/R1 tab switching) ───────────────────
// Separated from useGridNav so Layout.tsx can use it without a grid container.

const BTN_L1 = 4;
const BTN_R1 = 5;
const TAB_REPEAT_MS = 300;

/**
 * Polls gamepad shoulder buttons (L1 / R1) and calls `onSwitch(-1 | 1)`.
 * Intended for switching between tabs in the Layout bottom bar.
 */
export function useShoulderNav(onSwitch: (delta: -1 | 1) => void) {
  const lastTime = useRef(0);
  const rafId = useRef(0);
  const cbRef = useRef(onSwitch);
  cbRef.current = onSwitch;

  useEffect(() => {
    const poll = () => {
      const gamepads = navigator.getGamepads?.();
      const gp = gamepads?.[0] ?? gamepads?.[1] ?? gamepads?.[2] ?? gamepads?.[3];
      if (gp) {
        const now = performance.now();
        if (now - lastTime.current >= TAB_REPEAT_MS) {
          if (gp.buttons[BTN_L1]?.pressed) {
            cbRef.current(-1);
            lastTime.current = now;
          } else if (gp.buttons[BTN_R1]?.pressed) {
            cbRef.current(1);
            lastTime.current = now;
          }
        }
      }
      rafId.current = requestAnimationFrame(poll);
    };
    rafId.current = requestAnimationFrame(poll);
    return () => cancelAnimationFrame(rafId.current);
  }, []);
}

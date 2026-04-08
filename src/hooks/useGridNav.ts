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
 * Returns focusable elements inside a specific zone.
 */
function getZoneFocusable(container: HTMLElement, zone: string): HTMLElement[] {
  const zoneEl = container.querySelector<HTMLElement>(`[data-nav-zone="${zone}"]`);
  if (!zoneEl) return [];
  return Array.from(zoneEl.querySelectorAll<HTMLElement>("[data-deck-focusable]"));
}

/**
 * Get the zone name that the currently focused element belongs to.
 */
function getActiveZone(container: HTMLElement): string | null {
  const active = document.activeElement as HTMLElement;
  if (!active || !container.contains(active)) return null;
  const zone = active.closest("[data-nav-zone]");
  return zone?.getAttribute("data-nav-zone") ?? null;
}

/**
 * Get all zone names in document order within a container.
 */
function getZoneOrder(container: HTMLElement): string[] {
  const zones = container.querySelectorAll<HTMLElement>("[data-nav-zone]");
  return Array.from(zones).map((z) => z.getAttribute("data-nav-zone")!);
}

/**
 * Get the zone type: "horizontal" (top bar, sub-tabs) or "grid" (content).
 */
function getZoneType(container: HTMLElement, zone: string): "horizontal" | "grid" {
  const zoneEl = container.querySelector<HTMLElement>(`[data-nav-zone="${zone}"]`);
  return zoneEl?.getAttribute("data-nav-type") === "grid" ? "grid" : "horizontal";
}

/**
 * Detect column count from a grid zone's CSS grid-template-columns.
 */
function detectGridColumns(container: HTMLElement, zone: string): number {
  const zoneEl = container.querySelector<HTMLElement>(`[data-nav-zone="${zone}"]`);
  if (!zoneEl) return 1;
  const style = getComputedStyle(zoneEl);
  const cols = style.gridTemplateColumns;
  if (!cols || cols === "none") return 1;
  return cols.split(/\s+/).filter(Boolean).length;
}

/**
 * Enables zone-aware D-pad / arrow-key / analog-stick navigation.
 *
 * Mark zones with `data-nav-zone="name"` and `data-nav-type="horizontal"|"grid"`.
 * Horizontal zones: left/right navigates within; down goes to next zone.
 * Grid zones: arrow keys navigate in a grid with dynamically detected columns;
 * up from top row goes to previous zone.
 *
 * Falls back to flat navigation when no zones are defined.
 */
export function useGridNav(
  containerRef: RefObject<HTMLElement | null>,
  fallbackColumns: number,
) {
  const lastNavTime = useRef(0);

  const navigate = useCallback(
    (dir: "up" | "down" | "left" | "right") => {
      const container = containerRef.current;
      if (!container) return;

      const zones = getZoneOrder(container);

      // ── Flat navigation fallback (no zones defined) ──
      if (zones.length === 0) {
        const focusable = getFocusable(container);
        if (focusable.length === 0) return;
        const active = document.activeElement as HTMLElement;
        if (!focusable.includes(active)) {
          focusable[0].focus();
          focusable[0].scrollIntoView({ block: "nearest", behavior: "smooth" });
          return;
        }
        const idx = focusable.indexOf(active);
        const delta =
          dir === "right" ? 1 : dir === "left" ? -1 : dir === "down" ? fallbackColumns : -fallbackColumns;
        const next = Math.max(0, Math.min(idx + delta, focusable.length - 1));
        if (next !== idx) {
          focusable[next].focus();
          focusable[next].scrollIntoView({ block: "nearest", behavior: "smooth" });
        }
        return;
      }

      // ── Zone-aware navigation ──
      const activeZone = getActiveZone(container);

      // Nothing focused yet → focus first element in first zone
      if (!activeZone) {
        for (const z of zones) {
          const items = getZoneFocusable(container, z);
          if (items.length > 0) {
            items[0].focus();
            items[0].scrollIntoView({ block: "nearest", behavior: "smooth" });
            return;
          }
        }
        return;
      }

      const zoneType = getZoneType(container, activeZone);
      const items = getZoneFocusable(container, activeZone);
      const active = document.activeElement as HTMLElement;
      const idx = items.indexOf(active);

      if (zoneType === "horizontal") {
        // Horizontal zone: left/right within, down/up to adjacent zones
        if (dir === "left" && idx > 0) {
          items[idx - 1].focus();
          items[idx - 1].scrollIntoView({ block: "nearest", behavior: "smooth" });
        } else if (dir === "right" && idx < items.length - 1) {
          items[idx + 1].focus();
          items[idx + 1].scrollIntoView({ block: "nearest", behavior: "smooth" });
        } else if (dir === "down") {
          // Move to next zone
          const zIdx = zones.indexOf(activeZone);
          for (let i = zIdx + 1; i < zones.length; i++) {
            const nextItems = getZoneFocusable(container, zones[i]);
            if (nextItems.length > 0) {
              nextItems[0].focus();
              nextItems[0].scrollIntoView({ block: "nearest", behavior: "smooth" });
              return;
            }
          }
        } else if (dir === "up") {
          // Move to previous zone
          const zIdx = zones.indexOf(activeZone);
          for (let i = zIdx - 1; i >= 0; i--) {
            const prevItems = getZoneFocusable(container, zones[i]);
            if (prevItems.length > 0) {
              prevItems[prevItems.length - 1].focus();
              prevItems[prevItems.length - 1].scrollIntoView({ block: "nearest", behavior: "smooth" });
              return;
            }
          }
        }
      } else {
        // Grid zone: navigate in columns
        const columns = detectGridColumns(container, activeZone);
        const row = Math.floor(idx / columns);
        const col = idx % columns;

        if (dir === "right" && idx < items.length - 1) {
          items[idx + 1].focus();
          items[idx + 1].scrollIntoView({ block: "nearest", behavior: "smooth" });
        } else if (dir === "left" && idx > 0) {
          items[idx - 1].focus();
          items[idx - 1].scrollIntoView({ block: "nearest", behavior: "smooth" });
        } else if (dir === "down") {
          const nextIdx = idx + columns;
          if (nextIdx < items.length) {
            items[nextIdx].focus();
            items[nextIdx].scrollIntoView({ block: "nearest", behavior: "smooth" });
          }
          // At bottom of grid — stay put
        } else if (dir === "up") {
          if (row > 0) {
            const prevIdx = idx - columns;
            items[prevIdx].focus();
            items[prevIdx].scrollIntoView({ block: "nearest", behavior: "smooth" });
          } else {
            // Top of grid → go to previous zone
            const zIdx = zones.indexOf(activeZone);
            for (let i = zIdx - 1; i >= 0; i--) {
              const prevItems = getZoneFocusable(container, zones[i]);
              if (prevItems.length > 0) {
                // Try to land on same column position
                const target = Math.min(col, prevItems.length - 1);
                prevItems[target].focus();
                prevItems[target].scrollIntoView({ block: "nearest", behavior: "smooth" });
                return;
              }
            }
          }
        }
      }
    },
    [containerRef, fallbackColumns],
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

// ── Trigger-button hook (L2/R2 sub-tab switching) ────────────────

/**
 * Listens for L2/R2 gamepad button events and calls `onSwitch(-1 | 1)`.
 * Use for sub-tab navigation within a page.
 */
export function useTriggerNav(onSwitch: (delta: -1 | 1) => void) {
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

      if (ev.name === "L2") {
        cbRef.current(-1);
        lastTime.current = now;
      } else if (ev.name === "R2") {
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

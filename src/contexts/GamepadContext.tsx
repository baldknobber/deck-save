import {
  createContext,
  useContext,
  useState,
  useEffect,
  type ReactNode,
} from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

type InputMethod = "gamepad" | "mouse" | "keyboard";

interface GamepadContextValue {
  gamepadConnected: boolean;
  lastInputMethod: InputMethod;
}

const GamepadContext = createContext<GamepadContextValue>({
  gamepadConnected: false,
  lastInputMethod: "mouse",
});

export function useGamepad() {
  return useContext(GamepadContext);
}

export function GamepadProvider({ children }: { children: ReactNode }) {
  const [gamepadConnected, setGamepadConnected] = useState(false);
  const [lastInputMethod, setLastInputMethod] = useState<InputMethod>("mouse");

  // Listen for gamepad events from Rust gilrs backend
  useEffect(() => {
    let unlisten: UnlistenFn | undefined;

    listen<{ kind: string; name?: string; pressed?: boolean; value?: number }>(
      "gamepad-event",
      (event) => {
        const ev = event.payload;
        if (ev.kind === "button" && ev.pressed) {
          setGamepadConnected(true);
          setLastInputMethod("gamepad");
        } else if (ev.kind === "axis" && Math.abs(ev.value ?? 0) > 0.3) {
          setGamepadConnected(true);
          setLastInputMethod("gamepad");
        }
      },
    ).then((fn) => {
      unlisten = fn;
    });

    return () => unlisten?.();
  }, []);

  // Mouse → switch away from gamepad mode
  useEffect(() => {
    const onMouse = () => setLastInputMethod("mouse");
    window.addEventListener("mousemove", onMouse);
    window.addEventListener("mousedown", onMouse);
    return () => {
      window.removeEventListener("mousemove", onMouse);
      window.removeEventListener("mousedown", onMouse);
    };
  }, []);

  // Keyboard → switch to keyboard mode
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (["Shift", "Control", "Alt", "Meta"].includes(e.key)) return;
      setLastInputMethod("keyboard");
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  // Sync data-input attribute on <body> for CSS selectors
  useEffect(() => {
    document.body.setAttribute("data-input", lastInputMethod);
  }, [lastInputMethod]);

  return (
    <GamepadContext.Provider value={{ gamepadConnected, lastInputMethod }}>
      {children}
    </GamepadContext.Provider>
  );
}

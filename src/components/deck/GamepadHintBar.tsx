import { useGamepad } from "../../contexts/GamepadContext";
import { useLocation } from "react-router-dom";

function Glyph({ children }: { children: React.ReactNode }) {
  return (
    <span className="inline-flex items-center justify-center min-w-[24px] h-6 px-1.5 rounded bg-gray-700 text-gray-200 text-[11px] font-bold border border-gray-600 leading-none">
      {children}
    </span>
  );
}

function Hint({ glyph, label }: { glyph: string; label: string }) {
  return (
    <span className="inline-flex items-center gap-1.5 text-xs text-gray-400">
      <Glyph>{glyph}</Glyph>
      {label}
    </span>
  );
}

export default function GamepadHintBar() {
  const { lastInputMethod } = useGamepad();
  const location = useLocation();

  if (lastInputMethod !== "gamepad") return null;

  const hasSubTabs = location.pathname === "/sync";

  return (
    <div className="flex-shrink-0 bg-gray-800/90 border-t border-gray-700/50 px-4 py-1.5 flex items-center justify-center gap-5">
      <Hint glyph="D-Pad" label="Navigate" />
      <Hint glyph="A" label="Select" />
      <Hint glyph="B" label="Back" />
      <span className="inline-flex items-center gap-1 text-xs text-gray-400">
        <Glyph>L1</Glyph>
        <Glyph>R1</Glyph>
        <span className="ml-0.5">Tab</span>
      </span>
      {hasSubTabs && (
        <span className="inline-flex items-center gap-1 text-xs text-gray-400">
          <Glyph>L2</Glyph>
          <Glyph>R2</Glyph>
          <span className="ml-0.5">Sub-Tab</span>
        </span>
      )}
    </div>
  );
}

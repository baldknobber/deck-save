import { useGamepad } from "../../contexts/GamepadContext";
import { useLocation } from "react-router-dom";

// Steam Deck/Xbox colored face button glyphs
function AButton() {
  return (
    <span className="inline-flex items-center justify-center w-6 h-6 rounded-full bg-green-600 text-white text-[11px] font-bold leading-none border border-green-500">
      A
    </span>
  );
}

function BButton() {
  return (
    <span className="inline-flex items-center justify-center w-6 h-6 rounded-full bg-red-600 text-white text-[11px] font-bold leading-none border border-red-500">
      B
    </span>
  );
}

function ShoulderGlyph({ children }: { children: React.ReactNode }) {
  return (
    <span className="inline-flex items-center justify-center min-w-[28px] h-6 px-1 rounded bg-gray-600 text-gray-200 text-[10px] font-bold border border-gray-500 leading-none">
      {children}
    </span>
  );
}

function DPadGlyph() {
  return (
    <span className="inline-flex items-center justify-center min-w-[28px] h-6 px-1 rounded bg-gray-600 text-gray-200 text-[10px] font-bold border border-gray-500 leading-none">
      D-Pad
    </span>
  );
}

function Hint({ glyph, label }: { glyph: React.ReactNode; label: string }) {
  return (
    <span className="inline-flex items-center gap-1 text-xs text-gray-400">
      {glyph}
      <span>{label}</span>
    </span>
  );
}

export default function GamepadHintBar() {
  const { lastInputMethod } = useGamepad();
  const location = useLocation();

  if (lastInputMethod !== "gamepad") return null;

  const hasSubTabs = location.pathname === "/sync";

  return (
    <div className="flex items-center gap-4">
      <Hint glyph={<DPadGlyph />} label="Navigate" />
      <Hint glyph={<AButton />} label="Select" />
      <Hint glyph={<BButton />} label="Back" />
      <span className="inline-flex items-center gap-1 text-xs text-gray-400">
        <ShoulderGlyph>L1</ShoulderGlyph>
        <ShoulderGlyph>R1</ShoulderGlyph>
        <span className="ml-0.5">Tab</span>
      </span>
      {hasSubTabs && (
        <span className="inline-flex items-center gap-1 text-xs text-gray-400">
          <ShoulderGlyph>L2</ShoulderGlyph>
          <ShoulderGlyph>R2</ShoulderGlyph>
          <span className="ml-0.5">Sub-Tab</span>
        </span>
      )}
    </div>
  );
}

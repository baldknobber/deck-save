interface DeckProgressBarProps {
  current: number;
  total: number;
  label?: string;
  className?: string;
}

export default function DeckProgressBar({ current, total, label, className = "" }: DeckProgressBarProps) {
  const pct = total > 0 ? Math.round((current / total) * 100) : 0;

  return (
    <div className={`w-full ${className}`}>
      {label && (
        <div className="flex items-center justify-between mb-1">
          <span className="text-sm text-gray-300 truncate">{label}</span>
          <span className="text-xs text-gray-500 ml-2 flex-shrink-0">
            {current}/{total} ({pct}%)
          </span>
        </div>
      )}
      <div className="w-full h-2 bg-gray-700 rounded-full overflow-hidden">
        <div
          className="h-full bg-blue-500 rounded-full transition-all duration-300 ease-out"
          style={{ width: `${pct}%` }}
        />
      </div>
    </div>
  );
}

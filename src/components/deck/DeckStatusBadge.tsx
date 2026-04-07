const statusConfig = {
  backed_up: {
    label: "Backed Up",
    className: "bg-emerald-900/60 text-emerald-300 border-emerald-800",
    dot: "bg-emerald-400",
  },
  changed: {
    label: "Changed",
    className: "bg-amber-900/60 text-amber-300 border-amber-800",
    dot: "bg-amber-400 animate-pulse",
  },
  never_backed_up: {
    label: "Never Backed Up",
    className: "bg-gray-700/60 text-gray-400 border-gray-600",
    dot: "bg-gray-500",
  },
};

interface DeckStatusBadgeProps {
  status: string;
}

export default function DeckStatusBadge({ status }: DeckStatusBadgeProps) {
  const config =
    statusConfig[status as keyof typeof statusConfig] ??
    statusConfig.never_backed_up;

  return (
    <span
      className={`inline-flex items-center gap-2 px-3 py-1.5 rounded-lg text-sm font-medium border ${config.className}`}
    >
      <span className={`w-2 h-2 rounded-full ${config.dot}`} />
      {config.label}
    </span>
  );
}

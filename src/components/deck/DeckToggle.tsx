interface DeckToggleProps {
  checked: boolean;
  onChange: (checked: boolean) => void;
  label: string;
  description?: string;
}

export default function DeckToggle({
  checked,
  onChange,
  label,
  description,
}: DeckToggleProps) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      data-deck-focusable
      onClick={() => onChange(!checked)}
      className="flex items-center justify-between w-full min-h-deck px-4 py-3 bg-gray-800 rounded-xl border-2 border-gray-700 hover:border-gray-600 transition-colors"
    >
      <div className="text-left mr-4">
        <p className="text-base font-medium text-gray-100">{label}</p>
        {description && (
          <p className="text-sm text-gray-400 mt-0.5">{description}</p>
        )}
      </div>
      <div
        className={`relative flex-shrink-0 w-14 h-8 rounded-full transition-colors ${
          checked ? "bg-blue-600" : "bg-gray-600"
        }`}
      >
        <span
          className={`absolute top-1 left-1 w-6 h-6 bg-white rounded-full transition-transform shadow-sm ${
            checked ? "translate-x-6" : ""
          }`}
        />
      </div>
    </button>
  );
}

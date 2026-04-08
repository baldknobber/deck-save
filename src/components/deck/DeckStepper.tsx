import { forwardRef, type InputHTMLAttributes } from "react";

interface DeckStepperProps extends Omit<InputHTMLAttributes<HTMLInputElement>, "onChange"> {
  label?: string;
  value: number;
  onChange: (value: number) => void;
  min?: number;
  max?: number;
  step?: number;
}

const DeckStepper = forwardRef<HTMLInputElement, DeckStepperProps>(
  ({ label, value, onChange, min = 1, max = 50, step = 1, className = "", ...props }, ref) => {
    const decrement = () => {
      const next = Math.max(min, value - step);
      onChange(next);
    };

    const increment = () => {
      const next = Math.min(max, value + step);
      onChange(next);
    };

    return (
      <div className="w-full">
        {label && (
          <label className="block text-sm font-medium text-gray-300 mb-2">
            {label}
          </label>
        )}
        <div className="flex items-center gap-2">
          <button
            type="button"
            data-deck-focusable
            onClick={decrement}
            disabled={value <= min}
            className="min-h-deck min-w-deck flex items-center justify-center rounded-xl border-2 border-gray-700 bg-gray-800 text-gray-200 text-xl font-bold hover:border-gray-600 transition-colors disabled:opacity-30 disabled:cursor-not-allowed"
          >
            −
          </button>
          <input
            ref={ref}
            type="number"
            data-deck-focusable
            value={value}
            onChange={(e) => {
              const n = Number(e.target.value);
              if (!isNaN(n)) onChange(Math.max(min, Math.min(max, n)));
            }}
            min={min}
            max={max}
            className={`
              w-20 min-h-deck bg-gray-800 border-2 border-gray-700 rounded-xl
              px-3 py-3 text-base text-center text-gray-100 font-medium
              hover:border-gray-600 transition-colors
              [appearance:textfield] [&::-webkit-outer-spin-button]:appearance-none [&::-webkit-inner-spin-button]:appearance-none
              ${className}
            `}
            {...props}
          />
          <button
            type="button"
            data-deck-focusable
            onClick={increment}
            disabled={value >= max}
            className="min-h-deck min-w-deck flex items-center justify-center rounded-xl border-2 border-gray-700 bg-gray-800 text-gray-200 text-xl font-bold hover:border-gray-600 transition-colors disabled:opacity-30 disabled:cursor-not-allowed"
          >
            +
          </button>
        </div>
      </div>
    );
  },
);

DeckStepper.displayName = "DeckStepper";
export default DeckStepper;

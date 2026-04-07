import { forwardRef, type InputHTMLAttributes, type ReactNode } from "react";

interface DeckInputProps extends InputHTMLAttributes<HTMLInputElement> {
  label?: string;
  icon?: ReactNode;
}

const DeckInput = forwardRef<HTMLInputElement, DeckInputProps>(
  ({ label, icon, className = "", ...props }, ref) => {
    return (
      <div className="w-full">
        {label && (
          <label className="block text-sm font-medium text-gray-300 mb-2">
            {label}
          </label>
        )}
        <div className="relative">
          {icon && (
            <span className="absolute left-4 top-1/2 -translate-y-1/2 text-gray-500">
              {icon}
            </span>
          )}
          <input
            ref={ref}
            data-deck-focusable
            className={`
              w-full min-h-deck bg-gray-800 border-2 border-gray-700 rounded-xl
              px-4 py-3 text-base text-gray-100 placeholder-gray-500
              hover:border-gray-600 transition-colors
              ${icon ? "pl-11" : ""}
              ${className}
            `}
            {...props}
          />
        </div>
      </div>
    );
  },
);

DeckInput.displayName = "DeckInput";
export default DeckInput;

import { forwardRef, type HTMLAttributes, type ReactNode } from "react";

interface DeckCardProps extends HTMLAttributes<HTMLDivElement> {
  selected?: boolean;
  children: ReactNode;
}

const DeckCard = forwardRef<HTMLDivElement, DeckCardProps>(
  ({ selected = false, className = "", children, ...props }, ref) => {
    return (
      <div
        ref={ref}
        tabIndex={0}
        data-deck-focusable
        className={`
          bg-gray-800 rounded-xl border-2 p-4 transition-all cursor-pointer
          hover:bg-gray-750
          ${
            selected
              ? "border-blue-500 bg-gray-800/80 shadow-lg shadow-blue-500/10"
              : "border-gray-700 hover:border-gray-600"
          }
          ${className}
        `}
        {...props}
      >
        {children}
      </div>
    );
  },
);

DeckCard.displayName = "DeckCard";
export default DeckCard;

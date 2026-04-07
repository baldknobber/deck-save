import { forwardRef, type ButtonHTMLAttributes, type ReactNode } from "react";

const variants = {
  primary: "bg-blue-600 hover:bg-blue-500 active:bg-blue-700 text-white",
  secondary: "bg-gray-700 hover:bg-gray-600 active:bg-gray-800 text-gray-100",
  danger: "bg-red-600 hover:bg-red-500 active:bg-red-700 text-white",
  success: "bg-emerald-600 hover:bg-emerald-500 active:bg-emerald-700 text-white",
  ghost: "bg-transparent hover:bg-gray-800 active:bg-gray-700 text-gray-300",
};

const sizes = {
  sm: "min-h-[40px] px-4 text-sm rounded-lg gap-2",
  md: "min-h-deck px-5 text-base rounded-xl gap-2.5",
  lg: "min-h-[56px] px-6 text-lg rounded-xl gap-3",
};

interface DeckButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: keyof typeof variants;
  size?: keyof typeof sizes;
  loading?: boolean;
  icon?: ReactNode;
  fullWidth?: boolean;
}

const DeckButton = forwardRef<HTMLButtonElement, DeckButtonProps>(
  (
    {
      variant = "primary",
      size = "md",
      loading = false,
      icon,
      fullWidth = false,
      className = "",
      disabled,
      children,
      ...props
    },
    ref,
  ) => {
    return (
      <button
        ref={ref}
        disabled={disabled || loading}
        data-deck-focusable
        className={`
          inline-flex items-center justify-center font-medium transition-colors
          disabled:opacity-40 disabled:pointer-events-none
          ${variants[variant]} ${sizes[size]}
          ${fullWidth ? "w-full" : ""}
          ${className}
        `}
        {...props}
      >
        {loading ? (
          <svg
            className="animate-spin h-5 w-5"
            viewBox="0 0 24 24"
            fill="none"
          >
            <circle
              className="opacity-25"
              cx="12"
              cy="12"
              r="10"
              stroke="currentColor"
              strokeWidth="4"
            />
            <path
              className="opacity-75"
              fill="currentColor"
              d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"
            />
          </svg>
        ) : icon ? (
          <span className="flex-shrink-0">{icon}</span>
        ) : null}
        {children}
      </button>
    );
  },
);

DeckButton.displayName = "DeckButton";
export default DeckButton;

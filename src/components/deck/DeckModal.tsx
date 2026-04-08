import { useEffect, useRef, type ReactNode } from "react";
import { useGridNav } from "../../hooks/useGridNav";

interface DeckModalProps {
  open: boolean;
  onClose: () => void;
  title: string;
  children: ReactNode;
}

export default function DeckModal({
  open,
  onClose,
  title,
  children,
}: DeckModalProps) {
  const modalRef = useRef<HTMLDivElement>(null);
  useGridNav(modalRef, 1);

  // Trap focus inside modal & close on Escape (B button on Deck)
  useEffect(() => {
    if (!open) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        onClose();
      }
    };

    document.addEventListener("keydown", handleKeyDown);

    // Focus first focusable element in modal
    const timer = setTimeout(() => {
      const focusable = modalRef.current?.querySelector<HTMLElement>(
        "[data-deck-focusable], button, input, select, a",
      );
      focusable?.focus();
    }, 50);

    return () => {
      document.removeEventListener("keydown", handleKeyDown);
      clearTimeout(timer);
    };
  }, [open, onClose]);

  if (!open) return null;

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center animate-fade-in"
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      {/* Backdrop */}
      <div className="absolute inset-0 bg-black/70" />

      {/* Modal panel */}
      <div
        ref={modalRef}
        role="dialog"
        aria-modal="true"
        aria-label={title}
        className="relative z-10 w-full max-w-xl mx-4 bg-gray-800 border-2 border-gray-700 rounded-2xl shadow-2xl animate-slide-up max-h-[80vh] flex flex-col"
      >
        {/* Header */}
        <div className="flex items-center justify-between px-6 py-4 border-b border-gray-700">
          <h2 className="text-xl font-bold text-white">{title}</h2>
          <button
            onClick={onClose}
            data-deck-focusable
            className="min-w-deck min-h-[40px] flex items-center justify-center rounded-lg text-gray-400 hover:text-white hover:bg-gray-700 transition-colors"
            aria-label="Close"
          >
            <svg
              width="20"
              height="20"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2.5"
            >
              <path d="M18 6L6 18M6 6l12 12" />
            </svg>
          </button>
        </div>

        {/* Content */}
        <div className="px-6 py-4 overflow-y-auto flex-1">{children}</div>
      </div>
    </div>
  );
}

import { useEffect, type RefObject } from "react";

/**
 * Enables arrow-key grid navigation within a container.
 * Works with Steam Deck D-pad (mapped to arrow keys via Steam Input).
 *
 * Elements must have `data-deck-focusable` attribute to be navigable.
 */
export function useGridNav(
  containerRef: RefObject<HTMLElement | null>,
  columns: number,
) {
  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      const focusable = Array.from(
        container.querySelectorAll<HTMLElement>("[data-deck-focusable]"),
      );
      const active = document.activeElement as HTMLElement;
      const currentIndex = focusable.indexOf(active);
      if (currentIndex === -1) return;

      let nextIndex = currentIndex;
      switch (e.key) {
        case "ArrowRight":
          nextIndex = Math.min(currentIndex + 1, focusable.length - 1);
          break;
        case "ArrowLeft":
          nextIndex = Math.max(currentIndex - 1, 0);
          break;
        case "ArrowDown":
          nextIndex = Math.min(
            currentIndex + columns,
            focusable.length - 1,
          );
          break;
        case "ArrowUp":
          nextIndex = Math.max(currentIndex - columns, 0);
          break;
        default:
          return;
      }

      if (nextIndex !== currentIndex) {
        e.preventDefault();
        focusable[nextIndex].focus();
        focusable[nextIndex].scrollIntoView({
          block: "nearest",
          behavior: "smooth",
        });
      }
    };

    container.addEventListener("keydown", handleKeyDown);
    return () => container.removeEventListener("keydown", handleKeyDown);
  }, [containerRef, columns]);
}

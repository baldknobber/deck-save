import { Outlet, NavLink, useNavigate, useLocation } from "react-router-dom";
import { useCallback } from "react";
import { useShoulderNav } from "../hooks/useGridNav";
import GamepadHintBar from "./deck/GamepadHintBar";

const TAB_PATHS = ["/", "/sync", "/settings"] as const;

function NavIcon({ icon }: { icon: "games" | "sync" | "settings" }) {
  switch (icon) {
    case "games":
      return (
        <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <rect x="2" y="6" width="20" height="12" rx="2" />
          <path d="M6 12h4M8 10v4" />
          <circle cx="15" cy="11" r="1" />
          <circle cx="18" cy="13" r="1" />
        </svg>
      );
    case "sync":
      return (
        <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <path d="M21 2v6h-6" />
          <path d="M3 12a9 9 0 0115-6.7L21 8" />
          <path d="M3 22v-6h6" />
          <path d="M21 12a9 9 0 01-15 6.7L3 16" />
        </svg>
      );
    case "settings":
      return (
        <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <path d="M12.22 2h-.44a2 2 0 00-2 2v.18a2 2 0 01-1 1.73l-.43.25a2 2 0 01-2 0l-.15-.08a2 2 0 00-2.73.73l-.22.38a2 2 0 00.73 2.73l.15.1a2 2 0 011 1.72v.51a2 2 0 01-1 1.74l-.15.09a2 2 0 00-.73 2.73l.22.38a2 2 0 002.73.73l.15-.08a2 2 0 012 0l.43.25a2 2 0 011 1.73V20a2 2 0 002 2h.44a2 2 0 002-2v-.18a2 2 0 011-1.73l.43-.25a2 2 0 012 0l.15.08a2 2 0 002.73-.73l.22-.39a2 2 0 00-.73-2.73l-.15-.08a2 2 0 01-1-1.74v-.5a2 2 0 011-1.74l.15-.09a2 2 0 00.73-2.73l-.22-.38a2 2 0 00-2.73-.73l-.15.08a2 2 0 01-2 0l-.43-.25a2 2 0 01-1-1.73V4a2 2 0 00-2-2z" />
          <circle cx="12" cy="12" r="3" />
        </svg>
      );
  }
}

const tabs = [
  { to: "/", label: "Games", icon: "games" as const, end: true },
  { to: "/sync", label: "Sync", icon: "sync" as const, end: false },
  { to: "/settings", label: "Settings", icon: "settings" as const, end: false },
];

export default function Layout() {
  const navigate = useNavigate();
  const location = useLocation();

  const handleShoulderSwitch = useCallback(
    (delta: -1 | 1) => {
      const idx = TAB_PATHS.indexOf(location.pathname as typeof TAB_PATHS[number]);
      const cur = idx === -1 ? 0 : idx;
      const next = (cur + delta + TAB_PATHS.length) % TAB_PATHS.length;
      navigate(TAB_PATHS[next]);
    },
    [location.pathname, navigate],
  );

  useShoulderNav(handleShoulderSwitch);

  return (
    <div className="h-screen bg-gray-900 text-gray-100 flex flex-col overflow-hidden">
      {/* Header — compact, informational */}
      <header className="flex-shrink-0 bg-gray-800/50 border-b border-gray-800 px-6 py-2.5 flex items-center">
        <h1 className="text-lg font-bold text-white tracking-tight">DeckSave</h1>
      </header>

      {/* Main content — scrollable */}
      <main className="flex-1 overflow-y-auto p-5">
        <Outlet />
      </main>

      {/* Gamepad button hints — visible only when gamepad is active input */}
      <GamepadHintBar />

      {/* Bottom tab bar — 60px, thumb-friendly for Deck */}
      <nav className="flex-shrink-0 bg-gray-800 border-t border-gray-700 flex" role="tablist">
        {tabs.map((tab) => (
          <NavLink
            key={tab.to}
            to={tab.to}
            end={tab.end}
            role="tab"
            data-deck-focusable
            className={({ isActive }) =>
              `flex-1 flex flex-col items-center justify-center gap-1 min-h-[60px] transition-colors ${
                isActive
                  ? "text-blue-400 bg-gray-700/50"
                  : "text-gray-500 hover:text-gray-300 hover:bg-gray-800"
              }`
            }
          >
            <NavIcon icon={tab.icon} />
            <span className="text-xs font-medium">{tab.label}</span>
          </NavLink>
        ))}
      </nav>
    </div>
  );
}

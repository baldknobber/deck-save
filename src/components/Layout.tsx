import { Outlet, NavLink } from "react-router-dom";

export default function Layout() {
  return (
    <div className="min-h-screen bg-gray-900 text-gray-100 flex flex-col">
      <nav className="bg-gray-800 border-b border-gray-700 px-6 py-3 flex items-center gap-6">
        <h1 className="text-xl font-bold text-white mr-4">DeckSave</h1>
        <NavLink
          to="/"
          end
          className={({ isActive }) =>
            isActive
              ? "text-blue-400 font-medium"
              : "text-gray-400 hover:text-gray-200"
          }
        >
          Games
        </NavLink>
        <NavLink
          to="/sync"
          className={({ isActive }) =>
            isActive
              ? "text-blue-400 font-medium"
              : "text-gray-400 hover:text-gray-200"
          }
        >
          Sync
        </NavLink>
        <NavLink
          to="/settings"
          className={({ isActive }) =>
            isActive
              ? "text-blue-400 font-medium"
              : "text-gray-400 hover:text-gray-200"
          }
        >
          Settings
        </NavLink>
      </nav>
      <main className="flex-1 p-6">
        <Outlet />
      </main>
    </div>
  );
}

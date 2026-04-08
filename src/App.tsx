import { BrowserRouter, Routes, Route, Navigate } from "react-router-dom";
import { useState, useEffect } from "react";
import Layout from "./components/Layout";
import Dashboard from "./pages/Dashboard";
import Settings from "./pages/Settings";
import SyncWizard from "./pages/SyncWizard";
import SetupWizard from "./pages/SetupWizard";
import { getSettings } from "./lib/api";
import { GamepadProvider } from "./contexts/GamepadContext";
import { SetupContext } from "./contexts/SetupContext";

function App() {
  const [ready, setReady] = useState(false);
  const [needsSetup, setNeedsSetup] = useState(false);

  useEffect(() => {
    getSettings()
      .then((settings) => {
        const setupDone = settings.some(
          (s) => s.key === "setup_complete" && s.value === "true",
        );
        setNeedsSetup(!setupDone);
      })
      .catch(() => setNeedsSetup(true))
      .finally(() => setReady(true));
  }, []);

  if (!ready) return null;

  return (
    <SetupContext.Provider value={{ setNeedsSetup }}>
      <GamepadProvider>
        <BrowserRouter>
          <Routes>
            <Route path="/setup" element={<SetupWizard />} />
            <Route path="/" element={<Layout />}>
              <Route
                index
                element={
                  needsSetup ? <Navigate to="/setup" replace /> : <Dashboard />
                }
              />
              <Route path="settings" element={<Settings />} />
              <Route path="sync" element={<SyncWizard />} />
            </Route>
          </Routes>
        </BrowserRouter>
      </GamepadProvider>
    </SetupContext.Provider>
  );
}

export default App;

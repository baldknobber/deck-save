import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./index.css";

class ErrorBoundary extends React.Component<
  { children: React.ReactNode },
  { error: Error | null }
> {
  state = { error: null as Error | null };

  static getDerivedStateFromError(error: Error) {
    return { error };
  }

  render() {
    if (this.state.error) {
      return (
        <div style={{ color: "#fff", background: "#1e1e2e", padding: "2rem", fontFamily: "sans-serif" }}>
          <h1>DeckSave — Render Error</h1>
          <p>The UI crashed. This may be a bug — please report it.</p>
          <pre style={{ background: "#000", padding: "1rem", borderRadius: "8px", overflow: "auto", maxHeight: "60vh" }}>
            {this.state.error.message}
            {"\n\n"}
            {this.state.error.stack}
          </pre>
          <p style={{ marginTop: "1rem" }}>
            <strong>github.com/baldknobber/deck-save/issues</strong>
          </p>
        </div>
      );
    }
    return this.props.children;
  }
}

// Hide the startup fallback timer since React loaded
const fb = document.getElementById("startup-fallback");
if (fb) fb.remove();

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <ErrorBoundary>
      <App />
    </ErrorBoundary>
  </React.StrictMode>,
);

import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./styles.css";

async function boot() {
  if (import.meta.env.DEV && (!("__TAURI_INTERNALS__" in window) || location.search.includes("mock"))) {
    (await import("./dev-mock")).installMock();
  }
  ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
      <App />
    </React.StrictMode>,
  );
}

void boot();

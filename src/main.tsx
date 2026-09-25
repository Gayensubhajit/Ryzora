import "./index.css";
import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { homeDir } from "@tauri-apps/api/path";
import { invoke } from "@tauri-apps/api/core";

// Eagerly resolve home dir and media server port before React mounts.
// The media server port is essential for local video preview via HTTP (asset://
// does not forward Range headers on Linux/WebKitGTK, causing large video fails).
async function init() {
  if (typeof window === "undefined") return;
  try {
    const [home, port] = await Promise.all([
      homeDir(),
      invoke<number>("get_media_server_port"),
    ]);
    (window as any).__RYZORA_HOME__ = home;
    (window as any).__RYZORA_MEDIA_PORT__ = port;
  } catch {
    // Fallback: resolve individually so a single failure doesn't block
    try { (window as any).__RYZORA_HOME__ = await homeDir(); } catch {}
  }
}

// Await init so __RYZORA_MEDIA_PORT__ is set before any PackageItem is created
init().finally(() => {
  ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
      <App />
    </React.StrictMode>,
  );
});

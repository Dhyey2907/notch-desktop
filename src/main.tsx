import React from "react";
import ReactDOM from "react-dom/client";

import App from "./App";
import { connectEventBus } from "./core/eventBus";
import "./design/tokens.css";
import "./index.css";
import { useSettings } from "./settings";
import { GrabApp } from "./grab/GrabApp";
import { SettingsApp } from "./settings/SettingsApp";

// Start listening before the first render so no backend event is missed.
connectEventBus().catch((error) => console.error("failed to connect event bus", error));
useSettings
  .getState()
  .load()
  .catch((error) => console.error("failed to load settings", error));

// One bundle serves both windows: the island (default) and the settings window (#settings).
const isSettingsWindow = window.location.hash.startsWith("#settings");
// The screen text grabber's selection window (#grab).
const isGrabWindow = window.location.hash.startsWith("#grab");
if (isSettingsWindow) document.documentElement.classList.add("settings-window");
if (isGrabWindow) document.documentElement.classList.add("grab-window");

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    {isSettingsWindow ? <SettingsApp /> : isGrabWindow ? <GrabApp /> : <App />}
  </React.StrictMode>,
);

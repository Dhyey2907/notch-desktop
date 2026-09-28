import React from "react";
import ReactDOM from "react-dom/client";

import App from "./App";
import { connectEventBus } from "./core/eventBus";

// Start listening before the first render so no backend event is missed.
connectEventBus().catch((error) => console.error("failed to connect event bus", error));

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);

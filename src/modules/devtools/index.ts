// Developer: local dev servers (Open, Copy URL, Stop; a note when one stops), coding time per
// project with a streak, a quick JavaScript/Python runner, and your own widgets.
import { Code2 } from "lucide-react";
import { lazy } from "react";

import { ipc } from "../../core/ipc";
import type { ModuleContext, NotchModule } from "../types";
import { useDevTools } from "./devStore";

function init({ events, activities }: ModuleContext) {
  const offServers = events.on("DevServersChanged", ({ event }) =>
    useDevTools.setState({ servers: event.payload }),
  );
  const offStopped = events.on("DevServerStopped", ({ event }) =>
    activities.push({
      moduleId: "dev",
      priority: "normal",
      tone: "warning",
      dedupKey: `dev-stopped-${event.payload.port}`,
      countRepeats: false,
      title: `localhost:${event.payload.port} stopped`,
      body: `${event.payload.name} isn't running any more.`,
    }),
  );
  ipc.devServers().then(
    (servers) => useDevTools.setState({ servers }),
    () => {},
  );
  return () => {
    offServers();
    offStopped();
  };
}

export const devToolsModule: NotchModule = {
  id: "dev",
  name: "Developer",
  icon: Code2,
  order: 32,
  init,
  Expanded: lazy(() => import("./DevExpanded")),
};

// Git module (plan.md Phase 8): repository telemetry and confirmed Git actions.
import { GitBranch } from "lucide-react";
import { lazy } from "react";

import { ipc } from "../../core/ipc";
import type { ModuleContext, NotchModule } from "../types";
import { GitCompact, GitPeek } from "./GitIndicators";
import { useGit } from "./gitStore";
import { bindActivities } from "./notify";

function init({ events, activities }: ModuleContext) {
  bindActivities(activities);
  const off = events.on("GitChanged", ({ event }) => useGit.getState().set(event.payload));
  ipc
    .gitGetSnapshot()
    .then((snapshot) => {
      if (!useGit.getState().snapshot) useGit.getState().set(snapshot);
    })
    .catch((error) => console.error("failed to read Git state", error));
  return () => {
    off();
    bindActivities(null);
  };
}

export const gitModule: NotchModule = {
  id: "git",
  name: "Git",
  icon: GitBranch,
  order: 30,
  init,
  Compact: GitCompact,
  Peek: GitPeek,
  Expanded: lazy(() => import("./GitExpanded")),
};

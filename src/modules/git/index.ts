// Git module (plan.md Phase 8): repository telemetry and confirmed Git actions.
import { GitBranch } from "lucide-react";
import { lazy } from "react";

import { ipc } from "../../core/ipc";
import type { ModuleContext, NotchModule } from "../types";
import { GitCompact, GitPeek } from "./GitIndicators";
import { useGit } from "./gitStore";
import { alertCard, shouldAlert } from "./githubAlerts";
import { useGithub } from "./githubStore";
import { useSettings } from "../../settings";
import { bindActivities } from "./notify";

function init({ events, activities }: ModuleContext) {
  bindActivities(activities);
  const offInbox = events.on("GithubNotificationsChanged", ({ event }) =>
    useGithub.getState().setNotifications(event.payload),
  );
  const offAlert = events.on("GithubAlert", ({ event }) => {
    const n = event.payload;
    if (!shouldAlert(n.kind, useSettings.getState().settings?.githubAlerts)) return;
    activities.push(alertCard(n, { open: ipc.openExternal, markRead: ipc.githubMarkRead }));
  });
  ipc
    .githubNotifications()
    .then((list) => {
      if (useGithub.getState().notifications.length === 0) useGithub.getState().setNotifications(list);
    })
    .catch(() => {});
  const off = events.on("GitChanged", ({ event }) => useGit.getState().set(event.payload));
  const offGithub = events.on("GithubChanged", ({ event }) => useGithub.getState().set(event.payload));
  ipc
    .githubStatus()
    .then((status) => {
      if (!useGithub.getState().status) useGithub.getState().set(status);
    })
    .catch(() => {});
  ipc
    .gitGetSnapshot()
    .then((snapshot) => {
      if (!useGit.getState().snapshot) useGit.getState().set(snapshot);
    })
    .catch((error) => console.error("failed to read Git state", error));
  return () => {
    off();
    offGithub();
    offInbox();
    offAlert();
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

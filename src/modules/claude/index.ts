// Claude Code module (plan.md Phase 9): live agent activity and local usage estimates.
import { Sparkles } from "lucide-react";
import { lazy } from "react";

import type { ClaudeStatus } from "../../core/generated/ClaudeStatus";
import { ipc } from "../../core/ipc";
import type { ModuleContext, NotchModule } from "../types";
import { ClaudeCompact, ClaudePeek } from "./ClaudeIndicators";
import { useClaude } from "./claudeStore";

function init({ events, activities }: ModuleContext) {
  // Sessions we've already alerted for while they wait, so one wait = one alert.
  const alerted = new Map<string, string>();

  const apply = (status: ClaudeStatus) => {
    useClaude.getState().setStatus(status);
    const waiting = new Set<string>();
    for (const session of status.live) {
      if (session.state !== "waiting") continue;
      waiting.add(session.sessionId);
      if (alerted.has(session.sessionId)) continue;
      const id = activities.push({
        moduleId: "claude",
        priority: "high",
        tone: "warning",
        dedupKey: `claude-waiting-${session.sessionId}`,
        title: "Claude Code needs you",
        body: session.project ? `Waiting for your input in ${session.project}` : "Waiting for your input",
        persistent: true,
        actions: [{ id: "dismiss", label: "Dismiss", run: () => {} }],
      });
      alerted.set(session.sessionId, id);
    }
    // Clear alerts for sessions that are no longer waiting.
    for (const [sessionId, id] of alerted) {
      if (!waiting.has(sessionId)) {
        activities.dismiss(id);
        alerted.delete(sessionId);
      }
    }
  };

  const offStatus = events.on("ClaudeStatusChanged", ({ event }) => apply(event.payload));
  const offUsage = events.on("ClaudeUsageChanged", () => useClaude.getState().bumpUsage());
  ipc
    .claudeStatus()
    .then((status) => {
      if (!useClaude.getState().status) apply(status);
    })
    .catch((error) => console.error("failed to read Claude status", error));
  return () => {
    offStatus();
    offUsage();
  };
}

export const claudeModule: NotchModule = {
  id: "claude",
  name: "Claude Code",
  icon: Sparkles,
  order: 40,
  init,
  Compact: ClaudeCompact,
  Peek: ClaudePeek,
  // While an agent is working or waiting, hovering the notch previews it.
  usePeekClaim: () => useClaude((s) => Boolean(s.status?.live.some((l) => l.state !== "idle"))),
  Expanded: lazy(() => import("./ClaudeExpanded")),
};

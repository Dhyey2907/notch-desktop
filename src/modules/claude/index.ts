// AI agents module (plan.md Phases 9–10): live activity and approvals for Claude Code, Codex and
// Gemini CLI, plus local Claude Code usage.
import { Sparkles } from "lucide-react";
import { lazy } from "react";

import type { ApprovalRequest } from "../../core/generated/ApprovalRequest";
import type { ClaudeStatus } from "../../core/generated/ClaudeStatus";
import { ipc } from "../../core/ipc";
import type { ModuleContext, NotchModule } from "../types";
import { approvalActivity } from "./approvals";
import { ClaudeCompact, ClaudePeek } from "./ClaudeIndicators";
import { agentName, formatTurn, shouldAnnounceFinish, useClaude } from "./claudeStore";

function init({ events, activities }: ModuleContext) {
  // Sessions we've already alerted for while they wait, so one wait = one alert.
  const alerted = new Map<string, string>();
  // Pending approval id → activity id.
  const shownApprovals = new Map<string, string>();
  let approvalSessions = new Set<string>();

  const syncWaiting = (status: ClaudeStatus) => {
    const waiting = new Set<string>();
    for (const session of status.live) {
      // An approval card already asks for this session; a second alert would be noise.
      if (session.state !== "waiting" || approvalSessions.has(session.sessionId)) continue;
      waiting.add(session.sessionId);
      if (alerted.has(session.sessionId)) continue;
      const id = activities.push({
        moduleId: "claude",
        priority: "high",
        tone: "warning",
        dedupKey: `claude-waiting-${session.sessionId}`,
        title: `${agentName(session.agent)} needs you`,
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

  // When each session started its current turn, to announce long turns when they finish.
  const turnStarted = new Map<string, number>();
  const syncFinished = (status: ClaudeStatus) => {
    const now = Date.now();
    const seen = new Set<string>();
    for (const session of status.live) {
      seen.add(session.sessionId);
      if (session.state !== "idle") {
        if (!turnStarted.has(session.sessionId)) turnStarted.set(session.sessionId, now);
        continue;
      }
      const started = turnStarted.get(session.sessionId);
      turnStarted.delete(session.sessionId);
      if (started === undefined) continue;
      const ms = now - started;
      if (!shouldAnnounceFinish(ms)) continue;
      activities.push({
        moduleId: "claude",
        priority: "normal",
        tone: "success",
        dedupKey: `agent-finished-${session.sessionId}`,
        title: `${agentName(session.agent)} finished`,
        body: [session.project, formatTurn(ms)].filter(Boolean).join(" · "),
      });
    }
    for (const id of turnStarted.keys()) if (!seen.has(id)) turnStarted.delete(id);
  };

  const apply = (status: ClaudeStatus) => {
    useClaude.getState().setStatus(status);
    syncFinished(status);
    syncWaiting(status);
  };

  const applyApprovals = (pending: ApprovalRequest[]) => {
    approvalSessions = new Set(pending.map((r) => r.sessionId));
    const ids = new Set(pending.map((r) => r.id));
    for (const request of pending) {
      if (!shownApprovals.has(request.id))
        shownApprovals.set(request.id, activities.push(approvalActivity(request)));
    }
    // Answered, expired, or handed back to the terminal.
    for (const [requestId, activityId] of shownApprovals) {
      if (!ids.has(requestId)) {
        activities.dismiss(activityId);
        shownApprovals.delete(requestId);
      }
    }
    const status = useClaude.getState().status;
    if (status) syncWaiting(status);
  };

  const offStatus = events.on("ClaudeStatusChanged", ({ event }) => apply(event.payload));
  const offUsage = events.on("ClaudeUsageChanged", () => useClaude.getState().bumpUsage());
  const offApprovals = events.on("ApprovalsChanged", ({ event }) => applyApprovals(event.payload));
  ipc
    .claudeStatus()
    .then((status) => {
      if (!useClaude.getState().status) apply(status);
    })
    .catch((error) => console.error("failed to read Claude status", error));
  ipc
    .claudeApprovalsPending()
    .then(applyApprovals)
    .catch((error) => console.error("failed to read pending approvals", error));
  return () => {
    offStatus();
    offUsage();
    offApprovals();
  };
}

export const claudeModule: NotchModule = {
  id: "claude",
  name: "AI agents",
  icon: Sparkles,
  order: 40,
  init,
  Compact: ClaudeCompact,
  Peek: ClaudePeek,
  // While an agent is working or waiting, hovering the notch previews it.
  usePeekClaim: () => useClaude((s) => Boolean(s.status?.live.some((l) => l.state !== "idle"))),
  Expanded: lazy(() => import("./ClaudeExpanded")),
};

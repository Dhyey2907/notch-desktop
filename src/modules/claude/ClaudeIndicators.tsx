import { Sparkles } from "lucide-react";

import { agentName, primarySession, useClaude } from "./claudeStore";
import "./claude.css";

/** Collapsed: a pulsing mark while an agent works, amber when it's waiting on you. */
export function ClaudeCompact() {
  const session = useClaude((s) => (s.status ? primarySession(s.status.live) : null));
  if (!session || session.state === "idle") return null;
  const waiting = session.state === "waiting";
  return (
    <span
      className={`claude-compact${waiting ? " claude-compact--waiting" : ""}`}
      role="img"
      aria-label={`${agentName(session.agent)} is ${waiting ? "waiting for you" : "working"}`}
    >
      <Sparkles size={12} aria-hidden />
    </span>
  );
}

export function ClaudePeek() {
  const status = useClaude((s) => s.status);
  const session = status ? primarySession(status.live) : null;
  if (!session) {
    return (
      <div className="module-peek">
        <span className="module-peek__title">AI agents</span>
        <span className="module-peek__meta">
          {status?.hooks.installed ? "No active sessions" : "Click to see your usage"}
        </span>
      </div>
    );
  }
  const where = session.project ? ` in ${session.project}` : "";
  return (
    <div className="module-peek">
      <span className="module-peek__title">
        <Sparkles size={14} aria-hidden /> {agentName(session.agent)}
      </span>
      <span className="module-peek__meta">
        {session.state === "waiting"
          ? `Waiting for you${where}`
          : session.state === "working"
            ? `Working${where}${session.tool ? ` · ${session.tool}` : ""}`
            : `Idle${where}`}
      </span>
    </div>
  );
}

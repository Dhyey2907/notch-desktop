// Claude → Sessions: every live coding-agent session — project, what it's doing, tokens and an
// estimated cost today — and a click brings its terminal to the front.
import { useEffect, useState } from "react";

import type { SessionUsage } from "../../core/generated/SessionUsage";
import { ipc, isIpcError } from "../../core/ipc";
import { EmptyState } from "../../design/components";
import { useSettings } from "../../settings";
import { agentName, useClaude } from "./claudeStore";
import { NewTask } from "./NewTask";

const REFRESH_MS = 30_000;

/** "12.3k", "1.2M". */
function compact(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}k`;
  return String(n);
}

function ago(iso: string, now: number): string {
  const s = Math.max(0, Math.round((now - new Date(iso).getTime()) / 1000));
  if (s < 60) return "now";
  if (s < 3600) return `${Math.round(s / 60)}m ago`;
  return `${Math.round(s / 3600)}h ago`;
}

export function Sessions() {
  const live = useClaude((s) => s.status?.live ?? []);
  const [usage, setUsage] = useState<SessionUsage[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [now, setNow] = useState(() => Date.now());
  const cap = useSettings((st) => st.settings?.dev.claudeDailyCapUsd ?? 0);

  useEffect(() => {
    const read = () => {
      setNow(Date.now());
      void ipc.claudeSessionUsage().then(setUsage, () => {});
    };
    read();
    const id = window.setInterval(read, REFRESH_MS);
    return () => window.clearInterval(id);
  }, []);

  if (live.length === 0)
    return (
      <div className="sessions">
        <NewTask />
        <EmptyState
          title="No live sessions"
          body="Coding-agent sessions running on this PC show up here, with their tokens and cost today."
        />
      </div>
    );

  const total = usage.reduce((a, u) => a + u.costUsd, 0);
  return (
    <div className="sessions">
      <NewTask />
      <p className="sessions__total">
        Today: about ${total.toFixed(2)}
        {cap > 0 ? ` of your ${cap} cap` : ""} at API prices across {usage.length}{" "}
        {usage.length === 1 ? "session" : "sessions"} (your plan doesn't bill this).
      </p>
      {error && <p className="control-error">{error}</p>}
      <ul className="sessions__list">
        {live.map((s) => {
          const u = usage.find((x) => x.sessionId === s.sessionId);
          return (
            <li key={s.sessionId}>
              <button
                type="button"
                className={`sessions__item sessions__item--${s.state}`}
                title={s.pid ? "Bring its window to the front" : "Its window isn't known yet"}
                disabled={!s.pid}
                onClick={() =>
                  void ipc.claudeFocusSession(s.sessionId).then(
                    () => setError(null),
                    (e) => setError(isIpcError(e) ? e.message : "Couldn't focus it"),
                  )
                }
              >
                <span className={`sessions__dot sessions__dot--${s.state}`} aria-hidden />
                <span className="sessions__main">
                  <span className="sessions__project">{s.project || "Session"}</span>
                  <span className="sessions__meta">
                    {agentName(s.agent)} · {s.state === "working" ? (s.tool ?? "working") : s.state} ·{" "}
                    {ago(s.updatedAt, now)}
                  </span>
                </span>
                <span className="sessions__usage">
                  {u ? (
                    <>
                      <span>{compact(u.inputTokens + u.outputTokens + u.cacheTokens)} tokens</span>
                      <span className="sessions__cost">${u.costUsd.toFixed(2)}</span>
                    </>
                  ) : (
                    <span className="sessions__meta">no usage yet</span>
                  )}
                </span>
              </button>
            </li>
          );
        })}
      </ul>
    </div>
  );
}

import { useMail } from "./mailStore";

/** Hover: unread count and the newest sender. */
export function MailPeek() {
  const state = useMail((s) => s.state);
  const latest = state?.messages[0];
  return (
    <div className="module-peek">
      <span className="module-peek__title">
        {state?.status === "ready" ? `${state.unread} unread` : "Mail"}
      </span>
      <span className="module-peek__meta">
        {latest
          ? `${latest.from} · ${latest.subject}`
          : state?.status === "ready"
            ? "Inbox zero"
            : "Gmail glance"}
      </span>
    </div>
  );
}

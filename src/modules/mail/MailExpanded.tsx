import { Archive, ExternalLink, MailOpen, RotateCw } from "lucide-react";
import { useState } from "react";

import { ipc, isIpcError } from "../../core/ipc";
import { Button, EmptyState, IconButton, ModuleContainer, SegmentedControl } from "../../design/components";
import { age } from "../clipboard/clips";
import { useMinuteClock } from "../clipboard/useMinuteClock";
import { MailTriage } from "./MailTriage";
import { threadUrl, useMail } from "./mailStore";
// The list rows share the Clipboard module's look (styles only).
import "../clipboard/clipboard.css";

function message(error: unknown) {
  return isIpcError(error) ? error.message.replace(/^invalid input: /, "") : "That didn't work.";
}

export default function MailExpanded() {
  const state = useMail((s) => s.state);
  const [status, setStatus] = useState<string | null>(null);
  const [view, setView] = useState<"one" | "list">("one");
  const now = useMinuteClock();
  const run = (action: Promise<unknown>, done?: string) =>
    void action.then(
      () => setStatus(done ?? null),
      (e) => setStatus(message(e)),
    );

  let body;
  if (!state || state.status === "off") {
    body = (
      <EmptyState
        title="See your Gmail here"
        body="Turn on Gmail in Settings → Calendar, then sign in with Google again to allow it. Overline can read your inbox, archive and mark read — never send or delete."
        action={
          <Button onClick={() => void ipc.openSettingsWindow("calendar")}>Open Calendar settings</Button>
        }
      />
    );
  } else if (state.status === "needsSignIn") {
    body = (
      <EmptyState
        title="Allow Gmail"
        body="Gmail is turned on, but Google hasn't been asked yet. Sign in with Google again in Settings → Calendar."
        action={<Button onClick={() => void ipc.openSettingsWindow("calendar")}>Sign in again</Button>}
      />
    );
  } else if (view === "one") {
    body = <MailTriage messages={state.messages} unread={state.unread} snoozed={state.snoozed} now={now} />;
  } else if (state.messages.length === 0) {
    body = <EmptyState title="Inbox zero" body="No unread mail. Nice." />;
  } else {
    body = (
      <ul className="clips-list" aria-label="Unread mail">
        {state.messages.map((m) => (
          <li key={m.id} className="clip">
            <button
              type="button"
              className="clip__main"
              title="Open in Gmail"
              onClick={() => run(ipc.openExternal(threadUrl(m.threadId)))}
            >
              <span className="clip__icon" aria-hidden>
                {m.from.charAt(0).toUpperCase()}
              </span>
              <span className="clip__text">
                <span className="clip__title">
                  {m.from} · {m.subject}
                </span>
                <span className="clip__meta">
                  {age(m.at, now)} · {m.snippet}
                </span>
              </span>
            </button>
            <div className="clip__actions">
              <IconButton
                icon={MailOpen}
                label="Mark read"
                size="sm"
                onClick={() => run(ipc.mailMarkRead(m.id))}
              />
              <IconButton
                icon={Archive}
                label="Archive"
                size="sm"
                onClick={() => run(ipc.mailArchive(m.id), "Archived")}
              />
              <IconButton
                icon={ExternalLink}
                label="Open in Gmail"
                size="sm"
                onClick={() => run(ipc.openExternal(threadUrl(m.threadId)))}
              />
            </div>
          </li>
        ))}
      </ul>
    );
  }

  return (
    <ModuleContainer
      title={state?.status === "ready" ? `Mail · ${state.unread} unread` : "Mail"}
      trailing={
        state?.status === "ready" || state?.status === "error" ? (
          <>
            <SegmentedControl<"one" | "list">
              label="Mail view"
              value={view}
              options={[
                { value: "one", label: "One by one" },
                { value: "list", label: "List" },
              ]}
              onChange={setView}
            />
            <IconButton icon={RotateCw} label="Check now" size="sm" onClick={() => void ipc.mailRefresh()} />
          </>
        ) : undefined
      }
    >
      <div className="clips">
        {state?.status === "error" && (
          <p className="clips-status">Couldn't reach Gmail — showing what was there.</p>
        )}
        {status && <p className="clips-status">{status}</p>}
        {body}
      </div>
    </ModuleContainer>
  );
}

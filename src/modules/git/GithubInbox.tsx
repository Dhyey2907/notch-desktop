// Unread GitHub notifications: review requests, CI results, mentions. Open one in the browser
// or mark it read (on GitHub too).
import {
  Bell,
  Check,
  CheckCircle2,
  ExternalLink,
  GitPullRequest,
  Inbox,
  MessageSquare,
  XCircle,
} from "lucide-react";
import { useState } from "react";

import type { GithubNotification } from "../../core/generated/GithubNotification";
import type { NotificationKind } from "../../core/generated/NotificationKind";
import { ipc, isIpcError } from "../../core/ipc";
import { Button, EmptyState, IconButton, ModuleContainer } from "../../design/components";
import { heading } from "./githubAlerts";
import { useGithub } from "./githubStore";
import { useMinuteNow } from "./useMinuteNow";

function KindIcon({ kind }: { kind: NotificationKind }) {
  switch (kind) {
    case "reviewRequested":
      return <GitPullRequest size={14} />;
    case "ciFailed":
      return <XCircle size={14} className="gh-inbox__fail" />;
    case "ciPassed":
      return <CheckCircle2 size={14} className="gh-inbox__pass" />;
    case "mention":
    case "assigned":
      return <MessageSquare size={14} />;
    default:
      return <Bell size={14} />;
  }
}

function ago(at: string, now: number): string {
  const minutes = Math.floor((now - Date.parse(at)) / 60_000);
  if (minutes < 1) return "now";
  if (minutes < 60) return `${minutes}m`;
  if (minutes < 24 * 60) return `${Math.floor(minutes / 60)}h`;
  return `${Math.floor(minutes / (24 * 60))}d`;
}

/** Trailing button that opens the inbox, with the unread count. */
export function InboxButton({ onOpen }: { onOpen: () => void }) {
  const count = useGithub((s) => s.notifications.length);
  return (
    <Button onClick={onOpen}>
      <Inbox size={14} aria-hidden /> {count ? `Inbox · ${count}` : "Inbox"}
    </Button>
  );
}

export function GithubInbox({ onClose }: { onClose: () => void }) {
  const items = useGithub((s) => s.notifications);
  const now = useMinuteNow();
  const [error, setError] = useState<string | null>(null);
  const run = (action: Promise<void>) =>
    void action.then(
      () => setError(null),
      (e) => setError(isIpcError(e) ? e.message.replace(/^invalid input: /, "") : "Something went wrong."),
    );

  return (
    <ModuleContainer title="GitHub inbox" trailing={<Button onClick={onClose}>Done</Button>}>
      {error && <p className="gh-inbox__error">{error}</p>}
      {items.length === 0 ? (
        <EmptyState
          title="All caught up"
          body="Review requests, CI results on your runs, and mentions show up here — and pop up in the notch as they arrive."
        />
      ) : (
        <ul className="gh-inbox" aria-label="GitHub notifications">
          {items.map((n: GithubNotification) => (
            <li key={n.id} className="gh-inbox__item">
              <span className="gh-inbox__icon" aria-hidden>
                <KindIcon kind={n.kind} />
              </span>
              <button
                type="button"
                className="gh-inbox__main"
                title="Open on GitHub"
                onClick={() => run(ipc.openExternal(n.url))}
              >
                <span className="gh-inbox__title">{n.title}</span>
                <span className="gh-inbox__meta">
                  {heading(n.kind)} · {n.repo} · {ago(n.updatedAt, now)}
                </span>
              </button>
              <IconButton
                icon={ExternalLink}
                label="Open on GitHub"
                size="sm"
                onClick={() => run(ipc.openExternal(n.url))}
              />
              <IconButton
                icon={Check}
                label="Mark read"
                size="sm"
                onClick={() => run(ipc.githubMarkRead(n.id))}
              />
            </li>
          ))}
        </ul>
      )}
    </ModuleContainer>
  );
}

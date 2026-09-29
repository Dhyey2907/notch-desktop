// Recent phone notifications from Phone Link, newest first, with Reply (opens Phone Link)
// and Dismiss (clears it from Action Center).
import { MessageSquareReply, X } from "lucide-react";
import { useEffect, useState } from "react";

import { ipc } from "../../core/ipc";
import { Button, EmptyState, IconButton, ModuleContainer } from "../../design/components";
import { ago, usePhone } from "./phoneStore";
import "./phone.css";

function useMinuteClock() {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const id = window.setInterval(() => setNow(Date.now()), 60_000);
    return () => window.clearInterval(id);
  }, []);
  return now;
}

export default function PhoneExpanded() {
  const snapshot = usePhone((s) => s.snapshot);
  const now = useMinuteClock();
  const open = <Button onClick={() => void ipc.phoneOpen()}>Open Phone Link</Button>;

  let body;
  if (!snapshot) {
    body = <EmptyState title="Reading notifications…" body="One moment." />;
  } else if (!snapshot.phoneLinkInstalled) {
    body = (
      <EmptyState
        title="Phone Link isn't installed"
        body="Install Phone Link from the Microsoft Store and connect your phone; its notifications will show up here."
      />
    );
  } else if (snapshot.access !== "allowed") {
    body = (
      <EmptyState
        title="Allow notification access"
        body="Windows needs your OK for Notch to read notifications. Turn on “Notification access” for apps in Windows Settings."
        action={<Button onClick={() => void ipc.phoneOpenAccessSettings()}>Open Windows Settings</Button>}
      />
    );
  } else if (snapshot.notifications.length === 0) {
    body = (
      <EmptyState
        title="No phone notifications"
        body="New notifications from your phone appear here and pop up in the notch."
      />
    );
  } else {
    body = (
      <ul className="phone-list" aria-label="Phone notifications">
        {snapshot.notifications.map((n) => (
          <li key={n.id} className="phone-item">
            <span className="nd-avatar" aria-hidden>
              {n.title.trim().charAt(0).toUpperCase() || "•"}
            </span>
            <div className="phone-item__text">
              <p className="phone-item__title">
                <span>{n.title}</span>
                <time>{ago(n.at, now)}</time>
              </p>
              {n.body && <p className="phone-item__body">{n.body}</p>}
            </div>
            <div className="phone-item__actions">
              <IconButton
                icon={MessageSquareReply}
                label={`Reply to ${n.title} in Phone Link`}
                size="sm"
                onClick={() => void ipc.phoneOpen()}
              />
              <IconButton
                icon={X}
                label={`Dismiss ${n.title}`}
                size="sm"
                onClick={() => void ipc.phoneDismiss(n.id)}
              />
            </div>
          </li>
        ))}
      </ul>
    );
  }

  return (
    <ModuleContainer title="Phone" trailing={snapshot?.phoneLinkInstalled ? open : undefined}>
      <div className="phone">
        {body}
        {snapshot?.access === "allowed" && (
          <p className="phone-tip">
            Seeing messages twice? Turn off banners for Phone Link in Windows Settings → System →
            Notifications. They'll still show up here. Replies open in Phone Link.
          </p>
        )}
      </div>
    </ModuleContainer>
  );
}

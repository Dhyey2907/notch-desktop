// Inbox zero: unread mail one at a time. Swipe (or drag) left to archive, right to mark read,
// or use the buttons / keys: A archive, R read, S snooze, → skip, O open.
import { Archive, Clock, ExternalLink, MailOpen, SkipForward } from "lucide-react";
import { useEffect, useRef, useState, type KeyboardEvent, type PointerEvent } from "react";

import type { MailMessage } from "../../core/generated/MailMessage";
import { ipc, isIpcError } from "../../core/ipc";
import { EmptyState } from "../../design/components";
import { age } from "../clipboard/clips";
import { remindChoices } from "../todo/remind";
import { threadUrl } from "./mailStore";
import "./mail.css";

const SWIPE_PX = 90;

function message(error: unknown) {
  return isIpcError(error) ? error.message.replace(/^invalid input: /, "") : "That didn't work.";
}

/** The messages still to go through: not skipped, not just handled. */
function queue(messages: MailMessage[], gone: Set<string>): MailMessage[] {
  return messages.filter((m) => !gone.has(m.id));
}

export function MailTriage({
  messages,
  unread,
  snoozed,
  now,
}: {
  messages: MailMessage[];
  unread: number;
  snoozed: number;
  now: number;
}) {
  // Handled or skipped this session (handled ones also leave Gmail's list on the next check).
  const [gone, setGone] = useState<Set<string>>(() => new Set());
  const [done, setDone] = useState(0);
  const [snoozing, setSnoozing] = useState(false);
  const [status, setStatus] = useState<string | null>(null);
  const [dx, setDx] = useState(0);
  const drag = useRef<{ x: number; id: number } | null>(null);
  const root = useRef<HTMLDivElement>(null);
  const left = queue(messages, gone);
  const m = left[0];

  const next = (id: string, handled: boolean) => {
    setGone((g) => new Set(g).add(id));
    if (handled) setDone((d) => d + 1);
    setSnoozing(false);
    setDx(0);
  };
  const act = (id: string, action: Promise<unknown>, label: string) => {
    next(id, true);
    action.then(
      () => setStatus(label),
      (e) => {
        // Put it back so nothing is lost.
        setGone((g) => {
          const copy = new Set(g);
          copy.delete(id);
          return copy;
        });
        setDone((d) => Math.max(0, d - 1));
        setStatus(message(e));
      },
    );
  };
  const archive = (x: MailMessage) => act(x.id, ipc.mailArchive(x.id), "Archived");
  const read = (x: MailMessage) => act(x.id, ipc.mailMarkRead(x.id), "Marked read");
  const snooze = (x: MailMessage, at: number, label: string) =>
    act(x.id, ipc.mailSnooze(x.id, new Date(at).toISOString()), `Snoozed · ${label.toLowerCase()}`);

  useEffect(() => {
    root.current?.focus();
  }, []);

  if (!m) {
    return (
      <EmptyState
        title={done > 0 ? `Inbox zero · ${done} handled` : "Inbox zero"}
        body={
          gone.size > done
            ? "You skipped a few — they're still unread in Gmail."
            : snoozed > 0
              ? `Nothing unread. ${snoozed} snoozed ${snoozed === 1 ? "message comes" : "messages come"} back later.`
              : "No unread mail. Nice."
        }
      />
    );
  }

  const onKey = (e: KeyboardEvent) => {
    if (e.target instanceof HTMLInputElement) return;
    const key = e.key.toLowerCase();
    if (key === "a") archive(m);
    else if (key === "r") read(m);
    else if (key === "s") setSnoozing((s) => !s);
    else if (key === "arrowright" || key === "n") next(m.id, false);
    else if (key === "o") void ipc.openExternal(threadUrl(m.threadId));
    else return;
    e.preventDefault();
  };

  const onDown = (e: PointerEvent<HTMLElement>) => {
    if ((e.target as HTMLElement).closest("button")) return;
    drag.current = { x: e.clientX, id: e.pointerId };
    e.currentTarget.setPointerCapture(e.pointerId);
  };
  const onMove = (e: PointerEvent<HTMLElement>) => {
    if (drag.current?.id === e.pointerId) setDx(e.clientX - drag.current.x);
  };
  const onUp = () => {
    if (!drag.current) return;
    drag.current = null;
    if (dx <= -SWIPE_PX) archive(m);
    else if (dx >= SWIPE_PX) read(m);
    else setDx(0);
  };

  const position = gone.size + 1;
  const total = Math.max(left.length + gone.size, position);
  const hint = dx <= -SWIPE_PX / 2 ? "archive" : dx >= SWIPE_PX / 2 ? "read" : null;

  return (
    <div
      className="mail-triage"
      ref={root}
      tabIndex={-1}
      onKeyDown={onKey}
      aria-label="Go through unread mail"
    >
      <p className="mail-triage__count">
        {position} of {total}
        {unread > total ? ` · ${unread} unread in all` : ""}
        {status ? ` · ${status}` : ""}
      </p>
      <article
        key={m.id}
        className={`mail-card${hint ? ` mail-card--${hint}` : ""}`}
        style={{ transform: dx ? `translateX(${dx}px) rotate(${dx / 40}deg)` : undefined }}
        onPointerDown={onDown}
        onPointerMove={onMove}
        onPointerUp={onUp}
        onPointerCancel={onUp}
        aria-label={`${m.from}: ${m.subject}`}
      >
        <header className="mail-card__head">
          <span className="mail-card__avatar" aria-hidden>
            {m.from.charAt(0).toUpperCase()}
          </span>
          <span className="mail-card__from">{m.from}</span>
          <span className="mail-card__age">{age(m.at, now)}</span>
        </header>
        <p className="mail-card__subject">{m.subject}</p>
        <p className="mail-card__snippet">{m.snippet}</p>
        {hint && (
          <span className="mail-card__hint" aria-hidden>
            {hint === "archive" ? "Archive" : "Mark read"}
          </span>
        )}
      </article>
      {snoozing ? (
        <div className="mail-triage__actions" role="group" aria-label="Snooze until">
          {remindChoices(new Date(now)).map((c) => (
            <button key={c.label} type="button" className="nd-chip" onClick={() => snooze(m, c.at, c.label)}>
              {c.label}
            </button>
          ))}
          <button type="button" className="nd-chip" onClick={() => setSnoozing(false)}>
            Cancel
          </button>
        </div>
      ) : (
        <div className="mail-triage__actions">
          <button
            type="button"
            className="nd-chip"
            title="Archive (A, or swipe left)"
            onClick={() => archive(m)}
          >
            <Archive size={13} aria-hidden /> Archive
          </button>
          <button
            type="button"
            className="nd-chip"
            title="Mark read (R, or swipe right)"
            onClick={() => read(m)}
          >
            <MailOpen size={13} aria-hidden /> Read
          </button>
          <button type="button" className="nd-chip" title="Snooze (S)" onClick={() => setSnoozing(true)}>
            <Clock size={13} aria-hidden /> Snooze
          </button>
          <button type="button" className="nd-chip" title="Skip (→)" onClick={() => next(m.id, false)}>
            <SkipForward size={13} aria-hidden /> Skip
          </button>
          <button
            type="button"
            className="nd-chip"
            title="Open in Gmail (O)"
            onClick={() => void ipc.openExternal(threadUrl(m.threadId))}
          >
            <ExternalLink size={13} aria-hidden /> Open
          </button>
        </div>
      )}
    </div>
  );
}

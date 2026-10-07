// Read later: links saved to read, newest first, with the site, reading time and an AI summary
// on request. Paste a link, or use "Read later" on a copied link or in Quick capture.
import { BookmarkPlus, Check, ExternalLink, RotateCcw, Sparkles, Trash2 } from "lucide-react";
import { useState, type FormEvent } from "react";

import type { ReadItem } from "../../core/generated/ReadItem";
import { ipc, isIpcError } from "../../core/ipc";
import { Button, EmptyState, IconButton } from "../../design/components";
import { age } from "./clips";
import { useClipboard } from "./clipboardStore";
import { findLink, readingMinutes, useReadLater } from "./readLaterStore";
import { useMinuteClock } from "./useMinuteClock";

function message(error: unknown) {
  return isIpcError(error) ? error.message.replace(/^invalid input: /, "") : "That didn't work.";
}

export function ReadLaterView() {
  const snapshot = useReadLater((s) => s.snapshot);
  // The newest copied text, if it's a link not saved yet: one click to save it.
  const copiedLink = useClipboard((s) => {
    const clip = s.snapshot?.recent[0];
    return clip?.kind === "text" && clip.textKind === "url" ? findLink(clip.preview) : null;
  });
  const [text, setText] = useState("");
  const [status, setStatus] = useState<string | null>(null);
  const now = useMinuteClock();
  const items = snapshot?.items ?? [];
  const unread = items.filter((i) => !i.readAt);
  const read = items.filter((i) => i.readAt);
  const offerCopied = copiedLink && !items.some((i) => i.url === copiedLink) ? copiedLink : null;

  const run = (action: Promise<unknown>, done?: string) =>
    void action.then(
      () => setStatus(done ?? null),
      (e) => setStatus(message(e)),
    );

  const save = (url: string) => {
    setStatus("Saving…");
    run(
      ipc.readLaterAdd(url).then(() => setText("")),
      "Saved",
    );
  };

  return (
    <div className="clips read-later">
      <form
        className="read-later__add"
        onSubmit={(e: FormEvent) => {
          e.preventDefault();
          const url = findLink(text) ?? text.trim();
          if (url) save(url);
        }}
      >
        <input
          className="nd-input read-later__input"
          aria-label="Link to read later"
          placeholder="Paste a link to read later"
          value={text}
          maxLength={2048}
          onChange={(e) => setText(e.target.value)}
        />
        <IconButton
          icon={BookmarkPlus}
          label="Save link"
          size="sm"
          disabled={!text.trim()}
          onClick={() => {
            const url = findLink(text) ?? text.trim();
            if (url) save(url);
          }}
        />
      </form>
      {offerCopied && (
        <button type="button" className="nd-chip read-later__copied" onClick={() => save(offerCopied)}>
          <BookmarkPlus size={12} aria-hidden /> Save the link you copied
        </button>
      )}
      {(status || (snapshot?.saving ?? 0) > 0) && (
        <p className="clips-status">{snapshot?.saving ? "Saving…" : status}</p>
      )}
      {items.length === 0 ? (
        <EmptyState
          title="Nothing to read yet"
          body="Paste a link above, click Read later on a copied link, or type it into Quick capture."
        />
      ) : (
        <ul className="clips-list" aria-label="Read later">
          {unread.map((item) => (
            <ReadRow
              key={item.id}
              item={item}
              now={now}
              summarizing={snapshot?.summarizing.includes(item.id) ?? false}
              run={run}
            />
          ))}
          {read.length > 0 && (
            <li className="read-later__section">
              <span>Read · {read.length}</span>
              <Button onClick={() => run(ipc.readLaterClearRead())}>Clear</Button>
            </li>
          )}
          {read.map((item) => (
            <ReadRow key={item.id} item={item} now={now} summarizing={false} run={run} />
          ))}
        </ul>
      )}
    </div>
  );
}

function ReadRow({
  item,
  now,
  summarizing,
  run,
}: {
  item: ReadItem;
  now: number;
  summarizing: boolean;
  run: (action: Promise<unknown>, done?: string) => void;
}) {
  const minutes = readingMinutes(item.words);
  const open = () => {
    run(ipc.openExternal(item.url));
    if (!item.readAt) run(ipc.readLaterSetRead(item.id, true));
  };
  return (
    <li className={`clip read-later__item${item.readAt ? " read-later__item--read" : ""}`}>
      <div className="read-later__row">
        <button type="button" className="clip__main" title="Open (marks it read)" onClick={open}>
          <span className="clip__icon" aria-hidden>
            {item.site.charAt(0).toUpperCase()}
          </span>
          <span className="clip__text">
            <span className="clip__title">{item.title}</span>
            <span className="clip__meta">
              {item.site}
              {minutes ? ` · ${minutes} min read` : ""} · {age(item.addedAt, now)}
            </span>
          </span>
        </button>
        <div className="clip__actions">
          {!item.summary && !item.readAt && (
            <IconButton
              icon={Sparkles}
              label={summarizing ? "Summarizing…" : "Summarize with AI"}
              size="sm"
              disabled={summarizing}
              onClick={() => run(ipc.readLaterSummarize(item.id))}
            />
          )}
          <IconButton
            icon={item.readAt ? RotateCcw : Check}
            label={item.readAt ? "Mark unread" : "Mark read"}
            size="sm"
            onClick={() => run(ipc.readLaterSetRead(item.id, !item.readAt))}
          />
          <IconButton icon={ExternalLink} label="Open" size="sm" onClick={open} />
          <IconButton
            icon={Trash2}
            label="Remove"
            size="sm"
            onClick={() => run(ipc.readLaterRemove(item.id))}
          />
        </div>
      </div>
      {summarizing && <p className="read-later__summary read-later__summary--busy">Summarizing…</p>}
      {item.summary && !item.readAt && <p className="read-later__summary">{item.summary}</p>}
    </li>
  );
}

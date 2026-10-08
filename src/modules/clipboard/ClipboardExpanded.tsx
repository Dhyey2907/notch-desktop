// Copied | Files. Copied is clipboard history: click a clip to copy it again, pin the ones you
// reuse; links open, colors show a swatch, JSON can be copied indented, and short text can
// become a task. Files is the drop shelf (ShelfView).
import { convertFileSrc } from "@tauri-apps/api/core";
import {
  BookmarkPlus,
  Braces,
  Check,
  ExternalLink,
  FileText,
  Files,
  Image as ImageIcon,
  Link,
  ListPlus,
  PackagePlus,
  Palette,
  Pause,
  Pin,
  PinOff,
  Play,
  QrCode,
  ScanText,
  StickyNote,
  Trash2,
  Type,
  type LucideIcon,
} from "lucide-react";
import { useEffect, useState } from "react";

import type { Clip } from "../../core/generated/Clip";
import { ipc, isIpcError } from "../../core/ipc";
import {
  ConfirmButton,
  EmptyState,
  IconButton,
  ModuleContainer,
  SegmentedControl,
} from "../../design/components";
import { useSettings } from "../../settings";
import { age, clipTitle, matchesQuery, taskTitle } from "./clips";
import { useClipboard } from "./clipboardStore";
import { useShelf, type ClipboardTab } from "./shelfStore";
import { RecentView } from "./RecentView";
import { ShelfView } from "./ShelfView";
import { ReadLaterView } from "./ReadLaterView";
import { useReadLater } from "./readLaterStore";
import { useMinuteClock } from "./useMinuteClock";
import "./clipboard.css";

const COPIED_MS = 1400;

function message(error: unknown) {
  return isIpcError(error)
    ? error.message.replace(/^invalid input: /, "")
    : "Something went wrong. Try again.";
}

const KIND_ICONS: Record<string, LucideIcon> = {
  image: ImageIcon,
  file: FileText,
  files: Files,
  url: Link,
  color: Palette,
  json: Braces,
  text: Type,
};

function iconKey(clip: Clip): string {
  if (clip.kind === "files") return clip.files.length > 1 ? "files" : "file";
  if (clip.kind === "image") return "image";
  return clip.textKind ?? "text";
}

function ClipIcon({ clip }: { clip: Clip }) {
  const Icon = KIND_ICONS[iconKey(clip)] ?? Type;
  return <Icon size={14} />;
}

export default function ClipboardExpanded() {
  const tab = useShelf((s) => s.tab);
  const shelfCount = useShelf((s) => s.snapshot?.items.length ?? 0);
  const unread = useReadLater((s) => s.snapshot?.items.filter((i) => !i.readAt).length ?? 0);
  return (
    <ModuleContainer
      title="Clipboard"
      trailing={
        <SegmentedControl<ClipboardTab>
          label="Clipboard view"
          value={tab}
          options={[
            { value: "copied", label: "Copied" },
            { value: "files", label: shelfCount ? `Files · ${shelfCount}` : "Files" },
            { value: "recent", label: "Recent" },
            { value: "read", label: unread ? `Read later · ${unread}` : "Read later" },
          ]}
          onChange={(t) => useShelf.getState().setTab(t)}
        />
      }
    >
      {tab === "copied" ? (
        <CopiedView />
      ) : tab === "files" ? (
        <ShelfView />
      ) : tab === "read" ? (
        <ReadLaterView />
      ) : (
        <RecentView />
      )}
    </ModuleContainer>
  );
}

function CopiedView() {
  const snapshot = useClipboard((s) => s.snapshot);
  const paused = useSettings((s) => s.settings?.clipboard.paused ?? false);
  const [query, setQuery] = useState("");
  const [copied, setCopied] = useState<string | null>(null);
  const [status, setStatus] = useState<string | null>(null);
  const [qr, setQr] = useState<{ svg: string; title: string } | null>(null);
  const now = useMinuteClock();

  useEffect(() => {
    if (!copied) return;
    const timer = window.setTimeout(() => setCopied(null), COPIED_MS);
    return () => window.clearTimeout(timer);
  }, [copied]);

  const run = (action: Promise<unknown>, done?: string) =>
    void action.then(
      () => setStatus(done ?? null),
      (e) => setStatus(message(e)),
    );

  const copy = (clip: Clip, formatted = false) =>
    void ipc.clipboardCopy(clip.id, formatted).then(
      () => {
        setCopied(clip.id);
        setStatus(null);
      },
      (e) => setStatus(message(e)),
    );

  const setPaused = (value: boolean) =>
    void useSettings
      .getState()
      .update((s) => ({ ...s, clipboard: { ...s.clipboard, paused: value } }))
      .catch((e) => setStatus(message(e)));

  const pinned = (snapshot?.pinned ?? []).filter((c) => matchesQuery(c, query));
  const recent = (snapshot?.recent ?? []).filter((c) => matchesQuery(c, query));
  const empty = !snapshot || snapshot.pinned.length + snapshot.recent.length === 0;

  const row = (clip: Clip) => (
    <ClipRow
      key={clip.id}
      clip={clip}
      now={now}
      copied={copied === clip.id}
      onCopy={(formatted) => copy(clip, formatted)}
      onPin={() => run(clip.pinned ? ipc.clipboardUnpin(clip.id) : ipc.clipboardPin(clip.id))}
      onDelete={() => run(ipc.clipboardDelete(clip.id))}
      onOpen={() => run(ipc.openExternal(clip.preview.trim()))}
      onReadLater={() => run(ipc.readLaterAdd(clip.preview.trim()), "Saved to Read later")}
      onTask={(title) => run(ipc.todoAddTask(title, null, false), "Added to To-do")}
      onShelf={() => run(ipc.shelfAdd(clip.files), "Added to Files")}
      onQr={() =>
        void ipc
          .clipboardText(clip.id)
          .then((text) => ipc.qrSvg(text.trim()))
          .then(
            (svg) => setQr({ svg, title: clipTitle(clip) }),
            (e) => setStatus(message(e)),
          )
      }
      onNote={() =>
        run(
          ipc.clipboardText(clip.id).then((text) => ipc.noteAdd(text)),
          "Saved as a note in To-do",
        )
      }
    />
  );

  return (
    <div className="clips">
      <div className="clips-bar">
        {!empty && (
          <input
            className="clips-search"
            aria-label="Search clipboard history"
            placeholder="Search"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
          />
        )}
        <span className="clips-bar__spacer" />
        <IconButton
          icon={ScanText}
          label="Grab text from the screen (draw a box around it)"
          size="sm"
          onClick={() => run(ipc.grabBegin())}
        />
        <IconButton
          icon={paused ? Play : Pause}
          label={paused ? "Resume clipboard history" : "Pause clipboard history"}
          size="sm"
          active={paused}
          onClick={() => setPaused(!paused)}
        />
        {(snapshot?.recent.length ?? 0) > 0 && (
          <ConfirmButton confirmLabel="Clear history?" onConfirm={() => run(ipc.clipboardClear())}>
            Clear
          </ConfirmButton>
        )}
      </div>
      {status && <p className="clips-status">{status}</p>}
      {paused && <p className="clips-status">Paused — new copies aren't kept.</p>}
      {qr && (
        <div className="clips-qr" role="dialog" aria-label="QR code">
          {/* Generated by Overline from the copied text (not page content). */}
          <div className="clips-qr__code" dangerouslySetInnerHTML={{ __html: qr.svg }} />
          <div className="clips-qr__text">
            <p className="clips-qr__title">Scan with your phone</p>
            <p className="clips-qr__body">{qr.title}</p>
            <button type="button" className="home-link" onClick={() => setQr(null)}>
              Close
            </button>
          </div>
        </div>
      )}
      {empty ? (
        <EmptyState
          title="Nothing copied yet"
          body="Copy text, a picture or files and it shows up here. Password managers are never recorded."
        />
      ) : pinned.length + recent.length === 0 ? (
        <EmptyState title="No matches" body="Try a different search." />
      ) : (
        <ul className="clips-list" aria-label="Clipboard history">
          {pinned.length > 0 && <li className="clips-section">Pinned</li>}
          {pinned.map(row)}
          {pinned.length > 0 && recent.length > 0 && <li className="clips-section">Recent</li>}
          {recent.map(row)}
        </ul>
      )}
    </div>
  );
}

function ClipRow({
  clip,
  now,
  copied,
  onCopy,
  onPin,
  onDelete,
  onOpen,
  onReadLater,
  onTask,
  onNote,
  onShelf,
  onQr,
}: {
  clip: Clip;
  now: number;
  copied: boolean;
  onCopy: (formatted?: boolean) => void;
  onPin: () => void;
  onDelete: () => void;
  onOpen: () => void;
  onReadLater: () => void;
  onTask: (title: string) => void;
  onNote: () => void;
  onShelf: () => void;
  onQr: () => void;
}) {
  const title = clipTitle(clip);
  const task = taskTitle(clip);
  const color = clip.textKind === "color" ? clip.preview.trim() : null;

  return (
    <li className="clip">
      <button
        type="button"
        className="clip__main"
        aria-label={`Copy ${clip.kind === "text" ? title.slice(0, 80) : title}`}
        title="Copy"
        onClick={() => onCopy()}
      >
        {clip.kind === "image" && clip.image ? (
          <img className="clip__thumb" src={convertFileSrc(clip.image)} alt="" />
        ) : color ? (
          <span className="clip__swatch" style={{ background: color }} aria-hidden />
        ) : (
          <span className="clip__icon" aria-hidden>
            <ClipIcon clip={clip} />
          </span>
        )}
        <span className="clip__text">
          <span className="clip__title">{title}</span>
          <span className="clip__meta">
            {copied ? (
              <span className="clip__copied">
                <Check size={11} aria-hidden /> Copied
              </span>
            ) : (
              <>
                {clip.app ? `${clip.app} · ` : ""}
                {age(clip.at, now)}
                {clip.kind === "text" && clip.length > 200 ? ` · ${clip.length.toLocaleString()} chars` : ""}
              </>
            )}
          </span>
        </span>
      </button>
      <div className="clip__actions">
        {clip.textKind === "url" && (
          <>
            <IconButton icon={ExternalLink} label="Open link" size="sm" onClick={onOpen} />
            <IconButton icon={BookmarkPlus} label="Read later" size="sm" onClick={onReadLater} />
          </>
        )}
        {clip.textKind === "json" && (
          <IconButton icon={Braces} label="Copy formatted JSON" size="sm" onClick={() => onCopy(true)} />
        )}
        {task && <IconButton icon={ListPlus} label="Add as a task" size="sm" onClick={() => onTask(task)} />}
        {clip.kind === "text" && !clip.textKind && !task && (
          <IconButton icon={StickyNote} label="Save as a note" size="sm" onClick={onNote} />
        )}
        {clip.kind === "files" && (
          <IconButton icon={PackagePlus} label="Add to Files shelf" size="sm" onClick={onShelf} />
        )}
        {clip.kind === "text" && clip.length <= 2000 && (
          <IconButton icon={QrCode} label="Show as a QR code" size="sm" onClick={onQr} />
        )}
        <IconButton
          icon={clip.pinned ? PinOff : Pin}
          label={clip.pinned ? "Unpin" : "Pin"}
          size="sm"
          active={clip.pinned}
          onClick={onPin}
        />
        <IconButton icon={Trash2} label="Delete" size="sm" onClick={onDelete} />
      </div>
    </li>
  );
}

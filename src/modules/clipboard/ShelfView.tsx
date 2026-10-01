// The drop shelf: files dragged onto the notch wait here. Drag one (or "Drag all") out into
// any app, or copy them and paste. The shelf only remembers where files are, so moved or
// deleted ones show as missing.
import {
  Copy,
  File,
  FileArchive,
  FileAudio,
  FileCode,
  FileImage,
  FileSpreadsheet,
  FileText,
  FileVideo,
  Folder,
  FolderOpen,
  GripVertical,
  X,
  type LucideIcon,
} from "lucide-react";
import { useRef, useState, type PointerEvent } from "react";

import type { ShelfItem } from "../../core/generated/ShelfItem";
import { ipc, isIpcError } from "../../core/ipc";
import { ConfirmButton, EmptyState, IconButton } from "../../design/components";
import { useFileDrag } from "../../shell/fileDrag";
import { age } from "./clips";
import { formatSize, useShelf } from "./shelfStore";
import { useMinuteClock } from "./useMinuteClock";

/** Pointer travel before a press becomes a drag. */
const DRAG_THRESHOLD = 6;

const EXT_ICONS: [LucideIcon, string[]][] = [
  [FileImage, ["png", "jpg", "jpeg", "gif", "webp", "bmp", "svg", "heic", "ico"]],
  [FileVideo, ["mp4", "mov", "mkv", "avi", "webm"]],
  [FileAudio, ["mp3", "wav", "flac", "m4a", "ogg"]],
  [FileArchive, ["zip", "rar", "7z", "tar", "gz"]],
  [FileSpreadsheet, ["xlsx", "xls", "csv"]],
  [FileCode, ["js", "ts", "tsx", "rs", "py", "json", "html", "css", "md"]],
  [FileText, ["pdf", "doc", "docx", "txt", "rtf", "pptx"]],
];

function ItemIcon({ item }: { item: ShelfItem }) {
  const Icon = item.isDir
    ? Folder
    : (EXT_ICONS.find(([, exts]) => item.ext && exts.includes(item.ext))?.[0] ?? File);
  return <Icon size={14} />;
}

function message(error: unknown) {
  return isIpcError(error)
    ? error.message.replace(/^invalid input: /, "")
    : "Something went wrong. Try again.";
}

/** Press and move: drag these files out of the notch (no ids: the whole shelf). */
function useDragOut(ids: string[], onError: (message: string) => void) {
  const start = useRef<{ x: number; y: number } | null>(null);
  return {
    onPointerDown: (e: PointerEvent) => {
      if (e.button !== 0) return;
      start.current = { x: e.clientX, y: e.clientY };
      // Keep getting moves after the pointer leaves a small target (the "Drag all" grip).
      e.currentTarget.setPointerCapture(e.pointerId);
    },
    onPointerMove: (e: PointerEvent) => {
      const from = start.current;
      if (!from) return;
      if ((e.buttons & 1) === 0) {
        start.current = null;
        return;
      }
      if (Math.hypot(e.clientX - from.x, e.clientY - from.y) < DRAG_THRESHOLD) return;
      start.current = null;
      // Windows takes over the mouse for the drag.
      e.currentTarget.releasePointerCapture(e.pointerId);
      useFileDrag.setState({ draggingOut: true });
      ipc
        .shelfDrag(ids)
        .catch((err) => onError(message(err)))
        .finally(() => useFileDrag.setState({ draggingOut: false }));
    },
    onPointerUp: () => {
      start.current = null;
    },
  };
}

export function ShelfView() {
  const items = useShelf((s) => s.snapshot?.items ?? []);
  const dropHover = useShelf((s) => s.dropHover);
  const [status, setStatus] = useState<string | null>(null);
  const now = useMinuteClock();
  const dragAll = useDragOut([], setStatus);
  const available = items.filter((i) => i.exists).length;

  const run = (action: Promise<unknown>, done?: string) =>
    void action.then(
      () => setStatus(done ?? null),
      (e) => setStatus(message(e)),
    );

  if (items.length === 0 || dropHover) {
    return (
      <div className={`shelf-drop${dropHover ? " shelf-drop--active" : ""}`}>
        <EmptyState
          title={dropHover ? "Drop to park them here" : "Drop files on the notch"}
          body={
            dropHover
              ? "They stay where they are on disk — the shelf just remembers them."
              : "Drag files onto the notch to keep them handy, then drag them out into any app."
          }
        />
      </div>
    );
  }

  return (
    <div className="clips">
      <div className="clips-bar">
        {available > 0 && (
          <span className="shelf-grip" title="Drag all files out" {...dragAll}>
            <GripVertical size={14} aria-hidden /> Drag all
          </span>
        )}
        <span className="clips-bar__spacer" />
        {available > 0 && (
          <IconButton
            icon={Copy}
            label="Copy all files (then paste anywhere)"
            size="sm"
            onClick={() => run(ipc.shelfCopy(), "Copied — paste them anywhere")}
          />
        )}
        <ConfirmButton confirmLabel="Clear shelf?" onConfirm={() => run(ipc.shelfClear())}>
          Clear
        </ConfirmButton>
      </div>
      {status && <p className="clips-status">{status}</p>}
      <ul className="clips-list" aria-label="Files on the shelf">
        {items.map((item) => (
          <ShelfRow key={item.id} item={item} now={now} onStatus={setStatus} run={run} />
        ))}
      </ul>
    </div>
  );
}

function ShelfRow({
  item,
  now,
  onStatus,
  run,
}: {
  item: ShelfItem;
  now: number;
  onStatus: (message: string) => void;
  run: (action: Promise<unknown>, done?: string) => void;
}) {
  const drag = useDragOut([item.id], onStatus);
  const meta = !item.exists
    ? "Missing — moved or deleted"
    : [
        item.isDir ? "Folder" : item.ext?.toUpperCase(),
        item.size != null ? formatSize(item.size) : null,
        age(item.addedAt, now),
      ]
        .filter(Boolean)
        .join(" · ");

  return (
    <li className={`clip shelf-item${item.exists ? "" : " shelf-item--missing"}`} title={item.path}>
      <div className="clip__main shelf-item__drag" {...(item.exists ? drag : {})}>
        <span className="clip__icon" aria-hidden>
          <ItemIcon item={item} />
        </span>
        <span className="clip__text">
          <span className="clip__title">{item.name}</span>
          <span className="clip__meta">{meta}</span>
        </span>
      </div>
      <div className="clip__actions">
        {item.exists && (
          <>
            <IconButton
              icon={Copy}
              label={`Copy ${item.name}`}
              size="sm"
              onClick={() => run(ipc.shelfCopy([item.id]), "Copied — paste it anywhere")}
            />
            <IconButton
              icon={FolderOpen}
              label="Show in folder"
              size="sm"
              onClick={() => run(ipc.shelfReveal(item.id))}
            />
          </>
        )}
        <IconButton
          icon={X}
          label={`Remove ${item.name} from the shelf`}
          size="sm"
          onClick={() => run(ipc.shelfRemove([item.id]))}
        />
      </div>
    </li>
  );
}

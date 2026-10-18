// The drop shelf: files dragged onto the notch wait here. Drag one (or "Drag all") out into
// any app, or copy them and paste. The shelf only remembers where files are, so moved or
// deleted ones show as missing. Images can be converted or shrunk, PDFs merged; downloads in
// progress show on top, and the recorder saves what the PC plays here too.
import {
  Circle,
  Clapperboard,
  Film,
  Copy,
  Files,
  FolderOpen,
  GripVertical,
  Pin,
  PinOff,
  Smartphone,
  Square,
  Wand2,
  X,
} from "lucide-react";
import { useEffect, useState } from "react";

import type { RecentDownload } from "../../core/generated/RecentDownload";
import type { ShelfItem } from "../../core/generated/ShelfItem";
import { ipc, isIpcError } from "../../core/ipc";
import { Button, ConfirmButton, EmptyState, IconButton } from "../../design/components";
import { age } from "./clips";
import { FileIcon } from "./FileIcon";
import { convertTargets, isConvertible, mergeablePdfs } from "./fileTools";
import { formatSize, useShelf } from "./shelfStore";
import { BridgePanel } from "./BridgePanel";
import { useDragOut } from "./useDragOut";
import { useMinuteClock } from "./useMinuteClock";

function message(error: unknown) {
  return isIpcError(error)
    ? error.message.replace(/^invalid input: /, "")
    : "Something went wrong. Try again.";
}

/** Downloads finished in the last day: open, show in the folder, or drag into an app. */
function FinishedDownloads() {
  const busy = useShelf((s) => s.downloads.length);
  const [list, setList] = useState<RecentDownload[]>([]);
  const [open, setOpen] = useState(false);
  // Read again whenever a download finishes (the in-progress count changes).
  useEffect(() => {
    let live = true;
    ipc.filesRecentDownloads().then(
      (l) => live && setList(l),
      () => {},
    );
    return () => {
      live = false;
    };
  }, [busy]);
  if (list.length === 0) return null;
  const shown = open ? list : list.slice(0, 3);
  return (
    <div className="downloads downloads--done" aria-label="Recent downloads">
      <p className="downloads__head">
        Downloaded today
        {list.length > 3 && (
          <button type="button" className="downloads__more" onClick={() => setOpen(!open)}>
            {open ? "Less" : `All ${list.length}`}
          </button>
        )}
      </p>
      {shown.map((d) => (
        <div
          key={d.path}
          className="download download--done"
          draggable
          onDragStart={(e) => {
            e.preventDefault();
            void ipc.filesDrag([d.path]).catch(() => {});
          }}
        >
          <button
            type="button"
            className="download__name download__open"
            title={`Open ${d.name}`}
            onClick={() => void ipc.filesOpen(d.path).catch(() => {})}
          >
            {d.name}
          </button>
          <span className="download__meta">{formatSize(d.bytes)}</span>
          <IconButton
            icon={FolderOpen}
            label="Show in folder"
            size="sm"
            onClick={() => void ipc.filesReveal(d.path).catch(() => {})}
          />
        </div>
      ))}
    </div>
  );
}

function Downloads() {
  const downloads = useShelf((s) => s.downloads);
  if (downloads.length === 0) return <FinishedDownloads />;
  return (
    <div className="downloads" role="status" aria-label="Downloads in progress">
      {downloads.map((d) => (
        <div key={d.name}>
          <div className="download">
            <span className="download__name">{d.name}</span>
            <span className="download__meta">
              {formatSize(d.bytes)}
              {d.speed > 0 && ` · ${formatSize(d.speed)}/s`}
            </span>
          </div>
          <div className="download__bar" aria-hidden>
            <span />
          </div>
        </div>
      ))}
    </div>
  );
}

function RecordButton({ onStatus }: { onStatus: (text: string | null) => void }) {
  const recording = useShelf((s) => s.recorder.recording);
  const [busy, setBusy] = useState(false);
  const toggle = async () => {
    setBusy(true);
    try {
      if (recording) {
        onStatus("Saving the recording…");
        await ipc.recorderStop();
        onStatus(null);
      } else {
        await ipc.recorderStart();
        onStatus("Recording what your PC plays. Press stop to save it here.");
      }
    } catch (e) {
      onStatus(message(e));
    } finally {
      setBusy(false);
    }
  };
  return (
    <IconButton
      icon={recording ? Square : Circle}
      label={recording ? "Stop and save the recording" : "Record what your PC plays (saved as MP3)"}
      size="sm"
      disabled={busy}
      active={recording}
      onClick={() => void toggle()}
    />
  );
}

/** Record the screen under the pointer: MP4, or a short GIF. */
function ScreenRecButtons({ onStatus }: { onStatus: (text: string | null) => void }) {
  const rec = useShelf((s) => s.screenRec);
  const recording = rec.startedAt != null;
  const start = (gif: boolean) =>
    void ipc.screenrecStart(gif).then(
      () =>
        onStatus(
          gif
            ? "Recording a GIF of this screen (up to 30 s). Press stop to save it here."
            : "Recording this screen. Press stop to save it here.",
        ),
      (e) => onStatus(message(e)),
    );
  if (recording)
    return (
      <IconButton
        icon={Square}
        label={rec.gif ? "Stop and save the GIF" : "Stop and save the screen recording"}
        size="sm"
        active
        onClick={() => {
          onStatus("Saving…");
          void ipc.screenrecStop().then(
            () => onStatus(null),
            (e) => onStatus(message(e)),
          );
        }}
      />
    );
  return (
    <>
      <IconButton
        icon={Clapperboard}
        label="Record the screen (MP4)"
        size="sm"
        onClick={() => start(false)}
      />
      <IconButton
        icon={Film}
        label="Record a short GIF of the screen"
        size="sm"
        onClick={() => start(true)}
      />
    </>
  );
}

export function ShelfView() {
  const items = useShelf((s) => s.snapshot?.items ?? []);
  const dropHover = useShelf((s) => s.dropHover);
  const bridge = useShelf((s) => s.bridge);
  const [status, setStatus] = useState<string | null>(null);
  const now = useMinuteClock();
  const dragAll = useDragOut(
    () => ipc.shelfDrag([]),
    (e) => setStatus(message(e)),
  );
  const available = items.filter((i) => i.exists).length;
  const pdfs = mergeablePdfs(items);

  const run = (action: Promise<unknown>, done?: string) =>
    void action.then(
      () => setStatus(done ?? null),
      (e) => setStatus(message(e)),
    );

  const toPhone = (paths: string[]) => run(ipc.bridgeStart(paths));

  if (bridge) {
    return (
      <div className="clips">
        <BridgePanel bridge={bridge} />
        {status && <p className="clips-status">{status}</p>}
      </div>
    );
  }

  if (items.length === 0 || dropHover) {
    return (
      <div className="clips">
        <Downloads />
        <div className={`shelf-drop${dropHover ? " shelf-drop--active" : ""}`}>
          <EmptyState
            title={dropHover ? "Drop to park them here" : "Drop files on the notch"}
            body={
              dropHover
                ? "They stay where they are on disk — the shelf just remembers them."
                : "Drag files onto the notch to keep them handy, convert pictures or merge PDFs, then drag them out into any app."
            }
            action={
              dropHover ? undefined : (
                <span className="settings-inline">
                  <Button onClick={() => toPhone([])}>
                    <Smartphone size={14} aria-hidden /> Receive from phone
                  </Button>
                  <RecordButton onStatus={setStatus} />
                  <ScreenRecButtons onStatus={setStatus} />
                </span>
              )
            }
          />
        </div>
        {status && <p className="clips-status">{status}</p>}
      </div>
    );
  }

  return (
    <div className="clips">
      <Downloads />
      <div className="clips-bar">
        {available > 0 && (
          <span className="shelf-grip" title="Drag all files out" {...dragAll}>
            <GripVertical size={14} aria-hidden /> Drag all
          </span>
        )}
        <span className="clips-bar__spacer" />
        {pdfs.length >= 2 && (
          <IconButton
            icon={Files}
            label={`Merge the ${pdfs.length} PDFs into one (top to bottom)`}
            size="sm"
            onClick={() => run(ipc.filesMergePdfs(pdfs), "Merged — the new PDF is at the top")}
          />
        )}
        <RecordButton onStatus={setStatus} />
        <ScreenRecButtons onStatus={setStatus} />
        <IconButton
          icon={Smartphone}
          label={
            available > 0 ? "Send all to your phone (or receive from it)" : "Receive files from your phone"
          }
          size="sm"
          onClick={() => toPhone(items.filter((i) => i.exists).map((i) => i.path))}
        />
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
  const [tools, setTools] = useState(false);
  const drag = useDragOut(
    () => ipc.shelfDrag([item.id]),
    (e) => onStatus(message(e)),
  );
  const meta = !item.exists
    ? "Missing — moved or deleted"
    : [
        item.isDir ? "Folder" : item.ext?.toUpperCase(),
        item.size != null ? formatSize(item.size) : null,
        age(item.addedAt, now),
      ]
        .filter(Boolean)
        .join(" · ");
  const image = item.exists && !item.isDir && isConvertible(item.ext);

  return (
    <li className={`clip shelf-item${item.exists ? "" : " shelf-item--missing"}`} title={item.path}>
      <div className="shelf-item__row">
        <div className="clip__main shelf-item__drag" {...(item.exists ? drag : {})}>
          <span className="clip__icon" aria-hidden>
            <FileIcon ext={item.ext} isDir={item.isDir} />
          </span>
          <span className="clip__text">
            <span className="clip__title">{item.name}</span>
            <span className="clip__meta">{meta}</span>
          </span>
        </div>
        <div className="clip__actions">
          {image && (
            <IconButton
              icon={Wand2}
              label={`Convert or shrink ${item.name}`}
              size="sm"
              active={tools}
              onClick={() => setTools((t) => !t)}
            />
          )}
          {item.exists && (
            <>
              <IconButton
                icon={Copy}
                label={`Copy ${item.name}`}
                size="sm"
                onClick={() => run(ipc.shelfCopy([item.id]), "Copied — paste it anywhere")}
              />
              <IconButton
                icon={Smartphone}
                label={`Send ${item.name} to your phone`}
                size="sm"
                onClick={() => run(ipc.bridgeStart([item.path]))}
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
            icon={item.pinned ? PinOff : Pin}
            label={item.pinned ? `Let ${item.name} go with the clean-up` : `Keep ${item.name} on the shelf`}
            size="sm"
            active={item.pinned}
            onClick={() => run(ipc.shelfPin(item.id, !item.pinned))}
          />
          <IconButton
            icon={X}
            label={`Remove ${item.name} from the shelf`}
            size="sm"
            onClick={() => run(ipc.shelfRemove([item.id]))}
          />
        </div>
      </div>
      {image && tools && (
        <div className="shelf-tools" role="group" aria-label={`Tools for ${item.name}`}>
          {convertTargets(item.ext).map((to) => (
            <Button
              key={to}
              onClick={() => {
                setTools(false);
                run(
                  ipc.filesConvertImage(item.path, to),
                  `Saved a ${to.toUpperCase()} copy next to the original`,
                );
              }}
            >
              {to === "jpg" ? "JPG" : to === "png" ? "PNG" : "WebP"}
            </Button>
          ))}
          <Button
            onClick={() => {
              setTools(false);
              run(ipc.filesShrinkImage(item.path), "Saved a smaller copy, ready to share");
            }}
          >
            Smaller copy
          </Button>
        </div>
      )}
    </li>
  );
}

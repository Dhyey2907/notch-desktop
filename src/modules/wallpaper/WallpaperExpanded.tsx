// Recent | Saved | My folder, a screen switch, and a strip of tall cards: the one under the
// pointer opens wide. Click a card to make it the wallpaper on the chosen screen(s).
// Pictures dropped on the open panel are saved.
import { convertFileSrc } from "@tauri-apps/api/core";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { Check, FolderOpen, ImagePlus, X } from "lucide-react";
import { useEffect, useRef, useState, type MouseEvent, type WheelEvent } from "react";

import type { Wallpaper } from "../../core/generated/Wallpaper";
import { ipc, isIpcError } from "../../core/ipc";
import { EmptyState, IconButton, ModuleContainer, SegmentedControl, Button } from "../../design/components";
import { claimDrops } from "../../shell/dropOwner";
import {
  effectiveTarget,
  isCurrent,
  itemsFor,
  picturesOnly,
  screenOptions,
  shownTab,
  targetLabel,
  useWallpaper,
  type WallpaperTab,
} from "./wallpaperStore";
import "./wallpaper.css";

const STATUS_MS = 3200;

function message(error: unknown): string {
  return isIpcError(error) ? error.message : "Something went wrong";
}

export default function WallpaperExpanded() {
  const state = useWallpaper((s) => s.state);
  const tab = shownTab(
    useWallpaper((s) => s.tab),
    state,
  );
  const [status, setStatus] = useState<string | null>(null);
  const [applying, setApplying] = useState<string | null>(null);
  const [dropping, setDropping] = useState(false);
  const strip = useRef<HTMLDivElement>(null);

  // Re-read the folders each time the panel opens.
  useEffect(() => {
    ipc.wallpaperState().then(
      (next) => useWallpaper.getState().set(next),
      (error) => setStatus(message(error)),
    );
  }, []);

  useEffect(() => {
    if (!status) return;
    const timer = window.setTimeout(() => setStatus(null), STATUS_MS);
    return () => window.clearTimeout(timer);
  }, [status]);

  // Pictures dropped on the open panel become saved wallpapers.
  useEffect(() => {
    const release = claimDrops("wallpaper");
    let webview;
    try {
      webview = getCurrentWebview();
    } catch {
      return release;
    }
    const unlisten = webview.onDragDropEvent(({ payload }) => {
      if (payload.type === "enter") setDropping(true);
      else if (payload.type === "leave") setDropping(false);
      else if (payload.type === "drop") {
        setDropping(false);
        const pictures = picturesOnly(payload.paths);
        if (pictures.length === 0) {
          setStatus("Drop JPG, PNG, WebP or BMP pictures");
          return;
        }
        ipc.wallpaperAdd(pictures).then(
          (added) => {
            useWallpaper.getState().setTab("saved");
            setStatus(added ? `Saved ${added} wallpaper${added === 1 ? "" : "s"}` : "Already saved");
          },
          (error) => setStatus(message(error)),
        );
      }
    });
    return () => {
      release();
      void unlisten.then((off) => off());
    };
  }, []);

  const target = effectiveTarget(state);
  const items = itemsFor(state, tab);
  const screens = screenOptions(state);

  const apply = (w: Wallpaper) => {
    setApplying(w.path);
    ipc
      .wallpaperApply(w.path, target)
      .then(
        () => setStatus(`Set on ${targetLabel(state, target)}`),
        (error) => setStatus(message(error)),
      )
      .finally(() => setApplying(null));
  };

  const add = () =>
    ipc.wallpaperPick().then(
      (added) => {
        if (!added) return;
        useWallpaper.getState().setTab("saved");
        setStatus(`Saved ${added} wallpaper${added === 1 ? "" : "s"}`);
      },
      (error) => setStatus(message(error)),
    );

  const pickFolder = () =>
    ipc.wallpaperPickFolder().then(
      (picked) => picked && useWallpaper.getState().setTab("folder"),
      (error) => setStatus(message(error)),
    );

  const remove = (w: Wallpaper) =>
    ipc.wallpaperRemove(w.path).then(
      () => setStatus("Removed"),
      (error) => setStatus(message(error)),
    );

  // The wheel moves the strip sideways.
  const onWheel = (e: WheelEvent<HTMLDivElement>) => {
    if (strip.current && Math.abs(e.deltaY) > Math.abs(e.deltaX)) strip.current.scrollLeft += e.deltaY;
  };

  return (
    <ModuleContainer
      title="Wallpaper"
      trailing={
        <SegmentedControl<WallpaperTab>
          label="Wallpapers"
          value={tab}
          options={[
            { value: "recent", label: "Recent" },
            { value: "saved", label: state?.saved.length ? `Saved · ${state.saved.length}` : "Saved" },
            { value: "folder", label: "My folder" },
          ]}
          onChange={(t) => useWallpaper.getState().setTab(t)}
        />
      }
    >
      <div className={`walls${dropping ? " walls--drop" : ""}`}>
        <div className="walls__bar">
          {screens.length > 0 && (
            <SegmentedControl<string>
              label="Change the wallpaper on"
              value={target}
              options={screens}
              onChange={(next) => {
                void ipc.wallpaperSetTarget(next).catch((error) => setStatus(message(error)));
              }}
            />
          )}
          <span className="walls__status" role="status">
            {status ?? (tab === "folder" && state?.folderPath ? folderName(state.folderPath) : "")}
          </span>
          {tab === "folder" && state?.folderPath && (
            <IconButton
              icon={FolderOpen}
              label="Choose another folder"
              size="sm"
              onClick={() => void pickFolder()}
            />
          )}
          <IconButton icon={ImagePlus} label="Add wallpapers" size="sm" onClick={() => void add()} />
        </div>

        {items.length === 0 ? (
          <Empty tab={tab} hasFolder={Boolean(state?.folderPath)} onAdd={add} onPickFolder={pickFolder} />
        ) : (
          <div className="walls__strip" ref={strip} onWheel={onWheel} aria-label="Wallpapers">
            {items.map((w) => (
              <Card
                key={w.path}
                wallpaper={w}
                current={isCurrent(state, w.path)}
                busy={applying === w.path}
                onApply={() => apply(w)}
                onRemove={w.saved && tab !== "recent" ? () => void remove(w) : undefined}
              />
            ))}
          </div>
        )}
        {dropping && <div className="walls__drop">Drop pictures to save them</div>}
      </div>
    </ModuleContainer>
  );
}

function folderName(path: string): string {
  return path.split(/[\\/]/).filter(Boolean).pop() ?? path;
}

function Card({
  wallpaper,
  current,
  busy,
  onApply,
  onRemove,
}: {
  wallpaper: Wallpaper;
  current: boolean;
  busy: boolean;
  onApply: () => void;
  onRemove?: () => void;
}) {
  const onContextMenu = (e: MouseEvent) => {
    if (!onRemove) return;
    e.preventDefault();
    onRemove();
  };
  return (
    <div className={`wall${current ? " wall--current" : ""}${busy ? " wall--busy" : ""}`}>
      <button
        type="button"
        className="wall__pick"
        onClick={onApply}
        onContextMenu={onContextMenu}
        aria-label={`Set ${wallpaper.name || "this picture"} as wallpaper`}
        title={wallpaper.name}
      >
        {wallpaper.thumb ? (
          <img src={convertFileSrc(wallpaper.thumb)} alt="" draggable={false} />
        ) : (
          <span className="wall__loading" aria-hidden />
        )}
      </button>
      {current && (
        <span className="wall__check" aria-label="Current wallpaper">
          <Check size={12} strokeWidth={3} aria-hidden />
        </span>
      )}
      {onRemove && (
        <button
          type="button"
          className="wall__remove"
          aria-label="Remove from saved"
          title="Remove"
          onClick={onRemove}
        >
          <X size={12} strokeWidth={2.5} aria-hidden />
        </button>
      )}
    </div>
  );
}

function Empty({
  tab,
  hasFolder,
  onAdd,
  onPickFolder,
}: {
  tab: WallpaperTab;
  hasFolder: boolean;
  onAdd: () => void;
  onPickFolder: () => void;
}) {
  if (tab === "folder") {
    return hasFolder ? (
      <EmptyState
        title="No pictures here"
        body="This folder has no JPG, PNG, WebP or BMP pictures."
        action={<Button onClick={onPickFolder}>Choose another folder</Button>}
      />
    ) : (
      <EmptyState
        title="Choose a folder"
        body="Every picture in it shows up here, newest first."
        action={<Button onClick={onPickFolder}>Choose folder</Button>}
      />
    );
  }
  if (tab === "recent") {
    return <EmptyState title="Nothing set yet" body="Wallpapers you set from here show up in Recent." />;
  }
  return (
    <EmptyState
      title="Add a few photos"
      body="Click + or drop pictures here. Overline keeps its own copies."
      action={
        <Button variant="primary" onClick={onAdd}>
          Add photos
        </Button>
      }
    />
  );
}

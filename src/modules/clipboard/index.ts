// Clipboard module: history of what you copy (text, pictures, files) to copy again, pin, or
// turn into a task or note, plus the drop shelf: files dragged onto the notch, parked until
// you drag them out again. Ctrl+Alt+V opens it. Nothing pops up per copy.
import { ClipboardList } from "lucide-react";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { lazy } from "react";

import { ipc } from "../../core/ipc";
import type { ModuleContext, NotchModule } from "../types";
import { ClipboardPeek, FilesCompact } from "./ClipboardIndicators";
import { useClipboard } from "./clipboardStore";
import { useFileDrag } from "../../shell/fileDrag";
import { formatSize, useShelf } from "./shelfStore";
import { suggestionCard } from "./suggestions";
import { useReadLater } from "./readLaterStore";

/** Files dropped on this island go on the shelf. */
function watchDrops(): () => void {
  let webview;
  try {
    webview = getCurrentWebview();
  } catch {
    return () => {};
  }
  const unlisten = webview.onDragDropEvent(({ payload }) => {
    if (useFileDrag.getState().draggingOut) return;
    const shelf = useShelf.getState();
    if (payload.type === "enter") {
      shelf.setTab("files");
      shelf.setDropHover(true);
    } else if (payload.type === "drop") {
      shelf.setDropHover(false);
      if (payload.paths.length > 0)
        ipc
          .shelfAdd(payload.paths)
          .catch((error) => console.error("could not add files to the shelf", error));
    } else if (payload.type === "leave") {
      shelf.setDropHover(false);
    }
  });
  return () => void unlisten.then((off) => off());
}

function init({ events, activities }: ModuleContext) {
  const offClips = events.on("ClipboardChanged", ({ event }) => useClipboard.getState().set(event.payload));
  const offShelf = events.on("ShelfChanged", ({ event }) => useShelf.getState().set(event.payload));
  const offReadLater = events.on("ReadLaterChanged", ({ event }) =>
    useReadLater.getState().set(event.payload),
  );
  ipc.readLaterSnapshot().then(
    (snapshot) => {
      if (!useReadLater.getState().snapshot) useReadLater.getState().set(snapshot);
    },
    () => {},
  );
  ipc
    .clipboardSnapshot()
    .then((snapshot) => {
      if (!useClipboard.getState().snapshot) useClipboard.getState().set(snapshot);
    })
    .catch((error) => console.error("failed to read clipboard history", error));
  ipc
    .shelfSnapshot()
    .then((snapshot) => {
      if (!useShelf.getState().snapshot) useShelf.getState().set(snapshot);
    })
    .catch((error) => console.error("failed to read the shelf", error));
  const offDrops = watchDrops();
  const offBridge = events.on("BridgeChanged", ({ event }) => useShelf.getState().setBridge(event.payload));
  const offReceived = events.on("BridgeReceived", ({ event }) =>
    activities.push({
      moduleId: "clipboard",
      priority: "normal",
      tone: "success",
      countRepeats: false,
      title: `Received ${event.payload.name}`,
      body: "From your phone · saved to Downloads\\Notch and on the Files shelf",
    }),
  );
  ipc.bridgeState().then(
    (b) => useShelf.getState().setBridge(b),
    () => {},
  );
  const offDownloads = events.on("DownloadsChanged", ({ event }) =>
    useShelf.setState({ downloads: event.payload }),
  );
  const offDone = events.on("DownloadDone", ({ event }) => {
    const { name, path, size } = event.payload;
    activities.push({
      moduleId: "clipboard",
      priority: "normal",
      tone: "success",
      dedupKey: `download-${path}`,
      countRepeats: false,
      title: `Downloaded ${name}`,
      body: formatSize(size),
      actions: [
        { id: "shelf", label: "Keep on shelf", variant: "secondary", run: () => void ipc.shelfAdd([path]) },
        { id: "folder", label: "Show", variant: "secondary", run: () => ipc.filesReveal(path) },
        { id: "open", label: "Open", variant: "primary", run: () => ipc.filesOpen(path) },
      ],
    });
  });
  const offRecorder = events.on("RecorderChanged", ({ event }) =>
    useShelf.setState({ recorder: event.payload }),
  );
  const offSaved = events.on("RecordingSaved", ({ event }) => {
    const { name, path } = event.payload;
    activities.push({
      moduleId: "clipboard",
      priority: "normal",
      tone: "success",
      countRepeats: false,
      title: "Recording saved",
      body: `${name} · on the Files shelf`,
      actions: [
        { id: "folder", label: "Show", variant: "secondary", run: () => ipc.filesReveal(path) },
        { id: "open", label: "Play", variant: "primary", run: () => ipc.filesOpen(path) },
      ],
    });
  });
  ipc.filesDownloads().then(
    (downloads) => useShelf.setState({ downloads }),
    () => {},
  );
  ipc.recorderState().then(
    (recorder) => useShelf.setState({ recorder }),
    () => {},
  );
  const offCleaned = events.on("LinkCleaned", ({ event }) =>
    activities.push({
      moduleId: "clipboard",
      priority: "low",
      tone: "success",
      dedupKey: "link-cleaned",
      countRepeats: false,
      timeoutMs: 2500,
      title: "Link cleaned",
      body: `Removed ${event.payload.removed} tracking ${event.payload.removed === 1 ? "tag" : "tags"} before you paste it`,
    }),
  );
  const offGrabbed = events.on("TextGrabbed", ({ event }) => {
    const text = event.payload.text.trim();
    activities.push({
      moduleId: "clipboard",
      priority: "normal",
      tone: text ? "success" : "warning",
      dedupKey: "grabbed",
      countRepeats: false,
      title: text ? "Copied text from the screen" : "No text found there",
      body: text ? text.replace(/\s+/g, " ").slice(0, 90) : "Try a bigger box around the words.",
    });
  });
  const offSuggest = events.on("CopySuggestion", ({ event }) => {
    const card = suggestionCard(event.payload, {
      remind: ipc.todoAddReminder,
      open: ipc.openExternal,
      timeZone: Intl.DateTimeFormat().resolvedOptions().timeZone,
    });
    if (card) activities.push(card);
  });
  return () => {
    offClips();
    offShelf();
    offReadLater();
    offDrops();
    offBridge();
    offReceived();
    offSuggest();
    offDownloads();
    offDone();
    offRecorder();
    offSaved();
    offCleaned();
    offGrabbed();
  };
}

export const clipboardModule: NotchModule = {
  id: "clipboard",
  name: "Clipboard",
  icon: ClipboardList,
  order: 55,
  init,
  Peek: ClipboardPeek,
  Compact: FilesCompact,
  Expanded: lazy(() => import("./ClipboardExpanded")),
};

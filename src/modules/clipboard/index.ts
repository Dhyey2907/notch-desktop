// Clipboard module: history of what you copy (text, pictures, files) to copy again, pin, or
// turn into a task or note, plus the drop shelf: files dragged onto the notch, parked until
// you drag them out again. Ctrl+Alt+V opens it. Nothing pops up per copy.
import { ClipboardList } from "lucide-react";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { lazy } from "react";

import { ipc } from "../../core/ipc";
import type { ModuleContext, NotchModule } from "../types";
import { ClipboardPeek } from "./ClipboardIndicators";
import { useClipboard } from "./clipboardStore";
import { useFileDrag } from "../../shell/fileDrag";
import { useShelf } from "./shelfStore";

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

function init({ events }: ModuleContext) {
  const offClips = events.on("ClipboardChanged", ({ event }) => useClipboard.getState().set(event.payload));
  const offShelf = events.on("ShelfChanged", ({ event }) => useShelf.getState().set(event.payload));
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
  return () => {
    offClips();
    offShelf();
    offDrops();
  };
}

export const clipboardModule: NotchModule = {
  id: "clipboard",
  name: "Clipboard",
  icon: ClipboardList,
  order: 55,
  init,
  Peek: ClipboardPeek,
  Expanded: lazy(() => import("./ClipboardExpanded")),
};

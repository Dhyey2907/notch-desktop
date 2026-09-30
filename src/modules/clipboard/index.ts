// Clipboard module: history of what you copy (text, pictures, files) to copy again, pin, or
// turn into a task or note. Ctrl+Alt+V opens it. Nothing pops up per copy.
import { ClipboardList } from "lucide-react";
import { lazy } from "react";

import { ipc } from "../../core/ipc";
import type { ModuleContext, NotchModule } from "../types";
import { ClipboardPeek } from "./ClipboardIndicators";
import { useClipboard } from "./clipboardStore";

function init({ events }: ModuleContext) {
  const off = events.on("ClipboardChanged", ({ event }) => useClipboard.getState().set(event.payload));
  ipc
    .clipboardSnapshot()
    .then((snapshot) => {
      if (!useClipboard.getState().snapshot) useClipboard.getState().set(snapshot);
    })
    .catch((error) => console.error("failed to read clipboard history", error));
  return off;
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

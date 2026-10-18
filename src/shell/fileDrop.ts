// Files dragged over the notch: open it on the Clipboard module (whose Files tab takes the
// drop), and put it away again if the drag leaves without dropping. An open Wallpaper panel
// keeps the drop for itself (pictures become saved wallpapers).
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect } from "react";

import { findModule } from "../modules/registry";
import { dropTakenByOpenModule } from "./dropOwner";
import { useFileDrag } from "./fileDrag";
import { useIsland } from "./useIsland";
import { useModuleNav } from "./useModules";

const DROP_MODULE = "clipboard";

export function useFileDrop() {
  useEffect(() => {
    let openedForDrop = false;
    let dropped = false;
    let webview;
    try {
      webview = getCurrentWebview();
    } catch {
      return; // tests and non-Tauri previews
    }
    const unlisten = webview.onDragDropEvent(({ payload }) => {
      if (useFileDrag.getState().draggingOut || !findModule(DROP_MODULE) || dropTakenByOpenModule()) return;
      const island = useIsland.getState();
      if (payload.type === "enter") {
        dropped = false;
        useModuleNav.getState().setActive(DROP_MODULE);
        if (island.mode !== "expanded") {
          openedForDrop = true;
          void island.request("expanded");
        }
      } else if (payload.type === "drop") {
        dropped = true;
        openedForDrop = false;
        // Focus it so clicking elsewhere puts it away, like any other open.
        void getCurrentWindow().setFocus();
      } else if (payload.type === "leave") {
        if (openedForDrop && !dropped) void island.request("collapsed");
        openedForDrop = false;
      }
    });
    return () => void unlisten.then((off) => off());
  }, []);
}

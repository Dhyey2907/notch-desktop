import { useActivities } from "../../activity";
import type { DeskWindow } from "../../core/generated/DeskWindow";

/** Show a live view of a window in the notch (until closed or the notch is opened). */
export function showMini(w: DeskWindow) {
  useActivities.getState().push({
    moduleId: "desk",
    priority: "low",
    persistent: true,
    dedupKey: "mini-window",
    countRepeats: false,
    title: w.title,
    mini: { id: w.id, title: w.title, app: w.app },
  });
}

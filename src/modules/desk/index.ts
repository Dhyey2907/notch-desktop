// Desk: your open windows (pin on top, see-through, a live mini view in the notch) and
// routines — one click opens apps, sites and folders, closes clutter and starts focus.
import { AppWindow } from "lucide-react";
import { lazy } from "react";

import type { ModuleContext, NotchModule } from "../types";

function init({ events, activities }: ModuleContext) {
  const offPinned = events.on("WindowPinned", ({ event }) => {
    const { title, on, ok } = event.payload;
    activities.push({
      moduleId: "desk",
      priority: "normal",
      tone: ok ? "neutral" : "warning",
      dedupKey: "window-pinned",
      countRepeats: false,
      timeoutMs: 2500,
      title: ok ? (on ? "Pinned on top" : "No longer on top") : "Couldn't pin that window",
      body: ok ? title : "It may be running as administrator.",
    });
  });
  const offRan = events.on("RoutineRan", ({ event }) => {
    const { name, done, failed, scheduled } = event.payload;
    activities.push({
      moduleId: "desk",
      priority: "normal",
      tone: failed ? "warning" : "success",
      dedupKey: `routine-${name}`,
      countRepeats: false,
      title: `${scheduled ? "Ran on schedule" : "Ran"}: ${name}`,
      body: failed ? `${done} done, ${failed} didn't work` : `${done} ${done === 1 ? "step" : "steps"} done`,
    });
  });
  return () => {
    offPinned();
    offRan();
  };
}

export const deskModule: NotchModule = {
  id: "desk",
  name: "Desk",
  icon: AppWindow,
  order: 34,
  init,
  Expanded: lazy(() => import("./DeskExpanded")),
};

// Desk: your open windows (pin on top, see-through, a live mini view in the notch) and
// routines — one click opens apps, sites and folders, closes clutter and starts focus.
import { AppWindow } from "lucide-react";
import { lazy } from "react";

import { ipc } from "../../core/ipc";
import { useSettings } from "../../settings";
import { habitRoutine, habitText } from "./habit";
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
  // Apps that keep starting together: offer to open them for you.
  const offHabit = events.on("HabitFound", ({ event }) => {
    const suggestion = event.payload;
    activities.push({
      moduleId: "desk",
      priority: "normal",
      tone: "neutral",
      dedupKey: "habit",
      countRepeats: false,
      persistent: true,
      title: "Make it a routine?",
      body: habitText(suggestion),
      actions: [
        {
          id: "no",
          label: "No thanks",
          variant: "secondary",
          run: () => void ipc.habitDismiss(suggestion.key).catch(() => {}),
        },
        {
          id: "make",
          label: "Make routine",
          variant: "primary",
          run: () =>
            void useSettings.getState().update((st) => ({
              ...st,
              desk: {
                ...st.desk,
                routines: [
                  ...st.desk.routines,
                  habitRoutine(suggestion, crypto.randomUUID(), st.day.workDays),
                ],
              },
            })),
        },
      ],
    });
  });
  // After a restart: offer to reopen what was open last time (Rust answers only once per run).
  ipc.workspaceResumeOffer().then(
    (apps) => {
      if (apps.length === 0) return;
      const names = apps.slice(0, 4).join(", ") + (apps.length > 4 ? ` and ${apps.length - 4} more` : "");
      activities.push({
        moduleId: "desk",
        priority: "normal",
        tone: "neutral",
        dedupKey: "resume-session",
        persistent: true,
        title: `Reopen ${apps.length} ${apps.length === 1 ? "app" : "apps"} from last time?`,
        body: names,
        actions: [
          { id: "reopen", label: "Reopen", run: () => void ipc.workspaceRestore("__last").catch(() => {}) },
          { id: "dismiss", label: "Not now", run: () => {} },
        ],
      });
    },
    () => {},
  );
  return () => {
    offPinned();
    offRan();
    offHabit();
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

// Calendar module (plan.md Phase 7): Google Calendar agenda, meeting countdowns, join links.
import { CalendarDays } from "lucide-react";
import { lazy } from "react";

import type { CalendarEvent } from "../../core/generated/CalendarEvent";
import { ipc } from "../../core/ipc";
import type { ModuleContext, NotchModule } from "../types";
import { CalendarCompact, CalendarPeek } from "./CalendarIndicators";
import { showInPetCloud } from "../../pet/petStore";
import { useIsland } from "../../shell/useIsland";
import { countdownText, useCalendar } from "./calendarStore";

/** Keep the countdown up this long after the meeting starts, so "Join" stays reachable. */
const LINGER_AFTER_START_MS = 5 * 60_000;

function init({ events, activities }: ModuleContext) {
  const timers = new Set<number>();

  /** A countdown that ticks every second until shortly after the meeting starts. */
  const remind = (event: CalendarEvent) => {
    const start = Date.parse(event.start);
    const remaining = () => start - Date.now();
    const id = activities.push({
      moduleId: "calendar",
      priority: "high",
      tone: "info",
      dedupKey: `reminder-${event.id}`,
      title: event.title,
      body: countdownText(remaining()),
      timeoutMs: Math.max(60_000, remaining() + LINGER_AFTER_START_MS),
      actions: [
        ...(event.joinUrl
          ? [
              {
                id: "join",
                label: "Join",
                variant: "primary" as const,
                run: () => ipc.calendarOpenLink(event.joinUrl!),
              },
            ]
          : []),
        // Video meetings: one click to start meeting notes (set up first if Whisper isn't there).
        ...(event.joinUrl
          ? [
              {
                id: "notes",
                label: "Take notes",
                run: () =>
                  void ipc.meetingState().then((s) => {
                    if (s.ready) void ipc.meetingStart().catch(() => {});
                    else {
                      // Loaded here: the module list imports this file (a cycle at load).
                      void import("../../shell/useModules").then(({ useModuleNav }) => {
                        useModuleNav.getState().setActive("meeting");
                        void useIsland.getState().request("expanded");
                      });
                    }
                  }),
              },
            ]
          : []),
        { id: "dismiss", label: "Dismiss", run: () => {} },
      ],
    });
    // Out on the taskbar, the pet tells you too.
    showInPetCloud(`${event.title} · ${countdownText(remaining())}`);
    const timer = window.setInterval(() => {
      activities.update(id, { body: countdownText(remaining()) });
      if (remaining() < -LINGER_AFTER_START_MS) {
        window.clearInterval(timer);
        timers.delete(timer);
      }
    }, 1000);
    timers.add(timer);
  };

  const offChanged = events.on("CalendarChanged", ({ event }) => useCalendar.getState().set(event.payload));
  const offReminder = events.on("ReminderDue", ({ event }) => remind(event.payload.event));
  ipc
    .calendarGetSnapshot()
    .then((snapshot) => {
      if (!useCalendar.getState().snapshot) useCalendar.getState().set(snapshot);
    })
    .catch((error) => console.error("failed to read calendar", error));

  return () => {
    offChanged();
    offReminder();
    timers.forEach((t) => window.clearInterval(t));
  };
}

export const calendarModule: NotchModule = {
  id: "calendar",
  name: "Calendar",
  icon: CalendarDays,
  order: 20,
  init,
  Compact: CalendarCompact,
  Peek: CalendarPeek,
  Expanded: lazy(() => import("./CalendarExpanded")),
};

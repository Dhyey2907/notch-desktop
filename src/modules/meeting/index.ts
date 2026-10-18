// Meeting notes: record a meeting (your mic and the PC's sound), transcribed here with Whisper,
// then an AI summary with action items (saved to Notes and To-do). Not in the dock: the palette,
// the meeting card and Settings open it; a red timer shows in the bubble while it records.
import { NotebookPen } from "lucide-react";
import { createElement, lazy } from "react";

import { ipc } from "../../core/ipc";
import type { ModuleContext, NotchModule } from "../types";
import { MeetingCompact } from "./MeetingCompact";
import { useMeeting } from "./meetingStore";

function init({ events, activities }: ModuleContext) {
  const offChanged = events.on("MeetingChanged", ({ event }) => useMeeting.getState().set(event.payload));
  const offDone = events.on("MeetingDone", ({ event }) => {
    const { title, actions, ok } = event.payload;
    activities.push({
      moduleId: "meeting",
      priority: "normal",
      tone: ok ? "success" : "warning",
      dedupKey: "meeting-done",
      countRepeats: false,
      persistent: ok,
      title: ok ? `Meeting notes ready: ${title}` : "Meeting notes didn't work",
      body: ok
        ? `Saved to Notes${actions > 0 ? `, with ${actions} action ${actions === 1 ? "item" : "items"} added to To-do` : ""}.`
        : title,
      actions: ok ? [{ id: "dismiss", label: "Got it", run: () => {} }] : undefined,
    });
  });
  ipc.meetingState().then(
    (s) => useMeeting.getState().set(s),
    () => {},
  );
  return () => {
    offChanged();
    offDone();
  };
}

export const meetingModule: NotchModule = {
  id: "meeting",
  name: "Meeting notes",
  icon: NotebookPen,
  order: 996,
  alwaysOn: true,
  dockless: true,
  init,
  Compact: () => createElement(MeetingCompact),
  Expanded: lazy(() => import("./MeetingExpanded")),
};

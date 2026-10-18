// Shows the morning briefing card once a day (see briefing.ts). Lead island only.
import { useEffect } from "react";

import { useActivities } from "../../activity";
import { ipc } from "../../core/ipc";
import { isLeadIsland } from "../../core/window";
import { dayKey } from "../../pet/petLife";
import { usePetLive } from "../../pet/petStore";
import { useSettings } from "../../settings";
import { useCalendar } from "../calendar/calendarStore";
import { useMail } from "../mail/mailStore";
import { useTodo } from "../todo/todoStore";
import { briefingDue, briefingLines, greeting } from "./briefing";

const CHECK_MS = 60_000;
/** "At the PC" = input within this long. */
const ACTIVE_MS = 90_000;
const SHOWN_KEY = "day-briefing-shown";

function lastShown(): string | null {
  try {
    return localStorage.getItem(SHOWN_KEY);
  } catch {
    return null;
  }
}

function markShown(day: string) {
  try {
    localStorage.setItem(SHOWN_KEY, day);
  } catch {
    // Not remembered: at worst it shows twice.
  }
}

/** Gather the morning's numbers and show the card (also "Morning briefing" in the palette). */
export function showBriefing() {
  const now = new Date();
  const today = dayKey(now);
  const events = (useCalendar.getState().snapshot?.events ?? []).filter(
    (e) => dayKey(new Date(e.start)) === today,
  );
  const tasks = useTodo.getState().snapshot?.tasks ?? [];
  const lines = briefingLines(
    {
      weather: usePetLive.getState().weather ?? null,
      events,
      openTasks: tasks.filter((t) => !t.done).length,
      unread: useMail.getState().state?.unread ?? null,
    },
    now,
  );
  useActivities.getState().push({
    moduleId: "day",
    priority: "normal",
    tone: "info",
    title: greeting(now),
    body: lines.join("  ·  "),
    dedupKey: "day-briefing",
    countRepeats: false,
    timeoutMs: 20_000,
  });
}

export function useMorningBriefing() {
  const lead = isLeadIsland();
  const on = useSettings((s) => s.settings && !s.settings.day.noBriefing);
  useEffect(() => {
    if (!lead || !on) return;
    const check = async () => {
      const now = new Date();
      const today = dayKey(now);
      if (lastShown() === today || now.getHours() < 5) return;
      const idle = await ipc.systemIdleMs().catch(() => Infinity);
      // Wait a moment after start so the weather, calendar and mail have loaded.
      if (!briefingDue(now, lastShown(), today, idle < ACTIVE_MS)) return;
      markShown(today);
      showBriefing();
    };
    const first = window.setTimeout(() => void check(), 20_000);
    const id = window.setInterval(() => void check(), CHECK_MS);
    return () => {
      window.clearTimeout(first);
      window.clearInterval(id);
    };
  }, [lead, on]);
}

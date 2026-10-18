// The evening review (Settings → Your day): at the review time, once a day, a card summing up
// the day. Also on demand from the command palette ("Review my day"). Lead island only.
import { useEffect } from "react";

import { useActivities } from "../../activity";
import { ipc } from "../../core/ipc";
import { isLeadIsland } from "../../core/window";
import { useFocus } from "../../focus/focusStore";
import { currentStreak, dayKey } from "../../pet/petLife";
import { useSettings } from "../../settings";
import { useCalendar } from "../calendar/calendarStore";
import { useTodo } from "../todo/todoStore";
import { reviewDue, reviewLines } from "./review";

const CHECK_MS = 60_000;
const SHOWN_KEY = "day-review-shown";

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

/** Gather today and show the review card. */
export async function showReview() {
  const now = new Date();
  const today = dayKey(now);
  const [coding, screen] = await Promise.all([
    ipc.codingStats().catch(() => null),
    ipc.screenTimeDay(today).catch(() => null),
  ]);
  const tasks = useTodo.getState().snapshot?.tasks ?? [];
  const tasksDone = tasks.filter((t) => t.done && t.doneAt && dayKey(new Date(t.doneAt)) === today).length;
  const meetings = (useCalendar.getState().snapshot?.events ?? []).filter(
    (e) => !e.allDay && dayKey(new Date(e.start)) === today,
  ).length;
  const pet = useSettings.getState().settings?.pet;
  const lines = reviewLines({
    tasksDone,
    focusSessions: useFocus.getState().state?.doneToday ?? 0,
    coding: coding?.today ?? [],
    screenMinutes: screen?.minutes ?? 0,
    meetings,
    petStreak: pet ? currentStreak(pet.streakDays, pet.lastPlayDay, now) : 0,
  });
  useActivities.getState().push({
    moduleId: "day",
    priority: "normal",
    tone: "info",
    title: "Your day",
    body: lines.length > 0 ? lines.join("  ·  ") : "A quiet day. Rest well!",
    dedupKey: "day-review",
    countRepeats: false,
    timeoutMs: 20_000,
  });
}

export function useEveningReview() {
  const lead = isLeadIsland();
  const on = useSettings((s) => s.settings && !s.settings.day.noReview);
  useEffect(() => {
    if (!lead || !on) return;
    const check = () => {
      const s = useSettings.getState().settings;
      if (!s || s.day.noReview) return;
      const now = new Date();
      const today = dayKey(now);
      if (!reviewDue(now, s.day.reviewAt, lastShown(), today)) return;
      markShown(today);
      void showReview();
    };
    check();
    const id = window.setInterval(check, CHECK_MS);
    return () => window.clearInterval(id);
  }, [lead, on]);
}

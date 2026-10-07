// Runs the break reminders on the main island: counts time at the PC, waits while you focus,
// play, sit in a meeting or have quiet hours, and shows one card when a break is due.
import { useEffect } from "react";

import { useActivities } from "../activity";
import { ipc } from "../core/ipc";
import { isLeadIsland } from "../core/window";
import { isOngoing, useCalendar } from "../modules/calendar/calendarStore";
import { useSettings } from "../settings";
import { inQuietHours } from "../shell/behavior";
import { useFullscreen } from "../shell/fullscreen";
import {
  BREAK_TEXT,
  currentStreak,
  recordBreak,
  reset,
  tick,
  ZERO,
  type BreakKind,
  type Timers,
} from "./breaks";
import { focusActive, useFocus } from "./focusStore";
import { tellPet } from "../pet/petStore";
import { cheerPet } from "./petCheer";

const STEP_MS = 30_000;
const SNOOZE_MS = 5 * 60_000;
const CARD_KEY = "break";

function held(now: number): boolean {
  const focus = useFocus.getState();
  if (focus.gaming || focusActive(focus.state, now) || useFullscreen.getState().active) return true;
  const settings = useSettings.getState().settings;
  if (settings && inQuietHours(settings.behavior.quiet, new Date(now))) return true;
  const events = useCalendar.getState().snapshot?.events ?? [];
  return events.some((e) => isOngoing(e, now));
}

export function useBreaks() {
  const lead = isLeadIsland();
  const ready = useSettings((s) => s.settings !== null);

  useEffect(() => {
    if (!lead || !ready) return;
    let timers: Timers = ZERO;
    let showing: BreakKind | null = null;
    let cardId: string | null = null;

    const close = () => {
      if (cardId) useActivities.getState().dismiss(cardId);
      cardId = null;
      showing = null;
    };

    const show = (kind: BreakKind) => {
      const settings = useSettings.getState().settings;
      if (!settings) return;
      showing = kind;
      const streak = currentStreak(settings.breaks, new Date());
      const text = BREAK_TEXT[kind];
      const finish = (snoozeMs = 0) => {
        const s = useSettings.getState().settings;
        if (s) timers = reset(timers, s.breaks, kind, snoozeMs);
        showing = null;
        cardId = null;
      };
      cardId = useActivities.getState().push({
        moduleId: "todo",
        priority: "normal",
        persistent: true,
        dedupKey: CARD_KEY,
        countRepeats: false,
        title: text.title,
        body: streak > 0 ? `${text.body} · ${streak}-day streak` : text.body,
        onClose: () => finish(),
        actions: [
          { id: "skip", label: "Skip", run: () => finish() },
          { id: "snooze", label: "5 min", run: () => finish(SNOOZE_MS) },
          {
            id: "done",
            label: "Done",
            variant: "primary",
            run: async () => {
              finish();
              cheerPet();
              const before = useSettings.getState().settings?.breaks.streak ?? 0;
              await useSettings
                .getState()
                .update((s) => ({ ...s, breaks: { ...s.breaks, ...recordBreak(s.breaks, new Date()) } }));
              const after = useSettings.getState().settings?.breaks.streak ?? 0;
              tellPet(
                after > before
                  ? { event: "streak", vars: { days: after }, xp: 2 }
                  : { event: "breakDone", xp: 2 },
              );
            },
          },
        ],
      });
    };

    const step = async () => {
      const settings = useSettings.getState().settings;
      if (!settings) return;
      const idleMs = await ipc.systemIdleMs().catch(() => 0);
      const now = Date.now();
      const result = tick(timers, settings.breaks, { stepMs: STEP_MS, idleMs, held: held(now) });
      timers = result.timers;
      // Walked away while the card was up: that was the break.
      if (showing && timers === ZERO) close();
      if (result.due && !showing) show(result.due);
    };

    const id = window.setInterval(() => void step(), STEP_MS);
    // Dev builds only: show a reminder now, for testing.
    if (import.meta.env.DEV)
      (window as unknown as { __notchBreak?: (k: BreakKind) => void }).__notchBreak = show;
    return () => {
      window.clearInterval(id);
      close();
    };
  }, [lead, ready]);
}

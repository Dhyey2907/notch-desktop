import { useEffect, useState } from "react";

import { ipc } from "../core/ipc";
import { usePetCheer } from "../focus/petCheer";
import { focusActive, useFocus } from "../focus/focusStore";
import { agentName, useClaude } from "../modules/claude/claudeStore";
import { isOngoing, useCalendar } from "../modules/calendar/calendarStore";
import { useControl } from "../modules/control/controlStore";
import { useDay } from "../modules/day/dayStore";
import { useMedia } from "../modules/media/mediaStore";
import { brandOf } from "../modules/media/mediaTheme";
import { useSettings } from "../settings";
import { petLook, type PetLook } from "./petState";
import { usePetLive } from "./petStore";

const IDLE_CHECK_MS = 60_000;
const BUSY_MEETINGS = 4;
const DEADLINE_SOON_MS = 6 * 3600_000;

/** How the pet looks right now, its name and color. */
export function usePetLook(): (PetLook & { name: string; color: string }) | null {
  const pet = useSettings((s) => s.settings?.pet);
  const focus = useFocus((s) => s.state);
  const cheeringUntil = usePetCheer((s) => s.until);
  const { lovedUntil, weather } = usePetLive();
  const media = useMedia((s) => s.media);
  const claude = useClaude((s) => s.status);
  const power = useControl((s) => s.snapshot?.power);
  const events = useCalendar((s) => s.snapshot?.events);
  const assignments = useDay((s) => s.state?.assignments);
  const [idleMs, setIdleMs] = useState(0);
  const [now, setNow] = useState(() => Date.now());

  useEffect(() => {
    const check = () =>
      ipc.systemIdleMs().then(
        (ms) => {
          setIdleMs(ms);
          setNow(Date.now());
        },
        () => {},
      );
    void check();
    const id = window.setInterval(() => void check(), IDLE_CHECK_MS);
    return () => window.clearInterval(id);
  }, []);

  // Start and stop timed faces (a cheer, a pat) on time.
  useEffect(() => {
    const until = Math.max(cheeringUntil, lovedUntil);
    const left = until - Date.now();
    if (left <= 0) return;
    const start = window.setTimeout(() => setNow(Date.now()), 0);
    const end = window.setTimeout(() => setNow(Date.now()), left + 50);
    return () => {
      window.clearTimeout(start);
      window.clearTimeout(end);
    };
  }, [cheeringUntil, lovedUntil]);

  if (!pet) return null;
  const playing = media?.status === "playing" && !media.mock;
  const brand = media ? brandOf(media) : null;
  const live = claude?.live ?? [];
  const waiting = live.find((l) => l.state === "waiting");
  const working = live.find((l) => l.state === "working");
  const today = new Date(now);
  const meetingsToday = (events ?? []).filter(
    (e) => !e.allDay && new Date(e.start).toDateString() === today.toDateString(),
  ).length;
  const deadlineSoon = (assignments ?? []).some((a) => {
    const due = new Date(a.due).getTime() - now;
    return due > 0 && due < DEADLINE_SOON_MS;
  });
  const inMeeting = (events ?? []).some((e) => isOngoing(e, now));

  const look = petLook({
    now,
    hour: today.getHours(),
    idleMs,
    cheeringUntil,
    lovedUntil,
    focusing: focusActive(focus, now) || inMeeting,
    music: playing && !brand?.video,
    video: playing && !!brand?.video,
    coding: waiting ? "waiting" : working ? "working" : "none",
    agent: agentName((waiting ?? working)?.agent),
    weather,
    battery: power ? { percent: power.percent, plugged: power.plugged } : null,
    busyDay: meetingsToday >= BUSY_MEETINGS || deadlineSoon,
    settings: pet,
  });
  return { ...look, name: pet.name.trim() || "Pet", color: pet.color };
}

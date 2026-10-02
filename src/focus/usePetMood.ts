import { useEffect, useState } from "react";

import { ipc } from "../core/ipc";
import { useSettings } from "../settings";
import { currentStreak } from "./breaks";
import { focusActive, useFocus } from "./focusStore";
import { usePetCheer } from "./petCheer";
import { petMood, type PetMood } from "./petMood";

const IDLE_CHECK_MS = 60_000;

/**
 * The pet's mood, whether a 3+ session day earned it a star, and the healthy-break streak it
 * keeps (days in a row with three breaks).
 */
export function usePetMood(): { mood: PetMood; streak: boolean; breakStreak: number } {
  const state = useFocus((s) => s.state);
  const cheeringUntil = usePetCheer((s) => s.until);
  const [idleMs, setIdleMs] = useState(0);
  const [now, setNow] = useState(() => Date.now());
  const breaks = useSettings((s) => s.settings?.breaks);

  // Start and stop cheering on time (it may have begun before the pet was shown).
  useEffect(() => {
    const left = cheeringUntil - Date.now();
    if (left <= 0) return;
    const start = window.setTimeout(() => setNow(Date.now()), 0);
    const end = window.setTimeout(() => setNow(Date.now()), left + 50);
    return () => {
      window.clearTimeout(start);
      window.clearTimeout(end);
    };
  }, [cheeringUntil]);

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

  return {
    mood: petMood({ cheeringUntil, focusing: focusActive(state, now), idleMs, now }),
    streak: (state?.doneToday ?? 0) >= 3,
    breakStreak: breaks ? currentStreak(breaks, new Date(now)) : 0,
  };
}

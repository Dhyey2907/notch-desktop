// The pet in the resting bubble: its mood, what it's doing (headphones while music plays, popcorn
// during a show, a laptop while an agent codes) and its outfit, at bubble size. A star after three
// focus sessions in a day or three days in a row of healthy breaks. A file dropped on the shelf
// makes a nice bed: it naps on it for a minute.
import { useEffect, useState } from "react";

import { eventBus } from "../core/eventBus";
import { PetFigure } from "../pet/PetFigure";
import { usePetLook } from "../pet/usePetLook";
import { useSettings } from "../settings";
import { currentStreak } from "./breaks";
import { useFocus } from "./focusStore";
import "./pet.css";

/** How long it naps on a file you dropped on the shelf. */
const SHELF_NAP_MS = 60_000;

export function Pet() {
  const look = usePetLook();
  const [napUntil, setNapUntil] = useState(0);

  useEffect(() => {
    let count: number | null = null;
    return eventBus.on("ShelfChanged", ({ event }) => {
      const n = event.payload.items.length;
      if (count !== null && n > count) setNapUntil(Date.now() + SHELF_NAP_MS);
      count = n;
    });
  }, []);

  useEffect(() => {
    if (!napUntil) return;
    const id = window.setTimeout(() => setNapUntil(0), Math.max(0, napUntil - Date.now()));
    return () => window.clearTimeout(id);
  }, [napUntil]);

  const sessions = useFocus((s) => s.state?.doneToday ?? 0);
  const breakStreak = useSettings((s) => (s.settings ? currentStreak(s.settings.breaks, new Date()) : 0));
  if (!look) return null;
  const napping = napUntil > 0;
  const label = `${look.name}: ${napping ? "Napping on your file" : look.status}${breakStreak > 0 ? `, ${breakStreak}-day break streak` : ""}`;
  return (
    <span className="pet" title={label}>
      <PetFigure
        size={14}
        label={label}
        face={napping ? "sleepy" : look.face}
        prop={napping ? "none" : look.prop}
        outfit={look.outfit}
        motion={look.motion}
        color={look.color}
      />
      {napping && <span className="pet__bed" aria-hidden />}
      {(sessions >= 3 || breakStreak >= 3) && <span className="pet__star">★</span>}
    </span>
  );
}

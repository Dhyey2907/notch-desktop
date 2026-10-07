// The pet in the resting bubble: its mood, what it's doing (headphones while music plays, popcorn
// during a show, a laptop while an agent codes) and its outfit, at bubble size. A star after three
// focus sessions in a day or three days in a row of healthy breaks.
import { PetFigure } from "../pet/PetFigure";
import { usePetLook } from "../pet/usePetLook";
import { useSettings } from "../settings";
import { currentStreak } from "./breaks";
import { useFocus } from "./focusStore";
import "./pet.css";

export function Pet() {
  const look = usePetLook();
  const sessions = useFocus((s) => s.state?.doneToday ?? 0);
  const breakStreak = useSettings((s) => (s.settings ? currentStreak(s.settings.breaks, new Date()) : 0));
  if (!look) return null;
  const label = `${look.name}: ${look.status}${breakStreak > 0 ? `, ${breakStreak}-day break streak` : ""}`;
  return (
    <span className="pet" title={label}>
      <PetFigure
        size={14}
        label={label}
        face={look.face}
        prop={look.prop}
        outfit={look.outfit}
        motion={look.motion}
        color={look.color}
      />
      {(sessions >= 3 || breakStreak >= 3) && <span className="pet__star">★</span>}
    </span>
  );
}

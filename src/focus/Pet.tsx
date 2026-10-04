// A tiny face in the bubble: content most of the time, focused during a session, dozing when
// you're away, bouncing when you finish something or take a break. A star after three sessions
// in a day or three days in a row of healthy breaks.
import { usePetMood } from "./usePetMood";
import "./pet.css";

const LABELS = {
  happy: "Pet: content",
  working: "Pet: focusing with you",
  sleepy: "Pet: dozing",
  cheering: "Pet: cheering",
};

export function Pet() {
  const { mood, streak, breakStreak } = usePetMood();
  const label = breakStreak > 0 ? `${LABELS[mood]}, ${breakStreak}-day break streak` : LABELS[mood];
  return (
    <span className={`pet pet--${mood}`} role="img" aria-label={label} title={label}>
      <svg viewBox="0 0 18 14" width="13" height="10" aria-hidden>
        <path className="pet__body" d="M2 13 C2 5 5 1.5 9 1.5 C13 1.5 16 5 16 13 Z" />
        {mood === "sleepy" ? (
          <>
            <path className="pet__line" d="M5.6 7.6 h2.2" />
            <path className="pet__line" d="M10.2 7.6 h2.2" />
          </>
        ) : mood === "cheering" ? (
          <>
            <path className="pet__line" d="M5.5 8 l1.2 -1.4 l1.2 1.4" />
            <path className="pet__line" d="M10.1 8 l1.2 -1.4 l1.2 1.4" />
          </>
        ) : (
          <>
            <ellipse className="pet__eye" cx="6.7" cy="7.2" rx="1" ry={mood === "working" ? 0.75 : 1.15} />
            <ellipse className="pet__eye" cx="11.3" cy="7.2" rx="1" ry={mood === "working" ? 0.75 : 1.15} />
          </>
        )}
        {mood === "cheering" ? (
          <path className="pet__mouth" d="M7.3 9.6 Q9 11.6 10.7 9.6 Z" />
        ) : mood !== "sleepy" ? (
          <path className="pet__line" d="M8 10.2 Q9 10.9 10 10.2" />
        ) : null}
        {mood === "working" && <path className="pet__band" d="M3.6 4.6 Q9 2.6 14.4 4.6" />}
      </svg>
      {mood === "sleepy" && <span className="pet__z">z</span>}
      {(streak || breakStreak >= 3) && <span className="pet__star">★</span>}
    </span>
  );
}

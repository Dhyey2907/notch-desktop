// The pet's walk along the taskbar while you're away (the transparent #pet window).
import "./pet.css";

export function PetWalk() {
  return (
    <div className="petwalk" aria-hidden>
      <div className="petwalk__track">
        <div className="petwalk__pet">
          <svg viewBox="0 0 18 16" width="30" height="27">
            <path className="pet__body" d="M2 13 C2 5 5 1.5 9 1.5 C13 1.5 16 5 16 13 Z" />
            <ellipse className="pet__eye" cx="7.6" cy="7.2" rx="1" ry="1.15" />
            <ellipse className="pet__eye" cx="12.2" cy="7.2" rx="1" ry="1.15" />
            <path className="pet__line" d="M9 10.2 Q10 10.9 11 10.2" />
            <path className="petwalk__leg petwalk__leg--a" d="M6 13 v2.2" />
            <path className="petwalk__leg petwalk__leg--b" d="M12 13 v2.2" />
          </svg>
        </div>
      </div>
    </div>
  );
}

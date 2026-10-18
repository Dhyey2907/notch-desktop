// The walking pet's little body: face, what it carries, what the weather has it wear, a hat it
// unlocked, and a collar tag for a week-long play streak.
import type { Face, Outfit, Prop } from "../pet/petState";

export type WalkerFace = Face | "purr" | "dizzy";

/** A little heart where an eye is. */
function heartEye(x: number): string {
  return `M${x} 8.4 q-1.6 -1.1 -1.6 -2 a0.8 0.8 0 0 1 1.6 -0.3 a0.8 0.8 0 0 1 1.6 0.3 q0 0.9 -1.6 2 z`;
}

function Eyes({ face }: { face: WalkerFace }) {
  switch (face) {
    case "love":
      return (
        <>
          <path className="petwalk__love" d={heartEye(7.6)} />
          <path className="petwalk__love" d={heartEye(12.2)} />
        </>
      );
    case "purr":
    case "sleepy":
      return <path className="pet__line" d="M6.6 7.2 q1 0.9 2 0 M11.2 7.2 q1 0.9 2 0" />;
    case "cheer":
      return <path className="pet__line" d="M6.6 7.6 q1 -1.2 2 0 M11.2 7.6 q1 -1.2 2 0" />;
    case "droopy":
      return <path className="pet__line" d="M6.6 7.4 h2 M11.2 7.4 h2" />;
    case "dizzy":
      return (
        <g className="petwalk__dizzy">
          <path className="pet__line" d="M7.6 7.2 m-1 0 a1 1 0 1 0 2 0 a0.6 0.6 0 1 0 -1.2 0" />
          <path className="pet__line" d="M12.2 7.2 m-1 0 a1 1 0 1 0 2 0 a0.6 0.6 0 1 0 -1.2 0" />
        </g>
      );
    case "nervous":
      return (
        <>
          <ellipse className="pet__eye" cx="7.6" cy="7.2" rx="1.2" ry="1.4" />
          <ellipse className="pet__eye" cx="12.2" cy="7.2" rx="1.2" ry="1.4" />
        </>
      );
    default:
      return (
        <>
          <ellipse className="pet__eye" cx="7.6" cy="7.2" rx="1" ry={face === "focused" ? 0.8 : 1.15} />
          <ellipse className="pet__eye" cx="12.2" cy="7.2" rx="1" ry={face === "focused" ? 0.8 : 1.15} />
        </>
      );
  }
}

function mouth(face: WalkerFace): string {
  switch (face) {
    case "love":
    case "cheer":
    case "purr":
      return "M8.6 9.8 Q10 11.4 11.4 9.8";
    case "droopy":
    case "stressed":
      return "M9 10.8 Q10 10 11 10.8";
    case "nervous":
    case "dizzy":
      return "M8.8 10.4 q0.6 -0.5 1.2 0 q0.6 0.5 1.2 0";
    default:
      return "M9 10.2 Q10 10.9 11 10.2";
  }
}

function Hat({ hat }: { hat: string }) {
  switch (hat) {
    case "cap":
      return (
        <g>
          <path d="M5.4 3.6 a3.6 2.6 0 0 1 7.2 0 z" fill="#e5484d" />
          <path d="M11.6 3.5 h3.2" stroke="#e5484d" strokeWidth="1" strokeLinecap="round" />
        </g>
      );
    case "bow":
      return (
        <g transform="translate(12.4 2.4)">
          <path d="M0 0 l-2.2 -1.4 v2.8 z M0 0 l2.2 -1.4 v2.8 z" fill="#ff7ab0" />
          <circle r="0.7" fill="#e0558f" />
        </g>
      );
    case "party":
      return (
        <g>
          <path d="M7 2.4 L9 -3.2 L11 2.4 z" fill="#7c5cff" />
          <path d="M7.7 0.6 h2.6 M8.3 -1.2 h1.4" stroke="#ffd34d" strokeWidth="0.6" />
          <circle cx="9" cy="-3.4" r="0.7" fill="#ffd34d" />
        </g>
      );
    case "crown":
      return <path d="M5.8 2.6 l0.4 -3 l1.6 1.6 l1.2 -2.2 l1.2 2.2 l1.6 -1.6 l0.4 3 z" fill="#ffc83d" />;
    case "wizard":
      return (
        <g>
          <path d="M4.6 2.6 h8.8" stroke="#3b3f8f" strokeWidth="1" strokeLinecap="round" />
          <path d="M6 2.4 L10.4 -4.2 L12 2.4 z" fill="#3b3f8f" />
          <path
            d="M9.4 -0.6 l0.3 0.6 l0.6 0.1 l-0.45 0.4 l0.1 0.6 l-0.55 -0.3 l-0.55 0.3 l0.1 -0.6 l-0.45 -0.4 l0.6 -0.1 z"
            fill="#ffd34d"
          />
        </g>
      );
    default:
      return null;
  }
}

export function WalkerFigure({
  face,
  prop,
  outfit,
  hat,
  collar,
  color,
}: {
  face: WalkerFace;
  prop: Prop;
  outfit: Outfit;
  hat: string;
  collar: boolean;
  color?: string;
}) {
  return (
    <svg viewBox="0 0 18 16" width="30" height="27">
      {outfit === "umbrella" && (
        <g className="petwalk__umbrella">
          <path d="M14.6 13 V2" stroke="#444" strokeWidth="0.5" />
          <path d="M9.2 2.4 a5.4 3.6 0 0 1 10.8 0 z" fill="#5b8def" />
        </g>
      )}
      {prop === "headphones" && (
        <path className="petwalk__band" d="M3.4 8.6 v-1.8 a5.6 5.6 0 0 1 11.2 0 v1.8" />
      )}
      <path
        className="pet__body"
        d="M2 13 C2 5 5 1.5 9 1.5 C13 1.5 16 5 16 13 Z"
        style={color ? { fill: color } : undefined}
      />
      <Eyes face={face} />
      {outfit === "sunglasses" && face !== "love" && (
        <g>
          <rect x="6.1" y="6.2" width="3" height="1.9" rx="0.6" fill="#16161a" />
          <rect x="10.7" y="6.2" width="3" height="1.9" rx="0.6" fill="#16161a" />
          <path d="M9.1 6.9 h1.6" stroke="#16161a" strokeWidth="0.5" />
        </g>
      )}
      <path className="pet__line" d={mouth(face)} />
      {(face === "love" || face === "purr" || face === "cheer") && (
        <>
          <ellipse cx="5.8" cy="9.4" rx="0.9" ry="0.5" fill="#ff7a9a" opacity="0.55" />
          <ellipse cx="14" cy="9.4" rx="0.9" ry="0.5" fill="#ff7a9a" opacity="0.55" />
        </>
      )}
      {outfit === "scarf" && (
        <g>
          <path d="M3.4 11.4 h11.2" stroke="#e5484d" strokeWidth="1.5" strokeLinecap="round" />
          <path d="M12.4 11.6 v2.4" stroke="#e5484d" strokeWidth="1.2" strokeLinecap="round" />
        </g>
      )}
      {collar && (
        <g>
          <path d="M4 12.2 h10" stroke="#5b8def" strokeWidth="0.8" strokeLinecap="round" />
          <circle cx="9" cy="13.2" r="0.85" fill="#ffc83d" />
        </g>
      )}
      <Hat hat={hat} />
      <path className="petwalk__leg petwalk__leg--a" d="M6 13 v2.6" />
      <path className="petwalk__leg petwalk__leg--b" d="M12 13 v2.6" />
      {prop === "headphones" && (
        <>
          <rect className="petwalk__ink" x="2.2" y="7.4" width="2.2" height="3.4" rx="1" />
          <rect className="petwalk__ink" x="13.6" y="7.4" width="2.2" height="3.4" rx="1" />
        </>
      )}
      {prop === "laptop" && (
        <g>
          <rect x="5" y="9.4" width="8" height="4.6" rx="0.6" fill="#2b2d33" />
          <rect x="5.7" y="10.1" width="6.6" height="3.2" rx="0.3" fill="#7cc4ff" opacity="0.9" />
          <path d="M3.8 14.5 h10.4" stroke="#2b2d33" strokeWidth="1" strokeLinecap="round" />
        </g>
      )}
      {prop === "popcorn" && (
        <g>
          <circle cx="7.4" cy="10.1" r="1" fill="#fff6d6" />
          <circle cx="9" cy="9.6" r="1.1" fill="#fff6d6" />
          <circle cx="10.6" cy="10.1" r="1" fill="#fff6d6" />
          <path d="M6.3 10.4 h5.4 l-0.7 4 h-4 z" fill="#e5484d" />
          <path d="M8.1 10.4 l0.3 4 M9.9 10.4 l-0.3 4" stroke="#fff" strokeWidth="0.6" />
        </g>
      )}
    </svg>
  );
}

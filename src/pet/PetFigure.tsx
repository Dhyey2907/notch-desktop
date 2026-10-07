// The pet, drawn once for every size: a soft blob with a face for its mood, something in its
// hands for what it's doing (headphones, popcorn, a laptop) and an outfit for the weather.
import type { Face, Motion, Outfit, Prop } from "./petState";
import "./petFigure.css";

const INK = "#141416";

function Eyes({ face }: { face: Face }) {
  switch (face) {
    case "sleepy":
      return (
        <>
          <path d="M12.5 19.5 h5" className="pf-line" />
          <path d="M22.5 19.5 h5" className="pf-line" />
        </>
      );
    case "droopy":
      return (
        <>
          <path d="M12.5 19 q2.5 2 5 0" className="pf-line" />
          <path d="M22.5 19 q2.5 2 5 0" className="pf-line" />
        </>
      );
    case "cheer":
      return (
        <>
          <path d="M12.5 20 l2.5 -3 l2.5 3" className="pf-line" />
          <path d="M22.5 20 l2.5 -3 l2.5 3" className="pf-line" />
        </>
      );
    case "love":
      return (
        <>
          <path
            className="pf-heart"
            d="M15 21.5 l-2.6 -2.7 a1.5 1.5 0 0 1 2.6 -1.6 a1.5 1.5 0 0 1 2.6 1.6 z"
          />
          <path
            className="pf-heart"
            d="M25 21.5 l-2.6 -2.7 a1.5 1.5 0 0 1 2.6 -1.6 a1.5 1.5 0 0 1 2.6 1.6 z"
          />
        </>
      );
    default: {
      const ry = face === "focused" ? 1.4 : face === "nervous" ? 1.5 : 2.3;
      const rx = face === "nervous" ? 1.5 : 1.9;
      return (
        <>
          <ellipse className="pf-eye" cx="15" cy="19" rx={rx} ry={ry} />
          <ellipse className="pf-eye" cx="25" cy="19" rx={rx} ry={ry} />
        </>
      );
    }
  }
}

function Mouth({ face }: { face: Face }) {
  switch (face) {
    case "cheer":
      return <path d="M16.5 24 q3.5 4.5 7 0 z" fill={INK} />;
    case "love":
    case "happy":
      return <path d="M17.5 24.5 q2.5 2 5 0" className="pf-line" />;
    case "stressed":
      return <path d="M16.5 25.5 q1 -1 2 0 q1 1 2 0 q1 -1 2 0 q1 1 2 0" className="pf-line" />;
    case "nervous":
      return <ellipse cx="20" cy="25.3" rx="1.3" ry="1.1" fill={INK} />;
    case "droopy":
    case "focused":
      return <path d="M18 25.2 h4" className="pf-line" />;
    case "sleepy":
      return <ellipse cx="20" cy="25.2" rx="1.1" ry="0.8" fill={INK} />;
  }
}

export function PetFigure({
  face,
  prop,
  outfit,
  motion,
  color,
  size = 40,
  label,
  className = "",
}: {
  face: Face;
  prop: Prop;
  outfit: Outfit;
  motion: Motion;
  color?: string;
  size?: number;
  label?: string;
  className?: string;
}) {
  const body = color || "var(--text-primary)";
  // Shades cover open eyes; hearts, closed and happy-closed eyes show through.
  const shades = outfit === "sunglasses" && ["happy", "focused", "stressed"].includes(face);
  return (
    <span
      className={`pf pf--${motion} pf--${face} ${className}`}
      role={label ? "img" : undefined}
      aria-label={label}
      aria-hidden={label ? undefined : true}
      style={{ width: size, height: size }}
    >
      <svg viewBox="0 0 40 40" width={size} height={size}>
        {outfit === "umbrella" && (
          <g className="pf-umbrella">
            <path d="M31 3 a9 9 0 0 1 9 9 h-18 a9 9 0 0 1 9 -9 z" fill="#5aa9ff" />
            <path d="M31 12 v12 q0 2 -2 2" className="pf-stroke" stroke="#c9d6e6" />
          </g>
        )}
        <g className="pf-body">
          <path d="M6 35 C6 15 12 6 20 6 C28 6 34 15 34 35 Z" fill={body} />
          {(face === "love" || face === "cheer") && (
            <>
              <ellipse cx="11.5" cy="23.5" rx="2.2" ry="1.3" fill="#ff7a9a" opacity="0.55" />
              <ellipse cx="28.5" cy="23.5" rx="2.2" ry="1.3" fill="#ff7a9a" opacity="0.55" />
            </>
          )}
          {shades ? (
            <g>
              <rect x="11" y="16.6" width="8" height="4.6" rx="1.6" fill={INK} />
              <rect x="21" y="16.6" width="8" height="4.6" rx="1.6" fill={INK} />
              <path d="M19 18.2 h2" className="pf-line" />
            </g>
          ) : (
            <Eyes face={face} />
          )}
          <Mouth face={face} />
          {(face === "stressed" || face === "nervous") && (
            <path d="M31 11 q2 3 0 4.4 q-2 -1.4 0 -4.4 z" fill="#7cc4ff" />
          )}
          {outfit === "scarf" && (
            <g>
              <path d="M8.6 28.5 q11.4 4 22.8 0 v3.2 q-11.4 4 -22.8 0 z" fill="#e5484d" />
              <path d="M25 31 l2 6 h-4 z" fill="#c43c41" />
            </g>
          )}
          {prop === "headphones" && (
            <g>
              <path
                d="M8.5 21 v-3 a11.5 11.5 0 0 1 23 0 v3"
                className="pf-stroke"
                stroke={INK}
                strokeWidth="2"
              />
              <rect x="5.5" y="18.5" width="5" height="7.5" rx="2.2" fill={INK} />
              <rect x="29.5" y="18.5" width="5" height="7.5" rx="2.2" fill={INK} />
            </g>
          )}
        </g>
        {prop === "popcorn" && (
          <g className="pf-prop">
            <path d="M27 27 h10 l-1.6 11 h-6.8 z" fill="#f5f0e6" />
            <path d="M29 27 l0.8 11 M32 27 v11 M35 27 l-0.8 11" stroke="#e5484d" strokeWidth="1.1" />
            <circle cx="28.6" cy="26" r="1.7" fill="#fff6d8" />
            <circle cx="31.6" cy="25.2" r="1.9" fill="#fff6d8" />
            <circle cx="34.8" cy="26" r="1.7" fill="#fff6d8" />
          </g>
        )}
        {prop === "laptop" && (
          <g className="pf-prop">
            <rect x="11" y="27" width="18" height="10" rx="1.4" fill="#2b2d33" />
            <rect x="12.5" y="28.4" width="15" height="7.2" rx="0.6" fill="#7cc4ff" opacity="0.85" />
            <path d="M8.5 37.4 h23" stroke="#2b2d33" strokeWidth="2" strokeLinecap="round" />
          </g>
        )}
        {face === "sleepy" && (
          <text x="31" y="9" className="pf-z">
            z
          </text>
        )}
        {face === "nervous" && (
          <text x="33" y="10" className="pf-bang">
            !
          </text>
        )}
      </svg>
    </span>
  );
}

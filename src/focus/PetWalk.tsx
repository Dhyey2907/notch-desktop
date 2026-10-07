// The pet's walk along the taskbar while you're away (the transparent #pet window). When you
// come back and click, Notch sends "pet-leave": it jumps, then runs off the nearest edge.
import { listen } from "@tauri-apps/api/event";
import { useEffect, useRef, useState } from "react";

import { useSettings } from "../settings";
import "./pet.css";

type Leaving = { from: number; toRight: boolean } | null;

export function PetWalk() {
  const track = useRef<HTMLDivElement>(null);
  const [leaving, setLeaving] = useState<Leaving>(null);
  const color = useSettings((s) => s.settings?.pet.color) || undefined;

  useEffect(() => {
    const unlisten = listen("pet-leave", () => {
      const el = track.current;
      if (!el) return;
      // Freeze where it is right now, then jump and run to the closer edge.
      const x = new DOMMatrixReadOnly(getComputedStyle(el).transform).m41;
      setLeaving({ from: x, toRight: x > window.innerWidth / 2 });
    });
    return () => {
      void unlisten.then((off) => off());
    };
  }, []);

  // Back to walking the next time it comes out.
  useEffect(() => {
    if (!leaving) return;
    const id = window.setTimeout(() => setLeaving(null), 2500);
    return () => window.clearTimeout(id);
  }, [leaving]);

  return (
    <div className={`petwalk${leaving ? " petwalk--leaving" : ""}`} aria-hidden>
      <div
        ref={track}
        className="petwalk__track"
        style={
          leaving
            ? ({
                "--pet-from": `${leaving.from}px`,
                "--pet-to": leaving.toRight ? "calc(100vw + 40px)" : "-70px",
              } as React.CSSProperties)
            : undefined
        }
      >
        <div
          className="petwalk__pet"
          style={leaving ? { transform: `scaleX(${leaving.toRight ? 1 : -1})` } : undefined}
        >
          <svg viewBox="0 0 18 16" width="30" height="27">
            <path
              className="pet__body"
              d="M2 13 C2 5 5 1.5 9 1.5 C13 1.5 16 5 16 13 Z"
              style={color ? { fill: color } : undefined}
            />
            <ellipse className="pet__eye" cx="7.6" cy="7.2" rx="1" ry="1.15" />
            <ellipse className="pet__eye" cx="12.2" cy="7.2" rx="1" ry="1.15" />
            <path
              className="pet__line"
              d={leaving ? "M8.6 9.8 Q10 11.4 11.4 9.8" : "M9 10.2 Q10 10.9 11 10.2"}
            />
            <path className="petwalk__leg petwalk__leg--a" d="M6 13 v2.6" />
            <path className="petwalk__leg petwalk__leg--b" d="M12 13 v2.6" />
          </svg>
        </div>
      </div>
    </div>
  );
}

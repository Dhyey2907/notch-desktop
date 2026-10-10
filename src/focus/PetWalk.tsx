// The pet's walk along the taskbar while you're away (the transparent #pet window). When you
// come back and click, Notch sends "pet-leave": it jumps, then runs off the nearest edge.
import { emitTo, listen } from "@tauri-apps/api/event";
import { useEffect, useRef, useState } from "react";

import { WALK_HELLO_EVENT, WALK_LOOK_EVENT, WALK_TREAT_EVENT, type WalkLook } from "../pet/useWalkLook";
import { useSettings } from "../settings";
import "./pet.css";

type Leaving = { from: number; toRight: boolean } | null;

/** How long a line stays in the cloud (unless it asks for longer, like a reminder). */
const CLOUD_MS = 9000;
/** Hearts, a cookie and love eyes for this long after a treat. */
const TREAT_MS = 2600;
/** A click this close to the pet still counts as a click on it (it's small and it moves). */
const CLICK_SLOP_PX = 10;
/** Closer to a screen edge than this, the cloud opens towards the middle so it isn't cut off. */
const CLOUD_EDGE_PX = 150;

type Side = "left" | "center" | "right";

/** Which way the cloud opens, from where the pet stands. */
/** A little heart where an eye is, for a treat. */
function heartEye(x: number): string {
  return `M${x} 8.4 q-1.6 -1.1 -1.6 -2 a0.8 0.8 0 0 1 1.6 -0.3 a0.8 0.8 0 0 1 1.6 0.3 q0 0.9 -1.6 2 z`;
}

function cloudSide(x: number, screenWidth: number, edge = CLOUD_EDGE_PX): Side {
  if (x < edge) return "left";
  if (x > screenWidth - edge) return "right";
  return "center";
}

export function PetWalk() {
  const track = useRef<HTMLDivElement>(null);
  const [leaving, setLeaving] = useState<Leaving>(null);
  const color = useSettings((s) => s.settings?.pet.color) || undefined;
  const [look, setLook] = useState<WalkLook>({ face: "happy", prop: "none", line: null });
  const [cloud, setCloud] = useState<{ text: string; at: number; side: Side } | null>(null);
  const petEl = useRef<HTMLDivElement>(null);
  const [treatAt, setTreatAt] = useState(0);
  const [treating, setTreating] = useState(false);

  // Clicks pass through this window, so Notch tells us where you clicked: on the pet = a treat.
  useEffect(() => {
    const off = listen<{ x: number; y: number }>("pet-click", ({ payload }) => {
      const r = petEl.current?.getBoundingClientRect();
      if (!r) return;
      const hit =
        payload.x >= r.left - CLICK_SLOP_PX &&
        payload.x <= r.right + CLICK_SLOP_PX &&
        payload.y >= r.top - CLICK_SLOP_PX &&
        payload.y <= r.bottom + CLICK_SLOP_PX;
      if (!hit) return;
      setTreatAt(Date.now());
      void emitTo("main", WALK_TREAT_EVENT, null).catch(() => {});
    });
    return () => void off.then((f) => f());
  }, []);

  useEffect(() => {
    if (!treatAt) return;
    const on = window.setTimeout(() => setTreating(true), 0);
    const done = window.setTimeout(() => setTreating(false), TREAT_MS);
    return () => {
      window.clearTimeout(on);
      window.clearTimeout(done);
    };
  }, [treatAt]);

  // What it's carrying comes from the island (a laptop while you code, popcorn during a show).
  useEffect(() => {
    const off = listen<WalkLook>(WALK_LOOK_EVENT, ({ payload }) => setLook(payload));
    void emitTo("main", WALK_HELLO_EVENT, null).catch(() => {});
    return () => void off.then((f) => f());
  }, []);
  const prop = leaving ? "none" : look.prop;

  // A new line: show it in the cloud for a few seconds, opening away from the screen edge.
  const lineText = look.line?.text;
  const lineAt = look.line?.at ?? 0;
  const lineHold = look.line?.holdMs ?? CLOUD_MS;
  useEffect(() => {
    const left = lineAt + lineHold - Date.now();
    if (!lineText || left <= 0) return;
    const el = track.current;
    const x = el ? new DOMMatrixReadOnly(getComputedStyle(el).transform).m41 : window.innerWidth / 2;
    const show = window.setTimeout(
      () => setCloud({ text: lineText, at: lineAt, side: cloudSide(x, window.innerWidth) }),
      0,
    );
    const hide = window.setTimeout(() => setCloud(null), left);
    return () => {
      window.clearTimeout(show);
      window.clearTimeout(hide);
    };
  }, [lineText, lineAt, lineHold]);

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
        {cloud && !leaving && (
          <div className={`petwalk__cloud petwalk__cloud--${cloud.side}`} key={cloud.at}>
            {cloud.text}
            <span className="petwalk__puff petwalk__puff--a" />
            <span className="petwalk__puff petwalk__puff--b" />
          </div>
        )}
        {treating && !leaving && (
          <span className="petwalk__treat" key={treatAt}>
            <span className="petwalk__cookie">🍪</span>
            <span className="petwalk__heart petwalk__heart--a">♥</span>
            <span className="petwalk__heart petwalk__heart--b">♥</span>
            <span className="petwalk__heart petwalk__heart--c">♥</span>
          </span>
        )}
        <div
          ref={petEl}
          className="petwalk__pet"
          style={leaving ? { transform: `scaleX(${leaving.toRight ? 1 : -1})` } : undefined}
        >
          <svg viewBox="0 0 18 16" width="30" height="27">
            {prop === "headphones" && (
              <path className="petwalk__band" d="M3.4 8.6 v-1.8 a5.6 5.6 0 0 1 11.2 0 v1.8" />
            )}
            <path
              className="pet__body"
              d="M2 13 C2 5 5 1.5 9 1.5 C13 1.5 16 5 16 13 Z"
              style={color ? { fill: color } : undefined}
            />
            {treating ? (
              <>
                <path className="petwalk__love" d={heartEye(7.6)} />
                <path className="petwalk__love" d={heartEye(12.2)} />
              </>
            ) : (
              <>
                <ellipse className="pet__eye" cx="7.6" cy="7.2" rx="1" ry="1.15" />
                <ellipse className="pet__eye" cx="12.2" cy="7.2" rx="1" ry="1.15" />
              </>
            )}
            <path
              className="pet__line"
              d={leaving || treating ? "M8.6 9.8 Q10 11.4 11.4 9.8" : "M9 10.2 Q10 10.9 11 10.2"}
            />
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
        </div>
      </div>
    </div>
  );
}

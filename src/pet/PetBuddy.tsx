// The pet on your desktop (the transparent #buddy window): drag it anywhere and it sits there;
// tap it for a treat (love eyes, a cookie, hearts); its comments show in a speech bubble.
// Hover for "Send back". How it looks and what it says come from the island (useBuddyLink).
import { emitTo, listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect, useRef, useState, type PointerEvent } from "react";

import { ipc } from "../core/ipc";
import { PetFigure } from "./PetFigure";
import { BUDDY_HELLO_EVENT, BUDDY_STATE_EVENT, BUDDY_TREAT_EVENT, type BuddyState } from "./useBuddyLink";
import "./petBuddy.css";

const TREAT_MS = 2600;
/** How long a comment stays up. */
const BUBBLE_MS = 9000;
/** Moving further than this starts a drag; less is a tap. */
const DRAG_PX = 5;
const SAVE_POSITION_AFTER_MS = 600;

export function PetBuddy() {
  const [state, setState] = useState<BuddyState | null>(null);
  const [treatAt, setTreatAt] = useState(0);
  const [now, setNow] = useState(() => Date.now());
  const down = useRef<{ x: number; y: number; dragging: boolean } | null>(null);

  useEffect(() => {
    const off = listen<BuddyState>(BUDDY_STATE_EVENT, ({ payload }) => setState(payload));
    // Ask the island for the current state right away.
    void emitTo("main", BUDDY_HELLO_EVENT, null).catch(() => {});
    return () => void off.then((f) => f());
  }, []);

  // Remember where it was dropped.
  useEffect(() => {
    let timer: number | undefined;
    const off = getCurrentWindow().onMoved(() => {
      window.clearTimeout(timer);
      timer = window.setTimeout(() => void ipc.petBuddyMoved().catch(() => {}), SAVE_POSITION_AFTER_MS);
    });
    return () => {
      window.clearTimeout(timer);
      void off.then((f) => f());
    };
  }, []);

  // Re-render when the treat or the comment runs out.
  const lineAt = state?.line?.at ?? 0;
  useEffect(() => {
    const ends = [treatAt + TREAT_MS, lineAt + BUBBLE_MS].filter((t) => t > Date.now());
    if (ends.length === 0) return;
    const id = window.setTimeout(() => setNow(Date.now()), Math.min(...ends) - Date.now() + 30);
    return () => window.clearTimeout(id);
  }, [treatAt, lineAt, now]);

  if (!state) return null;
  const { look, line, talk } = state;
  const treating = now < treatAt + TREAT_MS;
  const showLine = talk && line && now < line.at + BUBBLE_MS;

  const onDown = (e: PointerEvent) => {
    if (e.button !== 0) return;
    down.current = { x: e.screenX, y: e.screenY, dragging: false };
  };
  const onMove = (e: PointerEvent) => {
    const d = down.current;
    if (!d || d.dragging) return;
    if (Math.hypot(e.screenX - d.x, e.screenY - d.y) > DRAG_PX) {
      d.dragging = true;
      void getCurrentWindow().startDragging();
    }
  };
  const onUp = () => {
    const d = down.current;
    down.current = null;
    if (!d || d.dragging) return;
    // A tap: a treat.
    const t = Date.now();
    setTreatAt(t);
    setNow(t);
    void emitTo("main", BUDDY_TREAT_EVENT, null).catch(() => {});
  };

  return (
    <div className="buddy">
      {showLine && (
        <div className="buddy__bubble" role="status" key={line.at}>
          {line.text}
        </div>
      )}
      <div
        className={`buddy__pet${treating ? " buddy__pet--treat" : ""}`}
        onPointerDown={onDown}
        onPointerMove={onMove}
        onPointerUp={onUp}
        onPointerCancel={() => (down.current = null)}
        title={`${look.name} — tap for a treat, drag to move`}
      >
        {treating && (
          <>
            <span className="buddy__cookie" aria-hidden>
              🍪
            </span>
            <span className="buddy__heart buddy__heart--a" aria-hidden>
              ♥
            </span>
            <span className="buddy__heart buddy__heart--b" aria-hidden>
              ♥
            </span>
            <span className="buddy__heart buddy__heart--c" aria-hidden>
              ♥
            </span>
          </>
        )}
        <PetFigure
          size={104}
          face={treating ? "love" : look.face}
          prop={treating ? "none" : look.prop}
          outfit={look.outfit}
          motion={treating ? "wiggle" : look.motion}
          color={look.color}
          label={`${look.name}: ${look.status}`}
        />
        <span className="buddy__shadow" aria-hidden />
      </div>
      <div className="buddy__tools">
        <span className="buddy__status">{treating ? "Yum!" : look.status}</span>
        <button type="button" className="buddy__back" onClick={() => void ipc.petBuddyHide()}>
          Send back
        </button>
      </div>
    </div>
  );
}

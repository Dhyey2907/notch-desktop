// The screen text grabber's window: the frozen screen, dimmed; drag a box around some text
// and it's read (on this PC) and copied. Esc or a right-click cancels.
import { useEffect, useRef, useState, type PointerEvent } from "react";

import { ipc } from "../core/ipc";
import "./grab.css";

interface Box {
  x: number;
  y: number;
  w: number;
  h: number;
}

function normalize(a: { x: number; y: number }, b: { x: number; y: number }): Box {
  return { x: Math.min(a.x, b.x), y: Math.min(a.y, b.y), w: Math.abs(a.x - b.x), h: Math.abs(a.y - b.y) };
}

export function GrabApp() {
  const [picture, setPicture] = useState<string | null>(null);
  const [box, setBox] = useState<Box | null>(null);
  const [reading, setReading] = useState(false);
  const start = useRef<{ x: number; y: number } | null>(null);
  const image = useRef<HTMLImageElement>(null);

  useEffect(() => {
    const load = () => {
      setBox(null);
      setReading(false);
      ipc.grabPicture().then(setPicture, () => void ipc.grabCancel());
    };
    load();
    // The window is reused: a new grab tells it to load the new picture.
    window.addEventListener("notch-grab", load);
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") void ipc.grabCancel();
    };
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("notch-grab", load);
      window.removeEventListener("keydown", onKey);
    };
  }, []);

  const at = (e: PointerEvent) => ({ x: e.clientX, y: e.clientY });

  const finish = () => {
    const img = image.current;
    if (!box || !img || box.w < 4 || box.h < 4) {
      setBox(null);
      return;
    }
    // CSS pixels → screen pixels.
    const scale = img.naturalWidth / img.clientWidth;
    setReading(true);
    void ipc
      .grabFinish(
        Math.round(box.x * scale),
        Math.round(box.y * scale),
        Math.round(box.w * scale),
        Math.round(box.h * scale),
      )
      .catch(() => {})
      .finally(() => {
        setReading(false);
        setBox(null);
        setPicture(null);
      });
  };

  return (
    <div
      className="grab"
      onPointerDown={(e) => {
        if (e.button === 2) {
          void ipc.grabCancel();
          return;
        }
        if (e.button !== 0 || reading) return;
        e.currentTarget.setPointerCapture(e.pointerId);
        start.current = at(e);
        setBox({ ...at(e), w: 0, h: 0 });
      }}
      onPointerMove={(e) => {
        if (start.current) setBox(normalize(start.current, at(e)));
      }}
      onPointerUp={() => {
        start.current = null;
        finish();
      }}
      onContextMenu={(e) => e.preventDefault()}
    >
      {picture && <img ref={image} className="grab__picture" src={picture} alt="" draggable={false} />}
      <div className="grab__shade" style={box ? { clipPath: hole(box) } : undefined} />
      {box && <div className="grab__box" style={{ left: box.x, top: box.y, width: box.w, height: box.h }} />}
      <p className="grab__hint" role="status">
        {reading ? "Reading the text…" : "Drag a box around the text to copy it · Esc to cancel"}
      </p>
    </div>
  );
}

/** A full-screen shade with the box cut out. */
function hole(b: Box) {
  const r = b.x + b.w;
  const bottom = b.y + b.h;
  return `polygon(evenodd, 0 0, 100% 0, 100% 100%, 0 100%, 0 0, ${b.x}px ${b.y}px, ${r}px ${b.y}px, ${r}px ${bottom}px, ${b.x}px ${bottom}px, ${b.x}px ${b.y}px)`;
}

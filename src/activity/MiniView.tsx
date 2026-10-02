// A live view of another window, drawn by Windows (DWM) straight into the notch over this
// placeholder. The placeholder reports where it is; Windows does the drawing.
import { X } from "lucide-react";
import { useEffect, useRef } from "react";

import { ipc } from "../core/ipc";
import { uiScale } from "../shell/appearance";
import type { Mini } from "./activityStore";

export function MiniView({ mini, onClose }: { mini: Mini; onClose: () => void }) {
  const area = useRef<HTMLButtonElement>(null);
  // The latest close handler, without re-placing the view each time it changes.
  const close = useRef(onClose);
  useEffect(() => {
    close.current = onClose;
  }, [onClose]);

  useEffect(() => {
    const el = area.current;
    if (!el) return;
    const place = () => {
      const r = el.getBoundingClientRect();
      // CSS px → physical pixels (the page is also zoomed by the notch size setting).
      const k = window.devicePixelRatio * uiScale();
      void ipc
        .deskMiniShow(
          mini.id,
          Math.round(r.left * k),
          Math.round(r.top * k),
          Math.round(r.right * k),
          Math.round(r.bottom * k),
        )
        .catch(() => close.current());
    };
    place();
    // The island animates into its size; place again once it settles, and on any resize.
    const settle = window.setTimeout(place, 450);
    const observer = new ResizeObserver(place);
    observer.observe(el);
    return () => {
      window.clearTimeout(settle);
      observer.disconnect();
      void ipc.deskMiniHide().catch(() => {});
    };
  }, [mini.id]);

  return (
    <div className="mini">
      <div className="mini__bar">
        <span className="mini__title">
          {mini.app} · {mini.title}
        </span>
        <button type="button" className="mini__close" aria-label="Stop watching" onClick={onClose}>
          <X size={12} aria-hidden />
        </button>
      </div>
      <button
        ref={area}
        type="button"
        className="mini__view"
        aria-label={`Live view of ${mini.title}. Click to bring it to the front.`}
        onClick={() => void ipc.deskFocus(mini.id).catch(() => {})}
      />
    </div>
  );
}

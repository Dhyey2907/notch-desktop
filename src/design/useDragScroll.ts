// A row that's wider than its box: drag it sideways with the mouse, or turn the wheel, to see
// the rest. A drag doesn't count as a click on whatever it started on.
import { useEffect, useRef } from "react";

/** Moving this far (px) turns a press into a drag. */
const DRAG_AFTER_PX = 4;

export function useDragScroll<T extends HTMLElement>() {
  const ref = useRef<T>(null);

  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    let start: { x: number; left: number; id: number } | null = null;
    let dragged = false;

    const down = (e: PointerEvent) => {
      if (e.button !== 0 || el.scrollWidth <= el.clientWidth) return;
      start = { x: e.clientX, left: el.scrollLeft, id: e.pointerId };
      dragged = false;
    };
    const move = (e: PointerEvent) => {
      if (!start || e.pointerId !== start.id) return;
      const dx = e.clientX - start.x;
      if (!dragged && Math.abs(dx) < DRAG_AFTER_PX) return;
      if (!dragged) {
        dragged = true;
        el.setPointerCapture(e.pointerId);
        el.dataset.dragging = "";
      }
      el.scrollLeft = start.left - dx;
    };
    const up = () => {
      start = null;
      delete el.dataset.dragging;
    };
    // Swallow the click that ends a drag, so it doesn't pick whatever is under the pointer.
    const click = (e: MouseEvent) => {
      if (!dragged) return;
      dragged = false;
      e.preventDefault();
      e.stopPropagation();
    };
    // A plain wheel scrolls the row sideways.
    const wheel = (e: WheelEvent) => {
      if (el.scrollWidth <= el.clientWidth || Math.abs(e.deltaX) > Math.abs(e.deltaY)) return;
      e.preventDefault();
      el.scrollLeft += e.deltaY;
    };

    el.addEventListener("pointerdown", down);
    el.addEventListener("pointermove", move);
    el.addEventListener("pointerup", up);
    el.addEventListener("pointercancel", up);
    el.addEventListener("click", click, true);
    el.addEventListener("wheel", wheel, { passive: false });
    return () => {
      el.removeEventListener("pointerdown", down);
      el.removeEventListener("pointermove", move);
      el.removeEventListener("pointerup", up);
      el.removeEventListener("pointercancel", up);
      el.removeEventListener("click", click, true);
      el.removeEventListener("wheel", wheel);
    };
  }, []);

  return ref;
}

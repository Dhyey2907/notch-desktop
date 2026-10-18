import { Check, Settings } from "lucide-react";
import { motion } from "motion/react";
import { useEffect, useLayoutEffect, useRef, type KeyboardEvent, type PointerEvent } from "react";

import { ipc } from "../core/ipc";
import { IconButton } from "../design/components";
import { contentEnter } from "../design/motion";
import { useReduceMotion } from "../design/useReduceMotion";
import { useSettings } from "../settings";
import { DockEditBar } from "./DockEditBar";
import { LONG_PRESS_MS, moveId, PRESS_SLOP_PX, slotFor, useDockEdit } from "./dockEdit";
import { useIsland } from "./useIsland";
import { saveDockOrder, useDockModules, useModuleNav } from "./useModules";

/**
 * Compact module switcher shown beneath the expanded island (design.md §17).
 * Arrow keys move between modules; the active one gets a raised treatment.
 * Long-press an icon to rearrange the dock: drag icons sideways, then Done (Esc cancels).
 */
export function Dock() {
  const modules = useDockModules();
  const activeId = useModuleNav((s) => s.activeId);
  const setActive = useModuleNav((s) => s.setActive);
  const step = useModuleNav((s) => s.step);
  const reduceMotion = useReduceMotion();
  const dock = useRef<HTMLDivElement>(null);
  const editing = useDockEdit((s) => s.editing);
  const draft = useDockEdit((s) => s.draft);
  const dragging = useDockEdit((s) => s.dragging);
  const press = useRef<{ id: string; x: number; y: number; timer: number } | null>(null);
  /** The click that ends a long press shouldn't also open the module. */
  const swallowClick = useRef(false);
  const hasSettings = useSettings((s) => s.settings !== null);

  // Tell the island how wide the dock wants to be, so the window grows to fit it (more modules
  // in the dock than the panel is wide would otherwise be cut off at both ends).
  useLayoutEffect(() => {
    const el = dock.current;
    if (!el) return;
    const report = () => void useIsland.getState().setDockWidth(Math.ceil(el.offsetWidth));
    report();
    const observer = new ResizeObserver(report);
    observer.observe(el);
    return () => observer.disconnect();
  }, []);

  // Leaving the expanded notch ends editing without saving.
  useEffect(() => () => useDockEdit.getState().stop(), []);

  const byId = new Map(modules.map((m) => [m.id, m]));
  const shown = editing && draft ? draft.flatMap((id) => byId.get(id) ?? []) : modules;

  const finish = (save: boolean) => {
    const { draft: order } = useDockEdit.getState();
    useDockEdit.getState().stop();
    if (save && order) void saveDockOrder(order);
  };

  const onKeyDown = (e: KeyboardEvent<HTMLDivElement>) => {
    if (editing && (e.key === "Escape" || e.key === "Enter")) {
      e.preventDefault();
      e.stopPropagation();
      finish(e.key === "Enter");
      return;
    }
    if (e.key !== "ArrowRight" && e.key !== "ArrowLeft") return;
    e.preventDefault();
    step(e.key === "ArrowRight" ? 1 : -1);
    const id = useModuleNav.getState().activeId;
    e.currentTarget.querySelector<HTMLElement>(`[data-module="${id}"] button`)?.focus();
  };

  const cancelPress = () => {
    if (press.current) window.clearTimeout(press.current.timer);
    press.current = null;
  };

  const onPointerDown = (id: string) => (e: PointerEvent<HTMLSpanElement>) => {
    if (e.button !== 0) return;
    if (editing) {
      e.currentTarget.setPointerCapture(e.pointerId);
      useDockEdit.getState().setDragging(id);
      return;
    }
    cancelPress();
    const timer = window.setTimeout(() => {
      press.current = null;
      swallowClick.current = true;
      useDockEdit.getState().start(
        modules.map((m) => m.id),
        id,
      );
    }, LONG_PRESS_MS);
    press.current = { id, x: e.clientX, y: e.clientY, timer };
  };

  const onPointerMove = (e: PointerEvent<HTMLSpanElement>) => {
    const p = press.current;
    if (p && Math.hypot(e.clientX - p.x, e.clientY - p.y) > PRESS_SLOP_PX) cancelPress();
    const { dragging: id, draft: order } = useDockEdit.getState();
    if (!editing || !id || !order || !dock.current) return;
    const centers = order
      .filter((x) => x !== id)
      .map((x) => {
        const el = dock.current?.querySelector<HTMLElement>(`[data-module="${x}"]`);
        const r = el?.getBoundingClientRect();
        return r ? r.left + r.width / 2 : 0;
      });
    const next = moveId(order, id, slotFor(e.clientX, centers));
    if (next.join() !== order.join()) useDockEdit.getState().setDraft(next);
  };

  const onPointerUp = (e: PointerEvent<HTMLSpanElement>) => {
    cancelPress();
    if (e.currentTarget.hasPointerCapture(e.pointerId)) e.currentTarget.releasePointerCapture(e.pointerId);
    if (editing) useDockEdit.getState().setDragging(null);
  };

  return (
    <motion.div
      ref={dock}
      className={`dock${editing ? " dock--editing" : ""}`}
      role="toolbar"
      aria-label={editing ? "Rearrange modules: drag icons, Enter to save, Esc to cancel" : "Modules"}
      onKeyDown={onKeyDown}
      initial={{ opacity: 0, y: -6 }}
      animate={{ opacity: 1, y: 0, transition: contentEnter("expanded", reduceMotion) }}
      exit={{ opacity: 0, transition: { duration: 0.08 } }}
    >
      {shown.map((module) => (
        <motion.span
          layout={!reduceMotion}
          key={module.id}
          data-module={module.id}
          className={`dock__item${dragging === module.id ? " dock__item--dragging" : ""}`}
          onPointerDown={onPointerDown(module.id)}
          onPointerMove={onPointerMove}
          onPointerUp={onPointerUp}
          onPointerCancel={onPointerUp}
          onClickCapture={(e) => {
            if (swallowClick.current || editing) {
              swallowClick.current = false;
              e.stopPropagation();
              e.preventDefault();
            }
          }}
        >
          <IconButton
            icon={module.icon}
            label={editing ? `Move ${module.name}` : module.name}
            active={module.id === activeId}
            onClick={() => setActive(module.id)}
          />
        </motion.span>
      ))}
      <span className="dock__divider" aria-hidden />
      {editing ? (
        <>
          {hasSettings && <DockEditBar />}
          <IconButton icon={Check} label="Done" active onClick={() => finish(true)} />
        </>
      ) : (
        <IconButton icon={Settings} label="Settings" onClick={() => void ipc.openSettingsWindow()} />
      )}
    </motion.div>
  );
}

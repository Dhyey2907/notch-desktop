import { getCurrentWindow } from "@tauri-apps/api/window";
import { AnimatePresence, motion, useReducedMotion } from "motion/react";
import { useEffect, useRef, type WheelEvent } from "react";

import { eventBus } from "../core/eventBus";
import { contentEnter, INSTANT, ISLAND_SPRING } from "../design/motion";
import { Dock } from "./Dock";
import { ISLAND_SIZE, WINDOW_MARGIN, type IslandMode } from "./geometry";
import { CompactHost, ModuleHost, PeekHost } from "./ModuleHost";
import { useIsland } from "./useIsland";
import { useModuleLifecycle, useModuleNav } from "./useModules";
import "./island.css";

/** Hover must linger this long before the island peeks (design.md §18: no accidental panels). */
const PEEK_INTENT_MS = 140;
/** Grace period before a peek collapses after the pointer leaves. */
const PEEK_LEAVE_MS = 200;
/** Horizontal scroll distance that switches module, and the cooldown between switches. */
const SWIPE_THRESHOLD = 60;
const SWIPE_COOLDOWN_MS = 350;

const ISLAND_RADIUS: Record<IslandMode, number> = {
  collapsed: ISLAND_SIZE.collapsed.height / 2,
  peek: ISLAND_SIZE.peek.height / 2,
  expanded: 24,
};

function focusActiveDockItem() {
  const id = useModuleNav.getState().activeId;
  requestAnimationFrame(() =>
    document.querySelector<HTMLElement>(`.dock [data-module="${id}"] button`)?.focus(),
  );
}

export function Island() {
  const mode = useIsland((s) => s.mode);
  const request = useIsland((s) => s.request);
  const settle = useIsland((s) => s.settle);
  const reduceMotion = useReducedMotion();
  const hoverTimer = useRef<number | undefined>(undefined);
  const swipe = useRef({ distance: 0, lockedUntil: 0 });

  useModuleLifecycle();

  // Sync the window to the initial mode once the webview is up.
  useEffect(() => {
    void settle();
  }, [settle]);

  // Keyboard path (Ctrl+Alt+N): after expanding, move focus to the dock so arrows/Tab work.
  // Pointer opens don't move focus, so no focus ring appears on click.
  useEffect(
    () =>
      eventBus.on("ShellToggleRequested", async () => {
        await useIsland.getState().toggleExpanded();
        if (useIsland.getState().mode === "expanded") focusActiveDockItem();
      }),
    [],
  );

  // Clicking elsewhere (window blur) or pressing Escape collapses the island.
  useEffect(() => {
    const unlisten = getCurrentWindow().onFocusChanged(({ payload: focused }) => {
      if (!focused && useIsland.getState().mode === "expanded")
        void useIsland.getState().request("collapsed");
    });
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") void useIsland.getState().request("collapsed");
    };
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("keydown", onKey);
      void unlisten.then((fn) => fn());
    };
  }, []);

  const scheduleHover = (target: IslandMode, delay: number) => {
    window.clearTimeout(hoverTimer.current);
    hoverTimer.current = window.setTimeout(() => void request(target), delay);
  };

  const onPointerEnter = () => {
    if (useIsland.getState().mode === "collapsed") scheduleHover("peek", PEEK_INTENT_MS);
    else window.clearTimeout(hoverTimer.current);
  };

  const onPointerLeave = () => {
    if (useIsland.getState().mode === "peek") scheduleHover("collapsed", PEEK_LEAVE_MS);
    else window.clearTimeout(hoverTimer.current);
  };

  const expand = () => {
    window.clearTimeout(hoverTimer.current);
    void request("expanded");
  };

  // Horizontal swipe / trackpad scroll switches module (design.md §18).
  const onWheel = (e: WheelEvent) => {
    if (mode !== "expanded" || Math.abs(e.deltaX) <= Math.abs(e.deltaY)) return;
    const now = performance.now();
    if (now < swipe.current.lockedUntil) return;
    swipe.current.distance += e.deltaX;
    if (Math.abs(swipe.current.distance) >= SWIPE_THRESHOLD) {
      useModuleNav.getState().step(swipe.current.distance > 0 ? 1 : -1);
      swipe.current = { distance: 0, lockedUntil: now + SWIPE_COOLDOWN_MS };
    }
  };

  const size = ISLAND_SIZE[mode];

  return (
    <div className="island-frame" style={{ paddingTop: WINDOW_MARGIN.top }}>
      <motion.section
        className="island"
        data-mode={mode}
        aria-label="Notch"
        initial={false}
        animate={{ width: size.width, height: size.height, borderRadius: ISLAND_RADIUS[mode] }}
        transition={reduceMotion ? INSTANT : ISLAND_SPRING[mode]}
        onAnimationComplete={() => void settle()}
        onPointerEnter={onPointerEnter}
        onPointerLeave={onPointerLeave}
        onWheel={onWheel}
      >
        {mode !== "expanded" && (
          <button className="island-hit" aria-label="Open Notch" aria-expanded={false} onClick={expand} />
        )}
        <AnimatePresence mode="popLayout" initial={false}>
          <motion.div
            key={mode}
            className={`island-content island-content--${mode}`}
            initial={{ opacity: 0, scale: 0.97 }}
            animate={{ opacity: 1, scale: 1, transition: contentEnter(mode, reduceMotion) }}
            exit={{ opacity: 0, transition: { duration: 0.08 } }}
          >
            {mode === "collapsed" && <CompactHost />}
            {mode === "peek" && <PeekHost />}
            {mode === "expanded" && <ModuleHost />}
          </motion.div>
        </AnimatePresence>
      </motion.section>
      <AnimatePresence>{mode === "expanded" && <Dock />}</AnimatePresence>
    </div>
  );
}

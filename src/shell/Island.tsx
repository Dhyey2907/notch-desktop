import { getCurrentWindow } from "@tauri-apps/api/window";
import { AnimatePresence, motion, useReducedMotion, type Transition } from "motion/react";
import { useEffect, useRef, useState } from "react";

import { eventBus } from "../core/eventBus";
import { ipc } from "../core/ipc";
import { ISLAND_SIZE, WINDOW_MARGIN, type IslandMode } from "./geometry";
import { useIsland } from "./useIsland";
import "./island.css";

/** Hover must linger this long before the island peeks (design.md §18: no accidental panels). */
const PEEK_INTENT_MS = 140;
/** Grace period before a peek collapses after the pointer leaves. */
const PEEK_LEAVE_MS = 200;

// design.md §8: peek 180–250ms, expansion 350–500ms (spring), collapse 250–400ms.
const ISLAND_SPRING: Record<IslandMode, Transition> = {
  collapsed: { type: "spring", visualDuration: 0.32, bounce: 0 },
  peek: { type: "spring", visualDuration: 0.22, bounce: 0.1 },
  expanded: { type: "spring", visualDuration: 0.42, bounce: 0.18 },
};

const ISLAND_RADIUS: Record<IslandMode, number> = {
  collapsed: ISLAND_SIZE.collapsed.height / 2,
  peek: ISLAND_SIZE.peek.height / 2,
  expanded: 24,
};

export function Island() {
  const mode = useIsland((s) => s.mode);
  const request = useIsland((s) => s.request);
  const settle = useIsland((s) => s.settle);
  const reduceMotion = useReducedMotion();
  const hoverTimer = useRef<number | undefined>(undefined);

  // Sync the window to the initial mode once the webview is up.
  useEffect(() => {
    void settle();
  }, [settle]);

  // Keyboard path (Ctrl+Alt+N): after expanding, move focus into the island so Tab works.
  // Pointer opens don't steal focus, so no focus ring appears on click.
  useEffect(
    () =>
      eventBus.on("ShellToggleRequested", async () => {
        await useIsland.getState().toggleExpanded();
        if (useIsland.getState().mode === "expanded") {
          requestAnimationFrame(() => document.querySelector<HTMLElement>(".island button")?.focus());
        }
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

  const size = ISLAND_SIZE[mode];

  return (
    <div className="island-frame" style={{ paddingTop: WINDOW_MARGIN.top }}>
      <motion.section
        className="island"
        data-mode={mode}
        aria-label="Notch"
        initial={false}
        animate={{ width: size.width, height: size.height, borderRadius: ISLAND_RADIUS[mode] }}
        transition={reduceMotion ? { duration: 0 } : ISLAND_SPRING[mode]}
        onAnimationComplete={() => void settle()}
        onPointerEnter={onPointerEnter}
        onPointerLeave={onPointerLeave}
      >
        {mode !== "expanded" && (
          <button className="island-hit" aria-label="Open Notch" aria-expanded={false} onClick={expand} />
        )}
        <AnimatePresence mode="popLayout" initial={false}>
          <motion.div
            key={mode}
            className="island-content"
            initial={{ opacity: 0, scale: 0.97 }}
            animate={{
              opacity: 1,
              scale: 1,
              // Content follows the container once it has started to unfold (design.md §8).
              transition: reduceMotion
                ? { duration: 0 }
                : { delay: mode === "expanded" ? 0.1 : 0.05, duration: 0.2 },
            }}
            exit={{ opacity: 0, transition: { duration: 0.08 } }}
          >
            {mode === "peek" && <PeekContent />}
            {mode === "expanded" && <ExpandedContent onCollapse={() => void request("collapsed")} />}
          </motion.div>
        </AnimatePresence>
      </motion.section>
    </div>
  );
}

function PeekContent() {
  return (
    <div className="island-peek">
      <span className="island-title">Notch Desktop</span>
      <span className="island-muted">Click to open</span>
    </div>
  );
}

function ExpandedContent({ onCollapse }: { onCollapse: () => void }) {
  const [version, setVersion] = useState<string | null>(null);

  useEffect(() => {
    ipc
      .appInfo()
      .then((info) => setVersion(info.version))
      .catch(() => setVersion(null));
  }, []);

  return (
    <div className="island-expanded">
      <header className="island-header">
        <span className="island-title">Notch Desktop</span>
        <button className="island-text-button" onClick={onCollapse}>
          Collapse
        </button>
      </header>
      <div className="island-empty">
        <p className="island-empty-title">No modules yet</p>
        <p className="island-muted">Media, Calendar, Git and Claude will appear here as they are built.</p>
      </div>
      <footer className="island-footer island-muted">
        {version && <span>v{version}</span>}
        <span>Ctrl+Alt+N to toggle · Esc to close</span>
      </footer>
    </div>
  );
}

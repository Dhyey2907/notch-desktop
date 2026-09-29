import { getCurrentWindow } from "@tauri-apps/api/window";
import { AnimatePresence, motion } from "motion/react";
import { useEffect, useRef, type WheelEvent } from "react";

import { useActivities } from "../activity";
import { isPersistent, type Activity } from "../activity/activityStore";
import { ActivityView } from "../activity/ActivityView";
import { eventBus } from "../core/eventBus";
import { contentEnter, INSTANT, ISLAND_SPRING, SERIOUS_SPRING } from "../design/motion";
import { useReduceMotion } from "../design/useReduceMotion";
import { findModule } from "../modules/registry";
import { Dock } from "./Dock";
import { activityIslandSize, islandRadius, WINDOW_MARGIN, type IslandMode } from "./geometry";
import { CompactHost, ModuleHost, PeekHost } from "./ModuleHost";
import { useFirstRunPrompt } from "./useFirstRunPrompt";
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

function focusActiveDockItem() {
  const id = useModuleNav.getState().activeId;
  requestAnimationFrame(() =>
    document.querySelector<HTMLElement>(`.dock [data-module="${id}"] button`)?.focus(),
  );
}

/** Critical and high-priority activities are serious: they never bounce (design.md §8). */
function isSerious(activity: Activity | null) {
  return activity?.priority === "critical" || activity?.priority === "high";
}

function currentActivity() {
  return useActivities.getState().activities[0] ?? null;
}

export function Island() {
  const mode = useIsland((s) => s.mode);
  const size = useIsland((s) => s.size);
  const request = useIsland((s) => s.request);
  const settle = useIsland((s) => s.settle);
  const activity = useActivities((s) => s.activities[0] ?? null);
  const queued = useActivities((s) => Math.max(0, s.activities.length - 1));
  const reduceMotion = useReduceMotion();
  const hoverTimer = useRef<number | undefined>(undefined);
  const swipe = useRef({ distance: 0, lockedUntil: 0 });

  useModuleLifecycle();
  useFirstRunPrompt();

  // Sync the window to the initial mode once the webview is up.
  useEffect(() => {
    void settle();
  }, [settle]);

  // Activities claim the island unless the user has it expanded; once none remain, it
  // returns to collapsed. Expanded is never interrupted — queued activities wait.
  useEffect(() => {
    const { mode: current, request } = useIsland.getState();
    if (activity && current !== "expanded") {
      const actions = activity.actions ?? [];
      const layout = actions.length === 0 ? "none" : actions.every((a) => a.icon) ? "inline" : "row";
      void request("activity", activityIslandSize(layout));
    } else if (!activity && current === "activity") {
      void request("collapsed");
    }
  }, [activity, mode]);

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

  // Clicking elsewhere (window blur) collapses; Escape collapses or dismisses the activity.
  useEffect(() => {
    const unlisten = getCurrentWindow().onFocusChanged(({ payload: focused }) => {
      if (!focused && useIsland.getState().mode === "expanded")
        void useIsland.getState().request("collapsed");
    });
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      const current = currentActivity();
      if (useIsland.getState().mode === "activity" && current) {
        // Persistent activities (e.g. approvals) need an explicit answer.
        if (!isPersistent(current)) useActivities.getState().dismiss(current.id);
        return;
      }
      void useIsland.getState().request("collapsed");
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
    const current = useIsland.getState().mode;
    if (current === "collapsed") scheduleHover("peek", PEEK_INTENT_MS);
    else window.clearTimeout(hoverTimer.current);
    // Don't let an activity vanish while the pointer is on it.
    const shown = currentActivity();
    if (current === "activity" && shown) useActivities.getState().hold(shown.id);
  };

  const onPointerLeave = () => {
    const current = useIsland.getState().mode;
    if (current === "peek") scheduleHover("collapsed", PEEK_LEAVE_MS);
    else window.clearTimeout(hoverTimer.current);
    const shown = currentActivity();
    if (current === "activity" && shown) useActivities.getState().release(shown.id);
  };

  const expand = () => {
    window.clearTimeout(hoverTimer.current);
    void request("expanded");
  };

  /** Clicking an activity opens its module; transient activities are consumed by that. */
  const openActivity = (shown: Activity) => {
    if (findModule(shown.moduleId)) useModuleNav.getState().setActive(shown.moduleId);
    if (!isPersistent(shown)) useActivities.getState().dismiss(shown.id);
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

  const shownActivity = mode === "activity" ? activity : null;
  const transition = reduceMotion ? INSTANT : isSerious(shownActivity) ? SERIOUS_SPRING : ISLAND_SPRING[mode];

  return (
    <div className="island-frame" style={{ paddingTop: WINDOW_MARGIN.top }}>
      <motion.section
        className="island"
        data-mode={mode}
        aria-label="Notch"
        initial={false}
        animate={{ width: size.width, height: size.height, borderRadius: islandRadius(size) }}
        transition={transition}
        onAnimationComplete={() => void settle()}
        onPointerEnter={onPointerEnter}
        onPointerLeave={onPointerLeave}
        onWheel={onWheel}
      >
        {(mode === "collapsed" || mode === "peek") && (
          <button className="island-hit" aria-label="Open Notch" aria-expanded={false} onClick={expand} />
        )}
        <AnimatePresence mode="popLayout" initial={false}>
          <motion.div
            key={shownActivity ? `activity-${shownActivity.id}` : mode}
            className={`island-content island-content--${mode}`}
            initial={{ opacity: 0, scale: 0.97 }}
            animate={{ opacity: 1, scale: 1, transition: contentEnter(mode, reduceMotion) }}
            exit={{ opacity: 0, transition: { duration: 0.08 } }}
          >
            {mode === "collapsed" && <CompactHost />}
            {mode === "peek" && <PeekHost />}
            {shownActivity && (
              <ActivityView
                activity={shownActivity}
                queued={queued}
                onRunAction={(actionId) =>
                  void useActivities.getState().runAction(shownActivity.id, actionId)
                }
                onOpen={() => openActivity(shownActivity)}
              />
            )}
            {mode === "expanded" && <ModuleHost />}
          </motion.div>
        </AnimatePresence>
      </motion.section>
      <AnimatePresence>{mode === "expanded" && <Dock />}</AnimatePresence>
    </div>
  );
}

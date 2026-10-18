import { getCurrentWindow } from "@tauri-apps/api/window";
import { AnimatePresence, motion } from "motion/react";
import { useEffect, useRef, type WheelEvent } from "react";

import { useActivities } from "../activity";
import { isPersistent, type Activity } from "../activity/activityStore";
import { ActivityView } from "../activity/ActivityView";
import { eventBus } from "../core/eventBus";
import { ipc } from "../core/ipc";
import { windowLabel } from "../core/window";
import { contentEnter, INSTANT, ISLAND_SPRING, SERIOUS_SPRING } from "../design/motion";
import { useReduceMotion } from "../design/useReduceMotion";
import { findModule } from "../modules/registry";
import { usePalette } from "../modules/palette/paletteStore";
import { useDockProfiles } from "./useDockProfiles";
import { useEveningReview } from "../modules/day/useEveningReview";
import { useMorningBriefing } from "../modules/day/useMorningBriefing";
import { useWallpaperAccent } from "./useWallpaperAccent";
import { PinnedWidget } from "./MiniWidget";
import { Dock } from "./Dock";
import { Pet } from "../focus/Pet";
import { usePetOut, usePetWalkLink } from "../pet/usePetPlace";
import { useWalkLook } from "../pet/useWalkLook";
import { useAmbient } from "../focus/useAmbient";
import { useFocusSession } from "../focus/useFocusSession";
import { useGameMode } from "./gameMode";
import { useSettings } from "../settings";
import { useFileDrop } from "./fileDrop";
import { useMusicPulse } from "./musicPulse";
import { addWheel, idleSwipe } from "./gestures";
import { useSystemHud } from "./systemHud";
import { trackFullscreen, useFullscreen } from "./fullscreen";
import { scaledRadius } from "./appearance";
import { HOVER_CLOSE_MS, hideOverFullscreen, OPEN_DELAY_MS, PEEK_DELAY_MS, SWIPE_DISTANCE } from "./behavior";
import { BubbleClock } from "./BubbleClock";
import { WorkdayLine } from "./WorkdayLine";
import { useAlertSound, useCardTime, useLook, useQuietHours } from "./useNotchPrefs";
import { useHistoryLogger } from "../activity/useHistoryLogger";
import { useLockScreenInfo } from "./useLockScreenInfo";
import { useNotchDrag, useNotchFade } from "./useNotchMove";
import {
  activityIslandSize,
  GAME_HUD_SIZE,
  MINI_SIZE,
  HUD_SIZE,
  islandRadius,
  WINDOW_MARGIN,
  type IslandMode,
} from "./geometry";
import { CompactHost, ModuleHost, PeekHost } from "./ModuleHost";
import { useBubbleFit } from "./useBubbleFit";
import { useFirstRunPrompt } from "./useFirstRunPrompt";
import { useBreaks } from "../focus/useBreaks";
import { usePetCheers } from "../focus/usePetCheers";
import { usePetTalk } from "../pet/usePetTalk";
import { useIsland } from "./useIsland";
import { openFirstModule, useModuleLifecycle, useModuleNav } from "./useModules";
import "./island.css";

/** Grace period before a peek collapses after the pointer leaves. */
const PEEK_LEAVE_MS = 200;
/** Let the island finish collapsing before it hides over a fullscreen app. */
const FULLSCREEN_HIDE_DELAY_MS = 450;

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

/** The activity that should hold the island: the most important one not put away. */
function frontActivity(activities: Activity[]) {
  return activities.find((a) => !a.minimized) ?? null;
}

function currentActivity() {
  return frontActivity(useActivities.getState().activities);
}

export function Island() {
  const mode = useIsland((s) => s.mode);
  const size = useIsland((s) => s.size);
  const request = useIsland((s) => s.request);
  const settle = useIsland((s) => s.settle);
  const activity = useActivities((s) => frontActivity(s.activities));
  // Put-away must-answer activities: the bubble shows a dot and a click brings them back.
  const waiting = useActivities((s) => s.activities.some((a) => a.minimized));
  const fullscreen = useFullscreen((s) => s.active);
  const queued = useActivities((s) => Math.max(0, s.activities.filter((a) => !a.minimized).length - 1));
  const reduceMotion = useReduceMotion();
  const hoverTimer = useRef<number | undefined>(undefined);
  const swipe = useRef(idleSwipe());

  useModuleLifecycle();
  useEffect(() => trackFullscreen(), []);
  useFileDrop();
  useFocusSession();
  useGameMode();
  useAmbient();
  const showPet = useSettings((s) => !s.settings?.focus.hidePet);
  const look = useSettings((s) => s.settings?.appearance);
  const bottomEdge = look?.edge === "bottom";
  const notchDrag = useNotchDrag();
  const faded = useNotchFade();
  const fullscreenBehavior = useSettings((s) => s.settings?.behavior.fullscreen ?? "alerts");
  const islandRef = useRef<HTMLElement>(null);
  useMusicPulse(islandRef);
  useBubbleFit(islandRef, mode === "collapsed", look?.bubble === "dot");
  useSystemHud();
  useFirstRunPrompt();
  useBreaks();
  usePetCheers();
  usePetTalk();
  usePetWalkLink();
  useWalkLook();
  useDockProfiles();
  useEveningReview();
  useHistoryLogger();
  useMorningBriefing();
  useLockScreenInfo();
  useWallpaperAccent();
  const petHome = !usePetOut();
  useLook();
  useCardTime();
  useQuietHours();
  useAlertSound();

  // Fullscreen app in front: hidden while resting; whether notifications still pop in is a
  // setting. Hiding waits for the collapse animation to finish.
  const hideForFullscreen =
    fullscreen && hideOverFullscreen(fullscreenBehavior, mode !== "expanded", activity);
  useEffect(() => {
    if (!hideForFullscreen) {
      void ipc.shellSetFullscreenHidden(false).catch(() => {});
      return;
    }
    const timer = window.setTimeout(
      () => void ipc.shellSetFullscreenHidden(true).catch(() => {}),
      FULLSCREEN_HIDE_DELAY_MS,
    );
    return () => window.clearTimeout(timer);
  }, [hideForFullscreen]);

  // Sync the window to the initial mode once the webview is up.
  useEffect(() => {
    void settle();
  }, [settle]);

  // Activities claim the island unless the user has it expanded; once none remain, it
  // returns to collapsed. Expanded is only interrupted by critical activities (an agent
  // blocked on a permission request) — everything else waits in the queue.
  useEffect(() => {
    const { mode: current, request } = useIsland.getState();
    if (activity && (current !== "expanded" || activity.priority === "critical")) {
      const actions = activity.actions ?? [];
      const layout = actions.length === 0 ? "none" : actions.every((a) => a.icon) ? "inline" : "row";
      void request(
        "activity",
        activity.mini
          ? MINI_SIZE
          : activity.game
            ? GAME_HUD_SIZE
            : activity.hud
              ? HUD_SIZE
              : activityIslandSize(layout, Boolean(activity.code)),
      );
    } else if (!activity && current === "activity") {
      void request("collapsed");
    }
  }, [activity, mode]);

  // Keyboard path (Ctrl+Alt+N): after expanding, move focus to the dock so arrows/Tab work.
  // Pointer opens don't move focus, so no focus ring appears on click.
  useEffect(
    () =>
      eventBus.on("ShellToggleRequested", async ({ event }) => {
        // The shortcut targets the island on the screen under the pointer.
        if (event.payload.label !== windowLabel()) return;
        if (useIsland.getState().mode !== "expanded") openFirstModule();
        await useIsland.getState().toggleExpanded();
        if (useIsland.getState().mode === "expanded") focusActiveDockItem();
      }),
    [],
  );

  // Ctrl+Alt+V: open straight to a module (clipboard history), even if already expanded.
  useEffect(
    () =>
      eventBus.on("ShellOpenModuleRequested", async ({ event }) => {
        // "palette:note " opens the palette with "note " already typed.
        const [module, ...rest] = event.payload.module.split(":");
        if (event.payload.label !== windowLabel() || !findModule(module)) return;
        if (module === "palette") usePalette.setState({ prefill: rest.join(":"), at: Date.now() });
        useModuleNav.getState().setActive(module);
        if (useIsland.getState().mode !== "expanded") await useIsland.getState().request("expanded");
        focusActiveDockItem();
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
        // Same as the close button: must-answer activities are put away, not dropped.
        useActivities.getState().close(current.id);
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

  const behavior = () => useSettings.getState().settings?.behavior;

  const onPointerEnter = () => {
    const current = useIsland.getState().mode;
    const prefs = behavior();
    window.clearTimeout(hoverTimer.current);
    if (current === "collapsed" && prefs?.openOnHover) {
      hoverTimer.current = window.setTimeout(() => {
        // Something put away is waiting: hovering brings it back, like a click would.
        if (useActivities.getState().activities.some((a) => a.minimized)) {
          useActivities.getState().restore();
          return;
        }
        openFirstModule();
        void request("expanded");
      }, OPEN_DELAY_MS[prefs.hoverSpeed]);
    } else if (current === "collapsed" && !prefs?.noHoverPeek) {
      scheduleHover("peek", PEEK_DELAY_MS[prefs?.hoverSpeed ?? "normal"]);
    }
    // Don't let an activity vanish while the pointer is on it.
    const shown = currentActivity();
    if (current === "activity" && shown) useActivities.getState().hold(shown.id);
  };

  const onPointerLeave = () => {
    const current = useIsland.getState().mode;
    const prefs = behavior();
    const closeAfter = prefs?.autoCloseSecs
      ? prefs.autoCloseSecs * 1000
      : prefs?.openOnHover
        ? HOVER_CLOSE_MS
        : 0;
    if (current === "peek") scheduleHover("collapsed", PEEK_LEAVE_MS);
    else if (current === "expanded" && closeAfter > 0) scheduleHover("collapsed", closeAfter);
    else window.clearTimeout(hoverTimer.current);
    const shown = currentActivity();
    if (current === "activity" && shown) useActivities.getState().release(shown.id);
  };

  const expand = () => {
    window.clearTimeout(hoverTimer.current);
    // Something put away is still waiting: bring it back instead of opening modules.
    if (useActivities.getState().activities.some((a) => a.minimized)) {
      useActivities.getState().restore();
      return;
    }
    openFirstModule();
    void request("expanded");
  };

  /** Clicking an activity opens its module; transient activities are consumed by that. */
  const openActivity = (shown: Activity) => {
    if (findModule(shown.moduleId)) useModuleNav.getState().setActive(shown.moduleId);
    if (!isPersistent(shown)) useActivities.getState().dismiss(shown.id);
    void request("expanded");
  };

  // Two-finger swipes (design.md §18). Expanded: left/right switches module. On the bubble or
  // peek: left/right skips the track, up opens the notch, down puts it away.
  const onWheel = (e: WheelEvent) => {
    const gestures = useSettings.getState().settings?.gestures;
    if (mode === "activity" || gestures?.disabled) return;
    const distance = SWIPE_DISTANCE[gestures?.sensitivity ?? "normal"];
    const result = addWheel(swipe.current, e.deltaX, e.deltaY, performance.now(), distance);
    swipe.current = result.state;
    const fired = result.swipe;
    if (!fired) return;
    if (mode === "expanded") {
      if (fired === "left" || fired === "right") useModuleNav.getState().step(fired === "right" ? 1 : -1);
      return;
    }
    if (fired === "left" || fired === "right") {
      if (gestures?.noTrackSwipe) return;
      void ipc.mediaCommand({ kind: fired === "right" ? "next" : "previous" }).catch(() => {});
    } else if (fired === "up") {
      window.clearTimeout(hoverTimer.current);
      openFirstModule();
      void request("expanded");
    } else {
      window.clearTimeout(hoverTimer.current);
      void request("collapsed");
    }
  };

  const shownActivity = mode === "activity" ? activity : null;
  const transition = reduceMotion ? INSTANT : isSerious(shownActivity) ? SERIOUS_SPRING : ISLAND_SPRING[mode];

  return (
    <div
      className="island-frame"
      data-edge={bottomEdge ? "bottom" : "top"}
      style={bottomEdge ? { paddingBottom: WINDOW_MARGIN.top } : { paddingTop: WINDOW_MARGIN.top }}
    >
      <motion.section
        ref={islandRef}
        className="island"
        data-mode={mode}
        data-faded={faded && mode === "collapsed" ? "" : undefined}
        data-bubble={look?.bubble ?? "pill"}
        aria-label="Overline"
        initial={false}
        animate={{
          width: size.width,
          height: size.height,
          borderRadius: scaledRadius(islandRadius(size), look?.roundness ?? 100),
        }}
        transition={transition}
        onAnimationComplete={() => void settle()}
        onPointerEnter={onPointerEnter}
        onPointerLeave={onPointerLeave}
        onWheel={onWheel}
      >
        {(mode === "collapsed" || mode === "peek") && (
          <button
            className="island-hit"
            aria-label="Open Overline"
            aria-expanded={false}
            title="Alt+drag to slide the notch along the edge"
            onPointerDown={notchDrag.onPointerDown}
            onPointerMove={notchDrag.onPointerMove}
            onPointerUp={notchDrag.onPointerUp}
            onPointerCancel={notchDrag.onPointerCancel}
            onClick={() => {
              if (!notchDrag.consumeClick()) expand();
            }}
          />
        )}
        <AnimatePresence mode="popLayout" initial={false}>
          <motion.div
            key={shownActivity ? `activity-${shownActivity.id}` : mode}
            className={`island-content island-content--${mode}`}
            initial={{ opacity: 0, scale: 0.97 }}
            animate={{ opacity: 1, scale: 1, transition: contentEnter(mode, reduceMotion) }}
            exit={{ opacity: 0, transition: { duration: 0.08 } }}
          >
            {mode === "collapsed" && (
              <>
                {waiting && (
                  <span className="island-attention" role="img" aria-label="Something is waiting for you" />
                )}
                <PinnedWidget />
                <CompactHost />
                {showPet && petHome && look?.bubble === "pill" && <Pet />}
                {look?.bubbleClock && look.bubble === "pill" && <BubbleClock />}
                {look?.bubble !== "dot" && <WorkdayLine />}
              </>
            )}
            {mode === "peek" && <PeekHost />}
            {shownActivity && (
              <ActivityView
                activity={shownActivity}
                queued={queued}
                onRunAction={(actionId) =>
                  void useActivities.getState().runAction(shownActivity.id, actionId)
                }
                onOpen={() => openActivity(shownActivity)}
                onClose={() => useActivities.getState().close(shownActivity.id)}
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

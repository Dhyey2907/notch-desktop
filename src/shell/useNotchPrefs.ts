// Island-side effects of Settings → Look and Behavior: the look (CSS variables and size),
// card timing, quiet hours and the notification sound.
import { useEffect } from "react";

import { useActivities } from "../activity";
import { setCardTimeScale } from "../activity/activityStore";
import { windowLabel } from "../core/window";
import { useFocus } from "../focus/focusStore";
import { showHeldSummary } from "../focus/useFocusSession";
import { useSettings } from "../settings";
import { applyAppearance, restingSize, setUiScale, SIZE_SCALE } from "./appearance";
import { CARD_TIME_SCALE, inQuietHours } from "./behavior";
import { ISLAND_SIZE } from "./geometry";
import { playAlert } from "./alertSound";
import { useIsland } from "./useIsland";

/** Apply the look to this island window. */
export function useLook() {
  const look = useSettings((s) => s.settings?.appearance);
  useEffect(() => {
    if (!look) return;
    const root = document.documentElement;
    applyAppearance(look, root);
    const scale = SIZE_SCALE[look.size];
    root.style.setProperty("zoom", String(scale));
    setUiScale(scale);
    root.dataset.bubble = look.bubble;
    const resting = restingSize(look);
    ISLAND_SIZE.collapsed = resting;
    const island = useIsland.getState();
    if (island.mode === "collapsed") void island.request("collapsed", resting).then(() => island.refit());
    else void island.refit();
  }, [look]);
}

/** Card timing follows Settings → Behavior. */
export function useCardTime() {
  const cardTime = useSettings((s) => s.settings?.behavior.cardTime ?? "normal");
  useEffect(() => setCardTimeScale(CARD_TIME_SCALE[cardTime]), [cardTime]);
}

/** Quiet hours hold ordinary cards; when they end, one summary shows what waited. */
export function useQuietHours() {
  const quiet = useSettings((s) => s.settings?.behavior.quiet);
  useEffect(() => {
    if (!quiet) return;
    const check = () => {
      const now = inQuietHours(quiet, new Date());
      const was = useFocus.getState().quiet;
      if (now === was) return;
      useFocus.setState({ quiet: now });
      if (!now) showHeldSummary("While it was quiet");
    };
    check();
    const id = window.setInterval(check, 30_000);
    return () => window.clearInterval(id);
  }, [quiet]);
}

/** A sound when a new card arrives (main screen only, so several screens don't all chime). */
export function useAlertSound() {
  useEffect(() => {
    if (windowLabel() !== "main") return;
    let seen = new Set(useActivities.getState().activities.map((a) => a.id));
    return useActivities.subscribe((state) => {
      const fresh = state.activities.filter((a) => !seen.has(a.id) && !a.hud && !a.game);
      seen = new Set(state.activities.map((a) => a.id));
      const behavior = useSettings.getState().settings?.behavior;
      if (fresh.length === 0 || !behavior || behavior.sound === "off") return;
      void playAlert(behavior.sound, behavior.soundVolume);
    });
  }, []);
}

// Game mode: a persistent HUD strip while a game plays fullscreen; other cards wait and are
// summarized when it ends. The first time an app goes fullscreen, ask whether it's a game.
import { useEffect } from "react";

import { useActivities } from "../activity";
import { eventBus } from "../core/eventBus";
import { ipc } from "../core/ipc";
import { useFocus } from "../focus/focusStore";
import { showHeldSummary } from "../focus/useFocusSession";

const HUD_KEY = "game-hud";

export function useGameMode() {
  useEffect(() => {
    let hudId: string | null = null;
    const offHud = eventBus.on("GameHud", ({ event }) => {
      const hud = event.payload;
      if (hud) {
        useFocus.setState({ gaming: true });
        hudId = useActivities.getState().push({
          moduleId: "system",
          priority: "high",
          persistent: true,
          dedupKey: HUD_KEY,
          countRepeats: false,
          title: hud.app,
          game: hud,
        });
      } else {
        useFocus.setState({ gaming: false });
        if (hudId) useActivities.getState().dismiss(hudId);
        hudId = null;
        showHeldSummary("While you played");
      }
    });
    const offAsk = eventBus.on("GameAsk", ({ event }) => {
      const { exe, app } = event.payload;
      useActivities.getState().push({
        moduleId: "system",
        priority: "high",
        tone: "info",
        dedupKey: `game-ask-${exe}`,
        countRepeats: false,
        timeoutMs: 12_000,
        title: `Is ${app} a game?`,
        body: "Shows FPS, GPU and ping, and holds alerts while you play.",
        actions: [
          { id: "no", label: "No", variant: "secondary", run: () => ipc.gameMark(exe, false) },
          { id: "yes", label: "Use game mode", variant: "primary", run: () => ipc.gameMark(exe, true) },
        ],
      });
    });
    return () => {
      offHud();
      offAsk();
    };
  }, []);
}

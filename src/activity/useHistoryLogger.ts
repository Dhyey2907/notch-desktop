// Notch history: every card the notch shows is remembered for two days (Home → What did I miss?).
// Readouts (volume, game HUD, live views) and cards marked private aren't; for private modules
// Rust keeps only the title. Lead island only, so each card is stored once.
import { useEffect } from "react";

import { ipc } from "../core/ipc";
import { isLeadIsland } from "../core/window";
import { useActivities } from ".";

export function useHistoryLogger() {
  const lead = isLeadIsland();
  useEffect(() => {
    if (!lead) return;
    // Cards already up when this starts aren't new.
    const seen = new Set(useActivities.getState().activities.map((a) => `${a.id}:${a.repeats}`));
    return useActivities.subscribe((state) => {
      for (const a of state.activities) {
        const key = `${a.id}:${a.repeats}`;
        if (seen.has(key)) continue;
        seen.add(key);
        if (a.hud || a.game || a.mini || a.private) continue;
        void ipc
          .historyAdd({ at: a.updatedAt, module: a.moduleId, title: a.title, body: a.body, tone: a.tone })
          .catch(() => {});
      }
      if (seen.size > 2000) seen.clear();
    });
  }, [lead]);
}

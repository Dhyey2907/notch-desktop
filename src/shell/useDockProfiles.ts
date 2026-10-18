// Dock profiles switch by themselves (when "auto" is on): to one listing the program in front,
// else one whose hours it is, else the main dock. Lead island only; every island follows the
// setting.
import { useEffect } from "react";

import { eventBus } from "../core/eventBus";
import { isLeadIsland } from "../core/window";
import { useSettings } from "../settings";
import { pickProfile } from "./dockProfiles";

const CHECK_MS = 60_000;

export function useDockProfiles() {
  const lead = isLeadIsland();
  const auto = useSettings((s) => s.settings?.dock.auto ?? false);
  useEffect(() => {
    if (!lead || !auto) return;
    let front: string | null = null;
    const choose = () => {
      const s = useSettings.getState().settings;
      if (!s || !s.dock.auto || s.dock.profiles.length === 0) return;
      const now = new Date();
      const next = pickProfile(s.dock.profiles, front, now.getHours() * 60 + now.getMinutes());
      if (next !== s.dock.active)
        void useSettings.getState().update((x) => ({ ...x, dock: { ...x.dock, active: next } }));
    };
    const off = eventBus.on("FrontAppChanged", ({ event }) => {
      front = event.payload.exe;
      choose();
    });
    choose();
    const id = window.setInterval(choose, CHECK_MS);
    return () => {
      off();
      window.clearInterval(id);
    };
  }, [lead, auto]);
}

// Keeps the lock-screen picture's info panel up to date (every 5 minutes while it's on; Rust
// skips repeats). Runs on the lead island only.
import { useEffect } from "react";

import { ipc } from "../core/ipc";
import { isLeadIsland } from "../core/window";
import { nextEvent, useCalendar } from "../modules/calendar/calendarStore";
import { useMail } from "../modules/mail/mailStore";
import { useMedia } from "../modules/media/mediaStore";
import { usePetLive } from "../pet/petStore";
import { useSettings } from "../settings";
import { lockLines } from "./lockLines";

const EVERY_MS = 5 * 60_000;

export function useLockScreenInfo() {
  const on = useSettings((s) => s.settings?.appearance.lockscreenInfo ?? false);
  const lead = isLeadIsland();
  useEffect(() => {
    if (!on || !lead) return;
    const push = () => {
      const now = Date.now();
      const lines = lockLines(
        {
          next: nextEvent(useCalendar.getState().snapshot?.events ?? [], now),
          weather: usePetLive.getState().weather ?? null,
          media: useMedia.getState().media,
          unread: useMail.getState().state?.unread ?? null,
        },
        now,
      );
      void ipc.lockscreenUpdate(lines).catch(() => {});
    };
    const first = window.setTimeout(push, 5_000);
    const id = window.setInterval(push, EVERY_MS);
    return () => {
      window.clearTimeout(first);
      window.clearInterval(id);
    };
  }, [on, lead]);
}

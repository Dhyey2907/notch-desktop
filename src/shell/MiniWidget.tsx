// One small widget pinned beside the resting bubble, always showing (Settings → Dock & modules,
// or the dock's rearrange bar): a timer counting down, the song playing, your next meeting,
// unread mail, your coding agents, or focus time left. Shows nothing when there's nothing to say.
import { useEffect, useState } from "react";

import type { SystemStats } from "../core/generated/SystemStats";
import { ipc } from "../core/ipc";

import { useFocus } from "../focus/focusStore";
import { useCalendar, nextEvent } from "../modules/calendar/calendarStore";
import { useClaude } from "../modules/claude/claudeStore";
import { useMail } from "../modules/mail/mailStore";
import { useMedia } from "../modules/media/mediaStore";
import { runningTimers, useTodo } from "../modules/todo/todoStore";
import { useSettings } from "../settings";
import { clock, type MiniWidgetId } from "./miniWidgets";

function useNow(everyMs: number, on: boolean): number {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    if (!on) return;
    const id = window.setInterval(() => setNow(Date.now()), everyMs);
    return () => window.clearInterval(id);
  }, [everyMs, on]);
  return now;
}

/** The pinned widget, if any (renders nothing when idle). */
export function PinnedWidget() {
  const pinned = useSettings((s) => s.settings?.dock.pinned) as MiniWidgetId | undefined;
  if (!pinned) return null;
  return <Widget id={pinned} />;
}

/** CPU and memory every few seconds, only while the system widget is pinned. */
function useStats(on: boolean): SystemStats | null {
  const [stats, setStats] = useState<SystemStats | null>(null);
  useEffect(() => {
    if (!on) return;
    let live = true;
    const read = () =>
      ipc.systemStats().then(
        (st) => live && setStats(st),
        () => {},
      );
    void read();
    const timer = window.setInterval(() => void read(), 3000);
    return () => {
      live = false;
      window.clearInterval(timer);
    };
  }, [on]);
  return stats;
}

function Widget({ id }: { id: MiniWidgetId }) {
  const timer = useTodo((s) => runningTimers(s.snapshot?.tasks ?? [])[0]);
  const media = useMedia((s) => s.media);
  const events = useCalendar((s) => s.snapshot?.events);
  const mail = useMail((s) => s.state);
  const live = useClaude((s) => s.status?.live);
  const focusEnds = useFocus((s) => s.state?.endsAt);
  const ticking = (id === "timer" && !!timer) || (id === "focus" && !!focusEnds) || id === "meeting";
  const now = useNow(id === "meeting" ? 30_000 : 1000, ticking);
  const stats = useStats(id === "sysmon");

  let text: string | null = null;
  let title = "";
  switch (id) {
    case "timer":
      if (timer?.timerEndsAt) {
        text = `⏱ ${clock(new Date(timer.timerEndsAt).getTime() - now)}`;
        title = timer.title;
      }
      break;
    case "media":
      if (media && media.status === "playing" && !media.mock) {
        text = `♪ ${media.title}`;
        title = [media.title, media.artist].filter(Boolean).join(" — ");
      }
      break;
    case "meeting": {
      const next = nextEvent(events ?? [], now);
      if (next && !next.allDay) {
        const at = new Date(next.start);
        const mins = Math.round((at.getTime() - now) / 60_000);
        text =
          mins <= 0
            ? `● ${next.title || "Meeting"}`
            : mins < 60
              ? `${mins}m · ${next.title || "Meeting"}`
              : `${at.toLocaleTimeString(undefined, { hour: "numeric", minute: "2-digit" })} · ${next.title || "Meeting"}`;
        title = next.title;
      }
      break;
    }
    case "mail":
      if (mail?.status === "ready" && mail.unread > 0) {
        text = `✉ ${mail.unread}`;
        title = `${mail.unread} unread`;
      }
      break;
    case "claude": {
      const working = (live ?? []).filter((l) => l.state === "working").length;
      const waiting = (live ?? []).filter((l) => l.state === "waiting").length;
      if (working + waiting > 0) {
        text = waiting > 0 ? `⚠ ${waiting} waiting` : `⚙ ${working} working`;
        title = "Coding agents";
      }
      break;
    }
    case "sysmon":
      if (stats?.cpu !== undefined || stats?.ram !== undefined) {
        text = `CPU ${stats.cpu ?? "–"}% · RAM ${stats.ram ?? "–"}%`;
        title = "Control → System for the busiest apps";
      }
      break;
    case "focus":
      if (focusEnds) {
        const left = new Date(focusEnds).getTime() - now;
        if (left > 0) {
          text = `◎ ${clock(left)}`;
          title = "Focus time left";
        }
      }
      break;
  }
  if (!text) return null;
  return (
    <span className="mini-widget" title={title}>
      {text}
    </span>
  );
}

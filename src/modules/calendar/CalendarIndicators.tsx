import { CalendarDays } from "lucide-react";
import { useEffect, useState } from "react";

import { MockBadge } from "../../design/components";
import { nextEvent, relativeStart, useCalendar } from "./calendarStore";
import "./calendar.css";

/** Show the collapsed indicator only when a meeting is this close. */
const SOON_MS = 30 * 60_000;

/** Re-render once a minute; enough for "in N min" labels. Local only, no IPC. */
function useMinuteClock() {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const id = window.setInterval(() => setNow(Date.now()), 30_000);
    return () => window.clearInterval(id);
  }, []);
  return now;
}

export function CalendarCompact() {
  const events = useCalendar((s) => s.snapshot?.events);
  const now = useMinuteClock();
  const next = events ? nextEvent(events, now) : null;
  if (!next || Date.parse(next.start) - now > SOON_MS) return null;
  const minutes = Math.max(1, Math.ceil((Date.parse(next.start) - now) / 60_000));
  return (
    <span className="calendar-compact" role="img" aria-label={`${next.title} in ${minutes} minutes`}>
      <CalendarDays size={12} aria-hidden />
      {minutes}m
    </span>
  );
}

export function CalendarPeek() {
  const snapshot = useCalendar((s) => s.snapshot);
  const now = useMinuteClock();
  const connected = snapshot?.status.state === "connected" || snapshot?.status.state === "error";
  const next = snapshot && connected ? nextEvent(snapshot.events, now) : null;
  return (
    <div className="module-peek">
      <span className="module-peek__title">
        {next ? next.title : connected ? "No more meetings" : "Calendar"} {snapshot?.mock && <MockBadge />}
      </span>
      <span className="module-peek__meta">
        {next
          ? relativeStart(next, now)
          : connected
            ? "Enjoy the free time"
            : "Click to connect your calendar"}
      </span>
    </div>
  );
}

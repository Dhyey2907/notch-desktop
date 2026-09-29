import { RefreshCw, Video } from "lucide-react";
import { useEffect, useState } from "react";

import type { CalendarEvent } from "../../core/generated/CalendarEvent";
import type { CalendarSnapshot } from "../../core/generated/CalendarSnapshot";
import { ipc } from "../../core/ipc";
import {
  Button,
  EmptyState,
  ErrorState,
  IconButton,
  MockBadge,
  ModuleContainer,
  Pill,
} from "../../design/components";
import {
  eventsOn,
  formatClock,
  isOngoing,
  isStale,
  monthGrid,
  nextEvent,
  relativeStart,
  sameLocalDay,
  useCalendar,
} from "./calendarStore";
import "./calendar.css";

function useClock(intervalMs: number) {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const id = window.setInterval(() => setNow(Date.now()), intervalMs);
    return () => window.clearInterval(id);
  }, [intervalMs]);
  return now;
}

const openSettings = () => void ipc.openSettingsWindow("calendar");

export default function CalendarExpanded() {
  const snapshot = useCalendar((s) => s.snapshot);
  const now = useClock(30_000);

  if (!snapshot) return <ModuleContainer title="Calendar">{null}</ModuleContainer>;
  const { status } = snapshot;

  if (status.state === "notConfigured" || status.state === "disconnected") {
    return (
      <ModuleContainer title="Calendar">
        <EmptyState
          title="Your day lives here."
          body="Connect Google Calendar to see meetings and get a countdown before they start."
          action={
            <Button variant="primary" onClick={openSettings}>
              Connect Calendar
            </Button>
          }
        />
      </ModuleContainer>
    );
  }
  if (status.state === "connecting") {
    return (
      <ModuleContainer title="Calendar">
        <EmptyState title="Finishing sign-in…" body="Complete the Google sign-in in your browser." />
      </ModuleContainer>
    );
  }
  if (status.state === "error" && status.reason === "authExpired") {
    return (
      <ModuleContainer title="Calendar">
        <ErrorState
          title="Calendar disconnected"
          body="Reconnect your account to restore meeting and reminder updates."
          action={<Button onClick={openSettings}>Reconnect</Button>}
        />
      </ModuleContainer>
    );
  }

  return <Agenda snapshot={snapshot} now={now} />;
}

function Agenda({ snapshot, now }: { snapshot: CalendarSnapshot; now: number }) {
  const today = new Date(now);
  const todays = eventsOn(snapshot.events, today);
  const upcoming = todays.filter((e) => e.allDay || Date.parse(e.end) > now);
  const next = nextEvent(snapshot.events, now);
  const offline = snapshot.status.state === "error";
  const daysWithEvents = new Set(snapshot.events.map((e) => new Date(e.start).toDateString()));

  const trailing = (
    <>
      {snapshot.mock && <MockBadge />}
      {offline ? (
        <Pill tone="warning">Offline · showing saved events</Pill>
      ) : (
        isStale(snapshot, now) && <Pill tone="warning">May be out of date</Pill>
      )}
      <IconButton icon={RefreshCw} label="Sync now" size="sm" onClick={() => void ipc.calendarSyncNow()} />
    </>
  );

  return (
    <ModuleContainer title="Calendar" trailing={trailing}>
      <div className="calendar">
        <MonthCard today={today} daysWithEvents={daysWithEvents} />
        <div className="calendar__agenda">
          <p className="calendar__date">
            {today.toLocaleDateString(undefined, { weekday: "long", day: "numeric", month: "long" })}
          </p>
          <p className="calendar__summary">
            {todays.length === 0
              ? "Nothing scheduled today"
              : `${todays.length} ${todays.length === 1 ? "event" : "events"} today`}
          </p>
          {upcoming.length > 0 ? (
            <ol className="calendar__list">
              {upcoming.slice(0, 5).map((event) => (
                <AgendaRow key={`${event.id}${event.start}`} event={event} now={now} />
              ))}
            </ol>
          ) : (
            next && (
              <p className="calendar__next">
                Next: {next.title} ·{" "}
                {sameLocalDay(new Date(next.start), today)
                  ? formatClock(next.start)
                  : new Date(next.start).toLocaleString(undefined, {
                      weekday: "short",
                      hour: "numeric",
                      minute: "2-digit",
                    })}
              </p>
            )
          )}
        </div>
      </div>
    </ModuleContainer>
  );
}

function AgendaRow({ event, now }: { event: CalendarEvent; now: number }) {
  const ongoing = isOngoing(event, now);
  const soon = !event.allDay && !ongoing && Date.parse(event.start) - now < 60 * 60_000;
  return (
    <li className={`calendar__row${ongoing ? " calendar__row--now" : ""}`}>
      <span className="calendar__time">{event.allDay ? "All day" : formatClock(event.start)}</span>
      <span className="calendar__title" title={event.title}>
        {event.title}
      </span>
      {ongoing ? (
        <Pill tone="info">Now</Pill>
      ) : (
        soon && <span className="calendar__soon">{relativeStart(event, now)}</span>
      )}
      {event.joinUrl && (
        <IconButton
          icon={Video}
          label={`Join ${event.title}`}
          size="sm"
          onClick={() => void ipc.calendarOpenLink(event.joinUrl!)}
        />
      )}
    </li>
  );
}

function MonthCard({ today, daysWithEvents }: { today: Date; daysWithEvents: Set<string> }) {
  const weeks = monthGrid(today);
  return (
    <div className="calendar__month" aria-hidden>
      <p className="calendar__month-name">{today.toLocaleDateString(undefined, { month: "long" })}</p>
      <div className="calendar__grid">
        {["M", "T", "W", "T", "F", "S", "S"].map((d, i) => (
          <span key={i} className="calendar__weekday">
            {d}
          </span>
        ))}
        {weeks.flat().map((day, i) =>
          day ? (
            <span
              key={i}
              className={`calendar__day${sameLocalDay(day, today) ? " calendar__day--today" : ""}${
                daysWithEvents.has(day.toDateString()) ? " calendar__day--busy" : ""
              }`}
            >
              {day.getDate()}
            </span>
          ) : (
            <span key={i} />
          ),
        )}
      </div>
    </div>
  );
}

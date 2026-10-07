// Calendar actions: a quick "new event" form and answers to invites. Both need
// "Add events and answer invites" in Settings → Calendar (and Google's OK at sign-in).
import { Check, HelpCircle, X } from "lucide-react";
import { useState } from "react";

import type { CalendarEvent } from "../../core/generated/CalendarEvent";
import type { InviteResponse } from "../../core/generated/InviteResponse";
import { ipc, isIpcError } from "../../core/ipc";
import { Button } from "../../design/components";
import { DURATIONS, buildEvent, dateInput, nextHalfHour } from "./eventForm";

function message(error: unknown) {
  return isIpcError(error) ? error.message.replace(/^invalid input: /, "") : "That didn't work.";
}

export function NewEventForm({ onDone }: { onDone: (status: string) => void }) {
  const [now] = useState(() => new Date());
  const [title, setTitle] = useState("");
  const [date, setDate] = useState(() => dateInput(now));
  const [time, setTime] = useState(() => nextHalfHour(now));
  const [minutes, setMinutes] = useState(30);
  const [allDay, setAllDay] = useState(false);
  const [location, setLocation] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const save = async () => {
    const event = buildEvent(title, date, time, minutes, allDay, location);
    if (!event || !event.title) {
      setError("Give it a name, a day and a time");
      return;
    }
    setBusy(true);
    try {
      await ipc.calendarCreateEvent(event);
      onDone(`Added “${event.title}”`);
    } catch (err) {
      setError(message(err));
    } finally {
      setBusy(false);
    }
  };

  return (
    <form
      className="calendar-form"
      aria-label="New event"
      onSubmit={(e) => {
        e.preventDefault();
        void save();
      }}
    >
      <input
        className="nd-input calendar-form__title"
        aria-label="Event name"
        placeholder="Event name"
        value={title}
        maxLength={200}
        autoFocus
        onChange={(e) => setTitle(e.target.value)}
      />
      <div className="calendar-form__row">
        <input
          className="nd-input"
          type="date"
          aria-label="Day"
          value={date}
          onChange={(e) => setDate(e.target.value)}
        />
        {!allDay && (
          <input
            className="nd-input"
            type="time"
            aria-label="Start time"
            value={time}
            onChange={(e) => setTime(e.target.value)}
          />
        )}
        <button
          type="button"
          className={`nd-chip${allDay ? " nd-chip--on" : ""}`}
          aria-pressed={allDay}
          onClick={() => setAllDay(!allDay)}
        >
          All day
        </button>
      </div>
      {!allDay && (
        <div className="calendar-form__row" role="group" aria-label="How long">
          {DURATIONS.map((m) => (
            <button
              key={m}
              type="button"
              className={`nd-chip${minutes === m ? " nd-chip--on" : ""}`}
              aria-pressed={minutes === m}
              onClick={() => setMinutes(m)}
            >
              {m < 60 ? `${m} min` : `${m / 60} h`}
            </button>
          ))}
        </div>
      )}
      <div className="calendar-form__row">
        <input
          className="nd-input calendar-form__where"
          aria-label="Where (optional)"
          placeholder="Where (optional)"
          value={location}
          maxLength={200}
          onChange={(e) => setLocation(e.target.value)}
        />
        <Button variant="primary" disabled={busy || !title.trim()} onClick={() => void save()}>
          {busy ? "Adding…" : "Add to Calendar"}
        </Button>
      </div>
      {error && <p className="calendar-form__error">{error}</p>}
    </form>
  );
}

const ANSWERS: { answer: InviteResponse; label: string; icon: typeof Check }[] = [
  { answer: "accepted", label: "Yes", icon: Check },
  { answer: "tentative", label: "Maybe", icon: HelpCircle },
  { answer: "declined", label: "No", icon: X },
];

/** Yes / Maybe / No for an invite; the organizer is told, as in Google Calendar. */
export function InviteAnswer({
  event,
  onStatus,
}: {
  event: CalendarEvent;
  onStatus: (status: string) => void;
}) {
  const [busy, setBusy] = useState<InviteResponse | null>(null);
  const answer = async (a: InviteResponse, label: string) => {
    setBusy(a);
    try {
      await ipc.calendarRespond(event.id, a);
      onStatus(`Answered “${label}” to ${event.title}`);
    } catch (err) {
      onStatus(message(err));
    } finally {
      setBusy(null);
    }
  };
  return (
    <span className="calendar-invite" role="group" aria-label={`Answer ${event.title}`}>
      {ANSWERS.map(({ answer: a, label, icon: Icon }) => (
        <button
          key={a}
          type="button"
          className={`nd-chip${event.myResponse === a ? " nd-chip--on" : ""}`}
          aria-pressed={event.myResponse === a}
          disabled={busy !== null}
          onClick={() => void answer(a, label)}
        >
          <Icon size={12} aria-hidden />
          {label}
        </button>
      ))}
    </span>
  );
}

// The away screen (#away): a dim, glanceable view while you're away. A big clock, then the
// next meeting, weather, what's playing and unread mail. It drifts slowly so nothing burns in;
// any key or mouse movement (watched by Overline) takes it away.
import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";

import type { CalendarSnapshot } from "../core/generated/CalendarSnapshot";
import type { Dashboard } from "../core/generated/Dashboard";
import type { MailState } from "../core/generated/MailState";
import type { MediaState } from "../core/generated/MediaState";
import { nextEvent } from "../modules/calendar/calendarStore";
import { lockLines } from "../shell/lockLines";
import "./away.css";

const REFRESH_MS = 60_000;
const DRIFT_MS = 45_000;

export function AwayApp() {
  const [now, setNow] = useState(() => new Date());
  const [lines, setLines] = useState<string[]>([]);
  const [drift, setDrift] = useState({ x: 0, y: 0 });

  useEffect(() => {
    const tick = window.setInterval(() => setNow(new Date()), 1000);
    const move = window.setInterval(
      () =>
        setDrift({ x: Math.round((Math.random() - 0.5) * 120), y: Math.round((Math.random() - 0.5) * 80) }),
      DRIFT_MS,
    );
    return () => {
      window.clearInterval(tick);
      window.clearInterval(move);
    };
  }, []);

  useEffect(() => {
    const read = async () => {
      const [cal, dash, media, mail] = await Promise.all([
        invoke<CalendarSnapshot>("calendar_get_snapshot").catch(() => null),
        invoke<Dashboard>("system_dashboard").catch(() => null),
        invoke<MediaState | null>("media_get_state").catch(() => null),
        invoke<MailState>("mail_state").catch(() => null),
      ]);
      const t = Date.now();
      setLines(
        lockLines(
          {
            next: nextEvent(cal?.events ?? [], t),
            weather: dash?.weather ?? null,
            media,
            unread: mail?.status === "ready" ? mail.unread : null,
          },
          t,
        ),
      );
    };
    void read();
    const id = window.setInterval(() => void read(), REFRESH_MS);
    return () => window.clearInterval(id);
  }, []);

  return (
    <div className="away">
      <div className="away__inner" style={{ transform: `translate(${drift.x}px, ${drift.y}px)` }}>
        <p className="away__time">{now.toLocaleTimeString([], { hour: "numeric", minute: "2-digit" })}</p>
        <p className="away__date">
          {now.toLocaleDateString([], { weekday: "long", day: "numeric", month: "long" })}
        </p>
        {lines.length > 0 && (
          <ul className="away__lines">
            {lines.map((l) => (
              <li key={l}>{l}</li>
            ))}
          </ul>
        )}
        <p className="away__hint">Move the mouse or press a key to come back</p>
      </div>
    </div>
  );
}

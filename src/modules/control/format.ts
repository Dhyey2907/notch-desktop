// Words and numbers for the Control center and its alerts.
import type { SystemAlert } from "../../core/generated/SystemAlert";
import type { Tone } from "../../design/components";

/** 1_250_000 → "1.2 MB/s". */
export function formatSpeed(bytesPerSecond: number): string {
  if (bytesPerSecond >= 1_000_000) return `${(bytesPerSecond / 1_000_000).toFixed(1)} MB/s`;
  if (bytesPerSecond >= 1_000) return `${Math.round(bytesPerSecond / 1_000)} KB/s`;
  return `${bytesPerSecond} B/s`;
}

/** 70 → "1 h 10 min". */
export function formatMinutes(minutes: number): string {
  if (minutes < 60) return `${minutes} min`;
  const h = Math.floor(minutes / 60);
  const m = minutes % 60;
  return m ? `${h} h ${m} min` : `${h} h`;
}

export interface AlertCard {
  title: string;
  body?: string;
  tone: Tone;
  /** Lock keys and language show as a slim readout instead of a card. */
  hud?: { kind: "caps" | "numlock" | "language"; on: boolean };
}

export function alertCard(alert: SystemAlert): AlertCard {
  switch (alert.kind) {
    case "internetDown":
      return {
        title: "No internet",
        body: "Your connection dropped. You'll see when it's back.",
        tone: "warning",
      };
    case "internetBack":
      return {
        title: "Back online",
        body:
          alert.seconds >= 60
            ? `It was down for ${formatMinutes(Math.round(alert.seconds / 60))}.`
            : undefined,
        tone: "success",
      };
    case "chargeLimit":
      return {
        title: `Battery at ${alert.percent}%`,
        body: "Unplug now to keep the battery healthy for longer.",
        tone: "info",
      };
    case "plugged":
      return {
        title: `Charging · ${alert.percent}%`,
        body: alert.minutes != null ? `Full in about ${formatMinutes(alert.minutes)}.` : undefined,
        tone: "success",
      };
    case "deviceLow":
      return {
        title: `${alert.name} battery low`,
        body: `${alert.percent}% left. Time to charge it.`,
        tone: "warning",
      };
    case "capsLock":
      return {
        title: alert.on ? "Caps Lock on" : "Caps Lock off",
        tone: "neutral",
        hud: { kind: "caps", on: alert.on },
      };
    case "numLock":
      return {
        title: alert.on ? "Num Lock on" : "Num Lock off",
        tone: "neutral",
        hud: { kind: "numlock", on: alert.on },
      };
    case "language":
      return { title: alert.name, tone: "neutral", hud: { kind: "language", on: true } };
  }
}

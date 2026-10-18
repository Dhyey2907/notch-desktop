// Words and numbers for the Control center and its alerts.
import type { SpeedTest } from "../../core/generated/SpeedTest";
import type { SystemAlert } from "../../core/generated/SystemAlert";
import type { Tone } from "../../design/components";

/** 1_250_000 → "1.2 MB/s". */
export function formatSpeed(bytesPerSecond: number): string {
  if (bytesPerSecond >= 1_000_000) return `${(bytesPerSecond / 1_000_000).toFixed(1)} MB/s`;
  if (bytesPerSecond >= 1_000) return `${Math.round(bytesPerSecond / 1_000)} KB/s`;
  return `${bytesPerSecond} B/s`;
}

/** The line under the speed test: what it's doing, or when it last ran. */
export function speedTestLine(test: SpeedTest, now: number): string {
  switch (test.phase) {
    case "ping":
      return "Checking the delay…";
    case "download":
      return "Measuring download…";
    case "upload":
      return "Measuring upload…";
    case "failed":
      return "The test didn't finish — check your connection and try again.";
    case "done": {
      const minutes = Math.round((now - Date.parse(test.at)) / 60_000);
      const when =
        minutes < 1
          ? "just now"
          : minutes < 60
            ? `${minutes} min ago`
            : new Date(test.at).toLocaleString(undefined, {
                weekday: "short",
                hour: "numeric",
                minute: "2-digit",
              });
      return `Tested ${when} · speed.cloudflare.com`;
    }
  }
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

/** "1.2 GB", "340 MB", "12 KB". */
export function formatBytes(bytes: number): string {
  const units = ["B", "KB", "MB", "GB", "TB"];
  let n = Math.max(0, bytes);
  let i = 0;
  while (n >= 1000 && i < units.length - 1) {
    n /= 1000;
    i += 1;
  }
  return `${n >= 100 || i === 0 ? Math.round(n) : n.toFixed(1)} ${units[i]}`;
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
    case "dataCap":
      return {
        title: alert.percent >= 100 ? "Data cap reached" : `${alert.percent}% of your data cap used`,
        body: `This month on this PC, against your ${alert.cap_gb} GB cap.`,
        tone: alert.percent >= 100 ? "danger" : "warning",
      };
    case "batteryLow":
      return {
        title: `Battery at ${alert.percent}%`,
        body: alert.percent <= 10 ? "Plug in soon — it's almost empty." : "Time to find a charger.",
        tone: alert.percent <= 10 ? "danger" : "warning",
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

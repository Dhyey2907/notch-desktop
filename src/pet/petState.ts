// What the pet looks like right now, from what's going on: music, video, your coding agents, the
// weather, the battery, how full your day is, and how long you've been away. Pure, so it's tested.
import type { PetSettings } from "../core/generated/PetSettings";

export type Face = "happy" | "focused" | "sleepy" | "cheer" | "stressed" | "nervous" | "droopy" | "love";
export type Prop = "none" | "headphones" | "popcorn" | "laptop";
export type Outfit = "none" | "umbrella" | "sunglasses" | "scarf";
export type Motion = "idle" | "dance" | "bounce" | "wiggle";

export interface PetLook {
  face: Face;
  prop: Prop;
  outfit: Outfit;
  motion: Motion;
  /** What it's up to, in a few words ("Dancing to your music"). */
  status: string;
}

export interface PetInputs {
  now: number;
  /** Local hour 0–23. */
  hour: number;
  idleMs: number;
  /** Cheering (a session done, a break taken…) until this time. */
  cheeringUntil: number;
  /** Just patted, until this time. */
  lovedUntil: number;
  focusing: boolean;
  music: boolean;
  video: boolean;
  coding: "none" | "working" | "waiting";
  /** A code editor or terminal is in front (you're coding yourself). */
  editing?: boolean;
  /** Who's coding, for the status line ("Claude Code"). */
  agent?: string;
  /** Weather icon (sun, rain, snow, storm…) and temperature, when known. */
  weather?: { icon?: string; temperature?: number } | null;
  battery?: { percent: number; plugged: boolean } | null;
  /** Lots of meetings or a deadline close. */
  busyDay: boolean;
  settings: PetSettings;
}

const SLEEPY_AFTER_MS = 10 * 60_000;
export const LOW_BATTERY = 20;

export function outfitFor(weather: PetInputs["weather"]): Outfit {
  if (!weather) return "none";
  const icon = weather.icon ?? "";
  const t = weather.temperature;
  if (/rain|drizzle|storm/.test(icon)) return "umbrella";
  if (icon === "snow" || (t !== undefined && t <= 12)) return "scarf";
  if (icon === "sun" && t !== undefined && t >= 30) return "sunglasses";
  return "none";
}

export function petLook(i: PetInputs): PetLook {
  const s = i.settings;
  const lowBattery = s.reactBattery && !!i.battery && !i.battery.plugged && i.battery.percent <= LOW_BATTERY;
  const lateNight = s.reactDay && i.hour >= 1 && i.hour < 5 && i.idleMs < SLEEPY_AFTER_MS;
  const waiting = s.reactCoding && i.coding === "waiting";
  const coding = s.reactCoding && (i.coding === "working" || !!i.editing);
  const video = s.reactVideo && i.video;
  const music = s.reactMusic && i.music && !video;

  const prop: Prop = video ? "popcorn" : music ? "headphones" : coding ? "laptop" : "none";
  const outfit: Outfit = s.reactWeather ? outfitFor(i.weather) : "none";

  let face: Face;
  let status: string;
  if (i.lovedUntil > i.now) {
    face = "love";
    status = "Loves the attention";
  } else if (i.cheeringUntil > i.now) {
    face = "cheer";
    status = "Cheering for you";
  } else if (waiting) {
    face = "nervous";
    status = `Nervous — ${i.agent ?? "your agent"} is waiting for you`;
  } else if (lowBattery) {
    face = "droopy";
    status = `Low on energy — battery at ${i.battery!.percent}%`;
  } else if (i.idleMs >= SLEEPY_AFTER_MS) {
    face = "sleepy";
    status = "Napping while you're away";
  } else if (lateNight) {
    face = "sleepy";
    status = "Yawning — it's late";
  } else if (s.reactDay && i.busyDay) {
    face = "stressed";
    status = "A busy day — hang in there";
  } else if (i.focusing || coding) {
    face = "focused";
    status = coding
      ? i.coding === "working"
        ? `Coding alongside ${i.agent ?? "your agent"}`
        : "Coding with you"
      : "Focusing with you";
  } else {
    face = "happy";
    status = "Happy to be here";
  }
  // What it's doing beats a plain mood in the status line.
  if (face === "happy" || face === "focused") {
    if (video) status = "Watching with you";
    else if (music) status = "Dancing to your music";
  }

  const motion: Motion =
    face === "love" ? "wiggle" : face === "cheer" ? "bounce" : music && face !== "sleepy" ? "dance" : "idle";
  return { face, prop, outfit, motion, status };
}

/** Level from experience: 1 at 0 xp, then every level needs a little more. */
export function levelFor(xp: number): { level: number; into: number; needed: number } {
  let level = 1;
  let need = 10;
  let left = Math.max(0, xp);
  while (left >= need) {
    left -= need;
    level += 1;
    need = Math.round(need * 1.4);
  }
  return { level, into: left, needed: need };
}

/** Whole days since `born` (YYYY-MM-DD, local). */
export function ageInDays(born: string, now: Date): number {
  const [y, m, d] = born.split("-").map(Number);
  if (!y || !m || !d) return 0;
  const start = new Date(y, m - 1, d).getTime();
  const today = new Date(now.getFullYear(), now.getMonth(), now.getDate()).getTime();
  return Math.max(0, Math.round((today - start) / 86_400_000));
}

// Quick capture, read locally (nothing leaves the PC unless you ask the AI): what a line is and
// when. "remind me at 5 to call mom" → reminder, "note: …" → note, "lunch with Ann friday 1pm"
// → calendar event, a link → read later, "tea 4m" → task with a timer, anything else → task.
import type { CaptureKind } from "../../core/generated/CaptureKind";
import { parseQuickAdd } from "../todo/parse";

export interface Capture {
  kind: CaptureKind;
  /** Title (or note text) with the time words taken out. */
  title: string;
  /** Epoch ms, for reminders and events. */
  at?: number;
  /** A timer, for tasks ("tea 4m"). */
  minutes?: number;
  /** For links. */
  url?: string;
}

const WEEKDAYS = ["sunday", "monday", "tuesday", "wednesday", "thursday", "friday", "saturday"];
const NOTE_PREFIX = /^(?:note|idea|n|thought)\s*[:-]\s*/i;
const EVENT_PREFIX = /^(?:event|meeting|cal|calendar)\s*[:-]\s*/i;
const LINK_PREFIX = /^(?:read(?:\s+later)?|later|link)\s*[:-]?\s*/i;
const TASK_PREFIX = /^(?:task|todo)\s*[:-]\s*/i;
const REMIND_WORDS = /\b(?:remind me(?: to| about)?|don'?t forget(?: to)?|remember to)\b\s*/i;
const EVENT_WORDS =
  /\b(?:meeting|meet|lunch|dinner|breakfast|coffee|call with|appointment|appt|interview|party|class|lecture|doctor|dentist|flight|standup|sync|1:1)\b/i;
const URL = /https?:\/\/[^\s<>"')]+/i;

function clean(text: string): string {
  return text
    .replace(/\s{2,}/g, " ")
    .trim()
    .replace(/^(?:to|about)\s+/i, "")
    .replace(/\s+(?:at|on|by|for)$/i, "")
    .replace(/^[\s,.;:-]+|[\s,;:-]+$/g, "")
    .trim();
}

interface When {
  at: number;
  /** The text with the time words removed. */
  rest: string;
}

function dayStart(d: Date, plus = 0): Date {
  return new Date(d.getFullYear(), d.getMonth(), d.getDate() + plus);
}

/** Find a time in the text: "at 5", "5:30pm", "17:00", "noon", "tomorrow", "friday", "in 2 hours". */
export function findWhen(text: string, now: Date): When | null {
  let rest = ` ${text} `;
  const take = (re: RegExp): RegExpExecArray | null => {
    const m = re.exec(rest);
    if (m) rest = rest.slice(0, m.index) + " " + rest.slice(m.index + m[0].length);
    return m;
  };

  // "in 20 minutes", "in 2 hours", "in an hour"
  const rel = take(/\s(?:in)\s+(an?|\d{1,3})\s*(m|min|mins|minutes?|h|hrs?|hours?)\b/i);
  if (rel) {
    const n = /^an?$/i.test(rel[1]) ? 1 : Number(rel[1]);
    const ms = /^h/i.test(rel[2]) ? n * 3600_000 : n * 60_000;
    return { at: now.getTime() + ms, rest: clean(rest) };
  }

  let day: Date | null = null;
  let dayNamed = false;
  if (take(/\s(?:today)\b/i)) {
    day = dayStart(now);
    dayNamed = true;
  } else if (take(/\s(?:tomorrow|tmrw|tmr)\b/i)) {
    day = dayStart(now, 1);
    dayNamed = true;
  } else {
    const wd = take(
      /\s(?:on\s+|next\s+)?(sun|mon|tue|tues|wed|thu|thur|thurs|fri|sat)(?:day|nesday|sday|urday)?\b/i,
    );
    if (wd) {
      const target = WEEKDAYS.findIndex((w) => w.startsWith(wd[1].toLowerCase().slice(0, 3)));
      let plus = (target - now.getDay() + 7) % 7;
      if (plus === 0) plus = 7;
      day = dayStart(now, plus);
      dayNamed = true;
    }
  }

  let hour: number | null = null;
  let minute = 0;
  let meridiem: string | null = null;
  const named = take(/\s(?:at\s+)?(noon|midday|midnight|morning|evening|tonight|afternoon)\b/i);
  if (named) {
    const word = named[1].toLowerCase();
    hour = { noon: 12, midday: 12, midnight: 0, morning: 9, afternoon: 15, evening: 18, tonight: 20 }[word]!;
    if (word === "midnight" && !dayNamed) day = dayStart(now, 1);
    meridiem = "set";
  } else {
    const clock =
      take(/\s(?:at\s+|@\s*)(\d{1,2})(?::(\d{2}))?\s*(am|pm|a\.m\.|p\.m\.)?(?=[\s,.!?]|$)/i) ??
      take(/\s(\d{1,2})(?::(\d{2}))?\s*(am|pm|a\.m\.|p\.m\.)(?=[\s,.!?]|$)/i) ??
      take(/\s(\d{1,2}):(\d{2})()(?=[\s,.!?]|$)/i);
    if (clock) {
      hour = Number(clock[1]);
      minute = Number(clock[2] ?? 0);
      meridiem = clock[3] ? clock[3].toLowerCase().replace(/\./g, "") : null;
      if (hour > 23 || minute > 59) return null;
      if (meridiem === "pm" && hour < 12) hour += 12;
      if (meridiem === "am" && hour === 12) hour = 0;
    }
  }

  if (hour === null && day === null) return null;
  if (hour === null) hour = 9; // "tomorrow" alone: the morning.

  const base = day ?? dayStart(now);
  const at = (h: number) =>
    new Date(base.getFullYear(), base.getMonth(), base.getDate(), h, minute).getTime();
  let when = at(hour);
  if (meridiem === null && hour < 12) {
    // "at 5" means 5 pm; "at 9" means the next 9 still ahead.
    const pm = at(hour + 12);
    if (hour <= 7) when = pm;
    else if (!dayNamed && when <= now.getTime()) when = pm;
  }
  if (!dayNamed && when <= now.getTime()) {
    when += 24 * 3600_000;
  }
  return { at: when, rest: clean(rest) };
}

export function parseCapture(input: string, now: Date): Capture {
  const text = input.trim();
  if (NOTE_PREFIX.test(text)) return { kind: "note", title: text.replace(NOTE_PREFIX, "").trim() };

  const url = URL.exec(text)?.[0].replace(/[.,;:!?]+$/, "");
  if (url) {
    const words = text.replace(url, "").replace(LINK_PREFIX, "").trim();
    // A bare link, or "read later <link>", goes to Read later; a link inside a task stays a task.
    if (!words || LINK_PREFIX.test(text)) return { kind: "link", title: url, url };
  }

  if (TASK_PREFIX.test(text)) {
    const q = parseQuickAdd(text.replace(TASK_PREFIX, ""));
    return { kind: "task", title: q.title, minutes: q.minutes };
  }

  const eventPrefix = EVENT_PREFIX.test(text);
  const body = text.replace(EVENT_PREFIX, "");
  const remind = REMIND_WORDS.test(body);
  const when = findWhen(body.replace(REMIND_WORDS, " "), now);

  if (eventPrefix || (!remind && when && EVENT_WORDS.test(body))) {
    return { kind: "event", title: clean(when ? when.rest : body), at: when?.at };
  }
  if (when) return { kind: "reminder", title: when.rest || clean(body), at: when.at };
  if (remind) return { kind: "reminder", title: clean(body.replace(REMIND_WORDS, " ")) };

  const q = parseQuickAdd(text);
  return { kind: "task", title: q.title, minutes: q.minutes };
}

export const KIND_LABELS: Record<CaptureKind, string> = {
  task: "Task",
  reminder: "Reminder",
  note: "Note",
  event: "Event",
  link: "Read later",
};

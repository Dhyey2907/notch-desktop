// The pet's voice (main island only): it notices moments — a meeting soon, a break taken, the
// battery running low, a late night — and says something in the tone you chose. Lines for things
// Notch already shows a card for (meetings, coding agents) appear in the Pet section only, so
// nothing is said twice. It also earns experience as you work.
import { useEffect } from "react";

import { useActivities } from "../activity";
import { eventBus } from "../core/eventBus";
import type { PetSettings } from "../core/generated/PetSettings";
import type { Weather } from "../core/generated/Weather";
import { ipc } from "../core/ipc";
import { isLeadIsland } from "../core/window";
import { useFocus, focusActive } from "../focus/focusStore";
import { useCalendar } from "../modules/calendar/calendarStore";
import { useControl } from "../modules/control/controlStore";
import { useSettings } from "../settings";
import { inQuietHours } from "../shell/behavior";
import { lineFor, maySpeak, type LineEvent } from "./lines";
import { LOW_BATTERY } from "./petState";
import { onPetNews, usePetLive, type PetNews } from "./petStore";

const TOPIC: Record<LineEvent, keyof PetSettings | null> = {
  greeting: "sayGreeting",
  meeting: "sayMeetings",
  focusDone: "sayFocus",
  breakDone: "sayBreaks",
  streak: "sayBreaks",
  agentDone: "sayAgents",
  agentWaiting: "sayAgents",
  batteryLow: "sayBattery",
  plugged: "sayBattery",
  late: "sayLate",
  rain: "sayWeather",
  pat: null,
  hello: null,
  treat: null,
  coding: null,
  watching: null,
  chat: null,
};
/** Notch already shows a card for these; the pet's line goes to its own section only. */
const SECTION_ONLY: LineEvent[] = ["meeting", "agentDone", "agentWaiting", "pat"];
const MEETING_LEAD_MS = 5 * 60_000;
const CHECK_MS = 30_000;

function today(now = new Date()) {
  return now.toDateString();
}

function remembered(key: string): string | null {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}

function remember(key: string, value: string) {
  try {
    localStorage.setItem(key, value);
  } catch {
    // Not remembered; at worst the pet says hello twice.
  }
}

async function addXp(xp: number) {
  if (xp <= 0) return;
  await useSettings.getState().update((s) => ({ ...s, pet: { ...s.pet, xp: s.pet.xp + xp } }));
}

/** Plays the pet's lines, and keeps its birthday. */
export function usePetTalk() {
  const lead = isLeadIsland();
  const ready = useSettings((s) => s.settings !== null);

  useEffect(() => {
    if (!lead || !ready) return;
    let lastSpokeAt = 0;
    const announcedMeetings = new Set<string>();
    let batteryWarned = false;
    let wasRaining: boolean | null = null;

    const pet = () => useSettings.getState().settings?.pet;

    // First launch: the pet is born today.
    const born = pet();
    if (born && !born.born) {
      const d = new Date();
      const p = (n: number) => String(n).padStart(2, "0");
      void useSettings.getState().update((s) => ({
        ...s,
        pet: { ...s.pet, born: `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}` },
      }));
    }

    const say = (event: LineEvent, vars: Record<string, string | number> = {}) => {
      const s = pet();
      const settings = useSettings.getState().settings;
      if (!s || !settings || !s.talk) return;
      const topic = TOPIC[event];
      if (topic && !s[topic]) return;
      const now = Date.now();
      const urgent = event === "meeting" || event === "agentWaiting";
      const focus = useFocus.getState();
      if (
        !urgent &&
        (inQuietHours(settings.behavior.quiet, new Date(now)) ||
          focus.gaming ||
          focusActive(focus.state, now))
      )
        return;
      if (!maySpeak(event, s.chattiness, lastSpokeAt, now)) return;
      lastSpokeAt = now;
      const name = s.name.trim() || "Pet";
      const text = lineFor(s.tone, event, { ...vars, name });
      usePetLive.setState({ line: { text, event, at: now } });
      if (SECTION_ONLY.includes(event)) return;
      // Out on the desktop, it says it in its own speech bubble (urgent ones still pop up here).
      if (s.buddyOut && !urgent) return;
      useActivities.getState().push({
        moduleId: "pet",
        priority: "low",
        title: name,
        body: text,
        avatar: "🐾",
        dedupKey: `pet-${event}`,
        countRepeats: false,
        timeoutMs: 7000,
      });
    };

    const onNews = ({ event, vars, xp }: PetNews) => {
      void addXp(xp ?? 0);
      say(event, vars);
    };
    const offNews = onPetNews(onNews);

    const offFocus = eventBus.on("FocusDone", () => onNews({ event: "focusDone", xp: 3 }));
    let doneTasks: number | null = null;
    const offTodo = eventBus.on("TodoChanged", ({ event }) => {
      const done = event.payload.tasks.filter((t) => t.done).length;
      if (doneTasks !== null && done > doneTasks) void addXp(done - doneTasks);
      doneTasks = done;
    });

    // Coding agents: a line in the Pet section when one finishes or needs you.
    const agentStates = new Map<string, string>();
    const offClaude = eventBus.on("ClaudeStatusChanged", ({ event }) => {
      for (const s of event.payload.live) {
        const before = agentStates.get(s.sessionId);
        const agent = s.agent === "codex" ? "Codex" : s.agent === "gemini" ? "Gemini CLI" : "Claude Code";
        if (s.state === "waiting" && before !== "waiting") say("agentWaiting", { agent });
        if (s.state === "idle" && before === "working") say("agentDone", { agent });
        agentStates.set(s.sessionId, s.state);
      }
    });

    // Weather: say something when rain starts, and keep the outfit current.
    const setWeather = (weather: Weather | null) => {
      usePetLive.setState({ weather });
      const raining = /rain|drizzle|storm/.test(weather?.icon ?? "");
      if (wasRaining === false && raining) say("rain");
      wasRaining = raining;
    };
    ipc.systemDashboard().then(
      (d) => setWeather(d.weather ?? null),
      () => {},
    );
    const offWeather = eventBus.on("WeatherChanged", ({ event }) => setWeather(event.payload));

    // Battery: once when it runs low, once when it's plugged back in.
    const offPower = useControl.subscribe((state) => {
      const p = state.snapshot?.power;
      if (!p) return;
      if (!p.plugged && p.percent <= LOW_BATTERY && !batteryWarned) {
        batteryWarned = true;
        say("batteryLow", { percent: p.percent });
      } else if (p.plugged && batteryWarned) {
        batteryWarned = false;
        say("plugged");
      }
    });

    const tick = async () => {
      const now = new Date();
      const idle = await ipc.systemIdleMs().catch(() => 0);
      const active = idle < 60_000;
      // Morning hello: the first time you're at the PC each day (after 5 am).
      if (active && now.getHours() >= 5 && remembered("pet-greeted") !== today(now)) {
        remember("pet-greeted", today(now));
        say("greeting");
      }
      // Late night: once per night between 1 and 5 am while you're still working.
      if (active && now.getHours() >= 1 && now.getHours() < 5 && remembered("pet-late") !== today(now)) {
        remember("pet-late", today(now));
        say("late");
      }
      // A meeting about to start.
      for (const e of useCalendar.getState().snapshot?.events ?? []) {
        if (e.allDay || announcedMeetings.has(e.id)) continue;
        const left = new Date(e.start).getTime() - now.getTime();
        if (left > 0 && left <= MEETING_LEAD_MS) {
          announcedMeetings.add(e.id);
          say("meeting", {
            minutes: Math.max(1, Math.round(left / 60_000)),
            title: e.title || "Your meeting",
          });
        }
      }
    };
    void tick();
    const id = window.setInterval(() => void tick(), CHECK_MS);

    return () => {
      window.clearInterval(id);
      offNews();
      offFocus();
      offTodo();
      offClaude();
      offWeather();
      offPower();
    };
  }, [lead, ready]);
}

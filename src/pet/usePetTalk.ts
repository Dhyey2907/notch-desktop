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
import { cheerPet } from "../focus/petCheer";
import { useMedia } from "../modules/media/mediaStore";
import { brandOf } from "../modules/media/mediaTheme";
import { digest, hungerOf, type Hunger } from "./petLife";
import { LOW_BATTERY, TIRED_AFTER_MIN } from "./petState";
import { onPetNews, usePetLive, type PetNews } from "./petStore";
import { playOnWalker } from "./useWalkLook";

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
  fetched: null,
  dropped: null,
  dizzy: null,
  pounce: null,
  friend: null,
  wake: "sayGreeting",
  snack: null,
  sulk: null,
  celebrate: null,
  longCoding: "sayBreaks",
  coffee: "sayBreaks",
  videoMarathon: "sayBreaks",
};
/** Notch already shows a card for these; the pet's line goes to its own section only. */
const SECTION_ONLY: LineEvent[] = ["meeting", "agentDone", "agentWaiting", "pat"];
const MEETING_LEAD_MS = 5 * 60_000;
const CHECK_MS = 30_000;
/** Away at least this long and it greets you with a stretch when you're back. */
const WAKE_AFTER_MS = 10 * 60_000;
/** A break this long (but not longer) is a coffee break. */
const COFFEE_MIN_MS = 10 * 60_000;
const COFFEE_MAX_MS = 25 * 60_000;
/** Hunger is saved this often while you're at the PC. */
const DIGEST_EVERY_MIN = 10;
const SNACK_ASK_GAP_MS = 60 * 60_000;
const VIDEO_MARATHON_MS = 90 * 60_000;

/** A cheer: on the taskbar if it's out, in the notch otherwise. */
function celebrate() {
  cheerPet();
  playOnWalker({ kind: "celebrate" });
}

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
      // Out on the taskbar it says it in its speech cloud there (urgent ones still pop up here).
      if (usePetLive.getState().walking && !urgent) return;
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

    const offFocus = eventBus.on("FocusDone", () => {
      celebrate();
      onNews({ event: "focusDone", xp: 3 });
    });
    let doneTasks: number | null = null;
    const offTodo = eventBus.on("TodoChanged", ({ event }) => {
      const done = event.payload.tasks.filter((t) => t.done).length;
      if (doneTasks !== null && done > doneTasks) {
        void addXp(done - doneTasks);
        celebrate();
        say("celebrate");
      }
      doneTasks = done;
    });
    // Inbox zero: unread mail just went down to none.
    let unread: number | null = null;
    const offMail = eventBus.on("MailChanged", ({ event }) => {
      const now = event.payload.status === "ready" ? event.payload.unread : null;
      if (now === 0 && unread !== null && unread > 0) {
        celebrate();
        say("celebrate");
      }
      if (now !== null) unread = now;
    });
    // A routine that ran without a hitch.
    const offRoutine = eventBus.on("RoutineRan", ({ event }) => {
      if (event.payload.failed === 0 && event.payload.done > 0) celebrate();
    });

    // Coding agents: a line in the Pet section when one finishes or needs you.
    const agentStates = new Map<string, string>();
    const offClaude = eventBus.on("ClaudeStatusChanged", ({ event }) => {
      for (const s of event.payload.live) {
        const before = agentStates.get(s.sessionId);
        const agent = s.agent === "codex" ? "Codex" : s.agent === "gemini" ? "Gemini CLI" : "Claude Code";
        if (s.state === "waiting" && before !== "waiting") say("agentWaiting", { agent });
        if (s.state === "idle" && before === "working") {
          celebrate();
          say("agentDone", { agent });
        }
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

    let lastIdle = 0;
    let activeMinutes = 0;
    let hunger: Hunger | null = null;
    let askedSnackAt = 0;
    let videoSince: number | null = null;
    let videoNoted = false;
    let codingNotedAt = 0;

    const tick = async () => {
      const now = new Date();
      const idle = await ipc.systemIdleMs().catch(() => 0);
      const active = idle < 60_000;
      const p = pet();

      // Back after a while: a stretch and a hello; a short break was a coffee break.
      if (active && lastIdle >= WAKE_AFTER_MS) {
        playOnWalker({ kind: "wake" });
        say(lastIdle >= COFFEE_MIN_MS && lastIdle <= COFFEE_MAX_MS ? "coffee" : "wake");
      }
      lastIdle = idle;

      // Hunger: a little while you're at the PC, saved every few minutes.
      if (p?.hunger && active) {
        activeMinutes += CHECK_MS / 60_000;
        if (activeMinutes >= DIGEST_EVERY_MIN) {
          const minutes = activeMinutes;
          activeMinutes = 0;
          void useSettings
            .getState()
            .update((s) => ({ ...s, pet: { ...s.pet, fullness: digest(s.pet.fullness, minutes) } }));
        }
      }
      if (p?.hunger) {
        const h = hungerOf(p.fullness);
        if (h !== hunger) {
          if (h === "sulky" && hunger !== null) say("sulk");
          else if (h === "peckish" && hunger !== "sulky" && now.getTime() - askedSnackAt > SNACK_ASK_GAP_MS) {
            askedSnackAt = now.getTime();
            say("snack");
          }
          hunger = h;
        }
      }

      // A long video session.
      const media = useMedia.getState().media;
      const watching = media?.status === "playing" && !media.mock && !!brandOf(media)?.video;
      if (watching) {
        videoSince ??= now.getTime();
        if (!videoNoted && now.getTime() - videoSince >= VIDEO_MARATHON_MS) {
          videoNoted = true;
          say("videoMarathon");
        }
      } else {
        videoSince = null;
        videoNoted = false;
      }

      // A long day of coding: once when it passes three hours, then every two hours.
      if (active && now.getTime() - codingNotedAt > 2 * 3600_000) {
        const stats = await ipc.codingStats().catch(() => null);
        const minutes = stats?.today.reduce((sum, t) => sum + t.minutes, 0) ?? 0;
        if (minutes >= TIRED_AFTER_MIN) {
          codingNotedAt = now.getTime();
          say("longCoding", { hours: Math.floor(minutes / 60) });
        }
      }
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
      offMail();
      offRoutine();
      offClaude();
      offWeather();
      offPower();
    };
  }, [lead, ready]);
}

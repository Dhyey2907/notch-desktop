// The command palette's commands: open any module, start focus or a timer, change the volume,
// switch wallpaper, write a note, call the pet, and more. Typing filters them; some take what
// you type after their name ("timer 10", "note buy milk", "volume 40").
import { ipc } from "../../core/ipc";
import { useSettings } from "../../settings";
import { useModuleNav } from "../../shell/useModules";
import { MODULES } from "../registry";
import type { InstalledApp } from "../../core/generated/InstalledApp";
import type { PowerAction } from "../../core/generated/PowerAction";

export interface Command {
  id: string;
  title: string;
  /** Extra words that should find it. */
  keywords?: string;
  /** Shown under the title. */
  hint?: string;
  /** Runs it; `arg` is what you typed after the command's first word. Returns a short result. */
  run: (arg: string) => Promise<string | void> | string | void;
  /** It needs an argument (shown as a placeholder hint). */
  takes?: string;
  /** Keep the palette open afterwards (it opened something in the notch). */
  stays?: boolean;
  /** Ask first: the first Enter shows this, a second Enter runs it. */
  confirm?: string;
}

function minutesFrom(arg: string, fallback: number): number {
  const n = Number.parseInt(arg, 10);
  return Number.isFinite(n) && n > 0 && n <= 600 ? n : fallback;
}

async function nextWallpaper(): Promise<string> {
  const state = await ipc.wallpaperState();
  const pool = state.saved.length > 0 ? state.saved : state.folder;
  if (pool.length === 0) return "No wallpapers saved yet";
  const target = state.screens.some((s) => s.id === state.target) ? state.target : "all";
  const current = state.screens[0]?.current?.toLowerCase();
  const i = pool.findIndex((w) => w.path.toLowerCase() === current);
  const next = pool[(i + 1) % pool.length];
  await ipc.wallpaperApply(next.path, target);
  return `Wallpaper: ${next.name}`;
}

const POWER: [PowerAction, string, string][] = [
  ["sleep", "Sleep", "suspend standby"],
  ["hibernate", "Hibernate", "suspend"],
  ["restart", "Restart", "reboot"],
  ["restartBios", "Restart to BIOS", "uefi firmware reboot"],
  ["shutdown", "Shut down", "power off turn off"],
  ["signOut", "Sign out", "log off logout"],
];

export function commands(): Command[] {
  const open: Command[] = MODULES.filter((m) => m.id !== "palette").map((m) => ({
    id: `open-${m.id}`,
    title: `Open ${m.name}`,
    keywords: `module ${m.id}`,
    stays: true,
    run: () => useModuleNav.getState().setActive(m.id),
  }));
  return [
    {
      id: "focus",
      title: "Start focus",
      keywords: "pomodoro concentrate deep work",
      takes: "minutes",
      hint: "focus 25",
      run: async (arg) => {
        const minutes = minutesFrom(arg, 25);
        await ipc.focusStart(minutes);
        return `Focusing for ${minutes} min`;
      },
    },
    { id: "focus-stop", title: "Stop focus", run: async () => void (await ipc.focusStop()) },
    {
      id: "timer",
      title: "Start a timer",
      keywords: "countdown alarm",
      takes: "minutes",
      hint: "timer 10",
      run: async (arg) => {
        const minutes = minutesFrom(arg, 10);
        await ipc.todoAddTask(`Timer · ${minutes} min`, minutes, true);
        return `Timer set for ${minutes} min`;
      },
    },
    {
      id: "note",
      title: "New note",
      keywords: "write jot memo",
      takes: "text",
      hint: "note buy milk",
      run: async (arg) => {
        if (!arg.trim()) return "Type the note after “note”";
        await ipc.noteAdd(arg.trim());
        return "Saved to notes";
      },
    },
    {
      id: "task",
      title: "New task",
      keywords: "todo to-do",
      takes: "text",
      hint: "task call the bank",
      run: async (arg) => {
        if (!arg.trim()) return "Type the task after “task”";
        await ipc.todoAddTask(arg.trim(), null, false);
        return "Added to To-do";
      },
    },
    {
      id: "mute",
      title: "Mute / unmute",
      keywords: "sound audio silence",
      run: async () => void (await ipc.soundToggleMute()),
    },
    {
      id: "volume",
      title: "Set volume",
      keywords: "sound audio loud",
      takes: "percent",
      hint: "volume 40",
      run: async (arg) => {
        const n = Math.max(0, Math.min(100, Number.parseInt(arg, 10)));
        if (!Number.isFinite(n)) return "Type a number, like “volume 40”";
        await ipc.soundSetVolume(n / 100);
        return `Volume ${n}%`;
      },
    },
    { id: "wallpaper", title: "Next wallpaper", keywords: "background desktop change", run: nextWallpaper },
    {
      id: "pet-out",
      title: "Call the pet out",
      keywords: "pet walk taskbar",
      run: async () => void (await ipc.petBuddyShow()),
    },
    {
      id: "pet-back",
      title: "Send the pet back",
      keywords: "pet home",
      run: async () => void (await ipc.petBuddyHide()),
    },
    {
      id: "clipboard-pause",
      title: "Pause / resume clipboard history",
      keywords: "clipboard private",
      run: async () => {
        let paused = false;
        await useSettings.getState().update((s) => {
          paused = !s.clipboard.paused;
          return { ...s, clipboard: { ...s.clipboard, paused } };
        });
        return paused ? "Clipboard history paused" : "Clipboard history on";
      },
    },
    {
      id: "grab",
      title: "Grab text from the screen",
      keywords: "ocr copy text",
      run: async () => void (await ipc.grabBegin()),
    },
    {
      id: "lock",
      title: "Lock the PC",
      keywords: "lock screen away",
      run: async () => void (await ipc.systemLock()),
    },
    ...POWER.map(([action, title, keywords]): Command => ({
      id: `power-${action}`,
      title,
      keywords: `power ${keywords}`,
      confirm: `Press Enter again to ${title.toLowerCase()}`,
      run: async () => void (await ipc.systemPower(action)),
    })),
    {
      id: "pip",
      title: "Picture-in-picture",
      keywords: "pip video float pop out mini player",
      run: async () => {
        const [windows, media] = await Promise.all([
          ipc.deskWindows(),
          ipc.mediaGetState().catch(() => null),
        ]);
        const app = media?.sourceApp.toLowerCase() ?? "";
        const pick =
          windows.find(
            (w) => app && (app.includes(w.app.toLowerCase()) || w.app.toLowerCase().includes(app)),
          ) ??
          windows.find((w) => /chrome|edge|firefox|brave|opera|vlc|media player|netflix|prime/i.test(w.app));
        if (!pick) return "Nothing playing to pop out. Use Desk → Windows for any window.";
        await ipc.pipOpen(pick.id);
        return `Picture-in-picture: ${pick.app}`;
      },
    },
    {
      id: "price-watch",
      title: "Price watch",
      keywords: "price drop deal amazon flipkart track shop",
      stays: true,
      run: () => useModuleNav.getState().setActive("prices"),
    },
    {
      id: "meeting",
      title: "Meeting notes",
      keywords: "record meeting transcribe minutes call notes start stop",
      stays: true,
      run: () => useModuleNav.getState().setActive("meeting"),
    },
    {
      id: "research",
      title: "Research a topic",
      keywords: "ai web search look up brief",
      takes: "topic",
      hint: "research best budget monitors",
      run: async (arg) => {
        if (!arg.trim()) return "Type the topic after “research”";
        await ipc.researchStart(arg.trim());
        return "Researching — a card will say when it's ready";
      },
    },
    {
      id: "explain",
      title: "Explain my screen",
      keywords: "ai what is this error help screenshot",
      stays: true,
      run: async () => {
        await ipc.askExplainScreen();
        useModuleNav.getState().setActive("ask");
      },
    },
    {
      id: "record-screen",
      title: "Record the screen",
      keywords: "screen recording video capture mp4 stop",
      run: async () => {
        const state = await ipc.screenrecState();
        if (state.startedAt != null) {
          const path = await ipc.screenrecStop();
          return path ? "Saved to the shelf" : "Stopped";
        }
        await ipc.screenrecStart(false);
        return "Recording this screen. Run it again to stop.";
      },
    },
    {
      id: "record-gif",
      title: "Record a GIF",
      keywords: "screen recording gif animation capture",
      run: async () => {
        const state = await ipc.screenrecState();
        if (state.startedAt != null) {
          await ipc.screenrecStop();
          return "Saved to the shelf";
        }
        await ipc.screenrecStart(true);
        return "Recording a GIF (up to 30 s). Run it again to stop.";
      },
    },
    {
      id: "blur",
      title: "Privacy blur",
      keywords: "hide screen cover privacy panic",
      run: async () => void (await ipc.blurToggle()),
    },
    {
      id: "claude-task",
      title: "Claude Code task",
      keywords: "claude agent code start new task",
      takes: "task",
      hint: "claude fix the failing tests",
      run: async (arg) => {
        const [project] = await ipc.claudeProjects();
        if (!project) return "Claude Code hasn't worked in any folder yet";
        await ipc.claudeLaunch(project, arg.trim());
        return `Claude Code started in ${project.split(/[\\/]/).pop()}`;
      },
    },
    {
      id: "briefing",
      title: "Morning briefing",
      keywords: "today summary morning weather meetings",
      run: async () => {
        const { showBriefing } = await import("../day/useMorningBriefing");
        showBriefing();
      },
    },
    {
      id: "review",
      title: "Review my day",
      keywords: "summary today stats evening",
      run: async () => {
        const { showReview } = await import("../day/useEveningReview");
        await showReview();
      },
    },
    {
      id: "output",
      title: "Switch sound output",
      keywords: "speakers headphones audio device",
      run: async () => {
        const mixer = await ipc.audioMixer();
        if (mixer.devices.length < 2) return "Only one output device";
        const i = mixer.devices.findIndex((d) => d.default);
        const next = mixer.devices[(i + 1) % mixer.devices.length];
        await ipc.audioSetOutput(next.id);
        return `Sound on ${next.name}`;
      },
    },
    {
      id: "calendar-sync",
      title: "Sync calendar",
      keywords: "refresh google events",
      run: async () => {
        await ipc.calendarSyncNow();
        return "Calendar synced";
      },
    },
    {
      id: "settings",
      title: "Open settings",
      keywords: "preferences options",
      run: async () => void (await ipc.openSettingsWindow()),
    },
    ...open,
  ];
}

/** The launcher: one "Open …" per installed app, plus "close <app>". */
export function appCommands(apps: InstalledApp[]): Command[] {
  return [
    {
      id: "close-app",
      title: "Close an app",
      keywords: "quit exit kill end task",
      takes: "app",
      hint: "close chrome",
      confirm: "Press Enter again to close it (unsaved work may prompt)",
      run: async (arg) => {
        const name = arg.trim();
        if (!name) return "Type the app after “close”, like “close chrome”";
        const n = await ipc.appClose(name.replace(/\s+/g, ""));
        return n > 0 ? `Asked ${n} window${n === 1 ? "" : "s"} to close` : `No open windows for “${name}”`;
      },
    },
    ...apps.map((a): Command => ({
      id: `app-${a.id}`,
      title: `Open ${a.name}`,
      keywords: `app launch start ${a.name}`,
      run: async () => {
        await ipc.appLaunch(a.id);
        return `Opening ${a.name}`;
      },
    })),
  ];
}

/** How well `query` matches `text`: higher is better, 0 = no match (letters in order). */
export function score(query: string, text: string): number {
  const q = query.trim().toLowerCase();
  if (!q) return 1;
  const t = text.toLowerCase();
  if (t.startsWith(q)) return 100 - t.length / 100;
  const word = t.split(/[\s/·-]+/).findIndex((w) => w.startsWith(q));
  if (word >= 0) return 80 - word;
  if (t.includes(q)) return 60;
  // Letters in order, with a bonus for runs.
  let at = 0;
  let run = 0;
  let total = 0;
  for (const ch of q) {
    const i = t.indexOf(ch, at);
    if (i < 0) return 0;
    run = i === at ? run + 1 : 0;
    total += 1 + run;
    at = i + 1;
  }
  return Math.min(50, total);
}

/** Split "timer 10" into the command word(s) to match and the argument. */
export function splitInput(input: string): { word: string; arg: string } {
  const t = input.trim();
  const space = t.indexOf(" ");
  return space < 0 ? { word: t, arg: "" } : { word: t.slice(0, space), arg: t.slice(space + 1) };
}

/** The commands for this input, best first. A command that takes an argument matches on its first word. */
export function search(list: Command[], input: string): { command: Command; arg: string }[] {
  const { word, arg } = splitInput(input);
  const scored = list.map((command) => {
    const first = command.hint?.split(" ")[0] ?? command.id;
    const haystack = `${command.title} ${command.keywords ?? ""}`;
    if (command.takes && arg && first.toLowerCase() === word.toLowerCase()) return { command, arg, s: 200 };
    const onTitle = score(input, command.title);
    const s = Math.max(score(input, haystack), onTitle > 0 ? onTitle + 5 : 0);
    return { command, arg: command.takes && first.toLowerCase() === word.toLowerCase() ? arg : "", s };
  });
  return scored
    .filter((x) => x.s > 0)
    .sort((a, b) => b.s - a.s)
    .map(({ command, arg: a }) => ({ command, arg: a }));
}

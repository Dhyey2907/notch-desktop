// The widgets that can be pinned beside the resting bubble (MiniWidget.tsx draws them).

export const MINI_WIDGETS = [
  { id: "timer", label: "Timer countdown" },
  { id: "media", label: "What's playing" },
  { id: "meeting", label: "Next meeting" },
  { id: "mail", label: "Unread mail" },
  { id: "claude", label: "Coding agents" },
  { id: "focus", label: "Focus time left" },
  { id: "sysmon", label: "CPU & memory" },
] as const;

export type MiniWidgetId = (typeof MINI_WIDGETS)[number]["id"];

/** "4:05" or "1:02:05". */
export function clock(ms: number): string {
  const total = Math.max(0, Math.round(ms / 1000));
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = total % 60;
  const mm = h > 0 ? String(m).padStart(2, "0") : String(m);
  return `${h > 0 ? `${h}:` : ""}${mm}:${String(s).padStart(2, "0")}`;
}

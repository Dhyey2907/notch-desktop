// Settings pages and a search index over every setting, so "accent", "sound" or "shortcut"
// finds the right row. Labels match the rows' visible labels (they are also the anchors).

export type PageId =
  | "general"
  | "look"
  | "behavior"
  | "shortcuts"
  | "home"
  | "dock"
  | "media"
  | "calendar"
  | "git"
  | "clipboard"
  | "focus"
  | "games"
  | "ai"
  | "privacy"
  | "about";

export interface PageInfo {
  id: PageId;
  label: string;
  group: "Notch" | "Features" | "More";
  /** One line under the page title. */
  lead: string;
}

export const PAGES: PageInfo[] = [
  {
    id: "general",
    label: "General",
    group: "Notch",
    lead: "Starting up, which screens show the notch, and motion.",
  },
  { id: "look", label: "Look", group: "Notch", lead: "Size, colors, shape and where the notch sits." },
  {
    id: "behavior",
    label: "Behavior",
    group: "Notch",
    lead: "How the notch opens, how long notifications stay, and quiet times.",
  },
  {
    id: "shortcuts",
    label: "Shortcuts & gestures",
    group: "Notch",
    lead: "Keyboard shortcuts that work in any app, and touchpad swipes on the notch.",
  },
  { id: "home", label: "Home", group: "Features", lead: "What the Home dashboard shows." },
  {
    id: "dock",
    label: "Dock & modules",
    group: "Features",
    lead: "Turn modules on or off, reorder the dock, and pick where the notch opens.",
  },
  {
    id: "media",
    label: "Media",
    group: "Features",
    lead: "Music controls, Spotify, lyrics and the music glow.",
  },
  { id: "calendar", label: "Calendar", group: "Features", lead: "Google Calendar and meeting reminders." },
  { id: "git", label: "Git", group: "Features", lead: "Repositories and GitHub alerts." },
  {
    id: "clipboard",
    label: "Clipboard & files",
    group: "Features",
    lead: "Clipboard history, smart suggestions and the file shelf.",
  },
  { id: "focus", label: "Focus", group: "Features", lead: "Focus sessions, the pet and ambient sounds." },
  { id: "games", label: "Games", group: "Features", lead: "The game mode strip while you play." },
  { id: "ai", label: "AI", group: "Features", lead: "Ask Claude from the notch." },
  {
    id: "privacy",
    label: "Privacy & data",
    group: "More",
    lead: "What Notch keeps, and moving your settings to another PC.",
  },
  { id: "about", label: "About", group: "More", lead: "Version and where Notch keeps its files." },
];

/** Old deep links (`#settings/appearance`) still land somewhere sensible. */
const LEGACY: Record<string, PageId> = { appearance: "look", modules: "dock", diagnostics: "about" };

export function pageFromId(id: string | undefined): PageId {
  const mapped = id && (LEGACY[id] ?? id);
  return PAGES.find((p) => p.id === mapped)?.id ?? "general";
}

interface Entry {
  page: PageId;
  label: string;
  /** Other words people might search for. */
  keywords?: string;
}

export const ENTRIES: Entry[] = [
  { page: "general", label: "Start with Windows", keywords: "startup login boot launch" },
  { page: "general", label: "Show the notch on", keywords: "screen monitor display second" },
  { page: "general", label: "Animation", keywords: "motion reduce effects" },
  { page: "look", label: "Notch size", keywords: "bigger smaller scale zoom" },
  { page: "look", label: "Text size", keywords: "font bigger smaller readable" },
  { page: "look", label: "Resting bubble", keywords: "idle pill dot hidden collapsed" },
  { page: "look", label: "Clock in the bubble", keywords: "time" },
  { page: "look", label: "Theme", keywords: "black graphite glass dark color background" },
  { page: "look", label: "Opacity", keywords: "transparent see-through translucent" },
  { page: "look", label: "Corner roundness", keywords: "radius rounded square shape" },
  { page: "look", label: "Accent color", keywords: "highlight colour tint" },
  { page: "look", label: "Match the music", keywords: "album art accent color" },
  { page: "look", label: "Move left or right", keywords: "position offset place" },
  { page: "look", label: "Distance from the top", keywords: "position offset lower" },
  { page: "behavior", label: "Preview on hover", keywords: "peek mouse pointer" },
  { page: "behavior", label: "Hover speed", keywords: "delay fast slow" },
  { page: "behavior", label: "Open on hover", keywords: "click expand mouse" },
  { page: "behavior", label: "Close by itself", keywords: "auto close collapse timeout" },
  { page: "behavior", label: "Notifications stay for", keywords: "time duration long short cards" },
  { page: "behavior", label: "Quiet hours", keywords: "do not disturb dnd night silence schedule" },
  { page: "behavior", label: "Notification sound", keywords: "chime alert audio volume" },
  { page: "behavior", label: "Over fullscreen apps", keywords: "games videos hide fullscreen" },
  { page: "behavior", label: "Show volume in the notch", keywords: "hud sound" },
  { page: "behavior", label: "Show brightness in the notch", keywords: "hud screen" },
  { page: "behavior", label: "Show Bluetooth devices connecting", keywords: "earbuds headphones battery" },
  { page: "behavior", label: "Show Caps Lock and Num Lock", keywords: "keyboard capital letters" },
  { page: "behavior", label: "Show keyboard language changes", keywords: "input layout win space" },
  { page: "behavior", label: "Warn when an accessory is low", keywords: "battery earbuds mouse controller" },
  { page: "behavior", label: "Internet down and back", keywords: "offline connection wifi network" },
  { page: "behavior", label: "Show charging", keywords: "battery plugged charger" },
  { page: "behavior", label: "Remind me to unplug", keywords: "battery health 80 charge limit" },
  { page: "shortcuts", label: "Open or close the notch", keywords: "keyboard shortcut hotkey ctrl alt n" },
  { page: "shortcuts", label: "Ask Claude", keywords: "keyboard shortcut hotkey ai ctrl alt a" },
  { page: "shortcuts", label: "Clipboard history", keywords: "keyboard shortcut hotkey ctrl alt v paste" },
  { page: "shortcuts", label: "Touchpad swipes", keywords: "gestures scroll flick" },
  { page: "shortcuts", label: "Swipe sensitivity", keywords: "gestures touchpad" },
  { page: "shortcuts", label: "Swipe to change songs", keywords: "next previous track gestures" },
  { page: "home", label: "Tiles", keywords: "dashboard cpu ram battery weather screen time desktops" },
  { page: "home", label: "Temperature", keywords: "celsius fahrenheit weather units" },
  { page: "home", label: "Clock", keywords: "24 hour 12 hour am pm time format" },
  { page: "dock", label: "Open the notch to", keywords: "first module default start" },
  { page: "dock", label: "Modules", keywords: "reorder order dock hide enable disable interrupt" },
  { page: "media", label: "React to music", keywords: "glow beat pulse" },
  { page: "media", label: "Show lyrics", keywords: "lrclib words" },
  { page: "media", label: "Spotify", keywords: "playlists account connect" },
  { page: "calendar", label: "Google Calendar", keywords: "meetings events reminders connect" },
  { page: "git", label: "Repositories", keywords: "github pull requests ci reviews" },
  { page: "clipboard", label: "Clipboard history", keywords: "copy paste pause blocked apps" },
  { page: "clipboard", label: "Suggestions", keywords: "remind me maps dates addresses" },
  { page: "focus", label: "Pet", keywords: "character bubble mood" },
  { page: "focus", label: "Ambient volume", keywords: "rain ocean sounds noise" },
  { page: "games", label: "Game mode", keywords: "fps gpu ping hud" },
  { page: "ai", label: "Claude", keywords: "ask ai assistant" },
  { page: "privacy", label: "Export settings", keywords: "backup save move file" },
  { page: "privacy", label: "Import settings", keywords: "restore load file" },
  { page: "privacy", label: "Logs", keywords: "diagnostics troubleshooting export" },
  { page: "about", label: "Version", keywords: "about update folders data" },
];

const normalize = (s: string) => s.toLowerCase().normalize("NFKD");

/** Settings matching every word of the query, best (label) matches first. */
export function searchSettings(query: string): (Entry & { pageLabel: string })[] {
  const words = normalize(query).split(/\s+/).filter(Boolean);
  if (words.length === 0) return [];
  return ENTRIES.map((entry) => {
    const page = PAGES.find((p) => p.id === entry.page)!;
    const label = normalize(entry.label);
    const haystack = `${label} ${normalize(entry.keywords ?? "")} ${normalize(page.label)}`;
    if (!words.every((w) => haystack.includes(w))) return null;
    const score = words.filter((w) => label.includes(w)).length;
    return { entry: { ...entry, pageLabel: page.label }, score };
  })
    .filter((r) => r !== null)
    .sort((a, b) => b.score - a.score)
    .map((r) => r.entry);
}

/** The DOM id of a setting's row, so search can scroll to it. */
export function settingAnchor(label: string): string {
  return `setting-${label.toLowerCase().replace(/[^a-z0-9]+/g, "-")}`;
}

export function pageLead(id: PageId): string {
  return PAGES.find((p) => p.id === id)?.lead ?? "";
}

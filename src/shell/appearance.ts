// How the notch looks (Settings → Look): size, text size, theme, opacity, corners, accent and
// the resting bubble. Applied as CSS variables, plus `zoom` for the notch size, so every
// component follows without knowing about these settings.
import type { AppearanceSettings } from "../core/generated/AppearanceSettings";
import type { Scale } from "../core/generated/Scale";
import type { Theme } from "../core/generated/Theme";
import type { Size } from "./geometry";

export const SIZE_SCALE: Record<Scale, number> = { small: 0.88, normal: 1, large: 1.15 };
export const TEXT_SCALE: Record<Scale, number> = { small: 0.92, normal: 1, large: 1.12 };

const THEME_RGB: Record<Theme, [number, number, number]> = {
  black: [9, 9, 9],
  graphite: [36, 36, 40],
  glass: [22, 22, 28],
};
/** Glass is always see-through; this is its most opaque. */
const GLASS_MAX_OPACITY = 75;

/** Text-size tokens from tokens.css (px). */
const TEXT_TOKENS: Record<string, number> = {
  "--text-xs": 11,
  "--text-sm": 12,
  "--text-md": 13,
  "--text-lg": 15,
  "--text-xl": 18,
};

export function islandBackground(a: Pick<AppearanceSettings, "theme" | "opacity">): string {
  const [r, g, b] = THEME_RGB[a.theme];
  const opacity = a.theme === "glass" ? Math.min(a.opacity, GLASS_MAX_OPACITY) : a.opacity;
  return `rgba(${r}, ${g}, ${b}, ${opacity / 100})`;
}

/** Text that reads on top of `hex` (for primary buttons and the like). */
export function readableOn(hex: string): string {
  const n = parseInt(hex.slice(1), 16);
  const [r, g, b] = [(n >> 16) & 255, (n >> 8) & 255, n & 255];
  const luminance = (0.2126 * r + 0.7152 * g + 0.0722 * b) / 255;
  return luminance > 0.55 ? "#0a0a0a" : "#ffffff";
}

/** CSS variables for a look. An empty accent keeps the stylesheet's white. */
export function appearanceVars(a: AppearanceSettings): Record<string, string> {
  const text = TEXT_SCALE[a.textSize];
  const vars: Record<string, string> = {
    "--island-bg": islandBackground(a),
    "--island-roundness": String(a.roundness / 100),
  };
  for (const [name, px] of Object.entries(TEXT_TOKENS)) vars[name] = `${Math.round(px * text * 10) / 10}px`;
  if (a.accent) {
    vars["--active"] = a.accent;
    vars["--on-active"] = readableOn(a.accent);
  }
  return vars;
}

/** Set (or clear) a look's variables on an element. */
export function applyAppearance(a: AppearanceSettings, el: HTMLElement) {
  const vars = appearanceVars(a);
  for (const name of ["--active", "--on-active"]) if (!(name in vars)) el.style.removeProperty(name);
  for (const [name, value] of Object.entries(vars)) el.style.setProperty(name, value);
  el.toggleAttribute("data-accent-music", a.accentFromMusic);
}

/** Corner radius after the roundness setting (never fully square). */
export function scaledRadius(radius: number, roundness: number): number {
  return Math.max(4, (radius * roundness) / 100);
}

/** The resting bubble's size for a look. */
export function restingSize(a: Pick<AppearanceSettings, "bubble" | "bubbleClock">): Size {
  if (a.bubble === "dot") return { width: 22, height: 12 };
  if (a.bubbleClock && a.bubble === "pill") return { width: 104, height: 18 };
  return { width: 64, height: 14 };
}

let scale = 1;

/** The notch's size multiplier (window sizes are multiplied by it). */
export function uiScale(): number {
  return scale;
}

export function setUiScale(next: number) {
  scale = next;
}

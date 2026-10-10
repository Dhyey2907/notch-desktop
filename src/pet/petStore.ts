// Live pet state shared by the bubble, the Pet section and the talking: when it was last patted,
// the weather it dresses for, and its latest line. Plus a tiny channel other parts of Notch use to
// tell the pet something happened ("a break was taken").
import { create } from "zustand";

import type { Weather } from "../core/generated/Weather";
import type { LineEvent } from "./lines";

export const LOVE_MS = 2500;

export interface PetLine {
  text: string;
  event: LineEvent;
  at: number;
  /** How long it stays in the pet's cloud on the taskbar (a reminder stays longer than a chat line). */
  holdMs?: number;
}

/** A reminder stays in the cloud this long. */
export const REMINDER_CLOUD_MS = 20_000;

export const usePetLive = create<{
  lovedUntil: number;
  weather: Weather | null;
  line: PetLine | null;
  /** Out walking the taskbar right now. */
  walking: boolean;
}>(() => ({ lovedUntil: 0, weather: null, line: null, walking: false }));

/**
 * There is one pet. It's out of the notch while it walks the taskbar, and from the moment you
 * call it out (so it doesn't linger in the notch while it gets going).
 */
export function petOut(calledOut: boolean, walking: boolean): boolean {
  return calledOut || walking;
}

export function lovePet(now = Date.now()) {
  usePetLive.setState({ lovedUntil: now + LOVE_MS });
}

export interface PetNews {
  event: LineEvent;
  vars?: Record<string, string | number>;
  /** Experience it earns the pet. */
  xp?: number;
}

const listeners = new Set<(news: PetNews) => void>();

/** Tell the pet something happened; the talking decides whether it says anything. */
export function tellPet(news: PetNews) {
  for (const l of listeners) l(news);
}

export function onPetNews(listener: (news: PetNews) => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

/**
 * While the pet is out on the taskbar, it shows this in its cloud (a reminder that just came due,
 * for instance). Does nothing while it's home in the notch. Returns whether it was shown.
 */
export function showInPetCloud(text: string, holdMs = REMINDER_CLOUD_MS, now = Date.now()): boolean {
  if (!usePetLive.getState().walking) return false;
  usePetLive.setState({ line: { text, event: "chat", at: now, holdMs } });
  return true;
}

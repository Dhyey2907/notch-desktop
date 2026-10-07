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
}

export const usePetLive = create<{ lovedUntil: number; weather: Weather | null; line: PetLine | null }>(
  () => ({ lovedUntil: 0, weather: null, line: null }),
);

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

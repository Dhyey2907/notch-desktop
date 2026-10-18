// The walking pet shows what the pet is doing (a laptop while you code, popcorn during a show,
// headphones with music) and speaks through a cloud above its head. The lead island works out
// both and sends them to the pet's window.
import { emitTo, listen } from "@tauri-apps/api/event";
import { useEffect, useRef } from "react";

import { isLeadIsland } from "../core/window";
import { useSettings } from "../settings";
import { inQuietHours } from "../shell/behavior";
import { lineFor, maySpeak, type LineEvent } from "./lines";
import { COLLAR_STREAK, currentStreak, FETCH_FILLS, feed, fetched, hungerOf, played } from "./petLife";
import type { Face, Outfit, Prop } from "./petState";
import { lovePet, usePetLive } from "./petStore";
import { usePetLook } from "./usePetLook";

export const WALK_WINDOW = "petwalk";
/** Island → walking pet: how it looks and its latest line. */
export const WALK_LOOK_EVENT = "pet-walk-look";
/** Walking pet → island: its window just opened and wants the current state. */
export const WALK_HELLO_EVENT = "pet-walk-hello";
/** Walking pet → island: you clicked it (a treat). */
export const WALK_TREAT_EVENT = "pet-walk-treat";
/** Island → walking pet: play fetch, cheer, wake up, or do a trick. */
export const WALK_PLAY_EVENT = "pet-walk-play";
/** Walking pet → island: something happened while you played with it. */
export const WALK_DID_EVENT = "pet-walk-did";

export interface WalkPlay {
  kind: "fetch" | "celebrate" | "wake" | "trick";
  trick?: "backflip" | "juggle";
}

export interface WalkDid {
  what: "pat" | "fetched" | "dropped" | "dizzy" | "pounce" | "friend";
}

/** Ask the walking pet to play (it ignores this while it isn't out). */
export function playOnWalker(play: WalkPlay) {
  void emitTo(WALK_WINDOW, WALK_PLAY_EVENT, play).catch(() => {});
}

export interface WalkLook {
  face: Face;
  prop: Prop;
  outfit?: Outfit;
  /** A hat it unlocked ("" = none). */
  hat?: string;
  /** A week of play: a collar tag. */
  collar?: boolean;
  /** Too hungry to walk properly. */
  sulky?: boolean;
  /** What it's saying (shown in the cloud for `holdMs`, or a few seconds, after `at`). */
  line: { text: string; at: number; holdMs?: number } | null;
}

const HELLO_AFTER_MS = 900;
const SAVE_TREATS_AFTER_MS = 1500;

/** Milliseconds between its timed messages while you keep it out; 0 = none. */
export function chatEveryMs(minutes: number | undefined): number {
  return minutes && minutes > 0 ? Math.round(minutes) * 60_000 : 0;
}

/** Call in the island: keeps the walking pet's look and speech current. */
export function useWalkLook() {
  const lead = isLeadIsland();
  const look = usePetLook();
  const walking = usePetLive((s) => s.walking);
  const line = usePetLive((s) => s.line);
  const talk = useSettings((s) => s.settings?.pet.talk ?? true);
  const calledOut = useSettings((s) => s.settings?.pet.buddyOut ?? false);
  const chatMs = chatEveryMs(useSettings((s) => s.settings?.pet.chatMinutes));
  const latest = useRef<WalkLook | null>(null);
  const lastSaid = useRef(0);
  const lastProp = useRef<Prop | null>(null);
  const face = look?.face;
  const prop = look?.prop;
  const outfit = look?.outfit;
  const hat = useSettings((s) => s.settings?.pet.hat ?? "");
  const collar = useSettings((s) => {
    const p = s.settings?.pet;
    return !!p && currentStreak(p.streakDays, p.lastPlayDay, new Date()) >= COLLAR_STREAK;
  });
  const sulky = useSettings((s) => !!s.settings?.pet.hunger && hungerOf(s.settings.pet.fullness) === "sulky");

  /** `timed`: the message you asked for every few minutes, which doesn't wait for a quiet spell. */
  const say = (event: LineEvent, timed = false) => {
    const s = useSettings.getState().settings;
    if (!s?.pet.talk) return;
    const now = Date.now();
    if (event !== "hello" && event !== "treat" && inQuietHours(s.behavior.quiet, new Date(now))) return;
    if (!timed && !maySpeak(event, s.pet.chattiness, lastSaid.current, now)) return;
    lastSaid.current = now;
    const text = lineFor(s.pet.tone, event, { name: s.pet.name.trim() || "Pet" });
    usePetLive.setState({ line: { text, event, at: now } });
  };

  // Send how it looks and what it says whenever that changes while it's out.
  const lineText = talk ? (line?.text ?? null) : null;
  const lineAt = talk ? (line?.at ?? 0) : 0;
  const lineHold = line?.holdMs;
  useEffect(() => {
    if (!lead || !face || !prop) return;
    latest.current = {
      face,
      prop,
      outfit,
      hat,
      collar,
      sulky,
      line: lineText ? { text: lineText, at: lineAt, holdMs: lineHold } : null,
    };
    if (walking) void emitTo(WALK_WINDOW, WALK_LOOK_EVENT, latest.current).catch(() => {});
  }, [lead, face, prop, outfit, hat, collar, sulky, walking, lineText, lineAt, lineHold]);

  // Treats: you clicked the pet on the taskbar.
  useEffect(() => {
    if (!lead) return;
    let unsaved = 0;
    let saveTimer: number | undefined;
    const off = listen(WALK_TREAT_EVENT, () => {
      lovePet();
      say("treat");
      unsaved += 1;
      window.clearTimeout(saveTimer);
      saveTimer = window.setTimeout(() => {
        const add = unsaved;
        unsaved = 0;
        void useSettings.getState().update((s) => ({
          ...s,
          pet: {
            ...s.pet,
            treats: s.pet.treats + add,
            xp: s.pet.xp + add,
            fullness: feed(s.pet.fullness, add * 12),
            ...played(s.pet.streakDays, s.pet.lastPlayDay, new Date()),
          },
        }));
      }, SAVE_TREATS_AFTER_MS);
    });
    return () => {
      window.clearTimeout(saveTimer);
      void off.then((f) => f());
    };
  }, [lead]);

  // Playing with it on the taskbar: petting, fetch, a drop, a dizzy spell, a pounce, a friend.
  useEffect(() => {
    if (!lead) return;
    const off = listen<WalkDid>(WALK_DID_EVENT, ({ payload }) => {
      const { what } = payload;
      if (what === "pat") lovePet();
      say(what);
      if (what !== "pat" && what !== "fetched") return;
      void useSettings.getState().update((s) => {
        const now = new Date();
        const streak = played(s.pet.streakDays, s.pet.lastPlayDay, now);
        if (what === "pat") return { ...s, pet: { ...s.pet, pats: s.pet.pats + 1, ...streak } };
        const f = fetched(s.pet.fetchesToday, s.pet.fetchDay, now);
        return {
          ...s,
          pet: {
            ...s.pet,
            ...streak,
            fetchesToday: f.fetchesToday,
            fetchDay: f.fetchDay,
            xp: s.pet.xp + f.xp,
            fullness: feed(s.pet.fullness, FETCH_FILLS),
          },
        };
      });
    });
    return () => void off.then((f) => f());
  }, [lead]);

  useEffect(() => {
    if (!lead) return;
    const off = listen(WALK_HELLO_EVENT, () => {
      if (latest.current) void emitTo(WALK_WINDOW, WALK_LOOK_EVENT, latest.current).catch(() => {});
    });
    return () => void off.then((f) => f());
  }, [lead]);

  // It says hello when it comes out, and chats now and then while you keep it out.
  useEffect(() => {
    if (!lead || !walking) return;
    lastProp.current = null;
    const hello = window.setTimeout(() => say("hello"), HELLO_AFTER_MS);
    // The message you asked for, every 2 or 5 minutes, while you keep it out.
    const chat = calledOut && chatMs > 0 ? window.setInterval(() => say("chat", true), chatMs) : undefined;
    return () => {
      window.clearTimeout(hello);
      window.clearInterval(chat);
    };
  }, [lead, walking, calledOut, chatMs]);

  // A word when you start coding or watching next to it.
  useEffect(() => {
    if (!lead || !walking || !prop) return;
    const before = lastProp.current;
    lastProp.current = prop;
    if (before === null || before === prop) return;
    if (prop === "laptop") say("coding");
    else if (prop === "popcorn") say("watching");
  }, [lead, walking, prop]);
}

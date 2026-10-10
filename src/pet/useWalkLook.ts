// The walking pet shows what the pet is doing (a laptop while you code, popcorn during a show,
// headphones with music) and speaks through a cloud above its head. The lead island works out
// both and sends them to the pet's window.
import { emitTo, listen } from "@tauri-apps/api/event";
import { useEffect, useRef } from "react";

import { isLeadIsland } from "../core/window";
import { useSettings } from "../settings";
import { inQuietHours } from "../shell/behavior";
import { lineFor, maySpeak, type BuddyEvent } from "./lines";
import type { Face, Prop } from "./petState";
import { lovePet, usePetLive } from "./petStore";
import { usePetLook } from "./usePetLook";

export const WALK_WINDOW = "petwalk";
/** Island → walking pet: how it looks and its latest line. */
export const WALK_LOOK_EVENT = "pet-walk-look";
/** Walking pet → island: its window just opened and wants the current state. */
export const WALK_HELLO_EVENT = "pet-walk-hello";
/** Walking pet → island: you clicked it (a treat). */
export const WALK_TREAT_EVENT = "pet-walk-treat";

export interface WalkLook {
  face: Face;
  prop: Prop;
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

  /** `timed`: the message you asked for every few minutes, which doesn't wait for a quiet spell. */
  const say = (event: BuddyEvent, timed = false) => {
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
      line: lineText ? { text: lineText, at: lineAt, holdMs: lineHold } : null,
    };
    if (walking) void emitTo(WALK_WINDOW, WALK_LOOK_EVENT, latest.current).catch(() => {});
  }, [lead, face, prop, walking, lineText, lineAt, lineHold]);

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
          pet: { ...s.pet, treats: s.pet.treats + add, xp: s.pet.xp + add },
        }));
      }, SAVE_TREATS_AFTER_MS);
    });
    return () => {
      window.clearTimeout(saveTimer);
      void off.then((f) => f());
    };
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

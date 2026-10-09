// The lead island is the pet's brain while it sits on the desktop: it works out how the pet looks
// and what it says, sends that to the pet's window, and handles the treats you give it there.
import { emitTo, listen } from "@tauri-apps/api/event";
import { useEffect, useRef } from "react";

import { isLeadIsland } from "../core/window";
import { useSettings } from "../settings";
import { inQuietHours } from "../shell/behavior";
import { lineFor, maySpeak, type BuddyEvent } from "./lines";
import type { PetLook } from "./petState";
import { lovePet, usePetLive, type PetLine } from "./petStore";
import { usePetLook } from "./usePetLook";

export const BUDDY_WINDOW = "petbuddy";
/** Island → pet window: how it looks and its latest line. */
export const BUDDY_STATE_EVENT = "pet-buddy";
/** Pet window → island: you gave it a treat. */
export const BUDDY_TREAT_EVENT = "pet-buddy-treat";
/** Pet window → island: it just opened and wants the current state. */
export const BUDDY_HELLO_EVENT = "pet-buddy-hello";

export interface BuddyState {
  look: PetLook & { name: string; color: string };
  line: PetLine | null;
  talk: boolean;
}

/** Resend now and then too, in case the pet's window opened after the last change. */
const RESEND_MS = 3000;
const SAVE_TREATS_AFTER_MS = 1500;

export function useBuddyLink() {
  const lead = isLeadIsland();
  const out = useSettings((s) => s.settings?.pet.buddyOut ?? false);
  const look = usePetLook();
  const line = usePetLive((s) => s.line);
  const talk = useSettings((s) => s.settings?.pet.talk ?? true);
  const latest = useRef<BuddyState | null>(null);
  const lastSaid = useRef(0);
  const lastProp = useRef<string | null>(null);

  const say = (event: BuddyEvent) => {
    const s = useSettings.getState().settings;
    if (!s?.pet.talk) return;
    const now = Date.now();
    const chatter = event === "chat" || event === "coding" || event === "watching";
    if (chatter && inQuietHours(s.behavior.quiet, new Date(now))) return;
    if (!maySpeak(event, s.pet.chattiness, lastSaid.current, now)) return;
    lastSaid.current = now;
    const text = lineFor(s.pet.tone, event, { name: s.pet.name.trim() || "Pet" });
    usePetLive.setState({ line: { text, event, at: now } });
  };

  // Send how it looks and what it says, whenever that changes (and every few seconds).
  const key = look ? JSON.stringify(look) : "";
  useEffect(() => {
    if (!lead || !out || !look) return;
    latest.current = { look, line, talk };
    void emitTo(BUDDY_WINDOW, BUDDY_STATE_EVENT, latest.current).catch(() => {});
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [lead, out, key, line, talk]);

  useEffect(() => {
    if (!lead || !out) return;
    const id = window.setInterval(() => {
      if (latest.current) void emitTo(BUDDY_WINDOW, BUDDY_STATE_EVENT, latest.current).catch(() => {});
    }, RESEND_MS);
    return () => window.clearInterval(id);
  }, [lead, out]);

  // Comments: hello when it comes out, when you start coding or watching, and now and then.
  useEffect(() => {
    if (!lead || !out) return;
    lastProp.current = null;
    const hello = window.setTimeout(() => say("hello"), 600);
    const chat = window.setInterval(() => say("chat"), 60_000);
    return () => {
      window.clearTimeout(hello);
      window.clearInterval(chat);
    };
  }, [lead, out]);

  const prop = look?.prop ?? null;
  useEffect(() => {
    if (!lead || !out || !prop) return;
    const before = lastProp.current;
    lastProp.current = prop;
    if (before === null || before === prop) return;
    if (prop === "laptop") say("coding");
    else if (prop === "popcorn") say("watching");
  }, [lead, out, prop]);

  // Treats from the pet's window, and its "I just opened" hello.
  useEffect(() => {
    if (!lead) return;
    let unsaved = 0;
    let saveTimer: number | undefined;
    const offTreat = listen(BUDDY_TREAT_EVENT, () => {
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
    const offHello = listen(BUDDY_HELLO_EVENT, () => {
      if (latest.current) void emitTo(BUDDY_WINDOW, BUDDY_STATE_EVENT, latest.current).catch(() => {});
    });
    return () => {
      window.clearTimeout(saveTimer);
      void offTreat.then((off) => off());
      void offHello.then((off) => off());
    };
  }, [lead]);
}

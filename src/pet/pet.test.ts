import { describe, expect, it } from "vitest";

import type { PetSettings } from "../core/generated/PetSettings";
import { lineFor, maySpeak, TONES } from "./lines";
import { ageInDays, levelFor, outfitFor, petLook, type PetInputs } from "./petState";
import { petOut, showInPetCloud, usePetLive } from "./petStore";
import { chatEveryMs } from "./useWalkLook";

const SETTINGS: PetSettings = {
  name: "Mochi",
  color: "",
  tone: "cheerful",
  talk: true,
  chattiness: "normal",
  chatMinutes: 5,
  sayMeetings: true,
  sayBreaks: true,
  sayFocus: true,
  sayAgents: true,
  sayBattery: true,
  sayWeather: true,
  sayLate: true,
  sayGreeting: true,
  reactMusic: true,
  reactVideo: true,
  reactCoding: true,
  reactWeather: true,
  reactBattery: true,
  reactDay: true,
  petting: true,
  leave: "click",
  born: "",
  pats: 0,
  xp: 0,
  treats: 0,
  buddyOut: false,
};

describe("where the one pet is", () => {
  it("is home in the notch unless it was called out or is walking", () => {
    expect(petOut(false, false)).toBe(false);
  });

  it("is out while it walks, and from the moment it's called out", () => {
    expect(petOut(false, true)).toBe(true);
    expect(petOut(true, false)).toBe(true);
    expect(petOut(true, true)).toBe(true);
  });
});

describe("the pet's cloud on the taskbar", () => {
  it("shows a reminder that comes due while it's out, and holds it longer than a chat line", () => {
    usePetLive.setState({ walking: true, line: null });
    expect(showInPetCloud("Reminder: Call mum", 20_000, 5000)).toBe(true);
    expect(usePetLive.getState().line).toMatchObject({
      text: "Reminder: Call mum",
      at: 5000,
      holdMs: 20_000,
    });
  });

  it("says nothing when the pet is home in the notch", () => {
    usePetLive.setState({ walking: false, line: null });
    expect(showInPetCloud("Reminder: Call mum")).toBe(false);
    expect(usePetLive.getState().line).toBeNull();
  });

  it("times its messages from the setting: every 2 or 5 minutes, or never", () => {
    expect(chatEveryMs(2)).toBe(120_000);
    expect(chatEveryMs(5)).toBe(300_000);
    expect(chatEveryMs(0)).toBe(0);
    expect(chatEveryMs(undefined)).toBe(0);
  });
});

const base: PetInputs = {
  now: 1000,
  hour: 14,
  idleMs: 0,
  cheeringUntil: 0,
  lovedUntil: 0,
  focusing: false,
  music: false,
  video: false,
  coding: "none",
  busyDay: false,
  settings: SETTINGS,
};

describe("pet look on the desktop", () => {
  it("gets its laptop out while you code (an editor or terminal in front)", () => {
    expect(petLook({ ...base, editing: true })).toMatchObject({
      prop: "laptop",
      face: "focused",
      status: "Coding with you",
    });
    // Coding on the pet's own screen while a video plays: laptop, not popcorn.
    expect(petLook({ ...base, editing: true, video: true, music: true })).toMatchObject({
      prop: "laptop",
      status: "Coding with you",
    });
    // Netflix on the other screen only: no popcorn on this one.
    expect(petLook({ ...base, video: true, videoElsewhere: true }).prop).toBe("none");
    // The video is on the pet's screen (or we can't tell where): popcorn.
    expect(petLook({ ...base, video: true }).prop).toBe("popcorn");
    // Turned off in settings: no laptop.
    expect(petLook({ ...base, editing: true, settings: { ...SETTINGS, reactCoding: false } }).prop).toBe(
      "none",
    );
  });
});

describe("pet look", () => {
  it("is happy by default, and reacts to what's going on", () => {
    expect(petLook(base)).toMatchObject({ face: "happy", prop: "none", motion: "idle" });
    expect(petLook({ ...base, music: true })).toMatchObject({
      prop: "headphones",
      motion: "dance",
      status: "Dancing to your music",
    });
    expect(petLook({ ...base, music: true, video: true })).toMatchObject({
      prop: "popcorn",
      status: "Watching with you",
    });
    expect(petLook({ ...base, coding: "working", agent: "Codex" })).toMatchObject({
      face: "focused",
      prop: "laptop",
      status: "Coding alongside Codex",
    });
    expect(petLook({ ...base, coding: "waiting", agent: "Claude Code" }).face).toBe("nervous");
    expect(petLook({ ...base, battery: { percent: 12, plugged: false } }).face).toBe("droopy");
    expect(petLook({ ...base, battery: { percent: 12, plugged: true } }).face).toBe("happy");
    expect(petLook({ ...base, busyDay: true }).face).toBe("stressed");
    expect(petLook({ ...base, hour: 2 }).face).toBe("sleepy");
    expect(petLook({ ...base, idleMs: 20 * 60_000 }).face).toBe("sleepy");
  });

  it("puts pats and cheers first, and honours the switches", () => {
    expect(petLook({ ...base, lovedUntil: 2000, coding: "waiting" })).toMatchObject({
      face: "love",
      motion: "wiggle",
    });
    expect(petLook({ ...base, cheeringUntil: 2000 })).toMatchObject({ face: "cheer", motion: "bounce" });
    const off = { ...SETTINGS, reactMusic: false, reactBattery: false, reactDay: false };
    expect(petLook({ ...base, settings: off, music: true }).prop).toBe("none");
    expect(petLook({ ...base, settings: off, battery: { percent: 5, plugged: false } }).face).toBe("happy");
    expect(petLook({ ...base, settings: off, busyDay: true }).face).toBe("happy");
  });

  it("dresses for the weather", () => {
    expect(outfitFor({ icon: "rain", temperature: 22 })).toBe("umbrella");
    expect(outfitFor({ icon: "sun", temperature: 34 })).toBe("sunglasses");
    expect(outfitFor({ icon: "cloud", temperature: 8 })).toBe("scarf");
    expect(outfitFor({ icon: "cloud", temperature: 22 })).toBe("none");
    expect(outfitFor(null)).toBe("none");
  });

  it("grows up", () => {
    expect(levelFor(0)).toEqual({ level: 1, into: 0, needed: 10 });
    expect(levelFor(10).level).toBe(2);
    expect(levelFor(24)).toEqual({ level: 3, into: 0, needed: 20 });
    expect(ageInDays("2026-10-01", new Date(2026, 9, 5, 15))).toBe(4);
    expect(ageInDays("", new Date())).toBe(0);
  });
});

describe("pet lines", () => {
  it("has every event in every tone, with placeholders filled", () => {
    for (const { id } of TONES) {
      const line = lineFor(id, "meeting", { minutes: 5, title: "Standup" }, () => 0);
      expect(line).not.toMatch(/\{\w+\}/);
      expect(line.length).toBeGreaterThan(5);
    }
    expect(lineFor("professional", "batteryLow", { percent: 14 }, () => 0)).toBe(
      "Battery at 14%. Please connect power.",
    );
    expect(lineFor("unknown", "pat", {}, () => 0)).toBe("Hehe, that tickles!");
  });

  it("has desktop-pet lines in every tone, and treats are always welcome", () => {
    for (const { id } of TONES) {
      for (const event of ["hello", "treat", "coding", "watching", "chat"] as const) {
        const line = lineFor(id, event, { name: "Mochi" }, () => 0);
        expect(line.length).toBeGreaterThan(5);
        expect(line).not.toMatch(/\{\w+\}/);
      }
    }
    expect(maySpeak("treat", "quiet", 0, 1000)).toBe(true);
    expect(maySpeak("chat", "quiet", 0, 1000)).toBe(false);
  });

  it("waits between lines, except for things that can't wait", () => {
    expect(maySpeak("greeting", "normal", 0, 5 * 60_000)).toBe(false);
    expect(maySpeak("greeting", "chatty", 0, 5 * 60_000)).toBe(true);
    expect(maySpeak("meeting", "quiet", 0, 1000)).toBe(true);
  });
});

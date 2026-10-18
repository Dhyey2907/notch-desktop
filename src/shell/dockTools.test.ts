import { describe, expect, it } from "vitest";

import type { DockProfile } from "../core/generated/DockProfile";
import { appCommands, commands, score, search, splitInput } from "../modules/palette/commands";
import { moveId, slotFor } from "./dockEdit";
import { activeDock, inHours, pickProfile, starterProfiles } from "./dockProfiles";
import { clock } from "./miniWidgets";

const profile = (over: Partial<DockProfile>): DockProfile => ({
  id: "p",
  name: "P",
  order: [],
  hidden: [],
  apps: [],
  ...over,
});

describe("dock rearranging", () => {
  it("moves an icon to a slot", () => {
    expect(moveId(["a", "b", "c", "d"], "a", 2)).toEqual(["b", "c", "a", "d"]);
    expect(moveId(["a", "b", "c"], "c", 0)).toEqual(["c", "a", "b"]);
    expect(moveId(["a", "b"], "a", 99)).toEqual(["b", "a"]);
  });

  it("finds the slot from the pointer", () => {
    expect(slotFor(5, [10, 30, 50])).toBe(0);
    expect(slotFor(35, [10, 30, 50])).toBe(2);
    expect(slotFor(99, [10, 30, 50])).toBe(3);
  });
});

describe("dock profiles", () => {
  const work = profile({ id: "work", fromMin: 9 * 60, toMin: 18 * 60 });
  const night = profile({ id: "night", fromMin: 22 * 60, toMin: 6 * 60 });
  const games = profile({ id: "games", apps: ["Steam.exe", "valorant"] });

  it("knows hours, even past midnight", () => {
    expect(inHours(work, 10 * 60)).toBe(true);
    expect(inHours(work, 19 * 60)).toBe(false);
    expect(inHours(night, 23 * 60)).toBe(true);
    expect(inHours(night, 3 * 60)).toBe(true);
    expect(inHours(night, 12 * 60)).toBe(false);
    expect(inHours(profile({}), 0)).toBe(false);
  });

  it("lets the app in front win over the time of day", () => {
    expect(pickProfile([work, games], "VALORANT.exe", 10 * 60)).toBe("games");
    expect(pickProfile([work, games], "code", 10 * 60)).toBe("work");
    expect(pickProfile([work, games], "code", 20 * 60)).toBe("");
  });

  it("shows the active profile's dock, or the main one", () => {
    const dock = {
      order: ["a"],
      hidden: [],
      profiles: [profile({ id: "x", order: ["b"], hidden: ["c"] })],
      active: "x",
      auto: false,
    };
    expect(activeDock(dock).order).toEqual(["b"]);
    expect(activeDock({ ...dock, active: "" }).order).toEqual(["a"]);
    expect(starterProfiles(["a"], []).map((p) => p.id)).toEqual(["work", "gaming", "study"]);
  });
});

describe("command palette", () => {
  it("ranks prefixes and words above scattered letters", () => {
    expect(score("tim", "Start a timer")).toBeGreaterThan(score("tmr", "Start a timer"));
    expect(score("xyz", "Start a timer")).toBe(0);
  });

  it("splits a command from what follows it", () => {
    expect(splitInput("timer 10")).toEqual({ word: "timer", arg: "10" });
    expect(splitInput(" note buy milk ")).toEqual({ word: "note", arg: "buy milk" });
  });

  it("puts a command with its argument first", () => {
    const list = commands();
    const top = search(list, "timer 10")[0];
    expect(top.command.id).toBe("timer");
    expect(top.arg).toBe("10");
    expect(search(list, "note call mom")[0]).toMatchObject({ arg: "call mom" });
    expect(search(list, "open mail")[0].command.id).toBe("open-mail");
    // Only what matches is listed.
    expect(search(list, "timer 10").map((r) => r.command.id)).toEqual(["timer"]);
  });
});

describe("palette launcher", () => {
  const apps = [
    { name: "Notepad", id: "Microsoft.WindowsNotepad!App" },
    { name: "Notion", id: "Notion" },
    { name: "Google Chrome", id: "Chrome" },
  ];
  const list = [...commands(), ...appCommands(apps)];

  it("finds an installed app by name", () => {
    expect(search(list, "notepad")[0].command.id).toBe("app-Microsoft.WindowsNotepad!App");
    expect(search(list, "chrome")[0].command.title).toBe("Open Google Chrome");
  });

  it("takes the app to close after the word close", () => {
    expect(search(list, "close chrome")[0]).toMatchObject({ arg: "chrome" });
    expect(search(list, "close chrome")[0].command.confirm).toBeTruthy();
  });

  it("asks before power commands", () => {
    const restart = search(list, "restart")[0].command;
    expect(restart.id).toBe("power-restart");
    expect(restart.confirm).toMatch(/again/);
  });
});

describe("mini widget clock", () => {
  it("formats time left", () => {
    expect(clock(65_000)).toBe("1:05");
    expect(clock(3_725_000)).toBe("1:02:05");
    expect(clock(-5)).toBe("0:00");
  });
});

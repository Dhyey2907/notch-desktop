import { describe, expect, it } from "vitest";

import { habitRoutine, habitText } from "./habit";

const s = {
  exes: ["spotify", "code", "chrome"],
  paths: ["C:/s.exe", "C:/c.exe", "C:/g.exe"],
  atMin: 573,
  key: "k",
};

describe("habit routines", () => {
  it("makes a timed routine that opens the apps", () => {
    const r = habitRoutine(s, "id1", [1, 2, 3, 4, 5]);
    expect(r.name).toBe("Start: Spotify, Code, Chrome");
    expect(r.steps).toEqual([
      { kind: "app", path: "C:/s.exe" },
      { kind: "app", path: "C:/c.exe" },
      { kind: "app", path: "C:/g.exe" },
    ]);
    expect(r.atMin).toBe(573);
  });

  it("explains it in plain words", () => {
    expect(habitText(s)).toBe(
      "You usually open Spotify, Code and Chrome around 9:35. Open them all at 9:33 on work days?",
    );
  });
});

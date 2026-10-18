import { describe, expect, it } from "vitest";

import { classify, onPet, restingAt, swipedThrough, type Sample } from "./petGestures";
import { carry, clampX, drop, PET_W, runTo, startBody, step, type Body } from "./petMotion";
import {
  currentStreak,
  dayKey,
  digest,
  feed,
  fetched,
  HATS,
  hungerOf,
  played,
  unlocked,
} from "../pet/petLife";

const W = 1000;

function run(b: Body, seconds: number, opts = {}) {
  let body = b;
  let arrived = false;
  let landed = false;
  for (let t = 0; t < seconds; t += 1 / 60) {
    const r = step(body, 1 / 60, W, opts);
    body = r.body;
    arrived ||= r.arrived;
    landed ||= r.landed;
  }
  return { body, arrived, landed };
}

describe("pet motion", () => {
  it("patrols, pausing and turning at the ends", () => {
    const end = run({ ...startBody(W), x: clampX(Infinity, W) - 5, dir: 1 }, 0.5);
    expect(end.body.mode).toBe("pause");
    const turned = run(end.body, 2.5);
    expect(turned.body.dir).toBe(-1);
    expect(turned.body.x).toBeLessThan(clampX(Infinity, W));
  });

  it("walks slower while sulking", () => {
    const a = run(startBody(W), 2).body.x;
    const b = run(startBody(W), 2, { sulky: true }).body.x;
    expect(b).toBeLessThan(a);
  });

  it("runs to a spot and says when it got there", () => {
    const r = run(runTo(startBody(W), 600), 4);
    expect(r.arrived).toBe(true);
    expect(r.body.x).toBe(600);
  });

  it("is carried within bounds, then falls, bounces and lands", () => {
    const held = carry(startBody(W), 5000, 300, W, 120);
    expect(held.x).toBe(W - Math.max(8, W * 0.02) - PET_W);
    expect(held.y).toBe(120);
    const r = run(drop(held), 3);
    expect(r.landed).toBe(true);
    expect(r.body.y).toBe(0);
    expect(r.body.mode).toBe("walk");
  });
});

const samples = (pts: [number, number, number][]): Sample[] => pts.map(([x, y, t]) => ({ x, y, t }));

describe("pet gestures", () => {
  it("reads a slow back-and-forth as petting", () => {
    const pts: [number, number, number][] = [];
    for (let i = 0; i < 12; i++) pts.push([100 + (i % 2 === 0 ? 0 : 14), 50, i * 90]);
    expect(classify(samples(pts))).toBe("pat");
  });

  it("reads a fast pass as a swipe", () => {
    expect(
      classify(
        samples([
          [0, 50, 0],
          [80, 50, 40],
        ]),
      ),
    ).toBe("swipe");
  });

  it("ignores a slow single pass", () => {
    expect(
      classify(
        samples([
          [0, 50, 0],
          [20, 50, 200],
          [40, 50, 400],
        ]),
      ),
    ).toBeNull();
  });

  it("finds a resting cursor only after it has rested long enough", () => {
    const still = samples([
      [300, 60, 0],
      [301, 60, 1000],
      [300, 61, 2200],
    ]);
    expect(restingAt(still, 2300)).toBe(300);
    expect(
      restingAt(
        samples([
          [300, 60, 1500],
          [300, 60, 2200],
        ]),
        2300,
      ),
    ).toBeNull();
    expect(
      restingAt(
        samples([
          [200, 60, 0],
          [300, 60, 2200],
        ]),
        2300,
      ),
    ).toBeNull();
  });

  it("sees a fast swipe that jumps over the pet between samples", () => {
    const r = { left: 100, right: 130, top: 90, bottom: 117 };
    expect(
      swipedThrough(
        samples([
          [40, 100, 0],
          [180, 102, 40],
        ]),
        r,
      ),
    ).toBe(true);
    expect(
      swipedThrough(
        samples([
          [40, 20, 0],
          [180, 20, 40],
        ]),
        r,
      ),
    ).toBe(false);
    expect(
      swipedThrough(
        samples([
          [40, 100, 0],
          [180, 100, 400],
        ]),
        r,
      ),
    ).toBe(false);
  });

  it("hit-tests the pet with a little slop", () => {
    const r = { left: 100, right: 130, top: 90, bottom: 117 };
    expect(onPet({ x: 95, y: 100 }, r)).toBe(true);
    expect(onPet({ x: 80, y: 100 }, r)).toBe(false);
  });
});

describe("pet life", () => {
  it("gets hungry slowly and treats fill it up", () => {
    expect(digest(80, 60)).toBe(72);
    expect(digest(3, 120)).toBe(0);
    expect(feed(95)).toBe(100);
    expect(hungerOf(70)).toBe("full");
    expect(hungerOf(20)).toBe("peckish");
    expect(hungerOf(5)).toBe("sulky");
  });

  it("keeps a daily play streak", () => {
    const d = new Date(2026, 9, 14, 10);
    expect(played(4, "2026-10-13", d)).toEqual({ streakDays: 5, lastPlayDay: "2026-10-14" });
    expect(played(4, "2026-10-14", d)).toEqual({ streakDays: 4, lastPlayDay: "2026-10-14" });
    expect(played(9, "2026-10-10", d).streakDays).toBe(1);
    expect(currentStreak(6, "2026-10-12", d)).toBe(0);
    expect(dayKey(new Date(2026, 0, 2))).toBe("2026-01-02");
  });

  it("unlocks hats with its level", () => {
    expect(unlocked(HATS, 1)).toEqual([]);
    expect(unlocked(HATS, 4).map((h) => h.id)).toEqual(["cap", "bow", "party"]);
  });

  it("gives fetch xp only for the first few a day", () => {
    const d = new Date(2026, 9, 14);
    expect(fetched(10, "2026-10-14", d).xp).toBe(0);
    expect(fetched(10, "2026-10-13", d)).toEqual({ fetchesToday: 1, fetchDay: "2026-10-14", xp: 1 });
  });
});

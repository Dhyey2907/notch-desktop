import { describe, expect, it } from "vitest";

import type { BreakSettings } from "../core/generated/BreakSettings";
import { currentStreak, recordBreak, reset, tick, ZERO } from "./breaks";

const S: BreakSettings = {
  noEyes: false,
  noStretch: false,
  noWater: true,
  eyeMin: 20,
  stretchMin: 60,
  waterMin: 90,
  day: "",
  count: 0,
  streak: 0,
  countedDay: "",
  best: 0,
};
const MIN = 60_000;

describe("breaks", () => {
  it("counts only time at the PC and fires the eyes first", () => {
    let t = ZERO;
    for (let i = 0; i < 19; i++) t = tick(t, S, { stepMs: MIN, idleMs: 1000, held: false }).timers;
    expect(t.eyes).toBe(19 * MIN);
    // Idle 2 min: no time counted, nothing due.
    expect(tick(t, S, { stepMs: MIN, idleMs: 2 * MIN, held: false })).toEqual({ timers: t, due: null });
    const r = tick(t, S, { stepMs: MIN, idleMs: 1000, held: false });
    expect(r.due).toBe("eyes");
  });

  it("waits while held, then reminds", () => {
    const t = { ...ZERO, eyes: 25 * MIN };
    expect(tick(t, S, { stepMs: MIN, idleMs: 0, held: true }).due).toBeNull();
    expect(tick(t, S, { stepMs: MIN, idleMs: 0, held: false }).due).toBe("eyes");
  });

  it("treats stepping away as a break, and snoozes", () => {
    const t = { eyes: 30 * MIN, stretch: 70 * MIN, water: 100 * MIN };
    expect(tick(t, S, { stepMs: MIN, idleMs: 6 * MIN, held: false })).toEqual({ timers: ZERO, due: null });
    expect(reset(t, S, "eyes").eyes).toBe(0);
    expect(reset(t, S, "eyes", 5 * MIN).eyes).toBe(15 * MIN);
    // Water is off: never due.
    expect(tick({ ...ZERO, water: 200 * MIN }, S, { stepMs: 0, idleMs: 0, held: false }).due).toBeNull();
  });

  it("builds a streak from days with three breaks", () => {
    let s = { ...S };
    const day1 = new Date(2026, 9, 1, 10);
    for (let i = 0; i < 3; i++) s = { ...s, ...recordBreak(s, day1) };
    expect(s.streak).toBe(1);
    const day2 = new Date(2026, 9, 2, 10);
    for (let i = 0; i < 3; i++) s = { ...s, ...recordBreak(s, day2) };
    expect(s.streak).toBe(2);
    expect(s.best).toBe(2);
    expect(currentStreak(s, new Date(2026, 9, 3, 9))).toBe(2);
    expect(currentStreak(s, new Date(2026, 9, 5, 9))).toBe(0);
    // A gap restarts it.
    const day5 = new Date(2026, 9, 5, 10);
    for (let i = 0; i < 3; i++) s = { ...s, ...recordBreak(s, day5) };
    expect(s.streak).toBe(1);
    expect(s.best).toBe(2);
  });
});

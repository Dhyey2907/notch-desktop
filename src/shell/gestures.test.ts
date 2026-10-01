import { describe, expect, it } from "vitest";

import { addWheel, idleSwipe, SWIPE_COOLDOWN_MS } from "./gestures";

function run(events: [number, number, number][]) {
  let state = idleSwipe();
  const fired: string[] = [];
  for (const [dx, dy, at] of events) {
    const r = addWheel(state, dx, dy, at);
    state = r.state;
    if (r.swipe) fired.push(r.swipe);
  }
  return fired;
}

describe("island swipes", () => {
  it("fires once per flick, on the dominant axis", () => {
    expect(
      run([
        [20, 2, 0],
        [25, 3, 16],
        [30, 1, 32],
      ]),
    ).toEqual(["right"]);
    expect(
      run([
        [-40, 0, 0],
        [-40, 5, 16],
      ]),
    ).toEqual(["left"]);
    expect(
      run([
        [2, -35, 0],
        [1, -35, 16],
      ]),
    ).toEqual(["up"]);
    expect(run([[0, 70, 0]])).toEqual(["down"]);
  });

  it("ignores small movements and waits out the cooldown", () => {
    expect(
      run([
        [20, 0, 0],
        [20, 0, 16],
      ]),
    ).toEqual([]);
    expect(
      run([
        [70, 0, 0],
        [70, 0, 50],
        [70, 0, SWIPE_COOLDOWN_MS + 10],
      ]),
    ).toEqual(["right", "right"]);
  });

  it("starts over after a pause", () => {
    expect(
      run([
        [40, 0, 0],
        [40, 0, 1000],
      ]),
    ).toEqual([]);
  });
});

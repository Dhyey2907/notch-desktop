import { describe, expect, it } from "vitest";

import { petMood, SLEEPY_AFTER_MS } from "./petMood";

describe("pet", () => {
  it("cheers first, then works during focus, sleeps when you're away", () => {
    const base = { cheeringUntil: 0, focusing: false, idleMs: 0, now: 1000 };
    expect(petMood(base)).toBe("happy");
    expect(petMood({ ...base, idleMs: SLEEPY_AFTER_MS })).toBe("sleepy");
    expect(petMood({ ...base, focusing: true, idleMs: SLEEPY_AFTER_MS })).toBe("working");
    expect(petMood({ ...base, focusing: true, cheeringUntil: 2000 })).toBe("cheering");
  });
});

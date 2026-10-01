import { describe, expect, it } from "vitest";

import { alertCard, formatMinutes, formatSpeed } from "./format";

describe("control center words", () => {
  it("formats speeds and times", () => {
    expect(formatSpeed(0)).toBe("0 B/s");
    expect(formatSpeed(340_000)).toBe("340 KB/s");
    expect(formatSpeed(12_345_678)).toBe("12.3 MB/s");
    expect(formatMinutes(45)).toBe("45 min");
    expect(formatMinutes(70)).toBe("1 h 10 min");
    expect(formatMinutes(120)).toBe("2 h");
  });

  it("words every alert", () => {
    expect(alertCard({ kind: "internetBack", seconds: 200 }).body).toBe("It was down for 3 min.");
    expect(alertCard({ kind: "internetBack", seconds: 20 }).body).toBeUndefined();
    expect(alertCard({ kind: "chargeLimit", percent: 80 }).title).toBe("Battery at 80%");
    expect(alertCard({ kind: "plugged", percent: 45, minutes: 70 }).body).toBe("Full in about 1 h 10 min.");
    expect(alertCard({ kind: "capsLock", on: true }).hud).toEqual({ kind: "caps", on: true });
    expect(alertCard({ kind: "language", name: "Hindi (India)" }).title).toBe("Hindi (India)");
    expect(alertCard({ kind: "deviceLow", name: "Buds", percent: 12 }).tone).toBe("warning");
  });
});

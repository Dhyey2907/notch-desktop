import { describe, expect, it } from "vitest";

import { alertCard, formatMinutes, formatSpeed, formatBytes } from "./format";

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

describe("data use", () => {
  it("formats bytes", () => {
    expect(formatBytes(512)).toBe("512 B");
    expect(formatBytes(1_234_000)).toBe("1.2 MB");
    expect(formatBytes(345_000_000_000)).toBe("345 GB");
  });

  it("words the data cap alerts", () => {
    expect(alertCard({ kind: "dataCap", percent: 80, cap_gb: 50 }).title).toBe("80% of your data cap used");
    expect(alertCard({ kind: "dataCap", percent: 100, cap_gb: 50 }).tone).toBe("danger");
  });
});

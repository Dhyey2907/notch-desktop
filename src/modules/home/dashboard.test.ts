import { describe, expect, it } from "vitest";

import { formatMinutes, greeting } from "./dashboard";

describe("home dashboard", () => {
  it("greets by time of day", () => {
    expect(greeting(8)).toBe("Good morning");
    expect(greeting(13)).toBe("Good afternoon");
    expect(greeting(19)).toBe("Good evening");
    expect(greeting(23)).toBe("Good night");
    expect(greeting(3)).toBe("Good night");
  });

  it("formats screen time", () => {
    expect(formatMinutes(378)).toBe("6h 18m");
    expect(formatMinutes(42)).toBe("42m");
    expect(formatMinutes(120)).toBe("2h");
    expect(formatMinutes(0)).toBe("0m");
  });
});

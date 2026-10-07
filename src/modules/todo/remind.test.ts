import { describe, expect, it } from "vitest";

import { remindChoices, reminderLabel, toLocalInput } from "./remind";

describe("reminder choices", () => {
  it("offers sensible times", () => {
    const sundayNoon = new Date(2026, 9, 4, 12, 0);
    const labels = remindChoices(sundayNoon).map((c) => c.label);
    expect(labels).toEqual(["In 1 hour", "This evening", "Tomorrow morning", "Next Monday"]);
    const choices = remindChoices(sundayNoon);
    expect(new Date(choices[1].at).getHours()).toBe(18);
    expect(new Date(choices[3].at).getDate()).toBe(5);
    // After 5 pm there's no "this evening".
    expect(remindChoices(new Date(2026, 9, 5, 19, 0)).map((c) => c.label)).not.toContain("This evening");
    // On a Monday, "next Monday" is a week away.
    expect(new Date(remindChoices(new Date(2026, 9, 5, 9, 0)).at(-1)!.at).getDate()).toBe(12);
  });

  it("reads reminder times naturally", () => {
    const now = new Date(2026, 9, 5, 10, 0);
    expect(reminderLabel(new Date(2026, 9, 6, 9, 0).getTime(), now)).toMatch(/^Tomorrow /);
    expect(reminderLabel(new Date(2026, 9, 5, 18, 0).getTime(), now)).not.toMatch(/Tomorrow/);
    expect(toLocalInput(new Date(2026, 0, 2, 3, 4).getTime())).toBe("2026-01-02T03:04");
  });
});

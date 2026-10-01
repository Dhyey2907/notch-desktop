import { beforeEach, describe, expect, it } from "vitest";

import { focusActive, focusProgress, heldSummary, holdDuringFocus, shouldHold, useFocus } from "./focusStore";

const at = (min: number) => new Date(Date.UTC(2026, 9, 1, 10, min)).toISOString();

describe("focus", () => {
  beforeEach(() => useFocus.setState({ state: null, held: [] }));

  it("holds only ordinary cards, and only during a session", () => {
    const card = { moduleId: "phone", priority: "normal" as const, title: "hi" };
    expect(holdDuringFocus(card)).toBe(false);
    useFocus.setState({
      state: {
        startedAt: new Date().toISOString(),
        endsAt: new Date(Date.now() + 60_000).toISOString(),
        doneToday: 0,
      },
    });
    expect(holdDuringFocus(card)).toBe(true);
    expect(holdDuringFocus({ ...card, priority: "high" })).toBe(false);
    expect(shouldHold({ ...card, priority: "low", hud: { kind: "volume", level: 1 } })).toBe(false);
    expect(useFocus.getState().takeHeld()).toHaveLength(1);
    expect(useFocus.getState().held).toHaveLength(0);
  });

  it("summarizes what was held", () => {
    const held = ["phone", "git", "phone"].map((moduleId) => ({
      moduleId,
      priority: "normal" as const,
      title: "x",
    }));
    const names = (id: string) => ({ phone: "Phone", git: "Git" })[id] ?? id;
    expect(heldSummary(held, names)).toBe("3 notifications · Phone 2, Git 1");
  });

  it("tracks progress", () => {
    const state = { startedAt: at(0), endsAt: at(20), doneToday: 0 };
    expect(focusProgress(state, Date.parse(at(5)))).toBeCloseTo(0.25);
    expect(focusActive(state, Date.parse(at(19)))).toBe(true);
    expect(focusActive(state, Date.parse(at(21)))).toBe(false);
  });
});

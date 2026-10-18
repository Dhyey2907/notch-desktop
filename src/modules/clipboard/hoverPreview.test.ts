import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { createHoverPreview } from "./hoverPreview";

describe("clipboard hover preview", () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it("waits before showing, and a quick pass shows nothing", () => {
    const show = vi.fn();
    const p = createHoverPreview(show, 300);
    p.enter("a");
    vi.advanceTimersByTime(200);
    p.leave();
    vi.advanceTimersByTime(500);
    expect(show).not.toHaveBeenCalled();
    p.enter("b");
    vi.advanceTimersByTime(300);
    expect(show).toHaveBeenLastCalledWith("b");
  });

  it("switches at once between entries while showing, and hides on leave", () => {
    const show = vi.fn();
    const p = createHoverPreview(show, 300);
    p.enter("a");
    vi.advanceTimersByTime(300);
    p.enter("b");
    expect(show).toHaveBeenLastCalledWith("b");
    p.leave();
    expect(show).toHaveBeenLastCalledWith(null);
    expect(show).toHaveBeenCalledTimes(3);
  });

  it("dispose cancels a pending show", () => {
    const show = vi.fn();
    const p = createHoverPreview(show, 300);
    p.enter("a");
    p.dispose();
    vi.advanceTimersByTime(1000);
    expect(show).not.toHaveBeenCalled();
  });
});

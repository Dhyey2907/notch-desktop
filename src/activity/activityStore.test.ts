import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { createActivityStore, MAX_ACTIVITIES, type ActivityInput } from "./activityStore";

let ids = 0;
function setup() {
  return createActivityStore({ now: () => Date.now(), createId: () => `a${++ids}` });
}

const base: ActivityInput = { moduleId: "test", priority: "normal", title: "Hello" };

beforeEach(() => {
  vi.useFakeTimers();
  vi.setSystemTime(0);
  ids = 0;
});
afterEach(() => vi.useRealTimers());

describe("ordering", () => {
  it("puts higher priority first regardless of arrival order", () => {
    const store = setup();
    store.getState().push({ ...base, priority: "low", title: "low" });
    store.getState().push({ ...base, priority: "critical", title: "critical" });
    store.getState().push({ ...base, priority: "high", title: "high" });

    expect(store.getState().activities.map((a) => a.title)).toEqual(["critical", "high", "low"]);
  });

  it("shows the newest first within the same priority", () => {
    const store = setup();
    store.getState().push({ ...base, title: "first" });
    vi.advanceTimersByTime(10);
    store.getState().push({ ...base, title: "second" });

    expect(store.getState().activities[0].title).toBe("second");
  });
});

describe("timeouts", () => {
  it("expires activities after their priority's default timeout", () => {
    const store = setup();
    store.getState().push({ ...base, priority: "low" });
    store.getState().push({ ...base, priority: "high" });

    vi.advanceTimersByTime(3000);
    expect(store.getState().activities.map((a) => a.priority)).toEqual(["high"]);
    vi.advanceTimersByTime(5000);
    expect(store.getState().activities).toEqual([]);
  });

  it("keeps critical and persistent activities until dismissed", () => {
    const store = setup();
    const critical = store.getState().push({ ...base, priority: "critical" });
    store.getState().push({ ...base, persistent: true });

    vi.advanceTimersByTime(60 * 60 * 1000);
    expect(store.getState().activities).toHaveLength(2);

    store.getState().dismiss(critical);
    expect(store.getState().activities).toHaveLength(1);
  });

  it("honours a custom timeout", () => {
    const store = setup();
    store.getState().push({ ...base, timeoutMs: 100 });
    vi.advanceTimersByTime(99);
    expect(store.getState().activities).toHaveLength(1);
    vi.advanceTimersByTime(1);
    expect(store.getState().activities).toHaveLength(0);
  });

  it("pauses while held and restarts the full timeout on release", () => {
    const store = setup();
    const id = store.getState().push({ ...base }); // 5s
    vi.advanceTimersByTime(4000);
    store.getState().hold(id);
    vi.advanceTimersByTime(60_000);
    expect(store.getState().activities).toHaveLength(1);

    store.getState().release(id);
    vi.advanceTimersByTime(4999);
    expect(store.getState().activities).toHaveLength(1);
    vi.advanceTimersByTime(1);
    expect(store.getState().activities).toHaveLength(0);
  });

  it("never gives a persistent activity a timeout on release", () => {
    const store = setup();
    const id = store.getState().push({ ...base, priority: "critical" });
    store.getState().hold(id);
    store.getState().release(id);
    vi.advanceTimersByTime(60 * 60 * 1000);
    expect(store.getState().activities).toHaveLength(1);
  });
});

describe("replacement", () => {
  it("replaces an activity with the same dedup key in place", () => {
    const store = setup();
    const id = store.getState().push({ ...base, dedupKey: "track", title: "Song A" });
    vi.advanceTimersByTime(4000);
    const again = store.getState().push({ ...base, dedupKey: "track", title: "Song B" });

    const [only, ...rest] = store.getState().activities;
    expect(rest).toEqual([]);
    expect(again).toBe(id);
    expect(only.title).toBe("Song B");
    expect(only.repeats).toBe(2);

    vi.advanceTimersByTime(4000); // timer restarted on replacement
    expect(store.getState().activities).toHaveLength(1);
  });

  it("does not merge dedup keys across modules", () => {
    const store = setup();
    store.getState().push({ ...base, moduleId: "a", dedupKey: "k" });
    store.getState().push({ ...base, moduleId: "b", dedupKey: "k" });
    expect(store.getState().activities).toHaveLength(2);
  });
});

it("updates content in place without a repeat or a new timer", () => {
  const store = setup();
  const id = store.getState().push({ ...base, title: "Song" });
  vi.advanceTimersByTime(4000);
  store.getState().update(id, { imageUrl: "art.png" });

  expect(store.getState().activities[0]).toMatchObject({ imageUrl: "art.png", repeats: 1 });
  vi.advanceTimersByTime(1000);
  expect(store.getState().activities).toHaveLength(0);
});

describe("actions", () => {
  it("runs an action and dismisses the activity", async () => {
    const store = setup();
    const run = vi.fn();
    const id = store.getState().push({ ...base, actions: [{ id: "ok", label: "OK", run }] });

    await store.getState().runAction(id, "ok");

    expect(run).toHaveBeenCalledOnce();
    expect(store.getState().activities).toEqual([]);
  });

  it("keeps the activity when the action asks to, or when it fails", async () => {
    const store = setup();
    const spy = vi.spyOn(console, "error").mockImplementation(() => {});
    const id = store.getState().push({
      ...base,
      actions: [
        { id: "keep", label: "Keep", run: () => {}, keepOpen: true },
        {
          id: "fail",
          label: "Fail",
          run: () => {
            throw new Error("nope");
          },
        },
      ],
    });

    await store.getState().runAction(id, "keep");
    await store.getState().runAction(id, "fail");

    expect(store.getState().activities).toHaveLength(1);
    spy.mockRestore();
  });
});

it("caps the queue, dropping the least important first", () => {
  const store = setup();
  store.getState().push({ ...base, priority: "critical", title: "keep me" });
  for (let i = 0; i < MAX_ACTIVITIES + 5; i++) store.getState().push({ ...base, priority: "low" });

  expect(store.getState().activities).toHaveLength(MAX_ACTIVITIES);
  expect(store.getState().activities[0].title).toBe("keep me");
});

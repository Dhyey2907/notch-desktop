import { describe, expect, it, vi } from "vitest";

import { EventBus } from "./eventBus";
import type { EventEnvelope } from "./generated/EventEnvelope";

function envelope(event: EventEnvelope["event"]): EventEnvelope {
  return { id: "id-1", v: 1, ts: "2026-09-28T00:00:00Z", source: "test", event };
}

describe("EventBus", () => {
  it("delivers events only to subscribers of that type", () => {
    const bus = new EventBus();
    const onReady = vi.fn();
    const onPong = vi.fn();
    bus.on("AppReady", onReady);
    bus.on("DiagnosticsPong", onPong);

    bus.dispatch(envelope({ type: "AppReady", payload: { version: "0.1.0" } }));

    expect(onReady).toHaveBeenCalledOnce();
    expect(onReady.mock.calls[0][0].event.payload.version).toBe("0.1.0");
    expect(onPong).not.toHaveBeenCalled();
  });

  it("delivers every event to wildcard subscribers", () => {
    const bus = new EventBus();
    const onAny = vi.fn();
    bus.onAny(onAny);

    bus.dispatch(envelope({ type: "AppReady", payload: { version: "0.1.0" } }));
    bus.dispatch(envelope({ type: "DiagnosticsPong", payload: { nonce: "n" } }));

    expect(onAny).toHaveBeenCalledTimes(2);
  });

  it("stops delivering after unsubscribe", () => {
    const bus = new EventBus();
    const handler = vi.fn();
    const off = bus.on("DiagnosticsPong", handler);
    off();

    bus.dispatch(envelope({ type: "DiagnosticsPong", payload: { nonce: "n" } }));

    expect(handler).not.toHaveBeenCalled();
  });

  it("isolates a throwing subscriber from the rest", () => {
    const bus = new EventBus();
    const good = vi.fn();
    const spy = vi.spyOn(console, "error").mockImplementation(() => {});
    bus.on("AppReady", () => {
      throw new Error("boom");
    });
    bus.on("AppReady", good);

    bus.dispatch(envelope({ type: "AppReady", payload: { version: "x" } }));

    expect(good).toHaveBeenCalledOnce();
    spy.mockRestore();
  });
});

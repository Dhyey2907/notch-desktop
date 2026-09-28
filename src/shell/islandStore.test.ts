import { describe, expect, it, vi } from "vitest";

import { windowSizeFor, type Size } from "./geometry";
import { createIslandStore } from "./islandStore";

function setup() {
  const sizes: Size[] = [];
  const resize = vi.fn(async (size: Size) => {
    sizes.push(size);
  });
  const store = createIslandStore(resize);
  return { store, sizes, resize };
}

describe("islandStore", () => {
  it("grows the window before switching to a larger mode", async () => {
    const { store, sizes } = setup();

    await store.getState().request("expanded");

    expect(sizes).toEqual([windowSizeFor("expanded")]);
    expect(store.getState().mode).toBe("expanded");
  });

  it("shrinks the window only after the animation settles", async () => {
    const { store, sizes } = setup();
    await store.getState().request("expanded");
    sizes.length = 0;

    await store.getState().request("collapsed");
    expect(store.getState().mode).toBe("collapsed");
    expect(sizes).toEqual([]); // still expanded-sized while animating

    await store.getState().settle();
    expect(sizes).toEqual([windowSizeFor("collapsed")]);
  });

  it("ignores requests for the current mode", async () => {
    const { store, resize } = setup();
    await store.getState().request("collapsed");
    expect(resize).not.toHaveBeenCalled();
  });

  it("lets the newest request win when requests overlap", async () => {
    let release!: () => void;
    const resize = vi.fn(
      (_: Size) =>
        new Promise<void>((resolve) => {
          release = resolve;
        }),
    );
    const store = createIslandStore(resize);

    const first = store.getState().request("expanded");
    const firstRelease = release;
    const second = store.getState().request("peek");
    release();
    firstRelease();
    await Promise.all([first, second]);

    expect(store.getState().mode).toBe("peek");
  });

  it("toggles between collapsed and expanded", async () => {
    const { store } = setup();
    await store.getState().toggleExpanded();
    expect(store.getState().mode).toBe("expanded");
    await store.getState().toggleExpanded();
    expect(store.getState().mode).toBe("collapsed");
  });

  it("keeps working after a failed resize", async () => {
    const resize = vi.fn().mockRejectedValueOnce(new Error("boom")).mockResolvedValue(undefined);
    const spy = vi.spyOn(console, "error").mockImplementation(() => {});
    const store = createIslandStore(resize);

    await store.getState().request("expanded");
    expect(store.getState().mode).toBe("expanded");
    spy.mockRestore();
  });
});

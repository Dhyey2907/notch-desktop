import { describe, expect, it } from "vitest";

import { createModuleStore } from "./moduleStore";

describe("moduleStore", () => {
  it("starts on the first module", () => {
    expect(createModuleStore(["home", "media"]).getState().activeId).toBe("home");
  });

  it("wraps when stepping past either end", () => {
    const store = createModuleStore(["a", "b", "c"]);
    store.getState().step(-1);
    expect(store.getState().activeId).toBe("c");
    store.getState().step(1);
    expect(store.getState().activeId).toBe("a");
    store.getState().step(2);
    expect(store.getState().activeId).toBe("c");
  });

  it("ignores unknown module ids", () => {
    const store = createModuleStore(["a", "b"]);
    store.getState().setActive("nope");
    expect(store.getState().activeId).toBe("a");
  });
});

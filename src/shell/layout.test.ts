import { describe, expect, it } from "vitest";

import { arrange, nudge, ordered } from "./layout";

const items = [{ id: "a" }, { id: "b" }, { id: "c" }, { id: "d" }];
const ids = (xs: { id: string }[]) => xs.map((x) => x.id).join("");

describe("panel layout", () => {
  it("keeps the built-in order until arranged", () => {
    expect(ids(arrange(items))).toBe("abcd");
  });

  it("puts arranged ids first and new ones after, in their built-in order", () => {
    expect(ids(ordered(items, { order: ["c", "a"], hidden: [], size: "normal" }))).toBe("cabd");
  });

  it("ignores ids that no longer exist and drops hidden ones", () => {
    expect(ids(arrange(items, { order: ["gone", "d"], hidden: ["b"], size: "normal" }))).toBe(
      "dacb".replace("b", ""),
    );
  });

  it("nudges within bounds", () => {
    expect(nudge(["a", "b", "c"], "b", -1)).toEqual(["b", "a", "c"]);
    expect(nudge(["a", "b", "c"], "a", -1)).toEqual(["a", "b", "c"]);
    expect(nudge(["a", "b", "c"], "c", 1)).toEqual(["a", "b", "c"]);
  });
});

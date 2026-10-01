import { describe, expect, it } from "vitest";

import { convertTargets, isConvertible, mergeablePdfs } from "./fileTools";

describe("shelf tools", () => {
  it("offers conversions an image doesn't already have", () => {
    expect(isConvertible("png")).toBe(true);
    expect(isConvertible("pdf")).toBe(false);
    expect(isConvertible(undefined)).toBe(false);
    expect(convertTargets("png")).toEqual(["jpg", "webp"]);
    expect(convertTargets("jpeg")).toEqual(["png", "webp"]);
    expect(convertTargets("gif")).toEqual(["png", "jpg", "webp"]);
  });

  it("merges only PDFs that are still there", () => {
    const item = (path: string, ext: string, exists = true) => ({ path, ext, exists, isDir: false });
    expect(
      mergeablePdfs([
        item("a.pdf", "pdf"),
        item("b.png", "png"),
        item("c.pdf", "pdf", false),
        item("d.pdf", "pdf"),
      ]),
    ).toEqual(["a.pdf", "d.pdf"]);
  });
});

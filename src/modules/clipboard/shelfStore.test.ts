import { describe, expect, it } from "vitest";

import { formatSize } from "./shelfStore";

describe("shelf", () => {
  it("formats file sizes", () => {
    expect(formatSize(0)).toBe("0 B");
    expect(formatSize(900)).toBe("900 B");
    expect(formatSize(1536)).toBe("1.5 KB");
    expect(formatSize(2_400_000)).toBe("2.3 MB");
    expect(formatSize(15 * 1024 ** 3)).toBe("15 GB");
  });
});

import { describe, expect, it } from "vitest";

import { findLink, readingMinutes } from "./readLaterStore";

describe("read later", () => {
  it("estimates reading time", () => {
    expect(readingMinutes(undefined)).toBeNull();
    expect(readingMinutes(60)).toBe(1);
    expect(readingMinutes(2300)).toBe(10);
  });

  it("finds a link in text", () => {
    expect(findLink("read this https://example.com/a?b=1, later")).toBe("https://example.com/a?b=1");
    expect(findLink("(see http://x.org/page).")).toBe("http://x.org/page");
    expect(findLink("no link here")).toBeNull();
  });
});

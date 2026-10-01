import { describe, expect, it } from "vitest";

import { matches, siteOf } from "./vaultStore";

const entry = {
  id: "GitHub.md",
  title: "GitHub",
  username: "dhyey",
  url: "https://www.github.com/login",
  tags: ["dev"],
  notes: "",
  hasPassword: true,
};

describe("vault", () => {
  it("searches titles, usernames, sites and tags", () => {
    expect(matches(entry, "git")).toBe(true);
    expect(matches(entry, "DHY")).toBe(true);
    expect(matches(entry, "dev")).toBe(true);
    expect(matches(entry, "bank")).toBe(false);
    expect(matches(entry, "  ")).toBe(true);
  });

  it("shows sites short", () => {
    expect(siteOf(entry.url)).toBe("github.com");
    expect(siteOf("example.org/path")).toBe("example.org");
    expect(siteOf("")).toBe("");
  });
});

import { describe, expect, it } from "vitest";

import { parseInline, parseMarkdown } from "./parseMarkdown";

describe("chat markdown", () => {
  it("parses inline code and bold", () => {
    expect(parseInline("Run `npm i` **now** please")).toEqual([
      { kind: "text", text: "Run " },
      { kind: "code", text: "npm i" },
      { kind: "text", text: " " },
      { kind: "bold", text: "now" },
      { kind: "text", text: " please" },
    ]);
  });

  it("splits blocks: paragraphs, lists, headings, code", () => {
    const md =
      "## Steps\nFirst line\nsame paragraph\n\n- one\n- two\n1. a\n2. b\n```ts\nconst x = 1;\n```\nDone.";
    const blocks = parseMarkdown(md);
    expect(blocks.map((b) => b.kind)).toEqual(["heading", "paragraph", "list", "list", "code", "paragraph"]);
    expect(blocks[1]).toEqual({
      kind: "paragraph",
      inline: [{ kind: "text", text: "First line same paragraph" }],
    });
    expect(blocks[2]).toMatchObject({ kind: "list", ordered: false });
    expect(blocks[3]).toMatchObject({ kind: "list", ordered: true });
    expect(blocks[4]).toEqual({ kind: "code", lang: "ts", text: "const x = 1;" });
  });

  it("shows an unfinished code block while it streams", () => {
    expect(parseMarkdown("```py\nprint(1)")).toEqual([{ kind: "code", lang: "py", text: "print(1)" }]);
  });

  it("never produces HTML", () => {
    const blocks = parseMarkdown("<img src=x onerror=alert(1)>");
    expect(blocks).toEqual([
      { kind: "paragraph", inline: [{ kind: "text", text: "<img src=x onerror=alert(1)>" }] },
    ]);
  });
});

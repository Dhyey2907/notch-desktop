// Just enough Markdown for chat answers: code fences, lists, headings, paragraphs, and inline
// `code` / **bold**. Parsed into plain data and rendered as React elements — never as HTML.

export type Inline = { kind: "text" | "code" | "bold"; text: string };
export type Block =
  | { kind: "code"; lang: string; text: string }
  | { kind: "list"; ordered: boolean; items: Inline[][] }
  | { kind: "heading"; inline: Inline[] }
  | { kind: "paragraph"; inline: Inline[] };

export function parseInline(text: string): Inline[] {
  const out: Inline[] = [];
  const re = /(`[^`\n]+`|\*\*[^*\n]+\*\*)/g;
  let last = 0;
  for (const m of text.matchAll(re)) {
    if (m.index! > last) out.push({ kind: "text", text: text.slice(last, m.index) });
    const token = m[0];
    out.push(
      token.startsWith("`")
        ? { kind: "code", text: token.slice(1, -1) }
        : { kind: "bold", text: token.slice(2, -2) },
    );
    last = m.index! + token.length;
  }
  if (last < text.length) out.push({ kind: "text", text: text.slice(last) });
  return out;
}

const LIST_ITEM = /^\s*(?:[-*+]|(\d+)[.)])\s+(.*)$/;

export function parseMarkdown(source: string): Block[] {
  const lines = source.replace(/\r\n/g, "\n").split("\n");
  const blocks: Block[] = [];
  let paragraph: string[] = [];
  const flush = () => {
    if (paragraph.length) blocks.push({ kind: "paragraph", inline: parseInline(paragraph.join(" ")) });
    paragraph = [];
  };
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const fence = line.match(/^\s*```(\S*)/);
    if (fence) {
      flush();
      const body: string[] = [];
      i++;
      // An unclosed fence (still streaming) runs to the end.
      while (i < lines.length && !/^\s*```/.test(lines[i])) body.push(lines[i++]);
      blocks.push({ kind: "code", lang: fence[1], text: body.join("\n") });
      continue;
    }
    const heading = line.match(/^#{1,6}\s+(.*)$/);
    if (heading) {
      flush();
      blocks.push({ kind: "heading", inline: parseInline(heading[1]) });
      continue;
    }
    const item = line.match(LIST_ITEM);
    if (item) {
      flush();
      const ordered = item[1] !== undefined;
      const last = blocks[blocks.length - 1];
      if (last?.kind === "list" && last.ordered === ordered) last.items.push(parseInline(item[2]));
      else blocks.push({ kind: "list", ordered, items: [parseInline(item[2])] });
      continue;
    }
    if (!line.trim()) {
      flush();
      continue;
    }
    paragraph.push(line.trim());
  }
  flush();
  return blocks;
}

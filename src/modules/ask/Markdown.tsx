import { useMemo } from "react";

import { parseMarkdown, type Inline } from "./parseMarkdown";

function Inlines({ parts }: { parts: Inline[] }) {
  return (
    <>
      {parts.map((p, i) =>
        p.kind === "code" ? (
          <code key={i}>{p.text}</code>
        ) : p.kind === "bold" ? (
          <strong key={i}>{p.text}</strong>
        ) : (
          <span key={i}>{p.text}</span>
        ),
      )}
    </>
  );
}

export function Markdown({ text }: { text: string }) {
  const blocks = useMemo(() => parseMarkdown(text), [text]);
  return (
    <div className="ask-md">
      {blocks.map((b, i) => {
        switch (b.kind) {
          case "code":
            return (
              <pre key={i} className="ask-md__code">
                <code>{b.text}</code>
              </pre>
            );
          case "heading":
            return (
              <p key={i} className="ask-md__heading">
                <Inlines parts={b.inline} />
              </p>
            );
          case "list": {
            const List = b.ordered ? "ol" : "ul";
            return (
              <List key={i}>
                {b.items.map((item, j) => (
                  <li key={j}>
                    <Inlines parts={item} />
                  </li>
                ))}
              </List>
            );
          }
          default:
            return (
              <p key={i}>
                <Inlines parts={b.inline} />
              </p>
            );
        }
      })}
    </div>
  );
}

// Rewrite: the text you selected, a few styles (or your own instruction), the AI's version, and
// Replace to put it where the selection was.
import { Check, Copy, Replace } from "lucide-react";
import { useEffect, useState } from "react";

import type { RewriteSession } from "../../core/generated/RewriteSession";
import { ipc, isIpcError } from "../../core/ipc";
import { Button, EmptyState, ModuleContainer } from "../../design/components";
import { useIsland } from "../../shell/useIsland";
import "./rewrite.css";

const STYLES = [
  { id: "fix", label: "Fix grammar" },
  { id: "shorter", label: "Shorter" },
  { id: "formal", label: "Formal" },
  { id: "friendlier", label: "Friendlier" },
] as const;

export default function RewriteExpanded() {
  const [session, setSession] = useState<RewriteSession | null | undefined>(undefined);
  const [custom, setCustom] = useState("");
  const [result, setResult] = useState<string | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    ipc.rewriteSession().then(setSession, () => setSession(null));
  }, []);

  const run = (style: string) => {
    setBusy(style);
    setError(null);
    setResult(null);
    void ipc
      .rewriteRun(style, style === "custom" ? custom : "")
      .then(setResult, (e) => setError(isIpcError(e) ? e.message : "The AI didn't answer"))
      .finally(() => setBusy(null));
  };

  const replace = () => {
    if (!result) return;
    void useIsland.getState().request("collapsed");
    void ipc.rewriteApply(result).catch((e) => setError(isIpcError(e) ? e.message : "Couldn't paste it"));
  };

  if (session === undefined) return <ModuleContainer title="Rewrite">{null}</ModuleContainer>;
  if (!session)
    return (
      <ModuleContainer title="Rewrite">
        <EmptyState
          title="Select some text first"
          body="Select text in any app, then press Ctrl+Alt+R: pick a style and the new version replaces it."
        />
      </ModuleContainer>
    );

  return (
    <ModuleContainer title={session.app ? `Rewrite · from ${session.app}` : "Rewrite"}>
      <div className="rewrite">
        <p className="rewrite__text" title={result ? "The new version" : "What you selected"}>
          {result ?? session.text}
        </p>
        <div className="rewrite__styles">
          {STYLES.map((s) => (
            <button
              key={s.id}
              type="button"
              className="nd-chip"
              disabled={busy !== null}
              aria-busy={busy === s.id}
              onClick={() => run(s.id)}
            >
              {busy === s.id ? "…" : s.label}
            </button>
          ))}
          <form
            className="rewrite__custom"
            onSubmit={(e) => {
              e.preventDefault();
              if (custom.trim()) run("custom");
            }}
          >
            <input
              className="nd-input"
              aria-label="Your own instruction"
              placeholder="Or say how: “make it a bullet list”"
              value={custom}
              maxLength={300}
              onChange={(e) => setCustom(e.target.value)}
              onKeyDown={(e) => e.stopPropagation()}
            />
          </form>
        </div>
        {error && <p className="rewrite__error">{error}</p>}
        <div className="rewrite__actions">
          <Button variant="primary" disabled={!result} onClick={replace}>
            <Replace size={13} aria-hidden /> Replace
          </Button>
          <Button
            disabled={!result}
            onClick={() => {
              if (!result) return;
              void navigator.clipboard.writeText(result).then(() => {
                setCopied(true);
                window.setTimeout(() => setCopied(false), 1500);
              });
            }}
          >
            {copied ? <Check size={13} aria-hidden /> : <Copy size={13} aria-hidden />} Copy
          </Button>
          {result && <Button onClick={() => setResult(null)}>Show original</Button>}
        </div>
      </div>
    </ModuleContainer>
  );
}

import { useAsk } from "./askStore";

export function AskPeek() {
  const last = useAsk((s) => s.snapshot?.exchanges.at(-1));
  return (
    <div className="module-peek">
      <span className="module-peek__title">{last ? last.question : "Ask Claude"}</span>
      <span className="module-peek__meta">
        {last?.pending ? "Answering…" : last ? "Click to see the answer" : "Ctrl+Alt+A from anywhere"}
      </span>
    </div>
  );
}

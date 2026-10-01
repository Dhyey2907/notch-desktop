// The Ask panel: the conversation, and a box to ask the next question. Enter sends,
// Shift+Enter adds a line. "Include <app>" sends a screenshot of the window you came from.
import { AppWindow, Copy, CornerDownLeft, Square, Trash2 } from "lucide-react";
import { useEffect, useRef, useState, type FormEvent, type KeyboardEvent } from "react";

import type { Exchange } from "../../core/generated/Exchange";
import { ipc, isIpcError } from "../../core/ipc";
import { EmptyState, IconButton, ModuleContainer } from "../../design/components";
import { useAsk } from "./askStore";
import { Markdown } from "./Markdown";
import "./ask.css";

function message(error: unknown) {
  return isIpcError(error)
    ? error.message.replace(/^invalid input: /, "")
    : "Something went wrong. Try again.";
}

export default function AskExpanded() {
  const snapshot = useAsk((s) => s.snapshot);
  const [question, setQuestion] = useState("");
  const [withScreen, setWithScreen] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const input = useRef<HTMLTextAreaElement>(null);
  const list = useRef<HTMLDivElement>(null);
  const exchanges = snapshot?.exchanges ?? [];
  const busy = exchanges.some((e) => e.pending);
  const lastAnswerLength = exchanges.at(-1)?.answer.length ?? 0;

  useEffect(() => input.current?.focus(), []);
  // Follow the answer as it streams in.
  useEffect(() => {
    list.current?.scrollTo({ top: list.current.scrollHeight });
  }, [exchanges.length, lastAnswerLength]);

  const send = async () => {
    if (!question.trim() || busy) return;
    try {
      await ipc.askQuestion(question, withScreen && Boolean(snapshot?.targetApp));
      setQuestion("");
      setError(null);
    } catch (e) {
      setError(message(e));
    }
  };

  const onKey = (e: KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      void send();
    }
  };

  return (
    <ModuleContainer
      title="Ask Claude"
      trailing={
        exchanges.length > 0 && (
          <IconButton
            icon={Trash2}
            label="Clear conversation"
            size="sm"
            onClick={() => void ipc.askClear()}
          />
        )
      }
    >
      <div className="ask">
        <div className="ask__list" ref={list}>
          {snapshot && !snapshot.available ? (
            <EmptyState
              title="Claude Code isn't installed"
              body="Ask uses the Claude Code app on this PC (claude.ai/code) — no API key needed."
            />
          ) : exchanges.length === 0 ? (
            <EmptyState
              title="Ask anything"
              body="Answers come from your Claude Code. Press Ctrl+Alt+A in any app to ask about the window you're in."
            />
          ) : (
            exchanges.map((e) => <ExchangeView key={e.id} exchange={e} />)
          )}
        </div>
        {error && <p className="ask__error">{error}</p>}
        <form
          className="ask__form"
          onSubmit={(e: FormEvent) => {
            e.preventDefault();
            void send();
          }}
        >
          <textarea
            ref={input}
            className="ask__input"
            aria-label="Your question"
            placeholder={busy ? "Answering…" : "Ask a question"}
            rows={1}
            maxLength={4000}
            value={question}
            onChange={(e) => setQuestion(e.target.value)}
            onKeyDown={onKey}
          />
          {snapshot?.targetApp && (
            <button
              type="button"
              className={`ask__screen${withScreen ? " ask__screen--on" : ""}`}
              aria-pressed={withScreen}
              title={`Send a screenshot of the ${snapshot.targetApp} window with your question`}
              onClick={() => setWithScreen(!withScreen)}
            >
              <AppWindow size={12} aria-hidden /> {snapshot.targetApp}
            </button>
          )}
          {busy ? (
            <IconButton icon={Square} label="Stop" size="sm" onClick={() => void ipc.askStop()} />
          ) : (
            <IconButton
              icon={CornerDownLeft}
              label="Send"
              size="sm"
              disabled={!question.trim()}
              onClick={() => void send()}
            />
          )}
        </form>
      </div>
    </ModuleContainer>
  );
}

function ExchangeView({ exchange }: { exchange: Exchange }) {
  const [copied, setCopied] = useState(false);
  const copy = () =>
    void navigator.clipboard.writeText(exchange.answer).then(
      () => {
        setCopied(true);
        window.setTimeout(() => setCopied(false), 1200);
      },
      () => {},
    );
  return (
    <div className="ask-ex">
      <p className="ask-ex__q">
        {exchange.question}
        {exchange.withScreen && <span className="ask-ex__tag">+ window</span>}
      </p>
      <div className="ask-ex__a">
        {exchange.answer ? (
          <Markdown text={exchange.answer} />
        ) : exchange.pending ? (
          <span className="ask-ex__dots" aria-label="Answering">
            <i />
            <i />
            <i />
          </span>
        ) : null}
        {exchange.error && <p className="ask-ex__error">{exchange.error}</p>}
        {!exchange.pending && exchange.answer && (
          <button type="button" className="ask-ex__copy" onClick={copy}>
            <Copy size={11} aria-hidden /> {copied ? "Copied" : "Copy"}
          </button>
        )}
      </div>
    </div>
  );
}

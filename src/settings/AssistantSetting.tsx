// Settings → AI → Assistant: which AI answers Ask, plans the day and writes commit messages.
import { useEffect, useState } from "react";

import type { AiSettings } from "../core/generated/AiSettings";
import { ipc, isIpcError } from "../core/ipc";
import { Button, ConfirmButton, Pill, Surface, TextField } from "../design/components";
import { useSettings } from ".";
import { Row, Select } from "./controls";
import { patch } from "./patch";

const PROVIDERS = [
  { value: "claude", label: "Claude Code" },
  { value: "codex", label: "Codex" },
  { value: "gemini", label: "Gemini CLI" },
  { value: "ollama", label: "Ollama (on this PC)" },
  { value: "openai", label: "An API (OpenAI, OpenRouter, Gemini API…)" },
];

const ABOUT: Record<string, string> = {
  claude: "Uses your Claude Code sign-in. No API key needed. Can look at your window.",
  codex: "Uses your Codex sign-in (the Codex app or codex command), read-only. Can look at your window.",
  gemini: "Uses your Gemini CLI sign-in. It can't see your window, so screenshots aren't offered.",
  ollama: "Runs a model on this PC — private and free. Leave the model empty to use the first one installed.",
  openai: "Any service that speaks the OpenAI API. Your key is kept in Windows Credential Manager.",
};

/** API address presets for OpenAI-compatible services. */
const PRESETS = [
  { value: "https://api.openai.com/v1", label: "OpenAI", model: "gpt-5-mini" },
  { value: "https://openrouter.ai/api/v1", label: "OpenRouter", model: "openai/gpt-5-mini" },
  {
    value: "https://generativelanguage.googleapis.com/v1beta/openai",
    label: "Google Gemini API",
    model: "gemini-2.5-flash",
  },
  { value: "https://api.groq.com/openai/v1", label: "Groq", model: "llama-3.3-70b-versatile" },
  { value: "http://127.0.0.1:1234/v1", label: "LM Studio (this PC)", model: "" },
];

function errorText(e: unknown): string {
  return isIpcError(e)
    ? e.message.replace(/^invalid input: /, "").replace(/^./, (c) => c.toUpperCase())
    : "That didn't work.";
}

export function AssistantSetting() {
  const ai = useSettings((s) => s.settings?.ai);
  if (!ai) return null;
  // Text fields keep a draft and save when you leave them; remount when settings change elsewhere.
  return <Assistant key={`${ai.provider}|${ai.models[ai.provider] ?? ""}|${ai.baseUrl}`} ai={ai} />;
}

function Assistant({ ai }: { ai: AiSettings }) {
  const current = ai.models[ai.provider] ?? "";
  const [model, setModel] = useState(current);
  const [baseUrl, setBaseUrl] = useState(ai.baseUrl);
  const [key, setKey] = useState("");
  const [keySaved, setKeySaved] = useState<boolean | null>(null);
  const [ollamaModels, setOllamaModels] = useState<string[]>([]);
  const [test, setTest] = useState<{ ok: boolean; text: string } | "running" | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (ai.provider === "openai") ipc.aiKeySaved().then(setKeySaved, () => setKeySaved(false));
    if (ai.provider === "ollama") ipc.aiOllamaModels().then(setOllamaModels, () => setOllamaModels([]));
  }, [ai.provider]);

  const save = (change: Partial<AiSettings>) => patch("ai", change);
  const saveModel = (m: string) => save({ models: { ...ai.models, [ai.provider]: m.trim() } });
  const preset = PRESETS.find((p) => p.value === ai.baseUrl);

  const runTest = async () => {
    setTest("running");
    try {
      setTest({ ok: true, text: await ipc.aiTest() });
    } catch (e) {
      setTest({ ok: false, text: errorText(e) });
    }
  };

  return (
    <Surface>
      <Row
        label="Assistant"
        description="Which AI answers Ask (Ctrl+Alt+A), plans your day and writes commit messages."
      >
        <Select<string>
          label="Assistant"
          value={ai.provider}
          options={PROVIDERS}
          onChange={(provider) => {
            setTest(null);
            save({ provider });
          }}
        />
      </Row>
      <p className="settings__note">{ABOUT[ai.provider]}</p>

      {ai.provider === "openai" && (
        <>
          <Row label="Service">
            <Select<string>
              label="Service"
              value={preset ? preset.value : "custom"}
              options={[
                ...PRESETS.map((p) => ({ value: p.value, label: p.label })),
                { value: "custom", label: "Other address" },
              ]}
              onChange={(v) => {
                const p = PRESETS.find((x) => x.value === v);
                save({ baseUrl: p ? p.value : "" });
              }}
            />
          </Row>
          {!preset && (
            <div className="settings__field">
              <TextField
                label="API address"
                value={baseUrl}
                placeholder="https://…/v1"
                onChange={setBaseUrl}
              />
              {baseUrl !== ai.baseUrl && (
                <Button onClick={() => save({ baseUrl: baseUrl.trim() })}>Save address</Button>
              )}
            </div>
          )}
        </>
      )}

      {ai.provider !== "claude" && (
        <div className="settings__field">
          {ai.provider === "ollama" && ollamaModels.length > 0 ? (
            <Row label="Model">
              <Select<string>
                label="Model"
                value={current}
                options={[
                  { value: "", label: "First installed" },
                  ...ollamaModels.map((m) => ({ value: m, label: m })),
                ]}
                onChange={(m) => saveModel(m)}
              />
            </Row>
          ) : (
            <>
              <TextField
                label="Model"
                value={model}
                placeholder={
                  ai.provider === "openai"
                    ? preset?.model
                      ? `e.g. ${preset.model}`
                      : "The model's name"
                    : "Leave empty for the default"
                }
                onChange={setModel}
              />
              {model !== current && <Button onClick={() => saveModel(model)}>Save model</Button>}
            </>
          )}
        </div>
      )}

      {ai.provider === "openai" && (
        <div className="settings__field">
          {keySaved ? (
            <Row label="API key" description="Saved in Windows Credential Manager.">
              <span className="settings__buttons">
                <Pill tone="success">Saved</Pill>
                <ConfirmButton
                  variant="danger"
                  confirmLabel="Remove the saved API key?"
                  onConfirm={() =>
                    void ipc.aiClearKey().then(
                      () => setKeySaved(false),
                      (e) => setError(errorText(e)),
                    )
                  }
                >
                  Remove
                </ConfirmButton>
              </span>
            </Row>
          ) : (
            <>
              <TextField
                label="API key"
                type="password"
                value={key}
                placeholder="Paste your key (not needed for LM Studio)"
                onChange={setKey}
              />
              {key.trim() && (
                <Button
                  variant="primary"
                  onClick={() =>
                    void ipc.aiSetKey(key).then(
                      () => {
                        setKey("");
                        setKeySaved(true);
                        setError(null);
                      },
                      (e) => setError(errorText(e)),
                    )
                  }
                >
                  Save key
                </Button>
              )}
            </>
          )}
        </div>
      )}

      <div className="settings__buttons">
        <Button onClick={() => void runTest()} disabled={test === "running"}>
          {test === "running" ? "Testing…" : "Test"}
        </Button>
        {test && test !== "running" && (
          <span className={`settings__note${test.ok ? "" : " settings__note--warning"}`}>
            {test.ok ? `It answered: “${test.text.slice(0, 80)}”` : test.text}
          </span>
        )}
      </div>
      {error && <p className="settings__note settings__note--warning">{error}</p>}
    </Surface>
  );
}

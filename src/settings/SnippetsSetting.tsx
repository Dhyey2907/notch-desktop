import { Pencil, X } from "lucide-react";
import { useEffect, useState } from "react";

import type { Snippet } from "../core/generated/Snippet";
import { ipc, isIpcError } from "../core/ipc";
import { Button, IconButton, Surface, Switch, TextField } from "../design/components";
import { useSettings } from ".";

/** Settings → Clipboard & files: text snippets (";addr" → your address, in any app). */
export function SnippetsSetting() {
  const off = useSettings((s) => s.settings?.clipboard.noSnippets ?? false);
  const [snippets, setSnippets] = useState<Snippet[]>([]);
  const [editing, setEditing] = useState<string | null>(null);
  const [trigger, setTrigger] = useState(";");
  const [body, setBody] = useState("");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    ipc.snippetsList().then(setSnippets, () => {});
  }, []);

  const reset = () => {
    setEditing(null);
    setTrigger(";");
    setBody("");
    setError(null);
  };

  const save = () =>
    void ipc.snippetSave(editing, trigger, body).then(
      (list) => {
        setSnippets(list);
        reset();
      },
      (e) => setError(isIpcError(e) ? e.message.replace(/^invalid input: /, "") : "Couldn't save that."),
    );

  return (
    <Surface>
      <div id="setting-text-snippets">
        <Switch
          label="Text snippets"
          description="Type a short trigger in any app and it turns into the full text — like ;addr for your address or ;sig for a sign-off. {date} and {time} fill in today's date and the time. Not used in password boxes."
          checked={!off}
          onChange={(on) =>
            void useSettings
              .getState()
              .update((s) => ({ ...s, clipboard: { ...s.clipboard, noSnippets: !on } }))
          }
        />
      </div>
      {!off && (
        <>
          {snippets.length > 0 && (
            <ul className="snippets" aria-label="Snippets">
              {snippets.map((s) => (
                <li key={s.id} className="snippets__item">
                  <kbd className="settings__kbd">{s.trigger}</kbd>
                  <span className="snippets__body" title={s.body}>
                    {s.body.replace(/\s+/g, " ")}
                  </span>
                  <IconButton
                    icon={Pencil}
                    label={`Edit ${s.trigger}`}
                    size="sm"
                    onClick={() => {
                      setEditing(s.id);
                      setTrigger(s.trigger);
                      setBody(s.body);
                    }}
                  />
                  <IconButton
                    icon={X}
                    label={`Delete ${s.trigger}`}
                    size="sm"
                    onClick={() => void ipc.snippetDelete(s.id).then(setSnippets)}
                  />
                </li>
              ))}
            </ul>
          )}
          <form
            className="snippets__form"
            onSubmit={(e) => {
              e.preventDefault();
              save();
            }}
          >
            <TextField label="Trigger" value={trigger} onChange={setTrigger} placeholder=";addr" />
            <label className="nd-field snippets__text">
              <span className="nd-field__label">Turns into</span>
              <textarea
                className="vault-form__notes"
                rows={2}
                value={body}
                onChange={(e) => setBody(e.target.value)}
                placeholder="12 Main Street, Ahmedabad"
              />
            </label>
            <div className="settings__buttons settings__buttons--start">
              <Button variant="primary" type="submit" disabled={trigger.length < 2 || !body}>
                {editing ? "Save snippet" : "Add snippet"}
              </Button>
              {editing && <Button onClick={reset}>Cancel</Button>}
            </div>
            {error && <p className="settings__note settings__note--warning">{error}</p>}
          </form>
        </>
      )}
    </Surface>
  );
}

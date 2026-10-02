import { FolderOpen, KeyRound } from "lucide-react";
import { useEffect, useState } from "react";

import type { VaultState } from "../core/generated/VaultState";
import { ipc, isIpcError } from "../core/ipc";
import { Button, ConfirmButton, Surface, Switch, TextField } from "../design/components";
import { useSettings } from ".";
import { Row, Select } from "./controls";
import { patch } from "./patch";
import { pageLead } from "./searchIndex";
import { Section } from "./Section";

function message(e: unknown) {
  return isIpcError(e) ? e.message.replace(/^invalid input: /, "") : "That didn't work.";
}

export function VaultSection() {
  const vault = useSettings((s) => s.settings?.vault);
  const [state, setState] = useState<VaultState | null>(null);
  const [key, setKey] = useState<string | null>(null);
  const [importing, setImporting] = useState("");
  const [note, setNote] = useState<{ text: string; warning?: boolean } | null>(null);

  useEffect(() => {
    ipc.vaultState().then(setState, () => {});
  }, [vault?.folder]);

  if (!vault) return null;

  return (
    <Section title="Vault" lead={pageLead("vault")}>
      <Surface>
        <Row
          label="Obsidian vault"
          description={
            vault.folder
              ? `Logins are notes in “${vault.folder}\\Notch Vault”. Edit them in Obsidian or in the notch — both stay in sync.`
              : "Pick your Obsidian vault folder. Logins are saved as notes in a “Notch Vault” folder inside it."
          }
        >
          <Button
            onClick={() =>
              void ipc.vaultChooseFolder().then(
                (s) => s && setState(s),
                (e) => setNote({ text: message(e), warning: true }),
              )
            }
          >
            <FolderOpen size={14} aria-hidden /> {vault.folder ? "Change" : "Choose folder"}
          </Button>
        </Row>
        {state?.folder && (
          <p className="settings__note">
            {state.entries.length} {state.entries.length === 1 ? "login" : "logins"}. Passwords in the notes
            are encrypted (AES-256); the key is kept in Windows Credential Manager on this PC, so a synced or
            shared vault never contains a readable password.
          </p>
        )}
      </Surface>
      <Surface>
        <div id="setting-ask-for-windows-hello">
          <Switch
            label="Ask for Windows Hello"
            description={
              state?.helloAvailable === false
                ? "Windows Hello isn't set up on this PC (Settings → Accounts → Sign-in options)."
                : "Confirm with your face, fingerprint or PIN before a password is shown or copied."
            }
            checked={vault.requireHello}
            disabled={state?.helloAvailable === false && !vault.requireHello}
            onChange={(requireHello) => patch("vault", { requireHello })}
          />
        </div>
        <Row
          label="Clear copied passwords after"
          description="Copied passwords and 2FA codes never go into clipboard history."
        >
          <Select
            label="Clear copied passwords after"
            value={vault.neverClear ? "never" : String(vault.clearSecs || 30)}
            options={[
              { value: "15", label: "15 seconds" },
              { value: "30", label: "30 seconds" },
              { value: "60", label: "1 minute" },
              { value: "120", label: "2 minutes" },
              { value: "never", label: "Never" },
            ]}
            onChange={(v) =>
              patch(
                "vault",
                v === "never" ? { neverClear: true } : { neverClear: false, clearSecs: Number(v) },
              )
            }
          />
        </Row>
      </Surface>
      <Surface>
        <Row
          label="Vault key"
          description="To open these passwords on another PC (with the same Obsidian vault), copy this key there. Keep it private — with the vault, it opens every password."
        >
          {key ? (
            <Button
              onClick={() => {
                void navigator.clipboard.writeText(key);
                setKey(null);
                setNote({ text: "Key copied. Paste it on the other PC, then clear your clipboard." });
              }}
            >
              <KeyRound size={14} aria-hidden /> Copy key
            </Button>
          ) : (
            <ConfirmButton
              confirmLabel="Show the key?"
              onConfirm={() =>
                void ipc.vaultExportKey().then(setKey, (e) => setNote({ text: message(e), warning: true }))
              }
            >
              Show key
            </ConfirmButton>
          )}
        </Row>
        {key && <p className="settings__note vault-key">{key}</p>}
        <form
          className="settings__row settings__row--stacked"
          onSubmit={(e) => {
            e.preventDefault();
            void ipc.vaultImportKey(importing).then(
              () => {
                setImporting("");
                setNote({ text: "Key saved. Passwords from the other PC open here now." });
              },
              (err) => setNote({ text: message(err), warning: true }),
            );
          }}
        >
          <TextField
            label="Use a key from another PC"
            type="password"
            value={importing}
            onChange={setImporting}
            description="Replaces this PC's key — passwords saved only with the old key won't open afterwards."
          />
          <div className="settings__buttons settings__buttons--start">
            <Button type="submit" disabled={!importing.trim()}>
              Use this key
            </Button>
          </div>
        </form>
        {note && (
          <p className={`settings__note${note.warning ? " settings__note--warning" : ""}`} role="status">
            {note.text}
          </p>
        )}
      </Surface>
    </Section>
  );
}

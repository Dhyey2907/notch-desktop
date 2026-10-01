// Vault: Logins | 2FA codes.
import {
  Copy,
  ExternalLink,
  Eye,
  EyeOff,
  KeyRound,
  Pencil,
  Plus,
  RefreshCw,
  ShieldCheck,
  Trash2,
  User,
} from "lucide-react";
import { useState } from "react";

import type { VaultEntry } from "../../core/generated/VaultEntry";
import { ipc, isIpcError } from "../../core/ipc";
import {
  Button,
  ConfirmButton,
  EmptyState,
  IconButton,
  ModuleContainer,
  SegmentedControl,
  TextField,
} from "../../design/components";
import { useCodes } from "./useCodes";
import { matches, siteOf, useVault, type VaultTab } from "./vaultStore";
// The list rows share the Clipboard module's look (styles only).
import "../clipboard/clipboard.css";
import "./vault.css";

function message(error: unknown) {
  return isIpcError(error)
    ? error.message.replace(/^invalid input: /, "")
    : "Something went wrong. Try again.";
}

export default function VaultExpanded() {
  const tab = useVault((s) => s.tab);
  return (
    <ModuleContainer
      title="Vault"
      trailing={
        <SegmentedControl<VaultTab>
          label="Vault view"
          value={tab}
          options={[
            { value: "logins", label: "Logins" },
            { value: "codes", label: "2FA codes" },
          ]}
          onChange={(t) => useVault.getState().setTab(t)}
        />
      }
    >
      {tab === "logins" ? <Logins /> : <Codes />}
    </ModuleContainer>
  );
}

function Logins() {
  const state = useVault((s) => s.state);
  const [query, setQuery] = useState("");
  const [status, setStatus] = useState<string | null>(null);
  const [editing, setEditing] = useState<VaultEntry | "new" | null>(null);

  const run = (action: Promise<unknown>, done?: string) =>
    void action.then(
      () => setStatus(done ?? null),
      (e) => setStatus(message(e)),
    );

  if (!state) return <div className="vault" />;
  if (!state.folder) {
    return (
      <EmptyState
        title="Keep your logins in Obsidian"
        body="Pick your Obsidian vault. Logins are saved as notes in a “Notch Vault” folder — passwords encrypted, so they're safe even if the vault syncs."
        action={
          <Button variant="primary" onClick={() => run(ipc.vaultChooseFolder())}>
            Choose Obsidian vault
          </Button>
        }
      />
    );
  }
  if (editing) {
    return (
      <LoginForm
        entry={editing === "new" ? null : editing}
        onDone={(text) => {
          setEditing(null);
          setStatus(text);
        }}
      />
    );
  }

  const shown = state.entries.filter((e) => matches(e, query));
  return (
    <div className="vault">
      <div className="clips-bar">
        <input
          className="clips-search"
          aria-label="Search logins"
          placeholder="Search"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
        <span className="clips-bar__spacer" />
        <IconButton icon={Plus} label="Add a login" size="sm" onClick={() => setEditing("new")} />
      </div>
      {status && <p className="clips-status">{status}</p>}
      {state.entries.length === 0 ? (
        <EmptyState
          title="No logins yet"
          body="Add one here, or create a note in Obsidian's “Notch Vault” folder."
        />
      ) : shown.length === 0 ? (
        <EmptyState title="No matches" body="Try a different search." />
      ) : (
        <ul className="clips-list" aria-label="Logins">
          {shown.map((entry) => (
            <li key={entry.id} className="clip vault-item">
              <button
                type="button"
                className="clip__main"
                title="Copy password"
                onClick={() =>
                  entry.hasPassword
                    ? run(
                        ipc.vaultCopy(entry.id, "password"),
                        `Copied the ${entry.title} password · clears in 30 s`,
                      )
                    : run(ipc.vaultCopy(entry.id, "username"), "Copied the username")
                }
              >
                <span className="clip__icon vault-item__avatar" aria-hidden>
                  {entry.title.charAt(0).toUpperCase()}
                </span>
                <span className="clip__text">
                  <span className="clip__title">{entry.title}</span>
                  <span className="clip__meta">
                    {[entry.username, siteOf(entry.url)].filter(Boolean).join(" · ") || "No username"}
                  </span>
                </span>
              </button>
              <div className="clip__actions">
                {entry.username && (
                  <IconButton
                    icon={User}
                    label={`Copy the ${entry.title} username`}
                    size="sm"
                    onClick={() => run(ipc.vaultCopy(entry.id, "username"), "Copied the username")}
                  />
                )}
                {entry.hasPassword && (
                  <IconButton
                    icon={KeyRound}
                    label={`Copy the ${entry.title} password`}
                    size="sm"
                    onClick={() =>
                      run(
                        ipc.vaultCopy(entry.id, "password"),
                        `Copied the ${entry.title} password · clears in 30 s`,
                      )
                    }
                  />
                )}
                {entry.url && (
                  <IconButton
                    icon={ExternalLink}
                    label={`Open ${siteOf(entry.url)}`}
                    size="sm"
                    onClick={() =>
                      run(ipc.openExternal(entry.url.includes("://") ? entry.url : `https://${entry.url}`))
                    }
                  />
                )}
                <IconButton
                  icon={Pencil}
                  label={`Edit ${entry.title}`}
                  size="sm"
                  onClick={() => setEditing(entry)}
                />
              </div>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

function LoginForm({ entry, onDone }: { entry: VaultEntry | null; onDone: (status: string | null) => void }) {
  const [title, setTitle] = useState(entry?.title ?? "");
  const [username, setUsername] = useState(entry?.username ?? "");
  const [url, setUrl] = useState(entry?.url ?? "");
  // Empty while editing means "keep the saved password".
  const [password, setPassword] = useState("");
  const [visible, setVisible] = useState(false);
  const [tags, setTags] = useState(entry?.tags.join(", ") ?? "");
  const [notes, setNotes] = useState(entry?.notes ?? "");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const save = async () => {
    setBusy(true);
    setError(null);
    try {
      await ipc.vaultSave({
        id: entry?.id,
        title,
        username,
        url,
        password: entry && password === "" ? undefined : password,
        tags: tags
          .split(",")
          .map((t) => t.trim())
          .filter(Boolean),
        notes,
      });
      onDone(entry ? "Saved" : `Added ${title.trim()}`);
    } catch (e) {
      setError(message(e));
    } finally {
      setBusy(false);
    }
  };

  const reveal = async () => {
    if (visible) return setVisible(false);
    if (entry?.hasPassword && password === "") {
      try {
        setPassword(await ipc.vaultReveal(entry.id));
      } catch (e) {
        return setError(message(e));
      }
    }
    setVisible(true);
  };

  return (
    <form
      className="vault-form"
      onSubmit={(e) => {
        e.preventDefault();
        void save();
      }}
    >
      <div className="vault-form__grid">
        <TextField label="Title" value={title} onChange={setTitle} placeholder="GitHub" />
        <TextField label="Username or email" value={username} onChange={setUsername} />
        <div className="vault-form__password">
          <TextField
            label="Password"
            type={visible ? "text" : "password"}
            value={password}
            onChange={setPassword}
            placeholder={entry?.hasPassword ? "Saved — type to change" : ""}
            autoComplete="new-password"
          />
          <IconButton
            icon={visible ? EyeOff : Eye}
            label={visible ? "Hide password" : "Show password"}
            size="sm"
            onClick={() => void reveal()}
          />
          <IconButton
            icon={RefreshCw}
            label="Make a strong password"
            size="sm"
            onClick={() =>
              void ipc.vaultGeneratePassword(20).then((p) => {
                setPassword(p);
                setVisible(true);
              })
            }
          />
        </div>
        <TextField label="Website" value={url} onChange={setUrl} placeholder="github.com" />
        <TextField label="Tags" value={tags} onChange={setTags} placeholder="work, dev" />
      </div>
      <label className="nd-field">
        <span className="nd-field__label">Notes</span>
        <textarea
          className="vault-form__notes"
          value={notes}
          onChange={(e) => setNotes(e.target.value)}
          rows={2}
        />
      </label>
      {error && <p className="clips-status vault-form__error">{error}</p>}
      <div className="vault-form__buttons">
        {entry && (
          <ConfirmButton
            confirmLabel="Move to Recycle Bin?"
            variant="danger"
            onConfirm={() =>
              void ipc.vaultDelete(entry.id).then(
                () => onDone(`Moved ${entry.title} to the Recycle Bin`),
                (e) => setError(message(e)),
              )
            }
          >
            <Trash2 size={13} aria-hidden /> Delete
          </ConfirmButton>
        )}
        <span className="clips-bar__spacer" />
        <Button onClick={() => onDone(null)}>Cancel</Button>
        <Button variant="primary" type="submit" disabled={busy || !title.trim()}>
          {entry ? "Save" : "Add login"}
        </Button>
      </div>
    </form>
  );
}

function Codes() {
  const { codes, refresh } = useCodes();
  const [adding, setAdding] = useState(false);
  const [input, setInput] = useState("");
  const [name, setName] = useState("");
  const [status, setStatus] = useState<string | null>(null);

  const add = () =>
    void ipc.totpAdd(input, name).then(
      () => {
        setAdding(false);
        setInput("");
        setName("");
        setStatus("Added");
        void refresh();
      },
      (e) => setStatus(message(e)),
    );

  if (adding) {
    return (
      <form
        className="vault-form"
        onSubmit={(e) => {
          e.preventDefault();
          add();
        }}
      >
        <TextField
          label="Setup link or secret key"
          value={input}
          onChange={setInput}
          type="password"
          placeholder="otpauth://totp/… or JBSW Y3DP EHPK 3PXP"
          description="When a site shows a QR code for an authenticator app, it also offers a key or link — paste it here."
        />
        {!input.trim().toLowerCase().startsWith("otpauth://") && (
          <TextField label="Account name" value={name} onChange={setName} placeholder="Discord" />
        )}
        {status && <p className="clips-status vault-form__error">{status}</p>}
        <div className="vault-form__buttons">
          <span className="clips-bar__spacer" />
          <Button onClick={() => setAdding(false)}>Cancel</Button>
          <Button variant="primary" type="submit" disabled={!input.trim()}>
            Add
          </Button>
        </div>
      </form>
    );
  }

  return (
    <div className="vault">
      <div className="clips-bar">
        <span className="vault-note">
          <ShieldCheck size={13} aria-hidden /> Secrets stay in Windows Credential Manager
        </span>
        <span className="clips-bar__spacer" />
        <IconButton icon={Plus} label="Add a 2FA account" size="sm" onClick={() => setAdding(true)} />
      </div>
      {status && <p className="clips-status">{status}</p>}
      {codes && codes.length === 0 ? (
        <EmptyState title="No 2FA accounts" body="Add one to see its codes here and copy them in a click." />
      ) : (
        <ul className="clips-list" aria-label="2FA codes">
          {(codes ?? []).map((c) => (
            <li key={c.id} className="clip vault-code">
              <button
                type="button"
                className="clip__main"
                title="Copy code"
                onClick={() =>
                  void ipc.totpCopy(c.id).then(
                    () => setStatus(`Copied the ${c.issuer} code`),
                    (e) => setStatus(message(e)),
                  )
                }
              >
                <span
                  className="vault-code__ring"
                  aria-hidden
                  style={{ "--left": c.remaining / c.period } as React.CSSProperties}
                />
                <span className="clip__text">
                  <span className="vault-code__digits">
                    {c.code.length === 6 ? `${c.code.slice(0, 3)} ${c.code.slice(3)}` : c.code}
                  </span>
                  <span className="clip__meta">
                    {c.issuer}
                    {c.label ? ` · ${c.label}` : ""} · {c.remaining}s
                  </span>
                </span>
              </button>
              <div className="clip__actions">
                <IconButton
                  icon={Copy}
                  label={`Copy the ${c.issuer} code`}
                  size="sm"
                  onClick={() => void ipc.totpCopy(c.id).then(() => setStatus(`Copied the ${c.issuer} code`))}
                />
                <ConfirmButton
                  confirmLabel="Remove?"
                  variant="danger"
                  onConfirm={() => void ipc.totpDelete(c.id).then(() => void refresh())}
                >
                  <Trash2 size={13} aria-hidden />
                </ConfirmButton>
              </div>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

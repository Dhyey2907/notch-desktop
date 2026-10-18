import { X } from "lucide-react";
import { useState, type FormEvent } from "react";

import { ipc } from "../core/ipc";
import { Button, ConfirmButton, IconButton, Surface, Switch, TextField } from "../design/components";
import { useSettings } from ".";
import { Row, Select } from "./controls";
import { exeName } from "./clipboardApps";
import { pageLead } from "./searchIndex";
import { Section } from "./Section";
import { SnippetsSetting } from "./SnippetsSetting";

const MAX_BLOCKED = 50;

export function ClipboardSection() {
  const clipboard = useSettings((s) => s.settings?.clipboard);
  const [app, setApp] = useState("");
  const [error, setError] = useState<string | null>(null);
  const blocked = clipboard?.blockedApps ?? [];

  const save = (change: {
    paused?: boolean;
    blockedApps?: string[];
    noSuggestions?: boolean;
    noLinkCleaning?: boolean;
    shelfKeep?: string;
    noPhoneCodes?: boolean;
    noScreenshots?: boolean;
  }) =>
    void useSettings
      .getState()
      .update((s) => ({ ...s, clipboard: { ...s.clipboard, ...change } }))
      .then(
        () => setError(null),
        () => setError("Couldn't save that. Try again."),
      );

  const block = () => {
    const name = exeName(app);
    if (!name || name.length > 100) return;
    if (!blocked.includes(name) && blocked.length < MAX_BLOCKED) save({ blockedApps: [...blocked, name] });
    setApp("");
  };

  return (
    <Section title="Clipboard & files" lead={pageLead("clipboard")}>
      <Surface>
        <Switch
          label="Keep clipboard history"
          description="Everything you copy shows up in the Clipboard module (Ctrl+Alt+V). History stays in memory and is gone when Overline restarts; only pinned clips are saved."
          checked={!clipboard?.paused}
          onChange={(on) => save({ paused: !on })}
        />
      </Surface>
      <Surface>
        <Switch
          label="Suggest reminders and maps"
          description="Copy a date or time (meeting Thu 4pm) to get Remind me or Add to Calendar, or an address to open it in Maps. Checked on this PC only."
          checked={!clipboard?.noSuggestions}
          onChange={(on) => save({ noSuggestions: !on })}
        />
        <Switch
          label="Clean tracking from links"
          description="When you copy a link, tracking tags like utm_source, fbclid or YouTube's si= are removed before you paste it. The page still opens the same."
          checked={!clipboard?.noLinkCleaning}
          onChange={(on) => save({ noLinkCleaning: !on })}
        />
        <div id="setting-copy-sign-in-codes-from-the-phone">
          <Switch
            label="Copy sign-in codes from the phone"
            description="A code in a phone notification (an OTP, a verification code) is copied for you — kept out of clipboard history and cleared after 30 seconds."
            checked={!clipboard?.noPhoneCodes}
            onChange={(on) => save({ noPhoneCodes: !on })}
          />
        </div>
        <div id="setting-screenshots-to-the-shelf">
          <Switch
            label="Screenshots to the shelf"
            description="Each new screenshot (Win+Shift+S, Print Screen) lands on the shelf, with Copy text, Mark up and To phone."
            checked={!clipboard?.noScreenshots}
            onChange={(on) => save({ noScreenshots: !on })}
          />
        </div>
        <div id="setting-keep-shelf-files">
          <Row
            label="Keep shelf files"
            description="Files you drop on the notch leave the shelf after this (pinned ones stay). The files themselves aren't touched."
          >
            <Select
              label="Keep shelf files"
              value={clipboard?.shelfKeep || "day"}
              options={[
                { value: "day", label: "For a day" },
                { value: "week", label: "For a week" },
                { value: "forever", label: "Until I remove them" },
              ]}
              onChange={(shelfKeep) => save({ shelfKeep })}
            />
          </Row>
        </div>
      </Surface>
      <SnippetsSetting />
      <p className="settings__note">
        Copies that apps mark as private are never kept — password managers such as 1Password, Bitwarden and
        KeePass do this, and they're skipped even if they don't.
      </p>

      <Surface>
        <form
          className="settings__row settings__row--stacked"
          onSubmit={(e: FormEvent) => {
            e.preventDefault();
            block();
          }}
        >
          <TextField
            label="Never keep copies from"
            value={app}
            onChange={setApp}
            placeholder="slack.exe"
            description="The app's program name, e.g. slack.exe or chrome.exe."
          />
          <div className="settings__buttons settings__buttons--start">
            <Button variant="primary" disabled={!exeName(app)} onClick={block}>
              Block app
            </Button>
          </div>
        </form>
        {blocked.length > 0 && (
          <ul className="settings__chips" aria-label="Blocked apps">
            {blocked.map((name) => (
              <li key={name} className="settings__chip">
                {name}
                <IconButton
                  icon={X}
                  label={`Stop blocking ${name}`}
                  size="sm"
                  onClick={() => save({ blockedApps: blocked.filter((b) => b !== name) })}
                />
              </li>
            ))}
          </ul>
        )}
      </Surface>

      <div className="settings__buttons settings__buttons--start">
        <ConfirmButton
          variant="danger"
          confirmLabel="Clear history?"
          onConfirm={() => void ipc.clipboardClear()}
        >
          Clear clipboard history
        </ConfirmButton>
      </div>
      {error && <p className="settings__note settings__note--warning">{error}</p>}
    </Section>
  );
}

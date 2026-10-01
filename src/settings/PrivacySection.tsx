import { Download, Upload } from "lucide-react";
import { useEffect, useState } from "react";

import type { AppInfo } from "../core/generated/AppInfo";
import { ipc, isIpcError } from "../core/ipc";
import { Button, Surface } from "../design/components";
import { Row } from "./controls";
import { pageLead } from "./searchIndex";
import { Section } from "./Section";

export function PrivacySection() {
  const [message, setMessage] = useState<{ text: string; warning?: boolean } | null>(null);

  const run = async (action: () => Promise<string | null>) => {
    try {
      const text = await action();
      if (text) setMessage({ text });
    } catch (e) {
      setMessage({
        text: isIpcError(e) ? e.message.replace(/^invalid input: /, "") : "That didn't work.",
        warning: true,
      });
    }
  };

  return (
    <Section title="Privacy & data" lead={pageLead("privacy")}>
      <Surface>
        <p className="settings__lead">
          Everything Notch keeps stays on this PC: settings and history in its data folder, passwords and
          sign-ins in Windows Credential Manager. Logs record what happened — never message text, copied text,
          commands or passwords.
        </p>
      </Surface>
      <Surface>
        <Row
          label="Export settings"
          description="Save your settings to a file, e.g. to set up another PC the same way. Sign-ins aren't included."
        >
          <Button
            onClick={() =>
              void run(async () => {
                const path = await ipc.settingsExport();
                return path ? `Saved to ${path}` : null;
              })
            }
          >
            <Download size={14} aria-hidden /> Export
          </Button>
        </Row>
        <Row
          label="Import settings"
          description="Load settings from an exported file. Accounts you've connected stay connected."
        >
          <Button
            onClick={() => void run(async () => ((await ipc.settingsImport()) ? "Settings imported." : null))}
          >
            <Upload size={14} aria-hidden /> Import
          </Button>
        </Row>
        {message && (
          <p className={`settings__note${message.warning ? " settings__note--warning" : ""}`} role="status">
            {message.text}
          </p>
        )}
      </Surface>
      <Surface>
        <Row label="Logs" description="For troubleshooting. Export them to share when something goes wrong.">
          <span className="settings-inline">
            <Button onClick={() => void ipc.diagnosticsOpenLogDir().catch(() => {})}>Open folder</Button>
            <Button onClick={() => void run(async () => `Saved to ${await ipc.diagnosticsExportLogs()}`)}>
              Export logs
            </Button>
          </span>
        </Row>
      </Surface>
    </Section>
  );
}

export function AboutSection() {
  const [info, setInfo] = useState<AppInfo | null>(null);
  useEffect(() => {
    ipc.appInfo().then(setInfo, () => {});
  }, []);

  return (
    <Section title="About" lead={pageLead("about")}>
      {info && (
        <Surface>
          <dl id="setting-version" className="settings__facts">
            <dt>Version</dt>
            <dd>{info.version}</dd>
            <dt>Data folder</dt>
            <dd>{info.dataDir}</dd>
            <dt>Log folder</dt>
            <dd>{info.logDir}</dd>
          </dl>
        </Surface>
      )}
      <Surface>
        <p className="settings__lead">
          Notch Desktop puts a Dynamic Island at the top of your screen: music, calendar, Claude Code
          approvals, clipboard, focus and more. Press <kbd className="settings__kbd">Ctrl + Alt + N</kbd> (or
          your own shortcut) to open it from anywhere.
        </p>
      </Surface>
    </Section>
  );
}

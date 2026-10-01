// Settings window (design.md §21): a short list of sections, not a dashboard. Sections for
// Media, Calendar, Git and AI are added alongside those modules.
import { useEffect, useState } from "react";

import type { AnimationPreference } from "../core/generated/AnimationPreference";
import type { AppInfo } from "../core/generated/AppInfo";
import type { ScreenInfo } from "../core/generated/ScreenInfo";
import { ipc } from "../core/ipc";
import { Button, MockBadge, Pill, SegmentedControl, Surface, Switch } from "../design/components";
import { MODULES } from "../modules/registry";
import { useSettings } from ".";
import { AiSection } from "./AiSection";
import { CalendarSection } from "./CalendarSection";
import { ClipboardSection } from "./ClipboardSection";
import { GitSection } from "./GitSection";
import { HudSetting } from "./HudSetting";
import { MediaSection } from "./MediaSection";
import { Section } from "./Section";
import { moduleSettings } from "./settingsStore";
import "./settings.css";

type SectionId =
  "general" | "appearance" | "modules" | "media" | "calendar" | "git" | "clipboard" | "ai" | "diagnostics";

const SECTIONS: { id: SectionId; label: string }[] = [
  { id: "general", label: "General" },
  { id: "appearance", label: "Appearance" },
  { id: "modules", label: "Modules" },
  { id: "media", label: "Media" },
  { id: "calendar", label: "Calendar" },
  { id: "git", label: "Git" },
  { id: "clipboard", label: "Clipboard" },
  { id: "ai", label: "AI" },
  { id: "diagnostics", label: "Diagnostics & About" },
];

/** `#settings/calendar` → "calendar"; anything unknown falls back to General. */
function sectionFromHash(): SectionId {
  const id = window.location.hash.split("/")[1];
  return SECTIONS.find((s) => s.id === id)?.id ?? "general";
}

export function SettingsApp() {
  const [section, setSection] = useState<SectionId>(sectionFromHash);
  const loaded = useSettings((s) => s.settings !== null);

  // Other windows can deep-link to a section (e.g. the island's "Connect Calendar").
  useEffect(() => {
    const onHash = () => setSection(sectionFromHash());
    window.addEventListener("hashchange", onHash);
    return () => window.removeEventListener("hashchange", onHash);
  }, []);

  return (
    <div className="settings">
      <nav className="settings__nav" aria-label="Settings sections">
        <p className="settings__app-name">Notch Desktop</p>
        {SECTIONS.map((s) => (
          <button
            key={s.id}
            type="button"
            className={`settings__nav-item${s.id === section ? " settings__nav-item--active" : ""}`}
            aria-current={s.id === section ? "page" : undefined}
            onClick={() => setSection(s.id)}
          >
            {s.label}
          </button>
        ))}
      </nav>
      <main className="settings__content">
        {!loaded ? null : (
          <>
            {section === "general" && <GeneralSection />}
            {section === "appearance" && <AppearanceSection />}
            {section === "modules" && <ModulesSection />}
            {section === "media" && <MediaSection />}
            {section === "calendar" && <CalendarSection />}
            {section === "git" && <GitSection />}
            {section === "clipboard" && <ClipboardSection />}
            {section === "ai" && <AiSection />}
            {section === "diagnostics" && <DiagnosticsSection />}
          </>
        )}
      </main>
    </div>
  );
}

function GeneralSection() {
  const [launchAtLogin, setLaunchAtLogin] = useState<boolean | null>(null);
  const [failed, setFailed] = useState(false);

  useEffect(() => {
    ipc
      .autostartGet()
      .then(setLaunchAtLogin)
      .catch(() => setFailed(true));
  }, []);

  const toggle = async (enabled: boolean) => {
    try {
      setLaunchAtLogin(await ipc.autostartSet(enabled));
      setFailed(false);
    } catch {
      setFailed(true);
    }
  };

  return (
    <Section title="General">
      <Surface>
        <Switch
          label="Start with Windows"
          description="Open Notch Desktop when you sign in."
          checked={launchAtLogin ?? false}
          disabled={launchAtLogin === null}
          onChange={(v) => void toggle(v)}
        />
        {failed && (
          <p className="settings__note settings__note--warning">
            Couldn't read or change the Windows startup setting. Try again, or check the logs.
          </p>
        )}
      </Surface>
      <ScreensSetting />
      <HudSetting />
      <Surface>
        <div className="settings__row">
          <span className="settings__row-text">
            <span className="settings__row-label">Open or close the notch</span>
            <span className="settings__row-description">Works from any app.</span>
          </span>
          <kbd className="settings__kbd">Ctrl + Alt + N</kbd>
        </div>
        <p className="settings__note">Custom shortcuts aren't available yet.</p>
      </Surface>
    </Section>
  );
}

const ALL_SCREENS = "all";

function ScreensSetting() {
  const display = useSettings((s) => s.settings?.display);
  const update = useSettings((s) => s.update);
  const [screens, setScreens] = useState<ScreenInfo[] | null>(null);

  useEffect(() => {
    const load = () => ipc.shellListScreens().then(setScreens, () => setScreens([]));
    void load();
    // Screens can be plugged in while Settings is open.
    window.addEventListener("focus", load);
    return () => window.removeEventListener("focus", load);
  }, []);

  if (!display || !screens) return null;
  const primary = screens.find((s) => s.primary) ?? screens[0];
  const value =
    display.mode === "all"
      ? ALL_SCREENS
      : display.mode === "one" && screens.some((s) => s.name === display.monitor)
        ? display.monitor!
        : (primary?.name ?? ALL_SCREENS);
  const choose = (choice: string) =>
    void update((s) => ({
      ...s,
      display: choice === ALL_SCREENS ? { mode: "all" } : { mode: "one", monitor: choice },
    }));
  const options = screens.map((s) => ({ value: s.name, label: s.primary ? `${s.label} (main)` : s.label }));
  if (screens.length > 1) options.push({ value: ALL_SCREENS, label: "All screens" });

  return (
    <Surface>
      <div className="settings__row settings__row--stacked">
        <span className="settings__row-text">
          <span className="settings__row-label">Show the notch on</span>
          <span className="settings__row-description">
            {screens.length > 1
              ? "Each extra screen runs its own notch (about 30–40 MB of memory each)."
              : "Connect another screen to choose where the notch appears."}
          </span>
        </span>
        <SegmentedControl<string>
          label="Show the notch on"
          value={value}
          options={options}
          onChange={choose}
        />
      </div>
      {display.mode === "one" && !screens.some((s) => s.name === display.monitor) && (
        <p className="settings__note">
          The chosen screen isn't connected, so the notch is on your main screen for now.
        </p>
      )}
    </Surface>
  );
}

function AppearanceSection() {
  const animation = useSettings((s) => s.settings?.animation ?? "system");
  const update = useSettings((s) => s.update);

  return (
    <Section title="Appearance">
      <Surface>
        <div className="settings__row">
          <span className="settings__row-text">
            <span className="settings__row-label">Animation</span>
            <span className="settings__row-description">
              “Match Windows” follows Settings → Accessibility → Visual effects.
            </span>
          </span>
          <SegmentedControl<AnimationPreference>
            label="Animation"
            value={animation}
            options={[
              { value: "system", label: "Match Windows" },
              { value: "reduced", label: "Reduced" },
            ]}
            onChange={(value) => void update((s) => ({ ...s, animation: value }))}
          />
        </div>
      </Surface>
    </Section>
  );
}

function ModulesSection() {
  const settings = useSettings((s) => s.settings);
  const setModule = useSettings((s) => s.setModule);

  return (
    <Section title="Modules">
      <p className="settings__lead">
        Turn modules off to remove them from the dock and stop their background work.
      </p>
      {MODULES.map((module) => {
        const prefs = moduleSettings(settings, module.id);
        const Icon = module.icon;
        return (
          <Surface key={module.id}>
            <div className="settings__module-head">
              <Icon aria-hidden size={16} />
              <span className="settings__row-label">{module.name}</span>
              {module.devOnly && <MockBadge />}
              {module.alwaysOn && <Pill>Always on</Pill>}
            </div>
            {prefs.autoDisabled && !prefs.enabled && (
              <p className="settings__note settings__note--warning" role="alert">
                Turned off after failing to start three times in a row. Turn it back on to try again.
              </p>
            )}
            <Switch
              label="Enabled"
              checked={module.alwaysOn || prefs.enabled}
              disabled={module.alwaysOn}
              onChange={(enabled) =>
                void setModule(module.id, { enabled, autoDisabled: false, initFailures: 0 })
              }
            />
            <Switch
              label="Can interrupt"
              description="Show its notifications in the notch. Agent approvals always appear."
              checked={prefs.mayPeek}
              disabled={!(module.alwaysOn || prefs.enabled)}
              onChange={(mayPeek) => void setModule(module.id, { mayPeek })}
            />
          </Surface>
        );
      })}
    </Section>
  );
}

function DiagnosticsSection() {
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [exported, setExported] = useState<string | null>(null);
  const [error, setError] = useState(false);

  useEffect(() => {
    ipc.appInfo().then(setInfo, () => setError(true));
  }, []);

  const exportLogs = async () => {
    try {
      setExported(await ipc.diagnosticsExportLogs());
      setError(false);
    } catch {
      setError(true);
    }
  };

  return (
    <Section title="Diagnostics & About">
      <Surface>
        <p className="settings__lead">
          Logs stay on this PC. They record what happened, never message contents, commands, or passwords.
        </p>
        <div className="settings__buttons">
          <Button onClick={() => void ipc.diagnosticsOpenLogDir().catch(() => setError(true))}>
            Open log folder
          </Button>
          <Button onClick={() => void exportLogs()}>Export logs</Button>
        </div>
        {exported && <p className="settings__note">Saved to {exported}</p>}
        {error && (
          <p className="settings__note settings__note--warning">That didn't work. Please try again.</p>
        )}
      </Surface>
      {info && (
        <Surface>
          <dl className="settings__facts">
            <dt>Version</dt>
            <dd>{info.version}</dd>
            <dt>Data folder</dt>
            <dd>{info.dataDir}</dd>
            <dt>Log folder</dt>
            <dd>{info.logDir}</dd>
          </dl>
        </Surface>
      )}
    </Section>
  );
}

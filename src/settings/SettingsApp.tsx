// Settings window (design.md §21): grouped pages with icons, a search box that finds any
// setting, and plain-language rows. Each feature's page lives in its own file.
import {
  Bot,
  Clipboard,
  Gamepad2,
  GitBranch,
  House,
  Info,
  KeyRound,
  Code2,
  AppWindow,
  Keyboard,
  LayoutGrid,
  MousePointerClick,
  Music,
  Palette,
  Search,
  Settings2,
  Shield,
  Timer,
  CalendarDays,
  X,
  type LucideIcon,
} from "lucide-react";
import { useEffect, useState, type ReactNode } from "react";

import type { AnimationPreference } from "../core/generated/AnimationPreference";
import type { ScreenInfo } from "../core/generated/ScreenInfo";
import { ipc } from "../core/ipc";
import { SegmentedControl, Surface, Switch } from "../design/components";
import { useSettings } from ".";
import { AiSection } from "./AiSection";
import { BehaviorSection } from "./BehaviorSection";
import { CalendarSection } from "./CalendarSection";
import { ClipboardSection } from "./ClipboardSection";
import { Row } from "./controls";
import { DeskSection } from "./DeskSection";
import { DeveloperSection } from "./DeveloperSection";
import { DockSection } from "./DockSection";
import { FocusSection } from "./FocusSection";
import { GamesSection } from "./GamesSection";
import { GitSection } from "./GitSection";
import { HomeSection } from "./HomeSection";
import { LookSection } from "./LookSection";
import { MediaSection } from "./MediaSection";
import { AboutSection, PrivacySection } from "./PrivacySection";
import { pageFromId, pageLead, PAGES, searchSettings, settingAnchor, type PageId } from "./searchIndex";
import { Section } from "./Section";
import { ShortcutsSection } from "./ShortcutsSection";
import { VaultSection } from "./VaultSection";
import "./settings.css";

const ICONS: Record<PageId, LucideIcon> = {
  general: Settings2,
  look: Palette,
  behavior: MousePointerClick,
  shortcuts: Keyboard,
  home: House,
  dock: LayoutGrid,
  media: Music,
  calendar: CalendarDays,
  git: GitBranch,
  clipboard: Clipboard,
  focus: Timer,
  games: Gamepad2,
  ai: Bot,
  vault: KeyRound,
  developer: Code2,
  desk: AppWindow,
  privacy: Shield,
  about: Info,
};

const CONTENT: Record<PageId, () => ReactNode> = {
  general: () => <GeneralSection />,
  look: () => <LookSection />,
  behavior: () => <BehaviorSection />,
  shortcuts: () => <ShortcutsSection />,
  home: () => <HomeSection />,
  dock: () => <DockSection />,
  media: () => <MediaSection />,
  calendar: () => <CalendarSection />,
  git: () => <GitSection />,
  clipboard: () => <ClipboardSection />,
  focus: () => <FocusSection />,
  games: () => <GamesSection />,
  ai: () => <AiSection />,
  vault: () => <VaultSection />,
  developer: () => <DeveloperSection />,
  desk: () => <DeskSection />,
  privacy: () => <PrivacySection />,
  about: () => <AboutSection />,
};

const GROUPS = ["Notch", "Features", "More"] as const;

/** `#settings/calendar` → "calendar"; anything unknown falls back to General. */
function pageFromHash(): PageId {
  return pageFromId(window.location.hash.split("/")[1]);
}

export function SettingsApp() {
  const [page, setPage] = useState<PageId>(pageFromHash);
  const [query, setQuery] = useState("");
  const [target, setTarget] = useState<{ label: string; at: number } | null>(null);
  const loaded = useSettings((s) => s.settings !== null);

  // Other windows can deep-link to a page (e.g. the island's "Connect Calendar").
  useEffect(() => {
    const onHash = () => setPage(pageFromHash());
    window.addEventListener("hashchange", onHash);
    return () => window.removeEventListener("hashchange", onHash);
  }, []);

  // After picking a search result, scroll its row into view and flash it.
  useEffect(() => {
    if (!target) return;
    const frame = requestAnimationFrame(() => {
      const el = document.getElementById(settingAnchor(target.label));
      if (!el) return;
      el.scrollIntoView({ block: "center", behavior: "smooth" });
      el.classList.add("settings__flash");
      window.setTimeout(() => el.classList.remove("settings__flash"), 1600);
    });
    return () => cancelAnimationFrame(frame);
  }, [target]);

  const results = searchSettings(query);
  const go = (id: PageId, label?: string) => {
    setPage(id);
    setQuery("");
    // Keep the address in step so a reload stays on this page (no hashchange is fired).
    history.replaceState(null, "", `#settings/${id}`);
    if (label) setTarget({ label, at: performance.now() });
    document.querySelector(".settings__content")?.scrollTo({ top: 0 });
  };

  return (
    <div className="settings">
      <nav className="settings__nav" aria-label="Settings sections">
        <p className="settings__app-name">Notch Desktop</p>
        <label className="settings__search">
          <Search size={14} aria-hidden />
          <input
            type="search"
            placeholder="Search settings"
            aria-label="Search settings"
            value={query}
            onChange={(e) => setQuery(e.currentTarget.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter" && results[0]) go(results[0].page, results[0].label);
              if (e.key === "Escape") setQuery("");
            }}
          />
          {query && (
            <button type="button" aria-label="Clear search" onClick={() => setQuery("")}>
              <X size={13} aria-hidden />
            </button>
          )}
        </label>
        {GROUPS.map((group) => (
          <div key={group} className="settings__nav-group" role="group" aria-label={group}>
            <p className="settings__nav-heading">{group}</p>
            {PAGES.filter((p) => p.group === group).map((p) => {
              const Icon = ICONS[p.id];
              return (
                <button
                  key={p.id}
                  type="button"
                  className={`settings__nav-item${p.id === page && !query ? " settings__nav-item--active" : ""}`}
                  aria-current={p.id === page && !query ? "page" : undefined}
                  onClick={() => go(p.id)}
                >
                  <Icon size={15} aria-hidden />
                  {p.label}
                </button>
              );
            })}
          </div>
        ))}
      </nav>
      <main className="settings__content">
        {!loaded ? null : query.trim() ? (
          <section className="settings__section" aria-label="Search results">
            <h1 className="settings__title">Results for “{query.trim()}”</h1>
            {results.length === 0 ? (
              <p className="settings__lead">
                No settings match. Try a different word, like “color”, “sound” or “shortcut”.
              </p>
            ) : (
              <ul className="settings-results">
                {results.map((r) => {
                  const Icon = ICONS[r.page];
                  return (
                    <li key={`${r.page}-${r.label}`}>
                      <button
                        type="button"
                        className="settings-results__item"
                        onClick={() => go(r.page, r.label)}
                      >
                        <Icon size={15} aria-hidden />
                        <span className="settings-results__label">{r.label}</span>
                        <span className="settings-results__page">{r.pageLabel}</span>
                      </button>
                    </li>
                  );
                })}
              </ul>
            )}
          </section>
        ) : (
          CONTENT[page]()
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
    <Section title="General" lead={pageLead("general")}>
      <Surface>
        <span id={settingAnchor("Start with Windows")} />
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
      <AnimationSetting />
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
      <div id={settingAnchor("Show the notch on")} className="settings__row settings__row--stacked">
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

function AnimationSetting() {
  const animation = useSettings((s) => s.settings?.animation ?? "system");
  const update = useSettings((s) => s.update);
  return (
    <Surface>
      <Row
        label="Animation"
        description="“Match Windows” follows Settings → Accessibility → Visual effects. Reduced turns off bouncing and the music pulse."
      >
        <SegmentedControl<AnimationPreference>
          label="Animation"
          value={animation}
          options={[
            { value: "system", label: "Match Windows" },
            { value: "reduced", label: "Reduced" },
          ]}
          onChange={(value) => void update((s) => ({ ...s, animation: value }))}
        />
      </Row>
    </Surface>
  );
}

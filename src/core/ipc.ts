// Typed wrappers over Tauri commands. Components call these, never `invoke` directly,
// so every backend call is discoverable in one place (plan.md Rule 4).
import { invoke } from "@tauri-apps/api/core";

import type { AppInfo } from "./generated/AppInfo";
import type { AppTraffic } from "./generated/AppTraffic";
import type { CaptureGuess } from "./generated/CaptureGuess";
import type { InviteResponse } from "./generated/InviteResponse";
import type { NewEvent } from "./generated/NewEvent";
import type { QuickAdded } from "./generated/QuickAdded";
import type { ReadItem } from "./generated/ReadItem";
import type { ReadLaterSnapshot } from "./generated/ReadLaterSnapshot";
import type { SpeedTest } from "./generated/SpeedTest";
import type { ClipboardSnapshot } from "./generated/ClipboardSnapshot";
import type { ShelfSnapshot } from "./generated/ShelfSnapshot";
import type { WallpaperState } from "./generated/WallpaperState";
import type { Mixer } from "./generated/Mixer";
import type { WifiNetwork } from "./generated/WifiNetwork";
import type { DisplayToggles } from "./generated/DisplayToggles";
import type { SpotifyNow } from "./generated/SpotifyNow";
import type { PriceItem } from "./generated/PriceItem";
import type { RecentDownload } from "./generated/RecentDownload";
import type { HistoryEntry } from "./generated/HistoryEntry";
import type { MeetingState } from "./generated/MeetingState";
import type { PcAnswer } from "./generated/PcAnswer";
import type { RewriteSession } from "./generated/RewriteSession";
import type { ScreenRecState } from "./generated/ScreenRecState";
import type { PowerAction } from "./generated/PowerAction";
import type { InstalledApp } from "./generated/InstalledApp";
import type { SystemMonitor } from "./generated/SystemMonitor";
import type { DiskHealth } from "./generated/DiskHealth";
import type { DataUsage } from "./generated/DataUsage";
import type { SessionUsage } from "./generated/SessionUsage";
import type { AskSnapshot } from "./generated/AskSnapshot";
import type { BridgeState } from "./generated/BridgeState";
import type { Dashboard } from "./generated/Dashboard";
import type { Agent } from "./generated/Agent";
import type { ScreenTime } from "./generated/ScreenTime";
import type { CodexUsage } from "./generated/CodexUsage";
import type { RgbStatus } from "./generated/RgbStatus";
import type { ControlSnapshot } from "./generated/ControlSnapshot";
import type { DownloadProgress } from "./generated/DownloadProgress";
import type { CodingStats } from "./generated/CodingStats";
import type { DevServer } from "./generated/DevServer";
import type { DayPlan } from "./generated/DayPlan";
import type { DayState } from "./generated/DayState";
import type { DeskWindow } from "./generated/DeskWindow";
import type { Workspace } from "./generated/Workspace";
import type { Zone } from "./generated/Zone";
import type { GameStatus } from "./generated/GameStatus";
import type { MailState } from "./generated/MailState";
import type { Language } from "./generated/Language";
import type { RunResult } from "./generated/RunResult";
import type { WidgetOutput } from "./generated/WidgetOutput";
import type { Snippet } from "./generated/Snippet";
import type { TotpCode } from "./generated/TotpCode";
import type { VaultField } from "./generated/VaultField";
import type { VaultInput } from "./generated/VaultInput";
import type { VaultState } from "./generated/VaultState";
import type { ImageTarget } from "./generated/ImageTarget";
import type { RecentFile } from "./generated/RecentFile";
import type { RecorderState } from "./generated/RecorderState";
import type { NetSpeed } from "./generated/NetSpeed";
import type { RadioKind } from "./generated/RadioKind";
import type { ShortcutProblem } from "./generated/ShortcutProblem";
import type { FocusState } from "./generated/FocusState";
import type { Lyrics } from "./generated/Lyrics";
import type { Palette } from "./generated/Palette";
import type { SoundState } from "./generated/SoundState";
import type { SystemStats } from "./generated/SystemStats";
import type { DeviceCode } from "./generated/DeviceCode";
import type { GithubRepo } from "./generated/GithubRepo";
import type { GithubRepoOverview } from "./generated/GithubRepoOverview";
import type { GithubNotification } from "./generated/GithubNotification";
import type { GithubStatus } from "./generated/GithubStatus";
import type { RepoInfo as GithubLocalRepo } from "./generated/RepoInfo";
import type { SignInOptions } from "./generated/SignInOptions";
import type { ApprovalChoice } from "./generated/ApprovalChoice";
import type { ApprovalRequest } from "./generated/ApprovalRequest";
import type { AuditEntry } from "./generated/AuditEntry";
import type { CalendarSnapshot } from "./generated/CalendarSnapshot";
import type { ClaudeStatus } from "./generated/ClaudeStatus";
import type { DevTestActivity } from "./generated/DevTestActivity";
import type { GitOperation } from "./generated/GitOperation";
import type { GitSnapshot } from "./generated/GitSnapshot";
import type { PullRequests } from "./generated/PullRequests";
import type { Note } from "./generated/Note";
import type { PhoneSnapshot } from "./generated/PhoneSnapshot";
import type { ScreenInfo } from "./generated/ScreenInfo";
import type { Task } from "./generated/Task";
import type { TodoSnapshot } from "./generated/TodoSnapshot";
import type { RepoInfo } from "./generated/RepoInfo";
import type { MediaCommand } from "./generated/MediaCommand";
import type { MediaState } from "./generated/MediaState";
import type { PlayOutcome } from "./generated/PlayOutcome";
import type { Settings } from "./generated/Settings";
import type { SpotifySnapshot } from "./generated/SpotifySnapshot";
import type { UsageRange } from "./generated/UsageRange";
import type { UsageSummary } from "./generated/UsageSummary";

/** Shape of `AppError` as serialized by the backend. */
export interface IpcError {
  kind: string;
  message: string;
}

export function isIpcError(value: unknown): value is IpcError {
  return typeof value === "object" && value !== null && "kind" in value && "message" in value;
}

export const ipc = {
  appInfo: () => invoke<AppInfo>("app_info"),
  diagnosticsPing: (nonce: string) => invoke<void>("diagnostics_ping", { nonce }),
  /** Resize the island window (logical px); the backend keeps it pinned top-center. */
  /** Debug builds only: fire a fake activity through the real backend event pipeline. */
  devEmitTestActivity: (activity: DevTestActivity, delayMs?: number) =>
    invoke<void>("dev_emit_test_activity", { activity, delayMs }),
  settingsGet: () => invoke<Settings>("settings_get"),
  settingsUpdate: (settings: Settings) => invoke<Settings>("settings_update", { settings }),
  /** Defaults, for "Reset this page". */
  settingsDefaults: () => invoke<Settings>("settings_defaults"),
  /** Shortcuts that couldn't be used (not a key combination, or another app has them). */
  shortcutProblems: () => invoke<ShortcutProblem[]>("shortcut_problems"),
  /** Save settings to a file the user picks; null if cancelled. */
  settingsExport: () => invoke<string | null>("settings_export"),
  /** Load settings from a file the user picks; null if cancelled. */
  settingsImport: () => invoke<Settings | null>("settings_import"),
  settingsRecordModuleInit: (moduleId: string, ok: boolean) =>
    invoke<Settings>("settings_record_module_init", { moduleId, ok }),
  autostartGet: () => invoke<boolean>("autostart_get"),
  autostartSet: (enabled: boolean) => invoke<boolean>("autostart_set", { enabled }),
  /** Connected screens, for "Show the notch on". */
  shellListScreens: () => invoke<ScreenInfo[]>("shell_list_screens"),
  /** Whether a fullscreen app is in front on this island's screen; `FullscreenChanged` reports changes. */
  shellIsFullscreen: () => invoke<boolean>("shell_is_fullscreen"),
  /** Hide this island while a fullscreen app is in front; `false` brings it back. */
  shellSetFullscreenHidden: (hidden: boolean) => invoke<void>("shell_set_fullscreen_hidden", { hidden }),
  /** Open (or focus) the settings window, optionally at a section such as "calendar". */
  openSettingsWindow: (section?: string) => invoke<void>("open_settings_window", { section }),
  diagnosticsOpenLogDir: () => invoke<void>("diagnostics_open_log_dir"),
  /** Copies logs into a new folder under Downloads, reveals it, and returns its path. */
  diagnosticsExportLogs: () => invoke<string>("diagnostics_export_logs"),
  mediaGetState: () => invoke<MediaState | null>("media_get_state"),
  mediaCommand: (command: MediaCommand) => invoke<void>("media_command", { command }),
  calendarGetSnapshot: () => invoke<CalendarSnapshot>("calendar_get_snapshot"),
  /** Opens Google sign-in in the browser; resolves when sign-in finishes. Empty secret = keep saved one. */
  calendarConnect: (clientId: string, clientSecret: string) =>
    invoke<void>("calendar_connect", { clientId, clientSecret }),
  calendarDisconnect: () => invoke<void>("calendar_disconnect"),
  calendarSyncNow: () => invoke<void>("calendar_sync_now"),
  /** Open an https meeting or event link in the default browser. */
  calendarOpenLink: (url: string) => invoke<void>("calendar_open_link", { url }),
  /** Needs "Add events and answer invites" on and Google's OK. */
  calendarCreateEvent: (event: NewEvent) => invoke<void>("calendar_create_event", { event }),
  /** Google reads a sentence ("Lunch with Ann tomorrow 1pm") into an event. */
  calendarQuickAdd: (text: string) => invoke<QuickAdded>("calendar_quick_add", { text }),
  calendarRespond: (eventId: string, answer: InviteResponse) =>
    invoke<void>("calendar_respond", { eventId, answer }),
  spotifyGetSnapshot: () => invoke<SpotifySnapshot>("spotify_get_snapshot"),
  /** Opens Spotify sign-in in the browser; resolves when sign-in finishes. */
  spotifyConnect: (clientId: string) => invoke<void>("spotify_connect", { clientId }),
  spotifyDisconnect: () => invoke<void>("spotify_disconnect"),
  spotifyRefreshPlaylists: () => invoke<void>("spotify_refresh_playlists"),
  spotifyPlayPlaylist: (playlistId: string) => invoke<PlayOutcome>("spotify_play_playlist", { playlistId }),
  spotifySetPinned: (playlistIds: string[]) => invoke<void>("spotify_set_pinned", { playlistIds }),
  /** Open an https link (e.g. a pull request) in the default browser. */
  openExternal: (url: string) => invoke<void>("open_external", { url }),
  clipboardSnapshot: () => invoke<ClipboardSnapshot>("clipboard_snapshot"),
  /** Copy a clip again; `formatted` indents JSON first. */
  clipboardCopy: (id: string, formatted = false) => invoke<void>("clipboard_copy", { id, formatted }),
  /** A clip's full text (the list carries only the start). */
  clipboardText: (id: string) => invoke<string>("clipboard_text", { id }),
  clipboardPin: (id: string) => invoke<void>("clipboard_pin", { id }),
  clipboardUnpin: (id: string) => invoke<void>("clipboard_unpin", { id }),
  clipboardDelete: (id: string) => invoke<void>("clipboard_delete", { id }),
  /** Empty history; pinned clips stay. */
  clipboardClear: () => invoke<void>("clipboard_clear"),
  askSnapshot: () => invoke<AskSnapshot>("ask_snapshot"),
  /** Ask Claude Code; the answer streams in as AskDelta events. */
  askQuestion: (question: string, includeScreen: boolean) =>
    invoke<string>("ask_question", { question, includeScreen }),
  askStop: () => invoke<void>("ask_stop"),
  askClear: () => invoke<void>("ask_clear"),
  controlSnapshot: () => invoke<ControlSnapshot>("control_snapshot"),
  /** Read radios, battery and accessories again now. */
  controlRefresh: () => invoke<ControlSnapshot>("control_refresh"),
  /** Bytes per second since the last call (call about once a second while shown). */
  controlSpeed: () => invoke<NetSpeed>("control_speed"),
  controlNetApps: () => invoke<AppTraffic[]>("control_net_apps"),
  controlSpeedTestLast: () => invoke<SpeedTest | null>("control_speed_test_last"),
  /** About 20 seconds against speed.cloudflare.com; progress arrives as SpeedTestChanged. */
  controlSpeedTest: () => invoke<SpeedTest>("control_speed_test"),
  controlSetRadio: (kind: RadioKind, on: boolean) => invoke<void>("control_set_radio", { kind, on }),
  /** Open a Windows Settings page: nightlight, airplane, wifi, bluetooth, hotspot, battery, radios. */
  controlOpenSettings: (page: string) => invoke<void>("control_open_settings", { page }),
  filesDownloads: () => invoke<DownloadProgress[]>("files_downloads"),
  /** Downloads finished in the last day, newest first. */
  filesRecentDownloads: () => invoke<RecentDownload[]>("files_recent_downloads"),
  /** Files opened recently in any app. */
  filesRecent: () => invoke<RecentFile[]>("files_recent"),
  /** Save a copy as PNG/JPG/WebP (it goes on the shelf); returns its path. */
  filesConvertImage: (path: string, to: ImageTarget) => invoke<string>("files_convert_image", { path, to }),
  /** Save a smaller copy for sharing (it goes on the shelf). */
  filesShrinkImage: (path: string) => invoke<string>("files_shrink_image", { path }),
  /** Join PDFs in this order (the result goes on the shelf). */
  filesMergePdfs: (paths: string[]) => invoke<string>("files_merge_pdfs", { paths }),
  filesOpen: (path: string) => invoke<void>("files_open", { path }),
  filesReveal: (path: string) => invoke<void>("files_reveal", { path }),
  /** Drag files out by path; resolves when dropped (true) or cancelled. */
  filesDrag: (paths: string[]) => invoke<boolean>("files_drag", { paths }),
  recorderState: () => invoke<RecorderState>("recorder_state"),
  /** Record what the PC plays. */
  recorderStart: () => invoke<RecorderState>("recorder_start"),
  /** Stop and save (MP3, on the shelf); returns the file's path. */
  recorderStop: () => invoke<string | null>("recorder_stop"),
  vaultState: () => invoke<VaultState>("vault_state"),
  /** Pick the Obsidian vault folder; null if cancelled. */
  vaultChooseFolder: () => invoke<VaultState | null>("vault_choose_folder"),
  /** Add or change a login (password: undefined keeps the saved one). Returns its id. */
  vaultSave: (entry: VaultInput) => invoke<string>("vault_save", { entry }),
  /** Move a login's note to the Recycle Bin. */
  vaultDelete: (id: string) => invoke<void>("vault_delete", { id }),
  /** Copy a username, or a password (private, cleared after a while; Windows Hello if on). */
  vaultCopy: (id: string, field: VaultField) => invoke<void>("vault_copy", { id, field }),
  vaultReveal: (id: string) => invoke<string>("vault_reveal", { id }),
  vaultGeneratePassword: (length: number) => invoke<string>("vault_generate_password", { length }),
  vaultExportKey: () => invoke<string>("vault_export_key"),
  vaultImportKey: (key: string) => invoke<void>("vault_import_key", { key }),
  totpCodes: () => invoke<TotpCode[]>("totp_codes"),
  /** From an otpauth:// link, or a secret key plus a name. */
  totpAdd: (input: string, name: string) => invoke<void>("totp_add", { input, name }),
  totpDelete: (id: string) => invoke<void>("totp_delete", { id }),
  totpCopy: (id: string) => invoke<void>("totp_copy", { id }),
  snippetsList: () => invoke<Snippet[]>("snippets_list"),
  snippetSave: (id: string | null, trigger: string, body: string) =>
    invoke<Snippet[]>("snippet_save", { id, trigger, body }),
  snippetDelete: (id: string) => invoke<Snippet[]>("snippet_delete", { id }),
  /** Freeze the screen under the pointer and draw a box to copy its text. */
  grabBegin: () => invoke<void>("grab_begin"),
  grabPicture: () => invoke<string>("grab_picture"),
  grabFinish: (x: number, y: number, w: number, h: number) => invoke<string>("grab_finish", { x, y, w, h }),
  grabCancel: () => invoke<void>("grab_cancel"),
  qrSvg: (text: string) => invoke<string>("qr_svg", { text }),
  /** Local dev servers, looked up now. */
  devServers: () => invoke<DevServer[]>("dev_servers"),
  devStopServer: (pid: number) => invoke<void>("dev_stop_server", { pid }),
  codingStats: () => invoke<CodingStats>("coding_stats"),
  /** Run a few lines of JavaScript (Node) or Python. */
  devRun: (language: Language, code: string) => invoke<RunResult>("dev_run", { language, code }),
  devWidgets: () => invoke<WidgetOutput[]>("dev_widgets"),
  /** A commit message for the current changes, written by Claude Code. */
  gitSuggestMessage: (includeUntracked: boolean) =>
    invoke<string>("git_suggest_message", { includeUntracked }),
  mailState: () => invoke<MailState>("mail_state"),
  mailRefresh: () => invoke<void>("mail_refresh"),
  /** Take a message out of the inbox (it stays in All Mail). */
  mailArchive: (id: string) => invoke<void>("mail_archive", { id }),
  mailMarkRead: (id: string) => invoke<void>("mail_mark_read", { id }),
  /** Hide a message until `until` (ISO time); it comes back with a card. */
  mailSnooze: (id: string, until: string) => invoke<void>("mail_snooze", { id, until }),
  dayState: () => invoke<DayState>("day_state"),
  /** Plan the rest of today now (Claude Code). */
  dayPlan: () => invoke<DayPlan>("day_plan"),
  dayRefresh: () => invoke<void>("day_refresh"),
  deskWindows: () => invoke<DeskWindow[]>("desk_windows"),
  deskSetOnTop: (id: number, on: boolean) => invoke<void>("desk_set_on_top", { id, on }),
  /** 20–100; 100 makes the window solid again. */
  deskSetOpacity: (id: number, percent: number) => invoke<void>("desk_set_opacity", { id, percent }),
  deskFocus: (id: number) => invoke<void>("desk_focus", { id }),
  /** Draw a live view of a window into this notch at a physical-pixel rectangle. */
  deskMiniShow: (id: number, left: number, top: number, right: number, bottom: number) =>
    invoke<void>("desk_mini_show", { id, left, top, right, bottom }),
  deskMiniHide: () => invoke<void>("desk_mini_hide"),
  /** The window switcher's live views (physical pixels in this island). */
  deskGridShow: (tiles: { id: number; left: number; top: number; right: number; bottom: number }[]) =>
    invoke<void>("desk_grid_show", { tiles }),
  deskGridHide: () => invoke<void>("desk_grid_hide"),
  /** Snap a window into a zone of its screen (fractions of the work area). */
  deskSnap: (id: number, zone: Zone) => invoke<void>("desk_snap", { id, zone }),
  workspaceList: () => invoke<Workspace[]>("workspace_list"),
  /** Show a window live in a small always-on-top picture-in-picture window. */
  pipOpen: (id: number) => invoke<void>("pip_open", { id }),
  /** Save every window's place under a name; returns how many windows. */
  workspaceSave: (name: string) => invoke<number>("workspace_save", { name }),
  /** Put a workspace back: [windows placed, programs started]. */
  workspaceRestore: (name: string) => invoke<[number, number]>("workspace_restore", { name }),
  workspaceDelete: (name: string) => invoke<void>("workspace_delete", { name }),
  /** After a restart: apps from last session that aren't open (once per run). */
  workspaceResumeOffer: () => invoke<string[]>("workspace_resume_offer"),
  routineRun: (id: string) => invoke<number>("routine_run", { id }),
  routinePickApp: () => invoke<string | null>("routine_pick_app"),
  routinePickFolder: () => invoke<string | null>("routine_pick_folder"),
  gameStatus: () => invoke<GameStatus>("game_status"),
  /** Remember whether a fullscreen app is a game. */
  gameMark: (exe: string, isGame: boolean) => invoke<void>("game_mark", { exe, isGame }),
  /** Download and verify PresentMon so the game HUD can show FPS. */
  gameSetupFps: () => invoke<void>("game_setup_fps"),
  focusState: () => invoke<FocusState>("focus_state"),
  /** Start a focus session; non-urgent cards wait until it ends. */
  focusStart: (minutes: number) => invoke<void>("focus_start", { minutes }),
  focusStop: () => invoke<void>("focus_stop"),
  /** Milliseconds since the last keyboard or mouse input. */
  systemIdleMs: () => invoke<number>("system_idle_ms"),
  /** Screen time on a past day (YYYY-MM-DD). */
  screenTimeDay: (date: string) => invoke<ScreenTime>("screen_time_day", { date }),
  /** Call the pet out of the notch: it walks the taskbar until it goes back. */
  petBuddyShow: () => invoke<void>("pet_buddy_show"),
  /** Send the pet back: off the taskbar and into the notch. */
  petBuddyHide: () => invoke<void>("pet_buddy_hide"),
  /** The pet is out walking the taskbar. */
  petWalking: () => invoke<boolean>("pet_walking"),
  /** A code editor or terminal is in front: on the pet's own screen, on another one, or not at all. */
  petCodingInFront: () => invoke<"here" | "elsewhere" | "none">("pet_coding_in_front"),
  /** Where the app that's playing is, seen from the pet: on its screen, only on another one, or unknown. */
  petMediaPlace: () => invoke<"here" | "elsewhere" | "unknown">("pet_media_place"),
  rgbMusicColor: (hex: string | null) => invoke<void>("rgb_music_color", { hex }),
  rgbStatus: () => invoke<RgbStatus>("rgb_status"),
  soundState: () => invoke<SoundState | null>("sound_state"),
  /** Synced lyrics for the song playing now, if found. */
  lyricsCurrent: () => invoke<Lyrics | null>("lyrics_current"),
  /** Two colors from a cached album-art file. */
  artworkPalette: (path: string) => invoke<Palette | null>("artwork_palette", { path }),
  /** Speaker volume, 0 to 1 (unmutes). */
  soundSetVolume: (level: number) => invoke<void>("sound_set_volume", { level }),
  soundToggleMute: () => invoke<void>("sound_toggle_mute"),
  /** Everything the Home dashboard shows. */
  systemDashboard: () => invoke<Dashboard>("system_dashboard"),
  /** CPU, memory and battery right now. */
  systemStats: () => invoke<SystemStats>("system_stats"),
  /** Laptop screen brightness, 0 to 100. */
  brightnessSet: (level: number) => invoke<void>("brightness_set", { level }),
  /** Switch Windows virtual desktop (0-based). */
  desktopSwitch: (index: number) => invoke<void>("desktop_switch", { index }),
  weatherRefresh: () => invoke<void>("weather_refresh"),
  openLocationSettings: () => invoke<void>("open_location_settings"),
  bridgeState: () => invoke<BridgeState | null>("bridge_state"),
  /** Offer files to the phone over Wi-Fi (none: just receive); shows a QR code. */
  bridgeStart: (paths: string[]) => invoke<BridgeState>("bridge_start", { paths }),
  /** Copy the text in a picture to the clipboard; returns the text. */
  /** Cover every screen with a blur, or uncover. */
  blurToggle: () => invoke<void>("blur_toggle"),
  /** Ask about the window that was in front, with its picture; the answer streams into Ask. */
  askExplainScreen: () => invoke<string>("ask_explain_screen"),
  /** Ask about your own files, notes (and Gmail / calendar when allowed for AI). */
  askPc: (question: string) => invoke<PcAnswer>("ask_pc", { question }),
  /** Research a topic in the background (a ResearchDone event follows). */
  researchStart: (topic: string) => invoke<void>("research_start", { topic }),
  meetingState: () => invoke<MeetingState>("meeting_state"),
  /** Remember a card the notch showed (private modules keep only the title). */
  historyAdd: (entry: HistoryEntry) => invoke<void>("history_add", { entry }),
  /** Cards from the last hours (at most 48), newest first. */
  historyList: (hours: number) => invoke<HistoryEntry[]>("history_list", { hours }),
  historyClear: () => invoke<void>("history_clear"),
  priceList: () => invoke<PriceItem[]>("price_list"),
  /** Wi-Fi networks nearby (connected, then known, then strongest). */
  wifiNetworks: () => invoke<WifiNetwork[]>("wifi_networks"),
  wifiScan: () => invoke<void>("wifi_scan"),
  /** Join a network this PC already knows. */
  wifiConnect: (ssid: string) => invoke<void>("wifi_connect", { ssid }),
  /** Windows' own Wi-Fi list (new networks, passwords). */
  wifiOpenSettings: () => invoke<void>("wifi_open_settings"),
  displayToggles: () => invoke<DisplayToggles>("display_toggles"),
  /** True if flipped here; false if Windows' Night light page was opened instead. */
  setNightLight: (on: boolean) => invoke<boolean>("set_night_light", { on }),
  setHdr: (on: boolean) => invoke<void>("set_hdr", { on }),
  setRefreshRate: (hz: number) => invoke<void>("set_refresh_rate", { hz }),
  openTouchpadSettings: () => invoke<void>("open_touchpad_settings"),
  /** The song playing on Spotify, whether it's liked, and what's up next. */
  spotifyNow: () => invoke<SpotifyNow>("spotify_now"),
  spotifyLike: (trackId: string, liked: boolean) => invoke<void>("spotify_like", { trackId, liked }),
  /** Pin a note to the desktop as a sticky. */
  stickyPin: (noteId: string) => invoke<void>("sticky_pin", { noteId }),
  /** Don't offer this habit routine again. */
  habitDismiss: (key: string) => invoke<void>("habit_dismiss", { key }),
  /** Watch a product page against a target price (checked right away). */
  priceAdd: (url: string, target: number) => invoke<PriceItem>("price_add", { url, target }),
  priceCheck: (id: string) => invoke<PriceItem>("price_check", { id }),
  priceSetTarget: (id: string, target: number) => invoke<void>("price_set_target", { id, target }),
  priceRemove: (id: string) => invoke<void>("price_remove", { id }),
  /** Download Whisper and its speech model (~150 MB); progress comes as MeetingChanged. */
  meetingInstall: () => invoke<void>("meeting_install"),
  /** Start recording the meeting (mic + the PC's sound). */
  meetingStart: () => invoke<MeetingState>("meeting_start"),
  /** Stop; notes come back as a MeetingDone event. */
  meetingStop: () => invoke<void>("meeting_stop"),
  meetingCancel: () => invoke<void>("meeting_cancel"),
  /** The selection waiting to be rewritten (Ctrl+Alt+R). */
  rewriteSession: () => invoke<RewriteSession | null>("rewrite_session"),
  /** The AI's version of the selection in a style ("custom" uses the instruction). */
  rewriteRun: (style: string, custom: string) => invoke<string>("rewrite_run", { style, custom }),
  /** Paste this where the selection was. */
  rewriteApply: (text: string) => invoke<void>("rewrite_apply", { text }),
  /** Record the screen under the pointer: MP4, or a short GIF. */
  screenrecStart: (gif: boolean) => invoke<ScreenRecState>("screenrec_start", { gif }),
  /** Stop and save the screen recording (it goes on the shelf); the path, if any. */
  screenrecStop: () => invoke<string | null>("screenrec_stop"),
  screenrecState: () => invoke<ScreenRecState>("screenrec_state"),
  /** Draw these lines onto the lock-screen picture (rate-limited); true if it was updated. */
  lockscreenUpdate: (lines: string[]) => invoke<boolean>("lockscreen_update", { lines }),
  /** Put the old lock-screen picture back; false = it was Spotlight, settings were opened. */
  lockscreenOff: () => invoke<boolean>("lockscreen_off"),
  screenshotText: (path: string) => invoke<string>("screenshot_text", { path }),
  /** Open a picture in Paint to mark it up. */
  screenshotAnnotate: (path: string) => invoke<void>("screenshot_annotate", { path }),
  bridgeStop: () => invoke<void>("bridge_stop"),
  shelfSnapshot: () => invoke<ShelfSnapshot>("shelf_snapshot"),
  /** Park files on the drop shelf; resolves to how many were new. */
  shelfAdd: (paths: string[]) => invoke<number>("shelf_add", { paths }),
  readLaterSnapshot: () => invoke<ReadLaterSnapshot>("read_later_snapshot"),
  readLaterAdd: (url: string) => invoke<ReadItem>("read_later_add", { url }),
  readLaterSetRead: (id: string, read: boolean) => invoke<void>("read_later_set_read", { id, read }),
  readLaterRemove: (id: string) => invoke<void>("read_later_remove", { id }),
  readLaterClearRead: () => invoke<void>("read_later_clear_read"),
  /** Sends the article's text to the AI chosen in Settings → AI. */
  readLaterSummarize: (id: string) => invoke<string>("read_later_summarize", { id }),
  /** Quick capture: the AI chosen in Settings → AI says what a line is. */
  captureSort: (text: string) => invoke<CaptureGuess>("capture_sort", { text }),
  shelfRemove: (ids: string[]) => invoke<void>("shelf_remove", { ids }),
  shelfClear: () => invoke<void>("shelf_clear"),
  /** Copy shelf files to the clipboard (no ids: all). */
  shelfCopy: (ids: string[] = []) => invoke<void>("shelf_copy", { ids }),
  shelfReveal: (id: string) => invoke<void>("shelf_reveal", { id }),
  /** Drag shelf files out of the notch (no ids: all); true if dropped somewhere. */
  shelfDrag: (ids: string[] = []) => invoke<boolean>("shelf_drag", { ids }),
  /** Keep a shelf file past the clean-up (or let it go again). */
  shelfPin: (id: string, pinned: boolean) => invoke<void>("shelf_pin", { id, pinned }),
  /** Paste stack on/off: copies queue up for the paste-next shortcut. */
  clipboardStack: (on: boolean) => invoke<void>("clipboard_stack", { on }),
  systemLock: () => invoke<void>("system_lock"),
  /** Sleep, hibernate, restart (or into BIOS), shut down or sign out. Confirm first. */
  systemPower: (action: PowerAction) => invoke<void>("system_power", { action }),
  /** Every Start menu app (cached for an hour). */
  appsList: () => invoke<InstalledApp[]>("apps_list"),
  appLaunch: (id: string) => invoke<void>("app_launch", { id }),
  /** Ask an app's windows to close, like clicking X; returns how many. */
  appClose: (exe: string) => invoke<number>("app_close", { exe }),
  /** CPU, memory and GPU load with the top apps (CPU needs two readings a few seconds apart). */
  systemMonitor: () => invoke<SystemMonitor>("system_monitor"),
  /** Drive health and free space (cached ten minutes unless `fresh`). */
  diskHealth: (fresh: boolean) => invoke<DiskHealth>("disk_health", { fresh }),
  /** Data used today and this month. */
  controlDataUsage: () => invoke<DataUsage>("control_data_usage"),
  /** Folders Claude Code has worked in, newest first. */
  claudeProjects: () => invoke<string[]>("claude_projects"),
  /** Open a terminal running Claude Code on a task in a project folder. */
  claudeLaunch: (cwd: string, prompt: string) => invoke<void>("claude_launch", { cwd, prompt }),
  /** Each app's volume on the default output, and the output devices. */
  audioMixer: () => invoke<Mixer>("audio_mixer"),
  audioSetAppVolume: (pid: number, level: number) => invoke<void>("audio_set_app_volume", { pid, level }),
  audioSetAppMute: (pid: number, muted: boolean) => invoke<void>("audio_set_app_mute", { pid, muted }),
  /** Send sound to another output device. */
  audioSetOutput: (id: string) => invoke<void>("audio_set_output", { id }),
  /** Tokens and estimated cost per Claude session today. */
  claudeSessionUsage: () => invoke<SessionUsage[]>("claude_session_usage"),
  /** Bring a Claude session's terminal to the front. */
  claudeFocusSession: (sessionId: string) => invoke<void>("claude_focus_session", { sessionId }),
  /** Copy text privately (not kept in history) and clear it after a while. */
  clipboardCopyPrivate: (text: string, clearSecs: number) =>
    invoke<void>("clipboard_copy_private", { text, clearSecs }),
  /** Two colours from the main screen's wallpaper (for "Accent from wallpaper"). */
  wallpaperAccent: () => invoke<Palette | null>("wallpaper_accent"),
  wallpaperState: () => invoke<WallpaperState>("wallpaper_state"),
  /** Save copies of pictures as wallpapers; resolves to how many were new. */
  wallpaperAdd: (paths: string[]) => invoke<number>("wallpaper_add", { paths }),
  /** Choose pictures to save; resolves to how many were new. */
  wallpaperPick: () => invoke<number>("wallpaper_pick"),
  wallpaperRemove: (path: string) => invoke<void>("wallpaper_remove", { path }),
  /** Choose the "My folder" folder; false if cancelled. */
  wallpaperPickFolder: () => invoke<boolean>("wallpaper_pick_folder"),
  wallpaperSetFolder: (folder: string | null) => invoke<void>("wallpaper_set_folder", { folder }),
  /** Which screen a click changes: "all" or a screen id. */
  wallpaperSetTarget: (target: string) => invoke<void>("wallpaper_set_target", { target }),
  wallpaperClearRecent: () => invoke<void>("wallpaper_clear_recent"),
  wallpaperApply: (path: string, target: string) => invoke<void>("wallpaper_apply", { path, target }),
  gitGetSnapshot: () => invoke<GitSnapshot>("git_get_snapshot"),
  gitRefresh: () => invoke<void>("git_refresh"),
  /** Folder picker; resolves `null` if cancelled. */
  gitPickRepo: () => invoke<RepoInfo | null>("git_pick_repo"),
  gitSelectRepo: (path: string) => invoke<RepoInfo>("git_select_repo", { path }),
  gitForgetRepo: (path: string) => invoke<void>("git_forget_repo", { path }),
  /** Commit / push / pull. Call only after the user has confirmed. */
  gitRun: (operation: GitOperation, message?: string, includeUntracked?: boolean) =>
    invoke<void>("git_run", { operation, message, includeUntracked }),
  gitPullRequests: () => invoke<PullRequests>("git_pull_requests"),
  gitOpenFolder: () => invoke<void>("git_open_folder"),
  claudeStatus: () => invoke<ClaudeStatus>("claude_status"),
  /** Local usage estimate for a range, from Claude Code transcripts on this PC. */
  claudeUsage: (range: UsageRange) => invoke<UsageSummary>("claude_usage", { range }),
  codexUsage: (range: UsageRange) => invoke<CodexUsage>("codex_usage", { range }),
  /** What connecting will add to Claude Code's settings (token masked). */
  claudeHooksPreview: () => invoke<string>("claude_hooks_preview"),
  claudeConnect: () => invoke<void>("claude_connect"),
  claudeDisconnect: () => invoke<void>("claude_disconnect"),
  /** Codex or Gemini CLI: what connecting adds, connect, disconnect. */
  agentPreview: (agent: Agent) => invoke<string>("agent_preview", { agent }),
  /** Settings → AI → Assistant. The API key goes straight to Credential Manager. */
  aiKeySaved: () => invoke<boolean>("ai_key_saved"),
  aiSetKey: (key: string) => invoke<void>("ai_set_key", { key }),
  aiClearKey: () => invoke<void>("ai_clear_key"),
  aiTest: () => invoke<string>("ai_test"),
  aiOllamaModels: () => invoke<string[]>("ai_ollama_models"),
  agentConnect: (agent: Agent) => invoke<void>("agent_connect", { agent }),
  agentDisconnect: (agent: Agent) => invoke<void>("agent_disconnect", { agent }),
  /** Exactly what enabling plan limits adds to Claude Code's settings (token masked). */
  claudeStatusLinePreview: () => invoke<string>("claude_status_line_preview"),
  claudeStatusLineEnable: () => invoke<void>("claude_status_line_enable"),
  claudeStatusLineDisable: () => invoke<void>("claude_status_line_disable"),
  /** Phone Link notifications in Action Center, plus whether Notch may read them. */
  /** Which "Continue with …" sign-ins this build offers. */
  signInOptions: () => invoke<SignInOptions>("sign_in_options"),
  calendarConnectBuiltin: () => invoke<void>("calendar_connect_builtin"),
  spotifyConnectBuiltin: () => invoke<void>("spotify_connect_builtin"),
  githubStatus: () => invoke<GithubStatus>("github_status"),
  /** Starts GitHub sign-in; returns the code to enter on github.com (opened in the browser). */
  githubSignIn: () => invoke<DeviceCode>("github_sign_in"),
  githubCancelSignIn: () => invoke<void>("github_cancel_sign_in"),
  githubSignOut: () => invoke<void>("github_sign_out"),
  /** Unread GitHub notifications, newest first. */
  githubNotifications: () => invoke<GithubNotification[]>("github_notifications"),
  /** Mark a GitHub notification read (on GitHub too). */
  githubMarkRead: (id: string) => invoke<void>("github_mark_read", { id }),
  /** Your GitHub repositories, most recently pushed first. */
  githubRepositories: () => invoke<GithubRepo[]>("github_repositories"),
  githubRepoOverview: (nameWithOwner: string) =>
    invoke<GithubRepoOverview>("github_repo_overview", { nameWithOwner }),
  /** Show a GitHub repo in the Git module; returns the local clone used, if you have one added. */
  githubSelectRepo: (nameWithOwner: string) =>
    invoke<GithubLocalRepo | null>("github_select_repo", { nameWithOwner }),
  phoneSnapshot: () => invoke<PhoneSnapshot>("phone_snapshot"),
  /** Clear a phone notification from Action Center. */
  phoneDismiss: (id: number) => invoke<void>("phone_dismiss", { id }),
  /** Bring up Phone Link (replies happen there). */
  phoneOpen: () => invoke<void>("phone_open"),
  phoneOpenAccessSettings: () => invoke<void>("phone_open_access_settings"),
  todoSnapshot: () => invoke<TodoSnapshot>("todo_snapshot"),
  /** Add a task; with `timerMinutes` its timer starts right away (`ring`: sound when done). */
  /** A reminder that rings at `atMs` (epoch ms), e.g. from a copied date. */
  todoAddReminder: (title: string, atMs: number) => invoke<Task>("todo_add_reminder", { title, atMs }),
  todoAddTask: (title: string, timerMinutes: number | null, ring: boolean) =>
    invoke<Task>("todo_add_task", { title, timerMinutes, ring }),
  todoSetDone: (id: string, done: boolean) => invoke<void>("todo_set_done", { id, done }),
  todoRename: (id: string, title: string) => invoke<void>("todo_rename", { id, title }),
  todoDelete: (id: string) => invoke<void>("todo_delete", { id }),
  todoClearDone: () => invoke<void>("todo_clear_done"),
  todoStartTimer: (id: string, minutes: number, ring: boolean) =>
    invoke<void>("todo_start_timer", { id, minutes, ring }),
  todoCancelTimer: (id: string) => invoke<void>("todo_cancel_timer", { id }),
  /** Remind about a task at a set time (epoch ms). */
  todoRemindAt: (id: string, atMs: number, ring: boolean) =>
    invoke<void>("todo_remind_at", { id, atMs, ring }),
  /** An example approval card in the notch (nothing runs). */
  claudeTestApproval: () => invoke<void>("claude_test_approval"),
  /** Dismiss a finished timer: stops the ring and clears the alert on every screen. */
  todoAcknowledge: (id: string) => invoke<void>("todo_acknowledge", { id }),
  noteAdd: (body: string) => invoke<Note>("note_add", { body }),
  noteUpdate: (id: string, body: string) => invoke<void>("note_update", { id, body }),
  noteDelete: (id: string) => invoke<void>("note_delete", { id }),
  claudeApprovalsPending: () => invoke<ApprovalRequest[]>("claude_approvals_pending"),
  /** Answer a waiting Claude Code permission request. Fails if it already expired. */
  claudeApprovalDecide: (id: string, choice: ApprovalChoice) =>
    invoke<void>("claude_approval_decide", { id, choice }),
  claudeApprovalAudit: (limit: number) => invoke<AuditEntry[]>("claude_approval_audit", { limit }),
  /** Debug builds only: run a fake permission request through the real approval flow. */
  devEmitTestApproval: (tool: string, input: unknown, delayMs?: number) =>
    invoke<void>("dev_emit_test_approval", { tool, input, delayMs }),
  setWindowSize: (width: number, height: number) => invoke<void>("shell_set_window_size", { width, height }),
};

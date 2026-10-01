// Typed wrappers over Tauri commands. Components call these, never `invoke` directly,
// so every backend call is discoverable in one place (plan.md Rule 4).
import { invoke } from "@tauri-apps/api/core";

import type { AppInfo } from "./generated/AppInfo";
import type { ClipboardSnapshot } from "./generated/ClipboardSnapshot";
import type { ShelfSnapshot } from "./generated/ShelfSnapshot";
import type { AskSnapshot } from "./generated/AskSnapshot";
import type { BridgeState } from "./generated/BridgeState";
import type { Dashboard } from "./generated/Dashboard";
import type { ControlSnapshot } from "./generated/ControlSnapshot";
import type { DownloadProgress } from "./generated/DownloadProgress";
import type { GameStatus } from "./generated/GameStatus";
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
  /** Bytes per second since the last call (call about once a second while shown). */
  controlSpeed: () => invoke<NetSpeed>("control_speed"),
  controlSetRadio: (kind: RadioKind, on: boolean) => invoke<void>("control_set_radio", { kind, on }),
  /** Open a Windows Settings page: nightlight, airplane, wifi, bluetooth, hotspot, battery, radios. */
  controlOpenSettings: (page: string) => invoke<void>("control_open_settings", { page }),
  filesDownloads: () => invoke<DownloadProgress[]>("files_downloads"),
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
  bridgeStop: () => invoke<void>("bridge_stop"),
  shelfSnapshot: () => invoke<ShelfSnapshot>("shelf_snapshot"),
  /** Park files on the drop shelf; resolves to how many were new. */
  shelfAdd: (paths: string[]) => invoke<number>("shelf_add", { paths }),
  shelfRemove: (ids: string[]) => invoke<void>("shelf_remove", { ids }),
  shelfClear: () => invoke<void>("shelf_clear"),
  /** Copy shelf files to the clipboard (no ids: all). */
  shelfCopy: (ids: string[] = []) => invoke<void>("shelf_copy", { ids }),
  shelfReveal: (id: string) => invoke<void>("shelf_reveal", { id }),
  /** Drag shelf files out of the notch (no ids: all); true if dropped somewhere. */
  shelfDrag: (ids: string[] = []) => invoke<boolean>("shelf_drag", { ids }),
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
  /** What connecting will add to Claude Code's settings (token masked). */
  claudeHooksPreview: () => invoke<string>("claude_hooks_preview"),
  claudeConnect: () => invoke<void>("claude_connect"),
  claudeDisconnect: () => invoke<void>("claude_disconnect"),
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

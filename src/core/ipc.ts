// Typed wrappers over Tauri commands. Components call these, never `invoke` directly,
// so every backend call is discoverable in one place (plan.md Rule 4).
import { invoke } from "@tauri-apps/api/core";

import type { AppInfo } from "./generated/AppInfo";
import type { ApprovalChoice } from "./generated/ApprovalChoice";
import type { ApprovalRequest } from "./generated/ApprovalRequest";
import type { AuditEntry } from "./generated/AuditEntry";
import type { CalendarSnapshot } from "./generated/CalendarSnapshot";
import type { ClaudeStatus } from "./generated/ClaudeStatus";
import type { DevTestActivity } from "./generated/DevTestActivity";
import type { GitOperation } from "./generated/GitOperation";
import type { GitSnapshot } from "./generated/GitSnapshot";
import type { PullRequests } from "./generated/PullRequests";
import type { ScreenInfo } from "./generated/ScreenInfo";
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
  settingsRecordModuleInit: (moduleId: string, ok: boolean) =>
    invoke<Settings>("settings_record_module_init", { moduleId, ok }),
  autostartGet: () => invoke<boolean>("autostart_get"),
  autostartSet: (enabled: boolean) => invoke<boolean>("autostart_set", { enabled }),
  /** Connected screens, for "Show the notch on". */
  shellListScreens: () => invoke<ScreenInfo[]>("shell_list_screens"),
  /** Whether a fullscreen app is in front on this island's screen; `FullscreenChanged` reports changes. */
  shellIsFullscreen: () => invoke<boolean>("shell_is_fullscreen"),
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

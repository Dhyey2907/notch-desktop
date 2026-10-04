//! Runs one question at a time through Claude Code and streams the answer to the notch.

use std::{
    io::{BufRead, BufReader, Write},
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

use serde::Serialize;
use tauri::{AppHandle, Manager};
use ts_rs::TS;

use super::{
    context_text, parse_line,
    providers::{self, Prompt, Provider},
    user_message, Chunk, Exchange, MAX_HISTORY, MAX_QUESTION, SYSTEM_PROMPT,
};
use crate::{
    error::{AppError, AppResult},
    events::AppEvent,
    state::AppState,
};

/// An answer taking longer than this is stopped.
const TIMEOUT: Duration = Duration::from_secs(180);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AskSnapshot {
    pub exchanges: Vec<Exchange>,
    /// App whose window "Include this window" would send, e.g. "Chrome".
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_app: Option<String>,
    /// The chosen AI can be used (installed / set up).
    pub available: bool,
    /// Its name for the UI: "Claude", "Codex", "Gemini", "Ollama" or "AI".
    pub provider: String,
    /// It can look at a screenshot of your window.
    pub sees_screen: bool,
}

#[derive(Default)]
struct Inner {
    exchanges: Vec<Exchange>,
    child: Option<Child>,
    /// Set by Stop for answers that aren't a Claude process (Codex, Gemini, APIs).
    cancel: Option<Arc<AtomicBool>>,
    /// Window in front when Ask was opened, and its app's name.
    target: Option<(isize, String)>,
}

#[derive(Clone)]
pub struct AskService {
    app: AppHandle,
    workdir: PathBuf,
    inner: Arc<Mutex<Inner>>,
}

/// `claude` on PATH, or where the installer puts it.
pub(crate) fn claude_exe() -> Option<PathBuf> {
    let exe = if cfg!(windows) { "claude.exe" } else { "claude" };
    std::env::var_os("PATH")
        .into_iter()
        .flat_map(|paths| std::env::split_paths(&paths).collect::<Vec<_>>())
        .chain(std::env::var_os("USERPROFILE").map(|home| PathBuf::from(home).join(".local").join("bin")))
        .map(|dir| dir.join(exe))
        .find(|path| path.is_file())
}

impl AskService {
    pub fn new(app: AppHandle, workdir: PathBuf) -> Self {
        Self {
            app,
            workdir,
            inner: Arc::default(),
        }
    }

    fn inner(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn ai(&self) -> crate::settings::AiSettings {
        self.app
            .state::<AppState>()
            .settings
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .ai
            .clone()
    }

    pub fn snapshot(&self) -> AskSnapshot {
        let ai = self.ai();
        let provider = Provider::from_settings(&ai);
        let inner = self.inner();
        AskSnapshot {
            exchanges: inner.exchanges.clone(),
            target_app: inner.target.as_ref().map(|(_, app)| app.clone()),
            available: providers::available(&ai),
            provider: provider.name().into(),
            sees_screen: provider.sees_images(),
        }
    }

    fn publish(&self) {
        self.app
            .state::<AppState>()
            .bus
            .emit("ask", AppEvent::AskChanged(self.snapshot()));
    }

    /// Remember the window the user was in (called before the notch takes focus).
    pub fn set_target(&self, hwnd: isize, app: Option<String>) {
        self.inner().target = app.map(|a| (hwnd, a));
        self.publish();
    }

    pub fn ask(&self, question: &str, include_screen: bool) -> AppResult<String> {
        let question = question.trim();
        if question.is_empty() {
            return Err(AppError::InvalidInput("Type a question".into()));
        }
        if question.chars().count() > MAX_QUESTION {
            return Err(AppError::InvalidInput("That question is too long".into()));
        }
        let ai = self.ai();
        let provider = Provider::from_settings(&ai);
        let exe = match provider {
            Provider::Claude => Some(claude_exe().ok_or_else(|| {
                AppError::InvalidInput("Claude Code isn't installed on this PC (claude.ai/code)".into())
            })?),
            _ => None,
        };
        let (history, target) = {
            let inner = self.inner();
            if inner.exchanges.iter().any(|e| e.pending) {
                return Err(AppError::InvalidInput(
                    "Still answering — wait or press Stop".into(),
                ));
            }
            (inner.exchanges.clone(), inner.target.clone())
        };
        let id = uuid::Uuid::new_v4().to_string();
        let screenshot = if include_screen && provider.sees_images() {
            #[cfg(windows)]
            {
                target
                    .as_ref()
                    .and_then(|(hwnd, _)| super::capture::window_png(*hwnd))
            }
            #[cfg(not(windows))]
            {
                let _ = target;
                None
            }
        } else {
            None
        };
        let message = match provider {
            Provider::Claude => user_message(&history, question, screenshot.as_deref()),
            _ => context_text(&history, question),
        };
        {
            let mut inner = self.inner();
            inner.exchanges.push(Exchange {
                id: id.clone(),
                question: question.to_owned(),
                answer: String::new(),
                pending: true,
                with_screen: screenshot.is_some(),
                error: None,
            });
            let excess = inner.exchanges.len().saturating_sub(MAX_HISTORY * 2);
            inner.exchanges.drain(..excess);
        }
        self.publish();

        let this = self.clone();
        let run_id = id.clone();
        std::thread::Builder::new()
            .name("ask".into())
            .spawn(move || match exe {
                Some(exe) => this.run(&exe, &run_id, &message),
                None => this.run_other(&ai, &run_id, &message, screenshot.as_deref()),
            })
            .map_err(|e| AppError::Platform(e.to_string()))?;
        Ok(id)
    }

    fn run(&self, exe: &PathBuf, id: &str, message: &str) {
        let _ = std::fs::create_dir_all(&self.workdir);
        let mut command = Command::new(exe);
        command
            .args([
                "-p",
                "--input-format",
                "stream-json",
                "--output-format",
                "stream-json",
                "--verbose",
                "--include-partial-messages",
                // No tools, no MCP servers, no user/project settings (so no hooks), no saved session.
                "--tools",
                "",
                "--setting-sources",
                "",
                "--strict-mcp-config",
                "--no-session-persistence",
                "--system-prompt",
                SYSTEM_PROMPT,
            ])
            .current_dir(&self.workdir)
            .env_remove("CLAUDECODE")
            .env_remove("CLAUDE_CODE_ENTRYPOINT")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(windows::Win32::System::Threading::CREATE_NO_WINDOW.0);
        }
        let mut child = match command.spawn() {
            Ok(child) => child,
            Err(_) => return self.finish(id, Some("Couldn't start Claude Code".into())),
        };
        let stdout = child.stdout.take();
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(message.as_bytes());
            let _ = stdin.write_all(b"\n");
            // Dropping stdin ends the input, so Claude answers once and exits.
        }
        self.inner().child = Some(child);

        // Stop answers that run too long.
        let watchdog = self.clone();
        let watched = id.to_owned();
        std::thread::spawn(move || {
            std::thread::sleep(TIMEOUT);
            if watchdog.is_pending(&watched) {
                watchdog.kill();
                watchdog.finish(&watched, Some("That took too long — try again".into()));
            }
        });

        let mut error = None;
        let mut done = false;
        if let Some(stdout) = stdout {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { break };
                match parse_line(&line) {
                    Chunk::Text(text) => self.append(id, &text),
                    Chunk::Done => done = true,
                    Chunk::Failed(message) => error = Some(message),
                    Chunk::Other => {}
                }
            }
        }
        if let Some(mut child) = self.inner().child.take() {
            let _ = child.wait();
        }
        if !done && error.is_none() {
            error = Some("Stopped".into());
        }
        self.finish(id, error);
    }

    /// Answer with Codex, Gemini CLI, Ollama or an API.
    fn run_other(&self, ai: &crate::settings::AiSettings, id: &str, text: &str, image: Option<&[u8]>) {
        let provider = Provider::from_settings(ai);
        let cancel = Arc::new(AtomicBool::new(false));
        self.inner().cancel = Some(cancel.clone());
        let prompt = Prompt {
            system: SYSTEM_PROMPT,
            text,
            image,
        };
        let result = match provider {
            Provider::Codex | Provider::Gemini => providers::start_cli(provider, ai, &prompt, &self.workdir)
                .and_then(|run| providers::finish_cli(provider, run, &cancel))
                .map(|answer| self.append(id, &answer)),
            _ => {
                let this = self.clone();
                let id = id.to_owned();
                tauri::async_runtime::block_on(providers::stream_http(
                    ai,
                    &prompt,
                    &mut |piece| this.append(&id, piece),
                    &cancel,
                ))
            }
        };
        self.inner().cancel = None;
        let error = result.err().map(|e| match e {
            AppError::InvalidInput(message) => message,
            other => other.to_string(),
        });
        self.finish(id, error);
    }

    fn is_pending(&self, id: &str) -> bool {
        self.inner().exchanges.iter().any(|e| e.id == id && e.pending)
    }

    fn append(&self, id: &str, text: &str) {
        let found = {
            let mut inner = self.inner();
            match inner.exchanges.iter_mut().find(|e| e.id == id && e.pending) {
                Some(e) => {
                    e.answer.push_str(text);
                    true
                }
                None => false,
            }
        };
        if found {
            self.app.state::<AppState>().bus.emit(
                "ask",
                AppEvent::AskDelta {
                    id: id.to_owned(),
                    text: text.to_owned(),
                },
            );
        }
    }

    fn finish(&self, id: &str, error: Option<String>) {
        {
            let mut inner = self.inner();
            let Some(e) = inner.exchanges.iter_mut().find(|e| e.id == id && e.pending) else {
                return;
            };
            e.pending = false;
            if e.answer.is_empty() || error.as_deref() != Some("Stopped") {
                e.error = error;
            }
        }
        self.publish();
    }

    fn kill(&self) {
        let mut inner = self.inner();
        if let Some(child) = inner.child.as_mut() {
            let _ = child.kill();
        }
        if let Some(cancel) = inner.cancel.as_ref() {
            cancel.store(true, Ordering::Relaxed);
        }
    }

    /// Stop the answer being written (what's arrived so far stays).
    pub fn stop(&self) {
        self.kill();
    }

    /// Forget the conversation.
    pub fn clear(&self) {
        self.kill();
        self.inner().exchanges.clear();
        self.publish();
    }
}

//! A single question to Claude Code with no tools, answered as plain text.

use std::{
    io::{Read, Write},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

use crate::error::{AppError, AppResult};

const TIMEOUT: Duration = Duration::from_secs(90);
/// Research reads several pages: give it longer.
const WEB_TIMEOUT: Duration = Duration::from_secs(300);

pub fn answer(system: &str, input: &str, workdir: &std::path::Path) -> AppResult<String> {
    run(system, input, workdir, None, TIMEOUT)
}

/// Like `answer`, but Claude may search and read the web (and nothing else).
pub fn answer_with_web(system: &str, input: &str, workdir: &std::path::Path) -> AppResult<String> {
    run(system, input, workdir, Some("WebSearch,WebFetch"), WEB_TIMEOUT)
}

fn run(
    system: &str,
    input: &str,
    workdir: &std::path::Path,
    tools: Option<&str>,
    timeout: Duration,
) -> AppResult<String> {
    let exe = super::service::claude_exe().ok_or_else(|| {
        AppError::InvalidInput("Claude Code isn't installed on this PC (claude.ai/code)".into())
    })?;
    let _ = std::fs::create_dir_all(workdir);
    let mut command = Command::new(exe);
    command.args(["-p", "--output-format", "text", "--tools", tools.unwrap_or("")]);
    // Print mode can't ask, so the tools it may use are allowed up front.
    if let Some(tools) = tools {
        command.args(["--allowedTools", tools]);
    }
    command
        .args([
            "--setting-sources",
            "",
            "--strict-mcp-config",
            "--no-session-persistence",
            "--system-prompt",
            system,
        ])
        .current_dir(workdir)
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
    let mut child = command
        .spawn()
        .map_err(|_| AppError::InvalidInput("Couldn't start Claude Code".into()))?;
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(input.as_bytes());
    }
    let mut stdout = child.stdout.take();
    let reader = std::thread::spawn(move || {
        let mut s = String::new();
        if let Some(o) = stdout.as_mut() {
            let _ = o.read_to_string(&mut s);
        }
        s
    });
    let started = Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            let text = reader.join().unwrap_or_default();
            if !status.success() || text.trim().is_empty() {
                return Err(AppError::InvalidInput(
                    "Claude Code didn't answer. Is it signed in? Run `claude` once in a terminal.".into(),
                ));
            }
            return Ok(text.trim().to_owned());
        }
        if started.elapsed() > timeout {
            let _ = child.kill();
            return Err(AppError::InvalidInput("Claude took too long — try again".into()));
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

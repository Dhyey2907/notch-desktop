//! `git` / `gh` child processes. Arguments are always passed as arrays (never through a
//! shell), no console window flashes, prompts are disabled, and every call has a timeout.

use std::{
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

use chrono::Local;

use super::{
    activity, parse_log, parse_prs, parse_status, summarize_error, BranchStatus, Commit, DayCount,
    PrUnavailable, PullRequests, LOG_FORMAT, PR_FIELDS,
};
use crate::error::{AppError, AppResult};

/// Local reads (status, log) are quick; network operations get longer.
pub const LOCAL_TIMEOUT: Duration = Duration::from_secs(20);
pub const NETWORK_TIMEOUT: Duration = Duration::from_secs(120);

pub struct Output {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

fn spawn_reader(mut pipe: impl Read + Send + 'static) -> std::thread::JoinHandle<String> {
    std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = pipe.read_to_end(&mut buf);
        String::from_utf8_lossy(&buf).into_owned()
    })
}

pub fn run(program: &str, args: &[&str], cwd: Option<&Path>, timeout: Duration) -> AppResult<Output> {
    let mut command = Command::new(program);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        // Never block on an interactive terminal prompt; credential helpers (GUI) still work.
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GH_PROMPT_DISABLED", "1")
        .env("NO_COLOR", "1");
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    let mut child = command.spawn().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            AppError::InvalidInput(format!("{program} is not installed"))
        } else {
            e.into()
        }
    })?;
    // Drain both pipes concurrently so a chatty process can't deadlock on a full buffer.
    let stdout = spawn_reader(child.stdout.take().expect("piped"));
    let stderr = spawn_reader(child.stderr.take().expect("piped"));

    let deadline = Instant::now() + timeout;
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(AppError::InvalidInput(format!(
                "{program} took too long and was stopped"
            )));
        }
        std::thread::sleep(Duration::from_millis(25));
    };
    Ok(Output {
        success: status.success(),
        stdout: stdout.join().unwrap_or_default(),
        stderr: stderr.join().unwrap_or_default(),
    })
}

fn git(repo: &Path, args: &[&str], timeout: Duration) -> AppResult<String> {
    let out = run("git", args, Some(repo), timeout)?;
    if out.success {
        Ok(out.stdout)
    } else {
        Err(AppError::InvalidInput(summarize_error(&out.stderr)))
    }
}

pub fn git_available() -> bool {
    run("git", &["--version"], None, LOCAL_TIMEOUT).is_ok_and(|o| o.success)
}

/// Resolve any folder inside a repository to its top level.
pub fn toplevel(path: &Path) -> AppResult<PathBuf> {
    let out = git(path, &["rev-parse", "--show-toplevel"], LOCAL_TIMEOUT)
        .map_err(|_| AppError::InvalidInput("that folder isn't inside a Git repository".into()))?;
    Ok(PathBuf::from(out.trim()))
}

pub fn status(repo: &Path) -> AppResult<BranchStatus> {
    // --no-optional-locks: don't refresh (write) the index while merely looking, so our own
    // status checks never wake the file watcher into a refresh loop.
    git(
        repo,
        &[
            "--no-optional-locks",
            "status",
            "--porcelain=v2",
            "--branch",
            "-z",
        ],
        LOCAL_TIMEOUT,
    )
    .map(|o| parse_status(&o))
}

pub fn log(repo: &Path, count: u32) -> AppResult<Vec<Commit>> {
    let n = format!("-n{count}");
    let format = format!("--format={LOG_FORMAT}");
    match git(repo, &["log", &n, &format], LOCAL_TIMEOUT) {
        Ok(out) => Ok(parse_log(&out)),
        // A brand-new repo has no commits yet.
        Err(_) => Ok(Vec::new()),
    }
}

/// Your commits per day (by `user.email`, all branches) over the last `days` days.
pub fn commit_activity(repo: &Path, days: u32) -> AppResult<Vec<DayCount>> {
    let email = git(repo, &["config", "user.email"], LOCAL_TIMEOUT).unwrap_or_default();
    let since = format!("--since={days} days ago");
    let mut args = vec!["log", "--all", &since, "--format=%ct"];
    let author = format!("--author={}", email.trim());
    if !email.trim().is_empty() {
        args.push(&author);
    }
    let out = git(repo, &args, LOCAL_TIMEOUT).unwrap_or_default();
    Ok(activity(&out, Local::now().date_naive(), days))
}

/// Commit all tracked changes (and, if asked, new files too).
pub fn commit(repo: &Path, message: &str, include_untracked: bool) -> AppResult<()> {
    if include_untracked {
        git(repo, &["add", "--all"], LOCAL_TIMEOUT)?;
    }
    // `-m <msg>` as separate args, so a message starting with '-' stays a message.
    git(repo, &["commit", "--all", "-m", message], LOCAL_TIMEOUT).map(|_| ())
}

pub fn push(repo: &Path, has_upstream: bool) -> AppResult<()> {
    let args: &[&str] = if has_upstream {
        &["push"]
    } else {
        &["push", "--set-upstream", "origin", "HEAD"]
    };
    git(repo, args, NETWORK_TIMEOUT).map(|_| ())
}

/// Fast-forward only: never creates merge commits or rewrites anything behind your back.
pub fn pull(repo: &Path) -> AppResult<()> {
    git(repo, &["pull", "--ff-only"], NETWORK_TIMEOUT).map(|_| ())
}

/// URL of the `origin` remote, if any.
pub fn origin_url(repo: &Path) -> Option<String> {
    git(repo, &["remote", "get-url", "origin"], LOCAL_TIMEOUT)
        .ok()
        .map(|url| url.trim().to_owned())
        .filter(|url| !url.is_empty())
}

/// Open pull requests for the repo via the GitHub CLI, using its existing sign-in.
pub fn pull_requests(repo: &Path) -> PullRequests {
    let unavailable = |reason| PullRequests::Unavailable { reason };
    match run("gh", &["auth", "status"], Some(repo), LOCAL_TIMEOUT) {
        Err(_) => return unavailable(PrUnavailable::GhMissing),
        Ok(out) if !out.success => return unavailable(PrUnavailable::NotAuthenticated),
        Ok(_) => {}
    }
    let args = [
        "pr", "list", "--state", "open", "--limit", "6", "--json", PR_FIELDS,
    ];
    match run("gh", &args, Some(repo), NETWORK_TIMEOUT) {
        Ok(out) if out.success => match parse_prs(&out.stdout) {
            Some(items) => PullRequests::Ready { items },
            None => unavailable(PrUnavailable::Failed),
        },
        Ok(out) => {
            let stderr = out.stderr.to_ascii_lowercase();
            if stderr.contains("remote") || stderr.contains("not a git repository") {
                unavailable(PrUnavailable::NotGithub)
            } else {
                tracing::info!(error = %summarize_error(&out.stderr), "gh pr list failed");
                unavailable(PrUnavailable::Failed)
            }
        }
        Err(_) => unavailable(PrUnavailable::Failed),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Runs against the real `git` on PATH in a throwaway repo under the temp dir.
    #[test]
    fn end_to_end_with_a_scratch_repo() {
        if !git_available() {
            return; // no git on this machine; parsing is covered elsewhere
        }
        let dir = std::env::temp_dir().join(format!("notch-git-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        let g = |args: &[&str]| git(&dir, args, LOCAL_TIMEOUT).unwrap();
        g(&["init", "-q", "-b", "main"]);
        g(&["config", "user.email", "test@example.com"]);
        g(&["config", "user.name", "Test"]);

        assert_eq!(toplevel(&dir.join("sub")).unwrap().file_name(), dir.file_name());
        assert!(log(&dir, 5).unwrap().is_empty()); // no commits yet

        std::fs::write(dir.join("a.txt"), "one").unwrap();
        let s = status(&dir).unwrap();
        assert_eq!(s.branch.as_deref(), Some("main"));
        assert_eq!(s.changes.untracked, 1);

        commit(&dir, "-starts with a dash", true).unwrap();
        let commits = log(&dir, 5).unwrap();
        assert_eq!(commits[0].subject, "-starts with a dash");
        assert_eq!(
            status(&dir).unwrap().changes,
            super::super::ChangeCounts::default()
        );

        std::fs::write(dir.join("a.txt"), "two").unwrap();
        assert_eq!(status(&dir).unwrap().changes.modified, 1);
        commit(&dir, "second", false).unwrap();

        let days = commit_activity(&dir, 7).unwrap();
        assert_eq!(days.last().unwrap().count, 2);

        assert!(push(&dir, false).is_err()); // no remote: a clean error, not a hang
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn reports_a_missing_program_clearly() {
        let err = run("definitely-not-a-real-program-xyz", &[], None, LOCAL_TIMEOUT)
            .err()
            .unwrap();
        assert!(err.to_string().contains("not installed"));
    }
}

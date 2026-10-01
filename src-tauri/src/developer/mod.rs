//! Developer tools: local dev servers (with Open / Copy URL / Stop and a note when one stops),
//! coding time per project (from the editor in front), a quick JavaScript/Python runner, and
//! custom widgets fed by a URL or a command.

#[cfg(windows)]
mod ports;
pub mod runner;
pub mod service;
pub mod widgets;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DevServer {
    pub port: u16,
    pub pid: u32,
    /// "node", "python", …
    pub process: String,
    /// What it's running, short ("vite", "next dev", "manage.py runserver").
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ProjectTime {
    pub project: String,
    pub minutes: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CodingStats {
    /// Today, busiest project first.
    pub today: Vec<ProjectTime>,
    /// Minutes per day for the last 7 days, oldest first (today last).
    pub week: Vec<u32>,
    /// Days in a row with at least `STREAK_MINUTES` of coding (today counts once reached).
    pub streak: u32,
}

/// Coding minutes that make a day count toward the streak.
pub const STREAK_MINUTES: u32 = 15;

/// Programs that run dev servers. Anything else listening (Windows services, games, chat
/// apps) isn't shown.
const DEV_PROCESSES: &[&str] = &[
    "node", "bun", "deno", "python", "python3", "pythonw", "py", "ruby", "java", "javaw", "dotnet", "php",
    "go", "air", "hugo", "caddy", "nginx", "httpd", "uvicorn", "gunicorn", "flask", "rails", "cargo",
    "trunk", "wrangler", "vercel", "netlify", "firebase", "supabase", "ngrok", "esbuild", "rustc",
];

pub fn is_dev_process(exe_stem: &str) -> bool {
    let stem = exe_stem.to_lowercase();
    DEV_PROCESSES.contains(&stem.as_str())
}

/// The interesting part of a command line: "node …/vite/bin/vite.js --port 5173" → "vite".
pub fn short_command(command_line: &str) -> Option<String> {
    let known = [
        ("next", "next dev"),
        ("vite", "vite"),
        ("nuxt", "nuxt dev"),
        ("astro", "astro dev"),
        ("remix", "remix dev"),
        ("webpack", "webpack"),
        ("react-scripts", "react-scripts start"),
        ("ng serve", "ng serve"),
        ("@angular", "ng serve"),
        ("svelte-kit", "svelte-kit dev"),
        ("storybook", "storybook"),
        ("parcel", "parcel"),
        ("nodemon", "nodemon"),
        ("tsx watch", "tsx watch"),
        ("manage.py runserver", "manage.py runserver"),
        ("uvicorn", "uvicorn"),
        ("flask", "flask run"),
        ("http.server", "python -m http.server"),
        ("jupyter", "jupyter"),
        ("streamlit", "streamlit"),
        ("rails", "rails server"),
        ("tauri", "tauri dev"),
        ("wrangler", "wrangler dev"),
        ("vercel", "vercel dev"),
    ];
    let lower = command_line.to_lowercase().replace('\\', "/");
    known
        .iter()
        .find(|(needle, _)| lower.contains(needle))
        .map(|(_, label)| (*label).to_owned())
        .or_else(|| {
            // Otherwise the script it runs: "node server.js" → "server.js".
            // Only a script file counts (not inline code after -e / -c).
            const SCRIPTS: &[&str] = &[
                ".js", ".mjs", ".cjs", ".ts", ".mts", ".py", ".rb", ".php", ".jar", ".dll",
            ];
            args_after_program(&lower)
                .split_whitespace()
                .map(|a| a.trim_matches('"'))
                .find(|a| !a.starts_with('-') && SCRIPTS.iter().any(|ext| a.ends_with(ext)))
                .and_then(|a| a.trim_matches('"').rsplit('/').next().map(str::to_owned))
                .filter(|a| !a.is_empty() && a.len() <= 40)
        })
}

/// A command line without its program, even when the program's path has spaces
/// (`"c:/users/my name/node.exe" app.js` or `c:/users/my name/node.exe app.js`).
fn args_after_program(line: &str) -> &str {
    let line = line.trim_start();
    if let Some(rest) = line.strip_prefix('"') {
        return rest.split_once('"').map_or("", |(_, args)| args);
    }
    match line.find(".exe") {
        Some(end) => &line[end + 4..],
        None => line.split_once(' ').map_or("", |(_, args)| args),
    }
}

/// Editors whose window title names the project.
const EDITORS: &[&str] = &[
    "code",
    "code - insiders",
    "cursor",
    "windsurf",
    "zed",
    "devenv",
    "sublime_text",
    "notepad++",
    "idea64",
    "pycharm64",
    "webstorm64",
    "rider64",
    "clion64",
    "goland64",
    "phpstorm64",
    "rubymine64",
    "datagrip64",
    "rustrover64",
    "studio64",
    "fleet",
    "nvim-qt",
    "neovide",
    "positron",
    "trae",
    "kiro",
];

pub fn is_editor(exe_stem: &str) -> bool {
    EDITORS.contains(&exe_stem.to_lowercase().as_str())
}

/// The project an editor window is on, from its title.
/// VS Code / Cursor: "file.ts - project - Visual Studio Code"; JetBrains: "project – file.kt";
/// Visual Studio: "Solution - Microsoft Visual Studio".
pub fn project_from_title(exe_stem: &str, title: &str) -> Option<String> {
    let exe = exe_stem.to_lowercase();
    let title = title
        .trim()
        .trim_start_matches('●')
        .trim_start_matches('*')
        .trim();
    let parts: Vec<&str> = title
        .split(" - ")
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .collect();
    let project = if exe.ends_with("64") || exe == "fleet" || exe == "studio64" {
        // JetBrains puts an en dash (with spaces) between project and file.
        title
            .split(" – ")
            .next()
            .and_then(|p| p.split(" - ").next())
            .map(str::trim)
    } else if exe == "devenv" {
        parts.first().copied()
    } else if exe == "sublime_text" {
        // "file.py (project) - Sublime Text"
        title
            .split_once('(')
            .and_then(|(_, rest)| rest.split_once(')'))
            .map(|(p, _)| p.trim())
    } else if parts.len() >= 3 {
        parts.get(parts.len() - 2).copied()
    } else if parts.len() == 2 {
        // "project - Visual Studio Code" (no file open)
        parts.first().copied()
    } else {
        None
    };
    project
        .map(|p| p.trim_start_matches('[').trim_end_matches(']').trim())
        .map(|p| p.split(" [").next().unwrap_or(p).trim().to_owned())
        .filter(|p| !p.is_empty() && p.len() <= 80 && !p.ends_with("Visual Studio Code"))
}

/// Days in a row (ending today, or yesterday if today hasn't reached the bar yet).
pub fn streak(minutes_by_day_newest_first: &[u32]) -> u32 {
    let mut days = minutes_by_day_newest_first.iter();
    let mut count = 0;
    if let Some(&today) = days.next() {
        if today >= STREAK_MINUTES {
            count += 1;
        }
    }
    for &m in days {
        if m >= STREAK_MINUTES {
            count += 1;
        } else {
            break;
        }
    }
    count
}

/// A custom widget's settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WidgetConfig {
    pub id: String,
    pub name: String,
    /// `url` (fetch JSON or text) or `command` (run in PowerShell).
    pub kind: String,
    pub source: String,
    pub refresh_secs: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_dev_servers_and_names_them() {
        assert!(is_dev_process("node"));
        assert!(is_dev_process("Python"));
        assert!(!is_dev_process("svchost"));
        assert!(!is_dev_process("Discord"));
        assert_eq!(
            short_command(
                r#""C:\Program Files\nodejs\node.exe" C:\dev\app\node_modules\vite\bin\vite.js --port 5173"#
            ),
            Some("vite".into())
        );
        assert_eq!(
            short_command("node node_modules/.bin/next dev"),
            Some("next dev".into())
        );
        assert_eq!(
            short_command("python manage.py runserver 8000"),
            Some("manage.py runserver".into())
        );
        assert_eq!(
            short_command(r"node C:\dev\api\server.js"),
            Some("server.js".into())
        );
        assert_eq!(short_command("node"), None);
        assert_eq!(
            short_command(r"C:\Users\Dhyey's Laptop\node\node.exe C:\dev\api\index.js"),
            Some("index.js".into())
        );
        assert_eq!(
            short_command(r#""C:\Users\My Name\node.exe" -e "inline code here and more""#),
            None
        );
    }

    #[test]
    fn reads_projects_from_editor_titles() {
        assert_eq!(
            project_from_title("Code", "● Island.tsx - notch-desktop - Visual Studio Code"),
            Some("notch-desktop".into())
        );
        assert_eq!(
            project_from_title("Cursor", "notch-desktop - Cursor"),
            Some("notch-desktop".into())
        );
        assert_eq!(
            project_from_title("idea64", "backend – UserService.kt"),
            Some("backend".into())
        );
        assert_eq!(
            project_from_title("webstorm64", "notch-desktop – app.ts"),
            Some("notch-desktop".into())
        );
        assert_eq!(
            project_from_title("devenv", "MyGame - Microsoft Visual Studio"),
            Some("MyGame".into())
        );
        assert_eq!(
            project_from_title("sublime_text", "main.py (scraper) - Sublime Text"),
            Some("scraper".into())
        );
        assert_eq!(project_from_title("Code", "Visual Studio Code"), None);
        assert_eq!(
            project_from_title(
                "Code",
                "lib.rs - notch-desktop [WSL: Ubuntu] - Visual Studio Code"
            ),
            Some("notch-desktop".into())
        );
        assert!(is_editor("Code"));
        assert!(!is_editor("chrome"));
    }

    #[test]
    fn counts_streaks() {
        assert_eq!(streak(&[20, 30, 15, 0, 60]), 3);
        // Today not reached yet: yesterday's run still counts.
        assert_eq!(streak(&[5, 30, 40, 2]), 2);
        assert_eq!(streak(&[0, 0]), 0);
        assert_eq!(streak(&[]), 0);
    }
}

//! Coding agents besides Claude Code: OpenAI Codex and Google's Gemini CLI.
//!
//! Both have Claude-style hooks, but only as commands (no HTTP hooks). So Notch registers
//! itself as the command — `notch-desktop.exe --agent-hook codex` — and that small relay
//! (see `relay`) forwards each event to the same loopback hook server Claude Code uses.
//! Events are mapped onto Claude Code's names, so live status, alerts and approvals work the
//! same for every agent.
//!
//! Like Claude Code's settings, the agents' config files usually hold the user's own setup,
//! so connecting only *merges*: it adds one group per event whose command is Notch's relay,
//! removal deletes exactly those, the rest is left as it was, and the file is backed up first.

use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::{json, Map, Value};
use ts_rs::TS;

use crate::error::{AppError, AppResult};

/// The relay's command-line flag; also how Notch recognises its own entries.
pub const RELAY_FLAG: &str = "--agent-hook";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Agent {
    #[default]
    Claude,
    Codex,
    Gemini,
}

impl Agent {
    pub const OTHERS: [Agent; 2] = [Agent::Codex, Agent::Gemini];

    pub fn id(self) -> &'static str {
        match self {
            Agent::Claude => "claude",
            Agent::Codex => "codex",
            Agent::Gemini => "gemini",
        }
    }

    pub fn from_id(id: &str) -> Option<Agent> {
        match id {
            "claude" => Some(Agent::Claude),
            "codex" => Some(Agent::Codex),
            "gemini" => Some(Agent::Gemini),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Agent::Claude => "Claude Code",
            Agent::Codex => "Codex",
            Agent::Gemini => "Gemini CLI",
        }
    }

    /// Hook path on Notch's server.
    pub fn hook_path(self) -> String {
        match self {
            Agent::Claude => super::hooks::HOOK_PATH.to_owned(),
            other => format!("{}/{}", super::hooks::HOOK_PATH, other.id()),
        }
    }

    /// Which agent a hook path belongs to.
    pub fn from_path(path: &str) -> Option<Agent> {
        let rest = path.strip_prefix(super::hooks::HOOK_PATH)?;
        match rest {
            "" => Some(Agent::Claude),
            _ => Agent::from_id(rest.strip_prefix('/')?).filter(|a| *a != Agent::Claude),
        }
    }

    /// The config file Notch's hooks go in.
    pub fn config_path(self, home: &Path) -> PathBuf {
        match self {
            Agent::Claude => home.join(".claude").join("settings.json"),
            Agent::Codex => home.join(".codex").join("hooks.json"),
            Agent::Gemini => home.join(".gemini").join("settings.json"),
        }
    }

    /// (event, matches tools, timeout) for each event Notch listens to.
    fn events(self) -> &'static [(&'static str, bool)] {
        match self {
            Agent::Claude => &[],
            // Codex has no session-end event; sessions end when its process exits.
            Agent::Codex => &[
                ("SessionStart", false),
                ("UserPromptSubmit", false),
                ("PreToolUse", true),
                ("PostToolUse", true),
                ("PermissionRequest", true),
                ("Stop", false),
            ],
            Agent::Gemini => &[
                ("SessionStart", false),
                ("SessionEnd", false),
                ("BeforeAgent", false),
                ("BeforeTool", true),
                ("AfterTool", true),
                ("AfterAgent", false),
                ("Notification", false),
            ],
        }
    }

    /// Whether the agent seems to be on this PC (its command or its config folder).
    pub fn detected(self, home: &Path) -> bool {
        let folder = match self {
            Agent::Claude => ".claude",
            Agent::Codex => ".codex",
            Agent::Gemini => ".gemini",
        };
        home.join(folder).is_dir() || find_exe(self).is_some()
    }
}

/// The agent's command-line program, when installed (PATH first, then where the Codex app keeps
/// its bundled copy).
pub fn find_exe(agent: Agent) -> Option<PathBuf> {
    let names: &[&str] = match agent {
        Agent::Claude => &["claude.exe", "claude.cmd", "claude"],
        Agent::Codex => &["codex.exe", "codex.cmd", "codex"],
        Agent::Gemini => &["gemini.cmd", "gemini.exe", "gemini"],
    };
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        for name in names {
            let candidate = dir.join(name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    if agent == Agent::Codex {
        // The Codex desktop app ships the CLI in %LOCALAPPDATA%\OpenAI\Codex\bin\<build>\.
        let bin = PathBuf::from(std::env::var_os("LOCALAPPDATA")?)
            .join("OpenAI")
            .join("Codex")
            .join("bin");
        let mut builds: Vec<(std::time::SystemTime, PathBuf)> = std::fs::read_dir(bin)
            .ok()?
            .flatten()
            .map(|e| e.path().join("codex.exe"))
            .filter(|p| p.is_file())
            .map(|p| {
                (
                    p.metadata()
                        .and_then(|m| m.modified())
                        .unwrap_or(std::time::UNIX_EPOCH),
                    p,
                )
            })
            .collect();
        builds.sort();
        return builds.pop().map(|(_, p)| p);
    }
    None
}

/// The relay command written into the agent's config.
pub fn relay_command(exe: &Path, agent: Agent) -> String {
    format!("\"{}\" {RELAY_FLAG} {}", exe.display(), agent.id())
}

fn is_ours(group: &Value) -> bool {
    group.get("hooks").and_then(Value::as_array).is_some_and(|hooks| {
        hooks.iter().any(|h| {
            h.get("command")
                .and_then(Value::as_str)
                .is_some_and(|c| c.contains(RELAY_FLAG))
        })
    })
}

fn our_group(agent: Agent, event: &str, tool_event: bool, command: &str) -> Value {
    let hook = match agent {
        // Codex: seconds. Approvals wait for you; the rest must be quick.
        Agent::Codex => json!({
            "type": "command",
            "command": command,
            "timeout": if event == "PermissionRequest" { super::approvals::HOOK_TIMEOUT_SECS } else { 10 },
        }),
        // Gemini CLI: milliseconds, and hooks have names.
        _ => json!({
            "name": "notch",
            "type": "command",
            "command": command,
            "timeout": 10_000,
        }),
    };
    if tool_event {
        json!({ "matcher": ".*", "hooks": [hook] })
    } else {
        json!({ "hooks": [hook] })
    }
}

pub fn is_installed(settings: &Value) -> bool {
    settings
        .get("hooks")
        .and_then(Value::as_object)
        .is_some_and(|hooks| {
            hooks
                .values()
                .filter_map(Value::as_array)
                .any(|groups| groups.iter().any(is_ours))
        })
}

/// The relay command currently written in the config, if any (to notice a moved Notch).
pub fn installed_command(settings: &Value) -> Option<String> {
    let hooks = settings.get("hooks")?.as_object()?;
    hooks
        .values()
        .filter_map(Value::as_array)
        .flatten()
        .find_map(|g| {
            g.get("hooks")?.as_array()?.iter().find_map(|h| {
                h.get("command")
                    .and_then(Value::as_str)
                    .filter(|c| c.contains(RELAY_FLAG))
                    .map(str::to_owned)
            })
        })
}

/// Remove every Notch group, dropping lists (and `hooks`) left empty by that.
pub fn remove_ours(settings: &mut Value) {
    let Some(hooks) = settings.get_mut("hooks").and_then(Value::as_object_mut) else {
        return;
    };
    for groups in hooks.values_mut() {
        if let Some(list) = groups.as_array_mut() {
            list.retain(|g| !is_ours(g));
        }
    }
    hooks.retain(|_, groups| groups.as_array().is_none_or(|l| !l.is_empty()));
    if hooks.is_empty() {
        if let Some(obj) = settings.as_object_mut() {
            obj.remove("hooks");
        }
    }
}

/// Add Notch's relay for every event (replacing earlier Notch entries).
pub fn add_ours(settings: &mut Value, agent: Agent, command: &str) -> AppResult<()> {
    remove_ours(settings);
    let file = match agent {
        Agent::Codex => "Codex's hooks.json",
        _ => "Gemini CLI's settings.json",
    };
    let root = settings
        .as_object_mut()
        .ok_or_else(|| AppError::InvalidInput(format!("{file} isn't a JSON object")))?;
    let hooks = root.entry("hooks").or_insert_with(|| Value::Object(Map::new()));
    let hooks = hooks
        .as_object_mut()
        .ok_or_else(|| AppError::InvalidInput(format!("\"hooks\" in {file} isn't an object")))?;
    for &(event, tool_event) in agent.events() {
        let groups = hooks.entry(event).or_insert_with(|| Value::Array(Vec::new()));
        let list = groups
            .as_array_mut()
            .ok_or_else(|| AppError::InvalidInput(format!("\"hooks.{event}\" in {file} isn't a list")))?;
        list.push(our_group(agent, event, tool_event, command));
    }
    Ok(())
}

/// What will be added, shown before the user agrees.
pub fn preview(agent: Agent, command: &str) -> String {
    let mut example = json!({});
    let _ = add_ours(&mut example, agent, command);
    serde_json::to_string_pretty(&example).unwrap_or_default()
}

/// Map an agent's event onto Claude Code's names (which the rest of Notch understands).
/// Returns `None` for events Notch doesn't use.
pub fn claude_event(
    agent: Agent,
    event: &str,
    notification_type: Option<&str>,
) -> Option<(String, Option<String>)> {
    let same = |e: &str| Some((e.to_owned(), notification_type.map(str::to_owned)));
    match agent {
        Agent::Claude | Agent::Codex => same(event),
        Agent::Gemini => match event {
            "SessionStart" | "SessionEnd" => same(event),
            "BeforeAgent" => same("UserPromptSubmit"),
            "BeforeTool" => same("PreToolUse"),
            "AfterTool" => same("PostToolUse"),
            "AfterAgent" => same("Stop"),
            "Notification" => match notification_type {
                Some("ToolPermission") => Some(("Notification".into(), Some("permission_prompt".into()))),
                _ => None,
            },
            _ => None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CMD: &str = r#""C:\Program Files\Notch\notch-desktop.exe" --agent-hook codex"#;

    #[test]
    fn merges_into_existing_config_and_removes_cleanly() {
        let mut settings = json!({
            "theme": "dark",
            "hooks": { "Stop": [{ "hooks": [{ "type": "command", "command": "say done" }] }] }
        });
        add_ours(&mut settings, Agent::Codex, CMD).unwrap();
        assert!(is_installed(&settings));
        assert_eq!(installed_command(&settings).as_deref(), Some(CMD));
        let stop = settings["hooks"]["Stop"].as_array().unwrap();
        assert_eq!(stop.len(), 2, "the user's own Stop hook stays");
        assert_eq!(settings["hooks"]["PermissionRequest"][0]["matcher"], ".*");
        assert_eq!(
            settings["hooks"]["PermissionRequest"][0]["hooks"][0]["timeout"],
            super::super::approvals::HOOK_TIMEOUT_SECS
        );
        // Adding again replaces rather than duplicates.
        add_ours(&mut settings, Agent::Codex, CMD).unwrap();
        assert_eq!(settings["hooks"]["Stop"].as_array().unwrap().len(), 2);

        remove_ours(&mut settings);
        assert!(!is_installed(&settings));
        assert_eq!(
            settings,
            json!({ "theme": "dark", "hooks": { "Stop": [{ "hooks": [{ "type": "command", "command": "say done" }] }] } })
        );
    }

    #[test]
    fn gemini_hooks_have_names_and_millisecond_timeouts() {
        let mut settings = json!({});
        add_ours(&mut settings, Agent::Gemini, "notch --agent-hook gemini").unwrap();
        let hook = &settings["hooks"]["BeforeTool"][0];
        assert_eq!(hook["matcher"], ".*");
        assert_eq!(hook["hooks"][0]["name"], "notch");
        assert_eq!(hook["hooks"][0]["timeout"], 10_000);
        assert!(settings["hooks"].get("PermissionRequest").is_none());
        remove_ours(&mut settings);
        assert_eq!(settings, json!({}));
    }

    #[test]
    fn maps_events_and_paths() {
        assert_eq!(Agent::from_path("/hook"), Some(Agent::Claude));
        assert_eq!(Agent::from_path("/hook/codex"), Some(Agent::Codex));
        assert_eq!(Agent::from_path("/hook/gemini"), Some(Agent::Gemini));
        assert_eq!(Agent::from_path("/hook/claude"), None);
        assert_eq!(Agent::from_path("/hook/x"), None);
        assert_eq!(Agent::from_path("/hooks"), None);
        assert_eq!(Agent::Codex.hook_path(), "/hook/codex");

        let ev = claude_event;
        assert_eq!(
            ev(Agent::Gemini, "BeforeTool", None),
            Some(("PreToolUse".into(), None))
        );
        assert_eq!(ev(Agent::Gemini, "AfterAgent", None), Some(("Stop".into(), None)));
        assert_eq!(
            ev(Agent::Gemini, "Notification", Some("ToolPermission")),
            Some(("Notification".into(), Some("permission_prompt".into())))
        );
        assert_eq!(ev(Agent::Gemini, "BeforeModel", None), None);
        assert_eq!(
            ev(Agent::Codex, "PermissionRequest", None),
            Some(("PermissionRequest".into(), None))
        );
    }
}

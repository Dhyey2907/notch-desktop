//! Add / remove Notch's HTTP hooks in Claude Code's user settings (`~/.claude/settings.json`).
//!
//! The file usually holds the user's own settings and hooks, so this only ever *merges*: it
//! appends one matcher group per event whose hook URL is Notch's, and removal deletes exactly
//! the groups with that URL. Key order is preserved, and the original is backed up first.

use std::path::{Path, PathBuf};

use serde_json::{json, Map, Value};

use super::hooks::{hook_url, HOOK_PORT, STATUS_LINE_PATH};
use crate::error::{AppError, AppResult};

/// Events Notch listens to. Tool events match every tool.
pub const EVENTS: [(&str, bool); 9] = [
    ("SessionStart", false),
    ("UserPromptSubmit", false),
    ("PreToolUse", true),
    ("PostToolUse", true),
    ("PostToolUseFailure", true),
    ("PermissionRequest", true),
    ("Notification", false),
    ("Stop", false),
    ("SessionEnd", false),
];
/// Seconds Claude Code waits for Notch before carrying on without it.
const HOOK_TIMEOUT_SECS: u64 = 5;

/// Permission requests wait for you; everything else must answer immediately.
fn timeout_for(event: &str) -> u64 {
    if event == "PermissionRequest" {
        super::approvals::HOOK_TIMEOUT_SECS
    } else {
        HOOK_TIMEOUT_SECS
    }
}

pub fn settings_path(home: &Path) -> PathBuf {
    home.join(".claude").join("settings.json")
}

fn our_group(event: &str, token: &str, tool_event: bool) -> Value {
    let hook = json!({
        "type": "http",
        "url": hook_url(),
        "headers": { "Authorization": format!("Bearer {token}") },
        "timeout": timeout_for(event),
    });
    if tool_event {
        json!({ "matcher": "*", "hooks": [hook] })
    } else {
        json!({ "hooks": [hook] })
    }
}

fn is_ours(group: &Value) -> bool {
    group.get("hooks").and_then(Value::as_array).is_some_and(|hooks| {
        hooks
            .iter()
            .any(|h| h.get("url").and_then(Value::as_str) == Some(&hook_url()))
    })
}

/// Remove every Notch hook group, dropping event arrays (and `hooks`) left empty by that.
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

/// Add Notch's hooks (replacing any previous Notch entries, e.g. with an old token).
pub fn add_ours(settings: &mut Value, token: &str) -> AppResult<()> {
    remove_ours(settings);
    let root = settings
        .as_object_mut()
        .ok_or_else(|| AppError::InvalidInput("Claude Code settings aren't a JSON object".into()))?;
    let hooks = root.entry("hooks").or_insert_with(|| Value::Object(Map::new()));
    let hooks = hooks
        .as_object_mut()
        .ok_or_else(|| AppError::InvalidInput("\"hooks\" in Claude Code settings isn't an object".into()))?;
    for (event, tool_event) in EVENTS {
        let groups = hooks.entry(event).or_insert_with(|| Value::Array(Vec::new()));
        let list = groups
            .as_array_mut()
            .ok_or_else(|| AppError::InvalidInput(format!("\"hooks.{event}\" isn't a list")))?;
        list.push(our_group(event, token, tool_event));
    }
    Ok(())
}

pub fn is_installed(settings: &Value) -> bool {
    settings
        .get("hooks")
        .and_then(Value::as_object)
        .is_some_and(|hooks| hooks.values().filter_map(Value::as_array).flatten().any(is_ours))
}

fn status_line_url() -> String {
    format!("http://127.0.0.1:{HOOK_PORT}{STATUS_LINE_PATH}")
}

/// Pipes Claude Code's status-line JSON to Notch and prints Notch's reply (e.g.
/// `5h 23% · week 41%`). `curl.exe` ships with Windows; every argument is quoted so the
/// command works under bash, cmd, and PowerShell. If Notch isn't running it prints nothing.
fn status_line_command(token: &str) -> String {
    format!(
        "curl.exe -s -m 2 -H \"Authorization: Bearer {token}\" -H \"Content-Type: application/json\" -H \"Expect:\" --data-binary \"@-\" \"{}\"",
        status_line_url()
    )
}

fn status_line_value(token: &str) -> Value {
    json!({ "type": "command", "command": status_line_command(token), "padding": 0 })
}

/// Notch's status line is the one configured.
pub fn has_our_status_line(settings: &Value) -> bool {
    settings
        .pointer("/statusLine/command")
        .and_then(Value::as_str)
        .is_some_and(|c| c.contains(&status_line_url()))
}

/// Someone else's status line is configured; Notch never replaces it.
pub fn status_line_taken(settings: &Value) -> bool {
    settings.get("statusLine").is_some() && !has_our_status_line(settings)
}

pub fn add_status_line(settings: &mut Value, token: &str) -> AppResult<()> {
    if status_line_taken(settings) {
        return Err(AppError::InvalidInput(
            "Claude Code already has a status line, so Overline won't replace it".into(),
        ));
    }
    let root = settings
        .as_object_mut()
        .ok_or_else(|| AppError::InvalidInput("Claude Code settings aren't a JSON object".into()))?;
    root.insert("statusLine".into(), status_line_value(token));
    Ok(())
}

pub fn remove_status_line(settings: &mut Value) {
    if has_our_status_line(settings) {
        if let Some(root) = settings.as_object_mut() {
            root.remove("statusLine");
        }
    }
}

/// What enabling plan limits adds, with the token masked.
pub fn status_line_preview() -> String {
    serde_json::to_string_pretty(&json!({
        "statusLine": status_line_value("•••••• (stored in Windows Credential Manager)")
    }))
    .unwrap_or_default()
}

/// Installed by an older Notch that didn't register every event (e.g. before approvals).
pub fn is_outdated(settings: &Value) -> bool {
    let Some(hooks) = settings.get("hooks").and_then(Value::as_object) else {
        return false;
    };
    is_installed(settings)
        && EVENTS.iter().any(|(event, _)| {
            !hooks
                .get(*event)
                .and_then(Value::as_array)
                .is_some_and(|groups| groups.iter().any(is_ours))
        })
}

/// What connecting will add, with the token masked, for the confirmation screen.
pub fn preview() -> String {
    let mut hooks = Map::new();
    for (event, tool_event) in EVENTS {
        hooks.insert(
            event.into(),
            json!([our_group(
                event,
                "•••••• (stored in Windows Credential Manager)",
                tool_event
            )]),
        );
    }
    serde_json::to_string_pretty(&json!({ "hooks": hooks })).unwrap_or_default()
}

pub fn read(path: &Path) -> AppResult<Value> {
    match std::fs::read_to_string(path) {
        Ok(text) if text.trim().is_empty() => Ok(json!({})),
        Ok(text) => serde_json::from_str(&text).map_err(|_| {
            AppError::InvalidInput(
                "Claude Code's settings.json isn't valid JSON, so Overline won't touch it. Fix it and try again"
                    .into(),
            )
        }),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(json!({})),
        Err(e) => Err(e.into()),
    }
}

/// Back up the current file, then write the new settings atomically (temp file + rename).
pub fn write(path: &Path, settings: &Value) -> AppResult<()> {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "settings.json".into());
    if path.exists() {
        let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
        std::fs::copy(path, path.with_file_name(format!("{name}.notch-backup-{stamp}")))?;
    } else if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let text =
        serde_json::to_string_pretty(settings).map_err(|e| AppError::InvalidInput(e.to_string()))? + "\n";
    let temp = path.with_file_name(format!("{name}.notch-tmp"));
    std::fs::write(&temp, text)?;
    std::fs::rename(&temp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn existing() -> Value {
        json!({
            "model": "opus",
            "hooks": {
                "PreToolUse": [{ "matcher": "Write|Edit", "hooks": [{ "type": "command", "command": "other-tool pre-edit" }] }],
                "Stop": [{ "hooks": [{ "type": "command", "command": "other-tool stop" }] }]
            },
            "theme": "dark"
        })
    }

    #[test]
    fn merges_without_touching_other_hooks_or_key_order() {
        let mut settings = existing();
        add_ours(&mut settings, "tok").unwrap();
        assert!(is_installed(&settings));
        let keys: Vec<_> = settings.as_object().unwrap().keys().cloned().collect();
        assert_eq!(keys, ["model", "hooks", "theme"]);
        let pre = settings["hooks"]["PreToolUse"].as_array().unwrap();
        assert_eq!(pre.len(), 2);
        assert_eq!(pre[0]["hooks"][0]["command"], "other-tool pre-edit");
        assert_eq!(pre[1]["matcher"], "*");
        assert_eq!(pre[1]["hooks"][0]["headers"]["Authorization"], "Bearer tok");
        assert!(settings["hooks"]["SessionEnd"].as_array().unwrap().len() == 1);
    }

    #[test]
    fn permission_requests_get_a_long_timeout_and_old_installs_are_outdated() {
        let mut settings = existing();
        add_ours(&mut settings, "tok").unwrap();
        assert!(!is_outdated(&settings));
        assert_eq!(
            settings["hooks"]["PermissionRequest"][0]["hooks"][0]["timeout"],
            120
        );
        assert_eq!(settings["hooks"]["Stop"][1]["hooks"][0]["timeout"], 5);
        settings["hooks"]
            .as_object_mut()
            .unwrap()
            .remove("PermissionRequest");
        assert!(is_outdated(&settings));
        assert!(!is_outdated(&existing()));
    }

    #[test]
    fn reconnecting_replaces_instead_of_duplicating() {
        let mut settings = existing();
        add_ours(&mut settings, "old").unwrap();
        add_ours(&mut settings, "new").unwrap();
        let stop = settings["hooks"]["Stop"].as_array().unwrap();
        assert_eq!(stop.len(), 2);
        assert_eq!(stop[1]["hooks"][0]["headers"]["Authorization"], "Bearer new");
    }

    #[test]
    fn removal_restores_the_original_exactly() {
        let mut settings = existing();
        add_ours(&mut settings, "tok").unwrap();
        remove_ours(&mut settings);
        assert_eq!(settings, existing());
        assert!(!is_installed(&settings));

        let mut empty = json!({});
        add_ours(&mut empty, "tok").unwrap();
        remove_ours(&mut empty);
        assert_eq!(empty, json!({}));
    }

    #[test]
    fn refuses_to_edit_unexpected_shapes() {
        let mut not_object = json!({ "hooks": [] });
        assert!(add_ours(&mut not_object, "tok").is_err());
        let mut bad_event = json!({ "hooks": { "Stop": "nope" } });
        assert!(add_ours(&mut bad_event, "tok").is_err());
    }

    #[test]
    fn status_line_is_added_and_removed_but_never_replaces_the_users() {
        let mut settings = existing();
        add_status_line(&mut settings, "tok").unwrap();
        assert!(has_our_status_line(&settings));
        let command = settings["statusLine"]["command"].as_str().unwrap();
        assert!(command.starts_with("curl.exe ") && command.contains("Bearer tok"));
        assert!(command.contains("\"@-\"") && command.contains("/statusline"));
        add_status_line(&mut settings, "new").unwrap(); // re-adding replaces our own
        assert!(settings["statusLine"]["command"]
            .as_str()
            .unwrap()
            .contains("Bearer new"));
        remove_status_line(&mut settings);
        assert_eq!(settings, existing());

        let mut theirs = json!({ "statusLine": { "type": "command", "command": "~/.claude/statusline.sh" } });
        assert!(status_line_taken(&theirs));
        assert!(add_status_line(&mut theirs, "tok").is_err());
        remove_status_line(&mut theirs);
        assert_eq!(theirs["statusLine"]["command"], "~/.claude/statusline.sh");
        assert!(status_line_preview().contains("Credential Manager"));
    }

    #[test]
    fn preview_masks_the_token() {
        let text = preview();
        assert!(text.contains("127.0.0.1"));
        assert!(text.contains("Credential Manager"));
    }

    #[test]
    fn writes_atomically_with_a_backup() {
        let dir = std::env::temp_dir().join(format!("notch-connect-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.json");
        std::fs::write(&path, "{\"a\":1}").unwrap();
        let mut settings = read(&path).unwrap();
        add_ours(&mut settings, "tok").unwrap();
        write(&path, &settings).unwrap();
        assert!(is_installed(&read(&path).unwrap()));
        let backups = std::fs::read_dir(&dir)
            .unwrap()
            .filter(|e| {
                e.as_ref()
                    .unwrap()
                    .file_name()
                    .to_string_lossy()
                    .contains("notch-backup")
            })
            .count();
        assert_eq!(backups, 1);
        std::fs::write(&path, "{ not json").unwrap();
        assert!(read(&path).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}

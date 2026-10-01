//! Custom widgets: a URL that returns JSON or text, or a PowerShell command whose output is
//! shown. JSON like `{"title": "...", "value": "...", "subtitle": "...", "items": [{"label":
//! "...", "value": "..."}]}` gets a tidy layout; anything else shows as text.

use std::{
    io::Read,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

use serde::Serialize;
use serde_json::Value;
use ts_rs::TS;

use super::WidgetConfig;
use crate::error::{AppError, AppResult};

const TIMEOUT: Duration = Duration::from_secs(10);
const MAX_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WidgetItem {
    pub label: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WidgetOutput {
    pub id: String,
    pub name: String,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,
    pub items: Vec<WidgetItem>,
    /// Plain output when it isn't the JSON layout.
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

fn scalar(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

/// Lay out a widget's raw output.
pub fn shape(config: &WidgetConfig, raw: &str) -> WidgetOutput {
    let mut out = WidgetOutput {
        id: config.id.clone(),
        name: config.name.clone(),
        title: None,
        value: None,
        subtitle: None,
        items: Vec::new(),
        text: None,
        error: None,
    };
    let trimmed = raw.trim();
    match serde_json::from_str::<Value>(trimmed) {
        Ok(Value::Object(map))
            if ["title", "value", "subtitle", "items"]
                .iter()
                .any(|k| map.contains_key(*k)) =>
        {
            out.title = map.get("title").and_then(scalar);
            out.value = map.get("value").and_then(scalar);
            out.subtitle = map.get("subtitle").and_then(scalar);
            if let Some(Value::Array(items)) = map.get("items") {
                out.items = items
                    .iter()
                    .take(12)
                    .filter_map(|i| {
                        Some(WidgetItem {
                            label: i.get("label").and_then(scalar)?,
                            value: i.get("value").and_then(scalar).unwrap_or_default(),
                        })
                    })
                    .collect();
            }
        }
        // Any other JSON object: its simple fields as rows.
        Ok(Value::Object(map)) => {
            out.items = map
                .iter()
                .filter_map(|(k, v)| {
                    scalar(v).map(|value| WidgetItem {
                        label: k.clone(),
                        value,
                    })
                })
                .take(12)
                .collect();
        }
        Ok(v) if scalar(&v).is_some() => out.value = scalar(&v),
        _ => {
            let text: String = trimmed.lines().take(12).collect::<Vec<_>>().join("\n");
            out.text = Some(text.chars().take(1500).collect());
        }
    }
    out
}

fn run_command(source: &str) -> AppResult<String> {
    let started = Instant::now();
    let mut cmd = Command::new("powershell.exe");
    cmd.args(["-NoProfile", "-NonInteractive", "-Command", source])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(windows::Win32::System::Threading::CREATE_NO_WINDOW.0);
    }
    let mut child = cmd.spawn()?;
    let mut stdout = child.stdout.take();
    let reader = std::thread::spawn(move || {
        let mut s = String::new();
        if let Some(o) = stdout.as_mut() {
            let _ = o.take(MAX_BYTES as u64).read_to_string(&mut s);
        }
        s
    });
    loop {
        if let Some(status) = child.try_wait()? {
            let out = reader.join().unwrap_or_default();
            if !status.success() && out.trim().is_empty() {
                return Err(AppError::InvalidInput("The command failed".into()));
            }
            return Ok(out);
        }
        if started.elapsed() > TIMEOUT {
            let _ = child.kill();
            return Err(AppError::InvalidInput(
                "The command took longer than 10 seconds".into(),
            ));
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

async fn fetch(url: &str) -> AppResult<String> {
    let response = reqwest::Client::builder()
        .timeout(TIMEOUT)
        .user_agent("Notch-Desktop")
        .build()
        .map_err(|e| AppError::Platform(e.to_string()))?
        .get(url)
        .send()
        .await
        .map_err(|_| AppError::InvalidInput("Couldn't reach that URL".into()))?
        .error_for_status()
        .map_err(|e| {
            AppError::InvalidInput(format!(
                "The URL answered {}",
                e.status().map_or(0, |s| s.as_u16())
            ))
        })?;
    let bytes = response
        .bytes()
        .await
        .map_err(|_| AppError::InvalidInput("The answer was cut off".into()))?;
    Ok(String::from_utf8_lossy(&bytes[..bytes.len().min(MAX_BYTES)]).into_owned())
}

pub async fn load(config: &WidgetConfig) -> WidgetOutput {
    let raw = match config.kind.as_str() {
        "url" => fetch(&config.source).await,
        _ => {
            let source = config.source.clone();
            tauri::async_runtime::spawn_blocking(move || run_command(&source))
                .await
                .unwrap_or_else(|e| Err(AppError::Platform(e.to_string())))
        }
    };
    match raw {
        Ok(raw) => shape(config, &raw),
        Err(error) => {
            let mut out = shape(config, "");
            out.text = None;
            out.error = Some(error.to_string().replace("invalid input: ", ""));
            out
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> WidgetConfig {
        WidgetConfig {
            id: "w".into(),
            name: "Server".into(),
            kind: "url".into(),
            source: "https://example.com".into(),
            refresh_secs: 60,
        }
    }

    #[test]
    fn lays_out_the_widget_format() {
        let out = shape(
            &config(),
            r#"{"title":"api","value":"42 ms","subtitle":"healthy","items":[{"label":"CPU","value":12},{"label":"RAM"}]}"#,
        );
        assert_eq!(out.value.as_deref(), Some("42 ms"));
        assert_eq!(out.items.len(), 2);
        assert_eq!(out.items[0].value, "12");
    }

    #[test]
    fn shows_other_json_and_text_sensibly() {
        let out = shape(&config(), r#"{"status":"ok","users":3,"nested":{"x":1}}"#);
        assert_eq!(out.items.len(), 2);
        assert_eq!(shape(&config(), "17").value.as_deref(), Some("17"));
        let text = shape(&config(), "line 1\nline 2\n");
        assert_eq!(text.text.as_deref(), Some("line 1\nline 2"));
    }
}

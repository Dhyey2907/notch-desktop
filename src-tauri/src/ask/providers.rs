//! Which AI answers Ask, plans the day and writes commit messages: Claude Code (the default),
//! Codex, Gemini CLI — each signed in through its own app — a local Ollama, or any
//! OpenAI-compatible API (OpenAI, OpenRouter, Groq, Google's Gemini API, LM Studio…) with a
//! key kept in Windows Credential Manager. Nothing here is logged: not the question, the
//! answer, or the key.

use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

use base64::Engine;
use serde_json::{json, Value};

use crate::{
    claude::agents::{find_exe, Agent},
    error::{AppError, AppResult},
    secrets::{self, Credential},
    settings::AiSettings,
};

pub const TIMEOUT: Duration = Duration::from_secs(180);
const DEFAULT_OLLAMA: &str = "http://127.0.0.1:11434";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provider {
    Claude,
    Codex,
    Gemini,
    Ollama,
    OpenAi,
}

impl Provider {
    pub fn from_settings(ai: &AiSettings) -> Provider {
        match ai.provider.as_str() {
            "codex" => Provider::Codex,
            "gemini" => Provider::Gemini,
            "ollama" => Provider::Ollama,
            "openai" => Provider::OpenAi,
            _ => Provider::Claude,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Provider::Claude => "Claude",
            Provider::Codex => "Codex",
            Provider::Gemini => "Gemini",
            Provider::Ollama => "Ollama",
            Provider::OpenAi => "AI",
        }
    }

    pub fn sees_images(self) -> bool {
        !matches!(self, Provider::Gemini)
    }
}

/// Whether the chosen provider can be used right now (installed / set up).
pub fn available(ai: &AiSettings) -> bool {
    match Provider::from_settings(ai) {
        Provider::Claude => super::service::claude_exe().is_some(),
        Provider::Codex => find_exe(Agent::Codex).is_some(),
        Provider::Gemini => find_exe(Agent::Gemini).is_some(),
        Provider::Ollama => true,
        Provider::OpenAi => !ai.base_url.trim().is_empty() && !ai.model().is_empty(),
    }
}

/// One request: instructions, the user's text (with any earlier conversation), and maybe a
/// PNG of the window they're looking at.
pub struct Prompt<'a> {
    pub system: &'a str,
    pub text: &'a str,
    pub image: Option<&'a [u8]>,
}

fn invalid(message: impl Into<String>) -> AppError {
    AppError::InvalidInput(message.into())
}

// ---------- Command-line agents (Codex, Gemini CLI) ----------

/// A started CLI answer: the process (kept so Stop can end it) and where the answer lands.
pub struct CliRun {
    pub child: Child,
    /// Codex writes its final answer to a file; Gemini to stdout.
    pub answer_file: Option<PathBuf>,
}

fn hidden(command: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(windows::Win32::System::Threading::CREATE_NO_WINDOW.0);
    }
    #[cfg(not(windows))]
    let _ = command;
}

/// Start Codex or Gemini CLI on a prompt (read-only: they may not change anything).
pub fn start_cli(provider: Provider, ai: &AiSettings, prompt: &Prompt, workdir: &Path) -> AppResult<CliRun> {
    let _ = std::fs::create_dir_all(workdir);
    let input = format!("{}\n\n{}", prompt.system, prompt.text);
    let model = ai.model();
    let (mut command, answer_file) = match provider {
        Provider::Codex => {
            let exe = find_exe(Agent::Codex).ok_or_else(|| invalid("Codex isn't installed on this PC"))?;
            let out = workdir.join(format!("codex-answer-{}.txt", uuid::Uuid::new_v4()));
            let mut command = Command::new(exe);
            command.args([
                "exec",
                "--skip-git-repo-check",
                "--ephemeral",
                "--sandbox",
                "read-only",
                "--color",
                "never",
                // Quick questions: answer fast rather than think long, and skip the plugins and
                // MCP servers in the user's config (they halve the start-up time, unused here).
                "-c",
                "model_reasoning_effort=\"low\"",
                "--ignore-user-config",
            ]);
            command.arg("-C").arg(workdir).arg("-o").arg(&out);
            if !model.is_empty() {
                command.args(["-m", model]);
            }
            if let Some(png) = prompt.image {
                let image = workdir.join("ask-window.png");
                std::fs::write(&image, png)?;
                command.arg("-i").arg(image);
            }
            command.arg("-");
            (command, Some(out))
        }
        Provider::Gemini => {
            let exe =
                find_exe(Agent::Gemini).ok_or_else(|| invalid("Gemini CLI isn't installed on this PC"))?;
            let mut command = Command::new(exe);
            command.args([
                "--prompt",
                "Follow the instructions and answer the question given on standard input.",
                "--output-format",
                "text",
            ]);
            if !model.is_empty() {
                command.args(["--model", model]);
            }
            (command, None)
        }
        _ => return Err(invalid("not a command-line provider")),
    };
    command
        .current_dir(workdir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    hidden(&mut command);
    let mut child = command
        .spawn()
        .map_err(|_| invalid(format!("Couldn't start {}", provider.name())))?;
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(input.as_bytes());
    }
    Ok(CliRun { child, answer_file })
}

/// Wait for a CLI answer (stdout is drained so the process never blocks on it).
pub fn finish_cli(provider: Provider, mut run: CliRun, cancelled: &AtomicBool) -> AppResult<String> {
    let mut stdout = run.child.stdout.take();
    let reader = std::thread::spawn(move || {
        let mut s = String::new();
        if let Some(o) = stdout.as_mut() {
            let _ = o.read_to_string(&mut s);
        }
        s
    });
    let started = Instant::now();
    let status = loop {
        if let Some(status) = run.child.try_wait()? {
            break status;
        }
        if cancelled.load(Ordering::Relaxed) || started.elapsed() > TIMEOUT {
            let _ = run.child.kill();
            let _ = run.child.wait();
            if let Some(f) = &run.answer_file {
                let _ = std::fs::remove_file(f);
            }
            return Err(invalid(if cancelled.load(Ordering::Relaxed) {
                "Stopped"
            } else {
                "That took too long — try again"
            }));
        }
        std::thread::sleep(Duration::from_millis(100));
    };
    let printed = reader.join().unwrap_or_default();
    let text = match &run.answer_file {
        Some(file) => {
            let text = std::fs::read_to_string(file).unwrap_or_default();
            let _ = std::fs::remove_file(file);
            text
        }
        None => printed,
    };
    if !status.success() || text.trim().is_empty() {
        return Err(invalid(match provider {
            Provider::Codex => "Codex didn't answer. Is it signed in? Open the Codex app once.",
            _ => "Gemini CLI didn't answer. Is it signed in? Run `gemini` once in a terminal.",
        }));
    }
    Ok(text.trim().to_owned())
}

// ---------- HTTP (Ollama, OpenAI-compatible) ----------

/// The request body for an OpenAI-compatible `/chat/completions`.
pub fn openai_body(model: &str, prompt: &Prompt) -> Value {
    let user = match prompt.image {
        Some(png) => json!([
            { "type": "text", "text": prompt.text },
            { "type": "image_url", "image_url": {
                "url": format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(png))
            } }
        ]),
        None => json!(prompt.text),
    };
    json!({
        "model": model,
        "stream": true,
        "messages": [
            { "role": "system", "content": prompt.system },
            { "role": "user", "content": user },
        ],
    })
}

/// The request body for Ollama's `/api/chat`.
pub fn ollama_body(model: &str, prompt: &Prompt) -> Value {
    let mut user = json!({ "role": "user", "content": prompt.text });
    if let Some(png) = prompt.image {
        user["images"] = json!([base64::engine::general_purpose::STANDARD.encode(png)]);
    }
    json!({
        "model": model,
        "stream": true,
        "messages": [{ "role": "system", "content": prompt.system }, user],
    })
}

/// One server-sent-events line from `/chat/completions`: new text, end, or nothing.
#[derive(Debug, PartialEq, Eq)]
pub enum Piece {
    Text(String),
    Done,
    Nothing,
}

pub fn parse_sse_line(line: &str) -> Piece {
    let Some(data) = line.trim().strip_prefix("data:") else {
        return Piece::Nothing;
    };
    let data = data.trim();
    if data == "[DONE]" {
        return Piece::Done;
    }
    serde_json::from_str::<Value>(data)
        .ok()
        .and_then(|v| v["choices"][0]["delta"]["content"].as_str().map(str::to_owned))
        .filter(|t| !t.is_empty())
        .map_or(Piece::Nothing, Piece::Text)
}

/// One line of Ollama's streamed answer.
pub fn parse_ollama_line(line: &str) -> Piece {
    let Ok(v) = serde_json::from_str::<Value>(line.trim()) else {
        return Piece::Nothing;
    };
    if let Some(text) = v["message"]["content"].as_str().filter(|t| !t.is_empty()) {
        return Piece::Text(text.to_owned());
    }
    if v["done"].as_bool() == Some(true) {
        Piece::Done
    } else {
        Piece::Nothing
    }
}

/// A readable error from a failed API response body.
fn api_error(status: reqwest::StatusCode, body: &str) -> AppError {
    let detail = serde_json::from_str::<Value>(body).ok().and_then(|v| {
        v["error"]["message"]
            .as_str()
            .or_else(|| v["error"].as_str())
            .map(str::to_owned)
    });
    let hint = match status.as_u16() {
        401 | 403 => "the API key was refused — check it in Settings → AI",
        404 => "that model or address wasn't found — check Settings → AI",
        429 => "the provider says you've hit a rate limit or run out of credit",
        _ => "the provider returned an error",
    };
    invalid(match detail {
        Some(d) => format!("{} ({})", hint, d.chars().take(160).collect::<String>()),
        None => hint.to_owned(),
    })
}

fn client() -> AppResult<reqwest::Client> {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(TIMEOUT)
        .build()
        .map_err(|e| AppError::Platform(e.to_string()))
}

/// Ollama's base address and the model to use (the first installed one when none is set).
async fn ollama_model(http: &reqwest::Client, ai: &AiSettings) -> AppResult<(String, String)> {
    let base = if ai.base_url.trim().is_empty() || Provider::from_settings(ai) != Provider::Ollama {
        DEFAULT_OLLAMA.to_owned()
    } else {
        ai.base_url.trim().trim_end_matches('/').to_owned()
    };
    if !ai.model().is_empty() {
        return Ok((base, ai.model().to_owned()));
    }
    let tags: Value = http
        .get(format!("{base}/api/tags"))
        .send()
        .await
        .map_err(|_| invalid("Ollama isn't running — start it, or pick another AI in Settings → AI"))?
        .json()
        .await
        .map_err(|_| invalid("Ollama didn't answer properly"))?;
    let model = tags["models"][0]["name"]
        .as_str()
        .ok_or_else(|| invalid("Ollama has no models yet — run `ollama pull llama3.2` once"))?;
    Ok((base, model.to_owned()))
}

/// Names of the models installed in a local Ollama.
pub async fn ollama_models() -> Vec<String> {
    let Ok(http) = client() else { return Vec::new() };
    let Ok(response) = http
        .get(format!("{DEFAULT_OLLAMA}/api/tags"))
        .timeout(Duration::from_secs(3))
        .send()
        .await
    else {
        return Vec::new();
    };
    let tags: Value = response.json().await.unwrap_or_default();
    tags["models"]
        .as_array()
        .map(|models| {
            models
                .iter()
                .filter_map(|m| m["name"].as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

/// Stream an answer from Ollama or an OpenAI-compatible API, calling `on_text` per piece.
pub async fn stream_http(
    ai: &AiSettings,
    prompt: &Prompt<'_>,
    on_text: &mut (dyn FnMut(&str) + Send),
    cancelled: &AtomicBool,
) -> AppResult<()> {
    let http = client()?;
    let provider = Provider::from_settings(ai);
    let (request, ollama) = match provider {
        Provider::Ollama => {
            let (base, model) = ollama_model(&http, ai).await?;
            (
                http.post(format!("{base}/api/chat"))
                    .json(&ollama_body(&model, prompt)),
                true,
            )
        }
        Provider::OpenAi => {
            let base = ai.base_url.trim().trim_end_matches('/');
            if base.is_empty() || ai.model().is_empty() {
                return Err(invalid("Set the API address and model in Settings → AI"));
            }
            let mut request = http
                .post(format!("{base}/chat/completions"))
                .json(&openai_body(ai.model(), prompt));
            if let Some(key) = secrets::get(Credential::AiApiKey)?.filter(|k| !k.is_empty()) {
                request = request.bearer_auth(key);
            }
            (request, false)
        }
        _ => return Err(invalid("not an API provider")),
    };
    let mut response = request.send().await.map_err(|_| {
        invalid(format!(
            "Couldn't reach {}",
            if ollama { "Ollama" } else { "the API" }
        ))
    })?;
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(api_error(status, &body));
    }
    let mut pending = String::new();
    while let Some(bytes) = response
        .chunk()
        .await
        .map_err(|_| invalid("The answer was cut off — try again"))?
    {
        if cancelled.load(Ordering::Relaxed) {
            return Err(invalid("Stopped"));
        }
        pending.push_str(&String::from_utf8_lossy(&bytes));
        while let Some(at) = pending.find('\n') {
            let line: String = pending.drain(..=at).collect();
            let piece = if ollama {
                parse_ollama_line(&line)
            } else {
                parse_sse_line(&line)
            };
            match piece {
                Piece::Text(text) => on_text(&text),
                Piece::Done => return Ok(()),
                Piece::Nothing => {}
            }
        }
    }
    Ok(())
}

/// Answer once, as plain text, with whichever AI is chosen (day plans, commit messages).
pub fn answer(ai: &AiSettings, system: &str, input: &str, workdir: &Path) -> AppResult<String> {
    let provider = Provider::from_settings(ai);
    let prompt = Prompt {
        system,
        text: input,
        image: None,
    };
    let never = AtomicBool::new(false);
    match provider {
        Provider::Claude => super::once::answer(system, input, workdir),
        Provider::Codex | Provider::Gemini => {
            let run = start_cli(provider, ai, &prompt, workdir)?;
            finish_cli(provider, run, &never)
        }
        Provider::Ollama | Provider::OpenAi => {
            let mut text = String::new();
            tauri::async_runtime::block_on(stream_http(ai, &prompt, &mut |t| text.push_str(t), &never))?;
            if text.trim().is_empty() {
                return Err(invalid(format!("{} gave an empty answer", provider.name())));
            }
            Ok(text.trim().to_owned())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prompt<'a>(image: Option<&'a [u8]>) -> Prompt<'a> {
        Prompt {
            system: "Be brief.",
            text: "Hi?",
            image,
        }
    }

    #[test]
    fn picks_the_provider_from_settings() {
        let ai = |p: &str| AiSettings {
            provider: p.into(),
            ..Default::default()
        };
        assert_eq!(Provider::from_settings(&ai("codex")), Provider::Codex);
        assert_eq!(Provider::from_settings(&ai("openai")), Provider::OpenAi);
        assert_eq!(Provider::from_settings(&ai("")), Provider::Claude);
        assert_eq!(Provider::from_settings(&ai("nonsense")), Provider::Claude);
        assert!(!available(&AiSettings {
            provider: "openai".into(),
            ..Default::default()
        }));
    }

    #[test]
    fn builds_api_requests_with_and_without_a_picture() {
        let plain = openai_body("gpt-x", &prompt(None));
        assert_eq!(plain["model"], "gpt-x");
        assert_eq!(plain["stream"], true);
        assert_eq!(plain["messages"][0]["content"], "Be brief.");
        assert_eq!(plain["messages"][1]["content"], "Hi?");
        let pic = openai_body("gpt-x", &prompt(Some(b"png")));
        assert_eq!(
            pic["messages"][1]["content"][1]["image_url"]["url"],
            "data:image/png;base64,cG5n"
        );
        let ollama = ollama_body("llama", &prompt(Some(b"png")));
        assert_eq!(ollama["messages"][1]["images"][0], "cG5n");
        assert!(ollama_body("llama", &prompt(None))["messages"][1]
            .get("images")
            .is_none());
    }

    #[test]
    fn reads_streamed_answers() {
        assert_eq!(
            parse_sse_line(r#"data: {"choices":[{"delta":{"content":"Hel"}}]}"#),
            Piece::Text("Hel".into())
        );
        assert_eq!(parse_sse_line("data: [DONE]"), Piece::Done);
        assert_eq!(
            parse_sse_line(r#"data: {"choices":[{"delta":{"role":"assistant"}}]}"#),
            Piece::Nothing
        );
        assert_eq!(parse_sse_line(": keep-alive"), Piece::Nothing);
        assert_eq!(
            parse_ollama_line(r#"{"message":{"role":"assistant","content":"lo"},"done":false}"#),
            Piece::Text("lo".into())
        );
        assert_eq!(
            parse_ollama_line(r#"{"message":{"role":"assistant","content":""},"done":true}"#),
            Piece::Done
        );
        assert_eq!(parse_ollama_line("garbage"), Piece::Nothing);
    }

    #[test]
    fn explains_api_errors() {
        let e = api_error(
            reqwest::StatusCode::UNAUTHORIZED,
            r#"{"error":{"message":"bad key"}}"#,
        );
        assert!(e.to_string().contains("API key was refused"));
        assert!(e.to_string().contains("bad key"));
        assert!(api_error(reqwest::StatusCode::TOO_MANY_REQUESTS, "")
            .to_string()
            .contains("rate limit"));
    }
}

//! "Ask" in the notch: questions answered by the user's installed Claude Code, run headless
//! (`claude -p`, stream-json in and out) with no tools, no MCP servers, no settings or hooks,
//! and no saved session. Questions, answers and screenshots are kept in memory only and never
//! logged.

#[cfg(windows)]
pub mod capture;
pub mod once;
pub mod providers;
pub mod service;

use base64::Engine;
use serde::Serialize;
use serde_json::{json, Value};
use ts_rs::TS;

/// Earlier exchanges sent along for follow-up questions.
pub const MAX_HISTORY: usize = 5;
pub const MAX_QUESTION: usize = 4000;
/// Older answers are trimmed when sent back as context.
const MAX_CONTEXT_ANSWER: usize = 2000;

pub const SYSTEM_PROMPT: &str = "You answer quick questions inside a small panel at the top of the user's \
Windows desktop. Be concise: lead with the answer, use short paragraphs or bullet lists, and use code blocks \
only for code. If a screenshot of a window is attached, it shows what the user is looking at right now.";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Exchange {
    pub id: String,
    pub question: String,
    pub answer: String,
    /// The answer is still streaming in.
    pub pending: bool,
    /// A window screenshot went with the question.
    pub with_screen: bool,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// One line of `--output-format stream-json`.
#[derive(Debug, PartialEq, Eq)]
pub enum Chunk {
    Text(String),
    Done,
    Failed(String),
    Other,
}

pub fn parse_line(line: &str) -> Chunk {
    let Ok(json) = serde_json::from_str::<Value>(line) else {
        return Chunk::Other;
    };
    match json.get("type").and_then(Value::as_str) {
        Some("stream_event") => {
            let event = &json["event"];
            if event["type"] == "content_block_delta" && event["delta"]["type"] == "text_delta" {
                if let Some(text) = event["delta"]["text"].as_str() {
                    return Chunk::Text(text.to_owned());
                }
            }
            Chunk::Other
        }
        Some("result") => {
            if json["is_error"].as_bool() == Some(true) || json["subtype"] != "success" {
                let message = json["result"]
                    .as_str()
                    .filter(|m| !m.is_empty())
                    .unwrap_or("Claude Code couldn't answer");
                Chunk::Failed(message.chars().take(300).collect())
            } else {
                Chunk::Done
            }
        }
        _ => Chunk::Other,
    }
}

/// The stream-json user message: earlier exchanges as context, the question, and optionally
/// a PNG of the window the user was looking at.
pub fn user_message(history: &[Exchange], question: &str, screenshot_png: Option<&[u8]>) -> String {
    let text = context_text(history, question);
    let mut content = vec![json!({ "type": "text", "text": text })];
    if let Some(png) = screenshot_png {
        content.push(json!({
            "type": "image",
            "source": {
                "type": "base64",
                "media_type": "image/png",
                "data": base64::engine::general_purpose::STANDARD.encode(png),
            }
        }));
    }
    json!({ "type": "user", "message": { "role": "user", "content": content } }).to_string()
}

/// The question with the last few exchanges before it, as plain text.
pub fn context_text(history: &[Exchange], question: &str) -> String {
    let mut text = String::new();
    let earlier: Vec<&Exchange> = history
        .iter()
        .filter(|e| !e.pending && e.error.is_none())
        .rev()
        .take(MAX_HISTORY)
        .collect();
    if !earlier.is_empty() {
        text.push_str("Earlier in this conversation:\n");
        for e in earlier.into_iter().rev() {
            let answer: String = e.answer.chars().take(MAX_CONTEXT_ANSWER).collect();
            text.push_str(&format!("\nQ: {}\nA: {}\n", e.question, answer));
        }
        text.push_str("\nNow answer this:\n");
    }
    text.push_str(question);
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_streamed_text_and_the_result() {
        let delta = r#"{"type":"stream_event","event":{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hel"}}}"#;
        assert_eq!(parse_line(delta), Chunk::Text("Hel".into()));
        assert_eq!(
            parse_line(r#"{"type":"result","subtype":"success","is_error":false,"result":"Hello"}"#),
            Chunk::Done
        );
        assert_eq!(
            parse_line(
                r#"{"type":"result","subtype":"error_during_execution","is_error":true,"result":"Not logged in"}"#
            ),
            Chunk::Failed("Not logged in".into())
        );
        assert_eq!(parse_line(r#"{"type":"system","subtype":"init"}"#), Chunk::Other);
        assert_eq!(parse_line("not json"), Chunk::Other);
    }

    #[test]
    fn builds_the_message_with_history_and_image() {
        let history = vec![
            Exchange {
                id: "1".into(),
                question: "What is 2+2?".into(),
                answer: "4".into(),
                pending: false,
                with_screen: false,
                error: None,
            },
            Exchange {
                id: "2".into(),
                question: "failed one".into(),
                answer: String::new(),
                pending: false,
                with_screen: false,
                error: Some("x".into()),
            },
        ];
        let msg: Value = serde_json::from_str(&user_message(&history, "And times 3?", Some(b"png"))).unwrap();
        let content = msg["message"]["content"].as_array().unwrap();
        let text = content[0]["text"].as_str().unwrap();
        assert!(text.contains("Q: What is 2+2?\nA: 4"));
        assert!(!text.contains("failed one"), "failed exchanges aren't context");
        assert!(text.ends_with("And times 3?"));
        assert_eq!(content[1]["source"]["data"], "cG5n");
        let plain: Value = serde_json::from_str(&user_message(&[], "Hi", None)).unwrap();
        assert_eq!(plain["message"]["content"][0]["text"], "Hi");
        assert_eq!(plain["message"]["content"].as_array().unwrap().len(), 1);
    }
}

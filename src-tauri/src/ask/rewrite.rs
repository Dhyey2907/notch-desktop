//! Rewrite selection: select text in any app, press the shortcut, pick a style (formal,
//! shorter, friendlier, fix grammar, or your own instruction) and the AI's version replaces
//! the selection. The copy Overline makes to read your selection, and the paste back, stay
//! out of clipboard history, and whatever you had copied before is put back afterwards.

use std::sync::Mutex;

use serde::Serialize;
use ts_rs::TS;

use crate::error::{AppError, AppResult};

/// The longest selection sent to the AI.
pub const MAX_CHARS: usize = 8_000;

pub const SYSTEM: &str = "You rewrite text. Reply with only the rewritten text: no quotes, no preamble, \
no explanation, no markdown fences. Keep the original language, meaning, names, numbers and links, and \
keep line breaks and list formatting.";

/// The selection being rewritten.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RewriteSession {
    pub text: String,
    /// The app it came from ("Chrome").
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app: Option<String>,
}

struct Session {
    target: isize,
    previous: Option<String>,
    shown: RewriteSession,
}

static SESSION: Mutex<Option<Session>> = Mutex::new(None);

/// What to ask for each style.
pub fn instruction(style: &str, custom: &str) -> Option<String> {
    let text = match style {
        "formal" => "Rewrite it in a formal, professional tone.",
        "shorter" => "Make it shorter and clearer, keeping everything important.",
        "friendlier" => "Rewrite it in a warm, friendly tone.",
        "fix" => "Fix grammar, spelling and punctuation only. Change nothing else.",
        "custom" => {
            let c = custom.trim();
            if c.is_empty() || c.len() > 300 {
                return None;
            }
            return Some(c.to_string());
        }
        _ => return None,
    };
    Some(text.to_string())
}

/// The model sometimes wraps its answer anyway: strip quotes and fences around it.
pub fn clean(answer: &str) -> String {
    let mut t = answer.trim();
    if let Some(inner) = t.strip_prefix("```").and_then(|r| r.strip_suffix("```")) {
        t = inner
            .split_once('\n')
            .map_or(inner, |(first, rest)| {
                if first.trim().chars().all(|c| c.is_ascii_alphanumeric()) {
                    rest
                } else {
                    inner
                }
            })
            .trim();
    }
    for (open, close) in [('"', '"'), ('“', '”'), ('\'', '\'')] {
        if t.len() > 2
            && t.starts_with(open)
            && t.ends_with(close)
            && t[open.len_utf8()..t.len() - close.len_utf8()]
                .find(close)
                .is_none()
        {
            t = &t[open.len_utf8()..t.len() - close.len_utf8()];
        }
    }
    t.to_string()
}

pub fn session() -> Option<RewriteSession> {
    SESSION
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .as_ref()
        .map(|s| s.shown.clone())
}

pub fn cancel() {
    #[cfg(windows)]
    if let Some(s) = SESSION.lock().unwrap_or_else(|e| e.into_inner()).take() {
        restore_clipboard(s.previous);
    }
    #[cfg(not(windows))]
    SESSION.lock().unwrap_or_else(|e| e.into_inner()).take();
}

#[cfg(windows)]
fn restore_clipboard(previous: Option<String>) {
    if let Some(text) = previous {
        crate::clipboard::ignore_copies_for(1500);
        let _ = crate::clipboard::win::write(crate::clipboard::win::Put::Text(&text));
    }
}

/// Copy the selection in the window in front (`target`) and start a session. Call before the
/// notch takes focus.
#[cfg(windows)]
pub fn capture(target: isize, app: Option<String>) -> AppResult<RewriteSession> {
    use crate::clipboard::win;
    let previous = win::read_text();
    let before = win::sequence();
    crate::clipboard::ignore_copies_for(2500);
    win::press_copy();
    // Wait for the app to put the selection on the clipboard.
    let waited = std::time::Instant::now();
    while win::sequence() == before && waited.elapsed() < std::time::Duration::from_millis(900) {
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    let text = if win::sequence() == before {
        String::new()
    } else {
        win::read_text().unwrap_or_default()
    };
    // Put back what was copied before.
    restore_clipboard(previous.clone());
    let text = text.trim_end_matches(['\r', '\n']).to_string();
    if text.trim().is_empty() {
        return Err(AppError::InvalidInput(
            "Select some text first, then press the shortcut".into(),
        ));
    }
    if text.chars().count() > MAX_CHARS {
        return Err(AppError::InvalidInput(
            "That's a lot of text — select a shorter part".into(),
        ));
    }
    let shown = RewriteSession { text, app };
    *SESSION.lock().unwrap_or_else(|e| e.into_inner()) = Some(Session {
        target,
        previous,
        shown: shown.clone(),
    });
    Ok(shown)
}

/// Replace the selection with `text`: back to the app, paste, then put the old clipboard back.
#[cfg(windows)]
pub fn apply(text: &str) -> AppResult<()> {
    use crate::clipboard::win;
    use windows::Win32::{Foundation::HWND, UI::WindowsAndMessaging::SetForegroundWindow};
    let s = SESSION
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .take()
        .ok_or_else(|| AppError::InvalidInput("Nothing to replace".into()))?;
    crate::clipboard::ignore_copies_for(3000);
    win::write(win::Put::PrivateText(text))?;
    // SAFETY: brings back the window the text came from; it may refuse, then the paste goes
    // wherever focus is (still the same window in practice).
    unsafe {
        let _ = SetForegroundWindow(HWND(s.target as *mut _));
    }
    std::thread::sleep(std::time::Duration::from_millis(250));
    win::press_paste();
    std::thread::sleep(std::time::Duration::from_millis(600));
    restore_clipboard(s.previous);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn styles_and_custom_instructions() {
        assert!(instruction("formal", "").unwrap().contains("formal"));
        assert_eq!(
            instruction("custom", "  make it rhyme "),
            Some("make it rhyme".into())
        );
        assert_eq!(instruction("custom", "   "), None);
        assert_eq!(instruction("nope", ""), None);
    }

    #[test]
    fn strips_wrapping_the_model_added() {
        assert_eq!(clean("\"Hello there.\""), "Hello there.");
        assert_eq!(clean("```\nHello\nthere\n```"), "Hello\nthere");
        assert_eq!(clean("```text\nHi\n```"), "Hi");
        assert_eq!(clean("He said \"hi\" and left"), "He said \"hi\" and left");
        assert_eq!(clean("\"a\" and \"b\""), "\"a\" and \"b\"");
    }
}

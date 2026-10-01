//! A vault login as an Obsidian note: YAML front matter with the fields (the password
//! encrypted), and the notes as the note's body. Readable and searchable in Obsidian; the
//! password is only ever written as `enc:v1:…`.

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm,
};
use base64::{engine::general_purpose::STANDARD, Engine};

use crate::error::{AppError, AppResult};

/// Marks a note as a Notch vault entry (other notes in the folder are left alone).
pub const MARKER: &str = "notch-vault";
const PREFIX: &str = "enc:v1:";

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Note {
    pub title: String,
    pub username: String,
    pub url: String,
    /// `enc:v1:…`, or empty.
    pub password: String,
    pub tags: Vec<String>,
    pub notes: String,
}

/// YAML scalar: plain when safe, otherwise double-quoted with escapes.
fn scalar(value: &str) -> String {
    let plain = !value.is_empty()
        && !value.starts_with(|c: char| "-?:,[]{}#&*!|>'\"%@` ".contains(c))
        && !value.contains(": ")
        && !value.contains(" #")
        && !value.ends_with([':', ' '])
        && !value.contains(['\n', '\r', '"', '\\'])
        && !["true", "false", "null", "yes", "no", "~"].contains(&value.to_lowercase().as_str())
        && value.parse::<f64>().is_err();
    if plain {
        return value.to_owned();
    }
    let escaped = value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "");
    format!("\"{escaped}\"")
}

fn unscalar(raw: &str) -> String {
    let raw = raw.trim();
    if raw.len() >= 2 && raw.starts_with('"') && raw.ends_with('"') {
        let inner = &raw[1..raw.len() - 1];
        let mut out = String::new();
        let mut chars = inner.chars();
        while let Some(c) = chars.next() {
            if c == '\\' {
                match chars.next() {
                    Some('n') => out.push('\n'),
                    Some('t') => out.push('\t'),
                    Some(other) => out.push(other),
                    None => {}
                }
            } else {
                out.push(c);
            }
        }
        return out;
    }
    if raw.len() >= 2 && raw.starts_with('\'') && raw.ends_with('\'') {
        return raw[1..raw.len() - 1].replace("''", "'");
    }
    raw.to_owned()
}

pub fn render(note: &Note) -> String {
    let mut out = String::from("---\n");
    out.push_str(&format!("{MARKER}: 1\n"));
    out.push_str(&format!("title: {}\n", scalar(&note.title)));
    if !note.username.is_empty() {
        out.push_str(&format!("username: {}\n", scalar(&note.username)));
    }
    if !note.url.is_empty() {
        out.push_str(&format!("url: {}\n", scalar(&note.url)));
    }
    if !note.password.is_empty() {
        out.push_str(&format!("password: {}\n", scalar(&note.password)));
    }
    if !note.tags.is_empty() {
        out.push_str("tags:\n");
        for tag in &note.tags {
            out.push_str(&format!("  - {}\n", scalar(tag)));
        }
    }
    out.push_str("---\n");
    if !note.notes.is_empty() {
        out.push('\n');
        out.push_str(note.notes.trim_end());
        out.push('\n');
    }
    out
}

/// Read a note; `None` if it isn't a Notch vault entry.
pub fn parse(text: &str) -> Option<Note> {
    let text = text
        .strip_prefix('\u{feff}')
        .unwrap_or(text)
        .replace("\r\n", "\n");
    let rest = text.strip_prefix("---\n")?;
    let end = rest.find("\n---")?;
    let (front, body) = (&rest[..end], &rest[end + 4..]);
    let mut note = Note::default();
    let mut is_vault = false;
    let mut in_tags = false;
    for line in front.lines() {
        if in_tags {
            if let Some(tag) = line.trim_start().strip_prefix("- ") {
                note.tags.push(unscalar(tag));
                continue;
            }
            in_tags = false;
        }
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let value = value.trim();
        match key.trim() {
            MARKER => is_vault = true,
            "title" => note.title = unscalar(value),
            "username" => note.username = unscalar(value),
            "url" => note.url = unscalar(value),
            "password" => note.password = unscalar(value),
            "tags" if value.is_empty() => in_tags = true,
            "tags" => {
                note.tags = value
                    .trim_start_matches('[')
                    .trim_end_matches(']')
                    .split(',')
                    .map(unscalar)
                    .filter(|t| !t.is_empty())
                    .collect();
            }
            _ => {}
        }
    }
    note.notes = body.trim_start_matches('\n').trim_end().to_owned();
    is_vault.then_some(note)
}

pub fn encrypt(key: &[u8; 32], plaintext: &str) -> AppResult<String> {
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| AppError::Platform("bad vault key".into()))?;
    let mut nonce = [0u8; 12];
    rand::fill(&mut nonce);
    let sealed = cipher
        .encrypt(&nonce.into(), plaintext.as_bytes())
        .map_err(|_| AppError::Platform("could not encrypt".into()))?;
    let mut bytes = nonce.to_vec();
    bytes.extend(sealed);
    Ok(format!("{PREFIX}{}", STANDARD.encode(bytes)))
}

pub fn decrypt(key: &[u8; 32], sealed: &str) -> AppResult<String> {
    let wrong = || {
        AppError::InvalidInput("This password was saved with a different vault key (from another PC?)".into())
    };
    let bytes = STANDARD
        .decode(sealed.strip_prefix(PREFIX).ok_or_else(wrong)?)
        .map_err(|_| wrong())?;
    if bytes.len() < 12 + 16 {
        return Err(wrong());
    }
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| wrong())?;
    let nonce: [u8; 12] = bytes[..12].try_into().map_err(|_| wrong())?;
    let plain = cipher.decrypt(&nonce.into(), &bytes[12..]).map_err(|_| wrong())?;
    String::from_utf8(plain).map_err(|_| wrong())
}

/// A file name for a title: Windows-safe, never a path.
pub fn file_name(title: &str) -> String {
    let clean: String = title
        .chars()
        .map(|c| {
            if "<>:\"/\\|?*".contains(c) || c.is_control() {
                '-'
            } else {
                c
            }
        })
        .collect();
    let clean = clean.trim().trim_matches('.').trim();
    let clean: String = clean.chars().take(80).collect();
    if clean.is_empty() {
        "Login.md".into()
    } else {
        format!("{clean}.md")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Note {
        Note {
            title: "GitHub: work".into(),
            username: "dhyey@example.com".into(),
            url: "https://github.com/login".into(),
            password: "enc:v1:abc".into(),
            tags: vec!["dev".into(), "work stuff".into()],
            notes: "Recovery codes are in the safe.\nSecond line.".into(),
        }
    }

    #[test]
    fn round_trips_notes() {
        let note = sample();
        let text = render(&note);
        assert!(text.contains("title: \"GitHub: work\""));
        assert!(!text.contains("hunter2"));
        assert_eq!(parse(&text), Some(note));
        let odd = Note {
            title: "yes".into(),
            username: "\"quoted\" \\ back".into(),
            ..Default::default()
        };
        assert_eq!(parse(&render(&odd)), Some(odd));
    }

    #[test]
    fn reads_obsidian_edits_and_ignores_other_notes() {
        let edited = "---\r\nnotch-vault: 1\r\ntitle: 'Bank'\r\ntags: [money, home]\r\nusername: me\r\n---\r\nPIN hint\r\n";
        let note = parse(edited).unwrap();
        assert_eq!(note.title, "Bank");
        assert_eq!(note.tags, vec!["money", "home"]);
        assert_eq!(note.notes, "PIN hint");
        assert!(parse("---\ntitle: Groceries\n---\nmilk").is_none());
        assert!(parse("# Just a note").is_none());
    }

    #[test]
    fn encrypts_so_only_the_key_opens_it() {
        let key = [7u8; 32];
        let sealed = encrypt(&key, "correct horse battery staple").unwrap();
        assert!(sealed.starts_with("enc:v1:"));
        assert!(!sealed.contains("horse"));
        assert_ne!(sealed, encrypt(&key, "correct horse battery staple").unwrap());
        assert_eq!(decrypt(&key, &sealed).unwrap(), "correct horse battery staple");
        assert!(decrypt(&[8u8; 32], &sealed).is_err());
        assert!(decrypt(&key, "enc:v1:AAAA").is_err());
        assert!(decrypt(&key, "plain").is_err());
    }

    #[test]
    fn makes_safe_file_names() {
        assert_eq!(file_name("GitHub: work"), "GitHub- work.md");
        assert_eq!(file_name("../../etc"), "-..-etc.md");
        assert_eq!(file_name("  ...  "), "Login.md");
    }
}

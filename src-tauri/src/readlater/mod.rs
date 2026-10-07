//! Read-later shelf: links you drop or paste, with the page's title, site, reading time and
//! (when you ask) a short AI summary. Pages are fetched once when saved; only the title, word
//! count and summary are kept, never the page itself. Links and titles are never logged.

pub mod service;

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use ts_rs::TS;
use url::Url;

use crate::error::{AppError, AppResult};

pub const MAX_ITEMS: usize = 200;
const MAX_URL_CHARS: usize = 2048;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ReadItem {
    pub id: String,
    pub url: String,
    pub title: String,
    /// "theverge.com" or the site's own name.
    pub site: String,
    /// Words in the article, when it's a web page; the UI turns it into minutes.
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub words: Option<u32>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    pub added_at: DateTime<Utc>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ReadLaterSnapshot {
    /// Unread first (newest first), then read.
    pub items: Vec<ReadItem>,
    /// Links being fetched right now.
    pub saving: u32,
    /// Ids being summarized right now.
    pub summarizing: Vec<String>,
}

/// Only http(s) links, without credentials in them.
pub fn check_url(raw: &str) -> AppResult<Url> {
    let raw = raw.trim();
    let bad = || AppError::InvalidInput("That isn't a web link".into());
    if raw.chars().count() > MAX_URL_CHARS {
        return Err(bad());
    }
    let url = Url::parse(raw).map_err(|_| bad())?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() || !url.username().is_empty() {
        return Err(bad());
    }
    Ok(url)
}

/// "www.theverge.com" → "theverge.com".
pub fn site_of(url: &Url) -> String {
    let host = url.host_str().unwrap_or_default();
    host.strip_prefix("www.").unwrap_or(host).to_owned()
}

/// What a page says about itself.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct PageInfo {
    pub title: Option<String>,
    pub site: Option<String>,
    /// The readable text (scripts, styles and tags removed).
    pub text: String,
}

fn decode(text: &str) -> String {
    let mut out = text
        .replace("&nbsp;", " ")
        .replace("&#39;", "'")
        .replace("&#x27;", "'")
        .replace("&apos;", "'")
        .replace("&quot;", "\"")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&mdash;", "—")
        .replace("&ndash;", "–")
        .replace("&hellip;", "…")
        .replace("&rsquo;", "’")
        .replace("&lsquo;", "‘")
        .replace("&ldquo;", "“")
        .replace("&rdquo;", "”");
    out = out.replace("&amp;", "&");
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Content of a `<meta property|name="key" content="...">`.
fn meta(html: &str, lower: &str, key: &str) -> Option<String> {
    let mut from = 0;
    while let Some(pos) = lower[from..].find("<meta") {
        let start = from + pos;
        let end = start + lower[start..].find('>')?;
        let tag = &lower[start..end];
        if tag.contains(&format!("property=\"{key}\"")) || tag.contains(&format!("name=\"{key}\"")) {
            let c = tag.find("content=\"")? + "content=\"".len();
            let close = tag[c..].find('"')?;
            let value = decode(&html[start + c..start + c + close]);
            if !value.is_empty() {
                return Some(value);
            }
        }
        from = end;
    }
    None
}

/// Remove `<tag ...>...</tag>` blocks (scripts, styles…).
fn drop_blocks(html: &str, tag: &str) -> String {
    let lower = html.to_ascii_lowercase();
    let (open, close) = (format!("<{tag}"), format!("</{tag}"));
    let mut out = String::with_capacity(html.len());
    let mut at = 0;
    while let Some(pos) = lower[at..].find(&open) {
        let start = at + pos;
        out.push_str(&html[at..start]);
        match lower[start..].find(&close) {
            Some(end) => {
                let after = start + end;
                at = after + lower[after..].find('>').map_or(lower.len() - after, |g| g + 1);
            }
            None => {
                at = html.len();
                break;
            }
        }
    }
    if at < html.len() {
        out.push_str(&html[at..]);
    }
    out
}

/// Title, site name and readable text from a page's HTML. Treated as untrusted input.
pub fn read_page(html: &str) -> PageInfo {
    let lower = html.to_ascii_lowercase();
    let title = meta(html, &lower, "og:title").or_else(|| {
        let s = lower.find("<title")?;
        let s = s + lower[s..].find('>')? + 1;
        let e = s + lower[s..].find("</title")?;
        Some(decode(&html[s..e])).filter(|t| !t.is_empty())
    });
    let site = meta(html, &lower, "og:site_name");
    // Prefer the article, if the page marks one.
    let body = match (lower.find("<article"), lower.rfind("</article")) {
        (Some(s), Some(e)) if e > s => &html[s..e],
        _ => match lower.find("<body") {
            Some(s) => &html[s..],
            None => html,
        },
    };
    let mut text = body.to_owned();
    for tag in [
        "script", "style", "noscript", "svg", "nav", "footer", "header", "form",
    ] {
        text = drop_blocks(&text, tag);
    }
    let mut plain = String::with_capacity(text.len());
    let mut in_tag = false;
    for c in text.chars() {
        match c {
            '<' => {
                in_tag = true;
                plain.push(' ');
            }
            '>' => in_tag = false,
            _ if !in_tag => plain.push(c),
            _ => {}
        }
    }
    PageInfo {
        title: title.map(|t| t.chars().take(300).collect()),
        site: site.map(|s| s.chars().take(80).collect()),
        text: decode(&plain),
    }
}

pub fn word_count(text: &str) -> u32 {
    text.split_whitespace()
        .filter(|w| w.chars().any(char::is_alphanumeric))
        .count() as u32
}

fn row(r: &rusqlite::Row) -> rusqlite::Result<ReadItem> {
    let time = |s: Option<String>| {
        s.and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
            .map(|t| t.with_timezone(&Utc))
    };
    Ok(ReadItem {
        id: r.get(0)?,
        url: r.get(1)?,
        title: r.get(2)?,
        site: r.get(3)?,
        words: r
            .get::<_, Option<i64>>(4)?
            .map(|w| w.clamp(0, u32::MAX as i64) as u32),
        summary: r.get(5)?,
        added_at: time(r.get(6)?).unwrap_or_default(),
        read_at: time(r.get(7)?),
    })
}

pub fn list(conn: &Connection) -> AppResult<Vec<ReadItem>> {
    let mut stmt = conn.prepare(
        "SELECT id, url, title, site, words, summary, added_at, read_at FROM read_later
         ORDER BY read_at IS NOT NULL, COALESCE(read_at, added_at) DESC",
    )?;
    let items = stmt.query_map([], row)?.collect::<Result<Vec<_>, _>>()?;
    Ok(items)
}

pub fn get(conn: &Connection, id: &str) -> AppResult<Option<ReadItem>> {
    Ok(conn
        .query_row(
            "SELECT id, url, title, site, words, summary, added_at, read_at FROM read_later WHERE id = ?1",
            [id],
            row,
        )
        .optional()?)
}

pub fn exists(conn: &Connection, url: &str) -> AppResult<bool> {
    Ok(conn
        .query_row("SELECT 1 FROM read_later WHERE url = ?1", [url], |_| Ok(()))
        .optional()?
        .is_some())
}

/// Save a link. A link already saved moves back to the top, unread.
pub fn insert(
    conn: &Connection,
    url: &Url,
    title: &str,
    site: &str,
    words: Option<u32>,
    now: DateTime<Utc>,
) -> AppResult<ReadItem> {
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM read_later", [], |r| r.get(0))?;
    if count as usize >= MAX_ITEMS && !exists(conn, url.as_str())? {
        return Err(AppError::InvalidInput(format!(
            "The list holds {MAX_ITEMS} links — clear some read ones first"
        )));
    }
    conn.execute(
        "INSERT INTO read_later (id, url, title, site, words, added_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(url) DO UPDATE SET title = excluded.title, site = excluded.site,
             words = excluded.words, added_at = excluded.added_at, read_at = NULL",
        params![
            uuid::Uuid::new_v4().to_string(),
            url.as_str(),
            title,
            site,
            words,
            now.to_rfc3339()
        ],
    )?;
    let id: String = conn.query_row("SELECT id FROM read_later WHERE url = ?1", [url.as_str()], |r| {
        r.get(0)
    })?;
    get(conn, &id)?.ok_or_else(|| AppError::Platform("saved link vanished".into()))
}

pub fn set_read(conn: &Connection, id: &str, read: bool, now: DateTime<Utc>) -> AppResult<()> {
    let at = read.then(|| now.to_rfc3339());
    if conn.execute(
        "UPDATE read_later SET read_at = ?2 WHERE id = ?1",
        params![id, at],
    )? == 0
    {
        return Err(AppError::InvalidInput("That link isn't saved".into()));
    }
    Ok(())
}

pub fn set_summary(conn: &Connection, id: &str, summary: &str) -> AppResult<()> {
    conn.execute(
        "UPDATE read_later SET summary = ?2 WHERE id = ?1",
        params![id, summary.chars().take(2000).collect::<String>()],
    )?;
    Ok(())
}

pub fn remove(conn: &Connection, id: &str) -> AppResult<()> {
    conn.execute("DELETE FROM read_later WHERE id = ?1", [id])?;
    Ok(())
}

pub fn clear_read(conn: &Connection) -> AppResult<()> {
    conn.execute("DELETE FROM read_later WHERE read_at IS NOT NULL", [])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn accepts_only_web_links() {
        assert!(check_url("https://example.com/a?b=1").is_ok());
        assert!(check_url(" http://example.com ").is_ok());
        for bad in [
            "file:///C:/x",
            "javascript:alert(1)",
            "example.com",
            "https://user:pw@x.com",
            "",
        ] {
            assert!(check_url(bad).is_err(), "{bad}");
        }
        assert_eq!(
            site_of(&check_url("https://www.theverge.com/x").unwrap()),
            "theverge.com"
        );
    }

    #[test]
    fn reads_title_site_and_text() {
        let html = r#"<html><head><title>Fallback &amp; title</title>
            <meta property="og:title" content="The &quot;Real&quot; Title">
            <meta property="og:site_name" content="The Site">
            <style>.a{}</style><script>var x = "<p>nope</p>";</script></head>
            <body><nav>Menu Home</nav><article><h1>Head</h1><p>One two three.</p>
            <p>Four&nbsp;five</p></article><footer>foot</footer></body></html>"#;
        let page = read_page(html);
        assert_eq!(page.title.as_deref(), Some("The \"Real\" Title"));
        assert_eq!(page.site.as_deref(), Some("The Site"));
        assert_eq!(page.text, "Head One two three. Four five");
        assert_eq!(word_count(&page.text), 6);

        let page = read_page("<title>Only &amp; title</title><body><p>Hi there</p>");
        assert_eq!(page.title.as_deref(), Some("Only & title"));
        assert_eq!(page.site, None);
        assert_eq!(page.text, "Hi there");
        assert_eq!(read_page("no html at all").title, None);
    }

    #[test]
    fn saves_reads_and_clears_links() {
        let mut conn = Connection::open_in_memory().unwrap();
        crate::db::migrate_for_tests(&mut conn);
        let now = Utc.timestamp_opt(1_790_000_000, 0).unwrap();
        let a = check_url("https://a.com/1").unwrap();
        let b = check_url("https://b.com/2").unwrap();
        let first = insert(&conn, &a, "A", "a.com", Some(900), now).unwrap();
        insert(&conn, &b, "B", "b.com", None, now + chrono::Duration::minutes(1)).unwrap();
        assert_eq!(list(&conn).unwrap()[0].title, "B");

        set_read(&conn, &first.id, true, now).unwrap();
        let items = list(&conn).unwrap();
        assert_eq!(items[1].id, first.id);
        assert!(items[1].read_at.is_some());

        // Saving it again brings it back unread, same id.
        let again = insert(
            &conn,
            &a,
            "A2",
            "a.com",
            Some(1000),
            now + chrono::Duration::minutes(2),
        )
        .unwrap();
        assert_eq!(again.id, first.id);
        assert!(again.read_at.is_none());
        assert_eq!(list(&conn).unwrap()[0].title, "A2");

        set_summary(&conn, &first.id, "Short.").unwrap();
        assert_eq!(
            get(&conn, &first.id).unwrap().unwrap().summary.as_deref(),
            Some("Short.")
        );
        set_read(&conn, &first.id, true, now).unwrap();
        clear_read(&conn).unwrap();
        assert_eq!(list(&conn).unwrap().len(), 1);
        assert!(set_read(&conn, "nope", true, now).is_err());
    }
}

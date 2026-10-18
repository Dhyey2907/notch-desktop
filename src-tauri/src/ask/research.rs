//! Background research: "research X" runs while you work and comes back as a card, saved as a
//! note with its sources. With Claude Code as the AI it searches and reads the web itself;
//! with other AIs Overline searches DuckDuckGo, reads the top pages and has the AI summarise.

use url::Url;

pub const CLAUDE_SYSTEM: &str = "You research a topic on the web for the user. Search, read the best \
few sources, then write a short brief: a 2-3 sentence summary, then up to 6 bullet points of key facts. \
End with a 'Sources:' list of the URLs you used, one per line. Plain text, no markdown headers.";

pub const SUMMARY_SYSTEM: &str = "You write a short research brief from the web pages given. A 2-3 \
sentence summary, then up to 6 bullet points of key facts, citing pages like [1]. Only use what the \
pages say. Plain text, no markdown headers.";

/// The longest topic accepted.
pub const MAX_TOPIC: usize = 300;
/// How much of each page the AI reads.
pub const PAGE_CHARS: usize = 4_000;

/// Result links from DuckDuckGo's HTML page (ads and DuckDuckGo's own links left out).
pub fn ddg_results(html: &str, max: usize) -> Vec<(String, Url)> {
    let mut out = Vec::new();
    let mut rest = html;
    while let Some(at) = rest.find("class=\"result__a\"") {
        rest = &rest[at..];
        let Some(href_at) = rest.find("href=\"") else {
            break;
        };
        let after = &rest[href_at + 6..];
        let Some(end) = after.find('"') else { break };
        let href = after[..end].replace("&amp;", "&");
        let title = after[end..]
            .find('>')
            .and_then(|gt| {
                let t = &after[end + gt + 1..];
                t.find("</a>").map(|close| &t[..close])
            })
            .map(strip_tags)
            .unwrap_or_default();
        rest = &after[end..];
        let Some(url) = target(&href) else { continue };
        let host = url.host_str().unwrap_or("");
        if host.ends_with("duckduckgo.com") || out.iter().any(|(_, u): &(String, Url)| u == &url) {
            continue;
        }
        out.push((if title.is_empty() { host.to_string() } else { title }, url));
        if out.len() >= max {
            break;
        }
    }
    out
}

/// DuckDuckGo wraps results as `//duckduckgo.com/l/?uddg=<the real link>`.
fn target(href: &str) -> Option<Url> {
    let full = if href.starts_with("//") {
        format!("https:{href}")
    } else {
        href.to_string()
    };
    let url = Url::parse(&full).ok()?;
    let real = if url.path() == "/l/" {
        let encoded = url.query_pairs().find(|(k, _)| k == "uddg")?.1.into_owned();
        Url::parse(&encoded).ok()?
    } else {
        url
    };
    matches!(real.scheme(), "http" | "https").then_some(real)
}

fn strip_tags(s: &str) -> String {
    let mut out = String::new();
    let mut inside = false;
    for c in s.chars() {
        match c {
            '<' => inside = true,
            '>' => inside = false,
            c if !inside => out.push(c),
            _ => {}
        }
    }
    out.replace("&amp;", "&")
        .replace("&#x27;", "'")
        .replace("&quot;", "\"")
        .trim()
        .to_string()
}

/// The pages as the AI reads them, numbered.
pub fn pages_context(topic: &str, pages: &[(String, Url, String)]) -> String {
    let mut out = format!("Topic: {topic}\n\n");
    for (i, (title, url, text)) in pages.iter().enumerate() {
        let cut: String = text.chars().take(PAGE_CHARS).collect();
        out.push_str(&format!("[{}] {title} ({url})\n{cut}\n\n", i + 1));
    }
    out
}

/// The note saved for a finished brief (notes hold up to 10,000 characters).
pub fn note_body(topic: &str, brief: &str, sources: &[Url]) -> String {
    let mut body = format!("Research: {topic}\n\n{}", brief.trim());
    if !sources.is_empty() && !brief.contains("Sources:") {
        body.push_str("\n\nSources:\n");
        for (i, u) in sources.iter().enumerate() {
            body.push_str(&format!("[{}] {u}\n", i + 1));
        }
    }
    body.chars().take(10_000).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const DDG: &str = r#"<div class="result results_links"><a rel="nofollow" class="result__a" href="//duckduckgo.com/l/?uddg=https%3A%2F%2Fexample.com%2Fsolar%3Fa%3D1&amp;rut=abc">Solar <b>panels</b> explained</a></div>
<div class="result result--ad"><a class="result__a" href="https://duckduckgo.com/y.js?ad=1">Ad</a></div>
<div><a class="result__a" href="https://second.org/page">Second &amp; more</a></div>
<div><a class="result__a" href="javascript:alert(1)">Bad</a></div>"#;

    #[test]
    fn reads_duckduckgo_results() {
        let r = ddg_results(DDG, 5);
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].0, "Solar panels explained");
        assert_eq!(r[0].1.as_str(), "https://example.com/solar?a=1");
        assert_eq!(r[1].0, "Second & more");
        assert_eq!(ddg_results(DDG, 1).len(), 1);
    }

    #[test]
    fn builds_the_context_and_the_note() {
        let u = Url::parse("https://a.com/").unwrap();
        let ctx = pages_context("solar", &[("A".into(), u.clone(), "x".repeat(10_000))]);
        assert!(ctx.starts_with("Topic: solar"));
        assert!(ctx.len() < PAGE_CHARS + 100);
        let note = note_body("solar", "Brief.", &[u]);
        assert!(note.starts_with("Research: solar\n\nBrief."));
        assert!(note.contains("[1] https://a.com/"));
        assert!(!note_body(
            "t",
            "x\nSources:\nhttps://b.com",
            &[Url::parse("https://a.com").unwrap()]
        )
        .contains("[1]"));
    }
}

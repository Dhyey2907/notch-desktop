//! Strip tracking parameters from copied links (`utm_*`, `fbclid`, YouTube's `si`, …), keeping
//! everything the page needs.

/// Parameters that only identify who shared or clicked. Matched case-insensitively.
const TRACKERS: &[&str] = &[
    "fbclid",
    "gclid",
    "gclsrc",
    "dclid",
    "gbraid",
    "wbraid",
    "msclkid",
    "twclid",
    "ttclid",
    "li_fat_id",
    "igshid",
    "igsh",
    "mc_cid",
    "mc_eid",
    "_hsenc",
    "_hsmi",
    "mkt_tok",
    "yclid",
    "ref_src",
    "ref_url",
    "s_cid",
    "spm",
    "vero_id",
    "oly_anon_id",
    "oly_enc_id",
    "rb_clickid",
    "_ga",
    "_gl",
];

/// Parameters that are tracking only on some sites (`si` is a real parameter elsewhere).
const SITE_TRACKERS: &[(&str, &[&str])] = &[
    ("youtube.com", &["si", "feature", "pp"]),
    ("youtu.be", &["si", "feature"]),
    ("music.youtube.com", &["si"]),
    ("open.spotify.com", &["si", "context"]),
    ("instagram.com", &["igsh", "igshid", "utm_source"]),
    ("twitter.com", &["s", "t", "ref_src"]),
    ("x.com", &["s", "t"]),
    (
        "amazon.",
        &[
            "ref", "ref_", "pd_rd_r", "pd_rd_w", "pd_rd_wg", "pf_rd_p", "pf_rd_r", "psc", "th",
        ],
    ),
    ("linkedin.com", &["trk", "trackingId", "lipi", "rcm"]),
    ("reddit.com", &["share_id", "utm_name", "rdt"]),
    ("tiktok.com", &["_r", "_t", "is_from_webapp", "sender_device"]),
];

fn tracked(host: &str, key: &str) -> bool {
    let key_lower = key.to_lowercase();
    if key_lower.starts_with("utm_") || TRACKERS.contains(&key_lower.as_str()) {
        return true;
    }
    SITE_TRACKERS.iter().any(|(site, keys)| {
        let on_site = if site.ends_with('.') {
            host.contains(site)
        } else {
            host == *site || host.ends_with(&format!(".{site}"))
        };
        on_site && keys.iter().any(|k| k.eq_ignore_ascii_case(key))
    })
}

/// The cleaned link and how many parameters went, when the text is a single link that had
/// tracking in it.
pub fn clean(text: &str) -> Option<(String, usize)> {
    let trimmed = text.trim();
    if trimmed.len() > 4000 || trimmed.contains(char::is_whitespace) {
        return None;
    }
    let mut url = url::Url::parse(trimmed).ok()?;
    if !matches!(url.scheme(), "http" | "https") {
        return None;
    }
    let host = url.host_str()?.to_lowercase();
    let query = url.query()?.to_owned();
    // Rebuild the query by hand so untouched parameters keep their exact spelling.
    let mut removed = 0;
    let kept: Vec<&str> = query
        .split('&')
        .filter(|pair| {
            let key = pair.split('=').next().unwrap_or("");
            let key = url::form_urlencoded::parse(key.as_bytes())
                .next()
                .map(|(k, _)| k.into_owned())
                .unwrap_or_default();
            let drop = !key.is_empty() && tracked(&host, &key);
            removed += usize::from(drop);
            !drop && !pair.is_empty()
        })
        .collect();
    if removed == 0 {
        return None;
    }
    let query = kept.join("&");
    url.set_query((!query.is_empty()).then_some(query.as_str()));
    Some((url.to_string(), removed))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_common_trackers() {
        assert_eq!(
            clean("https://example.com/post?id=7&utm_source=x&utm_medium=social&fbclid=abc"),
            Some(("https://example.com/post?id=7".into(), 3))
        );
        assert_eq!(
            clean("https://youtu.be/dQw4w9WgXcQ?si=Ab12Cd"),
            Some(("https://youtu.be/dQw4w9WgXcQ".into(), 1))
        );
        assert_eq!(
            clean("https://www.youtube.com/watch?v=dQw4w9WgXcQ&si=xyz&t=42"),
            Some(("https://www.youtube.com/watch?v=dQw4w9WgXcQ&t=42".into(), 1))
        );
        assert_eq!(
            clean("https://open.spotify.com/track/123?si=abc"),
            Some(("https://open.spotify.com/track/123".into(), 1))
        );
    }

    #[test]
    fn leaves_real_parameters_and_other_text_alone() {
        assert_eq!(clean("https://example.com/search?q=rust&page=2"), None);
        // `si` means something on other sites.
        assert_eq!(clean("https://example.com/?si=1"), None);
        assert_eq!(clean("not a link"), None);
        assert_eq!(clean("two https://a.com?utm_source=x links"), None);
        assert_eq!(clean("ftp://host/file?utm_source=x"), None);
        // Fragments and exact spelling survive.
        assert_eq!(
            clean("https://docs.rs/x?a=%2F&utm_campaign=y#section"),
            Some(("https://docs.rs/x?a=%2F#section".into(), 1))
        );
    }
}

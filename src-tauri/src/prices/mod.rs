//! Price watch: paste a product link and a target price; Overline checks the page every few
//! hours and a card says when it drops to (or below) your target. Prices are read from the
//! page's structured data (JSON-LD), its price meta tags, or Amazon's and Flipkart's price
//! markup. Best effort: a shop that blocks automated visits shows as "couldn't read".

pub mod service;

use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use ts_rs::TS;

use crate::error::{AppError, AppResult};

/// Pages are checked this often.
pub const CHECK_EVERY_SECS: i64 = 6 * 3600;
/// Most items watched at once.
pub const MAX_ITEMS: usize = 30;
const MAX_HISTORY: usize = 60;

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PriceItem {
    pub id: String,
    pub url: String,
    pub title: String,
    pub target: f64,
    /// "₹", "$", "INR"… whatever the page said.
    pub currency: String,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price: Option<f64>,
    /// When it was last checked (Unix ms); 0 = never.
    #[ts(type = "number")]
    pub checked_at: i64,
    /// (Unix ms, price) points, oldest first.
    #[ts(type = "Array<[number, number]>")]
    pub history: Vec<(i64, f64)>,
    /// Why the last check failed.
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// What a product page says.
#[derive(Debug, Clone, PartialEq)]
pub struct Found {
    pub price: f64,
    pub currency: String,
    pub title: Option<String>,
}

fn number(raw: &str) -> Option<f64> {
    // "1,299.00", "₹ 1,299", "1.299,00 €" (European: comma decimals).
    let digits: String = raw
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == '.' || *c == ',')
        .collect();
    let digits = digits.trim_matches(|c| c == '.' || c == ',');
    if digits.is_empty() {
        return None;
    }
    let normal = match (digits.rfind('.'), digits.rfind(',')) {
        (Some(d), Some(c)) if c > d => digits.replace('.', "").replace(',', "."),
        (None, Some(c))
            if digits.len() - c == 3 && digits.matches(',').count() == 1 && !digits.starts_with('0') =>
        {
            // "12,50" is a decimal in some shops, "1,299" a thousands separator in most:
            // treat a single comma with two digits after it as a decimal.
            digits.replace(',', ".")
        }
        _ => digits.replace(',', ""),
    };
    normal
        .parse::<f64>()
        .ok()
        .filter(|p| p.is_finite() && *p > 0.0 && *p < 1e9)
}

fn attr<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let key = format!("{name}=\"");
    let at = tag.find(&key)? + key.len();
    let end = tag[at..].find('"')?;
    Some(&tag[at..at + end])
}

/// `<meta property="og:price:amount" content="…">` and friends.
fn meta(html: &str, property: &str) -> Option<String> {
    let mut rest = html;
    while let Some(at) = rest.find("<meta") {
        let end = rest[at..].find('>').map_or(rest.len(), |e| at + e);
        let tag = &rest[at..end];
        if attr(tag, "property")
            .or_else(|| attr(tag, "name"))
            .or_else(|| attr(tag, "itemprop"))
            == Some(property)
        {
            return attr(tag, "content").map(str::to_string);
        }
        rest = &rest[end..];
    }
    None
}

fn json_ld_offer(value: &serde_json::Value) -> Option<(f64, String, Option<String>)> {
    match value {
        serde_json::Value::Array(items) => items.iter().find_map(json_ld_offer),
        serde_json::Value::Object(map) => {
            if let Some(graph) = map.get("@graph") {
                if let Some(found) = json_ld_offer(graph) {
                    return Some(found);
                }
            }
            let name = map.get("name").and_then(|n| n.as_str()).map(str::to_string);
            let offers = map.get("offers")?;
            let offer = match offers {
                serde_json::Value::Array(a) => a.first()?,
                o => o,
            };
            let price = offer
                .get("price")
                .or_else(|| offer.get("lowPrice"))
                .and_then(|p| match p {
                    serde_json::Value::Number(n) => n.as_f64(),
                    serde_json::Value::String(s) => number(s),
                    _ => None,
                })?;
            let currency = offer
                .get("priceCurrency")
                .and_then(|c| c.as_str())
                .unwrap_or("")
                .to_string();
            Some((price, currency, name))
        }
        _ => None,
    }
}

fn page_title(html: &str) -> Option<String> {
    meta(html, "og:title").or_else(|| {
        let lower = html.to_ascii_lowercase();
        let s = lower.find("<title")?;
        let s = s + lower[s..].find('>')? + 1;
        let e = s + lower[s..].find("</title")?;
        Some(html[s..e].trim().to_string())
    })
}

/// The price on a product page.
pub fn parse(html: &str) -> Option<Found> {
    let title = page_title(html).map(|t| t.replace("&amp;", "&").chars().take(140).collect::<String>());
    // 1. Structured data.
    let mut rest = html;
    while let Some(at) = rest.find("application/ld+json") {
        let after = &rest[at..];
        let start = after.find('>')? + 1;
        let end = after[start..].find("</script>")? + start;
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(after[start..end].trim()) {
            if let Some((price, currency, name)) = json_ld_offer(&v) {
                return Some(Found {
                    price,
                    currency,
                    title: name.or(title),
                });
            }
        }
        rest = &after[end..];
    }
    // 2. Price meta tags.
    for (amount, currency) in [
        ("og:price:amount", "og:price:currency"),
        ("product:price:amount", "product:price:currency"),
        ("price", "priceCurrency"),
    ] {
        if let Some(price) = meta(html, amount).as_deref().and_then(number) {
            return Some(Found {
                price,
                currency: meta(html, currency).unwrap_or_default(),
                title,
            });
        }
    }
    // 3. Amazon and Flipkart's own markup.
    let marked = |marker: &str| -> Option<f64> {
        let at = html.find(marker)?;
        let start = at + html[at..].find('>')? + 1;
        let end = start + html[start..].find('<')?;
        number(&html[start..end])
    };
    if let Some(price) = marked("class=\"a-price-whole\"").or_else(|| marked("id=\"priceblock_ourprice\"")) {
        return Some(Found {
            price,
            currency: "₹".into(),
            title,
        });
    }
    if let Some(price) = marked("class=\"Nx9bqj").or_else(|| marked("class=\"_30jeq3")) {
        return Some(Found {
            price,
            currency: "₹".into(),
            title,
        });
    }
    None
}

/// Did this check cross down to the target? (A card is shown once per drop.)
pub fn crossed(previous: Option<f64>, now: f64, target: f64) -> bool {
    now <= target && previous.is_none_or(|p| p > target)
}

pub fn check_url(raw: &str) -> AppResult<url::Url> {
    let u =
        url::Url::parse(raw.trim()).map_err(|_| AppError::InvalidInput("That isn't a web link".into()))?;
    if !matches!(u.scheme(), "http" | "https") || u.host_str().is_none() || raw.len() > 2000 {
        return Err(AppError::InvalidInput("That isn't a web link".into()));
    }
    Ok(u)
}

// ── Storage ────────────────────────────────────────────────────────────────

fn row(r: &rusqlite::Row<'_>) -> rusqlite::Result<PriceItem> {
    let history: String = r.get(7)?;
    Ok(PriceItem {
        id: r.get(0)?,
        url: r.get(1)?,
        title: r.get(2)?,
        target: r.get(3)?,
        currency: r.get(4)?,
        price: r.get(5)?,
        checked_at: r.get(6)?,
        history: serde_json::from_str(&history).unwrap_or_default(),
        error: r.get(8)?,
    })
}

const COLUMNS: &str = "id, url, title, target, currency, price, checked_at, history, error";

pub fn list(conn: &Connection) -> AppResult<Vec<PriceItem>> {
    let mut stmt = conn.prepare(&format!("SELECT {COLUMNS} FROM prices ORDER BY rowid DESC"))?;
    let rows = stmt.query_map([], row)?;
    Ok(rows.filter_map(Result::ok).collect())
}

pub fn get(conn: &Connection, id: &str) -> AppResult<Option<PriceItem>> {
    Ok(conn
        .query_row(
            &format!("SELECT {COLUMNS} FROM prices WHERE id = ?1"),
            params![id],
            row,
        )
        .optional()?)
}

pub fn insert(conn: &Connection, item: &PriceItem) -> AppResult<()> {
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM prices", [], |r| r.get(0))?;
    if count as usize >= MAX_ITEMS {
        return Err(AppError::InvalidInput(format!(
            "You can watch up to {MAX_ITEMS} prices"
        )));
    }
    conn.execute(
        &format!("INSERT INTO prices ({COLUMNS}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"),
        params![
            item.id,
            item.url,
            item.title,
            item.target,
            item.currency,
            item.price,
            item.checked_at,
            serde_json::to_string(&item.history).unwrap_or_else(|_| "[]".into()),
            item.error
        ],
    )?;
    Ok(())
}

/// Record a check: the price (or why it failed), keeping a short history.
pub fn record(
    conn: &Connection,
    id: &str,
    at: i64,
    result: Result<&Found, String>,
) -> AppResult<Option<PriceItem>> {
    let Some(mut item) = get(conn, id)? else {
        return Ok(None);
    };
    item.checked_at = at;
    match result {
        Ok(found) => {
            item.price = Some(found.price);
            if !found.currency.is_empty() {
                item.currency = found.currency.clone();
            }
            if item.title.is_empty() {
                if let Some(t) = &found.title {
                    item.title = t.clone();
                }
            }
            if item
                .history
                .last()
                .is_none_or(|(_, p)| (*p - found.price).abs() > f64::EPSILON)
            {
                item.history.push((at, found.price));
            }
            let excess = item.history.len().saturating_sub(MAX_HISTORY);
            item.history.drain(..excess);
            item.error = None;
        }
        Err(why) => item.error = Some(why),
    }
    conn.execute(
        "UPDATE prices SET title = ?2, currency = ?3, price = ?4, checked_at = ?5, history = ?6, error = ?7 WHERE id = ?1",
        params![
            item.id,
            item.title,
            item.currency,
            item.price,
            item.checked_at,
            serde_json::to_string(&item.history).unwrap_or_else(|_| "[]".into()),
            item.error
        ],
    )?;
    Ok(Some(item))
}

pub fn set_target(conn: &Connection, id: &str, target: f64) -> AppResult<()> {
    conn.execute("UPDATE prices SET target = ?2 WHERE id = ?1", params![id, target])?;
    Ok(())
}

pub fn remove(conn: &Connection, id: &str) -> AppResult<()> {
    conn.execute("DELETE FROM prices WHERE id = ?1", params![id])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_json_ld() {
        let html = r#"<html><head><title>Shop</title><script type="application/ld+json">
            {"@context":"https://schema.org","@type":"Product","name":"Desk Lamp",
             "offers":{"@type":"Offer","price":"1299.00","priceCurrency":"INR"}}</script></head></html>"#;
        assert_eq!(
            parse(html),
            Some(Found {
                price: 1299.0,
                currency: "INR".into(),
                title: Some("Desk Lamp".into())
            })
        );
    }

    #[test]
    fn reads_graphs_meta_tags_and_shop_markup() {
        let graph = r#"<script type="application/ld+json">{"@graph":[{"@type":"WebPage"},{"@type":"Product","name":"Mouse","offers":[{"price":499,"priceCurrency":"USD"}]}]}</script>"#;
        assert_eq!(parse(graph).unwrap().price, 499.0);
        let meta = r#"<meta property="og:title" content="Chair"><meta property="product:price:amount" content="2,499.50"><meta property="product:price:currency" content="INR">"#;
        let m = parse(meta).unwrap();
        assert_eq!(
            (m.price, m.currency.as_str(), m.title.as_deref()),
            (2499.5, "INR", Some("Chair"))
        );
        let amazon = r#"<span class="a-price-whole">1,49,990<span class="a-price-decimal">.</span></span>"#;
        assert_eq!(parse(amazon).unwrap().price, 149_990.0);
        let flipkart = r#"<div class="Nx9bqj CxhGGd">₹12,999</div>"#;
        assert_eq!(parse(flipkart).unwrap().price, 12_999.0);
        assert!(parse("<html>no price</html>").is_none());
    }

    #[test]
    fn numbers_in_shop_formats() {
        assert_eq!(number("₹ 1,299"), Some(1299.0));
        assert_eq!(number("1.299,00 €"), Some(1299.0));
        assert_eq!(number("12,50"), Some(12.5));
        assert_eq!(number("$19.99"), Some(19.99));
        assert_eq!(number("free"), None);
    }

    #[test]
    fn warns_once_per_drop() {
        assert!(crossed(Some(1500.0), 1200.0, 1300.0));
        assert!(!crossed(Some(1250.0), 1200.0, 1300.0));
        assert!(crossed(None, 1200.0, 1300.0));
        assert!(!crossed(Some(1500.0), 1400.0, 1300.0));
    }

    #[test]
    fn stores_items_and_history() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(include_str!("../db/migrations/018_prices.sql"))
            .unwrap();
        let item = PriceItem {
            id: "a".into(),
            url: "https://shop.example/p".into(),
            title: String::new(),
            target: 1000.0,
            currency: String::new(),
            price: None,
            checked_at: 0,
            history: vec![],
            error: None,
        };
        insert(&conn, &item).unwrap();
        let found = Found {
            price: 1200.0,
            currency: "₹".into(),
            title: Some("Lamp".into()),
        };
        record(&conn, "a", 1, Ok(&found)).unwrap();
        record(&conn, "a", 2, Ok(&found)).unwrap();
        let got = record(&conn, "a", 3, Err("blocked".into())).unwrap().unwrap();
        assert_eq!(got.title, "Lamp");
        assert_eq!(got.history.len(), 1, "same price isn't repeated");
        assert_eq!(got.error.as_deref(), Some("blocked"));
        assert_eq!(got.price, Some(1200.0));
        remove(&conn, "a").unwrap();
        assert!(list(&conn).unwrap().is_empty());
    }
}

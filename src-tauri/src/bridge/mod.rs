//! Phone file bridge: a tiny web server on this PC's Wi-Fi address, reachable through a QR code
//! with a random one-time token. The phone's browser downloads the offered files and can upload
//! files back (they land in Downloads\Notch). One session at a time; it expires after
//! 10 minutes. File names and contents are never logged.

pub mod service;

use std::{
    net::{IpAddr, Ipv4Addr},
    path::{Path, PathBuf},
};

use serde::Serialize;
use ts_rs::TS;

/// Sessions end on their own after this long.
pub const SESSION_MINUTES: i64 = 10;
/// Largest upload accepted.
pub const MAX_UPLOAD: u64 = 2 * 1024 * 1024 * 1024;
const MAX_NAME: usize = 120;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BridgeState {
    /// The address the phone opens (also in the QR code).
    pub url: String,
    /// The QR code as an SVG document.
    pub qr_svg: String,
    /// Names of the files offered to the phone.
    pub files: Vec<String>,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    /// Files received from the phone so far.
    pub received: u32,
}

/// A private LAN address (what a phone on the same Wi-Fi can reach).
pub fn is_lan(ip: Ipv4Addr) -> bool {
    let o = ip.octets();
    ip.is_private() && !(o[0] == 100 && (64..128).contains(&o[1]))
}

/// The address Windows would use to reach the internet — on Wi-Fi or Ethernet, that's the
/// LAN address. No packet is sent: connecting a UDP socket only picks the route.
pub fn lan_ip() -> Option<Ipv4Addr> {
    let socket = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    match socket.local_addr().ok()?.ip() {
        IpAddr::V4(ip) if is_lan(ip) => Some(ip),
        _ => None,
    }
}

/// Compare without leaking how much matched through timing.
pub fn token_matches(given: &str, expected: &str) -> bool {
    given.len() == expected.len()
        && given
            .bytes()
            .zip(expected.bytes())
            .fold(0u8, |acc, (a, b)| acc | (a ^ b))
            == 0
}

#[derive(Debug, PartialEq, Eq)]
pub enum Route {
    Page,
    Download(usize),
    Upload(String),
    Forbidden,
    NotFound,
}

/// `/<token>/`, `/<token>/file/<n>`, `/<token>/upload?name=…` (PUT).
pub fn route(method: &str, url: &str, token: &str) -> Route {
    let (path, query) = url.split_once('?').unwrap_or((url, ""));
    let mut parts = path.trim_start_matches('/').splitn(2, '/');
    let given = parts.next().unwrap_or("");
    if !token_matches(given, token) {
        return Route::Forbidden;
    }
    let rest = parts.next().unwrap_or("");
    match (method, rest) {
        ("GET", "") => Route::Page,
        ("GET", r) if r.starts_with("file/") => {
            r[5..].parse().map(Route::Download).unwrap_or(Route::NotFound)
        }
        ("PUT" | "POST", "upload") => {
            let name = query
                .split('&')
                .find_map(|kv| kv.strip_prefix("name="))
                .map(percent_decode)
                .unwrap_or_default();
            Route::Upload(name)
        }
        _ => Route::NotFound,
    }
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = bytes
            .get(i + 1..i + 3)
            .and_then(|h| std::str::from_utf8(h).ok())
            .and_then(|h| u8::from_str_radix(h, 16).ok());
        match (bytes[i], hex) {
            (b'%', Some(b)) => {
                out.push(b);
                i += 3;
            }
            (b'+', _) => {
                out.push(b' ');
                i += 1;
            }
            (b, _) => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// A safe file name: no folders, no characters Windows forbids, not a reserved device name.
pub fn clean_name(name: &str) -> String {
    let base = name.rsplit(['/', '\\']).next().unwrap_or("");
    let cleaned: String = base
        .chars()
        .map(|c| {
            if c.is_control() || "<>:\"|?*".contains(c) {
                '_'
            } else {
                c
            }
        })
        .collect();
    let mut cleaned = cleaned.trim().trim_matches('.').trim().to_owned();
    if cleaned.chars().count() > MAX_NAME {
        // Keep the extension when shortening.
        let ext = Path::new(&cleaned)
            .extension()
            .map(|e| e.to_string_lossy().into_owned());
        let keep = MAX_NAME - ext.as_ref().map_or(0, |e| e.len() + 1);
        let stem: String = cleaned.chars().take(keep).collect();
        cleaned = match ext {
            Some(e) if e.len() < 12 => format!("{stem}.{e}"),
            _ => cleaned.chars().take(MAX_NAME).collect(),
        };
    }
    let stem = cleaned.split('.').next().unwrap_or("").to_ascii_uppercase();
    let reserved = matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || ((stem.starts_with("COM") || stem.starts_with("LPT")) && stem.len() == 4);
    if cleaned.is_empty() {
        "file-upload".to_owned()
    } else if reserved {
        format!("file-{cleaned}")
    } else {
        cleaned
    }
}

/// `dir/name`, or `name (2).ext`… if that's taken.
pub fn unique_path(dir: &Path, name: &str) -> PathBuf {
    let first = dir.join(name);
    if !first.exists() {
        return first;
    }
    let path = Path::new(name);
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let ext = path
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();
    (2..10_000)
        .map(|n| dir.join(format!("{stem} ({n}){ext}")))
        .find(|p| !p.exists())
        .unwrap_or(first)
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// The page the phone opens: the offered files, and an upload button.
pub fn page(files: &[String], pc_name: &str) -> String {
    let list = if files.is_empty() {
        "<p class=muted>Nothing to download — send files from your phone below.</p>".to_owned()
    } else {
        files
            .iter()
            .enumerate()
            .map(|(i, f)| format!("<a class=file href=\"file/{i}\" download>⬇ {}</a>", escape(f)))
            .collect::<Vec<_>>()
            .join("")
    };
    format!(
        r#"<!doctype html><html lang=en><head><meta charset=utf-8><meta name=viewport content="width=device-width,initial-scale=1">
<title>Overline · {pc}</title><style>
body{{margin:0;font:16px system-ui,sans-serif;background:#0b0b0c;color:#f2f2f2}}main{{max-width:520px;margin:0 auto;padding:20px}}
h1{{font-size:20px;margin:8px 0 2px}}.muted{{color:#9a9aa0;font-size:14px}}.file{{display:block;padding:14px;margin:8px 0;border-radius:12px;background:#1b1b1f;color:#f2f2f2;text-decoration:none;word-break:break-all}}
label.up{{display:block;margin-top:22px;padding:16px;border-radius:12px;background:#2b6fe0;color:#fff;text-align:center;font-weight:600}}input{{display:none}}#s{{margin-top:12px;font-size:14px;color:#9a9aa0}}
</style></head><body><main><h1>Overline · {pc}</h1><p class=muted>Files from your PC</p>{list}
<label class=up>Send files to the PC<input id=f type=file multiple></label><p id=s></p></main>
<script>
const s=document.getElementById('s');
document.getElementById('f').addEventListener('change',async e=>{{
 for(const file of e.target.files){{
  await new Promise(done=>{{const x=new XMLHttpRequest();x.open('PUT','upload?name='+encodeURIComponent(file.name));
   x.upload.onprogress=p=>{{if(p.lengthComputable)s.textContent='Sending '+file.name+' — '+Math.round(p.loaded/p.total*100)+'%'}};
   x.onload=()=>{{s.textContent=x.status===200?'Sent '+file.name+' ✓':'Could not send '+file.name}};x.onerror=()=>{{s.textContent='Could not send '+file.name}};x.onloadend=done;x.send(file)}});
 }}
}});
</script></body></html>"#,
        pc = escape(pc_name),
    )
}

pub fn qr_svg(url: &str) -> Option<String> {
    let code = qrcode::QrCode::new(url.as_bytes()).ok()?;
    Some(
        code.render::<qrcode::render::svg::Color>()
            .min_dimensions(200, 200)
            .dark_color(qrcode::render::svg::Color("#000000"))
            .light_color(qrcode::render::svg::Color("#ffffff"))
            .build(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes_only_with_the_token() {
        let t = "abcd1234";
        assert_eq!(route("GET", "/abcd1234/", t), Route::Page);
        assert_eq!(route("GET", "/abcd1234", t), Route::Page);
        assert_eq!(route("GET", "/abcd1234/file/2", t), Route::Download(2));
        assert_eq!(
            route("PUT", "/abcd1234/upload?name=My%20photo.jpg", t),
            Route::Upload("My photo.jpg".into())
        );
        assert_eq!(route("GET", "/abcd1235/", t), Route::Forbidden);
        assert_eq!(route("GET", "/", t), Route::Forbidden);
        assert_eq!(route("GET", "/abcd1234/file/x", t), Route::NotFound);
        assert_eq!(route("DELETE", "/abcd1234/", t), Route::NotFound);
    }

    #[test]
    fn cleans_uploaded_names() {
        assert_eq!(clean_name("photo.jpg"), "photo.jpg");
        assert_eq!(clean_name("../../Windows/evil.exe"), "evil.exe");
        assert_eq!(clean_name("C:\\a\\b.txt"), "b.txt");
        assert_eq!(clean_name("a<b>:c?.txt"), "a_b__c_.txt");
        assert_eq!(clean_name("CON.txt"), "file-CON.txt");
        assert_eq!(clean_name("..."), "file-upload");
        assert_eq!(clean_name(""), "file-upload");
        let long = format!("{}.jpg", "x".repeat(300));
        let cleaned = clean_name(&long);
        assert!(cleaned.ends_with(".jpg") && cleaned.chars().count() <= MAX_NAME);
    }

    #[test]
    fn numbers_duplicates() {
        let dir = std::env::temp_dir().join(format!("notch-bridge-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.txt"), "1").unwrap();
        std::fs::write(dir.join("a (2).txt"), "2").unwrap();
        assert_eq!(unique_path(&dir, "a.txt"), dir.join("a (3).txt"));
        assert_eq!(unique_path(&dir, "b.txt"), dir.join("b.txt"));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn picks_lan_addresses_only() {
        assert!(is_lan("192.168.1.20".parse().unwrap()));
        assert!(is_lan("10.0.0.5".parse().unwrap()));
        assert!(!is_lan("100.111.165.0".parse().unwrap()), "Tailscale");
        assert!(!is_lan("8.8.8.8".parse().unwrap()));
    }

    #[test]
    fn escapes_names_in_the_page() {
        let html = page(&["<img src=x onerror=alert(1)>.png".into()], "PC");
        assert!(html.contains("&lt;img src=x onerror=alert(1)&gt;.png"));
        assert!(!html.contains("<img src=x"));
        assert!(qr_svg("http://192.168.1.2:5000/abc/")
            .unwrap()
            .starts_with("<?xml"));
    }
}

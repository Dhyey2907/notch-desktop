//! Provider-neutral pieces of OAuth 2.0 for installed apps (RFC 8252): PKCE, and a one-shot
//! loopback HTTP listener that receives the browser redirect. Used by Google Calendar and
//! Spotify; each provider keeps its own endpoints and token handling.

use std::{
    io::{BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
    time::{Duration, Instant},
};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use sha2::{Digest, Sha256};
use url::Url;

use crate::error::{AppError, AppResult};

/// How long we wait for the user to finish signing in.
pub const SIGN_IN_TIMEOUT: Duration = Duration::from_secs(5 * 60);

pub fn random_urlsafe(bytes: usize) -> String {
    let mut buf = vec![0u8; bytes];
    rand::fill(&mut buf[..]);
    URL_SAFE_NO_PAD.encode(buf)
}

/// PKCE verifier (43 chars) and its S256 challenge.
pub fn pkce_pair() -> (String, String) {
    let verifier = random_urlsafe(32);
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    (verifier, challenge)
}

/// Result of the browser redirect back to the loopback listener.
#[derive(Debug, PartialEq, Eq)]
pub enum Callback {
    Code(String),
    /// The user declined, or the provider reported an error.
    Denied(String),
}

/// Parse the redirect's request line, e.g. `GET /callback?code=…&state=… HTTP/1.1`, and check
/// `state` against what we sent (CSRF protection).
pub fn parse_callback(request_line: &str, expected_state: &str) -> AppResult<Callback> {
    let target = request_line
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| AppError::InvalidInput("malformed redirect".into()))?;
    let url = Url::parse(&format!("http://127.0.0.1{target}"))
        .map_err(|e| AppError::InvalidInput(e.to_string()))?;
    let param = |name: &str| {
        url.query_pairs()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.into_owned())
    };

    if param("state").as_deref() != Some(expected_state) {
        return Err(AppError::InvalidInput("sign-in state mismatch".into()));
    }
    if let Some(error) = param("error") {
        return Ok(Callback::Denied(error));
    }
    param("code")
        .filter(|c| !c.is_empty())
        .map(Callback::Code)
        .ok_or_else(|| AppError::InvalidInput("redirect had no code".into()))
}

/// A one-shot loopback server for the OAuth redirect.
pub struct Loopback {
    listener: TcpListener,
    /// Path the provider redirects to, e.g. `/` or `/callback`.
    path: String,
}

impl Loopback {
    /// `port` 0 picks a free port (Google); providers that require an exact registered
    /// redirect URI (Spotify) need a fixed one.
    pub fn bind(port: u16, path: &str) -> AppResult<Self> {
        let listener = TcpListener::bind(("127.0.0.1", port)).map_err(|e| {
            if e.kind() == std::io::ErrorKind::AddrInUse {
                AppError::InvalidInput(format!(
                    "port {port} is busy; close whatever is using it and try again"
                ))
            } else {
                e.into()
            }
        })?;
        Ok(Self {
            listener,
            path: path.to_owned(),
        })
    }

    pub fn redirect_uri(&self) -> AppResult<String> {
        Ok(format!(
            "http://127.0.0.1:{}{}",
            self.listener.local_addr()?.port(),
            self.path
        ))
    }

    /// Block until the browser delivers the redirect (ignoring unrelated requests such as
    /// `/favicon.ico`), then answer with a small page telling the user to return to the app.
    pub fn wait(self, expected_state: &str, timeout: Duration, service: &str) -> AppResult<Callback> {
        let deadline = Instant::now() + timeout;
        self.listener.set_nonblocking(true)?;
        loop {
            match self.listener.accept() {
                Ok((stream, _)) => {
                    if let Some(result) = handle_connection(stream, &self.path, expected_state, service)? {
                        return result;
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    if Instant::now() >= deadline {
                        return Err(AppError::InvalidInput("sign-in timed out".into()));
                    }
                    std::thread::sleep(Duration::from_millis(100));
                }
                Err(e) => return Err(e.into()),
            }
        }
    }
}

fn handle_connection(
    mut stream: TcpStream,
    path: &str,
    expected_state: &str,
    service: &str,
) -> AppResult<Option<AppResult<Callback>>> {
    stream.set_nonblocking(false)?;
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    let mut line = String::new();
    BufReader::new(&stream).read_line(&mut line)?;
    if !line.starts_with(&format!("GET {path}?")) {
        let _ = stream.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
        return Ok(None);
    }
    let result = parse_callback(&line, expected_state);
    let message = match &result {
        Ok(Callback::Code(_)) => {
            format!("{service} connected. You can close this tab and return to Overline.")
        }
        Ok(Callback::Denied(_)) => "Sign-in was cancelled. You can close this tab.".to_owned(),
        Err(_) => "Something went wrong with sign-in. Please try again from Overline.".to_owned(),
    };
    let body = format!(
        "<!doctype html><meta charset=utf-8><title>Overline</title>\
         <body style=\"font-family:system-ui;background:#090909;color:#f5f5f5;display:grid;place-items:center;height:100vh;margin:0\">\
         <p>{message}</p>"
    );
    let _ = write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    Ok(Some(result))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkce_challenge_is_the_s256_of_the_verifier() {
        let (verifier, challenge) = pkce_pair();
        assert_eq!(verifier.len(), 43);
        assert!(verifier
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'));
        assert_eq!(
            challenge,
            URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
        );
        assert_ne!(pkce_pair().0, verifier);
    }

    #[test]
    fn parses_the_redirect_and_checks_state() {
        assert_eq!(
            parse_callback("GET /callback?state=s1&code=4%2Fabc HTTP/1.1", "s1").unwrap(),
            Callback::Code("4/abc".into())
        );
        assert_eq!(
            parse_callback("GET /?error=access_denied&state=s1 HTTP/1.1", "s1").unwrap(),
            Callback::Denied("access_denied".into())
        );
        assert!(parse_callback("GET /?state=evil&code=x HTTP/1.1", "s1").is_err());
        assert!(parse_callback("GET /?state=s1 HTTP/1.1", "s1").is_err());
        assert!(parse_callback("garbage", "s1").is_err());
    }

    fn run_loopback(path: &str, requests: &'static [&'static [u8]]) -> (AppResult<Callback>, String) {
        let loopback = Loopback::bind(0, path).unwrap();
        let uri = loopback.redirect_uri().unwrap();
        assert!(uri.ends_with(path));
        let addr = uri
            .trim_start_matches("http://")
            .split('/')
            .next()
            .unwrap()
            .to_owned();
        let client = std::thread::spawn(move || {
            let mut last = String::new();
            for request in requests {
                let mut s = TcpStream::connect(&addr).unwrap();
                s.write_all(request).unwrap();
                last.clear();
                let _ = std::io::Read::read_to_string(&mut s, &mut last);
            }
            last
        });
        let result = loopback.wait("xyz", Duration::from_secs(5), "Test");
        (result, client.join().unwrap())
    }

    #[test]
    fn loopback_receives_the_code_on_its_path_and_ignores_others() {
        let (result, response) = run_loopback(
            "/callback",
            &[
                b"GET /favicon.ico HTTP/1.1\r\n\r\n",
                b"GET /?code=wrong-path&state=xyz HTTP/1.1\r\n\r\n",
                b"GET /callback?code=abc&state=xyz HTTP/1.1\r\nHost: x\r\n\r\n",
            ],
        );
        assert_eq!(result.unwrap(), Callback::Code("abc".into()));
        assert!(response.contains("Test connected"));
    }
}

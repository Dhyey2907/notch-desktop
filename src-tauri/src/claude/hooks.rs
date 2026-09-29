//! Live Claude Code activity via Claude Code's HTTP hooks.
//!
//! Claude Code POSTs each hook event as JSON to a loopback URL. This module validates the
//! request (loopback Host, bearer token, no browser Origin, size cap), keeps only what the
//! notch shows — event name, session id, project folder, tool name — and never stores prompt
//! text or tool inputs.

use std::{
    collections::HashMap,
    io::Read,
    sync::{Arc, Mutex},
};

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Fixed so the URL written into Claude Code's settings stays valid across restarts.
pub const HOOK_PORT: u16 = 43822;
pub const HOOK_PATH: &str = "/hook";
/// Where Notch's status-line command posts Claude Code's status-line JSON (plan limits).
pub const STATUS_LINE_PATH: &str = "/statusline";
/// Largest hook body accepted (tool inputs can be large; we only read a few fields).
const MAX_BODY_BYTES: u64 = 1024 * 1024;
/// Sessions with no events for this long are dropped from the live view.
const STALE_AFTER_MINUTES: i64 = 120;

pub fn hook_url() -> String {
    format!("http://127.0.0.1:{HOOK_PORT}{HOOK_PATH}")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum AgentState {
    /// Processing a prompt or running a tool.
    Working,
    /// Blocked on you (permission prompt or a question).
    Waiting,
    /// Finished its turn.
    Idle,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AgentSession {
    pub session_id: String,
    /// Folder name of the session's working directory.
    pub project: String,
    pub state: AgentState,
    /// Tool currently running, e.g. "Bash" or "Edit".
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool: Option<String>,
    pub updated_at: DateTime<Utc>,
}

/// The subset of a hook payload the notch uses. Everything else (prompt text, tool inputs,
/// transcript paths) is ignored.
#[derive(Debug, Deserialize)]
pub struct HookPayload {
    pub hook_event_name: String,
    #[serde(default)]
    pub session_id: String,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub tool_name: Option<String>,
    #[serde(default)]
    pub notification_type: Option<String>,
    /// Process that sent the hook (looked up from the loopback connection, not the body).
    #[serde(skip)]
    pub client_pid: Option<u32>,
}

fn folder_name(cwd: &str) -> String {
    cwd.trim_end_matches(['\\', '/'])
        .rsplit(['\\', '/'])
        .next()
        .unwrap_or(cwd)
        .to_owned()
}

/// Live sessions keyed by session id.
#[derive(Debug, Default)]
pub struct LiveSessions {
    sessions: HashMap<String, AgentSession>,
    /// Claude Code process behind each session, when known.
    pids: HashMap<String, u32>,
}

impl LiveSessions {
    /// Apply one hook event. Returns true if anything the UI shows changed.
    pub fn apply(&mut self, payload: &HookPayload, now: DateTime<Utc>) -> bool {
        if payload.session_id.is_empty() || payload.session_id.len() > 128 {
            return false;
        }
        if payload.hook_event_name == "SessionEnd" {
            self.pids.remove(&payload.session_id);
            return self.sessions.remove(&payload.session_id).is_some();
        }
        if let Some(pid) = payload.client_pid {
            self.pids.insert(payload.session_id.clone(), pid);
        }
        let (state, tool) = match payload.hook_event_name.as_str() {
            "SessionStart" | "Stop" | "SubagentStop" => (AgentState::Idle, None),
            "UserPromptSubmit" | "PostToolUse" | "PostToolUseFailure" => (AgentState::Working, None),
            "PreToolUse" => (AgentState::Working, payload.tool_name.clone()),
            "PermissionRequest" => (AgentState::Waiting, payload.tool_name.clone()),
            "Notification" => match payload.notification_type.as_deref() {
                Some("permission_prompt") | Some("agent_needs_input") | Some("elicitation_dialog") => {
                    (AgentState::Waiting, None)
                }
                Some("idle_prompt") => (AgentState::Idle, None),
                _ => return false,
            },
            _ => return false,
        };
        let tool = tool.map(|t| t.chars().take(48).collect::<String>());
        let is_new = !self.sessions.contains_key(&payload.session_id);
        let entry = self
            .sessions
            .entry(payload.session_id.clone())
            .or_insert_with(|| AgentSession {
                session_id: payload.session_id.clone(),
                project: String::new(),
                state,
                tool: None,
                updated_at: now,
            });
        if let Some(cwd) = payload.cwd.as_deref().filter(|c| !c.is_empty()) {
            entry.project = folder_name(cwd);
        }
        let changed = is_new || entry.state != state || entry.tool != tool;
        entry.state = state;
        entry.tool = tool;
        entry.updated_at = now;
        changed
    }

    /// Drop every session run by a process that has exited. Returns their ids.
    pub fn process_exited(&mut self, pid: u32) -> Vec<String> {
        let ended: Vec<String> = self
            .pids
            .iter()
            .filter(|(_, p)| **p == pid)
            .map(|(s, _)| s.clone())
            .collect();
        for session in &ended {
            self.pids.remove(session);
            self.sessions.remove(session);
        }
        ended
    }

    /// Sessions to show, most recently active first; stale ones are dropped.
    pub fn snapshot(&mut self, now: DateTime<Utc>) -> Vec<AgentSession> {
        self.sessions
            .retain(|_, s| now - s.updated_at < Duration::minutes(STALE_AFTER_MINUTES));
        let mut list: Vec<AgentSession> = self.sessions.values().cloned().collect();
        list.sort_by_key(|s| std::cmp::Reverse(s.updated_at));
        list
    }
}

/// Why a request was refused (logged by kind only).
#[derive(Debug, PartialEq, Eq)]
pub enum Rejection {
    NotFound,
    BadHost,
    BrowserOrigin,
    Unauthorized,
    TooLarge,
}

/// Validate method, path, Host, Origin, and token. Header names are matched case-insensitively.
pub fn check_request(
    method: &str,
    path: &str,
    headers: &[(String, String)],
    content_length: Option<u64>,
    token: &str,
    port: u16,
) -> Result<(), Rejection> {
    let header = |name: &str| {
        headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    };
    if method != "POST" || (path != HOOK_PATH && path != STATUS_LINE_PATH) {
        return Err(Rejection::NotFound);
    }
    // DNS-rebinding guard: only accept requests addressed to the loopback host itself.
    let host_ok =
        header("host").is_some_and(|h| h == format!("127.0.0.1:{port}") || h == format!("localhost:{port}"));
    if !host_ok {
        return Err(Rejection::BadHost);
    }
    // Browsers attach Origin to cross-site POSTs; Claude Code doesn't.
    if header("origin").is_some() {
        return Err(Rejection::BrowserOrigin);
    }
    let presented = header("authorization")
        .and_then(|v| v.strip_prefix("Bearer "))
        .unwrap_or("");
    if !constant_time_eq(presented.as_bytes(), token.as_bytes()) || token.is_empty() {
        return Err(Rejection::Unauthorized);
    }
    if content_length.is_some_and(|len| len > MAX_BODY_BYTES) {
        return Err(Rejection::TooLarge);
    }
    Ok(())
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

/// Called for every accepted hook with the parsed payload. Returns the JSON response body.
/// Handles one hook: the parsed common fields plus the raw body (permission requests need
/// the tool input). Returns the JSON response body.
pub type HookHandler = Arc<dyn Fn(HookPayload, &[u8]) -> String + Send + Sync>;
/// Handles one status-line update (raw body). Returns the text Claude Code shows.
pub type StatusLineHandler = Arc<dyn Fn(&[u8]) -> String + Send + Sync>;

#[derive(Clone)]
pub struct Handlers {
    pub hook: HookHandler,
    pub status_line: StatusLineHandler,
}

/// A running hook server. Dropping it stops accepting requests.
pub struct HookServer {
    server: Arc<tiny_http::Server>,
}

impl Drop for HookServer {
    fn drop(&mut self) {
        self.server.unblock();
    }
}

impl HookServer {
    pub fn start(port: u16, token: String, handlers: Handlers) -> std::io::Result<Self> {
        let server = tiny_http::Server::http(("127.0.0.1", port))
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::AddrInUse, e.to_string()))?;
        let server = Arc::new(server);
        let accept = server.clone();
        let token = Arc::new(Mutex::new(token));
        std::thread::Builder::new()
            .name("claude-hooks".into())
            .spawn(move || {
                for request in accept.incoming_requests() {
                    let token = token.lock().unwrap_or_else(|e| e.into_inner()).clone();
                    let handlers = handlers.clone();
                    // One thread per request: a permission request waits on the user.
                    std::thread::spawn(move || handle(request, &token, &handlers, port));
                }
            })?;
        Ok(Self { server })
    }
}

fn respond(request: tiny_http::Request, status: u16, body: String) {
    respond_as(request, status, body, "application/json");
}

fn respond_as(request: tiny_http::Request, status: u16, body: String, content_type: &str) {
    let header =
        tiny_http::Header::from_bytes(&b"Content-Type"[..], content_type.as_bytes()).expect("static header");
    let _ = request.respond(
        tiny_http::Response::from_string(body)
            .with_status_code(status)
            .with_header(header),
    );
}

fn handle(mut request: tiny_http::Request, token: &str, handlers: &Handlers, port: u16) {
    let headers: Vec<(String, String)> = request
        .headers()
        .iter()
        .map(|h| (h.field.as_str().as_str().to_owned(), h.value.as_str().to_owned()))
        .collect();
    let method = request.method().as_str().to_owned();
    // Look up the sender while its connection is still open.
    let client_pid = request
        .remote_addr()
        .filter(|a| a.ip().is_loopback())
        .and_then(|a| super::process::loopback_client_pid(a.port(), port));
    let path = request.url().split('?').next().unwrap_or("").to_owned();
    if let Err(rejection) = check_request(
        &method,
        &path,
        &headers,
        request.body_length().map(|l| l as u64),
        token,
        port,
    ) {
        tracing::debug!(?rejection, "hook request refused");
        let status = match rejection {
            Rejection::NotFound => 404,
            Rejection::TooLarge => 413,
            _ => 403,
        };
        respond(request, status, "{}".into());
        return;
    }
    let mut body = Vec::new();
    if request
        .as_reader()
        .take(MAX_BODY_BYTES + 1)
        .read_to_end(&mut body)
        .is_err()
        || body.len() as u64 > MAX_BODY_BYTES
    {
        respond(request, 413, "{}".into());
        return;
    }
    if path == STATUS_LINE_PATH {
        let text = (handlers.status_line)(&body);
        respond_as(request, 200, text, "text/plain; charset=utf-8");
        return;
    }
    match serde_json::from_slice::<HookPayload>(&body) {
        Ok(mut payload) => {
            payload.client_pid = client_pid;
            let response = (handlers.hook)(payload, &body);
            respond(request, 200, response);
        }
        Err(_) => respond(request, 400, "{}".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn payload(event: &str, session: &str) -> HookPayload {
        HookPayload {
            hook_event_name: event.into(),
            session_id: session.into(),
            cwd: Some(r"C:\dev\notch-desktop".into()),
            tool_name: None,
            notification_type: None,
            client_pid: None,
        }
    }

    #[test]
    fn sessions_end_when_their_process_exits() {
        let now = Utc::now();
        let mut live = LiveSessions::default();
        let from = |event: &str, session: &str, pid: u32| HookPayload {
            client_pid: Some(pid),
            ..payload(event, session)
        };
        live.apply(&from("UserPromptSubmit", "a", 10), now);
        live.apply(&from("UserPromptSubmit", "b", 10), now);
        live.apply(&from("UserPromptSubmit", "c", 20), now);
        let mut ended = live.process_exited(10);
        ended.sort();
        assert_eq!(ended, ["a", "b"]);
        let left: Vec<_> = live.snapshot(now).into_iter().map(|s| s.session_id).collect();
        assert_eq!(left, ["c"]);
        assert!(live.process_exited(99).is_empty());
    }

    /// Real sockets: start the server on a free port and talk HTTP to it.
    #[test]
    fn server_accepts_valid_hooks_and_refuses_everything_else() {
        use std::io::Write;
        use std::net::TcpStream;

        let port = std::net::TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let seen = Arc::new(Mutex::new(Vec::<(String, Option<u32>)>::new()));
        let seen_by_handler = seen.clone();
        let handler: HookHandler = Arc::new(move |p: HookPayload, _: &[u8]| {
            seen_by_handler
                .lock()
                .unwrap()
                .push((p.hook_event_name, p.client_pid));
            "{}".into()
        });
        let status_line: StatusLineHandler = Arc::new(|body: &[u8]| format!("got {}", body.len()));
        let _server = HookServer::start(
            port,
            "s3cret".into(),
            Handlers {
                hook: handler,
                status_line,
            },
        )
        .unwrap();

        let send_to = |path: &str, host: &str, auth: &str, extra: &str, body: &str| -> String {
            let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
            let request = format!(
                "POST {path} HTTP/1.1\r\nHost: {host}\r\nAuthorization: {auth}\r\n{extra}Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            s.write_all(request.as_bytes()).unwrap();
            let mut response = String::new();
            let _ = std::io::Read::read_to_string(&mut s, &mut response);
            response
        };
        let send = |host: &str, auth: &str, extra: &str, body: &str| -> String {
            send_to("/hook", host, auth, extra, body)
                .lines()
                .next()
                .unwrap_or_default()
                .to_owned()
        };
        let host = format!("127.0.0.1:{port}");
        let event =
            r#"{"hook_event_name":"Stop","session_id":"s1","cwd":"C:\\dev\\x","prompt":"never stored"}"#;

        assert!(send(&host, "Bearer s3cret", "", event).contains("200"));
        assert!(send(&host, "Bearer wrong", "", event).contains("403"));
        assert!(send("evil.example", "Bearer s3cret", "", event).contains("403"));
        assert!(send(&host, "Bearer s3cret", "Origin: https://evil.example\r\n", event).contains("403"));
        assert!(send(&host, "Bearer s3cret", "", "not json").contains("400"));
        // The sender is this test process, found from the loopback connection.
        let expected_pid = cfg!(windows).then(std::process::id);
        assert_eq!(*seen.lock().unwrap(), [("Stop".to_owned(), expected_pid)]);

        let status = send_to("/statusline", &host, "Bearer s3cret", "", "{}");
        assert!(status.starts_with("HTTP/1.1 200") && status.ends_with("got 2"));
        assert!(send_to("/statusline", &host, "Bearer wrong", "", "{}").contains("403"));
    }

    #[test]
    fn tracks_a_session_through_a_turn() {
        let mut live = LiveSessions::default();
        let now = Utc::now();
        assert!(live.apply(&payload("SessionStart", "s1"), now));
        assert!(live.apply(&payload("UserPromptSubmit", "s1"), now));
        let mut tool = payload("PreToolUse", "s1");
        tool.tool_name = Some("Bash".into());
        assert!(live.apply(&tool, now));
        let snap = live.snapshot(now);
        assert_eq!(snap[0].state, AgentState::Working);
        assert_eq!(snap[0].tool.as_deref(), Some("Bash"));
        assert_eq!(snap[0].project, "notch-desktop");

        let mut waiting = payload("Notification", "s1");
        waiting.notification_type = Some("permission_prompt".into());
        live.apply(&waiting, now);
        assert_eq!(live.snapshot(now)[0].state, AgentState::Waiting);

        live.apply(&payload("Stop", "s1"), now);
        assert_eq!(live.snapshot(now)[0].state, AgentState::Idle);
        assert!(live.apply(&payload("SessionEnd", "s1"), now));
        assert!(live.snapshot(now).is_empty());
    }

    #[test]
    fn ignores_unknown_events_and_drops_stale_sessions() {
        let mut live = LiveSessions::default();
        let now = Utc::now();
        assert!(!live.apply(&payload("SomethingNew", "s1"), now));
        assert!(!live.apply(&payload("Stop", ""), now));
        live.apply(&payload("Stop", "old"), now - Duration::hours(3));
        live.apply(&payload("Stop", "new"), now);
        let ids: Vec<_> = live.snapshot(now).into_iter().map(|s| s.session_id).collect();
        assert_eq!(ids, ["new"]);
    }

    fn headers(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn accepts_only_authenticated_loopback_requests() {
        let host = format!("127.0.0.1:{HOOK_PORT}");
        let good = headers(&[("Host", &host), ("Authorization", "Bearer tok")]);
        assert_eq!(
            check_request("POST", "/hook", &good, Some(10), "tok", HOOK_PORT),
            Ok(())
        );

        assert_eq!(
            check_request("GET", "/hook", &good, None, "tok", HOOK_PORT),
            Err(Rejection::NotFound)
        );
        assert_eq!(
            check_request("POST", "/other", &good, None, "tok", HOOK_PORT),
            Err(Rejection::NotFound)
        );
        let rebind = headers(&[("Host", "evil.example:43822"), ("Authorization", "Bearer tok")]);
        assert_eq!(
            check_request("POST", "/hook", &rebind, None, "tok", HOOK_PORT),
            Err(Rejection::BadHost)
        );
        let browser = headers(&[
            ("host", &host),
            ("authorization", "Bearer tok"),
            ("Origin", "https://evil.example"),
        ]);
        assert_eq!(
            check_request("POST", "/hook", &browser, None, "tok", HOOK_PORT),
            Err(Rejection::BrowserOrigin)
        );
        let wrong = headers(&[("Host", &host), ("Authorization", "Bearer nope")]);
        assert_eq!(
            check_request("POST", "/hook", &wrong, None, "tok", HOOK_PORT),
            Err(Rejection::Unauthorized)
        );
        assert_eq!(
            check_request("POST", "/hook", &good, None, "", HOOK_PORT),
            Err(Rejection::Unauthorized)
        );
        assert_eq!(
            check_request("POST", "/hook", &good, Some(MAX_BODY_BYTES + 1), "tok", HOOK_PORT),
            Err(Rejection::TooLarge)
        );
    }
}

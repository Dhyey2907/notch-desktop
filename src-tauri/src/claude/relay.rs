//! `notch-desktop.exe --agent-hook <codex|gemini>`: the command Codex and Gemini CLI run for
//! each hook event. It forwards the event to the running Notch over loopback and prints
//! Notch's answer — or `{}` (no decision) when Notch isn't running, so the agent carries on as
//! if Notch weren't there. It never prints anything else: both agents read stdout as JSON.

use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpStream},
    time::Duration,
};

use super::agents::{Agent, RELAY_FLAG};
use crate::secrets::{self, Credential};

const MAX_INPUT: u64 = 1024 * 1024;
const QUICK: Duration = Duration::from_secs(8);
const NO_DECISION: &str = "{}";

/// When started as a relay, run it and return the exit code.
pub fn from_args() -> Option<i32> {
    let args: Vec<String> = std::env::args().collect();
    let at = args.iter().position(|a| a == RELAY_FLAG)?;
    let agent = args.get(at + 1).and_then(|id| Agent::from_id(id));
    let output = agent.map(run).unwrap_or_else(|| NO_DECISION.to_owned());
    let mut stdout = std::io::stdout();
    let _ = stdout.write_all(output.as_bytes());
    let _ = stdout.flush();
    // Always succeed: a failing hook must never get in the agent's way.
    Some(0)
}

fn run(agent: Agent) -> String {
    let mut body = Vec::new();
    if std::io::stdin().take(MAX_INPUT).read_to_end(&mut body).is_err() {
        return NO_DECISION.into();
    }
    let event = serde_json::from_slice::<serde_json::Value>(&body)
        .ok()
        .and_then(|v| v.get("hook_event_name")?.as_str().map(str::to_owned))
        .unwrap_or_default();
    let Ok(Some(token)) = secrets::get(Credential::ClaudeHookToken) else {
        return NO_DECISION.into();
    };
    // Approvals wait for your answer in the notch; everything else is quick.
    let wait = if event == "PermissionRequest" {
        Duration::from_secs(super::approvals::HOOK_TIMEOUT_SECS + 5)
    } else {
        QUICK
    };
    post(agent, &token, agent_pid(), &body, wait).unwrap_or_else(|| NO_DECISION.into())
}

fn post(agent: Agent, token: &str, pid: Option<u32>, body: &[u8], wait: Duration) -> Option<String> {
    let addr = SocketAddr::from(([127, 0, 0, 1], super::hooks::HOOK_PORT));
    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_millis(500)).ok()?;
    stream.set_read_timeout(Some(wait)).ok()?;
    stream.set_write_timeout(Some(QUICK)).ok()?;
    let pid_header = pid
        .map(|p| format!("X-Notch-Agent-Pid: {p}\r\n"))
        .unwrap_or_default();
    let head = format!(
        "POST {} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nAuthorization: Bearer {token}\r\nContent-Type: application/json\r\n{pid_header}Content-Length: {}\r\nConnection: close\r\n\r\n",
        agent.hook_path(),
        super::hooks::HOOK_PORT,
        body.len()
    );
    stream.write_all(head.as_bytes()).ok()?;
    stream.write_all(body).ok()?;
    let mut response = Vec::new();
    stream.take(MAX_INPUT).read_to_end(&mut response).ok()?;
    parse_response(&response)
}

/// The body of a `200` response with a JSON object body.
pub fn parse_response(response: &[u8]) -> Option<String> {
    let text = std::str::from_utf8(response).ok()?;
    let (head, body) = text.split_once("\r\n\r\n")?;
    let status = head.lines().next()?.split_whitespace().nth(1)?;
    let body = body.trim();
    (status == "200" && body.starts_with('{')).then(|| body.to_owned())
}

/// Shells the agent may run the hook through; the agent is the first ancestor that isn't one.
const SHELLS: &[&str] = &[
    "cmd.exe",
    "powershell.exe",
    "pwsh.exe",
    "bash.exe",
    "sh.exe",
    "conhost.exe",
    "wsl.exe",
];

/// The agent's own process, so Notch can tell when that session ends.
fn agent_pid() -> Option<u32> {
    #[cfg(windows)]
    {
        let parents = win::process_table();
        let mut pid = std::process::id();
        for _ in 0..6 {
            let (parent, _) = *parents.get(&pid)?;
            let name = parents.get(&parent).map(|(_, n)| n.as_str()).unwrap_or("");
            if !SHELLS.contains(&name) {
                return Some(parent);
            }
            pid = parent;
        }
        None
    }
    #[cfg(not(windows))]
    None
}

#[cfg(windows)]
mod win {
    use std::collections::HashMap;

    use windows::Win32::{
        Foundation::CloseHandle,
        System::Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
        },
    };

    /// pid → (parent pid, lowercase exe name).
    pub fn process_table() -> HashMap<u32, (u32, String)> {
        let mut table = HashMap::new();
        // SAFETY: a snapshot handle closed before returning; the entry struct is sized properly.
        unsafe {
            let Ok(snapshot) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else {
                return table;
            };
            let mut entry = PROCESSENTRY32W {
                dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
                ..Default::default()
            };
            let mut ok = Process32FirstW(snapshot, &mut entry).is_ok();
            while ok {
                let len = entry
                    .szExeFile
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(entry.szExeFile.len());
                let name = String::from_utf16_lossy(&entry.szExeFile[..len]).to_lowercase();
                table.insert(entry.th32ProcessID, (entry.th32ParentProcessID, name));
                ok = Process32NextW(snapshot, &mut entry).is_ok();
            }
            let _ = CloseHandle(snapshot);
        }
        table
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_only_successful_json_answers() {
        let ok =
            b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 13\r\n\r\n{\"a\":\"b\"}\n";
        assert_eq!(parse_response(ok).as_deref(), Some("{\"a\":\"b\"}"));
        assert_eq!(parse_response(b"HTTP/1.1 403 Forbidden\r\n\r\n{}"), None);
        assert_eq!(parse_response(b"HTTP/1.1 200 OK\r\n\r\nhello"), None);
        assert_eq!(parse_response(b"garbage"), None);
    }

    #[cfg(windows)]
    #[test]
    fn finds_its_own_process_in_the_table() {
        let table = win::process_table();
        assert!(table.contains_key(&std::process::id()));
    }
}

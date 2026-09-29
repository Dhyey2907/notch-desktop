//! Which Claude Code process sent a hook, and noticing when it exits.
//!
//! Claude Code only reports a session ending when it shuts down normally. If it's killed
//! (Task Manager, a crash, closing the desktop app) no `SessionEnd` arrives, and the session
//! would look "working" forever. So when a hook arrives over loopback, Notch looks up the
//! process on the other end of the connection and waits on its exit — event-driven, no polling.

/// The process that owns the client side of a loopback connection to `server_port`.
#[cfg(windows)]
pub fn loopback_client_pid(client_port: u16, server_port: u16) -> Option<u32> {
    use windows::Win32::{
        NetworkManagement::IpHelper::{GetExtendedTcpTable, MIB_TCPTABLE_OWNER_PID, TCP_TABLE_OWNER_PID_ALL},
        Networking::WinSock::AF_INET,
    };

    // Ports are stored in network byte order in the low 16 bits.
    let port_of = |raw: u32| u16::from_be((raw & 0xFFFF) as u16);
    let mut size = 0u32;
    // SAFETY: the first call only reports the required buffer size.
    unsafe {
        GetExtendedTcpTable(
            None,
            &mut size,
            false,
            AF_INET.0 as u32,
            TCP_TABLE_OWNER_PID_ALL,
            0,
        );
    }
    // Connections can open between the two calls; leave some headroom.
    let mut buffer = vec![0u8; size as usize + 4096];
    size = buffer.len() as u32;
    // SAFETY: `buffer` is writable for `size` bytes; the table layout is defined by the API.
    let status = unsafe {
        GetExtendedTcpTable(
            Some(buffer.as_mut_ptr().cast()),
            &mut size,
            false,
            AF_INET.0 as u32,
            TCP_TABLE_OWNER_PID_ALL,
            0,
        )
    };
    if status != 0 {
        return None;
    }
    // SAFETY: on success the buffer holds a MIB_TCPTABLE_OWNER_PID with `dwNumEntries` rows.
    let rows = unsafe {
        let table = &*(buffer.as_ptr() as *const MIB_TCPTABLE_OWNER_PID);
        std::slice::from_raw_parts(table.table.as_ptr(), table.dwNumEntries as usize)
    };
    rows.iter()
        .find(|row| port_of(row.dwLocalPort) == client_port && port_of(row.dwRemotePort) == server_port)
        .map(|row| row.dwOwningPid)
        .filter(|pid| *pid != 0)
}

#[cfg(not(windows))]
pub fn loopback_client_pid(_client_port: u16, _server_port: u16) -> Option<u32> {
    None
}

/// Run `on_exit` once when process `pid` exits. Returns false if the process can't be
/// watched (already gone, or access denied); the caller keeps its time-based fallback.
#[cfg(windows)]
pub fn watch_exit(pid: u32, on_exit: impl FnOnce() + Send + 'static) -> bool {
    use windows::Win32::{
        Foundation::{CloseHandle, HANDLE},
        System::Threading::{OpenProcess, WaitForSingleObject, INFINITE, PROCESS_SYNCHRONIZE},
    };

    // SAFETY: SYNCHRONIZE access only allows waiting on the process.
    let Ok(handle) = (unsafe { OpenProcess(PROCESS_SYNCHRONIZE, false, pid) }) else {
        return false;
    };
    // HANDLE isn't Send; move the raw value into the waiting thread.
    let raw = handle.0 as usize;
    std::thread::Builder::new()
        .name("claude-exit-watch".into())
        .spawn(move || {
            let handle = HANDLE(raw as *mut _);
            // SAFETY: `handle` is a valid process handle owned by this thread until closed.
            unsafe {
                WaitForSingleObject(handle, INFINITE);
                let _ = CloseHandle(handle);
            }
            on_exit();
        })
        .is_ok()
}

#[cfg(not(windows))]
pub fn watch_exit(_pid: u32, _on_exit: impl FnOnce() + Send + 'static) -> bool {
    false
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use std::{sync::mpsc, time::Duration};

    #[test]
    fn finds_the_process_on_the_other_end_of_a_loopback_connection() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let server_port = listener.local_addr().unwrap().port();
        let client = std::net::TcpStream::connect(("127.0.0.1", server_port)).unwrap();
        let (_accepted, peer) = listener.accept().unwrap();
        assert_eq!(peer.port(), client.local_addr().unwrap().port());
        assert_eq!(
            loopback_client_pid(peer.port(), server_port),
            Some(std::process::id())
        );
        assert_eq!(loopback_client_pid(1, server_port), None);
    }

    #[test]
    fn notices_when_a_process_exits() {
        // Lives about a second, so the watch is always in place before it exits.
        let mut child = std::process::Command::new("ping")
            .args(["-n", "2", "127.0.0.1"])
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap();
        let (tx, rx) = mpsc::channel();
        assert!(watch_exit(child.id(), move || {
            let _ = tx.send(());
        }));
        assert!(rx.recv_timeout(Duration::from_millis(100)).is_err()); // still running
        let _ = child.wait();
        assert!(rx.recv_timeout(Duration::from_secs(5)).is_ok());
    }
}

//! Which apps are using the network right now. Windows only shows exact per-app bandwidth to
//! administrators, so this estimates it: apps with open internet connections, and how fast
//! their network-style I/O (Windows' "other" I/O bytes, mostly socket traffic) is moving.

use std::collections::HashMap;

use serde::Serialize;
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AppTraffic {
    /// "chrome", "Spotify"…
    pub name: String,
    /// Open connections to other machines.
    pub connections: u32,
    /// Estimated bytes per second.
    #[ts(type = "number")]
    pub rate: u64,
}

/// One process with internet connections: (pid, name, connections, I/O bytes so far).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcSample {
    pub pid: u32,
    pub name: String,
    pub connections: u32,
    pub io_bytes: u64,
}

/// Per-app rates from two samples `millis` apart; an app's processes are added together.
/// Busiest first, then most connections; at most `limit` apps.
pub fn aggregate(
    before: &HashMap<u32, u64>,
    now: &[ProcSample],
    millis: u64,
    limit: usize,
) -> Vec<AppTraffic> {
    let mut apps: HashMap<String, AppTraffic> = HashMap::new();
    for p in now {
        let rate = before
            .get(&p.pid)
            .map_or(0, |&b| super::rate(b, p.io_bytes, millis));
        let app = apps.entry(p.name.to_lowercase()).or_insert_with(|| AppTraffic {
            name: p.name.clone(),
            connections: 0,
            rate: 0,
        });
        app.connections += p.connections;
        app.rate += rate;
    }
    let mut apps: Vec<AppTraffic> = apps.into_values().collect();
    apps.sort_by(|a, b| {
        b.rate
            .cmp(&a.rate)
            .then(b.connections.cmp(&a.connections))
            .then(a.name.cmp(&b.name))
    });
    apps.truncate(limit);
    apps
}

#[cfg(windows)]
pub fn sample() -> Vec<ProcSample> {
    use windows::core::PWSTR;
    use windows::Win32::{
        Foundation::CloseHandle,
        NetworkManagement::IpHelper::{
            GetExtendedTcpTable, MIB_TCP6TABLE_OWNER_PID, MIB_TCPTABLE_OWNER_PID, MIB_TCP_STATE_ESTAB,
            TCP_TABLE_OWNER_PID_ALL,
        },
        Networking::WinSock::{AF_INET, AF_INET6},
        System::Threading::{
            GetProcessIoCounters, OpenProcess, QueryFullProcessImageNameW, IO_COUNTERS, PROCESS_NAME_WIN32,
            PROCESS_QUERY_LIMITED_INFORMATION,
        },
    };

    let mut per_pid: HashMap<u32, u32> = HashMap::new();
    // SAFETY: the table is read into a buffer of the size Windows asked for; rows are read
    // within the count it reports.
    unsafe {
        for family in [AF_INET.0 as u32, AF_INET6.0 as u32] {
            let mut size = 0u32;
            let _ = GetExtendedTcpTable(None, &mut size, false, family, TCP_TABLE_OWNER_PID_ALL, 0);
            if size == 0 {
                continue;
            }
            let mut buf = vec![0u8; size as usize + 1024];
            if GetExtendedTcpTable(
                Some(buf.as_mut_ptr().cast()),
                &mut size,
                false,
                family,
                TCP_TABLE_OWNER_PID_ALL,
                0,
            ) != 0
            {
                continue;
            }
            if family == AF_INET.0 as u32 {
                let table = &*(buf.as_ptr() as *const MIB_TCPTABLE_OWNER_PID);
                let rows = std::slice::from_raw_parts(table.table.as_ptr(), table.dwNumEntries as usize);
                for row in rows {
                    // Established, and not to this PC itself (127.x).
                    let remote = u32::from_be(row.dwRemoteAddr).to_be_bytes();
                    if row.dwState == MIB_TCP_STATE_ESTAB.0 as u32 && remote[3] != 127 && row.dwOwningPid > 4
                    {
                        *per_pid.entry(row.dwOwningPid).or_default() += 1;
                    }
                }
            } else {
                let table = &*(buf.as_ptr() as *const MIB_TCP6TABLE_OWNER_PID);
                let rows = std::slice::from_raw_parts(table.table.as_ptr(), table.dwNumEntries as usize);
                for row in rows {
                    let loopback =
                        row.ucRemoteAddr[..15].iter().all(|b| *b == 0) && row.ucRemoteAddr[15] == 1;
                    if row.dwState == MIB_TCP_STATE_ESTAB.0 as u32 && !loopback && row.dwOwningPid > 4 {
                        *per_pid.entry(row.dwOwningPid).or_default() += 1;
                    }
                }
            }
        }
    }

    let mut out = Vec::new();
    for (pid, connections) in per_pid {
        // SAFETY: read-only queries on a handle we open and close here.
        unsafe {
            let Ok(process) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else {
                continue;
            };
            let mut buf = [0u16; 512];
            let mut len = buf.len() as u32;
            let name =
                QueryFullProcessImageNameW(process, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut len)
                    .ok()
                    .map(|_| String::from_utf16_lossy(&buf[..len as usize]))
                    .and_then(|path| {
                        std::path::Path::new(&path)
                            .file_stem()
                            .map(|s| s.to_string_lossy().into_owned())
                    });
            let mut io = IO_COUNTERS::default();
            let io_bytes = GetProcessIoCounters(process, &mut io)
                .ok()
                .map(|_| io.OtherTransferCount);
            let _ = CloseHandle(process);
            if let (Some(name), Some(io_bytes)) = (name, io_bytes) {
                out.push(ProcSample {
                    pid,
                    name,
                    connections,
                    io_bytes,
                });
            }
        }
    }
    out
}

#[cfg(not(windows))]
pub fn sample() -> Vec<ProcSample> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(pid: u32, name: &str, connections: u32, io_bytes: u64) -> ProcSample {
        ProcSample {
            pid,
            name: name.into(),
            connections,
            io_bytes,
        }
    }

    #[test]
    fn adds_up_an_apps_processes_and_ranks_the_busiest() {
        let before = HashMap::from([(1, 1_000), (2, 5_000), (3, 0)]);
        let now = [
            p(1, "chrome", 10, 3_000),
            p(2, "Chrome", 2, 6_000),
            p(3, "Spotify", 1, 10_000),
            p(4, "Teams", 5, 999_999),
        ];
        let apps = aggregate(&before, &now, 1000, 10);
        assert_eq!(apps[0].name, "Spotify");
        assert_eq!(apps[0].rate, 10_000);
        assert_eq!(apps[1].connections, 12);
        assert_eq!(apps[1].rate, 3_000);
        // New process: no rate until the next sample, but it's listed.
        assert_eq!(apps[2].name, "Teams");
        assert_eq!(apps[2].rate, 0);
        assert_eq!(aggregate(&before, &now, 1000, 1).len(), 1);
    }
}

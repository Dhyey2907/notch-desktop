//! Who listens on which local TCP port (the same table `netstat -ano` shows), and those
//! processes' command lines (WMI), to find and name dev servers.

use std::collections::HashMap;

use windows::{
    core::{BSTR, PCWSTR},
    Win32::{
        NetworkManagement::IpHelper::{
            GetExtendedTcpTable, MIB_TCP6ROW_OWNER_PID, MIB_TCPROW_OWNER_PID, TCP_TABLE_OWNER_PID_LISTENER,
        },
        Networking::WinSock::{AF_INET, AF_INET6},
        System::{
            Com::{
                CoCreateInstance, CoInitializeEx, CoSetProxyBlanket, CLSCTX_INPROC_SERVER,
                COINIT_MULTITHREADED, EOAC_NONE, RPC_C_AUTHN_LEVEL_CALL, RPC_C_IMP_LEVEL_IMPERSONATE,
            },
            Rpc::{RPC_C_AUTHN_WINNT, RPC_C_AUTHZ_NONE},
            Variant::VARIANT,
            Wmi::{
                IWbemLocator, WbemLocator, WBEM_FLAG_FORWARD_ONLY, WBEM_FLAG_RETURN_IMMEDIATELY,
                WBEM_GENERIC_FLAG_TYPE, WBEM_INFINITE,
            },
        },
    },
};

/// (port, pid) for every listening TCP socket, IPv4 and IPv6, without duplicates.
pub fn listeners() -> Vec<(u16, u32)> {
    let mut out = Vec::new();
    for family in [AF_INET.0, AF_INET6.0] {
        let mut size = 0u32;
        // SAFETY: first call asks for the size; the second fills a buffer of that size, read
        // as the documented table layout (a count, then rows).
        unsafe {
            GetExtendedTcpTable(
                None,
                &mut size,
                false,
                u32::from(family),
                TCP_TABLE_OWNER_PID_LISTENER,
                0,
            );
            if size == 0 {
                continue;
            }
            let mut buffer = vec![0u64; (size as usize).div_ceil(8)];
            let ptr = buffer.as_mut_ptr().cast::<core::ffi::c_void>();
            if GetExtendedTcpTable(
                Some(ptr),
                &mut size,
                false,
                u32::from(family),
                TCP_TABLE_OWNER_PID_LISTENER,
                0,
            ) != 0
            {
                continue;
            }
            let count = *(ptr as *const u32) as usize;
            // Rows follow the u32 entry count.
            let rows = (ptr as *const u8).add(4);
            for i in 0..count {
                let (port, pid) = if family == AF_INET.0 {
                    let row = &*(rows.cast::<MIB_TCPROW_OWNER_PID>()).add(i);
                    (row.dwLocalPort, row.dwOwningPid)
                } else {
                    let row = &*(rows.cast::<MIB_TCP6ROW_OWNER_PID>()).add(i);
                    (row.dwLocalPort, row.dwOwningPid)
                };
                // The port is in network byte order in the low 16 bits.
                let port = u16::from_be(port as u16);
                if !out.contains(&(port, pid)) {
                    out.push((port, pid));
                }
            }
        }
    }
    out
}

/// Command lines of these processes (missing ones are left out).
pub fn command_lines(pids: &[u32]) -> HashMap<u32, String> {
    let mut out = HashMap::new();
    if pids.is_empty() {
        return out;
    }
    // SAFETY: COM on this thread; WMI objects are released when dropped.
    let result: windows::core::Result<()> = unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        (|| {
            let locator: IWbemLocator = CoCreateInstance(&WbemLocator, None, CLSCTX_INPROC_SERVER)?;
            let services = locator.ConnectServer(
                &BSTR::from("ROOT\\CIMV2"),
                &BSTR::new(),
                &BSTR::new(),
                &BSTR::new(),
                0,
                &BSTR::new(),
                None,
            )?;
            CoSetProxyBlanket(
                &services,
                RPC_C_AUTHN_WINNT,
                RPC_C_AUTHZ_NONE,
                PCWSTR::null(),
                RPC_C_AUTHN_LEVEL_CALL,
                RPC_C_IMP_LEVEL_IMPERSONATE,
                None,
                EOAC_NONE,
            )?;
            let filter = pids
                .iter()
                .map(|p| format!("ProcessId={p}"))
                .collect::<Vec<_>>()
                .join(" OR ");
            let rows = services.ExecQuery(
                &BSTR::from("WQL"),
                &BSTR::from(format!(
                    "SELECT ProcessId, CommandLine FROM Win32_Process WHERE {filter}"
                )),
                WBEM_GENERIC_FLAG_TYPE(WBEM_FLAG_FORWARD_ONLY.0 | WBEM_FLAG_RETURN_IMMEDIATELY.0),
                None,
            )?;
            loop {
                let mut found = [None];
                let mut returned = 0;
                rows.Next(WBEM_INFINITE, &mut found, &mut returned).ok()?;
                let Some(row) = found[0].take() else { break };
                if returned == 0 {
                    break;
                }
                let mut pid = VARIANT::default();
                let mut line = VARIANT::default();
                row.Get(windows::core::w!("ProcessId"), 0, &mut pid, None, None)?;
                row.Get(windows::core::w!("CommandLine"), 0, &mut line, None, None)?;
                if let (Ok(pid), Ok(line)) = (u32::try_from(&pid), BSTR::try_from(&line)) {
                    out.insert(pid, line.to_string());
                }
            }
            Ok(())
        })()
    };
    if let Err(error) = result {
        tracing::debug!(%error, "could not read command lines");
    }
    out
}

/// End a process (the user pressed Stop on its dev server).
pub fn kill(pid: u32) -> bool {
    use windows::Win32::{
        Foundation::CloseHandle,
        System::Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE},
    };
    // SAFETY: opening, terminating and closing a handle to one process.
    unsafe {
        let Ok(handle) = OpenProcess(PROCESS_TERMINATE, false, pid) else {
            return false;
        };
        let ok = TerminateProcess(handle, 1).is_ok();
        let _ = CloseHandle(handle);
        ok
    }
}

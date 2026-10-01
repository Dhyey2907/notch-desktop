//! Round-trip time to a well-known server (1.1.1.1), with Windows' ICMP helper — no admin.

use windows::Win32::NetworkManagement::IpHelper::{IcmpCloseHandle, IcmpCreateFile, IcmpSendEcho};

const TARGET: [u8; 4] = [1, 1, 1, 1];
const TIMEOUT_MS: u32 = 1000;

/// Milliseconds, or None if it timed out or the network is down.
pub fn ping_ms() -> Option<u32> {
    let payload = *b"notch";
    // Reply = ICMP_ECHO_REPLY (status at offset 4, round trip at 8) + payload + room for errors.
    let mut reply = [0u8; 128];
    // SAFETY: an ICMP handle opened and closed here; the reply buffer outlives the call and
    // is larger than one reply plus the payload.
    unsafe {
        let handle = IcmpCreateFile().ok()?;
        let count = IcmpSendEcho(
            handle,
            u32::from_ne_bytes(TARGET),
            payload.as_ptr().cast(),
            payload.len() as u16,
            None,
            reply.as_mut_ptr().cast(),
            reply.len() as u32,
            TIMEOUT_MS,
        );
        let _ = IcmpCloseHandle(handle);
        if count == 0 {
            return None;
        }
    }
    let status = u32::from_ne_bytes(reply[4..8].try_into().ok()?);
    let rtt = u32::from_ne_bytes(reply[8..12].try_into().ok()?);
    (status == 0).then_some(rtt)
}

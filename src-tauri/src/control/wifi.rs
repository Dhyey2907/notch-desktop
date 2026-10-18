//! Wi-Fi quick switch: nearby networks (Windows' Native Wifi API) with signal strength, and one
//! click to join a network this PC already knows. New networks open Windows' own Wi-Fi list,
//! so Overline never sees or types a password.

use serde::Serialize;
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WifiNetwork {
    pub ssid: String,
    /// 0–100.
    pub signal: u8,
    pub secure: bool,
    /// Joined before: one click connects (Windows has its profile).
    pub known: bool,
    pub connected: bool,
    /// The saved profile's name (usually the SSID), for connecting.
    #[serde(skip)]
    pub profile: String,
}

/// One entry per network name: connected first, then known, then by signal. Hidden networks
/// (no name) are left out.
pub fn tidy(mut raw: Vec<WifiNetwork>) -> Vec<WifiNetwork> {
    raw.retain(|n| !n.ssid.trim().is_empty());
    raw.sort_by(|a, b| {
        b.connected
            .cmp(&a.connected)
            .then(b.known.cmp(&a.known))
            .then(b.signal.cmp(&a.signal))
    });
    let mut out: Vec<WifiNetwork> = Vec::new();
    for n in raw {
        match out.iter_mut().find(|o| o.ssid == n.ssid) {
            Some(o) => {
                o.signal = o.signal.max(n.signal);
                o.known |= n.known;
                o.connected |= n.connected;
                if o.profile.is_empty() {
                    o.profile = n.profile;
                }
            }
            None => out.push(n),
        }
    }
    out.truncate(25);
    out
}

#[cfg(windows)]
pub mod win {
    use windows::{
        core::{GUID, PCWSTR},
        Win32::{
            Foundation::HANDLE,
            NetworkManagement::WiFi::{
                dot11_BSS_type_any, wlan_connection_mode_profile, WlanCloseHandle, WlanConnect,
                WlanEnumInterfaces, WlanFreeMemory, WlanGetAvailableNetworkList, WlanOpenHandle, WlanScan,
                WLAN_AVAILABLE_NETWORK_LIST, WLAN_CONNECTION_PARAMETERS, WLAN_INTERFACE_INFO_LIST,
            },
        },
    };

    use super::WifiNetwork;
    use crate::error::{AppError, AppResult};

    const CONNECTED: u32 = 0x1;
    const HAS_PROFILE: u32 = 0x2;

    struct Client(HANDLE);

    impl Drop for Client {
        fn drop(&mut self) {
            // SAFETY: closing the handle opened in `open`.
            unsafe {
                WlanCloseHandle(self.0, None);
            }
        }
    }

    fn open() -> AppResult<(Client, GUID)> {
        let mut version = 0;
        let mut handle = HANDLE::default();
        // SAFETY: out-params are locals; the interface list is freed below.
        unsafe {
            if WlanOpenHandle(2, None, &mut version, &mut handle) != 0 {
                return Err(AppError::Platform("Wi-Fi isn't available".into()));
            }
            let client = Client(handle);
            let mut list: *mut WLAN_INTERFACE_INFO_LIST = std::ptr::null_mut();
            if WlanEnumInterfaces(handle, None, &mut list) != 0 || list.is_null() {
                return Err(AppError::Platform("No Wi-Fi adapter".into()));
            }
            let count = (*list).dwNumberOfItems as usize;
            let guid = (count > 0).then(|| (*list).InterfaceInfo[0].InterfaceGuid);
            WlanFreeMemory(list.cast());
            let guid = guid.ok_or_else(|| AppError::Platform("No Wi-Fi adapter".into()))?;
            Ok((client, guid))
        }
    }

    fn text(units: &[u16]) -> String {
        let end = units.iter().position(|&c| c == 0).unwrap_or(units.len());
        String::from_utf16_lossy(&units[..end])
    }

    /// Ask the adapter to look again (results come a few seconds later).
    pub fn scan() -> AppResult<()> {
        let (client, guid) = open()?;
        // SAFETY: a scan request on a live handle and interface.
        unsafe {
            WlanScan(client.0, &guid, None, None, None);
        }
        Ok(())
    }

    pub fn networks() -> AppResult<Vec<WifiNetwork>> {
        let (client, guid) = open()?;
        let mut list: *mut WLAN_AVAILABLE_NETWORK_LIST = std::ptr::null_mut();
        // SAFETY: the list Windows allocates is read for its item count, then freed.
        let raw = unsafe {
            if WlanGetAvailableNetworkList(client.0, &guid, 0, None, &mut list) != 0 || list.is_null() {
                return Err(AppError::Platform("Couldn't list Wi-Fi networks".into()));
            }
            let count = (*list).dwNumberOfItems as usize;
            let items = std::slice::from_raw_parts((*list).Network.as_ptr(), count);
            let out: Vec<WifiNetwork> = items
                .iter()
                .map(|n| {
                    let len = (n.dot11Ssid.uSSIDLength as usize).min(32);
                    WifiNetwork {
                        ssid: String::from_utf8_lossy(&n.dot11Ssid.ucSSID[..len]).into_owned(),
                        signal: n.wlanSignalQuality.min(100) as u8,
                        secure: n.bSecurityEnabled.as_bool(),
                        known: n.dwFlags & HAS_PROFILE != 0,
                        connected: n.dwFlags & CONNECTED != 0,
                        profile: text(&n.strProfileName),
                    }
                })
                .collect();
            WlanFreeMemory(list.cast());
            out
        };
        Ok(super::tidy(raw))
    }

    /// Join a network Windows has a profile for.
    pub fn connect(profile: &str) -> AppResult<()> {
        let (client, guid) = open()?;
        let name: Vec<u16> = profile.encode_utf16().chain(std::iter::once(0)).collect();
        let params = WLAN_CONNECTION_PARAMETERS {
            wlanConnectionMode: wlan_connection_mode_profile,
            strProfile: PCWSTR(name.as_ptr()),
            pDot11Ssid: std::ptr::null_mut(),
            pDesiredBssidList: std::ptr::null_mut(),
            dot11BssType: dot11_BSS_type_any,
            dwFlags: 0,
        };
        // SAFETY: the profile name outlives the call.
        let result = unsafe { WlanConnect(client.0, &guid, &params, None) };
        if result == 0 {
            Ok(())
        } else {
            Err(AppError::Platform("Couldn't connect to that network".into()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn net(ssid: &str, signal: u8, known: bool, connected: bool) -> WifiNetwork {
        WifiNetwork {
            ssid: ssid.into(),
            signal,
            secure: true,
            known,
            connected,
            profile: if known { ssid.into() } else { String::new() },
        }
    }

    #[test]
    fn one_per_name_connected_then_known_then_strongest() {
        let list = tidy(vec![
            net("Cafe", 90, false, false),
            net("Home", 40, true, false),
            net("Home", 70, false, false),
            net("", 99, false, false),
            net("Office", 30, true, true),
        ]);
        let names: Vec<_> = list.iter().map(|n| n.ssid.as_str()).collect();
        assert_eq!(names, ["Office", "Home", "Cafe"]);
        assert_eq!(list[1].signal, 70);
        assert!(list[1].known);
        assert_eq!(list[1].profile, "Home");
    }
}

//! Network status (Windows' own internet check), Wi-Fi name and signal, VPNs, and byte
//! counters for live speed. Windows tells us when the network changes; nothing is sent.

use windows::{
    Networking::Connectivity::{
        NetworkConnectivityLevel, NetworkInformation, NetworkStatusChangedEventHandler,
    },
    Win32::NetworkManagement::{
        IpHelper::{FreeMibTable, GetIfTable2, MIB_IF_TABLE2},
        Ndis::IfOperStatusUp,
    },
};

use super::{vpn_name, Iface, Internet, NetworkInfo};

/// IANA types of physical links whose bytes count toward speed.
const ETHERNET: u32 = 6;
const WIFI: u32 = 71;
const CELLULAR: u32 = 243;
/// `HardwareInterface` in `InterfaceAndOperStatusFlags`.
const HARDWARE: u8 = 0x1;
/// `FilterInterface`: a filter layered on another adapter (would double count).
const FILTER: u8 = 0x2;

/// Bytes received and sent so far on physical adapters, and the VPN that's up (if any).
pub fn counters() -> ((u64, u64), Option<String>) {
    let mut table: *mut MIB_IF_TABLE2 = std::ptr::null_mut();
    // SAFETY: Windows allocates the table; it's read here and freed with FreeMibTable.
    unsafe {
        if GetIfTable2(&mut table).is_err() || table.is_null() {
            return ((0, 0), None);
        }
        let rows = std::slice::from_raw_parts((*table).Table.as_ptr(), (*table).NumEntries as usize);
        let mut totals = (0u64, 0u64);
        let mut up = Vec::new();
        let mut names = Vec::new();
        for row in rows {
            if row.OperStatus != IfOperStatusUp {
                continue;
            }
            let flags = row.InterfaceAndOperStatusFlags._bitfield;
            if flags & HARDWARE != 0 && flags & FILTER == 0 && [ETHERNET, WIFI, CELLULAR].contains(&row.Type)
            {
                totals.0 += row.InOctets;
                totals.1 += row.OutOctets;
            }
            let text =
                |s: &[u16]| String::from_utf16_lossy(&s[..s.iter().position(|&c| c == 0).unwrap_or(s.len())]);
            names.push((text(&row.Alias), text(&row.Description), row.Type));
        }
        FreeMibTable(table.cast());
        for (alias, description, if_type) in &names {
            if alias.contains("-0000") {
                continue; // filter-driver duplicates ("…-WFP Native MAC Layer…-0000")
            }
            up.push(Iface {
                alias,
                description,
                if_type: *if_type,
            });
        }
        (totals, vpn_name(&up))
    }
}

/// What Windows says about the connection to the internet right now.
pub fn info() -> NetworkInfo {
    let (_, vpn) = counters();
    let Ok(profile) = NetworkInformation::GetInternetConnectionProfile() else {
        return NetworkInfo {
            vpn,
            ..Default::default()
        };
    };
    let internet = match profile.GetNetworkConnectivityLevel() {
        Ok(NetworkConnectivityLevel::InternetAccess) => Internet::Online,
        Ok(NetworkConnectivityLevel::ConstrainedInternetAccess | NetworkConnectivityLevel::LocalAccess) => {
            Internet::Limited
        }
        _ => Internet::Offline,
    };
    let wifi = profile.IsWlanConnectionProfile().unwrap_or(false);
    let cellular = profile.IsWwanConnectionProfile().unwrap_or(false);
    let kind = if wifi {
        "wifi"
    } else if cellular {
        "cellular"
    } else if profile
        .NetworkAdapter()
        .and_then(|a| a.IanaInterfaceType())
        .is_ok_and(|t| t == ETHERNET)
    {
        "ethernet"
    } else {
        "other"
    };
    let ssid = wifi
        .then(|| {
            profile
                .WlanConnectionProfileDetails()
                .and_then(|d| d.GetConnectedSsid())
                .ok()
        })
        .flatten()
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty());
    let bars = wifi
        .then(|| profile.GetSignalBars().and_then(|b| b.Value()).ok())
        .flatten();
    NetworkInfo {
        internet,
        kind: kind.into(),
        ssid,
        bars,
        vpn,
    }
}

/// Call `changed` whenever Windows reports a network change. Keep the token alive.
pub fn watch(changed: impl Fn() + Send + Sync + 'static) -> Option<i64> {
    NetworkInformation::NetworkStatusChanged(&NetworkStatusChangedEventHandler::new(move |_| {
        changed();
        Ok(())
    }))
    .ok()
}

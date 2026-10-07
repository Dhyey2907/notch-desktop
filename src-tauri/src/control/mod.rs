//! Control center: the PC's network, power, radios and accessories at a glance, plus the
//! alerts that go with them (internet down or back, unplug at 80%, a device running low,
//! Caps Lock and keyboard language). Everything is read locally; nothing leaves the PC.

#[cfg(windows)]
mod battery;
#[cfg(windows)]
mod gamepad;
#[cfg(windows)]
mod keys;
pub mod netapps;
#[cfg(windows)]
mod network;
#[cfg(windows)]
mod radios;
pub mod service;
pub mod speedtest;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Internet {
    /// Connected to the internet.
    Online,
    /// A network, but no way out (captive portal, router down, sign-in needed).
    Limited,
    Offline,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct NetworkInfo {
    pub internet: Internet,
    /// `wifi`, `ethernet`, `cellular` or `other`.
    pub kind: String,
    /// Wi-Fi network name (Windows only shares it when location is on for apps).
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssid: Option<String>,
    /// Wi-Fi signal, 0–5 bars.
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bars: Option<u8>,
    /// Name of a VPN that's connected.
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vpn: Option<String>,
}

impl Default for NetworkInfo {
    fn default() -> Self {
        Self {
            internet: Internet::Offline,
            kind: "other".into(),
            ssid: None,
            bars: None,
            vpn: None,
        }
    }
}

/// Bytes per second right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct NetSpeed {
    #[ts(type = "number")]
    pub down: u64,
    #[ts(type = "number")]
    pub up: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PowerInfo {
    pub percent: u8,
    pub plugged: bool,
    pub charging: bool,
    /// Minutes until full while charging, or until empty on battery.
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minutes: Option<u32>,
    /// Capacity left compared to new, in percent.
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub health: Option<u8>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cycles: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DeviceBattery {
    pub name: String,
    /// `bluetooth` or `controller`.
    pub kind: String,
    pub percent: u8,
}

/// On/off state of something that can be switched; `None` when the PC doesn't have it.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Radios {
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wifi: Option<bool>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bluetooth: Option<bool>,
    /// Mobile hotspot (sharing this PC's internet).
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hotspot: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ControlSnapshot {
    pub network: NetworkInfo,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub power: Option<PowerInfo>,
    pub radios: Radios,
    pub devices: Vec<DeviceBattery>,
}

/// Something worth a pop-up in the notch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase", tag = "kind")]
#[ts(export)]
pub enum SystemAlert {
    InternetDown,
    InternetBack {
        /// How long it was down.
        seconds: u32,
    },
    /// Charged to the "unplug at" level.
    ChargeLimit {
        percent: u8,
    },
    /// Charger connected.
    Plugged {
        percent: u8,
        #[ts(optional)]
        #[serde(skip_serializing_if = "Option::is_none")]
        minutes: Option<u32>,
    },
    DeviceLow {
        name: String,
        percent: u8,
    },
    CapsLock {
        on: bool,
    },
    NumLock {
        on: bool,
    },
    Language {
        name: String,
    },
}

/// Which radio to switch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum RadioKind {
    Wifi,
    Bluetooth,
    Hotspot,
}

// ── Pure rules ──────────────────────────────────────────────────────────────

/// Adapters that are part of Windows or virtual machines, not a VPN the user turned on.
const NOT_VPN: &[&str] = &[
    "loopback",
    "teredo",
    "6to4",
    "ip-https",
    "isatap",
    "hyper-v",
    "vethernet",
    "vmware",
    "virtualbox",
    "wsl",
    "bluetooth",
    "wi-fi direct",
    "kernel debug",
    "wan miniport",
];
const VPN_HINTS: &[&str] = &[
    "vpn",
    "tunnel",
    "wireguard",
    "wintun",
    "tap-windows",
    "openvpn",
    "nordlynx",
    "tailscale",
    "zerotier",
    "fortinet",
    "cisco anyconnect",
    "globalprotect",
    "proton",
    "mullvad",
    "warp",
];

/// An interface that is up, from the interface table.
pub struct Iface<'a> {
    pub alias: &'a str,
    pub description: &'a str,
    /// IANA interface type (23 = PPP, 53 = virtual, 131 = tunnel).
    pub if_type: u32,
}

/// The connected VPN's name, if any.
pub fn vpn_name(up: &[Iface]) -> Option<String> {
    up.iter()
        .find(|i| {
            let text = format!("{} {}", i.alias, i.description).to_lowercase();
            !NOT_VPN.iter().any(|n| text.contains(n))
                && (i.if_type == 23 || VPN_HINTS.iter().any(|h| text.contains(h)))
        })
        .map(|i| i.alias.to_owned())
}

/// Bytes per second between two counter readings (counters can reset when adapters change).
pub fn rate(before: u64, after: u64, millis: u64) -> u64 {
    if millis == 0 || after < before {
        return 0;
    }
    (after - before) * 1000 / millis
}

/// Minutes to full (charging, `rate_mw` > 0) or to empty (discharging, < 0).
pub fn minutes_left(capacity: u32, full: u32, rate_mw: i32) -> Option<u32> {
    let minutes = match rate_mw {
        r if r > 0 && full > capacity => (full - capacity) as f64 / r as f64 * 60.0,
        r if r < 0 && capacity > 0 => capacity as f64 / -r as f64 * 60.0,
        _ => return None,
    };
    // Rates right after plugging in are wild; don't show silly numbers.
    (minutes.is_finite() && minutes < 48.0 * 60.0).then_some(minutes.round() as u32)
}

pub fn health(full: u32, designed: u32) -> Option<u8> {
    (designed > 0 && full > 0).then(|| ((full as f64 / designed as f64) * 100.0).round().min(100.0) as u8)
}

/// "Unplug at 80%": tell once per charge, when the level is crossed while plugged in — not
/// when Notch starts on a laptop that's already sitting at 100%.
#[derive(Default)]
pub struct ChargeWatch {
    /// Seen below the level while plugged in, so reaching it is news.
    armed: bool,
}

impl ChargeWatch {
    pub fn check(&mut self, percent: u8, plugged: bool, limit: u8) -> bool {
        if !plugged || limit == 0 {
            self.armed = false;
            return false;
        }
        if percent < limit {
            self.armed = true;
            return false;
        }
        std::mem::take(&mut self.armed)
    }
}

/// Low accessory batteries: tell once at or below 15%, again only after a recharge.
#[derive(Default)]
pub struct LowWatch {
    low: std::collections::HashSet<String>,
}

pub const LOW_PERCENT: u8 = 15;

impl LowWatch {
    pub fn check(&mut self, devices: &[DeviceBattery]) -> Vec<DeviceBattery> {
        let mut alerts = Vec::new();
        for d in devices {
            if d.percent <= LOW_PERCENT {
                if self.low.insert(d.name.clone()) {
                    alerts.push(d.clone());
                }
            } else if d.percent >= LOW_PERCENT + 15 {
                self.low.remove(&d.name);
            }
        }
        alerts
    }
}

/// Xbox controllers report a coarse level, not a percent.
pub fn gamepad_percent(level: u8) -> u8 {
    match level {
        0 => 5,
        1 => 25,
        2 => 60,
        _ => 100,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn iface<'a>(alias: &'a str, description: &'a str, if_type: u32) -> Iface<'a> {
        Iface {
            alias,
            description,
            if_type,
        }
    }

    #[test]
    fn spots_vpns_but_not_windows_plumbing() {
        assert_eq!(
            vpn_name(&[
                iface("Wi-Fi", "MediaTek Wi-Fi 6E", 71),
                iface("Tailscale", "Tailscale Tunnel", 53)
            ]),
            Some("Tailscale".into())
        );
        assert_eq!(vpn_name(&[iface("Office", "WAN Miniport (PPTP)", 23)]), None);
        assert_eq!(
            vpn_name(&[iface("Work VPN", "Some adapter", 23)]),
            Some("Work VPN".into())
        );
        assert_eq!(
            vpn_name(&[
                iface("vEthernet (WSL)", "Hyper-V Virtual Ethernet Adapter", 6),
                iface("Loopback Pseudo-Interface 1", "Software Loopback Interface 1", 24)
            ]),
            None
        );
        assert_eq!(
            vpn_name(&[iface("Proton", "WireGuard Tunnel", 53)]),
            Some("Proton".into())
        );
    }

    #[test]
    fn computes_rates_and_times() {
        assert_eq!(rate(1_000, 3_000, 500), 4_000);
        assert_eq!(rate(5_000, 1_000, 1000), 0);
        assert_eq!(rate(0, 10, 0), 0);
        // 45 Wh to go at 45 W: an hour.
        assert_eq!(minutes_left(45_000, 90_000, 45_000), Some(60));
        // 30 Wh left at 15 W: two hours.
        assert_eq!(minutes_left(30_000, 90_000, -15_000), Some(120));
        assert_eq!(minutes_left(90_000, 90_000, 0), None);
        assert_eq!(minutes_left(10_000, 90_000, 1), None);
        assert_eq!(health(81_000, 90_000), Some(90));
        assert_eq!(health(91_000, 90_000), Some(100));
        assert_eq!(health(90_000, 0), None);
    }

    #[test]
    fn says_unplug_once_per_charge() {
        let mut watch = ChargeWatch::default();
        // Already full when Notch starts: no nagging.
        assert!(!watch.check(100, true, 80));
        assert!(!watch.check(70, true, 80));
        assert!(watch.check(80, true, 80));
        assert!(!watch.check(85, true, 80));
        // Unplugged and plugged back in above the level: nothing new.
        assert!(!watch.check(85, false, 80));
        assert!(!watch.check(84, true, 80));
        // Drained and charged again: tell again.
        assert!(!watch.check(60, false, 80));
        assert!(!watch.check(61, true, 80));
        assert!(watch.check(80, true, 80));
        assert!(!watch.check(50, true, 0));
    }

    #[test]
    fn warns_about_low_devices_once_until_recharged() {
        let mut watch = LowWatch::default();
        let buds = |percent| DeviceBattery {
            name: "Buds".into(),
            kind: "bluetooth".into(),
            percent,
        };
        assert!(watch.check(&[buds(40)]).is_empty());
        assert_eq!(watch.check(&[buds(15)]).len(), 1);
        assert!(watch.check(&[buds(10)]).is_empty());
        assert!(watch.check(&[buds(25)]).is_empty());
        assert!(watch.check(&[buds(12)]).is_empty());
        assert!(watch.check(&[buds(90)]).is_empty());
        assert_eq!(watch.check(&[buds(14)]).len(), 1);
        assert_eq!(gamepad_percent(1), 25);
    }
}

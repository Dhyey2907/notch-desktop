//! CPU, memory and battery for the Home dashboard. Read only while Home is open (the view
//! asks every couple of seconds); CPU use is the busy share of time since the last reading.

use std::sync::Mutex;

use windows::Win32::{
    Foundation::FILETIME,
    System::{
        Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS},
        SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX},
        Threading::GetSystemTimes,
    },
};

use super::SystemStats;

fn ticks(t: FILETIME) -> u64 {
    (u64::from(t.dwHighDateTime) << 32) | u64::from(t.dwLowDateTime)
}

/// Idle and total CPU time from the previous reading.
static LAST: Mutex<Option<(u64, u64)>> = Mutex::new(None);

/// Busy share between two (idle, total) readings, as a percent.
pub fn cpu_percent(previous: (u64, u64), now: (u64, u64)) -> u8 {
    let idle = now.0.saturating_sub(previous.0);
    let total = now.1.saturating_sub(previous.1);
    if total == 0 {
        return 0;
    }
    ((total.saturating_sub(idle)) * 100 / total).min(100) as u8
}

pub fn read() -> SystemStats {
    // SAFETY: plain queries filling local out-structs of the right size.
    unsafe {
        let (mut idle, mut kernel, mut user) =
            (FILETIME::default(), FILETIME::default(), FILETIME::default());
        let cpu = if GetSystemTimes(Some(&mut idle), Some(&mut kernel), Some(&mut user)).is_ok() {
            // Kernel time includes idle time.
            let now = (ticks(idle), ticks(kernel) + ticks(user));
            let mut last = LAST.lock().unwrap_or_else(|e| e.into_inner());
            let percent = last.map(|prev| cpu_percent(prev, now));
            *last = Some(now);
            percent
        } else {
            None
        };

        let mut memory = MEMORYSTATUSEX {
            dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
            ..Default::default()
        };
        let ram = GlobalMemoryStatusEx(&mut memory)
            .ok()
            .map(|()| memory.dwMemoryLoad.min(100) as u8);

        let mut power = SYSTEM_POWER_STATUS::default();
        let (battery, charging) = if GetSystemPowerStatus(&mut power).is_ok() {
            // 255 = unknown; flag 128 = no system battery (a desktop PC).
            let has_battery = power.BatteryLifePercent <= 100 && power.BatteryFlag & 128 == 0;
            (
                has_battery.then_some(power.BatteryLifePercent),
                power.ACLineStatus == 1,
            )
        } else {
            (None, false)
        };

        SystemStats {
            cpu,
            ram,
            battery,
            charging,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::cpu_percent;

    #[test]
    fn computes_cpu_use_between_readings() {
        assert_eq!(cpu_percent((100, 1000), (150, 1100)), 50);
        assert_eq!(cpu_percent((100, 1000), (200, 1100)), 0);
        assert_eq!(cpu_percent((100, 1000), (100, 1100)), 100);
        assert_eq!(cpu_percent((100, 1000), (100, 1000)), 0);
    }
}

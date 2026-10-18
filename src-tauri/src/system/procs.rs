//! Which apps use the most CPU and memory right now, for the system monitor. Processes are
//! grouped by exe (Chrome's dozens of processes count as one "Chrome"); CPU is each app's
//! share of all cores since the previous reading.

use std::{collections::HashMap, sync::Mutex, time::Instant};

use serde::Serialize;
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AppUse {
    /// "Chrome", "Code".
    pub name: String,
    pub exe: String,
    /// Share of the whole CPU (all cores), percent; none on the first reading.
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu: Option<f32>,
    /// Working set, MB.
    pub ram_mb: u32,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SystemMonitor {
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu: Option<u8>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ram: Option<u8>,
    /// NVIDIA GPUs only.
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gpu: Option<u8>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gpu_temp: Option<u8>,
    /// Top apps by CPU, busiest first.
    pub by_cpu: Vec<AppUse>,
    /// Top apps by memory, biggest first.
    pub by_ram: Vec<AppUse>,
}

/// One process sample: pid → (exe, CPU time in 100 ns ticks, working set bytes).
pub type Sample = HashMap<u32, (String, u64, u64)>;

static LAST: Mutex<Option<(Instant, HashMap<u32, u64>)>> = Mutex::new(None);

/// Group a sample by exe, with CPU shares from the previous sample's times.
pub fn group(now: &Sample, previous: Option<&HashMap<u32, u64>>, wall_ticks: u64, cores: u32) -> Vec<AppUse> {
    let mut apps: HashMap<String, (f64, u64, bool)> = HashMap::new();
    for (pid, (exe, cpu, ram)) in now {
        let key = crate::clipboard::exe_key(exe);
        let entry = apps.entry(key).or_insert((0.0, 0, false));
        entry.1 += ram;
        if let Some(prev) = previous.and_then(|p| p.get(pid)) {
            let budget = wall_ticks.saturating_mul(u64::from(cores.max(1)));
            if budget > 0 {
                entry.0 += cpu.saturating_sub(*prev) as f64 * 100.0 / budget as f64;
                entry.2 = true;
            }
        }
    }
    apps.into_iter()
        .map(|(exe, (cpu, ram, seen))| AppUse {
            name: crate::clipboard::app_label(&exe),
            cpu: seen.then_some((cpu.min(100.0) * 10.0).round() as f32 / 10.0),
            ram_mb: (ram / (1024 * 1024)) as u32,
            exe,
        })
        .collect()
}

/// The top `n` of each list.
pub fn top(mut apps: Vec<AppUse>, n: usize) -> (Vec<AppUse>, Vec<AppUse>) {
    apps.retain(|a| !IGNORED.contains(&a.exe.as_str()));
    let mut by_cpu = apps.clone();
    by_cpu.sort_by(|a, b| b.cpu.unwrap_or(0.0).total_cmp(&a.cpu.unwrap_or(0.0)));
    by_cpu.truncate(n);
    apps.sort_by_key(|a| std::cmp::Reverse(a.ram_mb));
    apps.truncate(n);
    (by_cpu, apps)
}

/// Not apps you can do anything about.
const IGNORED: &[&str] = &[
    "system idle process",
    "[system process]",
    "system",
    "idle",
    "registry",
];

#[cfg(windows)]
pub fn read(n: usize) -> SystemMonitor {
    let stats = super::stats::read();
    let (gpu_temp, gpu) = crate::game::nvml::gpu();
    let sample = win::sample();
    let now = Instant::now();
    let times: HashMap<u32, u64> = sample.iter().map(|(pid, (_, t, _))| (*pid, *t)).collect();
    let mut last = LAST.lock().unwrap_or_else(|e| e.into_inner());
    let previous = last.as_ref().map(|(at, t)| (now.duration_since(*at), t));
    let apps = match previous {
        Some((elapsed, prev)) => group(&sample, Some(prev), elapsed.as_nanos() as u64 / 100, win::cores()),
        None => group(&sample, None, 0, 1),
    };
    *last = Some((now, times));
    let (by_cpu, by_ram) = top(apps, n);
    SystemMonitor {
        cpu: stats.cpu,
        ram: stats.ram,
        gpu: gpu.map(|g| g.min(100) as u8),
        gpu_temp: gpu_temp.map(|t| t.min(255) as u8),
        by_cpu,
        by_ram,
    }
}

#[cfg(windows)]
mod win {
    use windows::Win32::{
        Foundation::{CloseHandle, FILETIME},
        System::{
            Diagnostics::ToolHelp::{
                CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
                TH32CS_SNAPPROCESS,
            },
            ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS},
            SystemInformation::{GetSystemInfo, SYSTEM_INFO},
            Threading::{GetProcessTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION},
        },
    };

    use super::Sample;

    pub fn cores() -> u32 {
        let mut info = SYSTEM_INFO::default();
        // SAFETY: fills a local struct.
        unsafe { GetSystemInfo(&mut info) };
        info.dwNumberOfProcessors.max(1)
    }

    fn ticks(t: FILETIME) -> u64 {
        (u64::from(t.dwHighDateTime) << 32) | u64::from(t.dwLowDateTime)
    }

    /// Every process we may query: exe, CPU time so far and working set.
    pub fn sample() -> Sample {
        let mut out = Sample::new();
        // SAFETY: Toolhelp walk over a snapshot we own and close; each process handle is opened
        // with query-only rights and closed again.
        unsafe {
            let Ok(snap) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else {
                return out;
            };
            let mut entry = PROCESSENTRY32W {
                dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
                ..Default::default()
            };
            let mut ok = Process32FirstW(snap, &mut entry).is_ok();
            while ok {
                let pid = entry.th32ProcessID;
                let len = entry
                    .szExeFile
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(entry.szExeFile.len());
                let exe = String::from_utf16_lossy(&entry.szExeFile[..len]);
                if pid != 0 {
                    if let Ok(h) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) {
                        let (mut c, mut e, mut k, mut u) = Default::default();
                        let cpu = if GetProcessTimes(h, &mut c, &mut e, &mut k, &mut u).is_ok() {
                            ticks(k) + ticks(u)
                        } else {
                            0
                        };
                        let mut mem = PROCESS_MEMORY_COUNTERS {
                            cb: std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
                            ..Default::default()
                        };
                        let ram = if GetProcessMemoryInfo(h, &mut mem, mem.cb).is_ok() {
                            mem.WorkingSetSize as u64
                        } else {
                            0
                        };
                        let _ = CloseHandle(h);
                        out.insert(pid, (exe, cpu, ram));
                    }
                }
                ok = Process32NextW(snap, &mut entry).is_ok();
            }
            let _ = CloseHandle(snap);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(rows: &[(u32, &str, u64, u64)]) -> Sample {
        rows.iter()
            .map(|(pid, exe, cpu, ram)| (*pid, (exe.to_string(), *cpu, *ram)))
            .collect()
    }

    #[test]
    fn groups_processes_by_app_and_shares_cpu_across_cores() {
        let prev: HashMap<u32, u64> = [(1, 0), (2, 0), (3, 0)].into();
        // 1 s of wall time = 10_000_000 ticks; 4 cores = 40_000_000 to share.
        let now = sample(&[
            (1, "chrome.exe", 4_000_000, 100 << 20),
            (2, "chrome.exe", 4_000_000, 50 << 20),
            (3, "Code.exe", 20_000_000, 300 << 20),
        ]);
        let apps = group(&now, Some(&prev), 10_000_000, 4);
        let chrome = apps.iter().find(|a| a.exe == "chrome.exe").unwrap();
        assert_eq!(chrome.cpu, Some(20.0));
        assert_eq!(chrome.ram_mb, 150);
        let (by_cpu, by_ram) = top(apps, 1);
        assert_eq!(by_cpu[0].exe, "code.exe");
        assert_eq!(by_ram[0].exe, "code.exe");
    }

    #[test]
    fn first_reading_has_no_cpu_and_skips_idle() {
        let now = sample(&[(0, "System Idle Process", 9, 0), (7, "notepad.exe", 5, 1 << 20)]);
        let (by_cpu, _) = top(group(&now, None, 0, 1), 5);
        assert_eq!(by_cpu.len(), 1);
        assert_eq!(by_cpu[0].cpu, None);
    }
}

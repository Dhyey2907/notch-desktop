//! Power schedule: put the PC to sleep at a set time, but only if you've been away for a while,
//! and wake it at a set time (a Windows scheduled task that's allowed to wake the PC). Both on
//! the days you pick. The wake task lives in Task Scheduler as "Overline\Wake" and is removed
//! when you turn waking off.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub const WAKE_TASK: &str = "Overline\\Wake";

/// Settings → Power schedule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct PowerSchedule {
    /// Sleep at this time (minutes after midnight); none = never.
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sleep_at: Option<u32>,
    /// …if nothing was touched for this many minutes.
    pub sleep_if_idle_min: u32,
    /// Wake at this time; none = never.
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wake_at: Option<u32>,
    /// 1 = Monday … 7 = Sunday.
    pub days: Vec<u8>,
}

impl Default for PowerSchedule {
    fn default() -> Self {
        Self {
            sleep_at: None,
            sleep_if_idle_min: 15,
            wake_at: None,
            days: vec![1, 2, 3, 4, 5, 6, 7],
        }
    }
}

impl PowerSchedule {
    pub fn valid(&self) -> bool {
        self.sleep_at.is_none_or(|m| m < 24 * 60)
            && self.wake_at.is_none_or(|m| m < 24 * 60)
            && (1..=240).contains(&self.sleep_if_idle_min)
            && self.days.len() <= 7
            && self.days.iter().all(|d| (1..=7).contains(d))
    }
}

/// Should the PC go to sleep now? Within an hour after the time (a PC that was busy then still
/// sleeps once you leave), on a chosen day, after enough idle time, and not twice a day.
pub fn sleep_due(s: &PowerSchedule, weekday: u8, minute: u32, idle_min: u32, slept_today: bool) -> bool {
    let Some(at) = s.sleep_at else { return false };
    !slept_today
        && (s.days.is_empty() || s.days.contains(&weekday))
        && minute >= at
        && minute < at + 60
        && idle_min >= s.sleep_if_idle_min
}

/// Task Scheduler's XML for the wake task (it just wakes the PC; the action does nothing).
pub fn wake_task_xml(at_min: u32, days: &[u8]) -> String {
    const NAMES: [&str; 7] = [
        "Monday",
        "Tuesday",
        "Wednesday",
        "Thursday",
        "Friday",
        "Saturday",
        "Sunday",
    ];
    let picked: Vec<&str> = if days.is_empty() {
        NAMES.to_vec()
    } else {
        days.iter()
            .filter_map(|d| NAMES.get(usize::from(*d).wrapping_sub(1)).copied())
            .collect()
    };
    let day_tags: String = picked.iter().map(|d| format!("<{d} />")).collect();
    let start = format!("2026-01-05T{:02}:{:02}:00", at_min / 60, at_min % 60);
    format!(
        r#"<?xml version="1.0" encoding="UTF-16"?>
<Task version="1.2" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <RegistrationInfo><Description>Wakes the PC for Overline's power schedule.</Description></RegistrationInfo>
  <Triggers>
    <CalendarTrigger>
      <StartBoundary>{start}</StartBoundary>
      <Enabled>true</Enabled>
      <ScheduleByWeek><DaysOfWeek>{day_tags}</DaysOfWeek><WeeksInterval>1</WeeksInterval></ScheduleByWeek>
    </CalendarTrigger>
  </Triggers>
  <Principals><Principal id="Author"><LogonType>InteractiveToken</LogonType><RunLevel>LeastPrivilege</RunLevel></Principal></Principals>
  <Settings>
    <WakeToRun>true</WakeToRun>
    <DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>
    <StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>
    <ExecutionTimeLimit>PT1M</ExecutionTimeLimit>
    <Enabled>true</Enabled>
  </Settings>
  <Actions Context="Author">
    <Exec><Command>cmd.exe</Command><Arguments>/c exit</Arguments></Exec>
  </Actions>
</Task>"#
    )
}

/// Create (or replace) the wake task, or remove it when waking is off.
#[cfg(windows)]
pub fn apply_wake(s: &PowerSchedule, temp: &std::path::Path) -> crate::error::AppResult<()> {
    use std::os::windows::process::CommandExt;
    const NO_WINDOW: u32 = 0x0800_0000;
    let run = |args: &[&std::ffi::OsStr]| {
        std::process::Command::new("schtasks.exe")
            .args(args)
            .creation_flags(NO_WINDOW)
            .output()
    };
    match s.wake_at {
        Some(at) => {
            // schtasks reads the XML as UTF-16.
            let xml = wake_task_xml(at, &s.days);
            let mut bytes = vec![0xFF, 0xFE];
            for u in xml.encode_utf16() {
                bytes.extend_from_slice(&u.to_le_bytes());
            }
            let file = temp.join("overline-wake-task.xml");
            std::fs::write(&file, bytes)?;
            let out = run(&[
                "/Create".as_ref(),
                "/TN".as_ref(),
                WAKE_TASK.as_ref(),
                "/XML".as_ref(),
                file.as_os_str(),
                "/F".as_ref(),
            ]);
            let _ = std::fs::remove_file(&file);
            match out {
                Ok(o) if o.status.success() => Ok(()),
                _ => Err(crate::error::AppError::Platform(
                    "Windows didn't accept the wake schedule".into(),
                )),
            }
        }
        None => {
            let _ = run(&[
                "/Delete".as_ref(),
                "/TN".as_ref(),
                WAKE_TASK.as_ref(),
                "/F".as_ref(),
            ]);
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sched() -> PowerSchedule {
        PowerSchedule {
            sleep_at: Some(23 * 60),
            sleep_if_idle_min: 15,
            wake_at: Some(7 * 60 + 30),
            days: vec![1, 2, 3, 4, 5],
        }
    }

    #[test]
    fn sleeps_only_when_away_on_chosen_days() {
        let s = sched();
        assert!(sleep_due(&s, 1, 23 * 60 + 5, 20, false));
        assert!(!sleep_due(&s, 1, 23 * 60 + 5, 5, false), "still here");
        assert!(!sleep_due(&s, 6, 23 * 60 + 5, 20, false), "Saturday not picked");
        assert!(!sleep_due(&s, 1, 22 * 60, 20, false), "too early");
        assert!(!sleep_due(&s, 1, 23 * 60 + 5, 20, true), "already slept");
        assert!(!sleep_due(&PowerSchedule::default(), 1, 600, 600, false));
    }

    #[test]
    fn wake_task_has_the_time_and_days() {
        let xml = wake_task_xml(7 * 60 + 30, &[1, 5]);
        assert!(xml.contains("T07:30:00"));
        assert!(xml.contains("<Monday /><Friday />"));
        assert!(xml.contains("<WakeToRun>true</WakeToRun>"));
        assert!(wake_task_xml(60, &[]).contains("<Sunday />"));
    }

    #[test]
    fn validates() {
        assert!(sched().valid());
        let mut bad = sched();
        bad.days = vec![8];
        assert!(!bad.valid());
    }
}

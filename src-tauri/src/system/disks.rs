//! Disk health for the system monitor: each physical drive's health as Windows' storage stack
//! reports it (Healthy / Warning / Unhealthy, from the drive's SMART data) and how full each
//! fixed volume is. Read through `Get-PhysicalDisk` / `Get-Volume`, which need no admin.

use std::{
    sync::Mutex,
    time::{Duration, Instant},
};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::AppResult;

const FRESH_FOR: Duration = Duration::from_secs(10 * 60);
/// A volume this full gets a warning.
pub const NEARLY_FULL_PERCENT: u8 = 90;

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DiskHealth {
    pub drives: Vec<Drive>,
    pub volumes: Vec<Volume>,
    /// One line for a warning card; none when all is well.
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub problem: Option<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Drive {
    pub name: String,
    /// "SSD", "HDD" or "".
    pub kind: String,
    /// "Healthy", "Warning", "Unhealthy" or "Unknown".
    pub health: String,
    pub ok: bool,
    pub size_gb: u32,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Volume {
    /// "C".
    pub letter: String,
    pub label: String,
    pub size_gb: u32,
    pub free_gb: u32,
    pub used_percent: u8,
}

impl DiskHealth {
    fn find_problem(&self) -> Option<String> {
        if let Some(d) = self.drives.iter().find(|d| !d.ok && d.health != "Unknown") {
            return Some(format!(
                "Drive “{}” reports {}. Back up what matters on it.",
                d.name,
                d.health.to_lowercase()
            ));
        }
        self.volumes
            .iter()
            .find(|v| v.used_percent >= NEARLY_FULL_PERCENT)
            .map(|v| {
                format!(
                    "Drive {}: is {}% full ({} GB free).",
                    v.letter, v.used_percent, v.free_gb
                )
            })
    }
}

#[derive(Deserialize)]
struct Raw {
    #[serde(default)]
    disks: Vec<RawDisk>,
    #[serde(default)]
    vols: Vec<RawVol>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct RawDisk {
    friendly_name: Option<String>,
    health_status: Option<serde_json::Value>,
    media_type: Option<serde_json::Value>,
    size: Option<u64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct RawVol {
    drive_letter: Option<String>,
    file_system_label: Option<String>,
    size: Option<u64>,
    size_remaining: Option<u64>,
}

const GB: u64 = 1_000_000_000;

/// Health comes back as a word, or as the raw number on some systems.
fn health_word(v: Option<&serde_json::Value>) -> String {
    match v {
        Some(serde_json::Value::String(s)) => s.clone(),
        Some(serde_json::Value::Number(n)) => match n.as_u64() {
            Some(0) => "Healthy",
            Some(1) => "Warning",
            Some(2) => "Unhealthy",
            _ => "Unknown",
        }
        .to_string(),
        _ => "Unknown".to_string(),
    }
}

fn media_word(v: Option<&serde_json::Value>) -> String {
    match v {
        Some(serde_json::Value::String(s)) if s != "Unspecified" => s.clone(),
        Some(serde_json::Value::Number(n)) => match n.as_u64() {
            Some(3) => "HDD",
            Some(4) => "SSD",
            _ => "",
        }
        .to_string(),
        _ => String::new(),
    }
}

pub fn parse(json: &str) -> Option<DiskHealth> {
    let raw: Raw = serde_json::from_str(json.trim()).ok()?;
    let drives = raw
        .disks
        .into_iter()
        .map(|d| {
            let health = health_word(d.health_status.as_ref());
            Drive {
                name: d.friendly_name.unwrap_or_default().trim().to_string(),
                kind: media_word(d.media_type.as_ref()),
                ok: health == "Healthy",
                health,
                size_gb: (d.size.unwrap_or(0) / GB) as u32,
            }
        })
        .collect();
    let mut volumes: Vec<Volume> = raw
        .vols
        .into_iter()
        .filter_map(|v| {
            let size = v.size.filter(|s| *s > 0)?;
            let free = v.size_remaining.unwrap_or(0).min(size);
            Some(Volume {
                letter: v.drive_letter?.trim().to_string(),
                label: v.file_system_label.unwrap_or_default(),
                size_gb: (size / GB) as u32,
                free_gb: (free / GB) as u32,
                used_percent: ((size - free) * 100 / size) as u8,
            })
        })
        .collect();
    volumes.sort_by(|a, b| a.letter.cmp(&b.letter));
    let mut health = DiskHealth {
        drives,
        volumes,
        problem: None,
    };
    health.problem = health.find_problem();
    Some(health)
}

static CACHE: Mutex<Option<(Instant, DiskHealth)>> = Mutex::new(None);

/// Drive health and volume space (cached for ten minutes; `fresh` skips the cache).
pub fn read(fresh: bool) -> AppResult<DiskHealth> {
    if !fresh {
        if let Some((at, health)) = CACHE.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
            if at.elapsed() < FRESH_FOR {
                return Ok(health.clone());
            }
        }
    }
    let out = super::apps::powershell(
        "@{disks=@(Get-PhysicalDisk | Select-Object FriendlyName,HealthStatus,MediaType,Size); \
         vols=@(Get-Volume | Where-Object {$_.DriveLetter -and $_.DriveType -eq 'Fixed'} | \
         Select-Object DriveLetter,FileSystemLabel,Size,SizeRemaining)} | ConvertTo-Json -Depth 3 -Compress",
    )?;
    let health =
        parse(&out).ok_or_else(|| crate::error::AppError::Platform("could not read the drives".into()))?;
    *CACHE.lock().unwrap_or_else(|e| e.into_inner()) = Some((Instant::now(), health.clone()));
    Ok(health)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_words_or_numbers() {
        let json = r#"{"vols":[{"DriveLetter":"D","FileSystemLabel":"Data","Size":100000000000,"SizeRemaining":5000000000},
            {"DriveLetter":"C","FileSystemLabel":"OS","Size":500000000000,"SizeRemaining":200000000000}],
            "disks":[{"FriendlyName":"Samsung SSD","HealthStatus":"Healthy","MediaType":"SSD","Size":1000000000000},
            {"FriendlyName":"Old HDD","HealthStatus":1,"MediaType":3,"Size":2000000000000}]}"#;
        let h = parse(json).unwrap();
        assert_eq!(h.volumes[0].letter, "C");
        assert_eq!(h.volumes[0].used_percent, 60);
        assert_eq!(h.drives[1].health, "Warning");
        assert_eq!(h.drives[1].kind, "HDD");
        assert!(h.drives[0].ok && !h.drives[1].ok);
        assert!(h.problem.unwrap().contains("Old HDD"));
    }

    #[test]
    fn nearly_full_volume_is_a_problem_when_drives_are_fine() {
        let json = r#"{"disks":[{"FriendlyName":"SSD","HealthStatus":"Healthy","MediaType":"SSD","Size":1}],
            "vols":[{"DriveLetter":"C","FileSystemLabel":"OS","Size":100000000000,"SizeRemaining":5000000000}]}"#;
        assert_eq!(
            parse(json).unwrap().problem.unwrap(),
            "Drive C: is 95% full (5 GB free)."
        );
        let fine = r#"{"disks":[],"vols":[{"DriveLetter":"C","Size":100,"SizeRemaining":50}]}"#;
        assert!(parse(fine).unwrap().problem.is_none());
    }
}

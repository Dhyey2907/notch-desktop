//! Installed apps for the palette's launcher: everything in the Start menu, desktop and Store
//! apps alike (Windows' own `Get-StartApps`), cached for an hour. Launching goes through
//! `shell:AppsFolder\<AppID>`, which opens either kind the way the Start menu does.

use std::{
    sync::Mutex,
    time::{Duration, Instant},
};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::{AppError, AppResult};

const FRESH_FOR: Duration = Duration::from_secs(60 * 60);

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct InstalledApp {
    pub name: String,
    /// What `shell:AppsFolder` opens: an AUMID for Store apps, a path or GUID path otherwise.
    pub id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct StartApp {
    name: Option<String>,
    #[serde(rename = "AppID")]
    app_id: Option<String>,
}

static CACHE: Mutex<Option<(Instant, Vec<InstalledApp>)>> = Mutex::new(None);

/// The Start menu's apps, A–Z (cached; reading takes about a second).
pub fn list() -> AppResult<Vec<InstalledApp>> {
    if let Some((at, apps)) = CACHE.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
        if at.elapsed() < FRESH_FOR {
            return Ok(apps.clone());
        }
    }
    let apps = read()?;
    *CACHE.lock().unwrap_or_else(|e| e.into_inner()) = Some((Instant::now(), apps.clone()));
    Ok(apps)
}

fn read() -> AppResult<Vec<InstalledApp>> {
    let out = powershell("Get-StartApps | Select-Object Name, AppID | ConvertTo-Json -Compress")?;
    Ok(parse(&out))
}

/// `Get-StartApps` JSON → apps: no blanks, no uninstallers or help links, no duplicates, A–Z.
fn parse(json: &str) -> Vec<InstalledApp> {
    // One app comes back as an object, several as an array.
    let raw: Vec<StartApp> = serde_json::from_str::<Vec<StartApp>>(json)
        .or_else(|_| serde_json::from_str::<StartApp>(json).map(|a| vec![a]))
        .unwrap_or_default();
    let mut apps: Vec<InstalledApp> = raw
        .into_iter()
        .filter_map(|a| {
            let name = a.name?.trim().to_string();
            let id = a.app_id?.trim().to_string();
            (!name.is_empty() && !id.is_empty() && !is_noise(&name, &id)).then_some(InstalledApp { name, id })
        })
        .collect();
    apps.sort_by_key(|a| a.name.to_lowercase());
    apps.dedup_by(|a, b| a.name.eq_ignore_ascii_case(&b.name));
    apps
}

fn is_noise(name: &str, id: &str) -> bool {
    let n = name.to_lowercase();
    n.starts_with("uninstall")
        || n.contains("readme")
        || n.ends_with(" help")
        || n.ends_with(" website")
        || id.starts_with("http://")
        || id.starts_with("https://")
}

/// Open an app from [`list`].
pub fn launch(id: &str) -> AppResult<()> {
    // The id comes from our own list; still refuse anything that could become extra arguments.
    if id.is_empty() || id.contains('"') || id.contains('\n') {
        return Err(AppError::InvalidInput("bad app id".into()));
    }
    std::process::Command::new("explorer.exe")
        .arg(format!("shell:AppsFolder\\{id}"))
        .spawn()
        .map(|_| ())
        .map_err(|e| AppError::Platform(format!("could not open the app: {e}")))
}

pub(crate) fn powershell(script: &str) -> AppResult<String> {
    let mut cmd = std::process::Command::new("powershell.exe");
    cmd.args(["-NoProfile", "-NonInteractive", "-Command", script]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    let out = cmd
        .output()
        .map_err(|e| AppError::Platform(format!("could not list apps: {e}")))?;
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_apps_and_drops_noise() {
        let json = r#"[{"Name":"Notepad","AppID":"Microsoft.WindowsNotepad_8wekyb3d8bbwe!App"},
            {"Name":"Uninstall Foo","AppID":"{GUID}\\Foo\\uninstall.exe"},
            {"Name":"Foo Website","AppID":"https://foo.example"},
            {"Name":"  ","AppID":"x"},
            {"Name":"notepad","AppID":"dup"},
            {"Name":"Chrome","AppID":"Chrome"}]"#;
        let apps = parse(json);
        let names: Vec<_> = apps.iter().map(|a| a.name.as_str()).collect();
        assert_eq!(names, ["Chrome", "Notepad"]);
    }

    #[test]
    fn parses_a_single_app() {
        let apps = parse(r#"{"Name":"Paint","AppID":"Microsoft.Paint!App"}"#);
        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].id, "Microsoft.Paint!App");
    }

    #[test]
    fn rejects_odd_ids() {
        assert!(launch("a\"b").is_err());
        assert!(launch("").is_err());
    }
}

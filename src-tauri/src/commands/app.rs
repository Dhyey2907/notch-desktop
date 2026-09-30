use serde::Serialize;
use tauri::State;
use ts_rs::TS;

use crate::{
    db,
    error::AppResult,
    events::{AppEvent, EventEnvelope},
    state::AppState,
};

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AppInfo {
    pub name: String,
    pub version: String,
    pub data_dir: String,
    pub log_dir: String,
    pub db_schema_version: u32,
}

#[tauri::command]
pub fn app_info(app: tauri::AppHandle, state: State<'_, AppState>) -> AppResult<AppInfo> {
    let pkg = app.package_info();
    Ok(AppInfo {
        name: pkg.name.clone(),
        version: pkg.version.to_string(),
        data_dir: state.paths.data_dir.display().to_string(),
        log_dir: state.paths.log_dir.display().to_string(),
        db_schema_version: state.db.with(db::schema_version)?,
    })
}

/// Open a web link (a pull request, a copied link) in the default browser. Other schemes are
/// refused so a crafted link can't launch local programs.
#[tauri::command]
pub fn open_external(url: String, app: tauri::AppHandle) -> AppResult<()> {
    use tauri_plugin_opener::OpenerExt;
    let parsed =
        url::Url::parse(&url).map_err(|_| crate::error::AppError::InvalidInput("invalid link".into()))?;
    if !matches!(parsed.scheme(), "https" | "http") {
        return Err(crate::error::AppError::InvalidInput(
            "only web links can be opened".into(),
        ));
    }
    app.opener()
        .open_url(parsed.as_str(), None::<&str>)
        .map_err(|e| crate::error::AppError::Platform(e.to_string()))
}

/// Asks the backend to emit a `DiagnosticsPong` event correlated to `nonce`,
/// verifying the frontend → command → event → frontend round trip.
#[tauri::command]
pub fn diagnostics_ping(nonce: String, state: State<'_, AppState>) {
    let envelope = EventEnvelope::new("diagnostics", AppEvent::DiagnosticsPong { nonce: nonce.clone() })
        .with_correlation(nonce);
    state.bus.publish(envelope);
}

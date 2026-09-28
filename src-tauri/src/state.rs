use std::{path::PathBuf, sync::Mutex};

use crate::{db::Db, events::EventBus, settings::Settings};

/// Resolved per-user directories.
pub struct AppPaths {
    pub data_dir: PathBuf,
    pub log_dir: PathBuf,
}

/// Shared backend state, registered with `app.manage()` and read by commands.
pub struct AppState {
    pub db: Db,
    pub bus: EventBus,
    pub paths: AppPaths,
    /// In-memory copy of the persisted settings; writes go through `commands::settings`.
    pub settings: Mutex<Settings>,
}

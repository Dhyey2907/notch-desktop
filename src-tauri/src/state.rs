use std::path::PathBuf;

use crate::{db::Db, events::EventBus};

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
}

use tauri::{State, WebviewWindow};

use crate::{
    error::{AppError, AppResult},
    shell::{self, ShellState},
};

/// Largest logical dimension the island may request; guards against runaway sizes.
const MAX_LOGICAL_DIMENSION: f64 = 2000.0;

fn validate_dimension(name: &str, value: f64) -> AppResult<()> {
    if value.is_finite() && value > 0.0 && value <= MAX_LOGICAL_DIMENSION {
        Ok(())
    } else {
        Err(AppError::InvalidInput(format!("{name} out of range: {value}")))
    }
}

/// Resize the island window (logical pixels) and keep it pinned top-center.
#[tauri::command]
pub fn shell_set_window_size(
    window: WebviewWindow,
    shell: State<'_, ShellState>,
    width: f64,
    height: f64,
) -> AppResult<()> {
    validate_dimension("width", width)?;
    validate_dimension("height", height)?;
    shell.set_window_size((width, height));
    shell::place(&window, (width, height))
}

/// Whether a fullscreen app is in front (the island rests as a bubble and stays quiet).
#[tauri::command]
pub fn shell_is_fullscreen() -> bool {
    crate::shell::is_fullscreen()
}

use tauri::State;

use crate::{
    error::AppResult,
    game::service::{GameService, GameStatus},
};

#[tauri::command]
pub fn game_status(game: State<'_, GameService>) -> GameStatus {
    game.status()
}

/// Remember whether a fullscreen app is a game.
#[tauri::command]
pub async fn game_mark(exe: String, is_game: bool, game: State<'_, GameService>) -> AppResult<()> {
    let game = game.inner().clone();
    tauri::async_runtime::spawn_blocking(move || game.mark(&exe, is_game))
        .await
        .map_err(|e| crate::error::AppError::Platform(e.to_string()))?
}

/// Download and verify PresentMon so the HUD can show FPS.
#[tauri::command]
pub async fn game_setup_fps(game: State<'_, GameService>) -> AppResult<()> {
    game.setup_fps().await
}

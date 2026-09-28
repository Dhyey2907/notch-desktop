//! Thin `#[tauri::command]` entry points. Business logic lives in services;
//! commands only validate input, call into state, and map results.

pub mod app;
#[cfg(debug_assertions)]
pub mod dev;
pub mod diagnostics;
pub mod settings;
pub mod shell;

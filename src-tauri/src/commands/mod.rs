//! Thin `#[tauri::command]` entry points. Business logic lives in services;
//! commands only validate input, call into state, and map results.

pub mod app;
pub mod calendar;
pub mod claude;
#[cfg(debug_assertions)]
pub mod dev;
pub mod diagnostics;
pub mod git;
pub mod media;
pub mod settings;
pub mod shell;
pub mod spotify;

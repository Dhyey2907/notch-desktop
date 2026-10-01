//! Thin `#[tauri::command]` entry points. Business logic lives in services;
//! commands only validate input, call into state, and map results.

pub mod app;
pub mod ask;
pub mod calendar;
pub mod claude;
pub mod clipboard;
#[cfg(debug_assertions)]
pub mod dev;
pub mod diagnostics;
pub mod focus;
pub mod git;
pub mod media;
pub mod music;
pub mod phone;
pub mod settings;
pub mod shelf;
pub mod shell;
pub mod signin;
pub mod spotify;
pub mod system;
pub mod todo;

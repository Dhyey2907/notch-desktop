//! Thin `#[tauri::command]` entry points. Business logic lives in services;
//! commands only validate input, call into state, and map results.

pub mod app;
pub mod ask;
pub mod bridge;
pub mod calendar;
pub mod claude;
pub mod clipboard;
pub mod control;
pub mod day;
pub mod desk;
#[cfg(debug_assertions)]
pub mod dev;
pub mod developer;
pub mod diagnostics;
pub mod files;
pub mod focus;
pub mod fun;
pub mod game;
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
pub mod vault;

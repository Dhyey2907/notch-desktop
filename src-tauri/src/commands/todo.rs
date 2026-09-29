use tauri::State;

use crate::{
    error::AppResult,
    todo::{service::TodoService, Note, Task, TodoSnapshot},
};

#[tauri::command]
pub fn todo_snapshot(todo: State<'_, TodoService>) -> AppResult<TodoSnapshot> {
    todo.snapshot()
}

/// Add a task, optionally starting a timer right away (`ring`: sound when it ends).
#[tauri::command]
pub fn todo_add_task(
    title: String,
    timer_minutes: Option<u32>,
    ring: bool,
    todo: State<'_, TodoService>,
) -> AppResult<Task> {
    todo.add_task(&title, timer_minutes, ring)
}

#[tauri::command]
pub fn todo_set_done(id: String, done: bool, todo: State<'_, TodoService>) -> AppResult<()> {
    todo.set_done(&id, done)
}

#[tauri::command]
pub fn todo_rename(id: String, title: String, todo: State<'_, TodoService>) -> AppResult<()> {
    todo.rename(&id, &title)
}

#[tauri::command]
pub fn todo_delete(id: String, todo: State<'_, TodoService>) -> AppResult<()> {
    todo.delete_task(&id)
}

#[tauri::command]
pub fn todo_clear_done(todo: State<'_, TodoService>) -> AppResult<()> {
    todo.clear_done()
}

/// Start or restart a task's timer (also used for "+5 min").
#[tauri::command]
pub fn todo_start_timer(id: String, minutes: u32, ring: bool, todo: State<'_, TodoService>) -> AppResult<()> {
    todo.start_timer(&id, minutes, ring)
}

#[tauri::command]
pub fn todo_cancel_timer(id: String, todo: State<'_, TodoService>) -> AppResult<()> {
    todo.cancel_timer(&id)
}

/// Dismiss a finished timer (stops the ring on every screen).
#[tauri::command]
pub fn todo_acknowledge(id: String, todo: State<'_, TodoService>) {
    todo.acknowledge(&id)
}

#[tauri::command]
pub fn note_add(body: String, todo: State<'_, TodoService>) -> AppResult<Note> {
    todo.add_note(&body)
}

#[tauri::command]
pub fn note_update(id: String, body: String, todo: State<'_, TodoService>) -> AppResult<()> {
    todo.update_note(&id, &body)
}

#[tauri::command]
pub fn note_delete(id: String, todo: State<'_, TodoService>) -> AppResult<()> {
    todo.delete_note(&id)
}

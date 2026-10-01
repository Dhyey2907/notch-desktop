use tauri::{State, WebviewWindow};

use crate::{
    error::{AppError, AppResult},
    expander::{service::ExpanderService, Snippet},
    grab::service::GrabService,
    vault::{service::VaultService, TotpCode, VaultField, VaultInput, VaultState},
};

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> AppResult<T> + Send + 'static) -> AppResult<T> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| AppError::Platform(e.to_string()))?
}

#[tauri::command]
pub async fn vault_state(vault: State<'_, VaultService>) -> AppResult<VaultState> {
    let vault = vault.inner().clone();
    blocking(move || Ok(vault.state())).await
}

/// Pick the Obsidian vault folder.
#[tauri::command]
pub async fn vault_choose_folder(
    app: tauri::AppHandle,
    vault: State<'_, VaultService>,
) -> AppResult<Option<VaultState>> {
    use tauri_plugin_dialog::DialogExt;
    let picked = tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_title("Choose your Obsidian vault folder")
            .blocking_pick_folder()
    })
    .await
    .map_err(|e| AppError::Platform(e.to_string()))?;
    let Some(folder) = picked else { return Ok(None) };
    let path = folder
        .into_path()
        .map_err(|e| AppError::InvalidInput(e.to_string()))?;
    let vault = vault.inner().clone();
    blocking(move || vault.set_folder(path).map(Some)).await
}

#[tauri::command]
pub async fn vault_save(entry: VaultInput, vault: State<'_, VaultService>) -> AppResult<String> {
    let vault = vault.inner().clone();
    blocking(move || vault.save(entry)).await
}

/// Move a login's note to the Recycle Bin.
#[tauri::command]
pub async fn vault_delete(id: String, vault: State<'_, VaultService>) -> AppResult<()> {
    let vault = vault.inner().clone();
    blocking(move || vault.delete(&id)).await
}

#[tauri::command]
pub async fn vault_copy(
    id: String,
    field: VaultField,
    window: WebviewWindow,
    vault: State<'_, VaultService>,
) -> AppResult<()> {
    let vault = vault.inner().clone();
    blocking(move || vault.copy(&window, &id, field)).await
}

#[tauri::command]
pub async fn vault_reveal(
    id: String,
    window: WebviewWindow,
    vault: State<'_, VaultService>,
) -> AppResult<String> {
    let vault = vault.inner().clone();
    blocking(move || vault.reveal(&window, &id)).await
}

#[tauri::command]
pub fn vault_generate_password(length: usize) -> String {
    crate::vault::generate_password(length)
}

#[tauri::command]
pub async fn vault_export_key(window: WebviewWindow, vault: State<'_, VaultService>) -> AppResult<String> {
    let vault = vault.inner().clone();
    blocking(move || vault.export_key(&window)).await
}

#[tauri::command]
pub async fn vault_import_key(key: String, vault: State<'_, VaultService>) -> AppResult<()> {
    let vault = vault.inner().clone();
    blocking(move || vault.import_key(&key)).await
}

#[tauri::command]
pub async fn totp_codes(vault: State<'_, VaultService>) -> AppResult<Vec<TotpCode>> {
    let vault = vault.inner().clone();
    blocking(move || vault.totp_codes()).await
}

/// Add a 2FA account from an `otpauth://` link or a secret key plus a name.
#[tauri::command]
pub async fn totp_add(input: String, name: String, vault: State<'_, VaultService>) -> AppResult<()> {
    let vault = vault.inner().clone();
    blocking(move || vault.totp_add(&input, &name)).await
}

#[tauri::command]
pub async fn totp_delete(id: String, vault: State<'_, VaultService>) -> AppResult<()> {
    let vault = vault.inner().clone();
    blocking(move || vault.totp_delete(&id)).await
}

#[tauri::command]
pub async fn totp_copy(id: String, vault: State<'_, VaultService>) -> AppResult<()> {
    let vault = vault.inner().clone();
    blocking(move || vault.totp_copy(&id)).await
}

#[tauri::command]
pub fn snippets_list(expander: State<'_, ExpanderService>) -> AppResult<Vec<Snippet>> {
    expander.list()
}

#[tauri::command]
pub fn snippet_save(
    id: Option<String>,
    trigger: String,
    body: String,
    expander: State<'_, ExpanderService>,
) -> AppResult<Vec<Snippet>> {
    expander.save(id, &trigger, &body)
}

#[tauri::command]
pub fn snippet_delete(id: String, expander: State<'_, ExpanderService>) -> AppResult<Vec<Snippet>> {
    expander.delete(&id)
}

/// Start grabbing text from the screen under the pointer.
#[tauri::command]
pub async fn grab_begin(grab: State<'_, GrabService>) -> AppResult<()> {
    grab.inner().clone().begin().await
}

#[tauri::command]
pub async fn grab_picture(grab: State<'_, GrabService>) -> AppResult<String> {
    let grab = grab.inner().clone();
    blocking(move || grab.picture()).await
}

/// Read the text inside the box (screen pixels) and copy it.
#[tauri::command]
pub async fn grab_finish(x: u32, y: u32, w: u32, h: u32, grab: State<'_, GrabService>) -> AppResult<String> {
    let grab = grab.inner().clone();
    blocking(move || grab.finish(x, y, w, h)).await
}

#[tauri::command]
pub fn grab_cancel(grab: State<'_, GrabService>) {
    grab.cancel();
}

/// A QR code (SVG) for some text, to scan with a phone.
#[tauri::command]
pub fn qr_svg(text: String) -> AppResult<String> {
    if text.is_empty() || text.len() > 2000 {
        return Err(AppError::InvalidInput("That's too long for a QR code".into()));
    }
    crate::bridge::qr_svg(&text).ok_or_else(|| AppError::InvalidInput("That's too long for a QR code".into()))
}

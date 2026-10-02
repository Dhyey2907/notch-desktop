//! The vault: logins as notes in `<Obsidian vault>/Notch Vault/`, kept in sync both ways (the
//! folder is watched, so edits in Obsidian show up here), and 2FA accounts. Passwords and 2FA
//! secrets never reach the logs, the database or the UI unless the user reveals one.

use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};

use base64::{engine::general_purpose::STANDARD, Engine};
use chrono::Utc;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use tauri::{AppHandle, Manager, WebviewWindow};

use super::{
    entry::{self, Note},
    totp, TotpCode, VaultEntry, VaultField, VaultInput, VaultState, SUBFOLDER,
};
use crate::{
    error::{AppError, AppResult},
    events::AppEvent,
    secrets::{self, Credential},
    state::AppState,
};

const DEFAULT_CLEAR_SECS: u64 = 30;
/// Ignore stray huge files in the folder.
const MAX_NOTE_BYTES: u64 = 256 * 1024;

#[derive(Default)]
struct Inner {
    /// (entry, its file, its encrypted password)
    entries: Vec<(VaultEntry, PathBuf, String)>,
    watcher: Option<RecommendedWatcher>,
}

#[derive(Clone)]
pub struct VaultService {
    app: AppHandle,
    inner: Arc<Mutex<Inner>>,
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default()
}

impl VaultService {
    pub fn new(app: AppHandle) -> Self {
        Self {
            app,
            inner: Arc::default(),
        }
    }

    fn settings(&self) -> crate::settings::VaultSettings {
        self.app
            .state::<AppState>()
            .settings
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .vault
            .clone()
    }

    fn folder(&self) -> Option<PathBuf> {
        self.settings().folder.map(|f| Path::new(&f).join(SUBFOLDER))
    }

    pub fn start(&self) {
        if self.folder().is_some() {
            self.watch();
        }
        // Follow the folder setting (picked here, or brought in by importing settings).
        let this = self.clone();
        let mut events = self.app.state::<AppState>().bus.subscribe();
        tauri::async_runtime::spawn(async move {
            let mut folder = this.settings().folder;
            loop {
                match events.recv().await {
                    Ok(envelope) => {
                        if let AppEvent::SettingsChanged(s) = &envelope.event {
                            if s.vault.folder != folder {
                                folder = s.vault.folder.clone();
                                let this = this.clone();
                                tauri::async_runtime::spawn_blocking(move || {
                                    if this.folder().is_some() {
                                        this.watch();
                                    } else {
                                        this.inner.lock().unwrap_or_else(|e| e.into_inner()).watcher = None;
                                        this.reload();
                                    }
                                });
                            }
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(_) => break,
                }
            }
        });
    }

    /// Read every vault note in the folder and tell the UI.
    fn reload(&self) {
        let mut entries = Vec::new();
        if let Some(folder) = self.folder() {
            if let Ok(dir) = std::fs::read_dir(&folder) {
                for file in dir.flatten() {
                    let path = file.path();
                    let is_note = path.extension().is_some_and(|e| e.eq_ignore_ascii_case("md"));
                    let small = file
                        .metadata()
                        .is_ok_and(|m| m.is_file() && m.len() <= MAX_NOTE_BYTES);
                    if !is_note || !small {
                        continue;
                    }
                    let Some(note) = std::fs::read_to_string(&path).ok().and_then(|t| entry::parse(&t))
                    else {
                        continue;
                    };
                    let id = file.file_name().to_string_lossy().into_owned();
                    let title = if note.title.is_empty() {
                        id.trim_end_matches(".md").to_owned()
                    } else {
                        note.title.clone()
                    };
                    entries.push((
                        VaultEntry {
                            id,
                            title,
                            username: note.username,
                            url: note.url,
                            tags: note.tags,
                            notes: note.notes,
                            has_password: !note.password.is_empty(),
                        },
                        path,
                        note.password,
                    ));
                }
            }
        }
        entries.sort_by_key(|(e, _, _)| e.title.to_lowercase());
        self.inner.lock().unwrap_or_else(|e| e.into_inner()).entries = entries;
        self.app
            .state::<AppState>()
            .bus
            .emit("vault", AppEvent::VaultChanged(self.state()));
    }

    fn watch(&self) {
        let Some(folder) = self.folder() else { return };
        let _ = std::fs::create_dir_all(&folder);
        let this = self.clone();
        let pending = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let watcher = notify::recommended_watcher(move |_| {
            // Obsidian writes in bursts; read once it settles.
            if pending.swap(true, std::sync::atomic::Ordering::SeqCst) {
                return;
            }
            let this = this.clone();
            let pending = pending.clone();
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(300));
                pending.store(false, std::sync::atomic::Ordering::SeqCst);
                this.reload();
            });
        });
        match watcher {
            Ok(mut w) => {
                if let Err(error) = w.watch(&folder, RecursiveMode::NonRecursive) {
                    tracing::warn!(%error, "could not watch the vault folder");
                }
                self.inner.lock().unwrap_or_else(|e| e.into_inner()).watcher = Some(w);
            }
            Err(error) => tracing::warn!(%error, "could not watch the vault folder"),
        }
        self.reload();
    }

    pub fn state(&self) -> VaultState {
        VaultState {
            folder: self.settings().folder,
            entries: self
                .inner
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .entries
                .iter()
                .map(|(e, _, _)| e.clone())
                .collect(),
            #[cfg(windows)]
            hello_available: super::hello::available(),
            #[cfg(not(windows))]
            hello_available: false,
        }
    }

    /// Use this Obsidian vault folder (the logins go in its "Notch Vault" subfolder).
    pub fn set_folder(&self, folder: PathBuf) -> AppResult<VaultState> {
        if !folder.is_dir() {
            return Err(AppError::InvalidInput("That folder doesn't exist".into()));
        }
        std::fs::create_dir_all(folder.join(SUBFOLDER))?;
        {
            let state = self.app.state::<AppState>();
            let mut settings = state.settings.lock().unwrap_or_else(|e| e.into_inner()).clone();
            settings.vault.folder = Some(folder.to_string_lossy().into_owned());
            crate::commands::settings::persist_and_broadcast(&state, settings)?;
        }
        // The settings change starts watching the folder; read it now for the answer.
        self.reload();
        Ok(self.state())
    }

    fn key(&self, create: bool) -> AppResult<[u8; 32]> {
        if let Some(saved) = secrets::get(Credential::VaultKey)? {
            let bytes = STANDARD
                .decode(saved.trim())
                .map_err(|_| AppError::Platform("vault key is damaged".into()))?;
            return bytes
                .try_into()
                .map_err(|_| AppError::Platform("vault key is damaged".into()));
        }
        if !create {
            return Err(AppError::InvalidInput(
                "This PC doesn't have the vault key yet. Import it in Settings → Vault.".into(),
            ));
        }
        let mut key = [0u8; 32];
        rand::fill(&mut key);
        secrets::set(Credential::VaultKey, &STANDARD.encode(key))?;
        tracing::info!("vault key created");
        Ok(key)
    }

    fn find(&self, id: &str) -> AppResult<(VaultEntry, PathBuf, String)> {
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .entries
            .iter()
            .find(|(e, _, _)| e.id == id)
            .cloned()
            .ok_or_else(|| AppError::InvalidInput("That login is no longer in the vault".into()))
    }

    /// Add or change a login. Returns its id.
    pub fn save(&self, input: VaultInput) -> AppResult<String> {
        let folder = self
            .folder()
            .ok_or_else(|| AppError::InvalidInput("Choose your Obsidian vault first".into()))?;
        let title = input.title.trim().to_owned();
        if title.is_empty() || title.len() > 120 || input.username.len() > 300 || input.url.len() > 2000 {
            return Err(AppError::InvalidInput("Give the login a title".into()));
        }
        if input.notes.len() > 20_000
            || input.tags.len() > 20
            || input.password.as_ref().is_some_and(|p| p.len() > 1000)
        {
            return Err(AppError::InvalidInput("That's too long".into()));
        }
        let existing = input.id.as_deref().map(|id| self.find(id)).transpose()?;
        let password = match (&input.password, &existing) {
            (Some(p), _) if !p.is_empty() => entry::encrypt(&self.key(true)?, p)?,
            (Some(_), _) => String::new(),
            (None, Some((_, _, sealed))) => sealed.clone(),
            (None, None) => String::new(),
        };
        let note = Note {
            title: title.clone(),
            username: input.username.trim().to_owned(),
            url: input.url.trim().to_owned(),
            password,
            tags: input
                .tags
                .iter()
                .map(|t| t.trim().trim_start_matches('#').to_owned())
                .filter(|t| !t.is_empty())
                .collect(),
            notes: input.notes,
        };
        let path = match &existing {
            Some((_, path, _)) => path.clone(),
            None => crate::bridge::unique_path(&folder, &entry::file_name(&title)),
        };
        // Write through a temp file so Obsidian never sees half a note.
        let temp = path.with_extension("md.tmp");
        std::fs::write(&temp, entry::render(&note))?;
        std::fs::rename(&temp, &path)?;
        self.reload();
        Ok(path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default())
    }

    /// Move a login's note to the Recycle Bin (it can be restored from there).
    pub fn delete(&self, id: &str) -> AppResult<()> {
        let (_, path, _) = self.find(id)?;
        #[cfg(windows)]
        recycle(&path)?;
        #[cfg(not(windows))]
        std::fs::remove_file(&path)?;
        self.reload();
        Ok(())
    }

    fn require_hello(&self, window: &WebviewWindow, message: &str) -> AppResult<()> {
        if !self.settings().require_hello {
            return Ok(());
        }
        #[cfg(windows)]
        return super::hello::verify(window.hwnd()?.0 as isize, message);
        #[cfg(not(windows))]
        {
            let _ = (window, message);
            Ok(())
        }
    }

    fn reveal_inner(&self, id: &str) -> AppResult<String> {
        let (_, _, sealed) = self.find(id)?;
        if sealed.is_empty() {
            return Ok(String::new());
        }
        entry::decrypt(&self.key(false)?, &sealed)
    }

    pub fn reveal(&self, window: &WebviewWindow, id: &str) -> AppResult<String> {
        let (entry, _, _) = self.find(id)?;
        self.require_hello(window, &format!("Show the password for {}", entry.title))?;
        self.reveal_inner(id)
    }

    /// Copy a field; passwords are private and cleared after a short while.
    pub fn copy(&self, window: &WebviewWindow, id: &str, field: VaultField) -> AppResult<()> {
        let (entry, _, _) = self.find(id)?;
        match field {
            VaultField::Username => self.put(&entry.username, false),
            VaultField::Password => {
                self.require_hello(window, &format!("Copy the password for {}", entry.title))?;
                self.put(&self.reveal_inner(id)?, true)
            }
        }
    }

    fn put(&self, text: &str, private: bool) -> AppResult<()> {
        if text.is_empty() {
            return Err(AppError::InvalidInput("There's nothing to copy".into()));
        }
        #[cfg(windows)]
        {
            use crate::clipboard::win::{clear_if_unchanged, sequence, write, Put};
            if !private {
                return write(Put::Text(text));
            }
            write(Put::PrivateText(text))?;
            let copied = sequence();
            let settings = self.settings();
            if settings.never_clear {
                return Ok(());
            }
            let secs = match settings.clear_secs {
                0 => DEFAULT_CLEAR_SECS,
                s => u64::from(s),
            };
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_secs(secs));
                clear_if_unchanged(copied);
            });
            Ok(())
        }
        #[cfg(not(windows))]
        {
            let _ = (text, private);
            Err(AppError::Platform("not supported".into()))
        }
    }

    /// The vault key, to set up another PC (Windows Hello first when it's available).
    pub fn export_key(&self, window: &WebviewWindow) -> AppResult<String> {
        #[cfg(windows)]
        if super::hello::available() {
            super::hello::verify(window.hwnd()?.0 as isize, "Show your vault key")?;
        }
        #[cfg(not(windows))]
        let _ = window;
        Ok(STANDARD.encode(self.key(true)?))
    }

    /// Use the key from another PC so its passwords open here.
    pub fn import_key(&self, key: &str) -> AppResult<()> {
        let bytes = STANDARD
            .decode(key.trim())
            .map_err(|_| AppError::InvalidInput("That isn't a vault key".into()))?;
        if bytes.len() != 32 {
            return Err(AppError::InvalidInput("That isn't a vault key".into()));
        }
        secrets::set(Credential::VaultKey, &STANDARD.encode(bytes))?;
        tracing::info!("vault key imported");
        Ok(())
    }

    // ── 2FA ────────────────────────────────────────────────────────────────

    fn db<T>(&self, f: impl FnOnce(&rusqlite::Connection) -> AppResult<T>) -> AppResult<T> {
        self.app.state::<AppState>().db.with(f)
    }

    pub fn totp_add(&self, input: &str, name: &str) -> AppResult<()> {
        let account = totp::parse(input, name)?;
        let id = uuid::Uuid::new_v4().simple().to_string();
        let secret = data_encoding_base32(&account.secret);
        secrets::totp_set(&id, &secret)?;
        let algorithm = serde_json::to_value(account.algorithm)
            .ok()
            .and_then(|v| v.as_str().map(str::to_owned))
            .unwrap_or_else(|| "sha1".into());
        let saved = self.db(|conn| {
            conn.execute(
                "INSERT INTO totp_accounts (id, issuer, label, digits, period, algorithm, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                rusqlite::params![
                    id,
                    account.issuer,
                    account.label,
                    account.digits,
                    account.period,
                    algorithm,
                    Utc::now().to_rfc3339()
                ],
            )?;
            Ok(())
        });
        if saved.is_err() {
            let _ = secrets::totp_delete(&id);
        }
        saved
    }

    pub fn totp_delete(&self, id: &str) -> AppResult<()> {
        self.db(|conn| {
            conn.execute("DELETE FROM totp_accounts WHERE id = ?1", [id])?;
            Ok(())
        })?;
        secrets::totp_delete(id)
    }

    /// Every account's code right now (secrets stay in Credential Manager).
    pub fn totp_codes(&self) -> AppResult<Vec<TotpCode>> {
        type Row = (String, String, String, u32, u32, String);
        let rows: Vec<Row> = self.db(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, issuer, label, digits, period, algorithm FROM totp_accounts ORDER BY issuer COLLATE NOCASE",
            )?;
            let rows = stmt
                .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?)))?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(rows)
        })?;
        let now = now_secs();
        Ok(rows
            .into_iter()
            .filter_map(|(id, issuer, label, digits, period, algorithm)| {
                let secret = totp::base32(&secrets::totp_get(&id).ok()??)?;
                let algorithm = serde_json::from_value(serde_json::Value::String(algorithm)).ok()?;
                Some(TotpCode {
                    code: totp::code(&secret, algorithm, digits, period, now),
                    remaining: period - (now % u64::from(period)) as u32,
                    id,
                    issuer,
                    label,
                    period,
                })
            })
            .collect())
    }

    pub fn totp_copy(&self, id: &str) -> AppResult<()> {
        let code = self
            .totp_codes()?
            .into_iter()
            .find(|c| c.id == id)
            .ok_or_else(|| AppError::InvalidInput("That account is gone".into()))?;
        self.put(&code.code, true)
    }
}

/// Base32 (RFC 4648, no padding) for storing a secret as text.
fn data_encoding_base32(bytes: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    let mut out = String::new();
    let mut bits = 0u64;
    let mut count = 0;
    for &b in bytes {
        bits = (bits << 8) | u64::from(b);
        count += 8;
        while count >= 5 {
            count -= 5;
            out.push(ALPHABET[((bits >> count) & 31) as usize] as char);
        }
    }
    if count > 0 {
        out.push(ALPHABET[((bits << (5 - count)) & 31) as usize] as char);
    }
    out
}

#[cfg(windows)]
fn recycle(path: &Path) -> AppResult<()> {
    use windows::Win32::UI::Shell::{
        SHFileOperationW, FOF_ALLOWUNDO, FOF_NOCONFIRMATION, FOF_NOERRORUI, FOF_SILENT, FO_DELETE,
        SHFILEOPSTRUCTW,
    };
    let mut from: Vec<u16> = path.as_os_str().to_string_lossy().encode_utf16().collect();
    from.extend([0, 0]);
    let mut op = SHFILEOPSTRUCTW {
        wFunc: FO_DELETE,
        pFrom: windows::core::PCWSTR(from.as_ptr()),
        fFlags: (FOF_ALLOWUNDO | FOF_NOCONFIRMATION | FOF_SILENT | FOF_NOERRORUI).0 as u16,
        ..Default::default()
    };
    // SAFETY: `from` is double-NUL terminated and outlives the call.
    let result = unsafe { SHFileOperationW(&mut op) };
    if result == 0 && !op.fAnyOperationsAborted.as_bool() {
        Ok(())
    } else {
        Err(AppError::Platform(
            "could not move the note to the Recycle Bin".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base32_round_trips() {
        for bytes in [&b"1234567890"[..], b"hi", b"\x00\xff\x10"] {
            assert_eq!(totp::base32(&data_encoding_base32(bytes)).unwrap(), bytes);
        }
        assert_eq!(data_encoding_base32(b"1234567890"), "GEZDGNBVGY3TQOJQ");
    }
}

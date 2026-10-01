//! Runs the bridge's web server: one session at a time, ended by Stop or after 10 minutes.

use std::{
    io::{Read, Write},
    path::PathBuf,
    sync::{Arc, Mutex},
};

use chrono::Utc;
use tauri::{AppHandle, Manager};

use super::{
    clean_name, lan_ip, page, qr_svg, route, unique_path, BridgeState, Route, MAX_UPLOAD, SESSION_MINUTES,
};
use crate::{
    error::{AppError, AppResult},
    events::AppEvent,
    state::AppState,
};

struct Session {
    id: u64,
    server: Arc<tiny_http::Server>,
    state: BridgeState,
}

impl Drop for Session {
    fn drop(&mut self) {
        self.server.unblock();
    }
}

#[derive(Clone)]
pub struct BridgeService {
    app: AppHandle,
    inner: Arc<Mutex<Option<Session>>>,
}

impl BridgeService {
    pub fn new(app: AppHandle) -> Self {
        Self {
            app,
            inner: Arc::default(),
        }
    }

    fn inner(&self) -> std::sync::MutexGuard<'_, Option<Session>> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn state(&self) -> Option<BridgeState> {
        self.inner().as_ref().map(|s| s.state.clone())
    }

    fn publish(&self) {
        let state = self.state();
        self.app
            .state::<AppState>()
            .bus
            .emit("bridge", AppEvent::BridgeChanged(state));
    }

    /// Start a session offering `paths` to the phone (none: just receive). Replaces any
    /// running session.
    pub fn start(&self, paths: Vec<String>) -> AppResult<BridgeState> {
        let files: Vec<PathBuf> = paths
            .iter()
            .map(PathBuf::from)
            .filter(|p| p.is_absolute() && p.is_file())
            .take(50)
            .collect();
        if files.len() != paths.len() {
            return Err(AppError::InvalidInput(
                "Some of those files were moved or deleted".into(),
            ));
        }
        let ip = lan_ip().ok_or_else(|| {
            AppError::InvalidInput("Connect this PC to Wi-Fi (the same one as your phone) first".into())
        })?;
        let server = tiny_http::Server::http((ip, 0u16))
            .map_err(|e| AppError::Platform(format!("couldn't start the bridge: {e}")))?;
        let port = server
            .server_addr()
            .to_ip()
            .map(|a| a.port())
            .ok_or_else(|| AppError::Platform("no port".into()))?;
        let mut bytes = [0u8; 16];
        rand::fill(&mut bytes[..]);
        let token: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
        let url = format!("http://{ip}:{port}/{token}/");
        let state = BridgeState {
            qr_svg: qr_svg(&url).ok_or_else(|| AppError::Platform("couldn't draw the QR code".into()))?,
            url,
            files: files
                .iter()
                .map(|f| {
                    f.file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default()
                })
                .collect(),
            expires_at: Utc::now() + chrono::Duration::minutes(SESSION_MINUTES),
            received: 0,
        };
        let server = Arc::new(server);
        let id = random_u64();
        *self.inner() = Some(Session {
            id,
            server: server.clone(),
            state: state.clone(),
        });
        self.publish();

        // Serve requests, one thread each (an upload can take a while).
        let this = self.clone();
        std::thread::Builder::new()
            .name("bridge".into())
            .spawn(move || {
                for request in server.incoming_requests() {
                    let this = this.clone();
                    let token = token.clone();
                    let files = files.clone();
                    std::thread::spawn(move || this.handle(request, &token, &files));
                }
            })
            .map_err(|e| AppError::Platform(e.to_string()))?;

        // End the session when it expires.
        let this = self.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs(SESSION_MINUTES as u64 * 60));
            if this.inner().as_ref().is_some_and(|s| s.id == id) {
                this.stop();
            }
        });
        Ok(state)
    }

    pub fn stop(&self) {
        let ended = self.inner().take();
        if ended.is_some() {
            drop(ended);
            self.publish();
        }
    }

    fn handle(&self, mut request: tiny_http::Request, token: &str, files: &[PathBuf]) {
        let respond = |request: tiny_http::Request, status: u16, body: String, kind: &str| {
            let header =
                tiny_http::Header::from_bytes(&b"Content-Type"[..], kind.as_bytes()).expect("static header");
            let _ = request.respond(
                tiny_http::Response::from_string(body)
                    .with_status_code(status)
                    .with_header(header),
            );
        };
        let method = request.method().as_str().to_owned();
        match route(&method, request.url(), token) {
            Route::Forbidden => respond(request, 403, "Forbidden".into(), "text/plain"),
            Route::NotFound => respond(request, 404, "Not found".into(), "text/plain"),
            Route::Page => {
                let names: Vec<String> = files
                    .iter()
                    .map(|f| {
                        f.file_name()
                            .map(|n| n.to_string_lossy().into_owned())
                            .unwrap_or_default()
                    })
                    .collect();
                let pc = std::env::var("COMPUTERNAME").unwrap_or_else(|_| "this PC".into());
                respond(request, 200, page(&names, &pc), "text/html; charset=utf-8");
            }
            Route::Download(i) => match files
                .get(i)
                .and_then(|f| std::fs::File::open(f).ok().map(|h| (f, h)))
            {
                Some((path, file)) => {
                    let name = path
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    // RFC 5987 so non-ASCII names survive.
                    let encoded: String = name
                        .bytes()
                        .map(|b| {
                            if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
                                (b as char).to_string()
                            } else {
                                format!("%{b:02X}")
                            }
                        })
                        .collect();
                    let disposition = format!("attachment; filename*=UTF-8''{encoded}");
                    let header =
                        tiny_http::Header::from_bytes(&b"Content-Disposition"[..], disposition.as_bytes())
                            .expect("ascii header");
                    let _ = request.respond(tiny_http::Response::from_file(file).with_header(header));
                }
                None => respond(request, 404, "That file is gone".into(), "text/plain"),
            },
            Route::Upload(name) => {
                if request.body_length().is_some_and(|l| l as u64 > MAX_UPLOAD) {
                    return respond(request, 413, "Too large".into(), "text/plain");
                }
                match self.receive(request.as_reader(), &name) {
                    Ok(path) => {
                        respond(request, 200, "OK".into(), "text/plain");
                        self.received(path);
                    }
                    Err(_) => respond(request, 500, "Couldn't save it".into(), "text/plain"),
                }
            }
        }
    }

    /// Save an upload to Downloads\Notch (through a temp file, so a cut-off upload never
    /// appears as a finished file).
    fn receive(&self, body: &mut dyn Read, name: &str) -> AppResult<PathBuf> {
        let dir = self
            .app
            .path()
            .download_dir()
            .map_err(|e| AppError::Platform(e.to_string()))?
            .join("Notch");
        std::fs::create_dir_all(&dir)?;
        let partial = dir.join(format!(".notch-upload-{}.part", random_u64()));
        let result = (|| -> AppResult<()> {
            let mut out = std::fs::File::create(&partial)?;
            let copied = std::io::copy(&mut body.take(MAX_UPLOAD + 1), &mut out)?;
            out.flush()?;
            if copied > MAX_UPLOAD {
                return Err(AppError::InvalidInput("too large".into()));
            }
            Ok(())
        })();
        if let Err(error) = result {
            let _ = std::fs::remove_file(&partial);
            return Err(error);
        }
        let target = unique_path(&dir, &clean_name(name));
        std::fs::rename(&partial, &target)?;
        Ok(target)
    }

    fn received(&self, path: PathBuf) {
        if let Some(session) = self.inner().as_mut() {
            session.state.received += 1;
        }
        self.publish();
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let path_text = path.display().to_string();
        // Put it on the shelf so it's one drag away.
        let _ = self
            .app
            .state::<crate::shelf::service::ShelfService>()
            .add(std::slice::from_ref(&path_text));
        self.app
            .state::<AppState>()
            .bus
            .emit("bridge", AppEvent::BridgeReceived { name });
    }
}

fn random_u64() -> u64 {
    let mut bytes = [0u8; 8];
    rand::fill(&mut bytes[..]);
    u64::from_le_bytes(bytes)
}

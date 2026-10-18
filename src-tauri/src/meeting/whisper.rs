//! Whisper (whisper.cpp) for meeting notes: the official Windows build from whisper.cpp's
//! GitHub releases and the "base" speech model from its Hugging Face page, downloaded only when
//! you ask for them (Settings / the meeting card), into Overline's own folder. Speech is turned
//! into text here; no audio leaves the PC.

use std::{
    path::{Path, PathBuf},
    process::Command,
};

use crate::error::{AppError, AppResult};

const RELEASE_API: &str = "https://api.github.com/repos/ggml-org/whisper.cpp/releases/latest";
const ASSET: &str = "whisper-bin-x64.zip";
const MODEL_URL: &str = "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin";
const MODEL_FILE: &str = "ggml-base.bin";
/// The model is about 148 MB; anything much smaller is a failed download.
const MODEL_MIN_BYTES: u64 = 100_000_000;

pub fn dir(data: &Path) -> PathBuf {
    data.join("whisper")
}

fn model(data: &Path) -> PathBuf {
    dir(data).join(MODEL_FILE)
}

/// whisper.cpp's command-line program, wherever the zip put it.
fn cli(data: &Path) -> Option<PathBuf> {
    let root = dir(data).join("bin");
    let mut stack = vec![root];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d).ok()?.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p
                .file_name()
                .is_some_and(|n| n.eq_ignore_ascii_case("whisper-cli.exe"))
            {
                return Some(p);
            }
        }
    }
    None
}

pub fn ready(data: &Path) -> bool {
    cli(data).is_some() && std::fs::metadata(model(data)).is_ok_and(|m| m.len() >= MODEL_MIN_BYTES)
}

/// Download one file, reporting percent (0–100) as it goes.
async fn download(
    http: &reqwest::Client,
    url: &str,
    to: &Path,
    progress: &(dyn Fn(u8) + Send + Sync),
) -> AppResult<u64> {
    use std::io::Write;
    let mut resp = http
        .get(url)
        .send()
        .await
        .map_err(|_| AppError::InvalidInput("You're offline".into()))?;
    if !resp.status().is_success() {
        return Err(AppError::Platform(format!(
            "download failed ({})",
            resp.status().as_u16()
        )));
    }
    let total = resp.content_length().unwrap_or(0);
    let part = to.with_extension("part");
    let mut file = std::io::BufWriter::new(std::fs::File::create(&part)?);
    let mut done = 0u64;
    let mut last = 255u8;
    while let Some(chunk) = resp
        .chunk()
        .await
        .map_err(|_| AppError::InvalidInput("The download was interrupted".into()))?
    {
        file.write_all(&chunk)?;
        done += chunk.len() as u64;
        if let Some(pct) = (done * 100).checked_div(total).map(|p| p.min(100) as u8) {
            if pct != last {
                last = pct;
                progress(pct);
            }
        }
    }
    file.flush()?;
    drop(file);
    std::fs::rename(&part, to)?;
    Ok(done)
}

/// Download whisper.cpp and the speech model (about 150 MB in all).
pub async fn install(data: &Path, progress: impl Fn(u8) + Send + Sync) -> AppResult<()> {
    let root = dir(data);
    std::fs::create_dir_all(&root)?;
    let http = reqwest::Client::builder()
        .user_agent("Overline (whisper setup)")
        .build()
        .map_err(|e| AppError::Platform(e.to_string()))?;
    if cli(data).is_none() {
        let release: serde_json::Value = http
            .get(RELEASE_API)
            .send()
            .await
            .map_err(|_| AppError::InvalidInput("You're offline".into()))?
            .json()
            .await
            .map_err(|_| AppError::Platform("couldn't read whisper.cpp's release".into()))?;
        let url = release["assets"]
            .as_array()
            .and_then(|a| a.iter().find(|x| x["name"].as_str() == Some(ASSET)))
            .and_then(|x| x["browser_download_url"].as_str())
            .filter(|u| u.starts_with("https://github.com/ggml-org/whisper.cpp/releases/download/"))
            .ok_or_else(|| AppError::Platform("whisper.cpp's release has no Windows build".into()))?
            .to_string();
        let zip = root.join(ASSET);
        download(&http, &url, &zip, &|p| progress(p / 10)).await?;
        let bin = root.join("bin");
        let _ = std::fs::remove_dir_all(&bin);
        std::fs::create_dir_all(&bin)?;
        // Windows' own tar unpacks zips.
        let status = Command::new("tar")
            .arg("-xf")
            .arg(&zip)
            .arg("-C")
            .arg(&bin)
            .status()
            .map_err(|e| AppError::Platform(format!("couldn't unpack whisper.cpp: {e}")))?;
        let _ = std::fs::remove_file(&zip);
        if !status.success() || cli(data).is_none() {
            return Err(AppError::Platform("couldn't unpack whisper.cpp".into()));
        }
    }
    let m = model(data);
    if !std::fs::metadata(&m).is_ok_and(|x| x.len() >= MODEL_MIN_BYTES) {
        let got = download(&http, MODEL_URL, &m, &|p| progress(10 + p * 9 / 10)).await?;
        if got < MODEL_MIN_BYTES {
            let _ = std::fs::remove_file(&m);
            return Err(AppError::Platform(
                "the speech model didn't download fully".into(),
            ));
        }
    }
    progress(100);
    Ok(())
}

/// Text of a 16 kHz mono WAV (language detected automatically).
pub fn transcribe(data: &Path, wav: &Path) -> AppResult<String> {
    use std::os::windows::process::CommandExt;
    let exe = cli(data).ok_or_else(|| AppError::InvalidInput("Download the speech model first".into()))?;
    let out = wav.with_extension("");
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get().clamp(2, 8));
    let status = Command::new(&exe)
        .current_dir(exe.parent().unwrap_or(Path::new(".")))
        .arg("-m")
        .arg(model(data))
        .arg("-f")
        .arg(wav)
        .args(["-l", "auto", "-otxt", "-np", "-t", &threads.to_string(), "-of"])
        .arg(&out)
        .creation_flags(windows::Win32::System::Threading::CREATE_NO_WINDOW.0)
        .status()
        .map_err(|e| AppError::Platform(format!("couldn't run Whisper: {e}")))?;
    let txt = out.with_extension("txt");
    let text = std::fs::read_to_string(&txt).unwrap_or_default();
    let _ = std::fs::remove_file(&txt);
    if !status.success() && text.trim().is_empty() {
        return Err(AppError::Platform(
            "Whisper couldn't transcribe the recording".into(),
        ));
    }
    Ok(super::clean_transcript(&text))
}

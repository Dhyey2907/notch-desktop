//! Recently opened files, from the shortcuts Windows keeps in the Recent folder (the same
//! list File Explorer's "Recent" shows). Only names and paths are read, not contents.

use std::path::{Path, PathBuf};

use windows::{
    core::{Interface, HSTRING},
    Win32::{
        Storage::FileSystem::WIN32_FIND_DATAW,
        System::Com::{
            CoCreateInstance, CoInitializeEx, IPersistFile, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, STGM,
        },
        UI::Shell::{IShellLinkW, ShellLink, SLGP_RAWPATH},
    },
};

use super::RecentFile;

/// Shortcuts to look at (newest first) before giving up on finding enough files.
const SCAN: usize = 60;

fn recent_dir() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|appdata| Path::new(&appdata).join("Microsoft\\Windows\\Recent"))
}

fn target(link: &IShellLinkW, file: &IPersistFile, lnk: &Path) -> Option<PathBuf> {
    let mut buffer = [0u16; 1024];
    // SAFETY: COM calls on this thread's apartment with a local buffer.
    unsafe {
        file.Load(&HSTRING::from(lnk.as_os_str()), STGM(0)).ok()?;
        let mut data = WIN32_FIND_DATAW::default();
        link.GetPath(&mut buffer, &mut data, SLGP_RAWPATH.0 as u32).ok()?;
    }
    let len = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
    (len > 0).then(|| PathBuf::from(String::from_utf16_lossy(&buffer[..len])))
}

pub fn read(limit: usize) -> Vec<RecentFile> {
    let Some(dir) = recent_dir() else {
        return Vec::new();
    };
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut links: Vec<(std::time::SystemTime, PathBuf)> = entries
        .flatten()
        .filter(|e| {
            e.path()
                .extension()
                .is_some_and(|x| x.eq_ignore_ascii_case("lnk"))
        })
        .filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path())))
        .collect();
    links.sort_by_key(|l| std::cmp::Reverse(l.0));

    // SAFETY: COM on this thread; the interfaces are released when dropped.
    let (link, file) = unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let Ok(link) = CoCreateInstance::<_, IShellLinkW>(&ShellLink, None, CLSCTX_INPROC_SERVER) else {
            return Vec::new();
        };
        let Ok(file) = link.cast::<IPersistFile>() else {
            return Vec::new();
        };
        (link, file)
    };

    let mut out: Vec<RecentFile> = Vec::new();
    for (modified, lnk) in links.into_iter().take(SCAN) {
        if out.len() >= limit {
            break;
        }
        let Some(path) = target(&link, &file, &lnk) else {
            continue;
        };
        let Ok(meta) = std::fs::metadata(&path) else {
            continue;
        };
        if !meta.is_file() || out.iter().any(|f| Path::new(&f.path) == path) {
            continue;
        }
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        out.push(RecentFile {
            ext: path.extension().map(|e| e.to_string_lossy().to_lowercase()),
            path: path.to_string_lossy().into_owned(),
            name,
            size: meta.len(),
            opened_at: modified
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as i64)
                .unwrap_or_default(),
        });
    }
    out
}

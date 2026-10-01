//! Downloads in progress, from the browser's temporary files in the Downloads folder
//! (`.crdownload`, `.part`, …). Windows tells us when the folder changes; while something is
//! downloading, sizes are read once a second for the speed.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::mpsc::{channel, RecvTimeoutError},
    time::{Duration, Instant},
};

use notify::{RecursiveMode, Watcher};

use super::{final_name, speed, DownloadDone, DownloadProgress};

const TICK: Duration = Duration::from_secs(1);

struct Partial {
    bytes: u64,
    at: Instant,
    speed: u64,
}

fn partials(dir: &Path) -> HashMap<String, u64> {
    std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .flatten()
                .filter_map(|e| {
                    let name = e.file_name().to_string_lossy().into_owned();
                    final_name(&name)?;
                    let meta = e.metadata().ok()?;
                    meta.is_file().then_some((name, meta.len()))
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Watch `dir`. `progress` gets the downloads in progress (empty when they're all done);
/// `done` gets each download that finished.
pub fn watch(
    dir: PathBuf,
    progress: impl Fn(Vec<DownloadProgress>) + Send + 'static,
    done: impl Fn(DownloadDone) + Send + 'static,
) {
    std::thread::Builder::new()
        .name("downloads".into())
        .spawn(move || {
            let (tx, rx) = channel();
            let mut watcher = match notify::recommended_watcher(move |_| {
                let _ = tx.send(());
            }) {
                Ok(w) => w,
                Err(error) => {
                    tracing::warn!(%error, "could not watch the Downloads folder");
                    return;
                }
            };
            if let Err(error) = watcher.watch(&dir, RecursiveMode::NonRecursive) {
                tracing::warn!(%error, "could not watch the Downloads folder");
                return;
            }
            let mut known: HashMap<String, Partial> = HashMap::new();
            loop {
                // Idle: wait for the folder to change. Busy: also wake every second.
                let wait = if known.is_empty() {
                    Duration::from_secs(3600)
                } else {
                    TICK
                };
                match rx.recv_timeout(wait) {
                    Ok(()) => {
                        // Coalesce a burst of events.
                        while rx.try_recv().is_ok() {}
                    }
                    Err(RecvTimeoutError::Timeout) => {}
                    Err(RecvTimeoutError::Disconnected) => return,
                }
                let now = Instant::now();
                let seen = partials(&dir);
                let was_busy = !known.is_empty();
                // Finished (or cancelled): the temporary file is gone.
                let gone: Vec<String> = known.keys().filter(|k| !seen.contains_key(*k)).cloned().collect();
                for name in gone {
                    known.remove(&name);
                    let Some(final_name) = final_name(&name).filter(|n| !n.is_empty()) else {
                        continue;
                    };
                    let path = dir.join(&final_name);
                    // Give the browser a moment to rename the file.
                    for _ in 0..10 {
                        if path.is_file() {
                            break;
                        }
                        std::thread::sleep(Duration::from_millis(100));
                    }
                    if let Ok(meta) = std::fs::metadata(&path) {
                        if meta.is_file() && meta.len() > 0 {
                            done(DownloadDone {
                                name: final_name,
                                path: path.to_string_lossy().into_owned(),
                                size: meta.len(),
                            });
                        }
                    }
                }
                for (name, bytes) in &seen {
                    let entry = known.entry(name.clone()).or_insert(Partial {
                        bytes: *bytes,
                        at: now,
                        speed: 0,
                    });
                    let millis = now.duration_since(entry.at).as_millis() as u64;
                    if millis >= 500 {
                        entry.speed = speed(entry.bytes, *bytes, millis);
                        entry.bytes = *bytes;
                        entry.at = now;
                    }
                }
                if !known.is_empty() || was_busy {
                    let mut list: Vec<DownloadProgress> = known
                        .iter()
                        .map(|(name, p)| DownloadProgress {
                            name: final_name(name)
                                .filter(|n| !n.is_empty())
                                .unwrap_or_else(|| "Download".into()),
                            bytes: p.bytes,
                            speed: p.speed,
                        })
                        .collect();
                    list.sort_by(|a, b| a.name.cmp(&b.name));
                    progress(list);
                }
            }
        })
        .ok();
}

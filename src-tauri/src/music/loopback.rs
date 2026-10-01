//! The beat signal, from the level meters Windows keeps for every active output device
//! (`IAudioMeterInformation`). Reading every output and taking the loudest follows the music
//! wherever it plays — an app can send sound to headphones while the default output is the
//! speakers — and no audio is captured at all: only a peak number per device is read.
//! Runs only while music plays.

use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

use windows::Win32::{
    Media::Audio::{
        eRender, Endpoints::IAudioMeterInformation, IMMDeviceEnumerator, MMDeviceEnumerator,
        DEVICE_STATE_ACTIVE,
    },
    System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED},
};

use super::BeatDetector;

/// How often the meters are read (and the level reported, at most).
const READ_EVERY: Duration = Duration::from_millis(23);
/// Outputs come and go (headphones connect): look again this often.
const RESCAN_EVERY: Duration = Duration::from_secs(5);

/// A running reader; dropping it stops the thread.
pub struct Capture {
    stop: Arc<AtomicBool>,
}

impl Drop for Capture {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
    }
}

/// Start reading; `report(level, beat)` runs on the reader thread about 40 times a second.
pub fn start(mut report: impl FnMut(f32, bool) + Send + 'static) -> Capture {
    let stop = Arc::new(AtomicBool::new(false));
    let flag = stop.clone();
    std::thread::Builder::new()
        .name("music-beat".into())
        .spawn(move || {
            if let Err(error) = run(&flag, &mut report) {
                tracing::debug!(%error, "music beat reader stopped");
            }
        })
        .ok();
    Capture { stop }
}

fn meters(enumerator: &IMMDeviceEnumerator) -> Vec<IAudioMeterInformation> {
    // SAFETY: COM calls on this thread's MTA; failures just leave a device out.
    unsafe {
        let Ok(devices) = enumerator.EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE) else {
            return Vec::new();
        };
        (0..devices.GetCount().unwrap_or(0))
            .filter_map(|i| devices.Item(i).ok())
            .filter_map(|d| d.Activate::<IAudioMeterInformation>(CLSCTX_ALL, None).ok())
            .collect()
    }
}

fn run(stop: &AtomicBool, report: &mut dyn FnMut(f32, bool)) -> windows::core::Result<()> {
    // SAFETY: this thread joins the MTA and creates the system device enumerator.
    let enumerator: IMMDeviceEnumerator = unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;
        CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?
    };
    let mut outputs = meters(&enumerator);
    let mut scanned = Instant::now();
    let mut detector = BeatDetector::default();
    while !stop.load(Ordering::SeqCst) {
        std::thread::sleep(READ_EVERY);
        if scanned.elapsed() >= RESCAN_EVERY {
            outputs = meters(&enumerator);
            scanned = Instant::now();
        }
        // SAFETY: reading a peak value from live meter interfaces.
        let peak = outputs
            .iter()
            .filter_map(|m| unsafe { m.GetPeakValue() }.ok())
            .fold(0.0f32, f32::max);
        let (level, beat) = detector.feed(&[peak], READ_EVERY.as_millis() as u32);
        report(level, beat);
    }
    Ok(())
}

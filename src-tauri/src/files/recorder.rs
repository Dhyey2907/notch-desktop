//! Record what the PC plays (WASAPI loopback on the default output) to a WAV file, then turn
//! it into an MP3 with Windows' own encoder (Media Foundation). Nothing is sent anywhere.

use std::{
    fs::File,
    io::{BufWriter, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

use windows::{
    core::HSTRING,
    Win32::{
        Media::{
            Audio::{
                eConsole, eRender, IAudioCaptureClient, IAudioClient, IMMDeviceEnumerator,
                MMDeviceEnumerator, AUDCLNT_BUFFERFLAGS_SILENT, AUDCLNT_SHAREMODE_SHARED,
                AUDCLNT_STREAMFLAGS_LOOPBACK, WAVEFORMATEX, WAVEFORMATEXTENSIBLE,
            },
            MediaFoundation::{
                IMFSample, MFAudioFormat_MP3, MFAudioFormat_PCM, MFCreateMediaType,
                MFCreateSinkWriterFromURL, MFCreateSourceReaderFromURL, MFMediaType_Audio, MFShutdown,
                MFStartup, MFSTARTUP_FULL, MF_MT_AUDIO_AVG_BYTES_PER_SECOND, MF_MT_AUDIO_NUM_CHANNELS,
                MF_MT_AUDIO_SAMPLES_PER_SECOND, MF_MT_MAJOR_TYPE, MF_MT_SUBTYPE,
                MF_SOURCE_READERF_ENDOFSTREAM, MF_SOURCE_READER_FIRST_AUDIO_STREAM, MF_VERSION,
            },
        },
        System::Com::{CoCreateInstance, CoInitializeEx, CoTaskMemFree, CLSCTX_ALL, COINIT_MULTITHREADED},
    },
};

use crate::error::{AppError, AppResult};

const WAVE_FORMAT_PCM: u16 = 1;
const WAVE_FORMAT_IEEE_FLOAT: u16 = 3;
const WAVE_FORMAT_EXTENSIBLE: u16 = 0xFFFE;
/// 192 kbps.
const MP3_BYTES_PER_SECOND: u32 = 24_000;
/// Rates Windows' MP3 encoder takes.
const MP3_RATES: [u32; 3] = [32_000, 44_100, 48_000];
/// Recordings stop on their own after this long.
pub const MAX_LENGTH: Duration = Duration::from_secs(3 * 60 * 60);

/// A recording in progress; `stop` finishes it.
pub struct Recording {
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<AppResult<PathBuf>>>,
}

impl Recording {
    /// Stop and wait for the file (MP3 if it could be encoded, WAV otherwise).
    pub fn finish(mut self) -> AppResult<PathBuf> {
        self.stop.store(true, Ordering::SeqCst);
        self.thread
            .take()
            .ok_or_else(|| AppError::Platform("recording already finished".into()))?
            .join()
            .map_err(|_| AppError::Platform("the recorder crashed".into()))?
    }
}

impl Drop for Recording {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
    }
}

/// Start recording into `wav` (the MP3 goes next to it).
pub fn start(wav: PathBuf) -> Recording {
    let stop = Arc::new(AtomicBool::new(false));
    let flag = stop.clone();
    let thread = std::thread::Builder::new()
        .name("recorder".into())
        .spawn(move || {
            let format = capture(&wav, &flag)?;
            let mp3 = wav.with_extension("mp3");
            if MP3_RATES.contains(&format.rate) && format.channels <= 2 {
                match encode_mp3(&wav, &mp3, format) {
                    Ok(()) => {
                        let _ = std::fs::remove_file(&wav);
                        return Ok(mp3);
                    }
                    Err(error) => {
                        tracing::warn!(%error, "MP3 encoding failed; keeping the WAV");
                        let _ = std::fs::remove_file(&mp3);
                    }
                }
            }
            Ok(wav)
        })
        .ok();
    Recording { stop, thread }
}

#[derive(Clone, Copy)]
struct Format {
    rate: u32,
    /// Channels written (at most 2).
    channels: u16,
}

/// 16-bit PCM WAV writer; sizes are filled in when it's finished.
struct Wav {
    out: BufWriter<File>,
    data_bytes: u32,
}

impl Wav {
    fn create(path: &Path, f: Format) -> std::io::Result<Self> {
        let mut out = BufWriter::new(File::create(path)?);
        let block = f.channels * 2;
        out.write_all(b"RIFF\0\0\0\0WAVEfmt ")?;
        out.write_all(&16u32.to_le_bytes())?;
        out.write_all(&WAVE_FORMAT_PCM.to_le_bytes())?;
        out.write_all(&f.channels.to_le_bytes())?;
        out.write_all(&f.rate.to_le_bytes())?;
        out.write_all(&(f.rate * u32::from(block)).to_le_bytes())?;
        out.write_all(&block.to_le_bytes())?;
        out.write_all(&16u16.to_le_bytes())?;
        out.write_all(b"data\0\0\0\0")?;
        Ok(Self { out, data_bytes: 0 })
    }

    fn samples(&mut self, samples: &[i16]) -> std::io::Result<()> {
        for s in samples {
            self.out.write_all(&s.to_le_bytes())?;
        }
        self.data_bytes = self.data_bytes.saturating_add(samples.len() as u32 * 2);
        Ok(())
    }

    fn finish(mut self) -> std::io::Result<()> {
        self.out.flush()?;
        let mut file = self.out.into_inner().map_err(|e| e.into_error())?;
        file.seek(SeekFrom::Start(4))?;
        file.write_all(&(36 + self.data_bytes).to_le_bytes())?;
        file.seek(SeekFrom::Start(40))?;
        file.write_all(&self.data_bytes.to_le_bytes())?;
        Ok(())
    }
}

/// Convert one frame of the mix format to 16-bit samples (first two channels).
fn to_i16(frame: &[u8], float: bool, bits: u16, take: usize) -> impl Iterator<Item = i16> + '_ {
    let step = usize::from(bits / 8);
    (0..take).map(move |c| {
        let bytes = &frame[c * step..c * step + step];
        match (float, bits) {
            (true, 32) => {
                let v = f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
                (v.clamp(-1.0, 1.0) * 32767.0) as i16
            }
            (false, 16) => i16::from_le_bytes([bytes[0], bytes[1]]),
            (false, 24) => i16::from_le_bytes([bytes[1], bytes[2]]),
            (false, 32) => i16::from_le_bytes([bytes[2], bytes[3]]),
            _ => 0,
        }
    })
}

fn capture(wav: &Path, stop: &AtomicBool) -> AppResult<Format> {
    let platform = |e: windows::core::Error| AppError::Platform(e.to_string());
    // SAFETY: COM on this thread; the mix format is freed with CoTaskMemFree; buffers from
    // GetBuffer are read only for the frames reported and released right after.
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).map_err(platform)?;
        let device = enumerator
            .GetDefaultAudioEndpoint(eRender, eConsole)
            .map_err(platform)?;
        let client: IAudioClient = device.Activate(CLSCTX_ALL, None).map_err(platform)?;
        let mix = client.GetMixFormat().map_err(platform)?;
        let wf: WAVEFORMATEX = *mix;
        let float = match wf.wFormatTag {
            WAVE_FORMAT_IEEE_FLOAT => true,
            WAVE_FORMAT_EXTENSIBLE => {
                let ext = *mix.cast::<WAVEFORMATEXTENSIBLE>();
                // KSDATAFORMAT_SUBTYPE_IEEE_FLOAT starts with 3; PCM with 1.
                ext.SubFormat.data1 == 3
            }
            _ => false,
        };
        let bits = wf.wBitsPerSample;
        let channels_in = wf.nChannels;
        let block = usize::from(wf.nBlockAlign);
        let format = Format {
            rate: wf.nSamplesPerSec,
            channels: channels_in.min(2),
        };
        let result = (|| {
            if !(float && bits == 32) && !(!float && matches!(bits, 16 | 24 | 32)) {
                return Err(AppError::Platform("unsupported sound format".into()));
            }
            client
                .Initialize(
                    AUDCLNT_SHAREMODE_SHARED,
                    AUDCLNT_STREAMFLAGS_LOOPBACK,
                    10_000_000,
                    0,
                    mix,
                    None,
                )
                .map_err(platform)?;
            let capture: IAudioCaptureClient = client.GetService().map_err(platform)?;
            let mut out = Wav::create(wav, format)?;
            client.Start().map_err(platform)?;
            let started = Instant::now();
            let mut frames_written: u64 = 0;
            let take = usize::from(format.channels);
            let mut samples = Vec::new();
            while !stop.load(Ordering::SeqCst) && started.elapsed() < MAX_LENGTH {
                std::thread::sleep(Duration::from_millis(10));
                loop {
                    let packet = capture.GetNextPacketSize().map_err(platform)?;
                    if packet == 0 {
                        break;
                    }
                    let mut data = std::ptr::null_mut();
                    let mut frames = 0;
                    let mut flags = 0;
                    capture
                        .GetBuffer(&mut data, &mut frames, &mut flags, None, None)
                        .map_err(platform)?;
                    samples.clear();
                    if flags & AUDCLNT_BUFFERFLAGS_SILENT.0 as u32 != 0 || data.is_null() {
                        samples.resize(frames as usize * take, 0);
                    } else {
                        let bytes = std::slice::from_raw_parts(data, frames as usize * block);
                        for frame in bytes.chunks_exact(block) {
                            samples.extend(to_i16(frame, float, bits, take));
                        }
                    }
                    capture.ReleaseBuffer(frames).map_err(platform)?;
                    out.samples(&samples)?;
                    frames_written += u64::from(frames);
                }
                // Windows sends nothing while nothing plays: fill the gap with silence so
                // the recording keeps real time.
                let expected = (started.elapsed().as_secs_f64() * f64::from(format.rate)) as u64;
                let behind = expected.saturating_sub(frames_written);
                if behind > u64::from(format.rate) / 5 {
                    let pad = behind - u64::from(format.rate) / 10;
                    out.samples(&vec![0i16; pad as usize * take])?;
                    frames_written += pad;
                }
            }
            let _ = client.Stop();
            out.finish()?;
            Ok(format)
        })();
        CoTaskMemFree(Some(mix.cast()));
        result
    }
}

fn encode_mp3(wav: &Path, mp3: &Path, format: Format) -> windows::core::Result<()> {
    // SAFETY: Media Foundation calls on this thread, started and shut down here.
    unsafe {
        MFStartup(MF_VERSION, MFSTARTUP_FULL)?;
        let result = (|| {
            let reader = MFCreateSourceReaderFromURL(&HSTRING::from(wav.as_os_str()), None)?;
            let stream = MF_SOURCE_READER_FIRST_AUDIO_STREAM.0 as u32;
            let pcm = MFCreateMediaType()?;
            pcm.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Audio)?;
            pcm.SetGUID(&MF_MT_SUBTYPE, &MFAudioFormat_PCM)?;
            reader.SetCurrentMediaType(stream, None, &pcm)?;
            let input = reader.GetCurrentMediaType(stream)?;

            let writer = MFCreateSinkWriterFromURL(&HSTRING::from(mp3.as_os_str()), None, None)?;
            let out = MFCreateMediaType()?;
            out.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Audio)?;
            out.SetGUID(&MF_MT_SUBTYPE, &MFAudioFormat_MP3)?;
            out.SetUINT32(&MF_MT_AUDIO_SAMPLES_PER_SECOND, format.rate)?;
            out.SetUINT32(&MF_MT_AUDIO_NUM_CHANNELS, u32::from(format.channels))?;
            out.SetUINT32(&MF_MT_AUDIO_AVG_BYTES_PER_SECOND, MP3_BYTES_PER_SECOND)?;
            let index = writer.AddStream(&out)?;
            writer.SetInputMediaType(index, &input, None)?;
            writer.BeginWriting()?;
            loop {
                let mut flags = 0u32;
                let mut sample: Option<IMFSample> = None;
                reader.ReadSample(stream, 0, None, Some(&mut flags), None, Some(&mut sample))?;
                if flags & MF_SOURCE_READERF_ENDOFSTREAM.0 as u32 != 0 {
                    break;
                }
                if let Some(sample) = sample {
                    writer.WriteSample(index, &sample)?;
                }
            }
            writer.Finalize()
        })();
        let _ = MFShutdown();
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_mix_formats_to_16_bit() {
        let mut frame = Vec::new();
        frame.extend(0.5f32.to_le_bytes());
        frame.extend((-1.5f32).to_le_bytes());
        frame.extend(0.25f32.to_le_bytes());
        assert_eq!(
            to_i16(&frame, true, 32, 2).collect::<Vec<_>>(),
            vec![16383, -32767]
        );
        let pcm16 = [0x34, 0x12, 0xff, 0x7f];
        assert_eq!(
            to_i16(&pcm16, false, 16, 2).collect::<Vec<_>>(),
            vec![0x1234, 0x7fff]
        );
        let pcm24 = [0x00, 0x34, 0x12];
        assert_eq!(to_i16(&pcm24, false, 24, 1).collect::<Vec<_>>(), vec![0x1234]);
    }

    #[test]
    fn writes_a_valid_wav_header() {
        let path = std::env::temp_dir().join(format!("notch-wav-{}.wav", std::process::id()));
        let mut wav = Wav::create(
            &path,
            Format {
                rate: 48_000,
                channels: 2,
            },
        )
        .unwrap();
        wav.samples(&[1, 2, 3, 4]).unwrap();
        wav.finish().unwrap();
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(bytes.len(), 44 + 8);
        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(u32::from_le_bytes(bytes[4..8].try_into().unwrap()), 44);
        assert_eq!(u32::from_le_bytes(bytes[24..28].try_into().unwrap()), 48_000);
        assert_eq!(u32::from_le_bytes(bytes[40..44].try_into().unwrap()), 8);
        let _ = std::fs::remove_file(path);
    }

    /// Encodes a second of tone with Windows' MP3 encoder (needs Media Foundation).
    #[test]
    fn encodes_mp3_with_windows() {
        let dir = std::env::temp_dir();
        let wav = dir.join(format!("notch-tone-{}.wav", std::process::id()));
        let mp3 = wav.with_extension("mp3");
        let format = Format {
            rate: 48_000,
            channels: 2,
        };
        let mut out = Wav::create(&wav, format).unwrap();
        let tone: Vec<i16> = (0..48_000)
            .flat_map(|i| {
                let s = ((i as f32 * 440.0 * std::f32::consts::TAU / 48_000.0).sin() * 8000.0) as i16;
                [s, s]
            })
            .collect();
        out.samples(&tone).unwrap();
        out.finish().unwrap();
        encode_mp3(&wav, &mp3, format).unwrap();
        let size = std::fs::metadata(&mp3).unwrap().len();
        assert!(size > 10_000 && size < 60_000, "mp3 size {size}");
        let _ = std::fs::remove_file(wav);
        let _ = std::fs::remove_file(mp3);
    }
}

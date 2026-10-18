//! Runs a meeting recording: start, stop, then mix → transcribe → summarise → save, with the
//! state sent to the notch at each step.

use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use tauri::{AppHandle, Manager};

use super::MeetingState;
use crate::{
    error::{AppError, AppResult},
    events::AppEvent,
    state::AppState,
};

#[cfg(windows)]
type Rec = crate::files::recorder::Recording;
#[cfg(not(windows))]
type Rec = ();

struct Running {
    mic: Rec,
    pc: Rec,
    folder: PathBuf,
}

#[derive(Clone)]
pub struct MeetingService {
    app: AppHandle,
    state: Arc<Mutex<MeetingState>>,
    running: Arc<Mutex<Option<Running>>>,
}

impl MeetingService {
    pub fn new(app: AppHandle) -> Self {
        let this = Self {
            app,
            state: Arc::new(Mutex::new(MeetingState {
                phase: "idle".into(),
                ..Default::default()
            })),
            running: Arc::default(),
        };
        let ready = this.is_ready();
        this.state.lock().unwrap_or_else(|e| e.into_inner()).ready = ready;
        this
    }

    fn data(&self) -> AppResult<PathBuf> {
        self.app
            .path()
            .app_local_data_dir()
            .map_err(|e| AppError::Platform(e.to_string()))
    }

    fn is_ready(&self) -> bool {
        #[cfg(windows)]
        return self.data().is_ok_and(|d| super::whisper::ready(&d));
        #[cfg(not(windows))]
        false
    }

    pub fn state(&self) -> MeetingState {
        self.state.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    fn set(&self, change: impl FnOnce(&mut MeetingState)) {
        let next = {
            let mut st = self.state.lock().unwrap_or_else(|e| e.into_inner());
            change(&mut st);
            st.clone()
        };
        self.app
            .state::<AppState>()
            .bus
            .emit("meeting", AppEvent::MeetingChanged(next));
    }

    /// Download Whisper and the speech model (the user asked for it).
    pub async fn install(&self) -> AppResult<()> {
        #[cfg(windows)]
        {
            if self.state().phase == "installing" {
                return Ok(());
            }
            let data = self.data()?;
            self.set(|s| {
                s.phase = "installing".into();
                s.progress = Some(0);
                s.error = None;
            });
            let this = self.clone();
            let result = super::whisper::install(&data, move |p| this.set(|s| s.progress = Some(p))).await;
            let ready = self.is_ready();
            self.set(|s| {
                s.phase = "idle".into();
                s.progress = None;
                s.ready = ready;
                s.error = result.as_ref().err().map(|e| e.to_string());
            });
            result
        }
        #[cfg(not(windows))]
        Err(AppError::Platform("not supported".into()))
    }

    /// Start recording your microphone and the PC's sound.
    pub fn start(&self) -> AppResult<MeetingState> {
        if !self.is_ready() {
            return Err(AppError::InvalidInput("Download the speech model first".into()));
        }
        if self.state().phase != "idle" {
            return Ok(self.state());
        }
        #[cfg(windows)]
        {
            let base = self
                .app
                .path()
                .document_dir()
                .map_err(|e| AppError::Platform(e.to_string()))?
                .join("Overline meetings");
            let folder = base.join(chrono::Local::now().format("%Y-%m-%d %H.%M").to_string());
            std::fs::create_dir_all(&folder)?;
            let mic = crate::files::recorder::start_wav(folder.join("mic.wav"), true);
            let pc = crate::files::recorder::start_wav(folder.join("pc.wav"), false);
            *self.running.lock().unwrap_or_else(|e| e.into_inner()) = Some(Running { mic, pc, folder });
        }
        let now = chrono::Utc::now().timestamp_millis();
        self.set(|s| {
            s.phase = "recording".into();
            s.started_at = Some(now);
            s.error = None;
        });
        Ok(self.state())
    }

    /// Stop; the notes are made in the background and a card says when they're ready.
    pub fn stop(&self) -> AppResult<()> {
        let Some(run) = self.running.lock().unwrap_or_else(|e| e.into_inner()).take() else {
            return Ok(());
        };
        self.set(|s| {
            s.phase = "transcribing".into();
            s.started_at = None;
        });
        let this = self.clone();
        tauri::async_runtime::spawn(async move {
            let result = this.process(run).await;
            let (title, actions, ok) = match &result {
                Ok((title, actions)) => (title.clone(), *actions, true),
                Err(e) => (e.to_string(), 0, false),
            };
            this.set(|s| {
                s.phase = "idle".into();
                s.error = (!ok).then(|| title.clone());
            });
            this.app
                .state::<AppState>()
                .bus
                .emit("meeting", AppEvent::MeetingDone { title, actions, ok });
        });
        Ok(())
    }

    /// Throw the recording away.
    pub fn cancel(&self) {
        if let Some(run) = self.running.lock().unwrap_or_else(|e| e.into_inner()).take() {
            #[cfg(windows)]
            {
                let _ = run.mic.finish();
                let _ = run.pc.finish();
            }
            let _ = std::fs::remove_dir_all(&run.folder);
        }
        self.set(|s| {
            s.phase = "idle".into();
            s.started_at = None;
        });
    }

    #[cfg(windows)]
    async fn process(&self, run: Running) -> AppResult<(String, u32)> {
        let folder = run.folder.clone();
        let data = self.data()?;
        let blocking =
            |f: Box<dyn FnOnce() -> AppResult<String> + Send>| tauri::async_runtime::spawn_blocking(f);
        let fail = |e: tauri::Error| AppError::Platform(e.to_string());
        // Mix the two tracks into what Whisper reads, then transcribe.
        let f2 = folder.clone();
        let transcript = blocking(Box::new(move || {
            let mic = run.mic.finish()?;
            let pc = run.pc.finish()?;
            let track = |p: &std::path::Path| -> AppResult<Vec<f32>> {
                let (rate, ch, s) = super::read_wav(p)?;
                Ok(super::to_mono_16k(&s, rate, ch))
            };
            let mixed = super::mix(&track(&mic).unwrap_or_default(), &track(&pc).unwrap_or_default());
            if mixed.len() < super::WHISPER_RATE as usize * 2 {
                return Err(AppError::InvalidInput("The recording was too short".into()));
            }
            let wav = f2.join("meeting.wav");
            super::write_wav_16k(&wav, &mixed)?;
            let _ = std::fs::remove_file(&mic);
            let _ = std::fs::remove_file(&pc);
            let text = super::whisper::transcribe(&data, &wav)?;
            let _ = std::fs::remove_file(&wav);
            Ok(text)
        }))
        .await
        .map_err(fail)??;
        if transcript.trim().is_empty() {
            return Err(AppError::InvalidInput(
                "No speech was heard in the recording".into(),
            ));
        }
        let transcript_path = folder.join("transcript.txt");
        std::fs::write(&transcript_path, &transcript)?;
        self.set(|s| s.phase = "summarizing".into());
        let ask = self
            .app
            .state::<crate::ask::service::AskService>()
            .inner()
            .clone();
        let input: String = transcript.chars().take(super::MAX_TRANSCRIPT_CHARS).collect();
        let notes = blocking(Box::new(move || ask.answer_once(super::SUMMARY_SYSTEM, &input)))
            .await
            .map_err(fail)??;
        let title = notes
            .lines()
            .find(|l| !l.trim().is_empty())
            .unwrap_or("Meeting")
            .trim()
            .to_string();
        let body = format!(
            "Meeting notes · {}\n\n{}\n\nFull transcript: {}",
            chrono::Local::now().format("%d %b %Y"),
            notes.trim(),
            transcript_path.display()
        );
        let todo = self.app.state::<crate::todo::service::TodoService>();
        let _ = todo.add_note(&body.chars().take(10_000).collect::<String>());
        let items = super::action_items(&notes);
        for item in &items {
            let _ = todo.add_task(item, None, false);
        }
        Ok((title, items.len() as u32))
    }

    #[cfg(not(windows))]
    async fn process(&self, _run: Running) -> AppResult<(String, u32)> {
        Err(AppError::Platform("not supported".into()))
    }
}

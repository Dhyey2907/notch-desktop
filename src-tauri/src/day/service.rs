//! The day plan (made once on a workday morning if wanted, or on request) and Classroom
//! deadlines (checked every half hour while allowed).

use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
    time::Duration,
};

use chrono::{DateTime, Datelike, Local, NaiveTime, Utc};
use serde_json::Value;
use tauri::{AppHandle, Manager};

use super::{due_of, due_soon, plan_input, Assignment, ClassroomStatus, DayPlan, DayState, PLAN_PROMPT};
use crate::{
    calendar::{oauth::CLASSROOM_SCOPES, service::CalendarService},
    error::{AppError, AppResult},
    events::AppEvent,
    state::AppState,
};

const TICK: Duration = Duration::from_secs(300);
/// Classroom every this many ticks (30 min).
const CLASSROOM_EVERY: u64 = 6;
pub const PLAN_KEY: &str = "day_plan";
const API: &str = "https://classroom.googleapis.com/v1";
/// Only show work due within this many days.
const AHEAD_DAYS: i64 = 21;

struct Inner {
    state: DayState,
    reminded: HashSet<String>,
}

#[derive(Clone)]
pub struct DayService {
    app: AppHandle,
    http: reqwest::Client,
    inner: Arc<Mutex<Inner>>,
    wake: Arc<tokio::sync::Notify>,
}

fn time_of(minutes: u32) -> NaiveTime {
    NaiveTime::from_hms_opt(minutes / 60 % 24, minutes % 60, 0).unwrap_or_default()
}

impl DayService {
    pub fn new(app: AppHandle) -> Self {
        Self {
            app,
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(20))
                .user_agent("Overline")
                .build()
                .unwrap_or_default(),
            inner: Arc::new(Mutex::new(Inner {
                state: DayState {
                    plan: None,
                    planning: false,
                    classroom: ClassroomStatus::Off,
                    assignments: Vec::new(),
                },
                reminded: HashSet::new(),
            })),
            wake: Arc::default(),
        }
    }

    fn settings(&self) -> crate::settings::Settings {
        self.app
            .state::<AppState>()
            .settings
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    pub fn state(&self) -> DayState {
        self.inner.lock().unwrap_or_else(|e| e.into_inner()).state.clone()
    }

    fn update(&self, change: impl FnOnce(&mut DayState)) {
        let state = {
            let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
            change(&mut inner.state);
            inner.state.clone()
        };
        self.app
            .state::<AppState>()
            .bus
            .emit("day", AppEvent::DayChanged(state));
    }

    fn db<T>(&self, f: impl FnOnce(&rusqlite::Connection) -> AppResult<T>) -> AppResult<T> {
        self.app.state::<AppState>().db.with(f)
    }

    pub fn start(&self) {
        // Today's plan survives a restart.
        let saved: Option<DayPlan> = self
            .db(|conn| {
                Ok(conn
                    .query_row("SELECT value FROM kv_cache WHERE key = ?1", [PLAN_KEY], |r| {
                        r.get::<_, String>(0)
                    })
                    .ok())
            })
            .ok()
            .flatten()
            .and_then(|json| serde_json::from_str::<Value>(&json).ok())
            .and_then(|v| {
                Some(DayPlan {
                    text: v["text"].as_str()?.to_owned(),
                    made_at: v["madeAt"].as_str()?.parse().ok()?,
                })
            })
            .filter(|p| p.made_at.with_timezone(&Local).date_naive() == Local::now().date_naive());
        self.inner.lock().unwrap_or_else(|e| e.into_inner()).state.plan = saved;

        let this = self.clone();
        tauri::async_runtime::spawn(async move {
            let wake = this.wake.clone();
            let app = this.app.clone();
            tauri::async_runtime::spawn(async move {
                let mut events = app.state::<AppState>().bus.subscribe();
                let mut classroom = None;
                while let Ok(envelope) = events.recv().await {
                    if let AppEvent::SettingsChanged(s) = &envelope.event {
                        let now = (s.calendar.classroom, s.calendar.granted_scopes.len());
                        if classroom != Some(now) {
                            classroom = Some(now);
                            wake.notify_one();
                        }
                    }
                }
            });
            let mut ticks = 0u64;
            loop {
                if ticks.is_multiple_of(CLASSROOM_EVERY) {
                    this.check_classroom().await;
                }
                this.maybe_plan().await;
                ticks += 1;
                tokio::select! {
                    _ = tokio::time::sleep(TICK) => {}
                    _ = this.wake.notified() => ticks = 0,
                }
            }
        });
    }

    /// Workday mornings: make the plan once the PC is in use after work starts.
    async fn maybe_plan(&self) {
        let settings = self.settings();
        let day = &settings.day;
        let now = Local::now();
        let weekday = now.weekday().number_from_monday() as u8;
        let started = now.time() >= time_of(day.start_min) && now.time() < time_of(day.end_min);
        let planned_today = self
            .state()
            .plan
            .is_some_and(|p| p.made_at.with_timezone(&Local).date_naive() == now.date_naive());
        #[cfg(windows)]
        let active = crate::system::screentime::probe::idle_ms() < 120_000;
        #[cfg(not(windows))]
        let active = true;
        if day.no_auto_plan || !day.work_days.contains(&weekday) || !started || planned_today || !active {
            return;
        }
        match self.plan().await {
            Ok(plan) => {
                let first = plan.text.lines().next().unwrap_or_default().to_owned();
                self.app
                    .state::<AppState>()
                    .bus
                    .emit("day", AppEvent::DayPlanReady { first_line: first });
            }
            Err(error) => tracing::info!(kind = error.kind(), "morning plan not made"),
        }
    }

    /// Ask Claude Code to plan the rest of today.
    pub async fn plan(&self) -> AppResult<DayPlan> {
        if self.state().planning {
            return Err(AppError::InvalidInput("Already planning".into()));
        }
        self.update(|s| s.planning = true);
        let result = self.make_plan().await;
        match &result {
            Ok(plan) => {
                let json =
                    serde_json::json!({ "text": plan.text, "madeAt": plan.made_at.to_rfc3339() }).to_string();
                let _ = self.db(|conn| {
                    conn.execute(
                        "INSERT INTO kv_cache (key, value, fetched_at) VALUES (?1, ?2, ?3)
                         ON CONFLICT(key) DO UPDATE SET value = excluded.value, fetched_at = excluded.fetched_at",
                        rusqlite::params![PLAN_KEY, json, Utc::now().to_rfc3339()],
                    )?;
                    Ok(())
                });
                let plan = plan.clone();
                self.update(|s| {
                    s.plan = Some(plan);
                    s.planning = false;
                });
            }
            Err(_) => self.update(|s| s.planning = false),
        }
        result
    }

    async fn make_plan(&self) -> AppResult<DayPlan> {
        let settings = self.settings();
        let now = Local::now();
        let today = now.date_naive();
        let tasks: Vec<String> = self
            .app
            .state::<crate::todo::service::TodoService>()
            .snapshot()
            .map(|s| s.tasks.into_iter().filter(|t| !t.done).map(|t| t.title).collect())
            .unwrap_or_default();
        // Google data goes to the AI only if the user said so (Settings → Home → Your day).
        let share_google = settings.day.share_google;
        let meetings: Vec<(DateTime<Local>, DateTime<Local>, String)> = if !share_google {
            Vec::new()
        } else {
            self.app
                .state::<CalendarService>()
                .events()
                .into_iter()
                .filter(|e| !e.all_day && e.start.with_timezone(&Local).date_naive() == today)
                .map(|e| {
                    (
                        e.start.with_timezone(&Local),
                        e.end.with_timezone(&Local),
                        e.title,
                    )
                })
                .collect()
        };
        let due: Vec<(DateTime<Local>, String)> = if !share_google {
            Vec::new()
        } else {
            self.state()
                .assignments
                .into_iter()
                .filter(|a| a.due - Utc::now() <= chrono::Duration::days(3))
                .map(|a| (a.due.with_timezone(&Local), format!("{} ({})", a.title, a.course)))
                .collect()
        };
        let input = plan_input(
            share_google,
            now,
            &tasks,
            &meetings,
            &due,
            (time_of(settings.day.start_min), time_of(settings.day.end_min)),
        );
        let workdir = self
            .app
            .path()
            .app_cache_dir()
            .map_err(|e| AppError::Platform(e.to_string()))?
            .join("ask");
        let ai = settings.ai.clone();
        let text = tauri::async_runtime::spawn_blocking(move || {
            crate::ask::providers::answer(&ai, PLAN_PROMPT, &input, &workdir)
        })
        .await
        .map_err(|e| AppError::Platform(e.to_string()))??;
        Ok(DayPlan {
            text,
            made_at: Utc::now(),
        })
    }

    async fn get(&self, token: &str, url: &str) -> AppResult<Value> {
        let response = self
            .http
            .get(url)
            .bearer_auth(token)
            .send()
            .await
            .map_err(|_| AppError::InvalidInput("You're offline".into()))?;
        if !response.status().is_success() {
            return Err(AppError::Platform(format!(
                "classroom answered {}",
                response.status().as_u16()
            )));
        }
        response
            .json()
            .await
            .map_err(|e| AppError::Platform(e.to_string()))
    }

    async fn check_classroom(&self) {
        let calendar = self.app.state::<CalendarService>().inner().clone();
        if !self.settings().calendar.classroom {
            return self.update(|s| {
                s.classroom = ClassroomStatus::Off;
                s.assignments.clear();
            });
        }
        if !calendar.granted(CLASSROOM_SCOPES) {
            return self.update(|s| {
                s.classroom = ClassroomStatus::NeedsSignIn;
                s.assignments.clear();
            });
        }
        match self.fetch_assignments(&calendar).await {
            Ok(assignments) => {
                let now = Utc::now();
                let soon: Vec<Assignment> = {
                    let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
                    assignments
                        .iter()
                        .filter(|a| due_soon(a.due, now) && inner.reminded.insert(a.id.clone()))
                        .cloned()
                        .collect()
                };
                self.update(|s| {
                    s.classroom = ClassroomStatus::Ready;
                    s.assignments = assignments;
                });
                for a in soon {
                    self.app
                        .state::<AppState>()
                        .bus
                        .emit("day", AppEvent::AssignmentDue(a));
                }
            }
            Err(error) => {
                tracing::info!(kind = error.kind(), "classroom check failed");
                self.update(|s| s.classroom = ClassroomStatus::Error);
            }
        }
    }

    async fn fetch_assignments(&self, calendar: &CalendarService) -> AppResult<Vec<Assignment>> {
        let token = calendar.google_token().await?;
        let courses = self
            .get(
                &token,
                &format!("{API}/courses?studentId=me&courseStates=ACTIVE&pageSize=30"),
            )
            .await?;
        let now = Utc::now();
        let mut out = Vec::new();
        for course in courses["courses"].as_array().cloned().unwrap_or_default() {
            let (Some(course_id), Some(name)) = (course["id"].as_str(), course["name"].as_str()) else {
                continue;
            };
            let work = self
                .get(
                    &token,
                    &format!("{API}/courses/{course_id}/courseWork?orderBy=dueDate%20desc&pageSize=30"),
                )
                .await?;
            let done = self
                .get(
                    &token,
                    &format!(
                        "{API}/courses/{course_id}/courseWork/-/studentSubmissions?userId=me&states=TURNED_IN&states=RETURNED&pageSize=100"
                    ),
                )
                .await?;
            let finished: HashSet<&str> = done["studentSubmissions"]
                .as_array()
                .map(|subs| subs.iter().filter_map(|s| s["courseWorkId"].as_str()).collect())
                .unwrap_or_default();
            for w in work["courseWork"].as_array().cloned().unwrap_or_default() {
                let Some(id) = w["id"].as_str() else { continue };
                let Some(due) = due_of(&w) else { continue };
                let ahead = due - now;
                if finished.contains(id)
                    || ahead < chrono::Duration::hours(-24)
                    || ahead > chrono::Duration::days(AHEAD_DAYS)
                {
                    continue;
                }
                out.push(Assignment {
                    id: id.to_owned(),
                    course: name.to_owned(),
                    title: w["title"].as_str().unwrap_or("Assignment").to_owned(),
                    due,
                    link: w["alternateLink"].as_str().map(str::to_owned),
                });
            }
        }
        out.sort_by_key(|a| a.due);
        Ok(out)
    }

    pub fn refresh(&self) {
        self.wake.notify_one();
    }
}

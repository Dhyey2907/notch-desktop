use tauri::State;

use crate::{
    ask::service::{AskService, AskSnapshot},
    error::AppResult,
};

#[tauri::command]
pub fn ask_snapshot(ask: State<'_, AskService>) -> AskSnapshot {
    ask.snapshot()
}

/// Ask Claude Code; the answer streams in as `AskDelta` events. Returns the exchange id.
#[tauri::command]
pub fn ask_question(question: String, include_screen: bool, ask: State<'_, AskService>) -> AppResult<String> {
    ask.ask(&question, include_screen)
}

#[tauri::command]
pub fn ask_stop(ask: State<'_, AskService>) {
    ask.stop();
}

#[tauri::command]
pub fn ask_clear(ask: State<'_, AskService>) {
    ask.clear();
}

/// Whether an API key is saved for the OpenAI-compatible provider (the key itself never
/// leaves Windows Credential Manager).
#[tauri::command]
pub fn ai_key_saved() -> AppResult<bool> {
    Ok(crate::secrets::get(crate::secrets::Credential::AiApiKey)?.is_some_and(|k| !k.is_empty()))
}

#[tauri::command]
pub fn ai_set_key(key: String) -> AppResult<()> {
    let key = key.trim();
    if key.is_empty() || key.len() > 500 || key.chars().any(char::is_whitespace) {
        return Err(crate::error::AppError::InvalidInput(
            "that doesn't look like an API key".into(),
        ));
    }
    crate::secrets::set(crate::secrets::Credential::AiApiKey, key)
}

#[tauri::command]
pub fn ai_clear_key() -> AppResult<()> {
    crate::secrets::delete(crate::secrets::Credential::AiApiKey)
}

/// A one-line test of the chosen AI (Settings → AI → Test).
#[tauri::command]
pub async fn ai_test(app: tauri::AppHandle, state: State<'_, crate::state::AppState>) -> AppResult<String> {
    use tauri::Manager;
    let ai = state
        .settings
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .ai
        .clone();
    let workdir = app
        .path()
        .app_cache_dir()
        .map_err(|e| crate::error::AppError::Platform(e.to_string()))?
        .join("ask");
    tauri::async_runtime::spawn_blocking(move || {
        crate::ask::providers::answer(
            &ai,
            "You are checking that a connection works. Reply with a short friendly greeting of at most six words.",
            "Hello!",
            &workdir,
        )
    })
    .await
    .map_err(|e| crate::error::AppError::Platform(e.to_string()))?
}

/// Models installed in Ollama (empty if it isn't running).
#[tauri::command]
pub async fn ai_ollama_models() -> Vec<String> {
    crate::ask::providers::ollama_models().await
}

/// Ask about the window that was in front, with its picture (the palette and the shortcut).
#[tauri::command]
pub fn ask_explain_screen(ask: State<'_, AskService>) -> AppResult<String> {
    ask.explain_screen()
}

/// The selection waiting to be rewritten.
#[tauri::command]
pub fn rewrite_session() -> Option<crate::ask::rewrite::RewriteSession> {
    crate::ask::rewrite::session()
}

/// Rewrite the selection in a style ("formal", "shorter", "friendlier", "fix" or "custom" with
/// an instruction). Returns the new text; nothing is replaced until `rewrite_apply`.
#[tauri::command]
pub async fn rewrite_run(style: String, custom: String, ask: State<'_, AskService>) -> AppResult<String> {
    let session = crate::ask::rewrite::session().ok_or_else(|| {
        crate::error::AppError::InvalidInput("Select text and press the shortcut again".into())
    })?;
    let how = crate::ask::rewrite::instruction(&style, &custom)
        .ok_or_else(|| crate::error::AppError::InvalidInput("Say how to rewrite it".into()))?;
    let ask = ask.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let input = format!("{how}\n\nText:\n{}", session.text);
        ask.answer_once(crate::ask::rewrite::SYSTEM, &input)
            .map(|a| crate::ask::rewrite::clean(&a))
    })
    .await
    .map_err(|e| crate::error::AppError::Platform(e.to_string()))?
}

/// Replace the selection with this text (back in the app it came from).
#[tauri::command]
pub async fn rewrite_apply(text: String) -> AppResult<()> {
    if text.trim().is_empty() || text.chars().count() > crate::ask::rewrite::MAX_CHARS * 2 {
        return Err(crate::error::AppError::InvalidInput("Nothing to paste".into()));
    }
    tauri::async_runtime::spawn_blocking(move || {
        #[cfg(windows)]
        return crate::ask::rewrite::apply(&text);
        #[cfg(not(windows))]
        {
            let _ = text;
            Ok(())
        }
    })
    .await
    .map_err(|e| crate::error::AppError::Platform(e.to_string()))?
}

#[tauri::command]
pub fn rewrite_cancel() {
    crate::ask::rewrite::cancel();
}

/// "Ask your PC": search files, notes (and Gmail / the calendar if Google data is allowed for
/// AI) for what the question is about, then answer from what was found.
#[tauri::command]
pub async fn ask_pc(question: String, app: tauri::AppHandle) -> AppResult<crate::ask::pc::PcAnswer> {
    use crate::ask::pc::{self, Found};
    use tauri::Manager;
    let question = question.trim().to_string();
    if question.is_empty() || question.chars().count() > 500 {
        return Err(crate::error::AppError::InvalidInput(
            "Ask a short question".into(),
        ));
    }
    let ask = app.state::<AskService>().inner().clone();
    let now = chrono::Local::now();
    let today = now.format("%Y-%m-%d (%A)").to_string();
    let blocking = |f: Box<dyn FnOnce() -> AppResult<String> + Send>| tauri::async_runtime::spawn_blocking(f);

    // 1. What to look for.
    let plan_input = format!("Today is {today}.\nQuestion: {question}");
    let ask1 = ask.clone();
    let reply = blocking(Box::new(move || ask1.answer_once(pc::PLAN_SYSTEM, &plan_input)))
        .await
        .map_err(|e| crate::error::AppError::Platform(e.to_string()))??;
    let plan = pc::parse_plan(&reply);
    let mut words = plan.words.clone();
    if words.is_empty() {
        words = question
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| w.len() >= 4)
            .map(str::to_lowercase)
            .take(4)
            .collect();
    }

    // 2. Look, here on the PC.
    let mut found: Vec<Found> = Vec::new();
    let roots: Vec<std::path::PathBuf> = [
        app.path().desktop_dir().ok(),
        app.path().document_dir().ok(),
        app.path().download_dir().ok(),
        app.path().video_dir().ok(),
        app.path().picture_dir().ok(),
        // Your own folders (projects and the like); the scan is capped.
        app.path().home_dir().ok(),
    ]
    .into_iter()
    .flatten()
    .collect();
    let (w2, days) = (words.clone(), plan.days);
    let files = tauri::async_runtime::spawn_blocking(move || {
        #[cfg(windows)]
        let recent: Vec<std::path::PathBuf> = crate::files::recent::read(200)
            .into_iter()
            .map(|r| std::path::PathBuf::from(r.path))
            .collect();
        #[cfg(not(windows))]
        let recent: Vec<std::path::PathBuf> = Vec::new();
        pc::search_files(&roots, &recent, &w2, days, chrono::Utc::now())
    })
    .await
    .map_err(|e| crate::error::AppError::Platform(e.to_string()))?;
    for (path, modified, _) in files {
        found.push(Found {
            kind: if path.is_dir() { "folder" } else { "file" }.into(),
            title: path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
            detail: format!(
                "in {}, changed {}",
                path.parent().map(|p| p.display().to_string()).unwrap_or_default(),
                modified.with_timezone(&chrono::Local).format("%d %b %Y")
            ),
            path: Some(path.display().to_string()),
        });
    }
    let notes = app
        .state::<crate::state::AppState>()
        .db
        .with(crate::todo::list_notes)
        .unwrap_or_default();
    for n in notes.iter().filter(|n| pc::score(&n.body, &words) > 0).take(5) {
        let first: String = n.body.lines().next().unwrap_or("").chars().take(120).collect();
        found.push(Found {
            kind: "note".into(),
            title: first,
            detail: format!(
                "note from {}",
                n.updated_at.with_timezone(&chrono::Local).format("%d %b %Y")
            ),
            path: None,
        });
    }
    let share_google = app
        .state::<crate::state::AppState>()
        .settings
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .day
        .share_google;
    let wants_google = plan.mail.is_some() || plan.calendar.is_some();
    if share_google {
        if let Some(q) = &plan.mail {
            let mail = app.state::<crate::mail::service::MailService>().inner().clone();
            if let Ok(messages) = mail.search(q, 6).await {
                for m in messages {
                    found.push(Found {
                        kind: "mail".into(),
                        title: m.subject,
                        detail: format!(
                            "from {}, {}",
                            m.from,
                            m.at.with_timezone(&chrono::Local).format("%d %b %Y")
                        ),
                        path: None,
                    });
                }
            }
        }
        if let Some((from, to)) = plan.calendar.as_ref().and_then(pc::Range::dates) {
            let events = app.state::<crate::calendar::service::CalendarService>().events();
            for e in events
                .into_iter()
                .filter(|e| {
                    let d = e.start.with_timezone(&chrono::Local).date_naive();
                    d >= from && d <= to
                })
                .take(8)
            {
                found.push(Found {
                    kind: "event".into(),
                    detail: e
                        .start
                        .with_timezone(&chrono::Local)
                        .format("%a %d %b, %H:%M")
                        .to_string(),
                    title: e.title,
                    path: None,
                });
            }
        }
    }

    // 3. Answer from what was found.
    let input = pc::context(&question, &found, &today);
    let answer = blocking(Box::new(move || ask.answer_once(pc::ANSWER_SYSTEM, &input)))
        .await
        .map_err(|e| crate::error::AppError::Platform(e.to_string()))??;
    Ok(pc::PcAnswer {
        answer: answer.trim().to_string(),
        found,
        skipped_google: wants_google && !share_google,
    })
}

/// Research a topic in the background; a card says when the brief is ready (it's saved as a
/// note with its sources).
#[tauri::command]
pub fn research_start(topic: String, app: tauri::AppHandle) -> AppResult<()> {
    use tauri::Manager;
    let topic = topic.trim().to_string();
    if topic.is_empty() || topic.chars().count() > crate::ask::research::MAX_TOPIC {
        return Err(crate::error::AppError::InvalidInput(
            "Say what to research, briefly".into(),
        ));
    }
    tauri::async_runtime::spawn(async move {
        let result = research(&app, &topic).await;
        let (brief, ok) = match result {
            Ok(text) => (text.chars().take(280).collect(), true),
            Err(e) => (e.to_string(), false),
        };
        app.state::<crate::state::AppState>()
            .bus
            .emit("ask", crate::events::AppEvent::ResearchDone { topic, brief, ok });
    });
    Ok(())
}

async fn research(app: &tauri::AppHandle, topic: &str) -> AppResult<String> {
    use crate::ask::research as r;
    use tauri::Manager;
    let ask = app.state::<AskService>().inner().clone();
    let fail = |e: tauri::Error| crate::error::AppError::Platform(e.to_string());
    let (brief, sources) = if ask.uses_claude() {
        let (ask, t) = (ask.clone(), topic.to_string());
        let brief = tauri::async_runtime::spawn_blocking(move || ask.answer_web(r::CLAUDE_SYSTEM, &t))
            .await
            .map_err(fail)??;
        (brief, Vec::new())
    } else {
        let http = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) Overline")
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .map_err(|e| crate::error::AppError::Platform(e.to_string()))?;
        let q: String = url::form_urlencoded::byte_serialize(topic.as_bytes()).collect();
        let html = http
            .get(format!("https://html.duckduckgo.com/html/?q={q}"))
            .send()
            .await
            .map_err(|_| crate::error::AppError::InvalidInput("You're offline".into()))?
            .text()
            .await
            .unwrap_or_default();
        let mut pages = Vec::new();
        for (title, url) in r::ddg_results(&html, 4) {
            let Ok(resp) = http.get(url.clone()).send().await else {
                continue;
            };
            if !resp.status().is_success() {
                continue;
            }
            let Ok(body) = resp.text().await else { continue };
            let page = crate::readlater::read_page(&body.chars().take(1_500_000).collect::<String>());
            if page.text.trim().len() > 200 {
                pages.push((page.title.unwrap_or(title), url, page.text));
            }
        }
        if pages.is_empty() {
            return Err(crate::error::AppError::InvalidInput(
                "Couldn't find pages about that".into(),
            ));
        }
        let input = r::pages_context(topic, &pages);
        let ask2 = ask.clone();
        let brief = tauri::async_runtime::spawn_blocking(move || ask2.answer_once(r::SUMMARY_SYSTEM, &input))
            .await
            .map_err(fail)??;
        (brief, pages.into_iter().map(|(_, u, _)| u).collect())
    };
    let body = r::note_body(topic, &brief, &sources);
    let _ = app.state::<crate::todo::service::TodoService>().add_note(&body);
    Ok(brief)
}

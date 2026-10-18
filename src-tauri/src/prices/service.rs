//! Checks watched prices in the background (every few hours) and says when one drops.

use std::time::Duration;

use tauri::{AppHandle, Manager};

use super::{Found, PriceItem};
use crate::{
    error::{AppError, AppResult},
    events::AppEvent,
    state::AppState,
};

/// How often the list is looked at (each item is only fetched when it's due).
const TICK: Duration = Duration::from_secs(30 * 60);

#[derive(Clone)]
pub struct PriceService {
    app: AppHandle,
    http: reqwest::Client,
}

impl PriceService {
    pub fn new(app: AppHandle) -> Self {
        Self {
            app,
            // A normal browser's headers: many shops refuse requests that don't look like one.
            http: reqwest::Client::builder()
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0 Safari/537.36")
                .timeout(Duration::from_secs(20))
                .build()
                .unwrap_or_default(),
        }
    }

    fn db<T>(&self, f: impl FnOnce(&rusqlite::Connection) -> AppResult<T>) -> AppResult<T> {
        self.app.state::<AppState>().db.with(f)
    }

    fn publish(&self) {
        if let Ok(items) = self.db(super::list) {
            self.app
                .state::<AppState>()
                .bus
                .emit("prices", AppEvent::PricesChanged(items));
        }
    }

    pub fn list(&self) -> AppResult<Vec<PriceItem>> {
        self.db(super::list)
    }

    pub fn start(&self) {
        let this = self.clone();
        tauri::async_runtime::spawn(async move {
            // Not right at startup.
            tokio::time::sleep(Duration::from_secs(120)).await;
            loop {
                let now = chrono::Utc::now().timestamp_millis();
                let due: Vec<String> = this
                    .list()
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|i| now - i.checked_at >= super::CHECK_EVERY_SECS * 1000)
                    .map(|i| i.id)
                    .collect();
                for id in due {
                    let _ = this.check(&id).await;
                    // Spread the visits out.
                    tokio::time::sleep(Duration::from_secs(20)).await;
                }
                tokio::time::sleep(TICK).await;
            }
        });
    }

    async fn fetch(&self, url: &str) -> Result<Found, String> {
        let resp = self
            .http
            .get(url)
            .header("Accept-Language", "en-IN,en;q=0.9")
            .header("Accept", "text/html,application/xhtml+xml")
            .send()
            .await
            .map_err(|_| "Couldn't reach the shop".to_string())?;
        if !resp.status().is_success() {
            return Err(format!("The shop said no ({})", resp.status().as_u16()));
        }
        let html = resp
            .text()
            .await
            .map_err(|_| "Couldn't read the page".to_string())?;
        super::parse(&html).ok_or_else(|| "Couldn't find a price on the page".to_string())
    }

    /// Check one item now; a PriceDrop event when it reaches the target.
    pub async fn check(&self, id: &str) -> AppResult<PriceItem> {
        let before = self
            .db(|c| super::get(c, id))?
            .ok_or_else(|| AppError::InvalidInput("That item is gone".into()))?;
        let result = self.fetch(&before.url).await;
        let now = chrono::Utc::now().timestamp_millis();
        let after = self
            .db(|c| super::record(c, id, now, result.as_ref().map_err(Clone::clone)))?
            .ok_or_else(|| AppError::InvalidInput("That item is gone".into()))?;
        if let Some(price) = after.price.filter(|_| result.is_ok()) {
            if super::crossed(before.price, price, before.target) {
                self.app.state::<AppState>().bus.emit(
                    "prices",
                    AppEvent::PriceDrop {
                        title: after.title.clone(),
                        price,
                        target: after.target,
                        currency: after.currency.clone(),
                        url: after.url.clone(),
                    },
                );
            }
        }
        self.publish();
        Ok(after)
    }

    pub async fn add(&self, url: &str, target: f64) -> AppResult<PriceItem> {
        let url = super::check_url(url)?.to_string();
        if !(target.is_finite() && target > 0.0) {
            return Err(AppError::InvalidInput("Give a target price".into()));
        }
        let id = uuid::Uuid::new_v4().to_string();
        let item = PriceItem {
            id: id.clone(),
            url,
            title: String::new(),
            target,
            currency: String::new(),
            price: None,
            checked_at: 0,
            history: Vec::new(),
            error: None,
        };
        self.db(|c| super::insert(c, &item))?;
        self.check(&id).await
    }

    pub fn set_target(&self, id: &str, target: f64) -> AppResult<()> {
        if !(target.is_finite() && target > 0.0) {
            return Err(AppError::InvalidInput("Give a target price".into()));
        }
        self.db(|c| super::set_target(c, id, target))?;
        self.publish();
        Ok(())
    }

    pub fn remove(&self, id: &str) -> AppResult<()> {
        self.db(|c| super::remove(c, id))?;
        self.publish();
        Ok(())
    }
}

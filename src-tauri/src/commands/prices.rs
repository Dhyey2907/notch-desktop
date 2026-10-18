use tauri::State;

use crate::{
    error::AppResult,
    prices::{service::PriceService, PriceItem},
};

#[tauri::command]
pub fn price_list(prices: State<'_, PriceService>) -> AppResult<Vec<PriceItem>> {
    prices.list()
}

/// Watch a product page; it's checked right away.
#[tauri::command]
pub async fn price_add(url: String, target: f64, prices: State<'_, PriceService>) -> AppResult<PriceItem> {
    let prices = prices.inner().clone();
    prices.add(&url, target).await
}

/// Check one item now.
#[tauri::command]
pub async fn price_check(id: String, prices: State<'_, PriceService>) -> AppResult<PriceItem> {
    let prices = prices.inner().clone();
    prices.check(&id).await
}

#[tauri::command]
pub fn price_set_target(id: String, target: f64, prices: State<'_, PriceService>) -> AppResult<()> {
    prices.set_target(&id, target)
}

#[tauri::command]
pub fn price_remove(id: String, prices: State<'_, PriceService>) -> AppResult<()> {
    prices.remove(&id)
}

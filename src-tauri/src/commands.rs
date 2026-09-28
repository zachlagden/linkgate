use serde::Serialize;
use tauri::{AppHandle, State, WebviewWindow};

use crate::browsers::{self, Browser, BrowserView};
use crate::link::{LinkView, ParsedLink};
use crate::lists::{self, ListHit, ListsStatus};
use crate::{logging, placement, settings};

pub struct AppState {
    pub link: Option<ParsedLink>,
    pub raw: Option<String>,
    pub hits: Vec<ListHit>,
    pub browsers: std::sync::Mutex<Vec<Browser>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InitialState {
    link: Option<LinkView>,
    raw: Option<String>,
    hits: Vec<ListHit>,
    browsers: Vec<BrowserView>,
    lists: ListsStatus,
    version: &'static str,
}

fn refresh_browsers(state: &AppState) -> Vec<BrowserView> {
    let detected = browsers::detect();
    let prefs = browsers::apply_first_seen_defaults(&detected);
    let views = browsers::views(&detected, &prefs);
    if let Ok(mut cache) = state.browsers.lock() {
        *cache = detected;
    }
    views
}

#[tauri::command]
pub async fn initial_state(state: State<'_, AppState>) -> Result<InitialState, String> {
    Ok(InitialState {
        link: state.link.as_ref().map(|l| l.view.clone()),
        raw: state.raw.clone(),
        hits: state.hits.clone(),
        browsers: refresh_browsers(&state),
        lists: lists::status(),
        version: env!("CARGO_PKG_VERSION"),
    })
}

#[tauri::command]
pub fn present(window: WebviewWindow, height: f64) -> Result<(), String> {
    placement::present(&window, height).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn resize(window: WebviewWindow, height: f64) -> Result<(), String> {
    placement::resize(&window, height).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_in(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<(), String> {
    let link = state.link.as_ref().ok_or("There's no link to open.")?;
    if !link.view.openable {
        return Err("Browsers can't open this kind of link.".into());
    }
    let browser = state
        .browsers
        .lock()
        .map_err(|e| e.to_string())?
        .iter()
        .find(|b| b.id == id)
        .cloned()
        .ok_or("That browser is no longer installed.")?;
    browsers::launch(&browser, &link.view.href).inspect_err(|e| logging::error("launch_failed", e))?;
    app.exit(0);
    Ok(())
}

#[tauri::command]
pub fn copy_link(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let text = state
        .link
        .as_ref()
        .map(|l| l.view.href.clone())
        .or_else(|| state.raw.clone())
        .ok_or("There's no link to copy.")?;
    let mut clipboard = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    clipboard
        .set_text(text)
        .map_err(|e| e.to_string())
        .inspect_err(|e| logging::error("copy_failed", e))?;
    app.exit(0);
    Ok(())
}

#[tauri::command]
pub fn dismiss(app: AppHandle) {
    app.exit(0);
}

#[tauri::command]
pub async fn set_browser_hidden(
    state: State<'_, AppState>,
    id: String,
    hidden: bool,
) -> Result<Vec<BrowserView>, String> {
    let mut prefs = settings::load();
    prefs.hidden.retain(|h| h != &id);
    if hidden {
        prefs.hidden.push(id);
    }
    settings::save(&prefs)?;
    Ok(refresh_browsers(&state))
}

#[tauri::command]
pub async fn update_lists() -> Result<ListsStatus, String> {
    tauri::async_runtime::spawn_blocking(lists::update_all)
        .await
        .map_err(|e| e.to_string())??;
    Ok(lists::status())
}

#[tauri::command]
pub fn lists_status() -> ListsStatus {
    lists::status()
}

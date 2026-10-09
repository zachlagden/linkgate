use serde::Serialize;
use tauri::{AppHandle, State, WebviewWindow};

use crate::browsers::{self, Browser, BrowserView};
use crate::link::{LinkView, ParsedLink};
use crate::lists::{self, ListHit, ListsStatus};
use crate::updates::{self, UpdateStatus};
use crate::{logging, order, placement, settings, timeout};

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
    blocklist_enabled: bool,
    update: UpdateStatus,
    update_check_enabled: bool,
    auto_close_enabled: bool,
    timeout_seconds: u32,
    version: &'static str,
}

fn refresh_browsers(state: &AppState) -> Vec<BrowserView> {
    let detected = browsers::detect();
    let prefs = browsers::apply_first_seen_defaults(&detected);
    let detected = order::arrange(detected, |b| b.id.as_str(), &prefs.order);
    let views = browsers::views(&detected, &prefs);
    if let Ok(mut cache) = state.browsers.lock() {
        *cache = detected;
    }
    views
}

#[tauri::command]
pub async fn initial_state(state: State<'_, AppState>) -> Result<InitialState, String> {
    let prefs = settings::load();
    Ok(InitialState {
        link: state.link.as_ref().map(|l| l.view.clone()),
        raw: state.raw.clone(),
        hits: state.hits.clone(),
        browsers: refresh_browsers(&state),
        lists: lists::status(),
        blocklist_enabled: prefs.blocklist_enabled,
        update: updates::status(),
        update_check_enabled: prefs.update_check_enabled,
        auto_close_enabled: prefs.auto_close_enabled,
        timeout_seconds: prefs.timeout_seconds,
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
pub async fn set_browser_order(state: State<'_, AppState>, ids: Vec<String>) -> Result<Vec<BrowserView>, String> {
    let installed: Vec<String> = state
        .browsers
        .lock()
        .map_err(|e| e.to_string())?
        .iter()
        .map(|b| b.id.clone())
        .collect();
    let requested: Vec<String> = ids.into_iter().filter(|id| installed.contains(id)).collect();
    let mut prefs = settings::load();
    prefs.order = order::merge_saved(&prefs.order, &requested);
    settings::save(&prefs)?;
    Ok(refresh_browsers(&state))
}

#[tauri::command]
pub fn set_blocklist_enabled(enabled: bool) -> Result<(), String> {
    let mut prefs = settings::load();
    prefs.blocklist_enabled = enabled;
    settings::save(&prefs)
}

#[tauri::command]
pub fn set_update_check_enabled(enabled: bool) -> Result<(), String> {
    let mut prefs = settings::load();
    prefs.update_check_enabled = enabled;
    settings::save(&prefs)
}

#[tauri::command]
pub fn set_auto_close(enabled: bool, seconds: u32) -> Result<u32, String> {
    let mut prefs = settings::load();
    prefs.auto_close_enabled = enabled;
    prefs.timeout_seconds = timeout::clamp(seconds);
    settings::save(&prefs)?;
    Ok(prefs.timeout_seconds)
}

#[tauri::command]
pub fn run_update(app: AppHandle) -> Result<(), String> {
    updates::launch_installer()?;
    app.exit(0);
    Ok(())
}

#[tauri::command]
pub async fn update_lists() -> Result<ListsStatus, String> {
    if !settings::load().blocklist_enabled {
        return Err("Blocklist checks are turned off.".into());
    }
    tauri::async_runtime::spawn_blocking(lists::update_all)
        .await
        .map_err(|e| e.to_string())??;
    Ok(lists::status())
}

#[tauri::command]
pub fn lists_status() -> ListsStatus {
    lists::status()
}

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod browsers;
mod commands;
mod icons;
mod link;
mod lists;
mod logging;
mod paths;
mod placement;
mod settings;

use commands::AppState;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--update-lists") {
        if settings::load().blocklist_enabled {
            if let Err(error) = lists::update_all() {
                logging::error("list_update_failed", error);
            }
        }
        return;
    }
    let raw = args.into_iter().find(|a| !a.starts_with("--"));
    run(raw);
}

fn run(raw: Option<String>) {
    let link = raw.as_deref().and_then(link::parse);
    let blocklist_enabled = settings::load().blocklist_enabled;
    let hits = link
        .as_ref()
        .and_then(|l| l.host.as_deref())
        .filter(|_| blocklist_enabled)
        .map(lists::lookup)
        .unwrap_or_default();
    if blocklist_enabled && lists::is_stale() && !lists::is_updating() {
        lists::spawn_background_update();
    }
    let state = AppState {
        link,
        raw,
        hits,
        browsers: std::sync::Mutex::new(Vec::new()),
    };
    let result = tauri::Builder::default()
        .manage(state)
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::Focused(false) = event {
                let _ = window.set_always_on_top(false);
                let _ = window.set_always_on_top(true);
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::initial_state,
            commands::present,
            commands::resize,
            commands::open_in,
            commands::copy_link,
            commands::dismiss,
            commands::set_browser_hidden,
            commands::set_blocklist_enabled,
            commands::update_lists,
            commands::lists_status,
        ])
        .run(tauri::generate_context!());
    if let Err(error) = result {
        logging::error("app_failed", error);
    }
}

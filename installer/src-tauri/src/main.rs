#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod border;
mod checksums;
mod commands;
mod distros;
mod fsutil;
mod headless;
mod install;
mod jsonc;
mod logging;
mod paths;
mod progress;
mod registry;
mod shortcut;
mod source;
mod state;
mod uninstall;
mod wsl;

#[cfg(test)]
mod flow_tests;

use commands::{AppState, Mode};
use paths::Locations;
use source::Source;

fn mode_for(args: &[String], locations: &Result<Locations, String>) -> Mode {
    if args.iter().any(|arg| arg == "--uninstall") {
        return Mode::Uninstall;
    }
    let installed = locations
        .as_ref()
        .ok()
        .and_then(|locations| state::load(&locations.record_path()))
        .is_some();
    if installed || args.iter().any(|arg| arg == "--update") {
        Mode::Update
    } else {
        Mode::Install
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Some(index) = args.iter().position(|arg| arg == "--headless") {
        let code = match args.get(index + 1) {
            Some(path) => headless::run(std::path::Path::new(path)),
            None => 2,
        };
        std::process::exit(code);
    }

    let locations = Locations::detect();
    let current_exe = match std::env::current_exe() {
        Ok(path) => path,
        Err(error) => {
            logging::write(&std::env::temp_dir(), "error", "no_current_exe", &error.to_string());
            return;
        }
    };
    let app_state = AppState {
        mode: mode_for(&args, &locations),
        locations,
        source: Source::from_env(),
        current_exe,
    };
    let result = tauri::Builder::default()
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            commands::initial_state,
            commands::check_latest,
            commands::run_install,
            commands::run_uninstall,
            commands::copy_text,
            commands::open_linkgate,
            commands::present,
            commands::dismiss,
        ])
        .run(tauri::generate_context!());
    if let Err(error) = result {
        logging::write(&std::env::temp_dir(), "error", "app_failed", &error.to_string());
    }
}

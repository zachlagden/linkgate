use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::install::{self, Context, InstallChoices};
use crate::paths::Locations;
use crate::progress::Summary;
use crate::source::Source;
use crate::uninstall::{self, UninstallChoices};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Job {
    mode: String,
    #[serde(default)]
    install: InstallChoices,
    #[serde(default)]
    uninstall: UninstallChoices,
    result_path: String,
}

#[derive(Serialize)]
struct Outcome {
    ok: bool,
    error: Option<String>,
    summary: Option<Summary>,
}

fn execute(job: &Job) -> Result<Summary, String> {
    let locations = Locations::detect()?;
    let source = Source::from_env();
    let current_exe = std::env::current_exe().map_err(|e| format!("Couldn't find the running installer: {e}"))?;
    let ctx = Context {
        locations: &locations,
        source: &source,
        current_exe: &current_exe,
    };
    let mut ignore = |_| {};
    match job.mode.as_str() {
        "install" => install::run(&ctx, &job.install, &mut ignore),
        "uninstall" => uninstall::run(&ctx, &job.uninstall, &mut ignore),
        other => Err(format!("Unknown headless mode {other:?}.")),
    }
}

pub fn run(job_path: &Path) -> i32 {
    let parsed = std::fs::read(job_path)
        .map_err(|e| format!("Couldn't read {}: {e}", job_path.display()))
        .and_then(|bytes| serde_json::from_slice::<Job>(&bytes).map_err(|e| format!("Couldn't parse the job: {e}")));
    let job = match parsed {
        Ok(job) => job,
        Err(_) => return 2,
    };
    let result = execute(&job);
    let outcome = match &result {
        Ok(summary) => Outcome {
            ok: summary.failed == 0,
            error: None,
            summary: Some(summary.clone()),
        },
        Err(error) => Outcome {
            ok: false,
            error: Some(error.clone()),
            summary: None,
        },
    };
    let written = serde_json::to_vec_pretty(&outcome)
        .map_err(|e| e.to_string())
        .and_then(|json| std::fs::write(&job.result_path, json).map_err(|e| e.to_string()));
    match (written, outcome.ok) {
        (Ok(()), true) => 0,
        _ => 1,
    }
}

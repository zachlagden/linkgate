use std::os::windows::process::CommandExt;

use crate::logging;

const DETACHED_PROCESS: u32 = 0x0000_0008;
pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub fn spawn(flag: &str, failure_event: &str) {
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    if let Err(error) = std::process::Command::new(exe)
        .arg(flag)
        .creation_flags(DETACHED_PROCESS | CREATE_NO_WINDOW)
        .spawn()
    {
        logging::error(failure_event, error);
    }
}

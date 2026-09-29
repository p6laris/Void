//! Appends background errors to `void.log`, where stderr would draw over the TUI.

use std::io::Write;

pub fn log_error(message: &str) {
    let Ok(dir) = crate::db::data_dir() else {
        return;
    };
    let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("void.log"))
    else {
        return;
    };
    let stamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    let _ = writeln!(file, "{stamp} {message}");
}

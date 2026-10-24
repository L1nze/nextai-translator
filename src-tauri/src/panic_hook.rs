// Release builds on Windows use `windows_subsystem = "windows"`, so the
// process has no console and stdout/stderr are silently discarded. A panic
// that only prints to stderr leaves zero evidence behind, which makes
// background crashes impossible to diagnose. This hook mirrors every panic
// (payload, location, thread, backtrace) into a persistent log file next to
// the app config so crashes can be traced after the fact.

use std::backtrace::Backtrace;
use std::io::Write;
use std::path::PathBuf;

const LOG_MAX_BYTES: u64 = 1024 * 1024;

pub fn install() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let payload = if let Some(s) = info.payload().downcast_ref::<&str>() {
            (*s).to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "non-string panic payload".to_string()
        };
        let location = info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "unknown location".to_string());
        let thread = std::thread::current();
        let thread_name = thread.name().unwrap_or("<unnamed>").to_string();
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let backtrace = Backtrace::force_capture();
        let record = format!(
            "[{}] thread '{}' panicked at {}: {}\n{}\n\n",
            timestamp, thread_name, location, payload, backtrace
        );
        // File IO failures must be ignored: panicking inside the panic hook
        // would abort the process immediately.
        let _ = append_record(&record);
        // Keep the default behavior (print to stderr, abort-on-panic in
        // panic=abort builds) intact.
        default_hook(info);
    }));
}

fn append_record(record: &str) -> std::io::Result<()> {
    let Some(path) = log_path() else {
        return Ok(());
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    // Rotate by truncating once the file exceeds the cap so the log cannot
    // grow forever while still keeping the freshest panics.
    if let Ok(meta) = std::fs::metadata(&path) {
        if meta.len() > LOG_MAX_BYTES {
            std::fs::write(&path, b"")?;
        }
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;
    file.write_all(record.as_bytes())
}

pub fn log_path() -> Option<PathBuf> {
    let base = if cfg!(target_os = "windows") {
        std::env::var_os("APPDATA").map(PathBuf::from)
    } else {
        std::env::var_os("HOME").map(PathBuf::from)
    };
    let subdir = if cfg!(target_os = "windows") {
        "xyz.yetone.apps.openai-translator"
    } else {
        "Library/Application Support/xyz.yetone.apps.openai-translator"
    };
    Some(
        base.map(|b| b.join(subdir).join("panic.log"))
            .unwrap_or_else(|| std::env::temp_dir().join("nextai-translator-panic.log")),
    )
}

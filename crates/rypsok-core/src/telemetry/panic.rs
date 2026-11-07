//! What a panic writes (RFC 020, F-1 of the 020q review). The standard panic
//! hook writes `thread '…' panicked at file:line:` and the panic's message in
//! plain text to standard error, outside the JSON log and outside its privacy
//! rules (a formatted message can carry a value). The hook installed here
//! writes **one** line in the log's form and never the message; the process
//! then goes on as before (the panic unwinds, a contained one ends its call
//! only).

use std::io::Write;

use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::time::{FormatTime, UtcTime};

/// The line for a panic at `file:line` in `thread`: a JSON object with the
/// keys of every log line (`timestamp`, `level`, `event`, `message`) and the
/// `location` and the `thread`. It takes no message, so none can be in it.
pub fn panic_line(timestamp: &str, thread: &str, file: &str, line: u32) -> String {
    serde_json::json!({
        "timestamp": timestamp,
        "level": "ERROR",
        "event": "panic",
        "message": "a thread panicked; its message is not logged",
        "location": format!("{file}:{line}"),
        "thread": thread,
    })
    .to_string()
}

/// Installs the hook. Call it first in `main`, before anything can panic.
pub fn install_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        let (file, line) = info
            .location()
            .map_or(("<unknown>", 0), |at| (at.file(), at.line()));
        let current = std::thread::current();
        let thread = current.name().unwrap_or("<unnamed>");
        let mut stamp = String::new();
        if UtcTime::rfc_3339()
            .format_time(&mut Writer::new(&mut stamp))
            .is_err()
        {
            stamp.clear();
        }
        let mut stderr = std::io::stderr().lock();
        let _ = writeln!(stderr, "{}", panic_line(&stamp, thread, file, line));
    }));
}

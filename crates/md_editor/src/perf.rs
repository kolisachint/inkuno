//! Optional keystroke-to-paint timing for the performance pass (task 10.4, DESIGN §12).
//!
//! Set `NOTEAPP_PERF=1` to print one line to stderr per typed character:
//! `NOTEAPP_PERF keystroke_to_paint_ms=<ms>`. The clock starts when the text input handler
//! receives the character and stops when the block list finishes painting, so it covers the
//! edit, the re-split, the layout of the visible blocks and the paint of that frame. Without the
//! variable the cost is one cached boolean read per call.

use std::sync::{Mutex, OnceLock};
use std::time::Instant;

static ENABLED: OnceLock<bool> = OnceLock::new();
static PENDING: Mutex<Option<Instant>> = Mutex::new(None);

fn enabled() -> bool {
    *ENABLED.get_or_init(|| std::env::var_os("NOTEAPP_PERF").is_some())
}

/// A typed character arrived. Starts the clock for the next paint.
pub fn key_received() {
    if !enabled() {
        return;
    }
    if let Ok(mut pending) = PENDING.lock() {
        *pending = Some(Instant::now());
    }
}

/// The block list finished painting. Prints the time since the last character, if any.
pub fn painted() {
    if !enabled() {
        return;
    }
    if let Ok(mut pending) = PENDING.lock() {
        if let Some(start) = pending.take() {
            eprintln!(
                "NOTEAPP_PERF keystroke_to_paint_ms={:.2}",
                start.elapsed().as_secs_f64() * 1000.0
            );
        }
    }
}

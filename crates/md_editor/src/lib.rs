//! Editor element and modes for noteapp.
//!
//! Phase 3 skeleton. Every module is GPUI-dependent, so the crate is gated on macOS and
//! Linux. Each module belongs to one plan task (3.1 to 3.5); the task briefs are in
//! `docs/P3-interfaces.md`. The shared `Editor` entity is in `editor.rs`, which holds the
//! only wiring between modules: the `HOOK` comments in `Editor::render` and `editor::init`.

#[cfg(any(target_os = "macos", target_os = "linux"))]
pub mod block_list; // task 3.5
#[cfg(any(target_os = "macos", target_os = "linux"))]
pub mod blocks; // task 4.1
#[cfg(any(target_os = "macos", target_os = "linux"))]
pub mod clipboard; // task 3.3
#[cfg(any(target_os = "macos", target_os = "linux"))]
pub mod cursor; // task 3.2
#[cfg(any(target_os = "macos", target_os = "linux"))]
pub mod edit_keys; // task 3.7
#[cfg(any(target_os = "macos", target_os = "linux"))]
pub mod editor; // shared skeleton, owned by the phase
#[cfg(any(target_os = "macos", target_os = "linux"))]
pub mod element; // task 3.1
#[cfg(any(target_os = "macos", target_os = "linux"))]
pub mod ime; // task 3.4
#[cfg(any(target_os = "macos", target_os = "linux"))]
pub mod keys; // task 3.3
#[cfg(any(target_os = "macos", target_os = "linux"))]
pub mod selection; // task 3.2

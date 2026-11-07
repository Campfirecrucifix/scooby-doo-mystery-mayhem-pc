//! Rust port of Scooby-Doo! Mystery Mayhem (PS2) game logic, starting with the Tome of Doom (book).
//! See PORT_NOTES.md. Source of truth: `Decompiled Source/.gamedb/index.sqlite` and the decompiled C.
// Original C identifiers (pLocalDataTome, TomeDoomST_OPEN, ...) are kept verbatim for 1:1 traceability,
// and dead stores of the original are kept, so these lints are off crate-wide.
#![allow(non_snake_case, non_upper_case_globals, unused_assignments)]
pub mod crashlog;
pub mod engine;
pub mod libs;
pub mod logic;
pub mod modules;
pub mod plugin;

//! WP-P17: Episode 5 scripted probe routes, one entry per level (format = routes_ep2.rs / tests/engine_p17.rs
//! step script). Run: `P17_EP=5 cargo test --release --test engine_p17 -- --ignored --nocapture`.
#![allow(dead_code)]

/// (level, steps, expected next level). Game buffers are carried level to level (C EFGameMemoryMgr).
pub const EP5_ROUTE: &[(&str, &str, &str)] = &[
];

/// not a real binary: cargo auto-discovers every src/bin/*.rs as a bin target
fn main() {}

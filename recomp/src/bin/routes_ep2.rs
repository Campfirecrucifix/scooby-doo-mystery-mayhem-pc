//! WP-P11: Episode 2 scripted routes (EP2/L01A .. last level), one entry per level.
//! Step format = tests/engine_r7.rs / tests/engine_p11.rs step script (';'-separated): `f N` frames,
//! `x N` X presses (re-warp each 60 f), `p BTN N` hold pad bits, `m UID MSG P2 [W..]` SendMessageTo,
//! `warp UID` leader onto an object, `wxyz X Y Z` leader to a position, `g ID OFF VAL` game buffer write
//! (only for documented faked carried bits), `l UID OFF VAL` local write.
//! These are probe routes (warps to the C trigger points), not game.rs path-finding waypoint routes;
//! the coordinator wires them (game.rs level_route is not edited by this WP).
//! Run: `cargo test --release --test engine_p11 -- --ignored --nocapture` (P11_FROM=<level> to start later).
#![allow(dead_code)]

/// GB2+0xc 0x8000000 / 0x4000000 are set for real by Coll SetBit: L02B collectable 0x2a0000f3 (1/0x8000000)
/// and L02A collectable 0x2a0000b9 (1/0x4000000; WP-P13). 0xb9 (with 0xb8, 0x49) sits unplaced at the origin
/// headless, so the 2nd L02A visit touches it for 3 frames and warps back (no game-buffer write).
/// (level, steps, expected next level). Game buffers are carried level to level (C EFGameMemoryMgr).
pub const EP2_ROUTE: &[(&str, &str, &str)] = &[
    ("EP2/L01A", "f 60; warp 2a000122; x 600", "EP2/L01B"),
    ("EP2/L01B", "f 60; warp 2a0000dd; x 600", "EP2/L01C"),
    ("EP2/L01C", "f 60; m 2a00001a de 0; warp 2a00001a; f 30; warp 2a00001a; f 200", "EP2/L02A"),
    ("EP2/L02A", "f 60; warp 2a000112; x 600", "EP2/L02B"),
    ("EP2/L02B", "f 60; pos 2a0000f3; pos 2a0000f5; pl; c 2a0000f3; gd 2 10; c 2a0000f5; gd 2 10; f 200; warp 2a000005; x 600", "EP2/L02C"),
    ("EP2/L02C", "f 60; c 2a0000cc; gd 2 10; f 200; warp 2a000010; x 600", "EP2/L02B"),
    ("EP2/L02B", "f 60; warp 2a000004; x 600", "EP2/L02A"),
    ("EP2/L02A", "f 60; gd 2 10; warp 2a0000b9; f 3; wxyz 133.75 0.5 94.8; f 30; gd 2 10; pl; wxyz 104.76 5.7 91.5; f 20; x 200; f 600", "EP2/L03A"),
    ("EP2/L03A", "f 60; warp 2a000109; x 600", "EP2/L04"),
    // FAKED (documented): the 3-lap race is not driven; launch_control L+0x6c (laps done) set + 0x4f (R7 recipe)
    ("EP2/L04", "f 60; l @launch_control 6c 1; m @launch_control 4f 0; f 60", "STATUS"),
];

/// not a real binary: cargo auto-discovers every src/bin/*.rs as a bin target
fn main() {}

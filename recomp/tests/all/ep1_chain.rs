//! WP-P18: Episode 1 end to end. Runs the game binary (`--route ep1`: the built-in per-level routes
//! chained through the real level loads) from EP1/L01 and asserts every level switch, in order; the
//! game is killed once the expected switches were seen.
//! Ignored (long: ~10 min per level). `cargo test --release --features game --test ep1_chain -- --ignored`.
//! EP1_CHAIN_UPTO=<n> asserts only the first n switches (default all).
#![cfg(feature = "game")]

use std::io::BufRead;

use crate::common::ISO;
/// WP-P20 R12: + the real EP1 end menus (STATUS cross presses, game.rs menu_cross) into EP2/L01A
const CHAIN: [&str; 10] = ["EP1/L02", "EP1/L03A", "EP1/L03B", "EP1/L04", "LOADER", "STATUS", "SPLASH", "LOADER", "LOADER", "EP2/L01A"];

#[test]
#[ignore]
fn ep1_chain() {
    if !crate::common::iso_or_skip("ep1_chain") {
        return;
    }
    let n = std::env::var("EP1_CHAIN_UPTO").ok().and_then(|v| v.parse().ok()).unwrap_or(CHAIN.len());
    // P20 R127: fresh memory card (a save left by an earlier run changes the STATUS save menu: the X-only
    // menu_cross presses then never reach SPLASH)
    let card = std::env::temp_dir().join(format!("scooby_ep1_card_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&card);
    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_game"))
        .env("GAME_SAVEDIR", &card)
        .args(["EP1/L01", "--headless", "--autoplay", "--route", "ep1", "--frames", "120000", "--iso", ISO])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("run game");
    let mut got = Vec::new();
    for l in std::io::BufReader::new(child.stderr.take().unwrap()).lines().map_while(Result::ok) {
        if let Some(d) = l.strip_prefix("game: level switch -> ") {
            eprintln!("{l}");
            got.push(d.split(' ').next().unwrap().to_string());
            if got.len() >= n {
                break;
            }
        }
    }
    let _ = child.kill();
    assert!(got.len() >= n && got[..n] == CHAIN[..n], "ep1 chain {got:?}, expected {:?}", &CHAIN[..n]);
}

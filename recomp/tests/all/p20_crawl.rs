//! WP-P20 R19: per-build hero crouch anim ids (hero_crouch.rs rel): from the real EP1-end carry, EP2/L01A,
//! the bot walks to the beam at 257.8,282.1, presses X (crawl, heroInit_CrouchIn anim 0x2a in build 2a9862df)
//! and must come out north of it (the old group id 0x33 made StartAnim fail -> stuck in CrouchIn forever).
//! R27: CrouchOut is 0x2b in this build (not In - 1): the crawl now ends inside door 0x125 -> pocket.
#![cfg(feature = "game")]

use crate::common::ISO;

#[test]
#[ignore]
fn p20_ep2_l01a_crawl() {
    if !crate::common::iso_or_skip("p20_ep2_l01a_crawl") {
        return;
    }
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_game"))
        .args(["EP2/L01A", "--headless", "--autoplay", "--frames", "1300", "--route", "248.3,299,D;248,264,D;256,263.5,D;258.2,282.4;258.9,285.6,J", "--iso", ISO])
        .env("GAME_POSLOG", "2a00004d")
        .env("GAME_CARRY", concat!(env!("CARGO_MANIFEST_DIR"), "/tests/all/data/ep1_end_EP2L01A.bin"))
        .stdout(std::process::Stdio::null())
        .output()
        .expect("run game");
    let err = String::from_utf8_lossy(&out.stderr);
    // R27: crawling INTO door 0x125 (GB0+0x34 0x1000 window during CrouchOut, anim 0x2b) takes Scooby to the
    // pocket (door 0x125 target 277.24,306.32)
    let pocket = err.lines().filter(|l| l.contains("K5 pos") && l.contains("0x2a00004d")).any(|l| {
        let v: Vec<f32> = l.split(['[', ']', ',']).filter_map(|t| t.trim().parse().ok()).collect();
        v.len() >= 3 && (v[v.len() - 3] - 277.0).abs() < 3.0 && (v[v.len() - 1] - 306.0).abs() < 3.0
    });
    assert!(pocket, "crawl into door 0x125 failed");
}

/// WP-P20 R48: cheap EP1/L04 segment of ep1_chain: from the chained L03B->L04 carry (c4), `--route l04`
/// must reach LOADER (R9: f~12400). Ignored (~4 min). R50: carry = the real chained L03B->L04 dump (ep1_chain_EP1L04.bin); green after the F power threshold 70 -> 130. R48 note: it failed with and without GetBone
/// (GAME_NO_GETBONE=1 gives the identical run): the F/B ghost phase diverges from R9 at wp 16 (frame ~1732).
#[test]
#[ignore]
fn p20_ep1_l04_segment() {
    if !crate::common::iso_or_skip("p20_ep1_l04_segment") {
        return;
    }
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_game"))
        .args(["EP1/L04", "--headless", "--autoplay", "--frames", "16000", "--route", "l04", "--iso", ISO])
        .env("GAME_CARRY", concat!(env!("CARGO_MANIFEST_DIR"), "/tests/all/data/ep1_chain_EP1L04.bin"))
        .stdout(std::process::Stdio::null())
        .output()
        .expect("run game");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("game: level switch -> LOADER"), "EP1/L04 segment did not reach LOADER");
}

/// P20 R52 regression: EFInRangeIdRetriever (0xff000080). EP2/L01B poltergeist 0xf3 must choose a
/// throwable (L+0x184 = a 0x2a0000f5..fd object) via Poltergeist_BaseFUNC_ChooseThrowable; before
/// the retriever existed it stayed 0x7ffffffe and the ghost never moved. ~15 s.
#[test]
fn p20_ep2_l01b_poltergeist_chooses_throwable() {
    if !crate::common::iso_or_skip("p20_ep2_l01b_poltergeist_chooses_throwable") {
        return;
    }
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_game"))
        .args(["EP2/L01B", "--headless", "--autoplay", "--frames", "600", "--route", "1121.7,274.4,G", "--iso", ISO])
        .env("GAME_CARRY", concat!(env!("CARGO_MANIFEST_DIR"), "/tests/all/data/ep2_L01A_end_EP2L01B.bin"))
        .env("GAME_DEBUG", "1")
        .env("GAME_WATCH", "2a0000f3")
        .env("GAME_WATCHL", "106")
        .stdout(std::process::Stdio::null())
        .output()
        .expect("run game");
    let err = String::from_utf8_lossy(&out.stderr);
    let chose = err.lines().filter(|l| l.contains("watchl")).any(|l| {
        let w: Vec<&str> = l.split('[').nth(1).unwrap_or("").trim_end_matches(']').split(", ").collect();
        w.get(97).is_some_and(|s| s.trim_start_matches("0x").starts_with("2a0000f"))
    });
    assert!(chose, "poltergeist 0xf3 never chose a throwable");
}

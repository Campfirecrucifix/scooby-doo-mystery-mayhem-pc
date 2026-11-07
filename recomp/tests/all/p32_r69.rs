//! P32 R69: EP3/L04 route GAME_LOGICCLIP-robust (dustdevils 0x152/0x153 frozen off-view under clip, as C ClipGameObjects).
use crate::common::{ISO, load_carried, load_dump};
use crate::engine_p17::run;
use crate::p32_r59::routes_ep3;
use scooby_recomp::engine::iso::IsoFs;

/// In-process child (GAME_LOGICCLIP is process-wide): EP3/L04 route from the default chain's L04 entry carry reaches EP3/L05.
#[test]
#[ignore]
fn p32_r69_clip_child_ep3_l04() {
    if std::env::var("GAME_LOGICCLIP").as_deref() != Ok("1") { return }
    crate::common::install_crash_handler();
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let _card = crate::engine_p17::CARD.lock().unwrap_or_else(|e| e.into_inner());
    let bufs = load_dump(&format!("{}/tests/all/data/p32_r69_EP3L04_chain.bin", env!("CARGO_MANIFEST_DIR")));
    let r = routes_ep3::EP3_ROUTE.iter().find(|r| r.0 == "EP3/L04").unwrap().1;
    let (mut e, mut rt) = load_carried(&fs, "EP3/L04", &bufs).unwrap();
    let r = run(&mut e, &mut rt, &crate::engine_p17::route_steps("EP3/L04", r));
    println!("R69 EP3/L04 -> {r:?}");
    assert!(r.is_some_and(|(l, _)| l.eq_ignore_ascii_case("EP3/L05")), "clip EP3/L04 did not reach EP3/L05");
}

/// Re-runs the clip child in a child process with GAME_LOGICCLIP=1 (default mode = the p17_chains EP3 chain).
#[test]
fn p32_r69_clip_ep3_l04() {
    let Ok(_) = IsoFs::open(ISO) else { return };
    let t = "p32_r69::p32_r69_clip_child_ep3_l04";
    let o = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", t, "--include-ignored", "--nocapture", "--test-threads=1"])
        .env("GAME_LOGICCLIP", "1")
        .output()
        .unwrap();
    let out = String::from_utf8_lossy(&o.stdout);
    println!("{}", out.lines().filter(|l| l.starts_with("R69")).collect::<Vec<_>>().join("\n"));
    assert!(o.status.success() && out.contains("1 passed"), "{t} under GAME_LOGICCLIP=1 failed:\n{out}\n{}", String::from_utf8_lossy(&o.stderr));
}

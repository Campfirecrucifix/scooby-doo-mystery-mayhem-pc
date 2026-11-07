//! P32 R68: GAME_LOGICCLIP=1 (game.exe's interactive default) late-episode real play: EP3/L05 -> LOADER from the
//! default chain's L05 entry carry, and EP6/L05B (final boss) -> LOADER -> STATUS -> CREDITS -> INTRO (p32_r61).
use crate::common::{ISO, load_carried, load_dump};
use crate::engine_p17::run;
use crate::p32_r59::routes_ep3;
use scooby_recomp::engine::iso::IsoFs;

/// In-process child (GAME_LOGICCLIP is process-wide): EP3/L05 route from the chain entry carry reaches LOADER.
#[test]
#[ignore]
fn p32_r68_clip_child_ep3_l05() {
    if std::env::var("GAME_LOGICCLIP").as_deref() != Ok("1") { return }
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let _card = crate::engine_p17::CARD.lock().unwrap_or_else(|e| e.into_inner());
    let bufs = load_dump(&format!("{}/tests/all/data/p32_r68_EP3L05_chain.bin", env!("CARGO_MANIFEST_DIR")));
    let r = routes_ep3::EP3_ROUTE.iter().find(|r| r.0 == "EP3/L05").unwrap().1;
    let (mut e, mut rt) = load_carried(&fs, "EP3/L05", &bufs).unwrap();
    let r = run(&mut e, &mut rt, &crate::engine_p17::route_steps("EP3/L05", r));
    println!("R68 EP3/L05 -> {r:?}");
    assert!(r.is_some_and(|(l, _)| l.eq_ignore_ascii_case("LOADER")), "clip EP3/L05 did not reach LOADER");
}

/// Re-runs the clip children in a child process with GAME_LOGICCLIP=1.
#[test]
fn p32_r68_clip_ep3_l05_ep6_end_to_credits() {
    let Ok(_) = IsoFs::open(ISO) else { return };
    for t in ["p32_r68::p32_r68_clip_child_ep3_l05", "p32_r61::p32_r61_ep6_end_to_credits"] {
        let o = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", t, "--include-ignored", "--nocapture", "--test-threads=1"])
            .env("GAME_LOGICCLIP", "1")
            .output()
            .unwrap();
        let out = String::from_utf8_lossy(&o.stdout);
        println!("{}", out.lines().filter(|l| l.starts_with("R6")).collect::<Vec<_>>().join("\n"));
        assert!(o.status.success() && out.contains("1 passed"), "{t} under GAME_LOGICCLIP=1 failed:\n{out}\n{}", String::from_utf8_lossy(&o.stderr));
    }
}

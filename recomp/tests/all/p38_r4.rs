//! P38 R4: EP6/L04A exit under GAME_LOGICCLIP=1 (game.exe's default). With logic clipping the first book captures
//! run at other frames (C ClipGameObjects timing), a monster touch then scares the leader back to the level start
//! (~1306,208, courage 1.0, captures kept) and the clip-off route walked on from there into the wrong rooms. The
//! clip-only (`iflc`) recovery steps in routes_animsnd EP6_L04A_ANIMSND walk back to the current room
//! (`ifx L 1280 1340`) and retry the room's uncaptured ghosts (`ifcap UID 0`); the clip-off steps are unchanged.
use crate::common::{load_carried, load_dump, ISO};
use crate::engine_p17::{route_steps, routes_ep6, run};
use scooby_recomp::engine::iso::IsoFs;

fn l04a() -> Option<(String, u32)> {
    let fs = IsoFs::open(ISO).ok()?;
    let bufs = load_dump(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/all/data/p38_r4_EP6L04A.bin"));
    let (mut e, mut rt) = load_carried(&fs, "EP6/L04A", &bufs).unwrap();
    let steps = routes_ep6::EP6_ROUTE.iter().find(|r| r.0 == "EP6/L04A").unwrap().1;
    run(&mut e, &mut rt, &route_steps("EP6/L04A", steps))
}

#[test]
#[ignore]
fn p38_r4_clip_child_ep6_l04a() {
    if std::env::var("GAME_LOGICCLIP").as_deref() != Ok("1") { return }
    let Ok(_) = IsoFs::open(ISO) else { return };
    let r = l04a();
    println!("P38 R4 clip EP6/L04A {r:?}");
    assert!(r.as_ref().is_some_and(|r| r.0.eq_ignore_ascii_case("EP6/L04B")), "clip EP6/L04A: {r:?}");
}

/// GAME_LOGICCLIP is process-wide: the clip-on run is a child process.
/// P38 R5: ignored until the clip-on recovery route has run green once (the shared headless slot was held by the
/// driver gate for the whole round); run with `--include-ignored p38_r4`.
#[test]
#[ignore = "P38 R5: EP6/L04A clip-on recovery route not yet verified"]
fn p38_r4_ep6_l04a_exit_clip_on() {
    let Ok(_) = IsoFs::open(ISO) else { return };
    let o = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "p38_r4::p38_r4_clip_child_ep6_l04a", "--include-ignored", "--nocapture", "--test-threads=1"])
        .env("GAME_LOGICCLIP", "1")
        .output()
        .unwrap();
    let out = String::from_utf8_lossy(&o.stdout);
    assert!(o.status.success() && out.contains("1 passed"), "clip child failed:\n{}\n{}", out.lines().rev().take(40).collect::<Vec<_>>().join("\n"), String::from_utf8_lossy(&o.stderr));
}

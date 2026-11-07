//! P32 R90: EP4/L04 (zombie boss) exit under GAME_LOGICCLIP=1 (game.exe's interactive default).
//! The level controller gamecontroller__77b9d190 (GameController.c FUNC_MessageHandler) counts zombie captures
//! (0xd5 from the book, param != 0x2a0000bc); the 5th parks the three zombies and sends 0xde 0 to the zombieboss
//! 0x2a0000bc, which leaves its seat; a zombie catching the hero broadcasts 0xc9 (gamecontrollerFUNC_Reset, count 0).
//! The harness `bookzg` farming route gets caught before the 5th capture with clip on (the RNG-picked truck wisp
//! sizes differ: trucks 0x2a0000c4/c5 start 4 frames later while off-view, as C ClipGameObjects); the boss phase
//! and the level exit itself are clip-independent: 5 test-sent 0xd5 captures -> boss hook/drain -> LOADER, clip on and off.
use crate::common::{load_carried, load_dump, ISO};
use crate::engine_p17::run;
use scooby_recomp::engine::iso::IsoFs;

fn boss_exit() -> Option<(String, u32)> {
    let fs = IsoFs::open(ISO).ok()?;
    let bufs = load_dump(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/all/data/p32_r59_EP4L04.bin"));
    let (mut e, mut rt) = load_carried(&fs, "EP4/L04", &bufs).unwrap();
    let sp = e.mem.sp();
    let r = run(&mut e, &mut rt, &format!("f 60; goto 302 205 1500; f 2460; {}f 300; bookzg 18000 2 2a0000bc; f 1500", "m 2a0000c7 d5 2a0000b6; ".repeat(5)));
    // the harness lookups (common::pos for frame-less uids) leave the module stack where it was
    assert_eq!(e.mem.sp(), sp, "module stack leaked");
    r
}

#[test]
#[ignore]
fn p32_r90_clip_child_ep4_l04_boss_exit() {
    if std::env::var("GAME_LOGICCLIP").as_deref() != Ok("1") { return }
    let Ok(_) = IsoFs::open(ISO) else { return };
    let r = boss_exit();
    println!("R90 clip EP4/L04 {r:?}");
    assert!(r.as_ref().is_some_and(|r| r.0 == "LOADER"), "clip EP4/L04 boss exit: {r:?}");
}

/// Clip off in process, clip on in a child process (GAME_LOGICCLIP is process-wide).
#[test]
fn p32_r90_ep4_l04_boss_exit_clip_on_off() {
    let Ok(_) = IsoFs::open(ISO) else { return };
    if std::env::var("GAME_LOGICCLIP").as_deref() != Ok("1") {
        let r = boss_exit();
        assert!(r.as_ref().is_some_and(|r| r.0 == "LOADER"), "EP4/L04 boss exit: {r:?}");
    }
    let o = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "p32_r90::p32_r90_clip_child_ep4_l04_boss_exit", "--include-ignored", "--nocapture", "--test-threads=1"])
        .env("GAME_LOGICCLIP", "1")
        .output()
        .unwrap();
    let out = String::from_utf8_lossy(&o.stdout);
    assert!(o.status.success() && out.contains("1 passed"), "clip child failed:\n{out}\n{}", String::from_utf8_lossy(&o.stderr));
}

//! P32 R93: EP4/L04 zombie capture phase + boss, natural play (no test-sent captures) under GAME_LOGICCLIP=1
//! (game.exe's default; C ClipGameObjects always runs). The truck wisp sizes come from IntRandom(1,100) on the
//! shared libc rand() (truckFUNC_DropWisp), so the phase's difficulty depends on the rand() state at the level
//! entry, which on the PS2 depends on everything played before. R91's "clip on 0/7" was one rand() sample
//! (start timing does not change the drop sequence); `srand N` samples other entry states: clip on, 11 states
//! -> 5 pass the 5-capture phase, 2 beat the boss (999, 31337) with the harness `bookzg` bot; clip off 3/5.
use crate::common::{load_carried, load_dump, ISO};
use crate::engine_p17::run;
use scooby_recomp::engine::iso::IsoFs;

#[test]
#[ignore]
fn p32_r93_clip_child_ep4_l04_natural() {
    if std::env::var("GAME_LOGICCLIP").as_deref() != Ok("1") { return }
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let bufs = load_dump(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/all/data/p32_r59_EP4L04.bin"));
    let (mut e, mut rt) = load_carried(&fs, "EP4/L04", &bufs).unwrap();
    let r = run(&mut e, &mut rt, "f 60; srand 999; goto 302 205 1500; f 2460; bookzg 14000 2 2a0000b6 2a0000b8 2a0000ba 2a0000bc; f 600");
    println!("R93 clip EP4/L04 natural {r:?}");
    assert!(r.as_ref().is_some_and(|r| r.0 == "LOADER"), "clip EP4/L04 natural play: {r:?}");
}

/// The clip-on run in a child process (GAME_LOGICCLIP is process-wide).
#[test]
fn p32_r93_ep4_l04_natural_clip_on() {
    let Ok(_) = IsoFs::open(ISO) else { return };
    let o = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "p32_r93::p32_r93_clip_child_ep4_l04_natural", "--include-ignored", "--nocapture", "--test-threads=1"])
        .env("GAME_LOGICCLIP", "1")
        .output()
        .unwrap();
    let out = String::from_utf8_lossy(&o.stdout);
    assert!(o.status.success() && out.contains("1 passed"), "clip child failed:\n{out}\n{}", String::from_utf8_lossy(&o.stderr));
}

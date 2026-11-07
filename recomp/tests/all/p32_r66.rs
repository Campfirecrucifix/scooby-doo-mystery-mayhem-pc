//! P32 R66: GAME_LOGICCLIP=1 (game.exe's interactive default) EP1 end menus LOADER -> STATUS -> SPLASH -> EP2.
//! R65: SPLASH panicked "no game timer 0x0" (Splash_FadeInOut before Splash_CameraST_INIT created the timer).
use crate::common::{carry, load_carried, load_dump, ISO};
use crate::engine_p17::run;
use scooby_recomp::engine::iso::IsoFs;

/// The end menus from `bufs` at `lvl` until an EPx level loads; returns the visited (level, frame) list.
pub fn end_menus(fs: &IsoFs, mut lvl: String, mut bufs: Vec<(u32, Vec<u8>)>) -> Vec<(String, u32)> {
    let mut out = vec![];
    for _ in 0..8 {
        if lvl.to_ascii_uppercase().starts_with("EP") || lvl == "INTRO" { break }
        let (mut e, mut rt) = load_carried(fs, &lvl, &bufs).unwrap();
        // CREDITS: Panel.c scroll timer -> LoadLevel("INTRO") after 192 s (p32_r61)
        let steps = if lvl == "CREDITS" { "f 12000".to_string() } else { "f 282; s 80 ff 6; f 6; p 4000 6; ".repeat(30) };
        let (l, f) = run(&mut e, &mut rt, &steps).unwrap_or_else(|| panic!("{lvl}: no level request"));
        println!("R66 {lvl} -> {l} at f{f}");
        bufs = carry(&mut e);
        out.push((l.clone(), f));
        lvl = l;
    }
    out
}

/// In-process child: GAME_LOGICCLIP is a process-wide env switch, so the gate test re-runs this test binary with it set.
#[test]
#[ignore]
fn p32_r66_clip_child_ep1_end_menus() {
    if std::env::var("GAME_LOGICCLIP").as_deref() != Ok("1") { return }
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let bufs = load_dump(&std::env::var("R66CARRY").unwrap_or(format!("{}/tests/all/data/ep1_chain_EP1L04_LOADER.bin", env!("CARGO_MANIFEST_DIR"))));
    let v = end_menus(&fs, "LOADER".into(), bufs);
    assert!(v.last().is_some_and(|l| l.0.eq_ignore_ascii_case("EP2/L01A")), "clip EP1 end menus: {v:?}");
}

/// The episode-end menus (R66LVL, default SPLASH,STATUS) from each episode-end carry under GAME_LOGICCLIP=1 (the
/// carries are past the end menus, so each runs one episode further: EP2..EP4 STATUS -> SPLASH -> LOADER -> next EP,
/// episode 6 STATUS -> CREDITS -> INTRO; the episode-6 SPLASH wraps to EP1/L01, unreachable in play).
#[test]
#[ignore]
fn p32_r66_clip_child_menus() {
    if std::env::var("GAME_LOGICCLIP").as_deref() != Ok("1") { return }
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let lvls = std::env::var("R66LVL").unwrap_or("SPLASH,STATUS".into());
    for c in ["ep1_end_EP2L01A", "ep2_end_EP3L01", "ep3_end_EP4L01", "ep4_end_EP6L01"] {
        for lvl in lvls.split(',') {
            let bufs = load_dump(&format!("{}/tests/all/data/{c}.bin", env!("CARGO_MANIFEST_DIR")));
            let v = end_menus(&fs, lvl.into(), bufs);
            println!("R66M {c} {lvl}: {v:?}");
            assert!(v.last().is_some_and(|l| l.0.to_ascii_uppercase().starts_with("EP") || l.0 == "INTRO"), "clip {c} {lvl}: {v:?}");
        }
    }
}

/// GAME_LOGICCLIP=1 (game.exe's interactive default) is process-wide: re-run the clip children in a child process.
#[test]
fn p32_r66_clip_splash_status_menus() {
    let Ok(_) = IsoFs::open(ISO) else { return };
    for t in ["p32_r66::p32_r66_clip_child_ep1_end_menus", "p32_r66::p32_r66_clip_child_menus"] {
        let o = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", t, "--include-ignored", "--nocapture", "--test-threads=1"])
            .env("GAME_LOGICCLIP", "1")
            .output()
            .unwrap();
        let out = String::from_utf8_lossy(&o.stdout);
        println!("{}", out.lines().filter(|l| l.starts_with("R66")).collect::<Vec<_>>().join("\n"));
        assert!(o.status.success() && out.contains("1 passed"), "{t} under GAME_LOGICCLIP=1 failed:\n{out}\n{}", String::from_utf8_lossy(&o.stderr));
    }
}

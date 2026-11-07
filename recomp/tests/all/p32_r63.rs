//! P32 R63: real title New Game -> EP01 splash -> EP1/L01 (splash-seen bit); bonus L09 unlock (STATUS ingredients) ->
//! title Extras -> EP1/L09 -> minigame end (lives out) -> scoreboard -> play again.
use crate::common::{ISO, carry, load_carried};
use crate::engine_p17::run;
use scooby_recomp::engine::iso::IsoFs;

type Eng = scooby_recomp::engine::Engine;
fn gbw(e: &Eng, b: u32, o: u32) -> u32 { e.rt.game_buffers.get(&b).map_or(0xdead, |p| e.mem.r32(p.0 + o)) }

/// INTRO -> Start -> X on New Game -> SPLASH (EP01) -> LOADER -> EP1/L01: the Splash Controller sets the EP1
/// "splash seen" bit (save word GB2+0x28 bit 0, which gates the title Extras EP1 rerun, Panel 0x3e) as in the C.
#[test]
#[ignore]
fn p32_r63_newgame_sets_ep1_splash_bit() {
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let (mut e, mut rt) = load_carried(&fs, "INTRO", &[]).unwrap();
    let r = run(&mut e, &mut rt, "f 100; p 8 3; f 100; p 4000 3; f 600");
    assert_eq!(r.as_ref().map(|r| r.0.as_str()), Some("SPLASH"), "title New Game");
    let (mut bufs, mut last) = (carry(&mut e), "SPLASH".to_string());
    // the real menus (SPLASH/LOADER, X presses as game.rs menu_cross) into EP1/L01
    for _ in 0..8 {
        if last.to_ascii_uppercase().starts_with("EP") { break }
        let (mut e, mut rt) = load_carried(&fs, &last, &bufs).unwrap();
        let (l, f) = run(&mut e, &mut rt, &"f 282; s 80 ff 6; f 6; p 4000 6; ".repeat(30)).unwrap_or_else(|| panic!("{last}: no level request"));
        println!("R63NG {last} -> {l} at f{f} GB2+0x28 {:#x}", gbw(&e, 2, 0x28));
        bufs = carry(&mut e);
        last = l;
    }
    assert!(last.eq_ignore_ascii_case("EP1/L01"), "New Game led to {last}");
    let (mut e, mut rt) = load_carried(&fs, "EP1/L01", &bufs).unwrap();
    assert_eq!(run(&mut e, &mut rt, "f 60"), None);
    let w = gbw(&e, 2, 0x28);
    println!("R63NG EP1/L01 GB2+0x28 {w:#x}");
    assert_eq!(w & 1, 1, "EP1 splash-seen bit not set after a title New Game");
}

fn patch(b: &mut [(u32, Vec<u8>)], id: u32, o: usize, f: impl Fn(u32) -> u32) {
    let v = &mut b.iter_mut().find(|g| g.0 == id).unwrap().1;
    let w = f(u32::from_le_bytes(v[o..o + 4].try_into().unwrap()));
    v[o..o + 4].copy_from_slice(&w.to_le_bytes());
}

/// EP1 end STATUS with all 5 EP1 ingredients (GB2+0xc & 0x1f == 0x1f, GB0+0x18 episode 1): STPanelFUNC_Increase_Ingredients
/// sets GB2 word0 0x1000 (EP1 bonus unlock). Then the real flow SPLASH -> LOADER -> EP2/L01A -> pause quit -> title
/// Extras (main item 24) -> Mini Games (36) -> 43 (EP1 bonus, panel_baseFUNC_MiniGameSearch skips locked games)
/// -> X loads EP1/L09 (PanelCB_INPUT_X case 43 gated on GBP[3] = GB2 word0 & 0x1000), as the C. Without the
/// ingredients the bit stays clear and X on the menu does not load.
#[test]
#[ignore]
fn p32_r63_bonus_unlock_status_to_title_extras() {
    let Ok(fs) = IsoFs::open(ISO) else { return };
    for full in [true, false] {
        let mut bufs = crate::common::load_dump(&format!("{}/tests/all/data/ep1_end_EP2L01A.bin", env!("CARGO_MANIFEST_DIR")));
        patch(&mut bufs, 0, 0x18, |_| 1);
        patch(&mut bufs, 2, 0, |w| w & !0x1f000);
        patch(&mut bufs, 2, 0xc, |w| if full { w | 0x1f } else { (w & !0x1f) | 0xf });
        let mut last = "STATUS".to_string();
        for _ in 0..8 {
            if last.to_ascii_uppercase().starts_with("EP") { break }
            let (mut e, mut rt) = load_carried(&fs, &last, &bufs).unwrap();
            let (l, f) = run(&mut e, &mut rt, &"f 282; s 80 ff 6; f 6; p 4000 6; ".repeat(30)).unwrap_or_else(|| panic!("{last}: no level request"));
            println!("R63B {full} {last} -> {l} at f{f} GB2+0 {:#x}", gbw(&e, 2, 0));
            bufs = carry(&mut e);
            last = l;
        }
        assert!(last.eq_ignore_ascii_case("EP2/L01A"), "EP1 end menus led to {last}");
        let (mut e, mut rt) = load_carried(&fs, &last, &bufs).unwrap();
        let r = run(&mut e, &mut rt, "f 300; pausequit");
        assert_eq!(r.as_ref().map(|r| r.0.as_str()), Some("INTRO"), "pause quit");
        assert_eq!(gbw(&e, 2, 0) & 0x1000 != 0, full, "EP1 bonus unlock bit");
        let bufs = carry(&mut e);
        let (mut e, mut rt) = load_carried(&fs, "INTRO", &bufs).unwrap();
        let r = run(&mut e, &mut rt, "f 300; s 80 ff 10; f 30; s 80 ff 10; f 30; s 80 ff 10; f 30; p 4000 3; f 200; s 80 ff 10; f 30; p 4000 3; f 200; s 80 ff 10; f 30; p 4000 3; f 300");
        println!("R63B {full} title Extras > Mini Games > X -> {r:?}");
        if !full { assert_eq!(r, None, "locked bonus must not load"); continue }
        assert_eq!(r.as_ref().map(|r| r.0.as_str()), Some("EP1/L09"), "unlocked EP1 bonus must load");
        let bufs = carry(&mut e);
        let (mut e, mut rt) = load_carried(&fs, "EP1/L09", &bufs).unwrap();
        // idle: the ghosts take the 3 lives (inv_controlCB_TIMER_INITIALIZE: lives +52 == 0 -> inv_controlFUNC_EndMinigame
        // -> msg 209 to the scoreboard 0x2a0000bc), board shown; X = play again (scoreboardST_LEAVING_LEVEL loads the
        // GB0+0x7e0/0x800 level the title panel copied, panel_baseFUNC_PlayMiniGame) -> EP1/L09 again
        let r = run(&mut e, &mut rt, "f 12000; p 4000 3; f 300; p 4000 3; f 300; p 4000 3; f 600");
        println!("R63B EP1/L09 -> {r:?}");
        assert!(matches!(&r, Some((l, f)) if l == "EP1/L09" && *f < 13500), "EP1/L09 end -> {r:?}");
    }
}


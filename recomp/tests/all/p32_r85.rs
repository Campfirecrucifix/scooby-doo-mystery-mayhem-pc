//! P32 R85: pause -> Game Options -> (host PC Settings page) -> back -> resume, as the game sees it. While the
//! host page (bin/game_settings page_input) is open or its close button is still held the game reads an idle
//! pad, so here the page = idle frames. The panel must stay on the Options page and the same row, Triangle must
//! return to the pause menu on the same row as without the page, Continue (the first row after Back, as in C) must resume with the hero where it was.
use crate::common::{ISO, frame_stick, leader, load_carried, pos};
use crate::engine_p17::run;
use scooby_recomp::engine::iso::IsoFs;
use scooby_recomp::modules::panel::{OPTIONS_OPEN, PC_PAGE_ROW, menu_row};
use std::sync::atomic::Ordering::Relaxed;

#[test]
fn p32_r85_pc_page_roundtrip() {
    let _g = crate::common::serial();
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let (mut e, mut rt) = load_carried(&fs, "EP1/L01", &[]).unwrap();
    assert!(run(&mut e, &mut rt, "f 200; p 4000 3; f 200").is_none());
    let hero = leader(&e);
    let p0 = pos(&mut e, &rt, hero).unwrap();
    // Start -> pause menu, down to Game Options, X, Triangle: the C Back row (no PC page)
    assert!(run(&mut e, &mut rt, "p 8 3; f 100; s 80 ff 10; f 30; p 4000 3; f 100").is_none());
    assert!(OPTIONS_OPEN.load(Relaxed), "Game Options page not open");
    let opt_row0 = menu_row(&e).expect("panel_baseST_OPTIONS did not run");
    assert!(run(&mut e, &mut rt, "p 1000 3; f 200").is_none());
    assert!(!OPTIONS_OPEN.load(Relaxed));
    let go_row = menu_row(&e).unwrap();
    // C: Triangle = panel_baseCB_OPTIONS_CANCEL -> DrawNewMenu(7, 9, 0xc): pause menu, cursor on its first row (Continue)
    assert_eq!(go_row, 9, "Back did not reset the pause cursor to Continue");
    // again: down to Game Options, X, one row down, then the host page is open for 300 frames (game: idle pad)
    assert!(run(&mut e, &mut rt, "s 80 ff 10; f 30; p 4000 3; f 100").is_none());
    assert!(run(&mut e, &mut rt, "s 80 ff 10; f 30").is_none());
    assert!(OPTIONS_OPEN.load(Relaxed), "Game Options page not open (2)");
    let opt_row = menu_row(&e).unwrap();
    assert_ne!(opt_row, opt_row0, "stick down did not move the options row");
    PC_PAGE_ROW.store(2, Relaxed);
    assert!(run(&mut e, &mut rt, "f 300").is_none());
    PC_PAGE_ROW.store(-1, Relaxed);
    assert!(run(&mut e, &mut rt, "f 30").is_none());
    assert!(OPTIONS_OPEN.load(Relaxed), "panel left Options while the PC page was open");
    assert_eq!(menu_row(&e), Some(opt_row), "options row moved under the PC page");
    // Triangle: back on the pause menu, same row as without the page
    assert!(run(&mut e, &mut rt, "p 1000 3; f 100").is_none());
    assert!(!OPTIONS_OPEN.load(Relaxed));
    assert_eq!(menu_row(&e), Some(go_row), "pause menu row after Back");
    // Continue: the hero has not moved during the pause, and moves again after it
    assert!(run(&mut e, &mut rt, "p 4000 3; f 60").is_none());
    let p1 = pos(&mut e, &rt, hero).unwrap();
    println!("P85 rows go {go_row} opt {opt_row0}->{opt_row}; hero {p0:?} -> {p1:?}");
    assert!((0..3).all(|i| (p0[i] - p1[i]).abs() < 0.05), "hero moved during the pause: {p0:?} -> {p1:?}");
    for _ in 0..60 {
        frame_stick(&mut e, &mut rt, 0, (128, 0));
    }
    let p2 = pos(&mut e, &rt, hero).unwrap();
    assert!((0..3).map(|i| (p2[i] - p1[i]).abs()).sum::<f32>() > 0.5, "game did not resume: {p1:?} -> {p2:?}");
}

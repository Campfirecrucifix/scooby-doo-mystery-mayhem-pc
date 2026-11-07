//! P32 R54: pause -> Game Options -> "Back" must clear the port-side OPTIONS_OPEN flag. Only shape group 9 of
//! panel_baseFUNC_OptionsMenuConfirm also does SetNextState(SELF, 4) after Back; the other builds return to the
//! pause main menu without a state change, so the flag stayed set and the host kept drawing its extra
//! "GLOBAL RAY TRACING (R)" row (and taking R) on the pause menu and in game after Continue.
use crate::common::{ISO, load_carried};
use crate::engine_p17::run;
use scooby_recomp::engine::iso::IsoFs;
use scooby_recomp::modules::panel::OPTIONS_OPEN;
use std::sync::atomic::Ordering::Relaxed;

#[test]
fn p32_r54_options_back_clears_flag() {
    let _g = crate::common::serial(); // OPTIONS_OPEN is a process global shared with p32_r85
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let (mut e, mut rt) = load_carried(&fs, "EP1/L01", &[]).unwrap();
    // dismiss the tutorial text, Start -> pause menu, down to Game Options, X
    assert!(run(&mut e, &mut rt, "f 200; p 4000 3; f 200; p 8 3; f 100; s 80 ff 10; f 30; p 4000 3; f 100").is_none());
    assert!(OPTIONS_OPEN.load(Relaxed), "Game Options page not open");
    // camera / vibration / sound / back: 3x down, X on Back
    assert!(run(&mut e, &mut rt, "s 80 ff 10; f 30; s 80 ff 10; f 30; s 80 ff 10; f 30; p 4000 3; f 100").is_none());
    assert!(!OPTIONS_OPEN.load(Relaxed), "OPTIONS_OPEN leaked after Back");
    // Continue (up to the first row, X): back in game, flag still clear
    assert!(run(&mut e, &mut rt, "s 80 0 10; f 30; s 80 0 10; f 30; p 4000 3; f 100").is_none());
    assert!(!OPTIONS_OPEN.load(Relaxed));
    // P32 R55 (same test: OPTIONS_OPEN is a process global): Start -> Game Options -> Sound page (2x down, X): flag
    // clear; Triangle returns to the Game Options page (panel_baseCB_OPTIONS_CANCEL 0x1c branch): flag set again
    // (was cleared, the host row vanished); Triangle again -> pause menu: clear
    drop((e, rt));
    let (mut e, mut rt) = load_carried(&fs, "EP1/L01", &[]).unwrap();
    assert!(run(&mut e, &mut rt, "f 200; p 4000 3; f 200; p 8 3; f 100; s 80 ff 10; f 30; p 4000 3; f 100").is_none());
    assert!(OPTIONS_OPEN.load(Relaxed), "Game Options page not open (2)");
    assert!(run(&mut e, &mut rt, "s 80 ff 10; f 30; s 80 ff 10; f 30; p 4000 3; f 100").is_none());
    assert!(!OPTIONS_OPEN.load(Relaxed), "Sound page shows the options row");
    assert!(run(&mut e, &mut rt, "p 1000 3; f 100").is_none());
    assert!(OPTIONS_OPEN.load(Relaxed), "options row missing after Triangle on the Sound page");
    assert!(run(&mut e, &mut rt, "p 1000 3; f 100").is_none());
    assert!(!OPTIONS_OPEN.load(Relaxed), "OPTIONS_OPEN leaked after Triangle on Game Options");
}

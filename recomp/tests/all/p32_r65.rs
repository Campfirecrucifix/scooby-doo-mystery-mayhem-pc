//! P32 R65: EP2/L09 (inv_control 9989a482) ends after 3 lost rounds (CB_TIMER_COUNTDOWN 40 s -> PreEndLevel(1)
//! -> EndLevel lives-- -> CB_TIMER_INITIALIZE lives 0 -> EndMinigame), ~12.3k frames, as the C.
//! Scoreboard (ST_ACTIVE pages: 2 name entry -> X -> 1 -> X -> 3; on page 3 X = replay (+220), TRIANGLE = quit (+224)
//! -> ST_LEAVING_LEVEL -> LoadLevel INTRO).
use crate::common::ISO;
use scooby_recomp::engine::iso::IsoFs;
use crate::p32_r64::{bonus, intro_carry};

#[test]
#[ignore]
fn p32_r65_ep2_l09_idle_end_scoreboard_quit_to_title() {
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let base = intro_carry(&fs);
    let (lvl, r) = bonus(&fs, &base, 0x2000, "f 13000; p 4000 3; f 300; p 4000 3; f 300; p 1000 3; f 900");
    assert_eq!(lvl, "EP2/L09");
    let (to, f) = r.expect("EP2/L09: no level change after idle lose + scoreboard X, X, TRIANGLE");
    assert_eq!(to, "INTRO", "scoreboard page-3 TRIANGLE quits to title");
    assert!(f > 12000, "EP2/L09 ended early: f{f}");
}

//! P32 R62: bonus minigame levels (EPx/L09, unlocked in the title Extras by GB2 word0 0x1000..0x10000 =
//! all 5 ingredients of the episode, STPanelFUNC_Increase_Ingredients) load and run idle without panic/stall.
use crate::p32_r59::trace;

type Eng = scooby_recomp::engine::Engine;
fn nobj(e: &Eng) -> Vec<u32> { vec![e.rt.objects.iter().flatten().count() as u32] }

#[test]
#[ignore]
fn p32_r62_bonus_l09_levels_run() {
    for lv in ["EP1/L09", "EP2/L09", "EP3/L09", "EP4/L09A", "EP6/L09"] {
        let (res, log) = trace(None, lv, "", "f 3000", nobj);
        println!("R62L09 {lv} -> {res:?} objs {:?}", log.last().map(|l| l.1.clone()));
        if log.is_empty() { continue } // no ISO
        assert!(log.last().unwrap().1[0] > 50, "{lv}: level objects not spawned");
        assert_eq!(res, None, "{lv}: unexpected level change while idle");
    }
}

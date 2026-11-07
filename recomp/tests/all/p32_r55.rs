//! P32 R55: clue pickup. EP3/L03 coll_level3 (bb4c0e37) fcback_NOTIFY_COLLIDE_OBJ: the clue object 0x2a000061
//! sets GB2+0xc bit 0x80 (the STPanel clue count bit) and messages 0x2a10001f (0x33 + the clue msg), as the C.
use crate::common::{ISO, load_carried};
use crate::engine_p17::run;
use scooby_recomp::engine::iso::IsoFs;

#[test]
fn p32_r55_ep3_l03_clue_pickup_sets_bit() {
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let (mut e, mut rt) = load_carried(&fs, "EP3/L03", &[]).unwrap();
    let gb2 = |e: &scooby_recomp::engine::Engine| { let b = e.rt.game_buffers.get(&2).unwrap().0; e.mem.r32(b + 0xc) };
    assert!(run(&mut e, &mut rt, "f 200").is_none());
    assert_eq!(gb2(&e) & 0x3e0, 0, "clue bit before pickup");
    assert!(run(&mut e, &mut rt, "c 2a000061; f 60").is_none());
    assert_eq!(gb2(&e) & 0x3e0, 0x80, "EP3/L03 clue bit after pickup");
}

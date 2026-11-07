//! P32 R76: EP2/L09 (bonus 0x2000) ingredient 0x2a000073 floats at y ~3.2 over a counter; coll_level9 picks it up
//! when scooby (0x2a000009) is within the 3D proximity radius (1.3 - 0.1). Reachable as in C by jumping onto the
//! counter (leader y ~2.5): state 2 -> 4 (collected). Not a C divergence (R74/R75 OPEN row closed).
use crate::common::{ISO, carry, load_carried};
use crate::engine_p17::run;
use crate::p32_r64::{MENU, intro_carry};
use scooby_recomp::engine::iso::IsoFs;

#[test]
#[ignore]
fn p32_r76_ep2_l09_ingredient_jump_pickup() {
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let mut bufs = intro_carry(&fs);
    let v = &mut bufs.iter_mut().find(|g| g.0 == 2).unwrap().1;
    let w = (u32::from_le_bytes(v[0..4].try_into().unwrap()) & !0x1f000) | 0x2000;
    v[0..4].copy_from_slice(&w.to_le_bytes());
    let (mut e, mut rt) = load_carried(&fs, "INTRO", &bufs).unwrap();
    let (lvl, _) = run(&mut e, &mut rt, MENU).expect("menu X loaded nothing");
    assert_eq!(lvl, "EP2/L09");
    let bufs = carry(&mut e);
    let (mut e, mut rt) = load_carried(&fs, &lvl, &bufs).unwrap();
    let st = |e: &mut scooby_recomp::engine::Engine| scooby_recomp::engine::core::apigameobj::GetCurrentStateID(e, 0x2a000073);
    assert_eq!(run(&mut e, &mut rt, "f 400"), None);
    assert_eq!(st(&mut e), 2, "ingredient not placed");
    assert_eq!(run(&mut e, &mut rt, "goto 282.78 229.31 300; goto 282.78 229.31 200 X; go 282.78 229.31 200; f 5"), None);
    assert_eq!(st(&mut e), 4, "ingredient not collected after the counter jump");
}

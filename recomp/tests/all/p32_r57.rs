//! P32 R57: on-foot death. EP1/L02, team mode "alone" (GB0+0x34 |= 1, as _MSG_TEAMMODE_SET _TEAMMODE_ALONE),
//! courage poked to 0.05 and the hero warped onto ghost 0x2a000021. The scare hit (heroCB_Scared, courage -> 0 with
//! GB0+0x34 & 0x101) takes heroInit_Dead (GB0+0x14 |= 0x8000000, hero parked off-world), heroST_Dead_Loop restores the
//! checkpoint (msg 0x7b on GB0+0x7c0; none passed = level start [32,0,-2.5]) and courage is refilled to 1.0, as the C.
//! Without the alone bit the same hit only panics (heroInit_PanicIn, 0x4000000), also as the C.
use crate::common::{ISO, leader, load_carried, pos};
use crate::engine_p17::run;
use scooby_recomp::engine::iso::IsoFs;

#[test]
fn p32_r57_ep1_l02_on_foot_death() {
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let (mut e, mut rt) = load_carried(&fs, "EP1/L02", &[]).unwrap();
    let cg = |e: &scooby_recomp::engine::Engine| { let b = e.rt.game_buffers.get(&1).unwrap().0; f32::from_bits(e.mem.r32(b)) };
    let f14 = |e: &scooby_recomp::engine::Engine| { let b = e.rt.game_buffers.get(&0).unwrap().0; e.mem.r32(b + 0x14) };
    assert!(run(&mut e, &mut rt, "f 200").is_none());
    let l = leader(&e);
    let start = pos(&mut e, &rt, l).unwrap();
    assert!(run(&mut e, &mut rt, "gor 0 34 1; g 1 0 3d4ccccd; warp 2a000021; f 1").is_none());
    let (mut dead, mut back) = (None, None);
    for i in 0..80 {
        assert!(run(&mut e, &mut rt, "f 15").is_none());
        if dead.is_none() && f14(&e) & 0x8000000 != 0 { dead = Some(i * 15); assert_eq!(cg(&e), 0.0) }
        let p = pos(&mut e, &rt, l).unwrap();
        if dead.is_some() && (p[0] - start[0]).abs() < 0.01 && (p[2] - start[2]).abs() < 0.01 { back = Some(i * 15); break }
    }
    println!("R57 dead {dead:?} respawn {back:?}");
    assert!(dead.is_some(), "ghost hit at courage 0.05 in alone mode kills (heroInit_Dead)");
    assert!(back.is_some(), "respawn at the level-start checkpoint");
    assert_eq!(cg(&e), 1.0, "courage refilled");
}

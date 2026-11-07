//! P32 R56: death headless. EP3/L03 is the mine-cart ride (leader = mcart 0x2a000004). Pick the clue 0x2a000061
//! (GB2+0xc |= 0x80) after the level-start checkpoint and poke courage (GB1+0) to 0: the next track hit
//! (mcart MessageHandler hit 16: courage -= 0.25, <= 0 -> mineCartFUNC_StopCart) crashes the cart ->
//! mineCartFUNC_RestartRide (param 0: courage = 1.0, speed 17.0) at the checkpoint, and the clue is rolled back
//! (coll 0xc1: bit cleared), as the C.
use crate::common::{ISO, leader, load_carried, pos};
use crate::engine_p17::run;
use scooby_recomp::engine::iso::IsoFs;

#[test]
fn p32_r56_ep3_l03_death_respawn() {
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let (mut e, mut rt) = load_carried(&fs, "EP3/L03", &[]).unwrap();
    let cg = |e: &scooby_recomp::engine::Engine| { let b = e.rt.game_buffers.get(&1).unwrap().0; f32::from_bits(e.mem.r32(b)) };
    let gb2 = |e: &scooby_recomp::engine::Engine| { let b = e.rt.game_buffers.get(&2).unwrap().0; e.mem.r32(b + 0xc) };
    assert!(run(&mut e, &mut rt, "f 200; c 2a000061; f 60").is_none());
    let l = leader(&e);
    assert_eq!(gb2(&e) & 0x80, 0x80, "clue picked");
    let before = pos(&mut e, &rt, l).unwrap();
    assert!(before[0] > 480.0, "cart past the clue {before:?}");
    assert!(run(&mut e, &mut rt, "g 1 0 0; f 1").is_none());
    let mut resp = None;
    for i in 0..60 {
        assert!(run(&mut e, &mut rt, "f 10").is_none());
        let p = pos(&mut e, &rt, l).unwrap();
        if p[0] < 420.0 { resp = Some((i * 10, p)); break; }
        assert_eq!(cg(&e), 0.0, "courage stays 0 until the crash");
    }
    let (f, p) = resp.expect("cart respawned at the checkpoint");
    println!("R56 respawn f{f} {p:?}");
    assert_eq!(cg(&e), 1.0, "RestartRide refills courage");
    assert_eq!(gb2(&e) & 0x80, 0, "clue rolled back to the checkpoint");
}

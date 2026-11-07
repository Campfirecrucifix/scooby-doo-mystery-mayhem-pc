//! P32 R58: player-visible behaviour on EP1/L02.
//! - Item rollback on foot: the clue/snack 0x2a0000b5 next to the level start (coll_level2, GB2+0xc |= 0x100) is picked
//!   up, then the hero dies (alone bit + courage 0.05 + scare hit, as R57). On MSG_FADED with GB0+0x14 & 0x8000000 the hero
//!   broadcasts 0xc1 to group 6 (hero_msgfunc.c) and Coll_base MyMessageFunction 0xc1 puts every item collected since the
//!   checkpoint back (SetTranslation to its home, visible, ResetBit): the bit is cleared and the item is back, as the C.
//! - Pause menu (START) at f400: no game-buffer word changes while it is open and the partner 0x2a000021 keeps its x/z
//!   (world paused, *(_gpoWorldMgr+0x14)), and it moves again after the menu is closed.
use crate::common::{ISO, leader, load_carried, pos};
use crate::engine_p17::run;
use scooby_recomp::engine::iso::IsoFs;

type Snap = Vec<(u32, Vec<u32>)>;
fn snap(e: &scooby_recomp::engine::Engine) -> Snap {
    let mut k: Vec<_> = e.rt.game_buffers.iter().map(|(k, v)| (*k, *v)).collect();
    k.sort();
    k.into_iter().map(|(id, (p, n))| (id, (0..n.min(0x2000) / 4).map(|i| e.mem.r32(p + i * 4)).collect())).collect()
}
fn diff(a: &Snap, b: &Snap) -> Vec<String> {
    let mut o = vec![];
    for ((id, x), (_, y)) in a.iter().zip(b) { for (i, (u, v)) in x.iter().zip(y).enumerate() { if u != v { o.push(format!("GB{id}+{:#x}: {u:#x}->{v:#x}", i * 4)) } } }
    o
}

#[test]
fn p32_r58_ep1_l02_pickup_rollback_on_death() {
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let (mut e, mut rt) = load_carried(&fs, "EP1/L02", &[]).unwrap();
    let gb2c = |e: &scooby_recomp::engine::Engine| { let b = e.rt.game_buffers.get(&2).unwrap().0; e.mem.r32(b + 0xc) };
    let f14 = |e: &scooby_recomp::engine::Engine| { let b = e.rt.game_buffers.get(&0).unwrap().0; e.mem.r32(b + 0x14) };
    assert!(run(&mut e, &mut rt, "f 200").is_none());
    let l = leader(&e);
    let home = pos(&mut e, &rt, 0x2a0000b5).unwrap();
    assert_eq!(gb2c(&e) & 0x100, 0);
    assert!(run(&mut e, &mut rt, "c 2a0000b5; f 30").is_none());
    assert_eq!(gb2c(&e) & 0x100, 0x100, "item picked");
    assert!(run(&mut e, &mut rt, "gor 0 34 1; g 1 0 3d4ccccd; warp 2a000021; f 1").is_none());
    let mut dead = false;
    for _ in 0..40 {
        assert!(run(&mut e, &mut rt, "f 15").is_none());
        dead |= f14(&e) & 0x8000000 != 0;
        if dead && f14(&e) & 0x8000000 == 0 { break }
    }
    assert!(dead, "hero died");
    assert!(run(&mut e, &mut rt, "f 60").is_none());
    let p = pos(&mut e, &rt, l).unwrap();
    assert!((p[0] - 32.0).abs() < 0.01 && (p[2] + 2.5).abs() < 0.01, "respawn at the level start {p:?}");
    assert_eq!(gb2c(&e) & 0x100, 0, "pickup rolled back");
    let c = pos(&mut e, &rt, 0x2a0000b5).unwrap();
    assert!((c[0] - home[0]).abs() < 0.01 && (c[2] - home[2]).abs() < 0.01 && c[1] < 2.0, "item back home {c:?} vs {home:?}");
}

#[test]
fn p32_r58_ep1_l02_pause_menu_freezes() {
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let (mut e, mut rt) = load_carried(&fs, "EP1/L02", &[]).unwrap();
    assert!(run(&mut e, &mut rt, "f 400; p 8 3; f 30").is_none());
    let (s1, p1) = (snap(&e), pos(&mut e, &rt, 0x2a000021).unwrap());
    assert!(run(&mut e, &mut rt, "f 120").is_none());
    let (s2, p2) = (snap(&e), pos(&mut e, &rt, 0x2a000021).unwrap());
    assert!(diff(&s1, &s2).is_empty(), "game state advanced while paused: {:?}", diff(&s1, &s2));
    assert!(p1[0] == p2[0] && p1[2] == p2[2], "partner moved while paused {p1:?} {p2:?}");
    assert!(run(&mut e, &mut rt, "p 8 3; f 60").is_none());
    let p3 = pos(&mut e, &rt, 0x2a000021).unwrap();
    assert!(p3[0] != p2[0] || p3[2] != p2[2], "unpaused partner moves again");
}

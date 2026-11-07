//! P32 R64: bonus minigames 2..5 (title Extras > Mini Games items 44..47, GB2 word0 0x2000..0x10000) load, idle lose
//! path -> scoreboard -> play again / quit, as the C (panel_baseFUNC_MiniGameSearch / PlayMiniGame, scoreboard).
use crate::common::{ISO, carry, load_carried, load_dump};
use scooby_recomp::engine::iso::IsoFs;
use crate::engine_p17::run;

type Eng = scooby_recomp::engine::Engine;
fn gbw(e: &Eng, b: u32, o: u32) -> u32 { e.rt.game_buffers.get(&b).map_or(0xdead, |p| e.mem.r32(p.0 + o)) }
fn patch(b: &mut [(u32, Vec<u8>)], id: u32, o: usize, f: impl Fn(u32) -> u32) {
    let v = &mut b.iter_mut().find(|g| g.0 == id).unwrap().1;
    let w = f(u32::from_le_bytes(v[o..o + 4].try_into().unwrap()));
    v[o..o + 4].copy_from_slice(&w.to_le_bytes());
}

/// title carry: EP1 end save -> EP2/L01A -> pause quit -> INTRO (as p32_r63)
pub fn intro_carry(fs: &IsoFs) -> Vec<(u32, Vec<u8>)> {
    let bufs = load_dump(&format!("{}/tests/all/data/ep1_end_EP2L01A.bin", env!("CARGO_MANIFEST_DIR")));
    let (mut e, mut rt) = load_carried(fs, "EP2/L01A", &bufs).unwrap();
    let r = run(&mut e, &mut rt, "f 300; pausequit");
    assert_eq!(r.as_ref().map(|r| r.0.as_str()), Some("INTRO"), "pause quit");
    carry(&mut e)
}

pub const MENU: &str = "f 300; s 80 ff 10; f 30; s 80 ff 10; f 30; s 80 ff 10; f 30; p 4000 3; f 200; s 80 ff 10; f 30; p 4000 3; f 200; s 80 ff 10; f 30; p 4000 3; f 300";

/// Each single bonus bit: the Mini Games item lands on that game (locked ones skipped) and X loads its level;
/// idle -> lives 0 -> scoreboard; X = play again reloads the same level. Returns (level, end frame).
pub fn bonus(fs: &IsoFs, base: &[(u32, Vec<u8>)], bit: u32, end: &str) -> (String, Option<(String, u32)>) {
    let mut bufs = base.to_vec();
    patch(&mut bufs, 2, 0, |w| (w & !0x1f000) | bit);
    let (mut e, mut rt) = load_carried(fs, "INTRO", &bufs).unwrap();
    let (lvl, f) = run(&mut e, &mut rt, MENU).unwrap_or_else(|| panic!("bit {bit:#x}: menu X loaded nothing"));
    println!("R64B bit {bit:#x} -> {lvl} at f{f} GB2+0 {:#x}", gbw(&e, 2, 0));
    let bufs = carry(&mut e);
    let (mut e, mut rt) = load_carried(fs, &lvl, &bufs).unwrap();
    let r = run(&mut e, &mut rt, end);
    println!("R64B {lvl} end -> {r:?}");
    (lvl, r)
}

#[test]
#[ignore]
fn p32_r64_probe_bonus() {
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let base = intro_carry(&fs);
    let bits: Vec<u32> = std::env::var("R64BITS").map(|s| s.split(',').map(|x| u32::from_str_radix(x, 16).unwrap()).collect()).unwrap_or(vec![0x2000, 0x4000, 0x8000, 0x10000]);
    let end = std::env::var("R64END").unwrap_or("f 12000; p 4000 3; f 300; p 4000 3; f 300; p 4000 3; f 600".into());
    for b in bits { bonus(&fs, &base, b, &end); }
}

/// EP3/EP4/EP6 bonus (bits 0x4000/0x8000/0x10000): menu lands on the right game, it loads, idle -> lives out ->
/// scoreboard -> X play again reloads it (f12677, as EP1 in p32_r63). EP2/L09 idle does not end in 12000 f (R64 open).
#[test]
#[ignore]
fn p32_r64_bonus_ep3_ep4_ep6_replay() {
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let base = intro_carry(&fs);
    for (bit, want) in [(0x4000, "EP3/L09"), (0x8000, "EP4/L09A"), (0x10000, "EP6/L09")] {
        let (lvl, r) = bonus(&fs, &base, bit, "f 12000; p 4000 3; f 300; p 4000 3; f 300; p 4000 3; f 600");
        assert_eq!(lvl, want, "bit {bit:#x}");
        assert!(matches!(&r, Some((l, f)) if l == want && *f < 13500), "{want} end -> {r:?}");
    }
}

//! WP-L6: EP1L03A (first level with the book) logic modules link and run.
use scooby_recomp::engine::{iso::IsoFs, pack::*};
use scooby_recomp::engine::{Engine, Mem, logic_link::*};
use std::collections::HashMap;
/// Zero host, except EFGameObject::GetObjectFrame-backed matrices: in the C++ GetBoneLMat /
/// GetBoneWMat never return NULL for a live object (its RwFrame modelling matrix / LTM, 0x40
/// bytes at frame+0x10); l01_camera (ResetCamera/ReplaceOrientation) dereferences it.
#[derive(Default)]
struct NullHost(HashMap<(u32, u32), u32>);
impl NullHost {
    fn frame(&mut self, mem: &mut Mem, a0: u32, a1: u32) -> u32 {
        *self.0.entry((a0, a1)).or_insert_with(|| {
            let f = mem.malloc(0x90) + 0x10;
            for (i, v) in [1.0f32, 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.].iter().enumerate() {
                mem.w32(f + 4 * i as u32, v.to_bits());
            }
            f
        })
    }
}
impl scooby_recomp::engine::Host for NullHost {
    fn APIgameobj_GetBoneLMat(&mut self, mem: &mut Mem, a0: u32, a1: u32) -> i32 {
        self.frame(mem, a0, a1) as i32
    }
    fn APIgameobj_GetBoneWMat(&mut self, mem: &mut Mem, a0: u32, a1: u32) -> u32 {
        self.frame(mem, a0, a1)
    }
}

use crate::common::ISO;


fn load() -> Option<EFRessourcesMgr> {
    let fs = IsoFs::open(ISO).expect("ISO missing: tests must load real data");
    let mut m = EFRessourcesMgr::default();
    m.level_file_name = Some("EP1L03A.P2R".into());
    assert_eq!(m.LoadLevel(&fs, "", "EP1/L03A", "EN", &mut RawInstanciator, |_, _| {}).unwrap(), 0);
    Some(m)
}

#[test]
fn ep1l03a_links_and_runs() {
    let Some(m) = load() else { return };
    let mut e = Engine::new(Box::new(NullHost::default()));
    let (mut map, mut rep) = (LogicCodeMap::default(), LevelLogicReport::default());
    let objs = LinkLevelLogic(&mut e, &m, &mut map, &mut rep);
    let codes: Vec<String> = rep.codes.iter().map(|c| format!("{}:{}", c.base_name(), c.sha8)).collect();
    let unported: Vec<String> = rep.unported.iter().map(|c| format!("{}:{}", c.base_name(), c.sha8)).collect();
    eprintln!("codes: {codes:?}\nunported: {unported:?}");
    for _ in 0..120 {
        for &h in &objs {
            if e.rt.objects[h].is_some() {
                e.run_object(h, 1.0 / 60.0);
            }
        }
    }
}

/// The original hand-over path: the leader touching VelmaNew (collision notify 3 -> callback 6)
/// once GB2+0xc bit 0x2000000 is set (and GB2+8 bit 0x400000 = first talk done) sends msg 0x70
/// to the book 0x2a0000ee -> TomeDoomFUNC_GrabBook sets GB2 bits 1|2.
#[test]
fn ep1l03a_velmanew_hands_over_the_book() {
    let Some(m) = load() else { return };
    let mut e = Engine::new(Box::new(NullHost::default()));
    let (mut map, mut rep) = (LogicCodeMap::default(), LevelLogicReport::default());
    let objs = LinkLevelLogic(&mut e, &m, &mut map, &mut rep);
    let run = |e: &mut Engine, n| {
        for _ in 0..n {
            for &h in &objs {
                if e.rt.objects[h].is_some() {
                    e.run_object(h, 1.0 / 60.0);
                }
            }
        }
    };
    run(&mut e, 10);
    let velma = objs.iter().filter_map(|&h| e.rt.objects[h].as_ref()).find(|o| o.inst.code.is_some_and(|c| e.rt.codes[c].def.name == "velmanew")).map(|o| o.id).expect("velmanew object");
    assert!(objs.iter().filter_map(|&h| e.rt.objects[h].as_ref()).any(|o| o.id == 0x2a0000ee), "book");
    let (gb0, gb2) = (e.rt.game_buffers[&0].0, e.rt.game_buffers[&2].0);
    assert_eq!(e.mem.r32(gb2) & 1, 0, "book not held at start");
    let leader = 0x2a00_0001;
    e.mem.w32(gb0 + 8, leader);
    e.mem.w32(gb0 + 0x14, 0);
    let p = e.mem.malloc(0x20);
    e.mem.w32(p + 8, leader);
    // 1st touch: VelmaNew sets GB2+8 bit 0x400000 herself (cut-scene 0x80000002).
    e.notify_object(3, velma, p, false);
    run(&mut e, 2);
    assert_ne!(e.mem.r32(gb2 + 8) & 0x40_0000, 0, "first talk flag");
    assert_eq!(e.mem.r32(gb2) & 1, 0, "no book after the first talk");
    // Stand-in for coll_level3a SetBit of collectable 0x2a000048 (TmpMap entry 25:
    // status index 1 = GB2+0xc, bit 0x2000000); coll_level3a is not ported yet.
    let gc = e.mem.r32(gb2 + 0xc) | 0x200_0000;
    e.mem.w32(gb2 + 0xc, gc);
    e.notify_object(3, velma, p, false);
    run(&mut e, 2);
    assert_ne!(e.mem.r32(gb2 + 8) & 0x80_0000, 0, "hand-over flag");
    assert_eq!(e.mem.r32(gb2) & 3, 3, "book grabbed (msg 0x70)");
}

/// WP-L7 ghost_lock: msg 0xdf (beam) on lock 0x2a0000e5 -> state OFF sets its GB2+0xc bit 0x2000;
/// checkpoint msg 0xc1 clears it again.
#[test]
fn ep1l03a_ghost_lock_opens() {
    let Some(m) = load() else { return };
    let mut e = Engine::new(Box::new(NullHost::default()));
    let (mut map, mut rep) = (LogicCodeMap::default(), LevelLogicReport::default());
    let objs = LinkLevelLogic(&mut e, &m, &mut map, &mut rep);
    let run = |e: &mut Engine, n| {
        for _ in 0..n {
            for &h in &objs {
                if e.rt.objects[h].is_some() {
                    e.run_object(h, 1.0 / 60.0);
                }
            }
        }
    };
    run(&mut e, 10);
    let lock = *objs.iter().find(|&&h| e.rt.objects[h].as_ref().is_some_and(|o| o.id == 0x2a00_00e5)).expect("ghost lock 0x2a0000e5");
    let (gb0, gb2) = (e.rt.game_buffers[&0].0, e.rt.game_buffers[&2].0);
    e.mem.w32(gb0 + 0x14, 0);
    assert_eq!(e.mem.r32(gb2 + 0xc) & 0x2000, 0, "closed at start");
    // what EFGameLogicMgr does before a message: the receiver is the current stack object
    let top = e.rt.stack.top;
    e.rt.stack.objects[top] = Some(lock);
    e.SendMessage(lock, 0xdf, 0, 0);
    run(&mut e, 2);
    assert_ne!(e.mem.r32(gb2 + 0xc) & 0x2000, 0, "lock bit set");
    e.rt.stack.objects[top] = Some(lock);
    e.SendMessage(lock, 0xc1, 0, 0);
    assert_eq!(e.mem.r32(gb2 + 0xc) & 0x2000, 0, "checkpoint reset");
}

//! WP-L16: scooby_ghost / shaggy_ghost (8 builds, EP1 L01..L03B) link, construct and run 300 frames.
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
#[test]
fn ep1_hero_ghost_construct_and_run() {
    for (lvl, dir) in [("EP1L01.P2R", "EP1/L01"), ("EP1L02.P2R", "EP1/L02"), ("EP1L03A.P2R", "EP1/L03A"), ("EP1L03B.P2R", "EP1/L03B")] {
        let Ok(fs) = IsoFs::open(ISO) else { return };
        let mut m = EFRessourcesMgr::default();
        m.level_file_name = Some(lvl.into());
        if m.LoadLevel(&fs, "", dir, "EN", &mut RawInstanciator, |_, _| {}).unwrap() != 0 { eprintln!("{lvl} load fail"); continue; }
        let mut e = Engine::new(Box::new(NullHost::default()));
        let (mut map, mut rep) = (LogicCodeMap::default(), LevelLogicReport::default());
        let objs = LinkLevelLogic(&mut e, &m, &mut map, &mut rep);
        let g: Vec<String> = rep.codes.iter().map(|c| c.base_name()).filter(|n| n.contains("ghost")).collect();
        eprintln!("{lvl}: objects {} ghost codes {g:?} unported {:?}", objs.len(), rep.unported.iter().map(|c| c.base_name()).collect::<Vec<_>>());
        let nm = |e: &Engine, c: Option<scooby_recomp::logic::CodeId>| c.map(|c| e.rt.codes[c].def.name).unwrap_or("-");
        let mut launched = 0;
        for h in objs.clone() { let n = e.rt.objects[h].as_ref().map(|o| nm(&e, o.inst.code)).unwrap_or("-"); if n == "scooby_ghost" { e.LaunchLogicNow(h); assert_eq!(e.rt.objects[h].as_ref().unwrap().inst.state, 0, "{lvl}: ghost not constructed"); launched += 1; } }
        assert_eq!(launched, 2, "{lvl}: scooby_ghost + shaggy_ghost");
        for _ in 0..300 {
            for &h in &objs {
                if e.rt.objects[h].is_some() { e.run_object(h, 1.0 / 60.0); }
            }
        }
        eprintln!("{lvl}: ran 300 frames");
    }
}

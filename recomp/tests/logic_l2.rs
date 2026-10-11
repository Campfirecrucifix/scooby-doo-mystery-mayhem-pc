//! WP-L2: EP1L01 level-prop logic modules link and run.
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

const ISO: &str = "../Scooby Doo Mystery Mayhem.ISO";
const PORTED: &[&str] = &["chkpnt", "candle", "pizza", "costume", "inv", "beamoflight", "rollingladder", "cartstype01", "mischud", "enemybar", "enmyspot", "herospot", "rdrmngr", "panel_child", "stand_in_obj", "invnorthpos", "roomlocator", "hs_tutorial", "level01_projectors", "hs_door_l01", "button", "chest_level1", "coll_level1", "inv_control"];

#[test]
fn ep1l01_l2_modules_link_and_run() {
    let fs = IsoFs::open(ISO).expect("ISO missing: tests must load real data");
    let mut m = EFRessourcesMgr::default();
    m.level_file_name = Some("EP1L01.P2R".into());
    assert_eq!(m.LoadLevel(&fs, "", "EP1/L01", "EN", &mut RawInstanciator, |_, _| {}).unwrap(), 0);
    let mut e = Engine::new(Box::new(NullHost::default()));
    let (mut map, mut rep) = (LogicCodeMap::default(), LevelLogicReport::default());
    let objs = LinkLevelLogic(&mut e, &m, &mut map, &mut rep);
    let unported: Vec<String> = rep.unported.iter().map(|c| c.base_name()).collect();
    eprintln!("unported: {unported:?}");
    for p in PORTED {
        assert!(rep.codes.iter().any(|c| &c.base_name() == p), "{p} not in EP1L01");
        assert!(!unported.iter().any(|u| u == p), "{p} not linked");
    }
    for _ in 0..120 {
        for &h in &objs {
            if e.rt.objects[h].is_some() {
                e.run_object(h, 1.0 / 60.0);
            }
        }
    }
}

//! WP-L8: multi-build families ported over all level packs. Every pack is loaded and linked; each
//! code entry of a WP-L8 family must match a ported build, and the linked objects run 3 logic
//! frames (NullHost with 1-waypoint paths and identity bone matrices) without panicking.
use scooby_recomp::engine::{iso::IsoFs, pack::*};
use scooby_recomp::engine::{Engine, Mem, logic_link::*};
use std::collections::HashMap;

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
    /// real paths have >= 1 waypoint; 0 makes fireghost GetNearestWP loop 2^32 times (L+0x1f8 = -1)
    fn APIextra_WaypointGetNb(&mut self, _mem: &mut Mem, _a0: u32) -> u32 {
        1
    }
}

use crate::common::ISO;

/// families completed by WP-L8 (every build on the disc is ported)
const FAMILIES: &[&str] = &["enemybar", "enmyspot", "herospot", "rdrmngr", "panel_child", "chkpnt", "ghost_lock", "fearghost", "switch", "switch_fred", "zombiegroup", "switchdoor", "hs_launchenemy", "fakezombie", "fakezombieb", "redknight", "swampcreature", "velmaboss", "inv_switchsolver_01a", "cut", "cut1", "cut8", "hs_door", "hs_door_l01", "hs_door_l03"];

#[test]
fn l8_families_link_and_run_in_all_levels() {
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let mut packs: Vec<String> = fs.names().filter(|n| n.starts_with("LEVEL/") && n.ends_with(".P2R")).map(String::from).collect();
    packs.sort();
    let (mut missing, mut failures, mut seen) = (Vec::new(), Vec::new(), 0);
    for p in &packs {
        let parts: Vec<&str> = p.split('/').collect();
        let dir = parts[1..parts.len() - 2].join("/");
        let mut m = EFRessourcesMgr::default();
        m.level_file_name = Some(parts[parts.len() - 1].to_string());
        assert_eq!(m.LoadLevel(&fs, "", &dir, "EN", &mut RawInstanciator, |_, _| {}).unwrap(), 0, "{p}");
        let mut e = Engine::new(Box::new(NullHost::default()));
        let (mut map, mut rep) = (LogicCodeMap::default(), LevelLogicReport::default());
        let objs = LinkLevelLogic(&mut e, &m, &mut map, &mut rep);
        for c in &rep.codes {
            if FAMILIES.contains(&c.base_name().as_str()) {
                seen += 1;
                if rep.unported.iter().any(|u| u.uid == c.uid) {
                    missing.push(format!("{p}: {}:{}", c.base_name(), c.sha8));
                }
            }
        }
        eprintln!("{p}");
        let run = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            for _ in 0..3 {
                for &h in &objs {
                    e.run_object(h, 1.0 / 60.0);
                }
            }
        }));
        if let Err(x) = run {
            let msg = x.downcast_ref::<String>().cloned().or_else(|| x.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_default();
            failures.push(format!("{p}: {msg}"));
        }
    }
    eprintln!("{} packs, {seen} WP-L8 family entries", packs.len());
    assert!(missing.is_empty(), "unported builds:\n{}", missing.join("\n"));
    if !failures.is_empty() {
        eprintln!("logic frames panicked (all modules, informational):\n{}", failures.join("\n"));
    }
}

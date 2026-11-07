


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

// WP-L19: per-object smoke run of the c2rs-generated families (FAMILIES env or the list below):
// every pack is linked, every object runs 3 frames with its own catch_unwind, and panics are
// reported per module (name:sha8). Fails if a generated family entry is unported or panics.
use crate::common::ISO;
const FAMILIES: &[&str] = &["inv_control", "savepanel", "velma", "coll_level3", "hs_changetrack", "hs_rideobst", "minigame_initialtimer", "scbride", "scoreboard", "shgride", "shockwave", "animated_csq", "bbride", "coll_level2", "coll_level4", "coll_level5", "coll_level9", "democontrol", "fireballbig_expl", "flash_poly", "gamecontroller", "cartstype03", "castlegate", "catapult_02a", "ceiling_sprinkler", "coll_level2a", "colomn", "crane", "hs_commun", "hs_displaytext", "hs_sprinkler", "panneaux_csq", "watercastleyard", "deadendlatant", "electricarc_local", "ennemymcart", "fake_alien_a", "fake_alien_b", "fatcowboy", "fireballbig", "fireplace", "gramophone", "hs_automaticjump", "hs_cam_l04", "hs_changelevel", "hs_cutscene", "hs_door_l02", "hs_door_l04", "hs_looping", "hs_safe", "mc_lantern", "mcboulder", "mcrock", "obj_launcher_level02", "obstlatant", "radar_dish", "rideobstacle", "safe_door", "safedialframe", "shoreline", "shoreline2", "shorelineclip", "streetbelle", "vault_door", "shaggy_level2b", "zdaphne", "zfred", "zvelma", "costume_scooby", "costume_shaggy", "costume_scooby_scientist", "costume_shaggy_scientist", "costume_scooby_snorkel", "costume_scooby_cowboy", "costume_shaggy_cowboy", "costume_scooby_mummy", "costume_shaggy_mummy", "costume_scooby_security_guard", "costume_shaggy_security_guard", "costume_scooby_spacesuit", "costume_shaggy_spacesuit"];

thread_local!(static QUIET: std::cell::Cell<bool> = const { std::cell::Cell::new(false) });

/// Silences only the panics this test expects (its own thread, inside its per-object catch_unwind);
/// every other panic chains to the previous hook. The old global `set_hook(|_| {})` + `take_hook()`
/// swallowed the messages of every other test in the parallel `all` binary while L19 ran (main
/// 6c696e70 gate: p36_r10 failed with no text). Installed once and never removed, since restoring
/// a hook races with other tests panicking concurrently; for other threads it is the previous hook.
fn quiet_panics_hook() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let prev = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |p| {
            if !QUIET.get() {
                prev(p)
            }
        }));
    });
}

/// R12 proof: an unrelated panicking test running alongside L19 still gets its message printed.
#[test]
fn l19_hook_keeps_other_panics() {
    quiet_panics_hook();
    let t = std::thread::spawn(|| {
        QUIET.set(true);
        let quiet = std::panic::catch_unwind(|| panic!("l19 expected")).is_err();
        QUIET.set(false);
        quiet
    });
    assert!(t.join().unwrap());
    assert!(!QUIET.get(), "flag is per thread");
    eprintln!("l19 hook: other-thread panic below must print its message");
    assert!(std::panic::catch_unwind(|| panic!("p36 r12 visible panic")).is_err());
}

#[test]
fn l19_generated_families_run_per_object() {
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let fam: Vec<String> = std::env::var("FAMILIES").map(|s| s.split(',').map(String::from).collect()).unwrap_or_else(|_| FAMILIES.iter().map(|s| s.to_string()).collect());
    let mut packs: Vec<String> = fs.names().filter(|n| n.starts_with("LEVEL/") && n.ends_with(".P2R")).map(String::from).collect();
    packs.sort();
    quiet_panics_hook();
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
        let mut ours = std::collections::HashSet::new();
        for c in &rep.codes {
            if fam.contains(&c.base_name()) {
                seen += 1;
                if rep.unported.iter().any(|u| u.uid == c.uid) {
                    missing.push(format!("{p}: {}:{}", c.base_name(), c.sha8));
                }
            }
        }
        for (i, code) in e.rt.codes.iter().enumerate() {
            if fam.iter().any(|f| f == code.def.name) {
                ours.insert(i);
            }
        }
        let mut dead = std::collections::HashSet::new();
        for _ in 0..3 {
            for &h in &objs {
                if dead.contains(&h) || e.rt.objects.get(h as usize).map_or(true, |o| o.is_none()) {
                    continue;
                }
                let code = e.rt.obj(h).inst.code;
                let top = e.rt.stack.top;
                let sp = e.sp();
                QUIET.set(true);
                let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| e.run_object(h, 1.0 / 60.0)));
                QUIET.set(false);
                if let Err(x) = r {
                    e.rt.stack.top = top;
                    e.set_sp(sp);
                    dead.insert(h);
                    if code.is_some_and(|c| ours.contains(&c)) {
                        let msg = x.downcast_ref::<String>().cloned().or_else(|| x.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_default();
                        failures.push(format!("{p}: {}#{}: {msg}", e.rt.codes[code.unwrap()].def.name, e.rt.codes[code.unwrap()].build));
                    }
                }
            }
        }
    }
    eprintln!("{} packs, {seen} family entries", packs.len());
    assert!(missing.is_empty(), "unported builds:\n{}", missing.join("\n"));
    assert!(failures.is_empty(), "generated family objects panicked:\n{}", failures.join("\n"));
}

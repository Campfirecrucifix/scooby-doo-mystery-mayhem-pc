//! WP-E10: collision-dependent physics wiring, real EP1L01 map with the real EFCollisionSystem.
#![allow(non_snake_case)]
use scooby_recomp::engine::iso::IsoFs;
use scooby_recomp::engine::pack::*;
use scooby_recomp::engine::physic::collision_map::EFCollisionMap;
use scooby_recomp::engine::physic::floor::FindFloor;
use scooby_recomp::engine::physic::mgr::{CollisionSystem, EFPhysicMgr, EFUpdatePhysicStruct};
use scooby_recomp::engine::physic::model::{EFCollisionModel, ObjPrim, PrimExt};
use scooby_recomp::engine::physic::module::{EFDynamicPhysic, PhysicWorld};
use scooby_recomp::engine::physic::objcoll_prim::EFObjCollisionPrim;
use scooby_recomp::engine::physic::physic::flags;
use scooby_recomp::engine::physic::rwm::RwMatrix;
use scooby_recomp::engine::physic::system::EFCollisionSystem;
const ISO: &str = "../Scooby Doo Mystery Mayhem.ISO";

struct W;
impl PhysicWorld for W {}

/// Hero sphere (r 0.5) falling onto the real EP1L01 floor, solved by EFCollisionSystem with the
/// real prim tests (ObjPrimTests) and the map's env prims gathered by BuildEnvPrimsLists.
#[test]
fn hero_sphere_lands_on_ep1l01_with_real_system() {
    let Ok(fs) = IsoFs::open(ISO) else { eprintln!("ISO missing, skipped"); return };
    let mut m = EFRessourcesMgr::default();
    m.level_file_name = Some("EP1L01.P2R".into());
    assert_eq!(m.LoadLevel(&fs, "", "EP1/L01", "EN", &mut RawInstanciator, |_, _| {}).unwrap(), 0);
    let raw = m.objects.iter().filter_map(|o| o.as_raw()).find(|r| r.class_id == 0x24)
        .map(|r| r.data.clone()).expect("collision map");
    let mut map = EFCollisionMap::LoadFromRaw(&raw).unwrap().0;
    let mut floor = None;
    for (k, g) in m.game_objects.iter().enumerate() {
        let p = g.glo.pos;
        if p == [0.0; 3] { continue; }
        if let Some((hp, n, _)) = FindFloor(&mut map, [p[0], p[1] + 1.0, p[2]], 20.0, 1 + k as i32) {
            if n[1] > 0.9 { floor = Some(hp); break; }
        }
    }
    let floor = floor.expect("floor");
    let r = 0.5f32;
    let pos = [floor[0], floor[1] + r + 0.3, floor[2]];
    let mut mgr = EFPhysicMgr::new();
    // fresh map: FindFloor above left its query stamps in the prims
    mgr.maps.push(EFCollisionMap::LoadFromRaw(&raw).unwrap().0);
    let mut f = RwMatrix::identity();
    f.pos = pos;
    let id = mgr.CreatePhysic(1, f, 0);
    let mut b = EFObjCollisionPrim::new([[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [pos[0], pos[1], pos[2], 1.0]]);
    b.radius = r;
    for (o, v) in [(0xc, 1), (0x10, 1), (0x14, 1), (0x20, !0), (0x24, !0)] { b.set_word(o, v); }
    let mut cm = EFCollisionModel::new(1);
    cm.prims = vec![Some(ObjPrim { base: b, ext: PrimExt::Sphere })];
    cm.masks = [!0; 6];
    cm.primary = 0;
    {
        let p = mgr.PhysicMut(id).unwrap();
        p.flags |= flags::COLLIDE;
        p.model = cm;
        p.velocity = [0.0, -6.0, 0.0];
    }
    let d = EFDynamicPhysic { physic: id, sphere: pos, radius: r + 2.0, ..Default::default() };
    let mut ups = EFUpdatePhysicStruct { dt: 0.1, passes: [vec![d], vec![]], emitter_count: 0 };
    let mut sys = EFCollisionSystem::new();
    sys.SyncModels(&mut mgr, &W);
    sys.Init(&mut mgr, &mut ups, 0, 0);
    assert!(sys.stages[0].env.iter().any(|e| e.normal[1] > 0.9), "floor prim gathered");
    sys.Update(&mut mgr, &mut ups, &W);
    sys.PostUpdate(&mut mgr, &mut ups, &W);
    sys.Terminate(&mut ups);
    let p = mgr.Physic(id).unwrap();
    eprintln!("pos {:?} vel {:?} floor {:?}", p.frame.pos, p.velocity, floor);
    assert!(p.frame.pos[1] >= floor[1] + r - 0.05, "sank through: {:?}", p.frame.pos);
    assert!(p.frame.pos[1] < pos[1], "did not move: {:?}", p.frame.pos);
    assert!(p.velocity[1] > -1e-2, "vertical velocity cancelled: {:?}", p.velocity);
}

// Part B: dynamic modules (dm/*.rs)
mod ray_env { include!("e10/ray_env.rs"); }
mod edge_detect { include!("e10/edge_detect.rs"); }
mod edge_down { include!("e10/edge_down.rs"); }
mod grab_edge { include!("e10/grab_edge.rs"); }
mod climb { include!("e10/climb.rs"); }
mod rope_slide { include!("e10/rope_slide.rs"); }
mod water_simulation { include!("e10/water_simulation.rs"); }

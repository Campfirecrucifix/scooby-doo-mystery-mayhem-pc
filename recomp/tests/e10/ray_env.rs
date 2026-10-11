// WP-E10: EFDMRayIntersect (0x1b) / EFDMEnvColl (0x22). Included by tests/e10_ray_env.rs.
use scooby_recomp::engine::iso::IsoFs;
use scooby_recomp::engine::pack::*;
use scooby_recomp::engine::physic::collision_map::{EFCollisionMap, EFGetPrimitivesInSphereData};
use scooby_recomp::engine::physic::dm::env_coll::EFDMEnvColl;
use scooby_recomp::engine::physic::dm::ray_intersect::*;
use scooby_recomp::engine::physic::dm::NewModule;
use scooby_recomp::engine::physic::floor::FindFloor;
use scooby_recomp::engine::physic::mgr::{EFPhysicMgr, NoCollision};
use scooby_recomp::engine::physic::model::{EFCollisionModel, ObjPrim, PrimExt};
use scooby_recomp::engine::physic::module::*;
use scooby_recomp::engine::physic::objcoll_check::EnvPrim;
use scooby_recomp::engine::physic::objcoll_prim::EFObjCollisionPrim;
use scooby_recomp::engine::physic::physic::EFPhysic;
use scooby_recomp::engine::physic::rwm::RwMatrix;
use scooby_recomp::engine::physic::system::EFCollisionSystem;
const ISO: &str = "../Scooby Doo Mystery Mayhem.ISO";

/// 2x2 floor quad at y = 0 (x, z in [-1, 1]), material masks word1 = 1, word2 = 0, word5 = 0x55.
fn floor_quad() -> EnvPrim {
    EnvPrim {
        id: 7,
        mat: [9, 1, 0, 0, 0, 0x55, 0, 0],
        w4: 0,
        nverts: 4,
        normal: [0.0, 1.0, 0.0],
        d: 0.0,
        verts: vec![[-1.0, 0.0, -1.0], [1.0, 0.0, -1.0], [1.0, 0.0, 1.0], [-1.0, 0.0, 1.0]],
        edges: vec![[0.0, 0.0, -1.0, -1.0], [1.0, 0.0, 0.0, -1.0], [0.0, 0.0, 1.0, -1.0], [-1.0, 0.0, 0.0, -1.0]],
    }
}

fn phys_at(pos: [f32; 3]) -> EFPhysic {
    let mut f = RwMatrix::identity();
    f.pos = pos;
    EFPhysic::new(0x77, f)
}

#[test]
fn factory_kinds() {
    assert_eq!(NewModule(0x1b).unwrap().base().type_, 0x5000_001b);
    assert_eq!(NewModule(0x22).unwrap().base().type_, 0x4000_0022);
}

#[test]
fn ray_env_hit_local_frame_and_state_notify() {
    let mut ph = phys_at([0.25, 2.0, 0.0]);
    let mut notes = Vec::new();
    let mut ctx = DMCtx { phys: &mut ph, frame: 0, paused_frames: 0, prev_paused: 0, world: &NoWorld, notify: &mut notes };
    let mut m = EFDMRayIntersect::new();
    m.base.notify_bits = 1;
    m.base.handle = 0x10005;
    let mut p = [0u32; 27];
    p[4] = fw(-4.0); // local ray (0,0,0) -> (0,-4,0)
    p[8] = 1; // map mask1
    m.SetParams(&mut ctx, Some(&p));
    let env = [floor_quad()];
    let mut host = WorldRayHost(&NoWorld);
    m.PostColUpdateWith(&mut ctx, true, &env, &mut host);
    assert_eq!(m.result, 2);
    assert_eq!(m.base.state, 1);
    // hit back in frame-local space: (0, -2, 0)
    assert!((m.pos[1] + 2.0).abs() < 1e-6 && m.pos[0].abs() < 1e-6, "{:?}", m.pos);
    assert_eq!(m.normal, [0.0, 1.0, 0.0]);
    assert_eq!(m.info[0], 0x55);
    assert_eq!(&m.info[1..9], &[9, 1, 0, 0, 0, 0x55, 0, 0]);
    assert_eq!(m.origin, [0.0; 3], "ray restored");
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].data, [0x10005, 0x5000_001b, 1, 1]);
    // GetParams round trip
    let ctx = DMCtx { phys: &mut ph, frame: 0, paused_frames: 0, prev_paused: 0, world: &NoWorld, notify: &mut notes };
    let mut q = [0u32; 27];
    m.GetParams(&ctx, &mut q);
    assert_eq!(q[11], 2);
    assert_eq!(f32::from_bits(q[13]), -2.0);
    assert_eq!(q[18], 0x55);
}

#[test]
fn ray_misses_outside_edges_and_mask() {
    let mut ph = phys_at([3.0, 2.0, 0.0]);
    let mut notes = Vec::new();
    let mut ctx = DMCtx { phys: &mut ph, frame: 0, paused_frames: 0, prev_paused: 0, world: &NoWorld, notify: &mut notes };
    let mut m = EFDMRayIntersect::new();
    m.base.state = 1;
    m.base.notify_bits = 1;
    m.dir = [0.0, -4.0, 0.0];
    m.map_mask1 = 1;
    m.PostColUpdateWith(&mut ctx, true, &[floor_quad()], &mut WorldRayHost(&NoWorld));
    assert_eq!((m.result, m.base.state, notes.len()), (0, 0, 1));
    // inside but mask mismatch
    let mut ph = phys_at([0.0, 2.0, 0.0]);
    let mut ctx = DMCtx { phys: &mut ph, frame: 0, paused_frames: 0, prev_paused: 0, world: &NoWorld, notify: &mut notes };
    m.map_mask1 = 2;
    m.PostColUpdateWith(&mut ctx, true, &[floor_quad()], &mut WorldRayHost(&NoWorld));
    assert_eq!(m.result, 0);
    // segment too short (t >= 1)
    m.map_mask1 = 1;
    m.dir = [0.0, -1.0, 0.0];
    m.PostColUpdateWith(&mut ctx, true, &[floor_quad()], &mut WorldRayHost(&NoWorld));
    assert_eq!(m.result, 0);
}

#[test]
fn ray_precol_radius() {
    let mut ph = phys_at([0.0; 3]);
    let mut notes = Vec::new();
    let mut ctx = DMCtx { phys: &mut ph, frame: 0, paused_frames: 0, prev_paused: 0, world: &NoWorld, notify: &mut notes };
    let mut m = EFDMRayIntersect::new();
    m.origin = [0.0, 0.5, 0.0];
    m.dir = [0.0, -0.2, 0.0];
    // both ends inside r = 1: radius stays, IntersectCollision path
    let mut dp = EFDynamicPhysic { radius: 1.0, ..Default::default() };
    assert!(m.PreColUpdate(&mut ctx, &mut dp, 0.0));
    assert_eq!((dp.radius, m.use_all), (1.0, false));
    // end outside: -len -> use_all, radius unchanged
    m.dir = [0.0, -3.0, 0.0];
    m.PreColUpdate(&mut ctx, &mut dp, 0.0);
    assert_eq!((dp.radius, m.use_all), (1.0, true));
    // within the 0.0025 tolerance: len returned and radius grows to it
    m.origin = [0.0, 1.002, 0.0];
    m.dir = [0.0; 3];
    m.PreColUpdate(&mut ctx, &mut dp, 0.0);
    assert!(!m.use_all && (dp.radius - 1.002).abs() < 1e-6);
}

fn sphere_model(c: [f32; 3], r: f32, tag: u32) -> EFCollisionModel {
    let mut b = EFObjCollisionPrim::new([[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [c[0], c[1], c[2], 1.0]]);
    b.radius = r;
    b.tag = tag;
    b.center = c;
    for (o, v) in [(0xc, 1), (0x10, 1), (0x14, 1), (0x18, 1), (0x20, !0), (0x24, !0)] {
        b.set_word(o, v);
    }
    let mut cm = EFCollisionModel::new(1);
    cm.prims = vec![Some(ObjPrim { base: b, ext: PrimExt::Sphere })];
    cm.masks = [!0; 6];
    cm.primary = 0;
    cm.bsphere_world = c;
    cm.bsphere_radius = r;
    cm
}

#[test]
fn ray_object_hit_through_manager() {
    let mut mgr = EFPhysicMgr::new();
    let mut f = RwMatrix::identity();
    f.pos = [0.0, 0.0, 5.0];
    let id = mgr.CreatePhysic(0x42, f, 0);
    mgr.PhysicMut(id).unwrap().model = sphere_model([0.0, 0.0, 5.0], 1.0, 0xabc);
    let mut m = EFDMRayIntersect::new();
    m.dir = [0.0, 0.0, 10.0];
    m.obj_mask1 = 1;
    let mut cs = EFCollisionSystem::new();
    let mut host = MgrRayHost { mgr: &mut mgr, cs: &mut cs };
    let mut t = 1.0;
    m.IntersectObjCollision(&mut host, &mut t);
    assert_eq!(m.result, 1);
    assert_eq!(m.info[0], 0x42);
    assert_eq!(m.info[1], 0xabc);
    assert!((t - 0.4).abs() < 1e-4, "t {t}");
    assert!((m.pos[2] - 4.0).abs() < 1e-3 && m.normal[2] < -0.9, "{:?} {:?}", m.pos, m.normal);
    // excluded owner never hits
    m.result = 0;
    m.exclude = 0x42;
    let mut t = 1.0;
    m.IntersectObjCollision(&mut host, &mut t);
    assert_eq!(m.result, 0);
}

fn ep1l01() -> Option<Vec<u8>> {
    let fs = IsoFs::open(ISO).ok()?;
    let mut m = EFRessourcesMgr::default();
    m.level_file_name = Some("EP1L01.P2R".into());
    m.LoadLevel(&fs, "", "EP1/L01", "EN", &mut RawInstanciator, |_, _| {}).ok()?;
    let raw = m.objects.iter().filter_map(|o| o.as_raw()).find(|r| r.class_id == 0x24).map(|r| r.data.clone())?;
    // floor point from a game object
    let mut map = EFCollisionMap::LoadFromRaw(&raw).ok()?.0;
    for (k, g) in m.game_objects.iter().enumerate() {
        let p = g.glo.pos;
        if p == [0.0; 3] { continue; }
        if let Some((hp, n, _)) = FindFloor(&mut map, [p[0], p[1] + 1.0, p[2]], 20.0, 1 + k as i32) {
            if n[1] > 0.9 {
                let mut v = raw.clone();
                v.extend_from_slice(&hp[0].to_le_bytes());
                v.extend_from_slice(&hp[1].to_le_bytes());
                v.extend_from_slice(&hp[2].to_le_bytes());
                return Some(v);
            }
        }
    }
    None
}

/// Real EP1L01: the module's IntersectAll (EFPhysicMgr::RayIntersect) and its IntersectEnvCollision
/// over the map prims around the floor point agree on the hit, and EnvColl touches that floor.
#[test]
fn real_map_ray_and_envcoll() {
    let Some(v) = ep1l01() else { eprintln!("ISO missing, skipped"); return };
    let n = v.len() - 12;
    let fl = |i: usize| f32::from_le_bytes(v[n + i * 4..n + i * 4 + 4].try_into().unwrap());
    let floor = [fl(0), fl(1), fl(2)];
    let raw = &v[..n];
    let mut mgr = EFPhysicMgr::new();
    mgr.maps.push(EFCollisionMap::LoadFromRaw(raw).unwrap().0);

    let origin = [floor[0], floor[1] + 1.0, floor[2]];
    let mut ph = phys_at(origin);
    let mut notes = Vec::new();
    let mut m = EFDMRayIntersect::new();
    m.dir = [0.0, -2.0, 0.0];
    m.map_mask1 = !0;
    m.map_mask2 = !0;
    m.use_all = true;
    {
        let mut cs = NoCollision;
        let mut host = MgrRayHost { mgr: &mut mgr, cs: &mut cs };
        let mut ctx = DMCtx { phys: &mut ph, frame: 0, paused_frames: 0, prev_paused: 0, world: &NoWorld, notify: &mut notes };
        m.PostColUpdateWith(&mut ctx, true, &[], &mut host);
    }
    assert_eq!(m.result, 2);
    assert!((m.pos[1] + 1.0).abs() < 1e-3, "local hit {:?}", m.pos);
    assert!(m.normal[1] > 0.9);
    let all = (m.pos, m.info);

    // env path over the prims of the real map around the floor point
    let map = &mut mgr.maps[0];
    let mut d = EFGetPrimitivesInSphereData { center: floor, radius: 1.5, mask1: !0, mask2: !0, out: Vec::new(), capacity: 256 };
    map.GetPrimitivesInSphere(&mut d, 0x7fff_0000);
    let env: Vec<EnvPrim> = d.out.iter().map(|&p| EnvPrim::from_map(map, p)).collect();
    assert!(!env.is_empty());
    m.use_all = false;
    let mut ctx = DMCtx { phys: &mut ph, frame: 0, paused_frames: 0, prev_paused: 0, world: &NoWorld, notify: &mut notes };
    m.PostColUpdateWith(&mut ctx, true, &env, &mut WorldRayHost(&NoWorld));
    assert_eq!(m.result, 2);
    assert!((m.pos[1] - all.0[1]).abs() < 1e-4, "{:?} vs {:?}", m.pos, all.0);
    assert_eq!(m.info, all.1, "same material record");

    // EnvColl: sphere r 0.5 resting 0.3 above the floor touches it (bit 0), then lifted -> bit 1
    let mut ph = phys_at(floor);
    ph.model = sphere_model([floor[0], floor[1] + 0.3, floor[2]], 0.5, 0x11);
    let mut e = EFDMEnvColl::new();
    e.base.notify_bits = 3;
    let mut ctx = DMCtx { phys: &mut ph, frame: 0, paused_frames: 0, prev_paused: 0, world: &NoWorld, notify: &mut notes };
    let mut p = [0u32; 8];
    p[0] = 0x11;
    p[4] = fw(1.0);
    p[5] = !0;
    p[7] = !0;
    e.SetParams(&mut ctx, Some(&p));
    assert_eq!(e.prim, 0);
    e.PostColUpdateWith(&mut ctx, true, &env);
    assert_eq!(e.base.state, 1);
    assert!(e.normal[1] > 0.9 && (e.pos[1] - floor[1]).abs() < 1e-3, "{:?}", e.pos);
    let mut a = [0u32; 14];
    e.DoAction(&mut ctx, 1, &mut a);
    assert_eq!(&a[3..11], &all.1[1..9]);
    ctx.phys.model.prims[0].as_mut().unwrap().base.center[1] += 2.0;
    e.PostColUpdateWith(&mut ctx, true, &env);
    assert_eq!(e.base.state, 2);
    assert_eq!(e.mat, None);
}

#[test]
fn envcoll_scale_restores_prim_and_edge_contact() {
    let mut ph = phys_at([0.0; 3]);
    // sphere just beyond the +x edge: centre (1.3, 0.1, 0), r 0.4 -> edge distance ~0.316
    ph.model = sphere_model([1.3, 0.1, 0.0], 0.4, 5);
    let mut notes = Vec::new();
    let mut ctx = DMCtx { phys: &mut ph, frame: 0, paused_frames: 0, prev_paused: 0, world: &NoWorld, notify: &mut notes };
    let mut e = EFDMEnvColl::new();
    e.prim = 0;
    e.mask1 = 1;
    e.scale = 0.5; // r 0.2 for the test -> miss
    e.PostColUpdateWith(&mut ctx, true, &[floor_quad()]);
    assert_eq!(e.base.state, 0);
    assert_eq!(ctx.phys.model.prims[0].as_ref().unwrap().base.radius, 0.4, "ResetCollPrim");
    e.scale = 1.0;
    e.PostColUpdateWith(&mut ctx, true, &[floor_quad()]);
    assert_eq!(e.base.state, 1);
    assert!((e.pos[0] - 1.0).abs() < 1e-6 && e.pos[1].abs() < 1e-6, "edge point {:?}", e.pos);
    // GetParams quirk: prim index 0 reports tag 0
    let mut q = [0u32; 8];
    e.GetParams(&ctx, &mut q);
    assert_eq!(q[0], 0);
}

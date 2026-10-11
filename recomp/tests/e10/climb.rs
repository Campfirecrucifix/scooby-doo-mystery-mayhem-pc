// WP-E10 EFDMClimb tests: real FindWeightedAvgNormal on synthetic + real EP1L01 env prims.
use scooby_recomp::engine::iso::IsoFs;
use scooby_recomp::engine::pack::*;
use scooby_recomp::engine::physic::collision_map::EFCollisionMap;
use scooby_recomp::engine::physic::dm::climb::*;
use scooby_recomp::engine::physic::floor::FindFloor;
use scooby_recomp::engine::physic::module::*;
use scooby_recomp::engine::physic::objcoll_check::EnvPrim;
use scooby_recomp::engine::physic::physic::EFPhysic;
use scooby_recomp::engine::physic::rwm::*;
use scooby_recomp::engine::physic::stage::EFCollisionStage as Stage;
const ISO: &str = "../Scooby Doo Mystery Mayhem.ISO";

struct W(Option<[f32; 2]>);
impl PhysicWorld for W {
    fn InputActionGet(&self, _a: u32, _b: u32) -> Option<[f32; 2]> { self.0 }
    fn GetCameraLTM(&self) -> RwMatrix { RwMatrix::from_pos([0.0, 2.0, -5.0]) }
}

/// Square polygon (side 2s) with normal +y at height y, edge planes facing outward.
fn quad(y: f32, s: f32, mat1: u32) -> EnvPrim {
    let verts = vec![[-s, y, -s], [-s, y, s], [s, y, s], [s, y, -s]];
    let edges = vec![[-1.0, 0.0, 0.0, -s], [0.0, 0.0, 1.0, -s], [1.0, 0.0, 0.0, -s], [0.0, 0.0, -1.0, -s]];
    let mut mat = [0u32; 8];
    mat[1] = mat1;
    EnvPrim { id: 0, mat, w4: 0, nverts: 4, normal: [0.0, 1.0, 0.0], d: -y, verts, edges }
}

fn run(m: &mut EFDMClimb, p: &mut EFPhysic, w: &W, st: &Stage, n: &mut Vec<PhysicNotify>) -> EFDynamicPhysic {
    let mut dp = EFDynamicPhysic { radius: 0.7, ..Default::default() };
    let mut ctx = DMCtx { phys: p, frame: 0, paused_frames: 0, prev_paused: 0, world: w, notify: n };
    assert!(m.PreColUpdate(&mut ctx, &mut dp, 0.1));
    m.PostColUpdateStage(&mut ctx, st, Some(&mut dp), 0.1);
    dp
}

#[test]
fn climb_params_and_defaults() {
    let m = EFDMClimb::new();
    assert_eq!(m.base.type_, 0x5000_0021);
    assert_eq!(m.pf(0x68), 1.75);
    assert_eq!(m.pf(0x48), 9.8);
    assert_eq!(m.params[0x10], 9999999);
    let mut n = vec![];
    let mut p = EFPhysic::new(1, RwMatrix::identity());
    let mut m = EFDMClimb::new();
    let mut ctx = DMCtx { phys: &mut p, frame: 0, paused_frames: 0, prev_paused: 0, world: &NoWorld, notify: &mut n };
    let d = m.params;
    m.SetParams(&mut ctx, Some(&d));
    assert_eq!(m.gravity, [0.0, -9.8, 0.0]);
    assert_eq!(m.probes[0].offset, 0.5);
    assert_eq!(m.probes[4].offset, -0.5);
    let mut out = [0u32; 29];
    m.GetParams(&ctx, &mut out);
    assert_eq!(out, d);
}

#[test]
fn climb_on_surface_and_falls_off() {
    let w = W(Some([0.0, 1.0]));
    let mut n = vec![];
    let mut p = EFPhysic::new(1, RwMatrix::from_pos([0.0, 0.0, 0.0]));
    let mut m = EFDMClimb::new();
    m.base.notify_bits = !0;
    {
        let mut ctx = DMCtx { phys: &mut p, frame: 0, paused_frames: 0, prev_paused: 0, world: &w, notify: &mut n };
        let d = m.params;
        m.SetParams(&mut ctx, Some(&d));
    }
    // surface 0.1 below the probes (probe height 0.05): within the 0.5 threshold -> state 1
    let st = Stage { env: vec![quad(-0.05, 5.0, 0xa00)], ..Default::default() };
    let dp = run(&mut m, &mut p, &w, &st, &mut n);
    assert_eq!(m.base.state, 1);
    assert!(m.probes.iter().all(|q| q.found == 1));
    assert!((m.normal[1] - 1.0).abs() < 1e-5);
    assert_eq!(p.model.bsphere_radius, 1.75);
    assert!(dp.disp[2] > 5.9, "stick forward pushes along at: {:?}", dp.disp);
    assert_eq!(n.last().unwrap().data[2], 1);
    // mask mismatch: nothing found (dist stays 11) -> state 2, radius restored
    let st = Stage { env: vec![quad(-0.05, 5.0, 0x1)], ..Default::default() };
    run(&mut m, &mut p, &w, &st, &mut n);
    assert_eq!(m.base.state, 2);
    assert_eq!(m.probes[2].dist, 11.0);
    assert_eq!(p.model.bsphere_radius, 0.7);
    assert_eq!(n.last().unwrap().data[2], 2);
}

#[test]
fn climb_rotates_up_towards_tilted_wall() {
    let w = W(None);
    let mut n = vec![];
    let mut p = EFPhysic::new(1, RwMatrix::identity());
    let mut m = EFDMClimb::new();
    let mut e = quad(-0.05, 5.0, 0x200);
    // tilt the normal 30 degrees about z: RotateUpTowards limits the step to 6 rad/s * dt
    e.normal = [-(0.5f32), 0.8660254, 0.0];
    let st = Stage { env: vec![e], ..Default::default() };
    run(&mut m, &mut p, &w, &st, &mut n);
    assert_eq!(m.base.state, 1);
    let ang = p.frame.up[1].acos();
    assert!((ang - 0.5235988).abs() < 1e-3, "rotated by min(30deg, 0.6rad): {ang}");
    assert!(p.frame.up[0] < 0.0);
}

#[test]
fn climb_real_ep1l01_floor() {
    let Ok(fs) = IsoFs::open(ISO) else { eprintln!("ISO missing, skipped"); return };
    let mut r = EFRessourcesMgr::default();
    r.level_file_name = Some("EP1L01.P2R".into());
    assert_eq!(r.LoadLevel(&fs, "", "EP1/L01", "EN", &mut RawInstanciator, |_, _| {}).unwrap(), 0);
    let raw = r.objects.iter().filter_map(|o| o.as_raw()).find(|o| o.class_id == 0x24).map(|o| o.data.clone()).unwrap();
    let mut map = EFCollisionMap::LoadFromRaw(&raw).unwrap().0;
    let mut hit = None;
    for (k, g) in r.game_objects.iter().enumerate() {
        let q = g.glo.pos;
        if q == [0.0; 3] { continue; }
        if let Some(h) = FindFloor(&mut map, [q[0], q[1] + 1.0, q[2]], 20.0, 1 + k as i32) {
            if h.1[1] > 0.9 { hit = Some(h); break; }
        }
    }
    let (hp, nrm, prim) = hit.expect("floor");
    let mut e = EnvPrim::from_map(&map, prim);
    eprintln!("floor prim {prim} mat1 {:#x} normal {:?}", e.mat[1], e.normal);
    e.mat[1] |= 0xa00; // the module's fixed mask; the floor material may lack these bits
    let st = Stage { env: vec![e], ..Default::default() };
    let w = W(None);
    let mut n = vec![];
    let mut p = EFPhysic::new(1, RwMatrix::from_pos([hp[0], hp[1] + 0.1, hp[2]]));
    let mut m = EFDMClimb::new();
    for q in m.probes.iter_mut() { q.offset = 0.0; }
    run(&mut m, &mut p, &w, &st, &mut n);
    assert_eq!(m.probes[2].found, 1);
    assert!(m.probes[2].dist < 0.5, "dist {}", m.probes[2].dist);
    assert!(dot(m.normal, nrm) > 0.99);
    assert_eq!(m.base.state, 1);
}

use std::sync::{Arc, Mutex};
use scooby_recomp::engine::physic::dm::floating_object::*;
use scooby_recomp::engine::physic::dm::water_simulation::*;
use scooby_recomp::engine::physic::dm::NewModule;
use scooby_recomp::engine::physic::mgr::EFPhysicMgr;
use scooby_recomp::engine::physic::module::*;
use scooby_recomp::engine::physic::rwm::*;

/// 16x12 water grid, cell 1.0, origin (0, 2, 0).
struct Grid;
impl WaterRenderGrid for Grid {
    fn GetType(&self) -> u32 { 0x41 }
    fn UpdateGridInfo(&mut self) -> [f32; 9] { [0.0, 2.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0] }
    fn StepScale(&self, _i: u32) -> f32 { 1.0 }
    fn Dims(&self) -> (i32, i32) { (16, 12) }
}

/// object 7 moves from above the water into it
struct W(Mutex<V3>);
impl PhysicWorld for W {
    fn GetObjectFrame(&self, uid: u32) -> Option<RwMatrix> {
        (uid == 7).then(|| RwMatrix::from_pos(*self.0.lock().unwrap()))
    }
}

fn water_params() -> Vec<u32> {
    let mut p = vec![0u32; 15];
    p[0] = fw(0.98);
    p[1] = fw(0.2);
    p[2] = fw(1.0);
    p[7] = fw(0.2);
    p[8] = 1; // count
    p[9] = 1; // id list present
    p[13] = 2;
    p[14] = 7;
    p
}

#[test]
fn water_sim_disturb_and_propagate() {
    let world = W(Mutex::new([8.0, 3.0, 6.0]));
    let mut mgr = EFPhysicMgr::new();
    let pid = mgr.CreatePhysic(100, RwMatrix::identity(), 0);
    let mut m = EFDMWaterSimulation::new(0xc);
    m.grid = Some(Box::new(Grid));
    let sim = m.sim.clone();
    let h = mgr.AddModule(pid, Box::new(m));
    assert_ne!(h, 0);
    let p = water_params();
    mgr.WithModule(h, &world, |m, ctx| m.SetParams(ctx, Some(&p))).unwrap();
    assert_eq!(sim.read().unwrap().count, 1);
    let mut dp = EFDynamicPhysic::default();
    mgr.WithModule(h, &world, |m, ctx| m.PreColUpdate(ctx, &mut dp, 1.0 / 30.0)).unwrap();
    {
        let s = sim.read().unwrap();
        assert_eq!((s.w, s.h, s.n), (16, 12, 192));
        assert!(s.cur.iter().chain(s.prev.iter()).all(|&v| v == 0.0));
        assert_eq!(s.damp[17], 0.98);
        assert_eq!(s.damp[0], 0.0);
    }
    // move down into the water (level 2.0): impulse -min(len, 0.2*0.4)
    *world.0.lock().unwrap() = [8.0, 1.5, 6.0];
    mgr.notify.clear();
    mgr.WithModule(h, &world, |m, ctx| m.PreColUpdate(ctx, &mut dp, 1.0 / 30.0)).unwrap();
    let n: Vec<_> = mgr.notify.iter().filter(|n| n.code == 0xc).cloned().collect();
    assert_eq!(n.len(), 1);
    assert_eq!(n[0].owner, 7);
    assert_eq!(n[0].data, [100, fw(1.5 - 2.0), fw(3.0 - 2.0), 0]);
    {
        let s = sim.read().unwrap();
        let c = 8 + 6 * 16;
        assert!((s.prev[c] + 0.08).abs() < 1e-6);
        assert!((s.prev[c - 1] + 0.04).abs() < 1e-6);
        assert!((s.prev[c + 17] + 0.02).abs() < 1e-6);
        assert_eq!(s.tracked[0].pos, [8.0, 1.5, 6.0]);
    }
    // next step: the disturbance (now in the source buffer) spreads to the neighbours
    mgr.WithModule(h, &world, |m, ctx| m.PreColUpdate(ctx, &mut dp, 1.0 / 30.0)).unwrap();
    {
        let s = sim.read().unwrap();
        let mut hh = [0.0; 3];
        let mut nrm = [0.0; 3];
        assert_eq!(s.GetWaterValueAtThisPos([8.0, 0.0, 6.0], &mut nrm, &mut hh[0]), 0);
        assert!(hh[0] != 0.0);
        assert_eq!(s.GetWaterValueAtThisPos([1.0, 0.0, 6.0], &mut nrm, &mut hh[1]), 2);
        assert_eq!(nrm, [0.0, 1.0, 0.0]);
    }
    // DoAction(1) clears
    mgr.WithModule(h, &world, |m, ctx| m.DoAction(ctx, 1, &mut [])).unwrap();
    assert!(sim.read().unwrap().cur.iter().all(|&v| v == 0.0));
    // GetParams quirk: flags word = +0x54
    let mut g = vec![0u32; 14];
    mgr.WithModule(h, &world, |m, ctx| m.GetParams(ctx, &mut g)).unwrap();
    assert_eq!(g[13], 2);
    assert_eq!(g[8], 1);
}

#[test]
fn floating_object_bobs() {
    let world = W(Mutex::new([8.0, 3.0, 6.0]));
    let mut mgr = EFPhysicMgr::new();
    let pw = mgr.CreatePhysic(100, RwMatrix::identity(), 0);
    let mut m = EFDMWaterSimulation::new(0xc);
    m.grid = Some(Box::new(Grid));
    let hw = mgr.AddModule(pw, Box::new(m));
    let mut dp = EFDynamicPhysic::default();
    mgr.WithModule(hw, &world, |m, ctx| m.PreColUpdate(ctx, &mut dp, 1.0 / 30.0)).unwrap();
    assert!(NewModule(0x15).is_some() && NewModule(0xd).is_some());

    let pf = mgr.CreatePhysic(200, RwMatrix::identity(), 1);
    let mut f = EFDMFloatingObject::new();
    f.source = Some(Arc::new(WaterSimSnapshot(FindWaterSimModule(&mgr))));
    assert!(f.source.as_ref().unwrap().GetWaterSimModule().is_some());
    f.base.notify_bits = 1;
    let hf = mgr.AddModule(pf, Box::new(f));
    let mut dp = EFDynamicPhysic { pos: [8.0, 1.0, 6.0], ..Default::default() };
    mgr.notify.clear();
    mgr.WithModule(hf, &world, |m, ctx| m.PreColUpdate(ctx, &mut dp, 0.5)).unwrap();
    let p = mgr.Physic(pf).unwrap();
    // (1*0 + level 2 + offset 0 - 1) / 0.5
    assert!((p.velocity[1] - 2.0).abs() < 1e-6);
    assert_eq!(mgr.Module(hf).unwrap().base().state & 1, 1);
    assert!(mgr.notify.iter().any(|n| n.code == 6 && n.owner == 200));
    // above level + 0.1: out of the water
    let mut dp = EFDynamicPhysic { pos: [8.0, 2.5, 6.0], ..Default::default() };
    mgr.WithModule(hf, &world, |m, ctx| m.PreColUpdate(ctx, &mut dp, 0.5)).unwrap();
    assert_eq!(mgr.Module(hf).unwrap().base().state & 1, 0);
}

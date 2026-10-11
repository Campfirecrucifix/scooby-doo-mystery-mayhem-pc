//! WP-E8: remaining EFDM* dynamic modules.
use scooby_recomp::engine::physic::dm::NewModule;
use scooby_recomp::engine::physic::module::*;
use scooby_recomp::engine::physic::physic::*;
use scooby_recomp::engine::physic::rwm::*;

fn sp(tag: u32, c: V3) -> Option<ObjPrim> {
    use scooby_recomp::engine::physic::model::PrimExt;
    use scooby_recomp::engine::physic::objcoll_prim::*;
    let mut b = EFObjCollisionPrim::new(IDENTITY);
    b.tag = tag;
    b.center = c;
    Some(ObjPrim { base: b, ext: PrimExt::Sphere })
}
fn phys() -> EFPhysic { EFPhysic::new(5, RwMatrix::identity()) }
fn close(a: V3, b: V3) -> bool { (0..3).all(|i| (a[i] - b[i]).abs() < 1e-3) }

/// run `f` with a DMCtx over `p`
fn with<R>(p: &mut EFPhysic, frame: u32, w: &dyn PhysicWorld, f: impl FnOnce(&mut DMCtx) -> R) -> (R, Vec<PhysicNotify>) {
    let mut n = Vec::new();
    let r = {
        let mut c = DMCtx { phys: p, frame, paused_frames: 0, prev_paused: 0, world: w, notify: &mut n };
        f(&mut c)
    };
    (r, n)
}

#[test]
fn primmove_velocity_from_prim_motion() {
    let mut p = phys();
    p.model.prims = vec![sp(9, [1.0, 0.0, 0.0]), sp(10, [1.0, 0.0, 0.0])];
    let mut m = NewModule(0xb).unwrap();
    let mut dp = EFDynamicPhysic::default();
    with(&mut p, 5, &NoWorld, |c| {
        m.SetParams(c, Some(&[10]));
        let mut q = [0u32];
        m.GetParams(c, &mut q);
        assert_eq!(q[0], 10);
        m.PreColUpdate(c, &mut dp, 0.5); // frame gap -> re-seed, v = 0
    });
    assert!(close(p.velocity, [0.0; 3]));
    p.model.prims[1].as_mut().unwrap().base.center = [2.0, 0.0, 0.0];
    with(&mut p, 6, &NoWorld, |c| m.PreColUpdate(c, &mut dp, 0.5));
    // quirk: (prev - cur) / dt
    assert!(close(p.velocity, [-2.0, 0.0, 0.0]));
}

#[test]
fn wave_moves_between_points() {
    let mut p = phys();
    let mut m = NewModule(9).unwrap();
    let mut dp = EFDynamicPhysic::default();
    let mut prm = [0u32; 10];
    put_v(&mut prm, 3, [0.0, 0.0, 4.0]);
    prm[6] = fw(2.0);
    prm[7] = fw(0.0);
    prm[9] = 4;
    with(&mut p, 1, &NoWorld, |c| m.SetParams(c, Some(&prm)));
    // no accel: period = 4*half/speed = 4
    let mut out = [0u32; 10];
    with(&mut p, 1, &NoWorld, |c| m.GetParams(c, &mut out));
    assert_eq!(wf(&out, 8), 4.0);
    dp.pos = p.frame.pos;
    let (_, n) = with(&mut p, 1, &NoWorld, |c| m.PreColUpdate(c, &mut dp, 0.5));
    assert!(n.is_empty());
    assert!(close(p.velocity, [0.0, 0.0, 2.0]));
    // past the half period it comes back
    dp.pos = [0.0, 0.0, 4.0];
    for _ in 0..4 {
        with(&mut p, 1, &NoWorld, |c| m.PreColUpdate(c, &mut dp, 0.5));
    }
    assert!(p.velocity[2] < 0.0);
}

#[test]
fn constraint_rotation_snaps_at() {
    let mut p = phys();
    let mut m = NewModule(0xe).unwrap();
    m.base_mut().notify_bits = 1;
    let mut prm = [0u32; 10];
    put_v(&mut prm, 3, [1.0, 0.0, 0.0]);
    prm[9] = fw(100.0);
    let mut dp = EFDynamicPhysic::default();
    with(&mut p, 1, &NoWorld, |c| m.SetParams(c, Some(&prm)));
    let (_, n) = with(&mut p, 1, &NoWorld, |c| m.PreColUpdate(c, &mut dp, 0.1));
    assert!(close(p.frame.at, [1.0, 0.0, 0.0]));
    assert!(n.is_empty()); // the full 90° turn is not "small" yet
    let (_, n) = with(&mut p, 1, &NoWorld, |c| m.PreColUpdate(c, &mut dp, 0.1));
    assert_eq!(n.len(), 1);
    assert_eq!(m.base().state & 1, 1);
}

struct Ordered(Vec<u32>);
impl PhysicWorld for Ordered {
    fn world_order(&self) -> Option<Vec<u32>> { Some(self.0.clone()) }
}

#[test]
fn initframe_follows_world_order() {
    use scooby_recomp::engine::physic::mgr::*;
    let mut m = EFPhysicMgr::new();
    let a = m.CreatePhysic(1, RwMatrix::identity(), 0);
    let b = m.CreatePhysic(2, RwMatrix::identity(), 0);
    let mut ups = EFUpdatePhysicStruct::default();
    m.InitFrame(&mut ups, &Ordered(vec![1, 9, 2]));
    assert_eq!(ups.passes[0].iter().map(|d| d.physic).collect::<Vec<_>>(), vec![a, b]);
    m.InitFrame(&mut ups, &NoWorld);
    assert_eq!(ups.passes[0].iter().map(|d| d.physic).collect::<Vec<_>>(), vec![b, a]);
}

#[test]
fn plugin_notifies_callbacks() {
    let mut p = phys();
    let mut m = NewModule(0x25).unwrap();
    assert_eq!(m.base().type_ & 0xff, 0x25);
    let mut dp = EFDynamicPhysic::default();
    let (_, n) = with(&mut p, 1, &NoWorld, |c| {
        m.SetParams(c, Some(&[7, 0xffff_ffff]));
        m.PreColUpdate(c, &mut dp, 0.5);
        m.PostColUpdate(c, None, 0.5);
    });
    assert_eq!(n.len(), 1);
    assert_eq!((n[0].owner, n[0].code, n[0].data[0]), (5, 0x12, 7));
}

#[test]
fn objcoll_state_on_hit_and_release() {
    use scooby_recomp::engine::physic::dm::objcoll::EFDMObjColl;
    let mut p = phys();
    let mut m = EFDMObjColl::new();
    m.base.notify_bits = 3;
    let dp = EFDynamicPhysic::default();
    let st = EFCollisionStage::seam(&dp);
    let (_, n) = with(&mut p, 1, &NoWorld, |c| {
        m.SetParams(c, Some(&[1, 2, 9]));
        m.CollisionCB(&[0, 1, 9, 2]);
        m.PostColUpdate(c, Some(&st), 0.1);
        assert_eq!(m.base.state, 1);
        m.PostColUpdate(c, Some(&st), 0.1);
        assert_eq!(m.base.state, 2);
    });
    assert_eq!(n.len(), 2);
}

#[test]
fn envdetect_clamps_entries() {
    use scooby_recomp::engine::physic::dm::envdetect::*;
    let mut p = phys();
    let mut m = EFDMEnvDetect::new();
    with(&mut p, 1, &NoWorld, |c| m.SetParams(c, Some(&[3, 1])));
    let pr = EnvPrimSeam { attr: 4, plane: [0.0, 1.0, 0.0, 0.0], verts: vec![[0.0; 3]; 3], ..Default::default() };
    m.Detect(&[pr.clone(), pr]);
    let l = m.last.unwrap();
    assert_eq!((l.target, l.count, l.entries.len(), l.entries[0].2.len()), (5, 2, 1, 3));
}

#[test]
fn ropedetect_no_spline_clears() {
    let mut p = phys();
    let mut m = NewModule(0x1e).unwrap();
    m.base_mut().state = 2;
    let mut d = [0u32; 3];
    with(&mut p, 1, &NoWorld, |c| {
        m.SetParams(c, Some(&[0xffff_ffff; 5]));
        m.PostColUpdate(c, None, 0.1);
        m.GetData(c, &mut d);
    });
    assert_eq!(m.base().state, 0);
    assert_eq!(d[0], 0xffff_ffff);
}

#[test]
fn pfweighttilt_tilts_under_weight() {
    use scooby_recomp::engine::physic::dm::pfweighttilt::EFDMPFWeightTilt;
    let mut p = phys();
    let mut m = EFDMPFWeightTilt::new();
    let mut dp = EFDynamicPhysic::default();
    with(&mut p, 1, &NoWorld, |c| {
        m.SetParams(c, Some(&[(-0.5f32).to_bits(), 0.5f32.to_bits(), 0, 0, 1f32.to_bits(), 1f32.to_bits(), 0.1f32.to_bits(), 1f32.to_bits(), 1f32.to_bits(), 1f32.to_bits()]));
        m.Contact(c, -1.0, 9, 2.0, [0.0, 0.0, 1.0]);
        assert_eq!(m.count, 1);
    });
    with(&mut p, 2, &NoWorld, |c| m.PreColUpdate(c, &mut dp, 0.1));
    assert!(m.vel_a > 0.0);
    assert!(p.frame.at[1] != 0.0);
    assert!(close(p.frame.pos, [0.0; 3]));
}

#[test]
fn edge_snaps_to_ledge() {
    use scooby_recomp::engine::physic::dm::edge::*;
    use scooby_recomp::engine::physic::dm::envdetect::EnvPrimSeam;
    let mut p = phys();
    p.model.prims.push(sp(3, [1.2, 0.0, 0.0]));
    let floor = EnvPrimSeam {
        mask1: 1,
        plane: [0.0, 1.0, 0.0, 0.0],
        verts: vec![[1.0, 0.0, -1.0], [1.0, 0.0, 1.0], [-1.0, 0.0, 1.0], [-1.0, 0.0, -1.0]],
        edges: vec![[1.0, 0.0, 0.0, -1.0], [0.0, 0.0, 1.0, -1.0], [-1.0, 0.0, 0.0, -1.0], [0.0, 0.0, -1.0, -1.0]],
        ..Default::default()
    };
    let mut m = EFDMEdge::new();
    with(&mut p, 1, &NoWorld, |c| {
        m.SetParams(c, Some(&[3, 0, 0]));
        m.Edge(c, &[floor], EdgePrim { radius: 0.5, mask1: 1, mask2: 0 });
    });
    assert_eq!(m.base.state, 1);
    assert!(close(p.model.prims[0].as_ref().unwrap().base.center, [1.0, 0.0, 0.0]));
}

//! WP-E9: EFDM* physics modules batch (AnalogMotion, BoneTrackObject, CameraFlex, IKTwoJoints,
//! ParticleLauncher, Quadruped, Tung).
use scooby_recomp::engine::physic::dm::NewModule;
use scooby_recomp::engine::physic::module::*;
use scooby_recomp::engine::physic::physic::*;
use scooby_recomp::engine::physic::rwm::*;
use scooby_recomp::engine::physic::mgr::*;
use std::sync::Mutex;

fn phys() -> EFPhysic { EFPhysic::new(5, RwMatrix::identity()) }
fn close(a: V3, b: V3) -> bool { (0..3).all(|i| (a[i] - b[i]).abs() < 1e-3) }

fn with<R>(p: &mut EFPhysic, frame: u32, w: &dyn PhysicWorld, f: impl FnOnce(&mut DMCtx) -> R) -> (R, Vec<PhysicNotify>) {
    let mut n = Vec::new();
    let r = {
        let mut c = DMCtx { phys: p, frame, paused_frames: 0, prev_paused: 0, world: w, notify: &mut n };
        f(&mut c)
    };
    (r, n)
}

/// stick held right; camera = identity (looks down +z)
struct Stick([f32; 2]);
impl PhysicWorld for Stick {
    fn InputActionGet(&self, _a: u32, _b: u32) -> Option<[f32; 2]> { Some(self.0) }
}

#[test]
fn analog_motion_moves_and_turns_towards_stick() {
    let mut p = phys();
    let mut m = NewModule(6).unwrap();
    let w = Stick([0.0, 1.0]); // full forward
    let mut prm = [0u32; 13];
    with(&mut p, 1, &w, |c| {
        m.SetParams(c, None);
        m.GetParams(c, &mut prm);
    });
    assert_eq!(prm[7], 2);
    prm[2] = fw(4.0); // max speed
    let mut dp = EFDynamicPhysic::default();
    with(&mut p, 1, &w, |c| {
        m.SetParams(c, Some(&prm));
        m.PreColUpdate(c, &mut dp, 0.1);
    });
    // camera at = +z, owner up = +y: input forward projects to v = (0, 0, 1) -> motion along at
    assert!(close(dp.disp, [0.0, 0.0, 4.0]), "{:?}", dp.disp);
    // stick to the right: v = (-1, 0, 0) -> the owner turns (heading changes), motion stays local z
    let w = Stick([1.0, 0.0]);
    let mut dp = EFDynamicPhysic::default();
    with(&mut p, 2, &w, |c| m.PreColUpdate(c, &mut dp, 0.1));
    assert!(Heading(&p.frame).abs() > 0.05);
    assert!((RwV3dLength(dp.disp) - 4.0).abs() < 1e-3);
}

/// owner bone 1 at the origin (at = +z); target object at (1, 0, 1)
struct Bones(Mutex<RwMatrix>);
impl PhysicWorld for Bones {
    fn GetObjectFrame(&self, uid: u32) -> Option<RwMatrix> {
        (uid == 7).then(|| RwMatrix::from_pos([1.0, 0.0, 1.0]))
    }
    fn GetBone(&self, uid: u32, bone: u32) -> Option<(RwMatrix, RwMatrix)> {
        (uid == 5 && bone == 1).then(|| {
            let m = *self.0.lock().unwrap();
            (m, m)
        })
    }
    fn SetBoneModelling(&self, _uid: u32, _bone: u32, m: RwMatrix) { *self.0.lock().unwrap() = m; }
}

#[test]
fn bone_track_object_turns_bone_to_target() {
    let mut p = phys();
    let mut m = NewModule(0x1c).unwrap();
    let w = Bones(Mutex::new(RwMatrix::identity()));
    let mut prm = [0u32; 20];
    prm[0] = 1;
    prm[1] = 7;
    prm[14] = fw(0.0); // no blend: factor 1
    prm[15] = fw(10.0);
    prm[17] = fw(10.0); // max step
    prm[18] = fw(3.0); // max angle
    with(&mut p, 1, &w, |c| m.SetParams(c, Some(&prm)));
    let (_, n) = with(&mut p, 1, &w, |c| {
        m.base_mut().notify_bits = 4;
        m.PostColUpdate(c, None, 0.1)
    });
    assert_eq!(m.base().state & 4, 4);
    assert_eq!(n.len(), 1);
    let at = w.0.lock().unwrap().at;
    assert!(close(at, RwV3dNormalize([1.0, 0.0, 1.0])), "{at:?}");
    // unknown target -> 0x7ffffffe, no-op
    prm[1] = 99;
    with(&mut p, 2, &w, |c| m.SetParams(c, Some(&prm)));
    let mut q = [0u32; 20];
    with(&mut p, 2, &w, |c| m.GetParams(c, &mut q));
    assert_eq!(q[1], 0x7fff_fffe);
}

#[test]
fn camera_flex_orbit_keeps_distance() {
    struct W;
    impl PhysicWorld for W {
        fn GetObjectFrame(&self, uid: u32) -> Option<RwMatrix> { (uid == 7).then(RwMatrix::identity) }
        fn InputActionGet(&self, _a: u32, _b: u32) -> Option<[f32; 2]> { Some([1.0, 0.0]) }
    }
    let w = W;
    let mut n = Vec::new();
    let mut p = EFPhysic::new(5, RwMatrix::from_pos([0.0, 0.0, -2.0]));
    let mut m = NewModule(0x12).unwrap();
    let mut prm = [0u32; 30];
    prm[0] = 7; // target
    prm[2] = 4; // mode 4: fixed offset, orbit around the look point
    put_v(&mut prm, 4, [0.0, 1.0, -3.0]); // +0x8c offset
    prm[26] = fw(1.0); // orbit speed
    prm[28] = 2; // flags: orbit
    let mut dp = EFDynamicPhysic::default();
    {
        let mut ctx = DMCtx { phys: &mut p, frame: 0, paused_frames: 0, prev_paused: 0, world: &w, notify: &mut n };
        m.SetParams(&mut ctx, Some(&prm));
        assert!(m.PreColUpdate(&mut ctx, &mut dp, 0.5));
        let mut out = [0u32; 30];
        m.GetParams(&ctx, &mut out);
        let o = wv(&out, 4);
        // rotated 0.5 rad about +Y around the look point (0,0,0): radius 3 and height kept
        assert!(((o[0] * o[0] + o[2] * o[2]).sqrt() - 3.0).abs() < 1e-4, "{o:?}");
        assert_eq!(o[1], 1.0);
        assert!((o[0] - (-3.0f32) * 0.5f32.sin()).abs() < 1e-4 && (o[2] - (-3.0) * 0.5f32.cos()).abs() < 1e-4, "{o:?}");
    }
    // no spline -> stopped -> DM state bit0 set
    assert_eq!(m.base().state & 1, 1);
    // MoveToTarget produced a velocity
    assert!(RwV3dLength(p.velocity) > 0.0);
}

#[test]
fn ik_two_joints_bends_chain_to_pushed_prim() {
    // bone chain 0 (root) -> 1 (origin) -> 2 (+1y) -> 3 (+1y); prim 0 on bone 3
    struct Chain(Mutex<Vec<RwMatrix>>);
    impl Chain {
        fn ltm(&self, b: u32) -> RwMatrix {
            let m = self.0.lock().unwrap();
            let mut l = m[b as usize];
            let mut i = b as usize;
            while i > 0 { i -= 1; l = RwMatrix::Multiply(&l, &m[i]); }
            l
        }
    }
    impl PhysicWorld for Chain {
        fn GetBone(&self, _u: u32, b: u32) -> Option<(RwMatrix, RwMatrix)> {
            if b > 3 { return None; }
            let m = self.0.lock().unwrap()[b as usize];
            Some((m, self.ltm(b)))
        }
        fn GetBoneParent(&self, _u: u32, b: u32) -> Option<u32> { if b > 0 && b <= 3 { Some(b - 1) } else { None } }
        fn SetBoneModelling(&self, _u: u32, b: u32, m: RwMatrix) { self.0.lock().unwrap()[b as usize] = m; }
        fn StageContactPolys(&self, _o: u32) -> Vec<(V3, f32, Vec<V3>, Vec<[f32; 4]>)> {
            // plane x = -0.5 facing +x (prim at x=0 is 0.5 in front); one edge far away
            vec![([1.0, 0.0, 0.0], 0.5, vec![[0.0; 3]], vec![[0.0, 0.0, 1.0, -100.0]])]
        }
        fn StageContactPolyMaterials(&self, _o: u32) -> Vec<(u32, u32)> { vec![(0x800, 0)] }
    }
    let w = Chain(Mutex::new(vec![
        RwMatrix::identity(), RwMatrix::identity(),
        RwMatrix::from_pos([0.0, 1.0, 0.0]), RwMatrix::from_pos([0.0, 1.0, 0.0]),
    ]));
    let mut p = EFPhysic::new(5, RwMatrix::identity());
    {
        use scooby_recomp::engine::physic::model::*;
        use scooby_recomp::engine::physic::objcoll_prim::*;
        let mut b = EFObjCollisionPrim::new(IDENTITY);
        b.tag = 7;
        b.set_word(0xc, 0x100);
        b.center = [0.0, 2.0, 0.0];
        b.joint = 3;
        p.model.prims = vec![Some(ObjPrim { base: b, ext: PrimExt::Sphere })];
    }
    let mut n = Vec::new();
    let mut m = NewModule(0x20).unwrap();
    assert_eq!(m.base().type_, 0x4000_0020);
    let mut ctx = DMCtx { phys: &mut p, frame: 1, paused_frames: 0, prev_paused: 0, world: &w, notify: &mut n };
    m.SetParams(&mut ctx, Some(&[1, 7, 0xffff_ffff, 0xffff_ffff, 0xffff_ffff, 0, 0, 0]));
    let mut g = [0u32; 8];
    m.GetParams(&ctx, &mut g);
    assert_eq!(g, [1, 7, 0, 0, 0, 0, 0, 0]);
    m.PostColUpdate(&mut ctx, None, 1.0 / 60.0);
    let tip = w.ltm(3).pos;
    // target (-0.5, 2, 0) is out of reach (chain length 2): clamped onto the reach sphere
    let k = (2.0 - 0.0001) / (0.25f32 + 4.0).sqrt();
    assert!((tip[0] + 0.5 * k).abs() < 1e-3 && (tip[1] - 2.0 * k).abs() < 1e-3 && tip[2].abs() < 1e-3, "{tip:?}");
    m.SetParams(&mut ctx, None);
    m.GetParams(&ctx, &mut g);
    assert_eq!(g[0], 0);
}
#[test]
fn particle_launcher_launches_on_ray_hit() {
    struct W { calls: Mutex<Vec<(i32, RwMatrix)>>, rays: Mutex<Vec<EFRayIntersectIn>> }
    impl PhysicWorld for W {
        fn GetObjectFrame(&self, _uid: u32) -> Option<RwMatrix> { Some(RwMatrix::from_pos([0.0, 0.0, 1.0])) }
        fn RayIntersectResult(&self, rin: &EFRayIntersectIn) -> Option<EFRayResult> {
            self.rays.lock().unwrap().push(*rin);
            Some(EFRayResult { kind: 2, pos: [0.0, 0.0, 5.0], normal: [0.0, 0.0, -1.0], ..Default::default() })
        }
        fn LaunchParticle(&self, emitter: i32, setup: &mut dyn FnMut(&mut RwMatrix)) -> bool {
            let mut m = RwMatrix::identity();
            setup(&mut m);
            self.calls.lock().unwrap().push((emitter, m));
            true
        }
    }
    let w = W { calls: Mutex::new(vec![]), rays: Mutex::new(vec![]) };
    let mut p = phys();
    let mut m = NewModule(0x1d).unwrap();
    assert_eq!(m.base().type_, 0x5000_001d);
    assert_eq!(m.ParamWords(), 31);
    let mut prm = [0u32; 31];
    put_v(&mut prm, 3, [0.0, 0.0, 10.0]); // ray delta
    prm[8] = 1; // map mask
    put_v(&mut prm, 27, [0.0, 2.0, 0.0]); // up (normalized)
    prm[30] = 7; // emitter
    with(&mut p, 0, &w, |c| m.SetParams(c, Some(&prm)));
    let mut out = [0u32; 31];
    with(&mut p, 0, &w, |c| m.GetParams(c, &mut out));
    assert_eq!(wv(&out, 27), [0.0, 1.0, 0.0]);
    assert_eq!(out[30], 7);
    // far swept sphere -> ray starts outside -> IntersectAll path
    let mut dp = EFDynamicPhysic { sphere: [100.0, 0.0, 0.0], radius: 0.0, ..Default::default() };
    with(&mut p, 0, &w, |c| m.PreColUpdate(c, &mut dp, 0.0));
    // no stage: nothing
    with(&mut p, 0, &w, |c| m.PostColUpdate(c, None, 0.0));
    assert!(w.calls.lock().unwrap().is_empty());
    let st = EFCollisionStage::seam(&dp);
    with(&mut p, 0, &w, |c| m.PostColUpdate(c, Some(&st), 0.0));
    let rays = w.rays.lock().unwrap();
    assert_eq!(rays[0].map.origin, [0.0, 0.0, 1.0]); // world-space ray
    assert_eq!(rays[0].map.mask1, 1);
    let calls = w.calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    let (e, mt) = calls[0];
    assert_eq!(e, 7);
    let c = |a: V3, b: V3| (0..3).all(|i| (a[i] - b[i]).abs() < 1e-4);
    assert!(c(mt.pos, [0.0, 0.0, 4.999]), "{:?}", mt.pos); // world hit pos + n*0.001
    assert!(c(mt.at, [0.0, 0.0, -1.0]));
    assert!(c(mt.up, [0.0, 1.0, 0.0]));
    assert!(c(mt.right, [1.0, 0.0, 0.0]));
    // params restored to local, hit stored local
    with(&mut p, 0, &w, |c| m.GetParams(c, &mut out));
    assert_eq!(wv(&out, 0), [0.0, 0.0, 0.0]);
    assert_eq!(out[11], 2);
    assert!(c(wv(&out, 12), [0.0, 0.0, 4.0]));
    assert_eq!(m.base().state & 1, 1);
}

#[test]
fn quadruped_floor_tilt_and_bone_bend() {
    use std::sync::Mutex;
    struct W(Mutex<Vec<(u32, RwMatrix)>>);
    impl PhysicWorld for W {
        fn GetBone(&self, _u: u32, b: u32) -> Option<(RwMatrix, RwMatrix)> {
            if b > 2 { return None; }
            let m = self.0.lock().unwrap().iter().rev().find(|e| e.0 == b).map(|e| e.1).unwrap_or(RwMatrix::from_pos([0.0, 1.0, 0.0]));
            Some((m, m))
        }
        fn SetBoneModelling(&self, _u: u32, b: u32, m: RwMatrix) { self.0.lock().unwrap().push((b, m)); }
        fn StageContactPolys(&self, _o: u32) -> Vec<(V3, f32, Vec<V3>, Vec<[f32; 4]>)> {
            let n = RwV3dNormalize([0.0, 1.0, 0.3]);
            let v = vec![[-5.0, 0.0, -5.0], [5.0, 0.0, -5.0], [5.0, 0.0, 5.0], [-5.0, 0.0, 5.0]];
            vec![(n, 0.0, v, vec![[0.0, 0.0, 0.0, -1.0]; 4])]
        }
    }
    let w = W(Mutex::new(Vec::new()));
    let mut p = EFPhysic::new(5, RwMatrix::identity());
    let mut n = Vec::new();
    let mut m = NewModule(0x1f).unwrap();
    assert_eq!(m.base().type_, 0x5000_001f);
    assert_eq!(m.ParamWords(), 8);
    let dp = EFDynamicPhysic::default();
    let st = EFCollisionStage::seam(&dp);
    {
        let mut ctx = DMCtx { phys: &mut p, frame: 0, paused_frames: 0, prev_paused: 0, world: &w, notify: &mut n };
        let prm = [1u32, 2, fw(0.35), fw(0.35), fw(10.0), fw(0.1), 2, 3];
        m.SetParams(&mut ctx, Some(&prm));
        let mut g = [0u32; 8];
        m.GetParams(&ctx, &mut g);
        assert_eq!(g, prm);
        m.PostColUpdate(&mut ctx, Some(&st), 1.0);
        let mut d = [0u32; 6];
        m.DoAction(&mut ctx, 4, &mut d);
        assert!((wf(&d, 1) - 1.0 / 1.09f32.sqrt()).abs() < 1e-5);
        assert!((wf(&d, 4) - 1.0 / 1.09f32.sqrt()).abs() < 1e-5);
        let mut dp2 = EFDynamicPhysic::default();
        assert!(m.PreColUpdate(&mut ctx, &mut dp2, 1.0));
    }
    let wr = w.0.lock().unwrap().clone();
    let b0 = wr.iter().find(|e| e.0 == 0).expect("root bone rotated").1;
    assert_eq!(b0.pos, [0.0, 1.0, 0.0]);
    assert!((b0.up[1] - 0.1f32.cos()).abs() < 1e-4);
    let mut ctx = DMCtx { phys: &mut p, frame: 1, paused_frames: 0, prev_paused: 0, world: &w, notify: &mut n };
    for _ in 0..4 {
        ctx.phys.frame.Rotate(30.0, [0.0, 1.0, 0.0], 2);
        m.PostColUpdate(&mut ctx, Some(&st), 1.0);
    }
    let wr = w.0.lock().unwrap().clone();
    assert!(wr.iter().any(|e| e.0 == 1) && wr.iter().any(|e| e.0 == 2));
    m.DoAction(&mut ctx, 1, &mut []);
    m.PostColUpdate(&mut ctx, Some(&st), 1.0);
    m.SetParams(&mut ctx, None);
    let mut g = [0u32; 8];
    m.GetParams(&ctx, &mut g);
    assert_eq!(g, [0, 0, fw(0.35), fw(0.35), fw(10.0), fw(0.1), 2, 0]);
}

#[test]
fn tung_extend_and_retract() {
    use std::sync::Mutex;
    struct W { m: Mutex<Vec<RwMatrix>> }
    impl PhysicWorld for W {
        fn GetBone(&self, _u: u32, b: u32) -> Option<(RwMatrix, RwMatrix)> {
            let m = self.m.lock().unwrap();
            let i = b as usize;
            if i >= m.len() { return None; }
            let mut ltm = m[0];
            for k in 1..=i { ltm = RwMatrix::Multiply(&m[k], &ltm); }
            Some((m[i], ltm))
        }
        fn SetBoneModelling(&self, _u: u32, b: u32, x: RwMatrix) { self.m.lock().unwrap()[b as usize] = x; }
    }
    let w = W { m: Mutex::new(vec![RwMatrix::identity(); 3]) };
    let mut p = EFPhysic::new(5, RwMatrix::identity());
    let mut n = Vec::new();
    let frame = 0;
    let mut dm = NewModule(0x24).unwrap();
    assert_eq!(dm.base().type_, 0x4000_0024);
    assert_eq!(dm.base().state, 1);
    let mut ctx = DMCtx { phys: &mut p, frame, paused_frames: 0, prev_paused: 0, world: &w, notify: &mut n };
    let mut prm = vec![0u32; dm.ParamWords()];
    dm.GetParams(&ctx, &mut prm);
    assert_eq!(f32::from_bits(prm[2]), 8.0);
    prm[11] = 1;
    prm[12..15].copy_from_slice(&[0f32.to_bits(), 0f32.to_bits(), 10f32.to_bits()]);
    dm.SetParams(&mut ctx, Some(&prm));
    dm.PostColUpdate(&mut ctx, None, 0.5); // 1 -> 2, tip moves 1.5 along +z
    assert_eq!(dm.base().state, 2);
    let tip = w.m.lock().unwrap()[2].pos;
    assert!((tip[2] - 1.5).abs() < 1e-5, "{:?}", tip);
    for _ in 0..4 { dm.PostColUpdate(&mut ctx, None, 0.5); }
    assert_eq!(dm.base().state, 4);
    dm.PostColUpdate(&mut ctx, None, 0.1);
    assert_eq!(dm.base().state, 8);
    for _ in 0..200 { dm.PostColUpdate(&mut ctx, None, 0.1); if dm.base().state == 1 { break; } }
    assert_eq!(dm.base().state, 1);
}

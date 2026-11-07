//! WP-E7: EFDMFollow (0x14), EFDMSplineMove (3), APIphysic prim attribs / offsets / sphere
//! volume / JumpOnCoor / CheckRayIntersection packing.
use std::sync::atomic::{AtomicU32, Ordering};

use scooby_recomp::engine::physic::api;
use scooby_recomp::engine::physic::mgr::*;
use scooby_recomp::engine::physic::module::*;
use scooby_recomp::engine::physic::physic::*;
use scooby_recomp::engine::physic::rwm::*;
use scooby_recomp::engine::spline::{EFQuadraticSpline, SplineSegment};

struct W {
    leader: RwMatrix,
    rays: AtomicU32,
    splines: Vec<EFQuadraticSpline>,
}
impl PhysicWorld for W {
    fn GetObjectFrame(&self, uid: u32) -> Option<RwMatrix> { (uid == 77).then_some(self.leader) }
    fn RayIntersect(&self, rin: &EFRayIntersectIn) -> bool {
        assert_eq!(rin.map.mask1, 3);
        assert_eq!(rin.exclude, 5);
        self.rays.fetch_add(1, Ordering::Relaxed);
        false
    }
    fn Splines(&self) -> &[EFQuadraticSpline] { &self.splines }
}
fn world() -> W {
    // straight 2-segment spline along +z, 10 long (lc = 0 -> linear arc length)
    let seg = |z: f32| SplineSegment { p: [[0.0, 0.0, z], [0.0, 0.0, z + 2.5], [0.0, 0.0, z + 5.0]], length: 5.0, distance: z, b: [0.0, 0.0, 5.0], ..Default::default() };
    let sp = EFQuadraticSpline { id: 42, length: 10.0, segs: vec![seg(0.0), seg(5.0)], ..Default::default() };
    W { leader: RwMatrix::from_pos([8.0, 0.0, 6.0]), rays: AtomicU32::new(0), splines: vec![sp] }
}
fn setup() -> (EFPhysicMgr, PhysId) {
    let mut m = EFPhysicMgr::new();
    let id = api::CreatePhysic(&mut m, 5, 0, RwMatrix::identity());
    m.PhysicMut(id).unwrap().context = 1;
    api::SetFlags(&mut m, 5, 2);
    (m, id)
}
fn close(a: V3, b: V3) -> bool { (0..3).all(|i| (a[i] - b[i]).abs() < 1e-3) }

#[test]
fn follow_turns_and_moves_toward_leader() {
    let (mut m, id) = setup();
    let w = world();
    let h = api::DMAdd(&mut m, 5, 0x14);
    assert_ne!(h, 0);
    api::DMSetContextMask(&mut m, h, 1);
    let def = api::DMGetParams(&mut m, h, &w).unwrap();
    assert_eq!(def.len(), 8);
    assert_eq!(def[1], fw(1.0));
    assert_eq!(def[5], fw(7.0));
    // hero's block (heroFUNC_CreateFollowPhysic)
    let p = [77, 0x3fe0_0000, 0x4020_0000, 0, 0x3e80_0000, 0x3d99_999a, fw(2.2), 1];
    api::DMSetParams(&mut m, h, Some(&p), &w);
    let g = api::DMGetParams(&mut m, h, &w).unwrap();
    assert_eq!(&g[..7], &[77, 0x3fe0_0000, 0x4020_0000, 0, 0x3e80_0000, 0x3d99_999a, fw(2.2)]);
    m.Update(0.5, false, &w, &mut NoCollision);
    assert!(w.rays.load(Ordering::Relaxed) >= 1);
    let f = m.Physic(id).unwrap().frame;
    // DeltaForce(0.25, 0.5) = 1 -> facing the follow point (leader direction (0.8, 0, 0.6)).
    // (quirk: a leader exactly on the x or z axis line of the owner counts as "arrived")
    assert!(close(f.at, [0.8, 0.0, 0.6]), "{:?}", f.at);
    assert!(close(f.up, [0.0, 1.0, 0.0]), "{:?}", f.up);
    assert_eq!(api::DMGetState(&m, h) & 1, 0);
    // leader close by -> "arrived" bit 0
    let w2 = W { leader: RwMatrix::from_pos([0.8, 0.0, 0.6]), ..world() };
    m.Update(0.5, false, &w2, &mut NoCollision);
    assert_eq!(api::DMGetState(&m, h) & 1, 1);
    api::DMDoAction(&mut m, h, 1, &mut [], &w2);
}

#[test]
fn spline_move_follows_spline() {
    let (mut m, id) = setup();
    let w = world();
    m.PhysicMut(id).unwrap().frame.pos = [3.0, 0.0, 2.0];
    let h = api::DMAdd(&mut m, 5, 3);
    assert_ne!(h, 0);
    api::DMSetContextMask(&mut m, h, 1);
    let mut p = api::DMGetParams(&mut m, h, &w).unwrap();
    assert_eq!(p.len(), 15);
    assert_eq!(p[10], 0x2d);
    assert_eq!(p[11], 1);
    // speed 2 (min = max), no nearest search, no rotations
    p[4] = fw(2.0);
    p[5] = fw(2.0);
    p[10] = 0x80;
    api::DMSetParams(&mut m, h, Some(&p), &w);
    // attach by id, from the start
    api::DMDoAction(&mut m, h, 2, &mut [42, 1], &w);
    let f = m.Physic(id).unwrap().frame;
    assert!(close(f.pos, [0.0, 0.0, 0.0]), "{:?}", f.pos);
    assert!(close(f.at, [0.0, 0.0, 1.0]));
    m.Update(0.5, false, &w, &mut NoCollision);
    let ph = m.Physic(id).unwrap();
    // mode 1: velocity = displacement (1 unit along the spline in 0.5 s)
    assert!(close(ph.velocity, [0.0, 0.0, 2.0]), "{:?}", ph.velocity);
    assert!(close(ph.frame.pos, [0.0, 0.0, 1.0]), "{:?}", ph.frame.pos);
    assert_eq!(api::DMGetState(&m, h) & 2, 0);
    for _ in 0..10 {
        m.Update(0.5, false, &w, &mut NoCollision);
    }
    assert_eq!(api::DMGetState(&m, h) & 2, 2);
}

#[test]
fn prim_attribs_offsets_sphere() {
    let (mut m, id) = setup();
    use scooby_recomp::engine::physic::model::*;
    use scooby_recomp::engine::physic::objcoll_prim::*;
    let mut b = EFObjCollisionPrim::new(IDENTITY);
    b.tag = 9;
    b.center = [0.0; 3];
    b.words[0] = 0x10;
    b.words[11] = 0x10; // base of word 0
    b.words[11 + 5] = 0x3;
    b.words[5] = 0x3;
    m.PhysicMut(id).unwrap().model.prims = vec![Some(ObjPrim { base: b, ext: PrimExt::Sphere })];
    api::PrimSetIntAttribBits(&mut m, 5, 9, 5, 0x200);
    assert_eq!(api::PrimGetIntAttrib(&mut m, 5, 9, 5), 0x203);
    api::PrimResetIntAttribBits(&mut m, 5, 9, 5, 0x1);
    assert_eq!(api::PrimGetIntAttrib(&mut m, 5, 9, 5), 0x202);
    api::PrimUnlockIntAttrib(&mut m, 5, 9, 5, 0x201);
    assert_eq!(api::PrimGetIntAttrib(&mut m, 5, 9, 5), 0x3);
    api::PrimSetIntAttribValue(&mut m, 5, 9, 0, 0x7);
    assert_eq!(m.Physic(id).unwrap().model.prims[0].as_ref().unwrap().base.words[0], 0x7);
    assert_eq!(api::PrimGetIntAttrib(&mut m, 5, 9, 0), 0x7);
    assert_eq!(api::PrimGetIntAttrib(&mut m, 5, 8, 0), 0);
    assert_eq!(api::PrimSetOffsetFromJoint(&mut m, 5, 9, [1.0, 2.0, 3.0]), 0);
    assert_eq!(api::PrimGetOffsetFromJoint(&mut m, 5, 9), Some([1.0, 2.0, 3.0]));
    assert_eq!(m.Physic(id).unwrap().model.prims[0].as_ref().unwrap().base.center, [1.0, 2.0, 3.0]);
    assert_eq!(api::PrimSetOffsetFromJoint(&mut m, 5, 1, [0.0; 3]), 0xffff_ffff);
    api::SetSpherePrimaryVolume(&mut m, 5, 0.5, 0.0, 1.0, 0.0);
    let md = &m.Physic(id).unwrap().model;
    assert_eq!(md.prims.len(), 1);
    assert_eq!(md.prims[0].as_ref().unwrap().base.radius, 0.5);
    assert_eq!(md.b3c, 1);
    assert_eq!(md.bsphere_radius, 0.5);
    assert_eq!(md.bsphere_center, [0.0, 1.0, 0.0]);
}

#[test]
fn ray_out_no_hit_untouched() {
    // P32 R83: the JumpOnCoor half tested the removed partial physic::api copy (runtime: host/physic.rs)
    let (mut m, _id) = setup();
    let w = world();
    // no hit -> out untouched
    let rin = EFRayIntersectIn { exclude: 0x2a00_0000, ..Default::default() };
    assert!(api::CheckRayIntersectionOut(&mut m, rin, &w, &mut NoCollision).is_none());
}

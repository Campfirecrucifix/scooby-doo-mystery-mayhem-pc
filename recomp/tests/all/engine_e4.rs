use scooby_recomp::engine::physic::model::*;
use scooby_recomp::engine::physic::objcoll_check::*;
use scooby_recomp::engine::physic::objcoll_prim::*;
use scooby_recomp::engine::physic::prim_anim::*;
use scooby_recomp::engine::physic::report::*;

fn ltm(p: [f32; 3]) -> [[f32; 4]; 4] {
    let mut m = IDENTITY;
    m[3] = [p[0], p[1], p[2], 1.0];
    m
}
fn sphere(p: [f32; 3], r: f32) -> ObjPrim {
    let mut m = EFCollisionModel::new(1);
    let s = m.AddPrimitive(0, 1, PRIM_SPHERE, ltm(p)).unwrap();
    s.base.radius = r;
    s.PrecalcPrimData();
    s.clone()
}
fn bbox(p: [f32; 3], h: [f32; 3]) -> ObjPrim {
    let mut m = EFCollisionModel::new(1);
    let b = m.AddPrimitive(0, 2, PRIM_BBOX, ltm(p)).unwrap();
    if let PrimExt::BBox { half, .. } = &mut b.ext {
        *half = h;
    }
    b.PrecalcPrimData();
    b.clone()
}

#[test]
fn sphere_sphere() {
    let a = sphere([0.0; 3], 1.0);
    let b = sphere([3.0, 0.0, 0.0], 1.0);
    let mut o = EFCheckCollisionOut::default();
    let i = EFCheckCollisionIn { tmax: 1.0, motion: [2.0, 0.0, 0.0] };
    assert_eq!(a.CheckObjPrimCollision(&i, &mut o, &b), 1);
    assert!((o.t - 0.5).abs() < 1e-5, "{o:?}");
    let c = sphere([1.5, 0.0, 0.0], 1.0);
    assert_eq!(a.CheckObjPrimCollision(&i, &mut o, &c), 2);
    assert!((o.depth - 0.5).abs() < 1e-5);
}

#[test]
fn bbox_precalc_and_point() {
    let b = bbox([10.0, 0.0, 0.0], [1.0, 2.0, 3.0]);
    let PrimExt::BBox { m, inv, .. } = &b.ext else { panic!() };
    assert_eq!(m[3][0], 10.0);
    assert_eq!(inv[3][0], -10.0);
    assert!(b.CheckPointIntersection([10.5, 1.9, -2.9]));
    assert!(!b.CheckPointIntersection([12.0, 0.0, 0.0]));
    assert_eq!(b.GetMinRadius(), 1.0);
}

#[test]
fn sphere_vs_bbox_face_and_swap() {
    let s = sphere([-5.0, 0.0, 0.0], 1.0);
    let b = bbox([0.0; 3], [1.0, 1.0, 1.0]);
    let i = EFCheckCollisionIn { tmax: 1.0, motion: [6.0, 0.0, 0.0] };
    let mut o = EFCheckCollisionOut::default();
    assert_eq!(s.CheckObjPrimCollision(&i, &mut o, &b), 1);
    assert!((o.t - 0.5).abs() < 1e-5, "{o:?}");
    assert_eq!(o.normal, [-1.0, 0.0, 0.0]);
    // swapped: box moving towards the sphere
    let i2 = EFCheckCollisionIn { tmax: 1.0, motion: [-6.0, 0.0, 0.0] };
    let mut o2 = EFCheckCollisionOut::default();
    assert_eq!(b.CheckObjPrimCollision(&i2, &mut o2, &s), 1);
    assert_eq!(o2.normal, [1.0, 0.0, 0.0]);
    // overlapping
    let s2 = sphere([1.5, 0.0, 0.0], 1.0);
    assert_eq!(s2.CheckObjPrimCollision(&i, &mut o, &b), 2);
    assert!((o.depth - 0.5).abs() < 1e-5);
}

#[test]
fn polygon_min_radius_and_getcommand() {
    let mut m = EFCollisionModel::new(1);
    let p = m.AddPrimitive(0, 3, PRIM_POLYGON, ltm([1.0, 2.0, 3.0])).unwrap();
    p.PrecalcPrimData();
    assert_eq!(p.GetMinRadius(), 1.0);
    assert!(!p.CheckPointIntersection([1.0, 2.0, 3.0]));
    let a = EFCollisionPrimAnim::default();
    assert!(a.GetCommand(0, &mut 0.0).is_none());
}

#[test]
fn report_lists() {
    let ar: Vec<EFCollisionReport> = [0.5, 0.2, 0.9, 0.2].iter().map(|&t| EFCollisionReport { t, ..Default::default() }).collect();
    let mut l = vec![];
    for i in 0..4 {
        EFCollisionReport::InsertTo(&ar, i, &mut l);
    }
    assert_eq!(l, vec![3, 1, 0, 2]);
    let n = EFCollisionReport::CheckNotify((7, ReportPrim { tag: 1, flags: 4, user: [0; 4] }), Some((9, None)), [0.0; 3], [1.0; 3]);
    assert_eq!(n.len(), 2);
    assert_eq!((n[0].0, n[0].1, n[1].1), (7, 0x17, 3));
}

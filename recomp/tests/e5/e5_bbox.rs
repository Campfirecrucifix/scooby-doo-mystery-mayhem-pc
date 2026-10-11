use scooby_recomp::engine::physic::objcoll_check::bbox::*;
use scooby_recomp::engine::physic::collision_map::*;
use scooby_recomp::engine::physic::model::*;
use scooby_recomp::engine::physic::objcoll_check::*;
use scooby_recomp::engine::physic::objcoll_prim::*;

fn ltm(p: [f32; 3]) -> [[f32; 4]; 4] {
    let mut m = IDENTITY;
    m[3] = [p[0], p[1], p[2], 1.0];
    m
}
fn bbox(p: [f32; 3], h: [f32; 3]) -> ObjPrim {
    let mut m = EFCollisionModel::new(1);
    let b = m.AddPrimitive(0, 2, PRIM_BBOX, ltm(p)).unwrap();
    if let PrimExt::BBox { half, .. } = &mut b.ext {
        *half = h;
    }
    b.base.radius = length(h);
    b.PrecalcPrimData();
    b.clone()
}
fn poly(p: [f32; 3]) -> ObjPrim {
    let mut m = EFCollisionModel::new(1);
    let b = m.AddPrimitive(0, 3, PRIM_POLYGON, ltm(p)).unwrap();
    b.PrecalcPrimData();
    b.clone()
}
/// 2x2 floor at y=0 (normal +y), edge planes outward.
fn floor(mat0: u32) -> EnvPrim {
    EnvPrim {
        id: 0,
        mat: [mat0, 0, 0, 0, 0, 0, 0, 0],
        w4: 0,
        nverts: 4,
        normal: [0.0, 1.0, 0.0],
        d: 0.0,
        verts: vec![[-1.0, 0.0, -1.0], [-1.0, 0.0, 1.0], [1.0, 0.0, 1.0], [1.0, 0.0, -1.0]],
        edges: vec![[-1.0, 0.0, 0.0, -1.0], [0.0, 0.0, 1.0, -1.0], [1.0, 0.0, 0.0, -1.0], [0.0, 0.0, -1.0, -1.0]],
    }
}
fn near(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-4
}

#[test]
fn bbox_bbox() {
    let a = bbox([0.0; 3], [1.0; 3]);
    let b = bbox([5.0, 0.0, 0.0], [1.0; 3]);
    let i = EFCheckCollisionIn { tmax: 1.0, motion: [4.0, 0.0, 0.0] };
    let mut o = EFCheckCollisionOut::default();
    assert_eq!(a.CheckObjPrimCollision(&i, &mut o, &b), 1, "{o:?}");
    assert!(near(o.t, 0.75), "{o:?}");
    assert_eq!(o.normal, [-1.0, 0.0, 0.0]);
    assert!(near(o.motion[0], 3.0));
    // overlapping
    let c = bbox([1.5, 0.0, 0.0], [1.0; 3]);
    assert_eq!(a.BBox_CheckBBoxCollision(&i, &mut o, &c), 2);
    assert!(near(o.depth, 0.5), "{o:?}");
    // moving away
    let i2 = EFCheckCollisionIn { tmax: 1.0, motion: [-4.0, 0.0, 0.0] };
    assert_eq!(a.BBox_CheckBBoxCollision(&i2, &mut o, &b), 0);
    // too short
    let i3 = EFCheckCollisionIn { tmax: 1.0, motion: [1.0, 0.0, 0.0] };
    assert_eq!(a.BBox_CheckBBoxCollision(&i3, &mut o, &b), 0);
}

#[test]
fn bbpt() {
    let a = bbox([0.0; 3], [1.0; 3]);
    let b = bbox([5.0, 0.0, 0.0], [1.0; 3]);
    let mut c = EFCheckBBBBCollision { motion: [4.0, 0.0, 0.0], normal: [0.0; 3], tmax: 1.0, tenter: -100000.0, depth: 100000.0, flags: 2 };
    assert_eq!(a.BBox_CheckBBPtBBCollision(&mut c, &b), 1);
    assert!(near(c.tenter, 0.75) && near(c.tmax, 1.0), "{c:?}");
    // motion along an axis where the point is outside but not moving -> 0x10
    let mut c = EFCheckBBBBCollision { motion: [0.0, 1.0, 0.0], tmax: 1.0, tenter: -100000.0, depth: 100000.0, flags: 2, ..Default::default() };
    assert_eq!(a.BBox_CheckBBPtBBCollision(&mut c, &b), 0x10);
}

#[test]
fn bbox_env() {
    let a = bbox([0.0, 2.0, 0.0], [1.0; 3]);
    let i = EFCheckCollisionIn { tmax: 1.0, motion: [0.0, -2.0, 0.0] };
    let mut o = EFCheckCollisionOut::default();
    assert_eq!(a.CheckEnvCollision(&i, &mut o, &floor(0)), 1, "{o:?}");
    assert!(near(o.t, 0.5), "{o:?}");
    assert_eq!(o.normal, [0.0, 1.0, 0.0]);
    assert!(near(o.motion[1], -1.0));
    // one-way material (bit1) with motion along the normal -> rejected
    let up = EFCheckCollisionIn { tmax: 1.0, motion: [0.0, 2.0, 0.0] };
    assert_eq!(a.BBox_CheckEnvCollision(&up, &mut o, &floor(2)), 0);
    // overlapping the floor
    let b = bbox([0.0, 0.5, 0.0], [1.0; 3]);
    assert_eq!(b.BBox_CheckEnvCollision(&i, &mut o, &floor(1)), 2, "{o:?}");
    assert!(near(o.depth, 0.5), "{o:?}");
    // far away
    let c = bbox([0.0, 10.0, 0.0], [1.0; 3]);
    assert_eq!(c.BBox_CheckEnvCollision(&i, &mut o, &floor(0)), 0);
}

#[test]
fn bbox_polygon_prims() {
    let a = bbox([0.0, 3.0, 0.0], [1.0; 3]);
    let p = poly([0.0, 1.0, 0.0]);
    let PrimExt::Polygon { m, .. } = &p.ext else { panic!() };
    let i = EFCheckCollisionIn { tmax: 1.0, motion: [0.0, -2.0, 0.0] };
    let mut o = EFCheckCollisionOut::default();
    assert_eq!(a.BBox_CheckPolygonCollisionPrims(&i, &mut o, m, &[floor(0)]), 1, "{o:?}");
    assert!(near(o.t, 0.5), "{o:?}");
    // no prims mapped in the model -> 0 and a zeroed Out
    assert_eq!(a.BBox_CheckPolygonCollision(&i, &mut o, &p), 0);
    assert_eq!(o.t, 0.0);
    // the Polygon swap wrapper negates normal/motion
    assert_eq!(p.Polygon_CheckObjPrimCollision(&i, &mut o, &a), 0);
}

#[test]
fn bbox_ray() {
    let a = bbox([0.0; 3], [1.0; 3]);
    let rin = RayIntersectIn { origin: [-5.0, 0.0, 0.0], dir: [10.0, 0.0, 0.0], mask1: 0, mask2: 0 };
    let mut out = EFRayIntersectOut::default();
    let mut t = 1.0;
    assert_eq!(a.RayIntersect(&rin, &mut out, &mut t), 1);
    assert!(near(t, 0.4));
    assert_eq!(out.hit.normal, [-1.0, 0.0, 0.0]);
    let mut t = 0.3;
    assert_eq!(a.BBox_RayIntersect(&rin, &mut out, &mut t), 0);
    let miss = RayIntersectIn { origin: [-5.0, 3.0, 0.0], dir: [10.0, 0.0, 0.0], mask1: 0, mask2: 0 };
    let mut t = 1.0;
    assert_eq!(a.BBox_RayIntersect(&miss, &mut out, &mut t), 0);
}

#[test]
fn polygon_ray() {
    let p = poly([0.0, 1.0, 0.0]);
    let rin = RayIntersectIn { origin: [0.5, 3.0, 0.0], dir: [0.0, -4.0, 0.0], mask1: 0, mask2: 0 };
    let mut out = EFRayIntersectOut::default();
    let mut t = 0.1; // ignored (quirk)
    assert_eq!(p.Polygon_RayIntersectPrims(&[floor(0)], &rin, &mut out, &mut t), 1);
    assert!(near(t, 0.5));
    assert_eq!(out.hit.normal, [0.0, 1.0, 0.0]);
    assert_eq!(out.hit.pos, [0.0; 3]);
    let outside = RayIntersectIn { origin: [3.0, 3.0, 0.0], ..rin };
    assert_eq!(p.Polygon_RayIntersectPrims(&[floor(0)], &outside, &mut out, &mut t), 0);
    assert_eq!(p.RayIntersect(&rin, &mut out, &mut t), 0);
}

#[test]
fn bbox_escape() {
    let a = bbox([0.0; 3], [1.0; 3]);
    let mut out = [0.0; 3];
    let mut r = || 0u32;
    a.BBox_GetEscapeNoPF([3.0, 0.0, 0.5], &mut out, [0.0; 3], &mut r);
    assert_eq!(out, [1.0, 0.0, 0.0]);
    a.BBox_GetEscapeNoPF([-3.0, 5.0, 0.5], &mut out, [0.0; 3], &mut r); // y axis ignored
    assert_eq!(out, [-1.0, 0.0, 0.0]);
    a.BBox_GetEscapeNoPF([2.0, 0.0, 2.0], &mut out, [0.0; 3], &mut r);
    assert!(near(out[0], 0.70710677) && near(out[2], 0.70710677) && out[1] == 0.0);
}

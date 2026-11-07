use scooby_recomp::engine::physic::collision_map::*;
use scooby_recomp::engine::physic::model::*;
use scooby_recomp::engine::physic::objcoll_check::*;
use scooby_recomp::engine::physic::objcoll_prim::*;

fn ltm(p: [f32; 3]) -> [[f32; 4]; 4] {
    let mut m = IDENTITY;
    m[3] = [p[0], p[1], p[2], 1.0];
    m
}
fn swept(p: [f32; 3], v: [f32; 3], r: f32) -> ObjPrim {
    let mut m = EFCollisionModel::new(1);
    let s = m.AddPrimitive(0, 1, PRIM_SWEPT_SPHERE, ltm(p)).unwrap();
    if let PrimExt::SweptSphere { sradius, .. } = &mut s.ext {
        *sradius = r;
    }
    s.SetVector(v);
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
/// 2x2 floor square at y=0, normal +y, outward edge planes.
fn floor(mat0: u32) -> EnvPrim {
    EnvPrim {
        id: 0,
        mat: [mat0, 0, 0, 0, 0, 0, 0, 0],
        w4: 0,
        nverts: 4,
        normal: [0.0, 1.0, 0.0],
        d: 0.0,
        verts: vec![[-1.0, 0.0, -1.0], [1.0, 0.0, -1.0], [1.0, 0.0, 1.0], [-1.0, 0.0, 1.0]],
        edges: vec![[0.0, 0.0, -1.0, -1.0], [1.0, 0.0, 0.0, -1.0], [0.0, 0.0, 1.0, -1.0], [-1.0, 0.0, 0.0, -1.0]],
    }
}
fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-5
}

#[test]
fn swept_bbox_face_hit_and_overlap() {
    let b = bbox([0.0; 3], [1.0; 3]);
    let s = swept([-3.0, 0.0, 0.0], [0.0, 1.0, 0.0], 0.5);
    let i = EFCheckCollisionIn { tmax: 1.0, motion: [4.0, 0.0, 0.0] };
    let mut o = EFCheckCollisionOut::default();
    assert_eq!(s.Swept_CheckBBoxCollision(&i, &mut o, &b), 1);
    assert!(close(o.t, 0.375), "{o:?}");
    assert_eq!(o.normal, [-1.0, 0.0, 0.0]);
    // moving away -> 0
    let i2 = EFCheckCollisionIn { tmax: 1.0, motion: [-4.0, 0.0, 0.0] };
    assert_eq!(s.Swept_CheckBBoxCollision(&i2, &mut o, &b), 0);
    // overlapping a face
    let s2 = swept([1.2, 0.0, 0.0], [0.0, 1.0, 0.0], 0.5);
    let mut o = EFCheckCollisionOut::default();
    assert_eq!(s2.Swept_CheckBBoxCollision(&i, &mut o, &b), 2);
    assert!(close(o.depth, 0.3), "{o:?}");
    assert_eq!(o.normal, [1.0, 0.0, 0.0]);
    // swap wrapper through the dispatcher
    let mut o = EFCheckCollisionOut::default();
    assert_eq!(b.CheckObjPrimCollision(&i2, &mut o, &s), 1);
    assert_eq!(o.normal, [1.0, 0.0, 0.0]);
}

#[test]
fn swept_env_polygon() {
    let s = swept([0.0, 2.0, 0.0], [0.0, 0.5, 0.0], 0.25);
    let i = EFCheckCollisionIn { tmax: 1.0, motion: [0.0, -4.0, 0.0] };
    let mut o = EFCheckCollisionOut::default();
    assert_eq!(s.Swept_CheckEnvCollision(&i, &mut o, &floor(0)), 1);
    assert!(close(o.t, 0.3125), "{o:?}");
    assert_eq!(o.normal, [0.0, 1.0, 0.0]);
    // tmax too short: culled by the bounding radius against pd + n.(motion*tmax) -> 0
    let i4 = EFCheckCollisionIn { tmax: 0.2, motion: [0.0, -4.0, 0.0] };
    assert_eq!(s.Swept_CheckEnvCollision(&i4, &mut o, &floor(0)), 0);
    // overlapping
    let s2 = swept([0.0, 0.6, 0.0], [0.0, 0.5, 0.0], 0.25);
    let mut o = EFCheckCollisionOut::default();
    assert_eq!(s2.Swept_CheckEnvCollision(&i, &mut o, &floor(0)), 2);
    assert!(close(o.depth, 0.15), "{o:?}");
    // one-way (mat bit 1) with motion along the normal -> 0
    let iu = EFCheckCollisionIn { tmax: 1.0, motion: [0.0, 4.0, 0.0] };
    assert_eq!(s2.Swept_CheckEnvCollision(&iu, &mut o, &floor(2)), 0);
    // off the polygon, far: miss
    let s3 = swept([5.0, 2.0, 0.0], [0.0, 0.5, 0.0], 0.25);
    assert_eq!(s3.Swept_CheckEnvCollision(&i, &mut o, &floor(0)), 0);
}

#[test]
fn swept_polygon_object() {
    let mut m = EFCollisionModel::new(1);
    let p = m.AddPrimitive(0, 3, PRIM_POLYGON, ltm([0.0; 3])).unwrap();
    p.PrecalcPrimData();
    let p = p.clone();
    let s = swept([0.0, 2.0, 0.0], [0.0, 0.5, 0.0], 0.25);
    let i = EFCheckCollisionIn { tmax: 1.0, motion: [0.0, -4.0, 0.0] };
    let mut o = EFCheckCollisionOut::default();
    assert_eq!(s.Swept_CheckPolygonCollision_prims(&i, &mut o, &p, &[floor(0)]), 1);
    assert!(close(o.t, 0.3125));
    assert_eq!(s.Swept_CheckPolygonCollision(&i, &mut o, &p), 0);
}

#[test]
fn swept_swept_v2() {
    let a = swept([0.0; 3], [0.0, 1.0, 0.0], 0.5);
    let b = swept([0.8, 0.0, 0.0], [0.0, 0.0, 1.0], 0.5);
    let i = EFCheckCollisionIn { tmax: 1.0, motion: [0.0; 3] };
    let mut o = EFCheckCollisionOut::default();
    assert_eq!(a.Swept_CheckSweptSphereCollision(&i, &mut o, &b), 2);
    assert!(close(o.depth, 0.2), "{o:?}");
    let c = swept([4.0, 0.0, 0.0], [0.0, 0.0, 1.0], 0.5);
    let i = EFCheckCollisionIn { tmax: 1.0, motion: [4.0, 0.0, 0.0] };
    let mut o = EFCheckCollisionOut::default();
    assert_eq!(a.Swept_CheckSweptSphereCollision(&i, &mut o, &c), 1);
    assert!(close(o.t, 0.75), "{o:?}");
    let i = EFCheckCollisionIn { tmax: 0.5, motion: [4.0, 0.0, 0.0] };
    assert_eq!(a.Swept_CheckSweptSphereCollision(&i, &mut o, &c), 0);
}

#[test]
fn swept_ray_point_particles_escape() {
    let s = swept([0.0; 3], [0.0, 1.0, 0.0], 0.5);
    let rin = RayIntersectIn { origin: [-5.0, 0.5, 0.0], dir: [10.0, 0.0, 0.0], mask1: 0, mask2: 0 };
    let mut out = EFRayIntersectOut::default();
    let mut t = 1.0;
    assert_eq!(s.Swept_RayIntersect(&rin, &mut out, &mut t), 1);
    assert!(close(t, 0.45), "{t}");
    assert!(close(out.hit.normal[0], -1.0));
    let rin2 = RayIntersectIn { origin: [0.0, 0.0, 0.0], dir: [10.0, 0.0, 0.0], mask1: 0, mask2: 0 };
    let mut t = 1.0;
    assert_eq!(s.Swept_RayIntersect(&rin2, &mut out, &mut t), 1);
    // inside: CheckSphereSweepCollision answers 0x101 (segment body, t = 0), not 8
    assert_eq!(t, 0.0);

    assert!(s.Swept_CheckPointIntersection([0.3, 0.9, 0.0]));
    assert!(s.Swept_CheckPointIntersection([0.0, 1.4, 0.0]));
    assert!(!s.Swept_CheckPointIntersection([0.0, 1.6, 0.0]));
    assert!(!s.Swept_CheckPointIntersection([0.6, 0.0, 0.0]));

    let mut parts = vec![([0.0, 0.5, 0.0], 0.1, 0u16), ([5.0, 0.0, 0.0], 0.1, 0), ([0.0, -0.5, 0.0], 0.1, 7)];
    assert_eq!(s.Swept_CheckParticlesIntersection(&mut parts, true, 7), 1);
    assert_eq!(parts[0].2, 7);
    assert_eq!(parts[1].2, 0);

    let mut out = [0.0; 3];
    let mut rnd = || 0u32;
    s.Swept_GetEscapeNoPF([1.0, 0.0, 0.0], &mut out, [0.0; 3], &mut rnd);
    // w = (0,1,0): w.xz = 0 -> h = 0 -> 1/sqrt(0) (inf * 0 = NaN), as in the C++
    assert!(out[0].is_nan() || out[0] == 0.0);
    let s2 = swept([0.0; 3], [1.0, 0.0, 0.0], 0.5);
    s2.Swept_GetEscapeNoPF([0.0, 0.0, 1.0], &mut out, [0.0; 3], &mut rnd);
    assert_eq!(out, [0.0, 0.0, 1.0]);
}

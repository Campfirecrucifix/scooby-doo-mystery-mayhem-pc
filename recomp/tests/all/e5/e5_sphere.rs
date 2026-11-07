use scooby_recomp::engine::physic::collision_map::*;
use scooby_recomp::engine::physic::model::*;
use scooby_recomp::engine::physic::objcoll_check::*;
use scooby_recomp::engine::physic::objcoll_prim::*;

fn sphere(p: [f32; 3], r: f32) -> ObjPrim {
    let mut m = EFCollisionModel::new(1);
    let mut l = IDENTITY;
    l[3] = [p[0], p[1], p[2], 1.0];
    let s = m.AddPrimitive(0, 1, PRIM_SPHERE, l).unwrap();
    s.base.radius = r;
    s.PrecalcPrimData();
    s.clone()
}
/// 2x2 square on y=0, normal +Y, outward edge planes.
fn quad(mat0: u32) -> EnvPrim {
    let verts = vec![[-1.0, 0.0, -1.0], [-1.0, 0.0, 1.0], [1.0, 0.0, 1.0], [1.0, 0.0, -1.0]];
    let edges = vec![[-1.0, 0.0, 0.0, -1.0], [0.0, 0.0, 1.0, -1.0], [1.0, 0.0, 0.0, -1.0], [0.0, 0.0, -1.0, -1.0]];
    let mut mat = [0; 8];
    mat[0] = mat0;
    EnvPrim { id: 0, mat, w4: 0, nverts: 4, normal: [0.0, 1.0, 0.0], d: 0.0, verts, edges }
}
fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-4
}

#[test]
fn env_face_sweep() {
    let s = sphere([0.0, 2.0, 0.0], 0.5);
    let mut o = EFCheckCollisionOut::default();
    let i = EFCheckCollisionIn { tmax: 1.0, motion: [0.0, -3.0, 0.0] };
    assert_eq!(s.Sphere_CheckEnvCollision(&i, &mut o, &quad(0)), 1);
    assert!(close(o.t, 0.5) && o.normal == [0.0, 1.0, 0.0], "{o:?}");
    // beyond tmax -> 0 on the face path
    let i2 = EFCheckCollisionIn { tmax: 1.0, motion: [0.0, -1.0, 0.0] };
    assert_eq!(s.Sphere_CheckEnvCollision(&i2, &mut o, &quad(0)), 0);
}

#[test]
fn env_overlap_face_and_edge() {
    let s = sphere([0.0, 0.25, 0.0], 0.5);
    let mut o = EFCheckCollisionOut::default();
    let i = EFCheckCollisionIn { tmax: 1.0, motion: [0.0; 3] };
    assert_eq!(s.Sphere_CheckEnvCollision(&i, &mut o, &quad(0)), 2);
    assert!(close(o.depth, 0.25) && o.normal == [0.0, 1.0, 0.0]);
    // beside the +X edge, just outside: edge-distance overlap
    let s = sphere([1.2, 0.0, 0.0], 0.5);
    assert_eq!(s.Sphere_CheckEnvCollision(&i, &mut o, &quad(1)), 2);
    assert!(close(o.depth, 0.3) && close(o.normal[0], 1.0), "{o:?}");
}

#[test]
fn env_edge_sweep_and_polygon_list() {
    // sphere outside +X edge moving -X at plane height: hits the edge line
    let s = sphere([3.0, 0.0, 0.0], 0.5);
    let mut o = EFCheckCollisionOut::default();
    let i = EFCheckCollisionIn { tmax: 1.0, motion: [-2.0, 0.0, 0.0] };
    let r = s.Sphere_CheckSinglePolygonCollision(&i, &mut o, &quad(1), s.base.center, 1);
    assert_eq!(r, 1, "{o:?}");
    assert!(close(o.t, 0.75) && close(o.normal[0], 1.0));
    // polygon-object path with identity matrices
    let r = s.Sphere_CheckPolygonCollisionPrims(&i, &mut o, &IDENTITY, &IDENTITY, &[quad(0)]);
    assert_eq!(r, 1);
    assert!(close(o.t, 0.75));
}

#[test]
fn ray_and_particles() {
    let s = sphere([0.0, 0.0, 5.0], 1.0);
    let rin = RayIntersectIn { origin: [0.0; 3], dir: [0.0, 0.0, 10.0], mask1: 0, mask2: 0 };
    let mut out = EFRayIntersectOut::default();
    let mut t = 1.0;
    assert_eq!(s.Sphere_RayIntersect(&rin, &mut out, &mut t), 1);
    assert!(close(t, 0.4) && out.hit.normal == [0.0, 0.0, -1.0]);
    let mut parts = vec![([0.0, 0.0, 6.5], 0.6, 0u16), ([0.0, 0.0, 9.0], 0.1, 0), ([0.0, 0.0, 5.0], 0.1, 7)];
    assert_eq!(s.Sphere_CheckParticlesIntersection(&mut parts, true, 7), 1);
    assert_eq!(parts[0].2, 7);
    assert_eq!(s.Sphere_CheckParticlesIntersection(&mut parts, false, 7), 1);
}

use scooby_recomp::engine::camera::*;
use scooby_recomp::engine::light::*;
use scooby_recomp::engine::spline::*;

#[test]
fn camera_create_and_frustum() {
    let mut g = EFGraphicsMgrCameras::new(640, 448);
    let uid = g.CreateCamera(5, DTMatrix::default());
    assert_eq!(uid, 0x1000_0000);
    let c = g.GetCamera(uid).unwrap();
    assert_eq!(c.rw.frame_raster, Some([0, 0, 640, 448]));
    c.refresh_frustum();
    assert_eq!(c.IsSphereInFrustrum([0.0, 0.0, 5.0], 0.1), 1);
    assert_eq!(c.IsSphereInFrustrum([0.0, 0.0, -5.0], 0.1), 0);
    g.DestroyCamera(uid);
    assert!(g.GetCamera(uid).is_none());
}

#[test]
fn lights() {
    let mut m = EFLightMgr_BC::new();
    let u = m.CreateDynamicLight(1, 0x80);
    assert_eq!(u, 0x1200_0001);
    let l = m.GetLight(u).unwrap();
    l.SetConeAngle(0.5);
    assert_eq!(l.GetConeAngle(), 0.0);
    m.DestroyLight(u);
    assert_eq!(m.n_dynamic, 0);
}

#[test]
fn spline_line() {
    let seg = SplineSegment { p: [[0.0; 3], [1.0, 0.0, 0.0], [2.0, 0.0, 0.0]], length: 2.0, b: [2.0, 0.0, 0.0], ..Default::default() };
    let s = EFQuadraticSpline { length: 2.0, segs: vec![seg], ..Default::default() };
    assert_eq!(s.GetPoint(0, 0.5), [1.0, 0.0, 0.0]);
    let mut c = EFSplineCursor::default();
    c.MoveTo(&s, 0.0);
    c.MoveDistance(&s, 1.0);
    assert!((c.global - 0.5).abs() < 1e-3);
    assert_eq!(cubic_solve(1.0, 0.0, 0.0, -8.0), vec![2.0]);
}

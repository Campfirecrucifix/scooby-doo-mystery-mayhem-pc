include!("edge_common.rs");
use scooby_recomp::engine::physic::dm::edge_down::*;

/// wall z=0 facing +z, top edge at y=0
fn wall() -> EnvPrim {
    poly(
        vec![[0.0, 0.0, 0.0], [0.0, -1.0, 0.0], [1.0, -1.0, 0.0], [1.0, 0.0, 0.0]],
        [0.0, 0.0, 1.0],
        0.0,
        vec![[-1.0, 0.0, 0.0, 0.0], [0.0, -1.0, 0.0, -1.0], [1.0, 0.0, 0.0, -1.0], [0.0, 1.0, 0.0, 0.0]],
    )
}

#[test]
fn edge_down_snap_and_rotate() {
    let mut p = owner([0.5, 0.2, 0.1], [0.0; 3], 0.5);
    let mut m = EFDMEdgeDown::new();
    m.base.notify_bits = 1;
    with(&mut p, 0, &NoWorld, |c| m.SetParams(c, Some(&[7, fw(1.0)])));
    assert_eq!((m.prim, m.time), (0, 1.0));
    assert!((m.speed - std::f32::consts::PI).abs() < 1e-6);
    p.velocity = [1.0, 2.0, 3.0];
    let (_, n) = with(&mut p, 1, &NoWorld, |c| m.EdgeDown(c, &[wall()]));
    assert_eq!(m.base.state, 1);
    assert_eq!(n.len(), 1);
    assert_eq!(m.hit_mask, 1);
    assert!(close(m.edge, [-1.0, 0.0, 0.0]));
    assert!(close(center(&p), [0.5, 0.0, 0.0]), "{:?}", center(&p));
    assert_eq!(p.velocity, [0.0; 3]);
    // turned by pi about y: at = -z
    assert!(close(p.frame.at, [0.0, 0.0, -1.0]), "{:?}", p.frame.at);
    let mut out = [0u32; 3];
    with(&mut p, 1, &NoWorld, |c| m.GetParams(c, &mut out));
    assert_eq!(out, [7, fw(1.0), 1]);
    // PreColUpdate: pi/2 about the edge per 0.5 s, then state cleared after pi
    let mut dp = EFDynamicPhysic::default();
    let pos = p.frame.pos;
    with(&mut p, 2, &NoWorld, |c| m.PreColUpdate(c, &mut dp, 0.5));
    assert!((m.angle - std::f32::consts::FRAC_PI_2).abs() < 1e-5);
    assert_eq!(p.frame.pos, pos);
    assert!(close(p.frame.at, [0.0, 1.0, 0.0]) || close(p.frame.at, [0.0, -1.0, 0.0]), "{:?}", p.frame.at);
    with(&mut p, 3, &NoWorld, |c| m.PreColUpdate(c, &mut dp, 0.6));
    assert_eq!((m.base.state, m.angle), (0, 0.0));
    assert!(close(p.frame.at, [0.0, 0.0, 1.0]), "{:?}", p.frame.at);
}

#[test]
fn edge_down_no_edge_and_blocked() {
    let mut p = owner([0.5, 0.2, 2.0], [0.0; 3], 0.5);
    let mut m = EFDMEdgeDown::new();
    with(&mut p, 0, &NoWorld, |c| m.SetParams(c, Some(&[7, 0])));
    assert_eq!(m.speed, 0.0);
    with(&mut p, 1, &NoWorld, |c| m.EdgeDown(c, &[wall()]));
    assert_eq!(m.base.state, 0);
    // a big plane behind the edge test point blocks it (IsInSomeOtherPrim). quirk: the binary's
    // t = plane(pt) / (dir.n) has no minus sign, so t lands in [0,1] only for a plane the ray
    // moves away from: plane z=-0.1 facing +z, pt z=-0.05 -> t = 0.05/0.55
    let blocker = poly(
        vec![[-5.0, -5.0, -0.1], [-5.0, 5.0, -0.1], [5.0, 5.0, -0.1], [5.0, -5.0, -0.1]],
        [0.0, 0.0, 1.0],
        0.1,
        vec![[-1.0, 0.0, 0.0, -5.0], [0.0, 1.0, 0.0, -5.0], [1.0, 0.0, 0.0, -5.0], [0.0, -1.0, 0.0, -5.0]],
    );
    assert!(m.IsInSomeOtherPrim([0.5, 0.0, -0.05], [0.55, 0.0, 0.55], &[blocker.clone()]));
    let mut p = owner([0.5, 0.2, 0.1], [0.0; 3], 0.5);
    let mut b2 = blocker;
    b2.mat[1] = 0; // not matched by the masks, still tested by IsInSomeOtherPrim
    with(&mut p, 1, &NoWorld, |c| m.EdgeDown(c, &[wall(), b2]));
    assert_eq!(m.base.state, 0);
}

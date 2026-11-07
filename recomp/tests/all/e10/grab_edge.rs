include!("edge_common.rs");
use scooby_recomp::engine::physic::dm::grab_edge::*;

struct Stick([f32; 2]);
impl PhysicWorld for Stick {
    fn InputActionGet(&self, _a: u32, _b: u32) -> Option<[f32; 2]> { Some(self.0) }
}

#[test]
fn grab_edge_reach_move_corner() {
    // owner at z=-0.4 facing +z, prim 0.2 ahead; ledge edge (0,0,0)-(1,0,0)
    let mut p = owner([0.5, 0.3, -0.4], [0.0, 0.0, 0.2], 0.3);
    let mut m = EFDMGrabEdge::new();
    m.base.notify_bits = 7;
    let prm = [fw(1.0), fw(0.5), 7, 3, 4, fw(0.5), fw(1.0), fw(0.0), fw(4.0)]; // turn_speed > pi/2: the corner turn must finish before the factor leaves [0,1]
    with(&mut p, 0, &NoWorld, |c| m.SetParams(c, Some(&prm)));
    let mut out = [0u32; 9];
    with(&mut p, 0, &NoWorld, |c| m.GetParams(c, &mut out));
    assert_eq!(out, prm);
    let env = [ledge()];
    let still = Stick([0.0, 0.0]);
    let (_, n) = with(&mut p, 1, &still, |c| m.GrabEdge(c, &env, 0.1));
    assert_eq!((m.base.state, m.phase, n.len()), (1, 0, 1));
    assert!(close(m.grab, [0.5, 0.0, 0.0]) && (m.factor - 0.5).abs() < 1e-6);
    assert!((m.reach_time - 0.72111).abs() < 1e-3);
    let mut f = 2;
    while m.phase == 0 {
        with(&mut p, f, &still, |c| m.GrabEdge(c, &env, 0.1));
        f += 1;
        assert!(f < 20);
    }
    assert_eq!((m.base.state, m.phase), (2, 1));
    assert!(close(center(&p), [0.5, 0.0, 0.0]), "{:?}", center(&p));
    let mut d = [0u32; 6];
    with(&mut p, f, &still, |c| m.DoAction(c, 1, &mut d));
    assert_eq!([d[0], d[3], d[4]], [fw(1.0), fw(0.0), fw(1.0)]);
    // stick x=-1 -> camera right -> +x along the edge
    let right = Stick([-1.0, 0.0]);
    with(&mut p, f, &right, |c| m.GrabEdge(c, &env, 0.2));
    f += 1;
    assert_eq!(m.input, 1.0);
    assert!(close(center(&p), [0.7, 0.0, 0.0]), "{:?}", center(&p));
    // past the corner (1,0,0): adjacent edge to (1,0,1) -> corner phase
    while m.phase == 1 {
        with(&mut p, f, &right, |c| m.GrabEdge(c, &env, 0.2));
        f += 1;
        assert!(f < 40);
    }
    assert_eq!(m.phase, 2);
    assert!(close(m.next, [1.0, 0.0, 1.0]), "{:?}", m.next);
    assert!((m.angle - std::f32::consts::FRAC_PI_2).abs() < 1e-5);
    while m.phase == 2 {
        with(&mut p, f, &right, |c| m.GrabEdge(c, &env, 0.5));
        f += 1;
        assert!(f < 60);
    }
    // after the turn the grab edge is (1,0,0)-(1,0,1)
    assert!(close(m.v0, [1.0, 0.0, 0.0]) && close(m.v1, [1.0, 0.0, 1.0]), "{:?} {:?}", m.v0, m.v1);
}

#[test]
fn grab_edge_dead_end_and_frame_gap() {
    // only edge 0 flagged -> no adjacent edge -> state 4
    let mut e = ledge();
    e.w4 = 1;
    let mut p = owner([0.5, 0.3, -0.4], [0.0, 0.0, 0.2], 0.3);
    let mut m = EFDMGrabEdge::new();
    let prm = [fw(1.0), fw(0.5), 7, 0, 0, fw(10.0), fw(1.0), fw(0.0), fw(1.0)];
    with(&mut p, 0, &NoWorld, |c| m.SetParams(c, Some(&prm)));
    let right = Stick([-1.0, 0.0]);
    let mut f = 1;
    while m.base.state != 4 {
        with(&mut p, f, &right, |c| m.GrabEdge(c, &[e.clone()], 0.25));
        f += 1;
        assert!(f < 20, "state {}", m.base.state);
    }
    assert_eq!(m.factor, 1.0);
    // a skipped frame resets the module
    with(&mut p, f + 5, &right, |c| m.GrabEdge(c, &[], 0.1));
    assert_eq!((m.base.state, m.phase), (0, 0));
}

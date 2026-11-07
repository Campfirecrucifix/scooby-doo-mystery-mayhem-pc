//! WP-D2: EFAnimationMgr / EFAnimBlendNode tree.
use scooby_recomp::engine::anim::{mgr::*, nodes::*, res::*};
use std::sync::Arc;

/// 1 set (bone 0), 1 channel `ch`, curve value = t over [0,2]
fn synth(ch: u32) -> EFHAnimRes {
    let mut b = Vec::new();
    let w = |b: &mut Vec<u8>, v: u32| b.extend_from_slice(&v.to_le_bytes());
    w(&mut b, 0);
    w(&mut b, 0);
    w(&mut b, 2f32.to_bits());
    b.extend_from_slice(&[1, 0, 0, 0]);
    w(&mut b, 0);
    b.extend_from_slice(&[1, 0, 0, 0]);
    w(&mut b, ch);
    w(&mut b, 0x20);
    w(&mut b, 0f32.to_bits());
    w(&mut b, 2f32.to_bits());
    b.extend_from_slice(&[0, 2, 1, 0]);
    for v in [2f32, 0.0, 0.0, 2.0, 0.0] {
        w(&mut b, v.to_bits());
    }
    let n = b.len() as u32;
    b[..4].copy_from_slice(&n.to_le_bytes());
    let mut h = EFHAnimRes::default();
    h.base.anim = DTAnimHdr::ResolvePointers(b);
    h
}

fn mgr() -> EFAnimationMgr {
    let mut m = EFAnimationMgr::new(100, 4);
    for (id, ch) in [(100, 3), (101, 4)] {
        let mut n = EFHAnimNode::new(Arc::new(synth(ch)), id);
        n.obj = 7;
        m.AddAnim(n, id);
    }
    m
}

#[test]
fn euler_to_quat_matches_dtquaternion() {
    // EulerToQuatf((pi/2,0,0)) = (sin45, 0, 0, cos45)
    let q = DTTransfo::EulerToQuatf([std::f32::consts::FRAC_PI_2, 0.0, 0.0]);
    assert!((q[0] - 0.70710677).abs() < 1e-6 && (q[3] - 0.70710677).abs() < 1e-6);
    let mut t = DTTransfo::default();
    t.f[6] = 1.0;
    t.state = 1;
    assert_eq!(t.rotation_quat(), DTTransfo::EulerToQuatf([1.0, 0.0, 0.0]));
}

#[test]
fn play_then_crossfade_then_timeout() {
    let mut m = mgr();
    m.Play(100, state::LOOP, 0.0);
    m.Update(0.5);
    let p = m.GetPose(7).unwrap();
    assert!((p[0].f[3] - 0.5).abs() < 1e-5);
    // cross-fade (flags 0x10) to 101 over 1s
    m.Play(101, state::LOOP | 0x10, 1.0);
    m.Update(0.5);
    let p = m.GetPose(7).unwrap();
    // w = 1 - 0.5/1 ; a (old) f[3] = 1.0, b (new) f[3] = 0 ; f[4]: a 0, b 0.5
    assert!((p[0].f[3] - 0.5).abs() < 1e-5, "{:?}", p[0].f);
    assert!((p[0].f[4] - 0.25).abs() < 1e-5, "{:?}", p[0].f);
    m.Update(0.6); // blend times out, survivor = new anim
    let p = m.GetPose(7).unwrap();
    assert_eq!(p[0].f[3], 0.0);
    assert!(m.hanim(100).unwrap().state & state::STOPPED != 0);
    m.Stop(101);
    assert!(m.GetPose(7).is_none());
}

#[test]
fn layered_blend_and_stop_hanim() {
    let mut m = mgr();
    m.Play(100, state::LOOP, 0.0);
    let p = EFBlendParams { id: 0xbb00_0001, ramp: 1.0, weight: 0.5, fade_out: 0.0, sound_threshold: 0.2 };
    m.PlayBlend(101, state::LOOP, 100, 0.0, Some(p));
    let b = m.FindBlendNode(m.node(0).child, 0xbb00_0001).expect("blend node");
    m.Update(0.5);
    match &m.node(b).kind {
        NodeKind::Blend(bn) => assert!((bn.t - 0.5).abs() < 1e-6 && bn.ty == 3),
        _ => panic!(),
    }
    // ramp: w = 0.5 * 0.5/1
    let pose = m.GetPose(7).unwrap();
    assert!((pose[0].f[4] - 0.25 * 0.5).abs() < 1e-5, "{:?}", pose[0].f);
    m.ChangeBlendParam(&EFBlendParams { weight: 1.0, ramp: 0.0, ..p });
    m.Stop(101);
    assert!(m.GetPose(7).is_some());
    assert!(m.FindBlendNode(m.node(0).child, 0xbb00_0001).is_none());
    m.StopAll();
    assert!(m.GetPose(7).is_none());
}

#[test]
fn can_play_sound_cb() {
    let mut b = EFAnimBlendNode {
        id: 0, state: 0, ty: 4, w: 0.5, t: 0.0, dur: 0.0, ramp: 0.0, fade_out: 0.0,
        w_target: 0.0, w_time: 0.0, sound_threshold: 0.4, additive: false, snapshot: vec![],
    };
    assert!(!b.CanPlaySoundCB(true, f32::MAX));
    assert!(b.CanPlaySoundCB(false, f32::MAX));
    b.sound_threshold = -1.0;
    assert!(b.CanPlaySoundCB(true, f32::MAX));
}

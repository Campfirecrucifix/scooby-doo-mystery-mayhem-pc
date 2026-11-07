use scooby_recomp::engine::anim::{nodes::*, res::*, AnimInstanciator};
use scooby_recomp::engine::{iso::IsoFs, pack::*};
use std::sync::Arc;

use crate::common::ISO;

#[test]
fn ep1l01_anims_parse_and_sample() {
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let mut m = EFRessourcesMgr::default();
    m.level_file_name = Some("EP1L01.P2R".into());
    let err = m.LoadLevel(&fs, "", "EP1/L01", "EN", &mut AnimInstanciator::default(), |_, _| {}).unwrap();
    assert_eq!(err, 0);
    let (mut nh, mut na) = (0, 0);
    for o in m.objects.iter_mut() {
        if let Some(r) = o.as_raw() {
            assert!(r.class_id != 0x49 && r.class_id != 0x4f);
        }
        let Some(d) = o.as_dt() else { continue };
        if let Some(h) = d.as_any().downcast_ref::<EFHAnimRes>() {
            nh += 1;
            let a = h.base.anim.as_ref().expect("anim blob");
            assert_eq!(a.size as usize, a.raw.len());
            assert!(a.duration > 0.0, "uid {:x}", h.base.hdr.uid);
            let mut n = EFHAnimNode::new(Arc::new(h.clone()), 0);
            for t in [0.0, a.duration * 0.5, a.duration] {
                n.SetTime(t);
                n.UpdateHAnim();
                for (q, v) in n.sample() {
                    assert!(q.iter().chain(v.iter()).all(|x| x.is_finite()));
                }
            }
        } else if let Some(r) = d.as_any().downcast_ref::<EFAnimRes>() {
            na += 1;
            assert!(r.anim.as_ref().is_some_and(|a| a.duration > 0.0));
        }
    }
    assert_eq!((nh, na), (294, 9));
}

#[test]
fn decompress1075() {
    // 1.0 = sign 0, exp 127-107=20 (0x14<<10 = 0x5000), mantissa 0
    let (a, b) = Decompress1075(0x5000 | (0xd000 << 16));
    assert_eq!((a, b), (1.0, -1.0));
    assert_eq!(Decompress1075(0), (0.0, 0.0));
}

/// synthetic blob: 1 set, 1 channel (euler x), cubic curve x = t over [0,2]
fn synth() -> EFHAnimRes {
    let mut b = Vec::new();
    let w = |b: &mut Vec<u8>, v: u32| b.extend_from_slice(&v.to_le_bytes());
    w(&mut b, 0);
    w(&mut b, 0);
    w(&mut b, 2f32.to_bits());
    b.extend_from_slice(&[1, 0, 0, 0]);
    w(&mut b, 0); // bone
    b.extend_from_slice(&[1, 0, 0, 0]); // nb ch, flags
    w(&mut b, 6); // channel id 6 = +0x18 euler x (DTTransfo layout fixed in WP-D2)
    w(&mut b, 0x20); // curve offset
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
    // one callback at t=1.0, kind 1, index 7
    h.cb_count = 1;
    h.cb_elem_size = 12;
    h.callbacks = [1f32.to_bits(), 1, 7].iter().flat_map(|x| x.to_le_bytes()).collect();
    h
}

#[test]
fn play_loop_and_once() {
    let res = Arc::new(synth());
    let mut n = EFHAnimNode::new(res.clone(), 42);
    assert_eq!(n.duration, 2.0);
    n.Update(0.5);
    assert_eq!(n.time, 0.0, "stopped node does not advance");
    n.Play(state::LOOP, 0.0);
    n.Update(0.5);
    assert!((n.transfos[0].f[6] - 0.5).abs() < 1e-5);
    n.Update(1.0);
    assert!(n.events.contains(&AnimEvent::Callback { user: 42, kind: 1, index: 7 }));
    n.events.clear();
    n.Update(1.0); // 2.5 -> wraps to 0.5
    assert!((n.time - 0.5).abs() < 1e-5);
    assert!(n.events.iter().any(|e| matches!(e, AnimEvent::Notify { code: 1, .. })));

    let mut o = EFHAnimNode::new(res, 1);
    o.Play(state::PLAY_ONCE, 0.0);
    o.Update(5.0);
    assert_eq!(o.time, 2.0);
    assert!(o.state & state::FINISHED != 0);
    assert!(o.events.iter().any(|e| matches!(e, AnimEvent::Notify { code: 1, user: 1, .. })));
    o.Stop();
    assert_eq!(o.state, state::STOPPED);
    assert!(o.events.iter().any(|e| matches!(e, AnimEvent::Notify { code: 0, user: 1, .. })));
}

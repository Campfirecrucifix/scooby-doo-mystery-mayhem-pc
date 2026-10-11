//! WP-D3: APIanim_*, QLerp, notification 0x11, real-data playback.
use scooby_recomp::engine::anim::{api::*, mgr::*, nodes::*, quat::DTQuaternion, res::*, AnimInstanciator};
use scooby_recomp::engine::{iso::IsoFs, Mem, pack::*};
use std::sync::Arc;

const ISO: &str = "../Scooby Doo Mystery Mayhem.ISO";

#[test]
fn qlerp_endpoints_and_shortest_path() {
    let a = [0.0, 0.0, 0.0, 1.0];
    let b = [0.0, 0.70710677, 0.0, 0.70710677];
    let q = DTQuaternion::QLerp(&a, &b, 0.0);
    assert!((q[3] - 1.0).abs() < 1e-3);
    let q = DTQuaternion::QLerp(&a, &b, 1.0);
    assert!((q[1] - b[1]).abs() < 1e-3);
    // negated target takes the short way: same rotation result
    let nb = b.map(|x| -x);
    let q1 = DTQuaternion::QLerp(&a, &b, 0.5);
    let q2 = DTQuaternion::QLerp(&a, &nb, 0.5);
    assert!(q1.iter().zip(q2).all(|(x, y)| (x - y).abs() < 1e-5 || (x + y).abs() < 1e-5));
}

/// EP1L01 book (TomeDoom) animation through APIanim_*: the anim resource of the book object is
/// selected as the first non-looping HAnim with a small skeleton (the book's game object ->
/// anim UID table is WP-C data; see Findings).
#[test]
fn ep1l01_book_anim_through_apianim() {
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let mut rm = EFRessourcesMgr::default();
    rm.level_file_name = Some("EP1L01.P2R".into());
    rm.LoadLevel(&fs, "", "EP1/L01", "EN", &mut AnimInstanciator::default(), |_, _| {}).unwrap();
    let mut res = None;
    for o in rm.objects.iter_mut() {
        let Some(d) = o.as_dt() else { continue };
        if let Some(h) = d.as_any().downcast_ref::<EFHAnimRes>() {
            let a = h.base.anim.as_ref().unwrap();
            if (2..=10).contains(&a.sets.len()) && a.duration > 0.5 {
                res = Some(h.clone());
                break;
            }
        }
    }
    let h = Arc::new(res.expect("book-like anim"));
    let dur = h.base.anim.as_ref().unwrap().duration;
    let mut m = EFAnimationMgr::new(0x100, 2);
    m.owner_uid = 0x2a80_0001;
    m.notify_update = 1;
    let mut n = EFHAnimNode::new(h, 0x100);
    n.obj = 1;
    m.AddAnim(n, 0x100);
    assert_eq!(APIanim_IsAnimPlaying(Some(&m), 0x100), 0);
    // TomeDoom: PlayAnim(e, 0.0, anim, 1 = PLAY_ONCE, 6); flags 0 would skip Play (ConnectHAnim returns flags)
    APIanim_Play(Some(&mut m), 0x100, state::PLAY_ONCE, 0.0);
    assert_eq!(APIanim_IsAnimPlaying(Some(&m), 0x100), 1);
    assert_eq!(APIanim_GetDuration(Some(&m), 0x100), dur);
    assert_eq!(f32::from_bits(APIanim_GetSpeed(Some(&m), 0x100)), 1.0);
    m.Update(0.0);
    let p0 = m.GetPose(1).unwrap();
    let mut changed = false;
    let steps = 8;
    for _ in 0..steps {
        m.Update(dur / steps as f32 * 0.9);
        if let Some(p) = m.GetPose(1) {
            changed |= p.iter().zip(&p0).any(|(a, b)| a.f.iter().zip(b.f).any(|(x, y)| (x - y).abs() > 1e-4));
        }
    }
    assert!(changed, "pose never changed over time");
    let t = APIanim_GetTime(Some(&m), 0x100);
    assert!(t > 0.5 * dur && t <= dur, "{t} {dur}");
    // notification 0x11 once per Update
    assert_eq!(m.notifications.len(), steps + 1);
    assert_eq!(m.notifications[0], (0x11, 0x2a80_0001));
    APIanim_SetTime(Some(&mut m), 0x100, 0.0);
    assert_eq!(APIanim_GetTime(Some(&m), 0x100), 0.0);
    APIanim_StopAnim(Some(&mut m), 0x100);
    assert_eq!(APIanim_IsAnimPlaying(Some(&m), 0x100), 0);
    assert_eq!(APIanim_GetGroup(None, 0x100), u32::MAX);
}

#[test]
fn blend_params_from_mem() {
    let mut mem = Mem::new();
    let p = mem.malloc(0x14);
    mem.w32(p, 7);
    mem.wf(p + 4, 0.5);
    mem.wf(p + 8, 1.0);
    let b = ReadBlendParams(&mem, p).unwrap();
    assert_eq!((b.id, b.ramp, b.weight), (7, 0.5, 1.0));
    assert!(ReadBlendParams(&mem, 0).is_none());
}

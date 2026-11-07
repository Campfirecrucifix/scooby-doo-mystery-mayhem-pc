//! WP-S1: APIprojector / APIfbeffects manager semantics (pure state, synthetic data)
use scooby_recomp::engine::host::fbeffects::EFFBEffectMgr_BC;
use scooby_recomp::engine::host::projector::EFProjectorMgr_BC;

#[test]
fn projector_defaults_and_destroy() {
    let mut m = EFProjectorMgr_BC::default();
    let a = m.CreateProjector();
    let b = m.CreateProjector();
    assert_ne!(a, b);
    let p = &m.projectors[&a];
    assert_eq!((p.flags, p.size, p.reach, p.color), (1, [1.0, 1.0], 4.0, [1.0; 4]));
    m.with(a, |p| p.flags &= !1);
    assert_eq!(m.projectors[&a].flags, 0);
    m.DestroyProjector(a);
    m.DestroyProjector(a);
    assert!(!m.projectors.contains_key(&a) && m.projectors.contains_key(&b));
}

#[test]
fn fbeffect_slots_handles_enable() {
    let mut m = EFFBEffectMgr_BC::default();
    let a = m.CreateFB(2, 0x4000_0003);
    let b = m.CreateFB(2, 1);
    assert_eq!((a, b), (1, 2));
    m.EnableFB(b, 1);
    assert_eq!(m.slots[1].as_ref().unwrap().flags, 0x8000_0001);
    m.DestroyFB(a);
    // following slots shift down
    assert_eq!(m.slots[0].as_ref().unwrap().handle, b);
    assert!(m.slots[1].is_none());
    m.EnableFB(b, 0);
    assert_eq!(m.slots[0].as_ref().unwrap().flags, 1);
    assert_eq!(m.slots[0].as_ref().unwrap().obj, 0xffff_ffff);
}

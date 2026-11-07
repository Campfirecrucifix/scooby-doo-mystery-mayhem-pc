// P33 R1: EFGameObject::SetMyCurrentClipping (0x00209fe0) physic part, driven by EFWorld::ClipGameObjects.
use scooby_recomp::engine::host::gameobj::SetMyCurrentClipping;
use scooby_recomp::engine::physic::mgr::EFPhysicMgr;
use scooby_recomp::engine::physic::rwm::RwMatrix;
use std::collections::HashMap;

#[test]
fn p33_r1_set_my_current_clipping_physic_flags() {
    let mut mgr = EFPhysicMgr::new();
    let id = mgr.CreatePhysic(0x2a00_0042, RwMatrix::identity(), 0);
    let p = mgr.PhysicMut(id).unwrap();
    p.flags = 0xe;
    p.UpdatePhysicFlags();
    assert_eq!(mgr.Physic(id).unwrap().flags, 0xe_000e);
    let mut cur = HashMap::new();
    // word 0x80 (off the physic list): physic 0x40 -> 0x10000 (InitFrame / ray tests skip 0x10020)
    SetMyCurrentClipping(&mut cur, &mut mgr, 0x2a00_0042, 0x4a7);
    assert_eq!(mgr.Physic(id).unwrap().flags, 0xf_004e);
    // same 0x180 bits: no change (even if the flags were rewritten meanwhile)
    mgr.PhysicMut(id).unwrap().flags = 0x1234;
    SetMyCurrentClipping(&mut cur, &mut mgr, 0x2a00_0042, 0x6a0);
    assert_eq!(mgr.Physic(id).unwrap().flags, 0x1234);
    mgr.PhysicMut(id).unwrap().flags = 0x1_004e;
    // word 0x100: physic 0x80 -> dynamic/collide/particles off, stays listed
    SetMyCurrentClipping(&mut cur, &mut mgr, 0x2a00_0042, 0x100);
    assert_eq!(mgr.Physic(id).unwrap().flags, 0x8e);
    // word 0: restored from the low nibble
    SetMyCurrentClipping(&mut cur, &mut mgr, 0x2a00_0042, 0);
    assert_eq!(mgr.Physic(id).unwrap().flags, 0xe_000e);
    assert_eq!(cur[&0x2a00_0042], 0);
    // no physic: only +0x138 is stored
    SetMyCurrentClipping(&mut cur, &mut mgr, 0x2a00_0043, 0x80);
    assert_eq!(cur[&0x2a00_0043], 0x80);
}

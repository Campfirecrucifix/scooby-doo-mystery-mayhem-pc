//! WP-E2: physics manager + dynamic modules + APIphysic.
use scooby_recomp::engine::physic::api;
use scooby_recomp::engine::physic::mgr::*;
use scooby_recomp::engine::physic::module::*;
use scooby_recomp::engine::physic::physic::*;
use scooby_recomp::engine::physic::rwm::*;

struct W {
    target: Option<RwMatrix>,
}
impl PhysicWorld for W {
    fn GetObjectFrame(&self, uid: u32) -> Option<RwMatrix> { if uid == 77 { self.target } else { None } }
    fn CurrentLogicObject(&self) -> u32 { 77 }
}

fn setup() -> (EFPhysicMgr, PhysId) {
    let mut m = EFPhysicMgr::new();
    let id = api::CreatePhysic(&mut m, 5, 0, RwMatrix::identity());
    m.PhysicMut(id).unwrap().context = 1;
    api::SetFlags(&mut m, 5, 2); // bit1 -> DYNAMIC (0x20000)
    (m, id)
}
fn close(a: V3, b: V3) -> bool { (0..3).all(|i| (a[i] - b[i]).abs() < 1e-4) }

#[test]
fn handles_and_lists() {
    let (mut m, id) = setup();
    assert_eq!(m.PhysicOf(5), id);
    assert_eq!(api::CreatePhysic(&mut m, 5, 0, RwMatrix::identity()), 0);
    let h_rot = api::DMAdd(&mut m, 5, 10);
    let h_acc = api::DMAdd(&mut m, 5, 0);
    let h_mov = api::DMAdd(&mut m, 5, 4);
    assert_eq!(h_rot, 1 << 16 | 1);
    assert_eq!(h_acc, 2 << 16 | 1);
    // pre list sorted by type id
    assert_eq!(m.Physic(id).unwrap().pre_list, vec![h_acc, h_mov, h_rot]);
    assert_eq!(api::GetDM(&m, 5, 4, 0), h_mov);
    assert_eq!(api::GetDM(&m, 5, 1, 0), 0);
    assert_eq!(api::DMAdd(&mut m, 5, 0x99), 0);
    assert_eq!(api::DMAdd(&mut m, 6, 0), 0);
    m.DestroyModule(h_acc);
    assert!(m.Module(h_acc).is_none());
    assert_eq!(m.module_count, 2);
    // next alloc goes after last_slot (3), not into the freed slot 2
    let h = api::DMAdd(&mut m, 5, 0);
    assert_eq!(h >> 16, 4);
    m.DeletePhysic(id);
    m.UpdateDeletedPhysic();
    assert!(m.Physic(id).is_none());
    assert_eq!(m.module_count, 0);
    assert_eq!(m.PhysicOf(5), 0);
}

#[test]
fn acceleration_and_move_integrate() {
    let (mut m, id) = setup();
    let w = NoWorld;
    let ha = api::DMAdd(&mut m, 5, 0);
    api::DMSetContextMask(&mut m, ha, 1);
    api::DMSetParams(&mut m, ha, Some(&[fw(0.0), fw(-10.0), fw(0.0), fw(2.0), 1]), &w);
    let hm = api::DMAdd(&mut m, 5, 4);
    api::DMSetContextMask(&mut m, hm, 1);
    api::DMSetParams(&mut m, hm, Some(&[fw(1.0), 0, 0, 0]), &w);
    assert_eq!(api::DMGetParams(&mut m, hm, &w).unwrap()[0], fw(1.0));
    m.Update(0.5, false, &w, &mut NoCollision);
    let p = m.Physic(id).unwrap();
    assert!(close(p.velocity, [0.0, -5.0, 0.0]));
    // only the velocity is integrated by CollisionStage (disp feeds the collision solver)
    assert!(close(p.frame.pos, [0.0, -2.5, 0.0]));
    assert!(p.moved);
    assert_eq!(m.frame, 1);
    // inactive context: nothing
    m.PhysicMut(id).unwrap().context = 2;
    m.Update(0.5, false, &w, &mut NoCollision);
    assert!(close(m.Physic(id).unwrap().velocity, [0.0, -5.0, 0.0]));
    // one-shot (DMSetActive -1)
    api::DMSetActive(&mut m, ha, 0xffff_ffff);
    assert!(api::DMIsActive(&m, ha));
    m.Update(0.5, false, &w, &mut NoCollision);
    assert!(close(m.Physic(id).unwrap().velocity, [0.0, -10.0, 0.0]));
    assert!(!api::DMIsActive(&m, ha));
}

#[test]
fn trackobjet_turns_toward_target() {
    let (mut m, id) = setup();
    let w = W { target: Some(RwMatrix::from_pos([10.0, 0.0, 0.0])) };
    let h = api::DMAdd(&mut m, 5, 1);
    api::DMSetContextMask(&mut m, h, 1);
    // SELF id resolves to the current logic object (77)
    api::DMSetParams(&mut m, h, Some(&[0x2a00_0000, fw(100.0), 0, 0, 0, 1]), &w);
    assert_eq!(api::DMGetParams(&mut m, h, &w).unwrap()[0], 77);
    m.Update(0.1, false, &w, &mut NoCollision);
    let f = m.Physic(id).unwrap().frame;
    assert!(close(f.at, [1.0, 0.0, 0.0]), "{:?}", f.at);
    // lost target -> DM state bit0 + notify 6
    api::DMSetStateChangeNotifyBits(&mut m, h, 1);
    let w2 = W { target: None };
    m.Update(0.1, false, &w2, &mut NoCollision);
    assert_eq!(api::DMGetState(&m, h), 1);
    assert!(m.notify.iter().any(|n| n.code == 6 && n.data[..3] == [h, 0x1000_0001, 1]));
}

#[test]
fn rotate_shaking_airgrip() {
    let (mut m, id) = setup();
    let w = NoWorld;
    m.PhysicMut(id).unwrap().frame.pos = [1.0, 2.0, 3.0];
    let h = api::DMAdd(&mut m, 5, 10);
    api::DMSetContextMask(&mut m, h, 1);
    api::DMSetParams(&mut m, h, Some(&[fw(std::f32::consts::FRAC_PI_2), 0, fw(2.0), 0]), &w);
    m.Update(1.0, false, &w, &mut NoCollision);
    let f = m.Physic(id).unwrap().frame;
    assert!(close(f.pos, [1.0, 2.0, 3.0]));
    assert!(close(f.at, [1.0, 0.0, 0.0]) || close(f.at, [-1.0, 0.0, 0.0]));
    // AirGrip pulls velocity toward the dyn displacement (zero) on x/z, world space
    let g = api::DMAdd(&mut m, 5, 7);
    api::DMSetContextMask(&mut m, g, 1);
    api::DMSetParams(&mut m, g, Some(&[fw(1.0), 8 | 1]), &w);
    m.PhysicMut(id).unwrap().velocity = [4.0, 4.0, 0.0];
    api::DMSetActive(&mut m, h, 0);
    m.Update(0.5, false, &w, &mut NoCollision);
    assert!(close(m.Physic(id).unwrap().velocity, [2.0, 4.0, 0.0]));
    // Shaking: GetParams is empty, DoAction 1 resets time
    let s = api::DMAdd(&mut m, 5, 0x16);
    api::DMSetParams(&mut m, s, Some(&[1, fw(1.0), fw(0.1), fw(0.25), fw(0.25), fw(2.0)]), &w);
    api::DMDoAction(&mut m, s, 1, &mut [], &w);
}

#[test]
fn falling_state_and_notify() {
    let (mut m, id) = setup();
    let w = NoWorld;
    api::SetStateChangeNotifyBits(&mut m, 5, 2);
    api::SetVelocity(&mut m, 5, [0.0, -3.0, 0.0], &w);
    m.Update(0.1, false, &w, &mut NoCollision);
    assert_eq!(m.Physic(id).unwrap().state & 2, 2);
    assert!(m.notify.iter().any(|n| n.code == 5 && n.owner == 5));
    assert_eq!(api::GetFloorNormal(&m, 5), (0, [0.0; 3]));
}

#[test]
fn query_stamp_counts_queries() {
    let mut m = EFPhysicMgr::new();
    let rin = EFRayIntersectIn::default();
    let r = m.RayIntersect(&rin, &mut NoCollision);
    assert_eq!(r.kind, 0);
    assert_eq!(m.query_stamp, 1);
}

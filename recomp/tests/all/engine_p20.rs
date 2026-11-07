//! WP-P20: l03_camera (c2rs build 26ab69f9, EP1/L03B) follow-camera float globals. The generated port
//! stored ResetCamera's `fDistanceCameraFollow = 3.6` as the int 3 (bit pattern = a denormal) and did
//! int arithmetic on float bits in the follow state, so the distance went to garbage and the camera sat
//! on the hero looking up (L03B closet: the hero stalled because its move heading follows the camera).
use crate::common::*;
use scooby_recomp::engine::iso::IsoFs;

#[test]
fn p20_l03b_follow_camera_distance() {
    if !std::path::Path::new(ISO).exists() {
        return;
    }
    let fs = IsoFs::open(ISO).unwrap();
    let (mut e, mut rt) = load_carried(&fs, "EP1/L03B", &[]).unwrap();
    for _ in 0..300 {
        frame(&mut e, &mut rt, 0);
    }
    let c = e.rt.codes.iter().position(|c| c.def.name == "l03_camera").expect("l03_camera code");
    let data = e.rt.codes[c].data;
    // CameraFree.c globals: fDistanceCameraFollow 0x1032c, fDistanceCameraFollowTarget 0x10334
    let d = f32::from_bits(e.mem.r32(data + 0x32c));
    let t = f32::from_bits(e.mem.r32(data + 0x334));
    assert!((0.5..10.0).contains(&d), "fDistanceCameraFollow {d} ({:#x})", d.to_bits());
    assert!((2.0..10.0).contains(&t), "fDistanceCameraFollowTarget {t}");
}

/// WP-P20 R44: the runtime PhysicWorld must resolve bone frames from the renderer-published HAnim
/// nodes (RuntimeWorld::nodes). It used to return None, so EFPhysicSystem::SyncModels pinned every
/// bone-attached coll prim to the root frame (EP2/L01A castlegate 0x16a: the gate box on node 1
/// stayed closed after its open anim, the team could not reach exit door 0x122).
#[test]
fn p20_runtime_physic_world_get_bone_uses_nodes() {
    use scooby_recomp::engine::camera::DTMatrix;
    use scooby_recomp::engine::physic::module::PhysicWorld;
    use scooby_recomp::engine::physic::rwm::RwMatrix;
    use scooby_recomp::engine::runtime::{RuntimeInput, RuntimePhysicWorld, RuntimeWorld};
    let m = |y: f32| DTMatrix { right: [1., 0., 0.], up: [0., 1., 0.], at: [0., 0., 1.], pos: [880.7, y, 291.75] };
    let mut world = RuntimeWorld::default();
    world.nodes.insert(0x2a00_016a, vec![(m(0.0), m(1.73)), (m(2.6), m(4.33))]);
    world.node_ids.insert(0x2a00_016a, vec![0x10000, 0x10001]);
    let input = RuntimeInput::default();
    let w = RuntimePhysicWorld { world: &world, input: &input, camera: RwMatrix::identity(), splines: &[], anim: None, cam_req: Default::default() };
    assert_eq!(w.GetBone(0x2a00_016a, 1).map(|b| (b.0.pos[1], b.1.pos[1])), Some((2.6, 4.33)));
    assert_eq!(w.GetBone(0x2a00_016a, 0x10001).map(|b| b.1.pos[1]), Some(4.33));
    assert!(w.GetBone(0x2a00_016a, 2).is_none());
    assert!(w.GetBone(0x2a00_0001, 0).is_none());
}

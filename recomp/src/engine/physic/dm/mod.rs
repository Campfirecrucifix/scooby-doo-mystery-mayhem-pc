//! EFDM* dynamic modules + the DMAdd factory (APIphysic DMAdd switch).
pub mod basic;
pub mod analog;
pub mod edge;
pub mod pfweighttilt;
pub mod ropedetect;
pub mod envdetect;
pub mod objcoll;
pub mod plugin;
pub mod constraint_rotation;
pub mod wave;
pub mod primmove;
pub mod follow;
pub mod spline_move;
pub mod analog_motion;
pub mod bone_track;
pub mod camera_flex;
pub mod ik_two_joints;
pub mod particle_launcher;
pub mod quadruped;
pub mod tung;

pub mod ray_intersect;
pub mod env_coll;
pub mod edge_detect;
pub mod edge_down;
pub mod grab_edge;
pub mod climb;
pub mod rope_slide;
pub mod water_simulation;
pub mod floating_object;
use super::module::EFDM;
use basic::*;
use analog::*;

/// The DMAdd(kind) switch: constructs the module (with the per-case initial values). Kinds
/// whose classes are not ported return None (the API then returns 0, as for an unknown kind).
pub fn NewModule(kind: u32) -> Option<Box<dyn EFDM>> {
    Some(match kind {
        0 => Box::new(EFDMAcceleration::new()),
        1 => Box::new(EFDMTrackObjet::new()),
        2 => Box::new(EFDMAnalogRotation::new()),
        5 => Box::new(EFDMAnalogMove::new()),
        8 => Box::new(EFDMGroundGrip::new()),
        4 => Box::new(EFDMMove::new()),
        7 => Box::new(EFDMAirGrip::new()),
        10 => Box::new(EFDMRotate::new()),
        0x16 => Box::new(EFDMShaking::new()),
        0xb => Box::new(primmove::EFDMPrimMove::new()),
        9 => Box::new(wave::EFDMWave::new()),
        0xe => Box::new(constraint_rotation::EFDMConstraintRotation::new()),
        0x25 => Box::new(plugin::EFDMPlugin::new()),
        0x23 => Box::new(objcoll::EFDMObjColl::new()),
        0x1a => Box::new(envdetect::EFDMEnvDetect::new()),
        0x1e => Box::new(ropedetect::EFDMRopeDetect::new()),
        0x10 => Box::new(pfweighttilt::EFDMPFWeightTilt::new()),
        0x17 => Box::new(edge::EFDMEdge::new()),
        0x14 => Box::new(follow::EFDMFollow::new()),
        3 => Box::new(spline_move::EFDMSplineMove::new()),
        6 => Box::new(analog_motion::EFDMAnalogMotion::new()),
        0x1c => Box::new(bone_track::EFDMBoneTrackObject::new()),
        0x12 => Box::new(camera_flex::EFDMCameraFlex::new()),
        0x20 => Box::new(ik_two_joints::EFDMIKTwoJoints::new()),
        0x1d => Box::new(particle_launcher::EFDMParticleLauncher::new()),
        0x1f => Box::new(quadruped::EFDMQuadruped::new()),
        0x24 => Box::new(tung::EFDMTung::new()),
        0x1b => Box::new(ray_intersect::EFDMRayIntersect::new()),
        0x22 => Box::new(env_coll::EFDMEnvColl::new()),
        0x19 => Box::new(edge_detect::EFDMEdgeDetect::new()),
        0x13 => Box::new(edge_down::EFDMEdgeDown::new()),
        0x18 => Box::new(grab_edge::EFDMGrabEdge::new()),
        0x21 => Box::new(climb::EFDMClimb::new()),
        0x11 => Box::new(rope_slide::EFDMRopeSlide::new()),
        0xc | 0xd => Box::new(water_simulation::EFDMWaterSimulation::new(kind)),
        0x15 => Box::new(floating_object::EFDMFloatingObject::new()),
        _ => return None,
    })
}

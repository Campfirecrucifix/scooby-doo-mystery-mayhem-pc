//! EFPhysic (0xbc bytes) and EFPhysicData (Classes/EF/EFPhysic.cpp, EFPhysicData.cpp), 1:1.
//!
//! Memory model: native (the logic modules never get an EFPhysic pointer; they only talk through
//! object ids and DM handles). The C++ intrusive lists become:
//! - EFPhysicMgr list links (+0 next, +4 prev-link, +8 self): the manager keeps `Vec<PhysId>` lists.
//! - module lists (+0x10 pre, +0x14 col, +0x18 post, +0x1c all): `Vec<u32>` of DM handles in list
//!   order (the modules live in the manager's handle table, see mgr.rs).
//! - +0xac instance (EFPhysicData*): `owner` = the game object uid (instance+8) plus the instance's
//!   root frame matrix `frame` (RwFrameGetLTM(instance+0x14)), which the coordinator keeps in sync
//!   with the EFInstance/pos data.
//! - +0x54 EFCollisionModel (embedded, model.rs): EFPhysic+0x58 prims, +0x60 bsphere_center,
//!   +0x6c bsphere_radius, +0x70 bsphere_world, +0x80 primary, +0x90 b3c, +0x94.. masks.
//!   Filled by EF3dObj::CreateCollisionPrimitives (create_prims.rs) or SetSpherePrimaryVolume.
#![allow(non_snake_case)]

use super::rwm::*;

/// Index+1 of an EFPhysic in `EFPhysicMgr::physics` (0 = NULL).
pub type PhysId = u32;

/// EFPhysic+0xc flag bits used by the engine.
pub mod flags {
    /// bit5: deleted (DeletePhysic sets 0x21)
    pub const DELETED: u32 = 0x20;
    pub const DISABLED: u32 = 0x10000; // bit16 (mirror of bit0, or bit6)
    pub const DYNAMIC: u32 = 0x20000; // bit17: run the dynamic modules / move the frame
    pub const COLLIDE: u32 = 0x40000; // bit18: go through EFCollisionSystem
    pub const PARTICLES: u32 = 0x80000; // bit19: CheckParticleCollisions
    pub const RELATIVE_VEL: u32 = 0x400000; // bit22: velocities are relative to a carrier
}

pub use super::model::{EFCollisionModel, ObjPrim};

/// EFEnvPrimAttribs (0x20 bytes, the collision-map material record copied by GetFloorAttribs).
pub type EnvPrimAttribs = [u32; 8];

#[derive(Debug, Clone)]
pub struct EFPhysic {
    /// +0x08 self / +0xac instance: game object uid of the owner (instance+8)
    pub owner: u32,
    /// RwFrameGetLTM(instance+0x14): the owner's root frame (see module doc)
    pub frame: RwMatrix,
    /// instance+0xc bit5 "frame moved this frame" (cleared in PreCollisionStage)
    pub moved: bool,
    /// +0x0c flags
    pub flags: u32,
    /// +0x10 PreColUpdate modules (type bit 0x10000000)
    pub pre_list: Vec<u32>,
    /// +0x14 ColUpdate modules (type bit 0x20000000)
    pub col_list: Vec<u32>,
    /// +0x18 PostColUpdate modules (type bit 0x40000000)
    pub post_list: Vec<u32>,
    /// +0x1c every module, sorted by type id
    pub all_list: Vec<u32>,
    /// +0x20 priority (APIphysic::SetPriority)
    pub priority: u32,
    /// +0x24 weight (1.0)
    pub weight: f32,
    /// +0x28 velocity
    pub velocity: V3,
    /// +0x34 physic state (bit0 on floor, bit1 falling, bit2 ...)
    pub state: u32,
    /// +0x38 state change notify mask
    pub notify_bits: u32,
    /// +0x3c current context mask (DM context masks are tested against it)
    pub context: u32,
    /// +0x40 falling speed threshold (-1.5)
    pub falling_speed: f32,
    /// +0x44 falling delay (0.2)
    pub falling_delay: f32,
    /// +0x48 falling timer (starts = delay)
    pub falling_timer: f32,
    /// +0x4c friction (1.0)
    pub friction: f32,
    /// +0x50 floor slope cosine (cos 45° = 0x3f350481)
    pub floor_cos: f32,
    /// +0x54 collision model (embedded EFCollisionModel)
    pub model: EFCollisionModel,
    /// +0xb0 hardness (1.0)
    pub hardness: f32,
    /// +0xb4 rebound (0)
    pub rebound: f32,
    /// +0xb8 floor contact: (attribs of the floor material, floor normal at contact+0xc)
    pub floor: Option<(EnvPrimAttribs, V3)>,
}

impl EFPhysic {
    /// EFPhysic::EFPhysic(EFPhysicData*, uint)
    pub fn new(owner: u32, frame: RwMatrix) -> Self {
        EFPhysic {
            owner,
            frame,
            moved: false,
            flags: 0,
            pre_list: Vec::new(),
            col_list: Vec::new(),
            post_list: Vec::new(),
            all_list: Vec::new(),
            priority: 0,
            weight: 1.0,
            velocity: [0.0; 3],
            state: 0,
            notify_bits: 0,
            context: 0,
            falling_speed: f32::from_bits(0xbfc0_0000),
            falling_delay: f32::from_bits(0x3e4c_cccd),
            falling_timer: f32::from_bits(0x3e4c_cccd),
            friction: 1.0,
            floor_cos: f32::from_bits(0x3f35_0481),
            model: EFCollisionModel::new(0),
            hardness: 1.0,
            rebound: 0.0,
            floor: None,
        }
    }

    /// EFPhysic::UpdatePhysicFlags: bits 16..19 mirror bits 0..3; bit6 forces bit16; bits 7/8
    /// clear bits 17..19.
    pub fn UpdatePhysicFlags(&mut self) {
        let f = self.flags;
        let mut r = (f & 0xfff0_ffff) | ((f & 0xf) << 16);
        if f & 0x40 != 0 {
            r |= 0x10000;
        }
        if f & 0x180 != 0 {
            r &= 0xfff1_ffff;
        }
        self.flags = r;
    }

    /// EFPhysic::UpdatePhysicState(dt, phys, dyn): falling detection + state change notify.
    /// Returns the notification (state) to send to the instance (EFInstance::Notify(5, &state)).
    pub fn UpdatePhysicState(&mut self, dt: f32, prev_state: u32) -> Option<u32> {
        if self.state == 0 {
            let t = self.falling_timer;
            self.falling_timer = t + dt;
            if self.falling_delay <= t + dt && self.velocity[1] < self.falling_speed {
                self.state |= 2;
            }
        } else {
            self.falling_timer = 0.0;
        }
        let ch = self.state ^ prev_state;
        if ch != 0 && ch & self.notify_bits != 0 { Some(self.state) } else { None }
    }
}

/// EFPhysicData: EFPosData + `+0x28 EFPhysic*`. Here only the link is modelled (EFPosData is the
/// game object pos data, see core/apigameobj); the coordinator keeps one per game object.
#[derive(Debug, Clone, Copy, Default)]
pub struct EFPhysicData {
    /// +0x28
    pub physic: PhysId,
}

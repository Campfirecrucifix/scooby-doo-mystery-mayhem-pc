//! WP-I2: the PL particle system (EXE.1 Classes/PL/*: PLMgr_BC/PLMgr, PLType_BC/PLType,
//! PLEmitter_BC/PLEmitter, PLParticle, PLInterpol, PLLinkList) and the APIparticle_* host functions
//! (Engine API/EFApi_Particle.cpp).
//!
//! Memory model: everything is native Rust data (the logic modules only see integer ids).
//! PLLinkList<T> (intrusive singly linked lists, AddFirst = push at head) is replaced by Vec /
//! VecDeque with index 0 = head, so iteration order (update + draw order) is the original one.
//! The particle pool (PLMgr_BC +0x14, 600 x PLParticle 0x2c) is a Vec + free list of indices.
//! RwFrame/RwMatrix are replaced by [`PLFrame`] (an LTM the coordinator refreshes with
//! [`PLMgr_BC::SyncFrames`]). Rendering (RwIm3D / VU1 DMA) is replaced by a backend-agnostic
//! [`PLDrawList`] built by [`PLMgr_BC::Render`].

#![allow(non_snake_case, non_camel_case_types)]

pub mod api;
pub mod emitter;
pub mod mgr;
pub mod types;

pub use emitter::*;
pub use mgr::*;
pub use types::*;

pub use crate::engine::camera::DTMatrix;

pub type V3 = [f32; 3];

/// PLInterpol (0x20 bytes): a two-segment piecewise-linear curve over the particle age.
/// +0 v0, +4 v1, +8 v2, +0xc d1, +0x10 d2, +0x14 t1, +0x18 t2, +0x1c constant flag.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PLInterpol {
    pub v0: f32,
    pub v1: f32,
    pub v2: f32,
    pub d1: f32,
    pub d2: f32,
    pub t1: f32,
    pub t2: f32,
    pub constant: u32,
}

impl PLInterpol {
    /// PLInterpol::UpdateDelta (0x0020fcc0)
    pub fn UpdateDelta(&mut self) {
        self.d1 = if self.t1 == 0.0 { 0.0 } else { (self.v1 - self.v0) / self.t1 };
        self.d2 = if self.t2 - self.t1 == 0.0 { 0.0 } else { (self.v2 - self.v1) / (self.t2 - self.t1) };
    }
    /// The inlined evaluation used by PLEmitter::Render2dParticles/Render3dParticles.
    pub fn Get(&self, age: f32) -> f32 {
        if self.constant != 0 {
            self.v0
        } else if age < self.t1 {
            self.v0 + self.d1 * age
        } else {
            self.v1 + self.d2 * (age - self.t1)
        }
    }
    /// The "Set(v0,v1,v2,t1)" pattern repeated in SetColor/SetRotation/Initialize: a constant curve
    /// when v0==v1==v2 (no UpdateDelta), else UpdateDelta. t2 is always set by the caller.
    pub(crate) fn set3(&mut self, a: f32, b: f32, c: f32, t1: f32, t2: f32) {
        if a == b && a == c {
            self.constant = 1;
            self.v0 = a;
            self.v1 = a;
            self.v2 = a;
            self.t1 = t1;
            self.t2 = t2;
        } else {
            self.constant = 0;
            self.v0 = a;
            self.v1 = b;
            self.v2 = c;
            self.t1 = t1;
            self.t2 = t2;
            self.UpdateDelta();
        }
    }
}

/// PLParticle (0x2c bytes): +0 next (list), +4 pos, +0x10 velocity, +0x1c age, +0x20 start size,
/// +0x24 RwMatrix* (only for "3D" types, type flag 1), +0x28 u16 collision stamp
/// (EFObjCollision*::CheckParticlesIntersection marks with PLParticle::mui16GlobalCount).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PLParticle {
    pub pos: V3,
    pub vel: V3,
    pub age: f32,
    pub size: f32,
    pub matrix: Option<DTMatrix>,
    pub stamp: u16,
}

/// PS2 libc rand() (newlib 64-bit LCG, RAND_MAX 0x7fffffff). Replacement for the global libc
/// state: the particle manager owns its own generator (same sequence as a fresh libc seed 1, but
/// not interleaved with the other rand() users of the engine).
#[derive(Debug, Clone)]
pub struct CRand(pub u64);
impl Default for CRand {
    fn default() -> Self {
        CRand(1)
    }
}
impl CRand {
    pub fn rand(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1);
        ((self.0 >> 32) & 0x7fff_ffff) as u32
    }
}

/// C float -> int -> byte conversion as compiled (values >= 2^31 are rebased).
pub(crate) fn f2b(f: f32) -> u8 {
    if f < 2.147_483_6e9 { f as i32 as u8 } else { (f - 2.147_483_6e9) as i32 as u8 }
}

pub(crate) fn mat_point(m: &DTMatrix, v: V3) -> V3 {
    let mut r = mat_vec(m, v);
    for i in 0..3 {
        r[i] += m.pos[i];
    }
    r
}
pub(crate) fn mat_vec(m: &DTMatrix, v: V3) -> V3 {
    let mut r = [0.0; 3];
    for i in 0..3 {
        r[i] = v[0] * m.right[i] + v[1] * m.up[i] + v[2] * m.at[i];
    }
    r
}
/// child LTM = parent * local (RwFrame hierarchy).
pub fn mat_mul(local: &DTMatrix, parent: &DTMatrix) -> DTMatrix {
    DTMatrix {
        right: mat_vec(parent, local.right),
        up: mat_vec(parent, local.up),
        at: mat_vec(parent, local.at),
        pos: mat_point(parent, local.pos),
    }
}

/// One textured sprite of a "2D" type (PLEmitter::RenderBatch input: pos, packed colour,
/// rotation, size). The VU1 microcode that expanded them is not available: `size` is the value
/// the CPU uploads (scale * 0.75) and `rot_index` the 7-bit index into the 128-entry sin/cos
/// tables at 0x1f656d0/0x1f658d0 (assumed to cover one full turn: angle = rot_index * 2pi/128).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PLSprite {
    pub pos: V3,
    /// R,G,B,A = interpolated value * 120 (PS2 GS: 0x80 = 1.0)
    pub rgba: [u8; 4],
    pub rot_index: u8,
    pub size: f32,
}

/// One quad of a "3D" (oriented, type flag 1) type: PLEmitter::Render3dParticles Im3D vertices
/// (pos, uv) + indices {0,1,2, 2,3,0}; colour RGB * 120, alpha fixed 0x78 (quirk).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PLQuad {
    pub verts: [V3; 4],
    pub uvs: [[f32; 2]; 4],
    pub rgba: [u8; 4],
}

#[derive(Debug, Clone, PartialEq)]
pub enum PLPrims {
    Sprites(Vec<PLSprite>),
    Quads(Vec<PLQuad>),
}

/// Draw batch of one emitter.
#[derive(Debug, Clone, PartialEq)]
pub struct PLDrawBatch {
    pub emitter: u32,
    pub texture: String,
    /// RpSkyRenderStateSet(2, ...) alpha-blend code: 0x29 (default), 0x44 (type flag 0x200),
    /// 0xa1 (type flag 0x80).
    pub blend: u32,
    /// type flag 0x100: EFGraphicsMgr_SP::EnableStencilRead around the emitter.
    pub stencil_read: bool,
    pub prims: PLPrims,
}

/// Per-frame particle draw list. Render states set around it by PLEmitter::BeginRender:
/// z-write off (state 8 = 0), src blend 3 / dst blend 2? (states 2/9), state 0x14 = 1; fog off (0xe).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PLDrawList {
    pub batches: Vec<PLDrawBatch>,
}

/// Where an emitter frame comes from (replaces RwFrame parenting).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PLFrameAttach {
    /// EFPosData::CreateFrame(NULL): own root frame, LTM = local
    Own,
    /// own frame added as child of game object `obj`'s frame `bone` (PLMgr_BC::LoadEmitters)
    Child { obj: u32, bone: u32 },
    /// EmitterSetJoint: the emitter uses the game object's frame itself
    ObjectFrame { obj: u32, bone: u32 },
    /// EmitterSetJoint on a non-0x2a uid: EFWorld::GetObjectPosData(uid)->frame
    PosData { uid: u32 },
}

/// EFPosData +0x14 frame replacement.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PLFrame {
    pub attach: PLFrameAttach,
    /// modelling matrix (Own / Child)
    pub local: DTMatrix,
    /// RwFrameGetLTM result, refreshed by PLMgr_BC::SyncFrames
    pub ltm: DTMatrix,
}

/// The engine services the particle code queries (EFWorld / EFGameObject); implemented by the
/// coordinator.
pub trait PLWorld {
    /// _gpoWorldMgr +0x14 (world paused)
    fn IsPaused(&self) -> bool {
        false
    }
    /// EFWorld::GetGameObject(uid) != NULL
    fn GameObjectExists(&self, uid: u32) -> bool;
    /// EFGameObject::GetObjectFrame(bone) LTM (None = NULL frame)
    fn GetObjectFrame(&self, obj: u32, bone: u32) -> Option<DTMatrix>;
    /// EFWorld::GetObjectPosData(uid)->frame LTM
    fn GetObjectPosDataFrame(&self, uid: u32) -> Option<DTMatrix>;
}

/// World with nothing in it (tests / before the level is built).
pub struct PLNoWorld;
impl PLWorld for PLNoWorld {
    fn GameObjectExists(&self, _: u32) -> bool {
        false
    }
    fn GetObjectFrame(&self, _: u32, _: u32) -> Option<DTMatrix> {
        None
    }
    fn GetObjectPosDataFrame(&self, _: u32) -> Option<DTMatrix> {
        None
    }
}

/// EFNotificationMgr::Notify(_gpoGameLogicMgr+0x14, 0x1a, handler, emitterId, {clipped, id}, 0)
/// queued for the coordinator (PLEmitter_BC::UpdateClippingState).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PLNotification {
    pub msg: u32,
    pub handler: u32,
    pub sender: u32,
    /// payload word 0: 1 = became clipped, 0 = became visible
    pub clipped: u32,
    /// payload word 1: emitter id
    pub emitter: u32,
}

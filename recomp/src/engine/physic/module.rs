//! EFPhysicModule (Classes/EF/EFPhysicModule.cpp) — base of every EFDM* dynamic module — and the
//! virtual interface (`EFDM` trait = the C++ vtable: 0x08 dtor, 0x0c PreColUpdate, 0x10 ColUpdate,
//! 0x14 PostColUpdate, 0x18 SetParams, 0x1c GetParams, 0x20 DoAction, 0x24 GetData, InitDM).
#![allow(non_snake_case)]

use super::physic::*;
use super::rwm::*;

/// GetType()'s local static `_uiAllTypes` (0x70000000): the list bits of a module type.
pub const ALL_TYPES: u32 = 0x7000_0000;
pub const TYPE_PRE: u32 = 0x1000_0000;
pub const TYPE_COL: u32 = 0x2000_0000;
pub const TYPE_POST: u32 = 0x4000_0000;

/// EFPhysicModule fields (0x30 bytes).
#[derive(Debug, Clone, Default)]
pub struct EFPhysicModule {
    /// +0x04 owner physic
    pub physic: PhysId,
    /// +0x18 type: list bits (0x70000000) | DMAdd kind
    pub type_: u32,
    /// +0x1c context mask (active when `context & physic.context != 0`)
    pub context_mask: u32,
    /// +0x20 DM state bits
    pub state: u32,
    /// +0x24 state change notify mask
    pub notify_bits: u32,
    /// +0x28 flags: bit0 active (1 at creation), 0x10/0x20/0x40 one-shot pre/col/post
    pub flags: u32,
    /// +0x2c handle (slot << 16 | serial)
    pub handle: u32,
}

impl EFPhysicModule {
    /// Is the module selected this frame (`flags&1 && context_mask & physic.context`)?
    pub fn InContext(&self, phys_context: u32) -> bool {
        self.flags & 1 != 0 && self.context_mask & phys_context != 0
    }
    /// The DMSetStateBits / module-internal "state |= bits; notify(6) on change" pattern.
    pub fn SetStateBits(&mut self, bits: u32, ctx: &mut DMCtx) {
        let n = self.state | bits;
        self.ChangeState(n, ctx);
    }
    pub fn ResetStateBits(&mut self, bits: u32, ctx: &mut DMCtx) {
        let n = self.state & !bits;
        self.ChangeState(n, ctx);
    }
    fn ChangeState(&mut self, n: u32, ctx: &mut DMCtx) {
        let ch = self.state ^ n;
        self.state = n;
        if ch & self.notify_bits != 0 {
            // EFInstance::Notify(instance, 6, {handle, type, state}, 0)
            ctx.notify.push(PhysicNotify { owner: ctx.phys.owner, code: 6, data: [self.handle, self.type_, self.state, ch] });
        }
    }
}

/// EFInstance::Notify(instance, code, data) emitted by the physics (code 5 = physic state change
/// {state}, code 6 = DM state change {handle, type, state}). The coordinator forwards them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PhysicNotify {
    pub owner: u32,
    pub code: u32,
    /// code 6: [handle, type, new state, changed bits] (EFInstance::Notify 6 payload)
    pub data: [u32; 4],
}

/// Seam for the engine state the dynamic modules read outside the physics (EFWorld, EFInputMgr,
/// EFGraphicsMgr camera, logic stack). Every method has a neutral default.
pub trait PhysicWorld {
    /// EFWorld::GetGameObject(uid) != NULL, and its root frame LTM (obj+0x14)
    fn GetObjectFrame(&self, _uid: u32) -> Option<RwMatrix> { None }
    /// EFPhysicMgr::RayIntersect(_gpoPhysicMgr, in, out) != 0 issued by a module (WP-E7:
    /// EFDMFollow::IsIntersecting). The manager is borrowed during module updates, so the host
    /// answers it (e.g. against a snapshot / interior-mutable collision system).
    fn RayIntersect(&self, _rin: &super::mgr::EFRayIntersectIn) -> bool { false }
    /// _gpoSplineMgr spline table (+0x10, count +8) for EFDMSplineMove (WP-E7)
    fn Splines(&self) -> &[crate::engine::spline::EFQuadraticSpline] { &[] }
    /// WP-E9 Tung: `*(*(physic + 0xac) + 8)` — the game-object uid owning the physic hit by a ray
    /// (EFRayResult::physic, EFRayIntersectOut +0x1c). Default 0.
    fn PhysicOwnerUid(&self, _physic: super::physic::PhysId) -> u32 { 0 }
    /// WP-E9 Tung: EFInstance::Notify(game object `owner`, code, data, 0) with a message block
    /// larger than PhysicNotify's 3 words (code 3 = {1, 0x66, hit uid, 0, 0,0,0,0, v.xyz, 0,0,0}).
    fn InstanceNotify(&self, _owner: u32, _code: u32, _data: &[u32]) {}
    /// WP-E10 RopeSlide::ComputeIK: the frame of `owner`'s collision prim `prim`
    /// (EFObjCollisionPrim +0xdc, after its vtbl+0xc update) as a GetBone bone id. None = NULL frame.
    fn PrimFrameBone(&self, _owner: u32, _prim: u32) -> Option<u32> { None }
    /// WP-E9 IKTwoJoints: the parent frame (RwFrame +4) of a GetBone bone, as a GetBone bone id.
    fn GetBoneParent(&self, _uid: u32, _bone: u32) -> Option<u32> { None }
    /// WP-E9 IKTwoJoints: per StageContactPolys polygon (same order), its material (poly +0)
    /// flag words (+4, +8). Missing entries = (0, 0).
    fn StageContactPolyMaterials(&self, _owner: u32) -> Vec<(u32, u32)> { Vec::new() }
    /// WP-E9 ParticleLauncher: `lVar7 = PLMgr_BC::GetEmitter(_gpoPLParticleMgr, emitter)`, then
    /// `PLEmitter_BC::LaunchParticleImmediately(lVar7)` and the particle matrix `*(particle + 0x24)`.
    /// When all three are non-null call `setup` once with that matrix (RwMatrix layout: right,
    /// up, at, pos) and store it back; return true. Host: `PLMgr::emitter_mut(emitter)` +
    /// `PLEmitter::LaunchParticleImmediately(ty, &mut pool)`, then the launched particle
    /// (`emitter.particles[0]`) `pool.parts[pi].matrix` (Some only for type flags&1). Default: no-op.
    fn LaunchParticle(&self, _emitter: i32, _setup: &mut dyn FnMut(&mut RwMatrix)) -> bool { false }
    /// WP-E9 ParticleLauncher: libc `rand()` (the runtime's LibcRand). Default 0.
    fn Rand(&self) -> u32 { 0 }
    /// EFInputMgr::InputActionGet(a, b) -> analog value {x, y}
    fn InputActionGet(&self, _a: u32, _b: u32) -> Option<[f32; 2]> { None }
    /// WP-E9 Quadruped: EFDMQuadruped::FindNearestNormal's walk of the EFCollisionStage contact list
    /// (+4, count +8) of `owner`'s physic. Per contact polygon: (plane normal +0xc, plane d +0x18,
    /// vertices +0x1c (count +6 & 0x7fff), edge planes {nx,ny,nz,d} after the vertices).
    #[allow(clippy::type_complexity)]
    fn StageContactPolys(&self, _owner: u32) -> Vec<(V3, f32, Vec<V3>, Vec<[f32; 4]>)> { Vec::new() }
    /// WP-E9 Quadruped: RwFrameGetLTM(bone frame +4 = parent frame) of a GetBone bone. Default
    /// derives it from GetBone (LTM = modelling * parentLTM).
    fn GetBoneParentLTM(&self, uid: u32, bone: u32) -> Option<RwMatrix> {
        self.GetBone(uid, bone).map(|(m, ltm)| RwMatrix::Multiply(&m.Invert(), &ltm))
    }
    /// EFGraphicsMgr::GetCamera(0) frame LTM
    fn GetCameraLTM(&self) -> RwMatrix { RwMatrix::identity() }
    /// The currently running logic object's uid (_gpoLogicStack top +8) for the SELF (0x2a000000) id
    fn CurrentLogicObject(&self) -> u32 { 0 }
    /// obj+0x1c parent link -> parent game object uid (carrier for RELATIVE_VEL physics)
    fn GetParentObject(&self, _uid: u32) -> Option<u32> { None }
    /// WP-E9 CameraFlex: EFPhysicMgr::RayIntersect(_gpoPhysicMgr, in, out) with the EFRayIntersectOut
    /// read back (hit position +4, normal +0x10). None = no hit (return 0).
    fn RayIntersectResult(&self, _rin: &super::mgr::EFRayIntersectIn) -> Option<super::mgr::EFRayResult> { None }
    /// WP-E9 CameraFlex: target obj+0x28 (when != 0 and != -0x54) -> float +0x6c (distance scale
    /// of EFDMCameraFlex::UpdateCameraDistance). None = 1.0.
    fn CameraFlexTargetScale(&self, _uid: u32) -> Option<f32> { None }
    /// WP-E9 CameraFlex: EFDMCameraFlex::UpdateAnimation's walk of GetGameObject(id)+0x60 (anim set
    /// list: anim node +0x30 res +0x10 DTAnimHdr) for the channel set whose id == spline id:
    /// its channels as (channel id (+0, low 24 bits used), DTFCurve). None = no set found.
    fn CameraAnimChannels(&self, _uid: u32, _spline_id: i32) -> Option<Vec<(u32, crate::engine::anim::res::DTFCurve)>> { None }
    /// WP-E9 CameraFlex: RwCameraSetFarClipPlane(GetCamera(0)+0x30, v); camera+200 = 1.
    fn CameraSetFarClipPlane(&self, _v: f32) {}
    /// WP-E9 CameraFlex: EFCamera::SetFOV(EFGraphicsMgr::GetCamera(0), fov).
    fn CameraSetFOV(&self, _fov: f32) {}
    /// The game-object uids in _gpoWorldMgr+0x4148 list order (next link obj+0x168). EFPhysicMgr::
    /// InitFrame builds the EFDynamicPhysic array in this order (it decides module update and
    /// collision order). None = walk the manager physic lists instead (lists 0 then 1).
    fn world_order(&self) -> Option<Vec<u32>> { None }
    /// WP-E9: the EFCollisionStage contact list (+4, count +8) of `owner`'s physic in
    /// PostColUpdate: per contact (material friction (contact+0 -> +0xc), contact normal +0xc).
    fn StageContacts(&self, _owner: u32) -> Vec<(f32, V3)> { Vec::new() }
    /// WP-E9: EFLogicInstance::CallCB(obj logic instance + 0x2c, cb) of `owner`.
    fn CallCB(&self, _owner: u32, _cb: u32) {}
    /// WP-E9: a bone frame of `uid`'s 3d object (Get3dObject(0), needs the HAnim flag +0x2c&4):
    /// `bone` with any of the high 16 bits set = HAnim id (RpHAnimIDGetIndex), else a node index.
    /// Returns (frame modelling matrix +0x10, RwFrameGetLTM). None = no hierarchy / bad bone.
    fn GetBone(&self, _uid: u32, _bone: u32) -> Option<(RwMatrix, RwMatrix)> { None }
    /// WP-E9: write back a bone frame modelling matrix (RwFrameRotate result). Interior mutability
    /// on the host side (the world is shared during module updates).
    fn SetBoneModelling(&self, _uid: u32, _bone: u32, _m: RwMatrix) {}
    /// WP-E9: EFWorld::GetRetriever(id & 0xff000000 == 0xff000000, ObjectRetrieverParams) then
    /// retriever vtbl+0xc (update) / +0x10 (current object): the retrieved game-object uid.
    fn RetrieverGet(&self, _id: u32, _params: &[u32]) -> Option<u32> { None }
}

/// No-op world.
pub struct NoWorld;
impl PhysicWorld for NoWorld {}

/// Per-call context handed to a module (replaces `this+4` physic access and `_gpoPhysicMgr`).
pub struct DMCtx<'a> {
    pub phys: &'a mut EFPhysic,
    /// _gpoPhysicMgr+0x2024 frame counter
    pub frame: u32,
    /// +0x2028 paused frame count
    pub paused_frames: u32,
    /// +0x202c previous frame's "world paused" flag
    pub prev_paused: u32,
    pub world: &'a dyn PhysicWorld,
    pub notify: &'a mut Vec<PhysicNotify>,
}

/// EFDynamicPhysic (0x48 bytes, one per physic per frame, built by EFPhysicMgr::InitFrame).
#[derive(Debug, Clone, Default)]
pub struct EFDynamicPhysic {
    /// +0x00
    pub physic: PhysId,
    /// +0x04 summed acceleration of the frame
    pub accel: V3,
    /// +0x10 frame position (LTM pos) at Pre/PostCollisionUpdate
    pub pos: V3,
    /// +0x1c displacement velocity added by the modules this frame (not integrated into +0x28)
    pub disp: V3,
    /// +0x28 flags (bit1 = processed this pass)
    pub flags: u32,
    /// +0x2c physic state at frame start
    pub prev_state: u32,
    /// +0x30 primary sphere center (local -> world in PreCollisionStage)
    pub sphere: V3,
    /// +0x3c primary sphere radius (+ swept distance after PreCollisionStage)
    pub radius: f32,
    /// +0x40 / +0x44 particle emitters with live particles (count)
    pub emitters: u32,
    /// +0x40 list: indices into EFPhysicMgr::emitters (BuildParticleEmittersList)
    pub emitter_list: Vec<usize>,
}

/// Seam for EFColDMData (WP-E3's collision system hands one to each ColUpdate module): the
/// contact plane and what was touched.
#[derive(Debug, Clone, Default)]
pub struct EFColDMData {
    /// +0x04 contact normal
    pub normal: V3,
    /// +0x1c the touched physic (its velocity +0x28, flags +0xc, friction +0x4c)
    pub other: Option<(V3, u32, f32)>,
    /// +0x24 env primitive material: friction at material+0xc
    pub env_friction: Option<f32>,
    /// +0x28 contact list: per contact (env friction | other physic (flags, friction))
    pub contacts: Vec<(Option<f32>, Option<(u32, f32)>)>,
}

/// Seam for EFCollisionStage (+0 EFDynamicPhysic*): ColUpdate/PostColUpdate read the dynamic data.
pub struct EFCollisionStage<'a> {
    pub dyn_: &'a EFDynamicPhysic,
    /// stage+0x4/+0x8: the env prims of this stage (EFCollisionStage::BuildEnvPrimsLists); empty
    /// when the stage is a seam (tests / no collision system)
    pub env: &'a [super::objcoll_check::EnvPrim],
    /// _gpoPhysicMgr access (EFPhysicMgr::RayIntersect, physics, map materials) for the modules;
    /// set by EFPhysicMgr::PostCollisionUpdate around each module call (WP-J4)
    pub ray: Option<&'a std::cell::RefCell<dyn super::dm::ray_intersect::RayIntersectHost + 'a>>,
}

impl<'a> EFCollisionStage<'a> {
    /// a seam stage: dynamic data only (no env prims, no manager)
    pub fn seam(dyn_: &'a EFDynamicPhysic) -> Self {
        Self { dyn_, env: &[], ray: None }
    }
}

/// The EFPhysicModule vtable.
pub trait EFDM: Send + Sync {
    fn base(&self) -> &EFPhysicModule;
    fn base_mut(&mut self) -> &mut EFPhysicModule;
    /// words of the SetParams/GetParams block (what the API copies to/from Mem)
    fn ParamWords(&self) -> usize { 0 }
    /// safe downcast to the water simulation (P32 R76: replaces an unchecked pointer cast)
    fn as_water_sim(&self) -> Option<&super::dm::water_simulation::EFDMWaterSimulation> { None }
    /// vtbl+0x0c. Returns false to delete the module (EFPhysic::PreCollisionUpdate).
    fn PreColUpdate(&mut self, _ctx: &mut DMCtx, _dp: &mut EFDynamicPhysic, _dt: f32) -> bool { true }
    /// vtbl+0x10
    fn ColUpdate(&mut self, _ctx: &mut DMCtx, _stage: &EFCollisionStage, _d: &EFColDMData, _dt: f32) {}
    /// vtbl+0x14
    fn PostColUpdate(&mut self, _ctx: &mut DMCtx, _stage: Option<&EFCollisionStage>, _dt: f32) {}
    /// vtbl+0x18; None = the NULL-param reset branch
    fn SetParams(&mut self, _ctx: &mut DMCtx, _p: Option<&[u32]>) {}
    /// vtbl+0x1c
    fn GetParams(&self, _ctx: &DMCtx, _p: &mut [u32]) {}
    /// vtbl+0x20 (raw param block, words)
    fn DoAction(&mut self, _ctx: &mut DMCtx, _action: u32, _p: &mut [u32]) {}
    /// words written by GetData
    fn DataWords(&self) -> usize { 0 }
    /// vtbl+0x24
    fn GetData(&self, _ctx: &DMCtx, _p: &mut [u32]) {}
}

/// f32 <-> param word helpers
#[inline]
pub fn wf(p: &[u32], i: usize) -> f32 { f32::from_bits(p[i]) }
#[inline]
pub fn fw(x: f32) -> u32 { x.to_bits() }
#[inline]
pub fn wv(p: &[u32], i: usize) -> V3 { [wf(p, i), wf(p, i + 1), wf(p, i + 2)] }
#[inline]
pub fn put_v(p: &mut [u32], i: usize, v: V3) {
    p[i] = fw(v[0]);
    p[i + 1] = fw(v[1]);
    p[i + 2] = fw(v[2]);
}

/// The 0x2a000000 SELF id resolution used by modules that take an object id.
pub fn ResolveSelf(ctx: &DMCtx, id: u32) -> u32 {
    if id == 0x2a00_0000 { ctx.world.CurrentLogicObject() } else { id }
}

/// Wrap an angle into [-pi, pi] with the `while` loops of EFDMTrackObjet (loop form).
pub fn WrapPiLoop(mut a: f32) -> f32 {
    while 3.1415927 < a {
        a -= 6.2831855;
    }
    while a < -3.1415927 {
        a += 6.2831855;
    }
    a
}
/// Single-step wrap (`if` form, most modules)
pub fn WrapPiOnce(mut a: f32) -> f32 {
    if 3.1415927 < a {
        a -= 6.2831855;
    }
    if a < -3.1415927 {
        a += 6.2831855;
    }
    a
}
/// heading of a matrix: atan2(at.x, at.z)
pub fn Heading(m: &RwMatrix) -> f32 { m.at[0].atan2(m.at[2]) }
pub const RAD2DEG: f32 = 57.29578;

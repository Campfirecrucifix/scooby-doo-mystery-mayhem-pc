//! WP-J1: engine runtime (part 1). Owns the managers ported so far and runs them in the order of
//! the original main loop, replacing the TestEngine/NullHost zero stubs for those systems.
//!
//! Level load (EFEngineController::LoadLevel 0x001fc450 -> EFRessourcesMgr::LoadLevel 0x0020b0e0):
//! 1. LoadLevelTextureDictionnary: TEXDIC_n.TXD -> EFGraphicsMgr::tex_dict (EFTextureDict), whose
//!    names become `PLMgr_BC::textures` (RwTexDictionaryGetCurrent replacement);
//! 2. the pack, in FAT order: each entry registers itself with its manager while it streams in
//!    (PLMgr_BC::LoadFromStream for 0x29 then 0x25 [call counter], EFStatic3dObj 0x18 static
//!    lights -> EFLightMgr_BC::CreateStaticLight, EF3dObjRes 0x5b emitter containers kept per
//!    resource, DTWaypointGrp 0x30 -> DTWpGrpMgr, EFGenericDataLoader 0x48 -> EFSplineMgr);
//! 3. the trailing object container: logic codes + EF_GLO objects (logic_link::LinkLevelLogic).
//!
//! Frame (EFEngineController::MainLoop 0x001fbef0, flag 0x29c&2 = level running):
//! ComputeDeltaTime -> input -> EFGameLogicMgr::Update (0x001fcf70) -> [cut-scene] ->
//! EFWorld::Update (0x0026eb60: animation, FlushNotification, TexFader, EFPhysicMgr::Update,
//! _gpoPLParticleMgr->Update [vtbl 0x4c], kill list, PrepareRender) -> PaletteMgr -> MotionTrail ->
//! EFGraphicsMgr::Render (0x0026ae10) -> audio (vtbl 0x40).
//! Particle frames: RwFrameGetLTM is replaced by `PLMgr_BC::SyncFrames` run right before the
//! particle update; the camera RwFrame shared with its owner is refreshed before Render.

#![allow(non_snake_case)]

use crate::engine::camera::DTMatrix;
use crate::engine::dt::stream::DTMemoryPersistStream;
use crate::engine::gfx::mgr::{EFGraphicsMgr, RenderPass, RenderSeam};
use crate::engine::gfx::{EF3dObjRes, EFStatic3dObj, EFTextureDict};
use crate::engine::iso::IsoFs;
use crate::engine::light::EFLightMgr_BC;
use crate::engine::logic_link::{LevelLogicReport, LinkLevelLogic, LogicCodeMap};
use crate::engine::pack::{EFRessourcesMgr};
use crate::engine::particle::api::EFBillboardMgr;
use crate::engine::particle::{PLDrawBatch, PLMgr_BC, PLNotification, PLWorld};
use crate::engine::spline::EFSplineMgr;
use crate::engine::waypoint::DTWpGrpMgr;
use crate::engine::{Engine, Host, Mem, Ptr};
use crate::logic::ObjH;
use bevy_app::{App, Plugin, Update};
use bevy_ecs::prelude::*;
use std::collections::HashMap;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::sync::{Arc, Mutex};

// ---------------------------------------------------------------------------------------------
// libc rand

/// The libc rand() state shared by every engine user (newlib: `s = s*6364136223846793005+1;
/// return (s>>32)&0x7fffffff`, seed 1). In the C++ particles, audio (SOUND_ComputeRandomizedValue
/// "R/min/max" values) and logic all draw from this one sequence, so the interleaving matters.
/// Particles use it by swapping it into `PLParticlePool::rng` around the particle update.
/// Audio hook: call [`LibcRand::rand`] from the audio manager instead of its own generator.
/// J2: the state is the audio manager's `SharedRand` (EFNewAudioMgr::with_rng), so audio,
/// particles (state copied into `PLParticlePool::rng` around the update) and logic share it.
#[derive(Resource, Clone, Default)]
pub struct LibcRand(pub crate::engine::audio::mgr::SharedRand);

impl LibcRand {
    pub fn rand(&self) -> u32 {
        self.0.lock().unwrap().rand() as u32
    }
    /// srand(seed)
    pub fn srand(&self, seed: u32) {
        self.0.lock().unwrap().0 = seed as u64;
    }
}

// ---------------------------------------------------------------------------------------------
// world frames (PLWorld)

/// The frames of the game objects as far as the runtime knows them (EFGameObject +0x14 RwFrame
/// LTM). Initialised from the EF_GLO descriptor POS/LOOK_* at load; updated by the physics /
/// animation side once ported. Bones are not modelled yet: every bone resolves to the object
/// frame (EF3dObj hierarchy is WP-D/C work).
#[derive(Debug, Clone, Default)]
pub struct RuntimeWorld {
    pub frames: HashMap<u32, DTMatrix>,
    /// K6: frames of latent templates whose uid a launched instance reused (Subsystems::CloneInstances)
    pub latent_frames: HashMap<u32, DTMatrix>,
    /// J2: per game object, the HAnim pose joint translations (object space; see
    /// Subsystems::UpdateBones). Bone n resolves to joint n-1 (bone 0 = the object frame).
    pub bones: HashMap<u32, Vec<[f32; 3]>>,
    /// J6: per game object, the RpHAnimHierarchy node frames (node index order) as
    /// (modelling matrix, world LTM), published by the renderer after EF3dObj::UpdateHierarchy.
    /// When present, GetObjectFrame(bone) is 1:1 EFGameObject::GetObjectFrame (0x00200360):
    /// bone = node index -> pNodeInfo[bone].pFrame LTM (the J2 `bones` fallback only adds one
    /// joint's LOCAL translation to the object frame, ignoring the parent chain).
    pub nodes: HashMap<u32, Vec<(DTMatrix, DTMatrix)>>,
    /// J14: per game object, the HAnim node ids (node index order) for RpHAnimIDGetIndex
    pub node_ids: HashMap<u32, Vec<i32>>,
    /// _gpoWorldMgr+0x14
    pub paused: bool,
    /// J5: EF3dObj_SP visibility (EF3dObj +0x2c & 2) per game object, set by
    /// APIgameobj::SetGameObjectVisible (absent = visible, EF3dObj default after CreateInstances)
    pub hidden: std::collections::HashSet<u32>,
}

impl PLWorld for RuntimeWorld {
    fn IsPaused(&self) -> bool {
        self.paused
    }
    fn GameObjectExists(&self, uid: u32) -> bool {
        self.frames.contains_key(&uid)
    }
    fn GetObjectFrame(&self, obj: u32, bone: u32) -> Option<DTMatrix> {
        let mut f = self.frames.get(&obj).copied()?;
        // EFGameObject::GetObjectFrame: HAnim ids (high bits set) go through RpHAnimIDGetIndex
        let idx = if bone != 0x7fff_ffff && bone & 0xffff_0000 != 0 {
            self.node_ids.get(&obj).and_then(|ids| ids.iter().position(|&h| h as u32 == bone))
        } else {
            None
        };
        if let Some(n) = idx.and_then(|i| self.nodes.get(&obj).and_then(|n| n.get(i))) {
            return Some(n.1);
        }
        if bone != 0x7fff_ffff && bone & 0xffff_0000 == 0 {
            if let Some(n) = self.nodes.get(&obj).and_then(|n| n.get(bone as usize)) {
                return Some(n.1);
            }
        }
        if bone != 0 {
            if let Some(t) = self.bones.get(&obj).and_then(|b| b.get(bone as usize - 1)) {
                // joint LTM = object LTM * joint offset (rotation of the joint not modelled)
                for i in 0..3 {
                    f.pos[i] += f.right[i] * t[0] + f.up[i] * t[1] + f.at[i] * t[2];
                }
            }
        }
        Some(f)
    }
    fn GetObjectPosDataFrame(&self, uid: u32) -> Option<DTMatrix> {
        self.frames.get(&uid).copied()
    }
}

/// WP-C5 world queries on the runtime frames (SELF is resolved by the host before the call).
impl crate::engine::gfx::api::GfxWorld for RuntimeWorld {
    fn ResolveGameObject(&self, uid: u32) -> Option<u32> {
        self.frames.contains_key(&uid).then_some(uid)
    }
    fn GetObjectFrame(&self, obj: u32, bone: u32) -> Option<DTMatrix> {
        PLWorld::GetObjectFrame(self, obj, bone)
    }
    fn GetObjectPosDataFrame(&self, uid: u32) -> Option<DTMatrix> {
        self.frames.get(&uid).copied()
    }
    /// deviation: the runtime has no RpHAnim node table; HAnim ids (joint & 0xffff0000) resolve to
    /// the object frame, node indices to pose joint `node` (bone node+1).
    fn GetJointLTM(&self, obj: u32, joint: u32) -> Option<DTMatrix> {
        let b = if joint & 0xffff_0000 != 0 { 0 } else { joint + 1 };
        PLWorld::GetObjectFrame(self, obj, b)
    }
}

/// The particle state shared between the Bevy resource and the logic's APIparticle_* calls
/// (which arrive through the `Host` inside `Engine`).
pub struct ParticleShared {
    /// J14: game objects parented to another object's bone frame (APIgameobj::AddChild ->
    /// EFGameObject vtbl+0x60 / EFPosData::AddChildFrame): (child, parent, bone, local offset).
    /// The child's RwFrame is a child of the bone frame (local = identity + offset, flag 1 = no
    /// offset), so its world frame follows the bone; LeaveParent keeps the world LTM.
    pub parented: Vec<(u32, u32, u32, [f32; 3])>,
    pub pl: PLMgr_BC,
    pub bb: EFBillboardMgr,
    pub world: RuntimeWorld,
    /// J2: anim / physics / audio / camera / light managers reached by the logic's APIs
    pub sub: Subsystems,
    /// J2: the logic runtime while EFGameLogicMgr::Update runs (EngineRuntime::StepLogic), so the
    /// host resolves SELF (EFWorld::GetGameObject(0x2a000000) = running object). See findings.
    pub logic: Option<*const crate::logic::LogicRuntime>,
    /// J2: EFPlayerDeviceMgr (+ EFInputMgr) behind APIinput_* / APIsimpleinput_*
    pub devices: crate::engine::input::EFPlayerDeviceMgr,
    /// J3: (object, bone) -> Mem block standing in for the bone's RwFrame (EFGameObject::
    /// GetObjectFrame): +0x10 modelling matrix (GetBoneLMat / SetBoneLMat), +0x50 LTM (GetBoneWMat).
    pub bone_frames: HashMap<(u32, u32), Ptr>,
    /// J4 (WP-C5 step 1): APIgraphics_* state (goSpriteMgr, goMotionTrailMgr, lens flare, deferred)
    pub gfxapi: crate::engine::gfx::api::EFGraphicsApi,
    /// J7: _gpoHudMgr (EFHudMgr_SP) behind APIhud_*; updated/rendered by the HudUpdate pass
    pub hud: crate::engine::hud::EFHudMgr,
    /// J7: EFHud3DWindow objects: game object uid -> EF3dObj index (obj+0x174[n])
    pub hud3d: HashMap<u32, u32>,
    /// J8: EFCutScenePlayer singleton (0x1f83e70); the binary installs it at level load
    pub cutscene: Option<crate::engine::script::EFCutScenePlayer>,
    /// J8: Play/PlugNPlay/Stop/Skip requests made by the logic during a frame; the binary applies
    /// them after the frame (deviation: the C++ calls the player synchronously; here the player
    /// needs the Engine borrowed by the running logic)
    pub cutscene_req: Vec<CutSceneReq>,
    /// J12: EFGameObject proximity clipping: uid -> (+0x140 EFProximityClippingParams {target,
    /// radius, hysteresis, clip flags}, +0x150 last ClipProximityTest result, +0xcc bits 2/8)
    pub proximity: HashMap<u32, ([u32; 4], u32, u32)>,
    /// WP-S3: EFEngineController / EFIniFile / DTWpGrpMgr / SOUND_ karaoke state behind APIgame, APIextra, APIkaraoke
    pub game: crate::engine::host::game::HostGame,
    /// WP-S1: EFGameObject/EF3dObj fields behind the host/gameobj.rs APIs
    pub s1: crate::engine::host::gameobj::S1State,
}

/// J8: deferred EFApi_Game cut-scene call
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CutSceneReq {
    /// PlayCutScene (id, sound) / PlugNPlayCutScene (id, sound, pos object)
    Play(u32, u32, u32),
    Stop,
    Skip,
}
// SAFETY: the pointer is only set for the duration of GameLogicUpdate on the same thread.
unsafe impl Send for ParticleShared {}

impl Default for ParticleShared {
    fn default() -> Self {
        ParticleShared { pl: PLMgr_BC::default(), bb: EFBillboardMgr::default(), world: RuntimeWorld::default(), sub: Subsystems::new(Default::default()), logic: None, devices: crate::engine::input::EFPlayerDeviceMgr::new(), bone_frames: HashMap::new(), gfxapi: Default::default(), hud: crate::engine::hud::EFHudMgr::new(), hud3d: HashMap::new(), cutscene: None, cutscene_req: Vec::new(), proximity: HashMap::new(), game: Default::default(), parented: Vec::new(), s1: Default::default() }
    }
}

#[path = "runtime_j2.rs"]
pub mod j2;
pub use j2::*;

pub type SharedParticles = Arc<Mutex<ParticleShared>>;

// ---------------------------------------------------------------------------------------------
// Host

/// The `Host` of the runtime: APIparticle_* go to the shared PLMgr_BC / EFBillboardMgr,
/// APIgameobj GetPosition/GetLookAt read the world frames. Everything else keeps the trait
/// defaults (other WPs' hosts are not wired yet).
macro_rules! gx {
    ($s:ident, $f:ident($($a:expr),*)) => { $s.p.lock().unwrap().gfxapi.$f($($a),*) };
}
macro_rules! gxw {
    ($s:ident, $w:ident, $f:ident($($a:expr),*)) => {{
        let mut g = $s.p.lock().unwrap();
        let ParticleShared { gfxapi, world: $w, .. } = &mut *g;
        gfxapi.$f($($a),*)
    }};
}

macro_rules! hd {
    ($s:ident, $f:ident($($a:expr),*)) => { $s.p.lock().unwrap().hud.$f($($a),*) };
}

/// WP-P2: vsprintf of the game's libc (MSL, Metrowerks) over the module's vararg words (MIPS EABI:
/// each int-class vararg is one 32-bit word): flags `-+ #0`, width / precision (incl. `*`), length
/// h/hh/l (no-ops on 32-bit), conversions d i u x X o c s p %. No module passes floats to these APIs.
pub fn sprintf(m: &Mem, f: Ptr, args: &[u32]) -> String {
    let f = m.cstr_at(f).into_bytes();
    let mut o: Vec<u8> = Vec::new();
    let mut ai = 0usize;
    let mut next = || {
        let v = args.get(ai).copied().unwrap_or(0);
        ai += 1;
        v
    };
    let mut i = 0;
    while i < f.len() {
        let c = f[i];
        i += 1;
        if c != b'%' {
            o.push(c);
            continue;
        }
        let (mut left, mut plus, mut space, mut alt, mut zero) = (false, false, false, false, false);
        while i < f.len() {
            match f[i] {
                b'-' => left = true,
                b'+' => plus = true,
                b' ' => space = true,
                b'#' => alt = true,
                b'0' => zero = true,
                _ => break,
            }
            i += 1;
        }
        let mut width = 0usize;
        if i < f.len() && f[i] == b'*' {
            let w = next() as i32;
            if w < 0 {
                left = true;
            }
            width = w.unsigned_abs() as usize;
            i += 1;
        } else {
            while i < f.len() && f[i].is_ascii_digit() {
                width = width * 10 + (f[i] - b'0') as usize;
                i += 1;
            }
        }
        let mut prec: Option<usize> = None;
        if i < f.len() && f[i] == b'.' {
            i += 1;
            if i < f.len() && f[i] == b'*' {
                let p = next() as i32;
                prec = (p >= 0).then_some(p as usize);
                i += 1;
            } else {
                let mut p = 0;
                while i < f.len() && f[i].is_ascii_digit() {
                    p = p * 10 + (f[i] - b'0') as usize;
                    i += 1;
                }
                prec = Some(p);
            }
        }
        while i < f.len() && matches!(f[i], b'h' | b'l' | b'L' | b'z' | b'j' | b't') {
            i += 1;
        }
        let Some(&conv) = f.get(i) else { break };
        i += 1;
        let (mut sign, body): (Vec<u8>, Vec<u8>) = match conv {
            b'%' => {
                o.push(b'%');
                continue;
            }
            b'c' => (vec![], vec![next() as u8]),
            b's' => {
                let p = next();
                let mut s = if p == 0 { b"(null)".to_vec() } else { m.cstr_at(p).into_bytes() };
                if let Some(p) = prec {
                    s.truncate(p);
                }
                (vec![], s)
            }
            b'd' | b'i' => {
                let v = next() as i32;
                let sg: &[u8] = if v < 0 { b"-" } else if plus { b"+" } else if space { b" " } else { b"" };
                (sg.to_vec(), num(v.unsigned_abs(), 10, false, prec))
            }
            b'u' => (vec![], num(next(), 10, false, prec)),
            b'x' | b'X' | b'p' => {
                let v = next();
                let sg: &[u8] = if (alt && v != 0) || conv == b'p' { if conv == b'X' { b"0X" } else { b"0x" } } else { b"" };
                (sg.to_vec(), num(v, 16, conv == b'X', prec))
            }
            b'o' => {
                let mut d = num(next(), 8, false, prec);
                if alt && d.first() != Some(&b'0') {
                    d.insert(0, b'0');
                }
                (vec![], d)
            }
            _ => {
                o.push(conv);
                continue;
            }
        };
        let pad = width.saturating_sub(sign.len() + body.len());
        let numeric = !matches!(conv, b's' | b'c');
        if left {
            o.append(&mut sign);
            o.extend_from_slice(&body);
            o.extend(std::iter::repeat(b' ').take(pad));
        } else if zero && numeric && prec.is_none() {
            o.append(&mut sign);
            o.extend(std::iter::repeat(b'0').take(pad));
            o.extend_from_slice(&body);
        } else {
            o.extend(std::iter::repeat(b' ').take(pad));
            o.append(&mut sign);
            o.extend_from_slice(&body);
        }
    }
    String::from_utf8_lossy(&o).into_owned()
}

/// digits of `v` in `base`, at least `prec` digits (precision 0 with value 0 prints nothing)
fn num(v: u32, base: u32, upper: bool, prec: Option<usize>) -> Vec<u8> {
    let digs: &[u8] = if upper { b"0123456789ABCDEF" } else { b"0123456789abcdef" };
    let mut d = Vec::new();
    let mut x = v;
    while x != 0 {
        d.push(digs[(x % base) as usize]);
        x /= base;
    }
    while d.len() < prec.unwrap_or(1) {
        d.push(b'0');
    }
    d.reverse();
    d
}

pub struct RuntimeHost {
    pub p: SharedParticles,
}

impl RuntimeHost {
    /// EFWorld::GetGameObject id resolution for the host APIs: SELF -> the object whose logic is
    /// running (retriever ids 0xff.. are not resolvable from the host and pass through).
    fn r(&self, id: u32) -> u32 {
        if id != crate::engine::SELF {
            return id;
        }
        let Some(l) = self.p.lock().unwrap().logic else { return id };
        // SAFETY: set by StepLogic around GameLogicUpdate; the host is called from inside it and
        // only reads the stack top and the object id.
        let l = unsafe { &*l };
        l.current_object().and_then(|h| l.objects[h].as_ref()).map_or(id, |o| o.id)
    }
    /// EFGameObject::GetObjectFrame(bone) LTM (0x7fffffff = the object's own frame) / pos-data frame
    /// J14: world frame of each parented object = local (identity + offset) * parent bone LTM
    pub fn place_parented(&self) {
        place_parented(&mut self.p.lock().unwrap());
    }
    fn obj_ltm(&self, id: u32, bone: u32) -> Option<DTMatrix> {
        let id = self.r(id);
        let g = self.p.lock().unwrap();
        if bone == 0x7fff_ffff || id >> 24 != 0x2a {
            g.world.frames.get(&id).copied()
        } else {
            PLWorld::GetObjectFrame(&g.world, id, bone)
        }
    }
    /// EFWorld::GetGameObject(id) on the logic objects (the game object fields the logic runtime
    /// models), None = NULL.
    fn group<R>(&self, id: u32, f: impl FnOnce(&mut crate::logic::GameObject) -> R) -> Option<R> {
        let id = self.r(id);
        let l = self.p.lock().unwrap().logic?;
        // SAFETY: as in `r`: set around the logic update on this thread; the object fields touched
        // here (+0x154/+0x158) are not borrowed by the calling module code.
        let l = unsafe { &mut *(l as *mut crate::logic::LogicRuntime) };
        let h = l.map.find(id)?;
        l.objects[h].as_mut().map(f)
    }
}

/// J14: world frame of each parented object = local (identity + offset) * parent bone LTM
fn place_parented(g: &mut ParticleShared) {
    for (c, p, b, off) in g.parented.clone() {
        let m = if b == 0x7fff_ffff { g.world.frames.get(&p).copied() } else { PLWorld::GetObjectFrame(&g.world, p, b) };
        let Some(m) = m else { continue };
        let pos = std::array::from_fn(|i| m.pos[i] + off[0] * m.right[i] + off[1] * m.up[i] + off[2] * m.at[i]);
        if let Some(f) = g.world.frames.get_mut(&c) {
            *f = DTMatrix { pos, ..m };
        }
    }
}

/// RwMatrix in Mem: right +0, flags +0xc, up +0x10, at +0x20, pos +0x30.
fn write_rwmatrix(mem: &mut Mem, p: Ptr, m: &DTMatrix) {
    for (o, v) in [(0, m.right), (0x10, m.up), (0x20, m.at), (0x30, m.pos)] {
        wv3(mem, p + o, v);
    }
    mem.w32(p + 0xc, 3);
}
/// The persistent RwFrame stand-in of (obj, bone); created with the joint's local matrix.
fn bone_frame(p: &SharedParticles, mem: &mut Mem, obj: u32, bone: u32) -> Ptr {
    let mut g = p.lock().unwrap();
    if let Some(&f) = g.bone_frames.get(&(obj, bone)) {
        return f;
    }
    let f = mem.malloc(0x90);
    let node = (bone & 0xffff_0000 == 0).then(|| g.world.nodes.get(&obj).and_then(|n| n.get(bone as usize)).map(|n| n.0)).flatten();
    let local = if node.is_some() {
        node
    } else if bone == 0 {
        g.world.frames.get(&obj).copied()
    } else {
        let t = g.world.bones.get(&obj).and_then(|b| b.get(bone as usize - 1)).copied().unwrap_or([0.0; 3]);
        Some(DTMatrix { right: [1., 0., 0.], up: [0., 1., 0.], at: [0., 0., 1.], pos: t })
    };
    if let Some(m) = local {
        write_rwmatrix(mem, f + 0x10, &m);
    }
    let w = g.world.GetObjectFrame(obj, bone);
    if let Some(m) = w {
        write_rwmatrix(mem, f + 0x50, &m);
    }
    g.bone_frames.insert((obj, bone), f);
    f
}
fn wv3(mem: &mut Mem, a: Ptr, v: [f32; 3]) {
    for (i, x) in v.iter().enumerate() {
        mem.w32(a + 4 * i as u32, x.to_bits());
    }
}

macro_rules! sp {
    ($s:ident, $f:ident($($a:expr),*)) => {{ $s.p.lock().unwrap().s1.splines.$f($($a),*) }};
}
macro_rules! pl {
    ($s:ident, $f:ident($($a:expr),*)) => {{
        let mut g = $s.p.lock().unwrap();
        g.pl.$f($($a),*)
    }};
}

impl Host for RuntimeHost {
    // J7: APIhud_* -> _gpoHudMgr (EFApi_HUD.cpp). Varargs: the generated ABI passes the first
    // vararg word only (a2) -> args = [a2].
    fn APIhud_WindowAdd(&mut self, _m: &mut Mem, a0: u32, a1: u32, a2: u32, a3: u32) -> u32 { hd!(self, APIhud_WindowAdd(a0, a1, a2, a3)) }
    fn APIhud_WindowDelete(&mut self, _m: &mut Mem, a0: u32) -> u32 { hd!(self, APIhud_WindowDelete(a0)) }
    fn APIhud_WindowSetFlags(&mut self, _m: &mut Mem, a0: u32, a1: u32) -> u32 { hd!(self, APIhud_WindowSetFlags(a0, a1)) }
    fn APIhud_WindowResetFlags(&mut self, _m: &mut Mem, a0: u32, a1: u32) -> u32 { hd!(self, APIhud_WindowResetFlags(a0, a1)) }
    fn APIhud_WindowGetFlags(&mut self, _m: &mut Mem, a0: u32) -> u32 { hd!(self, APIhud_WindowGetFlags(a0)) }
    fn APIhud_WindowSetTimer(&mut self, _m: &mut Mem, a0: u32, a1: f32) -> u32 { hd!(self, APIhud_WindowSetTimer(a0, a1)) }
    fn APIhud_WindowSetPos(&mut self, _m: &mut Mem, a0: u32, a1: f32, a2: f32) -> u32 { hd!(self, APIhud_WindowSetPos(a0, a1, a2)) }
    fn APIhud_WindowSetDims(&mut self, _m: &mut Mem, a0: u32, a1: f32, a2: f32) -> u32 { hd!(self, APIhud_WindowSetDims(a0, a1, a2)) }
    fn APIhud_WindowSetGrid(&mut self, _m: &mut Mem, a0: u32, a1: u32, a2: u32, a3: u32, a4: u32, a5: u32, a6: u32) -> u32 { hd!(self, APIhud_WindowSetGrid(a0, a1, a2, a3, a4, a5, a6)) }
    fn APIhud_WindowSetBackGround(&mut self, _m: &mut Mem, a0: u32, a1: u32, a2: f32, a3: f32) -> u32 { hd!(self, APIhud_WindowSetBackGround(a0, a1, a2, a3)) }
    fn APIhud_WindowSetName(&mut self, m: &mut Mem, a0: u32, a1: Ptr) -> u32 { let s = m.cstr_at(a1); hd!(self, APIhud_WindowSetName(a0, &s)) }
    fn APIhud_WindowFind(&mut self, _m: &mut Mem, a0: u32) -> u32 { hd!(self, APIhud_WindowFind(a0)) }
    fn APIhud_WindowSetID(&mut self, _m: &mut Mem, a0: u32, a1: u32) -> u32 { hd!(self, APIhud_WindowSetID(a0, a1)) }
    fn APIhud_WindowGetNextSibling(&mut self, _m: &mut Mem, a0: u32) -> u32 { hd!(self, APIhud_WindowGetNextSibling(a0)) }
    fn APIhud_WindowGetFirstChild(&mut self, _m: &mut Mem, a0: u32) -> u32 { hd!(self, APIhud_WindowGetFirstChild(a0)) }
    fn APIhud_WindowGetParent(&mut self, _m: &mut Mem, a0: u32) -> u32 { hd!(self, APIhud_WindowGetParent(a0)) }
    /// GetPos/GetDims write both floats through the pointers (missing window: untouched).
    fn APIhud_WindowGetPos(&mut self, m: &mut Mem, a0: u32, a1: Ptr, a2: Ptr) -> u32 {
        if let Some((x, y)) = hd!(self, APIhud_WindowGetPos(a0)) {
            m.wf(a1, x);
            m.wf(a2, y);
        }
        0
    }
    fn APIhud_WindowGetDims(&mut self, m: &mut Mem, a0: u32, a1: Ptr, a2: Ptr) -> u32 {
        if let Some((x, y)) = hd!(self, APIhud_WindowGetDims(a0)) {
            m.wf(a1, x);
            m.wf(a2, y);
        }
        0
    }
    fn APIhud_WindowSaveAll(&mut self, m: &mut Mem, a0: Ptr) -> u32 { let s = m.cstr_at(a0); hd!(self, APIhud_WindowSaveAll(&s)) }
    fn APIhud_Window2DSetTexture(&mut self, m: &mut Mem, a0: u32, a1: Ptr) -> u32 { let s = m.cstr_at(a1); hd!(self, APIhud_Window2DSetTexture(a0, &s)) }
    fn APIhud_Window2DSetTiling(&mut self, _m: &mut Mem, a0: u32, a1: f32, a2: f32) -> u32 { hd!(self, APIhud_Window2DSetTiling(a0, a1, a2)) }
    fn APIhud_Window2DSetColor(&mut self, _m: &mut Mem, a0: u32, a1: u32) -> u32 { hd!(self, APIhud_Window2DSetColor(a0, a1)) }
    /// EFWorld::GetGameObject(a1) (NULL -> no-op); window+0x8c = obj+0x174[a2]. Deviation: no
    /// EF3dObj pointers -> the window stores the object uid, the index goes to `hud3d`.
    fn APIgame_PlayCutScene(&mut self, _m: &mut Mem, a0: u32, a1: u32) -> u32 {
        self.p.lock().unwrap().cutscene_req.push(CutSceneReq::Play(a0, a1, 0x7fff_ffff));
        0
    }
    fn APIgame_PlugNPlayCutScene(&mut self, _m: &mut Mem, a0: u32, a1: u32, a2: u32) -> u32 {
        let a2 = if a2 == 0x7fff_ffff { a2 } else { self.r(a2) };
        self.p.lock().unwrap().cutscene_req.push(CutSceneReq::Play(a0, a1, a2));
        0
    }
    fn APIgame_StopCutScene(&mut self, _m: &mut Mem) -> u32 {
        self.p.lock().unwrap().cutscene_req.push(CutSceneReq::Stop);
        0
    }
    fn APIgame_SkipCutScene(&mut self, _m: &mut Mem) -> u32 {
        self.p.lock().unwrap().cutscene_req.push(CutSceneReq::Skip);
        0
    }
    fn APIgame_SetCutSceneVolume(&mut self, _m: &mut Mem, a0: f32) -> u32 {
        if let Some(c) = self.p.lock().unwrap().cutscene.as_mut() {
            c.APISetCutSceneVolume(a0);
        }
        0
    }
    fn APIgameobj_IsInCutScene(&mut self, _m: &mut Mem, a0: u32, a1: u32) -> u32 {
        let a0 = self.r(a0);
        self.p.lock().unwrap().cutscene.as_ref().map_or(0, |c| c.IsInCutScene(a0, a1) as u32)
    }
    fn APIhud_Window3DSetObj(&mut self, _m: &mut Mem, a0: u32, a1: u32, a2: u32) -> u32 {
        let id = self.r(a1);
        let mut g = self.p.lock().unwrap();
        if !g.world.frames.contains_key(&id) {
            return 0;
        }
        // +0x174[a2]: NULL unless descriptor a2 is a 3D object
        let res = g.sub.obj3d_list.get(&id).and_then(|l| l.iter().find(|d| d.0 == a2)).map_or(0, |d| d.1);
        g.hud.APIhud_Window3DSetObj(a0, id, res);
        g.hud3d.insert(id, a2);
        0
    }
    fn APIhud_TextWindowSetText(&mut self, _m: &mut Mem, a0: u32, a1: u32, a2: &[u32]) -> u32 { hd!(self, APIhud_TextWindowSetText(a0, a1, a2)) }
    fn APIhud_TextWindowSetMargins(&mut self, _m: &mut Mem, a0: u32, a1: f32, a2: f32, a3: f32, a4: f32) -> u32 { hd!(self, APIhud_TextWindowSetMargins(a0, a1, a2, a3, a4)) }
    fn APIhud_TextWindowSetStringToDisplay(&mut self, m: &mut Mem, a0: u32, a1: Ptr, a2: &[u32]) -> u32 { let s = sprintf(m, a1, a2); hd!(self, APIhud_TextWindowSetStringToDisplay(a0, &s)) }
    fn APIhud_TextWindowSetColor(&mut self, _m: &mut Mem, a0: u32, a1: u32) -> u32 { hd!(self, APIhud_TextWindowSetColor(a0, a1)) }
    fn APIhud_TextWindowSetFont(&mut self, _m: &mut Mem, a0: u32, a1: u32) -> u32 { hd!(self, APIhud_TextWindowSetFont(a0, a1)) }
    fn APIhud_TextSetCurrentLanguage(&mut self, _m: &mut Mem, a0: u32) -> u32 { hd!(self, APIhud_TextSetCurrentLanguage(a0)) }
    fn APIhud_TextGetCurrentLanguage(&mut self, _m: &mut Mem) -> u32 { hd!(self, APIhud_TextGetCurrentLanguage()) }
    fn APIhud_ColorCopy(&mut self, _m: &mut Mem, a0: u32, a1: u32, a2: u32, a3: u32) -> u32 { hd!(self, APIhud_ColorCopy(a0, a1, a2, a3)) }
    fn APIhud_ColorBlend(&mut self, _m: &mut Mem, a0: u32, a1: u32, a2: u32, a3: u32, a4: f32, a5: u32) -> u32 { hd!(self, APIhud_ColorBlend(a0, a1, a2, a3, a4, a5)) }
    fn APIhud_ColorFadeStart(&mut self, _m: &mut Mem, a0: u32, a1: u32, a2: u32, a3: u32, a4: f32, a5: u32) -> u32 { hd!(self, APIhud_ColorFadeStart(a0, a1, a2, a3, a4, a5)) }
    fn APIhud_ColorFadeStop(&mut self, _m: &mut Mem, a0: u32, a1: u32) -> u32 { hd!(self, APIhud_ColorFadeStop(a0, a1)) }
    fn APIhud_ColorIsFading(&mut self, _m: &mut Mem, a0: u32, a1: u32) -> u8 { hd!(self, APIhud_ColorIsFading(a0, a1)) }
    fn APIhud_TextWindowSetString(&mut self, m: &mut Mem, a0: u32, a1: Ptr, a2: u32) -> u32 { let s = m.cstr_at(a1); hd!(self, APIhud_TextWindowSetString(a0, &s, &[a2])) }

    fn object_launched(&mut self, _mem: &mut Mem, template_id: u32, new_id: u32) {
        let mut g = self.p.lock().unwrap();
        // K6: keep the template's frame when an instance id reuses its uid
        if let Some(f) = g.world.frames.get(&new_id).copied() {
            g.world.latent_frames.entry(new_id).or_insert(f);
        }
        if let Some(f) = g.world.latent_frames.get(&template_id).or(g.world.frames.get(&template_id)).copied() {
            g.world.frames.insert(new_id, f);
        }
        g.sub.CloneInstances(template_id, new_id);
    }
    /// J12: EFGameObject::SetProximityClippingParams (0x00200520): +0x140..0x14c = params,
    /// +0x150 = 0, +0xcc &= ~2, |= 10 (test on + notify once) when target and radius are non-zero.
    fn APIgameobj_SetProximityClippingParams(&mut self, mem: &mut Mem, a0: u32, a1: Ptr) -> u32 {
        let a0 = self.r(a0);
        if a1 == 0 {
            return 0;
        }
        let w = [mem.r32(a1), mem.r32(a1 + 4), mem.r32(a1 + 8), mem.r32(a1 + 0xc)];
        let mut g = self.p.lock().unwrap();
        let e = g.proximity.entry(a0).or_insert(([0; 4], 0, 0));
        e.0 = w;
        e.1 = 0;
        e.2 &= !2;
        if w[0] != 0 && w[1] != 0 {
            e.2 |= 10;
        }
        0
    }
    /// J12: EFGameObject::GetProximityClippingParams (0x002005a0)
    fn APIgameobj_GetProximityClippingParams(&mut self, mem: &mut Mem, a0: u32, a1: Ptr) -> u32 {
        let a0 = self.r(a0);
        if a1 == 0 {
            return 0;
        }
        let w = self.p.lock().unwrap().proximity.get(&a0).map_or([0; 4], |e| e.0);
        for (i, v) in w.iter().enumerate() {
            mem.w32(a1 + 4 * i as u32, *v);
        }
        0
    }
    /// SetGameObjectVisible (0x001edde0): EFGameObject::SetGameObjectVisible -> EF3dObj_SP::
    /// SetVisible(p). J5: the flag is kept in RuntimeWorld::hidden for the renderer.
    fn APIgameobj_SetGameObjectVisible(&mut self, _mem: &mut Mem, a0: u32, a1: i32) -> u32 {
        let a0 = self.r(a0);
        let mut g = self.p.lock().unwrap();
        if a1 != 0 {
            g.world.hidden.remove(&a0);
        } else {
            g.world.hidden.insert(a0);
        }
        0
    }
    /// IsGameObjectVisible (EFGameObject 0x00200240): EF3dObj +0x2c & 2 (-1 without 3D object).
    fn APIgameobj_IsGameObjectVisible(&mut self, _mem: &mut Mem, a0: u32) -> u32 {
        let a0 = self.r(a0);
        let g = self.p.lock().unwrap();
        if !g.sub.obj3d.contains_key(&a0) {
            return 0xffff_ffff;
        }
        (!g.world.hidden.contains(&a0)) as u32
    }
    fn APIgameobj_GetPosition(&mut self, mem: &mut Mem, a0: u32, a1: u32, a2: Ptr) -> Ptr {
        // J4: SELF resolved, param_2 = bone (GetObjectFrame), as in EFApi_GameObj
        let f = self.obj_ltm(a0, a1);
        match f {
            Some(m) => {
                wv3(mem, a2, m.pos);
                a2
            }
            None => 0,
        }
    }
    /// GetBoneLMat (0x001f00a8): EFGameObject::GetObjectFrame(bone) + 0x10, the RwFrame
    /// modelling matrix. Replaced: the frame is a persistent Mem block (see `bone_frames`)
    /// initialised to the joint's local offset; writes are not fed back into the HAnim pose.
    fn APIgameobj_GetBoneLMat(&mut self, mem: &mut Mem, a0: u32, a1: u32) -> i32 {
        let a0 = self.r(a0);
        let f = bone_frame(&self.p, mem, a0, a1) + 0x10;
        // bone 0x7fffffff = the object frame itself (EFGameObject::GetObjectFrame): its modelling
        // matrix is the runtime world frame
        if a1 == 0x7fff_ffff {
            if let Some(m) = self.p.lock().unwrap().world.frames.get(&a0).copied() {
                write_rwmatrix(mem, f, &m);
            }
        }
        f as i32
    }
    /// SetBoneLMat (0x001f00d0): copies the matrix into the frame's modelling matrix.
    fn APIgameobj_SetBoneLMat(&mut self, mem: &mut Mem, a0: u32, a1: u32, a2: Ptr) -> u32 {
        let a0 = self.r(a0);
        let f = bone_frame(&self.p, mem, a0, a1) + 0x10;
        if f != a2 {
            for i in 0..16 {
                let v = mem.r32(a2 + 4 * i);
                mem.w32(f + 4 * i, v);
            }
        }
        if a1 == 0x7fff_ffff {
            // object frame: the modelling matrix is the runtime world frame (the camera logic
            // orients the camera with SetBoneLMat(cam, 0x7fffffff, m))
            let m = DTMatrix { right: rv3(mem, a2), up: rv3(mem, a2 + 0x10), at: rv3(mem, a2 + 0x20), pos: rv3(mem, a2 + 0x30) };
            if let Some(w) = self.p.lock().unwrap().world.frames.get_mut(&a0) {
                *w = m;
            }
        }
        0
    }
    /// GetBoneWMat (0x001f01b0): RwFrameGetLTM(GetObjectFrame(bone)), refreshed from the
    /// runtime world (object LTM + joint offset) on every call.
    fn APIgameobj_GetBoneWMat(&mut self, mem: &mut Mem, a0: u32, a1: u32) -> u32 {
        let a0 = self.r(a0);
        let f = bone_frame(&self.p, mem, a0, a1);
        let m = self.p.lock().unwrap().world.GetObjectFrame(a0, a1);
        if let Some(m) = m {
            write_rwmatrix(mem, f + 0x50, &m);
        }
        f + 0x50
    }
    fn APIgameobj_GetLookAt(&mut self, mem: &mut Mem, a0: u32, a1: u32, a2: Ptr) -> Ptr {
        // J4: SELF resolved, param_2 = bone (GetObjectFrame), as in EFApi_GameObj
        let f = self.obj_ltm(a0, a1);
        match f {
            Some(m) => {
                wv3(mem, a2, m.at);
                a2
            }
            None => 0,
        }
    }
    fn APIparticle_TypeConvertNameToID(&mut self, mem: &mut Mem, a0: Ptr) -> u32 { pl!(self, APIparticle_TypeConvertNameToID(mem, a0)) }
    fn APIparticle_TypeCreate(&mut self, mem: &mut Mem) -> u32 { pl!(self, APIparticle_TypeCreate(mem)) }
    fn APIparticle_TypeDestroy(&mut self, mem: &mut Mem, a0: i32) -> u32 { pl!(self, APIparticle_TypeDestroy(mem, a0)) }
    fn APIparticle_TypeSetInter(&mut self, mem: &mut Mem, a0: i32, a1: u32, a2: f32, a3: f32, a4: f32, a5: f32) -> u32 { pl!(self, APIparticle_TypeSetInter(mem, a0, a1, a2, a3, a4, a5)) }
    fn APIparticle_TypeSetInt(&mut self, mem: &mut Mem, a0: i32, a1: u32, a2: i32) -> u32 { pl!(self, APIparticle_TypeSetInt(mem, a0, a1, a2)) }
    fn APIparticle_TypeGetInt(&mut self, mem: &mut Mem, a0: i32, a1: u32, a2: Ptr) -> u32 { pl!(self, APIparticle_TypeGetInt(mem, a0, a1, a2)) }
    fn APIparticle_TypeSetFloat(&mut self, mem: &mut Mem, a0: i32, a1: u32, a2: f32) -> u32 { pl!(self, APIparticle_TypeSetFloat(mem, a0, a1, a2)) }
    fn APIparticle_TypeSetVec(&mut self, mem: &mut Mem, a0: i32, a1: u32, a2: f32, a3: f32, a4: f32) -> u32 { pl!(self, APIparticle_TypeSetVec(mem, a0, a1, a2, a3, a4)) }
    fn APIparticle_TypeSetTexture(&mut self, mem: &mut Mem, a0: i32, a1: Ptr) -> u32 { pl!(self, APIparticle_TypeSetTexture(mem, a0, a1)) }
    fn APIparticle_TypeSetName(&mut self, mem: &mut Mem, a0: i32, a1: Ptr) -> u8 { pl!(self, APIparticle_TypeSetName(mem, a0, a1)) }
    fn APIparticle_TypeGetName(&mut self, mem: &mut Mem, a0: i32, a1: Ptr) -> u8 { pl!(self, APIparticle_TypeGetName(mem, a0, a1)) }
    fn APIparticle_EmitterConvertNameToID(&mut self, mem: &mut Mem, a0: Ptr, a1: u32) -> u32 { pl!(self, APIparticle_EmitterConvertNameToID(mem, a0, a1)) }
    fn APIparticle_EmitterCreate(&mut self, mem: &mut Mem, a0: i32) -> u32 {
        let mut g = self.p.lock().unwrap();
        let ParticleShared { pl, world, .. } = &mut *g;
        pl.APIparticle_EmitterCreate(mem, a0, world)
    }
    fn APIparticle_EmitterDestroy(&mut self, mem: &mut Mem, a0: i32) -> u32 { pl!(self, APIparticle_EmitterDestroy(mem, a0)) }
    fn APIparticle_EmitterSetType(&mut self, mem: &mut Mem, a0: i32, a1: i32) -> u32 { pl!(self, APIparticle_EmitterSetType(mem, a0, a1)) }
    fn APIparticle_EmitterStart(&mut self, mem: &mut Mem, a0: i32, a1: u32) -> u32 { pl!(self, APIparticle_EmitterStart(mem, a0, a1)) }
    fn APIparticle_EmitterStartAt(&mut self, mem: &mut Mem, a0: i32, a1: u32, a2: f32) -> u32 { pl!(self, APIparticle_EmitterStartAt(mem, a0, a1, a2)) }
    fn APIparticle_EmitterStop(&mut self, mem: &mut Mem, a0: i32) -> u32 { pl!(self, APIparticle_EmitterStop(mem, a0)) }
    fn APIparticle_EmitterReset(&mut self, mem: &mut Mem, a0: i32) -> u32 { pl!(self, APIparticle_EmitterReset(mem, a0)) }
    fn APIparticle_EmitterSetCollisionPlane(&mut self, mem: &mut Mem, a0: i32, a1: u32, a2: Ptr, a3: Ptr, a4: f32) -> u32 { pl!(self, APIparticle_EmitterSetCollisionPlane(mem, a0, a1, a2, a3, a4)) }
    fn APIparticle_EmitterSetJoint(&mut self, mem: &mut Mem, a0: i32, a1: u32, a2: u32) -> u32 {
        let mut g = self.p.lock().unwrap();
        let ParticleShared { pl, world, .. } = &mut *g;
        pl.APIparticle_EmitterSetJoint(mem, a0, a1, a2, world)
    }
    fn APIparticle_EmitterSetFlags(&mut self, mem: &mut Mem, a0: i32, a1: u32) -> u32 { pl!(self, APIparticle_EmitterSetFlags(mem, a0, a1)) }
    fn APIparticle_EmitterGetFlags(&mut self, mem: &mut Mem, a0: i32, a1: Ptr) -> u32 { pl!(self, APIparticle_EmitterGetFlags(mem, a0, a1)) }
    fn APIparticle_EmitterSetBox(&mut self, mem: &mut Mem, a0: i32, a1: Ptr, a2: Ptr) -> u32 { pl!(self, APIparticle_EmitterSetBox(mem, a0, a1, a2)) }
    fn APIparticle_EmitterGetBBox(&mut self, mem: &mut Mem, a0: i32, a1: Ptr, a2: Ptr, a3: u32) -> u32 { pl!(self, APIparticle_EmitterGetBBox(mem, a0, a1, a2, a3)) }
    fn APIparticle_EmitterSetBSphereRadius(&mut self, mem: &mut Mem, a0: i32, a1: f32) -> u32 { pl!(self, APIparticle_EmitterSetBSphereRadius(mem, a0, a1)) }
    fn APIparticle_EmitterSetMessageHandlerID(&mut self, mem: &mut Mem, a0: u32, a1: u32) -> u32 {
        let mut g = self.p.lock().unwrap();
        let ParticleShared { pl, world, .. } = &mut *g;
        pl.APIparticle_EmitterSetMessageHandlerID(mem, a0, a1, world)
    }
    fn APIparticle_CreateBillboardGroup(&mut self, mem: &mut Mem, a0: u32, a1: Ptr, a2: u32, a3: Ptr, a4: Ptr, a5: Ptr, a6: Ptr) -> u32 {
        let mut g = self.p.lock().unwrap();
        let ParticleShared { pl, bb, .. } = &mut *g;
        pl.APIparticle_CreateBillboardGroup(mem, bb, a0, a1, a2, a3, a4, a5, a6)
    }
    fn APIparticle_RenderBillboardGroup(&mut self, mem: &mut Mem, a0: u32, a1: u32) -> u32 {
        let mut g = self.p.lock().unwrap();
        let ParticleShared { pl, bb, .. } = &mut *g;
        pl.APIparticle_RenderBillboardGroup(mem, bb, a0, a1)
    }
    fn APIparticle_DestroyBillboardGroup(&mut self, mem: &mut Mem, a0: u32) -> u32 {
        let mut g = self.p.lock().unwrap();
        let ParticleShared { pl, bb, .. } = &mut *g;
        pl.APIparticle_DestroyBillboardGroup(mem, bb, a0)
    }
    fn APIparticle_SetMaxNumBillboards(&mut self, mem: &mut Mem, a0: u32) -> u32 { pl!(self, APIparticle_SetMaxNumBillboards(mem, a0)) }

    // ---- J2: EFApi_Input.cpp / EFApi_SimpleInput.cpp (EFPlayerDeviceMgr, WP-H)
    fn APIinput_InputConfigAdd(&mut self, mem: &mut Mem, a0: i32) -> u32 { self.p.lock().unwrap().devices.APIinput_InputConfigAdd(mem, a0) }
    fn APIinput_InputConfigRemove(&mut self, mem: &mut Mem, a0: i32) -> u32 { self.p.lock().unwrap().devices.APIinput_InputConfigRemove(mem, a0) }
    fn APIinput_InputConfigRemoveAll(&mut self, mem: &mut Mem) -> u32 { self.p.lock().unwrap().devices.APIinput_InputConfigRemoveAll(mem) }
    fn APIinput_InputConfigSetState(&mut self, mem: &mut Mem, a0: i32, a1: i32) -> u32 { self.p.lock().unwrap().devices.APIinput_InputConfigSetState(mem, a0, a1) }
    fn APIinput_InputConfigGetState(&mut self, mem: &mut Mem, a0: i32) -> u32 { self.p.lock().unwrap().devices.APIinput_InputConfigGetState(mem, a0) }
    fn APIinput_InputConfigActivate(&mut self, mem: &mut Mem, a0: i32, a1: i32) -> u32 { self.p.lock().unwrap().devices.APIinput_InputConfigActivate(mem, a0, a1) }
    fn APIinput_InputActionAdd(&mut self, mem: &mut Mem, a0: i32, a1: i32, a2: u32) -> u32 {
        // the EFInputAction exists from here on: EFDMAnalogMotion::SetParams' InputActionGet (physics
        // world mirror) must see it before the first StepInput (WP-J4)
        let mut g = self.p.lock().unwrap();
        let r = g.devices.APIinput_InputActionAdd(mem, a0, a1, a2);
        g.sub.input.actions.entry((a0 as u32, a1 as u32)).or_insert([0.0; 2]);
        r
    }
    fn APIinput_InputActionRemove(&mut self, mem: &mut Mem, a0: i32, a1: i32) -> u32 { self.p.lock().unwrap().devices.APIinput_InputActionRemove(mem, a0, a1) }
    fn APIinput_InputActionRemoveAll(&mut self, mem: &mut Mem, a0: i32) -> u32 { self.p.lock().unwrap().devices.APIinput_InputActionRemoveAll(mem, a0) }
    fn APIinput_InputActionGetAnalogX(&mut self, mem: &mut Mem, a0: i32, a1: i32) -> f32 { self.p.lock().unwrap().devices.APIinput_InputActionGetAnalogX(mem, a0, a1) }
    fn APIinput_InputActionGetAnalogY(&mut self, mem: &mut Mem, a0: i32, a1: i32) -> f32 { self.p.lock().unwrap().devices.APIinput_InputActionGetAnalogY(mem, a0, a1) }
    fn APIinput_InputActionGetDigitalInfo(&mut self, mem: &mut Mem, a0: i32, a1: i32) -> u8 { self.p.lock().unwrap().devices.APIinput_InputActionGetDigitalInfo(mem, a0, a1) }
    fn APIinput_InputActionIsPush(&mut self, mem: &mut Mem, a0: i32, a1: i32) -> u8 { self.p.lock().unwrap().devices.APIinput_InputActionIsPush(mem, a0, a1) }
    fn APIinput_InputActionIsRelease(&mut self, mem: &mut Mem, a0: i32, a1: i32) -> u8 { self.p.lock().unwrap().devices.APIinput_InputActionIsRelease(mem, a0, a1) }
    fn APIinput_InputActionTogglePush(&mut self, mem: &mut Mem, a0: i32, a1: i32) -> u8 { self.p.lock().unwrap().devices.APIinput_InputActionTogglePush(mem, a0, a1) }
    fn APIinput_InputActionToggleRelease(&mut self, mem: &mut Mem, a0: i32, a1: i32) -> u8 { self.p.lock().unwrap().devices.APIinput_InputActionToggleRelease(mem, a0, a1) }
    fn APIinput_InputContextAdd(&mut self, mem: &mut Mem, a0: u32, a1: u32, a2: i32, a3: i32, a4: i32, a5: i32) -> u32 { let a0 = self.r(a0); self.p.lock().unwrap().devices.APIinput_InputContextAdd(mem, a0, a1, a2, a3, a4, a5) }
    fn APIinput_InputContextRemove(&mut self, mem: &mut Mem, a0: i32, a1: i32, a2: i32) -> u32 { self.p.lock().unwrap().devices.APIinput_InputContextRemove(mem, a0, a1, a2) }
    fn APIinput_InputContextRemoveAll(&mut self, mem: &mut Mem, a0: i32, a1: i32) -> u32 { self.p.lock().unwrap().devices.APIinput_InputContextRemoveAll(mem, a0, a1) }
    fn APIinput_InputContextSetState(&mut self, mem: &mut Mem, a0: i32, a1: i32, a2: i32, a3: u32) -> u32 { self.p.lock().unwrap().devices.APIinput_InputContextSetState(mem, a0, a1, a2, a3) }
    fn APIinput_InputContextGetState(&mut self, mem: &mut Mem, a0: i32, a1: i32, a2: i32) -> u32 { self.p.lock().unwrap().devices.APIinput_InputContextGetState(mem, a0, a1, a2) }
    fn APIinput_InputContextSetFlags(&mut self, mem: &mut Mem, a0: i32, a1: i32, a2: i32, a3: u32) -> u32 { self.p.lock().unwrap().devices.APIinput_InputContextSetFlags(mem, a0, a1, a2, a3) }
    fn APIinput_InputContextGetFlags(&mut self, mem: &mut Mem, a0: i32, a1: i32, a2: i32) -> u32 { self.p.lock().unwrap().devices.APIinput_InputContextGetFlags(mem, a0, a1, a2) }
    fn APIinput_InputContextSetCallback(&mut self, mem: &mut Mem, a0: u32, a1: u32, a2: i32, a3: i32, a4: i32) -> u32 { self.p.lock().unwrap().devices.APIinput_InputContextSetCallback(mem, a0, a1, a2, a3, a4) }
    fn APIinput_InputDeviceAdd(&mut self, mem: &mut Mem, a0: i32, a1: i32, a2: i32, a3: i32, a4: i32, a5: i32, a6: i32, a7: i32) -> u32 { self.p.lock().unwrap().devices.APIinput_InputDeviceAdd(mem, a0, a1, a2, a3, a4, a5, a6, a7) }
    fn APIinput_InputDeviceRemove(&mut self, mem: &mut Mem, a0: i32, a1: i32, a2: i32) -> u32 { self.p.lock().unwrap().devices.APIinput_InputDeviceRemove(mem, a0, a1, a2) }
    fn APIinput_InputDeviceRemoveAll(&mut self, mem: &mut Mem, a0: i32, a1: i32) -> u32 { self.p.lock().unwrap().devices.APIinput_InputDeviceRemoveAll(mem, a0, a1) }
    fn APIinput_InputEnableVibration(&mut self, mem: &mut Mem, a0: i32, a1: i32) -> u32 { self.p.lock().unwrap().devices.APIinput_InputEnableVibration(mem, a0, a1) }
    fn APIinput_InputSetVibration(&mut self, mem: &mut Mem, a0: i32, a1: u32, a2: f32, a3: f32, a4: f32, a5: f32) -> u32 { self.p.lock().unwrap().devices.APIinput_InputSetVibration(mem, a0, a1, a2, a3, a4, a5) }
    fn APIinput_ConfigIsActive(&mut self, mem: &mut Mem, a0: i32) -> u8 { self.p.lock().unwrap().devices.APIinput_ConfigIsActive(mem, a0) }
    fn APIinput_InputErrorRegisterNotify(&mut self, mem: &mut Mem, a0: i32, a1: u32, a2: u32) -> u32 { let a1 = self.r(a1); self.p.lock().unwrap().devices.APIinput_InputErrorRegisterNotify(mem, a0, a1, a2) }
    fn APIinput_InputSetActivePort(&mut self, mem: &mut Mem, a0: i32) -> u32 { self.p.lock().unwrap().devices.APIinput_InputSetActivePort(mem, a0) }
    fn APIsimpleinput_SimpleInput_ConfigCreate(&mut self, mem: &mut Mem) -> i32 { self.p.lock().unwrap().devices.APIsimpleinput_SimpleInput_ConfigCreate(mem) }
    fn APIsimpleinput_SimpleInput_ConfigEnable(&mut self, mem: &mut Mem, a0: u32) -> u32 { self.p.lock().unwrap().devices.APIsimpleinput_SimpleInput_ConfigEnable(mem, a0) }
    fn APIsimpleinput_SimpleInput_ConfigDisable(&mut self, mem: &mut Mem, a0: u32) -> u32 { self.p.lock().unwrap().devices.APIsimpleinput_SimpleInput_ConfigDisable(mem, a0) }
    fn APIsimpleinput_SimpleInput_ConfigIsEnabled(&mut self, mem: &mut Mem, a0: u32) -> u8 { self.p.lock().unwrap().devices.APIsimpleinput_SimpleInput_ConfigIsEnabled(mem, a0) }
    fn APIsimpleinput_SimpleInput_ConfigSetState(&mut self, mem: &mut Mem, a0: u32, a1: u32) -> u32 { self.p.lock().unwrap().devices.APIsimpleinput_SimpleInput_ConfigSetState(mem, a0, a1) }
    fn APIsimpleinput_SimpleInput_ConfigGetState(&mut self, mem: &mut Mem, a0: u32) -> u32 { self.p.lock().unwrap().devices.APIsimpleinput_SimpleInput_ConfigGetState(mem, a0) }
    fn APIsimpleinput_SimpleInput_AddLeftJoystickCallback(&mut self, mem: &mut Mem, a0: u32) -> u32 { self.p.lock().unwrap().devices.APIsimpleinput_SimpleInput_AddLeftJoystickCallback(mem, a0) }
    fn APIsimpleinput_SimpleInput_AddLeftJoystickIdleCallback(&mut self, mem: &mut Mem, a0: u32) -> u32 { self.p.lock().unwrap().devices.APIsimpleinput_SimpleInput_AddLeftJoystickIdleCallback(mem, a0) }
    fn APIsimpleinput_SimpleInput_AddRightJoystickCallback(&mut self, mem: &mut Mem, a0: u32) -> u32 { self.p.lock().unwrap().devices.APIsimpleinput_SimpleInput_AddRightJoystickCallback(mem, a0) }
    fn APIsimpleinput_SimpleInput_AddRightJoystickIdleCallback(&mut self, mem: &mut Mem, a0: u32) -> u32 { self.p.lock().unwrap().devices.APIsimpleinput_SimpleInput_AddRightJoystickIdleCallback(mem, a0) }
    fn APIsimpleinput_SimpleInput_AddButtonDownCallback(&mut self, mem: &mut Mem, a0: u32, a1: i32) -> u32 { self.p.lock().unwrap().devices.APIsimpleinput_SimpleInput_AddButtonDownCallback(mem, a0, a1) }
    fn APIsimpleinput_SimpleInput_AddButtonUpCallback(&mut self, mem: &mut Mem, a0: u32, a1: i32) -> u32 { self.p.lock().unwrap().devices.APIsimpleinput_SimpleInput_AddButtonUpCallback(mem, a0, a1) }
    fn APIsimpleinput_SimpleInput_AddButtonPushedCallback(&mut self, mem: &mut Mem, a0: u32, a1: i32) -> u32 { self.p.lock().unwrap().devices.APIsimpleinput_SimpleInput_AddButtonPushedCallback(mem, a0, a1) }
    fn APIsimpleinput_SimpleInput_AddButtonReleasedCallback(&mut self, mem: &mut Mem, a0: u32, a1: i32) -> u32 { self.p.lock().unwrap().devices.APIsimpleinput_SimpleInput_AddButtonReleasedCallback(mem, a0, a1) }
    fn APIsimpleinput_SimpleInput_AddKeyboardDownCallback(&mut self, mem: &mut Mem, a0: u32, a1: i32) -> u32 { self.p.lock().unwrap().devices.APIsimpleinput_SimpleInput_AddKeyboardDownCallback(mem, a0, a1) }
    fn APIsimpleinput_SimpleInput_AddKeyboardUpCallback(&mut self, mem: &mut Mem, a0: u32, a1: i32) -> u32 { self.p.lock().unwrap().devices.APIsimpleinput_SimpleInput_AddKeyboardUpCallback(mem, a0, a1) }
    fn APIsimpleinput_SimpleInput_AddKeyboardPushedCallback(&mut self, mem: &mut Mem, a0: u32, a1: i32) -> u32 { self.p.lock().unwrap().devices.APIsimpleinput_SimpleInput_AddKeyboardPushedCallback(mem, a0, a1) }
    fn APIsimpleinput_SimpleInput_AddKeyboardReleasedCallback(&mut self, mem: &mut Mem, a0: u32, a1: i32) -> u32 { self.p.lock().unwrap().devices.APIsimpleinput_SimpleInput_AddKeyboardReleasedCallback(mem, a0, a1) }
    fn APIsimpleinput_SimpleInput_ActionSetState(&mut self, mem: &mut Mem, a0: u32, a1: u32, a2: u32) -> u32 { self.p.lock().unwrap().devices.APIsimpleinput_SimpleInput_ActionSetState(mem, a0, a1, a2) }
    fn APIsimpleinput_SimpleInput_ActionGetState(&mut self, mem: &mut Mem, a0: u32, a1: u32) -> u32 { self.p.lock().unwrap().devices.APIsimpleinput_SimpleInput_ActionGetState(mem, a0, a1) }
    fn APIsimpleinput_SimpleInput_AddLeftJoystickCallback_Port(&mut self, mem: &mut Mem, a0: u32, a1: u32) -> u32 { self.p.lock().unwrap().devices.APIsimpleinput_SimpleInput_AddLeftJoystickCallback_Port(mem, a0, a1) }
    fn APIsimpleinput_SimpleInput_AddLeftJoystickIdleCallback_Port(&mut self, mem: &mut Mem, a0: u32, a1: u32) -> u32 { self.p.lock().unwrap().devices.APIsimpleinput_SimpleInput_AddLeftJoystickIdleCallback_Port(mem, a0, a1) }
    fn APIsimpleinput_SimpleInput_AddRightJoystickCallback_Port(&mut self, mem: &mut Mem, a0: u32, a1: u32) -> u32 { self.p.lock().unwrap().devices.APIsimpleinput_SimpleInput_AddRightJoystickCallback_Port(mem, a0, a1) }
    fn APIsimpleinput_SimpleInput_AddRightJoystickIdleCallback_Port(&mut self, mem: &mut Mem, a0: u32, a1: u32) -> u32 { self.p.lock().unwrap().devices.APIsimpleinput_SimpleInput_AddRightJoystickIdleCallback_Port(mem, a0, a1) }
    fn APIsimpleinput_SimpleInput_AddButtonDownCallback_Port(&mut self, mem: &mut Mem, a0: u32, a1: i32, a2: u32) -> u32 { self.p.lock().unwrap().devices.APIsimpleinput_SimpleInput_AddButtonDownCallback_Port(mem, a0, a1, a2) }
    fn APIsimpleinput_SimpleInput_AddButtonUpCallback_Port(&mut self, mem: &mut Mem, a0: u32, a1: i32, a2: u32) -> u32 { self.p.lock().unwrap().devices.APIsimpleinput_SimpleInput_AddButtonUpCallback_Port(mem, a0, a1, a2) }
    fn APIsimpleinput_SimpleInput_AddButtonPushedCallback_Port(&mut self, mem: &mut Mem, a0: u32, a1: i32, a2: u32) -> u32 { self.p.lock().unwrap().devices.APIsimpleinput_SimpleInput_AddButtonPushedCallback_Port(mem, a0, a1, a2) }
    fn APIsimpleinput_SimpleInput_AddButtonReleasedCallback_Port(&mut self, mem: &mut Mem, a0: u32, a1: i32, a2: u32) -> u32 { self.p.lock().unwrap().devices.APIsimpleinput_SimpleInput_AddButtonReleasedCallback_Port(mem, a0, a1, a2) }

    // ---- J2: EFApi_Anim.cpp
    fn APIanim_Play(&mut self, _m: &mut Mem, a0: u32, a1: u32, a2: u32, a3: f32) -> u32 { let a0 = self.r(a0); J2(&self.p).anim_play(a0, a1, a2, a3) }
    fn APIanim_PlayBlend(&mut self, m: &mut Mem, a0: u32, a1: u32, a2: u32, a3: f32, a4: Ptr) -> u32 { let a0 = self.r(a0); J2(&self.p).anim_play_blend(m, a0, a1, a2, a3, a4) }
    fn APIanim_ChangeBlendParams(&mut self, m: &mut Mem, a0: u32, a1: Ptr) -> u32 { let a0 = self.r(a0); J2(&self.p).anim_change_blend(m, a0, a1) }
    fn APIanim_StopAnim(&mut self, _m: &mut Mem, a0: u32, a1: u32) -> u32 { let a0 = self.r(a0); J2(&self.p).anim_stop(a0, a1) }
    fn APIanim_StopAllAnim(&mut self, _m: &mut Mem, a0: u32) -> u32 { let a0 = self.r(a0); J2(&self.p).anim_stop_all(a0) }
    fn APIanim_GetTime(&mut self, _m: &mut Mem, a0: u32, a1: u32) -> f32 { let a0 = self.r(a0); J2(&self.p).anim_get_time(a0, a1) }
    fn APIanim_SetTime(&mut self, _m: &mut Mem, a0: u32, a1: u32, a2: f32) -> u32 { let a0 = self.r(a0); J2(&self.p).anim_set_time(a0, a1, a2) }
    fn APIanim_GetSpeed(&mut self, _m: &mut Mem, a0: u32, a1: u32) -> u32 { let a0 = self.r(a0); J2(&self.p).anim_get_speed(a0, a1) }
    fn APIanim_SetSpeed(&mut self, _m: &mut Mem, a0: u32, a1: u32, a2: f32) -> u32 { let a0 = self.r(a0); J2(&self.p).anim_set_speed(a0, a1, a2) }
    fn APIanim_GetDuration(&mut self, _m: &mut Mem, a0: u32, a1: u32) -> f32 { let a0 = self.r(a0); J2(&self.p).anim_get_duration(a0, a1) }
    fn APIanim_Connect(&mut self, _m: &mut Mem, a0: u32, a1: u32, a2: u32) -> u32 { let a0 = self.r(a0); J2(&self.p).anim_connect(a0, a1, a2) }
    fn APIanim_GetGroup(&mut self, _m: &mut Mem, a0: u32, a1: u32) -> u32 { let a0 = self.r(a0); J2(&self.p).anim_get_group(a0, a1) }
    fn APIanim_BeginTree(&mut self, _m: &mut Mem, a0: u32) -> u32 { let a0 = self.r(a0); J2(&self.p).anim_begin_tree(a0) }
    fn APIanim_PlayBlendConnect(&mut self, m: &mut Mem, a0: u32, a1: u32, a2: u32, a3: u32, a4: Ptr) -> u32 { let a0 = self.r(a0); J2(&self.p).anim_play_blend_connect(m, a0, a1, a2, a3, a4) }
    fn APIanim_EndTree(&mut self, _m: &mut Mem, a0: u32) -> u32 { let a0 = self.r(a0); J2(&self.p).anim_end_tree(a0) }
    fn APIanim_IsAnimPlaying(&mut self, _m: &mut Mem, a0: u32, a1: u32) -> u8 { let a0 = self.r(a0); J2(&self.p).anim_is_playing(a0, a1) }

    // ---- J2: EFApi_Physic.cpp
    // EFGameObject::SetContextFlag/GetContextFlag: the object context word the DM context
    // masks are tested against (EFPhysic +0x3c)
    fn APIgameobj_SetContextFlag(&mut self, _m: &mut Mem, a0: u32, a1: u32) -> u32 {
        let a0 = self.r(a0);
        J2(&self.p).physic(|m, _| {
            let id = m.PhysicOf(a0);
            if let Some(p) = m.PhysicMut(id) {
                p.context = a1;
            }
        });
        0
    }
    fn APIgameobj_GetContextFlag(&mut self, _m: &mut Mem, a0: u32) -> u32 {
        let a0 = self.r(a0);
        J2(&self.p).physic(|m, _| { let id = m.PhysicOf(a0); m.PhysicMut(id).map_or(0, |p| p.context) })
    }
    fn object_has_physic(&mut self, id: u32) -> bool { let id = self.r(id); J2(&self.p).physic_has(id) }
    fn destroy_physic(&mut self, _m: &mut Mem, id: u32) { let id = self.r(id); J2(&self.p).physic_destroy(id) }
    fn APIphysic_CreatePhysic(&mut self, _m: &mut Mem, a0: u32, a1: u32) -> u32 { let a0 = self.r(a0); J2(&self.p).physic_create(a0, a1) }
    fn APIphysic_SetFlags(&mut self, _m: &mut Mem, a0: u32, a1: u32) -> u32 { let a0 = self.r(a0); J2(&self.p).physic(|m, _| crate::engine::physic::api::SetFlags(m, a0, a1)); 0 }
    fn APIphysic_ResetFlags(&mut self, _m: &mut Mem, a0: u32, a1: u32) -> u32 { let a0 = self.r(a0); J2(&self.p).physic(|m, _| crate::engine::physic::api::ResetFlags(m, a0, a1)); 0 }
    fn APIphysic_GetFlags(&mut self, _m: &mut Mem, a0: u32) -> u32 { let a0 = self.r(a0); J2(&self.p).physic(|m, _| crate::engine::physic::api::GetFlags(m, a0)) }
    fn APIphysic_GetPhysicState(&mut self, _m: &mut Mem, a0: u32) -> u32 { let a0 = self.r(a0); J2(&self.p).physic(|m, _| crate::engine::physic::api::GetPhysicState(m, a0)) }
    fn APIphysic_SetFallingSpeed(&mut self, _m: &mut Mem, a0: u32, a1: f32) -> u32 { let a0 = self.r(a0); J2(&self.p).physic(|m, _| crate::engine::physic::api::SetFallingSpeed(m, a0, a1)); 0 }
    fn APIphysic_SetFallingDelay(&mut self, _m: &mut Mem, a0: u32, a1: f32) -> u32 { let a0 = self.r(a0); J2(&self.p).physic(|m, _| crate::engine::physic::api::SetFallingDelay(m, a0, a1)); 0 }
    fn APIphysic_SetStateChangeNotifyBits(&mut self, _m: &mut Mem, a0: u32, a1: u32) -> u32 { let a0 = self.r(a0); J2(&self.p).physic(|m, _| crate::engine::physic::api::SetStateChangeNotifyBits(m, a0, a1)); 0 }
    fn APIphysic_ResetStateChangeNotifyBits(&mut self, _m: &mut Mem, a0: u32, a1: u32) -> u32 { let a0 = self.r(a0); J2(&self.p).physic(|m, _| crate::engine::physic::api::ResetStateChangeNotifyBits(m, a0, a1)); 0 }
    fn APIphysic_SetStateChangeNotifyValue(&mut self, _m: &mut Mem, a0: u32, a1: u32) -> u32 { let a0 = self.r(a0); J2(&self.p).physic(|m, _| crate::engine::physic::api::SetStateChangeNotifyValue(m, a0, a1)); 0 }
    fn APIphysic_SetFriction(&mut self, _m: &mut Mem, a0: u32, a1: f32) -> u32 { let a0 = self.r(a0); J2(&self.p).physic(|m, _| crate::engine::physic::api::SetFriction(m, a0, a1)); 0 }
    fn APIphysic_SetHardness(&mut self, _m: &mut Mem, a0: u32, a1: f32) -> u32 { let a0 = self.r(a0); J2(&self.p).physic(|m, _| crate::engine::physic::api::SetHardness(m, a0, a1)); 0 }
    fn APIphysic_SetRebound(&mut self, _m: &mut Mem, a0: u32, a1: f32) -> u32 { let a0 = self.r(a0); J2(&self.p).physic(|m, _| crate::engine::physic::api::SetRebound(m, a0, a1)); 0 }
    fn APIphysic_SetPriority(&mut self, _m: &mut Mem, a0: u32, a1: u32) -> u32 { let a0 = self.r(a0); J2(&self.p).physic(|m, _| crate::engine::physic::api::SetPriority(m, a0, a1)); 0 }
    fn APIphysic_SetWeight(&mut self, _m: &mut Mem, a0: u32, a1: f32) -> u32 { let a0 = self.r(a0); J2(&self.p).physic(|m, _| crate::engine::physic::api::SetWeight(m, a0, a1)); 0 }
    fn APIphysic_SetPrimaryPrim(&mut self, _m: &mut Mem, a0: u32, a1: u32) -> u32 { let a0 = self.r(a0); J2(&self.p).physic(|m, _| crate::engine::physic::api::SetPrimaryPrim(m, a0, a1)); 0 }
    fn APIphysic_Impulse(&mut self, m: &mut Mem, a0: u32, a1: Ptr) -> u32 { let a0 = self.r(a0); let v = m.rv3(a1); J2(&self.p).physic(|p, _| crate::engine::physic::api::Impulse(p, a0, v)); 0 }
    fn APIphysic_SetVelocity(&mut self, _m: &mut Mem, a0: u32, a1: f32, a2: f32, a3: f32) -> u32 { let a0 = self.r(a0); J2(&self.p).physic(|m, w| crate::engine::physic::api::SetVelocity(m, a0, [a1, a2, a3], w)); 0 }
    fn APIphysic_GetVelocity(&mut self, m: &mut Mem, a0: u32, a1: Ptr) -> u32 { let a0 = self.r(a0); J2(&self.p).get_velocity(m, a0, a1) }
    fn APIphysic_GetFloorNormal(&mut self, m: &mut Mem, a0: u32, a1: Ptr) -> u32 { let a0 = self.r(a0); J2(&self.p).get_floor_normal(m, a0, a1) }
    fn APIphysic_GetDM(&mut self, _m: &mut Mem, a0: u32, a1: u32, a2: u32) -> u32 { let a0 = self.r(a0); J2(&self.p).physic(|m, _| crate::engine::physic::api::GetDM(m, a0, a1, a2)) }
    fn APIphysic_CheckRayIntersection(&mut self, m: &mut Mem, a0: Ptr, a1: Ptr) -> u32 { let x = m.r32(a0 + 0x28); let x = self.r(x); m.w32(a0 + 0x28, x); J2(&self.p).ray_intersect(m, a0, a1) }
    fn APIphysic_DMAdd(&mut self, _m: &mut Mem, a0: u32, a1: u32) -> u32 { let a0 = self.r(a0); J2(&self.p).dm_add(a0, a1) }
    fn APIphysic_DMDoAction(&mut self, m: &mut Mem, a0: u32, a1: u32, a2: Ptr) -> u32 { J2(&self.p).dm_do_action(m, a0, a1, a2) }
    fn APIphysic_DMGetData(&mut self, m: &mut Mem, a0: u32, a1: Ptr) -> u32 { J2(&self.p).dm_get_data(m, a0, a1) }
    fn APIphysic_DMGetParams(&mut self, m: &mut Mem, a0: u32, a1: Ptr) -> u32 { J2(&self.p).dm_get_params(m, a0, a1) }
    fn APIphysic_DMSetParams(&mut self, m: &mut Mem, a0: u32, a1: Ptr) -> u32 { J2(&self.p).dm_set_params(m, a0, a1) }
    fn APIphysic_DMSetContextMask(&mut self, _m: &mut Mem, a0: u32, a1: u32) -> u32 { J2(&self.p).physic(|m, _| crate::engine::physic::api::DMSetContextMask(m, a0, a1)); 0 }
    fn APIphysic_DMSetActive(&mut self, _m: &mut Mem, a0: u32, a1: u32) -> u32 { J2(&self.p).physic(|m, _| crate::engine::physic::api::DMSetActive(m, a0, a1)); 0 }
    fn APIphysic_DMIsActive(&mut self, _m: &mut Mem, a0: u32) -> u8 { J2(&self.p).physic(|m, _| crate::engine::physic::api::DMIsActive(m, a0)) as u8 }
    fn APIphysic_DMGetState(&mut self, _m: &mut Mem, a0: u32) -> u32 { J2(&self.p).physic(|m, _| crate::engine::physic::api::DMGetState(m, a0)) }
    fn APIphysic_DMSetStateChangeNotifyBits(&mut self, _m: &mut Mem, a0: u32, a1: u32) -> u32 { J2(&self.p).physic(|m, _| crate::engine::physic::api::DMSetStateChangeNotifyBits(m, a0, a1)); 0 }
    fn APIphysic_DMResetStateChangeNotifyBits(&mut self, _m: &mut Mem, a0: u32, a1: u32) -> u32 { J2(&self.p).physic(|m, _| crate::engine::physic::api::DMResetStateChangeNotifyBits(m, a0, a1)); 0 }
    fn APIphysic_DMSetStateBits(&mut self, _m: &mut Mem, a0: u32, a1: u32) -> u32 { J2(&self.p).physic(|m, _| crate::engine::physic::api::DMSetStateBits(m, a0, a1)); 0 }
    fn APIphysic_DMResetStateBits(&mut self, _m: &mut Mem, a0: u32, a1: u32) -> u32 { J2(&self.p).physic(|m, _| crate::engine::physic::api::DMResetStateBits(m, a0, a1)); 0 }
    fn APIphysic_GetFloorAttribs(&mut self, m: &mut Mem, a0: u32, a1: Ptr) -> u32 { let a0 = self.r(a0); crate::engine::host::physic::GetFloorAttribs(&self.p, m, a0, a1) }
    fn APIphysic_SetSpherePrimaryVolume(&mut self, _m: &mut Mem, a0: u32, a1: f32, a2: f32, a3: f32, a4: f32) -> u32 { let a0 = self.r(a0); crate::engine::host::physic::SetSpherePrimaryVolume(&self.p, a0, a1, a2, a3, a4) }
    fn APIphysic_JumpOnCoor(&mut self, m: &mut Mem, a0: u32, a1: Ptr) -> u32 { let a0 = self.r(a0); crate::engine::host::physic::JumpOnCoor(&self.p, m, a0, a1) }
    fn APIphysic_PrimSetIntAttribBits(&mut self, _m: &mut Mem, a0: u32, a1: u32, a2: u32, a3: u32) -> u32 { let a0 = self.r(a0); crate::engine::host::physic::PrimIntAttrib(&self.p, 0, a0, a1, a2, a3) }
    fn APIphysic_PrimResetIntAttribBits(&mut self, _m: &mut Mem, a0: u32, a1: u32, a2: u32, a3: u32) -> u32 { let a0 = self.r(a0); crate::engine::host::physic::PrimIntAttrib(&self.p, 1, a0, a1, a2, a3) }
    fn APIphysic_PrimSetIntAttribValue(&mut self, _m: &mut Mem, a0: u32, a1: u32, a2: u32, a3: u32) -> u32 { let a0 = self.r(a0); crate::engine::host::physic::PrimIntAttrib(&self.p, 2, a0, a1, a2, a3) }
    fn APIphysic_PrimUnlockIntAttrib(&mut self, _m: &mut Mem, a0: u32, a1: u32, a2: u32, a3: u32) -> u32 { let a0 = self.r(a0); crate::engine::host::physic::PrimIntAttrib(&self.p, 3, a0, a1, a2, a3) }
    fn APIphysic_PrimGetIntAttrib(&mut self, _m: &mut Mem, a0: u32, a1: u32, a2: u32) -> u32 { let a0 = self.r(a0); crate::engine::host::physic::PrimGetIntAttrib(&self.p, a0, a1, a2) }
    fn APIphysic_PrimGetOffsetFromJoint(&mut self, m: &mut Mem, a0: u32, a1: u32, a2: Ptr) -> u32 { let a0 = self.r(a0); crate::engine::host::physic::PrimGetOffsetFromJoint(&self.p, m, a0, a1, a2) }
    fn APIphysic_PrimSetOffsetFromJoint(&mut self, m: &mut Mem, a0: u32, a1: u32, a2: Ptr) -> u32 { let a0 = self.r(a0); crate::engine::host::physic::PrimSetOffsetFromJoint(&self.p, m, a0, a1, a2) }

    // ---- J2: EFApi_NewSound.cpp
    fn APInewSound_Play(&mut self, _m: &mut Mem, a0: u32, a1: u32, a2: u32) -> u32 { let a0 = self.r(a0); J2(&self.p).sound_play(a0, a1, a2) }
    fn APInewSound_StopSound(&mut self, _m: &mut Mem, a0: u32, a1: u32) -> u32 { let a0 = self.r(a0); J2(&self.p).sound(|s| s.APInewSound_StopSound(a0, a1)) }
    fn APInewSound_SetListener(&mut self, _m: &mut Mem, a0: u32) -> u32 { let a0 = self.r(a0); J2(&self.p).sound(|s| s.APInewSound_SetListener(a0)); 0 }
    fn APInewSound_PauseSoundEvents(&mut self, _m: &mut Mem, a0: u32) -> u32 { let a0 = self.r(a0); J2(&self.p).sound(|s| s.APInewSound_PauseSoundEvents(a0)); 0 }
    fn APInewSound_ResumeSoundEvents(&mut self, _m: &mut Mem, a0: u32) -> u32 { let a0 = self.r(a0); J2(&self.p).sound(|s| s.APInewSound_ResumeSoundEvents(a0)); 0 }
    fn APInewSound_SetVolumeByGroup(&mut self, _m: &mut Mem, a0: u32, a1: f32) -> u32 { J2(&self.p).sound(|s| s.APInewSound_SetVolumeByGroup(a0, a1)); 0 }
    fn APInewSound_SetGlobalVolume(&mut self, _m: &mut Mem, a0: f32) -> u32 { J2(&self.p).sound(|s| s.APInewSound_SetGlobalVolume(a0)); 0 }
    fn APInewSound_GetGlobalVolume(&mut self, _m: &mut Mem) -> u32 { J2(&self.p).sound(|s| s.APInewSound_GetGlobalVolume().to_bits()) }
    fn APInewSound_SetFXVolume(&mut self, _m: &mut Mem, a0: f32) -> u32 { J2(&self.p).sound(|s| s.APInewSound_SetFXVolume(a0)); 0 }
    fn APInewSound_GetFXVolume(&mut self, _m: &mut Mem) -> u32 { J2(&self.p).sound(|s| s.APInewSound_GetFXVolume().to_bits()) }
    fn APInewSound_SetStreamVolume(&mut self, _m: &mut Mem, a0: f32) -> u32 { J2(&self.p).sound(|s| s.APInewSound_SetStreamVolume(a0)); 0 }
    fn APInewSound_GetStreamVolume(&mut self, _m: &mut Mem) -> u32 { J2(&self.p).sound(|s| s.APInewSound_GetStreamVolume().to_bits()) }
    fn APInewSound_SetSoundEventMasterVolume(&mut self, _m: &mut Mem, a0: u32, a1: u32, a2: f32) -> u32 { let a0 = self.r(a0); J2(&self.p).sound(|s| s.APInewSound_SetSoundEventMasterVolume(a0, a1, a2)) }
    fn APInewSound_GetSoundEventMasterVolume(&mut self, _m: &mut Mem, a0: u32, a1: u32) -> f32 { let a0 = self.r(a0); J2(&self.p).sound(|s| s.APInewSound_GetSoundEventMasterVolume(a0, a1)) }
    fn APInewSound_IsPlaying(&mut self, _m: &mut Mem, a0: u32, a1: u32) -> u8 { let a0 = self.r(a0); J2(&self.p).sound(|s| s.APInewSound_IsPlaying(a0, a1)) as u8 }
    fn APInewSound_SetPitch(&mut self, _m: &mut Mem, a0: u32, a1: u32, a2: f32) -> u32 { let a0 = self.r(a0); J2(&self.p).sound(|s| s.APInewSound_SetPitch(a0, a1, a2)) }
    fn APInewSound_ParentSoundEventToJoint(&mut self, _m: &mut Mem, a0: u32, a1: u32, a2: u32, _a3: u32) -> u32 { let a0 = self.r(a0); J2(&self.p).sound(|s| s.APInewSound_ParentSoundEventToJoint(a0, a1, a2)) }
    fn APInewSound_SetDSPEffectForGroup(&mut self, _m: &mut Mem, a0: u32, a1: u32) -> u32 { J2(&self.p).sound(|s| s.APInewSound_SetDSPEffectForGroup(a0, a1)); 0 }
    fn APInewSound_SetAudioEngineMode(&mut self, _m: &mut Mem, a0: u32) -> u32 { J2(&self.p).sound(|s| s.APInewSound_SetAudioEngineMode(a0)); 0 }
    fn APInewSound_SetVideoVolume(&mut self, _m: &mut Mem, a0: f32) -> u32 { J2(&self.p).sound(|s| s.APInewSound_SetVideoVolume(a0)); 0 }
    fn APInewSound_GetVideoVolume(&mut self, _m: &mut Mem) -> u32 { J2(&self.p).sound(|s| s.APInewSound_GetVideoVolume().to_bits()) }

    // ---- J2: EFApi_Camera.cpp (GetGameObject(obj) frame passed to CreateCamera)
    fn APIcamera_CreateCamera(&mut self, m: &mut Mem, a0: u32) -> u32 { let a0 = self.r(a0); J2(&self.p).camera_create(m, a0) }
    fn APIcamera_DestroyCamera(&mut self, m: &mut Mem, a0: u32) -> u32 { J2(&self.p).cameras(|c, _| c.APIcamera_DestroyCamera(m, a0)) }
    fn APIcamera_Activate(&mut self, m: &mut Mem, a0: u32, a1: u32) -> u32 { J2(&self.p).cameras(|c, _| c.APIcamera_Activate(m, a0, a1)) }
    fn APIcamera_SetPriority(&mut self, m: &mut Mem, a0: u32, a1: f32) -> u32 { J2(&self.p).cameras(|c, _| c.APIcamera_SetPriority(m, a0, a1)) }
    fn APIcamera_GetPriority(&mut self, m: &mut Mem, a0: u32) -> u32 { J2(&self.p).cameras(|c, _| c.APIcamera_GetPriority(m, a0)) }
    fn APIcamera_SetFarClipPlane(&mut self, m: &mut Mem, a0: u32, a1: f32) -> u32 { J2(&self.p).cameras(|c, _| c.APIcamera_SetFarClipPlane(m, a0, a1)) }
    fn APIcamera_SetNearClipPlane(&mut self, m: &mut Mem, a0: u32, a1: f32) -> u32 { J2(&self.p).cameras(|c, _| c.APIcamera_SetNearClipPlane(m, a0, a1)) }
    fn APIcamera_SetClearColor(&mut self, m: &mut Mem, a0: u32, a1: Ptr) -> u32 { J2(&self.p).cameras(|c, _| c.APIcamera_SetClearColor(m, a0, a1)) }
    fn APIcamera_SetClearMode(&mut self, m: &mut Mem, a0: u32, a1: u32) -> u32 { J2(&self.p).cameras(|c, _| c.APIcamera_SetClearMode(m, a0, a1)) }
    fn APIcamera_SetProjection(&mut self, m: &mut Mem, a0: u32, a1: u32) -> u32 { J2(&self.p).cameras(|c, _| c.APIcamera_SetProjection(m, a0, a1)) }
    fn APIcamera_GetViewWindow(&mut self, m: &mut Mem, a0: u32, a1: Ptr, a2: Ptr) -> u32 { J2(&self.p).cameras(|c, _| c.APIcamera_GetViewWindow(m, a0, a1, a2)) }
    fn APIcamera_SetViewWindow(&mut self, m: &mut Mem, a0: u32, a1: f32, a2: f32) -> u32 { J2(&self.p).cameras(|c, _| c.APIcamera_SetViewWindow(m, a0, a1, a2)) }
    fn APIcamera_SetDisplayArea(&mut self, m: &mut Mem, a0: u32, a1: Ptr) -> u32 { J2(&self.p).cameras(|c, _| c.APIcamera_SetDisplayArea(m, a0, a1)) }
    fn APIcamera_GetDisplayArea(&mut self, m: &mut Mem, a0: u32, a1: Ptr) -> u32 { J2(&self.p).cameras(|c, _| c.APIcamera_GetDisplayArea(m, a0, a1)) }
    fn APIcamera_SetRenderFlag(&mut self, m: &mut Mem, a0: u32, a1: u32) -> u32 { J2(&self.p).cameras(|c, _| c.APIcamera_SetRenderFlag(m, a0, a1)) }
    fn APIcamera_SetFOV(&mut self, m: &mut Mem, a0: u32, a1: f32) -> u32 { J2(&self.p).cameras(|c, _| c.APIcamera_SetFOV(m, a0, a1)) }
    fn APIcamera_TakeSnapShot(&mut self, m: &mut Mem, a0: u32, a1: Ptr) -> u32 { J2(&self.p).cameras(|c, _| c.APIcamera_TakeSnapShot(m, a0, a1)) }
    fn APIcamera_ProjectOntoFrustum(&mut self, m: &mut Mem, a0: u32, a1: Ptr, a2: Ptr) -> u8 { J2(&self.p).cameras(|c, _| c.APIcamera_ProjectOntoFrustum(m, a0, a1, a2)) }
    fn APIcamera_IsSphereInFrustrum(&mut self, m: &mut Mem, a0: u32, a1: Ptr, a2: f32) -> u32 { J2(&self.p).cameras(|c, _| c.APIcamera_IsSphereInFrustrum(m, a0, a1, a2)) }
    fn APIcamera_EnableFog(&mut self, m: &mut Mem, a0: u32, a1: i32) -> u32 { J2(&self.p).cameras(|c, _| c.APIcamera_EnableFog(m, a0, a1)) }
    fn APIcamera_SetFogType(&mut self, m: &mut Mem, a0: u32, a1: u32) -> u32 { J2(&self.p).cameras(|c, _| c.APIcamera_SetFogType(m, a0, a1)) }
    fn APIcamera_GetFogType(&mut self, m: &mut Mem, a0: u32) -> u32 { J2(&self.p).cameras(|c, _| c.APIcamera_GetFogType(m, a0)) }
    fn APIcamera_SetFogColor(&mut self, m: &mut Mem, a0: u32, a1: Ptr) -> u32 { J2(&self.p).cameras(|c, _| c.APIcamera_SetFogColor(m, a0, a1)) }
    fn APIcamera_SetFogDensity(&mut self, m: &mut Mem, a0: u32, a1: f32) -> u32 { J2(&self.p).cameras(|c, _| c.APIcamera_SetFogDensity(m, a0, a1)) }
    fn APIcamera_GetFogDensity(&mut self, m: &mut Mem, a0: u32) -> u32 { J2(&self.p).cameras(|c, _| c.APIcamera_GetFogDensity(m, a0)) }
    fn APIcamera_SetFogDistance(&mut self, m: &mut Mem, a0: u32, a1: f32) -> u32 { J2(&self.p).cameras(|c, _| c.APIcamera_SetFogDistance(m, a0, a1)) }
    fn APIcamera_GetFogDistance(&mut self, m: &mut Mem, a0: u32) -> u32 { J2(&self.p).cameras(|c, _| c.APIcamera_GetFogDistance(m, a0)) }

    // ---- J2: EFApi_Light.cpp
    fn APIlight_CreateLight(&mut self, m: &mut Mem, a0: u32) -> u32 { let a0 = self.r(a0); J2(&self.p).lights(|l| l.APIlight_CreateLight(m, a0, a0)) }
    fn APIlight_SetFlag(&mut self, m: &mut Mem, a0: u32, a1: u32) -> u32 { J2(&self.p).lights(|l| l.APIlight_SetFlag(m, a0, a1)) }
    fn APIlight_GetFlag(&mut self, m: &mut Mem, a0: u32) -> u8 { J2(&self.p).lights(|l| l.APIlight_GetFlag(m, a0)) }
    fn APIlight_SetColor(&mut self, m: &mut Mem, a0: u32, a1: Ptr) -> u32 { J2(&self.p).lights(|l| l.APIlight_SetColor(m, a0, a1)) }
    fn APIlight_GetColor(&mut self, m: &mut Mem, a0: u32, a1: Ptr) -> u32 { J2(&self.p).lights(|l| l.APIlight_GetColor(m, a0, a1)) }
    fn APIlight_SetRadius(&mut self, m: &mut Mem, a0: u32, a1: f32) -> u32 { J2(&self.p).lights(|l| l.APIlight_SetRadius(m, a0, a1)) }
    fn APIlight_GetRadius(&mut self, m: &mut Mem, a0: u32) -> u32 { J2(&self.p).lights(|l| l.APIlight_GetRadius(m, a0)) }
    fn APIlight_SetConeAngle(&mut self, m: &mut Mem, a0: u32, a1: f32) -> u32 { J2(&self.p).lights(|l| l.APIlight_SetConeAngle(m, a0, a1)) }
    fn APIlight_GetConeAngle(&mut self, m: &mut Mem, a0: u32) -> u32 { J2(&self.p).lights(|l| l.APIlight_GetConeAngle(m, a0)) }
    fn APIlight_SetVisible(&mut self, m: &mut Mem, a0: u32, a1: i32) -> u32 { J2(&self.p).lights(|l| l.APIlight_SetVisible(m, a0, a1)) }
    fn APIlight_IsVisible(&mut self, m: &mut Mem, a0: u32) -> u32 { J2(&self.p).lights(|l| l.APIlight_IsVisible(m, a0)) }
    // ---- J4: APIgraphics_* (WP-C5) -> ParticleShared::gfxapi
    fn APIgraphics_SpriteAddGroup(&mut self, m: &mut Mem) -> u32 { gx!(self, APIgraphics_SpriteAddGroup(m)) }
    fn APIgraphics_SpriteRemoveGroup(&mut self, m: &mut Mem, a0: u32) -> u32 { gx!(self, APIgraphics_SpriteRemoveGroup(m, a0)) }
    fn APIgraphics_SpriteSetGroupFlags(&mut self, m: &mut Mem, a0: u32, a1: u32) -> u32 { gx!(self, APIgraphics_SpriteSetGroupFlags(m, a0, a1)) }
    fn APIgraphics_SpriteGetGroupFlags(&mut self, m: &mut Mem, a0: u32) -> u32 { gx!(self, APIgraphics_SpriteGetGroupFlags(m, a0)) }
    fn APIgraphics_SpriteSetGroupTexture(&mut self, m: &mut Mem, a0: u32, a1: Ptr) -> u32 { gx!(self, APIgraphics_SpriteSetGroupTexture(m, a0, a1)) }
    fn APIgraphics_SpriteSetGroupBlendMode(&mut self, m: &mut Mem, a0: u32, a1: u32) -> u32 { gx!(self, APIgraphics_SpriteSetGroupBlendMode(m, a0, a1)) }
    fn APIgraphics_SpriteSetGroupColor(&mut self, m: &mut Mem, a0: u32, a1: f32, a2: f32, a3: f32, a4: f32) -> u32 { gx!(self, APIgraphics_SpriteSetGroupColor(m, a0, a1, a2, a3, a4)) }
    fn APIgraphics_SpriteGroupAddSprite(&mut self, m: &mut Mem, a0: u32, a1: u32, a2: u32, a3: f32) -> u32 { let a1 = self.r(a1); gxw!(self, w, APIgraphics_SpriteGroupAddSprite(m, a0, a1, a2, a3, w)) }
    fn APIgraphics_SpriteGroupRemoveSprite(&mut self, m: &mut Mem, a0: u32, a1: u32) -> u32 { gx!(self, APIgraphics_SpriteGroupRemoveSprite(m, a0, a1)) }
    fn APIgraphics_StartPaletteTransition(&mut self, m: &mut Mem, a0: Ptr) -> u32 { gx!(self, APIgraphics_StartPaletteTransition(m, a0)) }
    fn APIgraphics_TexFaderPlay(&mut self, m: &mut Mem, a0: u32, a1: Ptr) -> u32 { let a0 = self.r(a0); gxw!(self, w, APIgraphics_TexFaderPlay(m, a0, a1, w)) }
    fn APIgraphics_TexFaderPause(&mut self, m: &mut Mem, a0: u32, a1: u32) -> u32 { let a0 = self.r(a0); gxw!(self, w, APIgraphics_TexFaderPause(m, a0, a1, w)) }
    fn APIgraphics_MTCreateSystem(&mut self, m: &mut Mem, a0: u32, a1: u32, a2: u32) -> u32 { gx!(self, APIgraphics_MTCreateSystem(m, a0, a1, a2)) }
    fn APIgraphics_MTDestroySystem(&mut self, m: &mut Mem, a0: u32) -> u32 { gx!(self, APIgraphics_MTDestroySystem(m, a0)) }
    fn APIgraphics_MTSetFrequency(&mut self, m: &mut Mem, a0: u32, a1: f32) -> u32 { gx!(self, APIgraphics_MTSetFrequency(m, a0, a1)) }
    fn APIgraphics_MTSetLifeSpan(&mut self, m: &mut Mem, a0: u32, a1: u32, a2: f32) -> u32 { gx!(self, APIgraphics_MTSetLifeSpan(m, a0, a1, a2)) }
    fn APIgraphics_MTSetJoint(&mut self, m: &mut Mem, a0: u32, a1: u32, a2: u32, a3: u32) -> u32 { let a2 = self.r(a2); gxw!(self, w, APIgraphics_MTSetJoint(m, a0, a1, a2, a3, w)) }
    fn APIgraphics_MTSetPosition(&mut self, m: &mut Mem, a0: u32, a1: u32, a2: Ptr) -> u32 { gx!(self, APIgraphics_MTSetPosition(m, a0, a1, a2)) }
    fn APIgraphics_MTSetRGB(&mut self, m: &mut Mem, a0: u32, a1: u32, a2: f32, a3: f32, a4: f32) -> u32 { gx!(self, APIgraphics_MTSetRGB(m, a0, a1, a2, a3, a4)) }
    fn APIgraphics_MTStart(&mut self, m: &mut Mem, a0: u32) -> u32 { gx!(self, APIgraphics_MTStart(m, a0)) }
    fn APIgraphics_MTStopTracking(&mut self, m: &mut Mem, a0: u32) -> u32 { gx!(self, APIgraphics_MTStopTracking(m, a0)) }
    fn APIgraphics_MTStopDraw(&mut self, m: &mut Mem, a0: u32) -> u32 { gx!(self, APIgraphics_MTStopDraw(m, a0)) }
    fn APIgraphics_LensFlareActivate(&mut self, m: &mut Mem, a0: u8) -> u32 { gx!(self, APIgraphics_LensFlareActivate(m, a0)) }
    fn APIgraphics_LensFlareSetSunPosition(&mut self, m: &mut Mem, a0: Ptr) -> u32 { gx!(self, APIgraphics_LensFlareSetSunPosition(m, a0)) }
    fn APIgraphics_LensFlareAttachSunPositionToObject(&mut self, m: &mut Mem, a0: u32) -> u32 { let a0 = self.r(a0); gx!(self, APIgraphics_LensFlareAttachSunPositionToObject(m, a0)) }
    fn APIgraphics_LensFlareSetParams(&mut self, m: &mut Mem, a0: u32, a1: u32) -> u32 { gx!(self, APIgraphics_LensFlareSetParams(m, a0, a1)) }
    fn APIgraphics_SetGameObjBlendColor(&mut self, m: &mut Mem, a0: u32, a1: u32, a2: f32, a3: f32, a4: f32, a5: f32) -> u32 { let a0 = self.r(a0); gxw!(self, w, APIgraphics_SetGameObjBlendColor(m, a0, a1, a2, a3, a4, a5, w)) }
    fn APIgraphics_DrawLines(&mut self, m: &mut Mem, a0: Ptr) -> u32 { gx!(self, APIgraphics_DrawLines(m, a0)) }
    fn APIgraphics_DoCrappyXboxSpewHackThingy(&mut self, m: &mut Mem, a0: u32, a1: u32, a2: u8) -> u32 { let a0 = self.r(a0); gxw!(self, w, APIgraphics_DoCrappyXboxSpewHackThingy(m, a0, a1, a2, w)) }
    fn APIgraphics_DoStupidXboxShadowHack(&mut self, m: &mut Mem, a0: u8) -> u32 { gx!(self, APIgraphics_DoStupidXboxShadowHack(m, a0)) }
    /// AddChild (0x001ee250)(parent, joint, child, flags): child = EFWorld::GetObjectPosData(child).
    /// WP-C5 step 5: a sprite child -> EFSpriteMgr_BC::AttachSprite onto GetObjectFrame(joint)
    /// (joint 0x7fffffff or a non-0x2a parent: the parent pos-data frame = bone 0).
    /// deviation: game-object children (vtbl +0x60/+0x5c RwFrameAddChild of another game object's
    /// frame) are not modelled by the runtime frames and stay a no-op.
    fn APIgameobj_AddChild(&mut self, _m: &mut Mem, a0: u32, a1: u32, a2: u32, a3: u32) -> u32 {
        let a0 = self.r(a0);
        let a2 = self.r(a2);
        // J14: object-on-bone parenting (EFPosData::AddChildFrame): flag 1 = identity local frame,
        // else local translation = child world pos - parent frame world pos (RwFrameTranslate on
        // the identity, i.e. unrotated, as the original)
        // J23: bone ALL (0x7fffffff) = EFPosData::AddChild (0x00209e10) of the whole object: the child
        // frame becomes a child of the parent's root frame (flag 1 identity, else world offset);
        // was a no-op, so the book's tod_beam latent (hands, spirit sprites/particles, AddChild(book,
        // ALL, beam, 1) in TomeDoomFUNC_SET_CAPTURE) stayed at the world origin.
        if a0 >> 24 == 0x2a && a2 >> 24 == 0x2a {
            if let (Some(pf), Some(cf)) = (self.obj_ltm(a0, a1), self.obj_ltm(a2, 0x7fff_ffff)) {
                let off = if a3 & 1 != 0 { [0.0; 3] } else { std::array::from_fn(|i| cf.pos[i] - pf.pos[i]) };
                let mut g = self.p.lock().unwrap();
                g.parented.retain(|c| c.0 != a2);
                g.parented.push((a2, a0, a1, off));
                drop(g);
                self.place_parented();
            }
            return 0;
        }
        // WP-S4: a projector child (EFPosData::AddChildFrame of the projector frame): flag 1 = identity
        // local frame, else the local translation keeps its world position
        if self.p.lock().unwrap().s1.projectors.projectors.contains_key(&a2) {
            let cf = self.p.lock().unwrap().world.frames.get(&a2).copied();
            if let (Some(pf), Some(cf)) = (self.obj_ltm(a0, a1), cf) {
                let off = if a3 & 1 != 0 { [0.0; 3] } else { std::array::from_fn(|i| cf.pos[i] - pf.pos[i]) };
                let mut g = self.p.lock().unwrap();
                g.parented.retain(|c| c.0 != a2);
                g.parented.push((a2, a0, a1, off));
                drop(g);
                self.place_parented();
            }
            return 0;
        }
        let mut g = self.p.lock().unwrap();
        let bone = if a0 >> 24 == 0x2a && a1 != 0x7fff_ffff { a1 } else { 0 };
        if g.world.frames.contains_key(&a0) {
            g.gfxapi.sprites.AttachSprite(a2, a0, bone);
        }
        0
    }
    /// LeaveParent 0x001ee360 -> EFPosData::LeaveParent: flag 1 resets the frame to identity, else the
    /// frame keeps its world LTM (orthonormalized; the bone frame is already orthonormal here)
    fn APIgameobj_LeaveParent(&mut self, _m: &mut Mem, a0: u32, a1: u32) -> u32 {
        let a0 = self.r(a0);
        let mut g = self.p.lock().unwrap();
        g.parented.retain(|c| c.0 != a0);
        if a1 & 1 != 0 {
            if let Some(f) = g.world.frames.get_mut(&a0) {
                *f = DTMatrix { right: [1., 0., 0.], up: [0., 1., 0.], at: [0., 0., 1.], pos: [0.; 3] };
            }
        }
        0
    }
    /// J4: EFGameObject +0x154 group id / +0x158 group bit mask (APIgameobj Set/GetGroupID,
    /// Set/GetGroupBitMask 0x001ef9b0..0x001efa6c, 1:1); the fields live on the logic object (the
    /// retrievers 0xff000008/0xff000010 read them there). Unknown object: Get returns 0x7ffffffe.
    fn APIgameobj_SetGroupBitMask(&mut self, _m: &mut Mem, a0: u32, a1: u32) -> u32 { self.group(a0, |o| o.group_mask = a1); 0 }
    fn APIgameobj_SetGroupID(&mut self, _m: &mut Mem, a0: u32, a1: u32) -> u32 { self.group(a0, |o| o.group_id = a1); 0 }
    fn APIgameobj_GetGroupBitMask(&mut self, _m: &mut Mem, a0: u32) -> u32 { self.group(a0, |o| o.group_mask).unwrap_or(0x7fff_fffe) }
    fn APIgameobj_GetGroupID(&mut self, _m: &mut Mem, a0: u32) -> u32 { self.group(a0, |o| o.group_id).unwrap_or(0x7fff_fffe) }
    // ---- J4: EFApi_GameObj frame writes/queries on the runtime object frames (world.frames are the
    // objects' RwFrames; physics copies them in before its update). Bone frames other than the root
    // (param != 0x7fffffff) resolve through PLWorld::GetObjectFrame.
    /// GetLookRight 0x001ee860: LTM right (zero when no frame)
    fn APIgameobj_GetLookRight(&mut self, mem: &mut Mem, a0: u32, a1: u32, a2: Ptr) -> Ptr {
        let v = self.obj_ltm(a0, a1).map_or([0.0; 3], |m| m.right);
        wv3(mem, a2, v);
        a2
    }
    /// SetTranslation 0x001eea80: pos = 0 then RwFrameTranslate(v, POSTCONCAT) => pos = v.
    /// deviation: the physic primary-prim refresh (+0x28 vtbl+0xc, +0x84.. , +0x7c frame stamp) is
    /// done by the next EFPhysicMgr::Update copy-in of the frame.
    fn APIgameobj_SetTranslation(&mut self, mem: &mut Mem, a0: u32, a1: Ptr) -> u32 {
        let a0 = self.r(a0);
        let v = rv3(mem, a1);
        if let Some(f) = self.p.lock().unwrap().world.frames.get_mut(&a0) {
            f.pos = v;
        }
        0
    }
    /// TakeOrientation 0x001ef5b0: copy the 3x3 of param_2's (bone) LTM into param_1's modelling matrix
    fn APIgameobj_TakeOrientation(&mut self, _m: &mut Mem, a0: u32, a1: u32, a2: u32) -> u32 {
        let a0 = self.r(a0);
        let Some(s) = self.obj_ltm(a1, a2) else { return 0 };
        if let Some(f) = self.p.lock().unwrap().world.frames.get_mut(&a0) {
            (f.right, f.up, f.at) = (s.right, s.up, s.at);
        }
        0
    }
    /// TransformVector 0x001ef6f0: RwV3dTransformVectors by the object LTM (3x3 only)
    fn APIgameobj_TransformVector(&mut self, mem: &mut Mem, a0: u32, a1: Ptr, a2: Ptr) -> u32 {
        let Some(m) = self.obj_ltm(a0, 0x7fff_ffff) else { return 0 };
        let v = rv3(mem, a1);
        let o: [f32; 3] = std::array::from_fn(|i| v[0] * m.right[i] + v[1] * m.up[i] + v[2] * m.at[i]);
        wv3(mem, a2, o);
        0
    }
    /// InvertTransformVector 0x001ef770: by RwMatrixInvert(LTM) (3x3 part). Replaced: the frames are
    /// orthonormal, so the inverse rotation is the transpose (RwMatrixInvert's general path gives the
    /// same 3x3 for orthonormal input up to rounding).
    fn APIgameobj_InvertTransformVector(&mut self, mem: &mut Mem, a0: u32, a1: Ptr, a2: Ptr) -> u32 {
        let Some(m) = self.obj_ltm(a0, 0x7fff_ffff) else { return 0 };
        let v = rv3(mem, a1);
        let d = |a: [f32; 3]| a[0] * v[0] + a[1] * v[1] + a[2] * v[2];
        wv3(mem, a2, [d(m.right), d(m.up), d(m.at)]);
        0
    }
    /// MyRand 0x001f8a60: rand() & 0x7fff on the one shared libc sequence (J4: was the Host default 0,
    /// which made TomeDoomFUNC_GenerateButton's `while` spin forever)
    fn APIstdlib_MyRand(&mut self, _m: &mut Mem) -> u32 {
        let r = self.p.lock().unwrap().sub.collision.rand.clone();
        (r.lock().unwrap().rand() as u32) & 0x7fff
    }
    /// GetLMat (EFApi_GameObj): the object's RwFrame modelling matrix. Replaced like GetBoneLMat:
    /// a persistent Mem RwFrame stand-in (bone 0), refreshed from the current object frame on each
    /// call; writes are not fed back into the object frame.
    fn APIgameobj_GetLMat(&mut self, mem: &mut Mem, a0: u32) -> i32 {
        let a0 = self.r(a0);
        let f = bone_frame(&self.p, mem, a0, 0);
        if let Some(m) = self.p.lock().unwrap().world.frames.get(&a0).copied() {
            write_rwmatrix(mem, f + 0x10, &m);
            write_rwmatrix(mem, f + 0x50, &m);
        }
        (f + 0x10) as i32
    }
    fn APIlight_DestroyLight(&mut self, m: &mut Mem, a0: u32) -> u32 { J2(&self.p).lights(|l| l.APIlight_DestroyLight(m, a0)) }
    // WP-S3: APIgame / APIextra / APIstdlib / APIkaraoke / APIqa / APIconsole / APILoadSave -> crate::engine::host::{game,extra,stdlib}
    fn APIgame_Pause(&mut self, _m: &mut Mem, a0: u32) -> u32 { let mut g = self.p.lock().unwrap(); let was = g.world.paused as u32; g.world.paused = a0 != 0; if let Some(c) = g.cutscene.as_mut() { c.Pause(a0) } was }
    fn APIgame_LoadLevel(&mut self, m: &mut Mem, a0: Ptr, a1: Ptr, a2: Ptr) -> Ptr { self.p.lock().unwrap().game.LoadLevel(m, a0, a1, a2) }
    fn APIgame_SetGameLanguage(&mut self, m: &mut Mem, a0: Ptr) -> Ptr { self.p.lock().unwrap().game.SetGameLanguage(m, a0) }
    fn APIgame_GetGameLanguage(&mut self, m: &mut Mem) -> i32 { self.p.lock().unwrap().game.GetGameLanguage(m) as i32 }
    fn APIgame_QuitGame(&mut self, _m: &mut Mem) -> u32 { self.p.lock().unwrap().game.QuitGame() }
    fn APIgame_GetDemoLaunchData(&mut self, m: &mut Mem) -> u32 { self.p.lock().unwrap().game.GetDemoLaunchData(m) }
    fn APIgame_GetSystemRegion(&mut self, _m: &mut Mem) -> u32 { self.p.lock().unwrap().game.GetSystemRegion() }
    fn APIqa_CloseGame(&mut self, _m: &mut Mem) -> u32 { self.p.lock().unwrap().game.QuitGame() }
    fn APIqa_AppendToFile(&mut self, _m: &mut Mem, _a0: Ptr, _a1: Ptr) -> u32 { 0 } // DTFopen/"DTFputs Unimplemented"/DTFclose: writes nothing (1:1)
    fn APIqa_SendToGameLord(&mut self, _m: &mut Mem, _a0: Ptr) -> u32 { 0 } // empty in the PS2 build
    fn APIconsole_SetPrimDisplayFlag(&mut self, _m: &mut Mem, _a0: u32) -> u32 { 0 } // empty (debug-context reset only)
    fn APIconsole_GetPrimDisplayFlag(&mut self, _m: &mut Mem) -> u32 { 0 } // always 0
    fn APIextra_null_printf(&mut self, _m: &mut Mem, _a0: Ptr, _a1: u32) -> u32 { 0 } // returns 0, prints nothing
    fn APIextra_null_sprintf(&mut self, _m: &mut Mem, _a0: Ptr, _a1: Ptr, _a2: u32) -> u32 { 0 } // returns 0, buffer untouched
    fn APIextra_Assert(&mut self, _m: &mut Mem, _a0: u32, _a1: Ptr) -> u32 { 0 } // empty in the release build
    fn APIextra_WaypointGetPos(&mut self, m: &mut Mem, a0: u32, a1: u32, a2: Ptr) -> Ptr { crate::engine::host::extra::WaypointGetPos(&self.p.lock().unwrap().game, m, a0, a1, a2) }
    fn APIextra_WaypointSetPos(&mut self, m: &mut Mem, a0: u32, a1: u32, a2: Ptr) -> Ptr { crate::engine::host::extra::WaypointSetPos(&mut self.p.lock().unwrap().game, m, a0, a1, a2) }
    fn APIextra_WaypointGetNb(&mut self, _m: &mut Mem, a0: u32) -> u32 { crate::engine::host::extra::WaypointGetNb(&self.p.lock().unwrap().game, a0) }
    fn APIextra_DemoLoopIsPlaying(&mut self, _m: &mut Mem) -> u8 { crate::engine::host::extra::DemoLoopIsPlaying() }
    fn APIstdlib_abs(&mut self, _m: &mut Mem, a0: i32) -> i32 { crate::engine::host::stdlib::abs(a0) }
    fn APIstdlib_labs(&mut self, _m: &mut Mem, a0: i64) -> i64 { crate::engine::host::stdlib::labs(a0) }
    fn APIstdlib_div(&mut self, _m: &mut Mem, a0: i32, a1: i32) -> u64 { crate::engine::host::stdlib::div(a0, a1) }
    fn APIstdlib_ldiv(&mut self, m: &mut Mem, a0: Ptr, a1: i64, a2: i64) -> Ptr { crate::engine::host::stdlib::ldiv(m, a0, a1, a2) }
    fn APIstdlib_atof(&mut self, m: &mut Mem, a0: Ptr) -> f64 { crate::engine::host::stdlib::atof(m, a0) }
    fn APIstdlib_atoi(&mut self, m: &mut Mem, a0: Ptr) -> i32 { crate::engine::host::stdlib::atoi(m, a0) }
    fn APIstdlib_atol(&mut self, m: &mut Mem, a0: Ptr) -> i64 { crate::engine::host::stdlib::atol(m, a0) }
    fn APIstdlib_strtod(&mut self, m: &mut Mem, a0: Ptr, a1: Ptr) -> f64 { crate::engine::host::stdlib::strtod(m, a0, a1) }
    fn APIstdlib_strtol(&mut self, m: &mut Mem, a0: Ptr, a1: Ptr, a2: i32) -> i64 { crate::engine::host::stdlib::strtol(m, a0, a1, a2) }
    fn APIstdlib_strtoul(&mut self, m: &mut Mem, a0: Ptr, a1: Ptr, a2: i32) -> u64 { crate::engine::host::stdlib::strtoul(m, a0, a1, a2) }
    fn APIstdlib_mblen(&mut self, m: &mut Mem, a0: Ptr, a1: u32) -> i32 { crate::engine::host::stdlib::mblen(m, a0, a1) }
    /// srand: seeds the one shared libc sequence (the one MyRand reads)
    fn APIstdlib_srand(&mut self, _m: &mut Mem, a0: u32) -> u32 { let r = self.p.lock().unwrap().sub.collision.rand.clone(); r.lock().unwrap().0 = a0 as u64; 0 }
    fn APIkaraoke_AcquireMicrophone(&mut self, _m: &mut Mem, a0: u32) -> u32 { self.p.lock().unwrap().game.karaoke.SOUND_OpenUSBMicrophone(a0 as u16); 0 }
    fn APIkaraoke_ReleaseMicrophone(&mut self, _m: &mut Mem, a0: u32) -> u32 { self.p.lock().unwrap().game.karaoke.SOUND_CloseUSBMicrophone(a0 as u16); 0 }
    fn APIkaraoke_ActivateMicrophone(&mut self, _m: &mut Mem, a0: u32) -> u32 { self.p.lock().unwrap().game.karaoke.SOUND_StartRecording(a0 as u16); 0 }
    fn APIkaraoke_DeactivateMicrophone(&mut self, _m: &mut Mem, a0: u32) -> u32 { self.p.lock().unwrap().game.karaoke.SOUND_StopRecording(a0 as u16); 0 }
    fn APIkaraoke_GetAverageVolume(&mut self, _m: &mut Mem) -> u32 { self.p.lock().unwrap().game.karaoke.average.to_bits() }
    fn APIkaraoke_GetMaxVolume(&mut self, _m: &mut Mem) -> u32 { self.p.lock().unwrap().game.karaoke.maximum.to_bits() }
    fn APIkaraoke_SetTVOutputVolumeFromMicrophone(&mut self, _m: &mut Mem, _a0: u32, a1: f32) -> u32 { self.p.lock().unwrap().game.karaoke.SOUND_SetTVOutputVolume(a1); 0 }
    fn APIkaraoke_GetTVOutputVolumeFromMicrophone(&mut self, _m: &mut Mem, _a0: u32) -> u32 { self.p.lock().unwrap().game.karaoke.tv_output_volume.to_bits() }
    fn APIkaraoke_SetMicrophoneRecordingVolume(&mut self, _m: &mut Mem, _a0: u32, a1: f32) -> u32 { self.p.lock().unwrap().game.karaoke.SOUND_SetMicrophoneRecordingVolume(a1); 0 }
    fn APIkaraoke_GetMicrophoneRecordingVolume(&mut self, _m: &mut Mem, _a0: u32) -> u32 { self.p.lock().unwrap().game.karaoke.mic_recording_volume.to_bits() }
    fn APIkaraoke_SetHeadsetPlaybackVolume(&mut self, _m: &mut Mem, _a0: u32, a1: f32) -> u32 { self.p.lock().unwrap().game.karaoke.SOUND_SetHeadsetPlaybackVolume(a1); 0 }
    fn APIkaraoke_GetHeadsetPlaybackVolume(&mut self, _m: &mut Mem, _a0: u32) -> u32 { self.p.lock().unwrap().game.karaoke.headset_playback_volume.to_bits() }
    fn APILoadSave_EnableReset(&mut self, _m: &mut Mem, a0: u32) -> u32 { self.p.lock().unwrap().game.enable_reset = a0; 0 }
    // ---- WP-S1: APIspline -> EFSplineMgr (spline.rs), level copy in ParticleShared::s1
    fn APIspline_GetNbSplines(&mut self, m: &mut Mem) -> u32 { sp!(self, APIspline_GetNbSplines(m)) }
    fn APIspline_GetIndex(&mut self, m: &mut Mem, a0: u32) -> u32 { sp!(self, APIspline_GetIndex(m, a0)) }
    fn APIspline_GetID(&mut self, m: &mut Mem, a0: i32) -> u16 { sp!(self, APIspline_GetID(m, a0 as u32)) as u16 }
    fn APIspline_GetPosition(&mut self, m: &mut Mem, a0: u32, a1: f32, a2: Ptr) -> u32 { sp!(self, APIspline_GetPosition(m, a0, a1, a2)) }
    fn APIspline_GetSegmentPosition(&mut self, m: &mut Mem, a0: u32, a1: u32, a2: f32, a3: Ptr) -> u32 { sp!(self, APIspline_GetSegmentPosition(m, a0, a1, a2, a3)) }
    fn APIspline_GetNbSegments(&mut self, m: &mut Mem, a0: u32) -> u32 { sp!(self, APIspline_GetNbSegments(m, a0)) }
    fn APIspline_Segment2GlobalFactor(&mut self, m: &mut Mem, a0: u32, a1: u32, a2: f32, a3: Ptr) -> u32 { sp!(self, APIspline_Segment2GlobalFactor(m, a0, a1, a2, a3)) }
    fn APIspline_Global2SegmentFactor(&mut self, m: &mut Mem, a0: u32, a1: f32, a2: Ptr, a3: Ptr) -> u32 { sp!(self, APIspline_Global2SegmentFactor(m, a0, a1, a2, a3)) }
    fn APIspline_GetTangent(&mut self, m: &mut Mem, a0: u32, a1: f32, a2: Ptr) -> u32 { sp!(self, APIspline_GetTangent(m, a0, a1, a2)) }
    fn APIspline_GetLength(&mut self, m: &mut Mem, a0: u32) -> u32 { sp!(self, APIspline_GetLength(m, a0)) }
    fn APIspline_GetDistance(&mut self, m: &mut Mem, a0: u32, a1: u32) -> u32 { sp!(self, APIspline_GetDistance(m, a0, a1)) }
    fn APIspline_GetSegmentLength(&mut self, m: &mut Mem, a0: u32, a1: u32, a2: f32) -> u32 { sp!(self, APIspline_GetSegmentLength(m, a0, a1, a2)) }
    fn APIspline_MoveDistance(&mut self, m: &mut Mem, a0: u32, a1: f32, a2: f32) -> u32 { sp!(self, APIspline_MoveDistance(m, a0, a1, a2)) }
    fn APIspline_GetType(&mut self, m: &mut Mem, a0: u32) -> u16 { sp!(self, APIspline_GetType(m, a0)) as u16 }
    fn APIspline_GetNearest(&mut self, m: &mut Mem, a0: u32, a1: Ptr) -> u32 { sp!(self, APIspline_GetNearest(m, a0, a1)) }
    // ---- WP-S1: APIprojector / APIfbeffects -> host/{projector,fbeffects}.rs (ParticleShared::s1)
    /// WP-S4: the projector is an EFPosData: its RwFrame (+0x14, identity) is a world frame keyed by the handle
    fn APIprojector_CreateProjector(&mut self, _m: &mut Mem) -> u32 {
        let mut g = self.p.lock().unwrap();
        let h = g.s1.projectors.CreateProjector();
        g.world.frames.insert(h, DTMatrix { right: [1., 0., 0.], up: [0., 1., 0.], at: [0., 0., 1.], pos: [0.; 3] });
        h
    }
    fn APIprojector_DestroyProjector(&mut self, _m: &mut Mem, a0: u32) -> u32 {
        let mut g = self.p.lock().unwrap();
        if g.s1.projectors.projectors.contains_key(&a0) {
            g.world.frames.remove(&a0);
            g.parented.retain(|c| c.0 != a0);
        }
        g.s1.projectors.DestroyProjector(a0);
        0
    }
    fn APIprojector_SetTexture(&mut self, m: &mut Mem, a0: u32, a1: Ptr) -> u32 { self.p.lock().unwrap().s1.projectors.SetTexture(m, a0, a1); 0 }
    fn APIprojector_SetBlendingFunction(&mut self, _m: &mut Mem, a0: u32, a1: u32) -> u32 { self.p.lock().unwrap().s1.projectors.with(a0, |p| p.blend = a1); 0 }
    fn APIprojector_SetSize(&mut self, _m: &mut Mem, a0: u32, a1: f32, a2: f32) -> u32 { self.p.lock().unwrap().s1.projectors.with(a0, |p| p.size = [a1, a2]); 0 }
    fn APIprojector_SetColor(&mut self, _m: &mut Mem, a0: u32, a1: f32, a2: f32, a3: f32, a4: f32) -> u32 { self.p.lock().unwrap().s1.projectors.with(a0, |p| p.color = [a1, a2, a3, a4]); 0 }
    fn APIprojector_SetReachDistance(&mut self, _m: &mut Mem, a0: u32, a1: f32) -> u32 { self.p.lock().unwrap().s1.projectors.with(a0, |p| p.reach = a1); 0 }
    fn APIprojector_SetProjectionType(&mut self, _m: &mut Mem, a0: u32, a1: u32) -> u32 { self.p.lock().unwrap().s1.projectors.with(a0, |p| p.projection = a1); 0 }
    fn APIprojector_SetVisible(&mut self, _m: &mut Mem, a0: u32, a1: u32) -> u32 { self.p.lock().unwrap().s1.projectors.with(a0, |p| p.flags = if a1 == 0 { p.flags & !1 } else { p.flags | 1 }); 0 }
    fn APIfbeffects_CreateFB(&mut self, _m: &mut Mem, a0: u32, a1: u32) -> u32 { self.p.lock().unwrap().s1.fb.CreateFB(a0, a1) }
    fn APIfbeffects_DestroyFB(&mut self, _m: &mut Mem, a0: u32) -> u32 { self.p.lock().unwrap().s1.fb.DestroyFB(a0); 0 }
    fn APIfbeffects_EnableFB(&mut self, _m: &mut Mem, a0: u32, a1: u32) -> u32 { self.p.lock().unwrap().s1.fb.EnableFB(a0, a1); 0 }
    fn APIfbeffects_SetParamsFB(&mut self, m: &mut Mem, a0: u32, a1: u32, a2: Ptr) -> u32 { self.s1_SetParamsFB(m, a0, a1, a2) }
    fn APIfbeffects_GetParamsFB(&mut self, m: &mut Mem, a0: u32, a1: u32, a2: Ptr) -> u32 { self.p.lock().unwrap().s1.fb.GetParamsFB(m, a0, a1, a2); 0 }
    /// WP-S1: GetWMat (EFApi_GameObj): RwFrameGetLTM(GetObjectPosData(id)+0x14) -> the LTM
    fn APIgameobj_GetWMat(&mut self, mem: &mut Mem, a0: u32) -> u32 {
        let a0 = self.r(a0);
        let f = bone_frame(&self.p, mem, a0, 0);
        if let Some(m) = self.p.lock().unwrap().world.frames.get(&a0).copied() {
            write_rwmatrix(mem, f + 0x50, &m);
        }
        f + 0x50
    }
    // ---- WP-S1: APIgameobj (host/gameobj.rs)
    fn APIgameobj_SetLogicFlag(&mut self, _m: &mut Mem, a0: u32, a1: u32) -> u32 { self.s1_SetLogicFlag(a0, a1, true) }
    fn APIgameobj_ResetLogicFlag(&mut self, _m: &mut Mem, a0: u32, a1: u32) -> u32 { self.s1_SetLogicFlag(a0, a1, false) }
    fn APIgameobj_EnableClipping(&mut self, _m: &mut Mem, a0: u32, a1: u32, a2: u32) -> u32 { self.s1_EnableClipping(a0, a1, a2) }
    fn APIgameobj_GetClippingParams(&mut self, m: &mut Mem, a0: u32, a1: Ptr) -> u32 { self.s1_GetClippingParams(m, a0, a1) }
    fn APIgameobj_SetClippingParams(&mut self, m: &mut Mem, a0: u32, a1: Ptr, a2: u32) -> u32 { self.s1_SetClippingParams(m, a0, a1, a2) }
    fn APIgameobj_SetShadowParams(&mut self, m: &mut Mem, a0: u32, a1: Ptr) -> u32 { self.s1_SetShadowParams(m, a0, a1) }
    fn APIgameobj_GetShadowParams(&mut self, m: &mut Mem, a0: u32, a1: Ptr) -> u32 { self.s1_GetShadowParams(m, a0, a1) }
    fn APIgameobj_DisplayInfo(&mut self, _m: &mut Mem, _a0: u32, _a1: Ptr, _a2: u32, _a3: u32) -> u32 { 0 }
    fn APIgameobj_EnableObject(&mut self, _m: &mut Mem, a0: u32, a1: u32) -> u32 { self.s1_EnableObject(a0, a1) }
    fn APIgameobj_GetLookUp(&mut self, m: &mut Mem, a0: u32, a1: u32, a2: Ptr) -> Ptr { self.s1_GetLookUp(m, a0, a1, a2) }
    fn APIgameobj_Translate(&mut self, m: &mut Mem, a0: u32, a1: Ptr) -> u32 { self.s1_Translate(m, a0, a1) }
    fn APIgameobj_SetTranslationSafe(&mut self, m: &mut Mem, a0: u32, a1: Ptr) -> u32 { self.s1_SetTranslationSafe(m, a0, a1) }
    fn APIgameobj_Rotate(&mut self, m: &mut Mem, a0: u32, a1: Ptr, a2: f32) -> u32 { self.s1_Rotate(m, a0, a1, a2, 2) }
    fn APIgameobj_SetRotation(&mut self, m: &mut Mem, a0: u32, a1: Ptr, a2: f32) -> u32 { self.s1_Rotate(m, a0, a1, a2, 0) }
    fn APIgameobj_RotateAround(&mut self, m: &mut Mem, a0: u32, a1: Ptr, a2: f32, a3: Ptr) -> u32 { self.s1_RotateAround(m, a0, a1, a2, a3) }
    fn APIgameobj_AlignAtWith(&mut self, m: &mut Mem, a0: u32, a1: Ptr) -> u32 { self.s1_AlignAtWith(m, a0, a1) }
    fn APIgameobj_LookAtAlignUp(&mut self, m: &mut Mem, a0: u32, a1: Ptr, a2: Ptr) -> u32 { self.s1_LookAtAlignUp(m, a0, a1, a2) }
    fn APIgameobj_LookUpAlignAt(&mut self, m: &mut Mem, a0: u32, a1: Ptr, a2: Ptr) -> u32 { self.s1_LookUpAlignAt(m, a0, a1, a2) }
    fn APIgameobj_SetIdentity(&mut self, _m: &mut Mem, a0: u32) -> u32 { self.s1_SetIdentity(a0) }
    fn APIgameobj_SetLMat(&mut self, m: &mut Mem, a0: u32, a1: Ptr) -> u32 { self.s1_SetLMat(m, a0, a1) }
    fn APIgameobj_RemoveAllChildren(&mut self, _m: &mut Mem, a0: u32, a1: u32) -> u32 { self.s1_RemoveAllChildren(a0, a1) }
    fn APIgameobj_GetPhysicBSphereRadius(&mut self, _m: &mut Mem, a0: u32) -> u32 { self.s1_GetPhysicBSphereRadius(a0) }
    fn APIgameobj_GetBoundingSphereRadius(&mut self, _m: &mut Mem, a0: u32) -> u32 { self.s1_GetPhysicBSphereRadius(a0) }
    fn APIgameobj_GetRenderBSphere(&mut self, m: &mut Mem, a0: u32, a1: Ptr, a2: Ptr) -> f32 { self.s1_GetRenderBSphere(m, a0, a1, a2) }
    fn APIgameobj_SetRenderBSphere(&mut self, m: &mut Mem, a0: u32, a1: Ptr, a2: f32) -> u32 { self.s1_SetRenderBSphere(m, a0, a1, a2) }
    fn APIgameobj_SetTransparentPriority(&mut self, _m: &mut Mem, a0: u32, a1: u32, a2: u32) -> u32 { self.s1_SetTransparentPriority(a0, a1, a2) }
    fn APIgameobj_GetTransparentPriority(&mut self, _m: &mut Mem, a0: u32, a1: u32) -> u32 { self.s1_GetTransparentPriority(a0, a1) }
    fn APIgameobj_SetTransparentType(&mut self, _m: &mut Mem, a0: u32, a1: u32, a2: u32) -> u32 { self.s1_SetTransparentType(a0, a1, a2) }
    fn APIgameobj_GetNbJoints(&mut self, _m: &mut Mem, a0: u32) -> u32 { self.s1_GetNbJoints(a0) }
}

// ---------------------------------------------------------------------------------------------
// runtime resource

/// What one EFGraphicsMgr::Render produced: the 3D draw list and the particle/billboard batches
/// (the Particles / Billboards passes of RenderCamera, per camera slot).
#[derive(Debug, Clone, Default)]
pub struct RuntimeFrame {
    pub passes: Vec<RenderPass>,
    pub draws: Vec<crate::engine::gfx::render::DrawCmd>,
    /// (camera slot, batch) in pass order
    pub particles: Vec<(u32, PLDrawBatch)>,
    pub billboards: Vec<(u32, PLDrawBatch)>,
    /// WP-C5: (camera slot, pass, draws) of EFSpriteMgr_BC::Render*
    pub sprites: Vec<(u32, crate::engine::gfx::api::SpritePass, Vec<crate::engine::gfx::sprite::SpriteDraw>)>,
    /// WP-C5: EFMotionTrailMgr_BC::Render strips per camera slot
    pub trails: Vec<(u32, crate::engine::gfx::motion_trail::MTStrip)>,
    /// J7: EFHudMgr_SP::Update draw list (2D prims in pixels of the 640x448 screen, render order)
    pub hud: Vec<crate::engine::hud::HudDrawCmd>,
}

/// The managers (EFEngineController globals _gpoGraphicsMgr, _gpoPLParticleMgr, _gpoSplineMgr,
/// _gpoEFLightMgr, DTWpGrpMgr 0x3441c0) + per-level data.
#[derive(Resource)]
pub struct EngineRuntime {
    pub gfx: EFGraphicsMgr,
    pub particles: SharedParticles,
    pub splines: EFSplineMgr,
    pub waypoints: DTWpGrpMgr,
    pub lights: EFLightMgr_BC,
    pub rand: LibcRand,
    /// EF3dObjRes uid -> its DTEmitterContainerDescriptor (EF3dObjResData +0x18), attached to a
    /// game object by [`EngineRuntime::AttachResEmitters`] when its EF3dObj is created.
    pub res_emitters: HashMap<u32, crate::engine::dt::descriptors::DTEmitterContainerDescriptor>,
    pub logic_map: LogicCodeMap,
    pub report: LevelLogicReport,
    pub objects: Vec<ObjH>,
    pub frame: RuntimeFrame,
    /// WP-C5: this frame's deferred APIgraphics requests (owner managers unported)
    pub frame_gfx_deferred: Vec<crate::engine::gfx::api::GfxDeferred>,
    /// particle notifications delivered this frame (debug / tests)
    pub delivered: Vec<PLNotification>,
    pub frame_count: u32,
    pub level_loaded: bool,
}

/// Camera the runtime creates at level load until the camera logic/descriptors are wired.
pub const SCREEN: (u32, u32) = (640, 448);

impl EngineRuntime {
    pub fn new(particles: SharedParticles, rand: LibcRand) -> Self {
        EngineRuntime {
            gfx: EFGraphicsMgr::new(SCREEN.0, SCREEN.1),
            particles,
            splines: EFSplineMgr::default(),
            waypoints: DTWpGrpMgr::default(),
            lights: EFLightMgr_BC::new(),
            rand,
            res_emitters: HashMap::new(),
            logic_map: LogicCodeMap::default(),
            report: LevelLogicReport::default(),
            objects: Vec::new(),
            frame: RuntimeFrame::default(),
            frame_gfx_deferred: Vec::new(),
            delivered: Vec::new(),
            frame_count: 0,
            level_loaded: false,
        }
    }

    /// Builds an engine whose host shares this runtime's particle state.
    pub fn new_with_engine() -> (Engine, EngineRuntime) {
        let rand = LibcRand::default();
        let p: SharedParticles = Default::default();
        // one libc rand: audio (EFNewAudioMgr::with_rng), particles, logic
        p.lock().unwrap().sub = Subsystems::new(rand.0.clone());
        let e = Engine::new(Box::new(RuntimeHost { p: p.clone() }));
        (e, EngineRuntime::new(p, rand))
    }

    /// Reads the level pack (blocking: call off the frame, e.g. from [`spawn_level_load`]).
    /// J2: the WP-D anim resources and the audio env descriptor are parsed
    /// ([`RuntimeInstanciator`]), and the sound bank files SOUND_BankLoad will open are
    /// pre-read into `side_files` (no I/O in the frame).
    pub fn ReadLevel(fs: &IsoFs, dir: &str, file: &str) -> Result<EFRessourcesMgr, String> {
        let mut m = EFRessourcesMgr::default();
        m.level_file_name = Some(file.to_string());
        let mut inst = RuntimeInstanciator::default();
        let err = m.LoadLevel(fs, "", dir, "EN", &mut inst, |_, _| {}).map_err(|e| format!("{e:?}"))?;
        if err != 0 {
            return Err(format!("LoadLevel error {err}"));
        }
        if let Some(env) = inst.audio.env.lock().unwrap().first() {
            m.side_files.push(("AUDIOENV".into(), env.strings.join("\0").into_bytes()));
        }
        let mut probe = Subsystems::new(Default::default());
        let mut files = Vec::new();
        probe.LoadLevel(&mut m, dir, &mut |p| {
            let b = fs.read(p).ok()?;
            files.push((format!("AUDIO:{p}"), b.clone()));
            Some(b)
        });
        m.side_files.extend(files);
        Ok(m)
    }

    /// EFRessourcesMgr::LoadLevel registration order applied to the pre-loaded pack.
    pub fn LoadLevel(&mut self, e: &mut Engine, mut res: EFRessourcesMgr) {
        // EFWorld::Terminate (0x0026e760): DTWpGrpMgr::RemoveAllWaypointGrp, PLMgr_BC::Terminate,
        // EFLightMgr_BC::Terminate
        self.waypoints.RemoveAllWaypointGrp();
        self.lights.Terminate();
        let mut g = self.particles.lock().unwrap();
        g.pl.Terminate();
        // WP-C5 step 6: EFSpriteMgr_BC::Terminate + EFMotionTrailMgr_BC::Terminate at level unload
        g.gfxapi.Terminate();
        // J7: EFHudMgr_BC::Terminate (windows, fonts, palettes, text of the level)
        g.hud.Terminate();
        g.hud3d.clear();
        g.pl.load_count = 0;
        g.world = RuntimeWorld::default();

        // 3. LoadLevelTextureDictionnary(0,-1): TEXDIC_0.TXD, TEXDIC_1.TXD ... in order
        EFTextureDict::Clear(&mut self.gfx.tex_dict);
        let mut txd: Vec<&(String, Vec<u8>)> = res.side_files.iter().filter(|(p, _)| p.to_uppercase().contains("TEXDIC_")).collect();
        txd.sort_by(|a, b| a.0.cmp(&b.0));
        for (_, b) in txd {
            EFTextureDict::LoadPropertiesBinary(b, &mut self.gfx.tex_dict);
        }

        // 8. pack entries in FAT order
        for o in &res.objects {
            let Some(raw) = o.as_raw() else { continue };
            match raw.class_id {
                0x22 => {
                    // EFTextureDict inside the pack (LoadLevel reads it through a memory stream):
                    // UID word then the image chunks
                    if raw.data.len() > 4 {
                        EFTextureDict::LoadPropertiesBinary(&raw.data[4..], &mut self.gfx.tex_dict);
                    }
                }
                0x29 | 0x25 => {
                    // PLMgr_BC::Instanciate returns the manager: its LoadFromStream (call counter)
                    g.pl.textures = Some(self.gfx.tex_dict.textures.iter().map(|t| t.name.clone()).collect());
                    let mut s = DTMemoryPersistStream::new(raw.data.clone());
                    g.pl.LoadFromStream(&mut s);
                }
                0x18 => {
                    let mut st = EFStatic3dObj::default();
                    if st.LoadPropertiesBinary(&raw.data).is_ok() {
                        for l in &st.lights {
                            self.lights.CreateStaticLight(l.uid, l.light.light_type);
                        }
                    }
                }
                0x5b => {
                    let mut r = EF3dObjRes::default();
                    if r.LoadPropertiesBinary(&raw.data).is_ok() {
                        if let Some(em) = r.emitters.take() {
                            self.res_emitters.insert(r.res.uid, em);
                        }
                    }
                }
                _ => {}
            }
        }
        if g.pl.textures.is_none() {
            g.pl.textures = Some(self.gfx.tex_dict.textures.iter().map(|t| t.name.clone()).collect());
        }
        // WP-C5 step 1: the current RwTexDictionary names for SpriteSetGroupTexture
        g.gfxapi.textures = self.gfx.tex_dict.textures.iter().map(|t| t.name.clone()).collect();
        // J7: HUD level data (FONTn.FT / *.PAL / *.TX / LOAD.LS side files) + root window;
        // texture sizes stand in for the RwRaster queries of 2D windows (flags 0x30000).
        g.hud = crate::engine::hud::EFHudMgr::new();
        g.hud.Init();
        if let Err(e) = g.hud.LoadSideFiles(&res.side_files) {
            log::warn!("hud side files: {e:?}");
        }
        g.hud.texture_dims = self.gfx.tex_dict.textures.iter().map(|t| (t.name.clone(), (t.width, t.height))).collect();
        drop(g);
        self.waypoints.LoadFromLevel(&mut res);
        self.particles.lock().unwrap().game.waypoints = self.waypoints.clone(); // WP-S3: APIextra Waypoint*
        self.splines.LoadFromLevel(&res);
        self.particles.lock().unwrap().s1.splines = self.splines.clone(); // WP-S1: APIspline_*

        // object frames from the EF_GLO descriptors (EFGameObject::LoadFromStream frame setup)
        {
            let mut g = self.particles.lock().unwrap();
            for go in &res.game_objects {
                let d = &go.glo;
                g.world.frames.insert(go.uid, DTMatrix { right: d.look_right, up: d.look_up, at: d.look_at, pos: d.pos });
            }
            // J2: EFGameObject::CreateInstances (anims, sounds), collision maps, audio env +
            // banks (pre-read by ReadLevel)
            // J19: the level directory is the whole pack stem ("EP1L03A.P2R" -> "EP1/L03A"); the
            // former f[3..6] cut gave "EP1/L03" and lost the EP1L03A level + stream sound banks
            let dir = res.level_file_name.as_deref().map(|f| f.split('.').next().unwrap_or(f)).filter(|f| f.len() >= 6).map(|f| format!("{}/{}", &f[0..3], &f[3..])).unwrap_or_default();
            let side: HashMap<String, Vec<u8>> = res.side_files.iter().filter_map(|(p, b)| p.strip_prefix("AUDIO:").map(|p| (p.to_string(), b.clone()))).collect();
            g.sub.LoadLevel(&mut res, &dir, &mut |p| side.get(p).cloned());
        }
        // R8: EFGameObject::LoadFromStream -> InitHotspot (0x00200880) runs at object load:
        // CreateFrame + EFPhysicData::CreatePhysic(this, 0) + the hot-spot prim, for every hot spot,
        // whether or not its logic ever calls APIphysic_CreatePhysic (hs_commun, hs_changelevel never
        // do -> before, no physic and their COLLIDE_OBJ callbacks never ran). Pack order; a later
        // logic CreatePhysic returns this physic ("only once").
        let hotspots: Vec<u32> = {
            let g = self.particles.lock().unwrap();
            res.game_objects.iter().map(|go| go.uid).filter(|u| g.sub.hotspot.contains_key(u)).collect()
        };
        for uid in hotspots {
            J2(&self.particles).physic_create(uid, 0);
        }

        // the trailing object container: logic codes and game objects
        self.logic_map = LogicCodeMap::default();
        self.report = LevelLogicReport::default();
        self.objects = LinkLevelLogic(e, &res, &mut self.logic_map, &mut self.report);

        // J2 camera: no logic module calls APIcamera_CreateCamera; the game camera is the GLO
        // carrying the DT_CAMERA_ANIM_DESCR (0x55) descriptors (EP1L01: 0x2a00008f, logic
        // 0x8a3, whose code is not a ported module). Its CreateCamera(obj) + Activate(1) is
        // done here with the object's frame; the camera shares the object's RwFrame.
        if self.gfx.cameras.count == 0 {
            let cam_obj = res.game_objects.iter().find(|go| go.glo.object_descriptors.objects.iter().any(|d| d.GetClassID() == 0x55)).map(|go| go.uid);
            let frames = self.particles.lock().unwrap().world.frames.clone();
            let (obj, f) = match cam_obj {
                Some(o) => (o, frames.get(&o).copied()),
                None => (0, Some(DTMatrix { right: [1., 0., 0.], up: [0., 1., 0.], at: [0., 0., 1.], pos: [0.; 3] })),
            };
            let id = self.gfx.cameras.APIcamera_CreateCamera(&mut e.mem, obj, f);
            self.gfx.cameras.APIcamera_Activate(&mut e.mem, id, 1);
        }
        self.level_loaded = true;
    }

    /// PLMgr_BC::LoadEmitters(container of the EF3dObjRes, object) — EF3dObj creation with a
    /// resource that carries emitters (EF3dObjResData +0x18).
    pub fn AttachResEmitters(&mut self, obj_uid: u32, res_uid: u32) -> i32 {
        let Some(d) = self.res_emitters.get(&res_uid) else { return 0 };
        let mut g = self.particles.lock().unwrap();
        let ParticleShared { pl, world, .. } = &mut *g;
        pl.LoadEmitters(d, Some((obj_uid, &*world as &dyn PLWorld)))
    }

    // ---- frame steps (each is one Bevy system below) ----

    /// MainLoop input slot (before EFGameLogicMgr::Update): EFPlayerDeviceMgr::Update (devices +
    /// EFInputMgr), then the EFInputContext callbacks in call order: the ActionState goes to the
    /// InputNotify block and ExecLogicCallback({obj, cb}, &InputNotify) runs the logic callback.
    /// The physics analog actions (EFDMAnalogMove) read the same EFInputAction states.
    pub fn StepInput(&mut self, e: &mut Engine, raw_dt: f32) {
        let (notes, errs) = {
            let mut g = self.particles.lock().unwrap();
            g.devices.StartFrame();
            g.devices.Update(raw_dt);
            let ParticleShared { devices, sub, .. } = &mut *g;
            for c in &devices.input.configs {
                for a in &c.actions {
                    sub.input.actions.insert((c.id as u32, a.id as u32), [a.state.x, a.state.y]);
                }
            }
            (std::mem::take(&mut devices.notifies), std::mem::take(&mut devices.error_notifies))
        };
        self.particles.lock().unwrap().logic = Some(&e.rt as *const _);
        let block = e.mem.malloc(12);
        for n in notes {
            let Some(h) = e.rt.map.find(n.obj_uid) else { continue };
            e.mem.w32(block, n.state.x.to_bits());
            e.mem.w32(block + 4, n.state.y.to_bits());
            e.mem.w32(block + 8, n.state.flags);
            e.ExecLogicCallback(crate::logic::notify::LogicCallback { obj: h, cb: n.callback }, block);
        }
        for n in errs {
            let Some(h) = e.rt.map.find(n.obj_uid) else { continue };
            e.ExecLogicCallback(crate::logic::notify::LogicCallback { obj: h, cb: n.callback }, n.device_id as u32);
        }
        e.mem.free(block);
        self.particles.lock().unwrap().logic = None;
    }

    /// EFGameLogicMgr::Update (with ComputeDeltaTime). J2: the camera slots and the light
    /// manager are handed to the host while the logic runs (APIcamera_*/APIlight_*).
    pub fn StepLogic(&mut self, e: &mut Engine, raw_dt: f32) -> f32 {
        let dt = e.clock.ComputeDeltaTime(raw_dt);
        self.SwapLogicState();
        self.particles.lock().unwrap().logic = Some(&e.rt as *const _);
        e.GameLogicUpdate(dt);
        self.particles.lock().unwrap().logic = None;
        self.SwapLogicState();
        dt
    }

    fn SwapLogicState(&mut self) {
        let mut g = self.particles.lock().unwrap();
        std::mem::swap(&mut g.sub.cameras, &mut self.gfx.cameras);
        std::mem::swap(&mut g.sub.lights, &mut self.lights);
    }

    /// EFWorld::Update 0x0026eb60 up to the particles: animation (EFAnimationMgr::Update per
    /// object, Notify 0x11 queued deferred), FlushNotification, TexFader, EFPhysicMgr::Update
    /// (2 passes; physic notifications), joint frames, SyncFrames + PLMgr_BC::Update, then the
    /// particle notifications.
    pub fn StepParticles(&mut self, e: &mut Engine, dt: f32) {
        self.particles.lock().unwrap().logic = Some(&e.rt as *const _);
        // animation
        let anim_notes = {
            let mut g = self.particles.lock().unwrap();
            let paused = g.world.paused;
            // J16: EF3dObj collision context for EFHAnimNode::UpdateAnimPrimColl (prim kinds of the
            // EFPhysic model + the clump prim records), per bound object
            let ctxs: Vec<(u32, u32, crate::engine::anim::coll_prim::CollCtx)> = g
                .sub
                .anim_bind
                .iter()
                .filter_map(|(&uid, b)| {
                    let p = g.sub.physic.Physic(g.sub.physic.PhysicOf(uid))?;
                    let kinds = p.model.prims.iter().map(|q| q.as_ref().map_or(u32::MAX, |q| q.base.kind)).collect();
                    let defaults = g.sub.obj3d.get(&uid).and_then(|(res, _)| g.sub.res_collision.get(res)).map(|c| c.primitives.clone()).unwrap_or_default();
                    Some((uid, b.obj3d, crate::engine::anim::coll_prim::CollCtx { kinds, defaults }))
                })
                .collect();
            for (uid, o, c) in ctxs {
                if let Some(m) = g.sub.anim.get_mut(&uid) {
                    let same = m.coll_ctx.get(&o).is_some_and(|x| x.kinds == c.kinds && x.defaults.len() == c.defaults.len());
                    if !same {
                        m.coll_ctx.insert(o, std::sync::Arc::new(c));
                    }
                }
            }
            let notes = g.sub.UpdateAnims(dt, paused);
            // EFAnimNode::SetCollPrimData (J16): the root anim node's collision-prim chain onto the
            // object's EFPhysic collision model
            let colls: Vec<(u32, crate::engine::anim::coll_blend::CollPrimList)> = g
                .sub
                .anim_bind
                .iter()
                .filter_map(|(&uid, b)| Some((uid, g.sub.anim.get(&uid)?.GetCollPrims(b.obj3d)?)))
                .collect();
            for (uid, l) in colls {
                let ltm = g.world.frames.get(&uid).map(|f| crate::engine::physic::mgr::rows(&j2::dt_to_rw(f)));
                let col = g.sub.obj3d.get(&uid).and_then(|(res, _)| g.sub.res_collision.get(res)).cloned();
                let id = g.sub.physic.PhysicOf(uid);
                if let (Some(p), Some(ltm)) = (g.sub.physic.PhysicMut(id), ltm) {
                    crate::engine::physic::create_prims::SetCollPrimData(&mut p.model, &l, col.as_ref(), ltm);
                }
            }
            // EFWorld::Update: EF3dObj::UpdateHierarchy after each EFAnimationMgr::Update (J16).
            // Root transfer: when transfo[0] of the EF3dObj pool has state & 8 (EFHAnimNode::UpdateHAnim,
            // bone 0 of an anim whose DTAnimHdr byte +0xf == 1; blend outputs never carry it), the
            // root transfo becomes the modelling matrix of the game object's frame (EFGameObject
            // +0x14, the parent of the EF3dObj frame) and is reset (the renderer skips node 0).
            // quirk: the game object frame is world-parented, so the anim root is absolute (cut-scene
            // actors, the fail set at y~1000).
            let roots: Vec<(u32, DTMatrix)> = g
                .sub
                .anim_bind
                .iter()
                .filter_map(|(&uid, b)| {
                    let pose = g.sub.anim.get(&uid)?.GetPose(b.obj3d)?;
                    let t = pose.iter().find(|t| t.id == 0 && t.state & 8 != 0)?;
                    let m = crate::engine::gfx::hierarchy::GetMatrix(t);
                    Some((uid, DTMatrix { right: [m[0], m[1], m[2]], up: [m[4], m[5], m[6]], at: [m[8], m[9], m[10]], pos: [m[12], m[13], m[14]] }))
                })
                .collect();
            for (uid, m) in roots {
                if let Some(f) = g.world.frames.get_mut(&uid) {
                    *f = m;
                }
            }
            notes
        };
        for (msg, owner) in anim_notes {
            // EFNotificationMgr::Notify(gameLogic, 0x11, owner, owner, 0, 1)
            e.Notify(msg, owner, owner, 0, true);
            self.particles.lock().unwrap().sub.log.notifications.push((msg, owner));
        }
        // J3: EFAnimNode::Notify -> EFNotificationMgr::Notify(gameLogic, code, obj, user|obj, arg, 1)
        // (code 1 = last frame reached / loop, 0 = stopped, 0xb = anim callback {user, kind, index}).
        // Sound events: EFNewSoundMgr::PlayAnimationSound.
        let anim_events = std::mem::take(&mut self.particles.lock().unwrap().sub.anim_events);
        let mut payloads = Vec::new();
        for (obj, ev) in anim_events {
            use crate::engine::anim::nodes::AnimEvent;
            let (code, owner, p) = match ev {
                // EFHAnimNode::Update / EFAnimationMgr: EFAnimNode::Notify(this, 1, this+0x20) -> the
                // last-frame callback (ANIMATION_LAST_FRAME) receives the anim id as its argument
                AnimEvent::Notify { code, user, arg } => (code, user, arg),
                AnimEvent::Callback { user, kind, index } => {
                    let p = e.mem.malloc(0xc);
                    e.mem.w32(p, user);
                    e.mem.w32(p + 4, kind as u32);
                    e.mem.w32(p + 8, index);
                    payloads.push(p);
                    (0xb, obj, p)
                }
                AnimEvent::Sound { sound_id } => {
                    J2(&self.particles).sound_play(obj, sound_id, 0);
                    continue;
                }
            };
            e.Notify(code, obj, owner, p, true);
            self.particles.lock().unwrap().sub.log.notifications.push((code, obj));
        }
        // deferred payloads are freed by FlushNotification (EFNotificationMgr owns them)
        let _ = payloads;
        e.FlushNotification();
        // TexFader
        self.gfx.tex_fader.Update(dt);
        // physics
        let cam = self.gfx.cameras.cameras.iter().flatten().next().map(|c| c.ltm).unwrap_or(DTMatrix { right: [1., 0., 0.], up: [0., 1., 0.], at: [0., 0., 1.], pos: [0.; 3] });
        let phys_notes = {
            let mut g = self.particles.lock().unwrap();
            let ParticleShared { world, sub, .. } = &mut *g;
            let n = sub.UpdatePhysics(dt, world, crate::engine::physic::rwm::RwMatrix { right: cam.right, up: cam.up, at: cam.at, pos: cam.pos }, &self.splines.splines);
            sub.UpdateBones(world);
            place_parented(&mut g);
            n
        };
        let mut phys_notes = phys_notes.into_iter().peekable();
        while let Some(n) = phys_notes.next() {
            // EFDMEdgeDetect (0x19 module): RegisterNotify(0x18, owner, owner, ExecLogicCallback,
            // {GetGameObject(owner), user}) / Notify(0x18, owner, {tag, count, &edges[0x24 each]})
            if n.code == 0x118 {
                if std::env::var_os("GAME_DEBUG").is_some() {
                    eprintln!("runtime: edge register {:#x} cb {:#x}", n.owner, n.data[0]);
                }
                if let Some(h) = e.get_game_object(n.owner, 0) {
                    e.NotificationMgr_RegisterNotify(0x18, n.owner, n.owner, crate::logic::notify::LogicCallback { obj: h, cb: n.data[0] });
                }
                continue;
            }
            if n.code == 0x18 {
                let cnt = n.data[1];
                let edges = e.mem.malloc(0x24 * cnt.max(1));
                for i in 0..cnt * 3 {
                    let Some(w) = phys_notes.next_if(|m| m.code == 0x1018) else { break };
                    for (k, v) in w.data[..3].iter().enumerate() {
                        e.mem.w32(edges + 12 * i + 4 * k as u32, *v);
                    }
                }
                let p = e.mem.malloc(12);
                e.mem.w32(p, n.data[0]);
                e.mem.w32(p + 4, cnt);
                e.mem.w32(p + 8, edges);
                if std::env::var_os("GAME_DEBUG").is_some() {
                    eprintln!("runtime: edge notify {:#x} tag {:#x} count {cnt}", n.owner, n.data[0]);
                }
                e.Notify(0x18, n.owner, n.owner, p, false);
                e.mem.free(p);
                e.mem.free(edges);
                continue;
            }
            // EFDMPlugin (0x25) Pre/PostColUpdate: notify 0x12/0x13 -> EFGameLogicMgr::ExecLogicCallback
            // ({GetGameObject(owner), cb}, &dt). WP-J21: these went through Notify with no registered
            // handler, so e.g. TomeDoomCB_PHYSIC (book on the hero) never ran. Runs after the physics
            // update of all objects instead of inside it (the callbacks only Translate their owner).
            if n.code == 0x12 || n.code == 0x13 {
                if let Some(h) = e.get_game_object(n.owner, 0) {
                    let p = e.mem.malloc(4);
                    e.mem.w32(p, n.data[1]);
                    e.ExecLogicCallback(crate::logic::notify::LogicCallback { obj: h, cb: n.data[0] }, p);
                    e.mem.free(p);
                }
                continue;
            }
            // EFPhysic state-change notification: {handle/state words} to the owner's logic
            let p = e.mem.malloc(16);
            for (i, w) in n.data.iter().enumerate() {
                e.mem.w32(p + 4 * i as u32, *w);
            }
            e.Notify(n.code, n.owner, n.owner, p, false);
            e.mem.free(p);
            self.particles.lock().unwrap().sub.log.notifications.push((n.code, n.owner));
        }
        // J3: EFCollisionSystem CheckNotify -> EFInstance::Notify(target, 0x17 / 3, &CollisionNotifyMsg)
        let col_notes = std::mem::take(&mut self.particles.lock().unwrap().sub.col_notify);
        for (uid, code, m) in col_notes {
            if std::env::var("GAME_COLDBG").is_ok_and(|v| u32::from_str_radix(&v, 16) == Ok(uid)) {
                eprintln!("runtime: col notify {uid:#x} code {code:#x} hit {:#x} tag {:#x} other {:#x}/{:#x}", m.hit, m.tag, m.other_uid, m.other_tag);
            }
            let p = e.mem.malloc(0x38);
            let words = [m.hit, m.tag, m.other_uid, m.other_tag, m.user[0], m.user[1], m.user[2], m.user[3]];
            for (i, w) in words.iter().chain(m.p0.iter().chain(m.p1.iter()).map(|f| f.to_bits()).collect::<Vec<_>>().iter()).enumerate() {
                e.mem.w32(p + 4 * i as u32, *w);
            }
            e.Notify(code, uid, uid, p, false);
            e.mem.free(p);
            self.particles.lock().unwrap().sub.log.notifications.push((code, uid));
        }
        // J12: EFWorld::ClipGameObjects (0x0026f930) proximity part: ClipProximityTest (0x00270030)
        // and EFInstance::Notify(obj, 10, {target, inside}) when params+0xc & 0x10000000 and the
        // result changed (or +0xcc bit 8 = first test). Deviation: only the proximity part of
        // ClipObjects is ported (frustum/distance clipping unported; the `uVar13 |= params+0xc &
        // ~0x10000000` clip-flag merge has no consumer here).
        let prox = {
            let mut g = self.particles.lock().unwrap();
            let ParticleShared { proximity, world, .. } = &mut *g;
            let mut out = Vec::new();
            let mut keys: Vec<u32> = proximity.keys().copied().collect();
            keys.sort_unstable();
            for uid in keys {
                let (w, inside, cc) = proximity.get_mut(&uid).unwrap();
                if !world.frames.contains_key(&uid) {
                    continue;
                }
                let mut t = 0;
                if *cc & 2 != 0 {
                    t = *inside;
                    if let (Some(a), Some(b)) = (world.frames.get(&w[0]), world.frames.get(&uid)) {
                        let d = [a.pos[0] - b.pos[0], a.pos[1] - b.pos[1], a.pos[2] - b.pos[2]];
                        let d2 = d[2] * d[2] + d[0] * d[0] + d[1] * d[1];
                        let (r, h) = (f32::from_bits(w[1]), f32::from_bits(w[2]));
                        if *inside == 0 {
                            if d2 <= (r - h) * (r - h) {
                                t = 1;
                            }
                        } else if (r + h) * (r + h) < d2 {
                            t = 0;
                        }
                    }
                }
                if w[3] & 0x1000_0000 != 0 && (*cc & 8 != 0 || *inside != t) {
                    out.push((uid, w[0], t));
                }
                if *cc & 2 != 0 {
                    *inside = t;
                }
                *cc &= 0xffff_fff3;
            }
            out
        };
        for (uid, target, t) in prox {
            let p = e.mem.malloc(8);
            e.mem.w32(p, target);
            e.mem.w32(p + 4, t);
            e.Notify(10, uid, uid, p, false);
            e.mem.free(p);
            self.particles.lock().unwrap().sub.log.notifications.push((10, uid));
        }
        let notes = {
            let mut g = self.particles.lock().unwrap();
            let ParticleShared { pl, world, .. } = &mut *g;
            pl.SyncFrames(&*world);
            let paused = world.paused;
            // shared libc rand: the pool draws from the global sequence
            let mut r = self.rand.0.lock().unwrap();
            pl.pool.rng.0 = r.0;
            pl.Update(dt, paused, &mut self.gfx.cameras);
            r.0 = pl.pool.rng.0;
            std::mem::take(&mut pl.notifications)
        };
        self.DrainNotifications(e, notes);
        self.particles.lock().unwrap().logic = None;
    }

    /// MainLoop audio slot (after Render, vtbl 0x40): EFNewAudioMgr update with 3D geometry from
    /// the object frames and camera 0.
    pub fn StepAudio(&mut self, dt: f32) {
        let cam = self.gfx.cameras.cameras.iter().flatten().next().map(|c| c.ltm);
        let mut g = self.particles.lock().unwrap();
        let ParticleShared { world, sub, .. } = &mut *g;
        sub.UpdateAudio(dt, world, cam);
    }

    /// EFNotificationMgr::Notify(_gpoGameLogicMgr+0x14, 0x1a, handler, emitterId, {clipped, id}, 0)
    /// as raised by PLEmitter_BC::UpdateClippingState: immediate delivery (last arg 0).
    pub fn DrainNotifications(&mut self, e: &mut Engine, notes: Vec<PLNotification>) {
        for n in notes {
            let p = e.mem.malloc(8);
            e.mem.w32(p, n.clipped);
            e.mem.w32(p + 4, n.emitter);
            e.Notify(n.msg, n.handler, n.sender, p, false);
            e.mem.free(p);
            self.delivered.push(n);
        }
    }

    /// WP-C5 steps 2/3 (EFWorld::Update tail): EFMotionTrailMgr_BC::Update(dt) on the updated
    /// frames, then the deferred APIgraphics requests (tex fader / palette / blend colour / lines,
    /// whose owner managers are unported) are drained into `frame_gfx_deferred`.
    pub fn StepGfxApi(&mut self, dt: f32) {
        let mut g = self.particles.lock().unwrap();
        let ParticleShared { gfxapi, world, .. } = &mut *g;
        gfxapi.trails.Update(dt, world);
        self.frame_gfx_deferred = std::mem::take(&mut gfxapi.deferred);
    }

    /// End of EFWorld::Update: kill list + PrepareRender (logic part).
    pub fn StepFinishWorld(e: &mut Engine) {
        e.ProcessKillList();
        e.PrepareRender_Logic(None);
    }

    /// Camera RwFrame shared with its owning game object: refresh the LTM from the owner.
    pub fn SyncCameras(&mut self) {
        let g = self.particles.lock().unwrap();
        for c in self.gfx.cameras.cameras.iter_mut().flatten() {
            if let Some(f) = g.world.frames.get(&c.frame_owner) {
                c.ltm = *f;
            }
        }
    }

    /// EFGraphicsMgr::Render with the particle/billboard passes fed by PLMgr_BC::Render and
    /// EFBillboardMgr.
    pub fn StepRender(&mut self, e: &Engine, dt: f32) {
        let mut seam = RuntimeSeam { p: &self.particles, mem: &e.mem, out: RuntimeFrame::default(), slot: 0 };
        self.gfx.Render(dt, &mut seam);
        let mut out = seam.out;
        out.passes = self.gfx.passes.clone();
        out.draws = std::mem::take(&mut self.gfx.draws);
        self.frame = out;
        self.frame_count += 1;
    }

    /// One whole frame in MainLoop order (for headless use / tests). The Bevy plugin runs the
    /// same steps as separate systems.
    pub fn Frame(&mut self, e: &mut Engine, raw_dt: f32) {
        self.StepInput(e, raw_dt);
        let dt = self.StepLogic(e, raw_dt);
        self.StepParticles(e, dt);
        self.StepGfxApi(dt);
        Self::StepFinishWorld(e);
        self.SyncCameras();
        self.StepRender(e, dt);
        self.StepAudio(dt);
    }
}

/// RenderSeam of the runtime: Particles -> PLMgr_BC::Render, Billboards -> EFBillboardMgr.
pub struct RuntimeSeam<'a> {
    pub p: &'a SharedParticles,
    pub mem: &'a Mem,
    pub out: RuntimeFrame,
    pub slot: u32,
}

impl RenderSeam for RuntimeSeam<'_> {
    fn pass(&mut self, _mgr: &mut EFGraphicsMgr, p: RenderPass) {
        match p {
            RenderPass::CameraBegin(s) => self.slot = s,
            RenderPass::Particles => {
                let dl = self.p.lock().unwrap().pl.Render();
                self.out.particles.extend(dl.batches.into_iter().map(|b| (self.slot, b)));
            }
            RenderPass::SpritesNormal | RenderPass::SpritesTransparent | RenderPass::SpritesOccluded => {
                use crate::engine::gfx::api::SpritePass;
                let pass = match p {
                    RenderPass::SpritesNormal => SpritePass::Normal,
                    RenderPass::SpritesTransparent => SpritePass::Transparent,
                    _ => SpritePass::Occluded,
                };
                let Some(Some(c)) = _mgr.cameras.cameras.get(self.slot as usize) else { return };
                let (n, f) = (c.rw.near, c.rw.far);
                // RwCamera z mapping replacement: z = shift + scale / z_cam, near -> 0, far -> 1
                let scale = n * f / (n - f);
                let cam = crate::engine::gfx::sprite::SpriteCamera { ltm: c.ltm, view_window: c.rw.view_window, near: n, far: f, width: SCREEN.0 as i32, height: SCREEN.1 as i32, z_scale: scale, z_shift: -scale / n };
                let g = self.p.lock().unwrap();
                let d = g.gfxapi.RenderSprites(pass, &g.world, &cam);
                if !d.is_empty() {
                    self.out.sprites.push((self.slot, pass, d));
                }
            }
            // J7: EFGraphicsMgr::Render -> _gpoHudMgr->Update(camera, 0.033) (EFHudMgr_SP::Update:
            // palettes, window Update + Render). Live text args read from the logic Mem.
            RenderPass::HudUpdate(dt) => {
                let mut g = self.p.lock().unwrap();
                g.hud.Update(dt, SCREEN.0, SCREEN.1, self.mem);
                self.out.hud = g.hud.draw.clone();
            }
            RenderPass::MotionTrails => {
                let t = self.p.lock().unwrap().gfxapi.trails.Render();
                self.out.trails.extend(t.into_iter().map(|t| (self.slot, t)));
            }
            RenderPass::Billboards => {
                let b = self.p.lock().unwrap().bb.Render(self.mem);
                self.out.billboards.extend(b.into_iter().map(|b| (self.slot, b)));
            }
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Bevy

/// Phases of the runtime frame (chained, in `Update`).
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RuntimeSet {
    /// level streaming completion (no blocking I/O: the pack is read on a worker thread)
    Load,
    Logic,
    /// EFWorld::Update: animation/physics (other WPs) and particles
    World,
    FinishWorld,
    CameraSync,
    Render,
}

/// Fixed raw frame time; when absent, bevy_time's `Time` is used.
#[derive(Resource, Clone, Copy)]
pub struct RuntimeFixedDt(pub f32);

/// This frame's filtered dt (EFEngineController +0x2b8).
#[derive(Resource, Clone, Copy, Default)]
pub struct RuntimeDt(pub f32);

/// A level being read on a worker thread.
#[derive(Resource)]
pub struct PendingLevel(pub Mutex<Receiver<Result<EFRessourcesMgr, String>>>);

/// Reads `LEVEL/<dir>/GEN/<file>` from the ISO on a worker thread; insert the result as a
/// resource and [`runtime_poll_level`] applies it when ready.
pub fn spawn_level_load(iso: String, dir: String, file: String) -> PendingLevel {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let r = IsoFs::open(&iso).map_err(|e| format!("{e:?}")).and_then(|fs| EngineRuntime::ReadLevel(&fs, &dir, &file));
        let _ = tx.send(r);
    });
    PendingLevel(Mutex::new(rx))
}

/// Adds Engine (with the runtime host), EngineRuntime, LibcRand and the frame systems. Do not
/// add `ScoobyEnginePlugin` too: this plugin runs the logic update itself.
pub struct EngineRuntimePlugin;

impl Plugin for EngineRuntimePlugin {
    fn build(&self, app: &mut App) {
        let (e, rt) = EngineRuntime::new_with_engine();
        app.insert_resource(rt.rand.clone())
            .insert_resource(e)
            .insert_resource(rt)
            .init_resource::<RuntimeDt>()
            .configure_sets(
                Update,
                (RuntimeSet::Load, RuntimeSet::Logic, RuntimeSet::World, RuntimeSet::FinishWorld, RuntimeSet::CameraSync, RuntimeSet::Render).chain(),
            )
            .add_systems(Update, runtime_poll_level.in_set(RuntimeSet::Load))
            .add_systems(Update, runtime_logic.in_set(RuntimeSet::Logic))
            .add_systems(Update, runtime_particles.in_set(RuntimeSet::World))
            .add_systems(Update, runtime_finish_world.in_set(RuntimeSet::FinishWorld))
            .add_systems(Update, runtime_camera_sync.in_set(RuntimeSet::CameraSync))
            .add_systems(Update, runtime_render.in_set(RuntimeSet::Render));
    }
}

pub fn runtime_poll_level(mut commands: Commands, pending: Option<Res<PendingLevel>>, mut e: ResMut<Engine>, mut rt: ResMut<EngineRuntime>) {
    let Some(p) = pending else { return };
    let r = p.0.lock().unwrap().try_recv();
    match r {
        Ok(Ok(res)) => {
            rt.LoadLevel(&mut e, res);
            commands.remove_resource::<PendingLevel>();
        }
        Ok(Err(msg)) => {
            log::error!("level load failed: {msg}");
            commands.remove_resource::<PendingLevel>();
        }
        Err(TryRecvError::Empty) => {}
        Err(TryRecvError::Disconnected) => commands.remove_resource::<PendingLevel>(),
    }
}

fn level_running(rt: &EngineRuntime) -> bool {
    rt.level_loaded
}

pub fn runtime_logic(mut e: ResMut<Engine>, mut rt: ResMut<EngineRuntime>, mut dt: ResMut<RuntimeDt>, fixed: Option<Res<RuntimeFixedDt>>, time: Option<Res<bevy_time::Time>>) {
    if !level_running(&rt) {
        return;
    }
    let raw = fixed.map(|f| f.0).or_else(|| time.map(|t| t.delta_secs())).unwrap_or(0.0);
    rt.StepInput(&mut e, raw);
    dt.0 = rt.StepLogic(&mut e, raw);
}

pub fn runtime_particles(mut e: ResMut<Engine>, mut rt: ResMut<EngineRuntime>, dt: Res<RuntimeDt>) {
    if level_running(&rt) {
        rt.StepParticles(&mut e, dt.0);
    }
}

pub fn runtime_finish_world(mut e: ResMut<Engine>, rt: Res<EngineRuntime>) {
    if level_running(&rt) {
        EngineRuntime::StepFinishWorld(&mut e);
    }
}

pub fn runtime_camera_sync(mut rt: ResMut<EngineRuntime>) {
    if level_running(&rt) {
        rt.SyncCameras();
    }
}

pub fn runtime_render(e: Res<Engine>, mut rt: ResMut<EngineRuntime>, dt: Res<RuntimeDt>) {
    if level_running(&rt) {
        rt.StepRender(&e, dt.0);
        rt.StepAudio(dt.0);
    }
}
fn rv3(mem: &Mem, a: Ptr) -> [f32; 3] {
    std::array::from_fn(|i| f32::from_bits(mem.r32(a + 4 * i as u32)))
}

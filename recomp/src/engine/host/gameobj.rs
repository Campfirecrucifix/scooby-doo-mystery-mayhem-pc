//! WP-S1: APIgameobj host functions (Engine API (module interface)/EFApi_GameObj.cpp) that the
//! real runtime still answered with the Host default stub. runtime.rs keeps one-line delegations
//! (`self.s1_*`) into the inherent `impl RuntimeHost` below.
//!
//! Memory model: the object frames are the runtime's `RuntimeWorld::frames` (the EFPosData+0x14
//! RwFrame modelling matrix == LTM for root frames, as everywhere in the runtime); the per object
//! EFGameObject fields the logic reads back through these APIs (clipping params +0xd0, shadow
//! params +0x17c, 3D-object transparency +0x58/+0x2c, enabled list) live in [`S1State`]
//! (`ParticleShared::s1`). +0xcc clip flags are shared with the J12 proximity tuple
//! (`ParticleShared::proximity.2`).
#![allow(non_snake_case)]

use crate::engine::camera::DTMatrix;
use crate::engine::particle::PLWorld;
use crate::engine::physic::rwm::{cross, RwMatrix, RwV3dLength, RwV3dNormalize};
use crate::engine::runtime::{ParticleShared, RuntimeHost};
use crate::engine::{Engine, Mem, Ptr};
use std::collections::{HashMap, HashSet};

/// EFGameObject ctor (0x001ff000..) +0xd0..+0x10f EFClippingParams defaults
pub const CLIP_DEFAULT: [u32; 16] = [
    0x3f00_0000, 0xbf80_0000, 0x6a0, 0, 0xbf80_0000, 0x6a0, 0, 0xbf80_0000, 0x6a0, 0, 0xbf80_0000, 0x6a0, 0,
    0xbf80_0000, 0x6a0, 0,
];
/// EFGameObject ctor +0x17c..+0x1a4 _ShadowParams defaults
pub const SHADOW_DEFAULT: [u32; 11] =
    [2, 0, 0x3e4c_cccd, 0x4000_0000, 0x3f80_0000, 0x80, 0x7fff_ffff, 0x3e4c_cccd, 0x4080_0000, 0x80, 0x3f80_0000];
/// EFGameObject ctor +0xcc
pub const CC_DEFAULT: u32 = 4;

/// The EFGameObject / EF3dObj fields behind the WP-S1 APIs (absent entry = constructor value).
#[derive(Default, Debug, Clone)]
pub struct S1State {
    /// +0xd0 EFClippingParams (0x40 bytes)
    pub clip: HashMap<u32, [u32; 16]>,
    /// +0x17c _ShadowParams (0x2c bytes)
    pub shadow: HashMap<u32, [u32; 11]>,
    /// objects moved to EFWorld+0x4134 (disabled list) by EnableObject(obj, 0): not clip-tested
    /// (EFWorld::ClipGameObjects walks +0x4128 only) and their clumps are out of the RpWorld.
    pub disabled: HashSet<u32>,
    /// (obj, 3d object index) -> EF3dObj +0x58 transparent priority
    pub transp_prio: HashMap<(u32, u32), u32>,
    /// (obj, 3d object index) -> EF3dObj +0x2c bits 0x10/0x100/0x200 (transparent type)
    pub transp_type: HashMap<(u32, u32), u32>,
    /// EF3dObj clipping bsphere set by SetRenderBSphere (center, radius)
    pub render_bsphere: HashMap<u32, ([f32; 3], f32)>,
    /// EF3dObj clipping bsphere set by EFAnimNode::SetCollPrimData (head record +0x14 centre, +0x20 radius)
    pub anim_bsphere: HashMap<u32, ([f32; 3], f32)>,
    /// _gpoWorldMgr+0x18: last object that got logic flag 0x2000
    pub world18: u32,
    /// WP-S4 SetLogicFlags renderer side: EFAnimationMgr +0x50 bit0 (logic flag 0x8: anim frozen)
    pub anim_b0: std::collections::HashSet<u32>,
    /// EFGameObject +0x54 bit1 (logic flag 0x20)
    pub anim54_b1: std::collections::HashSet<u32>,
    /// EF3dObj::LightObject(flag 0x100) on every 3d object: geometry rpGEOMETRYLIGHT (0x20) forced
    /// on/off per owner (absent = the geometry's streamed flag; LightObject only runs on transitions)
    pub lit: HashMap<u32, bool>,
    /// goShadowMgr casters (flag 0xc00; EF3dObj 0 +0x2c bit 8)
    pub shadow_casters: Vec<u32>,
    /// logic flags of the casters (EFShadowMgr::Begin reads 0x400 / 0x800)
    pub caster_flags: HashMap<u32, u32>,
    /// goProjectorMgr receivers (flag 0x200): their 3d object 0 gets the DFF mapper pass
    pub proj_receivers: Vec<u32>,
    /// WP-S1 APIspline: copy of EngineRuntime::splines (EFSplineMgr) made at level load
    pub splines: crate::engine::spline::EFSplineMgr,
    /// WP-S1 APIprojector: goProjectorMgr
    pub projectors: crate::engine::host::projector::EFProjectorMgr_BC,
    /// WP-S1 APIfbeffects: goFBEffectMgr
    pub fb: crate::engine::host::fbeffects::EFFBEffectMgr_BC,
    /// P22 R26 GAME_LOGICCLIP: EF3dObj resource uid -> object-space atomic clip spheres (render::clip_spheres;
    /// filled by the node publisher at level load)
    pub clip_spheres: HashMap<u32, Vec<[f32; 4]>>,
    /// EFGameObject +0x116 + 8 * camera slot: last distance band per camera
    pub clip_band: HashMap<(u32, usize), u16>,
    /// EFGameObject +0x13c / +0x13e: last frustum result / min distance band
    pub clip_last: HashMap<u32, (u16, u16)>,
    /// EFGameObject +0x5c: current clipping word
    pub clip_word: HashMap<u32, u32>,
    /// EFGameObject +0x138: the word last applied by SetMyCurrentClipping (ctor: 0)
    pub clip_cur: HashMap<u32, u32>,
    /// EFGameObject +0x114 / +0x110 + 8 * camera slot: that camera's ClipFrustumTest result and clipping
    /// word (band word +0xd8 / +0xdc); EFShadowMgr::Begin reads them for the rendering camera
    pub clip_cam: HashMap<(u32, usize), (u16, u32)>,
}

impl S1State {
    pub fn is_enabled(&self, uid: u32) -> bool {
        !self.disabled.contains(&uid)
    }
    /// K10: EFWorld::ClipGameObjects (EFWorld.cpp ~0x830) render part for an in-frustum object:
    /// mask = +0xdc of distance level uVar11 (0 when clip-distance is off, +0xcc bit 1 = +0xd4 > 0);
    /// mask & 4 -> RpWorldRemoveClump (not drawn). Only the distance-off case is decided here (the
    /// GAME_LOGICCLIP distance/frustum word is ClipGameObjects' clip_word, game.rs): e.g. Monster_Base capture (SetClippingCaptured 0x2bf /
    /// MiscFunc_ChangeClipping(4) 0x6bf, far -1) removes the captured monster from rendering.
    /// P32 R1: + the words of every band the C can select (clip_always, `cc` = the proximity +0xcc) all
    /// have bit 4: e.g. Poltergeist_BaseFUNC_ActivateFBEffect (0x00014f60, 0x10000007..0x100006e7 in every
    /// band) keeps the poltergeist off the render list for good (only its FrameDistortion draws it).
    pub fn clip_render_off(&self, uid: u32, cc: u32) -> bool {
        let Some(c) = self.clip.get(&uid) else { return false };
        (!(0.0 < f32::from_bits(c[1])) && c[3] & 4 != 0) || clip_always(c, cc) & 4 != 0
    }
}

fn rv3(m: &Mem, a: Ptr) -> [f32; 3] {
    [m.rf(a), m.rf(a + 4), m.rf(a + 8)]
}
fn wv3(m: &mut Mem, a: Ptr, v: [f32; 3]) {
    for (i, x) in v.iter().enumerate() {
        m.w32(a + 4 * i as u32, x.to_bits());
    }
}
fn to_rw(m: &DTMatrix) -> RwMatrix {
    RwMatrix { right: m.right, up: m.up, at: m.at, pos: m.pos }
}
fn from_rw(m: &RwMatrix) -> DTMatrix {
    DTMatrix { right: m.right, up: m.up, at: m.at, pos: m.pos }
}
/// RW float "is zero" test used by AlignAtWith: exponent < 2^-15
fn tiny15(x: f32) -> bool {
    (x.to_bits() & 0x7f80_0000) < 0x3800_0000
}
/// DT_GetBits(x - 0) & 0x7f800000 < 0x34000000
fn tiny23(x: f32) -> bool {
    (x.to_bits() & 0x7f80_0000) < 0x3400_0000
}

impl RuntimeHost {
    /// EFWorld::GetGameObject SELF resolution (same as runtime.rs `r`)
    pub(crate) fn s1_r(&self, id: u32) -> u32 {
        if id != crate::engine::SELF {
            return id;
        }
        if !self.p.lock().unwrap().logic {
            return id;
        }
        crate::logic::with_bound(|l| l.current_object().and_then(|h| l.objects[h].as_ref()).map(|o| o.id)).flatten().unwrap_or(id)
    }
    /// EFWorld::GetGameObject(id) on the logic objects (None = NULL)
    fn s1_obj<R>(&self, id: u32, f: impl FnOnce(&mut crate::logic::GameObject) -> R) -> Option<R> {
        if !self.p.lock().unwrap().logic {
            return None;
        }
        crate::logic::with_bound(|l| l.map.find(id).and_then(|h| l.objects[h].as_mut()).map(f)).flatten()
    }
    /// GetGameObject != NULL: a logic object or (logic runtime not attached) a world frame
    pub(crate) fn s1_is_go(&self, id: u32) -> bool {
        if self.s1_obj(id, |_| ()).is_some() {
            return true;
        }
        self.p.lock().unwrap().world.frames.contains_key(&id)
    }
    /// EFWorld::GetObjectPosData(id)+0x14 modelling matrix edit; None = NULL pos data
    fn s1_frame<R>(&self, id: u32, f: impl FnOnce(&mut DTMatrix) -> R) -> Option<R> {
        let mut g = self.p.lock().unwrap();
        g.hud_frame(id); // P28 B10
        g.world.frames.get_mut(&id).map(f)
    }

    // ------------------------------------------------------------------ logic flags
    /// SetLogicFlag (0x001efab0) / ResetLogicFlag (0x001efb00) -> EFGameObject::SetLogicFlags
    /// (0x002005d0) with +0x15c | v / & ~v.
    pub(crate) fn s1_SetLogicFlag(&mut self, a0: u32, a1: u32, set: bool) -> u32 {
        let a0 = self.s1_r(a0);
        let Some(old) = self.s1_obj(a0, |o| o.logic_flags) else { return 0 };
        let new = if set { old | a1 } else { old & !a1 };
        self.s1_SetLogicFlags(a0, old, new);
        0
    }
    /// EFGameObject::SetLogicFlags 1:1 for the parts the runtime models:
    /// - 0x10: physic flag 0x100 (+0x28 EFPhysic +0xc) + UpdatePhysicFlags; no physic -> bit dropped
    /// - 0x2000: _gpoWorldMgr+0x18 = id
    /// - +0x15c = value.
    /// Unmodelled side effects (no consumer in the runtime yet): 0x8 -> EFAnimationMgr +0x50 bit0,
    /// 0x20 -> +0x54 bit1, 0x100 -> EF3dObj::LightObject, 0xc00 -> EFShadowMgr caster,
    /// 0x200 -> EFProjectorMgr receiver.
    fn s1_SetLogicFlags(&mut self, uid: u32, old: u32, mut new: u32) {
        if (new & 0x78) != (old & 0x78) && (new & 0x10) != (old & 0x10) {
            let j = crate::engine::runtime::j2::J2(&self.p);
            if !j.physic_has(uid) {
                new &= !0x10;
            } else if new & 0x10 != 0 {
                j.physic(|m, _| crate::engine::physic::api::SetFlags(m, uid, 0x100));
            } else {
                j.physic(|m, _| crate::engine::physic::api::ResetFlags(m, uid, 0x100));
            }
        }
        {
            let mut g = self.p.lock().unwrap();
            let g = &mut *g;
            let s = &mut g.s1;
            if (new & 0x78) != (old & 0x78) {
                let t = |set: &mut std::collections::HashSet<u32>, on: bool| if on { set.insert(uid); } else { set.remove(&uid); };
                if (new & 8) != (old & 8) {
                    t(&mut s.anim_b0, new & 8 != 0);
                    // EFGameObject +0xb0 = EFAnimationMgr(+0x60) +0x50 bit0: EFAnimationMgr::Update skips Evaluate
                    if let Some(am) = g.sub.anim.get_mut(&uid) {
                        am.flags = if new & 8 != 0 { am.flags | 1 } else { am.flags & !1 };
                    }
                }
                if (new & 0x20) != (old & 0x20) {
                    t(&mut s.anim54_b1, new & 0x20 != 0);
                }
            }
            // 0x100 -> EF3dObj::LightObject(each 3d object, on)
            if (new & 0x100) != (old & 0x100) {
                s.lit.insert(uid, new & 0x100 != 0);
            }
            // 0xc00: off->on AddCaster, on->off RemoveCaster
            if new & 0xc00 != 0 && old & 0xc00 == 0 {
                if !s.shadow_casters.contains(&uid) { s.shadow_casters.push(uid); }
                s.caster_flags.insert(uid, new);
            } else if new & 0xc00 == 0 && old & 0xc00 != 0 {
                s.shadow_casters.retain(|&u| u != uid);
            }
            // 0x200: AddReceiver / RemoveReceiver
            if new & 0xc00 != 0 {
                s.caster_flags.insert(uid, new);
            }
            if new & 0x200 != 0 && old & 0x200 == 0 {
                s.proj_receivers.push(uid);
            } else if new & 0x200 == 0 && old & 0x200 != 0 {
                s.proj_receivers.retain(|&u| u != uid);
            }
            if new & 0x2000 != 0 {
                s.world18 = uid;
            }
        }
        self.s1_obj(uid, |o| o.logic_flags = new);
    }

    // ------------------------------------------------------------------ clipping
    /// EnableClipping (0x001efb50)
    pub(crate) fn s1_EnableClipping(&mut self, a0: u32, a1: u32, a2: u32) -> u32 {
        let a0 = self.s1_r(a0);
        if !self.s1_is_go(a0) {
            return 0;
        }
        let mut g = self.p.lock().unwrap();
        let ParticleShared { proximity, s1, .. } = &mut *g;
        let d4 = f32::from_bits(s1.clip.get(&a0).unwrap_or(&CLIP_DEFAULT)[1]);
        let e = proximity.entry(a0).or_insert(([0; 4], 0, CC_DEFAULT));
        let cc = &mut e.2;
        if a1 == 0 {
            *cc &= !a2;
        } else {
            if a2 & 1 != 0 {
                if 0.0 < d4 {
                    *cc |= 1;
                } else {
                    *cc &= !1;
                }
                *cc |= 4;
            }
            if a2 & 2 != 0 {
                if e.0[0] == 0 || e.0[1] == 0 {
                    *cc &= !2;
                } else {
                    *cc |= 2;
                }
                *cc |= 8;
            }
        }
        0
    }
    /// GetClippingParams (0x001efc70) -> EFGameObject::GetClippingParams: memcpy(p, +0xd0, 0x40)
    pub(crate) fn s1_GetClippingParams(&mut self, m: &mut Mem, a0: u32, a1: Ptr) -> u32 {
        let a0 = self.s1_r(a0);
        if a1 == 0 || !self.s1_is_go(a0) {
            return 0;
        }
        let c = *self.p.lock().unwrap().s1.clip.get(&a0).unwrap_or(&CLIP_DEFAULT);
        for (i, w) in c.iter().enumerate() {
            m.w32(a1 + 4 * i as u32, *w);
        }
        0
    }
    /// SetClippingParams (0x001efcc0) -> EFGameObject::SetClippingParams (0x00200420).
    /// Deviation: the recursive copy into child pos datas (param_3 != 0, EFPosData +0x24/+0x20
    /// children that are game objects) uses the runtime's parented list (AddChild) as the tree.
    pub(crate) fn s1_SetClippingParams(&mut self, m: &mut Mem, a0: u32, a1: Ptr, a2: u32) -> u32 {
        let a0 = self.s1_r(a0);
        if a1 == 0 || !self.s1_is_go(a0) {
            return 0;
        }
        let c: [u32; 16] = std::array::from_fn(|i| m.r32(a1 + 4 * i as u32));
        // P23 R49 debug: GAME_CLIPSETDBG=<uid hex>|all prints every SetClippingParams
        if let Ok(v) = std::env::var("GAME_CLIPSETDBG") {
            if v == "all" || u32::from_str_radix(&v, 16) == Ok(a0) {
                let bt = std::backtrace::Backtrace::force_capture().to_string();
                let fr: Vec<&str> = bt.lines().filter(|l| l.contains("modules::")).take(3).map(|l| l.trim()).collect();
                eprintln!("CLIPSET {a0:#x} rec {a2} {:x?} {fr:?}", &c[..10]);
            }
        }
        self.s1_set_clip_words(a0, c, a2);
        0
    }
    /// EFGameObject::SetClippingParams body (words already read)
    pub(crate) fn s1_set_clip_words(&self, a0: u32, c: [u32; 16], a2: u32) {
        let mut g = self.p.lock().unwrap();
        let mut todo = vec![a0];
        if a2 != 0 {
            // SetClippingParamsRecursive(+0x24 first child): every descendant
            let mut i = 0;
            while i < todo.len() {
                let p = todo[i];
                for ch in g.parented.iter().filter(|c| c.1 == p).map(|c| c.0) {
                    if !todo.contains(&ch) {
                        todo.push(ch);
                    }
                }
                i += 1;
            }
        }
        let ParticleShared { proximity, s1, .. } = &mut *g;
        for (n, uid) in todo.into_iter().enumerate() {
            if n > 0 && uid >> 24 != 0x2a {
                continue;
            }
            s1.clip.insert(uid, c);
            let cc = &mut proximity.entry(uid).or_insert(([0; 4], 0, CC_DEFAULT)).2;
            *cc &= !1;
            let d4 = f32::from_bits(c[1]);
            if 0.0 < d4 && (c[3] != 0 || c[2] != 0) {
                *cc |= 4;
            }
            if 0.0 < d4 {
                *cc |= 5;
            }
        }
    }

    // ------------------------------------------------------------------ shadow
    /// SetShadowParams (0x001efdc0): +0x17c..+0x1a4 = 11 words (NULL object: the original
    /// writes through NULL; here a no-op)
    pub(crate) fn s1_SetShadowParams(&mut self, m: &mut Mem, a0: u32, a1: Ptr) -> u32 {
        let a0 = self.s1_r(a0);
        if !self.s1_is_go(a0) {
            return 0;
        }
        let s: [u32; 11] = std::array::from_fn(|i| m.r32(a1 + 4 * i as u32));
        self.p.lock().unwrap().s1.shadow.insert(a0, s);
        0
    }
    /// GetShadowParams (0x001efe50): copies the 11 words, returns 1
    pub(crate) fn s1_GetShadowParams(&mut self, m: &mut Mem, a0: u32, a1: Ptr) -> u32 {
        let a0 = self.s1_r(a0);
        let s = *self.p.lock().unwrap().s1.shadow.get(&a0).unwrap_or(&SHADOW_DEFAULT);
        for (i, w) in s.iter().enumerate() {
            m.w32(a1 + 4 * i as u32, *w);
        }
        1
    }

    // ------------------------------------------------------------------ enable
    /// EnableObject (0x001f0400): relink between EFWorld+0x4128 (enabled) and +0x4134 (disabled)
    /// and add/remove the clumps from the RpWorld (+0x58 bit0).
    pub(crate) fn s1_EnableObject(&mut self, a0: u32, a1: u32) -> u32 {
        let a0 = self.s1_r(a0);
        if !self.s1_is_go(a0) {
            return 0;
        }
        // debug: GAME_ENDBG=1 logs every EnableObject call
        if std::env::var_os("GAME_ENDBG").is_some() {
            eprintln!("enable f{} {a0:#x} {a1}", crate::engine::runtime::TRACE_FRAME.load(std::sync::atomic::Ordering::Relaxed));
        }
        let mut g = self.p.lock().unwrap();
        if a1 == 0 {
            g.s1.disabled.insert(a0);
        } else {
            g.s1.disabled.remove(&a0);
        }
        0
    }

    // ------------------------------------------------------------------ frames
    /// GetLookUp (0x001ee780): LTM up of GetObjectFrame(bone) (0x2a ids) / pos-data frame
    pub(crate) fn s1_GetLookUp(&mut self, m: &mut Mem, a0: u32, a1: u32, a2: Ptr) -> Ptr {
        let a0 = self.s1_r(a0);
        let f = {
            let g = self.p.lock().unwrap();
            if a0 >> 24 == 0x2a { g.world.GetObjectFrame(a0, a1) } else { g.world.frames.get(&a0).copied() }
        };
        wv3(m, a2, f.map_or([0.0; 3], |f| f.up));
        a2
    }
    /// Translate (0x001ee940): RwFrameTranslate(v, POSTCONCAT) unless v is (near) zero
    pub(crate) fn s1_Translate(&mut self, m: &mut Mem, a0: u32, a1: Ptr) -> u32 {
        let a0 = self.s1_r(a0);
        let v = rv3(m, a1);
        if tiny23(v[0]) && tiny23(v[1]) && tiny23(v[2]) {
            return 0;
        }
        self.s1_frame(a0, |f| (0..3).for_each(|i| f.pos[i] += v[i]));
        0
    }
    /// SetTranslationSafe (0x001eeb80): pos = 0; RwFrameTranslate(v, POSTCONCAT)
    pub(crate) fn s1_SetTranslationSafe(&mut self, m: &mut Mem, a0: u32, a1: Ptr) -> u32 {
        let a0 = self.s1_r(a0);
        let v = rv3(m, a1);
        self.s1_frame(a0, |f| f.pos = v);
        0
    }
    /// Rotate (0x001eebe0, combine POSTCONCAT) / SetRotation (0x001eedb0, combine REPLACE):
    /// RwFrameRotate(axis, rad * 57.29578) keeping the frame position.
    pub(crate) fn s1_Rotate(&mut self, m: &mut Mem, a0: u32, a1: Ptr, a2: f32, combine: i32) -> u32 {
        let a0 = self.s1_r(a0);
        let axis = rv3(m, a1);
        self.s1_frame(a0, |f| {
            let mut r = to_rw(f);
            let pos = r.pos;
            r.Rotate(a2 * 57.29578, axis, combine);
            r.pos = pos;
            *f = from_rw(&r);
        });
        0
    }
    /// RotateAround (0x001eec90): p0 = M3x3*pivot; RwFrameRotate(POSTCONCAT); p1 = M'3x3*pivot;
    /// pos -= (p1 - p0)  (note: pos is rotated by the POSTCONCAT, as in the original)
    pub(crate) fn s1_RotateAround(&mut self, m: &mut Mem, a0: u32, a1: Ptr, a2: f32, a3: Ptr) -> u32 {
        let a0 = self.s1_r(a0);
        let axis = rv3(m, a1);
        let pivot = rv3(m, a3);
        self.s1_frame(a0, |f| {
            let mut r = to_rw(f);
            let p0 = r.TransformVector(pivot);
            r.Rotate(a2 * 57.29578, axis, 2);
            let p1 = r.TransformVector(pivot);
            for i in 0..3 {
                r.pos[i] -= p1[i] - p0[i];
            }
            *f = from_rw(&r);
        });
        0
    }
    /// AlignAtWith (0x001eee50): rotate the frame so that `at` points along v (shortest arc,
    /// RwMatrixRotate POSTCONCAT, position kept); anti-parallel: negate at and right.
    pub(crate) fn s1_AlignAtWith(&mut self, m: &mut Mem, a0: u32, a1: Ptr) -> u32 {
        let a0 = self.s1_r(a0);
        let v = rv3(m, a1);
        self.s1_frame(a0, |f| {
            let la = 1.0 / RwV3dLength(f.at);
            let a = [f.at[0] * la, f.at[1] * la, f.at[2] * la];
            let lv = 1.0 / RwV3dLength(v);
            let b = [v[0] * lv, v[1] * lv, v[2] * lv];
            if tiny15(a[0] - b[0]) && tiny15(a[1] - b[1]) && tiny15(a[2] - b[2]) {
                return;
            }
            if tiny15(a[0] + b[0]) && tiny15(a[1] + b[1]) && tiny15(a[2] + b[2]) {
                f.at = [-f.at[0], -f.at[1], -f.at[2]];
                f.right = [-f.right[0], -f.right[1], -f.right[2]];
                return;
            }
            let ax = [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
            let d = a[2] * b[2] + a[0] * b[0] + a[1] * b[1];
            let ang = if d <= -1.0 {
                std::f32::consts::PI
            } else if 1.0 <= d {
                0.0
            } else {
                d.acos()
            };
            let ax = RwV3dNormalize(ax);
            let mut r = to_rw(f);
            let pos = r.pos;
            r.Rotate(ang * 57.29578, ax, 2);
            r.pos = pos;
            *f = from_rw(&r);
        });
        0
    }
    /// LookAtAlignUp (0x001ef310): at = v; right = norm(up_hint x at); up = norm(at x right)
    pub(crate) fn s1_LookAtAlignUp(&mut self, m: &mut Mem, a0: u32, a1: Ptr, a2: Ptr) -> u32 {
        let a0 = self.s1_r(a0);
        let (at, u) = (rv3(m, a1), rv3(m, a2));
        self.s1_frame(a0, |f| {
            f.at = at;
            f.right = RwV3dNormalize(cross(u, at));
            f.up = RwV3dNormalize(cross(at, f.right));
        });
        0
    }
    /// LookUpAlignAt (0x001ef460): up = v; right = norm(up x at_hint); at = norm(right x up)
    pub(crate) fn s1_LookUpAlignAt(&mut self, m: &mut Mem, a0: u32, a1: Ptr, a2: Ptr) -> u32 {
        let a0 = self.s1_r(a0);
        let (up, a) = (rv3(m, a1), rv3(m, a2));
        self.s1_frame(a0, |f| {
            f.up = up;
            f.right = RwV3dNormalize(cross(up, a));
            f.at = RwV3dNormalize(cross(f.right, up));
        });
        0
    }
    /// SetIdentity (0x001ef6c0): RwFrameSetIdentity
    pub(crate) fn s1_SetIdentity(&mut self, a0: u32) -> u32 {
        let a0 = self.s1_r(a0);
        self.s1_frame(a0, |f| *f = from_rw(&RwMatrix::identity()));
        0
    }
    /// SetLMat (0x001eff80): copy the DTMatrix (RW layout: right +0, up +0x10, at +0x20, pos +0x30)
    /// into the frame's modelling matrix
    pub(crate) fn s1_SetLMat(&mut self, m: &mut Mem, a0: u32, a1: Ptr) -> u32 {
        let a0 = self.s1_r(a0);
        let n = DTMatrix { right: rv3(m, a1), up: rv3(m, a1 + 0x10), at: rv3(m, a1 + 0x20), pos: rv3(m, a1 + 0x30) };
        self.s1_frame(a0, |f| *f = n);
        0
    }
    /// RemoveAllChildren (0x001ee3c0): EFPosData vtbl+0x68 -> every child LeaveParent(flags)
    /// (flag 1 = identity frame, else keeps its world LTM). Deviation: sprite children attached
    /// with AddChild stay attached (no such call in the shipped modules).
    pub(crate) fn s1_RemoveAllChildren(&mut self, a0: u32, a1: u32) -> u32 {
        let a0 = self.s1_r(a0);
        let mut g = self.p.lock().unwrap();
        let kids: Vec<u32> = g.parented.iter().filter(|c| c.1 == a0).map(|c| c.0).collect();
        g.parented.retain(|c| c.1 != a0);
        if a1 & 1 != 0 {
            for k in kids {
                if let Some(f) = g.world.frames.get_mut(&k) {
                    *f = from_rw(&RwMatrix::identity());
                }
            }
        }
        0
    }

    // ------------------------------------------------------------------ 3D object / physic queries
    /// GetPhysicBSphereRadius (0x001ef830) / GetBoundingSphereRadius: EFPhysic +0x6c (0 without
    /// physic); the float is returned as its bit pattern
    pub(crate) fn s1_GetPhysicBSphereRadius(&mut self, a0: u32) -> u32 {
        let a0 = self.s1_r(a0);
        crate::engine::runtime::j2::J2(&self.p)
            .physic(|m, _| m.Physic(m.PhysicOf(a0)).map_or(0.0, |p| p.model.bsphere_radius))
            .to_bits()
    }
    /// GetRenderBSphere (0x001ef880): EF3dObj::GetClippingBSphere of 3d object 0 -> center, radius
    /// (no object: 0, outputs untouched). Without SetRenderBSphere the runtime's bsphere is the
    /// one the renderer published for the object (not modelled: 0 center/radius).
    pub(crate) fn s1_GetRenderBSphere(&mut self, m: &mut Mem, a0: u32, a1: Ptr, a2: Ptr) -> f32 {
        let a0 = self.s1_r(a0);
        if !self.s1_is_go(a0) {
            return 0.0;
        }
        let (c, r) = self.p.lock().unwrap().s1.render_bsphere.get(&a0).copied().unwrap_or(([0.0; 3], 0.0));
        wv3(m, a1, c);
        m.wf(a2, r);
        r
    }
    /// SetRenderBSphere (0x001ef930): EF3dObj::SetClippingBSphere(3d object 0)
    pub(crate) fn s1_SetRenderBSphere(&mut self, m: &mut Mem, a0: u32, a1: Ptr, a2: f32) -> u32 {
        let a0 = self.s1_r(a0);
        if !self.s1_is_go(a0) {
            return 0;
        }
        let c = rv3(m, a1);
        self.p.lock().unwrap().s1.render_bsphere.insert(a0, (c, a2));
        0
    }
    /// SetTransparentPriority (0x001f0200): Get3dObject(n) +0x58 = v
    pub(crate) fn s1_SetTransparentPriority(&mut self, a0: u32, a1: u32, a2: u32) -> u32 {
        let a0 = self.s1_r(a0);
        self.p.lock().unwrap().s1.transp_prio.insert((a0, a1), a2);
        0
    }
    /// GetTransparentPriority (0x001f0250)
    pub(crate) fn s1_GetTransparentPriority(&mut self, a0: u32, a1: u32) -> u32 {
        let a0 = self.s1_r(a0);
        self.p.lock().unwrap().s1.transp_prio.get(&(a0, a1)).copied().unwrap_or(0)
    }
    /// SetTransparentType (0x001f02a0): EF3dObj +0x2c bits 0x10 (1) / 0x100 (2) / 0x200 (3) / none
    pub(crate) fn s1_SetTransparentType(&mut self, a0: u32, a1: u32, a2: u32) -> u32 {
        let a0 = self.s1_r(a0);
        let mut g = self.p.lock().unwrap();
        let e = g.s1.transp_type.entry((a0, a1)).or_insert(0);
        *e = match a2 {
            1 => *e & 0xffff_fcff | 0x10,
            2 => *e & 0xffff_fdef | 0x100,
            3 => *e & 0xffff_feef | 0x200,
            _ => *e & 0xffff_fcef,
        };
        0
    }
    /// APIfbeffects::SetParamsFB: EFWorld::GetGameObject(id)+8 (the uid)
    pub(crate) fn s1_SetParamsFB(&mut self, m: &mut Mem, h: u32, ty: u32, p: Ptr) -> u32 {
        let id = if p != 0 { self.s1_r(m.r32(p + 4)) } else { 0 };
        let ok = p != 0 && self.s1_is_go(id);
        self.p.lock().unwrap().s1.fb.SetParamsFB(m, h, ty, p, |_| ok.then_some(id));
        0
    }
    /// GetNbJoints (0x001f03b0): RpHAnimHierarchy numNodes of 3d object 0 (0 without)
    pub(crate) fn s1_GetNbJoints(&mut self, a0: u32) -> u32 {
        let a0 = self.s1_r(a0);
        let g = self.p.lock().unwrap();
        g.world.node_ids.get(&a0).map(|n| n.len()).or_else(|| g.world.nodes.get(&a0).map(|n| n.len())).unwrap_or(0) as u32
    }
}

/// FreeGameBuffer (0x001edda0) -> EFGameMemoryMgr::FreeGameBuffer (0x001fe5c0): refcount (+8)
/// 0 and not persistent (+0xc == 0, GetGameBuffer param_3) -> buffer freed and the id removed;
/// refcount != 0 -> decremented. The persistent flag is read from the EFGameBuffer header that
/// [`GetGameBuffer`] writes in front of the data (buffers created elsewhere without that header
/// count as persistent).
pub fn FreeGameBuffer(e: &mut Engine, id: u32) -> u32 {
    let Some(&(p, rc)) = e.rt.game_buffers.get(&id) else { return 0 };
    if rc != 0 {
        e.rt.game_buffers.insert(id, (p, rc - 1));
        if has_header(&e.mem, id, p) {
            e.mem.w32(p - 0x14 + 8, rc - 1);
        }
        return 0;
    }
    if has_header(&e.mem, id, p) && e.mem.r8(p - 0x14 + 0xc) == 0 {
        e.mem.free(p - 0x14);
        e.rt.game_buffers.remove(&id);
    }
    0
}
/// EFGameMemoryMgr::Terminate (0x001fe9d0), run by EFGameLogicMgr::Terminate from EFEngineController::Reset
/// at the start of every EFEngineController::LoadLevel: every NON-persistent buffer (+0xc == 0) is
/// deleted whatever its refcount; persistent ones (and headerless ones, see [`FreeGameBuffer`]) survive
/// into the next level. WP-P19: carrying the non-persistent ones too kept ApiSectorClipping's client-list
/// buffer 40000 (GetGameBuffer(40000, count*4, 0)) alive with the old level's size (EP4/L01: 4 sectors)
/// -> EP4/L02A (0x22 sectors) wrote its list handles past the end -> api_list fault on the first fade.
pub fn TerminateGameBuffers(e: &mut Engine) {
    let ids: Vec<u32> = e.rt.game_buffers.iter().filter(|&(&id, &(p, _))| has_header(&e.mem, id, p) && e.mem.r8(p - 0x14 + 0xc) == 0).map(|(&id, _)| id).collect();
    for id in ids {
        let (p, _) = e.rt.game_buffers.remove(&id).unwrap();
        e.mem.free(p - 0x14);
    }
}
/// The buffers that survive a level switch ([`TerminateGameBuffers`] first): (id, data bytes) for the next
/// level's [`GetGameBuffer`] (bin/game.rs level switch, route tests).
pub fn CarriedGameBuffers(e: &mut Engine) -> Vec<(u32, Vec<u8>)> {
    TerminateGameBuffers(e);
    e.rt.game_buffers.iter().map(|(&id, &(p, _))| (id, (0..e.mem.r32(p - 0x10)).map(|i| e.mem.r8(p + i)).collect())).collect()
}
fn has_header(m: &Mem, id: u32, p: Ptr) -> bool {
    p >= 0x14 && m.r32(p - 0x14) == id && m.r32(p - 0x14 + 0x10) == p
}
/// GetGameBuffer (0x001edd60) -> EFGameMemoryMgr::GetGameBuffer: existing id -> refcount+1, data;
/// else EFGameBuffer(id, size, persistent): MMMalloc(size + 0x14) {id, size, refcount 0,
/// persistent = param_3 != 0, data = +0x14}, zeroed, refcount 1.
pub fn GetGameBuffer(e: &mut Engine, id: u32, size: u32, flag: u32) -> Ptr {
    if let Some((p, rc)) = e.rt.game_buffers.get_mut(&id) {
        *rc += 1;
        let (p, rc) = (*p, *rc);
        if has_header(&e.mem, id, p) {
            e.mem.w32(p - 0x14 + 8, rc);
        }
        return p;
    }
    let b = e.mem.malloc(size + 0x14);
    let p = b + 0x14;
    e.mem.w32(b, id);
    e.mem.w32(b + 4, size);
    e.mem.w32(b + 8, 1);
    e.mem.w8(b + 0xc, (flag != 0) as u8);
    e.mem.w32(b + 0x10, p);
    e.rt.game_buffers.insert(id, (p, 1));
    p
}

/// GAME_LOGICCLIP=1 (P22 R26, opt-in like GAME_CLIPORDER): EFWorld::ClipGameObjects decides which objects
/// run logic next frame (clip word without 0x400). Default off: every object runs every frame.
pub fn logic_clip_enabled() -> bool {
    matches!(std::env::var("GAME_LOGICCLIP").as_deref().map(str::trim), Ok("1"))
}

/// P21 R133 (default, GAME_LOGICCLIP off): the clip-word bits EFWorld::ClipGameObjects sets whatever the
/// cameras see = AND of the words of every band the object can be in (band 0 only without the distance
/// test, +0xcc bit 0; bands after the first negative distance are unreachable). The proximity OR only adds
/// bits, so these bits are set in the C every frame. Used for the physic list (0x80): e.g. EP6/L02
/// electricarc_local switched off (SetClipping(0): all words 0x4a7) has no collision in the C.
/// P21 R135: conservative = AND of ALL ten band words (replaced in P23 R39, see below).
/// P23 R39: exactly the bands the C can select. EFWorld::ClipGameObjects (0x0026f930): band = 0 when +0xcc bit 0
/// is clear, else the min over cameras of EFWorld::ClipDistanceTest (0x0026fe80), which only returns a band
/// <= the first one with a negative distance (`if (fVar7 < 0.0) return;` in its band loop) and at most 4. The
/// word is +0xd8 (out of frustum) / +0xdc (in) of that band. Bands past that (GetClippingParams defaults, e.g.
/// 6a0/0 after an arc's band-2 distance -1.0) are never read, so they must not mask the 0x80 bit (the R135 AND
/// over all five bands kept switched-off EP6/L02 arcs, all reachable words 0x4a7, on the physic list).
pub fn clip_always(c: &[u32; 16], cc: u32) -> u32 {
    // P23 R48 debug: GAME_CLIPR135=1 restores the R135 AND over all five bands (A/B tests)
    static R135: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    if *R135.get_or_init(|| std::env::var_os("GAME_CLIPR135").is_some()) { return (0..5).fold(!0, |w, i| w & c[2 + 3 * i] & c[3 + 3 * i]) }
    let k = if cc & 1 == 0 { 0 } else { (0..4).find(|&i| f32::from_bits(c[1 + 3 * i]) < 0.0).unwrap_or(4) };
    (0..=k).fold(!0, |w, i| w & c[2 + 3 * i] & c[3 + 3 * i])
}

/// What one EFWorld::ClipGameObjects pass produced.
pub struct ClipResult {
    /// world +0x4140: the objects to update next frame, oldest first
    pub logic: Vec<u32>,
    /// EFInstance::Notify(obj, 9, {band, visible, word}) (clip word has 0x10000000), newest first
    pub notify9: Vec<(u32, [u32; 3])>,
    /// not on the animation list +0x414c: disabled objects and clip word 0x20
    pub anim_off: HashSet<u32>,
    /// not on the physic list +0x4148 (EFPhysicMgr::InitFrame): disabled objects and clip word 0x80
    pub phys_off: HashSet<u32>,
    /// P33 R1: EFGameObject::SetMyCurrentClipping(obj, word) calls (word != +0x5c), in C order
    pub set_clipping: Vec<(u32, u32)>,
}

/// P32 R83: GAME_ANIMCLIP switch for [`SetAnimClipMode`] + the UpdateHierarchy skip (only reached with
/// GAME_LOGICCLIP=1). Default ON (C behaviour); `GAME_ANIMCLIP=0` = the pre-R83 port behaviour.
pub fn anim_clip_on() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var("GAME_ANIMCLIP").as_deref() != Ok("0"))
}
/// P32 R82/R83 (P33 table B #4): the anim part of EFGameObject::SetMyCurrentClipping (EFGameObject.cpp:1152-1162,
/// = EFAnimationMgr::SetClipMode inline): EFAnimationMgr +0x54 (EFGameObject +0xb4) = 0x20 (word 0x20: no
/// Evaluate), else 0x40 (word 0x40: time-only Evaluate, EFAnimationMgr.cpp:383), else 0. The +0x54 bit 4
/// (word 0x400 = logic off) is the port's ClipResult::logic list.
pub fn SetAnimClipMode(anim: &mut HashMap<u32, crate::engine::anim::mgr::EFAnimationMgr>, uid: u32, word: u32) {
    if let Some(m) = anim.get_mut(&uid) {
        m.mode = if word & 0x20 != 0 { 0x20 } else if word & 0x40 != 0 { 0x40 } else { 0 };
    }
}

/// P33 R1: EFGameObject::SetMyCurrentClipping (0x00209fe0), physic part: when the 0x180 bits differ from
/// +0x138, EFPhysic +0xc = (flags & ~0xc0) | (word 0x80 -> 0x40) | (word 0x100 -> 0x80), then
/// EFPhysic::UpdatePhysicFlags (0x40 -> 0x10000 = off the physic list and the object ray tests; 0x80 (word 0x100) ->
/// clear the DYNAMIC/COLLIDE/PARTICLES bits 0xe0000); +0x138 = word. Not ported here: +0xb4 anim LOD word
/// (0x20/0x40) = SetAnimClipMode (P32 R83) and +0x54 bit 4 (0x400; the port's logic list comes from ClipResult::logic).
pub fn SetMyCurrentClipping(cur: &mut HashMap<u32, u32>, physic: &mut crate::engine::physic::mgr::EFPhysicMgr, uid: u32, word: u32) {
    let old = cur.insert(uid, word).unwrap_or(0);
    if word & 0x180 != old & 0x180 {
        let id = physic.PhysicOf(uid);
        if let Some(p) = physic.PhysicMut(id) {
            let mut f = p.flags & 0xffff_ff3f;
            if word & 0x80 != 0 {
                f |= 0x40;
            }
            if word & 0x100 != 0 {
                f |= 0x80;
            }
            p.flags = f;
            p.UpdatePhysicFlags();
        }
    }
}

/// EFWorld::ClipDistanceTest (0x0026fe80): band of `d2` (squared camera distance) with the +0xd0
/// hysteresis around the current band `b`; bands with a negative distance are infinite.
pub fn ClipDistanceTest(c: &[u32; 16], d2: f32, b: u16) -> u16 {
    let h = f32::from_bits(c[0]);
    let dist = |i: usize| f32::from_bits(c[1 + 3 * i]);
    let bi = b as usize;
    let mut keep = true;
    if bi != 0 && d2 < (dist(bi - 1) - h) * (dist(bi - 1) - h) {
        keep = false;
    }
    if bi < 4 && 0.0 < dist(bi) && (dist(bi) + h) * (dist(bi) + h) < d2 {
        keep = false;
    }
    if keep {
        return b;
    }
    for i in 0..4 {
        let x = dist(i);
        if x < 0.0 || d2 < x * x {
            return i as u16;
        }
    }
    4
}

/// EFWorld::ClipFrustumTest (0x00270480): 1 when any atomic clip sphere of 3D object 0 is inside or
/// crosses all 6 world frustum planes; without a 3D object / atomics, the object frame position
/// (radius 0.01). `planes` = the camera's world frustum (EFCamera +0xcc).
pub fn ClipFrustumTest(planes: &[crate::engine::camera::EFPlane; 6], spheres: &[[f32; 4]], pos: [f32; 3]) -> u32 {
    let inside = |p: [f32; 3], r: f32| planes.iter().all(|pl| pl.dist(p) <= r);
    if spheres.is_empty() {
        return inside(pos, 0.01) as u32;
    }
    spheres.iter().any(|s| inside([s[0], s[1], s[2]], s[3])) as u32
}

/// EFWorld::ClipGameObjects (0x0026f930), logic part. `ids` = the enabled world objects oldest first.
/// Per active camera: ClipFrustumTest (OR = visible) and, with +0xcc bit 0, ClipDistanceTest (min band);
/// word = band's +0xd8 (not visible) / +0xdc (visible); a failed proximity test (+0xcc bit 1) ORs
/// +0x14c & ~0x10000000; the animation list (+0x414c: enabled objects without 0x20) is `anim_off`'s
/// complement, the physic list (+0x4148: without 0x80) `phys_off`'s. Not ported: the render list / RpWorld
/// clump add-remove (0x4) and SetMyCurrentClipping (0x100 physic). Deviation: atomic spheres are
/// at the rest pose (render::clip_spheres). Returns None without cameras (the C leaves the list).
/// P26 R5: `render_only` (the port's logic clipping is off: headless / --route / GAME_LOGICCLIP=0) still
/// fills the per-camera words +0x114 / +0x110 (s1.clip_cam, read by EFShadowMgr::Begin; the C runs
/// ClipGameObjects every frame) but leaves the logic side (proximity entries, clip_word/clip_last, lists) alone.
pub fn ClipGameObjects(
    render_only: bool,
    ids: &[u32],
    first: &HashSet<u32>,
    cams: &crate::engine::camera::EFGraphicsMgrCameras,
    frames: &HashMap<u32, DTMatrix>,
    obj3d: &HashMap<u32, (u32, bool)>,
    s1: &mut S1State,
    proximity: &mut HashMap<u32, ([u32; 4], u32, u32)>,
) -> Option<ClipResult> {
    use crate::engine::camera::{EFFrustum, EFPlane};
    if cams.count <= 0 {
        if std::env::var("GAME_CLIPDBG").is_ok() {
            eprintln!("clip: no cameras");
        }
        return None;
    }
    // EFWorld::PrepareRender: the world frustum of every active camera from its frame LTM
    let cams: Vec<(usize, [EFPlane; 6], [f32; 3])> = (0..cams.count as usize)
        .filter_map(|i| cams.cameras.get(i)?.as_ref())
        .filter(|c| c.flags & 1 != 0)
        .map(|c| {
            let ltm = frames.get(&c.frame_owner).copied().unwrap_or(c.ltm);
            if std::env::var("GAME_CLIPDBG").as_deref() == Ok("objs") {
                eprintln!("clipcam uid {:#x} owner {:#x} pos {:.1?} at {:.2?} vw {:?} near {} far {} par {}", c.uid, c.frame_owner, ltm.pos, ltm.at, c.GetViewWindow(), c.rw.near, c.rw.far, c.GetCameraProjection());
            }
            let [x, y] = c.GetViewWindow();
            let f = EFFrustum::SetFromCameraParams(x, y, c.rw.near, c.rw.far, c.GetCameraProjection());
            let mut w = EFFrustum::default();
            f.TransformPlanes(&ltm, &mut w);
            ((c.uid & 0xfff) as usize, w.planes, ltm.pos)
        })
        .collect();
    let mut out = ClipResult { logic: Vec::new(), notify9: Vec::new(), anim_off: HashSet::new(), phys_off: HashSet::new(), set_clipping: Vec::new() };
    for &uid in ids.iter().rev() {
        if s1.disabled.contains(&uid) {
            out.anim_off.insert(uid);
            out.phys_off.insert(uid);
            if std::env::var("GAME_CLIPDBG").is_ok_and(|v| u32::from_str_radix(&v, 16) == Ok(uid)) {
                eprintln!("clip {uid:#x}: disabled");
            }
            continue;
        }
        let Some(m) = frames.get(&uid) else {
            if std::env::var("GAME_CLIPDBG").is_ok_and(|v| u32::from_str_radix(&v, 16) == Ok(uid)) {
                eprintln!("clip {uid:#x}: no frame");
            }
            // no frame in the port: keep its logic running
            out.logic.push(uid);
            continue;
        };
        let pos = m.pos;
        let c = match s1.clip.get(&uid) {
            Some(c) => *c,
            // EFGameObject::Initialize (EFGameObject.cpp:388): "update first" objects (+0x15c & 0x4000)
            // get the band-0 invisible word 0 (always active) until SetClippingParams
            None if first.contains(&uid) => {
                let mut c = CLIP_DEFAULT;
                c[2] = 0;
                c
            }
            None => CLIP_DEFAULT,
        };
        let e = if render_only { proximity.get(&uid).copied().unwrap_or(([0; 4], 0, CC_DEFAULT)) } else { *proximity.entry(uid).or_insert(([0; 4], 0, CC_DEFAULT)) };
        let cc = e.2;
        let spheres: Vec<[f32; 4]> = match (s1.render_bsphere.get(&uid), obj3d.get(&uid).and_then(|(r, _)| s1.clip_spheres.get(r))) {
            // SetRenderBSphere: every atomic gets that sphere
            (Some(&(ctr, r)), Some(_)) => vec![[ctr[0], ctr[1], ctr[2], r]],
            (_, Some(v)) => v.clone(),
            _ => vec![],
        };
        let world: Vec<[f32; 4]> = spheres
            .iter()
            .map(|s| {
                let p = |k: usize| m.right[k] * s[0] + m.up[k] * s[1] + m.at[k] * s[2] + m.pos[k];
                [p(0), p(1), p(2), s[3]]
            })
            .collect();
        let mut vis = 0u32;
        let mut band = if cc & 1 != 0 { 5u16 } else { 0 };
        for (slot, planes, cpos) in &cams {
            let v = ClipFrustumTest(planes, &world, pos);
            vis |= v;
            let mut b8 = 0usize;
            if cc & 1 != 0 {
                let d = [pos[0] - cpos[0], pos[1] - cpos[1], pos[2] - cpos[2]];
                let d2 = d[2] * d[2] + d[0] * d[0] + d[1] * d[1];
                let b = s1.clip_band.entry((uid, *slot)).or_insert(0);
                *b = ClipDistanceTest(&c, d2, *b);
                band = band.min(*b);
                b8 = (*b as usize).min(4);
            }
            // +0x114 = frustum result, +0x110 = word of this camera's band (+0xd8 out / +0xdc in)
            let w = if v == 0 { c[2 + 3 * b8] } else { c[3 + 3 * b8] };
            s1.clip_cam.insert((uid, *slot), (v as u16, w));
        }
        if render_only {
            continue;
        }
        // band 5 (distance on, no active camera) would read past the table: band 0 here
        let bi = if band > 4 { 0 } else { band as usize };
        let mut word = if vis == 0 { c[2 + 3 * bi] } else { c[3 + 3 * bi] };
        if cc & 2 != 0 && e.1 == 0 {
            word |= e.0[3] & 0xefff_ffff;
        }
        if word & 0x400 == 0 {
            out.logic.push(uid);
        }
        if word & 0x20 != 0 {
            out.anim_off.insert(uid);
        }
        if word & 0x80 != 0 {
            out.phys_off.insert(uid);
        }
        if std::env::var("GAME_CLIPDBG").as_deref() == Ok("objs") {
            eprintln!("clipo {uid:#x} vis {vis} band {band} word {word:#x} sph {} pos {:.1?}", world.len(), pos);
        }
        if std::env::var("GAME_CLIPDBG").is_ok_and(|v| u32::from_str_radix(&v, 16) == Ok(uid)) {
            eprintln!("clip {uid:#x} cc {cc:#x} vis {vis} band {band} word {word:#x} spheres {} cams {} params {:x?}", world.len(), cams.len(), c);
        }
        // EFWorld.cpp:910-912: if (word != +0x5c) { +0x5c = word; SetMyCurrentClipping(word); }
        if s1.clip_word.insert(uid, word).unwrap_or(0) != word {
            out.set_clipping.push((uid, word));
        }
        let last = s1.clip_last.entry(uid).or_insert((0, 0));
        if word & 0x1000_0000 != 0 && (cc & 4 != 0 || last.0 as u32 != vis || last.1 != band) {
            out.notify9.push((uid, [band as u32, vis, word]));
        }
        last.0 = vis as u16;
        if cc & 1 != 0 {
            last.1 = band;
        }
    }
    out.logic.reverse();
    if std::env::var("GAME_CLIPDBG").as_deref() == Ok("all") {
        eprintln!("clip: {} objects, {} cams, {} run logic {:x?}", ids.len(), cams.len(), out.logic.len(), ids.iter().filter(|&&u| (0x2a000100..0x2a000110).contains(&u)).collect::<Vec<_>>());
    }
    Some(out)
}

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
}

impl S1State {
    pub fn is_enabled(&self, uid: u32) -> bool {
        !self.disabled.contains(&uid)
    }
    /// K10: EFWorld::ClipGameObjects (EFWorld.cpp ~0x830) render part for an in-frustum object:
    /// mask = +0xdc of distance level uVar11 (0 when clip-distance is off, +0xcc bit 1 = +0xd4 > 0);
    /// mask & 4 -> RpWorldRemoveClump (not drawn). Only the distance-off case is decided here (the
    /// distance test is not ported): e.g. Monster_Base capture (SetClippingCaptured 0x2bf /
    /// MiscFunc_ChangeClipping(4) 0x6bf, far -1) removes the captured monster from rendering.
    pub fn clip_render_off(&self, uid: u32) -> bool {
        let Some(c) = self.clip.get(&uid) else { return false };
        !(0.0 < f32::from_bits(c[1])) && c[3] & 4 != 0
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
        let Some(l) = self.p.lock().unwrap().logic else { return id };
        // SAFETY: see runtime.rs `r`
        let l = unsafe { &*l };
        l.current_object().and_then(|h| l.objects[h].as_ref()).map_or(id, |o| o.id)
    }
    /// EFWorld::GetGameObject(id) on the logic objects (None = NULL)
    fn s1_obj<R>(&self, id: u32, f: impl FnOnce(&mut crate::logic::GameObject) -> R) -> Option<R> {
        let l = self.p.lock().unwrap().logic?;
        // SAFETY: as runtime.rs `group`: only scalar fields not borrowed by the calling module
        let l = unsafe { &mut *(l as *mut crate::logic::LogicRuntime) };
        let h = l.map.find(id)?;
        l.objects[h].as_mut().map(f)
    }
    /// GetGameObject != NULL: a logic object or (logic runtime not attached) a world frame
    fn s1_is_go(&self, id: u32) -> bool {
        if self.s1_obj(id, |_| ()).is_some() {
            return true;
        }
        self.p.lock().unwrap().world.frames.contains_key(&id)
    }
    /// EFWorld::GetObjectPosData(id)+0x14 modelling matrix edit; None = NULL pos data
    fn s1_frame<R>(&self, id: u32, f: impl FnOnce(&mut DTMatrix) -> R) -> Option<R> {
        let mut g = self.p.lock().unwrap();
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
        0
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

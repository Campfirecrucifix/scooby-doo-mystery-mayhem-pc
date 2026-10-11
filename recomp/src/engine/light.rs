//! WP-I: EFLight / EFLight_BC, EFLightMgr / EFLightMgr_BC, EFWorld Add/RemoveLightInstance +
//! SetVisible(EFLight*) and the APIlight_* host functions (EXE.1 Classes/EF/EFLight*.cpp,
//! EFWorld.cpp, EFApi_Light.cpp).
//!
//! Replaced (not 1:1): RenderWare's RpLight is the plain [`RpLight`] (only the fields EFLight
//! reads: +1 type, +2 flags, +0x14 radius, +0x18 colour, cone angle) and "is in an RpWorld" is a
//! bool; the Bevy side turns visible lights into Bevy lights. EFLight's DTQuickPool allocator
//! (operator new/delete, ClearPools) is replaced by owning the lights in a Vec.
#![allow(non_snake_case)]

use crate::engine::camera::DTMatrix;
use crate::engine::{Mem, Ptr};

/// RpLightType values (RpLightCreate argument, RpLight +1).
pub mod light_type {
    pub const DIRECTIONAL: u8 = 1;
    pub const AMBIENT: u8 = 2;
    pub const POINT: u8 = 0x80;
    pub const SPOT: u8 = 0x81;
    pub const SPOTSOFT: u8 = 0x82;
}

/// Replacement for RpLight. Defaults = RpLightCreate (colour white, radius 0, flags 3).
#[derive(Debug, Clone, PartialEq)]
pub struct RpLight {
    /// +1
    pub ty: u8,
    /// +2 rpLIGHTLIGHTATOMICS(1) | rpLIGHTLIGHTWORLD(2)
    pub flags: u8,
    /// +0x14
    pub radius: f32,
    /// +0x18 RGBA floats
    pub color: [f32; 4],
    /// minusCosAngle (RpLightSetConeAngle stores -cos(angle))
    pub minus_cos_angle: f32,
    /// RpLightGetWorld() != 0
    pub in_world: bool,
}

impl RpLight {
    pub fn RpLightCreate(ty: u8) -> Self {
        RpLight { ty, flags: 3, radius: 0.0, color: [1.0; 4], minus_cos_angle: 0.0, in_world: false }
    }
    /// RpLightGetConeAngle = acos(-minusCosAngle)
    pub fn cone_angle(&self) -> f32 {
        (-self.minus_cos_angle).acos()
    }
}

/// EFLight (0x3c bytes).
#[derive(Debug, Clone, PartialEq)]
pub struct EFLight {
    /// +0x8 uid (0x12xxxxxx for dynamic lights, the descriptor uid for static ones)
    pub uid: u32,
    /// +0x10 owning EFGameObject (object id here; 0 for static lights)
    pub owner: u32,
    /// +0x14 frame LTM (None for ambient lights: no frame is created for type 2)
    pub frame: Option<DTMatrix>,
    /// +0x28 kind: 0x10000 dynamic world light, 0x20000 static, 0x40000 HUD light
    pub kind: u32,
    /// +0x34
    pub rp: Option<RpLight>,
    /// +0x38 world saved by SetVisible(0) (true = the light was in the world)
    pub saved_world: bool,
    /// EFHudMgr_SP::AddLight was called (HUD light list belongs to WP-F)
    pub in_hud: bool,
}

impl EFLight {
    /// EFLight::EFLight(LightType_e)
    pub fn new(ty: u8) -> Self {
        EFLight {
            uid: 0,
            owner: 0,
            frame: if ty != light_type::AMBIENT { Some(DTMatrix::default()) } else { None },
            kind: 0,
            rp: Some(RpLight::RpLightCreate(ty)),
            saved_world: false,
            in_hud: false,
        }
    }
    /// EFLight::SetVisible
    pub fn SetVisible(&mut self, v: u32) {
        let Some(rp) = self.rp.as_mut() else { return };
        if v == 0 {
            // quirk: saved_world takes RpLightGetWorld even if the light is not in a world
            if !self.saved_world {
                self.saved_world = rp.in_world;
                rp.in_world = false;
            }
        } else if self.saved_world {
            rp.in_world = true;
            self.saved_world = false;
        }
    }
    pub fn IsVisible(&self) -> bool {
        self.rp.as_ref().is_some_and(|r| r.in_world)
    }
    pub fn GetType(&self) -> u8 {
        self.rp.as_ref().map_or(0, |r| r.ty)
    }
    pub fn SetColor(&mut self, c: [f32; 4]) {
        if let Some(r) = self.rp.as_mut() {
            r.color = c;
        }
    }
    pub fn GetColor(&self) -> [f32; 4] {
        self.rp.as_ref().map_or([0.0; 4], |r| r.color)
    }
    pub fn SetRadius(&mut self, v: f32) {
        if let Some(r) = self.rp.as_mut() {
            r.radius = v;
        }
    }
    pub fn GetRadius(&self) -> f32 {
        self.rp.as_ref().map_or(0.0, |r| r.radius)
    }
    /// only spot lights (0x81 / 0x82)
    pub fn SetConeAngle(&mut self, a: f32) {
        if let Some(r) = self.rp.as_mut() {
            if r.ty == light_type::SPOT || r.ty == light_type::SPOTSOFT {
                r.minus_cos_angle = -a.cos();
            }
        }
    }
    pub fn GetConeAngle(&self) -> f32 {
        match &self.rp {
            Some(r) if r.ty == light_type::SPOT || r.ty == light_type::SPOTSOFT => r.cone_angle(),
            _ => 0.0,
        }
    }
}

/// EFLightMgr / EFLightMgr_BC. `lights[0]` is the list head (+0x10; new lights are pushed in front).
#[derive(Debug, Clone)]
pub struct EFLightMgr_BC {
    /// +4 dynamic, +6 static, +8 HUD light counts
    pub n_dynamic: i16,
    pub n_static: i16,
    pub n_hud: i16,
    pub lights: Vec<EFLight>,
    /// +0x18
    pub next_uid: u32,
    /// _giLightCount (EFApi_Light.cpp)
    pub api_light_count: i32,
}

impl Default for EFLightMgr_BC {
    fn default() -> Self {
        EFLightMgr_BC { n_dynamic: 0, n_static: 0, n_hud: 0, lights: vec![], next_uid: 1, api_light_count: 0 }
    }
}

impl EFLightMgr_BC {
    pub fn new() -> Self {
        Self::default()
    }
    /// EFLightMgr_BC::Terminate
    pub fn Terminate(&mut self) {
        self.lights.clear();
        self.next_uid = 1;
    }
    /// EFLightMgr_BC::GenerateUID
    pub fn GenerateUID(&mut self) -> u32 {
        let u = self.next_uid;
        self.next_uid = self.next_uid.wrapping_add(1);
        if self.next_uid == 0xffff {
            self.next_uid = 1;
        }
        u & 0xff_ffff | 0x1200_0000
    }
    /// EFLightMgr_BC::CreateStaticLight(uid, type)
    pub fn CreateStaticLight(&mut self, uid: u32, ty: u8) {
        let mut l = EFLight::new(ty);
        self.n_static += 1;
        l.kind = 0x20000;
        l.uid = uid;
        l.owner = 0;
        self.lights.insert(0, l);
    }
    /// EFLightMgr_BC::CreateDynamicLight(obj, type): bit 31 of type = HUD light.
    pub fn CreateDynamicLight(&mut self, owner: u32, ty: u32) -> u32 {
        let mut l = EFLight::new((ty & 0x7fff_ffff) as u8);
        if ty & 0x8000_0000 == 0 {
            l.kind = 0x10000;
            self.n_dynamic += 1;
        } else {
            l.kind = 0x40000;
            self.n_hud += 1;
        }
        l.uid = self.GenerateUID();
        l.owner = owner;
        let uid = l.uid;
        self.lights.insert(0, l);
        uid
    }
    /// EFLightMgr_BC::GetLight: first match from the head
    pub fn GetLight(&mut self, uid: u32) -> Option<&mut EFLight> {
        self.lights.iter_mut().find(|l| l.uid == uid)
    }
    /// EFLightMgr_BC::DestroyLight
    pub fn DestroyLight(&mut self, uid: u32) {
        if let Some(i) = self.lights.iter().position(|l| l.uid == uid) {
            match self.lights[i].kind {
                0x40000 => self.n_hud -= 1,
                0x20000 => self.n_static -= 1,
                0x10000 => self.n_dynamic -= 1,
                _ => {}
            }
            self.lights.remove(i);
        }
    }

    /// EFWorld::AddLightInstance: RpWorldAddLight if not in a world yet
    pub fn AddLightInstance(l: &mut EFLight) {
        if let Some(r) = l.rp.as_mut() {
            r.in_world = true;
        }
    }
    /// EFWorld::RemoveLightInstance
    pub fn RemoveLightInstance(l: &mut EFLight) {
        if let Some(r) = l.rp.as_mut() {
            r.in_world = false;
        }
    }

    // ---- APIlight (EFApi_Light.cpp). CreateLight takes the running object (GetGameObject(SELF)) as `owner`.

    pub fn APIlight_CreateLight(&mut self, _mem: &mut Mem, a0: u32, owner: u32) -> u32 {
        let uid = self.CreateDynamicLight(owner, a0);
        if let Some(l) = self.GetLight(uid) {
            if a0 & 0x8000_0000 == 0 {
                Self::AddLightInstance(l);
            } else {
                // EFHudMgr_SP::AddLight (WP-F)
                l.in_hud = true;
            }
        }
        self.api_light_count += 1;
        uid
    }
    /// writes RpLight +2 (flags byte)
    pub fn APIlight_SetFlag(&mut self, _mem: &mut Mem, a0: u32, a1: u32) -> u32 {
        if a0 != 0 {
            if let Some(r) = self.GetLight(a0).and_then(|l| l.rp.as_mut()) {
                r.flags = a1 as u8;
            }
        }
        0
    }
    pub fn APIlight_GetFlag(&mut self, _mem: &mut Mem, a0: u32) -> u8 {
        if a0 == 0 {
            return 0;
        }
        self.GetLight(a0).and_then(|l| l.rp.as_ref()).map_or(0, |r| r.flags)
    }
    /// quirk: DTColor::Clamp clamps the caller's CColor in place (module memory is written)
    pub fn APIlight_SetColor(&mut self, mem: &mut Mem, a0: u32, a1: Ptr) -> u32 {
        if a0 == 0 {
            return 0;
        }
        if let Some(l) = self.GetLight(a0) {
            let mut c = [0.0; 4];
            for i in 0..4 {
                let a = a1 + 4 * i as u32;
                let v = mem.rf(a);
                if v < 0.0 {
                    mem.wf(a, 0.0);
                } else if 1.0 < v {
                    mem.wf(a, 1.0);
                }
                c[i] = mem.rf(a);
            }
            l.SetColor(c);
        }
        0
    }
    pub fn APIlight_GetColor(&mut self, mem: &mut Mem, a0: u32, a1: Ptr) -> u32 {
        if a0 == 0 {
            return 0;
        }
        if let Some(l) = self.GetLight(a0) {
            for (i, v) in l.GetColor().iter().enumerate() {
                mem.wf(a1 + 4 * i as u32, *v);
            }
        }
        0
    }
    pub fn APIlight_SetRadius(&mut self, _mem: &mut Mem, a0: u32, a1: f32) -> u32 {
        if a0 != 0 {
            if let Some(l) = self.GetLight(a0) {
                l.SetRadius(a1);
            }
        }
        0
    }
    /// returns the float bits
    pub fn APIlight_GetRadius(&mut self, _mem: &mut Mem, a0: u32) -> u32 {
        if a0 == 0 {
            return 0;
        }
        self.GetLight(a0).map_or(0, |l| l.GetRadius().to_bits())
    }
    pub fn APIlight_SetConeAngle(&mut self, _mem: &mut Mem, a0: u32, a1: f32) -> u32 {
        if a0 != 0 {
            if let Some(l) = self.GetLight(a0) {
                l.SetConeAngle(a1);
            }
        }
        0
    }
    /// returns the float bits
    pub fn APIlight_GetConeAngle(&mut self, _mem: &mut Mem, a0: u32) -> u32 {
        if a0 == 0 {
            return 0;
        }
        self.GetLight(a0).map_or(0, |l| l.GetConeAngle().to_bits())
    }
    /// EFWorld::SetVisible(light, v) -> EFLight::SetVisible. quirk: a missing light is dereferenced
    /// in the original (reads address 0x34 = 0 -> no-op); here a no-op.
    pub fn APIlight_SetVisible(&mut self, _mem: &mut Mem, a0: u32, a1: i32) -> u32 {
        if a0 != 0 {
            if let Some(l) = self.GetLight(a0) {
                l.SetVisible(a1 as u32);
            }
        }
        0
    }
    pub fn APIlight_IsVisible(&mut self, _mem: &mut Mem, a0: u32) -> u32 {
        if a0 == 0 {
            return 0;
        }
        self.GetLight(a0).map_or(0, |l| l.IsVisible() as u32)
    }
    pub fn APIlight_DestroyLight(&mut self, _mem: &mut Mem, a0: u32) -> u32 {
        if a0 == 0 {
            return 0;
        }
        if let Some(l) = self.GetLight(a0) {
            if l.kind == 0x40000 {
                l.in_hud = false; // EFHudMgr_SP::RemoveLight
            } else {
                Self::RemoveLightInstance(l);
            }
            self.DestroyLight(a0);
            self.api_light_count -= 1;
        }
        0
    }
    /// SetName / GetName: empty in EXE.1
    pub fn APIlight_SetName(&mut self, _mem: &mut Mem, _a0: u32, _a1: Ptr) -> u32 {
        0
    }
    pub fn APIlight_GetName(&mut self, _mem: &mut Mem, _a0: u32, _a1: Ptr) -> u32 {
        0
    }
}

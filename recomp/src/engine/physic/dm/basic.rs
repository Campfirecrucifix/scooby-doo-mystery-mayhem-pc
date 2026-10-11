//! Small EFDM* modules, 1:1 from Classes/EFDM/*.cpp (DMAdd init values from EFApi_Physic.cpp).
//! Frame access: `ctx.phys.frame` is the owner's root frame (LTM == modelling matrix, as the
//! physics owners are root frames). "rotate + restore pos" = RwFrameRotate followed by the copy
//! of frame+0x40 back.
#![allow(non_snake_case)]

use crate::engine::physic::module::*;
use crate::engine::physic::rwm::*;

macro_rules! base_impl {
    () => {
        fn base(&self) -> &EFPhysicModule { &self.base }
        fn base_mut(&mut self) -> &mut EFPhysicModule { &mut self.base }
    };
}

fn rotate_keep_pos(m: &mut RwMatrix, deg: f32, axis: V3, combine: i32) {
    let pos = m.pos;
    m.Rotate(deg, axis, combine);
    m.pos = pos;
}

fn base(ty: u32) -> EFPhysicModule { EFPhysicModule { type_: ty, flags: 1, ..Default::default() } }

// ------------------------------------------------------------------ 0 EFDMAcceleration (0x44)
/// +0x30 accel, +0x3c max speed (2.0), +0x40 flags (1; bit1 = local accel + speed clamp)
pub struct EFDMAcceleration {
    pub base: EFPhysicModule,
    pub accel: V3,
    pub max_speed: f32,
    pub flags: u32,
}
impl EFDMAcceleration {
    pub fn new() -> Self { EFDMAcceleration { base: base(0x1000_0000), accel: [0.0; 3], max_speed: 2.0, flags: 1 } }
}
impl EFDM for EFDMAcceleration {
    base_impl!();
    fn ParamWords(&self) -> usize { 5 }
    fn PreColUpdate(&mut self, ctx: &mut DMCtx, dp: &mut EFDynamicPhysic, dt: f32) -> bool {
        let p = &mut *ctx.phys;
        if self.flags & 2 == 0 {
            dp.accel = add(dp.accel, self.accel);
            p.velocity = add(p.velocity, scale(self.accel, dt));
        } else {
            let a = scale(p.frame.TransformVector(self.accel), dt);
            let mut v = add(p.velocity, a);
            if self.max_speed < RwV3dLength(v) {
                v = scale(RwV3dNormalize(v), self.max_speed);
            }
            // quirk: dyn accel gets the *clamped new velocity* / dt, and the velocity gets the
            // clamped velocity added on top of itself (v_new = v_old + clamp(v_old + a*dt)).
            dp.accel = add(dp.accel, scale(v, 1.0 / dt));
            p.velocity = add(p.velocity, v);
        }
        true
    }
    fn SetParams(&mut self, _c: &mut DMCtx, p: Option<&[u32]>) {
        match p {
            None => {
                self.accel = [0.0; 3];
                self.max_speed = 2.0;
                self.flags = 1;
            }
            Some(p) => {
                self.accel = wv(p, 0);
                self.max_speed = wf(p, 3);
                self.flags = p[4];
            }
        }
    }
    fn GetParams(&self, _c: &DMCtx, p: &mut [u32]) {
        put_v(p, 0, self.accel);
        p[3] = fw(self.max_speed);
        p[4] = self.flags;
    }
}

// ------------------------------------------------------------------ 1 EFDMTrackObjet (0x48)
/// +0x30 speed (1.0), +0x34 target uid (-1), +0x38 flags, +0x3c offset.
/// flags: 1 yaw toward, 2 yaw+pitch, 4 add offset (world), 8 offset in target space, 0x10 away.
pub struct EFDMTrackObjet {
    pub base: EFPhysicModule,
    pub speed: f32,
    pub target: u32,
    pub flags: u32,
    pub offset: V3,
}
impl EFDMTrackObjet {
    pub fn new() -> Self { EFDMTrackObjet { base: base(0x1000_0001), speed: 1.0, target: 0xffff_ffff, flags: 0, offset: [0.0; 3] } }
}
impl EFDM for EFDMTrackObjet {
    base_impl!();
    fn ParamWords(&self) -> usize { 6 }
    fn PreColUpdate(&mut self, ctx: &mut DMCtx, _dp: &mut EFDynamicPhysic, dt: f32) -> bool {
        let mut tf: Option<RwMatrix> = None;
        if self.target != 0 {
            match ctx.world.GetObjectFrame(self.target) {
                None => {
                    self.base.SetStateBits(1, ctx);
                    return true;
                }
                Some(m) => tf = Some(m),
            }
        }
        let mut t = [0.0f32; 3];
        if self.flags & 8 == 0 {
            if let Some(m) = tf {
                t = m.pos;
            }
            if self.flags & 4 != 0 {
                t = add(t, self.offset);
            }
        } else if let Some(m) = tf {
            t = m.TransformPoint(self.offset);
        }
        let f = &mut ctx.phys.frame;
        let mut d = sub(t, f.pos);
        if self.flags & 0x10 != 0 {
            d = scale(d, -1.0);
        }
        let len2 = d[2] * d[2] + d[0] * d[0] + d[1] * d[1];
        let k = if dt * self.speed <= 1.0 { dt * self.speed } else { 1.0 };
        if self.flags & 2 == 0 {
            if self.flags & 1 != 0 && DT_EPSILON_FLOAT <= len2 {
                let a = WrapPiLoop(d[0].atan2(d[2]) - Heading(f));
                rotate_keep_pos(f, k * a * RAD2DEG, [0.0, 1.0, 0.0], rwCOMBINEPOSTCONCAT);
            }
        } else if DT_EPSILON_FLOAT <= len2 {
            let cur = Heading(f);
            let a = WrapPiOnce(d[0].atan2(d[2]) - cur);
            let s = f.at[1].asin();
            let pitch = WrapPiOnce((-d[1]).atan2((d[2] * d[2] + d[0] * d[0]).sqrt()) - -s);
            rotate_keep_pos(f, (-s + k * pitch) * RAD2DEG, [1.0, 0.0, 0.0], rwCOMBINEREPLACE);
            rotate_keep_pos(f, (cur + k * a) * RAD2DEG, [0.0, 1.0, 0.0], rwCOMBINEPOSTCONCAT);
        }
        true
    }
    fn SetParams(&mut self, ctx: &mut DMCtx, p: Option<&[u32]>) {
        match p {
            None => {
                self.target = 0;
                self.speed = 1.0;
                self.flags = 0;
                self.offset = [0.0; 3];
            }
            Some(p) => {
                self.target = ResolveSelf(ctx, p[0]);
                self.speed = wf(p, 1);
                self.offset = wv(p, 2);
                self.flags = p[5];
            }
        }
    }
    fn GetParams(&self, _c: &DMCtx, p: &mut [u32]) {
        p[0] = self.target;
        p[1] = fw(self.speed);
        put_v(p, 2, self.offset);
        p[5] = self.flags;
    }
}

// ------------------------------------------------------------------ 4 EFDMMove (0x40)
/// +0x30 displacement velocity, +0x3c flags (bit0 local)
pub struct EFDMMove {
    pub base: EFPhysicModule,
    pub vel: V3,
    pub flags: u32,
}
impl EFDMMove {
    pub fn new() -> Self { EFDMMove { base: base(0x1000_0004), vel: [0.0; 3], flags: 0 } }
}
impl EFDM for EFDMMove {
    base_impl!();
    fn ParamWords(&self) -> usize { 4 }
    fn PreColUpdate(&mut self, ctx: &mut DMCtx, dp: &mut EFDynamicPhysic, _dt: f32) -> bool {
        let v = if self.flags & 1 == 0 { self.vel } else { ctx.phys.frame.TransformVector(self.vel) };
        dp.disp = add(dp.disp, v);
        true
    }
    fn SetParams(&mut self, _c: &mut DMCtx, p: Option<&[u32]>) {
        match p {
            None => {
                self.flags = 0;
                self.vel = [0.0; 3];
            }
            Some(p) => {
                self.flags = p[3];
                self.vel = wv(p, 0);
            }
        }
    }
    fn GetParams(&self, _c: &DMCtx, p: &mut [u32]) {
        p[3] = self.flags;
        put_v(p, 0, self.vel);
    }
}

// ------------------------------------------------------------------ 7 EFDMAirGrip (0x38)
/// +0x30 grip (1.0), +0x34 flags (1/2/4 axis x/y/z, 8 world space)
pub struct EFDMAirGrip {
    pub base: EFPhysicModule,
    pub grip: f32,
    pub flags: u32,
}
impl EFDMAirGrip {
    pub fn new() -> Self { EFDMAirGrip { base: base(0x1000_0007), grip: 1.0, flags: 0 } }
}
impl EFDM for EFDMAirGrip {
    base_impl!();
    fn ParamWords(&self) -> usize { 2 }
    fn PreColUpdate(&mut self, ctx: &mut DMCtx, dp: &mut EFDynamicPhysic, dt: f32) -> bool {
        let k = if self.grip * dt <= 1.0 { self.grip * dt } else { 1.0 };
        let p = &mut *ctx.phys;
        let mut d = sub(dp.disp, p.velocity);
        if self.flags & 8 == 0 {
            // uRam003439d0 = Invert(OrthoNormalize(LTM))
            d = p.frame.OrthoNormalize().Invert().TransformVector(d);
        }
        let mut r = [0.0f32; 3];
        for i in 0..3 {
            if self.flags & (1 << i) != 0 {
                r[i] = d[i] * k;
            }
        }
        if self.flags & 8 == 0 {
            r = p.frame.TransformVector(r);
        }
        p.velocity = add(p.velocity, r);
        true
    }
    fn SetParams(&mut self, _c: &mut DMCtx, p: Option<&[u32]>) {
        match p {
            None => {
                self.grip = 1.0;
                self.flags = 0;
            }
            Some(p) => {
                self.grip = wf(p, 0);
                self.flags = p[1];
            }
        }
    }
    fn GetParams(&self, _c: &DMCtx, p: &mut [u32]) {
        p[0] = fw(self.grip);
        p[1] = self.flags;
    }
}

// ------------------------------------------------------------------ 10 EFDMRotate (0x40)
/// +0x30 local axis (0,1,0), +0x3c angular speed (rad/s)
pub struct EFDMRotate {
    pub base: EFPhysicModule,
    pub axis: V3,
    pub speed: f32,
}
impl EFDMRotate {
    pub fn new() -> Self { EFDMRotate { base: base(0x1000_000a), axis: [0.0, 1.0, 0.0], speed: 0.0 } }
}
impl EFDM for EFDMRotate {
    base_impl!();
    fn ParamWords(&self) -> usize { 4 }
    fn PreColUpdate(&mut self, ctx: &mut DMCtx, _dp: &mut EFDynamicPhysic, dt: f32) -> bool {
        if self.speed != 0.0 {
            let f = &mut ctx.phys.frame;
            let ax = f.TransformVector(self.axis);
            rotate_keep_pos(f, self.speed * dt * RAD2DEG, ax, rwCOMBINEPOSTCONCAT);
        }
        true
    }
    fn SetParams(&mut self, _c: &mut DMCtx, p: Option<&[u32]>) {
        match p {
            None => {
                self.speed = 0.0;
                self.axis = [0.0, 1.0, 0.0];
            }
            Some(p) => {
                self.speed = wf(p, 0);
                self.axis = RwV3dNormalize(wv(p, 1));
            }
        }
    }
    fn GetParams(&self, _c: &DMCtx, p: &mut [u32]) {
        p[0] = fw(self.speed);
        put_v(p, 1, self.axis);
    }
}

// ------------------------------------------------------------------ 0x16 EFDMShaking (0x50)
/// +0x30 axis (0 right, 1 up, 2 at), +0x34 duration, +0x38 amplitude, +0x3c fade-in end,
/// +0x40 fade-out start, +0x44 frequency, +0x48 time, +0x4c 2*pi*frequency
pub struct EFDMShaking {
    pub base: EFPhysicModule,
    pub axis: u32,
    pub duration: f32,
    pub amplitude: f32,
    pub fade_in: f32,
    pub fade_out: f32,
    pub freq: f32,
    pub time: f32,
    pub omega: f32,
}
impl EFDMShaking {
    pub fn new() -> Self {
        EFDMShaking { base: base(0x1000_0016), axis: 0, duration: 0.0, amplitude: 0.0, fade_in: 0.0, fade_out: 0.0, freq: 0.0, time: 0.0, omega: 0.0 }
    }
}
impl EFDM for EFDMShaking {
    base_impl!();
    fn ParamWords(&self) -> usize { 6 }
    fn PreColUpdate(&mut self, ctx: &mut DMCtx, _dp: &mut EFDynamicPhysic, dt: f32) -> bool {
        let t = self.time;
        let dur = self.duration;
        if dur < t {
            return true;
        }
        let mut k = 1.0;
        if t < self.fade_in {
            k = t / self.fade_in;
        } else if self.fade_out < t {
            k = (dur - t) / (dur - self.fade_out);
        }
        let a = ((self.amplitude as f64 * ((t * self.omega) as f64).sin()) as f32) * k;
        let f = &mut ctx.phys.frame;
        let ax = match self.axis {
            0 => Some(f.right),
            1 => Some(f.up),
            2 => Some(f.at),
            _ => None,
        };
        if let Some(ax) = ax {
            // RwFrameRotate(frame, frame+0x10/0x20/0x30 axis, rwCOMBINEPOSTCONCAT): combine = 2
            // verified in the MIPS asm (0x001e6144 addiu a2,zero,2); pos restored afterwards.
            rotate_keep_pos(f, a * RAD2DEG, ax, rwCOMBINEPOSTCONCAT);
        }
        self.time = t + dt;
        true
    }
    fn SetParams(&mut self, _c: &mut DMCtx, p: Option<&[u32]>) {
        if let Some(p) = p {
            self.time = 0.0;
            self.amplitude = wf(p, 2);
            self.duration = wf(p, 1);
            self.fade_in = wf(p, 3) * self.duration;
            self.fade_out = self.duration - wf(p, 4) * self.duration;
            self.freq = wf(p, 5);
            self.omega = self.freq * 6.283185;
            self.axis = p[0];
        }
    }
    /// EFDMShaking::GetParams is empty
    fn GetParams(&self, _c: &DMCtx, _p: &mut [u32]) {}
    fn DoAction(&mut self, _c: &mut DMCtx, action: u32, _p: &mut [u32]) {
        if action == 1 {
            self.time = 0.0;
        }
    }
}

//! EFDMGroundGrip (kind 8), EFDMAnalogRotation (kind 2), EFDMAnalogMove (kind 5), 1:1 from
//! Classes/EFDM/EFDMGroundGrip.cpp, EFDMAnalogRotation.cpp, EFDMAnalogMove.cpp.
#![allow(non_snake_case)]

use crate::engine::physic::module::*;
use crate::engine::physic::rwm::*;

fn base(ty: u32) -> EFPhysicModule { EFPhysicModule { type_: ty, flags: 1, ..Default::default() } }

macro_rules! base_impl {
    () => {
        fn base(&self) -> &EFPhysicModule { &self.base }
        fn base_mut(&mut self) -> &mut EFPhysicModule { &mut self.base }
    };
}

// ------------------------------------------------------------------ 8 EFDMGroundGrip (0x68, Col)
/// +0x30 grip (1.0), +0x34 cos(max slope) (0x3f333333), +0x38 last frame processed.
/// +0x3c..+0x64 are zeroed by DMAdd and never read.
pub struct EFDMGroundGrip {
    pub base: EFPhysicModule,
    pub grip: f32,
    pub cos_slope: f32,
    pub last_frame: u32,
}
impl EFDMGroundGrip {
    pub fn new() -> Self { EFDMGroundGrip { base: base(0x2000_0008), grip: 1.0, cos_slope: f32::from_bits(0x3f33_3333), last_frame: 0 } }
}
impl EFDM for EFDMGroundGrip {
    base_impl!();
    fn ParamWords(&self) -> usize { 2 }
    fn SetParams(&mut self, _c: &mut DMCtx, p: Option<&[u32]>) {
        match p {
            // quirk: the NULL branch only resets the grip
            None => self.grip = 1.0,
            Some(p) => {
                self.grip = wf(p, 0);
                self.cos_slope = wf(p, 1).cos();
            }
        }
    }
    fn GetParams(&self, _c: &DMCtx, p: &mut [u32]) {
        p[0] = fw(self.grip);
        p[1] = fw(self.cos_slope.acos());
    }
    fn ColUpdate(&mut self, ctx: &mut DMCtx, stage: &EFCollisionStage, d: &EFColDMData, dt: f32) {
        let mut dt = dt;
        if self.last_frame == ctx.frame || d.normal[1] < self.cos_slope {
            return;
        }
        let p = &mut *ctx.phys;
        p.floor_cos = self.cos_slope;
        let n = d.normal;
        let mut v = p.velocity;
        if let Some((ov, _, _)) = d.other {
            v = sub(v, ov);
        }
        let mut vn = [0.0f32; 3];
        let f = v[2] * n[2] + v[0] * n[0] + v[1] * n[1];
        if 0.001 < f {
            if 0.01 < f {
                dt *= 0.01 / f;
            }
            vn = scale(n, f);
            v = sub(v, vn);
        }
        self.last_frame = ctx.frame;
        let mut g = self.grip;
        if d.contacts.is_empty() {
            match (d.env_friction, d.other) {
                (Some(ef), _) => g *= ef,
                (None, Some((_, fl, fr))) => {
                    if fl & 0x60_0000 == 0 {
                        return;
                    }
                    g *= fr;
                }
                // C++ dereferences a NULL other physic here
                (None, None) => return,
            }
        } else {
            let mut m = 0.0f32;
            for c in &d.contacts {
                let mut x = m;
                match c {
                    (Some(ef), _) => x = if *ef <= m { m } else { *ef },
                    (None, Some((fl, fr))) if fl & 0x60_0000 != 0 => x = if *fr <= m { m } else { *fr },
                    _ => {}
                }
                m = x;
            }
            g *= m;
        }
        let k = if g < 50.0 { g * dt } else { 1.0 };
        let mut t = stage.dyn_.disp;
        let len = RwV3dLength(t);
        if DT_MIN_FLOAT < len {
            let q = t[2] * n[2] + t[0] * n[0] + t[1] * n[1];
            t = sub(t, scale(n, q));
            let l2 = RwV3dLength(t);
            if DT_MIN_FLOAT < l2 {
                t = scale(t, len / l2);
            }
        }
        if k < 1.0 {
            let dd = sub(t, v);
            if 0.01 < dd[2] * dd[2] + dd[0] * dd[0] + dd[1] * dd[1] {
                t = add(v, scale(dd, k));
            }
        }
        if let Some((ov, _, _)) = d.other {
            t = add(t, ov);
        }
        p.velocity = add(t, vn);
        p.falling_timer = p.falling_delay;
    }
}

// ------------------------------------------------------------------ 2 EFDMAnalogRotation (0x54)
/// +0x30 speed (1.0), +0x34 flags (2: clamp to +-speed, 8: raw x-axis turn), +0x38/+0x3c input
/// action (a, b), +0x40 = 1, +0x44..+0x4c zero (unused).
pub struct EFDMAnalogRotation {
    pub base: EFPhysicModule,
    pub speed: f32,
    pub flags: u32,
    pub action: [u32; 2],
    pub w40: u32,
}
impl EFDMAnalogRotation {
    pub fn new() -> Self { EFDMAnalogRotation { base: base(0x1000_0002), speed: 1.0, flags: 0, action: [0; 2], w40: 1 } }
}
impl EFDM for EFDMAnalogRotation {
    base_impl!();
    fn ParamWords(&self) -> usize { 4 }
    fn SetParams(&mut self, _c: &mut DMCtx, p: Option<&[u32]>) {
        match p {
            None => {
                self.speed = 1.0;
                self.flags = 0;
                self.action = [0; 2];
                self.w40 = 1;
            }
            Some(p) => {
                self.speed = wf(p, 0);
                self.flags = p[1];
                self.action = [p[2], p[3]];
            }
        }
    }
    fn GetParams(&self, _c: &DMCtx, p: &mut [u32]) {
        p[0] = fw(self.speed);
        p[1] = self.flags;
        p[2] = self.action[0];
        p[3] = self.action[1];
    }
    fn PreColUpdate(&mut self, ctx: &mut DMCtx, _dp: &mut EFDynamicPhysic, dt: f32) -> bool {
        let Some([x, y]) = ctx.world.InputActionGet(self.action[0], self.action[1]) else { return true };
        if x == 0.0 && y == 0.0 {
            return true;
        }
        let cam = ctx.world.GetCameraLTM();
        let h = Heading(&cam);
        let (s, c) = (h.sin(), h.cos());
        let yy = s * y + -c * x;
        let xx = c * y + s * x;
        let f = &mut ctx.phys.frame;
        let a = if self.flags & 8 == 0 {
            if xx * xx + yy * yy < 0.01 {
                return true;
            }
            let a = WrapPiOnce(yy.atan2(xx) - Heading(f));
            if self.flags & 2 == 0 {
                let k = if dt * self.speed <= 1.0 { dt * self.speed } else { 1.0 };
                a * k
            } else {
                let sp = self.speed;
                if a <= sp && -sp <= a { a } else if a > sp { sp } else { -sp }
            }
        } else {
            x * self.speed * dt
        };
        let up = f.up;
        let pos = f.pos;
        f.Rotate(a * RAD2DEG, up, rwCOMBINEPOSTCONCAT);
        f.pos = pos;
        true
    }
}

// ------------------------------------------------------------------ 5 EFDMAnalogMove (0x50)
/// +0x30/+0x34 input action (a, b), +0x38/+0x3c/+0x40 per-axis scale, +0x44 camera relative (1),
/// +0x48 additive (1; 0 = overwrite the velocity), +0x4c flags (1 xy, 2 xz, 4 yz(-ish), 8 x,
/// 0x10 y, 0x20 z, 0x100 object space).
pub struct EFDMAnalogMove {
    pub base: EFPhysicModule,
    pub action: [u32; 2],
    pub scale: V3,
    pub camera: u32,
    pub additive: u32,
    pub flags: u32,
}
impl EFDMAnalogMove {
    pub fn new() -> Self { EFDMAnalogMove { base: base(0x1000_0005), action: [0; 2], scale: [0.0; 3], camera: 0, additive: 1, flags: 0 } }

    /// EFDMAnalogMove::CameraTransfo(v, k, normalize, out)
    pub fn CameraTransfo(&self, ctx: &DMCtx, v: V3, k: V3, norm: bool) -> V3 {
        let mut v = ctx.world.GetCameraLTM().TransformVector(v);
        if self.flags & 0x100 != 0 {
            v = ctx.phys.frame.Invert().TransformVector(v);
        }
        let mut l = RwV3dLength(v);
        v = [v[0] * -k[0], v[1] * k[1], v[2] * k[2]];
        if norm {
            let q = v[2] * v[2] + v[0] * v[0] + v[1] * v[1];
            if q != 0.0 {
                l /= q.sqrt();
            }
        } else {
            l = 1.0;
        }
        scale(v, l)
    }

    /// EFDMAnalogMove::ProjectInputs(x, y, out)
    pub fn ProjectInputs(&self, ctx: &DMCtx, mut x: f32, mut y: f32) -> V3 {
        let mut m = (y * y + x * x).sqrt();
        if 1.0 < m {
            x /= m;
            y /= m;
            m = 1.0;
        }
        let f = self.flags;
        let cam = self.camera != 0;
        let mut o = [0.0f32; 3];
        if f & 1 != 0 {
            o = if cam { self.CameraTransfo(ctx, [-x, y, 0.0], [1.0, 1.0, 0.0], true) } else { [x, y, 0.0] };
        } else if f & 2 != 0 {
            o = if cam { self.CameraTransfo(ctx, [-x, 0.0, y], [1.0, 0.0, 1.0], true) } else { [x, 0.0, y] };
        } else if f & 4 != 0 {
            // quirk: non-camera case puts (y, x) into (y, z)
            o = if cam { self.CameraTransfo(ctx, [-x, y, 0.0], [0.0, 1.0, 1.0], true) } else { [0.0, y, x] };
        } else if f & 8 != 0 {
            o = if cam { self.CameraTransfo(ctx, [-x, 0.0, y], [m, 0.0, 0.0], false) } else { [m, 0.0, 0.0] };
        } else if f & 0x10 != 0 {
            o = if cam { self.CameraTransfo(ctx, [-x, y, 0.0], [0.0, m, 0.0], false) } else { [0.0, m, 0.0] };
        } else if f & 0x20 != 0 {
            o = if cam { self.CameraTransfo(ctx, [-x, 0.0, y], [0.0, 0.0, m], false) } else { [0.0, 0.0, m] };
        }
        o = [o[0] * -self.scale[0], o[1] * self.scale[1], o[2] * self.scale[2]];
        if f & 0x100 != 0 {
            o = ctx.phys.frame.TransformVector(o);
        }
        o
    }
}
impl EFDM for EFDMAnalogMove {
    base_impl!();
    fn ParamWords(&self) -> usize { 8 }
    fn SetParams(&mut self, _c: &mut DMCtx, p: Option<&[u32]>) {
        match p {
            None => {
                self.action = [0; 2];
                self.scale = [0.0; 3];
                self.camera = 1;
                self.additive = 1;
                self.flags = 0;
            }
            Some(p) => {
                self.action = [p[0], p[1]];
                self.scale = wv(p, 2);
                self.camera = p[5];
                self.additive = p[6];
                self.flags = p[7];
            }
        }
    }
    fn GetParams(&self, _c: &DMCtx, p: &mut [u32]) {
        p[0] = self.action[0];
        p[1] = self.action[1];
        put_v(p, 2, self.scale);
        p[5] = self.camera;
        p[6] = self.additive;
        p[7] = self.flags;
    }
    fn PreColUpdate(&mut self, ctx: &mut DMCtx, dp: &mut EFDynamicPhysic, _dt: f32) -> bool {
        let Some([x, y]) = ctx.world.InputActionGet(self.action[0], self.action[1]) else { return true };
        if x == 0.0 && y == 0.0 {
            return true;
        }
        let v = self.ProjectInputs(ctx, x, y);
        dp.disp = add(dp.disp, v);
        if self.additive == 0 {
            let p = &mut *ctx.phys;
            p.velocity = dp.disp;
            p.falling_timer = p.falling_delay;
        }
        true
    }
    /// action 1: {x, y} in, projected vector written at +8
    fn DoAction(&mut self, ctx: &mut DMCtx, action: u32, p: &mut [u32]) {
        if action == 1 && p.len() >= 5 {
            let v = self.ProjectInputs(ctx, wf(p, 0), wf(p, 1));
            put_v(p, 2, v);
        }
    }
}

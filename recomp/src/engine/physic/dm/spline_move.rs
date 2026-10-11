//! EFDMSplineMove (Classes/EFDM/EFDMSplineMove.cpp, DMAdd kind 3, type 0x50000003 = PRE|POST,
//! 0xb8 bytes), 1:1. Moves the owner along an EFQuadraticSpline (cursor at +0x68) with an
//! analog-driven side/up offset and tilt. No logic module in recomp/src/modules uses kind 3.
//! The spline table (_gpoSplineMgr) comes from `PhysicWorld::Splines`; the module keeps the
//! spline index. The cursor point/tangent cache bits (+0x68 bits 0/1) are a memo of
//! GetPoint/GetTangent at the cursor: recomputed on demand here (same values).
#![allow(non_snake_case)]

use crate::engine::physic::module::*;
use crate::engine::physic::rwm::*;
use crate::engine::spline::{EFQuadraticSpline, EFSplineCursor};

/// `(bits & 0x7f800000) < lim` tiny test used by the module
fn tiny(x: f32, lim: u32) -> bool { (x.to_bits() & 0x7f80_0000) < lim }
fn tiny3(v: V3, lim: u32) -> bool { tiny(v[0] - 0.0, lim) && tiny(v[1] - 0.0, lim) && tiny(v[2] - 0.0, lim) }
/// dot in the original's summation order ((z + x) + y)
fn dotz(a: V3, b: V3) -> f32 { a[2] * b[2] + a[0] * b[0] + a[1] * b[1] }

pub struct EFDMSplineMove {
    pub base: EFPhysicModule,
    /// +0x30 / +0x34 input actions b (offset input, speed input), +0x38 input action a
    pub action_offset: u32,
    pub action_speed: u32,
    pub action_a: u32,
    /// +0x3c max offset length (2.0)
    pub max_offset: f32,
    /// +0x40 max speed (2.0), +0x44 min speed (0)
    pub max_speed: f32,
    pub min_speed: f32,
    /// +0x48 offset speed (2.0)
    pub offset_speed: f32,
    /// +0x4c local offset from the spline point
    pub offset: V3,
    /// +0x58 target position (spline point + offset)
    pub target: V3,
    /// +0x64 (2.0, unread)
    pub unk64: f32,
    /// +0x6c spline (index into `PhysicWorld::Splines`; None = NULL)
    pub spline: Option<usize>,
    /// +0x70 global factor, +0x74 segment (-1 at start), +0x78 t
    pub cursor: EFSplineCursor,
    /// +0x94 max tilt (0.70701), +0x98 tilt speed (0.70701)
    pub max_tilt: f32,
    pub tilt_speed: f32,
    /// +0x9c / +0xa0 tilt x / y
    pub tilt: [f32; 2],
    /// +0xa4 / +0xa8 / +0xac tangent-angle factors (right / up / tangent axis), 1.0
    pub factors: V3,
    /// +0xb0 flags (0x2d): 1/2/4 rotate by tangent angle about at/up/right, 8/0x10/0x20 tilt,
    /// 0x40 align frame to the tangent, 0x80 no MoveToNearest, 0x100 slow down with tilt
    pub flags: u32,
    /// +0xb4 mode (u8, 1): 0 = planar (no y), 1 = vertical offset + velocity override
    pub mode: u8,
}

impl EFDMSplineMove {
    /// 0x0023b850
    pub fn new() -> Self {
        let mut s = EFDMSplineMove {
            base: EFPhysicModule { type_: 0x5000_0003, flags: 1, ..Default::default() },
            action_offset: 0,
            action_speed: 0,
            action_a: 0,
            max_offset: 0.0,
            max_speed: 0.0,
            min_speed: 0.0,
            offset_speed: 0.0,
            offset: [0.0; 3],
            target: [0.0; 3],
            unk64: 0.0,
            spline: None,
            cursor: EFSplineCursor { global: 0.0, seg: 0xffff_ffff, t: 0.0 },
            max_tilt: 0.0,
            tilt_speed: 0.0,
            tilt: [0.0; 2],
            factors: [0.0; 3],
            flags: 0,
            mode: 0,
        };
        s.SetDefaultParams();
        s
    }

    /// 0x0023ba60
    pub fn SetDefaultParams(&mut self) {
        self.offset = [0.0; 3];
        self.unk64 = 2.0;
        self.max_tilt = f32::from_bits(0x3f34_fdf4);
        self.tilt_speed = f32::from_bits(0x3f34_fdf4);
        self.tilt = [0.0; 2];
        self.action_offset = 0xffff_ffff;
        self.action_speed = 0xffff_ffff;
        self.action_a = 0xffff_ffff;
        self.min_speed = 0.0;
        self.max_speed = 2.0;
        self.max_offset = 2.0;
        self.offset_speed = 2.0;
        self.flags = 0x2d;
        self.mode = 1;
        self.factors = [1.0; 3];
    }

    fn point(&self, sp: &EFQuadraticSpline) -> V3 { sp.GetPoint(self.cursor.seg as i32, self.cursor.t) }
    fn tangent(&self, sp: &EFQuadraticSpline) -> V3 { sp.GetTangent(self.cursor.seg as i32, self.cursor.t) }

    /// 0x0023d5e0: analog input -> camera space vector
    pub fn ProjectInputs(&self, ctx: &DMCtx, mut x: f32, mut y: f32) -> V3 {
        let cam = ctx.world.GetCameraLTM();
        let l = (y * y + x * x).sqrt();
        if 1.0 < l {
            x /= l;
            y /= l;
        }
        let mut v = [0.0f32; 3];
        let a = (cam.at[2] * 0.0 + cam.at[0] * 0.0 + cam.at[1] * 1.0).abs();
        match self.mode {
            0 => v = [-x, y * a, y * (1.0 - a)],
            1 => v = [-x, y * (1.0 - a), y * a],
            _ => {}
        }
        cam.TransformVector(v)
    }

    /// 0x0023cac0: signed angle between the tangent and the chord to global+0.01 (xz plane)
    pub fn GetTangentAngle(&self, sp: &EFQuadraticSpline) -> f32 {
        let mut tg = self.tangent(sp);
        let f = sp.AdjustGlobalFactor(self.cursor.global + 0.01) * sp.nseg() as f32;
        let i = if f < 2.1474836e9 { f as u32 } else { ((f - 2.1474836e9) as i32 as u32) | 0x8000_0000 };
        let mut d = sp.GetPoint(i as i32, f - i as f32);
        let p = self.point(sp);
        d[0] -= p[0];
        d[2] -= p[2];
        let mut r = 0.0f32;
        // quirk: the y component is tested as (d.y - p.y) but never stored
        if tiny3([d[0], d[1] - p[1], d[2]], 0x3c80_0000) || tiny3(tg, 0x3c80_0000) {
            return r;
        }
        tg[1] = 0.0;
        d[1] = 0.0;
        tg = RwV3dNormalize(tg);
        d = RwV3dNormalize(d);
        let mut c = cross(d, tg);
        let s = RwV3dLength(c);
        let dt = d[2] * tg[2] + d[0] * tg[0] + d[1] * tg[1];
        if tiny(s, 0x3800_0000) {
            r = if dt <= 0.0 { std::f32::consts::PI } else { 0.0 };
        } else {
            c = scale(c, 1.0 / s);
            r = if 1.0 <= s { 1.5707964 } else if s <= -1.0 { -1.5707964 } else { s.asin() };
            if dt < 0.0 {
                r = 3.1415927 - r;
            }
        }
        if c[1] < 0.0 {
            r = -r;
        }
        r
    }
}

fn rotate_keep_pos(m: &mut RwMatrix, deg: f32, axis: V3) {
    // quirk: without flag 0x40 the tangent/right axes stay (0,0,0) in the original (RW would
    // normalise a null axis); a null-axis rotation is skipped here.
    if axis == [0.0; 3] {
        return;
    }
    let pos = m.pos;
    m.Rotate(deg, axis, rwCOMBINEPOSTCONCAT);
    m.pos = pos;
}

impl EFDM for EFDMSplineMove {
    fn base(&self) -> &EFPhysicModule { &self.base }
    fn base_mut(&mut self) -> &mut EFPhysicModule { &mut self.base }
    fn ParamWords(&self) -> usize { 15 }

    /// 0x0023bc00
    fn PreColUpdate(&mut self, ctx: &mut DMCtx, dp: &mut EFDynamicPhysic, dt: f32) -> bool {
        let Some(si) = self.spline else { return true };
        let world = ctx.world;
        let Some(sp) = world.Splines().get(si) else { return true };
        let mut speed = self.min_speed;
        let mut delta = [0.0f32; 3];
        let in1 = world.InputActionGet(self.action_a, self.action_offset);
        let in2 = world.InputActionGet(self.action_a, self.action_speed);
        let (mut ix, mut iy) = (0.0f32, 0.0f32);
        if let Some([x, y]) = in1 {
            let proj = self.ProjectInputs(ctx, x, y);
            let up = [0.0f32, 1.0, 0.0];
            let t = RwV3dNormalize(self.tangent(sp));
            let side = cross(t, up);
            ix = x.abs() * dotz(side, proj);
            iy = if self.mode == 1 { y.abs() * dotz(up, proj) } else { y.abs() * dotz(t, proj) };
            delta[0] = dt * -ix * self.offset_speed;
            if self.mode == 1 {
                delta[1] = dt * iy * self.offset_speed;
            }
            self.offset = add(self.offset, delta);
            let l = RwV3dLength(self.offset);
            if self.max_offset < l {
                self.offset = scale(RwV3dNormalize(self.offset), self.max_offset);
            }
            if self.mode == 0 {
                speed = (self.max_speed + self.min_speed) / 2.0 + (iy * (self.max_speed - self.min_speed)) / 2.0;
            }
        }
        if let Some([x, y]) = in2 {
            // InputActionGet result word 2 bit 0x20 selects "max speed": the PhysicWorld seam
            // only returns {x, y}, so that word is taken as 0.
            let t = self.tangent(sp);
            let proj = self.ProjectInputs(ctx, x, y);
            speed = (self.max_speed + self.min_speed) / 2.0 + ((y.abs() * dotz(t, proj)) * (self.max_speed - self.min_speed)) / 2.0;
        }
        if self.flags & 0x100 != 0 {
            speed *= 1.0 - self.tilt[0].abs() / 1.5707964;
        }
        let pos = ctx.phys.frame.pos;
        let mut up = [0.0f32, 1.0, 0.0];
        let mut at = [0.0f32; 3];
        let mut right = [0.0f32; 3];
        if self.flags & 0x80 == 0 {
            self.cursor.MoveToNearest(sp, pos);
        }
        self.cursor.MoveDistance(sp, speed * dt);
        if self.flags & 0x40 != 0 {
            at = RwV3dNormalize(self.tangent(sp));
            let k = up[2] * at[2] + up[0] * at[0] + up[1] * at[1];
            up = RwV3dNormalize(sub(up, scale(at, k)));
            right = cross(up, at);
            let f = &mut ctx.phys.frame;
            f.at = at;
            f.up = up;
            f.right = right;
        }
        let w = ctx.phys.frame.TransformVector(self.offset);
        self.target = add(self.point(sp), w);
        let mut v = sub(self.target, pos);
        if self.mode == 0 {
            v[1] = 0.0;
        }
        dp.disp = add(dp.disp, scale(v, 1.0 / dt));
        if self.mode == 1 {
            let p = &mut *ctx.phys;
            p.velocity = dp.disp;
            p.falling_timer = p.falling_delay;
        }
        if ix != 0.0 || iy != 0.0 {
            self.tilt[0] += dt * self.tilt_speed * ix;
            self.tilt[1] += dt * self.tilt_speed * iy;
            for i in 0..2 {
                let m = self.max_tilt;
                let mut v = self.tilt[i];
                if v <= -m {
                    v = -m;
                }
                self.tilt[i] = if m < v { m } else { v };
            }
        }
        for i in 0..2 {
            self.tilt[i] = if tiny(self.tilt[i], 0x3c00_0000) { 0.0 } else { self.tilt[i] / (dt * self.tilt_speed * 0.3 + 1.0) };
        }
        let ang = self.GetTangentAngle(sp);
        let f = &mut ctx.phys.frame;
        if self.flags & 1 != 0 {
            rotate_keep_pos(f, ang * self.factors[2] * 57.29578, at);
        }
        if self.flags & 8 != 0 {
            rotate_keep_pos(f, self.tilt[0] * 57.29578, at);
        }
        if self.flags & 2 != 0 {
            rotate_keep_pos(f, ang * self.factors[1] * 57.29578, up);
        }
        if self.flags & 0x10 != 0 {
            rotate_keep_pos(f, -self.tilt[0] * 57.29578, up);
        }
        if self.flags & 4 != 0 {
            rotate_keep_pos(f, ang * self.factors[0] * 57.29578, right);
        }
        if self.flags & 0x20 != 0 {
            rotate_keep_pos(f, -self.tilt[1] * 57.29578, right);
        }
        if 1.0 <= self.cursor.global {
            self.base.SetStateBits(2, ctx);
        } else {
            self.base.ResetStateBits(2, ctx);
        }
        true
    }

    /// 0x0023cff0: after a collision (stage+8 != 0, here: a stage is given) pulled the owner
    /// more than 0.01 off the target, the offset keeps its direction with the real length.
    fn PostColUpdate(&mut self, ctx: &mut DMCtx, stage: Option<&EFCollisionStage>, _dt: f32) {
        if stage.is_none() {
            return;
        }
        let Some(sp) = self.spline.and_then(|i| ctx.world.Splines().get(i)) else { return };
        let pos = ctx.phys.frame.pos;
        let mut d = sub(self.target, pos);
        if self.mode == 0 {
            d[1] = 0.0;
        }
        if 0.01 < RwV3dLength(d) {
            self.target = self.point(sp);
            let mut d = sub(pos, self.target);
            if self.mode == 0 {
                d[1] = 0.0;
            }
            if !tiny3(self.offset, 0x3c80_0000) {
                self.offset = scale(RwV3dNormalize(self.offset), RwV3dLength(d));
            }
        }
    }

    /// 0x0023bae0
    fn SetParams(&mut self, _ctx: &mut DMCtx, p: Option<&[u32]>) {
        let Some(p) = p else { return self.SetDefaultParams() };
        self.action_offset = p[1];
        self.action_speed = p[2];
        self.action_a = p[0];
        self.min_speed = wf(p, 4);
        self.max_speed = wf(p, 5);
        self.max_offset = wf(p, 3);
        self.offset_speed = wf(p, 6);
        self.max_tilt = wf(p, 7);
        self.tilt_speed = wf(p, 8);
        self.flags = p[10];
        self.mode = p[11] as u8;
        self.factors = wv(p, 12);
    }

    /// 0x0023bb80 (word 9 untouched)
    fn GetParams(&self, _ctx: &DMCtx, p: &mut [u32]) {
        p[1] = self.action_offset;
        p[2] = self.action_speed;
        p[0] = self.action_a;
        p[3] = fw(self.max_offset);
        p[4] = fw(self.min_speed);
        p[5] = fw(self.max_speed);
        p[6] = fw(self.offset_speed);
        p[7] = fw(self.max_tilt);
        p[8] = fw(self.tilt_speed);
        p[10] = self.flags;
        p[11] = self.mode as u32;
        put_v(p, 12, self.factors);
    }

    /// 0x0023d240: 1 = {spline index, from start}, 2 = {spline id, from start}: attach and
    /// snap the frame onto the spline (nearest point, or global 0 when word 1 != 0).
    fn DoAction(&mut self, ctx: &mut DMCtx, action: u32, p: &mut [u32]) {
        let world = ctx.world;
        let splines = world.Splines();
        match action {
            2 => {
                // quirk: no break, the last spline with that id wins
                for (i, s) in splines.iter().enumerate() {
                    if p[0] == s.id as u32 {
                        self.spline = Some(i);
                        self.cursor = EFSplineCursor { global: 0.0, seg: 0xffff_ffff, t: 0.0 };
                    }
                }
            }
            1 => {
                self.spline = Some(p[0] as usize);
                self.cursor = EFSplineCursor { global: 0.0, seg: 0xffff_ffff, t: 0.0 };
            }
            _ => return,
        }
        // C++ dereferences a NULL / out-of-range spline here; the port returns.
        let Some(sp) = self.spline.and_then(|i| splines.get(i)) else { return };
        if p.get(1).copied().unwrap_or(0) == 0 {
            let pos = ctx.phys.frame.pos;
            self.cursor.SetToNearest(sp, pos);
        } else {
            self.cursor.MoveTo(sp, 0.0);
        }
        let f = &mut ctx.phys.frame;
        // frame pos = 0 then RwFrameTranslate(point, POSTCONCAT)
        f.pos = [0.0; 3];
        f.pos = add(f.pos, self.point(sp));
        f.at = RwV3dNormalize(self.tangent(sp));
        f.up = [0.0, 1.0, 0.0];
        let k = f.up[2] * f.at[2] + f.up[0] * f.at[0] + f.up[1] * f.at[1];
        f.up = RwV3dNormalize(sub(f.up, scale(f.at, k)));
        f.right = cross(f.up, f.at);
        self.offset = [0.0; 3];
        self.tilt = [0.0; 2];
    }
}

//! 0x10 EFDMPFWeightTilt (Classes/EFDM/EFDMPFWeightTilt.cpp, 0xcc bytes), 1:1: a platform that
//! tilts under the weight of the physics standing on it (spring-damper on two axes).
//! Seam: EFColDMData has no touched-physic identity/weight/frame, so the ColUpdate body is
//! `Contact(frame, normal_y, other id, other weight (+0x24), other LTM pos)`; the trait ColUpdate
//! is a no-op.
#![allow(non_snake_case)]

use crate::engine::physic::module::*;
use crate::engine::physic::rwm::*;

pub struct EFDMPFWeightTilt {
    pub base: EFPhysicModule,
    /// +0x30/+0x34 min/max tilt about the right axis (rad)
    pub min_a: f32,
    pub max_a: f32,
    /// +0x38/+0x3c min/max tilt about the at axis
    pub min_b: f32,
    pub max_b: f32,
    /// +0x40 weight gain a, +0x44 spring a, +0x48 damping a, +0x4c/+0x50/+0x54 same for b
    pub k: [f32; 6],
    /// +0x58/+0x5c angular velocities
    pub vel_a: f32,
    pub vel_b: f32,
    /// +0x60 frame of the contact list
    pub frame: u32,
    /// +0x64 contact count (max 5)
    pub count: u32,
    /// +0x68 weights, +0x7c physics, +0x90 positions
    pub weights: [f32; 5],
    pub others: [u32; 5],
    pub pos: [V3; 5],
}

impl EFDMPFWeightTilt {
    pub fn new() -> Self {
        EFDMPFWeightTilt {
            base: EFPhysicModule { type_: 0x3000_0010, flags: 1, ..Default::default() },
            min_a: 0.0,
            max_a: 0.0,
            min_b: 0.0,
            max_b: 0.0,
            k: [1.0; 6],
            vel_a: 0.0,
            vel_b: 0.0,
            frame: 0,
            count: 0,
            weights: [0.0; 5],
            others: [0; 5],
            pos: [[0.0; 3]; 5],
        }
    }

    /// GetWeightFactors(&x, &z): weighted sum of the contacts' local x / z (only when the list is
    /// from the previous frame).
    pub fn GetWeightFactors(&self, ctx: &DMCtx, want_x: bool, want_z: bool) -> (f32, f32) {
        let (mut x, mut z) = (0.0f32, 0.0f32);
        if self.count != 0 && self.frame.wrapping_add(1) == ctx.frame {
            let inv = ctx.phys.frame.Invert();
            let n = self.count as usize;
            let loc: Vec<V3> = self.pos[..n].iter().map(|&p| inv.TransformPoint(p)).collect();
            if want_x {
                for i in 0..n {
                    x += self.weights[i] * loc[i][0];
                }
            }
            if want_z {
                for i in 0..n {
                    z += self.weights[i] * loc[i][2];
                }
            }
        }
        (x, z)
    }

    /// ColUpdate body (d+0x1c != NULL).
    pub fn Contact(&mut self, ctx: &DMCtx, normal_y: f32, other: u32, weight: f32, pos: V3) {
        if self.frame == ctx.frame {
            if self.count == 5 {
                return;
            }
        } else {
            self.count = 0;
            self.frame = ctx.frame;
        }
        if normal_y <= -ctx.phys.floor_cos {
            let n = self.count as usize;
            // quirk: the duplicate test compares others[count] (not others[i]) on every iteration
            for _ in 0..n {
                if n < 5 && self.others[n] == other {
                    return;
                }
            }
            self.weights[n] = weight;
            self.others[n] = other;
            self.pos[n] = pos;
            self.count += 1;
        }
    }
}

fn spring(vel: &mut f32, ang: f32, force: f32, damp: f32, dt: f32, lo: f32, hi: f32) -> f32 {
    *vel += force * dt;
    let d = (1.0 - damp * dt).max(0.001).min(0.999);
    let v = *vel;
    *vel = v * d;
    let t = ang + v * d * dt;
    let c = if t <= hi && lo <= t { t } else if t <= hi { lo } else { hi };
    c - ang
}

impl EFDM for EFDMPFWeightTilt {
    fn base(&self) -> &EFPhysicModule { &self.base }
    fn base_mut(&mut self) -> &mut EFPhysicModule { &mut self.base }
    fn ParamWords(&self) -> usize { 10 }
    fn SetParams(&mut self, _c: &mut DMCtx, p: Option<&[u32]>) {
        match p {
            None => {
                (self.min_a, self.max_a, self.min_b, self.max_b) = (0.0, 0.0, 0.0, 0.0);
                self.k = [1.0; 6];
            }
            Some(p) => {
                (self.min_a, self.max_a, self.min_b, self.max_b) = (wf(p, 0), wf(p, 1), wf(p, 2), wf(p, 3));
                for i in 0..6 {
                    self.k[i] = wf(p, 4 + i);
                }
            }
        }
    }
    fn GetParams(&self, _c: &DMCtx, p: &mut [u32]) {
        for (i, v) in [self.min_a, self.max_a, self.min_b, self.max_b].into_iter().chain(self.k).enumerate() {
            p[i] = fw(v);
        }
    }
    fn PreColUpdate(&mut self, ctx: &mut DMCtx, _dp: &mut EFDynamicPhysic, dt: f32) -> bool {
        let ta = self.min_a != self.max_a;
        let tb = self.min_b != self.max_b;
        if !(ta || tb) {
            return true;
        }
        let (x, z) = self.GetWeightFactors(ctx, tb, ta);
        // the LTM pointer is read once; RwFrameUpdateObjects only dirties it, so both axes use the
        // pre-rotation LTM
        let m0 = ctx.phys.frame.clone();
        let a = -m0.at[1].asin();
        let d = spring(&mut self.vel_a, a, -a * self.k[1] + z * self.k[0], self.k[2], dt, self.min_a, self.max_a);
        if (d.abs() as f64) > 1e-4 {
            let pos = ctx.phys.frame.pos;
            ctx.phys.frame.Rotate(d * RAD2DEG, m0.right, 2);
            ctx.phys.frame.pos = pos;
        }
        let b = m0.right[1].asin();
        let d = spring(&mut self.vel_b, b, -b * self.k[4] - x * self.k[3], self.k[5], dt, self.min_b, self.max_b);
        if (d.abs() as f64) > 1e-4 {
            let pos = ctx.phys.frame.pos;
            ctx.phys.frame.Rotate(d * RAD2DEG, m0.at, 2);
            ctx.phys.frame.pos = pos;
        }
        true
    }
}

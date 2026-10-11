//! EFDMFollow (Classes/EFDM/EFDMFollow.cpp, DMAdd kind 0x14, type 0x50000014 = PRE|POST, 0x1b4
//! bytes), 1:1. Follows a leader object along a breadcrumb trail (25 recorded leader positions)
//! and turns the owner toward the follow point. Used by the hero (heroFUNC_CreateFollowPhysic).
#![allow(non_snake_case)]

use crate::engine::anim::quat::DTQuaternion;
use crate::engine::physic::collision_map::RayIntersectIn;
use crate::engine::physic::mgr::EFRayIntersectIn;
use crate::engine::physic::module::*;
use crate::engine::physic::rwm::*;

pub const LEAD_COUNT: usize = 25;

pub struct EFDMFollow {
    pub base: EFPhysicModule,
    /// +0x30 leader object uid (param 0)
    pub leader: u32,
    /// +0x34 follow distance behind the leader (param 1, 1.0)
    pub distance: f32,
    /// +0x38 "close enough" distance while state bit0 is set (param 2, 2.0)
    pub stop_distance: f32,
    /// +0x3c rotation time (DeltaForce, param +0x10, 0.3)
    pub rot_time: f32,
    /// +0x40 max speed (param +0x14, 7.0)
    pub max_speed: f32,
    /// +0x44 min speed (param +0x18, 0.1)
    pub min_speed: f32,
    /// +0x48 ray object mask (RayIntersectIn+0x1c, param +0x1c, set only)
    pub ray_mask: u32,
    /// +0x4c..+0x178 leader position ring (25 x V3)
    pub lead: [V3; LEAD_COUNT],
    /// +0x178 ring write index
    pub lead_index: i32,
    /// +0x17c last physic frame counter
    pub frame: u32,
    /// +0x180 owner position
    pub pos: V3,
    /// +0x18c leader position
    pub leader_pos: V3,
    /// +0x198 follow target point
    pub target: V3,
    /// +0x1a4 primary prim radius (prim+0xbc)
    pub prim_radius: f32,
    /// +0x1a8 stuck frame counter
    pub stuck_count: i32,
    /// +0x1ac stuck flag (u8)
    pub stuck: bool,
    /// +0x1b0 physic floor slope cosine (EFPhysic+0x50; stored, unread)
    pub floor_cos: f32,
    /// quirk: `sbFirstUpdate` (0x00343908) is a process-wide static in the original: only the very
    /// first EFDMFollow ever updated calls Reset() (later ones start with a zeroed ring). Kept
    /// per instance here (no global state allowed); set `first_update = false` on creation to get
    /// the exact original behaviour for a non-first module.
    pub first_update: bool,
}

impl EFDMFollow {
    pub fn new() -> Self {
        let mut s = EFDMFollow {
            base: EFPhysicModule { type_: 0x5000_0014, flags: 1, ..Default::default() },
            leader: 0,
            distance: 0.0,
            stop_distance: 0.0,
            rot_time: 0.0,
            max_speed: 0.0,
            min_speed: 0.0,
            ray_mask: 0,
            lead: [[0.0; 3]; LEAD_COUNT],
            lead_index: 0,
            frame: 0,
            pos: [0.0; 3],
            leader_pos: [0.0; 3],
            target: [0.0; 3],
            prim_radius: 0.0,
            stuck_count: 0,
            stuck: false,
            floor_cos: 0.0,
            first_update: true,
        };
        s.SetDefaultParams();
        s
    }

    /// 0x001f6920
    pub fn SetDefaultParams(&mut self) {
        self.leader = 0;
        self.distance = 1.0;
        self.stop_distance = 2.0;
        self.rot_time = 0.3;
        self.min_speed = 0.1;
        self.max_speed = 7.0;
        self.stuck = false;
        self.frame = 0;
        self.target = [0.0; 3];
        self.lead_index = 0;
        self.stuck_count = 0;
        self.floor_cos = f32::from_bits(0x3f35_0481);
        self.ray_mask = 0;
    }

    /// 0x00234080
    pub fn Reset(&mut self) {
        self.lead = [self.leader_pos; LEAD_COUNT];
        self.lead_index = 0;
    }

    /// 0x002341b0: record the leader position (+0.5 y) when it moved >= 0.25 from the last one.
    pub fn UpdateLeadPosArray(&mut self, p: V3) -> bool {
        let last = if self.lead_index == 0 { self.lead[24] } else { self.lead[(self.lead_index - 1) as usize] };
        let d = [last[0] - p[0], last[1] - (p[1] + 0.5), last[2] - p[2]];
        if 0.0625 <= d[2] * d[2] + d[0] * d[0] + d[1] * d[1] {
            self.lead[self.lead_index as usize] = [p[0], p[1] + 0.5, p[2]];
            self.lead_index += 1;
            if 0x18 < self.lead_index {
                self.lead_index = 0;
            }
            return true;
        }
        false
    }

    /// 0x00234340: ray from the owner (+0.5 y) along `d` extended by the prim radius.
    pub fn IsIntersecting(&self, ctx: &DMCtx, d: V3) -> bool {
        let mut delta = [0.0f32; 3];
        let l = RwV3dLength(d);
        if 0.0 < l {
            let k = self.prim_radius / l + 1.0;
            delta = scale(d, k);
        }
        let rin = EFRayIntersectIn {
            map: RayIntersectIn { origin: [self.pos[0], self.pos[1] + 0.5, self.pos[2]], dir: delta, mask1: 3, mask2: 0 },
            obj_mask1: 0,
            obj_mask2: self.ray_mask,
            exclude: ctx.phys.owner,
        };
        ctx.world.RayIntersect(&rin)
    }

    /// 0x00233bc0
    pub fn DeltaForce(t: f32, dt: f32) -> f32 {
        let mut f = 1.0;
        if 1e-6 <= t {
            f = (1.0 / t) * dt;
            if 1.0 < f {
                f = 1.0;
            } else if f <= 1e-6 {
                f = 0.0;
            }
        }
        f
    }

    /// 0x00233d90: look-at quaternion from `from` toward `to` (up = +y).
    pub fn GetLookAt(from: V3, to: V3) -> [f32; 4] {
        let at = RwV3dNormalize(sub(to, from));
        let up0 = [0.0f32, 1.0, 0.0];
        let right = cross(up0, at);
        let up = cross(at, right);
        let m = RwMatrix { right: RwV3dNormalize(right), up: RwV3dNormalize(up), at, pos: [0.0; 3] };
        MatToQuat(&m)
    }

    /// 0x00233c30: lerp the owner frame's rotation toward the look-at of the target.
    pub fn LookAtTarget(&self, frame: &mut RwMatrix, dt: f32) {
        let cur = MatToQuat(frame);
        let mut q = Self::GetLookAt(self.pos, self.target);
        Normalise(&mut q);
        let f = Self::DeltaForce(self.rot_time, dt);
        let r = DTQuaternion::Lerp(&cur, &q, f);
        let mut m = QuatToMat(&r);
        // quirk: the up row of the lerped matrix is replaced by the frame's old up before the
        // orthonormalisation; the frame position is untouched.
        m.up = frame.up;
        let m = m.OrthoNormalize();
        frame.at = m.at;
        frame.up = m.up;
        frame.right = m.right;
    }

    fn set_bits(&mut self, bits: u32, ctx: &mut DMCtx) { self.base.SetStateBits(bits, ctx) }
    fn reset_bits(&mut self, bits: u32, ctx: &mut DMCtx) { self.base.ResetStateBits(bits, ctx) }

    /// 0x00233870
    pub fn UpdateVelocity(&mut self, ctx: &mut DMCtx, dp: &mut EFDynamicPhysic, dt: f32) {
        let v = scale(sub(self.target, self.pos), 1.0 / dt);
        let mut speed = RwV3dLength(v);
        let mut v = v;
        let d = sub(self.leader_pos, self.pos);
        if speed < self.min_speed || d[2] * d[2] + d[0] * d[0] + d[1] * d[1] < self.distance * self.distance {
            self.set_bits(1, ctx);
        } else {
            if self.max_speed < speed {
                v = RwV3dNormalize(v);
                speed = self.max_speed;
                v = scale(v, speed);
            }
            self.target[1] = self.pos[1];
            let mut frame = ctx.phys.frame;
            self.LookAtTarget(&mut frame, dt);
            ctx.phys.frame = frame;
            self.reset_bits(1, ctx);
            dp.disp = add(dp.disp, v);
        }
    }
}

/// DTQuaternion::QuatToMat (0x0010e210), rows right/up/at.
pub fn QuatToMat(q: &[f32; 4]) -> RwMatrix {
    let (x, y, z, w) = (q[0], q[1], q[2], q[3]);
    let (z2, y2) = (z + z, y + y);
    let xx = x * (x + x);
    let wx = w * (x + x);
    let xz = x * z2;
    let yz = y * z2;
    let wy = w * y2;
    RwMatrix {
        right: [1.0 - (y * y2 + z * z2), x * y2 + w * z2, xz - wy],
        up: [x * y2 - w * z2, 1.0 - (xx + z * z2), yz + wx],
        at: [xz + wy, yz - wx, 1.0 - (xx + y * y2)],
        pos: [0.0; 3],
    }
}

/// PS2 FPU division: no inf/NaN, x/0 gives ±MAX (so MAX * 0 = 0 downstream).
fn ps2_div(a: f32, b: f32) -> f32 {
    if b == 0.0 { f32::MAX.copysign(a) * if b.is_sign_negative() { -1.0 } else { 1.0 } } else { a / b }
}

/// DTQuaternion::MatToQuat (0x0010e380).
/// quirk: in the trace <= 0 path the three branches are sequential `if`s (as decompiled) and
/// the z branch is unconditional, so it always overwrites the result.
pub fn MatToQuat(m: &RwMatrix) -> [f32; 4] {
    let mut q = [0.0f32; 4];
    let tr = m.at[2] + m.right[0] + m.up[1];
    if tr > 0.0 {
        let s = (tr + 1.0).sqrt();
        let f = 0.5 / s;
        q[3] = s / 2.0;
        q[0] = f * (-m.at[1] + m.up[2]);
        q[1] = f * (-m.right[2] + m.at[0]);
        q[2] = f * (m.right[1] - m.up[0]);
        return q;
    }
    if m.right[0] < m.up[1] && m.at[2] < m.up[1] {
        q[1] = (((m.up[1] - m.at[2]) - m.right[0]) + 1.0).abs().sqrt() / 2.0;
        let f = ps2_div(1.0, q[1] * 4.0);
        q[3] = f * (-m.right[2] + m.at[0]);
        q[0] = f * (m.right[1] + m.up[0]);
        q[2] = f * (m.at[1] + m.up[2]);
    }
    if m.up[1] < m.right[0] && m.at[2] < m.right[0] {
        q[0] = (((m.right[0] - m.up[1]) - m.at[2]) + 1.0).abs().sqrt() / 2.0;
        let f = ps2_div(1.0, q[0] * 4.0);
        q[3] = f * (-m.at[1] + m.up[2]);
        q[1] = f * (m.right[1] + m.up[0]);
        q[2] = f * (m.right[2] + m.at[0]);
    }
    q[2] = (((m.at[2] - m.right[0]) - m.up[1]) + 1.0).abs().sqrt() / 2.0;
    let f = ps2_div(1.0, q[2] * 4.0);
    q[3] = f * (-m.up[0] + m.right[1]);
    q[0] = f * (m.right[2] + m.at[0]);
    q[1] = f * (m.at[1] + m.up[2]);
    q
}

/// DTQuaternion::Normalise (0x0010ebd0)
pub fn Normalise(q: &mut [f32; 4]) {
    let l = q[2] * q[2] + q[1] * q[1] + q[0] * q[0] + q[3] * q[3];
    if l > 0.0 {
        let s = 1.0 / l.sqrt();
        q.iter_mut().for_each(|x| *x *= s);
    }
}

impl EFDM for EFDMFollow {
    fn base(&self) -> &EFPhysicModule { &self.base }
    fn base_mut(&mut self) -> &mut EFPhysicModule { &mut self.base }
    fn ParamWords(&self) -> usize { 8 }

    /// 0x00233300
    fn PreColUpdate(&mut self, ctx: &mut DMCtx, dp: &mut EFDynamicPhysic, dt: f32) -> bool {
        let Some(lf) = ctx.world.GetObjectFrame(self.leader) else { return true };
        self.leader_pos = lf.pos;
        self.pos = ctx.phys.frame.pos;
        if self.first_update {
            self.first_update = false;
            self.Reset();
        }
        self.frame = ctx.frame;
        if self.frame % 5 == 0 {
            let p = self.leader_pos;
            self.UpdateLeadPosArray(p);
        }
        if self.base.state & 1 != 0 && RwV3dLength(sub(self.pos, self.leader_pos)) <= self.stop_distance {
            return true;
        }
        let mut d = sub(self.leader_pos, self.pos);
        if d != [0.0; 3] {
            d = RwV3dNormalize(d);
        }
        self.target = sub(self.leader_pos, scale(d, self.distance));
        let mut idx = self.lead_index;
        let mut n = 0;
        loop {
            let r = sub(self.target, self.pos);
            if !self.IsIntersecting(ctx, r) || 0x19 < n {
                break;
            }
            n += 1;
            idx = if idx == 0 { 0x18 } else { idx - 1 };
            self.target = self.lead[idx as usize];
        }
        // quirk: "reached" when x OR z equals the owner's (exact float compare)
        if self.target[0] == self.pos[0] || self.target[2] == self.pos[2] {
            self.set_bits(1, ctx);
        } else {
            self.stuck = false;
            self.reset_bits(2, ctx);
            self.UpdateVelocity(ctx, dp, dt);
        }
        if self.stuck {
            self.set_bits(2, ctx);
        }
        true
    }

    /// 0x00233b20: 10 frames away from the target (> 0.001 on x or z) -> stuck.
    fn PostColUpdate(&mut self, ctx: &mut DMCtx, _stage: Option<&EFCollisionStage>, _dt: f32) {
        let p = ctx.phys.frame.pos;
        if 0.001 < self.target[0] - p[0] || 0.001 < self.target[2] - p[2] {
            self.stuck_count += 1;
            if self.stuck_count == 10 {
                self.stuck = true;
                self.stuck_count = 0;
            }
        } else {
            self.stuck_count = 0;
        }
    }

    /// 0x002331c0
    fn SetParams(&mut self, ctx: &mut DMCtx, p: Option<&[u32]>) {
        match p {
            None => self.SetDefaultParams(),
            Some(p) => {
                self.floor_cos = ctx.phys.floor_cos;
                self.leader = p[0];
                self.distance = wf(p, 1);
                self.stop_distance = wf(p, 2);
                self.rot_time = wf(p, 4);
                self.max_speed = wf(p, 5);
                self.min_speed = wf(p, 6);
                self.ray_mask = p[7];
                // quirk: with no primary prim the C++ reads address 0xbc (uRam000000bc); 0 here
                let m = &ctx.phys.model;
                self.prim_radius = if m.primary != -1 { m.prims.get(m.primary as usize).and_then(|q| q.as_ref()).map_or(0.0, |q| q.base.radius) } else { 0.0 };
            }
        }
    }

    /// 0x002332c0 (word 3 and 7 untouched)
    fn GetParams(&self, _ctx: &DMCtx, p: &mut [u32]) {
        p[0] = self.leader;
        p[1] = fw(self.distance);
        p[2] = fw(self.stop_distance);
        p[4] = fw(self.rot_time);
        p[5] = fw(self.max_speed);
        p[6] = fw(self.min_speed);
    }

    /// 0x002344d0: action 1 = Reset
    fn DoAction(&mut self, _ctx: &mut DMCtx, action: u32, _p: &mut [u32]) {
        if action == 1 {
            self.Reset();
        }
    }
}

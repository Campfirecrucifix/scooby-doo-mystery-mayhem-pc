//! 0x18 EFDMGrabEdge (Classes/EFDM/EFDMGrabEdge.cpp, 0xe8 bytes), 1:1: grabs the nearest flagged
//! env polygon edge within `radius` of a prim of the owner (reach phase: turn + slide onto it),
//! then shuffles along it with the analog input (factor along the edge) and transitions onto the
//! adjacent edge sharing a vertex (rotating around the corner).
//!
//! Seams: the trait's `EFCollisionStage` (module.rs) carries no env prim list, so the body
//! `GrabEdge` takes the stage's touched env prims (stage+4/+8) as `objcoll_check::EnvPrim`s; the
//! trait PostColUpdate runs it with none. Input = `PhysicWorld::InputActionGet`, camera =
//! `PhysicWorld::GetCameraLTM`. The prim vtbl+0xc (PrecalcPrimData) calls resync root-frame
//! prims (joint 0xffffffff) from the owner frame first (see edge_down::sync_prim_ltm).
//! Hidden `this` operands of Ghidra's `DTVector3d::operator*(result, k)` were reconstructed from
//! the surrounding data flow (marked `// operand:`).
#![allow(non_snake_case)]

use super::edge_down::sync_prim_ltm;
use crate::engine::physic::module::*;
use crate::engine::physic::objcoll_check::EnvPrim;
use crate::engine::physic::rwm::*;

pub struct EFDMGrabEdge {
    pub base: EFPhysicModule,
    /// +0x30 max grab angle (1.0)
    pub max_angle: f32,
    /// +0x34 grab radius (0.5)
    pub radius: f32,
    /// +0x38 prim index
    pub prim: u32,
    /// +0x3c / +0x40 InputActionGet ids
    pub action: [u32; 2],
    /// +0x44 reach speed (0.5)
    pub reach_speed: f32,
    /// +0x48 move speed at full input (1.0)
    pub speed_max: f32,
    /// +0x4c move speed at zero input (0)
    pub speed_min: f32,
    /// +0x50 corner turn speed (rad/s, 1.0)
    pub turn_speed: f32,
    /// +0x54 probe offset along the owner's at axis
    pub offset: f32,
    /// +0x58 last frame
    pub frame: u32,
    /// +0x5c position factor along the edge (0..1)
    pub factor: f32,
    /// +0x60 signed input factor (1.0)
    pub input: f32,
    /// +0x64 (byte) phase: 0 reach, 1 move on edge, 2 corner transition
    pub phase: u8,
    /// +0x68 edge length
    pub edge_len: f32,
    /// +0x6c edge vector (DoAction 1 word 0..2)
    pub edge: V3,
    /// +0x78 edge polygon normal (DoAction 1 word 3..5)
    pub normal: V3,
    /// +0x84 grab point
    pub grab: V3,
    /// +0x90 edge start vertex
    pub v0: V3,
    /// +0x9c edge end vertex
    pub v1: V3,
    /// +0xa8 adjacent edge's far vertex
    pub next: V3,
    /// +0xb4 reach start (prim center)
    pub reach_start: V3,
    /// +0xc0 reach velocity (dir / reach time)
    pub reach_dir: V3,
    /// +0xcc rotation axis
    pub axis: V3,
    /// +0xd8 rotation: angular rate (reach) / total corner angle (transition)
    pub angle: f32,
    /// +0xdc current corner angle
    pub cur_angle: f32,
    /// +0xe0 reach time
    pub reach_time: f32,
    /// +0xe4 reach elapsed
    pub reach_t: f32,
}

#[inline]
fn d3(a: V3, b: V3) -> f32 { a[2] * b[2] + a[0] * b[0] + a[1] * b[1] }

/// acos clamped as in the binary (`<= -1` -> pi, `>= 1` -> 0)
#[inline]
fn acos_c(x: f32) -> f32 {
    if x <= -1.0 { 3.1415927 } else if 1.0 <= x { 0.0 } else { x.acos() }
}

/// RwFrameRotate(frame, axis, deg, POSTCONCAT) keeping the frame position (+0x40..+0x48).
fn rot_keep_pos(f: &mut RwMatrix, deg: f32, axis: V3) {
    let pos = f.pos;
    f.Rotate(deg, axis, 2);
    f.pos = pos;
}

impl EFDMGrabEdge {
    pub fn new() -> Self {
        EFDMGrabEdge {
            base: EFPhysicModule { type_: 0x4000_0018, flags: 1, ..Default::default() },
            max_angle: 1.0,
            radius: 0.5,
            prim: 0,
            action: [0, 0],
            reach_speed: 0.5,
            speed_max: 1.0,
            speed_min: 0.0,
            turn_speed: 1.0,
            offset: 0.0,
            frame: 0,
            factor: 0.0,
            input: 1.0,
            phase: 0,
            edge_len: 0.0,
            edge: [0.0; 3],
            normal: [0.0; 3],
            grab: [0.0; 3],
            v0: [0.0; 3],
            v1: [0.0; 3],
            next: [0.0; 3],
            reach_start: [0.0; 3],
            reach_dir: [0.0; 3],
            axis: [0.0; 3],
            angle: 0.0,
            cur_angle: 0.0,
            reach_time: 0.0,
            reach_t: 0.0,
        }
    }

    /// prim vtbl+0xc; returns the new world center (+0xc0).
    fn PrecalcPrim(&self, ctx: &mut DMCtx) -> V3 {
        let frame = ctx.phys.frame.clone();
        let Some(Some(pr)) = ctx.phys.model.prims.get_mut(self.prim as usize) else { return [0.0; 3] };
        if pr.base.joint == 0xffff_ffff {
            sync_prim_ltm(&mut pr.base.ltm, &frame);
        }
        pr.PrecalcPrimData();
        pr.base.center
    }

    /// EFCollisionModel::Translate + velocity (+0x28) = 0, falling timer (+0x48) = delay (+0x44).
    fn TranslateOwner(ctx: &mut DMCtx, d: V3) {
        if ctx.phys.model.Translate(d) {
            ctx.phys.frame.Translate(d, 2);
        }
        ctx.phys.velocity = [0.0; 3];
        ctx.phys.falling_timer = ctx.phys.falling_delay;
    }

    /// PostColUpdate body (stage != NULL).
    pub fn GrabEdge(&mut self, ctx: &mut DMCtx, env: &[EnvPrim], dt: f32) {
        // quirk: prim+0xc0 through a possibly NULL prim pointer; a missing prim aborts here
        let Some(Some(pr)) = ctx.phys.model.prims.get(self.prim as usize) else { return };
        let mut c = pr.base.center;
        let (m1, m2) = (pr.base.word(0x20), pr.base.word(0x24));
        let pos = ctx.phys.frame.pos;
        let at = ctx.phys.frame.at;
        if ctx.frame != self.frame.wrapping_add(1)
            && (ctx.prev_paused != 1 || ctx.frame.wrapping_sub(self.frame.wrapping_add(1)) != ctx.paused_frames)
        {
            self.phase = 0;
            self.base.state = 0;
        }
        self.frame = ctx.frame;
        if self.base.state == 0 {
            let k = -d3(sub(c, pos), at);
            self.offset = k;
            let p = add(scale(at, k), c);
            if self.FindNewGrabEdge(env, m1, m2, p, c) {
                let a = sub(c, p);
                let b = sub(self.grab, p);
                let right = ctx.phys.frame.right;
                // at x right (= up for an orthonormal frame)
                let up = [at[1] * right[2] - at[2] * right[1], at[2] * right[0] - at[0] * right[2], at[0] * right[1] - at[1] * right[0]];
                let a = RwV3dNormalize(sub(a, scale(up, d3(up, a))));
                let b = RwV3dNormalize(sub(b, scale(up, d3(up, b))));
                let mut ang = acos_c(d3(a, at)) - acos_c(d3(b, at));
                if 0.0 < d3(b, right) {
                    ang *= -1.0;
                }
                if (0.0 < ang && ang <= self.max_angle) || (ang <= 0.0 && -self.max_angle <= ang) {
                    self.axis = [0.0, 1.0, 0.0];
                    self.reach_dir = sub(self.grab, c);
                    self.reach_start = c;
                    self.reach_t = 0.0;
                    let l = RwV3dLength(self.reach_dir);
                    self.reach_time = l / self.reach_speed;
                    self.angle = ang / (l / self.reach_speed);
                    let inv = 1.0 / self.reach_time;
                    self.reach_dir = scale(self.reach_dir, inv);
                    self.ReachEdge(ctx, dt);
                    if self.base.state & 1 == 0 {
                        self.base.state = 0;
                        self.base.SetStateBits(1, ctx);
                    } else {
                        self.base.state = 1;
                    }
                }
            }
            return;
        }
        self.ComputeInputFactor(ctx);
        if self.phase != 2 {
            if self.phase != 1 {
                if self.phase != 0 {
                    return;
                }
                self.ReachEdge(ctx, dt);
                if self.reach_t < self.reach_time {
                    return;
                }
                if self.base.state & 2 == 0 {
                    self.base.state = 0;
                    self.base.SetStateBits(2, ctx);
                } else {
                    self.base.state = 2;
                }
                self.phase = 1;
            }
            let moved = self.UpdateFactor(dt);
            if moved || !self.FindAdjEdge(env, m1, m2) {
                if moved {
                    // operand: edge (+0x6c) * factor
                    self.grab = add(scale(self.edge, self.factor), self.v0);
                    self.MoveOnEdge(ctx);
                    return;
                }
                if self.base.state & 4 != 0 {
                    self.base.state = 4;
                    return;
                }
                self.base.state = 0;
                self.base.SetStateBits(4, ctx);
                return;
            }
            let mut a = sub(self.v1, self.v0);
            let mut b = sub(self.next, self.v1);
            self.edge = b;
            self.edge_len = RwV3dLength(self.edge);
            a = RwV3dNormalize(a);
            b = RwV3dNormalize(b);
            self.cur_angle = 0.0;
            let cs = d3(a, b);
            self.angle = if cs <= -1.0 { 3.1415927 } else if cs < 1.0 { cs.acos() } else { 0.0 };
            self.axis = [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
            self.phase = 2;
        }
        c = ctx.phys.model.prims[self.prim as usize].as_ref().map_or(c, |p| p.base.center);
        let target = add(scale(at, self.offset), c);
        if !self.Transition(ctx, target, dt) {
            self.phase = 1;
        }
        self.MoveOnEdge(ctx);
    }

    /// FindNewGrabEdge(stage, prim, p, center)
    pub fn FindNewGrabEdge(&mut self, env: &[EnvPrim], m1: u32, m2: u32, p: V3, c: V3) -> bool {
        let mut found = false;
        let mut r2 = self.radius * self.radius;
        for e in env {
            if !((e.mat[1] & m1 != 0 || e.mat[2] & m2 != 0) && e.w4 != 0) {
                continue;
            }
            let n = e.nverts as usize;
            let mut j = 1usize;
            for i in 0..n {
                if e.w4 as u32 & (1u32 << (i & 0x1f)) != 0 {
                    if j == n {
                        j = 0;
                    }
                    // quirk: raw u16 count; EnvPrim holds `& 0x7fff` vertices
                    let (Some(&a), Some(&b)) = (e.verts.get(i), e.verts.get(j)) else { break };
                    let s = sub(b, a);
                    let t = d3(sub(p, a), s) / d3(s, s);
                    let q = add(scale(s, t), a);
                    let d = sub(c, q);
                    let l = d3(d, d);
                    if l <= r2 && 0.0 <= t && t <= 1.0 {
                        found = true;
                        self.grab = q;
                        self.factor = t;
                        self.normal = e.normal;
                        self.edge = s;
                        self.edge_len = RwV3dLength(self.edge);
                        self.v0 = a;
                        self.v1 = b;
                        r2 = l;
                    }
                }
                j += 1;
            }
        }
        found
    }

    fn swap_ends(&mut self) {
        std::mem::swap(&mut self.v0, &mut self.v1);
        self.input *= -1.0;
    }

    /// FindAdjEdge(stage, prim): the flagged edge sharing v1 (and not going back to v0).
    pub fn FindAdjEdge(&mut self, env: &[EnvPrim], m1: u32, m2: u32) -> bool {
        let mut found = false;
        let f = self.factor;
        if f < 0.0 {
            self.swap_ends();
            self.factor = -self.factor;
        } else {
            self.factor = f - 1.0;
        }
        for e in env {
            if found {
                break;
            }
            if !((e.mat[1] & m1 != 0 || e.mat[2] & m2 != 0) && e.w4 != 0) {
                continue;
            }
            let n = e.nverts as usize;
            let mut j = 1usize;
            let mut i = 0usize;
            while i < n && !found {
                if e.w4 as u32 & (1u32 << (i & 0x1f)) != 0 {
                    if j == n {
                        j = 0;
                    }
                    let (Some(&a), Some(&b)) = (e.verts.get(i), e.verts.get(j)) else { break };
                    if a == self.v1 && b != self.v0 {
                        found = true;
                        self.next = b;
                    } else if b == self.v1 && a != self.v0 {
                        found = true;
                        self.next = a;
                    }
                }
                i += 1;
                j += 1;
            }
        }
        if !found {
            if f < 0.0 {
                self.swap_ends();
                self.factor = 0.0;
            } else {
                self.factor = 1.0;
            }
        }
        found
    }

    /// ReachEdge(&dt, frame, prim)
    pub fn ReachEdge(&mut self, ctx: &mut DMCtx, dt: f32) {
        let t = self.reach_t;
        let l = self.reach_time;
        if t < l {
            let mut step = dt;
            if l < t + step {
                step = l - t;
                self.reach_t = l;
            } else {
                self.reach_t = t + step;
            }
            rot_keep_pos(&mut ctx.phys.frame, step * self.angle * RAD2DEG, self.axis);
            let c = self.PrecalcPrim(ctx);
            // operand: reach_dir (+0xc0) * reach_t
            let q = add(scale(self.reach_dir, self.reach_t), self.reach_start);
            Self::TranslateOwner(ctx, sub(q, c));
        }
    }

    /// MoveOnEdge(prim)
    pub fn MoveOnEdge(&mut self, ctx: &mut DMCtx) {
        let c = self.PrecalcPrim(ctx);
        Self::TranslateOwner(ctx, sub(self.grab, c));
    }

    /// UpdateFactor(dt): true while the factor stays in [0, 1].
    pub fn UpdateFactor(&mut self, dt: f32) -> bool {
        let i = self.input;
        let mut d = 0.0;
        if 0.0 < i {
            d = (dt * (self.speed_min + i * (self.speed_max - self.speed_min))) / self.edge_len;
        }
        if i < 0.0 {
            d = (dt * (-self.speed_min + i * (self.speed_max - self.speed_min))) / self.edge_len;
        }
        self.factor += d;
        0.0 <= self.factor && self.factor <= 1.0
    }

    /// Transition(target, frame, dt). quirk: `_target` is unused.
    pub fn Transition(&mut self, ctx: &mut DMCtx, _target: V3, dt: f32) -> bool {
        if !self.UpdateFactor(dt) {
            self.factor += 1.0;
            self.edge = sub(self.v1, self.v0);
            self.edge_len = RwV3dLength(self.edge);
            // operand: edge (+0x6c) * factor
            self.grab = add(scale(self.edge, self.factor), self.v0);
            rot_keep_pos(&mut ctx.phys.frame, -self.cur_angle * RAD2DEG, self.axis);
            self.cur_angle = 0.0;
        } else {
            self.grab = add(scale(self.edge, self.factor), self.v1);
            let step = if 0.0 < self.input {
                let s = dt * self.turn_speed;
                if self.angle - self.cur_angle <= dt * self.turn_speed { self.angle - self.cur_angle } else { s }
            } else {
                let s = -dt * self.turn_speed;
                if s <= -self.cur_angle { -self.cur_angle } else { s }
            };
            rot_keep_pos(&mut ctx.phys.frame, step * RAD2DEG, self.axis);
            self.cur_angle += step;
            if self.angle <= self.cur_angle {
                self.v0 = self.v1;
                self.v1 = self.next;
            } else if 0.0 <= self.cur_angle {
                return true;
            }
        }
        false
    }

    /// ComputeInputFactor(): signed stick magnitude along the edge, in camera space.
    pub fn ComputeInputFactor(&mut self, ctx: &DMCtx) {
        self.input = 0.0;
        if let Some([mut x, mut y]) = ctx.world.InputActionGet(self.action[0], self.action[1]) {
            let mut m = (y * y + x * x).sqrt();
            if 1.0 < m {
                x /= m;
                y /= m;
                m = 1.0;
            }
            let cam = ctx.world.GetCameraLTM();
            // operand: camera at * y - camera right * x
            let v = sub(scale(cam.at, y), scale(cam.right, x));
            self.input = if d3(v, self.edge) < 0.0 { -m } else { m };
        }
    }
}

impl EFDM for EFDMGrabEdge {
    fn base(&self) -> &EFPhysicModule { &self.base }
    fn base_mut(&mut self) -> &mut EFPhysicModule { &mut self.base }
    fn ParamWords(&self) -> usize { 9 }
    fn SetParams(&mut self, ctx: &mut DMCtx, p: Option<&[u32]>) {
        match p {
            None => {
                self.max_angle = 1.0;
                self.radius = 0.5;
                self.prim = 0;
                self.action = [0, 0];
                self.reach_speed = 0.5;
                self.speed_max = 1.0;
                self.speed_min = 0.0;
                self.turn_speed = 1.0;
            }
            Some(p) => {
                self.max_angle = wf(p, 0);
                self.radius = wf(p, 1);
                self.prim = ctx.phys.model.PrimTagToPrimIndex(p[2]);
                self.action = [p[3], p[4]];
                self.reach_speed = wf(p, 5);
                self.speed_max = wf(p, 6);
                self.speed_min = wf(p, 7);
                self.turn_speed = wf(p, 8);
            }
        }
        self.base.state = 0;
    }
    fn GetParams(&self, ctx: &DMCtx, p: &mut [u32]) {
        p[0] = fw(self.max_angle);
        p[1] = fw(self.radius);
        p[2] = ctx.phys.model.prims.get(self.prim as usize).and_then(|q| q.as_ref()).map_or(0, |q| q.base.tag);
        p[3] = self.action[0];
        p[4] = self.action[1];
        p[5] = fw(self.reach_speed);
        p[6] = fw(self.speed_max);
        p[7] = fw(self.speed_min);
        p[8] = fw(self.turn_speed);
    }
    fn DoAction(&mut self, _ctx: &mut DMCtx, action: u32, p: &mut [u32]) {
        if action == 1 && p.len() >= 6 {
            put_v(p, 0, self.edge);
            put_v(p, 3, self.normal);
        }
    }
    fn PostColUpdate(&mut self, ctx: &mut DMCtx, stage: Option<&EFCollisionStage>, dt: f32) {
        // the stage's env prims (stage+0x4/+0x8, WP-J4); empty for seam stages
        if let Some(st) = stage {
            self.GrabEdge(ctx, st.env, dt);
        }
    }
}

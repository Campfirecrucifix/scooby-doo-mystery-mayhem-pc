//! 0x11 EFDMRopeSlide (Classes/EFDM/EFDMRopeSlide.cpp, 0xac bytes, type 0x50000011; ctor inlined
//! in EFApi_Physic DMAdd), 1:1: the hero hangs below a rope spline (_gpoSplineMgr), slides along it
//! (slope acceleration + stick speed), sways sideways (stick x while state bit 1), and at the end of
//! the rope continues on the linked spline that best matches its orientation (state bit 2 when
//! there is none). Optional IK (+0x60) pulls a collision prim's frame (the hands) onto the rope.
//!
//! Seams:
//! - _gpoSplineMgr splines: `PhysicWorld::Splines()`; the spline pointer +0x68 is an index into it.
//!   The spline link groups (_gpoSplineMgr+0xc, 0x10-byte entries: +4 count, +0xc spline index
//!   list, selected by spline header +8) are not decoded by spline.rs: they are the `links` field
//!   (`SplineLinks`, host-filled; empty = every rope ends, i.e. state bit 2).
//! - root frame (owner obj+0x14): `ctx.phys.frame` = its LTM and modelling matrix (no parent).
//! - ComputeIK: prim frame +0xdc via `PrimFrameBone`, parents via `GetBoneParent`, LTMs via
//!   `GetBone`/`GetBoneParentLTM`, written with `SetBoneModelling` (= RwFrameUpdateObjects). The
//!   RwMatrixCreate/Destroy temporaries and the global scratch matrix (pfRam003439d0) are locals.
//! - InputActionGet: `PhysicWorld::InputActionGet`.
#![allow(non_snake_case)]

use crate::engine::physic::module::*;
use crate::engine::physic::rwm::*;
use crate::engine::spline::{EFQuadraticSpline, EFSplineCursor};

/// _gpoSplineMgr+0xc link groups (see module doc): `group_of[spline]` (spline header +8) indexes
/// `groups` (entry +0xc list, count +4).
#[derive(Debug, Clone, Default)]
pub struct SplineLinks {
    pub group_of: Vec<u32>,
    pub groups: Vec<Vec<u32>>,
}
impl SplineLinks {
    pub fn Links(&self, spline: usize) -> &[u32] {
        self.group_of.get(spline).and_then(|&g| self.groups.get(g as usize)).map_or(&[], |v| v.as_slice())
    }
}

/// EFSplineCursor as embedded at this+100 (+0 flags: 1 point cached, 2 tangent cached; +4 spline;
/// +8 global; +0xc seg; +0x10 t; +0x14 point; +0x20 tangent). Every move clears the flags.
#[derive(Debug, Clone, Copy, Default)]
pub struct RopeCursor {
    pub flags: u32,
    pub spline: Option<usize>,
    pub cur: EFSplineCursor,
    pub point: V3,
    pub tangent: V3,
}

impl RopeCursor {
    /// spline = s, seg = -1, t = 0, global = 0, flags = 0
    fn Reset(&mut self, s: usize) {
        self.spline = Some(s);
        self.cur = EFSplineCursor { global: 0.0, seg: 0xffff_ffff, t: 0.0 };
        self.flags = 0;
    }
    fn Point(&mut self, sp: &EFQuadraticSpline) -> V3 {
        if self.flags & 1 == 0 {
            self.point = sp.GetPoint(self.cur.seg as i32, self.cur.t);
            self.flags |= 1;
        }
        self.point
    }
    fn Tangent(&mut self, sp: &EFQuadraticSpline) -> V3 {
        if self.flags & 2 == 0 {
            self.tangent = sp.GetTangent(self.cur.seg as i32, self.cur.t);
            self.flags |= 2;
        }
        self.tangent
    }
    fn MoveToNearest(&mut self, sp: &EFQuadraticSpline, p: V3) {
        self.cur.MoveToNearest(sp, p);
        self.flags = 0;
    }
    fn SetToNearest(&mut self, sp: &EFQuadraticSpline, p: V3) {
        self.cur.SetToNearest(sp, p);
        self.flags = 0;
    }
    fn MoveTo(&mut self, sp: &EFQuadraticSpline, f: f32) {
        self.cur.MoveTo(sp, f);
        self.flags = 0;
    }
    fn MoveDistance(&mut self, sp: &EFQuadraticSpline, d: f32) {
        self.cur.MoveDistance(sp, d);
        self.flags = 0;
    }
}

pub struct EFDMRopeSlide {
    pub base: EFPhysicModule,
    /// +0x30 input action b (param +4)
    pub input_b: u32,
    /// +0x34 input action a (param +0)
    pub input_a: u32,
    /// +0x38 sway stick scale (5)
    pub sway_input: f32,
    /// +0x3c hang distance below the rope (0.045)
    pub hang: f32,
    /// +0x40 extra stretch range (0.8)
    pub stretch: f32,
    /// +0x44 / +0x48 pull-up speed at no / full stretch (5 / 24)
    pub pull_min: f32,
    pub pull_max: f32,
    /// +0x4c slope acceleration (fabs of param +0x30, 7)
    pub slope_accel: f32,
    /// +0x50 max slide speed (8)
    pub speed_max: f32,
    /// +0x54 min slide speed (4)
    pub speed_min: f32,
    /// +0x58 base stick speed (6)
    pub speed_base: f32,
    /// +0x5c speed blend rate (4)
    pub speed_rate: f32,
    /// +0x60 IK enabled (param +0x20 == 1)
    pub ik: u32,
    /// +0x64..+0x90 cursor (+0x68 spline)
    pub cursor: RopeCursor,
    /// +0x90 sway angle
    pub sway: f32,
    /// +0x94 sway angular velocity
    pub sway_vel: f32,
    /// +0x98 / +0x9c sway blend rate in state 1 / otherwise (10 / 5)
    pub sway_rate_on: f32,
    pub sway_rate_off: f32,
    /// +0xa0 slide speed
    pub speed: f32,
    /// +0xa4 IK prim index (-1)
    pub prim: u32,
    /// +0xa8 max sway angle (pi/4)
    pub sway_max: f32,
    /// _gpoSplineMgr+0xc link groups (seam)
    pub links: SplineLinks,
}

impl Default for EFDMRopeSlide {
    fn default() -> Self { Self::new() }
}

impl EFDMRopeSlide {
    /// DMAdd case 0x11 (EFApi_Physic.cpp)
    pub fn new() -> Self {
        EFDMRopeSlide {
            base: EFPhysicModule { type_: 0x5000_0011, flags: 1, ..Default::default() },
            input_b: 0xffff_ffff,
            input_a: 0xffff_ffff,
            sway_input: 5.0,
            hang: f32::from_bits(0x3d3851ec),
            stretch: f32::from_bits(0x3f4ccccd),
            pull_min: 5.0,
            pull_max: 24.0,
            slope_accel: 7.0,
            speed_max: 8.0,
            speed_min: 4.0,
            speed_base: 6.0,
            speed_rate: 4.0,
            ik: 0,
            cursor: RopeCursor { cur: EFSplineCursor { global: 0.0, seg: 0xffff_ffff, t: 0.0 }, ..Default::default() },
            sway: 0.0,
            sway_vel: 0.0,
            sway_rate_on: 10.0,
            sway_rate_off: 5.0,
            speed: 0.0,
            prim: 0xffff_ffff,
            sway_max: f32::from_bits(0x3f490fdb),
            links: SplineLinks::default(),
        }
    }

    /// ComputeIK() (0x0021ea20)
    pub fn ComputeIK(&mut self, ctx: &mut DMCtx) {
        let world = ctx.world;
        let owner = ctx.phys.owner;
        let Some(si) = self.cursor.spline else { return };
        let Some(sp) = world.Splines().get(si) else { return };
        let Some(Some(prim)) = ctx.phys.model.prims.get(self.prim as usize) else { return };
        let (center, offset) = (prim.base.center, prim.base.offset);
        let Some(f) = world.PrimFrameBone(owner, self.prim) else { return };
        let Some((_, fl)) = world.GetBone(owner, f) else { return };
        let mut c = RopeCursor::default();
        c.Reset(si);
        c.SetToNearest(sp, center);
        let t = RwV3dNormalize(c.Tangent(sp));
        let root = ctx.phys.frame;
        let inv_root = root.Invert();
        let Some(fpl) = world.GetBoneParentLTM(owner, f) else { return };
        let inv_fp = fpl.Invert();
        // scratch = root LTM with at = rope tangent, up made orthogonal (not normalised), right = up x at
        let mut s = root;
        s.at = t;
        let d = dot(s.up, s.at);
        s.up = sub(s.up, scale(s.at, d));
        s.right = cross(s.up, s.at);
        let m2 = RwMatrix::Multiply(&s, &inv_root);
        let mut m1 = RwMatrix::Multiply(&fl, &m2);
        let off = m1.TransformVector(offset);
        let q = c.Point(sp);
        m1.pos = sub(q, off);
        let half = scale(sub(m1.pos, fl.pos), 0.5);
        m1.pos = sub(m1.pos, half);
        world.SetBoneModelling(owner, f, RwMatrix::Multiply(&m1, &inv_fp));
        // the parent frame moves by the other half
        let Some(g) = world.GetBoneParent(owner, f) else { return };
        let Some(gpl) = world.GetBoneParentLTM(owner, g) else { return };
        let v = gpl.Invert().TransformVector(half);
        if let Some((mut gm, _)) = world.GetBone(owner, g) {
            gm.pos = add(gm.pos, v);
            world.SetBoneModelling(owner, g, gm);
        }
    }
}

impl EFDM for EFDMRopeSlide {
    fn base(&self) -> &EFPhysicModule { &self.base }
    fn base_mut(&mut self) -> &mut EFPhysicModule { &mut self.base }
    fn ParamWords(&self) -> usize { 0x11 }
    /// SetParams(void*) (0x0021d880)
    fn SetParams(&mut self, ctx: &mut DMCtx, p: Option<&[u32]>) {
        match p {
            None => {
                self.input_b = 0xffff_ffff;
                self.input_a = 0xffff_ffff;
                self.sway_input = 5.0;
                self.hang = f32::from_bits(0x3d3851ec);
                self.stretch = f32::from_bits(0x3f4ccccd);
                self.pull_min = 5.0;
                self.pull_max = 24.0;
                self.speed_min = 4.0;
                self.speed_max = 8.0;
                self.speed_base = 6.0;
                self.slope_accel = 7.0;
                self.sway_rate_on = 10.0;
                self.sway_rate_off = 5.0;
                self.speed_rate = 4.0;
                self.prim = 0xffff_ffff;
                self.sway_max = f32::from_bits(0x3f490fdb);
                self.ik = 0;
            }
            Some(p) => {
                self.input_b = p[1];
                self.input_a = p[0];
                self.sway_input = wf(p, 2);
                self.speed_rate = wf(p, 3);
                self.hang = wf(p, 4);
                self.stretch = wf(p, 5);
                self.pull_min = wf(p, 6);
                self.pull_max = wf(p, 7);
                self.speed_min = wf(p, 9);
                self.speed_max = wf(p, 10);
                self.speed_base = wf(p, 11);
                self.sway_rate_on = wf(p, 13);
                self.sway_rate_off = wf(p, 14);
                self.slope_accel = (wf(p, 12) as f64).abs() as f32;
                self.sway_max = wf(p, 16);
                self.prim = ctx.phys.model.PrimTagToPrimIndex(p[15]);
                self.ik = (p[8] == 1) as u32;
            }
        }
    }
    /// GetParams(void*) (0x0021d9e0)
    fn GetParams(&self, ctx: &DMCtx, p: &mut [u32]) {
        p[1] = self.input_b;
        p[0] = self.input_a;
        p[2] = fw(self.sway_input);
        p[3] = fw(self.speed_rate);
        p[4] = fw(self.hang);
        p[5] = fw(self.stretch);
        p[6] = fw(self.pull_min);
        p[7] = fw(self.pull_max);
        p[9] = fw(self.speed_min);
        p[10] = fw(self.speed_max);
        p[11] = fw(self.speed_base);
        p[12] = fw(self.slope_accel);
        p[13] = fw(self.sway_rate_on);
        p[14] = fw(self.sway_rate_off);
        p[16] = fw(self.sway_max);
        p[8] = self.ik;
        p[15] = if self.prim == 0xffff_ffff {
            0xffff_ffff
        } else {
            // quirk: NULL slot reads tag at address 8 in the original; 0 here
            ctx.phys.model.prims.get(self.prim as usize).and_then(|q| q.as_ref()).map_or(0, |q| q.base.tag)
        };
    }
    /// DoAction(uint, void*) (0x0021f0c0): action 1 {spline id, mode}: attach to the first spline
    /// with type bit 2 and that id; mode 0 = at the nearest point, else at the start; snaps the
    /// root frame below the rope facing along it.
    fn DoAction(&mut self, ctx: &mut DMCtx, action: u32, p: &mut [u32]) {
        if action != 1 {
            return;
        }
        let world = ctx.world;
        let sps = world.Splines();
        let Some(si) = sps.iter().position(|s| s.ty & 4 != 0 && p[0] == s.id as u32) else { return };
        let sp = &sps[si];
        self.cursor.Reset(si);
        if p[1] == 0 {
            let pos = ctx.phys.frame.pos;
            self.cursor.SetToNearest(sp, pos);
        } else {
            self.cursor.MoveTo(sp, 0.0);
        }
        let pt = self.cursor.Point(sp);
        let target = [pt[0], pt[1] + self.hang, pt[2]];
        let m = &mut ctx.phys.frame;
        m.pos = [0.0; 3];
        m.Translate(target, rwCOMBINEPOSTCONCAT);
        m.at = RwV3dNormalize(self.cursor.Tangent(sp));
        m.up = [0.0, 1.0, 0.0];
        let d = dot(m.up, m.at);
        m.up = sub(m.up, scale(m.at, d));
        m.up = RwV3dNormalize(m.up);
        m.right = cross(m.up, m.at);
    }
    /// PreColUpdate(EFDynamicPhysic&, float) (0x0021dab0)
    fn PreColUpdate(&mut self, ctx: &mut DMCtx, _dp: &mut EFDynamicPhysic, dt: f32) -> bool {
        let world = ctx.world;
        let sps = world.Splines();
        let Some(mut si) = self.cursor.spline else { return true };
        let Some(mut sp) = sps.get(si) else { return true };
        let mut end = false;
        let mut lat = 0.0;
        let mut spd = 0.0;
        if let Some(inp) = world.InputActionGet(self.input_a, self.input_b) {
            let y = inp[1];
            let x = inp[0];
            spd = if 0.0 <= y { self.speed_base + self.speed_max * y } else { self.speed_base + self.speed_min * y };
            if x != 0.0 && self.base.state & 1 != 0 {
                lat = x * self.sway_input;
            }
        }
        let rate = if self.base.state & 1 == 0 { self.sway_rate_off } else { self.sway_rate_on };
        self.sway_vel += rate * dt * (lat - self.sway_vel);
        let a = self.sway + self.sway_vel * dt;
        self.sway = a;
        if self.sway_max < a {
            self.sway = self.sway_max;
        } else if a < -self.sway_max {
            self.sway = -self.sway_max;
        }
        let pos = ctx.phys.frame.pos;
        self.cursor.MoveToNearest(sp, pos);
        let mut tan = self.cursor.Tangent(sp);
        let p0 = self.cursor.Point(sp);
        let dv = sub(pos, p0);
        let mut dist = (dv[2] * dv[2] + dv[0] * dv[0] + dv[1] * dv[1]).sqrt();
        tan = RwV3dNormalize(tan);
        self.speed += dt * -tan[1] * self.slope_accel;
        let mut k = 1.0;
        if self.speed_rate * dt <= 1.0 {
            k = self.speed_rate * dt;
        }
        let v = self.speed + k * (spd - self.speed);
        self.speed = v;
        if v < self.speed_min {
            self.speed = self.speed_min;
        } else if self.speed_max < v {
            self.speed = self.speed_max;
        }
        let up0 = ctx.phys.frame.up;
        let right0 = ctx.phys.frame.right;
        let seg = self.cursor.cur.seg;
        let over = (self.speed * dt + sp.segs[seg as usize].distance + sp.GetSegmentLengthAt(seg, self.cursor.cur.t)) - sp.length;
        if 0.0 < over {
            let links = self.links.Links(si);
            if links.is_empty() {
                end = true;
            } else {
                let mut best_i = 0usize;
                let mut best = -10000.0f32;
                for (i, &c) in links.iter().enumerate() {
                    let Some(cs) = sps.get(c as usize) else { continue };
                    // quirk: the candidate cursor starts at seg -1 (MoveDistance(1.0) reads the
                    // 0xa4 bytes before the segment array as "segment -1"); modelled as a
                    // zero-length segment -1, i.e. the move continues from the start of seg 0.
                    let mut cc = RopeCursor::default();
                    cc.spline = Some(c as usize);
                    cc.cur = EFSplineCursor { global: 0.0, seg: 0, t: 0.0 };
                    cc.MoveDistance(cs, 1.0);
                    let t = RwV3dNormalize(cc.Tangent(cs));
                    let d = dot(t, up0);
                    let ad = if d <= 0.0 { -d } else { d };
                    let e = 1.0 - dot(t, right0);
                    let s = d * ad + e * e;
                    if !(s <= best) {
                        best = s;
                        best_i = i;
                    }
                }
                si = links[best_i] as usize;
                if let Some(n) = sps.get(si) {
                    sp = n;
                    self.cursor.Reset(si);
                    self.cursor.MoveTo(sp, 0.0);
                    self.cursor.MoveDistance(sp, over);
                }
            }
        } else {
            self.cursor.MoveDistance(sp, self.speed * dt);
        }
        let t = RwV3dNormalize(tan);
        let up = RwV3dNormalize(sub([0.0, 1.0, 0.0], scale(t, t[2] * 0.0 + t[0] * 0.0 + t[1] * 1.0)));
        let right = cross(up, t);
        {
            let m = &mut ctx.phys.frame;
            let pos = m.pos;
            // RwFrameRotate(frame, tangent, 180, POSTCONCAT): fully overwritten just below
            m.Rotate(180.0, t, rwCOMBINEPOSTCONCAT);
            m.pos = pos;
            m.at = t;
            m.up = up;
            m.right = right;
            let pos = m.pos;
            m.Rotate(self.sway * RAD2DEG, t, rwCOMBINEPOSTCONCAT);
            m.pos = pos;
        }
        let m = ctx.phys.frame;
        let vel = ctx.phys.velocity;
        let vu = vel[2] * m.up[2] + vel[0] * m.up[0] + vel[1] * m.up[1];
        let p1 = self.cursor.Point(sp);
        let inv = 1.0 / dt;
        let mut nv = scale(sub(p1, p0), inv);
        let (rest, range) = (self.hang, self.stretch);
        if range < dist - rest {
            dist = rest + range;
        }
        let d = if rest <= dist { dist } else { rest };
        let f = (d - rest) / range;
        let f = if f < 0.0 { 0.0 } else if f <= 1.0 { f } else { 1.0 };
        let mut w = vu - dt * (self.pull_min + f * (self.pull_max - self.pull_min));
        if rest + range < d + w * dt {
            w = ((rest + range) - d) / dt;
        }
        if d + w * dt < rest {
            w = (rest - d) / dt;
        }
        nv = add(nv, scale(m.up, w));
        ctx.phys.velocity = nv;
        ctx.phys.falling_timer = ctx.phys.falling_delay;
        ctx.phys.frame.pos = add(p0, scale(m.up, d));
        if end { self.base.SetStateBits(2, ctx) } else { self.base.ResetStateBits(2, ctx) }
        true
    }
    /// PostColUpdate(EFCollisionStage*, float) (0x0021e760)
    fn PostColUpdate(&mut self, ctx: &mut DMCtx, _s: Option<&EFCollisionStage>, _dt: f32) {
        let world = ctx.world;
        // quirk: no NULL-spline check in the original
        let Some(si) = self.cursor.spline else { return };
        let Some(sp) = world.Splines().get(si) else { return };
        let pos = ctx.phys.frame.pos;
        self.cursor.MoveToNearest(sp, pos);
        let pt = self.cursor.Point(sp);
        let dv = sub(pos, pt);
        let dist = (dv[2] * dv[2] + dv[0] * dv[0] + dv[1] * dv[1]).sqrt();
        let up = ctx.phys.frame.up;
        ctx.phys.frame.pos = add(self.cursor.Point(sp), scale(up, dist));
        if dist <= self.hang + 0.01 { self.base.SetStateBits(1, ctx) } else { self.base.ResetStateBits(1, ctx) }
        if self.ik != 0 {
            self.ComputeIK(ctx);
        }
    }
}

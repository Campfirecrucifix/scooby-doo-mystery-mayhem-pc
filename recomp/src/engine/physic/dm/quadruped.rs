//! 0x1f EFDMQuadruped (Classes/EFDM/EFDMQuadruped.cpp, 0x134 bytes, type 0x5000001f), 1:1:
//! flags bit0 tilts the 3d object's root bone (HAnim node 0) to the floor normal found among the
//! collision stage contact polygons; bit1 bends two bones (+0x30 / +0x38) by a 10-frame averaged
//! angle between the previous and current object axis (turn lean).
//! Seams: bones via `PhysicWorld::GetBone`/`SetBoneModelling`/`GetBoneParentLTM`, the stage's
//! contact polygons via `PhysicWorld::StageContactPolys`. A missing bone (C++ NULL frame deref)
//! skips the write instead of crashing.
#![allow(non_snake_case)]
use crate::engine::physic::module::*;
use crate::engine::physic::rwm::*;

pub struct EFDMQuadruped {
    pub base: EFPhysicModule,
    /// +0x30 bone 2 (param +0, rotated by +0x6c)
    pub bone0: u32,
    /// +0x34 bone 2 max angle (param +0xc)
    pub max0: f32,
    /// +0x38 bone 1 (param +4, rotated by +0x70)
    pub bone1: u32,
    /// +0x3c bone 1 max angle (param +8)
    pub max1: f32,
    /// +0x40 floor tilt speed (param +0x14)
    pub tilt_speed: f32,
    /// +0x44 bone angle scale (param +0x10)
    pub angle_scale: f32,
    /// +0x48 bone axis type (1 = rotate about at / reference up, else about up / reference at)
    pub axis_type: u32,
    /// +0x4c flags: bit0 floor adapt, bit1 bone bend
    pub flags: u32,
    /// +0x50 snap the tilt to the target next frame (DoAction 2)
    pub snap: bool,
    /// +0x51 reset the tilt (DoAction 1)
    pub reset_tilt: bool,
    /// +0x52 reset the bone bend (DoAction 1)
    pub reset_bend: bool,
    /// +0x54 floor normal found
    pub normal: V3,
    /// +0x60 floor normal projected into the up/at plane (normalised)
    pub proj_normal: V3,
    /// +0x6c bone 2 averaged angle
    pub angle0: f32,
    /// +0x70 bone 1 averaged angle
    pub angle1: f32,
    /// +0x74 previous reference axis
    pub ref_axis: V3,
    /// +0x80 owner LTM at
    pub at: V3,
    /// +0x8c owner LTM up
    pub up: V3,
    /// +0x98 owner LTM right
    pub right: V3,
    /// +0xa4 owner LTM pos
    pub pos: V3,
    /// +0xb0 root bone LTM at
    pub bone_at: V3,
    /// +0xbc root bone LTM right
    pub bone_right: V3,
    /// +0xc8 root bone LTM up
    pub bone_up: V3,
    /// +0xd4 current tilt angle
    pub tilt: f32,
    /// +0xd8 target tilt angle
    pub tilt_target: f32,
    /// +0xdc bone 1 angle history
    pub hist1: [f32; 10],
    /// +0x104 bone 2 angle history
    pub hist0: [f32; 10],
    /// +0x12c history slot
    pub slot: i32,
    /// +0x130 min normal y of a floor polygon (0.7 / physic floor cosine)
    pub min_ny: f32,
}

impl Default for EFDMQuadruped {
    fn default() -> Self { Self::new() }
}

impl EFDMQuadruped {
    pub fn new() -> Self {
        let mut s = EFDMQuadruped {
            base: EFPhysicModule { type_: 0x5000_001f, flags: 1, ..Default::default() },
            bone0: 0,
            max0: 0.0,
            bone1: 0,
            max1: 0.0,
            tilt_speed: 0.0,
            angle_scale: 0.0,
            axis_type: 0,
            flags: 0,
            snap: false,
            reset_tilt: false,
            reset_bend: false,
            normal: [0.0; 3],
            proj_normal: [0.0; 3],
            angle0: 0.0,
            angle1: 0.0,
            ref_axis: [0.0; 3],
            at: [0.0; 3],
            up: [0.0; 3],
            right: [0.0; 3],
            pos: [0.0; 3],
            bone_at: [0.0; 3],
            bone_right: [0.0; 3],
            bone_up: [0.0; 3],
            tilt: 0.0,
            tilt_target: 0.0,
            hist1: [0.0; 10],
            hist0: [0.0; 10],
            slot: 0,
            min_ny: 0.0,
        };
        s.SetDefaultParams();
        s
    }

    pub fn SetDefaultParams(&mut self) {
        self.bone0 = 0;
        self.max0 = 0.35;
        self.bone1 = 0;
        self.max1 = 0.35;
        self.angle_scale = 10.0;
        self.tilt_speed = 0.1;
        self.axis_type = 2;
        self.flags = 0;
        self.snap = false;
        self.reset_tilt = false;
        self.reset_bend = false;
        self.tilt = 0.0;
        self.tilt_target = 0.0;
        self.angle0 = 0.0;
        self.angle1 = 0.0;
        self.slot = 0;
        self.min_ny = 0.7;
        self.hist1 = [0.0; 10];
        self.hist0 = [0.0; 10];
        self.ref_axis = [0.0; 3];
        self.at = [0.0, 0.0, 1.0];
        self.up = [0.0, 1.0, 0.0];
        self.right = [1.0, 0.0, 0.0];
        self.pos = [0.0; 3];
        self.bone_at = [0.0, 0.0, 1.0];
        self.bone_right = [1.0, 0.0, 0.0];
        self.bone_up = [0.0, 1.0, 0.0];
        self.normal = [0.0, 1.0, 0.0];
        self.proj_normal = [0.0, 1.0, 0.0];
    }

    /// ApplyBoneRotation(angle, frame, type): rotate the bone about the owner at (type 1) / up axis
    /// expressed in the bone's parent space, keeping the bone position.
    pub fn ApplyBoneRotation(&self, ctx: &DMCtx, angle: f32, bone: Option<u32>, type_: u32) {
        if 0.05 < angle.abs() {
            let Some(bone) = bone else { return };
            let (Some((mut m, _)), Some(parent)) =
                (ctx.world.GetBone(ctx.phys.owner, bone), ctx.world.GetBoneParentLTM(ctx.phys.owner, bone))
            else {
                return;
            };
            // RwMatrixInvert into the uRam003439d0 scratch matrix
            let inv = parent.Invert();
            let axis = if type_ == 1 { inv.TransformVector(self.at) } else { inv.TransformVector(self.up) };
            let p = m.pos;
            m.Rotate(angle * RAD2DEG, axis, 2);
            m.pos = p;
            ctx.world.SetBoneModelling(ctx.phys.owner, bone, m);
        }
    }

    /// AdaptFrameToFloor(stage, frame, dt)
    pub fn AdaptFrameToFloor(&mut self, ctx: &DMCtx, stage: Option<&EFCollisionStage>, frame: bool, dt: f32) {
        if let (Some(stage), true) = (stage, frame) {
            let mut t = self.tilt_target;
            self.ComputeFrameRotationParams(ctx, stage, &mut t);
            self.tilt_target = t;
            if !self.snap {
                let mut f2 = self.tilt_speed * dt;
                if self.tilt_target < 0.0 {
                    self.tilt -= f2;
                    let f1 = self.tilt_target;
                    f2 = self.tilt;
                    if f2 <= f1 {
                        f2 = f1;
                    }
                    // quirk: upper clamp is -target (the opposite side), not 0
                    self.tilt = if -f1 < f2 { -f1 } else { f2 };
                } else if 0.0 < self.tilt_target {
                    self.tilt += f2;
                    let f1 = self.tilt_target;
                    f2 = self.tilt;
                    if f2 <= -f1 {
                        f2 = -f1;
                    }
                    self.tilt = if f1 < f2 { f1 } else { f2 };
                } else {
                    let f1 = self.tilt;
                    if 0.0 <= f1 || 0.0 <= f1 + f2 {
                        if f1 <= 0.0 || f1 - f2 <= 0.0 {
                            self.tilt = 0.0;
                        } else {
                            self.tilt -= f2;
                        }
                    } else {
                        self.tilt = f1 + f2;
                    }
                }
            } else {
                self.tilt = self.tilt_target;
                self.snap = false;
            }
        }
    }

    /// FindNearestNormal(stage, pos, &normal): normal of the nearest floor polygon within 1.01²
    pub fn FindNearestNormal(&self, ctx: &DMCtx, _stage: &EFCollisionStage, pos: V3, out: &mut V3) -> bool {
        let polys = ctx.world.StageContactPolys(ctx.phys.owner);
        let mut best = 1.01f32;
        *out = [0.0; 3];
        let mut lim = 2.0f32;
        if polys.is_empty() {
            return false;
        }
        for (n, d, verts, edges) in &polys {
            if self.min_ny <= n[1] {
                let mut maxd = 0.0f32;
                let mut idx = 0usize;
                for (i, e) in edges.iter().enumerate().take(verts.len()) {
                    let v = e[3] + pos[2] * e[2] + pos[0] * e[0] + pos[1] * e[1];
                    if v > maxd {
                        maxd = v;
                        idx = i;
                    }
                }
                let kind;
                let mut d2;
                if maxd == 0.0 {
                    let f = d + n[2] * pos[2] + n[0] * pos[0] + n[1] * pos[1];
                    d2 = f * f;
                    if lim < d2 {
                        kind = 0;
                        d2 = lim;
                    } else {
                        kind = 1;
                    }
                } else {
                    let mut j = idx + 1;
                    if j == verts.len() {
                        j = 0;
                    }
                    let e = sub(verts[j], verts[idx]);
                    let mut w = sub(verts[idx], pos);
                    let t = e[2] * w[2] + e[0] * w[0] + e[1] * w[1];
                    let ee = e[2] * e[2] + e[0] * e[0] + e[1] * e[1];
                    // quirk: kept as decompiled (t > 0 -> vertex idx, t < -|e|² -> vertex j)
                    let mut k = if 0.0 < t {
                        3
                    } else if t < -ee {
                        w = add(w, e);
                        3
                    } else {
                        w = sub(w, scale(e, t / ee));
                        2
                    };
                    d2 = w[2] * w[2] + w[0] * w[0] + w[1] * w[1];
                    if lim < d2 {
                        k = 0;
                        d2 = lim;
                    }
                    kind = k;
                }
                // quirk: the 2.0 limit tracks the last floor polygon's distance, not the best
                lim = d2;
                if kind != 0 && d2 < best {
                    *out = *n;
                    best = d2;
                }
            }
        }
        let tiny = |x: f32| (x.to_bits() & 0x7f80_0000) < 0x3c80_0000;
        !(tiny(out[0]) && tiny(out[1]) && tiny(out[2]))
    }

    /// ComputeFrameRotationParams(stage, &angle)
    pub fn ComputeFrameRotationParams(&mut self, ctx: &DMCtx, stage: &EFCollisionStage, angle: &mut f32) {
        let mut n = self.normal;
        let found = self.FindNearestNormal(ctx, stage, self.pos, &mut n);
        self.normal = n;
        if found {
            let c = [
                self.up[1] * self.at[2] - self.up[2] * self.at[1],
                self.up[2] * self.at[0] - self.up[0] * self.at[2],
                self.up[0] * self.at[1] - self.up[1] * self.at[0],
            ];
            let d = c[2] * self.normal[2] + c[0] * self.normal[0] + c[1] * self.normal[1];
            self.proj_normal = RwV3dNormalize(sub(self.normal, scale(c, d)));
            let p = self.proj_normal;
            let f = p[2] * self.up[2] + p[0] * self.up[0] + p[1] * self.up[1];
            *angle = if f <= -1.0 {
                3.1415927
            } else if 1.0 <= f {
                0.0
            } else {
                f.acos()
            };
            if p[2] * self.at[2] + p[0] * self.at[0] + p[1] * self.at[1] < 0.0 {
                *angle = -*angle;
            }
        }
    }

    /// ComputeBoneRotationParams(&a0 (+0x6c), &a1 (+0x70))
    pub fn ComputeBoneRotationParams(&mut self) -> (f32, f32) {
        let (mut a, mut side) = (0.0f32, 0.0f32);
        if self.axis_type == 1 {
            self.ComputeObjRefRotation(self.up, &mut a, &mut side);
            self.ref_axis = self.up;
        } else {
            self.ComputeObjRefRotation(self.at, &mut a, &mut side);
            self.ref_axis = self.at;
        }
        let mut f4 = self.max1;
        let mut f1 = a * self.angle_scale;
        let mut f2 = f1;
        if f1 <= -f4 {
            f2 = -f4;
        }
        let mut f3 = self.max0;
        if f4 >= f2 {
            f4 = f2;
        }
        if f1 <= -f3 {
            f1 = -f3;
        }
        if f1 <= f3 {
            f3 = f1;
        }
        let i = self.slot as usize;
        if 0.0 <= side {
            self.hist1[i] = -f4;
            self.hist0[i] = f3;
        } else {
            self.hist1[i] = f4;
            self.hist0[i] = -f3;
        }
        self.ComputeBoneRotationAverage()
    }

    /// ComputeBoneRotationAverage(&a0, &a1): 10-slot means of +0x104 and +0xdc
    pub fn ComputeBoneRotationAverage(&self) -> (f32, f32) {
        let (mut s0, mut s1) = (0.0f32, 0.0f32);
        for i in 0..10 {
            s0 += self.hist0[i];
            s1 += self.hist1[i];
        }
        (s0 * 0.1, s1 * 0.1)
    }

    /// ComputeObjRefRotation(v, &angle, &side)
    pub fn ComputeObjRefRotation(&self, v: V3, angle: &mut f32, side: &mut f32) {
        let tiny = |x: f32| (x.to_bits() & 0x7f80_0000) < 0x3d80_0000;
        let r = self.ref_axis;
        if tiny(r[0] - v[0]) && tiny(r[1] - v[1]) && tiny(r[2] - v[2]) {
            *angle = 0.0;
            *side = 0.0;
            return;
        }
        let rt = self.right;
        let k = (r[2] * rt[2] + r[0] * rt[0] + r[1] * rt[1]) / (rt[2] * rt[2] + rt[0] * rt[0] + rt[1] * rt[1]);
        let a = scale(rt, k);
        let k = (r[2] * v[2] + r[0] * v[0] + r[1] * v[1]) / (v[2] * v[2] + v[0] * v[0] + v[1] * v[1]);
        let b = scale(v, k);
        let s = add(a, b);
        let f = v[2] * s[2] + v[0] * s[0] + v[1] * s[1];
        *angle = 0.0;
        if f < 1.0 {
            let f = if f <= 0.0 { 0.0 } else { f };
            *angle = (if f <= 1.0 { f } else { 1.0 }).acos();
        }
        *side = s[2] * rt[2] + s[0] * rt[0] + s[1] * rt[1];
    }
}

impl EFDM for EFDMQuadruped {
    fn base(&self) -> &EFPhysicModule { &self.base }
    fn base_mut(&mut self) -> &mut EFPhysicModule { &mut self.base }
    fn ParamWords(&self) -> usize { 8 }
    /// (Ghidra types `this` as the EFDynamicPhysic param.) Rotate the root bone about its right axis.
    fn PreColUpdate(&mut self, ctx: &mut DMCtx, _dp: &mut EFDynamicPhysic, _dt: f32) -> bool {
        if self.flags & 1 != 0 && 0.05 < self.tilt.abs() {
            // HAnim node 0 frame (NULL without hierarchy: skipped)
            if let Some((mut m, _)) = ctx.world.GetBone(ctx.phys.owner, 0) {
                let (axis, p) = (m.right, m.pos);
                m.Rotate(self.tilt * RAD2DEG, axis, 2);
                m.pos = p;
                ctx.world.SetBoneModelling(ctx.phys.owner, 0, m);
            }
        }
        true
    }
    fn PostColUpdate(&mut self, ctx: &mut DMCtx, stage: Option<&EFCollisionStage>, dt: f32) {
        self.slot += 1;
        if 9 < self.slot {
            self.slot = 0;
        }
        let ltm = ctx.phys.frame;
        self.pos = ltm.pos;
        self.at = ltm.at;
        self.right = ltm.right;
        self.up = ltm.up;
        if self.flags & 1 != 0 {
            let root = ctx.world.GetBone(ctx.phys.owner, 0);
            // quirk: RwFrameGetLTM(NULL) without a hierarchy; leave the copies unchanged
            if let Some((_, l)) = root {
                self.bone_at = l.at;
                self.bone_right = l.right;
                self.bone_up = l.up;
            }
            if !self.reset_tilt {
                self.AdaptFrameToFloor(ctx, stage, root.is_some(), dt);
            } else {
                self.tilt = 0.0;
                self.tilt_target = 0.0;
                self.reset_tilt = false;
            }
        }
        if self.flags & 2 == 0 {
            return;
        }
        if self.reset_bend {
            self.angle0 = 0.0;
            self.angle1 = 0.0;
            self.reset_bend = false;
            self.hist0 = [0.0; 10];
            self.hist1 = [0.0; 10];
            self.ref_axis = if self.axis_type == 1 { self.up } else { self.at };
            return;
        }
        // bone frames resolved (HAnim id / index) by GetBone at apply time
        let (a0, a1) = self.ComputeBoneRotationParams();
        self.angle0 = a0;
        self.angle1 = a1;
        self.ApplyBoneRotation(ctx, self.angle1, Some(self.bone1), self.axis_type);
        self.ApplyBoneRotation(ctx, self.angle0, Some(self.bone0), self.axis_type);
    }
    fn SetParams(&mut self, ctx: &mut DMCtx, p: Option<&[u32]>) {
        let Some(p) = p else {
            self.SetDefaultParams();
            return;
        };
        self.bone0 = p[0];
        self.bone1 = p[1];
        self.max0 = wf(p, 3);
        self.max1 = wf(p, 2);
        self.angle_scale = wf(p, 4);
        self.axis_type = p[6];
        self.tilt_speed = wf(p, 5);
        self.flags = p[7];
        if self.flags & 1 != 0 {
            self.min_ny = ctx.phys.floor_cos;
        }
        self.reset_tilt = false;
        self.reset_bend = false;
        self.snap = false;
    }
    fn GetParams(&self, _c: &DMCtx, p: &mut [u32]) {
        p[0] = self.bone0;
        p[1] = self.bone1;
        p[3] = fw(self.max0);
        p[2] = fw(self.max1);
        p[4] = fw(self.angle_scale);
        p[6] = self.axis_type;
        p[5] = fw(self.tilt_speed);
        p[7] = self.flags;
    }
    fn DoAction(&mut self, _ctx: &mut DMCtx, action: u32, p: &mut [u32]) {
        if action & 1 != 0 {
            self.reset_tilt = true;
            self.reset_bend = true;
        }
        if action & 2 != 0 {
            self.snap = true;
        }
        if action & 4 != 0 {
            put_v(p, 0, self.normal);
            put_v(p, 3, self.proj_normal);
        }
    }
}

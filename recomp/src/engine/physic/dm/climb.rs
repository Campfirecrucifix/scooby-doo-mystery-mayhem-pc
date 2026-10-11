//! 0x21 EFDMClimb (Classes/EFDM/EFDMClimb.cpp, 0x19c bytes, type 0x50000021), 1:1, plus the free
//! helpers it uses (Free Functions/Character movement helpers: ProjectInputOntoLocalFloor,
//! DMClimbParamsInit; EFCollisionStage_FindWeightedAvgNormal = stage.rs
//! `EFCollisionStage::FindWeightedAvgNormal`) and DTVector3d::GetRotationTo.
//!
//! A wall/ceiling climbing mover: five probes in front of the hero (root LTM pos + up*h + at*off_i)
//! average the nearby env polygon normals (material word1 & 0xa00); the hero's up is rotated
//! towards the averaged normal, the analog stick (projected onto the local floor, camera
//! relative) yaws the hero about its up and pushes it along its at; three bones (hips/spine) bend
//! towards the side probes' normal. State 1 = climbing (gravity applied in the hero's local
//! frame), 2 = no surface under the middle probe (gravity in world space, bsphere restored).
//!
//! Seams:
//! - root frame (owner obj+0x14, `this+400`): `ctx.phys.frame` is used as both its LTM and its
//!   modelling matrix (the root frame has no parent). quirk: the original rotates the modelling
//!   matrix with RwMatrixRotate (not RwFrameRotate), so its LTM only refreshes when something
//!   else dirties the frame; here the change is visible immediately.
//! - bones +0xb4/+0xb8/+0xbc (HAnim frames resolved in SetParams): `PhysicWorld::GetBone` ids
//!   (same HAnim-id / node-index rule), written back with `SetBoneModelling`. A NULL bone frame
//!   (crash in the original) skips that bone.
//! - +0xa4 `InputActionGet` pointer: the (a, b) action pair is re-queried each frame (the
//!   original keeps a pointer to the live value); None = {0,0}.
//! - camera: `PhysicWorld::GetCameraLTM`.
//! - EFCollisionStage contact list: the trait PostColUpdate rebuilds a real `stage::EFCollisionStage`
//!   from `StageContactPolys` + `StageContactPolyMaterials`; hosts holding the real stage call
//!   `PostColUpdateStage` directly. SetBSphereRadius's dyn-physic write (+0x3c through the stage)
//!   needs `&mut EFDynamicPhysic`, which the trait PostColUpdate does not get: there only the
//!   physic bsphere radius (EFPhysic+0x6c) is written.
#![allow(non_snake_case)]

use crate::engine::physic::module::*;
use crate::engine::physic::objcoll_check::EnvPrim;
use crate::engine::physic::rwm::*;
use crate::engine::physic::stage;

const PI: f32 = 3.1415927;

/// DMClimbParamsInit (0x00263d30): the 0x74-byte DMClimbParams block (this+0x30..+0xa4).
pub fn DMClimbParamsInit(p: &mut [u32; 29]) {
    let f = |x: f32| x.to_bits();
    p[0] = 0; // +0x00 input action a
    p[1] = 0; // +0x04 input action b
    p[2] = f(6.0); // +0x08 max move speed
    p[3] = 0x3f19999a; // +0x0c min move speed 0.6
    p[4] = f(6.0); // +0x10 yaw speed
    p[6] = 0x411ccccd; // +0x18 gravity 9.8
    p[7] = f(6.0); // +0x1c up-rotation speed
    p[0x10] = 9999999; // +0x40 bone 1
    p[0x11] = 9999999; // +0x44 bone 2
    p[0x12] = 9999999; // +0x48 bone 3
    p[0x13] = f(6.0); // +0x4c bend speed (right axis)
    p[0x14] = f(6.0); // +0x50 (unused)
    p[0x16] = 0; // +0x58 bend angle (right axis, state)
    p[0x15] = f(1.0); // +0x54 bend scale (right axis)
    p[0x17] = f(6.0); // +0x5c bend speed (up axis)
    p[0x19] = 0; // +0x64 bend angle (up axis, state)
    p[0x18] = f(1.0); // +0x60 bend scale (up axis)
    p[0x1a] = f(5.0); // +0x68 hip yaw scale
    p[0xe] = 0x3fe00000; // +0x38 climbing bsphere radius 1.75
    p[8] = f(0.5); // +0x20 probe offsets (along at)
    p[9] = 0x3e99999a; // +0x24 0.3
    p[10] = 0; // +0x28 0
    p[0xb] = 0xbe99999a; // +0x2c -0.3
    p[0xc] = 0xbf000000; // +0x30 -0.5
    p[0xd] = 0x3d4ccccd; // +0x34 probe height (along up) 0.05
    p[0xf] = f(0.5); // +0x3c max wall distance
    p[5] = 0x40490fdb; // +0x14 pi
    p[0x1b] &= !0xff; // +0x6c byte = 0
}

/// DTVector3d::GetRotationTo (0x002645e0): axis = normalised from x to, angle (asin of |axis|,
/// pi - that when facing away). Returns false (axis untouched/unnormalised) when parallel.
pub fn GetRotationTo(from: V3, to: V3, axis: &mut V3, angle: &mut f32) -> bool {
    *axis = cross(from, to);
    let l = RwV3dLength(*axis);
    let d = dot(from, to);
    if (l.to_bits() & 0x7f80_0000) < 0x3800_0000 {
        *angle = if 0.0 < d { 0.0 } else { PI };
        false
    } else {
        *axis = scale(*axis, 1.0 / l);
        *angle = if 1.0 <= l {
            1.5707964
        } else if l <= -1.0 {
            -1.5707964
        } else {
            l.asin()
        };
        if d < 0.0 {
            *angle = PI - *angle;
        }
        true
    }
}

/// ProjectInputOntoLocalFloor (0x002633f0): stick (x, y) -> direction on the plane of `up`
/// relative to the camera; returns the stick magnitude (clamped to 1), 0 inside the 1e-4 dead zone.
pub fn ProjectInputOntoLocalFloor(mut x: f32, mut y: f32, cam: &RwMatrix, m: &RwMatrix, up: V3, out: &mut V3) -> f32 {
    let mut mag = 1.0;
    let l = (y * y + x * x).sqrt();
    if 1.0 < l {
        x /= l;
        y /= l;
    } else {
        mag = l;
        // _dpflt(l, 1e-4): double compare l < 0.0001
        if (l as f64) < 1e-4 {
            return 0.0;
        }
    }
    let d = sub(cam.pos, m.pos);
    let a = RwV3dNormalize(cross(cam.up, d));
    let b = RwV3dNormalize(cross(cam.right, d));
    let fwd = RwV3dNormalize(cross(b, up));
    let side = RwV3dNormalize(cross(up, a));
    *out = add(scale(fwd, x), scale(side, y));
    mag
}

/// One floor probe (0x24 bytes at this+0xc0 + i*0x24).
#[derive(Debug, Clone, Copy, Default)]
pub struct ClimbProbe {
    /// +0x00 offset along the hero's at
    pub offset: f32,
    /// +0x04 world position
    pub pos: V3,
    /// +0x10 found a polygon
    pub found: u32,
    /// +0x14 weighted average normal
    pub normal: V3,
    /// +0x20 nearest weight distance (11 = none)
    pub dist: f32,
}

pub struct EFDMClimb {
    pub base: EFPhysicModule,
    /// +0x30..+0xa4 DMClimbParams (memcpy'd by Set/GetParams; +0x88/+0x94 double as bend state,
    /// +0xa0 is overwritten with the floor mask 0xa00 by UpdateFloorDetect)
    pub params: [u32; 29],
    /// +0xa4 InputActionGet(a, b) (re-queried, see module doc)
    pub input: (u32, u32),
    /// +0xa8 gravity velocity (0, -g, 0)
    pub gravity: V3,
    /// +0xb4/+0xb8/+0xbc bend bones (GetBone ids)
    pub bones: [Option<u32>; 3],
    /// +0xc0 five probes
    pub probes: [ClimbProbe; 5],
    /// +0x174 averaged surface normal
    pub normal: V3,
    /// +0x180 averaged side-probe normal
    pub side_normal: V3,
    /// +0x18c saved dyn-physic bsphere radius
    pub saved_radius: f32,
    /// +0x194 hip yaw angle
    pub hip: f32,
    /// +0x198 stick magnitude
    pub input_mag: f32,
}

impl Default for EFDMClimb {
    fn default() -> Self { Self::new() }
}

impl EFDMClimb {
    /// EFDMClimb(EFPhysic*) (0x00263f20)
    pub fn new() -> Self {
        let mut params = [0u32; 29];
        DMClimbParamsInit(&mut params);
        EFDMClimb {
            base: EFPhysicModule { type_: 0x5000_0021, flags: 1, ..Default::default() },
            params,
            input: (0, 0),
            gravity: [0.0; 3],
            bones: [None; 3],
            probes: [ClimbProbe::default(); 5],
            normal: [0.0; 3],
            side_normal: [0.0; 3],
            saved_radius: 0.0,
            hip: 0.0,
            input_mag: 0.0,
        }
    }

    /// param float at this+off
    pub fn pf(&self, off: usize) -> f32 { f32::from_bits(self.params[(off - 0x30) / 4]) }
    fn set_pf(&mut self, off: usize, v: f32) { self.params[(off - 0x30) / 4] = v.to_bits(); }
    fn pu(&self, off: usize) -> u32 { self.params[(off - 0x30) / 4] }

    /// RwMatrixRotate(deg, bone.modelling, axis(bone.modelling), POSTCONCAT) keeping the position.
    fn rotate_bone(ctx: &DMCtx, bone: Option<u32>, deg: f32, axis: impl Fn(&RwMatrix) -> V3) {
        let Some(b) = bone else { return };
        let Some((mut m, _)) = ctx.world.GetBone(ctx.phys.owner, b) else { return };
        let pos = m.pos;
        let ax = axis(&m);
        m.Rotate(deg, ax, rwCOMBINEPOSTCONCAT);
        m.pos = pos;
        ctx.world.SetBoneModelling(ctx.phys.owner, b, m);
    }

    /// UpdateFloorDetect(EFCollisionStage const&) (0x00264280)
    pub fn UpdateFloorDetect(&mut self, ctx: &DMCtx, stage: &stage::EFCollisionStage) {
        let ltm = ctx.phys.frame;
        let h = self.pf(0x64);
        let base = add(ltm.pos, scale(ltm.up, h));
        self.params[(0xa0 - 0x30) / 4] = 0xa00;
        for i in 0..5 {
            let mask = self.pu(0xa0);
            let p = &mut self.probes[i];
            p.pos = add(base, scale(ltm.at, p.offset));
            p.found = stage.FindWeightedAvgNormal(p.pos, mask, &mut p.normal, &mut p.dist) as u32;
        }
        self.normal = [0.0; 3];
        let pr = self.probes;
        let mut n = (pr[0].found != 0) as u8;
        if pr[0].found != 0 {
            self.normal = add(self.normal, pr[0].normal);
            // quirk: the side normal is only reset when the first probe hits
            self.side_normal = [0.0; 3];
        }
        for i in [1, 2] {
            if pr[i].found != 0 {
                n = n.wrapping_add(1);
                self.normal = add(self.normal, pr[i].normal);
                self.side_normal = add(self.side_normal, pr[i].normal);
            }
        }
        for i in [3, 4] {
            if pr[i].found != 0 {
                n = n.wrapping_add(1);
                self.normal = add(self.normal, pr[i].normal);
            }
        }
        if n != 0 {
            self.normal = RwV3dNormalize(self.normal);
            self.side_normal = RwV3dNormalize(self.side_normal);
        }
    }

    /// PostColUpdate body with the real stage (vtbl+0x14, 0x00264810).
    pub fn PostColUpdateStage(&mut self, ctx: &mut DMCtx, stage: &stage::EFCollisionStage, dp: Option<&mut EFDynamicPhysic>, dt: f32) {
        self.UpdateFloorDetect(ctx, stage);
        if self.pf(0x6c) < self.probes[2].dist {
            // quirk: state is zeroed before the change test, so the notify sees bit 2 changing
            // whenever bit 2 was not set (and nothing when it was)
            if self.base.state & 2 == 0 {
                self.base.state = 0;
                self.base.SetStateBits(2, ctx);
            } else {
                self.base.state = 2;
            }
            self.hip = 0.0;
            self.SetBSphereRadius(ctx, self.saved_radius, dp);
        } else {
            if self.base.state & 1 == 0 {
                self.base.state = 0;
                self.base.SetStateBits(1, ctx);
            } else {
                self.base.state = 1;
            }
            let mut m = ctx.phys.frame;
            self.RotateUpTowards(&mut m, self.normal, dt);
            ctx.phys.frame = m;
            if 0.0 < self.input_mag {
                let side = self.side_normal;
                let n = self.normal;
                self.BendBack(ctx, dt, side, n);
            }
        }
    }

    /// RotateUpTowards(DTMatrix*, DTVector3d const&, float) (0x002649c0)
    pub fn RotateUpTowards(&self, m: &mut RwMatrix, target: V3, dt: f32) {
        let mut axis = cross(m.up, target);
        let l = RwV3dLength(axis);
        let d = dot(m.up, target);
        let mut a;
        if (l.to_bits() & 0x7f80_0000) < 0x3800_0000 {
            a = if 0.0 < d { 0.0 } else { PI };
        } else {
            axis = scale(axis, 1.0 / l);
            a = if 1.0 <= l {
                1.5707964
            } else if l <= -1.0 {
                -1.5707964
            } else {
                l.asin()
            };
            if d < 0.0 {
                a = PI - a;
            }
        }
        let lim = self.pf(0x4c) * dt;
        if lim <= a {
            a = lim;
        }
        let pos = m.pos;
        m.Rotate(a * RAD2DEG, axis, rwCOMBINEPOSTCONCAT);
        m.pos = pos;
    }

    /// BendBack(float dt, DTVector3d side, DTVector3d const& normal) (0x00264c60)
    pub fn BendBack(&mut self, ctx: &DMCtx, dt: f32, side: V3, normal: V3) {
        let ltm = ctx.phys.frame;
        // about the hero's right axis
        let r = ltm.right;
        let a = RwV3dNormalize(sub(normal, scale(r, dot(normal, r))));
        let b = RwV3dNormalize(sub(side, scale(r, dot(side, r))));
        let mut axis = [0.0; 3];
        let mut ang = 0.0;
        GetRotationTo(a, b, &mut axis, &mut ang);
        if dot(axis, r) < 0.0 {
            ang *= -1.0;
        }
        let cur = self.pf(0x88);
        let tgt = ang * self.pf(0x84);
        let step = self.pf(0x7c) * dt;
        if cur < tgt {
            let v = cur + step;
            self.set_pf(0x88, v);
            if tgt < v {
                self.set_pf(0x88, tgt);
            }
        } else {
            let v = cur - step;
            self.set_pf(0x88, v);
            if v < tgt {
                self.set_pf(0x88, tgt);
            }
        }
        let deg = self.pf(0x88) * 0.33333 * RAD2DEG;
        for b in self.bones {
            Self::rotate_bone(ctx, b, deg, |m| m.right);
        }
        // about the hero's at axis
        let r = ltm.at;
        let a = RwV3dNormalize(sub(normal, scale(r, dot(normal, r))));
        let b = RwV3dNormalize(sub(side, scale(r, dot(side, r))));
        GetRotationTo(a, b, &mut axis, &mut ang);
        if dot(axis, r) < 0.0 {
            ang *= -1.0;
        }
        let cur = self.pf(0x94);
        let tgt = ang * self.pf(0x90);
        let step = self.pf(0x8c) * dt;
        if cur < tgt {
            let v = cur + step;
            self.set_pf(0x94, v);
            if tgt < v {
                self.set_pf(0x94, tgt);
            }
        } else {
            let v = cur - step;
            self.set_pf(0x94, v);
            if v < tgt {
                self.set_pf(0x94, tgt);
            }
        }
        Self::rotate_bone(ctx, self.bones[1], self.pf(0x94) * RAD2DEG, |m| m.up);
    }

    /// SetBSphereRadius(float, EFDynamicPhysic*) (0x00265210)
    pub fn SetBSphereRadius(&self, ctx: &mut DMCtx, r: f32, dp: Option<&mut EFDynamicPhysic>) {
        if let Some(dp) = dp {
            dp.radius = r;
        }
        ctx.phys.model.bsphere_radius = r;
    }

    /// InputMove(EFDynamicPhysic&, float) (0x00265490)
    pub fn InputMove(&mut self, ctx: &mut DMCtx, dp: &mut EFDynamicPhysic, dt: f32) {
        let cam = ctx.world.GetCameraLTM();
        let m = ctx.phys.frame;
        let inp = ctx.world.InputActionGet(self.input.0, self.input.1).unwrap_or([0.0, 0.0]);
        let mut dir = [0.0; 3];
        let mag = ProjectInputOntoLocalFloor(inp[0], inp[1], &cam, &m, m.up, &mut dir);
        self.input_mag = mag;
        if mag == 0.0 {
            Self::rotate_bone(ctx, self.bones[0], self.hip * RAD2DEG, |m| m.at);
        } else {
            let dir = RwV3dNormalize(dir);
            self.RotateHero(ctx, dp, dir, dt);
        }
    }

    /// RotateHero(EFDynamicPhysic&, DTVector3d&, float) (0x002655d0)
    pub fn RotateHero(&mut self, ctx: &mut DMCtx, dp: &mut EFDynamicPhysic, dir: V3, dt: f32) {
        let ltm = ctx.phys.frame;
        let m = ctx.phys.frame;
        let d = dot(m.at, dir);
        let mut a = 0.0f32;
        if d <= -1.0 {
            a = PI;
        } else if d < 1.0 {
            a = d.acos();
        }
        let mut step = self.pf(0x40) * dt;
        if a * self.input_mag <= step {
            step = a * self.input_mag;
        }
        if dot(cross(m.at, dir), ltm.up) < 0.0 {
            step = -step;
            a = -a;
        }
        self.hip = self.hip - (self.hip - -a * self.pf(0x98)) * 0.15;
        Self::rotate_bone(ctx, self.bones[0], self.hip * RAD2DEG, |m| m.at);
        let mut m = ctx.phys.frame;
        let pos = m.pos;
        m.Rotate(step * (self.input_mag * 0.6 + 1.0) * RAD2DEG, ltm.up, rwCOMBINEPOSTCONCAT);
        m.pos = pos;
        ctx.phys.frame = m;
        let sp = self.input_mag * (self.pf(0x38) - self.pf(0x3c)) + self.pf(0x3c);
        dp.disp = add(dp.disp, scale(m.at, sp));
    }
}

/// Rebuild a real EFCollisionStage contact list from the PhysicWorld seams.
pub fn StageFromWorld(ctx: &DMCtx) -> stage::EFCollisionStage {
    let polys = ctx.world.StageContactPolys(ctx.phys.owner);
    let mats = ctx.world.StageContactPolyMaterials(ctx.phys.owner);
    let env = polys
        .into_iter()
        .enumerate()
        .map(|(i, (normal, d, verts, edges))| {
            let (m1, m2) = mats.get(i).copied().unwrap_or((0, 0));
            let mut mat = [0u32; 8];
            mat[1] = m1;
            mat[2] = m2;
            EnvPrim { id: i, mat, w4: 0, nverts: verts.len() as u16, normal, d, verts, edges }
        })
        .collect();
    stage::EFCollisionStage { env, ..Default::default() }
}

impl EFDM for EFDMClimb {
    fn base(&self) -> &EFPhysicModule { &self.base }
    fn base_mut(&mut self) -> &mut EFPhysicModule { &mut self.base }
    fn ParamWords(&self) -> usize { 29 }
    /// SetParams(void*) (0x00264080)
    fn SetParams(&mut self, ctx: &mut DMCtx, p: Option<&[u32]>) {
        // quirk: no NULL check in the original (memcpy from NULL); None keeps the block
        if let Some(p) = p {
            self.params.copy_from_slice(&p[..29]);
        }
        let owner = ctx.phys.owner;
        for (k, off) in [0x70, 0x74, 0x78].into_iter().enumerate() {
            let id = self.pu(off);
            self.bones[k] = ctx.world.GetBone(owner, id).map(|_| id);
        }
        self.gravity = [0.0, -self.pf(0x48), 0.0];
        for (i, off) in [0x50, 0x54, 0x58, 0x5c, 0x60].into_iter().enumerate() {
            self.probes[i].offset = self.pf(off);
        }
        self.input = (self.pu(0x30), self.pu(0x34));
    }
    /// GetParams(void*) (0x00264250)
    fn GetParams(&self, _ctx: &DMCtx, p: &mut [u32]) {
        p[..29].copy_from_slice(&self.params);
    }
    /// DoAction(uint, void*) (0x00264240): any action resets the hip yaw and both bend angles.
    fn DoAction(&mut self, _ctx: &mut DMCtx, _action: u32, _p: &mut [u32]) {
        self.hip = 0.0;
        self.set_pf(0x88, 0.0);
        self.set_pf(0x94, 0.0);
    }
    /// PreColUpdate(EFDynamicPhysic&, float) (0x00265280)
    fn PreColUpdate(&mut self, ctx: &mut DMCtx, dp: &mut EFDynamicPhysic, dt: f32) -> bool {
        self.saved_radius = dp.radius;
        self.SetBSphereRadius(ctx, self.pf(0x68), Some(dp));
        let v = if self.base.state == 1 { ctx.phys.frame.TransformVector(self.gravity) } else { self.gravity };
        dp.accel = add(dp.accel, v);
        ctx.phys.velocity = add(ctx.phys.velocity, scale(v, dt));
        self.InputMove(ctx, dp, dt);
        true
    }
    fn PostColUpdate(&mut self, ctx: &mut DMCtx, _s: Option<&EFCollisionStage>, dt: f32) {
        let st = StageFromWorld(ctx);
        self.PostColUpdateStage(ctx, &st, None, dt);
    }
}

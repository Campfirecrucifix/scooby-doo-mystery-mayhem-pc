//! 0x1c EFDMBoneTrackObject (Classes/EFDM/EFDMBoneTrackObject.cpp, 0x94 bytes, type 0x4000001c),
//! 1:1: turns a bone (its modelling "at" axis) towards a target object/bone with a blend-in/out
//! timer. DM state bits: 4 tracking, 1 releasing, 2 released.
//! Seams: bone frames via `PhysicWorld::GetBone`/`SetBoneModelling`; EFObjectRetriever via
//! `PhysicWorld::RetrieverGet`; the uid map lookup via `PhysicWorld::GetObjectFrame`.
#![allow(non_snake_case)]

use crate::engine::physic::module::*;
use crate::engine::physic::rwm::*;

pub const NO_TARGET: u32 = 0x7fff_fffe;

pub struct EFDMBoneTrackObject {
    pub base: EFPhysicModule,
    /// +0x30 own bone (id or index)
    pub bone: u32,
    /// +0x34 target object id (0x7ffffffe = none; 0xff...... = retriever id)
    pub target: u32,
    /// +0x38 target bone (0 = the target's root frame)
    pub target_bone: u32,
    /// +0x3c offset in the target frame
    pub offset: V3,
    /// +0x48..+0x67 ObjectRetrieverParams (8 words)
    pub retr_params: [u32; 8],
    /// +0x68 blend time
    pub blend: f32,
    /// +0x6c max distance squared
    pub dist2: f32,
    /// +0x70 height band (0 = 3D distance)
    pub band: f32,
    /// +0x74 max angle (param +0x48)
    pub max_angle: f32,
    /// +0x78 max rotation step (param +0x44)
    pub max_step: f32,
    /// +0x7c invert the bone axis
    pub invert: u32,
    /// +0x80 EFObjectRetriever* != NULL
    pub retriever: bool,
    /// +0x84 bone axis (modelling at)
    pub axis: V3,
    /// +0x90 blend timer
    pub timer: f32,
}

impl EFDMBoneTrackObject {
    pub fn new() -> Self {
        EFDMBoneTrackObject {
            base: EFPhysicModule { type_: 0x4000_001c, flags: 1, ..Default::default() },
            bone: 0,
            target: 0xffff_ffff,
            target_bone: 0,
            offset: [0.0; 3],
            retr_params: [0; 8],
            blend: 0.0,
            dist2: 1.0,
            band: 0.0,
            max_angle: 0.0,
            max_step: 0.0,
            invert: 0,
            retriever: false,
            axis: [0.0, 0.0, 1.0],
            timer: 0.0,
        }
    }

    /// GetInterpolationFactor(dist², angle, dt)
    pub fn GetInterpolationFactor(&mut self, ctx: &mut DMCtx, d2: f32, angle: f32, dt: f32) -> f32 {
        let st = self.base.state;
        if st & 4 == 0 {
            if st & 1 == 0 {
                if d2 < self.dist2 && angle.abs() < self.max_angle {
                    self.timer = 0.0;
                    self.base.SetStateBits(4, ctx);
                    self.base.ResetStateBits(2, ctx);
                }
            } else if d2 < self.dist2 && angle.abs() < self.max_angle {
                self.base.ResetStateBits(1, ctx);
                self.base.SetStateBits(4, ctx);
            } else if self.timer < DT_EPSILON_FLOAT {
                self.base.ResetStateBits(1, ctx);
                self.base.SetStateBits(2, ctx);
                self.timer = 0.0;
            }
        } else if !(d2 < self.dist2) || angle.abs() > self.max_angle {
            self.base.ResetStateBits(4, ctx);
            self.base.SetStateBits(1, ctx);
        }
        let st = self.base.state;
        if st & 4 == 0 {
            if st & 1 != 0 {
                self.timer = if self.timer - dt < 0.0 { 0.0 } else { self.timer - dt };
            }
        } else if self.blend < self.timer + dt {
            self.timer = self.blend;
        } else {
            self.timer += dt;
        }
        if DT_EPSILON_FLOAT <= self.blend { self.timer / self.blend } else { 1.0 }
    }
}

impl EFDM for EFDMBoneTrackObject {
    fn base(&self) -> &EFPhysicModule { &self.base }
    fn base_mut(&mut self) -> &mut EFPhysicModule { &mut self.base }
    fn ParamWords(&self) -> usize { 20 }
    fn SetParams(&mut self, ctx: &mut DMCtx, p: Option<&[u32]>) {
        // EFWorld::DeleteRetriever(+0x80)
        self.retriever = false;
        let Some(p) = p else {
            self.bone = 0;
            self.target = 0xffff_ffff;
            self.target_bone = 0;
            self.offset = [0.0; 3];
            self.blend = 0.0;
            self.dist2 = 1.0;
            self.band = 0.0;
            self.max_angle = 0.0;
            self.max_step = 0.0;
            self.invert = 0;
            return;
        };
        self.bone = p[0];
        self.target = p[1];
        self.target_bone = p[2];
        self.offset = wv(p, 3);
        self.retr_params.copy_from_slice(&p[6..14]);
        self.blend = wf(p, 14);
        self.dist2 = wf(p, 15) * wf(p, 15);
        self.band = wf(p, 16);
        self.max_angle = wf(p, 18);
        self.max_step = wf(p, 17);
        self.invert = (p[19] != 0) as u32;
        if self.target & 0xff00_0000 == 0xff00_0000 {
            self.retriever = true;
        } else {
            let id = ResolveSelf(ctx, self.target);
            // DTMap Find + (obj+0xc & 8) == 0 -> obj+0x10 (instance) +8 uid
            self.target = if ctx.world.GetObjectFrame(id).is_some() { id } else { NO_TARGET };
        }
    }
    fn GetParams(&self, _c: &DMCtx, p: &mut [u32]) {
        p[0] = self.bone;
        p[1] = self.target;
        p[2] = self.target_bone;
        put_v(p, 3, self.offset);
        p[6..14].copy_from_slice(&self.retr_params);
        p[14] = fw(self.blend);
        p[15] = fw(self.dist2.sqrt());
        p[16] = fw(self.band);
        p[18] = fw(self.max_angle);
        p[17] = fw(self.max_step);
        p[19] = (self.invert != 0) as u32;
    }
    fn PostColUpdate(&mut self, ctx: &mut DMCtx, _stage: Option<&EFCollisionStage>, dt: f32) {
        if !self.retriever && self.target == NO_TARGET {
            return;
        }
        let owner = ctx.phys.owner;
        let Some((model, ltm)) = ctx.world.GetBone(owner, self.bone) else { return };
        self.axis = model.at;
        if self.invert != 0 {
            self.axis = scale(self.axis, -1.0);
        }
        self.axis = RwV3dNormalize(self.axis);
        let tgt = if self.retriever {
            ctx.world.RetrieverGet(self.target, &self.retr_params)
        } else {
            let id = ResolveSelf(ctx, self.target);
            ctx.world.GetObjectFrame(id).map(|_| id)
        };
        let Some(tgt) = tgt else { return };
        let Some(mut tm) = ctx.world.GetObjectFrame(tgt) else { return };
        if self.target_bone != 0 {
            let Some((_, l)) = ctx.world.GetBone(tgt, self.target_bone) else { return };
            tm = l;
        }
        let tp = tm.TransformPoint(self.offset);
        let d = sub(ltm.pos, tp);
        let d2 = if self.band == 0.0 {
            dot(d, d)
        } else if (d[1] < self.band && 0.0 <= d[1]) || (-self.band < d[1] && d[1] < 0.0) {
            d[2] * d[2] + d[0] * d[0]
        } else {
            self.dist2 + 0.01
        };
        let mut dir = ltm.Invert().TransformPoint(tp);
        if (dir[0].to_bits() & 0x7f80_0000) < 0x3c80_0000
            && (dir[1].to_bits() & 0x7f80_0000) < 0x3c80_0000
            && (dir[2].to_bits() & 0x7f80_0000) < 0x3c80_0000
        {
            dir = self.axis;
        } else {
            dir = RwV3dNormalize(dir);
        }
        let c = dot(dir, self.axis).max(0.0).min(1.0);
        let angle = c.acos();
        let f = self.GetInterpolationFactor(ctx, d2, angle, dt);
        if DT_EPSILON_FLOAT < f {
            let mut a = angle * f;
            let m = self.max_step;
            if a <= -m {
                a = -m;
            }
            let a = if a <= m { a } else { m };
            let ax = if (a.to_bits() & 0x7f80_0000) < 0x3d80_0000 { self.axis } else { cross(self.axis, dir) };
            let mut model = model;
            let pos = model.pos;
            model.Rotate(a * RAD2DEG, ax, rwCOMBINEPOSTCONCAT);
            model.pos = pos;
            ctx.world.SetBoneModelling(owner, self.bone, model);
        }
    }
}

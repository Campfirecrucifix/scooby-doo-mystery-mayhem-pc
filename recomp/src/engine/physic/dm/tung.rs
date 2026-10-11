//! 0x24 EFDMTung (Classes/EFDM/EFDMTung.cpp, 0xc0 bytes, type 0x40000024), 1:1: a tongue made of
//! two bones of the owner's HAnim hierarchy (+0x94 base bone, +0x98 tip bone) that shoots towards a
//! target point, sticks on a ray hit, then retracts (bending symmetrically) and shrinks away.
//! Exclusive DM states: 1 idle, 2 extending, 4 fully extended, 0x10 hit, 8 retracting.
//! Seams: frames are bone ids re-read through `PhysicWorld::GetBone` (modelling, LTM) and written
//! back with `SetBoneModelling` (the host recomputes the LTMs = RwFrameUpdateObjects); the ray
//! through `RayIntersectResult`; the hit physic's game object through `PhysicOwnerUid`; the
//! EFInstance::Notify(owner, 3, ..) message through `InstanceNotify`.
#![allow(non_snake_case)]

use crate::engine::physic::mgr::EFRayIntersectIn;
use crate::engine::physic::collision_map::RayIntersectIn;
use crate::engine::physic::module::*;
use crate::engine::physic::rwm::*;

pub struct EFDMTung {
    pub base: EFPhysicModule,
    /// +0x30 base bone (HAnim id or node index)
    pub bone: u32,
    /// +0x34 tip bone
    pub tip_bone: u32,
    /// +0x38 max length
    pub max_len: f32,
    /// +0x3c start length
    pub start_len: f32,
    /// +0x40 retract end distance
    pub min_dist: f32,
    /// +0x44 extend speed
    pub extend_speed: f32,
    /// +0x48 retract speed
    pub retract_speed: f32,
    /// +0x4c (param +0x1c, stored, unused)
    pub unk4c: u32,
    /// +0x50 bend factor (forced to 1.3 by PostColUpdate)
    pub bend: f32,
    /// +0x54 shoot request (0 = stop)
    pub active: u32,
    /// +0x58 target point
    pub target: V3,
    /// +0x64..+0x78 (zeroed by the ctor, unused)
    pub unk64: [u32; 6],
    /// +0x7c shrink time (forced to 0.06 by PostColUpdate)
    pub shrink_time: f32,
    /// +0x84 current length
    pub len: f32,
    /// +0x8c shrink timer
    pub shrink: f32,
    /// +0x90 root frame (node 0); None = NULL (no HAnim hierarchy)
    pub root_frame: Option<u32>,
    /// +0x94 base bone frame
    pub bone_frame: Option<u32>,
    /// +0x98 tip bone frame
    pub tip_frame: Option<u32>,
    /// +0x9c RwFrameGetLTM(+0x94) (snapshot of the pointed matrix)
    pub ltm: RwMatrix,
    /// +0xa4 RwMatrixCreate() scratch: inverse of +0x9c
    pub inv: RwMatrix,
    /// +0xa8 tip world position
    pub tip: V3,
    /// +0xb4 bend axis
    pub axis: V3,
}

/// RwMatrixScale(m, s, rwCOMBINEPOSTCONCAT) (local: rwm.rs is not part of this WP's edit set)
fn ScalePost(m: &RwMatrix, s: V3) -> RwMatrix {
    let f = |v: V3| [v[0] * s[0], v[1] * s[1], v[2] * s[2]];
    RwMatrix { right: f(m.right), up: f(m.up), at: f(m.at), pos: f(m.pos) }
}

impl Default for EFDMTung {
    fn default() -> Self { Self::new() }
}

impl EFDMTung {
    /// EFDMTung(EFPhysic*): the ctor's SetParams({1, 2, 8, 1, 0.01, 3, 3, 0, 3, 1, ?, 0, 0,0,0})
    /// then SetExclusiveState(1) (state 0 -> 1 with notify mask 0: no notify).
    pub fn new() -> Self {
        EFDMTung {
            base: EFPhysicModule { type_: 0x4000_0024, flags: 1, state: 1, ..Default::default() },
            bone: 1,
            tip_bone: 2,
            max_len: 8.0,
            start_len: 1.0,
            min_dist: 0.01,
            extend_speed: 3.0,
            retract_speed: 3.0,
            unk4c: 0,
            bend: 1.0,
            active: 0,
            target: [0.0; 3],
            unk64: [0; 6],
            shrink_time: 0.0,
            len: 0.0,
            shrink: 0.0,
            // not 1:1: the ctor's SetParams resolves the frames from the owner's 3d object, which
            // needs the world; the default bones are assumed resolvable (GetBone None = NULL).
            root_frame: Some(0),
            bone_frame: Some(1),
            tip_frame: Some(2),
            ltm: RwMatrix::identity(),
            inv: RwMatrix::identity(),
            tip: [0.0; 3],
            axis: [0.0; 3],
        }
    }

    /// SetExclusiveState(s): `(s & state) == 0` -> state = s and notify(6) when s & mask
    pub fn SetExclusiveState(&mut self, ctx: &mut DMCtx, s: u32) {
        if s & self.base.state == 0 {
            self.base.state = 0;
            let ch = self.base.state ^ s;
            self.base.state = s;
            if ch & self.base.notify_bits != 0 {
                ctx.notify.push(PhysicNotify { owner: ctx.phys.owner, code: 6, data: [self.base.handle, self.base.type_, self.base.state, ch] });
            }
        } else {
            self.base.state = s;
        }
    }

    pub fn GetExclusiveState(&self) -> u32 { self.base.state }

    fn Bone(ctx: &DMCtx, f: Option<u32>) -> Option<(RwMatrix, RwMatrix)> {
        ctx.world.GetBone(ctx.phys.owner, f?)
    }

    /// AlignLocalWithGlobal(global, local): local modelling = LTM(global) * inv(LTM(parent)),
    /// keeping its position.
    pub fn AlignLocalWithGlobal(&mut self, ctx: &mut DMCtx, global: Option<u32>, local: Option<u32>) {
        let (Some(lf), Some((mut m, ltm))) = (local, Self::Bone(ctx, local)) else { return };
        // parent LTM (frame+4) recovered as inv(modelling) * LTM (before the diagonal writes)
        let parent = RwMatrix::Multiply(&m.Invert(), &ltm);
        // quirk: only the diagonal is set to 1 (overwritten by the multiply below anyway)
        m.right[0] = 1.0;
        m.up[1] = 1.0;
        m.at[2] = 1.0;
        let pos = m.pos;
        let inv = parent.Invert();
        let g = Self::Bone(ctx, global).map(|b| b.1).unwrap_or_default();
        m = RwMatrix::Multiply(&g, &inv);
        m.pos = pos;
        ctx.world.SetBoneModelling(ctx.phys.owner, lf, m);
    }

    /// Bend_Symmetric: rotate the tip modelling about cross(LTM.at, dir) by
    /// len/3 * bend * |axis| (deg), keeping its position.
    pub fn Bend_Symmetric(&mut self, ctx: &mut DMCtx) {
        let mut d = sub(self.tip, self.ltm.pos);
        // quirk: "tiny" here is exponent < 0x3c800000 (|x| < 2^-6), not DTIsTiny
        let tiny = |x: f32| (x.to_bits() & 0x7f80_0000) < 0x3c80_0000;
        if tiny(d[0] - 0.0) && tiny(d[1] - 0.0) && tiny(d[2] - 0.0) {
            return;
        }
        let l = RwV3dLength(d);
        d = scale(d, 1.0 / l);
        let at = self.ltm.at;
        self.axis = [at[1] * d[2] - at[2] * d[1], at[2] * d[0] - at[0] * d[2], at[0] * d[1] - at[1] * d[0]];
        let a = RwV3dLength(self.axis);
        let (Some(tf), Some((mut m, _))) = (self.tip_frame, Self::Bone(ctx, self.tip_frame)) else { return };
        let pos = m.pos;
        m.Rotate((l / 3.0) * self.bend * a * RAD2DEG, self.axis, rwCOMBINEPOSTCONCAT);
        m.pos = pos;
        ctx.world.SetBoneModelling(ctx.phys.owner, tf, m);
    }

    /// UpdateTipLocalPosition(tip): tip modelling pos = inv(LTM +0x9c) * tip
    pub fn UpdateTipLocalPosition(&mut self, ctx: &mut DMCtx) {
        self.inv = self.ltm.Invert();
        let (Some(tf), Some((mut m, _))) = (self.tip_frame, Self::Bone(ctx, self.tip_frame)) else { return };
        m.pos = self.inv.TransformPoint(self.tip);
        ctx.world.SetBoneModelling(ctx.phys.owner, tf, m);
    }

    /// The RayIntersectIn of both rays: origin LTM pos, delta tip - origin, obj mask1 2, map
    /// mask1 1, exclude the owner. quirk: +0x1c obj mask2 / +0x24 map mask2 are uninitialised
    /// stack words (0 here).
    fn Ray(&self, ctx: &DMCtx) -> (EFRayIntersectIn, V3) {
        let delta = sub(self.tip, self.ltm.pos);
        let rin = EFRayIntersectIn {
            map: RayIntersectIn { origin: self.ltm.pos, dir: delta, mask1: 1, mask2: 0 },
            obj_mask1: 2,
            obj_mask2: 0,
            exclude: ctx.phys.owner,
        };
        (rin, delta)
    }

    /// EFInstance::Notify(owner, 3, {1, 0x66, hit uid, 0, 0,0,0,0, v, 0,0,0}, 0)
    fn NotifyHit(ctx: &DMCtx, physic: u32, v: V3) {
        let uid = ctx.world.PhysicOwnerUid(physic);
        let d = [1, 0x66, uid, 0, 0, 0, 0, 0, fw(v[0]), fw(v[1]), fw(v[2]), 0, 0, 0];
        ctx.world.InstanceNotify(ctx.phys.owner, 3, &d);
    }
}

impl EFDM for EFDMTung {
    fn base(&self) -> &EFPhysicModule { &self.base }
    fn base_mut(&mut self) -> &mut EFPhysicModule { &mut self.base }
    fn ParamWords(&self) -> usize { 15 }

    fn SetParams(&mut self, ctx: &mut DMCtx, p: Option<&[u32]>) {
        // quirk: no NULL branch in the original (it dereferences the block); NULL = no-op here
        let Some(p) = p else { return };
        self.bone = p[0];
        self.tip_bone = p[1];
        self.bend = wf(p, 9);
        self.max_len = wf(p, 2);
        self.start_len = wf(p, 3);
        self.min_dist = wf(p, 4);
        self.extend_speed = wf(p, 5);
        self.retract_speed = wf(p, 6);
        self.unk4c = p[7];
        self.active = p[11];
        self.target = wv(p, 12);
        // Get3dObject(owner, 0) frames: NULL without an HAnim hierarchy / bad bone
        let owner = ctx.phys.owner;
        self.bone_frame = ctx.world.GetBone(owner, self.bone).map(|_| self.bone);
        self.tip_frame = ctx.world.GetBone(owner, self.tip_bone).map(|_| self.tip_bone);
        self.root_frame = ctx.world.GetBone(owner, 0).map(|_| 0);
    }

    fn GetParams(&self, _ctx: &DMCtx, p: &mut [u32]) {
        p[0] = self.bone;
        p[1] = self.tip_bone;
        p[9] = fw(self.bend);
        p[2] = fw(self.max_len);
        p[3] = fw(self.start_len);
        p[4] = fw(self.min_dist);
        p[5] = fw(self.extend_speed);
        p[6] = fw(self.retract_speed);
        p[7] = self.unk4c;
        p[11] = self.active;
        put_v(p, 12, self.target);
    }

    fn PostColUpdate(&mut self, ctx: &mut DMCtx, _stage: Option<&EFCollisionStage>, dt: f32) {
        // quirk: the params are overwritten every frame
        self.bend = 1.3;
        self.shrink_time = 0.06;
        self.AlignLocalWithGlobal(ctx, self.root_frame, self.bone_frame);
        // quirk: the original dereferences NULL frames; skipped here
        let Some((_, ltm)) = Self::Bone(ctx, self.bone_frame) else { return };
        self.ltm = ltm;
        let st = self.GetExclusiveState();
        if st == 8 {
            let step = self.retract_speed * dt;
            self.len -= step;
            let d = sub(self.ltm.pos, self.tip);
            let mut dir = d;
            let mut rest = sub(d, scale(d, 0.95));
            let l = RwV3dLength(dir);
            dir = scale(dir, 1.0 / l); // quirk: divides before the l != 0 test
            if l != 0.0 {
                let n = scale(dir, -1.0);
                let at = self.ltm.at;
                let c = [n[1] * at[2] - n[2] * at[1], n[2] * at[0] - n[0] * at[2], n[0] * at[1] - n[1] * at[0]];
                let k = -(at[2] * dir[2] + at[0] * dir[0] + at[1] * dir[1]);
                // quirk: y component forced to 0 instead of c.z*n.x - c.x*n.z
                let mut b = [c[1] * n[2] - c[2] * n[1], 0.0, c[0] * n[1] - c[1] * n[0]];
                b = RwV3dNormalize(b);
                let k = 1.0 - k;
                dir = add(dir, scale(scale(b, k), l));
                rest = scale(rest, k);
            }
            dir = RwV3dNormalize(dir);
            if self.shrink_time == self.shrink && l < 1.6 {
                self.shrink -= dt;
            }
            if self.shrink_time == self.shrink {
                let (rin, delta) = self.Ray(ctx);
                if let Some(r) = ctx.world.RayIntersectResult(&rin) {
                    if r.kind == 1 {
                        Self::NotifyHit(ctx, r.physic, delta);
                    }
                }
                let mv = add(scale(dir, step), rest);
                self.tip = add(self.tip, mv);
                self.Bend_Symmetric(ctx);
                self.UpdateTipLocalPosition(ctx);
            } else {
                self.UpdateTipLocalPosition(ctx);
                let s = self.shrink / self.shrink_time;
                if let (Some(bf), Some((m, _))) = (self.bone_frame, Self::Bone(ctx, self.bone_frame)) {
                    ctx.world.SetBoneModelling(ctx.phys.owner, bf, ScalePost(&m, [s, s, s]));
                }
                self.shrink -= dt;
            }
            if self.shrink <= 0.0 || l <= self.min_dist {
                self.active = 0;
                if let (Some(bf), Some((m, _))) = (self.bone_frame, Self::Bone(ctx, self.bone_frame)) {
                    ctx.world.SetBoneModelling(ctx.phys.owner, bf, ScalePost(&m, [0.0; 3]));
                }
                self.SetExclusiveState(ctx, 1);
            }
        } else if st == 4 || st == 0x10 {
            self.SetExclusiveState(ctx, 8);
            self.UpdateTipLocalPosition(ctx);
        } else {
            if st != 2 {
                if st != 1 || self.active == 0 {
                    return;
                }
                self.len = self.start_len;
                self.shrink = self.shrink_time;
                self.tip = self.ltm.pos;
                // tip modelling = identity (flags |= 0x20003: RwMatrix dirty bits, host side)
                if let Some(tf) = self.tip_frame {
                    ctx.world.SetBoneModelling(ctx.phys.owner, tf, RwMatrix::identity());
                }
                self.SetExclusiveState(ctx, 2);
            }
            let old = self.len;
            let step = self.extend_speed * dt;
            self.len = old + step;
            if self.max_len <= old + step || self.active == 0 {
                self.len = self.max_len;
                self.SetExclusiveState(ctx, 4);
                self.UpdateTipLocalPosition(ctx);
            } else {
                let mv = scale(RwV3dNormalize(sub(self.target, self.tip)), step);
                let (rin, _) = self.Ray(ctx);
                match ctx.world.RayIntersectResult(&rin) {
                    None => {
                        self.tip = add(self.tip, mv);
                        self.UpdateTipLocalPosition(ctx);
                        let n = RwV3dNormalize(sub(self.tip, self.ltm.pos));
                        let rat = Self::Bone(ctx, self.root_frame).map(|b| b.1.at).unwrap_or([0.0, 0.0, 1.0]);
                        if n[2] * rat[2] + n[0] * rat[0] + n[1] * rat[1] < -0.7 {
                            self.SetExclusiveState(ctx, 8);
                        }
                    }
                    Some(r) => {
                        self.tip = r.pos;
                        self.target = self.tip;
                        self.len = self.max_len;
                        if r.kind == 1 {
                            Self::NotifyHit(ctx, r.physic, mv);
                        }
                        self.SetExclusiveState(ctx, 0x10);
                        self.UpdateTipLocalPosition(ctx);
                    }
                }
            }
        }
    }
}

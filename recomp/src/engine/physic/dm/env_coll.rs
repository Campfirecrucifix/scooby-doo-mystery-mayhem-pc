//! 0x22 EFDMEnvColl (Classes/EFDM/EFDMEnvColl.cpp, 0x7c bytes), 1:1 logic.
//!
//! Tests one collision prim of the owner (optionally offset / scaled for the test) against the
//! collision stage's env polygons: closest point of each polygon (plane inside the edges, else the
//! nearest edge segment) within the prim radius, confirmed by the prim's CheckPointIntersection
//! (vtbl+0x1c). State bit 0 = touching (first contact), bit 1 = contact lost (the module then
//! stops testing: PostColUpdate is skipped while state & 2). DoAction(1) reads the contact.
//!
//! Seam: the env prims of the stage (stage+4 / +8) — the EFDM trait path takes them from
//! `PhysicWorld::StageContactPolys` (+ material words 1/2); `PostColUpdateWith` takes the real
//! `EnvPrim`s of the EFCollisionStage. The prim is the real one of `ctx.phys.model`.
#![allow(non_snake_case)]

use crate::engine::physic::model::{ObjPrim, PrimExt};
use crate::engine::physic::module::*;
use crate::engine::physic::objcoll_check::EnvPrim;
use crate::engine::physic::rwm::*;

pub use super::ray_intersect::WorldStageEnvPrims;

pub struct EFDMEnvColl {
    pub base: EFPhysicModule,
    /// +0x30 prim index in the owner's collision model (-1 none)
    pub prim: i32,
    /// +0x34 test offset of the prim center
    pub offset: V3,
    /// +0x40 test scale of the prim (1.0)
    pub scale: f32,
    /// +0x44 env mask vs material word 2 (param +0x14)
    pub mask2: u32,
    /// +0x48 env mask vs material word 1 (param +0x1c)
    pub mask1: u32,
    /// +0x4c offset is frame-local (transformed by the owner LTM) (param +0x18 != 0)
    pub local: bool,
    /// +0x50 contact normal
    pub normal: V3,
    /// +0x5c contact material (None = the static zero record 0x01f7a580)
    pub mat: Option<[u32; 8]>,
    /// +0x60 contact point
    pub pos: V3,
    /// +0x6c saved prim radius (sphere) / sphere radius (swept)
    pub saved_r: f32,
    /// +0x70 saved prim vector (bbox half / swept local vector)
    pub saved_v: V3,
}

impl Default for EFDMEnvColl {
    fn default() -> Self { Self::new() }
}

#[inline]
fn d3(a: V3, b: V3) -> f32 { a[2] * b[2] + a[0] * b[0] + a[1] * b[1] }

/// The SweptSphere tiny-vector fix shared by InitCollPrim / ResetCollPrim (quirk: scales by
/// |v|*0.01). `radius` = prim +0xbc.
fn swept_fix(vec: &mut V3, sradius: f32, radius: &mut f32) {
    let l2 = vec[2] * vec[2] + vec[0] * vec[0] + vec[1] * vec[1];
    if l2 < 0.0001 {
        if l2 == 0.0 {
            *vec = [0.0, 0.01, 0.0];
        } else {
            let s = l2.sqrt() * 0.01;
            *vec = scale(*vec, s);
        }
        *radius = sradius + 0.01;
    }
}

impl EFDMEnvColl {
    /// EFDMEnvColl(EFPhysic*): type 0x40000022 (Post).
    pub fn new() -> Self {
        EFDMEnvColl {
            base: EFPhysicModule { type_: 0x4000_0022, flags: 1, ..Default::default() },
            prim: -1,
            offset: [0.0; 3],
            scale: 1.0,
            mask2: 0,
            mask1: 0,
            local: false,
            normal: [0.0; 3],
            mat: None,
            pos: [0.0; 3],
            saved_r: 0.0,
            saved_v: [0.0; 3],
        }
    }

    /// 0x00260330. Returns the (world) offset; `_center` (param_2) is unused by the C++.
    pub fn InitCollPrim(&mut self, ltm: &RwMatrix, p: &mut ObjPrim, _center: V3) -> V3 {
        let mut off = self.offset;
        if self.local {
            off = ltm.TransformVector(self.offset);
        }
        if self.scale != 1.0 {
            let k = self.scale;
            let radius = &mut p.base.radius;
            match (p.base.kind, &mut p.ext) {
                (2, PrimExt::BBox { half, .. }) => {
                    self.saved_v = *half;
                    *half = scale(self.saved_v, k);
                    *radius = RwV3dLength(*half);
                }
                (1, PrimExt::SweptSphere { vec, sradius, .. }) => {
                    self.saved_v = *vec;
                    self.saved_r = *sradius; // vtbl+0x28 GetMinRadius
                    *vec = scale(self.saved_v, k);
                    *radius = *sradius + RwV3dLength(*vec);
                    swept_fix(vec, *sradius, radius);
                    *sradius = self.scale * self.saved_r;
                    *radius = *sradius + RwV3dLength(*vec);
                }
                (0, _) => {
                    self.saved_r = *radius;
                    *radius = self.scale * self.saved_r;
                }
                _ => {}
            }
        }
        off
    }

    /// 0x00260660
    pub fn ResetCollPrim(&self, p: &mut ObjPrim) {
        let radius = &mut p.base.radius;
        match (p.base.kind, &mut p.ext) {
            (2, PrimExt::BBox { half, .. }) => {
                *half = self.saved_v;
                *radius = RwV3dLength(*half);
            }
            (1, PrimExt::SweptSphere { vec, sradius, .. }) => {
                *vec = self.saved_v;
                *radius = *sradius + RwV3dLength(*vec);
                swept_fix(vec, *sradius, radius);
                *sradius = self.saved_r;
                *radius = *sradius + RwV3dLength(*vec);
            }
            (0, _) => *radius = self.saved_r,
            _ => {}
        }
    }

    /// 0x0025faa0 with explicit stage env prims. `stage` = EFCollisionStage* != NULL.
    pub fn PostColUpdateWith(&mut self, ctx: &mut DMCtx, stage: bool, env: &[EnvPrim]) {
        if self.base.state & 2 != 0 || !stage {
            return;
        }
        let ltm = ctx.phys.frame;
        // physic+0x58[prim] (the C++ derefs NULL when missing)
        let Some(Some(prim)) = (if self.prim < 0 { None } else { ctx.phys.model.prims.get_mut(self.prim as usize) }) else { return };
        let mut center = prim.base.center;
        let mut off = [0.0f32; 3];
        let tiny = |x: f32| (x.to_bits() & 0x7f80_0000) < 0x3c80_0000;
        let all_tiny = tiny(self.offset[0] - 0.0) && tiny(self.offset[1] - 0.0) && tiny(self.offset[2] - 0.0);
        if !all_tiny || self.scale != 1.0 {
            off = self.InitCollPrim(&ltm, prim, center);
            center = add(center, off);
        }
        let r2 = prim.base.radius * prim.base.radius;
        let mut best = r2;
        let mut found = false;
        let mut cv = [0.0f32; 3];
        for e in env {
            if found {
                break;
            }
            if e.mat[2] & self.mask2 == 0 && e.mat[1] & self.mask1 == 0 {
                continue;
            }
            // farthest positive edge plane (raw u16 count)
            let mut dmax = 0.0f32;
            let mut imax = 0usize;
            for i in 0..e.nverts as usize {
                let Some(ep) = e.edges.get(i) else { break };
                let v = ep[3] + center[2] * ep[2] + center[0] * ep[0] + center[1] * ep[1];
                if v > dmax {
                    dmax = v;
                    imax = i;
                }
            }
            let kind;
            if dmax == 0.0 {
                let dist = e.d + e.normal[2] * center[2] + e.normal[0] * center[0] + e.normal[1] * center[1];
                if best < dist * dist {
                    kind = 0;
                } else {
                    cv = scale(e.normal, -dist);
                    kind = 1;
                    best = dist * dist;
                }
            } else {
                let mut j = imax + 1;
                if j == e.nverts as usize {
                    j = 0;
                }
                let v0 = e.verts.get(imax).copied().unwrap_or_default();
                let v1 = e.verts.get(j).copied().unwrap_or_default();
                let ed = sub(v1, v0);
                let w = sub(v0, center);
                let t = d3(ed, w);
                let l2 = d3(ed, ed);
                let mut c = w;
                if 0.0 < t {
                    kind = 3;
                } else if t < -l2 {
                    c = add(w, ed);
                    kind = 3;
                } else {
                    c = sub(w, scale(ed, t / l2));
                    kind = 2;
                }
                let d2 = d3(c, c);
                if best < d2 {
                    continue; // iVar12 = 0
                }
                cv = c;
                best = d2;
            }
            if kind != 0 {
                let p = sub(add(cv, center), off);
                if prim.CheckPointIntersection(p) {
                    self.normal = e.normal;
                    self.mat = Some(e.mat);
                    self.pos = add(cv, center);
                    found = true;
                }
                best = r2;
            }
        }
        if self.scale != 1.0 {
            self.ResetCollPrim(prim);
        }
        if found || self.base.state & 1 == 0 {
            if found && self.base.state == 0 {
                self.base.state = 0;
                self.base.SetStateBits(1, ctx);
            }
        } else {
            self.normal = [0.0; 3];
            self.mat = None;
            self.pos = [0.0; 3];
            if self.base.state & 2 == 0 {
                self.base.state = 0;
                self.base.SetStateBits(2, ctx);
            } else {
                self.base.state = 2;
            }
        }
    }
}

impl EFDM for EFDMEnvColl {
    fn base(&self) -> &EFPhysicModule { &self.base }
    fn base_mut(&mut self) -> &mut EFPhysicModule { &mut self.base }
    fn ParamWords(&self) -> usize { 8 }
    fn PostColUpdate(&mut self, ctx: &mut DMCtx, stage: Option<&EFCollisionStage>, _dt: f32) {
        // real stage: stage+0x4/+0x8 env prims (WP-J4); seam stage: PhysicWorld contact polys
        if let Some(st) = stage.filter(|s| s.ray.is_some()) {
            return self.PostColUpdateWith(ctx, true, st.env);
        }
        let env = if stage.is_some() { WorldStageEnvPrims(ctx.world, ctx.phys.owner) } else { Vec::new() };
        self.PostColUpdateWith(ctx, stage.is_some(), &env);
    }
    /// 0x0025f8c0
    fn SetParams(&mut self, ctx: &mut DMCtx, p: Option<&[u32]>) {
        match p {
            None => {
                self.prim = -1;
                self.offset = [0.0; 3];
                self.scale = 1.0;
                self.mask2 = 0;
                self.mask1 = 0;
                self.local = false;
            }
            Some(p) => {
                self.prim = ctx.phys.model.PrimTagToPrimIndex(p[0]) as i32;
                self.offset = wv(p, 1);
                self.scale = wf(p, 4);
                self.mask2 = p[5];
                self.mask1 = p[7];
                self.local = p[6] != 0;
            }
        }
    }
    /// 0x0025f980 (quirk: prim index 0 reports tag 0)
    fn GetParams(&self, ctx: &DMCtx, p: &mut [u32]) {
        p[0] = if self.prim < 1 {
            0
        } else {
            ctx.phys.model.prims.get(self.prim as usize).and_then(|q| q.as_ref()).map_or(0, |q| q.base.tag)
        };
        put_v(p, 1, self.offset);
        p[4] = fw(self.scale);
        p[5] = self.mask2;
        p[7] = self.mask1;
        p[6] = self.local as u32;
    }
    /// 0x0025fa10: action 1 = contact {normal, material record (8 words), point} (14 words)
    fn DoAction(&mut self, _ctx: &mut DMCtx, action: u32, p: &mut [u32]) {
        if action == 1 {
            put_v(p, 0, self.normal);
            p[3..11].copy_from_slice(&self.mat.unwrap_or([0; 8]));
            put_v(p, 11, self.pos);
        }
    }
}

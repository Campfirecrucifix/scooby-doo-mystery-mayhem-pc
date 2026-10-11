//! 0x1e EFDMRopeDetect (Classes/EFDM/EFDMRopeDetect.cpp, 0x50 bytes), 1:1.
//! Seams: _gpoSplineMgr is passed to `Detect` (the EFDM trait has no spline access; the trait
//! PostColUpdate runs Detect over an empty spline set). The prim's GetRadius virtual (vtbl+0x28)
//! is the `radius(prim index)` closure (EFObjCollisionPrimSeam has no radius).
#![allow(non_snake_case)]

use crate::engine::physic::module::*;
use crate::engine::spline::EFSplineMgr;

pub struct EFDMRopeDetect {
    pub base: EFPhysicModule,
    /// +0x30..+0x40 five prim indices (-1 = unused)
    pub prims: [u32; 5],
    /// +0x44 nearest spline index (-1)
    pub spline: u32,
    /// +0x48 tag of the prim that touched it (-1)
    pub prim_tag: u32,
    /// +0x4c spline +2 type word
    pub spline_ty: u32,
}

impl EFDMRopeDetect {
    pub fn new() -> Self {
        EFDMRopeDetect {
            base: EFPhysicModule { type_: 0x4000_001e, flags: 1, ..Default::default() },
            prims: [0xffff_ffff; 5],
            spline: 0,
            prim_tag: 0,
            spline_ty: 0,
        }
    }

    /// PostColUpdate body.
    pub fn Detect(&mut self, ctx: &mut DMCtx, splines: &EFSplineMgr, radius: &dyn Fn(u32) -> f32) {
        let mut best = f32::MAX; // DT_MAX_FLOAT
        let mut found = false;
        self.spline = 0xffff_ffff;
        self.prim_tag = 0xffff_ffff;
        for &pi in &self.prims {
            if pi == 0xffff_ffff {
                continue;
            }
            let Some(Some(prim)) = ctx.phys.model.prims.get(pi as usize) else { continue };
            let c = prim.base.center;
            let r = radius(pi);
            for (si, s) in splines.splines.iter().enumerate() {
                let mut f = 0.0;
                if s.CheckSphereIntersection(c, r, &mut f) {
                    let g = s.AdjustGlobalFactor(f) * s.segs.len() as f32;
                    // (uint) conversion as compiled (values >= 2^31 handled by the |0x80000000 path)
                    let i = if g < 2.1474836e9 { g as i32 as u32 } else { ((g - 2.1474836e9) as i32 as u32) | 0x8000_0000 };
                    let p = s.GetPoint(i as i32, g - i as f32);
                    let d = [p[0] - c[0], p[1] - c[1], p[2] - c[2]];
                    let dd = d[2] * d[2] + d[0] * d[0] + d[1] * d[1];
                    if dd < best {
                        found = true;
                        self.spline = si as u32;
                        self.prim_tag = prim.base.tag;
                        self.spline_ty = s.ty as u32;
                        best = dd;
                    }
                }
            }
        }
        if found { self.base.SetStateBits(2, ctx) } else { self.base.ResetStateBits(2, ctx) }
    }
}

impl EFDM for EFDMRopeDetect {
    fn base(&self) -> &EFPhysicModule { &self.base }
    fn base_mut(&mut self) -> &mut EFPhysicModule { &mut self.base }
    fn ParamWords(&self) -> usize { 5 }
    fn DataWords(&self) -> usize { 3 }
    fn SetParams(&mut self, ctx: &mut DMCtx, p: Option<&[u32]>) {
        match p {
            None => self.prims = [0xffff_ffff; 5],
            Some(p) => {
                for i in 0..5 {
                    self.prims[i] = if p[i] == 0xffff_ffff { 0xffff_ffff } else { ctx.phys.model.PrimTagToPrimIndex(p[i]) };
                }
            }
        }
    }
    fn GetParams(&self, ctx: &DMCtx, p: &mut [u32]) {
        for i in 0..5 {
            p[i] = if self.prims[i] == 0xffff_ffff {
                0xffff_ffff
            } else {
                ctx.phys.model.prims.get(self.prims[i] as usize).and_then(|q| q.as_ref()).map_or(0, |q| q.base.tag)
            };
        }
    }
    fn PostColUpdate(&mut self, ctx: &mut DMCtx, _s: Option<&EFCollisionStage>, _dt: f32) {
        self.Detect(ctx, &EFSplineMgr::default(), &|_| 0.0);
    }
    fn GetData(&self, _c: &DMCtx, p: &mut [u32]) {
        p[0] = self.spline;
        p[1] = self.prim_tag;
        p[2] = self.spline_ty;
    }
}

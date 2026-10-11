//! 0x17 EFDMEdge (Classes/EFDM/EFDMEdge.cpp, 0x44 bytes), 1:1: snaps a prim of the owner onto
//! the nearest walkable env edge (hang on ledge) and turns the owner to face it.
//! Seams: the stage's touched env prims are passed as `EnvPrimSeam`s (build them with
//! `EnvPrimSeam::from_map` from `stage::EFCollisionStage::env_prims`), and the fields of the
//! owner's EFObjCollisionPrim that the seam lacks (+0xbc radius, +0x20/+0x24 masks) as `EdgePrim`.
//! The trait PostColUpdate runs the body with no env prims. The prim's vtbl+0xc call before
//! EFCollisionModel::Translate is not modelled; Translate moves the seam prims' world positions.
#![allow(non_snake_case)]

use super::envdetect::EnvPrimSeam;
use crate::engine::physic::module::*;
use crate::engine::physic::rwm::*;

/// The owner prim's fields read by EFDMEdge that EFObjCollisionPrimSeam does not carry.
#[derive(Debug, Clone, Copy, Default)]
pub struct EdgePrim {
    /// prim+0xbc radius
    pub radius: f32,
    /// prim+0x20 / +0x24 env masks
    pub mask1: u32,
    pub mask2: u32,
}

pub struct EFDMEdge {
    pub base: EFPhysicModule,
    /// +0x30 prim index
    pub prim: u32,
    /// +0x34 mask bits of the matched env material
    pub hit_mask: u32,
    /// +0x38 flip flag (negates z of the edge direction)
    pub flip: u32,
    /// +0x3c probe offset along the owner's at axis (captured when state bit0 is clear)
    pub offset: f32,
    /// +0x40 last frame
    pub frame: u32,
}

fn plane(p: [f32; 4], v: V3) -> f32 { p[3] + v[2] * p[2] + v[0] * p[0] + v[1] * p[1] }

impl EFDMEdge {
    pub fn new() -> Self {
        EFDMEdge { base: EFPhysicModule { type_: 0x4000_0017, flags: 1, ..Default::default() }, prim: 0, hit_mask: 0, flip: 0, offset: 0.0, frame: 0 }
    }

    /// EFDMEdge::IsInSomeOtherPrim(pt, dir, stage)
    pub fn IsInSomeOtherPrim(pt: V3, dir: V3, prims: &[EnvPrimSeam]) -> bool {
        for e in prims {
            let n = [e.plane[0], e.plane[1], e.plane[2]];
            let t = plane(e.plane, pt) / (dir[2] * n[2] + dir[0] * n[0] + dir[1] * n[1]);
            if t <= 1.0 && 0.0 <= t {
                let x = add(pt, scale(dir, t));
                if !e.edges.iter().any(|&ep| 0.0 < plane(ep, x)) {
                    return true;
                }
            }
        }
        false
    }

    /// PostColUpdate body (stage != NULL).
    pub fn Edge(&mut self, ctx: &mut DMCtx, prims: &[EnvPrimSeam], ep: EdgePrim) {
        if self.frame.wrapping_add(1) != ctx.frame {
            self.base.state = 0;
        }
        self.frame = ctx.frame;
        let Some(Some(pr)) = ctx.phys.model.prims.get(self.prim as usize) else { return };
        let c = pr.base.center;
        let ltm = ctx.phys.frame.clone();
        if self.base.state & 1 == 0 {
            let d = sub(c, ltm.pos);
            self.offset = -(d[2] * ltm.at[2] + d[0] * ltm.at[0] + d[1] * ltm.at[1]);
        }
        let r = ep.radius;
        let mut best = f32::MAX;
        let mut found: Option<V3> = None;
        let p = add(scale(ltm.at, self.offset), c);
        for e in prims {
            let m = e.mask1 & ep.mask1;
            if m == 0 && e.mask2 & ep.mask2 == 0 {
                continue;
            }
            self.hit_mask = if m != 0 { m } else { e.mask2 & ep.mask2 };
            if !(0.6 <= e.plane[1]) {
                continue;
            }
            let d = plane(e.plane, c);
            if (d.abs() as f64) > r as f64 {
                continue;
            }
            let n = e.edges.len();
            let mut sel = u32::MAX as usize;
            let mut bd = -10000.0f32;
            let mut broke = false;
            for (j, &pl) in e.edges.iter().enumerate() {
                let f = plane(pl, c);
                if bd < f {
                    bd = f;
                    sel = j;
                    if 0.0 < f && r * r < d * d + f * f {
                        broke = true; // sphere is outside past this edge: prim skipped
                        break;
                    }
                }
            }
            if broke || sel >= n {
                continue;
            }
            let v0 = e.verts[sel];
            let v1 = e.verts[if sel + 1 == n { 0 } else { sel + 1 }];
            let seg = sub(v1, v0);
            let t = dot(sub(p, v0), seg) / dot(seg, seg);
            let q = if 1.0 < t { add(seg, v0) } else if t < 0.0 { v0 } else { add(scale(seg, t), v0) };
            let w = sub(p, q);
            let k = 0.01 / RwV3dLength(w);
            let tp = [w[0] * k + q[0], (w[1] * k + q[1]) - 0.05, w[2] * k + q[2]];
            if !Self::IsInSomeOtherPrim(tp, [0.0, r * 1.1, 0.0], prims) {
                let dd = w[2] * w[2] + w[0] * w[0] + w[1] * w[1];
                if dd <= best {
                    found = Some(q);
                    best = dd;
                }
            }
        }
        match found {
            Some(q) => {
                let a = sub(c, p);
                let a1 = a[0].atan2(a[2]);
                let mut b = sub(q, p);
                if self.flip != 0 {
                    b[2] *= -1.0;
                }
                let a2 = b[0].atan2(b[2]);
                let f = &mut ctx.phys.frame;
                f.up = [0.0, 1.0, 0.0];
                f.at[1] = 0.0;
                f.at = RwV3dNormalize(f.at);
                f.right = cross(f.up, f.at);
                let pos = f.pos;
                f.Rotate((a2 - a1) * RAD2DEG, [0.0, 1.0, 0.0], 2);
                f.pos = pos;
                // EFCollisionModel::Translate(q - prim world pos)
                let dlt = sub(q, c);
                if ctx.phys.model.Translate(dlt) {
                    ctx.phys.frame.Translate(dlt, 2);
                }
                ctx.phys.model.translated = false;
                ctx.phys.moved = true;
                ctx.phys.velocity = [0.0; 3];
                ctx.phys.falling_timer = ctx.phys.falling_delay;
                self.base.SetStateBits(1, ctx);
            }
            None => {
                self.hit_mask = 0;
                self.base.ResetStateBits(1, ctx);
            }
        }
    }
}

impl EFDM for EFDMEdge {
    fn base(&self) -> &EFPhysicModule { &self.base }
    fn base_mut(&mut self) -> &mut EFPhysicModule { &mut self.base }
    fn ParamWords(&self) -> usize { 3 }
    fn SetParams(&mut self, ctx: &mut DMCtx, p: Option<&[u32]>) {
        match p {
            None => self.prim = 0,
            Some(p) => {
                self.prim = ctx.phys.model.PrimTagToPrimIndex(p[0]);
                self.flip = p[2];
            }
        }
        self.hit_mask = 0;
    }
    fn GetParams(&self, ctx: &DMCtx, p: &mut [u32]) {
        p[0] = ctx.phys.model.prims.get(self.prim as usize).and_then(|q| q.as_ref()).map_or(0, |q| q.base.tag);
        p[1] = self.hit_mask;
        p[2] = self.flip;
    }
    fn PostColUpdate(&mut self, ctx: &mut DMCtx, stage: Option<&EFCollisionStage>, _dt: f32) {
        if stage.is_some() {
            self.Edge(ctx, &[], EdgePrim::default());
        }
    }
}

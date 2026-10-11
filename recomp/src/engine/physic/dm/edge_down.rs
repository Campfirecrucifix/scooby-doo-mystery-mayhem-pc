//! 0x13 EFDMEdgeDown (Classes/EFDM/EFDMEdgeDown.cpp, 0x58 bytes, DMAdd case 0x13 inline ctor), 1:1:
//! snaps a prim of the owner onto the nearest env edge facing the owner's at axis (climb down a
//! ledge), turns the owner toward it, then PreColUpdate rotates the owner by pi about the edge
//! over `time` seconds.
//!
//! Seams: the trait's `EFCollisionStage` (module.rs) carries no env prim list, so the body
//! `EdgeDown` takes the stage's touched env prims (stage+4/+8) as `objcoll_check::EnvPrim`s; the
//! trait PostColUpdate runs it with none. The prim's vtbl+0xc (PrecalcPrimData) after the frame
//! rotation is modelled for root-frame prims (joint 0xffffffff) by resyncing the prim LTM from the
//! owner frame; bone prims are recomputed by the owner.
#![allow(non_snake_case)]

use crate::engine::physic::module::*;
use crate::engine::physic::objcoll_check::EnvPrim;
use crate::engine::physic::rwm::*;

pub struct EFDMEdgeDown {
    pub base: EFPhysicModule,
    /// +0x30 prim index
    pub prim: u32,
    /// +0x34 rotation time (param)
    pub time: f32,
    /// +0x38 mask bits of the matched env material
    pub hit_mask: u32,
    /// +0x3c edge vector (rotation axis)
    pub edge: V3,
    /// +0x48 angular speed (pi / time)
    pub speed: f32,
    /// +0x4c last PostColUpdate frame
    pub post_frame: u32,
    /// +0x50 accumulated angle
    pub angle: f32,
    /// +0x54 last PreColUpdate frame
    pub pre_frame: u32,
}

#[inline]
fn plane3(n: V3, d: f32, v: V3) -> f32 { v[2] * n[2] + v[0] * n[0] + v[1] * n[1] + d }

/// Resync a root-frame prim's LTM from the owner frame (keeps the 4th column).
pub(crate) fn sync_prim_ltm(ltm: &mut [[f32; 4]; 4], f: &RwMatrix) {
    for (r, v) in ltm.iter_mut().zip([f.right, f.up, f.at, f.pos]) {
        r[0] = v[0];
        r[1] = v[1];
        r[2] = v[2];
    }
}

impl EFDMEdgeDown {
    pub fn new() -> Self {
        EFDMEdgeDown {
            base: EFPhysicModule { type_: 0x5000_0013, flags: 1, ..Default::default() },
            prim: 0,
            time: 0.0,
            hit_mask: 0,
            edge: [0.0; 3],
            speed: 0.0,
            post_frame: 0,
            angle: 0.0,
            pre_frame: 0,
        }
    }

    /// IsInSomeOtherPrim(pt, dir, stage)
    pub fn IsInSomeOtherPrim(&self, pt: V3, dir: V3, env: &[EnvPrim]) -> bool {
        for e in env {
            let t = plane3(e.normal, e.d, pt) / (dir[2] * e.normal[2] + dir[0] * e.normal[0] + dir[1] * e.normal[1]);
            if t <= 1.0 && 0.0 <= t {
                let x = add(pt, scale(dir, t));
                // quirk: the loop runs over the raw u16 count; EnvPrim keeps `& 0x7fff` planes
                if !e.edges.iter().any(|ep| 0.0 < ep[3] + x[2] * ep[2] + x[0] * ep[0] + x[1] * ep[1]) {
                    return true;
                }
            }
        }
        false
    }

    /// PostColUpdate body (stage != NULL).
    pub fn EdgeDown(&mut self, ctx: &mut DMCtx, env: &[EnvPrim]) {
        if self.post_frame.wrapping_add(1) != ctx.frame {
            self.base.state = 0;
        }
        self.post_frame = ctx.frame;
        if self.base.state & 1 != 0 {
            return;
        }
        let Some(Some(pr)) = ctx.phys.model.prims.get(self.prim as usize) else { return };
        let c = pr.base.center;
        let r = pr.base.radius;
        let (m1, m2) = (pr.base.word(0x20), pr.base.word(0x24));
        let at = ctx.phys.frame.at;
        let d0 = sub(c, ctx.phys.frame.pos);
        let k = -(d0[2] * at[2] + d0[0] * at[0] + d0[1] * at[1]);
        let p = add(scale(at, k), c);
        let mut best = f32::MAX;
        let mut found: Option<V3> = None;
        for e in env {
            let m = e.mat[1] & m1;
            if m == 0 && e.mat[2] & m2 == 0 {
                continue;
            }
            self.hit_mask = m;
            if self.hit_mask == 0 {
                self.hit_mask = e.mat[2] & m2;
            }
            let n = e.normal;
            if !(0.25 < at[2] * n[2] + at[0] * n[0] + at[1] * n[1]) {
                continue;
            }
            let dist = n[2] * c[2] + n[0] * c[0] + n[1] * c[1] + e.d;
            if (dist.abs() as f64) > r as f64 {
                continue;
            }
            let cnt = e.edges.len();
            let mut sel = u32::MAX as usize;
            let mut bd = -10000.0f32;
            let mut broke = false;
            for (j, pl) in e.edges.iter().enumerate() {
                let f = pl[3] + c[2] * pl[2] + c[0] * pl[0] + c[1] * pl[1];
                if bd < f {
                    bd = f;
                    sel = j;
                    if 0.0 < f && r * r < dist * dist + f * f {
                        broke = true;
                        break;
                    }
                }
            }
            // quirk: the binary reads vertex[-1] when no edge plane beat -10000; skipped here
            if broke || sel >= cnt {
                continue;
            }
            let v0 = e.verts[sel];
            let v1 = e.verts[if sel + 1 == cnt { 0 } else { sel + 1 }];
            let seg = sub(v1, v0);
            let w0 = sub(p, v0);
            let t = (w0[2] * seg[2] + w0[0] * seg[0] + w0[1] * seg[1]) / (seg[2] * seg[2] + seg[0] * seg[0] + seg[1] * seg[1]);
            let q = if 1.0 < t { add(seg, v0) } else if t < 0.0 { v0 } else { add(scale(seg, t), v0) };
            let w = sub(p, q);
            let kk = 0.01 / RwV3dLength(w);
            let a = r * 1.1;
            // quirk: -0.05 on x and z (EFDMEdge offsets y)
            let tp = [(w[0] * kk + q[0]) - 0.05, w[1] * kk + q[1], (w[2] * kk + q[2]) - 0.05];
            if !self.IsInSomeOtherPrim(tp, [a, 0.0, a], env) {
                let dd = sub(p, q);
                let l = dd[2] * dd[2] + dd[0] * dd[0] + dd[1] * dd[1];
                if l <= best {
                    found = Some(q);
                    self.edge = seg;
                    best = l;
                }
            }
        }
        match found {
            Some(q) => {
                let a = sub(c, p);
                let b = sub(q, p);
                let a1 = a[0].atan2(a[2]);
                let a2 = b[0].atan2(b[2]);
                let f = &mut ctx.phys.frame;
                let pos = f.pos;
                f.Rotate((a2 - a1) * RAD2DEG, [0.0, 1.0, 0.0], 2);
                f.pos = pos;
                let frame = ctx.phys.frame.clone();
                // prim vtbl+0xc (PrecalcPrimData) after the rotation
                let pr = ctx.phys.model.prims[self.prim as usize].as_mut().unwrap();
                if pr.base.joint == 0xffff_ffff {
                    sync_prim_ltm(&mut pr.base.ltm, &frame);
                }
                pr.PrecalcPrimData();
                let dlt = sub(q, pr.base.center);
                if ctx.phys.model.Translate(dlt) {
                    ctx.phys.frame.Translate(dlt, 2);
                }
                ctx.phys.velocity = [0.0; 3];
                ctx.phys.falling_timer = ctx.phys.falling_delay;
                self.base.SetStateBits(1, ctx);
            }
            None => self.base.ResetStateBits(1, ctx),
        }
    }
}

impl EFDM for EFDMEdgeDown {
    fn base(&self) -> &EFPhysicModule { &self.base }
    fn base_mut(&mut self) -> &mut EFPhysicModule { &mut self.base }
    fn ParamWords(&self) -> usize { 3 }
    fn PreColUpdate(&mut self, ctx: &mut DMCtx, _dp: &mut EFDynamicPhysic, dt: f32) -> bool {
        let fr = ctx.frame;
        if self.pre_frame.wrapping_add(1) != fr {
            self.angle = 0.0;
        }
        if self.base.state & 1 != 0 && 0.0 < self.speed {
            let mut step = self.speed * dt;
            let a = self.angle + step;
            if 3.1415927 < a {
                self.base.ResetStateBits(1, ctx);
                step = 3.1415927 - self.angle;
                self.angle = 0.0;
            } else {
                self.angle = a;
            }
            let f = &mut ctx.phys.frame;
            let pos = f.pos;
            f.Rotate(step * RAD2DEG, self.edge, 2);
            f.pos = pos;
        }
        self.pre_frame = fr;
        true
    }
    fn SetParams(&mut self, ctx: &mut DMCtx, p: Option<&[u32]>) {
        match p {
            None => {
                self.prim = 0;
                self.time = 0.0;
                self.speed = 0.0;
            }
            Some(p) => {
                self.prim = ctx.phys.model.PrimTagToPrimIndex(p[0]);
                self.time = wf(p, 1);
                self.speed = if self.time == 0.0 { 0.0 } else { 3.1415927 / self.time };
            }
        }
        self.hit_mask = 0;
    }
    fn GetParams(&self, ctx: &DMCtx, p: &mut [u32]) {
        p[0] = ctx.phys.model.prims.get(self.prim as usize).and_then(|q| q.as_ref()).map_or(0, |q| q.base.tag);
        p[1] = fw(self.time);
        p[2] = self.hit_mask;
    }
    fn PostColUpdate(&mut self, ctx: &mut DMCtx, stage: Option<&EFCollisionStage>, _dt: f32) {
        if stage.is_some() {
            self.EdgeDown(ctx, &[]);
        }
    }
}

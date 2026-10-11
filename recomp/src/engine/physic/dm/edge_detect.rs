//! 0x19 EFDMEdgeDetect (Classes/EFDM/EFDMEdgeDetect.cpp, 0x1b4 bytes), 1:1: collects up to 10
//! polygon edges (flagged in the polygon's +4 edge mask) passing within `radius` of a point of the
//! owner prim, and notifies the owner (EFNotificationMgr code 0x18) when the edge count changed.
//!
//! Seams: the trait's `EFCollisionStage` (module.rs) carries no prim lists, so the body
//! `EdgeDetect` takes the stage's touched env prims (stage+4/+8, `objcoll_check::EnvPrim`) and the
//! collision models of the stage's collidable dyn physics (stage+0xc/+0x10: `**entry` physic
//! +0x54 model). The trait PostColUpdate runs the body with empty lists. The
//! RegisterNotify/Notify(0x18) calls are recorded in `registered` / `last` (coordinator forwards).
//! DrawEdges is an empty function in the binary.
#![allow(non_snake_case)]

use crate::engine::physic::model::{EFCollisionModel, PrimExt};
use crate::engine::physic::module::*;
use crate::engine::physic::objcoll_check::EnvPrim;
use crate::engine::physic::objcoll_prim::{transform_point, transform_vector, EFObjCollisionPrim, PRIM_POLYGON};
use crate::engine::physic::rwm::*;

/// Max edges (this+0x48 .. this+0x1b0, 10 entries of 0x24 bytes).
pub const EDGE_DETECT_MAX: usize = 10;

/// One detected edge (0x24 bytes at this+0x48 + i*0x24).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct DetectedEdge {
    /// +0x00 first vertex
    pub v0: V3,
    /// +0x0c second vertex
    pub v1: V3,
    /// +0x18 polygon normal (prim +0xc)
    pub normal: V3,
}

/// Notify(0x18, owner, {tag (+0x38), count (+0x44), &edges (+0x48)}) payload.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EdgeDetectNotify {
    pub target: u32,
    pub tag: u32,
    pub count: u32,
    pub edges: Vec<DetectedEdge>,
}

pub struct EFDMEdgeDetect {
    pub base: EFPhysicModule,
    /// +0x30 detection radius (0.5; prim radius +0xbc when flags&1; <0 -> 0.1)
    pub radius: f32,
    /// +0x34 prim index
    pub prim: u32,
    /// +0x38 prim tag
    pub tag: u32,
    /// +0x3c user value (RegisterNotify payload {game object, user})
    pub user: u32,
    /// +0x40 flags: 1 radius from prim, 2 draw, 4 env prims, 8 object polygon prims (default 4)
    pub flags: u32,
    /// +0x44 edge count
    pub count: u32,
    /// +0x48 edges
    pub edges: [DetectedEdge; EDGE_DETECT_MAX],
    /// +0x1b0 owner uid (physic +0xac +8)
    pub target: u32,
    /// RegisterNotify(0x18, owner, owner, cb 0x1fd300, {GetGameObject(owner), user}) issued
    pub registered: bool,
    /// last Notify(0x18) payload
    pub last: Option<EdgeDetectNotify>,
}

/// `(bits & 0x7f800000) < 0x3c800000`: exponent test |x| < 2^-6 (quirk: not a float compare)
#[inline]
fn tiny(x: f32) -> bool { (x.to_bits() & 0x7f80_0000) < 0x3c80_0000 }

impl EFDMEdgeDetect {
    pub fn new() -> Self {
        EFDMEdgeDetect {
            base: EFPhysicModule { type_: 0x4000_0019, flags: 1, ..Default::default() },
            radius: 0.5,
            prim: 0,
            tag: 0,
            user: 0,
            flags: 4,
            count: 0,
            edges: [DetectedEdge::default(); EDGE_DETECT_MAX],
            target: 0,
            registered: false,
            last: None,
        }
    }

    /// PostColUpdate body (stage != NULL). `env` = stage+4/+8, `others` = models of stage+0xc/+0x10.
    pub fn EdgeDetect(&mut self, ctx: &mut DMCtx, env: &[EnvPrim], others: &[&EFCollisionModel]) {
        // quirk: prim+0xc0 read through a possibly NULL prim pointer; here a missing prim aborts
        let Some(Some(pr)) = ctx.phys.model.prims.get(self.prim as usize) else { return };
        let prim = pr.base.clone();
        let c = prim.center;
        let ltm = &ctx.phys.frame;
        let pos = ltm.pos;
        let at = ltm.at;
        let d = sub(pos, c);
        let t = (d[2] * at[2] + d[0] * at[0] + d[1] * at[1]) / (at[2] * at[2] + at[0] * at[0] + at[1] * at[1]);
        let p = add(c, scale(at, t));
        if self.FindEdges(env, others, &prim, p, c) != 0 {
            // Notify(0x18, owner, {tag, count, &edges}): code 0x18 {tag, count, 0} followed by
            // 3 x count code-0x1018 words (v0, v1, normal per edge); the runtime rebuilds the payload
            ctx.notify.push(PhysicNotify { owner: self.target, code: 0x18, data: [self.tag, self.count, 0, 0] });
            for e in &self.edges[..self.count as usize] {
                for v in [e.v0, e.v1, e.normal] {
                    ctx.notify.push(PhysicNotify { owner: self.target, code: 0x1018, data: { let w = v.map(f32::to_bits); [w[0], w[1], w[2], 0] } });
                }
            }
            self.last = Some(EdgeDetectNotify {
                target: self.target,
                tag: self.tag,
                count: self.count,
                edges: self.edges.to_vec(),
            });
        }
        if self.flags & 2 != 0 {
            Self::DrawEdges();
        }
    }

    /// DrawEdges(): empty in the binary.
    pub fn DrawEdges() {}

    /// FindEdges(stage, prim, p, center). quirk: `_p` (the projected point) is unused; the prim
    /// center is the test point.
    pub fn FindEdges(&mut self, env: &[EnvPrim], others: &[&EFCollisionModel], prim: &EFObjCollisionPrim, _p: V3, c: V3) -> u32 {
        let mut found = 0u32;
        let old = self.count;
        self.count = 0;
        if self.flags & 4 != 0 {
            let (m1, m2) = (prim.word(0x20), prim.word(0x24));
            for e in env {
                if (e.mat[1] & m1 != 0 || e.mat[2] & m2 != 0) && e.w4 != 0 {
                    self.FindEdgeOnPolygon(e, c, &mut found, None);
                }
            }
        }
        if self.flags & 8 != 0 && prim.word(0x14) & 0x200 != 0 {
            for m in others {
                for op in m.prims.iter() {
                    // quirk: a NULL prim array/slot would be dereferenced; empty slots skipped
                    let Some(op) = op else { continue };
                    if op.base.kind != PRIM_POLYGON {
                        continue;
                    }
                    if op.base.word(0x10) & 0x200 != 0 {
                        if let PrimExt::Polygon { m: mtx, inv, prims } = &op.ext {
                            let lp = transform_point(inv, c);
                            // +0xe8 count; walk stops at the count (prims.len() == count)
                            if prims.first().is_some_and(|f| f.w4 != 0) {
                                // quirk: only the FIRST polygon's edge mask gates the whole list
                                for pp in prims {
                                    self.FindEdgeOnPolygon(pp, lp, &mut found, Some(mtx));
                                }
                            }
                        }
                    }
                }
            }
        }
        if self.count != old {
            found = 1;
        }
        found
    }

    /// FindEdgeOnPolygon(prim, p, &found, matrix)
    pub fn FindEdgeOnPolygon(&mut self, e: &EnvPrim, p: V3, found: &mut u32, m: Option<&[[f32; 4]; 4]>) {
        let n = e.nverts as usize;
        let mask = e.w4 as u32;
        let r2 = self.radius * self.radius;
        let mut j = 1usize;
        let mut i = 0usize;
        while i < n {
            if mask & (1u32 << (i & 0x1f)) != 0 {
                if j == n {
                    j = 0;
                }
                // quirk: vertex stride uses the raw u16 count; EnvPrim holds `& 0x7fff` vertices
                let (Some(&a), Some(&b)) = (e.verts.get(i), e.verts.get(j)) else { return };
                let s = sub(b, a);
                let w = sub(p, a);
                let t = (w[2] * s[2] + w[0] * s[0] + w[1] * s[1]) / (s[2] * s[2] + s[0] * s[0] + s[1] * s[1]);
                let q = add(scale(s, t), a);
                let da = sub(p, a);
                let db = sub(p, b);
                let dq = sub(p, q);
                let l = |v: V3| v[2] * v[2] + v[0] * v[0] + v[1] * v[1];
                if l(da) <= r2 || l(db) <= r2 || (l(dq) <= r2 && 0.0 <= t && t <= 1.0) {
                    let k = self.count as usize;
                    if 9 < k {
                        return;
                    }
                    let cur = self.edges[k];
                    let same = tiny(a[0] - cur.v0[0])
                        && tiny(a[1] - cur.v0[1])
                        && tiny(a[2] - cur.v0[2])
                        && tiny(b[0] - cur.v1[0])
                        && tiny(b[1] - cur.v1[1])
                        && tiny(b[2] - cur.v1[2]);
                    // quirk: compared against the stored slot in local space (before the
                    // matrix transform), so object edges are rewritten every frame
                    if !same {
                        let ed = &mut self.edges[k];
                        ed.v0 = a;
                        ed.v1 = b;
                        ed.normal = e.normal;
                        *found = 1;
                        if let Some(m) = m {
                            ed.v0 = transform_point(m, ed.v0);
                            ed.v1 = transform_point(m, ed.v1);
                            ed.normal = transform_vector(m, ed.normal);
                        }
                    }
                    self.count += 1;
                }
            }
            i += 1;
            j += 1;
        }
    }
}

impl EFDM for EFDMEdgeDetect {
    fn base(&self) -> &EFPhysicModule { &self.base }
    fn base_mut(&mut self) -> &mut EFPhysicModule { &mut self.base }
    fn ParamWords(&self) -> usize { 4 }
    fn SetParams(&mut self, ctx: &mut DMCtx, p: Option<&[u32]>) {
        let Some(p) = p else { return };
        self.tag = p[1];
        self.prim = ctx.phys.model.PrimTagToPrimIndex(p[1]);
        self.flags = p[3];
        if self.flags & 1 == 0 {
            self.radius = wf(p, 2);
        } else {
            // quirk: NULL prim deref in the binary; missing prim reads 0 here
            self.radius = ctx.phys.model.prims.get(self.prim as usize).and_then(|q| q.as_ref()).map_or(0.0, |q| q.base.radius);
        }
        if self.radius < 0.0 {
            self.radius = 0.1;
        }
        self.user = p[0];
        self.target = ctx.phys.owner;
        if !self.registered {
            // RegisterNotify(0x18, owner, owner, ExecLogicCallback, {GetGameObject(owner), user})
            ctx.notify.push(PhysicNotify { owner: self.target, code: 0x118, data: [self.user, 0, 0, 0] });
        }
        self.registered = true;
    }
    fn GetParams(&self, _ctx: &DMCtx, p: &mut [u32]) {
        p[2] = fw(self.radius);
        p[1] = self.tag;
        p[0] = self.user;
        p[3] = self.flags;
    }
    fn PostColUpdate(&mut self, ctx: &mut DMCtx, stage: Option<&EFCollisionStage>, _dt: f32) {
        if stage.is_some() {
            self.EdgeDetect(ctx, stage.unwrap().env, &[]);
        }
    }
}

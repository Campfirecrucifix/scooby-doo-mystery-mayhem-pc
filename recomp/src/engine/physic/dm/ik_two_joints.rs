//! 0x20 EFDMIKTwoJoints (Classes/EFDM/EFDMIKTwoJoints.cpp, 0x110 bytes, type 0x40000020), 1:1:
//! up to four collision prims, each attached to a bone, are pushed out of the stage's contact
//! polygons; the bone chain (frame, parent, grand-parent) is bent with the two-joint IK solver
//! EF2D2JointsIK (Classes/EF/EF2D2JointsIK.cpp, ported here as `EF2D2JointsIK`) so the prim's
//! frame reaches the pushed-out position. +0x44 mode bits: 1 also align the frame's up axis to the
//! contact normal (matrix Solve), 2 only solve while penetrating (dist² < radius²) or facing away.
//! Seams: the prim frame (+0xdc) = `EFObjCollisionPrim::joint` passed to `GetBone` (0xffffffff = root frame: host must map it), frame parents (+4) via
//! `GetBoneParent`, frames via `GetBone`/`GetBoneParentLTM`, written back with `SetBoneModelling`
//! (host recomputes LTMs = RwFrameUpdateObjects); the stage contact polygons via
//! `StageContactPolys` + `StageContactPolyMaterials`. RwMatrixRotate/Invert/Multiply,
//! RwV3dNormalize/Length/TransformVectors -> rwm.rs. The prim vtbl+0xc update call is the host's
//! job (prim center +0xc0 is read as is); the prim GetRadius (vtbl+0x28) is the seam
//! `radius` (+0xbc).
#![allow(non_snake_case)]

use crate::engine::physic::module::*;
use crate::engine::physic::rwm::*;

const PI: f32 = 3.1415927;

/// DTQuaternion::SetFromAxes(a, b) ([x, y, z, w])
fn SetFromAxes(a: V3, b: V3) -> [f32; 4] {
    let d = dot(a, b);
    if 0.99999 < d {
        [0.0, 0.0, 0.0, 1.0]
    } else if d < -0.99999 {
        let (mut f4, mut f5, mut f2) = (a[0], 0.0f32, -a[1]);
        let mut f3 = 0.0;
        if ((f2 * f2 + f4 * f4).sqrt() as f64) < 1e-6 {
            f2 = a[0];
            f4 = 0.0;
            f5 = -a[2];
            f3 = f5 * f5;
        }
        let s = 1.0 / (f2 * f2 + f3 + f4 * f4).sqrt();
        [f5 * s, f4 * s, f2 * s, 0.0]
    } else {
        let h = ((1.0 - d) * 0.5).sqrt();
        let c = cross(a, b);
        let s = 1.0 / (c[2] * c[2] + c[0] * c[0] + c[1] * c[1]).sqrt();
        [c[0] * s * h, c[1] * s * h, c[2] * s * h, ((d + 1.0) * 0.5).sqrt()]
    }
}

/// DTQuaternion::GetValue(axis, angle)
fn GetValue(q: [f32; 4]) -> (V3, f32) {
    let l = q[2] * q[2] + q[0] * q[0] + q[1] * q[1];
    let (axis, mut ang) = if !((l as f64) > 1e-6) {
        ([0.0, 0.0, 1.0], 0.0)
    } else {
        // quirk: divides by the squared length (not its root); the caller re-normalises
        let s = 1.0 / l;
        ([q[0] * s, q[1] * s, q[2] * s], q[3].acos() * 2.0)
    };
    if PI < ang {
        ang = PI;
    }
    (axis, ang)
}

fn AcosClamp(c: f32) -> f32 {
    if c <= -1.0 { PI } else if 1.0 <= c { 0.0 } else { c.acos() }
}

/// RwV3dNormalize + "if (len == 0) z = 1"
fn NormZ(v: V3) -> V3 {
    let mut n = RwV3dNormalize(v);
    if RwV3dLength(v) == 0.0 {
        n[2] = 1.0;
    }
    n
}

/// RwMatrixRotate(m, axis, deg, rwCOMBINEPOSTCONCAT) with the position saved/restored.
fn RotateKeepPos(m: &mut RwMatrix, deg: f32, axis: V3) {
    let pos = m.pos;
    m.Rotate(deg, axis, rwCOMBINEPOSTCONCAT);
    m.pos = pos;
}

/// EF2D2JointsIK (vtable only, no data). Frames are GetBone bone ids of `owner`.
pub struct EF2D2JointsIK;

impl EF2D2JointsIK {
    /// EF2D2JointsIK::Solve(DTFrame &, DTVector3d const &): bends parent (p1) and grand-parent (p2)
    /// so `frame` reaches `target`. Missing frames (C++ NULL deref) abort.
    pub fn Solve(&self, ctx: &DMCtx, frame: u32, target: V3) -> Option<()> {
        let w = ctx.world;
        let o = ctx.phys.owner;
        let p1 = w.GetBoneParent(o, frame)?;
        let p2 = w.GetBoneParent(o, p1)?;
        let ltm2 = w.GetBone(o, p2)?.1;
        let ltm3 = w.GetBoneParentLTM(o, p2)?; // RwFrameGetLTM(p2 parent)
        let P2 = ltm2.pos;
        let P1 = w.GetBone(o, p1)?.1.pos;
        let P0 = w.GetBone(o, frame)?.1.pos;
        let u0 = sub(P1, P2);
        let l1 = RwV3dLength(u0);
        let u = RwV3dNormalize(u0);
        let v0 = sub(P0, P1);
        let l2 = RwV3dLength(v0);
        let v = RwV3dNormalize(v0);
        let mut wv = sub(P0, P2);
        let lw = RwV3dLength(wv);
        let mut t = sub(target, P2);
        let mut lt = RwV3dLength(t);
        if l1 + l2 < lt {
            let r = (l1 + l2) - 0.0001;
            t = scale(t, r / lt);
            lt = r;
        }
        let lw2 = lw * lw;
        let a13 = AcosClamp(((lw2 - l1 * l1) - l2 * l2) / (l1 * 2.0 * l2));
        let mut a10 = AcosClamp(((l2 * l2 - lw2) - l1 * l1) / (l1 * 2.0 * lw));
        let c = ((lt * lt - l1 * l1) - l2 * l2) / (l1 * 2.0 * l2);
        let (a14, a11) = if c <= -1.0 || 1.0 <= c {
            (0.0, 0.0)
        } else {
            // quirk: the inner clamp re-tests the already excluded bounds
            (AcosClamp(c), AcosClamp(((l2 * l2 - lt * lt) - l1 * l1) / (l1 * 2.0 * lt)))
        };
        a10 -= PI;
        let n = cross(u, v);
        let inv3 = ltm3.Invert(); // uRam003439d0 scratch keeps this for the 3rd rotation
        let axisE = NormZ(ltm2.Invert().TransformVector(n));
        let axisD = NormZ(inv3.TransformVector(n));
        let mut m1 = w.GetBone(o, p1)?.0;
        RotateKeepPos(&mut m1, (a14 - a13) * RAD2DEG, axisE);
        w.SetBoneModelling(o, p1, m1);
        let mut m2 = w.GetBone(o, p2)?.0;
        RotateKeepPos(&mut m2, ((a11 - PI) - a10) * RAD2DEG, axisD);
        wv = RwV3dNormalize(wv);
        t = RwV3dNormalize(t);
        let (ax, ang) = GetValue(SetFromAxes(wv, t));
        let axisD = NormZ(inv3.TransformVector(ax));
        RotateKeepPos(&mut m2, ang * RAD2DEG, axisD);
        w.SetBoneModelling(o, p2, m2);
        Some(())
    }

    /// EF2D2JointsIK::Solve(DTFrame &, DTMatrix &): position solve, then frame modelling =
    /// m * inverse(parent LTM).
    pub fn SolveMatrix(&self, ctx: &DMCtx, frame: u32, m: &RwMatrix) -> Option<()> {
        self.Solve(ctx, frame, m.pos)?;
        let pl = ctx.world.GetBoneParentLTM(ctx.phys.owner, frame)?;
        ctx.world.SetBoneModelling(ctx.phys.owner, frame, RwMatrix::Multiply(m, &pl.Invert()));
        Some(())
    }
}

pub struct EFDMIKTwoJoints {
    pub base: EFPhysicModule,
    /// +0x30 prim count
    pub count: u32,
    /// +0x34..+0x40 prim indices (-1 = none)
    pub prims: [u32; 4],
    /// +0x44 mode bits (1 align to normal, 2 only when penetrating)
    pub mode: u32,
    /// +0x48 prim mask (vs prim +0x18; 0 = prims with flag 0x100)
    pub prim_mask: u32,
    /// +0x4c material mask (vs material +8; 0 = materials with flag 0x800)
    pub mat_mask: u32,
    /// +0x50..+0x60 prim pointers this frame (Some = prim index)
    pub prim_ptr: [Option<u32>; 4],
    /// +0x60..+0x70 prim frames (bone ids)
    pub frames: [Option<u32>; 4],
    /// +0x70 frame LTM positions
    pub frame_pos: [V3; 4],
    /// +0xa0 push-out offsets
    pub offset: [V3; 4],
    /// +0xd0 contact normals
    pub normal: [V3; 4],
    /// +0x100 best squared distances
    pub best: [f32; 4],
}

impl Default for EFDMIKTwoJoints {
    fn default() -> Self { Self::new() }
}

impl EFDMIKTwoJoints {
    pub fn new() -> Self {
        EFDMIKTwoJoints {
            base: EFPhysicModule { type_: 0x4000_0020, flags: 1, ..Default::default() },
            count: 0,
            prims: [0xffff_ffff; 4],
            mode: 0,
            prim_mask: 0,
            mat_mask: 0,
            prim_ptr: [None; 4],
            frames: [None; 4],
            frame_pos: [[0.0; 3]; 4],
            offset: [[0.0; 3]; 4],
            normal: [[0.0; 3]; 4],
            best: [0.0; 4],
        }
    }

    /// quirk: +0x30 is not bounded in the C++ (count > 4 overruns the arrays); clamped here.
    fn n(&self) -> usize { (self.count as usize).min(4) }
}

impl EFDM for EFDMIKTwoJoints {
    fn base(&self) -> &EFPhysicModule { &self.base }
    fn base_mut(&mut self) -> &mut EFPhysicModule { &mut self.base }
    fn ParamWords(&self) -> usize { 8 }

    fn SetParams(&mut self, ctx: &mut DMCtx, p: Option<&[u32]>) {
        match p {
            None => {
                self.count = 0;
                self.mode = 0;
                self.prims = [0xffff_ffff; 4];
                self.prim_mask = 0;
                self.mat_mask = 0;
            }
            Some(p) => {
                self.count = p[0];
                self.mode = p[5];
                for i in 0..self.n() {
                    let t = p[1 + i];
                    self.prims[i] = if t == 0xffff_ffff { 0xffff_ffff } else { ctx.phys.model.PrimTagToPrimIndex(t) };
                }
                self.prim_mask = p[6];
                self.mat_mask = p[7];
            }
        }
    }

    fn GetParams(&self, ctx: &DMCtx, p: &mut [u32]) {
        p[0] = self.count;
        for i in 0..self.n() {
            p[1 + i] = if self.prims[i] == 0xffff_ffff {
                0xffff_ffff
            } else {
                // NULL prim table -> *(0 + 8) in the C++; 0 here
                ctx.phys.model.prims.get(self.prims[i] as usize).and_then(|q| q.as_ref()).map_or(0, |q| q.base.tag)
            };
        }
        p[5] = self.mode;
        p[6] = self.prim_mask;
        p[7] = self.mat_mask;
    }

    fn PostColUpdate(&mut self, ctx: &mut DMCtx, _stage: Option<&EFCollisionStage>, _dt: f32) {
        let owner = ctx.phys.owner;
        let mut any = false;
        for i in 0..self.n() {
            self.prim_ptr[i] = None;
            self.frames[i] = None;
            self.frame_pos[i] = [0.0; 3];
            self.offset[i] = [0.0; 3];
            self.normal[i] = [0.0; 3];
            self.best[i] = 10000.0; // 0x461c4000
            if self.prims[i] != 0xffff_ffff {
                let pi = self.prims[i];
                let Some(prim) = ctx.phys.model.prims.get(pi as usize).and_then(|q| q.as_ref()).map(|q| &q.base) else { continue };
                self.prim_ptr[i] = Some(pi);
                if (self.prim_mask & prim.word(0x18)) != 0 || ((prim.word(0xc) & 0x100) != 0 && self.prim_mask == 0) {
                    self.frames[i] = Some(prim.joint);
                    // RwFrameGetLTM(NULL) would crash; a missing frame keeps pos 0
                    if let Some((_, ltm)) = self.frames[i].and_then(|f| ctx.world.GetBone(owner, f)) {
                        self.frame_pos[i] = ltm.pos;
                    }
                    any = true;
                }
            }
        }
        if !any {
            return;
        }
        let polys = ctx.world.StageContactPolys(owner);
        if polys.is_empty() {
            return;
        }
        let mats = ctx.world.StageContactPolyMaterials(owner);
        for (k, (pn, pd, verts, edges)) in polys.iter().enumerate() {
            let (mflags, mmask) = mats.get(k).copied().unwrap_or((0, 0));
            if !(((mflags & 0x800) != 0 && self.mat_mask == 0) || (mmask & self.mat_mask) != 0) {
                continue;
            }
            for i in 0..self.n() {
                // quirk: a NULL prim (-1 index) is dereferenced (+0xc0) in the C++; skipped here
                let Some(prim) = self.prim_ptr[i].and_then(|pi| ctx.phys.model.prims.get(pi as usize)).and_then(|q| q.as_ref()).map(|q| &q.base) else { continue };
                let c = prim.center;
                let mut maxd = 0.0f32;
                let mut vi = 0usize;
                for (e, pl) in edges.iter().enumerate() {
                    let d = pl[3] + c[2] * pl[2] + c[0] * pl[0] + c[1] * pl[1];
                    if d > maxd {
                        maxd = d;
                        vi = e;
                    }
                }
                let hit;
                if maxd == 0.0 {
                    // inside every edge: push along the plane normal
                    let d = pd + pn[2] * c[2] + pn[0] * c[0] + pn[1] * c[1];
                    if self.best[i] < d * d {
                        hit = false;
                    } else {
                        self.best[i] = d * d;
                        self.offset[i] = scale(*pn, -d);
                        hit = true;
                    }
                } else {
                    // closest point on edge vi -> vi+1
                    let mut ni = vi + 1;
                    if ni == verts.len() {
                        ni = 0;
                    }
                    let (Some(&va), Some(&vb)) = (verts.get(vi), verts.get(ni)) else { continue };
                    let ed = sub(vb, va);
                    let e = sub(va, c);
                    let mut b = e;
                    let de = ed[2] * e[2] + ed[0] * e[0] + ed[1] * e[1];
                    let dd = ed[2] * ed[2] + ed[0] * ed[0] + ed[1] * ed[1];
                    if 0.0 < de {
                    } else if de < -dd {
                        b = add(e, ed);
                    } else {
                        b = sub(b, scale(ed, de / dd));
                    }
                    let l = b[2] * b[2] + b[0] * b[0] + b[1] * b[1];
                    if self.best[i] < l {
                        hit = false;
                    } else {
                        self.best[i] = l;
                        self.offset[i] = b;
                        hit = true;
                    }
                }
                if hit {
                    if self.best[i] < 0.0 {
                        self.best[i] *= -1.0;
                    }
                    self.normal[i] = *pn;
                }
            }
        }
        let ik = EF2D2JointsIK; // EF2D2JointsIK::EF2D2JointsIK; RwMatrixCreate
        for i in 0..self.n() {
            let Some(prim) = self.prim_ptr[i].and_then(|pi| ctx.phys.model.prims.get(pi as usize)).and_then(|q| q.as_ref()).map(|q| &q.base) else { continue };
            let Some(frame) = self.frames[i] else { continue };
            let nn = self.normal[i];
            if (prim.word(0xc) & 0x10) != 0 || !((self.mode & 3) == 0 || nn[2] * nn[2] + nn[0] * nn[0] + nn[1] * nn[1] != 0.0) {
                continue;
            }
            let o = self.offset[i];
            if o[2] * o[2] + o[0] * o[0] + o[1] * o[1] == 0.0 {
                continue;
            }
            let dir = RwV3dNormalize(o);
            if (self.mode & 2) != 0 {
                let r = prim.radius; // vtbl+0x28 GetRadius, called twice
                if r * r < self.best[i] && nn[2] * dir[2] + nn[0] * dir[0] + nn[1] * dir[1] <= 0.0 {
                    continue;
                }
            }
            let Some((_, ltm)) = ctx.world.GetBone(owner, frame) else { continue };
            let mut m = ltm;
            let c = prim.center;
            let delta = sub(c, self.frame_pos[i]);
            m.pos = sub(add(c, o), delta);
            if (self.mode & 1) == 0 {
                ik.Solve(ctx, frame, m.pos);
            } else {
                m.up = RwV3dNormalize(nn);
                m.at = sub(m.at, scale(m.up, dot(m.at, m.up)));
                m.at = RwV3dNormalize(m.at);
                m.right = cross(m.up, m.at);
                ik.SolveMatrix(ctx, frame, &m);
            }
        }
    }
}

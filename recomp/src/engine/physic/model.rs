//! EFCollisionModel (Classes/EF/EFCollisionModel.cpp): the set of object collision primitives of one
//! EFPhysicData, plus the concrete primitive data of EFObjCollisionSphere / SweptSphere / BBox /
//! Polygon that AddPrimitive constructs.
//!
//! Seams (owned by WP-E2's EFPhysicData): the physic data's frame LTM (`EFPhysicData+0x14` frame) is
//! passed in as a matrix; FlagTranslatedChild's `EFPhysicData+0xc |= 0x20` over the child tree is
//! reported through `translated` and applied by the owner; `_gpoPhysicMgr+0x2024` (frame counter) is
//! a parameter.
#![allow(non_snake_case)]

use super::objcoll_prim::*;

/// Type-specific part of a primitive (virtual dispatch on EFObjCollisionPrim::kind).
#[derive(Debug, Clone)]
pub enum PrimExt {
    /// EFObjCollisionSphere (0xe0 bytes): radius = base.radius (+0xbc).
    Sphere,
    /// EFObjCollisionSweptSphere (0xfc): +0xe0 world vector, +0xec sphere radius, +0xf0 local vector.
    SweptSphere { world_vec: V3, sradius: f32, vec: V3 },
    /// EFObjCollisionBBox (0xf8): +0xe0 half extents; +0xec/+0xf0/+0xf4 RwMatrix (world, inverse, tmp).
    BBox { half: V3, m: [[f32; 4]; 4], inv: [[f32; 4]; 4], tmp: [[f32; 4]; 4] },
    /// EFObjCollisionPolygon (0xf0): +0xe0/+0xe4 RwMatrix; +0xe8 count / +0xec packed
    /// EFEnvCollisionPrim array in polygon-local space (stride 0x1c + 0x1c*raw nverts; material
    /// never read on these paths). Filled by the owner (whoever builds the polygon prim).
    Polygon { m: [[f32; 4]; 4], inv: [[f32; 4]; 4], prims: Vec<super::objcoll_check::EnvPrim> },
}

#[derive(Debug, Clone)]
pub struct ObjPrim {
    pub base: EFObjCollisionPrim,
    pub ext: PrimExt,
}

impl ObjPrim {
    /// vtable+0xc PrecalcPrimData
    pub fn PrecalcPrimData(&mut self) {
        // every override starts with the base center update (LTM.pos + LTM * offset)
        self.base.PrecalcPrimData();
        let ltm = self.base.ltm;
        let c = self.base.center;
        match &mut self.ext {
            PrimExt::Sphere => {}
            PrimExt::SweptSphere { world_vec, vec, .. } => {
                *world_vec = transform_vector(&ltm, *vec);
            }
            PrimExt::BBox { half, m, inv, tmp } => {
                // EFObjCollisionBBox::PrecalcPrimData (0x001cda70): m = LTM with pos = center,
                // inv = RwMatrixInvert(m), tmp = transpose3x3(LTM) (pos kept) then RwMatrixScale(half, POSTCONCAT)
                *m = ltm;
                m[3][0] = c[0];
                m[3][1] = c[1];
                m[3][2] = c[2];
                *inv = RwMatrixInvert(m);
                *tmp = ltm;
                for (a, b) in [(0usize, 2usize), (0, 1), (1, 2)] {
                    let t = tmp[a][b];
                    tmp[a][b] = tmp[b][a];
                    tmp[b][a] = t;
                }
                for r in tmp.iter_mut() {
                    r[0] *= half[0];
                    r[1] *= half[1];
                    r[2] *= half[2];
                }
            }
            PrimExt::Polygon { m, inv, .. } => {
                // EFObjCollisionPolygon::PrecalcPrimData (0x00270fd0): m = LTM with pos = center, inv = RwMatrixInvert(m)
                *m = ltm;
                m[3][0] = c[0];
                m[3][1] = c[1];
                m[3][2] = c[2];
                *inv = RwMatrixInvert(m);
            }
        }
    }
    /// vtable+0x10 Translate. BBox/Polygon override (0x001cdcb0 / 0x00271220): center += v,
    /// m.pos = center, inv.pos = -(inv * center) (vector transform).
    pub fn Translate(&mut self, v: V3) {
        self.base.Translate(v);
        let c = self.base.center;
        match &mut self.ext {
            PrimExt::BBox { m, inv, .. } | PrimExt::Polygon { m, inv, .. } => {
                m[3][0] = c[0];
                m[3][1] = c[1];
                m[3][2] = c[2];
                let p = transform_vector(inv, c);
                inv[3][0] = -p[0];
                inv[3][1] = -p[1];
                inv[3][2] = -p[2];
            }
            _ => {}
        }
    }
    /// vtable+0x28 GetMinRadius: sphere +0xbc, swept +0xec, bbox min(half) (0x001cdda0),
    /// polygon constant 1.0 (0x3f800000).
    pub fn GetMinRadius(&self) -> f32 {
        match &self.ext {
            PrimExt::Sphere => self.base.radius,
            PrimExt::SweptSphere { sradius, .. } => *sradius,
            PrimExt::BBox { half, .. } => {
                let (x, y, z) = (half[0], half[1], half[2]);
                if x < y {
                    if z <= x { z } else { x }
                } else if z <= y {
                    z
                } else {
                    y
                }
            }
            PrimExt::Polygon { .. } => 1.0,
        }
    }
    /// EFObjCollisionSweptSphere::SetVector
    pub fn SetVector(&mut self, v: V3) {
        if let PrimExt::SweptSphere { vec, sradius, .. } = &mut self.ext {
            *vec = v;
            self.base.radius = *sradius + length(*vec);
            let l2 = vec[2] * vec[2] + vec[0] * vec[0] + vec[1] * vec[1];
            if l2 < 0.0001 {
                if l2 == 0.0 {
                    *vec = [0.0, 0.01, 0.0];
                } else {
                    // quirk: scales by |v|*0.01 (not 0.01/|v|)
                    let s = l2.sqrt() * 0.01;
                    *vec = scale(*vec, s);
                }
                self.base.radius = *sradius + 0.01;
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct EFCollisionModel {
    /// this+0x4 primitive slots, this+0x8 max prims
    pub prims: Vec<Option<ObjPrim>>,
    /// this+0xc physic bounding-sphere center, frame-local (EF3dObj::CreateCollisionPrimitives:
    /// ApClumpGetBoundingSphere; EFPhysic+0x60)
    pub bsphere_center: V3,
    /// this+0x18 physic bounding-sphere radius (EFPhysic+0x6c)
    pub bsphere_radius: f32,
    /// this+0x1c world bounding-sphere center (ComputePhysicBSphere output, moved by Translate;
    /// EFPhysic+0x70)
    pub bsphere_world: V3,
    /// this+0x28 frame stamp of the primary prim position
    pub stamp: u32,
    /// this+0x2c primary primitive index (-1 none)
    pub primary: i32,
    /// this+0x30 primary prim position
    pub primary_pos: V3,
    /// this+0x3c
    pub b3c: u8,
    /// this+0x40..+0x54 ORed prim words (prim+0x10, +0x18, +0x14, +0x1c, +0x20, +0x24)
    pub masks: [u32; 6],
    /// FlagTranslatedChild was called (seam: the owner sets EFPhysicData+0xc |= 0x20 on the subtree)
    pub translated: bool,
}

impl EFCollisionModel {
    /// EFCollisionModel(EFPhysicData*, uint maxPrims)
    pub fn new(max: u32) -> Self {
        EFCollisionModel {
            prims: vec![None; max as usize],
            bsphere_center: [0.0; 3],
            bsphere_radius: 0.0, // quirk: +0x18 is not initialised by the ctor (heap garbage)
            bsphere_world: [0.0; 3],
            stamp: 0,
            primary: -1,
            primary_pos: [0.0; 3],
            b3c: 0,
            masks: [0; 6],
            translated: false,
        }
    }
    /// SetMaxPrims: drops every primitive and reallocates empty slots.
    pub fn SetMaxPrims(&mut self, n: u32) {
        self.prims = vec![None; n as usize];
    }
    /// AddPrimitive(index, tag, kind, frame LTM). Unknown kinds leave the slot as is (the C++ then
    /// writes the tag through the old slot pointer: NULL deref if empty -> here a no-op + None).
    pub fn AddPrimitive(&mut self, idx: u32, tag: u32, kind: u32, ltm: [[f32; 4]; 4]) -> Option<&mut ObjPrim> {
        let mut base = EFObjCollisionPrim::new(ltm);
        let ext = match kind {
            PRIM_SPHERE => PrimExt::Sphere,
            PRIM_SWEPT_SPHERE => PrimExt::SweptSphere { world_vec: [0.0; 3], sradius: 0.0, vec: [0.0; 3] },
            PRIM_BBOX => PrimExt::BBox { half: [0.0; 3], m: IDENTITY, inv: IDENTITY, tmp: IDENTITY },
            PRIM_POLYGON => PrimExt::Polygon { m: IDENTITY, inv: IDENTITY, prims: Vec::new() },
            _ => {
                let p = self.prims.get_mut(idx as usize)?.as_mut()?;
                p.base.tag = tag;
                return Some(p);
            }
        };
        base.kind = kind;
        let mut p = ObjPrim { base, ext };
        if kind == PRIM_SWEPT_SPHERE {
            p.SetVector([0.0, 0.1, 0.0]); // 0x3dcccccd
            if let PrimExt::SweptSphere { sradius, vec, .. } = &mut p.ext {
                *sradius = 0.0;
                p.base.radius = *sradius + length(*vec);
            }
        }
        // BBox: radius = |half| = 0
        p.base.tag = tag;
        let slot = self.prims.get_mut(idx as usize)?;
        *slot = Some(p);
        slot.as_mut()
    }
    /// PrimTagToPrimIndex: first slot with that tag, else 0xffffffff.
    pub fn PrimTagToPrimIndex(&self, tag: u32) -> u32 {
        self.prims.iter().position(|p| p.as_ref().is_some_and(|p| p.base.tag == tag)).map_or(0xffff_ffff, |i| i as u32)
    }
    /// Translate(v). Returns true when the frame must be translated (RwFrameTranslate(frame, v,
    /// POSTCONCAT)) — skipped when every component is < 2^-23 in exponent (quirk: exponent test).
    pub fn Translate(&mut self, v: V3) -> bool {
        let tiny = |x: f32| ((x - 0.0).to_bits() & 0x7f80_0000) < 0x3400_0000;
        let move_frame = !(tiny(v[0]) && tiny(v[1]) && tiny(v[2]));
        self.FlagTranslatedChild();
        self.bsphere_world = add(self.bsphere_world, v);
        for p in self.prims.iter_mut().flatten() {
            p.Translate(v);
        }
        move_frame
    }
    /// FlagTranslatedChild(EFPhysicData*): seam, see module doc.
    pub fn FlagTranslatedChild(&mut self) {
        self.translated = true;
    }
    /// PrecalcData(ltm of the physic data frame)
    pub fn PrecalcData(&mut self, ltm: &[[f32; 4]; 4]) {
        self.ComputePhysicBSphere(ltm);
        self.masks[..4].fill(0);
        for p in self.prims.iter_mut().flatten() {
            p.PrecalcPrimData();
            let w = |o| p.base.word(o);
            self.masks[0] |= w(0x10);
            self.masks[1] |= w(0x18);
            self.masks[2] |= w(0x14);
            self.masks[3] |= w(0x1c);
            self.masks[4] |= w(0x20);
            self.masks[5] |= w(0x24);
        }
    }
    /// ComputePhysicBSphere (0x00217f70): RwV3dTransformPoints(+0x1c, +0xc, 1, LTM).
    pub fn ComputePhysicBSphere(&mut self, ltm: &[[f32; 4]; 4]) {
        self.bsphere_world = transform_point(ltm, self.bsphere_center);
    }
    /// ValidatePrimaryPrimTrans(stage). `frame` = _gpoPhysicMgr+0x2024; `validate(pos, &mut delta,
    /// mask1, mask2) -> bool` = EFCollisionStage::ValidateTranslation (true = blocked, delta
    /// shortened). Returns the frame-translate request of the inner Translate.
    pub fn ValidatePrimaryPrimTrans(&mut self, frame: u32, mut validate: impl FnMut(V3, &mut V3, u32, u32) -> bool) -> bool {
        let mut moved = false;
        if self.primary == -1 {
            return false;
        }
        let pi = self.primary as usize;
        let Some(Some(_)) = self.prims.get(pi) else { return false };
        if self.stamp.wrapping_add(1) < frame {
            let p = self.prims[pi].as_mut().unwrap();
            p.PrecalcPrimData();
            self.primary_pos = p.base.center;
            self.stamp = frame;
            return false;
        }
        let p = self.prims[pi].as_mut().unwrap();
        p.PrecalcPrimData();
        let mut d = sub(p.base.center, self.primary_pos);
        let l2 = d[2] * d[2] + d[0] * d[0] + d[1] * d[1]; // (z*z+x*x)+y*y as in the C++
        // cut-off re-checked against 0x00217c70: `if (0.01 < len) (len-0.01)/len else 0` (unchanged)
        let r = p.GetMinRadius();
        if r * r * 0.25 <= l2 {
            let (m1, m2) = (p.base.word(0x20), p.base.word(0x24));
            if validate(self.primary_pos, &mut d, m1, m2) {
                let l = length(d);
                if 0.01 < l {
                    let s = (l - 0.01) / l;
                    d = scale(d, s);
                } else {
                    d = [0.0; 3];
                }
                let target = add(self.primary_pos, d);
                let c = self.prims[pi].as_ref().unwrap().base.center;
                moved = self.Translate(sub(target, c));
            }
            let p = self.prims[pi].as_mut().unwrap();
            p.PrecalcPrimData();
            self.primary_pos = p.base.center;
            self.stamp = frame;
        }
        moved
    }
}

/// RwMatrixInvert replacement: general affine inverse (rows right/up/at/pos, row-vector convention).
/// RW takes an orthonormal fast path when the matrix is flagged orthonormal; the result is the same.
pub fn RwMatrixInvert(m: &[[f32; 4]; 4]) -> [[f32; 4]; 4] {
    let a = |r: usize, c: usize| m[r][c];
    let det = a(0, 0) * (a(1, 1) * a(2, 2) - a(1, 2) * a(2, 1)) - a(0, 1) * (a(1, 0) * a(2, 2) - a(1, 2) * a(2, 0))
        + a(0, 2) * (a(1, 0) * a(2, 1) - a(1, 1) * a(2, 0));
    let id = if det == 0.0 { 0.0 } else { 1.0 / det };
    let mut r = IDENTITY;
    r[0][0] = (a(1, 1) * a(2, 2) - a(1, 2) * a(2, 1)) * id;
    r[0][1] = (a(0, 2) * a(2, 1) - a(0, 1) * a(2, 2)) * id;
    r[0][2] = (a(0, 1) * a(1, 2) - a(0, 2) * a(1, 1)) * id;
    r[1][0] = (a(1, 2) * a(2, 0) - a(1, 0) * a(2, 2)) * id;
    r[1][1] = (a(0, 0) * a(2, 2) - a(0, 2) * a(2, 0)) * id;
    r[1][2] = (a(0, 2) * a(1, 0) - a(0, 0) * a(1, 2)) * id;
    r[2][0] = (a(1, 0) * a(2, 1) - a(1, 1) * a(2, 0)) * id;
    r[2][1] = (a(0, 1) * a(2, 0) - a(0, 0) * a(2, 1)) * id;
    r[2][2] = (a(0, 0) * a(1, 1) - a(0, 1) * a(1, 0)) * id;
    let p = [m[3][0], m[3][1], m[3][2]];
    let q = transform_vector(&r, p);
    r[3] = [-q[0], -q[1], -q[2], 1.0];
    r
}

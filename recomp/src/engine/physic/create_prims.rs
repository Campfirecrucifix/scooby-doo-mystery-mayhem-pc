//! EF3dObj::CreateCollisionPrimitives (0x00223340), EF3dObj::GetDefaultPhysicBSphere (0x002237e0)
//! and EF3dObj::GetDefaultCollPrimAttrib(uint, EFObjCollisionPrim&) (0x00223800), 1:1.
//!
//! Seams: the EF3dObj is reduced to what these read: the clump's ApCollisionPrimitives plugin data
//! (`rw::clump::ApClumpCollision`, EF3dObj+0x30 clump), the EF3dObj+0x2c flags (bit 2 = HAnim
//! hierarchy) and the frames (EF3dObj+0x14 root frame / HAnim node frames `hier+0x10 + i*0x10 +
//! 0xc`), passed as LTMs. The prim keeps the node index in `EFObjCollisionPrim::joint` so the
//! collision system can resync its LTM every stage. The C++ side effect on the HAnim hierarchy
//! flags (`*hier |= 0x1000`) is returned to the caller.
#![allow(non_snake_case)]

use super::model::{EFCollisionModel, ObjPrim, PrimExt, RwMatrixInvert};
use super::objcoll_check::EnvPrim;
use super::objcoll_prim::*;
use crate::engine::rw::clump::{ApClumpCollision, ApCollisionPrimitive};

type M = [[f32; 4]; 4];

/// ApClumpGetBoundingSphere + GetDefaultPhysicBSphere: (center, radius); None = no sphere (the
/// C++ leaves its stack DTSphere {0,0,0, uninitialised radius} -> radius 0 here).
pub fn GetDefaultPhysicBSphere(col: Option<&ApClumpCollision>) -> ([f32; 3], f32) {
    match col.and_then(|c| c.bsphere) {
        Some(s) => ([s[0], s[1], s[2]], s[3]),
        None => ([0.0; 3], 0.0),
    }
}

/// ApClumpGetCollisionPolygonData(clump, i, &count) -> (count, prims parsed from polygon_data at
/// the record's +0x50 byte offset). The packed EFEnvCollisionPrim layout (stride 0x1c + 0x1c *
/// nverts) is decoded the same way as EFCollisionMap prims (EnvPrim::from_map); the material word
/// (+0, a pointer) is not resolved (never read on the polygon paths).
pub fn ApClumpGetCollisionPolygonData(col: &ApClumpCollision, i: usize) -> (u32, Vec<EnvPrim>) {
    let Some(p) = col.primitives.get(i) else { return (0xffff_ffff, Vec::new()) };
    let count = p.extra.first().copied().unwrap_or(0);
    let mut off = p.extra.get(1).copied().unwrap_or(0) as usize;
    let b = &col.polygon_data;
    let w = |o: usize| -> u32 { b.get(o..o + 4).map_or(0, |s| u32::from_le_bytes([s[0], s[1], s[2], s[3]])) };
    let fl = |o: usize| f32::from_bits(w(o));
    let mut out = Vec::new();
    for _ in 0..count {
        if off + 0x1c > b.len() {
            break;
        }
        let w1 = w(off + 4);
        let nverts = (w1 >> 16) as u16;
        let n = (nverts & 0x7fff) as usize;
        let v0 = off + 0x1c;
        let e0 = v0 + n * 0xc;
        out.push(EnvPrim {
            id: off / 4,
            mat: [w(off), 0, 0, 0, 0, 0, 0, 0],
            w4: (w1 & 0xffff) as u16,
            nverts,
            normal: [fl(off + 0xc), fl(off + 0x10), fl(off + 0x14)],
            d: fl(off + 0x18),
            verts: (0..n).map(|k| [fl(v0 + k * 0xc), fl(v0 + k * 0xc + 4), fl(v0 + k * 0xc + 8)]).collect(),
            edges: (0..n).map(|k| [fl(e0 + k * 0x10), fl(e0 + k * 0x10 + 4), fl(e0 + k * 0x10 + 8), fl(e0 + k * 0x10 + 12)]).collect(),
        });
        off += 0x1c + 0x1c * nverts as usize; // stride uses the raw u16 count
    }
    (count, out)
}

/// EF3dObj::GetDefaultCollPrimAttrib(index, prim). `rec` = ApClumpGetCollisionPrimitive(index);
/// `frame_ltm` = RwFrameGetLTM(prim+0xdc) (polygon matrices).
pub fn GetDefaultCollPrimAttrib(col: &ApClumpCollision, index: u32, prim: &mut ObjPrim, frame_ltm: M) {
    if prim.base.d8 & 1 != 0 {
        return;
    }
    let Some(rec) = col.primitives.get(index as usize) else { return }; // C++: NULL record deref
    let x = |i: usize| f32::from_bits(rec.extra.get(i).copied().unwrap_or(0));
    match (prim.base.kind, &mut prim.ext) {
        (3, PrimExt::Polygon { m, inv, prims }) => {
            let (_count, p) = ApClumpGetCollisionPolygonData(col, index as usize);
            *prims = p;
            *m = frame_ltm;
            *inv = RwMatrixInvert(m);
        }
        (2, PrimExt::BBox { half, .. }) => {
            *half = [x(0), x(1), x(2)];
            prim.base.radius = length(*half);
        }
        (1, PrimExt::SweptSphere { sradius, vec, .. }) => {
            *sradius = x(0);
            // quirk: the first radius uses the old vector, then it is recomputed
            prim.base.radius = *sradius + length(*vec);
            *vec = [x(1), x(2), x(3)];
            prim.base.radius = *sradius + length(*vec);
            let l2 = vec[2] * vec[2] + vec[0] * vec[0] + vec[1] * vec[1];
            if l2 < 0.0001 {
                if l2 == 0.0 {
                    *vec = [0.0, f32::from_bits(0x3c23_d70a), 0.0];
                } else {
                    // quirk: multiplies by |v|*0.01 (not a renormalisation to 0.01)
                    let s = l2.sqrt() * 0.01;
                    *vec = [vec[0] * s, vec[1] * s, vec[2] * s];
                }
                prim.base.radius = *sradius + 0.01;
            }
        }
        (0, _) => prim.base.radius = x(0),
        _ => {}
    }
    Attribs(rec, prim);
}

/// LAB_00223a1c tail: offset, center += offset, base int attribs and current = base & !lock |
/// value & lock, then +0xd8 |= 1.
fn Attribs(rec: &ApCollisionPrimitive, prim: &mut ObjPrim) {
    let b = &mut prim.base;
    b.offset = [rec.reals[0], rec.reals[1], rec.reals[2]];
    for k in 0..3 {
        b.center[k] += b.offset[k];
    }
    // record +0x20.. = attribs[i] -> prim base word +0x38 + 4i; current +0xc + 4i; value +0x64; lock +0x90
    // quirk: attrib 6 (+0x50 / record +0x38) is never copied (index 4 is done twice instead)
    for i in [0usize, 1, 2, 3, 4, 5, 4, 7, 8, 9, 10] {
        let base = rec.attribs[i] as u32;
        b.set_word(0x38 + 4 * i, base);
        let lock = b.word(0x90 + 4 * i);
        let val = b.word(0x64 + 4 * i);
        b.set_word(0xc + 4 * i, (base & !lock) | (val & lock));
    }
    b.d8 |= 1;
}

/// EF3dObj::CreateCollisionPrimitives(EFPhysic&) on the physic's collision model (EFPhysic+0x54,
/// bsphere at +0x60/+0x6c). `obj_flags` = EF3dObj+0x2c, `root` = LTM of EF3dObj+0x14,
/// `node_ltm(i)` = LTM of HAnim node i. Returns true when the C++ sets the HAnim hierarchy flag
/// 0x1000 (obj_flags & 4).
pub fn CreateCollisionPrimitives(col: Option<&ApClumpCollision>, obj_flags: u32, root: M, node_ltm: &dyn Fn(u32) -> M, model: &mut EFCollisionModel) -> bool {
    let hanim = obj_flags & 4 != 0;
    // ApClumpGetNbCollisionPrimitives (+4 count). Seam: the reader keeps only the records it
    // actually read, so the count is their number.
    let n = col.map_or(0, |c| c.primitives.len() as u32);
    if n != 0 {
        let col = col.unwrap();
        let (c, r) = GetDefaultPhysicBSphere(Some(col));
        model.bsphere_center = c;
        model.bsphere_radius = r;
        model.SetMaxPrims(n);
        for i in 0..n {
            let rec = &col.primitives[i as usize];
            let joint = rec.head[2] as u32;
            let (ltm, j) = if joint == 0xffff_ffff || !hanim { (root, 0xffff_ffff) } else { (node_ltm(joint), joint) };
            let Some(p) = model.AddPrimitive(i, rec.head[1] as u32, rec.head[0] as u32, ltm) else { continue };
            p.base.joint = j;
            GetDefaultCollPrimAttrib(col, i, p, ltm);
        }
    }
    hanim
}

/// J9: EFGameObject::InitHotspot(DTHotSpotDescriptor*) (0x00200880), collision part: one prim
/// (slot 0, tag 0) of `prim_type` on the object's root frame, filled from the descriptor
/// (+0x70.. floats, +0x30 translation, +0x44..+0x6c the 11 int attribs: base / current =
/// base & !lock | value & lock), then physic bsphere = (prim offset, prim radius).
pub fn InitHotspot(d: &crate::engine::dt::descriptors::DTHotSpotDescriptor, root: M, model: &mut EFCollisionModel) {
    model.SetMaxPrims(1);
    let Some(p) = model.AddPrimitive(0, 0, d.prim_type, root) else { return }; // kind > 3: no prim
    let f = d.prim;
    match &mut p.ext {
        PrimExt::Sphere => p.base.radius = f[0],
        PrimExt::SweptSphere { sradius, vec, .. } => {
            *sradius = f[0];
            *vec = [f[1], f[2], f[3]];
            p.base.radius = *sradius + length(*vec);
            let l2 = vec[2] * vec[2] + vec[0] * vec[0] + vec[1] * vec[1];
            if l2 < 0.0001 {
                if l2 == 0.0 {
                    *vec = [0.0, f32::from_bits(0x3c23_d70a), 0.0];
                } else {
                    // quirk (as GetDefaultCollPrimAttrib): scales by |v|*0.01
                    let s = l2.sqrt() * 0.01;
                    *vec = [vec[0] * s, vec[1] * s, vec[2] * s];
                }
                p.base.radius = *sradius + 0.01;
            }
        }
        PrimExt::BBox { half, .. } => {
            *half = [f[0], f[1], f[2]];
            p.base.radius = length(*half);
        }
        PrimExt::Polygon { .. } => {}
    }
    let b = &mut p.base;
    b.offset = [d.translation_x, d.translation_y, d.translation_z];
    for k in 0..3 {
        b.center[k] += b.offset[k];
    }
    let attribs = [d.detection_mode, d.type_, d.detect_type, d.ud_type, d.ud_detect_type, d.env_detect_type, d.ud_env_detect_type, d.notify_type, d.arg1, d.arg2, d.arg3];
    for (i, &base) in attribs.iter().enumerate() {
        b.set_word(0x38 + 4 * i, base);
        let lock = b.word(0x90 + 4 * i);
        let val = b.word(0x64 + 4 * i);
        b.set_word(0xc + 4 * i, (base & !lock) | (val & lock));
    }
    let (r, o) = (b.radius, b.offset);
    model.bsphere_radius = r;
    model.bsphere_center = o;
}

/// EFAnimNode::SetCollPrimData(EFObjCollisionPrim&, EFAnimCollPrim const&) (0x0022d970): one
/// EFAnimCollPrim record (anim/coll_prim.rs) onto a physic prim.
pub fn SetCollPrimDataPrim(prim: &mut ObjPrim, r: &crate::engine::anim::coll_blend::EFAnimBaseCollPrim) {
    match (r.ty, &mut prim.ext) {
        (3, PrimExt::BBox { half, .. }) => {
            *half = r.v(0x4c);
            prim.base.radius = length(*half);
        }
        (4, PrimExt::SweptSphere { sradius, vec, .. }) => {
            *sradius = r.f(0x4c);
            // quirk: the first radius uses the old vector, then it is recomputed
            prim.base.radius = *sradius + length(*vec);
            *vec = r.v(0x50);
            prim.base.radius = *sradius + length(*vec);
            let l2 = vec[2] * vec[2] + vec[0] * vec[0] + vec[1] * vec[1];
            if l2 < 0.0001 {
                if l2 == 0.0 {
                    *vec = [0.0, f32::from_bits(0x3c23_d70a), 0.0];
                } else {
                    let s = l2.sqrt() * 0.01;
                    *vec = [vec[0] * s, vec[1] * s, vec[2] * s];
                }
                prim.base.radius = *sradius + 0.01;
            }
        }
        (2, _) => prim.base.radius = r.f(0x4c),
        _ => {}
    }
    let b = &mut prim.base;
    b.offset = r.v(0x40);
    for k in 0..3 {
        b.center[k] += b.offset[k];
    }
    // quirk: as in GetDefaultCollPrimAttrib, attrib 6 is never copied (index 4 twice)
    for i in [0usize, 1, 2, 3, 4, 5, 4, 7, 8, 9, 10] {
        let base = r.body.get(i).copied().unwrap_or(0);
        b.set_word(0x38 + 4 * i, base);
        let lock = b.word(0x90 + 4 * i);
        let val = b.word(0x64 + 4 * i);
        b.set_word(0xc + 4 * i, (base & !lock) | (val & lock));
    }
    b.d8 &= !1;
    if r.flags & 1 != 0 {
        b.d8 |= 2;
    }
}

/// EFAnimNode::SetCollPrimData(void) (0x0022d760) for the root anim node of an EF3dObj with an
/// EFPhysic: the chain's head (clipping bsphere, EF3dObj::SetClippingBSphere: render clipping, not
/// modelled), an optional type-1 record (physic bsphere), then one record per prim index (else
/// the clump defaults). quirk not reproduced: the type-1 bsphere center is transformed through
/// the hierarchy root LTM and the inverse frame LTM; both are the object frame here (identity).
pub fn SetCollPrimData(model: &mut EFCollisionModel, list: &crate::engine::anim::coll_blend::CollPrimList, col: Option<&ApClumpCollision>, frame_ltm: M) {
    let Some(_head) = list.prims.first() else { return };
    let mut k = 1;
    match list.prims.get(1) {
        Some(r) if r.ty == 1 => {
            model.bsphere_center = r.v(0x14);
            model.bsphere_radius = r.f(0x20);
            k = 2;
        }
        _ => {
            if !model.prims.is_empty() && col.and_then(|c| c.bsphere).is_some() {
                let (c, r) = GetDefaultPhysicBSphere(col);
                model.bsphere_center = c;
                model.bsphere_radius = r;
            }
        }
    }
    for u in 0..model.prims.len() {
        let Some(prim) = model.prims[u].as_mut() else { continue };
        match list.prims.get(k) {
            Some(r) if r.tag == u as u32 => {
                if r.flags & 2 == 0 {
                    SetCollPrimDataPrim(prim, r);
                } else if let Some(c) = col {
                    GetDefaultCollPrimAttrib(c, u as u32, prim, frame_ltm);
                }
                k += 1;
            }
            _ => {
                if let Some(c) = col {
                    GetDefaultCollPrimAttrib(c, u as u32, prim, frame_ltm);
                }
            }
        }
    }
}

//! WP-E4: EFObjCollision{Sphere,SweptSphere,BBox,Polygon}::Check*Collision and the shared
//! EFCheckCollisionIn/Out blocks (placeholder structs in the decomp; fields named from usage).
//!
//! Return codes of the Check*Collision family: 0 no hit, 1 hit at `t` (+0x1c), 2 already
//! overlapping (`depth` +0x18, normal), 4 hit beyond `tmax`; helper flags 0x10/0x20 pass through.
#![allow(non_snake_case)]

use super::model::{ObjPrim, PrimExt};
use super::collision_map::{EFCollisionMap, EFRayIntersectOut, RayIntersectIn};
use super::objcoll_prim::*;

/// EFEnvCollisionPrim: a collision-map polygon (see collision_map.rs) copied out of the map blob.
/// C++: +0 material pointer (material = 8 words, word0 flags), +4 u16, +6 u16 nverts
/// (`& 0x7fff` for the vertex stride), +8 stamp, +0xc normal, +0x18 d, +0x1c verts, then edge
/// planes. `verts`/`edges` hold `nverts & 0x7fff` entries; `nverts` keeps the raw u16.
#[derive(Debug, Clone, Default)]
pub struct EnvPrim {
    /// word index of the primitive in the map blob (identity)
    pub id: usize,
    /// material record (8 words; `**prim` = mat[0])
    pub mat: [u32; 8],
    pub w4: u16,
    pub nverts: u16,
    pub normal: V3,
    pub d: f32,
    pub verts: Vec<V3>,
    pub edges: Vec<[f32; 4]>,
}

impl EnvPrim {
    pub fn from_map(map: &EFCollisionMap, p: usize) -> Self {
        let m = map.PrimMaterial(p);
        let mut mat = [0u32; 8];
        for (k, w) in mat.iter_mut().enumerate() {
            *w = map.w.get(m + k).copied().unwrap_or(0);
        }
        let n = map.PrimCount(p) & 0x7fff;
        EnvPrim {
            id: p,
            mat,
            w4: (map.w[p + 1] & 0xffff) as u16,
            nverts: (map.w[p + 1] >> 16) as u16,
            normal: map.PrimNormal(p),
            d: map.PrimD(p),
            verts: (0..n).map(|i| map.PrimVertex(p, i)).collect(),
            edges: (0..n).map(|i| map.PrimEdgePlane(p, i)).collect(),
        }
    }
}

/// EFCheckCollisionIn: +0 max t, +4 motion (world) of the testing primitive relative to the other.
#[derive(Debug, Clone, Copy, Default)]
pub struct EFCheckCollisionIn {
    pub tmax: f32,
    pub motion: V3,
}

/// EFCheckCollisionOut: +0 normal (world), +0xc motion done (= motion * t), +0x18 depth, +0x1c t.
#[derive(Debug, Clone, Copy, Default)]
pub struct EFCheckCollisionOut {
    pub normal: V3,
    pub motion: V3,
    pub depth: f32,
    pub t: f32,
}

/// EFObjCollisionPolygon +0xe8/+0xec prim list (empty for other kinds).
pub(crate) fn poly_prims(p: &ObjPrim) -> &[EnvPrim] {
    match &p.ext {
        PrimExt::Polygon { prims, .. } => prims,
        _ => &[],
    }
}

/// RwMatrix row `i` (right/up/at) of a matrix.
#[inline]
pub(crate) fn row(m: &[[f32; 4]; 4], i: usize) -> V3 {
    [m[i][0], m[i][1], m[i][2]]
}
#[inline]
pub(crate) fn sign_differs(a: f32, b: f32) -> bool {
    (a.to_bits() ^ b.to_bits()) & 0x8000_0000 != 0
}
/// @1309 (0x00315ca0): third axis of an edge pair, indexed by i*2+j: {2,1,0,0} (index 1,2,4 used).
pub(crate) const THIRD_AXIS: [usize; 5] = [0, 2, 1, 0, 0];

impl ObjPrim {
    /// vtable CheckObjPrimCollision: dispatch on the other primitive's kind (+4); unknown -> 0.
    pub fn CheckObjPrimCollision(&self, i: &EFCheckCollisionIn, o: &mut EFCheckCollisionOut, other: &ObjPrim) -> u32 {
        match &self.ext {
            PrimExt::Sphere => match other.base.kind {
                PRIM_SPHERE => self.Sphere_CheckSphereCollision(i, o, other),
                PRIM_SWEPT_SPHERE => self.Sphere_CheckSweptSphereCollision(i, o, other),
                PRIM_BBOX => self.Sphere_CheckBBoxCollision(i, o, other),
                PRIM_POLYGON => self.Sphere_CheckPolygonCollision(i, o, other),
                _ => 0,
            },
            PrimExt::SweptSphere { .. } => match other.base.kind {
                PRIM_SPHERE => self.Swept_CheckSphereCollision(i, o, other),
                PRIM_SWEPT_SPHERE => self.Swept_CheckSweptSphereCollision(i, o, other),
                PRIM_BBOX => self.Swept_CheckBBoxCollision(i, o, other),
                PRIM_POLYGON => self.Swept_CheckPolygonCollision(i, o, other),
                _ => 0,
            },
            PrimExt::BBox { .. } => match other.base.kind {
                PRIM_SPHERE => self.BBox_CheckSphereCollision(i, o, other),
                PRIM_SWEPT_SPHERE => self.BBox_CheckSweptSphereCollision(i, o, other),
                PRIM_BBOX => self.BBox_CheckBBoxCollision(i, o, other),
                PRIM_POLYGON => self.BBox_CheckPolygonCollision(i, o, other),
                _ => 0,
            },
            PrimExt::Polygon { .. } => self.Polygon_CheckObjPrimCollision(i, o, other),
        }
    }
}

// ------------------------------------------------------------------------------------------------
// EFObjCollisionSphere
// ------------------------------------------------------------------------------------------------
impl ObjPrim {
    /// EFObjCollisionSphere::CheckPointIntersection: |center - p|^2 <= r^2.
    pub fn Sphere_CheckPointIntersection(&self, p: V3) -> bool {
        let d = sub(self.base.center, p);
        dot(d, d) <= self.base.radius * self.base.radius
    }

    /// EFObjCollisionSphere::GetEscapeNoPF(pos, out, dir): horizontal escape direction from `dir`
    /// (x,z), or a random one (`rand()` seam) when |dir.xz|^2 <= 0.01.
    pub fn Sphere_GetEscapeNoPF(_p: V3, out: &mut V3, dir: V3, rand: &mut dyn FnMut() -> u32) {
        let x = dir[0];
        let l2 = dir[2] * dir[2] + x * x;
        if 0.01 < l2 {
            let r = 1.0 / l2.sqrt();
            *out = [x * r, 0.0, dir[2] * r];
        } else {
            let a = (rand() & 0xff) as f32 * 0.00390625 * 6.2831855;
            *out = [a.sin(), 0.0, a.cos()];
        }
    }

    /// EFObjCollisionSphere::CheckBBoxCollision (0x001d1d80).
    pub fn Sphere_CheckBBoxCollision(&self, i: &EFCheckCollisionIn, o: &mut EFCheckCollisionOut, bb: &ObjPrim) -> u32 {
        let PrimExt::BBox { half, m: world, inv, .. } = &bb.ext else { return 0 };
        let r = self.base.radius;
        let p = transform_point(inv, self.base.center);
        let mot = transform_vector(inv, i.motion);
        let mut sum = 0.0f32; // fVar22
        let mut maxaxis: i32 = -1; // iVar14
        let mut n_inside = 0u32; // uVar12 (|p|<h)
        let mut n_near = 0u32; // uVar13 (|p|-h<r)
        let mut inside = true; // bVar4
        let mut tenter = -1000.0f32; // fVar19
        let mut nearest = [0.0f32; 3];
        let r2 = r * r;
        let mut maxd = -100000.0f32; // fVar18
        for a in 0..3 {
            // fabs in double, minus half, back to float
            let d = ((p[a] as f64).abs() - half[a] as f64) as f32;
            if d < r {
                n_near += 1;
                if d < 0.0 {
                    n_inside += 1;
                    nearest[a] = 0.0;
                } else {
                    nearest[a] = if p[a] < 0.0 { -d } else { d };
                    sum += d * d;
                    if r2 < sum {
                        inside = false;
                    }
                }
                if d > maxd {
                    maxd = d;
                    maxaxis = a as i32;
                }
            } else {
                inside = false;
                if !sign_differs(p[a], mot[a]) {
                    return 0;
                }
                let t = ((d - r) as f64 / (mot[a] as f64).abs()) as f32;
                if tenter < t {
                    tenter = t;
                    if i.tmax < t {
                        return 0;
                    }
                }
            }
        }
        if inside {
            if 1 < n_inside || maxd < 0.0 {
                let ax = maxaxis as usize; // quirk: -1 impossible here (maxd>=-100000 always updated)
                let n = row(world, ax);
                o.normal = if p[ax] < 0.0 { neg(n) } else { n };
                o.depth = -maxd + r;
            } else {
                o.depth = r - sum.sqrt();
                o.normal = nearest;
                normalize(&mut o.normal);
                o.normal = transform_vector(world, o.normal);
            }
            return 2;
        }
        // contact point in box space
        let q = if n_near < 3 { add(p, scale(mot, tenter)) } else { p };
        let mut count = 0;
        let mut mask = 0u32;
        let mut last = 0usize;
        for a in 0..3 {
            if (q[a] as f64).abs() - half[a] as f64 > 0.0 {
                count += 1;
                mask |= 1 << a;
                last = a;
            }
        }
        if count == 1 {
            let n = row(world, last);
            o.normal = if p[last] < 0.0 { neg(n) } else { n };
            o.t = tenter;
            o.motion = scale(i.motion, tenter);
            return 1;
        }
        // edges: every pair of out-of-box axes
        let mut best = 0u32; // uVar17
        let mut bt = -10000.0f32; // fVar18
        let mut vin = EFCheckSphereVectorCollisionIn { vec: [0.0; 3], center: [0.0; 3], motion: mot, radius: r };
        // quirk: the out t is an uninitialised stack slot in the C++ (read before written on early returns)
        let mut vout = EFCheckSphereVectorCollisionOut::default();
        let (mut svec, mut scen, mut snorm) = ([0.0f32; 3], [0.0f32; 3], [0.0f32; 3]);
        for a in 0..2usize {
            if mask & (1 << a) == 0 {
                continue;
            }
            for b in a + 1..3 {
                if mask & (1 << b) == 0 {
                    continue;
                }
                vin.center = p;
                let k = THIRD_AXIS[a * 2 + b];
                vin.center[a] = if 0.0 < p[a] { p[a] - half[a] } else { p[a] + half[a] };
                vin.center[b] = if 0.0 < p[b] { p[b] - half[b] } else { p[b] + half[b] };
                vin.center[k] = p[k] + half[k];
                vin.vec = [0.0; 3];
                vin.vec[k] = half[k] * 2.0;
                let rr = EFObjCollisionPrim::CheckSphereVectorCollision(&vin, &mut vout);
                if rr & 0x30 != 0 {
                    return rr;
                }
                if i.tmax < vout.t {
                    return 4;
                }
                if rr & 8 != 0 && best == 0 {
                    svec = vin.vec;
                    bt = 0.0;
                    scen = vin.center;
                    snorm = vout.normal;
                    best = rr;
                }
                if rr & 1 != 0 && bt < vout.t {
                    svec = vin.vec;
                    scen = vin.center;
                    snorm = vout.normal;
                    best = rr;
                    bt = vout.t;
                }
            }
        }
        if best == 0 {
            return 0;
        }
        if best & 0x100 == 0 {
            let c = if best & 0x80 != 0 { sub(scen, svec) } else { scen };
            let sin = EFCheckSphereSphereCollisionIn { center: c, motion: mot, radius: r };
            let mut sout = EFCheckSphereSphereCollisionOut::default();
            let rr = EFObjCollisionPrim::CheckSphereSphereCollision(&sin, &mut sout);
            if rr & 1 == 0 {
                return 0;
            }
            if i.tmax < sout.t {
                return 4;
            }
            o.normal = sout.normal;
            o.t = sout.t;
        } else {
            o.t = bt;
            o.normal = snorm;
        }
        o.normal = transform_vector(world, o.normal);
        o.motion = scale(i.motion, o.t);
        1
    }

    /// EFObjCollisionSphere::CheckSweptSphereCollision (0x001d29c0): bool-returning (1/2/0).
    pub fn Sphere_CheckSweptSphereCollision(&self, i: &EFCheckCollisionIn, o: &mut EFCheckCollisionOut, sw: &ObjPrim) -> u32 {
        let PrimExt::SweptSphere { world_vec, sradius, .. } = &sw.ext else { return 0 };
        let wv = *world_vec;
        // relative center from the start of the capsule axis (center - world_vec)
        let mut c = add(sub(self.base.center, sw.base.center), wv);
        let v = scale(wv, 2.0);
        let cr = cross(c, v);
        let rad = self.base.radius + *sradius;
        let vv = dot(v, v);
        if dot(cr, cr) / vv < rad * rad {
            let mut pr = dot(c, v);
            if 0.0 <= pr {
                if vv < pr {
                    c = sub(c, v);
                } else {
                    pr /= vv;
                    c = sub(c, scale(v, pr));
                }
            }
            let l = dot(c, c).sqrt();
            if l < rad {
                o.depth = rad - l;
                if l == 0.0 {
                    o.normal = [0.0, 0.0, 1.0];
                } else {
                    let s = 1.0 / l;
                    o.normal = scale(c, s);
                }
                return 2;
            }
        }
        let sin = EFCheckSphereSweepCollisionIn { vec: v, center: c, motion: i.motion, radius: rad, tmax: i.tmax };
        let mut sout = EFCheckSphereSweepCollisionOut::default();
        let r = EFObjCollisionPrim::CheckSphereSweepCollision(&sin, &mut sout);
        if r & 1 != 0 {
            o.t = sout.t;
            o.normal = sout.normal;
            o.motion = scale(i.motion, o.t);
            1
        } else {
            0
        }
    }

    /// EFObjCollisionSphere::CheckSphereCollision (0x001d2ed0).
    pub fn Sphere_CheckSphereCollision(&self, i: &EFCheckCollisionIn, o: &mut EFCheckCollisionOut, s: &ObjPrim) -> u32 {
        let c = sub(self.base.center, s.base.center);
        let rad = self.base.radius + s.base.radius;
        let l2 = dot(c, c);
        if l2 < rad * rad {
            let tiny = |x: f32| ((x - 0.0).to_bits() & 0x7f80_0000) < 0x3c80_0000;
            if tiny(c[0]) && tiny(c[1]) && tiny(c[2]) {
                o.normal = [0.0, 1.0, 0.0];
            } else {
                o.normal = c;
                normalize(&mut o.normal);
            }
            o.depth = rad - l2.sqrt();
            return 2;
        }
        let sin = EFCheckSphereSphereCollisionIn { center: c, motion: i.motion, radius: rad };
        let mut sout = EFCheckSphereSphereCollisionOut::default();
        let r = EFObjCollisionPrim::CheckSphereSphereCollision(&sin, &mut sout);
        if r & 0x30 != 0 {
            return r;
        }
        if r & 1 == 0 {
            return 0;
        }
        if sout.t <= i.tmax {
            o.normal = sout.normal;
            o.t = sout.t;
            o.motion = sout.motion;
            1
        } else {
            4
        }
    }
}

/// The "swap" wrappers (e.g. EFObjCollisionBBox::CheckSphereCollision 0x001cd720): call the other
/// primitive's test with the motion negated (the C++ fills three copies of -motion into the In
/// block; only +4 is read by the callees), then negate normal and motion on a hit (result & 3).
/// quirk: the C++ wrappers are `void`; the caller reads the leftover v0 = the inner result.
fn swapped(i: &EFCheckCollisionIn, o: &mut EFCheckCollisionOut, f: impl FnOnce(&EFCheckCollisionIn, &mut EFCheckCollisionOut) -> u32) -> u32 {
    let ni = EFCheckCollisionIn { tmax: i.tmax, motion: neg(i.motion) };
    let r = f(&ni, o);
    if r & 3 != 0 {
        o.normal = neg(o.normal);
        o.motion = neg(o.motion);
    }
    r
}

impl ObjPrim {
    // ---- EFObjCollisionSweptSphere ----
    /// EFObjCollisionSweptSphere::CheckSphereCollision (0x001d4c40): swapped Sphere::CheckSweptSphereCollision.
    pub fn Swept_CheckSphereCollision(&self, i: &EFCheckCollisionIn, o: &mut EFCheckCollisionOut, s: &ObjPrim) -> u32 {
        swapped(i, o, |i, o| s.Sphere_CheckSweptSphereCollision(i, o, self))
    }

    /// EFObjCollisionSweptSphere::GetMinRadius: +0xec.
    // (see ObjPrim::GetMinRadius)

    // ---- swap wrappers of BBox / Polygon ----
    /// EFObjCollisionBBox::CheckSphereCollision (0x001cd720)
    pub fn BBox_CheckSphereCollision(&self, i: &EFCheckCollisionIn, o: &mut EFCheckCollisionOut, s: &ObjPrim) -> u32 {
        swapped(i, o, |i, o| s.Sphere_CheckBBoxCollision(i, o, self))
    }
    /// EFObjCollisionBBox::CheckSweptSphereCollision (0x001cd8b0)
    pub fn BBox_CheckSweptSphereCollision(&self, i: &EFCheckCollisionIn, o: &mut EFCheckCollisionOut, s: &ObjPrim) -> u32 {
        swapped(i, o, |i, o| s.Swept_CheckBBoxCollision(i, o, self))
    }
    /// EFObjCollisionPolygon::CheckObjPrimCollision (0x00271320): sphere / swept / bbox, else 0.
    pub fn Polygon_CheckObjPrimCollision(&self, i: &EFCheckCollisionIn, o: &mut EFCheckCollisionOut, other: &ObjPrim) -> u32 {
        match other.base.kind {
            PRIM_SPHERE => swapped(i, o, |i, o| other.Sphere_CheckPolygonCollision(i, o, self)),
            PRIM_SWEPT_SPHERE => swapped(i, o, |i, o| other.Swept_CheckPolygonCollision(i, o, self)),
            PRIM_BBOX => swapped(i, o, |i, o| other.BBox_CheckPolygonCollision(i, o, self)),
            _ => 0,
        }
    }

    // ---- point / particle tests ----
    /// EFObjCollisionBBox::CheckPointIntersection (0x001cc810): |inv*p| <= half per axis (double compare).
    pub fn BBox_CheckPointIntersection(&self, p: V3) -> bool {
        let PrimExt::BBox { half, inv, .. } = &self.ext else { return false };
        let q = transform_point(inv, p);
        (0..3).all(|a| !((q[a] as f64).abs() > half[a] as f64))
    }
    /// EFObjCollisionBBox::CheckParticlesIntersection (0x001cde10) over (pos, radius, stamp) particles
    /// (PLParticle +4 pos, +0x20 radius, +0x28 u16 stamp; list order). `global` = PLParticle::mui16GlobalCount.
    /// Polygon returns 0.
    pub fn BBox_CheckParticlesIntersection(&self, parts: &mut [(V3, f32, u16)], mark: bool, global: u16) -> i32 {
        let PrimExt::BBox { half, inv, .. } = &self.ext else { return 0 };
        let mut n = 0;
        for (pos, r, stamp) in parts.iter_mut() {
            if *stamp == global {
                continue;
            }
            let q = transform_point(inv, *pos);
            if !((q[0] as f64).abs() > (half[0] + *r) as f64)
                && !((q[1] as f64).abs() > (half[1] + *r) as f64)
                && !((q[2] as f64).abs() > (half[2] + *r) as f64)
            {
                n += 1;
                if !mark {
                    return n;
                }
                *stamp = global;
            }
        }
        n
    }
}

// ------------------------------------------------------------------------------------------------
// virtual dispatchers (vtable slots) for the WP-E5 ports
// ------------------------------------------------------------------------------------------------
impl ObjPrim {
    /// vtable CheckEnvCollision (Sphere 0x001d31e0, Swept 0x001d6200, BBox 0x001cda40: all call
    /// their CheckSinglePolygonCollision(..., 1)); Polygon 0x00271310 returns 0.
    pub fn CheckEnvCollision(&self, i: &EFCheckCollisionIn, o: &mut EFCheckCollisionOut, e: &EnvPrim) -> u32 {
        match &self.ext {
            PrimExt::Sphere => self.Sphere_CheckEnvCollision(i, o, e),
            PrimExt::SweptSphere { .. } => self.Swept_CheckEnvCollision(i, o, e),
            PrimExt::BBox { .. } => self.BBox_CheckEnvCollision(i, o, e),
            PrimExt::Polygon { .. } => 0,
        }
    }
    /// vtable RayIntersect(RayIntersectIn const&, EFRayIntersectOut&, float&).
    pub fn RayIntersect(&self, rin: &RayIntersectIn, out: &mut EFRayIntersectOut, t: &mut f32) -> u32 {
        match &self.ext {
            PrimExt::Sphere => self.Sphere_RayIntersect(rin, out, t),
            PrimExt::SweptSphere { .. } => self.Swept_RayIntersect(rin, out, t),
            PrimExt::BBox { .. } => self.BBox_RayIntersect(rin, out, t),
            PrimExt::Polygon { .. } => self.Polygon_RayIntersect(rin, out, t),
        }
    }
    /// vtable CheckPointIntersection. Polygon (0x00271390) returns 0.
    pub fn CheckPointIntersection(&self, p: V3) -> bool {
        match &self.ext {
            PrimExt::Sphere => self.Sphere_CheckPointIntersection(p),
            PrimExt::SweptSphere { .. } => self.Swept_CheckPointIntersection(p),
            PrimExt::BBox { .. } => self.BBox_CheckPointIntersection(p),
            PrimExt::Polygon { .. } => false,
        }
    }
    /// vtable CheckParticlesIntersection(PLParticle* list, bool mark) over (pos +4, radius +0x20,
    /// u16 stamp +0x28) in list order; `global` = PLParticle::mui16GlobalCount. Polygon returns 0.
    pub fn CheckParticlesIntersection(&self, parts: &mut [(V3, f32, u16)], mark: bool, global: u16) -> i32 {
        match &self.ext {
            PrimExt::Sphere => self.Sphere_CheckParticlesIntersection(parts, mark, global),
            PrimExt::SweptSphere { .. } => self.Swept_CheckParticlesIntersection(parts, mark, global),
            PrimExt::BBox { .. } => self.BBox_CheckParticlesIntersection(parts, mark, global),
            PrimExt::Polygon { .. } => 0,
        }
    }
    /// vtable GetEscapeNoPF(pos, out, dir). `rand` = libc rand() seam. Polygon (0x00271810) no-op.
    pub fn GetEscapeNoPF(&self, p: V3, out: &mut V3, dir: V3, rand: &mut dyn FnMut() -> u32) {
        match &self.ext {
            PrimExt::Sphere => Self::Sphere_GetEscapeNoPF(p, out, dir, rand),
            PrimExt::SweptSphere { .. } => self.Swept_GetEscapeNoPF(p, out, dir, rand),
            PrimExt::BBox { .. } => self.BBox_GetEscapeNoPF(p, out, dir, rand),
            PrimExt::Polygon { .. } => {}
        }
    }
}

// ================================================================================================
#[allow(unused_variables, unused_imports)]
pub mod sphere {
    // WP-E5 part: EFObjCollisionSphere (merged into objcoll_check.rs at the end)
    use crate::engine::physic::collision_map::{EFRayIntersectOut, RayIntersectIn};
    use crate::engine::physic::model::{ObjPrim, PrimExt};
    use super::*;
    use crate::engine::physic::objcoll_prim::*;

    #[inline]
    fn sign_bit(f: f32) -> bool {
        f.to_bits() & 0x8000_0000 != 0
    }

    /// Shared tail of the hit paths: single-sided env polygons (flags && !(mat0 & 1)) reject a
    /// normal facing against the polygon normal.
    #[inline]
    fn back_face(flags: u32, matf: u32, o: &EFCheckCollisionOut, e: &EnvPrim) -> bool {
        flags != 0 && matf & 1 == 0 && dot(o.normal, e.normal) < 0.0
    }

    impl ObjPrim {
        /// EFObjCollisionSphere::CheckEnvCollision (0x001d31e0).
        pub fn Sphere_CheckEnvCollision(&self, i: &EFCheckCollisionIn, o: &mut EFCheckCollisionOut, e: &EnvPrim) -> u32 {
            self.Sphere_CheckSinglePolygonCollision(i, o, e, self.base.center, 1)
        }

        /// EFObjCollisionSphere::CheckSinglePolygonCollision (0x001d3200). `flags` != 0 = env mode
        /// (material word0: bit0 double-sided, bit1 skip when moving along the normal).
        /// Returns 0 / 1 hit at t / 2 overlap (depth) / 4 beyond tmax, or the raw
        /// CheckSphereVectorCollision flags (0x10/0x20 | ...) of an edge sweep.
        pub fn Sphere_CheckSinglePolygonCollision(&self, i: &EFCheckCollisionIn, o: &mut EFCheckCollisionOut, e: &EnvPrim, pos: V3, flags: u32) -> u32 {
            let r = self.base.radius;
            let ln = length(e.normal);
            if ln < 0.99 || 1.01 < ln {
                return 0;
            }
            let matf = if flags != 0 { e.mat[0] } else { 0 };
            let mn = dot(i.motion, e.normal);
            let along = mn * i.tmax;
            if flags != 0 && matf & 2 != 0 && 0.0 < along {
                return 0;
            }
            let dist = dot(pos, e.normal) + e.d;
            let end = dist + along;
            // quirk: second clause tests `end > r` again (0x001d33dc c.ole f1,f6), not `end < -r`
            if (r < dist && r < end) || (dist < -r && r < end) {
                return 0;
            }
            let rr = r * r;
            // quirk: C++ loops over the raw u16 (no & 0x7fff); clamped to the copied vertices
            let n = (e.nverts as usize).min(e.verts.len());
            let mut mask: u32 = 0;
            if (dist.abs() as f64) < r as f64 {
                // overlapping the plane
                let mut best: u32 = 0xffff_ffff;
                let mut k = 0usize;
                let mut maxd = 0.0f32;
                while k < n {
                    let ed = e.edges[k];
                    let de = dot(pos, [ed[0], ed[1], ed[2]]) + ed[3];
                    if 0.0 < de {
                        mask |= 1u32.wrapping_shl(k as u32);
                        if maxd < de {
                            maxd = de;
                            if rr < dist * dist + de * de {
                                break; // quirk: best not updated for the breaking edge
                            }
                            best = k as u32;
                        }
                    }
                    k += 1;
                }
                let hit;
                if mask == 0 {
                    o.normal = if 0.0 < dist { e.normal } else { neg(e.normal) };
                    o.depth = (r as f64 - (dist.abs() as f64)) as f32;
                    hit = true;
                } else if k == n {
                    let b = best as usize;
                    let d = sub(pos, e.verts[b]);
                    let nx = if b + 1 == n { 0 } else { b + 1 };
                    let ev = sub(e.verts[nx], e.verts[b]);
                    let s = dot(d, ev) / dot(ev, ev);
                    let proj = if !sign_bit(s) { if s <= 1.0 { scale(ev, s) } else { ev } } else { [0.0; 3] };
                    o.normal = sub(d, proj);
                    if dot(o.normal, o.normal) < rr {
                        o.depth = r - length(o.normal);
                        normalize(&mut o.normal);
                        hit = true;
                    } else {
                        hit = false;
                    }
                } else {
                    hit = false;
                }
                if hit {
                    if flags != 0 && matf & 1 == 0 {
                        let dn = dot(e.normal, o.normal);
                        if dn < 0.0 {
                            o.normal = sub(o.normal, scale(e.normal, dn));
                            o.normal = scale(o.normal, o.depth);
                            o.depth = length(o.normal);
                            if o.depth <= 0.001 {
                                return 0;
                            }
                            o.normal = scale(o.normal, 1.0 / o.depth);
                        }
                    }
                    return 2;
                }
            } else if i.tmax.to_bits() == 0 {
                // quirk: int compare of the float bits (-0.0 passes)
                return 0;
            }
            if !((dist.abs() as f64) < r as f64) {
                // sweep against the plane
                let target = if 0.0 < dist { dist - r } else { dist + r };
                let c = -mn;
                let t = target / c;
                if sign_differs(c, target) {
                    return 0;
                }
                if (c.abs() as f64) < f64::from_bits(0x3ee4f8b580000000) {
                    return 0;
                }
                if i.tmax < t || t < 0.0 {
                    return 0;
                }
                o.motion = scale(i.motion, t);
                let p = add(pos, o.motion);
                for k in 0..n {
                    let ed = e.edges[k];
                    let de = dot(p, [ed[0], ed[1], ed[2]]) + ed[3];
                    if !sign_bit(de) {
                        mask |= 1u32.wrapping_shl(k as u32);
                    }
                }
                if mask == 0 {
                    o.t = t;
                    o.normal = if 0.0 < dist { e.normal } else { neg(e.normal) };
                    return if back_face(flags, matf, o, e) { 0 } else { 1 };
                }
            }
            if mask != 0 {
                let mut best: u32 = 0xffff_ffff;
                let mut bt = -10000.0f32;
                let mut bflags = 0u32;
                let mut sv = EFCheckSphereVectorCollisionOut::default();
                let mut sel = EFCheckSphereVectorCollisionOut::default();
                for k in 0..n {
                    if mask & 1u32.wrapping_shl(k as u32) == 0 {
                        continue;
                    }
                    let nx = if k + 1 == n { 0 } else { k + 1 };
                    let vi = EFCheckSphereVectorCollisionIn {
                        vec: sub(e.verts[nx], e.verts[k]),
                        center: sub(pos, e.verts[k]),
                        motion: i.motion,
                        radius: r,
                    };
                    let res = EFObjCollisionPrim::CheckSphereVectorCollision(&vi, &mut sv);
                    if res & 0x30 != 0 {
                        return res; // quirk: raw helper flags returned (0x001d3e74)
                    }
                    if i.tmax < sv.t {
                        return 4;
                    }
                    if res & 8 != 0 && best == 0xffff_ffff {
                        bt = 0.0;
                        sel.normal = sv.normal;
                        sel.motion = [0.0; 3];
                        best = k as u32;
                        bflags = res;
                    }
                    // quirk: keeps the LARGEST t
                    if res & 1 != 0 && bt < sv.t {
                        sel.normal = sv.normal;
                        sel.motion = sv.motion;
                        bt = sv.t;
                        best = k as u32;
                        bflags = res;
                    }
                }
                if best != 0xffff_ffff {
                    if bflags & 0x100 != 0 {
                        o.t = bt;
                        o.normal = sel.normal;
                        o.motion = sel.motion;
                        return if back_face(flags, matf, o, e) { 0 } else { 1 };
                    }
                    let b = best as usize;
                    let v = if bflags & 0x40 == 0 { e.verts[if b + 1 == n { 0 } else { b + 1 }] } else { e.verts[b] };
                    let si = EFCheckSphereSphereCollisionIn { center: sub(pos, v), motion: i.motion, radius: r };
                    let mut so = EFCheckSphereSphereCollisionOut::default();
                    let res = EFObjCollisionPrim::CheckSphereSphereCollision(&si, &mut so);
                    // quirk: an 8 (already inside the vertex sphere) result falls through to 0
                    if res & 1 != 0 {
                        if i.tmax < so.t {
                            return 4;
                        }
                        o.t = so.t;
                        o.normal = so.normal;
                        o.motion = so.motion;
                        return if back_face(flags, matf, o, e) { 0 } else { 1 };
                    }
                }
            }
            0
        }

        /// EFObjCollisionSphere::CheckPolygonCollision (0x001d4590). The C++ reads from the polygon prim
        /// +0xe0 m, +0xe4 inv, +0xe8 u32 polygon count, +0xec pointer to packed EFEnvCollisionPrim records
        /// (stride 0x1c + raw nverts*0x1c) = PrimExt::Polygon::prims.
        pub fn Sphere_CheckPolygonCollision(&self, i: &EFCheckCollisionIn, o: &mut EFCheckCollisionOut, p: &ObjPrim) -> u32 {
            let PrimExt::Polygon { m, inv, prims } = &p.ext else { return 0 };
            self.Sphere_CheckPolygonCollisionPrims(i, o, m, inv, prims)
        }

        /// Body of 0x001d4590 with the polygon prim's +0xe8/+0xec list passed explicitly.
        pub fn Sphere_CheckPolygonCollisionPrims(&self, i: &EFCheckCollisionIn, o: &mut EFCheckCollisionOut, m: &[[f32; 4]; 4], inv: &[[f32; 4]; 4], polys: &[EnvPrim]) -> u32 {
            let pos = transform_point(inv, self.base.center);
            let li = EFCheckCollisionIn { tmax: i.tmax, motion: transform_vector(inv, i.motion) };
            let mut res = 0;
            let mut best = EFCheckCollisionOut::default();
            let mut bt = i.tmax + 1.0;
            for e in polys {
                let r = self.Sphere_CheckSinglePolygonCollision(&li, o, e, pos, 0);
                if r == 1 || r == 2 {
                    o.normal = transform_vector(m, o.normal);
                    // quirk: motion transformed with RwV3dTransformPoints (adds m.pos)
                    o.motion = transform_point(m, o.motion);
                    // quirk: a result-2 hit compares the stale o.t (not written on that path)
                    if o.t < bt {
                        best = *o;
                        res = r;
                        bt = o.t;
                    }
                }
            }
            *o = best;
            res
        }

        /// EFObjCollisionSphere::RayIntersect (0x001d4420): ray as a moving point vs the sphere.
        pub fn Sphere_RayIntersect(&self, rin: &RayIntersectIn, out: &mut EFRayIntersectOut, t: &mut f32) -> u32 {
            let si = EFCheckSphereSphereCollisionIn { center: sub(rin.origin, self.base.center), motion: rin.dir, radius: self.base.radius };
            let mut so = EFCheckSphereSphereCollisionOut::default();
            let r = EFObjCollisionPrim::CheckSphereSphereCollision(&si, &mut so);
            if r & 9 == 0 {
                return 0;
            }
            if r & 8 == 0 {
                if *t < so.t {
                    return 0;
                }
                *t = so.t;
            } else {
                *t = 0.0;
            }
            out.hit.normal = so.normal;
            1
        }

        /// EFObjCollisionSphere::CheckParticlesIntersection (0x001d4280) over (pos +4, radius +0x20, stamp +0x28).
        pub fn Sphere_CheckParticlesIntersection(&self, parts: &mut [(V3, f32, u16)], mark: bool, global: u16) -> i32 {
            let mut n = 0;
            for (pos, pr, stamp) in parts.iter_mut() {
                if mark && *stamp == global {
                    continue;
                }
                let d = sub(*pos, self.base.center);
                let rs = self.base.radius + *pr;
                if dot(d, d) <= rs * rs {
                    if !mark {
                        return 1;
                    }
                    *stamp = global;
                    n += 1;
                }
            }
            n
        }
    }

}

// ================================================================================================
#[allow(unused_variables, unused_imports)]
pub mod swept {
    // WP-E5 part: EFObjCollisionSweptSphere (merged into objcoll_check.rs at the end)
    // (Classes/EF/EFObjCollisionSweptSphere.cpp). `this+0xe0` world_vec (half segment, world),
    // `+0xec` sradius, `+0xbc` bounding radius, `+0xc0` center; the capsule axis is center ± world_vec.
    use crate::engine::physic::collision_map::{EFRayIntersectOut, RayIntersectIn};
    use crate::engine::physic::model::{ObjPrim, PrimExt};
    use super::*;
    use crate::engine::physic::objcoll_prim::*;

    /// OtherAxeIndex@1318 (0x00315cb0): third axis indexed by the bit mask of two axes (3->2, 5->1, 6->0).
    const OTHER_AXE: [usize; 8] = [usize::MAX, usize::MAX, usize::MAX, 2, usize::MAX, 1, 0, usize::MAX];
    /// 0x3f50624de0000000 (0.001 as double)
    const EPS_001: u64 = 0x3f50_624d_e000_0000;

    #[inline]
    fn neg_bit(x: f32) -> bool {
        x.to_bits() & 0x8000_0000 != 0
    }
    /// edge plane test as compiled: `e.w + (p.z*e.z + p.x*e.x + p.y*e.y)`
    #[inline]
    fn edge_dot(e: [f32; 4], p: V3) -> f32 {
        e[3] + dot(p, [e[0], e[1], e[2]])
    }
    #[inline]
    fn bit(k: usize) -> u32 {
        1u32.wrapping_shl(k as u32) // `1 << (k & 0x1f)`
    }

    impl ObjPrim {
        fn swept_parts(&self) -> (V3, f32) {
            match &self.ext {
                PrimExt::SweptSphere { world_vec, sradius, .. } => (*world_vec, *sradius),
                _ => ([0.0; 3], 0.0),
            }
        }

        /// EFObjCollisionSweptSphere::CheckBBoxCollision (0x001d4d30). Returns 0 / 1 hit at t / 2 overlap.
        pub fn Swept_CheckBBoxCollision(&self, i: &EFCheckCollisionIn, o: &mut EFCheckCollisionOut, b: &ObjPrim) -> u32 {
            let PrimExt::BBox { half, m: world, inv, .. } = &b.ext else { return 0 };
            let half = *half;
            let (wv, sr) = self.swept_parts();
            // everything in box space (+0xf0 inverse)
            let p = transform_point(inv, self.base.center);
            let mot = transform_vector(inv, i.motion);
            let w = transform_vector(inv, wv);
            let mut inside = true; // bVar1
            let mut ax19 = usize::MAX; // uVar19 (entering axis)
            let mut tenter = -10000.0f32; // fVar22
            let mut fmax = -10000.0f32; // fVar23: max face distance of the near axes
            let mut ax7 = 0usize; // uVar7
            for a in 0..3 {
                let d = (((p[a] as f64).abs() - half[a] as f64) - (w[a] as f64).abs()) as f32;
                if d < sr {
                    if d > fmax {
                        fmax = d;
                        ax7 = a;
                    }
                } else {
                    inside = false;
                    if (mot[a] as f64).abs() < f64::from_bits(EPS_001) {
                        return 0;
                    }
                    if !sign_differs(p[a], mot[a]) {
                        return 0;
                    }
                    let t = (((d - sr) / mot[a]) as f64).abs() as f32;
                    if tenter < t {
                        tenter = t;
                        ax19 = a;
                        if i.tmax < t {
                            return 0;
                        }
                    }
                }
            }
            // box edges (infinite lines along each axis through the box center) vs the capsule axis
            let mut emax = -10000.0f32; // fVar21
            let hs = [half[0] * half[0], half[1] * half[1], half[2] * half[2]];
            let hsum = hs[2] + hs[0] + hs[1];
            let mut edist = [0.0f32; 3]; // afStack_220
            for a in 0..3 {
                let mut e = [0.0f32; 3];
                e[a] = 1.0;
                let din = EFGetVectorVectorDistanceIn { a: e, b: w, off: p, maxdist: 0.0 };
                let mut dout = EFGetVectorVectorDistanceOut::default();
                if EFObjCollisionPrim::GetVectorVectorDistance(&din, &mut dout) & 0x400 == 0 {
                    edist[a] = -10000.0;
                } else {
                    let v = dout.dist - (hsum - hs[a]).sqrt();
                    edist[a] = v;
                    if v < sr {
                        if emax < v {
                            o.normal = dout.dir;
                            emax = v;
                        }
                    } else {
                        let dd = dot(dout.dir, mot);
                        inside = false;
                        if (dd.to_bits() & 0x7f80_0000) < 0x3c00_0000 || !neg_bit(dd) {
                            return 0;
                        }
                    }
                }
            }
            let face_normal = |o: &mut EFCheckCollisionOut, ax: usize| {
                let n = row(world, ax);
                o.normal = if p[ax] < 0.0 { neg(n) } else { n };
            };
            if inside {
                if emax < 0.0 || fmax < 0.0 {
                    if emax <= fmax {
                        o.depth = -fmax + sr;
                        face_normal(o, ax7);
                    } else {
                        o.depth = -emax + sr;
                        o.normal = transform_vector(world, o.normal);
                    }
                    return 2;
                }
                // capsule end on the box side of the max face axis
                let q = add(if !sign_differs(p[ax7], w[ax7]) { neg(w) } else { w }, p);
                let mut cnt = 0;
                let mut best = -10000.0f32;
                let mut k = 0usize; // iVar13
                for bb in 0..3 {
                    if bb == ax7 {
                        continue;
                    }
                    if ((q[bb] as f64).abs() - half[bb] as f64) as f32 >= 0.0 {
                        let kk = OTHER_AXE[(1 << bb) | (1 << ax7)];
                        if edist[kk] > best {
                            best = edist[kk];
                            k = kk;
                        }
                    } else {
                        cnt += 1;
                    }
                }
                if cnt == 2 {
                    o.depth = -fmax + sr;
                    face_normal(o, ax7);
                    return 2;
                }
                let mut a = [0.0f32; 3];
                a[k] = half[k] * 2.0;
                let mut off = [0.0f32; 3];
                for j in 0..3 {
                    off[j] = if j == k {
                        p[j] + half[j]
                    } else if 0.0 < q[j] {
                        p[j] - half[j]
                    } else {
                        p[j] + half[j]
                    };
                }
                let nin = EFVectorVectorNearestPointIn { a, b: add(w, w), off: sub(off, w), maxdist: sr };
                let mut nout = EFVectorVectorNearestPointOut::default();
                if EFObjCollisionPrim::GetVectorVectorNearestPoint(&nin, &mut nout) & 0x400 != 0 {
                    o.depth = sr - nout.dist;
                    o.normal = transform_vector(world, nout.dir);
                    return 2;
                }
            }
            // face sweep
            let pos;
            if 0.0 <= tenter {
                let ax = ax19;
                let mut ps = add(p, scale(mot, tenter));
                // quirk: the end is chosen with the sign of the start position p[ax], not ps[ax]
                let end = if !sign_differs(p[ax], w[ax]) { sub(ps, w) } else { add(ps, w) };
                let mut all = true;
                for bb in 0..3 {
                    if bb != ax && (end[bb] as f64).abs() > half[bb] as f64 {
                        all = false;
                        break;
                    }
                }
                if all {
                    let n = row(world, ax);
                    o.normal = if 0.0 < ps[ax] { n } else { neg(n) };
                    o.t = tenter;
                    o.motion = scale(i.motion, tenter);
                    return 1;
                }
                ps[ax] = if 0.0 < ps[ax] { sr + half[ax] } else { -half[ax] - sr };
                pos = ps;
            } else {
                pos = p;
            }
            // classify both capsule ends per axis (aiStack_360[end][axis])
            let ends = [sub(pos, w), add(pos, w)];
            let mut cls = [[0i32; 3]; 2];
            for a in 0..3 {
                for j in 0..2 {
                    let v = ends[j][a];
                    cls[j][a] = if half[a] < v {
                        1
                    } else if v < -half[a] {
                        -1
                    } else {
                        0
                    };
                }
            }
            // box edges vs the capsule line. quirk: the edge offsets use the start p (not pos).
            let mut best_flags = 0u32; // uVar19
            let mut off = [0.0f32; 3]; // afStack_308 (persists across pairs)
            let (mut ba, mut boff, mut bnorm) = ([0.0f32; 3], [0.0f32; 3], [0.0f32; 3]);
            let mut bt = 0.0f32; // quirk: fStack_290 is uninitialised when no pair is accepted
            let mut iout = EFCheckInfiniteVectorVectorCollisionOut::default();
            let side = |c: i32, a: usize| if c < 1 { half[a] + p[a] } else { p[a] - half[a] };
            for a in 0..2 {
                for bb in a + 1..3 {
                    if cls[0][a] == 0 || cls[1][bb] == 0 {
                        if cls[1][a] != 0 && cls[0][bb] != 0 {
                            off[bb] = side(cls[0][bb], bb);
                            off[a] = side(cls[1][a], a);
                        } else {
                            continue;
                        }
                    } else {
                        off[a] = side(cls[0][a], a);
                        off[bb] = side(cls[1][bb], bb);
                    }
                    let k = OTHER_AXE[(1 << a) | (1 << bb)];
                    let mut ev = [0.0f32; 3];
                    ev[k] = half[k] + half[k];
                    off[k] = p[k] + half[k];
                    let iin = EFCheckInfiniteVectorVectorCollisionIn { a: ev, b: w, off, motion: mot, radius: sr };
                    let r = EFObjCollisionPrim::CheckInfiniteVectorVectorCollision(&iin, &mut iout);
                    if r & 0x30 != 0 {
                        return 0;
                    }
                    // quirk: compares with the constant -10000 (0xc61c4000 @0x001d5f80), so every later
                    // hit replaces the previous one (no nearest-t selection)
                    if (r & 8 != 0 && best_flags & 1 == 0) || (r & 1 != 0 && -10000.0 < iout.t) {
                        boff = off;
                        ba = ev;
                        bnorm = iout.normal;
                        best_flags = r;
                        if r & 8 == 0 {
                            if i.tmax < iout.t {
                                return 0;
                            }
                            bt = iout.t;
                        } else {
                            bt = 0.0;
                        }
                    }
                }
            }
            let fin = EFCheckFiniteVectorVectorCollisionIn {
                a: ba,
                b: add(w, w),
                off: sub(boff, w),
                motion: mot,
                radius: sr,
                normal: bnorm,
                t: bt,
                tmax: i.tmax,
            };
            let mut fout = EFCheckFiniteVectorVectorCollisionOut::default();
            if EFObjCollisionPrim::CheckFiniteVectorVectorCollision(&fin, &mut fout, best_flags) & 1 != 0 {
                o.t = fout.t;
                o.normal = transform_vector(world, fout.normal);
                o.motion = scale(i.motion, fout.t);
                1
            } else {
                0
            }
        }

        /// EFObjCollisionSweptSphere::CheckPolygonCollision (0x001d8140). The C++ reads polygon
        /// `+0xe8` (u32 count) and `+0xec` (EFEnvCollisionPrim* array, stride `0x1c + 0x1c*raw nverts`)
        /// = PrimExt::Polygon::prims.
        pub fn Swept_CheckPolygonCollision(&self, i: &EFCheckCollisionIn, o: &mut EFCheckCollisionOut, p: &ObjPrim) -> u32 {
            self.Swept_CheckPolygonCollision_prims(i, o, p, poly_prims(p))
        }

        /// EFObjCollisionSweptSphere::CheckPolygonCollision (0x001d8140) with the polygon's env prims
        /// (polygon +0xe8 count / +0xec array). Tests every prim in polygon space (+0xe4 inverse) with
        /// flags 0, keeps the result with the smallest t (< tmax+1). quirks: the out motion is mapped
        /// back with RwV3dTransformPoints (+0xe0, adds the translation); a result 2 compares a stale t;
        /// depth/t of the kept copy are uninitialised (0 here) when nothing is kept.
        pub fn Swept_CheckPolygonCollision_prims(&self, i: &EFCheckCollisionIn, o: &mut EFCheckCollisionOut, p: &ObjPrim, prims: &[EnvPrim]) -> u32 {
            let PrimExt::Polygon { m, inv, .. } = &p.ext else { return 0 };
            let (wv, _) = self.swept_parts();
            let li = EFCheckCollisionIn { tmax: i.tmax, motion: transform_vector(inv, i.motion) };
            let c = transform_point(inv, self.base.center);
            let w = transform_vector(inv, wv);
            let mut best = EFCheckCollisionOut::default();
            let mut res = 0;
            let mut bt = i.tmax + 1.0;
            for e in prims {
                let r = self.Swept_CheckSinglePolygonCollision(&li, o, e, c, w, 0);
                if r == 1 || r == 2 {
                    o.normal = transform_vector(m, o.normal);
                    o.motion = transform_point(m, o.motion);
                    if o.t < bt {
                        best = *o;
                        res = r;
                        bt = o.t;
                    }
                }
            }
            *o = best;
            res
        }

        /// EFObjCollisionSweptSphere::CheckEnvCollision (0x001d6200): CheckSinglePolygonCollision(center, world_vec, 1).
        /// (void in the decomp; v0 = inner result)
        pub fn Swept_CheckEnvCollision(&self, i: &EFCheckCollisionIn, o: &mut EFCheckCollisionOut, e: &EnvPrim) -> u32 {
            let (wv, _) = self.swept_parts();
            self.Swept_CheckSinglePolygonCollision(i, o, e, self.base.center, wv, 1)
        }

        /// EFObjCollisionSweptSphere::CheckSinglePolygonCollision (0x001d6230): capsule (center `c`,
        /// half axis `w`) vs one polygon. `flags` 1 = env (material word 0: bit 1 one-way, bit 0
        /// double-sided), 0 = object polygon. Returns 0 / 1 hit at t / 2 overlap / 4 beyond tmax.
        /// quirk: the vertex/edge loops run over the raw u16 nverts (C++ reads past the prim when bit 15
        /// is set); here clamped to the stored edges.
        pub fn Swept_CheckSinglePolygonCollision(&self, i: &EFCheckCollisionIn, o: &mut EFCheckCollisionOut, e: &EnvPrim, c: V3, w: V3, flags: u32) -> u32 {
            let (_, sr) = self.swept_parts();
            let n = e.normal;
            let mut mat0 = 0u32; // uStack_4c0
            let mm = scale(i.motion, i.tmax);
            let mn = dot(mm, n);
            if flags == 1 {
                mat0 = e.mat[0];
            }
            if !(flags == 0 || mat0 & 2 == 0 || mn <= 0.0) {
                return 0;
            }
            let pd = dot(c, n) + e.d; // fVar27
            let wn = dot(w, n); // fStack_18
            let pe = pd + mn;
            // wp: the half axis pointing toward the plane
            let (wp, wpn) = if !sign_differs(pd, wn) { (neg(w), -wn) } else { (w, wn) };
            let r = self.base.radius;
            // quirk: the second test was probably meant `pe < -r`
            if (r < pd && r < pe) || (pd < -r && r < pe) {
                return 0;
            }
            let nv = (e.nverts as usize).min(e.edges.len()).min(e.verts.len());
            let mut out_bits = 0u32; // uVar21
            let fo = (((pd as f64).abs() - (wpn as f64).abs()) - sr as f64) as f32;
            if fo < 0.0 {
                let mut ok = true; // bVar2
                let near = add(c, wp);
                o.normal = if 0.0 < pd { n } else { neg(n) };
                let mut in_bits = 0u32; // uVar17
                o.depth = -fo;
                let mut use_near = true; // bVar1
                if !((wpn as f64).abs() > f64::from_bits(EPS_001)) {
                    in_bits = 0xffff_ffff;
                } else {
                    // axis/plane crossing point (operator*(wp, pd / -wpn) verified at 0x001d66d4)
                    let x = add(c, scale(wp, pd / -wpn));
                    for k in 0..nv {
                        if !neg_bit(edge_dot(e.edges[k], x)) {
                            in_bits |= bit(k);
                        }
                    }
                    if (wpn as f64).abs() > (pd as f64).abs() {
                        out_bits = if in_bits == 0 { 0xffff_ffff } else { in_bits };
                        use_near = false;
                    }
                }
                if use_near {
                    for k in 0..nv {
                        if !neg_bit(edge_dot(e.edges[k], near)) {
                            out_bits |= bit(k);
                        }
                    }
                }
                if out_bits != 0 {
                    let mut best = 100000.0f32;
                    let (mut bdir, mut boff, mut ba) = ([0.0f32; 3], [0.0f32; 3], [0.0f32; 3]);
                    for k in 0..nv {
                        if out_bits & bit(k) == 0 {
                            continue;
                        }
                        let j = if e.nverts as usize == k + 1 || k + 1 >= nv { 0 } else { k + 1 };
                        let a = sub(e.verts[j], e.verts[k]);
                        let off = sub(c, e.verts[k]);
                        let din = EFGetVectorVectorDistanceIn { a, b: wp, off, maxdist: 0.0 };
                        let mut dout = EFGetVectorVectorDistanceOut::default();
                        EFObjCollisionPrim::GetVectorVectorDistance(&din, &mut dout);
                        let v = if in_bits & bit(k) == 0 {
                            dout.dist + sr
                        } else {
                            let v = sr - dout.dist;
                            if v < 0.0 {
                                ok = false;
                                break;
                            }
                            v
                        };
                        if v < best {
                            bdir = dout.dir;
                            boff = off;
                            ba = a;
                            best = v;
                        }
                    }
                    if ok {
                        if out_bits == 0xffff_ffff {
                            if best < o.depth {
                                o.depth = best;
                                o.normal = bdir;
                            }
                        } else {
                            let nin = EFVectorVectorNearestPointIn { a: ba, b: add(wp, wp), off: sub(boff, wp), maxdist: sr };
                            let mut nout = EFVectorVectorNearestPointOut::default();
                            if EFObjCollisionPrim::GetVectorVectorNearestPoint(&nin, &mut nout) & 4 == 0 {
                                if sr - nout.dist < o.depth {
                                    o.depth = sr - nout.dist;
                                    o.normal = nout.dir;
                                }
                            } else {
                                ok = false;
                            }
                        }
                    }
                }
                if ok {
                    if flags != 0 && mat0 & 1 == 0 {
                        let k = dot(n, o.normal);
                        if k < 0.0 {
                            // single-sided: drop the component against the plane normal
                            o.normal = sub(o.normal, scale(n, k));
                            o.normal = scale(o.normal, o.depth);
                            let l = length(o.normal);
                            o.depth = l;
                            if l <= 0.001 {
                                return 0;
                            }
                            o.normal = scale(o.normal, 1.0 / l);
                        }
                    }
                    return 2;
                }
                // not overlapping after all: fall through to the edge sweep
            } else {
                let mv = dot(i.motion, n);
                if !sign_differs(pd, mv) {
                    return 0;
                }
                let t = ((fo as f64) / (mv as f64).abs()) as f32;
                if i.tmax < t {
                    return 4;
                }
                o.motion = scale(i.motion, t);
                let x = add(add(c, wp), o.motion);
                for k in 0..nv {
                    if !neg_bit(edge_dot(e.edges[k], x)) {
                        out_bits |= bit(k);
                    }
                }
                if out_bits == 0 {
                    o.t = t;
                    o.normal = if 0.0 < pd { n } else { neg(n) };
                    if flags != 0 && mat0 & 1 == 0 && dot(o.normal, n) < 0.0 {
                        return 0;
                    }
                    return 1;
                }
            }
            // polygon edges vs the capsule axis
            if !(0.0001 <= dot(i.motion, i.motion)) || out_bits == 0 {
                return 0;
            }
            let mut best_flags = 0u32; // uStack_484
            let (mut ba, mut boff, mut bnorm) = ([0.0f32; 3], [0.0f32; 3], [0.0f32; 3]);
            let mut bt = 0.0f32; // quirk: fStack_2a0 uninitialised when no edge is accepted
            let mut iout = EFCheckInfiniteVectorVectorCollisionOut::default();
            for k in 0..nv {
                if out_bits & bit(k) == 0 {
                    continue;
                }
                let j = if e.nverts as usize == k + 1 || k + 1 >= nv { 0 } else { k + 1 };
                let off = sub(c, e.verts[k]);
                let a = sub(e.verts[j], e.verts[k]);
                let iin = EFCheckInfiniteVectorVectorCollisionIn { a, b: w, off, motion: i.motion, radius: sr };
                let r = EFObjCollisionPrim::CheckInfiniteVectorVectorCollision(&iin, &mut iout);
                if r & 0x30 != 0 {
                    return 0;
                }
                // quirk: constant -10000 compare as in CheckBBoxCollision (last hit wins)
                if (r & 8 != 0 && best_flags & 1 == 0) || (r & 1 != 0 && -10000.0 < iout.t) {
                    boff = off;
                    ba = a;
                    bnorm = iout.normal;
                    best_flags = r;
                    if r & 8 == 0 {
                        if i.tmax < iout.t {
                            return 0;
                        }
                        bt = iout.t;
                    } else {
                        bt = 0.0;
                    }
                }
            }
            let fin = EFCheckFiniteVectorVectorCollisionIn {
                a: ba,
                b: add(w, w),
                off: sub(boff, w),
                motion: i.motion,
                radius: sr,
                normal: bnorm,
                t: bt,
                tmax: i.tmax,
            };
            let mut fout = EFCheckFiniteVectorVectorCollisionOut::default();
            if EFObjCollisionPrim::CheckFiniteVectorVectorCollision(&fin, &mut fout, best_flags) & 1 == 0 {
                return 0;
            }
            o.t = fout.t;
            o.normal = fout.normal;
            o.motion = scale(i.motion, fout.t);
            if flags == 0 || mat0 & 1 != 0 || 0.0 <= dot(o.normal, n) {
                1
            } else {
                0
            }
        }

        /// EFObjCollisionSweptSphere::CheckSweptSphereCollision (0x001d7690), return values resolved
        /// from the disassembly (the existing objcoll_check port guessed them):
        /// - 0x001d786c..0x001d78a4: NearestPoint & 0x400 -> depth/normal written, `v0 = 2`, branch to
        ///   the epilogue 0x001d7a98 (returns immediately: no sweep test).
        /// - 0x001d79a0: Infinite & 9 == 0 -> 0x001d7a94 `move v0, zero`.
        /// - 0x001d79a8..0x001d79cc: Infinite & 1 and !(t <= tmax) (`c.ole.s t, tmax`) -> `v0 = 0`.
        /// - 0x001d7a0c: Finite & 1 == 0 -> v0 = 0; else t/normal/motion written, `v0 = 1` (0x001d7a78).
        pub fn Swept_CheckSweptSphereCollision(&self, i: &EFCheckCollisionIn, o: &mut EFCheckCollisionOut, other: &ObjPrim) -> u32 {
            let (wa, ra) = self.swept_parts();
            let (wb, rb) = other.swept_parts();
            let rsum = ra + rb;
            let start_b = sub(other.base.center, wb);
            let start_a = sub(self.base.center, wa);
            let off = sub(start_a, start_b);
            let va = add(wb, wb);
            let vb = add(wa, wa);
            let nin = EFVectorVectorNearestPointIn { a: va, b: vb, off, maxdist: rsum };
            let mut nout = EFVectorVectorNearestPointOut::default();
            if EFObjCollisionPrim::GetVectorVectorNearestPoint(&nin, &mut nout) & 0x400 != 0 {
                o.depth = rsum - nout.dist;
                o.normal = nout.dir;
                return 2;
            }
            let iin = EFCheckInfiniteVectorVectorCollisionIn { a: va, b: vb, off, motion: i.motion, radius: rsum };
            let mut iout = EFCheckInfiniteVectorVectorCollisionOut::default();
            let r = EFObjCollisionPrim::CheckInfiniteVectorVectorCollision(&iin, &mut iout);
            if r & 9 == 0 {
                return 0;
            }
            if r & 1 != 0 && i.tmax < iout.t {
                return 0;
            }
            let fin = EFCheckFiniteVectorVectorCollisionIn {
                a: va,
                b: vb,
                off,
                motion: i.motion,
                radius: rsum,
                normal: iout.normal,
                t: iout.t,
                tmax: i.tmax,
            };
            let mut fout = EFCheckFiniteVectorVectorCollisionOut::default();
            if EFObjCollisionPrim::CheckFiniteVectorVectorCollision(&fin, &mut fout, r) & 1 != 0 {
                o.t = fout.t;
                o.normal = fout.normal;
                o.motion = scale(i.motion, fout.t);
                1
            } else {
                0
            }
        }

        /// EFObjCollisionSweptSphere::RayIntersect (0x001d7f00): CheckSphereSweepCollision of the ray
        /// origin (radius sradius) against the axis, tmax = `*t`. Writes only out+0x10 normal; on an
        /// overlap (8) the normal is -normalize(dir) and t = 0.
        pub fn Swept_RayIntersect(&self, rin: &RayIntersectIn, out: &mut EFRayIntersectOut, t: &mut f32) -> u32 {
            let (w, sr) = self.swept_parts();
            let sin = EFCheckSphereSweepCollisionIn {
                vec: add(w, w),
                center: add(sub(rin.origin, self.base.center), w),
                motion: rin.dir,
                radius: sr,
                tmax: *t,
            };
            let mut sout = EFCheckSphereSweepCollisionOut::default();
            let r = EFObjCollisionPrim::CheckSphereSweepCollision(&sin, &mut sout);
            if r & 9 == 0 {
                return 0;
            }
            if r & 8 == 0 {
                *t = sout.t;
            } else {
                sout.normal = neg(rin.dir);
                normalize(&mut sout.normal);
                *t = 0.0;
            }
            out.hit.normal = sout.normal;
            1
        }

        /// EFObjCollisionSweptSphere::CheckPointIntersection (0x001d8430).
        pub fn Swept_CheckPointIntersection(&self, p: V3) -> bool {
            let (w, sr) = self.swept_parts();
            let mut d = sub(self.base.center, p);
            if self.base.radius * self.base.radius < dot(d, d) {
                return false;
            }
            let r2 = sr * sr;
            let s = dot(d, w) / dot(w, w);
            let q = [d[0] - w[0] * s, d[1] - w[1] * s, d[2] - w[2] * s];
            if r2 < dot(q, q) {
                return false;
            }
            if s < -1.0 {
                d = add(d, w);
            } else {
                if s <= 1.0 {
                    return true;
                }
                d = sub(d, w);
            }
            !(r2 < dot(d, d))
        }

        /// EFObjCollisionSweptSphere::CheckParticlesIntersection (0x001d7bb0) over (pos +4, radius
        /// +0x20, u16 stamp +0x28) in list order. quirk: uses the bounding radius (+0xbc), not sradius.
        pub fn Swept_CheckParticlesIntersection(&self, parts: &mut [(V3, f32, u16)], mark: bool, global: u16) -> i32 {
            let (w, _) = self.swept_parts();
            let ww = dot(w, w);
            let mut n = 0;
            for (pos, pr, stamp) in parts.iter_mut() {
                if *stamp == global {
                    continue;
                }
                let d = sub(*pos, self.base.center);
                let cr = cross(d, w);
                let rr = (self.base.radius + *pr) * (self.base.radius + *pr);
                if dot(cr, cr) <= ww * rr {
                    let pj = dot(d, w);
                    if pj < -ww {
                        let e = add(d, w);
                        if rr < dot(e, e) {
                            continue;
                        }
                    } else if ww < pj {
                        let e = sub(d, w);
                        if rr < dot(e, e) {
                            continue;
                        }
                    }
                    n += 1;
                    if !mark {
                        return n;
                    }
                    *stamp = global;
                }
            }
            n
        }

        /// EFObjCollisionSweptSphere::GetEscapeNoPF (0x001d4940): horizontal direction perpendicular to
        /// the axis, on the side of `p`. quirk: the random direction is taken when |d·w| >= |w|^2
        /// (beyond the axis ends), and a random side (`rand() & 1`) when |cross_y| <= 0.01*|w.xz|^2.
        pub fn Swept_GetEscapeNoPF(&self, p: V3, out: &mut V3, dir: V3, rand: &mut dyn FnMut() -> u32) {
            let (w, _) = self.swept_parts();
            let d = sub(p, self.base.center);
            let pj = dot(d, w);
            let ww = w[2] * w[2] + w[0] * w[0] + w[1] * w[1];
            if !((pj as f64).abs() < ww as f64) {
                let a = (rand() & 0xff) as f32 * 0.00390625 * 6.2831855;
                *out = [a.sin(), 0.0, a.cos()];
            } else {
                let cy = d[0] * w[2] - d[2] * w[0];
                let h = w[2] * w[2] + w[0] * w[0];
                let side = if !((cy as f64).abs() > h as f64 * 0.01) {
                    rand() & 1
                } else if cy < 0.0 {
                    1
                } else {
                    0
                };
                *out = if side == 0 { [w[2], 0.0, -w[0]] } else { [-w[2], 0.0, w[0]] };
                let s = 1.0 / h.sqrt();
                *out = scale(*out, s);
            }
        }
    }

}

// ================================================================================================
#[allow(unused_variables, unused_imports)]
pub mod bbox {
    // WP-E5 part: EFObjCollisionBBox + EFObjCollisionPolygon::RayIntersect (merged into objcoll_check.rs at the end)
    //
    // Unmapped fields: EFObjCollisionPolygon +0xe8 (u32 primitive count) and +0xec (pointer to a packed
    // array of EFEnvCollisionPrim in polygon-local space, stride `0x1c + raw_nverts*0x1c`) are not in
    // `PrimExt::Polygon`. The `*Prims` variants take them explicitly; the vtable-signature versions
    // pass an empty list until the model carries them.
    use crate::engine::physic::collision_map::{EFRayIntersectOut, RayIntersectIn};
    use crate::engine::physic::model::{ObjPrim, PrimExt};
    use super::*;
    use crate::engine::physic::objcoll_prim::*;

    type M4 = [[f32; 4]; 4];

    /// EFCheckBBBBCollision (stack block of CheckBBoxCollision): +0 motion, +0xc normal, +0x18 tmax
    /// (exit t), +0x1c enter t, +0x20 depth, +0x24 flags (bit1 = still overlapping, 1 = hit).
    #[derive(Debug, Clone, Copy, Default)]
    pub struct EFCheckBBBBCollision {
        pub motion: V3,
        pub normal: V3,
        pub tmax: f32,
        pub tenter: f32,
        pub depth: f32,
        pub flags: u32,
    }

    /// fabs sum done with dpadd in the C++: (|z| + |x|) + |y| in double, back to float.
    #[inline]
    fn abs_sum(v: V3) -> f32 {
        ((v[2].abs() as f64 + v[0].abs() as f64) + v[1].abs() as f64) as f32
    }
    /// RwMatrixMultiply(dst, a, b) = a * b (row-vector convention; 3x3 + pos).
    fn mat_mul(a: &M4, b: &M4) -> M4 {
        let mut r = IDENTITY;
        for i in 0..4 {
            for j in 0..3 {
                r[i][j] = a[i][0] * b[0][j] + a[i][1] * b[1][j] + a[i][2] * b[2][j];
            }
            if i == 3 {
                for j in 0..3 {
                    r[3][j] += b[3][j];
                }
            }
        }
        r
    }
    const F001: f64 = 0.001f32 as f64; // 0x3f50624de0000000
    const F0001: f64 = 0.0001f32 as f64; // 0x3f1a36e2e0000000

    impl ObjPrim {
        /// EFObjCollisionBBox::CheckBBPtBBCollision (0x001cc8b0): this box's center (with this box's
        /// extents projected) against `other`'s box slabs. Returns 0/1/2 or 0x10 (miss) / 0x20 (behind).
        pub fn BBox_CheckBBPtBBCollision(&self, c: &mut EFCheckBBBBCollision, other: &ObjPrim) -> u32 {
            let (PrimExt::BBox { tmp, .. }, PrimExt::BBox { half: oh, m: om, inv: oinv, .. }) = (&self.ext, &other.ext) else { return 0 };
            let mut res = 0u32;
            let p = transform_point(oinv, self.base.center);
            let mv = transform_vector(oinv, c.motion);
            let r = mat_mul(om, tmp);
            for a in 0..3 {
                let ext = abs_sum([r[a][0], r[a][1], r[a][2]]);
                let pa = p[a];
                let d = ((pa.abs() as f64 - ext as f64) - oh[a] as f64) as f32;
                let va = mv[a];
                let n = if 0.0 < pa { row(om, a) } else { neg(row(om, a)) };
                if d < 0.0 {
                    if c.flags & 2 != 0 && -d < c.depth {
                        res = 2;
                        c.depth = -d;
                        c.normal = n;
                    }
                } else {
                    res &= !2;
                    c.flags &= !2;
                    if (va.abs() as f64) < F001 {
                        return 0x10;
                    }
                    if !sign_differs(va, pa) {
                        return 0x20;
                    }
                    let t = (d as f64 / va.abs() as f64) as f32;
                    if c.tenter < t {
                        if c.tmax < t {
                            return 0x10;
                        }
                        c.tenter = t;
                        res = 1;
                        c.flags = 1;
                        c.normal = n;
                    }
                }
                if va.abs() as f64 > F0001 {
                    let x = if va < 0.0 { (oh[a] + pa + ext) / -va } else { (oh[a] + -pa + ext) / va };
                    if x < c.tmax {
                        if x < 0.0 {
                            return 0x20;
                        }
                        c.tmax = x;
                        if x < c.tenter {
                            return 0x10;
                        }
                    }
                }
            }
            res
        }

        /// EFObjCollisionBBox::CheckBBoxCollision (0x001ccdf0): SAT, 3+3 face axes through
        /// CheckBBPtBBCollision both ways, then the 9 edge-cross axes.
        /// quirk: the C++ first calls `b->PrecalcPrimData()` (vtable+0xc), mutating the other prim; here
        /// it is applied to a copy (the caller's prim keeps its data).
        /// quirk: returns the shared flags word, which can be 1 (left by CheckBBPtBBCollision) even when t > exit t.
        pub fn BBox_CheckBBoxCollision(&self, i: &EFCheckCollisionIn, o: &mut EFCheckCollisionOut, b: &ObjPrim) -> u32 {
            let (PrimExt::BBox { m: am, tmp: at, .. }, PrimExt::BBox { .. }) = (&self.ext, &b.ext) else { return 0 };
            let mut c = EFCheckBBBBCollision { motion: i.motion, normal: [0.0; 3], tmax: i.tmax, tenter: -100000.0, depth: 100000.0, flags: 2 };
            let mut b = b.clone();
            b.PrecalcPrimData();
            let PrimExt::BBox { m: bm, tmp: bt, .. } = &b.ext else { return 0 };
            if self.BBox_CheckBBPtBBCollision(&mut c, &b) & 0x34 != 0 {
                return 0;
            }
            c.motion = neg(c.motion);
            let r = b.BBox_CheckBBPtBBCollision(&mut c, self);
            if r & 0x34 != 0 {
                return 0;
            }
            if r & 3 != 0 {
                c.normal = neg(c.normal);
            }
            let rel = sub(self.base.center, b.base.center);
            for ia in 0..3 {
                let pa = row(am, ia);
                for ib in 0..3 {
                    let pb = row(bm, ib);
                    let mut n = cross(pa, pb);
                    let l2 = dot(n, n);
                    if l2 as f64 > 0.001f64 {
                        // 0x3f50624dd2f1a9fc
                        n = scale(n, 1.0 / l2.sqrt());
                        let mut dist = dot(rel, n);
                        if dist < 0.0 {
                            dist = -dist;
                            n = neg(n);
                        }
                        let ext = abs_sum(transform_vector(at, n)) + abs_sum(transform_vector(bt, n));
                        let sep = dist - ext;
                        let vel = dot(i.motion, n);
                        if sep < 0.0 {
                            if c.flags & 2 != 0 && -sep < c.depth {
                                c.normal = n;
                                c.depth = -sep;
                            }
                        } else {
                            if (vel.abs() as f64) < 0.001f64 {
                                return 0;
                            }
                            if 0.0 < vel {
                                return 0;
                            }
                            let t = sep / -vel;
                            c.flags = 0;
                            if c.tenter < t {
                                if c.tmax < t {
                                    return 0;
                                }
                                c.normal = n;
                                c.tenter = t;
                            }
                        }
                        if vel.abs() as f64 > F0001 {
                            let x = if vel < 0.0 { (dist + ext) / -vel } else { (-dist + ext) / vel };
                            if x < c.tmax {
                                if x < 0.0 {
                                    return 0;
                                }
                                c.tmax = x;
                                if x < c.tenter {
                                    return 0;
                                }
                            }
                        }
                    }
                }
            }
            o.t = c.tenter;
            o.normal = c.normal;
            o.depth = c.depth;
            if c.flags & 2 == 0 {
                let t = o.t;
                if t <= c.tmax {
                    o.motion = scale(i.motion, t);
                    // C++: DTVector3d::ToString(motion) when dot(motion, normal) >= 0 (debug, no effect)
                    c.flags = 1;
                }
            }
            c.flags
        }

        /// EFObjCollisionBBox::CheckPolygonCollision (0x001ce340) : prims = PrimExt::Polygon::prims
        /// (+0xe8 count, +0xec array), see `BBox_CheckPolygonCollisionPrims`.
        pub fn BBox_CheckPolygonCollision(&self, i: &EFCheckCollisionIn, o: &mut EFCheckCollisionOut, p: &ObjPrim) -> u32 {
            let PrimExt::Polygon { m, .. } = &p.ext else { return 0 };
            self.BBox_CheckPolygonCollisionPrims(i, o, m, poly_prims(p))
        }

        /// EFObjCollisionBBox::CheckPolygonCollision (0x001ce340): each polygon primitive is moved to
        /// world space with the polygon's +0xe0 matrix (plane normal by vector transform, d = -(n'.v0'),
        /// raw-nverts points into the 0x01f6f370 scratch) and tested with CheckSinglePolygonCollision(...,0);
        /// keeps the result with the smallest t (start tmax+1).
        /// quirk: an overlap (2) carries t = -100000 from CheckSinglePolygonCollision, so it always wins.
        /// quirk: the saved depth/t are uninitialised stack when nothing hit; written here as 0 (the whole
        /// Out block is always overwritten).
        pub fn BBox_CheckPolygonCollisionPrims(&self, i: &EFCheckCollisionIn, o: &mut EFCheckCollisionOut, m: &M4, prims: &[EnvPrim]) -> u32 {
            let mut res = 0u32;
            let mut saved = EFCheckCollisionOut::default();
            let mut best = i.tmax + 1.0;
            for e in prims {
                let n = transform_vector(m, e.normal);
                let nv = (e.nverts as usize).min(e.verts.len()); // quirk: raw u16 count (bit 15 not masked)
                let vs: Vec<V3> = e.verts[..nv].iter().map(|v| transform_point(m, *v)).collect();
                let v0 = vs.first().copied().unwrap_or([0.0; 3]); // C++ reads the stale scratch when nverts==0
                let plane = [n[0], n[1], n[2], -dot(n, v0)];
                let r = self.BBox_CheckSinglePolygonCollision(i, o, e, Some(&plane), Some(&vs), 0);
                if r != 0 && r & 3 != 0 && o.t < best {
                    saved = *o;
                    res = r;
                    best = o.t;
                }
            }
            *o = saved;
            res
        }

        /// EFObjCollisionBBox::CheckSinglePolygonCollision (0x001ce580). `flags != 0`: plane/vertices
        /// from `e` (world) and material word0 bit0 = double sided, bit1 = one-way; else `plane`/`verts`
        /// (the caller's world copies) and double sided. Returns 0 / 1 (hit at t) / 2 (overlap).
        /// quirk: the reject test is `(r<dist && r<end) || (dist<-r && r<end)` (r = +0xbc radius).
        /// quirk: the box-axis pass uses the WORLD motion components against box-local slabs.
        /// quirk: the "flip normal" flag of the box-axis pass is sticky across axes.
        /// quirk: the edge-cross axes never update the normal.
        pub fn BBox_CheckSinglePolygonCollision(
            &self,
            i: &EFCheckCollisionIn,
            o: &mut EFCheckCollisionOut,
            e: &EnvPrim,
            plane: Option<&[f32; 4]>,
            verts: Option<&[V3]>,
            flags: u32,
        ) -> u32 {
            let PrimExt::BBox { half, m, inv, tmp } = &self.ext else { return 0 };
            let mut state = 2u32; // iVar23
            let mut one_way = 0u32;
            let mut dbl = 1u32;
            let (pn, pd, verts): (V3, f32, &[V3]) = if flags != 0 {
                dbl = e.mat[0] & 1;
                one_way = e.mat[0] & 2;
                (e.normal, e.d, &e.verts)
            } else {
                let (Some(p), Some(v)) = (plane, verts) else { return 0 }; // NULL deref in the C++
                ([p[0], p[1], p[2]], p[3], v)
            };
            // quirk: loop count is the raw u16 nverts
            let nv = (e.nverts as usize).min(verts.len());
            let l = length(pn);
            if l <= 0.99 || !(length(pn) < 1.01) {
                return 0;
            }
            let mt = scale(i.motion, i.tmax);
            let mn = dot(mt, pn);
            if one_way != 0 && !(mn <= 0.0) {
                return 0;
            }
            let mut texit = i.tmax; // fVar27
            let mut dist = pd + dot(self.base.center, pn);
            let r = self.base.radius;
            let end = dist + mn;
            if (r < dist && r < end) || (dist < -r && r < end) {
                return 0;
            }
            o.t = -100000.0; // 0xc7c35000
            let ext = abs_sum(transform_vector(tmp, pn));
            if 0.0 < dist {
                o.normal = pn;
            } else {
                dist = -dist;
                o.normal = neg(pn);
            }
            let mut sep = dist - ext;
            let vel = dot(i.motion, o.normal);
            if (sep.abs() as f64) < F001 {
                sep = 0.0;
            }
            if sep < 0.0 {
                o.depth = -sep;
            } else {
                if 0.0 <= vel {
                    return 0;
                }
                o.t = -sep / vel;
                state = 0;
                if i.tmax < -sep / vel {
                    return 0;
                }
            }
            if vel.abs() as f64 > F0001 {
                texit = if 0.0 < vel { sep / vel } else { (dist + ext) / -vel };
            }
            if texit < o.t {
                return 0;
            }
            // polygon AABB in box space (DTMatrix::TransformPoints(+0xf0, verts, stack, raw nverts))
            let lv: Vec<V3> = verts[..nv].iter().map(|v| transform_point(inv, *v)).collect();
            let mut mn3 = [10000.0f32; 3];
            let mut mx3 = [-10000.0f32; 3];
            for v in &lv {
                for a in 0..3 {
                    if mx3[a] < v[a] {
                        mx3[a] = v[a];
                    }
                    if v[a] < mn3[a] {
                        mn3[a] = v[a];
                    }
                }
            }
            let ah = scale(sub(mx3, mn3), 0.5);
            let ac = add(mn3, ah);
            let mut set_n = false; // bVar1
            let mut flip = false; // bVar2
            let mut best = 0usize;
            for a in 0..3 {
                let s = ((half[a] + ah[a]) as f64 - ac[a].abs() as f64) as f32;
                let mv = i.motion[a];
                if 0.0 < s {
                    if state != 0 && s < o.depth {
                        set_n = true;
                        if 0.0 < ac[a] {
                            flip = true;
                        }
                        o.depth = s;
                        best = a;
                    }
                } else {
                    if sign_differs(ac[a], mv) {
                        return 0;
                    }
                    if (mv.abs() as f64) < 0.0001f64 {
                        // 0x3f1a36e2eb1c432d
                        return 0;
                    }
                    let t = ((-s) as f64 / mv.abs() as f64) as f32;
                    if o.t < t {
                        if i.tmax < t {
                            return 0;
                        }
                        if texit < t {
                            return 0;
                        }
                        o.t = t;
                        set_n = true;
                        best = a;
                        if 0.0 < ac[a] {
                            flip = true;
                        }
                    }
                    state = 0;
                }
                if mv.abs() as f64 > F0001 {
                    let x = (if 0.0 < mv { half[a] + ac[a] + ah[a] } else { (ac[a] - half[a]) - ah[a] }) / mv;
                    if x < texit {
                        texit = x;
                        if x < o.t {
                            return 0;
                        }
                    }
                }
            }
            if set_n {
                o.normal = if flip { neg(row(m, best)) } else { row(m, best) };
            }
            // polygon edges x box axes
            for k in 0..nv {
                let k1 = if k + 1 == nv { 0 } else { k + 1 };
                let k2 = if k1 + 1 == nv { 0 } else { k1 + 1 };
                let ed = sub(verts[k1], verts[k]);
                let ed2 = sub(verts[k2], verts[k1]);
                let rel = sub(self.base.center, verts[k]);
                for a in 0..3 {
                    let mut n = cross(ed, row(m, a));
                    let l2 = dot(n, n);
                    if 0.0001 < l2 {
                        n = scale(n, 1.0 / l2.sqrt());
                        let pr = dot(n, ed2);
                        if pr.abs() as f64 > 0.01f32 as f64 {
                            if 0.0 < pr {
                                n = neg(n);
                            }
                            let v = dot(n, i.motion);
                            let ex = abs_sum(transform_vector(tmp, n));
                            let proj = dot(rel, n);
                            let s = ex - proj;
                            if 0.0 < s {
                                if state != 0 && s < o.depth {
                                    o.depth = s;
                                }
                            } else {
                                if 0.0 <= v {
                                    return 0;
                                }
                                if (v.abs() as f64) < F0001 {
                                    return 0;
                                }
                                let t = s / v;
                                state = 0;
                                if o.t < t {
                                    o.t = t;
                                    if i.tmax < t {
                                        return 0;
                                    }
                                }
                            }
                            if v.abs() as f64 > F0001 {
                                let x = if 0.0 < v { s / v } else { (ex + proj) / -v };
                                if 0.0 <= x && x < texit {
                                    texit = x;
                                    if x < o.t {
                                        return 0;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if state == 0 {
                let t = o.t;
                if t <= texit {
                    o.motion = scale(i.motion, t);
                    if dbl == 0 {
                        return (0.0 <= dot(o.normal, pn)) as u32;
                    }
                    return 1;
                }
                0
            } else {
                let mut ret = state;
                if dbl == 0 {
                    let f = dot(pn, o.normal);
                    if f < 0.0 {
                        o.normal = sub(o.normal, scale(pn, f));
                        o.normal = scale(o.normal, o.depth);
                        o.depth = length(o.normal);
                        if o.depth <= 0.001 {
                            ret = 0;
                        } else {
                            o.normal = scale(o.normal, 1.0 / o.depth);
                        }
                    }
                }
                ret
            }
        }

        /// EFObjCollisionBBox::CheckEnvCollision (0x001cda40): CheckSinglePolygonCollision(i, o, e, NULL, NULL, 1).
        pub fn BBox_CheckEnvCollision(&self, i: &EFCheckCollisionIn, o: &mut EFCheckCollisionOut, e: &EnvPrim) -> u32 {
            self.BBox_CheckSinglePolygonCollision(i, o, e, None, None, 1)
        }

        /// EFObjCollisionBBox::RayIntersect (0x001cdf80): slab test in box space (+0xf0) for t in
        /// [0, *t]; writes the face normal (+0xec row, out+0x10) and *t = enter t. Hit position, prim and
        /// callback are not touched.
        pub fn BBox_RayIntersect(&self, rin: &RayIntersectIn, out: &mut EFRayIntersectOut, t: &mut f32) -> u32 {
            let PrimExt::BBox { half, m, inv, .. } = &self.ext else { return 0 };
            let p = transform_point(inv, rin.origin);
            let d = transform_vector(inv, rin.dir);
            let mut texit = *t;
            let mut inside = -10000.0f32;
            let mut ax = 0usize;
            let mut tenter = 0.0f32;
            for a in 0..3 {
                let c = p[a];
                let h = half[a];
                let dv = d[a];
                let g = (c.abs() as f64 - h as f64) as f32;
                if 0.0 < g {
                    if !sign_differs(c, dv) || dv == 0.0 {
                        return 0;
                    }
                    let tt = (g as f64 / dv.abs() as f64) as f32;
                    if tenter < tt {
                        tenter = tt;
                        ax = a;
                        if texit < tt {
                            return 0;
                        }
                    }
                } else if inside < g && tenter == 0.0 {
                    inside = g;
                    ax = a;
                }
                if dv != 0.0 {
                    let x = (if dv.to_bits() & 0x8000_0000 == 0 { -c + h } else { -(c + h) }) / dv;
                    if x < texit {
                        texit = x;
                        if x < tenter {
                            return 0;
                        }
                    }
                }
            }
            out.hit.normal = if p[ax].to_bits() & 0x8000_0000 == 0 { row(m, ax) } else { neg(row(m, ax)) };
            *t = tenter;
            1
        }

        /// EFObjCollisionBBox::GetEscapeNoPF (0x001cc510): `p` = position (box frame LTM, not +0xec), `out`
        /// = escape direction; `dir`/`rand` unused. The LTM axis with the largest |y| is ignored; with two
        /// other axes outside the box the diagonal (normalized, LTM-rotated) is used, else the face axis
        /// with the largest outside distance (sign of the local position).
        pub fn BBox_GetEscapeNoPF(&self, p: V3, out: &mut V3, dir: V3, rand: &mut dyn FnMut() -> u32) {
            let PrimExt::BBox { half, .. } = &self.ext else { return };
            let ltm = &self.base.ltm;
            let mut up = 0usize;
            let mut best = 0.0f32;
            for a in 0..3 {
                let v = (ltm[a][1] as f64).abs() as f32;
                if best < v {
                    best = v;
                    up = a;
                }
            }
            let mut d = [0.0f32; 3];
            let q = transform_point(&crate::engine::physic::model::RwMatrixInvert(ltm), p);
            let mut count = 0;
            let mut maxd = -10000.0f32;
            let mut ax = 0usize;
            for a in 0..3 {
                if a == up {
                    continue;
                }
                let g = (q[a].abs() as f64 - half[a] as f64) as f32;
                if 0.0 < g {
                    d[a] = if 0.0 < q[a] { g } else { -g };
                    count += 1;
                }
                if maxd < g {
                    maxd = g;
                    ax = a;
                }
            }
            if count == 2 {
                normalize(&mut d);
                *out = transform_vector(ltm, d);
            } else if 0.0 < q[ax] {
                *out = row(ltm, ax);
            } else {
                *out = neg(row(ltm, ax));
            }
        }

        /// EFObjCollisionPolygon::RayIntersect (0x002713b0) over PrimExt::Polygon::prims (+0xe8/+0xec).
        pub fn Polygon_RayIntersect(&self, rin: &RayIntersectIn, out: &mut EFRayIntersectOut, t: &mut f32) -> u32 {
            self.Polygon_RayIntersectPrims(poly_prims(self), rin, out, t)
        }

        /// EFObjCollisionPolygon::RayIntersect (0x002713b0): ray to polygon space (+0xe4), nearest
        /// front-or-back plane hit with 0 < t <= 1 inside all edge planes (<= 0); normal by +0xe0.
        /// quirk: the input *t is ignored (range is [0,1], best starts at 1.01); the hit position
        /// (out+4) is written from a never-filled zeroed stack vector -> (0,0,0) (disasm 0x2717ac..c0
        /// copies sp+0xe4.., only zeroed at 0x27149c..b4); callback/prim untouched.
        pub fn Polygon_RayIntersectPrims(&self, prims: &[EnvPrim], rin: &RayIntersectIn, out: &mut EFRayIntersectOut, t: &mut f32) -> u32 {
            let PrimExt::Polygon { m, inv, .. } = &self.ext else { return 0 };
            let p = transform_point(inv, rin.origin);
            let d = transform_vector(inv, rin.dir);
            let mut best = 1.01f32;
            let mut normal = [0.0f32; 3];
            for e in prims {
                let den = dot(e.normal, d);
                if den != 0.0 {
                    let tt = (dot(e.normal, p) + e.d) / -den;
                    if 0.0 < tt && tt <= 1.0 && tt < best {
                        let hp = add(p, scale(d, tt));
                        let n = (e.nverts as usize).min(e.edges.len()); // quirk: raw u16 count
                        if e.edges[..n].iter().all(|q| q[3] + dot(hp, [q[0], q[1], q[2]]) <= 0.0) {
                            normal = transform_vector(m, e.normal);
                            best = tt;
                        }
                    }
                }
            }
            if best != 1.01 {
                out.hit.normal = normal;
                out.hit.pos = [0.0; 3];
                *t = best;
                1
            } else {
                0
            }
        }
    }

}
pub use bbox::EFCheckBBBBCollision;

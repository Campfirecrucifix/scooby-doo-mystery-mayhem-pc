//! EFObjCollisionPrim (Classes/EF/EFObjCollisionPrim.cpp): base of the object collision primitives
//! (sphere / swept sphere / bbox / polygon) plus the static geometric helpers every collision test
//! uses. The `EF*In`/`EF*Out` parameter blocks are placeholder structs in the decomp; their fields are
//! named here from usage with the original offsets in doc comments.
//!
//! Result bit flags (shared by all helpers): 1 hit, 4 too far, 8 already inside/overlapping at t=0,
//! 0x10 no hit (miss / parallel), 0x20 hit behind (t<0), 0x40 before segment start, 0x80 after
//! segment end, 0x100 within the segment, 0x200 parallel, 0x400 near enough, 0x800 nearest = start
//! of 2nd segment, 0x1000 = end of 2nd segment, 0x2000 = inside 2nd segment.
#![allow(non_snake_case)]

pub type V3 = [f32; 3];

// ---- DTVector3d / RwV3d helpers (RenderWare replaced: plain f32 math, same evaluation order) ----
#[inline]
pub fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
#[inline]
pub fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
#[inline]
pub fn scale(a: V3, s: f32) -> V3 {
    [a[0] * s, a[1] * s, a[2] * s]
}
#[inline]
pub fn neg(a: V3) -> V3 {
    [-a[0], -a[1], -a[2]]
}
/// dot in the order the Metrowerks code evaluates it (z*z + x*x + y*y)
#[inline]
pub fn dot(a: V3, b: V3) -> f32 {
    a[2] * b[2] + a[0] * b[0] + a[1] * b[1]
}
#[inline]
pub fn cross(a: V3, b: V3) -> V3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
/// RwV3dLength replacement.
#[inline]
pub fn length(a: V3) -> f32 {
    dot(a, a).sqrt()
}
/// RwV3dNormalize replacement: returns the original length; a zero vector is left unchanged.
#[inline]
pub fn normalize(a: &mut V3) -> f32 {
    let l = length(*a);
    if l > 0.0 {
        let r = 1.0 / l;
        *a = scale(*a, r);
    }
    l
}
#[inline]
fn neg_bit(x: f32) -> bool {
    x.to_bits() & 0x8000_0000 != 0
}

/// EFCheckSphereVectorCollisionIn: segment `vec` (+0) from the origin, sphere center `center`
/// (+0xc, relative to the segment start) moving by `motion` (+0x18), radius (+0x24).
#[derive(Debug, Clone, Copy, Default)]
pub struct EFCheckSphereVectorCollisionIn {
    pub vec: V3,
    pub center: V3,
    pub motion: V3,
    pub radius: f32,
}
/// +0 normal (from the segment to the sphere center), +0xc motion done, +0x18 t.
#[derive(Debug, Clone, Copy, Default)]
pub struct EFCheckSphereVectorCollisionOut {
    pub normal: V3,
    pub motion: V3,
    pub t: f32,
}
/// EFCheckSphereSphereCollisionIn: +0 relative center, +0xc motion, +0x18 combined radius.
#[derive(Debug, Clone, Copy, Default)]
pub struct EFCheckSphereSphereCollisionIn {
    pub center: V3,
    pub motion: V3,
    pub radius: f32,
}
pub type EFCheckSphereSphereCollisionOut = EFCheckSphereVectorCollisionOut;
/// EFCheckSphereSweepCollisionIn: +0 segment, +0xc center, +0x18 motion, +0x24 radius, +0x28 max t.
#[derive(Debug, Clone, Copy, Default)]
pub struct EFCheckSphereSweepCollisionIn {
    pub vec: V3,
    pub center: V3,
    pub motion: V3,
    pub radius: f32,
    pub tmax: f32,
}
/// +0 normal, +0xc t.
#[derive(Debug, Clone, Copy, Default)]
pub struct EFCheckSphereSweepCollisionOut {
    pub normal: V3,
    pub t: f32,
}
/// EFGetVectorVectorDistanceIn / EFVectorVectorNearestPointIn: segments `a` (+0) and `b` (+0xc),
/// `off` (+0x18) = start of b relative to the start of a, (+0x24 NearestPoint only) max distance.
#[derive(Debug, Clone, Copy, Default)]
pub struct EFVectorVectorNearestPointIn {
    pub a: V3,
    pub b: V3,
    pub off: V3,
    pub maxdist: f32,
}
pub type EFGetVectorVectorDistanceIn = EFVectorVectorNearestPointIn;
/// EFGetVectorVectorDistanceOut: +0 distance, +4 direction.
#[derive(Debug, Clone, Copy, Default)]
pub struct EFGetVectorVectorDistanceOut {
    pub dist: f32,
    pub dir: V3,
}
/// EFVectorVectorNearestPointOut: +0 direction (a -> b), +0xc distance.
#[derive(Debug, Clone, Copy, Default)]
pub struct EFVectorVectorNearestPointOut {
    pub dir: V3,
    pub dist: f32,
}
/// EFCheckInfiniteVectorVectorCollisionIn: +0 a, +0xc b, +0x18 off, +0x24 motion of b, +0x30 radius.
#[derive(Debug, Clone, Copy, Default)]
pub struct EFCheckInfiniteVectorVectorCollisionIn {
    pub a: V3,
    pub b: V3,
    pub off: V3,
    pub motion: V3,
    pub radius: f32,
}
pub type EFCheckInfiniteVectorVectorCollisionOut = EFCheckSphereSweepCollisionOut;
/// EFCheckFiniteVectorVectorCollisionIn: +0 a, +0xc b, +0x18 off, +0x24 motion, +0x30 radius,
/// +0x34 normal of the infinite-line contact, +0x40 t of the infinite-line contact, +0x44 max t.
#[derive(Debug, Clone, Copy, Default)]
pub struct EFCheckFiniteVectorVectorCollisionIn {
    pub a: V3,
    pub b: V3,
    pub off: V3,
    pub motion: V3,
    pub radius: f32,
    pub normal: V3,
    pub t: f32,
    pub tmax: f32,
}
pub type EFCheckFiniteVectorVectorCollisionOut = EFCheckSphereSweepCollisionOut;

/// Primitive kind (this+0x4): 0 sphere, 1 swept sphere, 2 bbox, 3 polygon.
pub const PRIM_SPHERE: u32 = 0;
pub const PRIM_SWEPT_SPHERE: u32 = 1;
pub const PRIM_BBOX: u32 = 2;
pub const PRIM_POLYGON: u32 = 3;

/// EFObjCollisionPrim common data. The four 0x2c-byte blocks (+0xc, +0x38, +0x64, +0x90) are the
/// collision flag/mask words; PrecalcData of EFCollisionModel ORs words 4..9 (this+0x10..+0x24).
#[derive(Debug, Clone)]
pub struct EFObjCollisionPrim {
    /// +0x04 kind (PRIM_*)
    pub kind: u32,
    /// +0x08 tag (EFCollisionModel::AddPrimitive / PrimTagToPrimIndex)
    pub tag: u32,
    /// +0x0c..+0xbc as 0x2c words (memset in the ctor); word i = this+0xc+4i.
    pub words: [u32; 44],
    /// +0xbc bounding radius
    pub radius: f32,
    /// +0xc0 world center (FLT_MAX x until PrecalcPrimData)
    pub center: V3,
    /// +0xcc local offset in the frame
    pub offset: V3,
    /// +0xd8
    pub d8: u32,
    /// +0xdc identity (WP-E10): the DTFrame the prim was created on = joint index into the
    /// EF3dObj HAnim hierarchy (0xffffffff = the object's root frame +0x14). `ltm` is resynced
    /// from that frame every collision stage.
    pub joint: u32,
    /// +0xdc frame (DTFrame*) — replaced by the frame's LTM, set by the owner each frame
    pub ltm: [[f32; 4]; 4],
}

impl EFObjCollisionPrim {
    /// EFObjCollisionPrim(DTFrame*). `ltm` = RwFrameGetLTM(frame) (rows right/up/at/pos).
    pub fn new(ltm: [[f32; 4]; 4]) -> Self {
        EFObjCollisionPrim {
            kind: 0,
            tag: 0,
            words: [0; 44],
            radius: 0.0,
            center: [f32::MAX, 0.0, 0.0],
            offset: [0.0; 3],
            d8: 0,
            joint: 0xffff_ffff,
            ltm,
        }
    }
    /// word at byte offset `o` (0xc..0xbc)
    pub fn word(&self, o: usize) -> u32 {
        self.words[(o - 0xc) / 4]
    }
    pub fn set_word(&mut self, o: usize, v: u32) {
        self.words[(o - 0xc) / 4] = v;
    }

    /// Translate(v): center += v.
    pub fn Translate(&mut self, v: V3) {
        self.center = add(self.center, v);
    }

    /// PrecalcPrimData: center = LTM.pos + LTM(rotation) * offset.
    pub fn PrecalcPrimData(&mut self) {
        let m = &self.ltm;
        self.center = [m[3][0], m[3][1], m[3][2]];
        let t = transform_vector(m, self.offset);
        self.center = add(self.center, t);
    }

    /// CheckSphereSweepCollision: sphere moving along `motion` against the segment `vec`
    /// (capsule-like: the segment body, then the end caps via CheckSphereSphereCollision).
    pub fn CheckSphereSweepCollision(i: &EFCheckSphereSweepCollisionIn, o: &mut EFCheckSphereSweepCollisionOut) -> u32 {
        let vin = EFCheckSphereVectorCollisionIn { vec: i.vec, center: i.center, motion: i.motion, radius: i.radius };
        let mut vout = EFCheckSphereVectorCollisionOut::default();
        let mut r = Self::CheckSphereVectorCollision(&vin, &mut vout);
        // quirk: vout.t is compared even when the helper returned early without writing it (0)
        if i.tmax < vout.t {
            return 4;
        }
        if r & 0x30 != 0 {
            return r;
        }
        if r & 0x100 != 0 {
            o.t = vout.t;
            o.normal = vout.normal;
            return 0x101;
        }
        // end caps: the start (0x40) or the end (0x80) of the segment
        let c = if r & 0x80 == 0 { i.center } else { sub(i.center, i.vec) };
        let sin = EFCheckSphereSphereCollisionIn { center: c, motion: i.motion, radius: i.radius };
        let mut sout = EFCheckSphereSphereCollisionOut::default();
        r = Self::CheckSphereSphereCollision(&sin, &mut sout);
        if r & 0x38 == 0 {
            if i.tmax < sout.t {
                r = 4;
            } else {
                o.t = sout.t;
                r = r & 0xc0 | 1;
                o.normal = sout.normal;
            }
        }
        r
    }

    /// CheckInfiniteVectorVectorCollision: line `b` (at `off`, moving by `motion`) against line `a`
    /// with tolerance `radius`.
    pub fn CheckInfiniteVectorVectorCollision(i: &EFCheckInfiniteVectorVectorCollisionIn, o: &mut EFCheckInfiniteVectorVectorCollisionOut) -> u32 {
        let c = cross(i.a, i.b);
        o.normal = c;
        if 1e-5 <= dot(c, c) {
            normalize(&mut o.normal);
            let mut d = dot(i.off, o.normal);
            if neg_bit(d) {
                d = -d;
                o.normal = neg(o.normal);
            }
            let mut r = 8;
            if i.radius <= d {
                let m = dot(o.normal, i.motion);
                // _dpflt(|m|, 0x3f1a36e2e0000000 = 1e-4 as double): |m| < 1e-4 -> no hit (0x10)
                if (m.abs() as f64) < f64::from_bits(0x3f1a36e2e0000000) {
                    r = 0x10;
                } else {
                    r = 0x20;
                    if m <= 0.0 {
                        r = 1;
                        o.t = (d - i.radius) / -m;
                    }
                }
            }
            r
        } else {
            // parallel lines: sphere at off vs segment a
            let vin = EFCheckSphereVectorCollisionIn { vec: i.a, center: i.off, motion: i.motion, radius: i.radius };
            let mut vout = EFCheckSphereVectorCollisionOut::default();
            let r = Self::CheckSphereVectorCollision(&vin, &mut vout);
            if r & 9 != 0 {
                o.t = vout.t;
                o.normal = vout.normal;
            }
            r
        }
    }

    /// CheckFiniteVectorVectorCollision(in, out, flags). Returns `flags` unchanged when the segments
    /// are parallel and overlapping (0x2100), else 1 (hit) / 0x10.
    pub fn CheckFiniteVectorVectorCollision(i: &EFCheckFiniteVectorVectorCollisionIn, o: &mut EFCheckFiniteVectorVectorCollisionOut, mut flags: u32) -> u32 {
        // position of b at the infinite-line contact time (flag 8 = already in contact)
        // decomp: `operator*(tmp, t)` — the vector operand is the motion (+0x24)
        let off = if flags & 8 == 0 { add(i.off, scale(i.motion, i.t)) } else { i.off };
        let nin = EFVectorVectorNearestPointIn { a: i.a, b: i.b, off, maxdist: 1_000_000.0 };
        let mut nout = EFVectorVectorNearestPointOut::default();
        let r4 = Self::GetVectorVectorNearestPoint(&nin, &mut nout);
        if r4 & 0x2100 == 0x2100 {
            o.t = i.t;
            o.normal = i.normal;
            return flags; // quirk: the incoming flags are returned as is
        }
        o.t = 10000.0;
        flags = 0x10;
        let mut r5 = 0;
        if r4 & 0x1800 != 0 {
            let p = if r4 & 0x800 == 0 { add(i.off, i.b) } else { i.off };
            let sin = EFCheckSphereSweepCollisionIn { vec: i.a, center: p, motion: i.motion, radius: i.radius, tmax: i.tmax };
            let mut sout = EFCheckSphereSweepCollisionOut::default();
            r5 = Self::CheckSphereSweepCollision(&sin, &mut sout);
            if r5 & 1 != 0 {
                flags = 1;
                o.t = sout.t;
                o.normal = sout.normal;
            }
        }
        if r4 & 0xc0 != 0 && r5 & 0x100 == 0 {
            let p = if r4 & 0x40 == 0 { add(neg(i.off), i.a) } else { neg(i.off) };
            let sin = EFCheckSphereSweepCollisionIn { vec: i.b, center: p, motion: neg(i.motion), radius: i.radius, tmax: i.tmax };
            let mut sout = EFCheckSphereSweepCollisionOut::default();
            let r = Self::CheckSphereSweepCollision(&sin, &mut sout);
            if r & 1 != 0 && sout.t < o.t {
                o.t = sout.t;
                flags = 1;
                o.normal = neg(sout.normal);
            }
        }
        flags
    }

    /// GetVectorVectorNearestPoint: nearest points of segments a (from 0) and b (from off).
    pub fn GetVectorVectorNearestPoint(i: &EFVectorVectorNearestPointIn, o: &mut EFVectorVectorNearestPointOut) -> u32 {
        let din = *i;
        let mut dout = EFGetVectorVectorDistanceOut::default();
        let r5 = Self::GetVectorVectorDistance(&din, &mut dout);
        if i.maxdist < dout.dist {
            return 4;
        }
        let (a, b, off) = (i.a, i.b, i.off);
        let (pb, pa, mut f): (V3, V3, u32);
        if r5 & 0x200 == 0 {
            let ca = cross(b, a);
            let cb = cross(off, a);
            let num = -(ca[2] * cb[2] + ca[0] * cb[0] + ca[1] * cb[1]);
            let (mut q, mut uf);
            if num < 0.0 {
                q = off;
                uf = 0x800;
            } else {
                let den = dot(ca, ca);
                if den < num {
                    q = add(b, off);
                    uf = 0x1000;
                } else {
                    q = add(scale(b, num / den), off);
                    uf = 0x2000;
                }
            }
            let t = dot(q, a);
            let p;
            if t < 0.0 {
                p = [0.0; 3];
                uf |= 0x40;
            } else {
                let aa = dot(a, a);
                if aa < t {
                    uf |= 0x80;
                    p = a;
                } else {
                    let s = t / aa;
                    uf |= 0x100;
                    p = scale(a, s);
                }
            }
            if uf & 0xc0 != 0 && uf & 0x2000 != 0 {
                // a clamped: re-project the clamped point onto b
                let w = sub(p, off);
                let tb = dot(w, b);
                if tb < 0.0 {
                    q = off;
                    uf = uf & !0x2000 | 0x800;
                } else {
                    let bb = dot(b, b);
                    if bb < tb {
                        q = add(b, off);
                        uf = uf & !0x2000 | 0x1000;
                    } else {
                        q = add(scale(b, tb / bb), off);
                    }
                }
            }
            pb = q;
            pa = p;
            f = uf;
        } else {
            // parallel segments
            let s0 = dot(off, a);
            let s1 = s0 + dot(b, a);
            let mut p = [0.0; 3];
            let mut q = [0.0; 3];
            if 0.0 <= s0 || 0.0 <= s1 {
                let aa = dot(a, a);
                if s0 <= aa || s1 <= aa {
                    p = scale(a, s0 / aa);
                    f = 0x2100;
                    q = off;
                } else {
                    f = 0x80;
                    p = a;
                }
            } else {
                f = 0x40;
            }
            if f & 0xc0 != 0 {
                if 0.0 < -dot(off, b) {
                    f |= 0x1000;
                    q = add(b, off);
                } else {
                    f |= 0x800;
                    q = off;
                }
            }
            pb = q;
            pa = p;
        }
        o.dir = sub(pb, pa);
        let l = length(o.dir);
        if l <= i.maxdist {
            o.dist = l;
            if 0.0 < l {
                o.dir = scale(o.dir, 1.0 / l);
            } else {
                o.dir[2] = 1.0;
            }
            f |= 0x400;
            return f;
        }
        4
    }

    /// GetVectorVectorDistance: distance between the infinite lines a and b (b through off).
    /// Returns 0x400 (skew lines, dir = common normal toward b) or 0x200 (parallel).
    pub fn GetVectorVectorDistance(i: &EFGetVectorVectorDistanceIn, o: &mut EFGetVectorVectorDistanceOut) -> u32 {
        let c = cross(i.a, i.b);
        o.dir = c;
        if 1e-5 <= dot(c, c) {
            normalize(&mut o.dir);
            let d = dot(i.off, o.dir);
            o.dist = d;
            if d < 0.0 {
                o.dist = -d;
                o.dir = neg(o.dir);
            }
            0x400
        } else {
            let c2 = cross(i.off, i.a);
            let aa = dot(i.a, i.a);
            o.dist = (dot(c2, c2) / aa).sqrt();
            let t = dot(i.off, i.a) / aa;
            o.dir = sub(i.off, scale(i.a, t));
            let l = normalize(&mut o.dir);
            if l == 0.0 {
                o.dir[2] = 1.0;
            }
            0x200
        }
    }

    /// CheckSphereSphereCollision: moving point `center` (+motion) against the origin sphere of
    /// `radius`. 8 = already inside, 0x10 miss, 0x20 behind, 1 hit (normal at the contact).
    pub fn CheckSphereSphereCollision(i: &EFCheckSphereSphereCollisionIn, o: &mut EFCheckSphereSphereCollisionOut) -> u32 {
        let c = i.center;
        let cc = dot(c, c);
        let rr = i.radius * i.radius;
        let cm = dot(c, i.motion);
        let mm = dot(i.motion, i.motion);
        if cc < rr {
            o.t = 0.0;
            o.normal = c;
            if normalize(&mut o.normal) == 0.0 {
                o.normal[2] = 1.0;
            }
            return 8;
        }
        if mm <= 0.0 {
            return 0x10;
        }
        let disc = cm * cm - mm * (cc - rr);
        if neg_bit(disc) {
            return 0x10;
        }
        let t = (-cm - disc.sqrt()) / mm;
        o.t = t;
        if neg_bit(t) {
            return 0x20;
        }
        o.motion = scale(i.motion, o.t);
        o.normal = add(c, o.motion);
        normalize(&mut o.normal);
        1
    }

    /// CheckSphereVectorCollision: moving sphere against the infinite line through `vec`;
    /// then classifies the contact along the segment (0x40 / 0x100 / 0x80).
    pub fn CheckSphereVectorCollision(i: &EFCheckSphereVectorCollisionIn, o: &mut EFCheckSphereVectorCollisionOut) -> u32 {
        let v = i.vec;
        let c1 = cross(i.center, v);
        let c2 = cross(i.motion, v);
        let vv = dot(v, v);
        let c11 = dot(c1, c1);
        let rr = i.radius * i.radius;
        let c12 = dot(c1, c2);
        let c22 = dot(c2, c2);
        let p;
        let r4;
        if c11 < vv * rr {
            p = i.center;
            r4 = 8;
            o.motion = [0.0; 3];
            o.t = 0.0;
        } else {
            if c22 < 1e-11 {
                return 0x10;
            }
            let disc = c12 * c12 - c22 * (c11 - rr * dot(v, v));
            if neg_bit(disc) {
                return 0x10;
            }
            let t = (-c12 - disc.sqrt()) / c22;
            if neg_bit(t) {
                return 0x20;
            }
            o.t = t;
            o.motion = scale(i.motion, t);
            p = add(i.center, o.motion);
            r4 = 1;
        }
        let s = dot(p, v) / vv;
        let r5 = if !neg_bit(s) {
            if 1.0 < s { 0x80 } else { 0x100 }
        } else {
            0x40
        };
        o.normal = sub(p, scale(v, s));
        if normalize(&mut o.normal) == 0.0 {
            o.normal[2] = 1.0;
        }
        r4 | r5
    }
}

/// RwV3dTransformVectors replacement (rotation part only, RW row-vector convention).
pub fn transform_vector(m: &[[f32; 4]; 4], v: V3) -> V3 {
    [
        v[0] * m[0][0] + v[1] * m[1][0] + v[2] * m[2][0],
        v[0] * m[0][1] + v[1] * m[1][1] + v[2] * m[2][1],
        v[0] * m[0][2] + v[1] * m[1][2] + v[2] * m[2][2],
    ]
}
/// RwV3dTransformPoints replacement.
pub fn transform_point(m: &[[f32; 4]; 4], v: V3) -> V3 {
    add(transform_vector(m, v), [m[3][0], m[3][1], m[3][2]])
}
pub const IDENTITY: [[f32; 4]; 4] = [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]];

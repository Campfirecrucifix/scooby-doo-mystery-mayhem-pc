//! WP-I: EFQuadraticSpline, EFSplineCursor, EFSplineMgr and the APIspline_* host functions
//! (EXE.1 Classes/EF/EFQuadraticSpline.cpp, EFSplineCursor.cpp, EFSplineMgr.cpp, EFApi_Spline.cpp).
//!
//! Data: EFSplineMgr::LoadData reads a blob (size, +4, nb splines (+8), +0xc, spline table offset
//! (+0x10)); the table holds 0x30-byte EFQuadraticSpline headers whose +0x2c is an offset to an
//! array of 0xa4-byte segments. [`EFSplineMgr::from_blob`] parses that blob into owned structs.
//!
//! Segment layout (0xa4): +0 P0, +0xc P1, +0x18 P2, +0x24 length, +0x28 distance (GetDistance),
//! +0x2c..+0x3c closed-form length coefficients (a, b, c, sqrt(a)/k, const), +0x40 B, +0x4c C
//! (point(t) = P0 + B t + C t^2), +0x58 / +0x5c / +0x60 nearest-point polynomial coefficients,
//! +0x64..+0xa0 four culling planes.
//!
//! Replaced: GetNearest's per-segment plane culling (pure optimisation: it only skips segments
//! that cannot beat the current best) is not ported; DTCubic::Solve is a standard real-root solver.
#![allow(non_snake_case)]

use crate::engine::{Mem, Ptr};
use crate::engine::dt::stream::DTBasePersistStream;

type V3 = [f32; 3];

fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn len2(a: V3) -> f32 {
    a[2] * a[2] + a[0] * a[0] + a[1] * a[1]
}

/// One 0xa4-byte segment.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SplineSegment {
    pub p: [V3; 3],
    /// +0x24
    pub length: f32,
    /// +0x28
    pub distance: f32,
    /// +0x2c..+0x3c
    pub lc: [f32; 5],
    /// +0x40 / +0x4c
    pub b: V3,
    pub c: V3,
    /// +0x58 / +0x5c / +0x60
    pub n: [f32; 3],
    /// +0x64..+0xa0
    pub planes: [[f32; 4]; 4],
}

/// EFQuadraticSpline (0x30 header).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EFQuadraticSpline {
    /// +0 id
    pub id: u16,
    /// +2 type flags (bit 3 = closed / looping)
    pub ty: u16,
    /// +0xc total length
    pub length: f32,
    /// +0x10 bbox centre, +0x1c half extents
    pub center: V3,
    pub extents: V3,
    /// +0x2c (+0x28 = count)
    pub segs: Vec<SplineSegment>,
}

/// C (uint)float as compiled
fn f2u(f: f32) -> u32 {
    if f < 2.147_483_6e9 { f as i32 as u32 } else { (f - 2.147_483_6e9) as i32 as u32 | 0x8000_0000 }
}

/// DTCubic::Solve (1:1 f32, host::dtmath)
pub use crate::engine::host::dtmath::cubic_solve;

impl EFQuadraticSpline {
    pub fn nseg(&self) -> u32 {
        self.segs.len() as u32
    }
    fn closed(&self) -> bool {
        self.ty & 8 != 0
    }
    /// AdjustGlobalFactor (closed: frac; open: clamp 0..1)
    pub fn AdjustGlobalFactor(&self, f: f32) -> f32 {
        if !self.closed() { if f <= 1.0 { if f < 0.0 { 0.0 } else { f } } else { 1.0 } } else { f - f as i32 as f32 }
    }
    fn clamp_seg(&self, i: i32, t: f32) -> (usize, f32) {
        if i < 0 {
            (0, 0.0)
        } else if self.nseg() <= i as u32 {
            (self.segs.len() - 1, 1.0)
        } else {
            (i as usize, t)
        }
    }
    /// GetPoint: P0 + B t + C t^2 (evaluated as ((P0 + B t) + C t^2) like the original)
    pub fn GetPoint(&self, i: i32, t: f32) -> V3 {
        let (i, t) = self.clamp_seg(i, t);
        let s = &self.segs[i];
        let t2 = t * t;
        std::array::from_fn(|k| (s.p[0][k] + s.b[k] * t) + s.c[k] * t2)
    }
    /// GetTangent: B + C 2t
    pub fn GetTangent(&self, i: i32, t: f32) -> V3 {
        let (i, t) = self.clamp_seg(i, t);
        let s = &self.segs[i];
        let k2 = t * 2.0;
        std::array::from_fn(|k| s.b[k] + s.c[k] * k2)
    }
    pub fn GetSegmentLength(&self, i: u32) -> f32 {
        self.segs[i as usize].length
    }
    /// GetSegmentLength(seg, t): arc length from 0 to t (closed form; adaptive fallback).
    pub fn GetSegmentLengthAt(&self, i: u32, t: f32) -> f32 {
        let s = &self.segs[i as usize];
        let [a, b, c, k, cst] = s.lc;
        if (a.to_bits() & 0x7f80_0000) < 0x3a00_0000 && (b.to_bits() & 0x7f80_0000) < 0x3a00_0000 {
            return t * s.length;
        }
        let g = b + t * 2.0 * a;
        let r = (c + b * t + t * a * t).sqrt();
        let l = r + (b * 0.5 + a * t) / k;
        if l <= 0.0 {
            self.GetCurveLength(i, 0.0, t as f64, 9) as f32
        } else {
            let l = l.ln();
            (((r * g) / (a * 4.0) + (l * 0.5 * c) / k) - (b * l * 0.125 * b) / (a * k)) * 2.0 + cst
        }
    }
    fn dist(a: V3, b: V3) -> f64 {
        (len2(sub(a, b)) as f64).sqrt()
    }
    /// GetSectionLength (recursive Simpson-like subdivision, depth guard 0x1f).
    /// quirk: the depth parameter is never incremented in the original (the guard is dead); kept.
    #[allow(clippy::too_many_arguments)]
    fn GetSectionLength(&self, seg: u32, t0: f64, t1: f64, t2: f64, p0: V3, p1: V3, p2: V3, depth: u32) -> f64 {
        let d02 = Self::dist(p0, p2);
        let d01 = Self::dist(p0, p1);
        let d12 = Self::dist(p1, p2);
        let s = d01 + d12;
        let chord = |s: f64| s + (s - d02) / 3.0;
        if depth >= 0x1f || s < 1e-5 {
            return chord(s);
        }
        let flat = d02 >= 1e-5
            && s / d02 <= 1.05
            && d01 >= 1e-6
            && d12 / d01 <= 1.2
            && d12 >= 1e-6
            && d01 / d12 <= 1.2;
        if flat {
            return chord(s);
        }
        let ma = (t0 + t1) / 2.0;
        let pa = self.GetPoint(seg as i32, ma as f32);
        let l = self.GetSectionLength(seg, t0, ma, t1, p0, pa, p1, depth);
        let mb = (t1 + t2) / 2.0;
        let pb = self.GetPoint(seg as i32, mb as f32);
        l + self.GetSectionLength(seg, t1, mb, t2, p1, pb, p2, depth)
    }
    /// GetCurveLength(seg, t0, t1, n): n forced odd, sections of 3 samples.
    pub fn GetCurveLength(&self, seg: u32, t0: f64, t1: f64, n: i32) -> f64 {
        let n = if n & 1 == 0 { n + 1 } else { n };
        let ts: Vec<f64> = (0..n).map(|i| t0 + (i as f64 * (t1 - t0)) / (n - 1) as f64).collect();
        let ps: Vec<V3> = ts.iter().map(|t| self.GetPoint(seg as i32, *t as f32)).collect();
        let mut sum = 0.0;
        let mut i = 0;
        while i < (n - 1) as usize {
            sum += self.GetSectionLength(seg, ts[i], ts[i + 1], ts[i + 2], ps[i], ps[i + 1], ps[i + 2], 0);
            i += 2;
        }
        sum
    }
    /// solve the nearest-t cubic on one segment; returns (t, squared distance)
    fn nearest_t(&self, i: usize, p: V3) -> (f32, f32) {
        let s = &self.segs[i];
        let d = sub(s.p[0], p);
        let c1 = (s.b[2] * d[2] + s.b[0] * d[0] + s.b[1] * d[1]) * 2.0;
        let c2 = s.n[2] + (s.c[2] * d[2] + s.c[0] * d[0] + s.c[1] * d[1]) * 2.0;
        let (n0, n1) = (s.n[0], s.n[1]);
        let mut best = (0.0f32, f32::MAX);
        for r in cubic_solve(n0 * 4.0, n1 * 3.0, c2 * 2.0, c1) {
            let t = r.clamp(0.0, 1.0);
            let t2 = t * t;
            let v = t * c1 + t2 * c2 + n1 * t2 * t + n0 * t2 * t2;
            if v < best.1 {
                best = (t, v);
            }
        }
        (best.0, best.1 + len2(d))
    }
    /// GetNearestOnSegment: global factor of the nearest point on segment i (+ squared distance).
    pub fn GetNearestOnSegment(&self, i: u32, p: V3) -> (f32, f32) {
        let (t, d) = self.nearest_t(i as usize, p);
        ((i as f32 + t) / self.nseg() as f32, d)
    }
    /// GetNearest(p): global factor of the nearest point on the whole spline.
    pub fn GetNearest(&self, p: V3) -> f32 {
        // seed = segment whose P2 is nearest (ties keep the first)
        let mut seed = 0;
        let mut bd = f32::MAX;
        for (i, s) in self.segs.iter().enumerate() {
            let d = len2(sub(s.p[2], p));
            if d < bd {
                bd = d;
                seed = i;
            }
        }
        let (mut res, mut best) = self.GetNearestOnSegment(seed as u32, p);
        for i in 0..self.nseg() {
            if i as usize != seed {
                let (f, d) = self.GetNearestOnSegment(i, p);
                if d < best {
                    best = d;
                    res = f;
                }
            }
        }
        res
    }
}

impl EFQuadraticSpline {
    /// EFQuadraticSpline::GetNearest(int, const DTVector3d&, float*) (0x00220ef0): local search
    /// starting at segment `seg`, walking to the neighbour segment while the nearest t sticks to
    /// an end (stops when it bounces back or reaches the spline end). `dist` receives the squared
    /// distance of the best root of the last segment examined. Returns the global factor.
    pub fn GetNearestFrom(&self, mut seg: i32, p: V3, mut dist: Option<&mut f32>) -> f32 {
        let mut prev = 0.5f32;
        let mut t;
        loop {
            let s = &self.segs[seg as usize];
            let d = sub(s.p[0], p);
            let (n0, n1) = (s.n[0], s.n[1]);
            let c1 = (s.b[2] * d[2] + s.b[0] * d[0] + s.b[1] * d[1]) * 2.0;
            let c2 = s.n[2] + (s.c[2] * d[2] + s.c[0] * d[0] + s.c[1] * d[1]) * 2.0;
            let k = len2(d);
            t = 0.0f32;
            let mut best = f32::MAX;
            for r in cubic_solve(n0 * 4.0, n1 * 3.0, c2 * 2.0, c1) {
                let u = if r < 0.0 { 0.0 } else if r <= 1.0 { r } else { 1.0 };
                let u2 = u * u;
                let v = k + u * c1 + u2 * c2 + n1 * u2 * u + n0 * u2 * u2;
                if v < best {
                    best = v;
                    t = u;
                    if let Some(o) = dist.as_deref_mut() {
                        *o = v;
                    }
                }
            }
            let done = if (0.0 < t && t < 1.0) || (t <= 0.0 && 1.0 <= prev) || (1.0 <= t && prev <= 0.0) {
                true
            } else if t <= 0.0 {
                if seg < 1 {
                    true
                } else {
                    seg -= 1;
                    false
                }
            } else if (seg as u32) < self.nseg().wrapping_sub(1) {
                seg += 1;
                false
            } else {
                true
            };
            prev = t;
            if done {
                break;
            }
        }
        (seg as f32 + t) / self.nseg() as f32
    }

    /// EFQuadraticSpline::CheckSphereIntersection (0x00220700): sphere (center, radius) vs the
    /// spline. Bounding box reject (|c - center| - extent > r on any axis, computed in double),
    /// then per segment: plane 0 (segment plane) distance <= r, the 3 edge planes (+0x74/+0x84/
    /// +0x94) select the most violated edge whose end points are distance-tested; surviving
    /// segments run GetNearestOnSegment and keep the closest (squared distance < r^2, shrinking).
    /// Returns true and writes the global factor to `out` when hit.
    pub fn CheckSphereIntersection(&self, c: V3, r: f32, out: &mut f32) -> bool {
        for k in 0..3 {
            if (c[k] as f64 - self.center[k] as f64).abs() - self.extents[k] as f64 > r as f64 {
                return false;
            }
        }
        let mut r2 = r * r;
        let mut hit = false;
        let pd = |pl: &[f32; 4]| pl[3] + c[2] * pl[2] + c[0] * pl[0] + c[1] * pl[1];
        for (i, s) in self.segs.iter().enumerate() {
            let d0 = pd(&s.planes[0]);
            let rem = r2 - d0 * d0;
            if rem < 0.0 {
                continue;
            }
            let mut f18 = pd(&s.planes[1]);
            if !(f18 <= 0.0 || f18 * f18 <= rem) {
                continue;
            }
            let f17 = pd(&s.planes[2]);
            let mut e = 0usize;
            if !(f17 <= 0.0 || f17 * f17 <= rem) {
                continue;
            }
            if f18 < f17 {
                e = 2;
                f18 = f17;
            }
            let f13 = pd(&s.planes[3]);
            if !(f13 <= 0.0 || f13 * f13 <= rem) {
                continue;
            }
            if f18 < f13 {
                e = 1;
                f18 = f13;
            }
            if 0.0 < f18 {
                let a = s.p[e];
                let b = s.p[if e == 0 { 2 } else { e - 1 }];
                let ab = sub(b, a);
                let t = (ab[2] * (c[2] - a[2]) + ab[0] * (c[0] - a[0]) + ab[1] * (c[1] - a[1])) / len2(ab);
                let dd = if 1.0 < t {
                    Some(len2(sub(c, b)))
                } else if 0.0 <= t {
                    None
                } else {
                    Some(len2(sub(c, a)))
                };
                if let Some(dd) = dd {
                    if r2 < dd {
                        continue;
                    }
                }
            }
            let (f, d) = self.GetNearestOnSegment(i as u32, c);
            if d < r2 {
                *out = f;
                hit = true;
                r2 = d;
            }
        }
        hit
    }
}

/// EFSplineCursor (0x30 on stack in the API): +4 spline, +8 global factor, +0xc seg, +0x10 t.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct EFSplineCursor {
    pub global: f32,
    pub seg: u32,
    pub t: f32,
}

impl EFSplineCursor {
    /// MoveTo(seg, t)
    pub fn MoveToSeg(&mut self, sp: &EFQuadraticSpline, seg: i32, t: f32) {
        let n = sp.nseg();
        let seg = if seg < 0 { 0 } else if n <= seg as u32 { n - 1 } else { seg as u32 };
        let t = if t < 0.0 { 0.0 } else if t <= 1.0 { t } else { 1.0 };
        self.seg = seg;
        self.t = t;
        self.global = (seg as i32 as f32 + t) / n as f32;
    }
    /// MoveTo(global)
    pub fn MoveTo(&mut self, sp: &EFQuadraticSpline, f: f32) {
        self.global = sp.AdjustGlobalFactor(f);
        let x = self.global * sp.nseg() as f32;
        self.t = x - x as i32 as f32;
        self.seg = x as i32 as u32;
        if sp.nseg().wrapping_sub(1) < self.seg {
            self.seg = sp.nseg() - 1;
            self.t = 1.0;
        }
    }
    /// MoveDistance(d, next = None): arc-length move with regula falsi (10 iterations, 0.001 tol).
    pub fn MoveDistance(&mut self, sp: &EFQuadraticSpline, d: f32) {
        let mut target = sp.GetSegmentLengthAt(self.seg, self.t) + d;
        let mut seg = self.seg;
        if sp.GetSegmentLength(seg) < target {
            while sp.GetSegmentLength(seg) < target {
                target -= sp.GetSegmentLength(seg);
                seg += 1;
                if sp.nseg() <= seg {
                    seg = 0;
                    if !sp.closed() {
                        self.MoveTo(sp, 1.0);
                        return;
                    }
                }
            }
        } else if target < 0.0 {
            while target < 0.0 {
                seg = seg.wrapping_sub(1);
                if (seg as i32) < 0 {
                    if !sp.closed() {
                        self.MoveTo(sp, 0.0);
                        return;
                    }
                    seg = sp.nseg() - 1;
                }
                target += sp.GetSegmentLength(seg);
            }
        }
        let (mut hi, mut lo, mut llo) = (1.0f32, 0.0f32, 0.0f32);
        let mut lhi = sp.GetSegmentLength(seg);
        let mut x = 0.0;
        let mut num = target - 0.0;
        for _ in 0..10 {
            x = lo + (num * (hi - lo)) / (lhi - llo);
            let l = sp.GetSegmentLengthAt(seg, x);
            let err;
            if l < target {
                err = target - l;
                llo = l;
                lo = x;
            } else {
                err = l - target;
                hi = x;
                lhi = l;
            }
            if err < 0.001 {
                break;
            }
            num = target - llo;
        }
        self.MoveToSeg(sp, seg as i32, x);
    }
    /// GetLengthFactor
    pub fn GetLengthFactor(&self, sp: &EFQuadraticSpline) -> f32 {
        let mut s = 0.0;
        for i in 0..self.seg {
            s += sp.GetSegmentLength(i);
        }
        (s + sp.GetSegmentLengthAt(self.seg, self.t)) / sp.length
    }
    /// EFSplineCursor::MoveToNearest (0x00221740): local search from the cursor's segment.
    pub fn MoveToNearest(&mut self, sp: &EFQuadraticSpline, p: V3) {
        let f = sp.GetNearestFrom(self.seg as i32, p, None);
        self.MoveTo(sp, f);
    }
    pub fn SetToNearest(&mut self, sp: &EFQuadraticSpline, p: V3) {
        let f = sp.GetNearest(p);
        self.MoveTo(sp, f);
    }
}

/// EFSplineMgr (_gpoSplineMgr)
#[derive(Debug, Clone, Default)]
pub struct EFSplineMgr {
    pub splines: Vec<EFQuadraticSpline>,
}

fn rd(b: &[u8], o: usize) -> u32 {
    b.get(o..o + 4).map_or(0, |s| u32::from_le_bytes(s.try_into().unwrap()))
}
fn rdf(b: &[u8], o: usize) -> f32 {
    f32::from_bits(rd(b, o))
}
fn rdv(b: &[u8], o: usize) -> V3 {
    [rdf(b, o), rdf(b, o + 4), rdf(b, o + 8)]
}

impl EFSplineMgr {
    /// LoadData payload after the 5 header words: `table_off` = header +0x10 value (offset in the
    /// blob), `n` = header +8. Segment pointers (+0x2c) are blob offsets.
    pub fn from_blob(blob: &[u8], n: u32, table_off: u32) -> Self {
        let mut splines = vec![];
        for i in 0..n as usize {
            let h = table_off as usize + i * 0x30;
            let nseg = rd(blob, h + 0x28) as usize;
            let so = rd(blob, h + 0x2c) as usize;
            let segs = (0..nseg)
                .map(|k| {
                    let o = so + k * 0xa4;
                    SplineSegment {
                        p: [rdv(blob, o), rdv(blob, o + 0xc), rdv(blob, o + 0x18)],
                        length: rdf(blob, o + 0x24),
                        distance: rdf(blob, o + 0x28),
                        lc: std::array::from_fn(|j| rdf(blob, o + 0x2c + 4 * j)),
                        b: rdv(blob, o + 0x40),
                        c: rdv(blob, o + 0x4c),
                        n: rdv(blob, o + 0x58),
                        planes: std::array::from_fn(|p| std::array::from_fn(|j| rdf(blob, o + 0x64 + p * 16 + j * 4))),
                    }
                })
                .collect();
            splines.push(EFQuadraticSpline {
                id: rd(blob, h) as u16,
                ty: (rd(blob, h) >> 16) as u16,
                length: rdf(blob, h + 0xc),
                center: rdv(blob, h + 0x10),
                extents: rdv(blob, h + 0x1c),
                segs,
            });
        }
        EFSplineMgr { splines }
    }
    /// EFSplineMgr::LoadData (0x0021f5e0), called by EFGenericDataLoader through
    /// EFLoadingMgr::mspoMgrs[1] (vtbl 0x10): u32 blob size, u32 +4, u32 +8 spline count,
    /// u32 +0xc, u32 +0x10 spline table offset, then the blob (offsets relocated in place in the
    /// C++; here decoded by `from_blob`).
    pub fn LoadData(&mut self, _kind: u32, _size: u32, s: &mut dyn DTBasePersistStream) -> i32 {
        let mut w = [0u32; 5];
        for x in w.iter_mut() {
            s.LoadData_uint(x);
        }
        let mut blob = vec![0u8; w[0] as usize];
        s.LoadBinaryData(&mut blob);
        *self = Self::from_blob(&blob, w[2], w[4]);
        0
    }

    /// Loading-stream hook: EFGenericDataLoader (class 0x48 "EF_GENERIC_FILE", IsLoader) entries
    /// of the level pack, kept raw by the pack reader. LoadPropertiesBinary: u32 UID, u32, u32
    /// kind (index into EFLoadingMgr::mspoMgrs; 1 = _gpoSplineMgr, the only registered manager),
    /// u32 size, then mgr->LoadData(kind, size, stream). Entries are applied in pack order.
    pub fn LoadFromLevel(&mut self, res: &crate::engine::pack::EFRessourcesMgr) -> u32 {
        let mut n = 0;
        for o in &res.objects {
            let Some(raw) = o.as_raw() else { continue };
            if raw.class_id != 0x48 || raw.data.len() < 16 {
                continue;
            }
            let kind = rd(&raw.data, 8);
            if kind != 1 {
                continue;
            }
            let mut ms = crate::engine::dt::stream::DTMemoryPersistStream::new(raw.data[16..].to_vec());
            self.LoadData(kind, rd(&raw.data, 12), &mut ms);
            n += 1;
        }
        n
    }
    /// Free
    pub fn Free(&mut self) {
        self.splines.clear();
    }

    // ---- APIspline. quirk: no index bound checks in the original (out of range reads garbage);
    // here out-of-range indices return 0 / write nothing.

    pub fn APIspline_GetNbSplines(&mut self, _m: &mut Mem) -> u32 {
        self.splines.len() as u32
    }
    pub fn APIspline_GetIndex(&mut self, _m: &mut Mem, id: u32) -> u32 {
        self.splines.iter().position(|s| s.id as u32 == id).map_or(0xffff_ffff, |i| i as u32)
    }
    pub fn APIspline_GetID(&mut self, _m: &mut Mem, i: u32) -> u32 {
        self.splines.get(i as usize).map_or(0, |s| s.id as u32)
    }
    pub fn APIspline_GetPosition(&mut self, m: &mut Mem, i: u32, f: f32, out: Ptr) -> u32 {
        if let Some(s) = self.splines.get(i as usize) {
            let x = s.AdjustGlobalFactor(f) * s.nseg() as f32;
            let k = f2u(x);
            m.wv3(out, s.GetPoint(k as i32, x - k as f32));
        }
        0
    }
    pub fn APIspline_GetSegmentPosition(&mut self, m: &mut Mem, i: u32, seg: u32, t: f32, out: Ptr) -> u32 {
        if let Some(s) = self.splines.get(i as usize) {
            m.wv3(out, s.GetPoint(seg as i32, t));
        }
        0
    }
    pub fn APIspline_GetNbSegments(&mut self, _m: &mut Mem, i: u32) -> u32 {
        self.splines.get(i as usize).map_or(0, |s| s.nseg())
    }
    pub fn APIspline_Segment2GlobalFactor(&mut self, m: &mut Mem, i: u32, seg: u32, t: f32, out: Ptr) -> u32 {
        if let Some(s) = self.splines.get(i as usize) {
            let mut c = EFSplineCursor::default();
            c.MoveToSeg(s, seg as i32, t);
            m.wf(out, c.global);
        }
        0
    }
    pub fn APIspline_Global2SegmentFactor(&mut self, m: &mut Mem, i: u32, f: f32, seg: Ptr, t: Ptr) -> u32 {
        if let Some(s) = self.splines.get(i as usize) {
            let mut c = EFSplineCursor::default();
            c.MoveTo(s, f);
            m.wf(t, c.t);
            m.w32(seg, c.seg);
        }
        0
    }
    pub fn APIspline_GetTangent(&mut self, m: &mut Mem, i: u32, f: f32, out: Ptr) -> u32 {
        if let Some(s) = self.splines.get(i as usize) {
            let x = s.AdjustGlobalFactor(f) * s.nseg() as f32;
            let k = f2u(x);
            m.wv3(out, s.GetTangent(k as i32, x - k as f32));
        }
        0
    }
    /// float bits
    pub fn APIspline_GetLength(&mut self, _m: &mut Mem, i: u32) -> u32 {
        self.splines.get(i as usize).map_or(0, |s| s.length.to_bits())
    }
    /// float bits (segment +0x28)
    pub fn APIspline_GetDistance(&mut self, _m: &mut Mem, i: u32, seg: u32) -> u32 {
        self.splines.get(i as usize).and_then(|s| s.segs.get(seg as usize)).map_or(0, |g| g.distance.to_bits())
    }
    /// float bits. NOTE: the original is `void` (result left in f0); the float is returned here.
    pub fn APIspline_GetSegmentLength(&mut self, _m: &mut Mem, i: u32, seg: u32, t: f32) -> u32 {
        self.splines
            .get(i as usize)
            .filter(|s| seg < s.nseg())
            .map_or(0, |s| s.GetSegmentLengthAt(seg, t).to_bits())
    }
    /// returns the new global factor (float bits)
    pub fn APIspline_MoveDistance(&mut self, _m: &mut Mem, i: u32, f: f32, d: f32) -> u32 {
        let Some(s) = self.splines.get(i as usize) else { return 0 };
        let mut c = EFSplineCursor::default();
        c.MoveTo(s, f);
        c.MoveDistance(s, d);
        c.global.to_bits()
    }
    pub fn APIspline_GetType(&mut self, _m: &mut Mem, i: u32) -> u32 {
        self.splines.get(i as usize).map_or(0, |s| s.ty as u32)
    }
    /// float bits
    pub fn APIspline_GetNearest(&mut self, m: &mut Mem, i: u32, p: Ptr) -> u32 {
        let v = m.rv3(p);
        self.splines.get(i as usize).map_or(0, |s| s.GetNearest(v).to_bits())
    }
}

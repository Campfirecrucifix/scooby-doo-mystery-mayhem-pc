//! EFCollisionMap (class 0x24, "EF_COLLISION_MAP"): the level collision mesh, a relocatable blob
//! holding materials, convex polygon primitives and an octree (Classes/EF/EFCollisionMap.cpp,
//! EFBaseAdressRecalc.cpp).
//!
//! Memory model: the original keeps the raw blob and patches its offsets into absolute pointers
//! (Init + EFBaseAdressRecalc). Here the blob is kept as little-endian `u32` words and the patched
//! "pointers" are WORD INDICES into that same vector, so every `this + 0xNN` access of the C++
//! becomes `w[p + 0xNN/4]`. Nothing of it is read by the logic modules, so it stays native.
//!
//! Blob layout (bytes, from Init):
//! - +0x00 u32 total size (written by LoadPropertiesBinary), +0x04 ?, +0x08 primitives offset,
//!   +0x0c octree root offset; both offsets relative to `base` = blob+0x10.
//! - base+0: material records, 0x20 bytes each: {u32 flags (bit0 = double sided), u32 mask1,
//!   u32 mask2, ...}. A primitive's word 0 is a material index (`< base`) patched to base+idx*0x20.
//! - primitive (EFCollisionPrim): +0 material, +4 u16 ?, +6 u16 nverts, +8 i32 query stamp,
//!   +0xc plane normal, +0x18 plane d (n.p + d = 0), +0x1c nverts*vec3 vertices, then nverts edge
//!   planes {nx,ny,nz,d} (point inside when every edge plane gives <= 0). Size = (7 + 7*nverts) words.
//! - octree node (EFOctreeNodeHeader): +0 flags (bit0 leaf, bits 1/2/3 = split on x/y/z), +4 center,
//!   +0x10 half extents, +0x1c children[8] (offset from the octree root, 0 = none; child index bit0=x,
//!   bit1=y, bit2=z, set = positive side) or for a leaf EFOctreeNodeEnd {u32 count, prim offsets
//!   relative to base}.
#![allow(non_snake_case)]

use crate::engine::error::EngineError;

pub type V3 = [f32; 3];

/// RayIntersectIn: +0 origin, +0xc delta (segment: t in [0,1]), +0x20 mask1, +0x24 mask2.
#[derive(Debug, Clone, Copy, Default)]
pub struct RayIntersectIn {
    pub origin: V3,
    pub dir: V3,
    /// +0x20, tested against material word 1
    pub mask1: u32,
    /// +0x24, tested against material word 2
    pub mask2: u32,
}

/// EFRayIntersectOut. +0 callback object (vtable+8 `(out, 2) -> bool`; null = keep nearest hit),
/// +4 hit position, +0x10 hit normal, +0x24 primitive.
#[derive(Default)]
pub struct EFRayIntersectOut<'a> {
    /// +0x00: when set, every accepted hit is reported; returning false aborts the query.
    pub callback: Option<&'a mut dyn FnMut(&EFRayHit, i32) -> bool>,
    pub hit: EFRayHit,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct EFRayHit {
    /// +0x04 (only written in callback mode, as in the C++; the nearest-hit caller computes it)
    pub pos: V3,
    /// +0x10
    pub normal: V3,
    /// +0x24 primitive (word index into the map blob)
    pub prim: Option<usize>,
}

/// EFGetPrimitivesInSphereData: +0 center, +0xc radius, +0x10 mask1, +0x14 mask2,
/// +0x18 output cursor, +0x1c remaining capacity.
#[derive(Debug, Clone, Default)]
pub struct EFGetPrimitivesInSphereData {
    pub center: V3,
    pub radius: f32,
    pub mask1: u32,
    pub mask2: u32,
    /// +0x18 output primitives (word indices)
    pub out: Vec<usize>,
    /// +0x1c remaining capacity
    pub capacity: i32,
}

#[derive(Debug, Clone, Default)]
pub struct EFCollisionMap {
    pub uid: u32,
    /// this+0xc: the blob (words; index 0 = byte 0)
    pub w: Vec<u32>,
    /// this+0x10: base (word index, = 4)
    pub base: usize,
    /// this+0x18: octree root (word index)
    pub root: usize,
    /// this+0x1c: first primitive (word index)
    pub prims: usize,
}

#[inline]
fn f(x: u32) -> f32 {
    f32::from_bits(x)
}
/// `(bits & 0x7f800000) < 0x3c000000`, i.e. |x| < 2^-7: the octree walk treats such a ray
/// component as zero (quirk: not an epsilon on the value, an exponent test).
#[inline]
fn tiny(x: f32) -> bool {
    (x.to_bits() & 0x7f80_0000) < 0x3c00_0000
}

impl EFCollisionMap {
    pub fn GetClassID(&self) -> u32 {
        0x24
    }
    pub fn GetClassName(&self) -> &'static str {
        "EF_COLLISION_MAP"
    }

    /// Pack entry = LoadPropertiesBinary directly; its first ignored u32 is the UID slot
    /// (0xffffffff in EP1L01), the second is 1. `data` is the
    /// whole FAT record (RawStreamable::data). Returns the map and the bytes consumed.
    pub fn LoadFromRaw(data: &[u8]) -> Result<(Self, usize), EngineError> {
        let rd = |o: usize| -> Result<u32, EngineError> {
            data.get(o..o + 4)
                .map(|b| u32::from_le_bytes(b.try_into().unwrap()))
                .ok_or_else(|| EngineError::Format("collision map truncated".into()))
        };
        let mut m = EFCollisionMap { uid: rd(0)?, ..Default::default() };
        let used = m.LoadPropertiesBinary(data)?;
        Ok((m, used))
    }

    /// LoadPropertiesBinary: LoadData(uint) x2 (ignored), LoadData(uint size), buf = new[size],
    /// buf[0] = size, LoadBinaryData(buf+4, size-4), Init(buf), EFPhysicMgr::AddMap(this) (the
    /// caller does AddMap here). Returns bytes consumed.
    pub fn LoadPropertiesBinary(&mut self, d: &[u8]) -> Result<usize, EngineError> {
        if d.len() < 12 {
            return Err(EngineError::Format("collision map header".into()));
        }
        let size = u32::from_le_bytes(d[8..12].try_into().unwrap()) as usize;
        if size < 0x10 || 12 + size - 4 > d.len() || size % 4 != 0 {
            return Err(EngineError::Format(format!("collision map size {size:#x}")));
        }
        let mut w = Vec::with_capacity(size / 4);
        w.push(size as u32);
        for c in d[12..12 + size - 4].chunks_exact(4) {
            w.push(u32::from_le_bytes(c.try_into().unwrap()));
        }
        self.Init(w)?;
        Ok(12 + size - 4)
    }

    /// Init(void*): set pointers, relocate the octree (EFBaseAdressRecalc visitor) and the
    /// primitives' material pointers.
    pub fn Init(&mut self, w: Vec<u32>) -> Result<(), EngineError> {
        let bad = || EngineError::Format("collision map offsets".into());
        self.w = w;
        self.base = 4;
        let n = self.w.len();
        self.root = self.base + self.w[3] as usize / 4;
        self.prims = self.base + self.w[2] as usize / 4;
        if self.root + 7 > n || self.prims > n {
            return Err(bad());
        }
        self.Relocate(self.root, 0)?;
        let end = self.w[0] as usize / 4;
        let mut p = self.prims;
        while p < end {
            if p + 7 > n {
                return Err(bad());
            }
            // original: `if (*p < base) *p = base + *p * 0x20` (index -> pointer)
            self.w[p] = (self.base + self.w[p] as usize * 8) as u32;
            let nv = (self.w[p + 1] >> 16) as usize;
            p += nv * 7 + 7;
        }
        Ok(())
    }

    /// VisitOctree(node, EFBaseAdressRecalc) — the visitor patches a node before its children are
    /// visited: children += octree root, leaf primitive offsets += base.
    fn Relocate(&mut self, node: usize, depth: u32) -> Result<(), EngineError> {
        let n = self.w.len();
        if depth > 64 || node + 7 > n {
            return Err(EngineError::Format("collision octree".into()));
        }
        if self.w[node] & 1 == 0 {
            for i in 0..8 {
                let c = self.w[node + 7 + i];
                if c != 0 {
                    let ci = self.root + c as usize / 4;
                    self.w[node + 7 + i] = ci as u32;
                }
            }
            for i in 0..8 {
                let c = self.w[node + 7 + i];
                if c != 0 {
                    self.Relocate(c as usize, depth + 1)?;
                }
            }
        } else {
            let cnt = self.w[node + 7] as usize;
            if node + 8 + cnt > n {
                return Err(EngineError::Format("collision leaf".into()));
            }
            for i in 0..cnt {
                let v = self.base + self.w[node + 8 + i] as usize / 4;
                self.w[node + 8 + i] = v as u32;
            }
        }
        Ok(())
    }

    // ---- primitive accessors (EFCollisionPrim) ----
    pub fn PrimCount(&self, p: usize) -> usize {
        (self.w[p + 1] >> 16) as usize
    }
    pub fn PrimMaterial(&self, p: usize) -> usize {
        self.w[p] as usize
    }
    pub fn PrimNormal(&self, p: usize) -> V3 {
        [f(self.w[p + 3]), f(self.w[p + 4]), f(self.w[p + 5])]
    }
    pub fn PrimD(&self, p: usize) -> f32 {
        f(self.w[p + 6])
    }
    pub fn PrimVertex(&self, p: usize, i: usize) -> V3 {
        let o = p + 7 + i * 3;
        [f(self.w[o]), f(self.w[o + 1]), f(self.w[o + 2])]
    }
    /// edge plane i, located after `nverts & 0x7fff` vertices (quirk: loop counts use the full u16)
    pub fn PrimEdgePlane(&self, p: usize, i: usize) -> [f32; 4] {
        let o = p + 7 + (self.PrimCount(p) & 0x7fff) * 3 + i * 4;
        [f(self.w[o]), f(self.w[o + 1]), f(self.w[o + 2]), f(self.w[o + 3])]
    }
    /// material word i (0 flags, 1 mask1, 2 mask2)
    pub fn MaterialWord(&self, mat: usize, i: usize) -> u32 {
        self.w[mat + i]
    }
    /// All primitives in blob order (word indices).
    pub fn Primitives(&self) -> Vec<usize> {
        let mut v = Vec::new();
        let end = self.w[0] as usize / 4;
        let mut p = self.prims;
        while p < end {
            v.push(p);
            p += self.PrimCount(p) * 7 + 7;
        }
        v
    }
    fn node_v(&self, node: usize, o: usize) -> V3 {
        [f(self.w[node + o]), f(self.w[node + o + 1]), f(self.w[node + o + 2])]
    }

    /// GetPrimitivesInSphere(data). `stamp` = the global query counter iRam00343cc0 (primitives
    /// already stamped with it are skipped, so each primitive is reported once per query).
    pub fn GetPrimitivesInSphere(&mut self, d: &mut EFGetPrimitivesInSphereData, stamp: i32) -> i32 {
        self.GetPrimitivesInSphereForOctreeNode(self.root, d, stamp)
    }

    pub fn GetPrimitivesInSphereForOctreeNode(&mut self, node: usize, d: &mut EFGetPrimitivesInSphereData, stamp: i32) -> i32 {
        let r = d.radius;
        let c = self.node_v(node, 1);
        let h = self.node_v(node, 4);
        let rel = [d.center[0] - c[0], d.center[1] - c[1], d.center[2] - c[2]];
        let mut ex = [0f32; 3];
        for a in 0..3 {
            let e = rel[a].abs() - h[a];
            if e < 0.0 {
                ex[a] = 0.0;
            } else if r < e {
                return 0;
            } else {
                ex[a] = e;
            }
        }
        if ex[2] * ex[2] + ex[0] * ex[0] + ex[1] * ex[1] > r * r {
            return 0;
        }
        let flags = self.w[node];
        if flags & 1 != 0 {
            return self.GetPrimitivesInSphereForOctreeLeaf(d, node + 7, stamp);
        }
        let (fx, fy, fz) = (rel[0], rel[1], rel[2]);
        let u5: u32 = if flags & 2 == 0 {
            0x55
        } else if r < fx {
            0xaa
        } else if fx < -r {
            0x55
        } else {
            0xff
        };
        let mut u9 = u5 & 0x33;
        if flags & 4 != 0 {
            u9 = if r < fy {
                u5 & 0xcc
            } else if fy < -r {
                u5 & 0x33
            } else {
                u5
            };
        }
        if flags & 8 == 0 {
            u9 &= 0xf;
        } else if r < fz {
            u9 &= 0xf0;
        } else if fz < -r {
            u9 &= 0xf;
        }
        let mut n = 0;
        for i in 0..8 {
            let ch = self.w[node + 7 + i] as usize;
            if u9 & (1 << i) != 0 && ch != 0 {
                n += self.GetPrimitivesInSphereForOctreeNode(ch, d, stamp);
            }
        }
        n
    }

    /// GetPrimitivesInSphereForOctreeLeaf(data, EFOctreeNodeEnd& at word `end`).
    pub fn GetPrimitivesInSphereForOctreeLeaf(&mut self, d: &mut EFGetPrimitivesInSphereData, end: usize, stamp: i32) -> i32 {
        let mut found = 0;
        let mut remaining = d.capacity;
        if remaining == 0 {
            return 0;
        }
        let r2 = d.radius * d.radius;
        let cnt = self.w[end] as usize;
        let cen = d.center;
        for k in 0..cnt {
            let p = self.w[end + 1 + k] as usize;
            if self.w[p + 2] as i32 == stamp {
                continue;
            }
            self.w[p + 2] = stamp as u32;
            let nv = self.PrimCount(p);
            let nrm = self.PrimNormal(p);
            let dist = nrm[2] * cen[2] + nrm[0] * cen[0] + nrm[1] * cen[1] + self.PrimD(p);
            let rad = d.radius;
            if (dist.abs() as f64) > rad as f64 {
                continue;
            }
            // the edge plane the center is most outside of; give up once beyond the radius
            let mut i = 0;
            let mut best = -100000.0f32;
            let mut best_i = 0usize;
            while i < nv {
                let e = self.PrimEdgePlane(p, i);
                let v = e[3] + e[2] * cen[2] + e[0] * cen[0] + e[1] * cen[1];
                if best < v {
                    best = v;
                    best_i = i;
                    if rad < v {
                        break;
                    }
                }
                i += 1;
            }
            if i != nv {
                continue;
            }
            let accept = if best <= 0.0 {
                true
            } else if dist * dist + best * best <= r2 {
                // distance to the edge segment [v(best_i), v(best_i+1)]
                let j = if best_i + 1 == nv { 0 } else { best_i + 1 };
                let a = self.PrimVertex(p, best_i);
                let b = self.PrimVertex(p, j);
                let e = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
                let q = [cen[0] - a[0], cen[1] - a[1], cen[2] - a[2]];
                let t = e[2] * q[2] + e[0] * q[0] + e[1] * q[1];
                if t < 0.0 {
                    q[2] * q[2] + q[0] * q[0] + q[1] * q[1] <= r2
                } else if t <= e[2] * e[2] + e[0] * e[0] + e[1] * e[1] {
                    // quirk: within the segment's span = touching (the plane tests above bound it)
                    true
                } else {
                    (q[2] - e[2]) * (q[2] - e[2]) + (q[0] - e[0]) * (q[0] - e[0]) + (q[1] - e[1]) * (q[1] - e[1]) <= r2
                }
            } else {
                false
            };
            if accept {
                let mat = self.PrimMaterial(p);
                if d.mask1 & self.w[mat + 1] != 0 || d.mask2 & self.w[mat + 2] != 0 {
                    d.out.push(p);
                    remaining -= 1;
                    found += 1;
                    if remaining == 0 {
                        break;
                    }
                }
            }
        }
        d.capacity = remaining;
        found
    }

    /// RayIntersect(in, out, t): returns 0 no hit, 2 hit, 3 callback mode / aborted.
    /// `t` must be initialised by the caller (nearest-hit mode: 1.0 or larger).
    pub fn RayIntersect(&mut self, rin: &RayIntersectIn, out: &mut EFRayIntersectOut, t: &mut f32, stamp: i32) -> i32 {
        let hit = self.RayIntersectNode(rin, out, t, self.root, stamp);
        if out.callback.is_none() && *t != -1.0 {
            if hit { 2 } else { 0 }
        } else {
            3
        }
    }

    /// RayIntersect(in, out, t, node): front-to-back octree walk; stops at the first child that
    /// produced a hit.
    pub fn RayIntersectNode(&mut self, rin: &RayIntersectIn, out: &mut EFRayIntersectOut, t: &mut f32, node: usize, stamp: i32) -> bool {
        let mut child = 0i32;
        if !self.FindFirstChild(rin, node, &mut child) {
            return false;
        }
        if self.w[node] & 1 != 0 {
            return self.RayIntersectNodeEnd(rin, out, node + 7, t, stamp);
        }
        loop {
            let c = self.w[node + 7 + child as usize] as usize;
            if c != 0 && self.RayIntersectNode(rin, out, t, c, stamp) {
                return true;
            }
            if !self.FindNextChild(rin, node, &mut child) {
                return false;
            }
        }
    }

    /// FindFirstChild: slab test of the segment against the node box; on entry, the octant
    /// containing the entry point.
    pub fn FindFirstChild(&self, rin: &RayIntersectIn, node: usize, child: &mut i32) -> bool {
        let c = self.node_v(node, 1);
        let h = self.node_v(node, 4);
        let o = rin.origin;
        let dv = rin.dir;
        let mut d = [c[0] - o[0], c[1] - o[1], c[2] - o[2]];
        let mut inside = 0;
        for a in 0..3 {
            if (d[a].abs() as f64) < h[a] as f64 {
                inside += 1;
            } else if tiny(dv[a]) {
                return false;
            }
        }
        let flags = self.w[node];
        let octant = |p: V3| -> i32 {
            let r = [p[0] - c[0], p[1] - c[1], p[2] - c[2]];
            let mut b = 0;
            if flags & 2 != 0 && 0.0 < r[0] {
                b = 1;
            }
            if flags & 4 != 0 && 0.0 < r[1] {
                b |= 2;
            }
            if flags & 8 != 0 && 0.0 < r[2] {
                b |= 4;
            }
            b
        };
        if inside == 3 {
            *child = octant(o);
            return true;
        }
        let mut tin = [0f32; 3];
        let mut tout = [2f32; 3];
        for a in 0..3 {
            d[a] = if 0.0 < dv[a] { d[a] - h[a] } else { d[a] + h[a] };
            if !tiny(dv[a]) {
                tin[a] = d[a] / dv[a];
                tout[a] = (tin[a] as f64 + ((h[a] / dv[a]).abs() as f64) * 2.0) as f32;
            } // quirk: a ~zero axis contributes t=0 to the entry max and 2.0 to the exit min
        }
        let mut tmin = if tin[1] <= tin[0] { tin[0] } else { tin[1] };
        if tmin < tin[2] {
            tmin = tin[2];
        }
        if 1.0 < tmin {
            return false;
        }
        if tmin < 0.0 {
            tmin = 0.0;
        }
        let mut tmax = if tout[0] <= tout[1] { tout[0] } else { tout[1] };
        if tout[2] < tmax {
            tmax = tout[2];
        }
        if tmax < tmin {
            return false;
        }
        let p = [o[0] + dv[0] * tmin, o[1] + dv[1] * tmin, o[2] + dv[2] * tmin];
        *child = octant(p);
        true
    }

    /// FindNextChild: step to the neighbouring octant through the nearest exit plane of the
    /// current one (quirk: the node's split flags are not consulted).
    pub fn FindNextChild(&self, rin: &RayIntersectIn, node: usize, child: &mut i32) -> bool {
        let c = self.node_v(node, 1);
        let h = self.node_v(node, 4);
        let cur = *child as u32;
        let mut t = [2f32; 3];
        for a in 0..3 {
            let q = h[a] / 2.0;
            let cc = if cur & (1 << a) == 0 { c[a] - q } else { c[a] + q };
            let plane = if 0.0 < rin.dir[a] { cc + q } else { cc - q };
            if !tiny(rin.dir[a]) {
                t[a] = (plane - rin.origin[a]) / rin.dir[a];
            }
        }
        let mut axis = (t[1] < t[0]) as usize;
        let mut tm = if t[1] < t[0] { t[1] } else { t[0] };
        if t[2] < tm {
            axis = 2;
            tm = t[2];
        }
        if (tm as f64) > 1.0 {
            return false;
        }
        let mut b = cur & (1 << axis) != 0;
        if 0.0 < rin.dir[axis] {
            b = !b;
        }
        if b {
            *child ^= 1 << axis;
            true
        } else {
            false
        }
    }

    /// RayIntersectNodeEnd: test the leaf's primitives (each once per query, via the stamp).
    pub fn RayIntersectNodeEnd(&mut self, rin: &RayIntersectIn, out: &mut EFRayIntersectOut, end: usize, t: &mut f32, stamp: i32) -> bool {
        let mut res = false;
        let cnt = self.w[end] as usize;
        let (o, dv) = (rin.origin, rin.dir);
        for k in 0..cnt {
            let p = self.w[end + 1 + k] as usize;
            if self.w[p + 2] as i32 == stamp {
                continue;
            }
            self.w[p + 2] = stamp as u32;
            let mat = self.PrimMaterial(p);
            if self.w[mat + 1] & rin.mask1 == 0 && self.w[mat + 2] & rin.mask2 == 0 {
                continue;
            }
            let nv = self.PrimCount(p);
            let n = self.PrimNormal(p);
            let dn = n[2] * dv[2] + n[0] * dv[0] + n[1] * dv[1];
            // back faces only for double sided materials (flags bit0)
            if !(dn <= 0.0 || self.w[mat] & 1 != 0) {
                continue;
            }
            if (dn.abs() as f64) < 0.0005 {
                continue;
            }
            let tt = (-self.PrimD(p) - (n[2] * o[2] + n[0] * o[0] + n[1] * o[1])) / dn;
            if !(0.0 <= tt && tt <= 1.0) {
                continue;
            }
            let hp = [dv[0] * tt, dv[1] * tt, dv[2] * tt];
            let mut i = 0;
            while i < nv {
                let e = self.PrimEdgePlane(p, i);
                if 0.0 < e[3] + (o[2] + hp[2]) * e[2] + (o[0] + hp[0]) * e[0] + (o[1] + hp[1]) * e[1] {
                    break;
                }
                i += 1;
            }
            if i == nv && (tt < *t || out.callback.is_some()) {
                *t = tt;
                res = out.callback.is_none();
                out.hit.prim = Some(p);
                out.hit.normal = n;
                if let Some(cb) = out.callback.as_mut() {
                    let tv = *t;
                    out.hit.pos = [o[0] + dv[0] * tv, o[1] + dv[1] * tv, o[2] + dv[2] * tv];
                    if !cb(&out.hit, 2) {
                        *t = -1.0;
                        out.callback = None;
                        return true;
                    }
                    *t = 1.0;
                }
            }
        }
        res
    }
}

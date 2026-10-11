//! EFCollisionStage (Classes/EF/EFCollisionStage.cpp): ValidateTranslation, InitColPrims,
//! CheckPush, SolveMultipleCollisions, FindWeightedAvgNormal (free fn 0x002637a0).
//! CheckMultipleCollisions (0x001dddc0) needs EFPhysic state and lives in system.rs
//! (`EFCollisionSystem::Stage_CheckMultipleCollisions`).
//!
//! Seams: EFPhysic (WP-E2) is abstracted by `StagePhysic` (prim list +0x58/+0x5c, child physics of
//! the EFInstance +0xac child chain with flags bit 2).
#![allow(non_snake_case)]

use super::collision_map::EFCollisionMap;
use super::objcoll_check::EnvPrim;
use super::objcoll_prim::V3;

/// EFPhysic view needed by InitColPrims.
pub trait StagePhysic {
    /// physic+0x5c count, physic+0x58[i] primitive pointer (here an opaque id; 0 when the array is NULL)
    fn col_prims(&self) -> Vec<u32>;
    /// EFInstance(+0xac)->first child (+0x24) .. next (+0x20) with child flags (+0x18) & 2 and physic (+0x28) != NULL
    fn pushable_children(&self) -> Vec<&dyn StagePhysic>;
    /// opaque id of this EFPhysic
    fn id(&self) -> u32;
}

/// Maximum collision primitives of a stage: this+0x40 .. this+0x7c = 15 slots.
pub const STAGE_MAX_PRIMS: usize = 15;

/// EFPushReport (0x1c bytes, +0x18 next): push-out of an overlap.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct EFPushReport {
    /// +0x00 direction (CheckPush multiplies it by the depth)
    pub v: V3,
    /// +0x0c depth (CheckPush: then squared length)
    pub depth: f32,
    /// +0x10 priority of the pusher (int; env = 100, stored as the float bits 1.4013e-43)
    pub prio: i32,
    /// +0x14 weight of the pusher (env = 1.0)
    pub weight: f32,
}

/// EFContactReport (0x1c bytes, +0x18 next): a resting contact plane.
#[derive(Debug, Clone, Default)]
pub struct EFContactReport {
    /// +0x00 normal
    pub n: V3,
    /// +0x0c other EFPhysic (PhysId, 0 = none); SolveMultipleCollisions reads its velocity +0x28
    pub other: u32,
    /// +0x10 other object primitive: (owner uid, prim index)
    pub prim: Option<(u32, usize)>,
    /// +0x14 env primitive
    pub env: Option<EnvPrim>,
}

#[derive(Debug, Clone, Default)]
pub struct EFCollisionStage {
    /// this+0x4 env prim pointers, this+0x8 count (here: collision-map primitive word offsets)
    pub env_prims: Vec<usize>,
    /// this+0x3c count, this+0x40[i] primitive, this+0x7c[i] owning EFPhysic
    pub col_prims: Vec<(u32, u32)>,
    /// this+0x00 EFDynamicPhysic*: (pass, index) in EFUpdatePhysicStruct
    pub dyn_ref: (usize, usize),
    /// this+0x4/+0x8 as EFEnvCollisionPrim copies (BuildEnvPrimsLists; may span several maps)
    pub env: Vec<EnvPrim>,
    /// this+0xc/+0x10 and +0x14/+0x18: collidable dyn lists (pass, index)
    pub lists: [Vec<(usize, usize)>; 2],
    /// +0x24 active
    pub active: bool,
    /// +0x28 dyn radius squared
    pub r2: f32,
    /// +0x2c push report count
    pub push_count: u32,
    /// +0x30 accumulated translation (EFCollisionSystem::Update)
    pub moved: V3,
    /// +0xf4 collision reports (indices into the system's report arena, t-sorted)
    pub reports: Vec<usize>,
    /// +0xf8 time step (0.001/|v| or dt)
    pub step: f32,
    /// +0xfc smallest report t
    pub min_t: f32,
    /// +0x100 push reports (index 0 = list head)
    pub pushes: Vec<EFPushReport>,
    /// +0x104 contact reports (index 0 = list head)
    pub contacts: Vec<EFContactReport>,
}

/// C++ dot order (z*z + x*x) + y*y
#[inline]
fn dz(a: V3, b: V3) -> f32 {
    a[2] * b[2] + a[0] * b[0] + a[1] * b[1]
}
#[inline]
fn sc(a: V3, s: f32) -> V3 {
    [a[0] * s, a[1] * s, a[2] * s]
}
#[inline]
fn sb(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
#[inline]
fn ad(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
/// `_dpflt(fabs((double)x), c) == 0`
#[inline]
fn not_tiny(x: f32, c: u64) -> bool {
    !((x as f64).abs() < f64::from_bits(c))
}
#[inline]
fn normalized(a: V3) -> V3 {
    let l = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt();
    if l > 0.0 { sc(a, 1.0 / l) } else { a }
}

impl EFCollisionStage {
    /// ValidateTranslation(pos, &delta, mask1, mask2) (0x001de0d0): clips `delta` against the env
    /// polygons whose material mask1 (word 1) & m1 or mask2 (word 2) & m2; true when shortened (t<1).
    pub fn ValidateTranslation(&self, map: &EFCollisionMap, pos: V3, delta: &mut V3, m1: u32, m2: u32) -> bool {
        let mut best = 2.0f32;
        for &p in &self.env_prims {
            let mat = map.PrimMaterial(p);
            let flags = map.MaterialWord(mat, 0);
            if map.MaterialWord(mat, 1) & m1 == 0 && map.MaterialWord(mat, 2) & m2 == 0 {
                continue;
            }
            let n = map.PrimNormal(p);
            let dn = n[2] * delta[2] + n[0] * delta[0] + n[1] * delta[1];
            if dn == 0.0 {
                continue;
            }
            let dist = map.PrimD(p) + n[2] * pos[2] + n[0] * pos[0] + n[1] * pos[1];
            if (dist.to_bits() ^ dn.to_bits()) & 0x8000_0000 == 0 {
                continue;
            }
            if !(dn <= 0.0 || flags & 1 != 0) {
                continue;
            }
            let (mut a, mut b) = (dist, dn);
            if dist.to_bits() & 0x8000_0000 != 0 {
                a = -dist;
                b = -dn;
            }
            a = if 0.01 < a { a - 0.01 } else { 0.0 };
            let t = a / -b;
            if t < best {
                let q = [pos[0] + delta[0] * t, pos[1] + delta[1] * t, pos[2] + delta[2] * t];
                let mut inside = true;
                for e in 0..map.PrimCount(p) {
                    let ep = map.PrimEdgePlane(p, e);
                    if ep[3] + q[2] * ep[2] + q[0] * ep[0] + q[1] * ep[1] > 0.01 {
                        inside = false;
                        break;
                    }
                }
                if inside {
                    best = t;
                }
            }
        }
        if best < 1.0 {
            *delta = [delta[0] * best, delta[1] * best, delta[2] * best];
        }
        best < 1.0
    }

    /// InitColPrims(EFPhysic*) (0x001de300): appends every primitive of the physic, then recurses
    /// into the pushable children. quirk: no bound check against the 15 slots.
    pub fn InitColPrimsFor(&mut self, ph: &dyn StagePhysic) {
        for pr in ph.col_prims() {
            self.col_prims.push((pr, ph.id()));
        }
        for c in ph.pushable_children() {
            self.InitColPrimsFor(c);
        }
    }
    /// InitColPrims() (0x001de3e0): count = 0, then from the stage root physic (*this).
    pub fn InitColPrims(&mut self, root: &dyn StagePhysic) {
        self.col_prims.clear();
        self.InitColPrimsFor(root);
    }

    /// ValidateTranslation (0x001ddfa0) over the `env` copies: same code as ValidateTranslation
    /// (which reads a single map), used by EFCollisionSystem whose env list can span maps.
    pub fn ValidateTranslationEnv(&self, pos: V3, delta: &mut V3, m1: u32, m2: u32) -> bool {
        let mut best = 2.0f32;
        for e in &self.env {
            if e.mat[1] & m1 == 0 && e.mat[2] & m2 == 0 {
                continue;
            }
            let n = e.normal;
            let dn = n[2] * delta[2] + n[0] * delta[0] + n[1] * delta[1];
            if dn == 0.0 {
                continue;
            }
            let dist = e.d + n[2] * pos[2] + n[0] * pos[0] + n[1] * pos[1];
            if (dist.to_bits() ^ dn.to_bits()) & 0x8000_0000 == 0 {
                continue;
            }
            if !(dn <= 0.0 || e.mat[0] & 1 != 0) {
                continue;
            }
            let (mut a, mut b) = (dist, dn);
            if dist.to_bits() & 0x8000_0000 != 0 {
                a = -dist;
                b = -dn;
            }
            a = if 0.01 < a { a - 0.01 } else { 0.0 };
            let t = a / -b;
            if t < best {
                let q = [pos[0] + delta[0] * t, pos[1] + delta[1] * t, pos[2] + delta[2] * t];
                // quirk: loop bound = raw u16 count (bit 15 included); only stored edges exist here
                let inside = e.edges.iter().take(e.nverts as usize).all(|ep| ep[3] + q[2] * ep[2] + q[0] * ep[0] + q[1] * ep[1] <= 0.01);
                if inside {
                    best = t;
                }
            }
        }
        if best < 1.0 {
            *delta = [delta[0] * best, delta[1] * best, delta[2] * best];
        }
        best < 1.0
    }

    /// CheckPush() (0x001dca50). Returns the translation the C++ passes to
    /// EFCollisionModel::Translate on the stage physic's model (None = no Translate call);
    /// always frees the push list and zeroes the push count (+0x2c).
    pub fn CheckPush(&mut self) -> Option<V3> {
        if self.pushes.is_empty() {
            return None;
        }
        let mut out = None;
        if self.pushes.len() == 1 {
            let p = self.pushes[0];
            // _dpfgt((double)depth, 0x3f1a36e2eb1c432d = 1e-4)
            if (p.depth as f64) > f64::from_bits(0x3f1a_36e2_eb1c_432d) {
                out = Some(sc(p.v, p.depth));
            }
        } else {
            let mut acc = [0.0f32; 3];
            for p in self.pushes.iter_mut() {
                let d = p.depth;
                p.v = sc(p.v, d);
                p.depth = p.depth * p.depth;
            }
            for _ in 0..3 {
                let mut sum = [0.0f32; 3];
                // priority: int compare (slt at 0x001dcbe8) against the -1 sentinel (Ghidra: -NAN)
                let mut best: i32 = -1;
                for p in &self.pushes {
                    if !(p.depth < 1e-6) && best <= p.prio {
                        if best == p.prio {
                            sum = ad(sum, sc(p.v, p.weight));
                        } else {
                            sum = sc(p.v, p.weight);
                            best = p.prio;
                        }
                    }
                }
                if best == -1 {
                    break;
                }
                let (mut wsum, mut maxw, mut mx, mut mn) = (0.0f32, 0.0f32, -10000.0f32, 10000.0f32);
                for p in &self.pushes {
                    if p.prio == best {
                        let d = dz(sum, p.v);
                        if !(d <= mx) {
                            mx = d;
                        }
                        if !(mn <= d) {
                            mn = d;
                        }
                        if !(p.weight <= maxw) {
                            maxw = p.weight;
                        }
                        wsum += d * p.weight;
                    }
                }
                let avg = wsum / maxw;
                let mut r = avg;
                if 0.0 < mx {
                    r = mx;
                    if avg <= mx {
                        r = avg;
                    }
                }
                let mut r2 = r;
                if mn < 0.0 {
                    r2 = mn;
                    if mn <= r {
                        r2 = r;
                    }
                }
                let mut s2 = dz(sum, sum);
                if 0.0 < s2 {
                    let scaled = sc(sum, r2 / s2);
                    sum = scaled;
                    let a2 = dz(acc, acc);
                    if a2 < s2 {
                        let f = (s2.sqrt() - a2.sqrt()) / s2.sqrt();
                        sum = sc(scaled, f);
                        acc = ad(acc, sum);
                        s2 = dz(sum, sum);
                    }
                }
                // quirk: when s2 == 0 the projection below divides by zero (NaN), as in the C++
                for p in self.pushes.iter_mut() {
                    let f = dz(p.v, sum) / s2;
                    p.v = sb(p.v, sc(sum, f));
                    p.depth = dz(p.v, p.v);
                }
            }
            out = Some(acc);
        }
        self.pushes.clear();
        self.push_count = 0;
        out
    }

    /// SolveMultipleCollisions(DTVector3d&) (0x001dd120): constrains the velocity `v` by the
    /// contact planes (+0x104 list; needs >= 2 contacts): slide on one plane, follow the crease of
    /// two opposing planes, stop against a third. `vel_of(physic)` = EFPhysic+0x28 of a
    /// contact's other physic (moving contacts only constrain the relative part).
    pub fn SolveMultipleCollisions(&self, v: &mut V3, vel_of: &dyn Fn(u32) -> V3) {
        const T4: u64 = 0x3f1a_36e2_e000_0000; // 1e-4 rounded to float
        const T5: u64 = 0x3ee4_f8b5_8000_0000; // 1e-5 rounded to float
        let cs = &self.contacts;
        if cs.len() < 2 {
            return;
        }
        let mut res = *v;
        'outer: for (i1, c1) in cs.iter().enumerate() {
            let n1 = c1.n;
            let mut w = res;
            let mut a = [0.0f32; 3];
            let f12 = dz(n1, res);
            if !(f12 <= 0.0 && not_tiny(f12, T4)) {
                continue;
            }
            if c1.other != 0 {
                let f11 = dz(n1, vel_of(c1.other));
                if f11 < 0.0 {
                    if f11 <= f12 {
                        continue 'outer;
                    }
                    a = sc(w, f11 / f12);
                    w = sb(w, a);
                }
            }
            w = sb(w, sc(n1, dz(n1, w)));
            let wn = w; // fStack_1f0
            for (i2, c2) in cs.iter().enumerate() {
                if i2 == i1 {
                    continue;
                }
                let n2 = c2.n;
                let mut u = w;
                let mut b = [0.0f32; 3];
                let mut q = wn;
                let mut f = dz(n2, w);
                if !(f <= 0.0 && not_tiny(f, T4)) {
                    continue;
                }
                if c2.other != 0 {
                    let f11 = dz(n2, vel_of(c2.other));
                    if f11 < 0.0 {
                        if f11 <= f {
                            // `goto code_r0x001ddc98` = end of this loop body
                            continue;
                        }
                        b = sc(u, f11 / f);
                        f = (f - f11) / f;
                        u = sb(u, b);
                        q = sc(q, f);
                    }
                }
                u = sb(u, sc(n2, dz(n2, u)));
                if dz(n1, n2) < 0.0 {
                    let cr = [n1[1] * n2[2] - n1[2] * n2[1], n1[2] * n2[0] - n1[0] * n2[2], n1[0] * n2[1] - n1[1] * n2[0]];
                    let l2 = dz(cr, cr);
                    if 1e-5 < l2 {
                        let line = sc(sc(cr, dz(q, cr)), 1.0 / l2);
                        u = line;
                        let dir = normalized(sb(line, q));
                        for (i3, c3) in cs.iter().enumerate() {
                            if i3 == i2 || i3 == i1 {
                                continue;
                            }
                            let n3 = c3.n;
                            let mut z = u;
                            let mut e = [0.0f32; 3];
                            let mut k = line;
                            let mut g = dz(n3, line);
                            if !(g <= 0.0 && not_tiny(g, T5)) {
                                continue;
                            }
                            if c3.other != 0 {
                                let f11 = dz(n3, vel_of(c3.other));
                                if f11 < 0.0 {
                                    if f11 <= g {
                                        // `goto code_r0x001ddbb8` = end of this loop body
                                        continue;
                                    }
                                    e = sc(z, f11 / g);
                                    g = (g - f11) / g;
                                    z = sb(z, e);
                                    k = sc(k, g);
                                }
                            }
                            if dz(k, n3) < 0.0 && dz(dir, n3) < 0.0 {
                                z = [0.0; 3];
                            }
                            u = ad(z, e);
                            if dz(u, u) < 1e-6 {
                                u = [0.0; 3];
                                break;
                            }
                        }
                    } else {
                        u = [0.0; 3];
                    }
                }
                w = ad(u, b);
                if dz(w, w) < 1e-6 {
                    w = [0.0; 3];
                    break;
                }
            }
            res = ad(w, a);
            if dz(res, res) < 1e-6 {
                res = [0.0; 3];
                break;
            }
        }
        *v = res;
    }

    /// EFCollisionStage_FindWeightedAvgNormal(stage, pos, mask, &normal, &dist) (0x002637a0, free
    /// function, caller EFDMClimb): average of the normals of the env polygons (material word1 &
    /// mask) near `pos`, weighted by 1/max(d²,0.01)²; `dist` = smallest weight distance (init 11).
    pub fn FindWeightedAvgNormal(&self, pos: V3, mask: u32, normal: &mut V3, dist: &mut f32) -> bool {
        *dist = 11.0;
        *normal = [0.0; 3];
        let mut count = 0;
        for e in &self.env {
            if e.mat[1] & mask == 0 {
                continue;
            }
            let mut maxd = 0.0f32;
            let mut best = 0usize;
            for (k, ep) in e.edges.iter().take(e.nverts as usize).enumerate() {
                let d = ep[3] + pos[2] * ep[2] + pos[0] * ep[0] + pos[1] * ep[1];
                if !(d <= maxd) {
                    maxd = d;
                    best = k;
                }
            }
            let mut kind;
            let mut f;
            if maxd == 0.0 {
                // inside every edge: plane distance
                let pd = e.d + e.normal[2] * pos[2] + e.normal[0] * pos[0] + e.normal[1] * pos[1];
                if 10.0 < pd * pd {
                    kind = 0;
                    f = 10.0;
                } else {
                    kind = 1;
                    f = pd * pd;
                }
            } else {
                let mut nx = best + 1;
                if nx == e.nverts as usize {
                    nx = 0;
                }
                let v0 = e.verts.get(best).copied().unwrap_or_default();
                let v1 = e.verts.get(nx).copied().unwrap_or_default();
                let ed = sb(v1, v0);
                let mut r = sb(v0, pos);
                let p = dz(ed, r);
                let l2 = dz(ed, ed);
                if 0.0 < p {
                    kind = 3;
                } else if p < -l2 {
                    kind = 3;
                    r = ad(r, ed);
                } else {
                    kind = 2;
                    r = sb(r, sc(ed, p / l2));
                }
                f = dz(r, r);
                if 10.0 < f {
                    kind = 0;
                    f = 10.0;
                }
            }
            if kind != 0 {
                let w = if 0.01 <= f { f } else { 0.01 };
                count += 1;
                *normal = ad(*normal, sc(e.normal, 1.0 / (w * w)));
                if w < *dist {
                    *dist = w;
                }
            }
        }
        if count != 0 {
            *normal = normalized(*normal);
        }
        count != 0
    }
}

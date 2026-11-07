//! WP-S4: projector rendering (texture projection onto the static world).
//!
//! Original chain, per frame (EFGraphicsMgr_SP render):
//! - EFProjectorMgr_BC::Begin (0x0024c260): occurrence count = 0, every projector -> EFProjector::Begin
//! - EFProjector::Begin (0x0026e290): texture != NULL && visible -> EFProjector_BC::Begin builds the
//!   `_sProjectorOccurence` (0x64 bytes), RpWorldForAllWorldSectorIntersections(sphere) marks the
//!   hit sectors (+0x8e = frame counter), AddOccurence (max 8).
//! - EFProjectorMgr_SP::WorldSectorCallBackRender: marked sectors -> BSPMapper pipeline (sector
//!   drawn + projected texture pass), the rest -> simple BSP pipeline.
//! - BSPMapperPipelineDataUpload: projector camera view matrix (parallel flag = RwCamera +0x14 == 2),
//!   TEX0 of the texture, ALPHA_1 by blend function (0: 0x68 Cs+Cd, 1: 0xa1 Cd-Cs, 2: 0x44
//!   (Cs-Cd)As+Cd, FIX 0x80), RGBA = color * 128, the two clip planes (occurrence +0, +0x10).
//!
//! The VU1 micro code (texture coordinate generation) is not in the decompiled source; the UVs are
//! the RW camera projection of the projector camera: camera-space x/y over the view window, mapped
//! to [0,1] (RW screen x runs along -right, y along -up), perspective divides by z.
#![allow(non_snake_case)]

use crate::engine::camera::DTMatrix;
use crate::engine::rw::world::RpWorld;

/// EFProjectorMgr_BC::_sProjectorOccurence (0x64 bytes)
#[derive(Debug, Clone, PartialEq)]
pub struct ProjectorOccurence {
    /// +0x00 near plane (at, -at.pos): >= 0 in front of the projector
    pub near: [f32; 4],
    /// +0x10 far plane (-at, at.(pos + reach at))
    pub far_plane: [f32; 4],
    /// +0x20 projector position
    pub pos: [f32; 3],
    /// +0x30 pos + reach * at
    pub far: [f32; 3],
    /// +0x40 color rgba
    pub color: [f32; 4],
    /// +0x50 texture (name)
    pub texture: String,
    /// +0x58 projector camera: LTM (frame), view window, parallel projection
    pub ltm: DTMatrix,
    pub view_window: [f32; 2],
    pub parallel: bool,
    /// +0x5c blending function
    pub blend: u32,
    /// +0x60 reach distance
    pub reach: f32,
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// EFProjector_BC::Begin (0x0024cea0): the occurrence of one projector (frame LTM `m`)
pub fn ProjectorBegin(p: &crate::engine::host::projector::EFProjector_BC, m: &DTMatrix) -> Option<ProjectorOccurence> {
    let tex = p.texture.clone()?;
    if p.flags & 1 == 0 {
        return None;
    }
    let at = m.at;
    let r = p.reach;
    let far: [f32; 3] = std::array::from_fn(|i| m.pos[i] + r * at[i]);
    Some(ProjectorOccurence {
        // -(pos.z*at.z + pos.x*at.x + pos.y*at.y), operand order as compiled
        near: [at[0], at[1], at[2], -(m.pos[2] * at[2] + m.pos[0] * at[0] + m.pos[1] * at[1])],
        far_plane: [
            -at[0],
            -at[1],
            -at[2],
            -(-at[2] * (m.pos[2] + r * at[2]) + -at[0] * (m.pos[0] + r * at[0]) + -at[1] * (m.pos[1] + r * at[1])),
        ],
        pos: m.pos,
        far,
        color: p.color,
        texture: tex,
        ltm: *m,
        view_window: p.size,
        // SetCameraProjection(type == 0 ? 1 : 0): EF 1 -> rwPARALLEL
        parallel: p.projection == 0,
        blend: p.blend,
        reach: r,
    })
}

/// EFProjectorMgr_BC::Begin + AddOccurence: occurrences in hash-table order, at most 8
pub fn ProjectorMgrBegin(
    mgr: &crate::engine::host::projector::EFProjectorMgr_BC,
    ltm: impl Fn(u32) -> Option<DTMatrix>,
) -> Vec<ProjectorOccurence> {
    let mut out = Vec::new();
    for (h, p) in &mgr.projectors {
        let Some(m) = ltm(*h) else { continue };
        if let Some(o) = ProjectorBegin(p, &m) {
            if out.len() < 8 {
                out.push(o);
            }
        }
    }
    out
}

impl ProjectorOccurence {
    /// EFProjector::Begin intersection sphere: center = pos + (far - pos) / 2, radius = |far - pos|
    /// (quirk: the full length, not half)
    pub fn sphere(&self) -> ([f32; 3], f32) {
        let d: [f32; 3] = std::array::from_fn(|i| self.far[i] - self.pos[i]);
        let c = std::array::from_fn(|i| self.pos[i] + d[i] * 0.5);
        (c, (d[2] * d[2] + d[0] * d[0] + d[1] * d[1]).sqrt())
    }
    /// RpWorldForAllWorldSectorIntersections(sphere): sectors whose bbox meets the sphere
    pub fn sectors(&self, w: &RpWorld) -> Vec<usize> {
        let (c, r) = self.sphere();
        w.sectors
            .iter()
            .enumerate()
            .filter(|(_, s)| {
                let mut d2 = 0.0;
                for i in 0..3 {
                    let v = c[i].clamp(s.bbox_inf[i], s.bbox_sup[i]) - c[i];
                    d2 += v * v;
                }
                d2 <= r * r
            })
            .map(|(i, _)| i)
            .collect()
    }
    /// projector camera projection of a world point -> (u, v)
    pub fn uv(&self, p: [f32; 3]) -> [f32; 2] {
        let d: [f32; 3] = std::array::from_fn(|i| p[i] - self.ltm.pos[i]);
        let (x, y, z) = (dot(d, self.ltm.right), dot(d, self.ltm.up), dot(d, self.ltm.at));
        let s = if self.parallel || z == 0.0 { 1.0 } else { z };
        [0.5 - 0.5 * x / (self.view_window[0] * s), 0.5 - 0.5 * y / (self.view_window[1] * s)]
    }
    /// The projected-texture pass geometry: the hit sectors' triangles clipped to the near/far
    /// planes and to the texture square (GS clamp), as (positions, uvs) triangle list.
    pub fn mesh(&self, w: &RpWorld) -> (Vec<[f32; 3]>, Vec<[f32; 2]>) {
        self.mesh_sub(w, if self.parallel { 1 } else { SUB })
    }
    /// P37 R3: [`Self::mesh`] without the perspective UV split (the clipped fan triangles as they are, UVs
    /// per vertex): for materials that project per fragment from the world position (the shadow pass)
    pub fn mesh_flat(&self, w: &RpWorld) -> (Vec<[f32; 3]>, Vec<[f32; 2]>) {
        self.mesh_sub(w, 1)
    }
    fn mesh_sub(&self, w: &RpWorld, sub: usize) -> (Vec<[f32; 3]>, Vec<[f32; 2]>) {
        let (mut pos, mut uvs) = (Vec::new(), Vec::new());
        let (c, r) = self.sphere();
        for si in self.sectors(w) {
            for sm in &w.sectors[si].meshes {
                let vp = &sm.mesh.positions;
                for t in sm.indices.chunks_exact(3) {
                    let tri = [vp[t[0] as usize], vp[t[1] as usize], vp[t[2] as usize]];
                    // per-triangle sphere reject (cheap cull before clipping)
                    let mut lo = [f32::MAX; 3];
                    let mut hi = [f32::MIN; 3];
                    for v in &tri {
                        for i in 0..3 {
                            lo[i] = lo[i].min(v[i]);
                            hi[i] = hi[i].max(v[i]);
                        }
                    }
                    let d2: f32 = (0..3).map(|i| (c[i].clamp(lo[i], hi[i]) - c[i]).powi(2)).sum();
                    if d2 > r * r {
                        continue;
                    }
                    self.clip_tri(tri, &mut pos, &mut uvs, sub);
                }
            }
        }
        (pos, uvs)
    }
    /// DFF mapper pass (EFProjectorMgr_SP::DFFMapper*/SkinnedDFFMapper*): world-space triangles of a
    /// receiver object (projector receiver, logic flag 0x200), same clipping/UVs as the world pass
    pub fn mesh_tris(&self, tris: impl Iterator<Item = [[f32; 3]; 3]>) -> (Vec<[f32; 3]>, Vec<[f32; 2]>) {
        let (mut pos, mut uvs) = (Vec::new(), Vec::new());
        for t in tris {
            self.clip_tri(t, &mut pos, &mut uvs, if self.parallel { 1 } else { SUB });
        }
        (pos, uvs)
    }
    /// P37 R8: clip plane k of the occurrence: 0 near, 1 far, 2..5 the texture square (the arithmetic
    /// of the former per-plane closures, unchanged)
    fn plane_d(&self, k: usize, p: [f32; 3]) -> f32 {
        match k {
            0 => dot([self.near[0], self.near[1], self.near[2]], p) + self.near[3],
            1 => dot([self.far_plane[0], self.far_plane[1], self.far_plane[2]], p) + self.far_plane[3],
            _ => {
                let m = &self.ltm;
                let (vw, vh) = (self.view_window[0], self.view_window[1]);
                let d: [f32; 3] = std::array::from_fn(|i| p[i] - m.pos[i]);
                let s = if self.parallel { 1.0 } else { dot(d, m.at) };
                match k {
                    2 => vw * s - dot(d, m.right),
                    3 => vw * s + dot(d, m.right),
                    4 => vh * s - dot(d, m.up),
                    _ => vh * s + dot(d, m.up),
                }
            }
        }
    }
    fn clip_tri(&self, tri: [[f32; 3]; 3], pos: &mut Vec<[f32; 3]>, uvs: &mut Vec<[f32; 2]>, sub: usize) {
        // P37 R8: whole-triangle early out before any polygon Vec. Sutherland-Hodgman (`clip`) of a
        // polygon whose vertices are all outside a plane (!(d >= 0), NaN included) is empty, and of
        // one whose vertices are all inside is the same polygon; so while the raw triangle is wholly
        // inside the planes so far, a plane that has it wholly outside rejects it with exactly the
        // result of the full path (same plane arithmetic, `plane_d`).
        for k in 0..6 {
            let inside = tri.map(|p| self.plane_d(k, p) >= 0.0);
            if inside == [false; 3] {
                return;
            }
            if inside != [true; 3] {
                break;
            }
        }
        // polygon of (position, uv)
        let mut poly: Vec<([f32; 3], [f32; 2])> = tri.iter().map(|&p| (p, [0.0; 2])).collect();
        // near, far, then the texture square |x| <= vw s, |y| <= vh s (s = z, or 1 parallel): linear in position
        for k in 0..6 {
            poly = clip(&poly, |v| self.plane_d(k, v.0));
            if poly.is_empty() {
                return;
            }
        }
        // P26 R22: the GS maps the projected texture with STQ (perspective-correct q = 1/z); the
        // mesh carries plain affine UVs, so a perspective projector's polygon (the frustum footprint,
        // strongly oblique on the EP4/L02A water) put the spot at the affine centre of the trapezoid,
        // too far and too big. Each fan triangle is split into SUB x SUB triangles with exact
        // per-vertex projection UVs, so the affine error per sub-triangle is small.
        // (sub = SUB, or 1 for parallel projectors / mesh_flat; the constant is module level, P37 R3)
        let n = sub;
        let lerp = |a: [f32; 3], b: [f32; 3], c: [f32; 3], i: usize, j: usize| -> [f32; 3] {
            let (u, v) = (i as f32 / n as f32, j as f32 / n as f32);
            std::array::from_fn(|k| a[k] + (b[k] - a[k]) * u + (c[k] - a[k]) * v)
        };
        for t in 1..poly.len() - 1 {
            let (a, b, c) = (poly[0].0, poly[t].0, poly[t + 1].0);
            for i in 0..n {
                for j in 0..n - i {
                    // P37 R7: pushed in place (was a Vec per sub-triangle); same vertices, same order
                    let mut put = |p: [f32; 3]| {
                        pos.push(p);
                        uvs.push(self.uv(p));
                    };
                    put(lerp(a, b, c, i, j));
                    put(lerp(a, b, c, i + 1, j));
                    put(lerp(a, b, c, i, j + 1));
                    if i + j + 1 < n {
                        put(lerp(a, b, c, i + 1, j));
                        put(lerp(a, b, c, i + 1, j + 1));
                        put(lerp(a, b, c, i, j + 1));
                    }
                }
            }
        }
    }
}

/// per-edge split of a perspective projector's fan triangles (P26 R22, see clip_tri)
const SUB: usize = 8;

/// Sutherland-Hodgman against `d(v) >= 0`, linear in position and uv
fn clip(poly: &[([f32; 3], [f32; 2])], d: impl Fn(&([f32; 3], [f32; 2])) -> f32) -> Vec<([f32; 3], [f32; 2])> {
    let mut out = Vec::with_capacity(poly.len() + 2);
    for i in 0..poly.len() {
        let (a, b) = (poly[i], poly[(i + 1) % poly.len()]);
        let (da, db) = (d(&a), d(&b));
        if da >= 0.0 {
            out.push(a);
        }
        if (da >= 0.0) != (db >= 0.0) {
            let t = da / (da - db);
            out.push((
                std::array::from_fn(|k| a.0[k] + (b.0[k] - a.0[k]) * t),
                std::array::from_fn(|k| a.1[k] + (b.1[k] - a.1[k]) * t),
            ));
        }
    }
    out
}

#[cfg(test)]
mod p37_tests {
    use super::*;

    fn area(p: &[[f32; 3]]) -> f32 {
        p.chunks_exact(3)
            .map(|t| {
                let (a, b): ([f32; 3], [f32; 3]) = (std::array::from_fn(|i| t[1][i] - t[0][i]), std::array::from_fn(|i| t[2][i] - t[0][i]));
                let c = [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
                0.5 * dot(c, c).sqrt()
            })
            .sum()
    }

    /// P37 R3: the shadow pass's flat receiver (n = 1) covers exactly the surface of the subdivided one
    /// (n = SUB) of a perspective (point-light) projector, with 1/SUB^2 of the triangles; its vertex UVs
    /// are the exact projection (shadow.wgsl projects per fragment anyway).
    #[test]
    fn p37_flat_receiver_same_surface() {
        let m = DTMatrix { right: [1.0, 0.0, 0.0], up: [0.0, 0.0, 1.0], at: [0.0, -1.0, 0.0], pos: [0.3, 3.0, -0.2] };
        let o = ProjectorOccurence {
            near: [0.0, -1.0, 0.0, 3.0],
            far_plane: [0.0, 1.0, 0.0, 2.0],
            pos: m.pos,
            far: [0.3, -2.0, -0.2],
            color: [0.5; 4],
            texture: String::new(),
            ltm: m,
            view_window: [0.6, 0.6],
            parallel: false,
            blend: 0,
            reach: 5.0,
        };
        let floor = [[-5.0, 0.0, -5.0], [5.0, 0.0, -5.0], [0.0, 0.0, 5.0]];
        let (mut ps, mut us, mut pf, mut uf) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        o.clip_tri(floor, &mut ps, &mut us, SUB);
        o.clip_tri(floor, &mut pf, &mut uf, 1);
        assert!(!pf.is_empty());
        assert_eq!(ps.len(), pf.len() * SUB * SUB);
        let (a, b) = (area(&ps), area(&pf));
        assert!((a - b).abs() < 1e-3 * b && b > 1.0, "{a} {b}");
        for (p, u) in pf.iter().zip(&uf) {
            assert_eq!(*u, o.uv(*p));
            assert!(u.iter().all(|c| (-1e-4..=1.0001).contains(c)), "{u:?}");
        }
    }
}

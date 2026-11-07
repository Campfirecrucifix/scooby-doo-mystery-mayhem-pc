//! WP-S5: EFShadowMgr (0x0024f8d0..) caster pass for logic flag 0x400 (EFGameObject::SetLogicFlags
//! 0xc00 -> AddCaster). Ported 1:1: EFShadowMgr::Begin decision (max 8 shadows, _ShadowParams
//! +0x17c bit 4 = off, bit 2 = parallel cast vs drop-down), EFShadowMgr_SP::InitCaster (mean of
//! the atomic bounding-sphere centres + enclosing radius), PrepareParallelCamera (camera frame,
//! clip planes, view window = radius, near/far points) -> a projector occurrence with blend 1
//! (Cd - Cs, colour = params+0x184) that goes through the projector pass (gfx::projector).
//!
//! APPROXIMATED (VU1/GS code, not in the decompiled source): EFShadowMgr_SP::Cast renders the
//! caster atomics into the 64/128/256 raster through the DFF / skinned-DFF shadow micro code, then
//! ApplySoftener blurs it. Here [`CastSilhouette`] rasterises the caster's drawn triangles with the
//! occurrence's own projection on the CPU, then runs the decoded GS softener passes ([`ApplySoftener`]).
//! Flag 0x800 per-light shadows are ported ([`ShadowBeginLights`] -> EFShadowMgr::Prepare +
//! PreparePerspectiveCamera + AdjustShadowIntensity). Still approximated: the drop-down texture
//! path (+0x17c bit 2 clear: the C uses the blob texture this+0x20 created by
//! EFShadowMgr_SP::Initialize); the drop-down case uses the cast silhouette too.
#![allow(non_snake_case)]

use super::projector::ProjectorOccurence;
use crate::engine::camera::DTMatrix;

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
fn norm(a: [f32; 3]) -> [f32; 3] {
    let l = dot(a, a).sqrt();
    if l > 0.0 { [a[0] / l, a[1] / l, a[2] / l] } else { a }
}

/// EFShadowMgr_BC::_sObjectEntry +0xc centre (local to the object frame here) / +0x18 radius
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CasterEntry {
    pub center: [f32; 3],
    pub radius: f32,
}

/// EFShadowMgr_SP::InitCaster: centre = mean of the atomic sphere centres, radius = max
/// (|centre - c_i| + r_i). `spheres` = (centre, radius) of the caster atomics (object space).
pub fn InitCaster(spheres: &[([f32; 3], f32)]) -> CasterEntry {
    let n = spheres.len() as f32;
    let mut c = [0.0f32; 3];
    for s in spheres {
        for i in 0..3 {
            c[i] += s.0[i];
        }
    }
    for v in c.iter_mut() {
        *v /= n;
    }
    let mut r = 0.0f32;
    for s in spheres {
        let d = [c[0] - s.0[0], c[1] - s.0[1], c[2] - s.0[2]];
        let v = (d[2] * d[2] + d[0] * d[0] + d[1] * d[1]).sqrt() + s.1;
        if v > r {
            r = v;
        }
    }
    CasterEntry { center: c, radius: r }
}

/// EFShadowMgr::PrepareParallelCamera (0x00250a20). `light` = the light frame LTM (drop-down:
/// this+0x1c identity frame with at = (0,-1,0)), `o` = caster EF3dObj frame LTM, `reach` =
/// occurrence +0x60, `scale` = param_5 (1.0).
pub fn PrepareParallelCamera(light: &DTMatrix, o: &DTMatrix, e: &CasterEntry, reach: f32, scale: f32) -> (DTMatrix, [f32; 4], [f32; 4], [f32; 3], [f32; 3], f32) {
    let d = light.at;
    let cam_pos: [f32; 3] = std::array::from_fn(|i| e.center[i] + (o.pos[i] - e.radius * d[i]));
    let b4 = [o.pos[0], e.center[1] * 0.5 + o.pos[1], o.pos[2]];
    let c0: [f32; 3] = std::array::from_fn(|i| b4[i] + reach * d[i]);
    let vw = e.radius * scale;
    // camera frame: identity, translate (unless ~0), at = normalize(dir), right = Y x at (Z x at if short)
    let at = norm(d);
    let mut right = cross([0.0, 1.0, 0.0], at);
    if dot(right, right) < 0.4 {
        right = cross([0.0, 0.0, 1.0], at);
    }
    let right = norm(right);
    let up = cross(at, right);
    let ltm = DTMatrix { right, up, at, pos: cam_pos };
    let near = [at[0], at[1], at[2], -(b4[2] * at[2] + b4[0] * at[0] + b4[1] * at[1])];
    let far = [-at[0], -at[1], -at[2], -(c0[2] * -at[2] + c0[0] * -at[0] + c0[1] * -at[1])];
    (ltm, near, far, b4, c0, vw)
}

/// One shadow of EFShadowMgr::Begin
#[derive(Debug, Clone, PartialEq)]
pub struct ShadowOcc {
    pub caster: u32,
    /// raster size (_ShadowParams +0x24 / +0x190: 0x40, 0x80, 0x100)
    pub size: u32,
    pub occ: ProjectorOccurence,
}

/// EFShadowMgr::PreparePerspectiveCamera (0x00250f50): camera at the light, looking at the
/// caster centre; view window = tan of the sphere's half angle r / sqrt(d^2 - r^2); near plane
/// through the object position, far point = object + reach * at.
pub fn PreparePerspectiveCamera(light: &DTMatrix, o: &DTMatrix, e: &CasterEntry, reach: f32) -> (DTMatrix, [f32; 4], [f32; 4], [f32; 3], [f32; 3], f32) {
    let lp = light.pos;
    let v: [f32; 3] = std::array::from_fn(|i| (o.pos[i] + e.center[i]) - lp[i]);
    // quirk: EE FPU SQRT.S of a negative operand returns sqrt(|x|) (light inside the caster sphere: the
    // PS2 still gets a finite view window; IEEE sqrt would give NaN and drop the shadow)
    let vw = e.radius * (1.0 / ((v[2] * v[2] + v[0] * v[0] + v[1] * v[1]) - e.radius * e.radius).abs().sqrt());
    let at = norm(v);
    let mut right = cross([0.0, 1.0, 0.0], at);
    if dot(right, right) < 0.4 {
        right = cross([0.0, 0.0, 1.0], at);
    }
    let right = norm(right);
    let up = cross(at, right);
    let b0 = o.pos;
    let a4: [f32; 3] = std::array::from_fn(|i| b0[i] + reach * at[i]);
    let near = [at[0], at[1], at[2], -(b0[2] * at[2] + b0[0] * at[0] + b0[1] * at[1])];
    let far = [-at[0], -at[1], -at[2], -(a4[2] * -at[2] + a4[0] * -at[0] + a4[1] * -at[1])];
    (DTMatrix { right, up, at, pos: lp }, near, far, b0, a4, vw)
}

/// EFShadowMgr::AdjustShadowIntensity (0x00251980): fade with light distance / radius past the
/// params +0x28 threshold.
pub fn AdjustShadowIntensity(c: [f32; 4], p: &[u32; 11], light: [f32; 3], radius: f32, obj: [f32; 3]) -> [f32; 4] {
    let t = f32::from_bits(p[10]);
    let d = [light[0] - obj[0], light[1] - obj[1], light[2] - obj[2]];
    let mut f = ((d[2] * d[2] + d[0] * d[0] + d[1] * d[1]).sqrt() / radius - t) / (1.0 - t);
    if f > 1.0 {
        f = 1.0;
    } else if f < 0.0 {
        f = 0.0;
    }
    let k = 1.0 - f;
    [c[0] * k, c[1] * k, c[2] * k, c[3] * k]
}

/// A light of EFLightMgr_BC::GetLightList as EFShadowMgr::Begin filters it: (type, RpLight
/// flags, frame LTM, radius, kind +0x28, visible).
pub type ShadowLight = (u8, u8, DTMatrix, f32, u32, bool);

fn occ(caster: u32, size: u32, cam: (DTMatrix, [f32; 4], [f32; 4], [f32; 3], [f32; 3], f32), color: [f32; 4], parallel: bool, reach: f32) -> ShadowOcc {
    let (ltm, near, far_plane, pos, far, vw) = cam;
    let size = match size {
        0x40 | 0x80 | 0x100 => size,
        _ => 0x80,
    };
    ShadowOcc {
        caster,
        size,
        occ: ProjectorOccurence {
            near,
            far_plane,
            pos,
            far,
            color,
            texture: format!("#shadow{caster:08x}"),
            ltm,
            view_window: [vw, vw],
            parallel,
            blend: 1,
            reach,
        },
    }
}

/// EFShadowMgr::Prepare (0x0024fe80) for one light: point/spot lights only within their radius
/// (perspective cast + intensity fade), directional lights parallel along the light, others none.
pub fn Prepare(uid: u32, l: &ShadowLight, o: &DTMatrix, e: &CasterEntry, p: &[u32; 11]) -> Option<ShadowOcc> {
    let (ty, _, lm, radius, _, _) = *l;
    let c = f32::from_bits(p[7]);
    let color = [c, c, c, 1.0];
    let reach = f32::from_bits(p[8]);
    match ty {
        0x80 | 0x81 | 0x82 => {
            let d: [f32; 3] = std::array::from_fn(|i| lm.pos[i] - o.pos[i]);
            if radius * radius < d[2] * d[2] + d[0] * d[0] + d[1] * d[1] {
                return None;
            }
            let cam = PreparePerspectiveCamera(&lm, o, e, reach);
            Some(occ(uid, p[9], cam, AdjustShadowIntensity(color, p, lm.pos, radius, o.pos), false, reach))
        }
        1 => Some(occ(uid, p[9], PrepareParallelCamera(&lm, o, e, reach, 1.0), color, true, reach)),
        _ => None,
    }
}

/// EFShadowMgr::Begin. _ShadowParams words (+0x17c..): [0 flags, 1 lod, 2 color, 3 reach,
/// 4 dropdown param, 5 size, 6, 7 light color, 8 light reach, 9 light size, 10 fade threshold].
/// `clip` per caster = the rendering camera's EFGameObject +0x114 (ClipFrustumTest result: casters
/// outside the camera frustum cast nothing) / +0x110 (clipping word: bit 0x10 = no drop-down / parallel
/// shadow, bit 8 = no light shadows).
pub fn ShadowBeginLights(casters: &[(u32, u32, [u32; 11], DTMatrix, CasterEntry, (u16, u32))], lights: &[ShadowLight]) -> Vec<ShadowOcc> {
    let mut out: Vec<ShadowOcc> = Vec::new();
    for c in casters {
        let (uid, fl, p, o, e, (vis, word)) = *c;
        if vis == 0 || p[0] & 4 != 0 {
            continue;
        }
        if out.len() < 8 && word & 0x10 == 0 {
            out.extend(ShadowBegin(&[(uid, fl, p, o, e)]));
        }
        if out.len() > 7 {
            break;
        }
        if fl & 0x800 == 0 || word & 8 != 0 {
            continue;
        }
        for l in lights {
            if l.1 & 0x30 != 0 && l.5 && l.4 & 0x40000 == 0 {
                if let Some(s) = Prepare(uid, l, &o, &e, &p) {
                    out.push(s);
                }
                if out.len() > 7 {
                    return out;
                }
            }
        }
    }
    out
}

/// the 0x400 part of EFShadowMgr::Begin
pub fn ShadowBegin(casters: &[(u32, u32, [u32; 11], DTMatrix, CasterEntry)]) -> Vec<ShadowOcc> {
    let mut out = Vec::new();
    // drop-down light frame (this+0x1c): identity, at (+0x30 of the frame = LTM at) = (0,-1,0)
    let light = DTMatrix { right: [1.0, 0.0, 0.0], up: [0.0, 1.0, 0.0], at: [0.0, -1.0, 0.0], pos: [0.0; 3] };
    for (uid, logic_flags, p, o, e) in casters {
        if p[0] & 4 != 0 || out.len() >= 8 {
            continue;
        }
        if logic_flags & 0x400 == 0 {
            continue;
        }
        let reach = f32::from_bits(p[3]);
        let color = f32::from_bits(p[2]);
        let (ltm, near, far_plane, pos, far, vw) = PrepareParallelCamera(&light, o, e, reach, 1.0);
        let size = match p[5] {
            0x40 | 0x80 | 0x100 => p[5],
            _ => 0x80,
        };
        out.push(ShadowOcc {
            caster: *uid,
            size,
            occ: ProjectorOccurence {
                near,
                far_plane,
                pos,
                far,
                color: [color, color, color, 1.0],
                texture: format!("#shadow{uid:08x}"),
                ltm,
                view_window: [vw, vw],
                parallel: true,
                blend: 1,
                reach,
            },
        });
    }
    out
}

/// APPROXIMATION of EFShadowMgr_SP::Cast + ApplySoftener: caster triangles (world space) rasterised
/// into a size x size coverage mask in the occurrence's projection (same UVs as the receiver pass),
/// then the decoded [`ApplySoftener`]. Returns coverage 0..=255 row-major (v down).
pub fn CastSilhouette(o: &ProjectorOccurence, size: u32, tris: &[[[f32; 3]; 3]]) -> Vec<u8> {
    // GAME_SHADOW_OLDRASTER: the R8 reference path (A/B check of the P37 R9 fast path)
    if std::env::var_os("GAME_SHADOW_OLDRASTER").is_some() {
        return CastSilhouetteRef(o, size, tris);
    }
    let n = size as usize;
    // P37 R9: single thread on purpose: a per-caster thread split (tried) lost to the spawn cost and to
    // Bevy's task pool (EP1/L03B silhouette 2.2 -> 7.2 ms)
    let mut m = raster_cover(o, n, tris);
    m = soften_fast(m, n);
    for i in 0..n {
        for j in [0, n - 1] {
            m[i * n + j] = 0;
            m[j * n + i] = 0;
        }
    }
    m
}

/// P37 R9: the [`CastSilhouetteRef`] coverage loop without the per-triangle Vec, skipping texels already
/// covered (the write is the constant 0xf8, so the result is the same)
fn raster_cover(o: &ProjectorOccurence, n: usize, tris: &[[[f32; 3]; 3]]) -> Vec<u8> {
    let mut m = vec![0u8; n * n];
    let e = |a: [f32; 2], b: [f32; 2], c: [f32; 2]| (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
    for t in tris {
        let p: [[f32; 2]; 3] = std::array::from_fn(|i| {
            let uv = o.uv(t[i]);
            [uv[0] * n as f32, uv[1] * n as f32]
        });
        let (x0, x1) = (p.iter().map(|q| q[0]).fold(f32::MAX, f32::min).floor().max(0.0) as i32, p.iter().map(|q| q[0]).fold(f32::MIN, f32::max).ceil().min(n as f32 - 1.0) as i32);
        let (y0, y1) = (p.iter().map(|q| q[1]).fold(f32::MAX, f32::min).floor().max(0.0) as i32, p.iter().map(|q| q[1]).fold(f32::MIN, f32::max).ceil().min(n as f32 - 1.0) as i32);
        if x1 < x0 || y1 < y0 {
            continue;
        }
        let area = e(p[0], p[1], p[2]);
        if area.abs() < 1e-9 {
            continue;
        }
        for y in y0..=y1 {
            let row = &mut m[y as usize * n..][..n];
            for x in x0..=x1 {
                if row[x as usize] != 0 {
                    continue;
                }
                let c = [x as f32 + 0.5, y as f32 + 0.5];
                let (w0, w1, w2) = (e(p[1], p[2], c), e(p[2], p[0], c), e(p[0], p[1], c));
                if (w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0) || (w0 <= 0.0 && w1 <= 0.0 && w2 <= 0.0) {
                    row[x as usize] = 0xf8;
                }
            }
        }
    }
    m
}

/// P37 R9: [`ApplySoftener`] with one reused scratch buffer and the edge clamp only on the border
/// rows/columns (same per-texel integer arithmetic, same pass order)
fn soften_fast(mut m: Vec<u8>, n: usize) -> Vec<u8> {
    if n < 3 {
        return ApplySoftener(m, n);
    }
    if n >= 0x80 {
        for v in m.iter_mut() {
            *v = (((*v as u32 * 0x10) >> 7) as u8) & 0xf8;
        }
    }
    const D: [(i32, i32); 8] = [(-1, -1), (0, -1), (1, -1), (1, 0), (1, 1), (0, 1), (-1, -1), (-1, 0)];
    let mut src = vec![0u8; n * n];
    let f = |a: u8, b: u8| ((a as u32 + ((b as u32 * 0x40) >> 7)).min(255) as u8) & 0xf8;
    for (dx, dy) in D {
        std::mem::swap(&mut src, &mut m);
        for y in 0..n {
            let yy = (y as i32 + dy).clamp(0, n as i32 - 1) as usize;
            let (dst, s, sn) = (&mut m[y * n..][..n], &src[y * n..][..n], &src[yy * n..][..n]);
            // interior columns 1..n-1: x + dx is always in range
            for x in 1..n - 1 {
                dst[x] = f(s[x], sn[(x as i32 + dx) as usize]);
            }
            for x in [0, n - 1] {
                let xx = (x as i32 + dx).clamp(0, n as i32 - 1) as usize;
                dst[x] = f(s[x], sn[xx]);
            }
        }
    }
    m
}

/// The R8 [`CastSilhouette`] (reference for the fast path; GAME_SHADOW_OLDRASTER)
pub fn CastSilhouetteRef(o: &ProjectorOccurence, size: u32, tris: &[[[f32; 3]; 3]]) -> Vec<u8> {
    let n = size as usize;
    let mut m = vec![0u8; n * n];
    for t in tris {
        let p: Vec<[f32; 2]> = t.iter().map(|v| {
            let uv = o.uv(*v);
            [uv[0] * n as f32, uv[1] * n as f32]
        }).collect();
        let (x0, x1) = (p.iter().map(|q| q[0]).fold(f32::MAX, f32::min).floor().max(0.0) as i32, p.iter().map(|q| q[0]).fold(f32::MIN, f32::max).ceil().min(n as f32 - 1.0) as i32);
        let (y0, y1) = (p.iter().map(|q| q[1]).fold(f32::MAX, f32::min).floor().max(0.0) as i32, p.iter().map(|q| q[1]).fold(f32::MIN, f32::max).ceil().min(n as f32 - 1.0) as i32);
        let e = |a: [f32; 2], b: [f32; 2], c: [f32; 2]| (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
        let area = e(p[0], p[1], p[2]);
        if area.abs() < 1e-9 {
            continue;
        }
        for y in y0..=y1 {
            for x in x0..=x1 {
                let c = [x as f32 + 0.5, y as f32 + 0.5];
                let (w0, w1, w2) = (e(p[1], p[2], c), e(p[2], p[0], c), e(p[0], p[1], c));
                if (w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0) || (w0 <= 0.0 && w1 <= 0.0 && w2 <= 0.0) {
                    // RGBAQ 0x80ffffff into the 16-bit raster: 0xff stored as 5 bits = texel 0xf8
                    m[y as usize * n + x as usize] = 0xf8;
                }
            }
        }
    }
    let mut b = ApplySoftener(m, n);
    // ClearOuterLine: border texels cleared (GS clamp would smear the edge)
    for i in 0..n {
        for j in [0, n - 1] {
            b[i * n + j] = 0;
            b[j * n + i] = 0;
        }
    }
    b
}

/// EFShadowMgr_SP::ApplySoftener (0x00258a10) as the GS executes its packet (raster = the caster drawn
/// 0x80ffffff on black, so one channel is enough):
/// - ApplyClearPass (only if width >= 0x80): full-raster sprite, ALPHA_2 0x10000000a9 = Cd*0x10>>7.
/// - EF_SHADOW_SOFT_PASS_TECHNIQUE 2, COUNT 1, EF_SHADOW_SOFT_PASS[0] = 1.0 texel: 8 ApplySoftenerPass
///   sprites reading the raster itself (TEX0 = ourGlobals+0x60, ST shifted by (dx,dy) texels, RGBAQ 0x80 =
///   modulate 1.0, nearest) with ALPHA_2 0x40_00000068 = Cs*0x40>>7 + Cd, clamped.
/// quirk: the 7th offset repeats (-1,-1) instead of (-1,+1) (as in the C).
/// P26 R7: the raster is 16-bit (RwRasterCreate(n,n,0x10,5)): every pass writes 5 bits per channel, so
/// each result is truncated to `& 0xf8` (what the next pass / the projector samples).
/// Replacement: the GS texture/frame feedback is taken per pass from the pre-pass raster (what PCSX2 does
/// for a draw that samples its own target); outside texels clamp to the edge.
pub fn ApplySoftener(mut m: Vec<u8>, n: usize) -> Vec<u8> {
    if n >= 0x80 {
        for v in m.iter_mut() {
            *v = (((*v as u32 * 0x10) >> 7) as u8) & 0xf8;
        }
    }
    const D: [(i32, i32); 8] = [(-1, -1), (0, -1), (1, -1), (1, 0), (1, 1), (0, 1), (-1, -1), (-1, 0)];
    for (dx, dy) in D {
        let src = m.clone();
        for y in 0..n {
            let yy = (y as i32 + dy).clamp(0, n as i32 - 1) as usize;
            for x in 0..n {
                let xx = (x as i32 + dx).clamp(0, n as i32 - 1) as usize;
                let v = src[y * n + x] as u32 + ((src[yy * n + xx] as u32 * 0x40) >> 7);
                m[y * n + x] = (v.min(255) as u8) & 0xf8;
            }
        }
    }
    m
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn caster_shadow_under_object() {
        let e = InitCaster(&[([0.0, 1.0, 0.0], 0.5), ([0.0, 0.0, 0.0], 0.5)]);
        assert_eq!(e.center, [0.0, 0.5, 0.0]);
        assert!((e.radius - 1.0).abs() < 1e-6);
        let o = DTMatrix { right: [1.0, 0.0, 0.0], up: [0.0, 1.0, 0.0], at: [0.0, 0.0, 1.0], pos: [5.0, 0.0, 5.0] };
        let mut p = [0u32; 11];
        p[2] = 0.2f32.to_bits();
        p[3] = 2.0f32.to_bits();
        p[5] = 0x40;
        let s = ShadowBegin(&[(7, 0xc00, p, o, e)]);
        assert_eq!(s.len(), 1);
        let uv = s[0].occ.uv([5.0, -1.0, 5.0]);
        assert!((uv[0] - 0.5).abs() < 1e-3 && (uv[1] - 0.5).abs() < 1e-3, "{uv:?}");
        let m = CastSilhouette(&s[0].occ, 64, &[[[4.8, 0.5, 4.8], [5.2, 0.5, 4.8], [5.0, 0.5, 5.2]]]);
        assert!(m[32 * 64 + 32] > 0);
        // P37 R9: the fast raster/softener path is bit-identical to the reference (random caster soup,
        // several triangle counts, every raster size)
        let mut r = 0x1234_5678u32;
        let mut rnd = || {
            r ^= r << 13;
            r ^= r >> 17;
            r ^= r << 5;
            (r as f32 / u32::MAX as f32) * 1.6 - 0.8
        };
        let soup: Vec<[[f32; 3]; 3]> = (0..3000).map(|_| std::array::from_fn(|_| [5.0 + rnd(), 0.5 + rnd() * 0.5, 5.0 + rnd()])).collect();
        for n in [0x40u32, 0x80, 0x100] {
            for k in [1usize, 7, 1500, 3000] {
                assert_eq!(CastSilhouette(&s[0].occ, n, &soup[..k]), CastSilhouetteRef(&s[0].occ, n, &soup[..k]), "n {n} k {k}");
            }
        }
        p[0] = 4;
        assert!(ShadowBegin(&[(7, 0xc00, p, o, e)]).is_empty());
        // EFShadowMgr::Begin: +0x114 = 0 (outside the camera frustum) casts nothing, +0x110 bit 0x10
        // drops the 0x400 shadow
        p[0] = 0;
        assert_eq!(ShadowBeginLights(&[(7, 0xc00, p, o, e, (1, 0))], &[]).len(), 1);
        assert!(ShadowBeginLights(&[(7, 0xc00, p, o, e, (0, 0))], &[]).is_empty());
        assert!(ShadowBeginLights(&[(7, 0xc00, p, o, e, (1, 0x10))], &[]).is_empty());
    }
}

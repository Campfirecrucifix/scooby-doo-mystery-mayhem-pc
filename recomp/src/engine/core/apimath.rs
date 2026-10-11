//! APImath: the engine math table (EXE.1 `Engine API (module interface)/EFApi_math.cpp`, 60 slots,
//! filled by `Init_math` @0x001f2a10 in this order).
//!
//! Non-obvious, deliberate:
//! - The CVector* functions write `uRam00343d5c` ($gp-0x7314): the address of the API function being run
//!   on entry, 0 on exit. It is a debug tracker that nothing reads, so it is not modelled.
//! - The libm slots use Rust's f32 libm. The PS2's newlib float routines can differ by about 1 ULP
//!   (see rwmath.rs for the FPU rounding note).
//! - Vectors are `DTVector3d` (3 floats) and matrices are RwMatrix (see rwmath.rs), both in [`Mem`].
#![allow(non_snake_case, unused_variables)]
use super::rwmath::*;
use crate::engine::{Engine, Ptr};

// ---- libm slots 0x00..0x54 ----
/// slot 0x00
pub fn acosf(e: &mut Engine, a0: f32) -> f32 {
    a0.acos()
}
/// slot 0x04
pub fn asinf(e: &mut Engine, a0: f32) -> f32 {
    a0.asin()
}
/// slot 0x08
pub fn atanf(e: &mut Engine, a0: f32) -> f32 {
    a0.atan()
}
/// slot 0x0c
pub fn atan2f(e: &mut Engine, a0: f32, a1: f32) -> f32 {
    a0.atan2(a1)
}
/// slot 0x10
pub fn ceilf(e: &mut Engine, a0: f32) -> f32 {
    a0.ceil()
}
/// slot 0x14
pub fn cosf(e: &mut Engine, a0: f32) -> f32 {
    a0.cos()
}
/// slot 0x18
pub fn coshf(e: &mut Engine, a0: f32) -> f32 {
    a0.cosh()
}
/// slot 0x1c
pub fn expf(e: &mut Engine, a0: f32) -> f32 {
    a0.exp()
}
/// slot 0x20 @0x001f29c0: fabs through double
pub fn myfabs(e: &mut Engine, a0: f32) -> f32 {
    (a0 as f64).abs() as f32
}
/// slot 0x24
pub fn floorf(e: &mut Engine, a0: f32) -> f32 {
    a0.floor()
}
/// slot 0x28 (C fmodf: result has the sign of the dividend, same as Rust `%`)
pub fn fmodf(e: &mut Engine, a0: f32, a1: f32) -> f32 {
    a0 % a1
}
/// slot 0x2c: mantissa in [0.5, 1), exponent written to `*a1` (int)
pub fn frexpf(e: &mut Engine, a0: f32, a1: Ptr) -> f32 {
    if a0 == 0.0 || !a0.is_finite() {
        e.wi32(a1, 0);
        return a0;
    }
    let mut x = a0;
    let mut ex = 0i32;
    if x.is_subnormal() {
        x *= 2f32.powi(25);
        ex = -25;
    }
    let b = x.to_bits();
    ex += ((b >> 23) & 0xff) as i32 - 126;
    e.wi32(a1, ex);
    f32::from_bits((b & 0x807f_ffff) | (126 << 23))
}
/// slot 0x30
pub fn ldexpf(e: &mut Engine, a0: f32, a1: i32) -> f32 {
    (a0 as f64 * 2f64.powi(a1)) as f32
}
/// slot 0x34
pub fn logf(e: &mut Engine, a0: f32) -> f32 {
    a0.ln()
}
/// slot 0x38
pub fn log10f(e: &mut Engine, a0: f32) -> f32 {
    a0.log10()
}
/// slot 0x3c: fractional part returned, integral part written to `*a1` (float)
pub fn modff(e: &mut Engine, a0: f32, a1: Ptr) -> f32 {
    let i = a0.trunc();
    e.wf(a1, i);
    if a0.is_infinite() {
        0.0f32.copysign(a0)
    } else {
        a0 - i
    }
}
/// slot 0x40
pub fn powf(e: &mut Engine, a0: f32, a1: f32) -> f32 {
    a0.powf(a1)
}
/// slot 0x44
pub fn sinf(e: &mut Engine, a0: f32) -> f32 {
    a0.sin()
}
/// slot 0x48
pub fn sinhf(e: &mut Engine, a0: f32) -> f32 {
    a0.sinh()
}
/// slot 0x4c
pub fn sqrtf(e: &mut Engine, a0: f32) -> f32 {
    a0.sqrt()
}
/// slot 0x50
pub fn tanf(e: &mut Engine, a0: f32) -> f32 {
    a0.tan()
}
/// slot 0x54
pub fn tanhf(e: &mut Engine, a0: f32) -> f32 {
    a0.tanh()
}

// ---- vectors ----
#[inline]
fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// slot 0x58 @0x001f2e00
pub fn CVectorSet(e: &mut Engine, a0: Ptr, a1: f32, a2: f32, a3: f32) {
    e.wv3(a0, [a1, a2, a3]);
}
/// slot 0x5c @0x001f2e10 `a0 = a1 + a2`
pub fn CVectorAdd(e: &mut Engine, a0: Ptr, a1: Ptr, a2: Ptr) -> Ptr {
    let (p, q) = (e.rv3(a1), e.rv3(a2));
    e.wv3(a0, [p[0] + q[0], p[1] + q[1], p[2] + q[2]]);
    a0
}
/// slot 0x60 @0x001f2e60 `a0 = a1 - a2`
pub fn CVectorSub(e: &mut Engine, a0: Ptr, a1: Ptr, a2: Ptr) -> Ptr {
    let (p, q) = (e.rv3(a1), e.rv3(a2));
    e.wv3(a0, [p[0] - q[0], p[1] - q[1], p[2] - q[2]]);
    a0
}
/// slot 0x64 @0x001f2eb0
pub fn CVectorDot(e: &mut Engine, a0: Ptr, a1: Ptr) -> f32 {
    dot(e.rv3(a0), e.rv3(a1))
}
/// slot 0x68 @0x001f2ef0 `a0 = a1 x a2`
pub fn CVectorCross(e: &mut Engine, a0: Ptr, a1: Ptr, a2: Ptr) -> Ptr {
    let (p, q) = (e.rv3(a1), e.rv3(a2));
    e.wv3(a0, [p[1] * q[2] - p[2] * q[1], p[2] * q[0] - p[0] * q[2], p[0] * q[1] - p[1] * q[0]]);
    a0
}
/// slot 0x6c @0x001f2f60
pub fn CVectorLength(e: &mut Engine, a0: Ptr) -> f32 {
    let v = e.rv3(a0);
    dot(v, v).sqrt()
}
/// slot 0x70 @0x001f2fa0
pub fn CVectorNegate(e: &mut Engine, a0: Ptr, a1: Ptr) -> Ptr {
    let v = e.rv3(a1);
    e.wv3(a0, [-v[0], -v[1], -v[2]]);
    a0
}
/// slot 0x74 @0x001f2fe0 (no zero check: a zero vector gives NaNs, as on the PS2 it gives garbage)
pub fn CVectorNormalize(e: &mut Engine, a0: Ptr, a1: Ptr) -> Ptr {
    let v = e.rv3(a1);
    let fVar1 = 1.0 / dot(v, v).sqrt();
    e.wv3(a0, [v[0] * fVar1, v[1] * fVar1, v[2] * fVar1]);
    a0
}
/// slot 0x78 @0x001f3050
pub fn CVectorScale(e: &mut Engine, a0: Ptr, a1: Ptr, a2: f32) -> Ptr {
    let v = e.rv3(a1);
    e.wv3(a0, [v[0] * a2, v[1] * a2, v[2] * a2]);
    a0
}
/// slot 0x7c @0x001f3090: every component difference has a binary exponent below `a2` (1/0)
pub fn CVectorAreEqualEpsilon(e: &mut Engine, a0: Ptr, a1: Ptr, a2: u32) -> u64 {
    let uVar3 = a2.wrapping_add(0x7f).wrapping_mul(0x80_0000);
    let (p, q) = (e.rv3(a0), e.rv3(a1));
    let close = |k: usize| ((p[k] - q[k]).to_bits() & 0x7f80_0000) < uVar3;
    (close(0) && close(1) && close(2)) as u64
}
/// slot 0x80 @0x001f3150: angle between two unit vectors (radians)
pub fn CVectorGetAngleTo(e: &mut Engine, a0: Ptr, a1: Ptr) -> f32 {
    let fVar1 = dot(e.rv3(a0), e.rv3(a1));
    if fVar1 <= -1.0 {
        3.141_592_7
    } else if 1.0 <= fVar1 {
        0.0
    } else {
        fVar1.acos()
    }
}
/// slot 0x84 @0x001f31f0 `a0 = a2 * (a1.a2 / a2.a2)`
pub fn CVectorProjectOnto(e: &mut Engine, a0: Ptr, a1: Ptr, a2: Ptr) -> Ptr {
    let (v, o) = (e.rv3(a1), e.rv3(a2));
    let fVar1 = dot(v, o) / dot(o, o);
    e.wv3(a0, [o[0] * fVar1, o[1] * fVar1, o[2] * fVar1]);
    a0
}
/// slot 0x88 @0x001f32f0 `n = a2 x a3` (not normalised); `a0 = a1 - n * (n.a1)`
pub fn CVectorProjectOntoPlane(e: &mut Engine, a0: Ptr, a1: Ptr, a2: Ptr, a3: Ptr) -> Ptr {
    let (v, p, q) = (e.rv3(a1), e.rv3(a2), e.rv3(a3));
    let n = [p[1] * q[2] - p[2] * q[1], p[2] * q[0] - p[0] * q[2], p[0] * q[1] - p[1] * q[0]];
    let s = dot(n, v);
    e.wv3(a0, [v[0] - n[0] * s, v[1] - n[1] * s, v[2] - n[2] * s]);
    a0
}
/// slot 0x8c @0x001f3400: `a0 = normalize(a2 - a1)`. Returns the distance in f0, though Ghidra typed it void.
pub fn CVectorGetDirectionAndDistanceTo(e: &mut Engine, a0: Ptr, a1: Ptr, a2: Ptr) -> f32 {
    let (f, t) = (e.rv3(a1), e.rv3(a2));
    e.wv3(a0, [t[0] - f[0], t[1] - f[1], t[2] - f[2]]);
    let fVar1 = RwV3dLength(e, a0);
    let inv = 1.0 / fVar1;
    let d = e.rv3(a0);
    e.wv3(a0, [d[0] * inv, d[1] * inv, d[2] * inv]);
    fVar1
}

// ---- matrices ----
/// slot 0x90 @0x001f34a0
pub fn CMatGetWorkMatrix(e: &mut Engine) -> Ptr {
    e.work_matrix
}
/// slot 0x94 @0x001f34b0 (pad words untouched; flags |= 0x20003)
pub fn CMatSetIdentity(e: &mut Engine, a0: Ptr) {
    for (o, v) in [(0x28, 0x3f80_0000), (0x14, 0x3f80_0000), (0, 0x3f80_0000), (0x10, 0), (8, 0), (4, 0), (0x24, 0), (0x20, 0), (0x18, 0), (0x38, 0), (0x34, 0), (0x30, 0)] {
        e.w32(a0 + o, v);
    }
    let f = e.r32(a0 + 0xc) | 0x20003;
    e.w32(a0 + 0xc, f);
}
/// slot 0x98 @0x001f3500 (returns RwMatrixCreate's result in v0; Ghidra typed it void)
pub fn CMatCreateIdentity(e: &mut Engine) -> Ptr {
    RwMatrixCreate(e)
}
/// slot 0x9c @0x001f3520
pub fn CMatDestroy(e: &mut Engine, a0: Ptr) {
    RwMatrixDestroy(e, a0);
}
/// slot 0xa0 @0x001f3540
pub fn CMatGetPos(e: &mut Engine, a0: Ptr, a1: Ptr) {
    let v = e.rv3(a1 + 0x30);
    e.wv3(a0, v);
}
/// slot 0xa4 @0x001f3560
pub fn CMatGetAt(e: &mut Engine, a0: Ptr, a1: Ptr) {
    let v = e.rv3(a1 + 0x20);
    e.wv3(a0, v);
}
/// slot 0xa8 @0x001f3580
pub fn CMatGetUp(e: &mut Engine, a0: Ptr, a1: Ptr) {
    let v = e.rv3(a1 + 0x10);
    e.wv3(a0, v);
}
/// slot 0xac @0x001f35a0
pub fn CMatGetRight(e: &mut Engine, a0: Ptr, a1: Ptr) {
    let v = e.rv3(a1);
    e.wv3(a0, v);
}
fn set_row(e: &mut Engine, m: Ptr, off: u32, v: Ptr) {
    let v = e.rv3(v);
    e.wv3(m + off, v);
    e.w32(m + 0xc, 0);
}
/// slot 0xb0 @0x001f35c0 (flags cleared)
pub fn CMatSetPos(e: &mut Engine, a0: Ptr, a1: Ptr) {
    set_row(e, a0, 0x30, a1);
}
/// slot 0xb4 @0x001f35e0
pub fn CMatSetAt(e: &mut Engine, a0: Ptr, a1: Ptr) {
    set_row(e, a0, 0x20, a1);
}
/// slot 0xb8 @0x001f3600
pub fn CMatSetUp(e: &mut Engine, a0: Ptr, a1: Ptr) {
    set_row(e, a0, 0x10, a1);
}
/// slot 0xbc @0x001f3620
pub fn CMatSetRight(e: &mut Engine, a0: Ptr, a1: Ptr) {
    set_row(e, a0, 0, a1);
}
/// slot 0xc0 @0x001f3640 `a0 = inverse(a1)`. It clears a1's flags first, so this is always the generic inverse.
pub fn CMatInvert(e: &mut Engine, a0: Ptr, a1: Ptr) {
    e.w32(a1 + 0xc, 0);
    RwMatrixInvert(e, a0, a1);
}
/// slot 0xc4 @0x001f3660 `a0 = a1 * rot(a2)`
pub fn CMatTransformVector(e: &mut Engine, a0: Ptr, a1: Ptr, a2: Ptr) {
    RwV3dTransformVectors(e, a0, a1, 1, a2);
}
/// slot 0xc8 @0x001f36a0 `a0 = a1 * a2 + pos(a2)`
pub fn CMatTransformPoint(e: &mut Engine, a0: Ptr, a1: Ptr, a2: Ptr) {
    RwV3dTransformPoints(e, a0, a1, 1, a2);
}
/// slot 0xcc @0x001f36e0 `a0 = a1 * a2`, flags cleared
pub fn CMatMultiply(e: &mut Engine, a0: Ptr, a1: Ptr, a2: Ptr) {
    RwMatrixMultiply(e, a0, a1, a2);
    e.w32(a0 + 0xc, 0);
}
/// slot 0xd0 @0x001f3710 rotate the 3x3 part of `a0` by `a2` RADIANS about `a1` (post-concat); position kept
pub fn CMatRotate(e: &mut Engine, a0: Ptr, a1: Ptr, a2: f32) {
    let pos = e.rv3(a0 + 0x30);
    let axis = e.rv3(a1);
    RwMatrixRotate(e, a2 * 57.29578, a0, axis, 2);
    e.wv3(a0 + 0x30, pos);
    e.w32(a0 + 0xc, 0);
}
/// slot 0xd4 @0x001f3780 (post-concat: the position is scaled too)
pub fn CMatUniformScale(e: &mut Engine, a0: Ptr, a1: f32) {
    RwMatrixScale(e, a0, [a1, a1, a1], 2);
    e.w32(a0 + 0xc, 0);
}

/// shared body of CMatAlignAtWith (row 0x20) / CMatAlignUpWith (row 0x10): rotate the matrix so that
/// row `row` points along `a1`. Exact (anti)parallel cases: nothing / flip row and right.
fn align_with(e: &mut Engine, a0: Ptr, a1: Ptr, row: u32) {
    let cur = e.rv3(a0 + row);
    let l = (cur[0] * cur[0] + cur[1] * cur[1] + cur[2] * cur[2]).sqrt();
    let inv = 1.0 / l;
    let c = [cur[0] * inv, cur[1] * inv, cur[2] * inv];
    let t = e.rv3(a1);
    let l = (t[0] * t[0] + t[1] * t[1] + t[2] * t[2]).sqrt();
    let inv = 1.0 / l;
    let t = [t[0] * inv, t[1] * inv, t[2] * inv];
    // CVectorAreEqualEpsilon with exponent < 0x70 (|diff| < 2^-15)
    let near = |a: [f32; 3], b: [f32; 3]| (0..3).all(|k| ((a[k] - b[k]).to_bits() & 0x7f80_0000) < 0x3800_0000);
    if !near(c, t) {
        let nt = [-t[0], -t[1], -t[2]];
        if near(c, nt) {
            let r = e.rv3(a0 + row);
            e.wv3(a0 + row, [-r[0], -r[1], -r[2]]);
            let r = e.rv3(a0);
            e.wv3(a0, [-r[0], -r[1], -r[2]]);
        } else {
            let n = [c[1] * t[2] - c[2] * t[1], c[2] * t[0] - c[0] * t[2], c[0] * t[1] - c[1] * t[0]];
            let mut fVar3 = c[0] * t[0] + c[1] * t[1] + c[2] * t[2];
            if fVar3 <= -1.0 {
                fVar3 = 3.141_592_7;
            } else if 1.0 <= fVar3 {
                fVar3 = 0.0;
            } else {
                fVar3 = fVar3.acos();
            }
            // RwV3dNormalize(axis, axis)
            let l = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
            let k = if 0.0 < l { 1.0 / l } else { l };
            if l <= 0.0 {
                e.host.rw_error(0x19);
            }
            let axis = [n[0] * k, n[1] * k, n[2] * k];
            let pos = e.rv3(a0 + 0x30);
            RwMatrixRotate(e, fVar3 * 57.29578, a0, axis, 2);
            e.wv3(a0 + 0x30, pos);
        }
    }
    e.w32(a0 + 0xc, 0);
}
/// slot 0xd8 @0x001f37c0
pub fn CMatAlignAtWith(e: &mut Engine, a0: Ptr, a1: Ptr) {
    align_with(e, a0, a1, 0x20);
}
/// slot 0xdc @0x001f3c50
pub fn CMatAlignUpWith(e: &mut Engine, a0: Ptr, a1: Ptr) {
    align_with(e, a0, a1, 0x10);
}
/// slot 0xe0 @0x001f40e0 at = a1; right = normalize(a2 x at); up = normalize(at x right); flags cleared
pub fn CMatLookAtAlignUp(e: &mut Engine, a0: Ptr, a1: Ptr, a2: Ptr) {
    let at = e.rv3(a1);
    e.wv3(a0 + 0x20, at);
    let u = e.rv3(a2);
    e.wv3(a0, [u[1] * at[2] - u[2] * at[1], u[2] * at[0] - u[0] * at[2], u[0] * at[1] - u[1] * at[0]]);
    RwV3dNormalize(e, a0, a0);
    let r = e.rv3(a0);
    e.wv3(a0 + 0x10, [at[1] * r[2] - at[2] * r[1], at[2] * r[0] - at[0] * r[2], at[0] * r[1] - at[1] * r[0]]);
    RwV3dNormalize(e, a0 + 0x10, a0 + 0x10);
    e.w32(a0 + 0xc, 0);
}
/// slot 0xe4 @0x001f41e0 up = a1; right = normalize(up x a2); at = normalize(right x up); flags cleared
pub fn CMatLookUpAlignAt(e: &mut Engine, a0: Ptr, a1: Ptr, a2: Ptr) {
    let up = e.rv3(a1);
    e.wv3(a0 + 0x10, up);
    let t = e.rv3(a2);
    e.wv3(a0, [up[1] * t[2] - up[2] * t[1], up[2] * t[0] - up[0] * t[2], up[0] * t[1] - up[1] * t[0]]);
    RwV3dNormalize(e, a0, a0);
    let r = e.rv3(a0);
    e.wv3(a0 + 0x20, [r[1] * up[2] - r[2] * up[1], r[2] * up[0] - r[0] * up[2], r[0] * up[1] - r[1] * up[0]]);
    RwV3dNormalize(e, a0 + 0x20, a0 + 0x20);
    e.w32(a0 + 0xc, 0);
}
/// slot 0xe8 @0x001f42e0 copies a0 -> a1 (source first), all 0x40 bytes
pub fn CMatCopy(e: &mut Engine, a0: Ptr, a1: Ptr) {
    e.memcpy(a1, a0, 0x40);
}
/// slot 0xec @0x001f4370 16 floats from the array a0 into the matrix a1
pub fn CMatCopyFromArray(e: &mut Engine, a0: Ptr, a1: Ptr) {
    e.memcpy(a1, a0, 0x40);
}

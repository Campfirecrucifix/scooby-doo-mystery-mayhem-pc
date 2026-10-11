//! The RenderWare 3.x matrix/vector routines that the engine math API (EFApi_math.cpp) is built on
//! (EXE.1 `RenderWare/RwMatrix.c`, `RenderWare/RwV3d (vector maths).c`).
//!
//! RwMatrix layout (0x40 bytes): right +0x00, flags +0x0c, up +0x10, pad +0x1c, at +0x20, pad +0x2c,
//! pos +0x30, pad +0x3c. Flags: 3 = orthonormal, 0x20000 = rwMATRIXINTERNALIDENTITY.
//!
//! Non-obvious, deliberate:
//! - Rotate and Multiply are VU0 macro code on the PS2 that computes all 4 lanes. The w lanes land in
//!   the pad words. They are reproduced here, with the VU0 flushing denormal inputs to zero (so flag
//!   words read as floats count as 0). Stack words the original never initialised (the rotation
//!   matrix's w lanes) are taken as 0.
//! - Arithmetic is IEEE f32. The R5900 FPU/VU round toward zero and have no denormals or infinities,
//!   so results can differ by about 1 ULP per operation. Sums keep the PS2 accumulation order
//!   (x + y) + z (ADDA.S/MADD.S).
//! - RwV3dTransformPoints/Vectors dispatch through RW's vector-module function table; the PS2 target
//!   was not resolved, so they are implemented as the row-vector transform RW documents.
#![allow(non_snake_case)]
use crate::engine::{Engine, Ptr, NULL};

/// DAT_00343088, degrees -> radians (0x3c8efa35)
pub const RW_DEG_TO_RAD: f32 = 0.017_453_292;

#[inline]
fn ftz(x: f32) -> f32 {
    if x.is_subnormal() {
        0.0f32.copysign(x)
    } else {
        x
    }
}

/// the 4 rows of a matrix as VU0 sees them (16 bytes each)
fn load_rows(e: &Engine, m: Ptr) -> [[f32; 4]; 4] {
    let mut r = [[0.0; 4]; 4];
    for (i, row) in r.iter_mut().enumerate() {
        for (j, v) in row.iter_mut().enumerate() {
            *v = ftz(e.rf(m + (i * 16 + j * 4) as u32));
        }
    }
    r
}

fn store_rows(e: &mut Engine, m: Ptr, r: &[[f32; 4]; 4]) {
    for (i, row) in r.iter().enumerate() {
        for (j, v) in row.iter().enumerate() {
            e.wf(m + (i * 16 + j * 4) as u32, *v);
        }
    }
}

/// VU0 `vmula.bc / vmadda.bc / vmadd.bc` row combine: a.x*b0 + a.y*b1 + a.z*b2 (+ w_add)
fn vu_row(a: [f32; 4], b0: [f32; 4], b1: [f32; 4], b2: [f32; 4], add: Option<[f32; 4]>) -> [f32; 4] {
    let mut o = [0.0; 4];
    for k in 0..4 {
        let mut acc = b0[k] * a[0];
        acc += b1[k] * a[1];
        acc += b2[k] * a[2];
        if let Some(t) = add {
            acc += t[k];
        }
        o[k] = acc;
    }
    o
}

/// 0x001934c0
pub fn RwV3dLength(e: &Engine, v: Ptr) -> f32 {
    let [x, y, z] = e.rv3(v);
    (x * x + y * y + z * z).sqrt()
}

/// 0x00193408 normalises `src` into `dst`, returns the original length. A zero vector stays zero
/// (RW raises error 0x19, which this game ignores).
pub fn RwV3dNormalize(e: &mut Engine, dst: Ptr, src: Ptr) -> f32 {
    let [x, y, z] = e.rv3(src);
    let fVar2 = (x * x + y * y + z * z).sqrt();
    let mut fVar1 = fVar2;
    if 0.0 < fVar2 {
        fVar1 = 1.0 / fVar2;
    }
    e.wv3(dst, [x * fVar1, y * fVar1, z * fVar1]);
    if fVar2 <= 0.0 {
        e.host.rw_error(0x19);
    }
    fVar2
}

/// 0x0018feb0 a new identity matrix (RW freelist -> module heap). Pad words are left as allocated.
pub fn RwMatrixCreate(e: &mut Engine) -> Ptr {
    let p = e.malloc(0x40);
    if p != NULL {
        e.w32(p + 0x30, 0);
        e.w32(p, 0x3f80_0000);
        e.w32(p + 0xc, 0x20003);
        e.w32(p + 0x28, 0x3f80_0000);
        e.w32(p + 0x14, 0x3f80_0000);
        for o in [0x10, 8, 4, 0x24, 0x20, 0x18, 0x38, 0x34] {
            e.w32(p + o, 0);
        }
    }
    p
}

/// 0x0018fe78
pub fn RwMatrixDestroy(e: &mut Engine, m: Ptr) -> u32 {
    e.free(m);
    1
}

/// 0x0018f5d8 rotation from (1 - cos, sin) about a unit axis, combined with `m`
/// (0 replace, 1 pre-concat `R*M`, 2 post-concat `M*R`). Returns `m`, or NULL for a bad combine op.
pub fn RwMatrixRotateOneMinusCosineSine(e: &mut Engine, omc: f32, s: f32, m: Ptr, axis: [f32; 3], combine: i64) -> Ptr {
    let [fVar12, fVar9, fVar6] = axis;
    let fVar8 = fVar9 * fVar6 * omc;
    let fVar3 = fVar12 * fVar9 * omc;
    let fVar7 = fVar6 * fVar12 * omc;
    let fVar4 = 1.0 - (1.0 - fVar12 * fVar12) * omc;
    let fVar14 = fVar3 - fVar6 * s;
    let fVar13 = fVar7 + fVar9 * s;
    let fVar5 = fVar8 - fVar12 * s;
    let fVar10 = 1.0 - (1.0 - fVar6 * fVar6) * omc;
    let fVar11 = 1.0 - (1.0 - fVar9 * fVar9) * omc;
    let fVar3 = fVar3 + fVar6 * s;
    let fVar7 = fVar7 - fVar9 * s;
    let fVar8 = fVar8 + fVar12 * s;
    // rotation rows; w lanes: int 3 (denormal -> 0 on VU0) and uninitialised stack words (0)
    let r0 = [fVar4, fVar3, fVar7, 0.0];
    let r1 = [fVar14, fVar11, fVar8, 0.0];
    let r2 = [fVar13, fVar5, fVar10, 0.0];
    let r3 = [0.0, 0.0, 0.0, 0.0];
    let uVar1 = e.r32(m + 0xc);
    match combine {
        0 => {
            store_rows(e, m, &[r0, r1, r2, r3]);
            e.w32(m + 0xc, uVar1);
        }
        1 | 2 => {
            let mm = load_rows(e, m);
            let out = if combine == 1 {
                [
                    vu_row(r0, mm[0], mm[1], mm[2], None),
                    vu_row(r1, mm[0], mm[1], mm[2], None),
                    vu_row(r2, mm[0], mm[1], mm[2], None),
                    vu_row(r3, mm[0], mm[1], mm[2], Some(mm[3])),
                ]
            } else {
                [
                    vu_row(mm[0], r0, r1, r2, None),
                    vu_row(mm[1], r0, r1, r2, None),
                    vu_row(mm[2], r0, r1, r2, None),
                    vu_row(mm[3], r0, r1, r2, Some(r3)),
                ]
            };
            store_rows(e, m, &out);
            e.w32(m + 0xc, uVar1);
        }
        _ => {
            e.host.rw_error(0x8000_0003); // "Invalid combination type"
            return NULL;
        }
    }
    let f = e.r32(m + 0xc) & 0xfffd_ffff;
    e.w32(m + 0xc, f);
    m
}

/// 0x00190198 rotate `m` by `angle` DEGREES about `axis` (normalised here)
/// (`axis` is passed by value; the original takes an RwV3d pointer and only reads it)
pub fn RwMatrixRotate(e: &mut Engine, angle: f32, m: Ptr, axis: [f32; 3], combine: i64) -> Ptr {
    let a = angle * RW_DEG_TO_RAD;
    let [x, y, z] = axis;
    let mut fStack_4c = (x * x + y * y + z * z).sqrt();
    if 0.0 < fStack_4c {
        fStack_4c = 1.0 / fStack_4c;
    }
    let unit = [x * fStack_4c, y * fStack_4c, z * fStack_4c];
    let fVar1 = a.sin();
    let fVar2 = a.cos();
    RwMatrixRotateOneMinusCosineSine(e, 1.0 - fVar2, fVar1, m, unit, combine);
    m
}

/// 0x0018ff38 `dst = a * b` (row vectors: `a` applied first). The flags become `a.flags & b.flags`.
pub fn RwMatrixMultiply(e: &mut Engine, dst: Ptr, a: Ptr, b: Ptr) {
    let ma = load_rows(e, a);
    let uVar1 = e.r32(a + 0xc);
    let mb = load_rows(e, b);
    let uVar2 = e.r32(b + 0xc);
    let out = [
        vu_row(ma[0], mb[0], mb[1], mb[2], None),
        vu_row(ma[1], mb[0], mb[1], mb[2], None),
        vu_row(ma[2], mb[0], mb[1], mb[2], None),
        vu_row(ma[3], mb[0], mb[1], mb[2], Some(mb[3])),
    ];
    store_rows(e, dst, &out);
    e.w32(dst + 0xc, uVar2 & uVar1);
}

/// 0x0018ed40 full 3x4 inverse (singular -> determinant taken as 1)
pub fn MatrixInvertGeneric(e: &mut Engine, dst: Ptr, src: Ptr) -> Ptr {
    let p = |i: u32| e.rf(src + i * 4);
    let s: [f32; 16] = std::array::from_fn(|i| p(i as u32));
    let fVar7 = s[5] * s[10] - s[6] * s[9];
    let fVar3 = s[2] * s[9] - s[1] * s[10];
    let fVar2 = s[1] * s[6] - s[2] * s[5];
    let mut det = fVar7 * s[0] + fVar3 * s[4] + fVar2 * s[8];
    if det == 0.0 {
        det = 1.0;
    } else {
        det = 1.0 / det;
    }
    let (fVar11, fVar14, fVar13) = (fVar7, fVar3, fVar2);
    let w = |e: &mut Engine, i: u32, v: f32| e.wf(dst + i * 4, v);
    w(e, 0, fVar11 * det);
    w(e, 1, fVar14 * det);
    w(e, 2, fVar13 * det);
    let fVar10 = (s[6] * s[8] - s[4] * s[10]) * det;
    w(e, 4, fVar10);
    let fVar12 = (s[0] * s[10] - s[2] * s[8]) * det;
    w(e, 5, fVar12);
    let fVar9 = (s[2] * s[4] - s[0] * s[6]) * det;
    w(e, 6, fVar9);
    let fVar4 = (s[4] * s[9] - s[5] * s[8]) * det;
    w(e, 8, fVar4);
    let fVar8 = (s[1] * s[8] - s[0] * s[9]) * det;
    w(e, 9, fVar8);
    let fVar6 = (s[0] * s[5] - s[1] * s[4]) * det;
    w(e, 10, fVar6);
    let f = e.r32(dst + 0xc) & 0xfffd_ffff;
    e.w32(dst + 0xc, f);
    w(e, 12, -(s[12] * fVar11 * det + s[13] * fVar10 + s[14] * fVar4));
    w(e, 13, -(s[12] * fVar14 * det + s[13] * fVar12 + s[14] * fVar8));
    w(e, 14, -(s[12] * fVar13 * det + s[13] * fVar9 + s[14] * fVar6));
    dst
}

/// 0x0018f880 `dst = inverse(src)`: identity copy (when RW's identity optimisation is on), transpose for
/// orthonormal matrices (flags & 3 == 3), generic inverse otherwise
pub fn RwMatrixInvert(e: &mut Engine, dst: Ptr, src: Ptr) -> Ptr {
    let flags = e.r32(src + 0xc);
    if flags & 0x20000 & e.rw_matrix_module_flags == 0 {
        if flags & 3 == 3 {
            let s: [f32; 16] = std::array::from_fn(|i| e.rf(src + i as u32 * 4));
            let w = |e: &mut Engine, i: u32, v: f32| e.wf(dst + i * 4, v);
            w(e, 0, s[0]);
            w(e, 1, s[4]);
            w(e, 2, s[8]);
            w(e, 4, s[1]);
            w(e, 5, s[5]);
            w(e, 6, s[9]);
            w(e, 8, s[2]);
            w(e, 9, s[6]);
            w(e, 10, s[10]);
            w(e, 12, -(s[12] * s[0] + s[13] * s[1] + s[14] * s[2]));
            w(e, 13, -(s[12] * s[4] + s[13] * s[5] + s[14] * s[6]));
            w(e, 14, -(s[12] * s[8] + s[13] * s[9] + s[14] * s[10]));
            let f = (e.r32(dst + 0xc) & 0xfffd_ffff) | 3;
            e.w32(dst + 0xc, f);
        } else {
            MatrixInvertGeneric(e, dst, src);
        }
    } else {
        e.memcpy(dst, src, 0x40);
    }
    dst
}

/// 0x0018fa08 scale by `s` (0 replace, 1 pre-concat, 2 post-concat incl. position; `s` by value)
pub fn RwMatrixScale(e: &mut Engine, m: Ptr, s: [f32; 3], combine: i64) -> Ptr {
    let [sx, sy, sz] = s;
    let mul = |e: &mut Engine, i: u32, k: f32| {
        let v = e.rf(m + i * 4) * k;
        e.wf(m + i * 4, v);
    };
    match combine {
        1 => {
            for (i, k) in [(0, sx), (1, sx), (2, sx), (4, sy), (5, sy), (6, sy), (8, sz), (9, sz), (10, sz)] {
                mul(e, i, k);
            }
        }
        0 => {
            for (i, v) in [(0, 1.0), (12, 0.0), (10, 1.0), (5, 1.0), (4, 0.0), (2, 0.0), (1, 0.0), (9, 0.0), (8, 0.0), (6, 0.0), (14, 0.0), (13, 0.0)] {
                e.wf(m + i * 4, v);
            }
            let f = e.r32(m + 0xc) | 0x20003;
            e.w32(m + 0xc, f);
            e.wf(m, sx);
            e.wf(m + 0x14, sy);
            e.wf(m + 0x28, sz);
        }
        2 => {
            for (i, k) in [(0, sx), (1, sy), (2, sz), (4, sx), (5, sy), (6, sz), (8, sx), (9, sy), (10, sz), (12, sx), (13, sy), (14, sz)] {
                mul(e, i, k);
            }
        }
        _ => {
            e.host.rw_error(0x8000_0003);
            return NULL;
        }
    }
    let f = e.r32(m + 0xc) & 0xfffd_fffc;
    e.w32(m + 0xc, f);
    m
}

/// RwV3dTransformPoints: `out[i] = in[i] * M + pos` for `n` vectors (`dst` may equal `src`)
pub fn RwV3dTransformPoints(e: &mut Engine, dst: Ptr, src: Ptr, n: u32, m: Ptr) -> Ptr {
    transform(e, dst, src, n, m, true)
}

/// RwV3dTransformVectors: `out[i] = in[i] * M` (no translation)
pub fn RwV3dTransformVectors(e: &mut Engine, dst: Ptr, src: Ptr, n: u32, m: Ptr) -> Ptr {
    transform(e, dst, src, n, m, false)
}

fn transform(e: &mut Engine, dst: Ptr, src: Ptr, n: u32, m: Ptr, point: bool) -> Ptr {
    let (r, u, a, p) = (e.rv3(m), e.rv3(m + 0x10), e.rv3(m + 0x20), e.rv3(m + 0x30));
    for i in 0..n {
        let [x, y, z] = e.rv3(src + i * 12);
        let mut o = [0.0f32; 3];
        for k in 0..3 {
            o[k] = x * r[k] + y * u[k] + z * a[k];
            if point {
                o[k] += p[k];
            }
        }
        e.wv3(dst + i * 12, o);
    }
    dst
}

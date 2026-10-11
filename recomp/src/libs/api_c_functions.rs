//! ApiCFunctions custom library (`Game Logic Modules/Globals/Lib/apicfunc__<sha8>/ApiCFunc.c`, 26 functions).
//! It registers the 20-slot table "ApiCFunctions" (0x50 bytes): orientation and range tests, random
//! numbers, look-at/track helpers, line of sight, and the screen fade sequencer (DoFade/RequestFade).
//!
//! Six builds share the same code. The only difference is the id of the object that receives the
//! fade messages 0x4d/0x52 ([`BUILDS`]).
//!
//! Local data (`pLocalDataApiCFunction`, 0x28 bytes, the lib object's own): +0 fade mode,
//! +4 fade-in duration, +8 fade-out duration, +0xc timer duration, +0x10 delay, +0x14 fade timer,
//! +0x18 HUD window, +0x1c lib object id, +0x20 caller id, +0x24 fade step.
//!
//! Float code: the module is unoptimised (no madd.s/msub.s); every expression is evaluated in the
//! same order as in the object code.
#![allow(non_snake_case, non_upper_case_globals, unused_variables, unused_mut)]
use super::{lib_build, lib_data, set_global};
use crate::engine::core::apicons as cons;
use crate::engine::*;
use crate::logic::{LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "apicfunc",
    data_size: 0x40,
    ctor: apicfuncConstructor,
    dtor: apicfuncDestructor,
    register: Some(apicfuncRegister),
    init: &[],
    build_init: None,
};

/// (build sha8, id of the object that receives fade messages 0x4d/0x52)
pub const BUILDS: [(&str, u32); 6] = [
    ("0126adf1", 0x2a10_0007),
    ("0ceedab3", 0x2a10_0000),
    ("0d32b6d8", 0x2a10_0006),
    ("4ed8d0ba", 0x2a10_000c),
    ("577e6e2b", 0x2a10_0013),
    ("a1d1e457", 0x2a10_001f),
];

/// Index of a build in [`BUILDS`] by the first 8 hex digits of its SHA-1.
pub fn build_index(sha8: &str) -> Option<usize> {
    BUILDS.iter().position(|b| b.0 == sha8)
}

// module globals (_globals.h)
const pExtra: u32 = 0x10000;
const pGameObj: u32 = 0x10004;
const pHud: u32 = 0x10008;
const pMath: u32 = 0x1000c;
const pPhysic: u32 = 0x10010;
const pRegLib: u32 = 0x10014;
const pStdlib: u32 = 0x10018;
const pParticle: u32 = 0x1001c;
const pFunction: u32 = 0x10020;
const pDebug: u32 = 0x10024;
const pLocalDataApiCFunction: u32 = 0x10028;

const ALL: u32 = 0x7fff_ffff;
/// PI/8 (0x3ec90fdb), the sector width of CheckPositionOrientationPoint
const PI_8: f32 = f32::from_bits(0x3ec9_0fdb);
const PI: f32 = f32::from_bits(0x4049_0fdb);
const TWO_PI: f32 = f32::from_bits(0x40c9_0fdb);

fn fade_id(e: &Engine) -> u32 {
    BUILDS[lib_build(e, &MODULE)].1
}
/// value of pLocalDataApiCFunction
fn local(e: &Engine) -> Ptr {
    e.r32(lib_data(e, &MODULE, pLocalDataApiCFunction))
}
/// `pLocalDataApiCFunction[i]`
fn lw(e: &Engine, i: u32) -> u32 {
    let l = local(e);
    e.r32(l + i * 4)
}
fn lf(e: &Engine, i: u32) -> f32 {
    f32::from_bits(lw(e, i))
}
fn sw(e: &mut Engine, i: u32, v: u32) {
    let l = local(e);
    e.w32(l + i * 4, v);
}
/// `pDebug != NULL`
fn debug_on(e: &Engine) -> bool {
    e.r32(lib_data(e, &MODULE, pDebug)) != 0
}
fn dbg(e: &mut Engine, msg: &str) {
    if debug_on(e) {
        let (a, b) = (e.cstr("APICFunc"), e.cstr(msg));
        LIBdebug::PrintDebugInfo(e, a, b);
    }
}
fn dbg_val(e: &mut Engine, msg: &str, p: Ptr, kind: u32) {
    if debug_on(e) {
        let (a, b) = (e.cstr("APICFunc"), e.cstr(msg));
        LIBdebug::PrintDebugInfoWithValue(e, a, b, p, kind);
    }
}

/// 0x00011070 is the point `param_2` in one of the 16 PI/8 sectors around object `param_1`'s facing
/// (bit 0 = [-PI/8, 0], counter-clockwise ... bit 15 = [0, PI/8]; each sector is lo < angle <= hi)?
/// The bounds are computed at run time exactly as in the object code: (-s)+(-s), (-s)*3.0, s*7.0, ...
pub fn CheckPositionOrientationPoint(e: &mut Engine, param_1: u32, param_2: Ptr, param_3: u32) -> u32 {
    let s = PI_8;
    let p = e.rv3(param_2);
    let DStack_80 = e.alloca(0x10);
    let DStack_70 = e.alloca(0x10);
    APIgameobj::GetPosition(e, param_1, ALL, DStack_80);
    APIgameobj::GetLookAt(e, param_1, ALL, DStack_70);
    let o = e.rv3(DStack_80);
    let look = e.rv3(DStack_70);
    let d = [p[0] - o[0], p[1] - o[1], p[2] - o[2]];
    let fVar3 = APImath::atan2f(e, look[0], look[2]);
    let mut a = APImath::atan2f(e, d[0], d[2]);
    a = a - fVar3;
    if a < -PI {
        a = a + TWO_PI;
    } else if PI < a {
        a = a - TWO_PI;
    }
    let n = -s;
    // (bit, lo, hi): the sector test is `lo < a && a <= hi`
    let sectors: [(u32, f32, f32); 16] = [
        (0x1, n, 0.0),
        (0x2, n + n, n),
        (0x4, n * 3.0, n + n),
        (0x8, n * 4.0, n * 3.0),
        (0x10, n * 5.0, n * 4.0),
        (0x20, n * 6.0, n * 5.0),
        (0x40, n * 7.0, n * 6.0),
        (0x80, -PI, n * 7.0),
        (0x100, s * 7.0, PI),
        (0x200, s * 6.0, s * 7.0),
        (0x400, s * 5.0, s * 6.0),
        (0x800, s * 4.0, s * 5.0),
        (0x1000, s * 3.0, s * 4.0),
        (0x2000, s + s, s * 3.0),
        (0x4000, s, s + s),
        (0x8000, 0.0, s),
    ];
    for (bit, lo, hi) in sectors {
        if param_3 & bit != 0 && a <= hi && lo < a {
            return 1;
        }
    }
    0
}

/// 0x00011858 CheckPositionOrientationPoint with the position of object `param_2`
pub fn CheckPositionOrientation(e: &mut Engine, param_1: u32, param_2: u32, param_3: u32) -> u32 {
    let DStack_40 = e.alloca(0x10);
    APIgameobj::GetPosition(e, param_2, ALL, DStack_40);
    CheckPositionOrientationPoint(e, param_1, DStack_40, param_3)
}

/// 0x000118f0 angle of object `param_4` in object `param_3`'s space (from its joint `param_5`) within
/// [`param_1`, `param_2`] (wrapping through PI when `param_1` > `param_2`)
pub fn IsObjectInMyOrientation(e: &mut Engine, param_1: f32, param_2: f32, param_3: u32, param_4: u32, param_5: u32) -> u32 {
    let DStack_60 = e.alloca(0x10);
    let DStack_50 = e.alloca(0x10);
    let aDStack_40 = e.alloca(0x20);
    APIgameobj::GetPosition(e, param_3, param_5, DStack_60);
    APIgameobj::GetPosition(e, param_4, ALL, aDStack_40);
    let (b, a) = (e.rv3(aDStack_40), e.rv3(DStack_60));
    e.wv3(DStack_50, [b[0] - a[0], b[1] - a[1], b[2] - a[2]]);
    APIgameobj::InvertTransformVector(e, param_3, DStack_50, DStack_50);
    let d = e.rv3(DStack_50);
    let fVar2 = -APImath::atan2f(e, d[0], d[2]);
    if param_1 <= param_2 {
        if param_2 < fVar2 || fVar2 < param_1 { 0 } else { 1 }
    } else if param_1 <= fVar2 || (fVar2 <= param_2 && -PI <= fVar2) {
        1
    } else {
        0
    }
}

/// 0x00011ac8 object `param_2` within `param_1` of the point `param_3` (`param_4` == 0: ignore height)
pub fn CheckRangePoint(e: &mut Engine, param_1: f32, param_2: u32, param_3: Ptr, param_4: i32) -> u8 {
    let DStack_80 = e.alloca(0x10);
    let DStack_60 = e.alloca(0x10);
    let aDStack_50 = e.alloca(0x20);
    let p = e.rv3(param_3);
    e.wv3(DStack_80, p);
    APIgameobj::GetPosition(e, param_2, ALL, DStack_60);
    APImath::CVectorSub(e, aDStack_50, DStack_60, DStack_80);
    if param_4 == 0 {
        e.wf(aDStack_50 + 4, 0.0);
    }
    let d = e.rv3(aDStack_50);
    (d[0] * d[0] + d[1] * d[1] + d[2] * d[2] <= param_1 * param_1) as u8
}

/// 0x00011bf8 CheckRangePoint with the position of object `param_3`
pub fn CheckRangeObject(e: &mut Engine, param_1: f32, param_2: u32, param_3: u32, param_4: i32) -> u8 {
    let aDStack_50 = e.alloca(0x20);
    APIgameobj::GetPosition(e, param_3, ALL, aDStack_50);
    CheckRangePoint(e, param_1, param_2, aDStack_50, param_4)
}

/// 0x00011c98 falling faster than 1 unit/s
pub fn bJumpOnIt(e: &mut Engine, param_1: u32) -> u8 {
    let DStack_30 = e.alloca(0x10);
    APIphysic::GetVelocity(e, param_1, DStack_30);
    (e.rf(DStack_30 + 4) < -1.0) as u8
}

/// 0x00011d18 random int in [min, max] (either order). The original traps (divide by zero) when the
/// range size wraps to 0; Rust panics there as well.
pub fn IntRandom(e: &mut Engine, param_1: i32, param_2: i32) -> i32 {
    let (mut lo, mut hi) = (param_1, param_2);
    if param_2 < param_1 {
        lo = param_2;
        hi = param_1;
    }
    let uVar1 = APIstdlib::MyRand(e);
    let iVar2 = hi.wrapping_sub(lo).wrapping_add(1);
    (uVar1 as i32).wrapping_rem(iVar2).wrapping_add(lo)
}

/// 0x00011dc8 random multiple of `param_3` (<= 0: 1.0) from the lower bound, counting the steps
/// like the original loop does
pub fn FloatRandom(e: &mut Engine, param_1: f32, param_2: f32, param_3: f32) -> f32 {
    let mut n = 0i32;
    let mut step = param_3;
    if param_3 <= 0.0 {
        step = 1.0;
    }
    let (mut lo, mut hi) = (param_2, param_1);
    if param_1 <= param_2 {
        lo = param_1;
        hi = param_2;
    }
    let mut f = lo;
    while f <= hi {
        n += 1;
        f = f + step;
    }
    let uVar1 = APIstdlib::MyRand(e);
    lo + step * ((uVar1 as i32).wrapping_rem(n) as f32)
}

/// 0x00011f18 turn object `param_2` (around Y) to face like object `param_3`, plus `param_1` radians
pub fn TakeRotation(e: &mut Engine, param_1: f32, param_2: u32, param_3: i32) {
    if param_3 != 0 {
        let DStack_80 = e.alloca(0x10);
        let DStack_70 = e.alloca(0x10);
        let DStack_60 = e.alloca(0x10);
        APIgameobj::GetLookAt(e, param_2, ALL, DStack_70);
        APIgameobj::GetLookAt(e, param_3 as u32, ALL, DStack_60);
        e.wv3(DStack_80, [0.0, 1.0, 0.0]);
        let b = e.rv3(DStack_60);
        let fVar2 = APImath::atan2f(e, b[0], b[2]);
        let a = e.rv3(DStack_70);
        let fVar3 = APImath::atan2f(e, a[0], a[2]);
        APIgameobj::Rotate(e, param_2, DStack_80, (fVar2 - fVar3) + param_1);
    }
}

/// 0x00012048 move object `param_1` to joint `param_3` of object `param_2`, plus the offset `param_4` (may be NULL)
pub fn TakeCoor(e: &mut Engine, param_1: u32, param_2: u32, param_3: u32, param_4: Ptr) {
    let DStack_40 = e.alloca(0x10);
    APIgameobj::GetPosition(e, param_2, param_3, DStack_40);
    if param_4 != NULL {
        let (p, o) = (e.rv3(DStack_40), e.rv3(param_4));
        e.wv3(DStack_40, [p[0] + o[0], p[1] + o[1], p[2] + o[2]]);
    }
    APIgameobj::SetTranslation(e, param_1, DStack_40);
}

/// 0x00012118
pub fn TakeCoorNRotation(e: &mut Engine, param_1: f32, param_2: u32, param_3: i32, param_4: u32, param_5: Ptr) {
    TakeCoor(e, param_2, param_3 as u32, param_4, param_5);
    TakeRotation(e, param_1, param_2, param_3);
}

/// 0x00012198 turn object `param_2` to face the direction `param_3` (XZ), plus `param_1` radians
pub fn SetLookAt(e: &mut Engine, param_1: f32, param_2: u32, param_3: Ptr) {
    if param_3 != NULL && (e.rf(param_3) != 0.0 || e.rf(param_3 + 8) != 0.0) {
        let DStack_70 = e.alloca(0x10);
        let DStack_60 = e.alloca(0x10);
        APIgameobj::GetLookAt(e, param_2, ALL, DStack_60);
        e.wv3(DStack_70, [0.0, 1.0, 0.0]);
        let (dx, dz) = (e.rf(param_3), e.rf(param_3 + 8));
        let fVar2 = APImath::atan2f(e, dx, dz);
        let l = e.rv3(DStack_60);
        let fVar3 = APImath::atan2f(e, l[0], l[2]);
        APIgameobj::Rotate(e, param_2, DStack_70, (fVar2 - fVar3) + param_1);
    }
}

/// 0x000122e0 turn object `param_2` towards object `param_3` (XZ), plus `param_1` radians
pub fn TrackID(e: &mut Engine, param_1: f32, param_2: i32, param_3: u32) {
    if param_2 != 0 {
        let DStack_c0 = e.alloca(0x10);
        let DStack_b0 = e.alloca(0x10);
        let DStack_90 = e.alloca(0x10);
        let DStack_80 = e.alloca(0x10);
        APIgameobj::GetPosition(e, param_2 as u32, ALL, DStack_90);
        APIgameobj::GetPosition(e, param_3, ALL, DStack_80);
        let (t, s) = (e.rv3(DStack_80), e.rv3(DStack_90));
        let dx = t[0] - s[0];
        let dz = t[2] - s[2];
        if dx != 0.0 || dz != 0.0 {
            let mut inv = 1.0 / (dx * dx + dz * dz);
            let mut nx = APImath::powf(e, dx * dx * inv, 0.5);
            let mut nz = APImath::powf(e, dz * dz * inv, 0.5);
            if dx < 0.0 {
                nx = nx * -1.0;
            }
            if dz < 0.0 {
                nz = nz * -1.0;
            }
            // computed and never used, as in the original
            inv = APImath::powf(e, nx * nx + nz * nz, 0.5);
            e.wv3(DStack_c0, [0.0, 1.0, 0.0]);
            APIgameobj::GetLookAt(e, param_2 as u32, ALL, DStack_b0);
            let fVar2 = APImath::atan2f(e, nx, nz);
            let l = e.rv3(DStack_b0);
            let fVar3 = APImath::atan2f(e, l[0], l[2]);
            APIgameobj::Rotate(e, param_2 as u32, DStack_c0, (fVar2 - fVar3) + param_1);
        }
    }
}

/// 0x000125c0 distance from object `param_1` to joint `param_3` (0 = origin) of object `param_2`;
/// `param_4` == 0 ignores height; -1.0 if `param_1` is 0. (The original tests `param_1` twice and
/// never `param_2`; kept.)
pub fn GetDistance(e: &mut Engine, param_1: i32, param_2: u32, param_3: i32, param_4: i32) -> f32 {
    if param_1 == 0 || param_1 == 0 {
        return -1.0;
    }
    let DStack_50 = e.alloca(0x10);
    let DStack_40 = e.alloca(0x10);
    let DStack_30 = e.alloca(0x10);
    APIgameobj::GetPosition(e, param_1 as u32, ALL, DStack_50);
    if param_3 == 0 {
        APIgameobj::GetPosition(e, param_2, ALL, DStack_40);
    } else {
        APIgameobj::GetPosition(e, param_2, param_3 as u32, DStack_40);
    }
    APImath::CVectorSub(e, DStack_30, DStack_50, DStack_40);
    if param_4 == 0 {
        e.wf(DStack_30 + 4, 0.0);
    }
    APImath::CVectorLength(e, DStack_30)
}

/// 0x00012710 can observer `p[0]` see target `p[1]`? Range `p[0xd]`, view cone [`p[0xe]`, `p[0xf]`],
/// then a ray (start/direction lifted by `p[0xb]`/`p[0xc]`, masks + ignored object `p[3..8]`) must
/// hit an OBJECT whose id is `p[8]`, `p[9]` or `p[10]`.
///
/// Non-obvious, kept: on the PS2 stack the RayIntersectIn (0x2c bytes written) sits right below the
/// RayIntersectOut, so the engine's reads of in+0x2c/+0x30 see the out struct; the port keeps that
/// layout (in at frame+0, out at frame+0x30). The "hit something" test reads the halfword at out+0x1a,
/// which for an object hit is the high half of the object id the engine stored at +0x18.
pub fn InLineOfSight(e: &mut Engine, param_1: Ptr) -> u32 {
    let p = |e: &Engine, i: u32| e.r32(param_1 + i * 4);
    let frame = e.alloca(0x70);
    let (ray_in, ray_out) = (frame, frame + 0x30);
    let DStack_60 = e.alloca(0x10);
    let DStack_50 = e.alloca(0x10);
    let aDStack_40 = e.alloca(0x20);
    let (r, a0, a1) = (f32::from_bits(p(e, 0xd)), p(e, 0), p(e, 1));
    let bVar1 = CheckRangeObject(e, r, a0, a1, 1);
    if bVar1 != 0 {
        let (mn, mx) = (f32::from_bits(p(e, 0xe)), f32::from_bits(p(e, 0xf)));
        if IsObjectInMyOrientation(e, mn, mx, a0, a1, 0x120_0000) != 0 {
            APIgameobj::GetPosition(e, a0, ALL, DStack_60);
            APIgameobj::GetPosition(e, a1, ALL, DStack_50);
            APImath::CVectorSub(e, aDStack_40, DStack_50, DStack_60);
            let (s, d) = (e.rv3(DStack_60), e.rv3(aDStack_40));
            let (oy, dy) = (f32::from_bits(p(e, 0xb)), f32::from_bits(p(e, 0xc)));
            e.wv3(ray_in, [s[0], s[1] + oy, s[2]]);
            e.wv3(ray_in + 0xc, [d[0], d[1] + dy, d[2]]);
            for k in 0..5 {
                let v = p(e, 3 + k);
                e.w32(ray_in + 0x18 + k * 4, v);
            }
            let iVar2 = APIphysic::CheckRayIntersection(e, ray_in, ray_out);
            let sStack_86 = e.r16(ray_out + 0x1a);
            let iStack_84 = e.r32(ray_out + 0x1c);
            if sStack_86 != 0 && iVar2 == 1 && (iStack_84 == p(e, 8) || iStack_84 == p(e, 9) || iStack_84 == p(e, 10)) {
                return 1;
            }
        }
    }
    0
}

/// 0x00012950 speed of object `param_1` over the axes in `param_2` (bit 0 x, 1 y, 2 z; 0 = all);
/// -1.0 for object 0 or when the running object's id reads as 0x7ffffffe (retriever done)
pub fn GetVelocity(e: &mut Engine, param_1: i32, param_2: u32) -> f32 {
    if param_1 == 0 || APIgameobj::GetID(e, SELF, NULL) == 0x7fff_fffe {
        return -1.0;
    }
    let DStack_30 = e.alloca(0x10);
    APIphysic::GetVelocity(e, param_1 as u32, DStack_30);
    let mut m = param_2;
    if param_2 == 0 {
        m = 7;
    }
    if m & 1 == 0 {
        e.wf(DStack_30, 0.0);
    }
    if m & 2 == 0 {
        e.wf(DStack_30 + 4, 0.0);
    }
    if m & 4 == 0 {
        e.wf(DStack_30 + 8, 0.0);
    }
    APImath::CVectorLength(e, DStack_30)
}

/// 0x00012a70 start a screen fade. `param_1`: {+0 mode (1 in+out, 2 in, 4 out; flag 0x10 = restart
/// even if one is running, cleared here), +4 in duration, +8 out duration, +0xc hold time, +0x10 delay}.
/// Busy (step != 0, no 0x10): message 10000 to the caller, -1. The caller receives 0x2711 when the fade
/// in is done, 0x2712 at the end, 10000 when discarded.
pub fn DoFade(e: &mut Engine, param_1: Ptr) -> u32 {
    if lw(e, 9) == 0 || e.r32(param_1) & 0x10 != 0 {
        let v = e.r32(param_1) & 0xffff_ffef;
        e.w32(param_1, v);
        sw(e, 9, 0);
        let (o, t) = (lw(e, 7), lw(e, 5));
        APIextra::TimerUnregisterNotify(e, o, t);
        let v = e.r32(param_1 + 0x10);
        sw(e, 4, v);
        for i in 0..4 {
            let v = e.r32(param_1 + i * 4);
            sw(e, i, v);
        }
        if lf(e, 1) <= 0.0 {
            sw(e, 1, 0x3f80_0000);
        }
        if lf(e, 2) <= 0.0 {
            sw(e, 2, 0x3f80_0000);
        }
        if 0.0 < lf(e, 3) && lf(e, 3) < 0.1 {
            sw(e, 3, 0x3dcc_cccd);
        }
        let uVar1 = APIgameobj::GetID(e, SELF, NULL);
        sw(e, 8, uVar1);
        let l = local(e);
        dbg(e, "DoFade --------------------Validating----------------------------------");
        dbg_val(e, "DoFade -> CallerID", l + 0x20, 1);
        dbg_val(e, "DoFade -> uiFadeMode", l, 1);
        dbg_val(e, "DoFade -> fFadeInDuration", l + 4, 2);
        dbg_val(e, "DoFade -> fFadeOutDuration", l + 8, 2);
        dbg_val(e, "DoFade -> fTimerDuration", l + 0xc, 2);
        dbg_val(e, "DoFade -> fFadeDelay", l + 0x10, 2);
        dbg_val(e, "DoFade -> API ID", l + 0x1c, 1);
        let f = fade_id(e);
        APIgameobj::SendMessageTo(e, f, NULL, 0x4d, 0, NULL);
        RequestFade(e)
    } else {
        dbg(e, "DoFade --------------------Fade still processing--------------------");
        dbg(e, "DoFade --------------------Discard----------------------------------");
        APIgameobj::SendMessageTo(e, SELF, NULL, 10000, 0, NULL);
        0xffff_ffff
    }
}

/// 0x00012f80 advance the fade sequence one step (steps: 1 delayed, 2 fade in, 3 fade in done,
/// 4 fade out, 5 end) and arm the timer for the next one. Recursive when there is no hold time.
pub fn RequestFade(e: &mut Engine) -> u32 {
    let afStack_40 = e.alloca(0x10);
    e.wf(afStack_40, 0.0);
    dbg(e, "RequestFade --------------------Processing----------------------------------");
    let w = lw(e, 6);
    APIhud::WindowResetFlags(e, w, 0x100);
    let uVar1 = lw(e, 0);
    let step = lw(e, 9);
    let unknown_mode = if uVar1 == 2 {
        if 0.0 < lf(e, 4) {
            sw(e, 9, 1);
        } else if step == 0 || step == 1 {
            sw(e, 9, 2);
        } else if step == 3 {
            sw(e, 9, 5);
        } else {
            sw(e, 9, step.wrapping_add(1));
        }
        let f = fade_id(e);
        APIgameobj::SendMessageTo(e, f, NULL, 0x4d, 1, NULL);
        false
    } else if uVar1 < 3 {
        if uVar1 != 1 {
            true
        } else {
            if 0.0 < lf(e, 4) {
                sw(e, 9, 1);
            } else if step == 0 {
                sw(e, 9, 2);
            } else {
                sw(e, 9, step.wrapping_add(1));
            }
            let f = fade_id(e);
            APIgameobj::SendMessageTo(e, f, NULL, 0x4d, 1, NULL);
            false
        }
    } else if uVar1 != 4 {
        true
    } else {
        if 0.0 < lf(e, 4) {
            sw(e, 9, 1);
        } else if step == 0 || step == 1 {
            sw(e, 9, 4);
        } else if step == 4 {
            sw(e, 9, 5);
        } else {
            sw(e, 9, step.wrapping_add(1));
        }
        false
    };
    if unknown_mode {
        let l = local(e);
        dbg_val(e, "RequestFade -> default unknown FADE MODE -> DISCARDED", l, 1);
        let f = fade_id(e);
        APIgameobj::SendMessageTo(e, f, NULL, 0x52, 1, NULL);
        let c = lw(e, 8);
        APIgameobj::SendMessageTo(e, c, NULL, 10000, 0, NULL);
        return 0xffff_ffff;
    }
    match lw(e, 9) {
        1 => {
            let v = lf(e, 4);
            e.wf(afStack_40, v);
            sw(e, 4, 0);
            dbg_val(e, "RequestFade -> FADE_STEP_DELAYED", afStack_40, 2);
        }
        2 => {
            let v = lf(e, 1);
            e.wf(afStack_40, v);
            dbg_val(e, "RequestFade -> FADE_STEP_DO_IN", afStack_40, 2);
            FadeInOut(e);
        }
        3 => {
            dbg_val(e, "RequestFade -> FADE_STEP_DO_IN_DONE", afStack_40, 2);
            let c = lw(e, 8);
            APIgameobj::SendMessageTo(e, c, NULL, 0x2711, 0, NULL);
            if lf(e, 3) <= 0.0 {
                RequestFade(e);
                return 0;
            }
            let v = lf(e, 3);
            e.wf(afStack_40, v);
        }
        4 => {
            let v = lf(e, 2);
            e.wf(afStack_40, v);
            dbg_val(e, "RequestFade -> FADE_STEP_DO_OUT", afStack_40, 2);
            FadeInOut(e);
        }
        5 => {
            sw(e, 9, 0);
            dbg_val(e, "RequestFade -> FADE_STEP_END", afStack_40, 2);
            let w = lw(e, 6);
            APIhud::WindowSetFlags(e, w, 0x100);
            let c = lw(e, 8);
            APIgameobj::SendMessageTo(e, c, NULL, 0x2712, 0, NULL);
            let f = fade_id(e);
            APIgameobj::SendMessageTo(e, f, NULL, 0x52, 1, NULL);
            return 0;
        }
        _ => {
            let l = local(e);
            dbg_val(e, "RequestFade -> uiFadeStep -> default unknown step-> DISCARDED", l + 0x24, 1);
            let w = lw(e, 6);
            APIhud::WindowSetFlags(e, w, 0x100);
            let c = lw(e, 8);
            APIgameobj::SendMessageTo(e, c, NULL, 10000, 0, NULL);
            let f = fade_id(e);
            APIgameobj::SendMessageTo(e, f, NULL, 0x52, 1, NULL);
            return 0xffff_ffff;
        }
    }
    let t = lw(e, 5);
    APIextra::TimerReset(e, t);
    let v = e.rf(afStack_40);
    APIextra::TimerSetFrequency(e, t, v);
    let o = lw(e, 7);
    APIextra::TimerRegisterNotify(e, o, t, 0);
    0
}

/// 0x000137f0 HUD colour fade (step 2: from colour 0x8000000b in, step 4: out)
pub fn FadeInOut(e: &mut Engine) -> u64 {
    let l = local(e);
    if lw(e, 9) == 2 {
        dbg_val(e, "FadeInOut -> IN", l + 4, 2);
        let d = lf(e, 1);
        APIhud::ColorFadeStart(e, 0, 0x8000_000b, 1, 0, d, 0xff);
    } else if lw(e, 9) == 4 {
        dbg_val(e, "FadeInOut -> OUT", l + 8, 2);
        let d = lf(e, 2);
        APIhud::ColorFadeStart(e, 0, 0x8000_000b, 0, 1, d, 0xff);
    }
    0
}

/// 0x00013960 cosine of the angle between `param_1` and `param_2` (`param_3` == 0: in XZ only);
/// 1.0 if either is zero
pub fn GetCos(e: &mut Engine, param_1: Ptr, param_2: Ptr, param_3: i32) -> f32 {
    let DStack_40 = e.alloca(0x10);
    let DStack_30 = e.alloca(0x10);
    let a = e.rv3(param_1);
    e.wv3(DStack_40, a);
    let b = e.rv3(param_2);
    e.wv3(DStack_30, b);
    if param_3 == 0 {
        e.wf(DStack_30 + 4, 0.0);
        e.wf(DStack_40 + 4, 0.0);
    }
    let (a, b) = (e.rv3(DStack_40), e.rv3(DStack_30));
    if (a[0] == 0.0 && a[1] == 0.0 && a[2] == 0.0) || (b[0] == 0.0 && b[1] == 0.0 && b[2] == 0.0) {
        1.0
    } else {
        APImath::CVectorNormalize(e, DStack_40, DStack_40);
        APImath::CVectorNormalize(e, DStack_30, DStack_30);
        APImath::CVectorDot(e, DStack_40, DStack_30)
    }
}

/// 0x00013b00 callback 0 of the lib object: the fade timer fired
pub fn fcback_TIMER(e: &mut Engine, param_1: u32) -> LResult<u64> {
    if param_1 == lw(e, 5) {
        let (o, t) = (lw(e, 7), lw(e, 5));
        APIextra::TimerUnregisterNotify(e, o, t);
        dbg(e, "CB_TIMER -> hTimerFade");
        RequestFade(e);
    }
    Ok(0)
}

/// 0x00013bc8 particle type with texture `param_1` and name `param_2`. `param_3` is an API table
/// (slots +4 TypeCreate, +0x20 TypeSetTexture, +0x24 TypeSetName); every shipped caller passes
/// pParticle, anything else is outside what the port supports and panics.
pub fn CreateParticleType(e: &mut Engine, param_1: u32, param_2: u32, param_3: i32) -> u32 {
    assert_eq!(param_3 as u32, cons::GetApiParticle(e), "CreateParticleType: table is not APIparticle");
    let uVar1 = APIparticle::TypeCreate(e);
    APIparticle::TypeSetTexture(e, uVar1 as i32, param_1);
    APIparticle::TypeSetName(e, uVar1 as i32, param_2);
    uVar1
}

/// 0x00013c58 emitter of type `param_1` on joint `param_3` of object `param_2`
pub fn CreateEmitter(e: &mut Engine, param_1: i32, param_2: u32, param_3: u32) -> i32 {
    let iVar1 = APIparticle::EmitterCreate(e, param_1) as i32;
    APIparticle::EmitterSetJoint(e, iVar1, param_2, param_3);
    iVar1
}

/// 0x00013cd8 store the API tables, then RegisterLib("ApiCFunctions", 0x50)
pub fn apicfuncRegister(e: &mut Engine, api: Ptr) {
    let tables: [(u32, fn(&mut Engine) -> Ptr); 8] = [
        (pExtra, |e| cons::GetApiExtra(e)),
        (pGameObj, |e| cons::GetApiGameObj(e)),
        (pHud, |e| cons::GetApiHUD(e)),
        (pMath, |e| cons::GetApiMath(e)),
        (pPhysic, |e| cons::GetApiPhysic(e)),
        (pStdlib, |e| cons::GetApiStdlib(e)),
        (pParticle, |e| cons::GetApiParticle(e)),
        (pRegLib, |e| cons::GetApiRegLib(e)),
    ];
    for (a, f) in tables {
        let v = f(e);
        set_global(e, &MODULE, a, v);
    }
    let n = e.cstr("ApiCFunctions");
    let v = cons::GetApiCustom(e, n);
    set_global(e, &MODULE, pFunction, v);
    let d = e.cstr("debug");
    let v = cons::GetApiCustom(e, d);
    set_global(e, &MODULE, pDebug, v);
    let v = APIRegLib::RegisterLib(e, n, 0x50);
    set_global(e, &MODULE, pFunction, v);
}

/// 0x00013fc8 fade timer, a hidden full-screen HUD window (background 0xb)
pub fn apicfuncConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    if instanceCount == 1 {
        cons::SetExternalFunction(e, 0, fcback_TIMER);
    }
    let slot = lib_data(e, &MODULE, pLocalDataApiCFunction);
    APIcons::GetLocalData(e, slot, 0x28);
    let v = APIextra::TimerCreate(e);
    sw(e, 5, v);
    let v = APIgameobj::GetID(e, SELF, NULL);
    sw(e, 7, v);
    sw(e, 9, 0);
    let w = APIhud::WindowAdd(e, 1, 0xffff_ffff, 0x800, 1);
    sw(e, 6, w);
    APIhud::WindowSetPos(e, w, 0.0, 0.0);
    APIhud::WindowSetDims(e, w, 1.0, 1.0);
    APIhud::WindowSetBackGround(e, w, 0xb, 0.0, 0.0);
    APIhud::WindowSetFlags(e, w, 0x100);
    Ok(())
}

/// 0x00014178
pub fn apicfuncDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    let n = e.cstr("ApiCFunctions");
    APIRegLib::UnRegisterLib(e, n);
    let t = lw(e, 5);
    APIextra::TimerDestroy(e, t);
    let w = lw(e, 6);
    APIhud::WindowDelete(e, w);
    Ok(())
}

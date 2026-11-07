//! MiscFunc custom library (`Game Logic Modules/Globals/Lib/miscfunc__1e3ebcb2/MiscFunc.c`, 19 functions).
//! Both builds (03db0ccb, 1e3ebcb2) are the same code. It registers the 16-slot table "MiscFunc" (0x40 bytes).
//!
//! Struct layouts written here (offsets from the decompiled stack frames):
//! - EFClippingParams (0x40): +0 f32, then 5 entries of 0xc {f32 distance, u32 flags_a, u32 flags_b}
//!   at +4; a distance of 0 ends the list, -1.0 is the last entry.
//! - EFProximityClippingParams (0x10): +0 value, +4 14.0, +8 0.1, +0xc flags (0x10000000 = notify).
//! - RayIntersectIn: +0 start, +0xc direction (not normalised), +0x18..+0x24 four masks, +0x28 ignored
//!   object, +0x30 = 0. RayIntersectOut goes to pLocalDataMiscFunc+0x40.
#![allow(non_snake_case, non_upper_case_globals, unused_variables, unused_mut)]
use super::{lib_data, set_global};
use crate::engine::core::apicons as cons;
use crate::engine::*;
use crate::logic::{LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "miscfunc",
    data_size: 0x670,
    ctor: miscfuncConstructor,
    dtor: miscfuncDestructor,
    register: Some(miscfuncRegister),
    init: &[],
    build_init: Some(build_init),
};

/// preCalcSin @0x10028 (400 floats, sin(2*pi*i/400)) is read from the module on the disc (WP-X4); both
/// builds hold the same table.
fn build_init(m: &mut Mem, data: Ptr, _b: usize) {
    crate::engine::gamedata::init_syms(m, data, "1e3ebcb2", &["preCalcSin"]);
}

// module globals (_globals.h)
const pExtra: u32 = 0x10000;
const pGameObj: u32 = 0x10004;
const pParticle: u32 = 0x10008;
const pPhysic: u32 = 0x1000c;
const pRegLib: u32 = 0x10010;
const pSound: u32 = 0x10014;
const pStdlib: u32 = 0x10018;
const pMiscFunc: u32 = 0x1001c;
const pGraphic: u32 = 0x10020;
const preCalcSin: u32 = 0x10028;
const pLocalDataMiscFunc: u32 = 0x10668;
const pGlobalDataMiscFunc: u32 = 0x1066c;

fn g(e: &Engine, a: u32) -> Ptr {
    lib_data(e, &MODULE, a)
}
fn local(e: &Engine) -> Ptr {
    e.r32(g(e, pLocalDataMiscFunc))
}

/// 0x00011338 proximity + standard clipping for object `param_1` (proximity value `param_2`;
/// `param_3` != 0 = notify `param_3` on proximity, through RegisterNotify(obj, -1, 10, param_3))
pub fn MiscFunc_SetClipping(e: &mut Engine, param_1: u32, param_2: u32, param_3: i32) {
    let uStack_70 = e.alloca(0x40);
    let uStack_30 = e.alloca(0x10);
    APIgameobj::GetProximityClippingParams(e, param_1, uStack_30);
    e.w32(uStack_30 + 4, 0x4160_0000);
    e.w32(uStack_30 + 8, 0x3dcc_cccd);
    e.w32(uStack_30, param_2);
    if param_3 == 0 {
        e.w32(uStack_30 + 0xc, 0);
    } else {
        e.w32(uStack_30 + 0xc, 0x1000_0000);
        APIgameobj::RegisterNotify(e, param_1, 0xffff_ffff, 10, param_3 as u32);
    }
    APIgameobj::SetProximityClippingParams(e, param_1, uStack_30);
    APIgameobj::GetClippingParams(e, param_1, uStack_70);
    for (o, v) in [(0, 0x3dcc_cccd), (4, 0x4160_0000), (0xc, 0), (8, 0), (0x10, 0x41c0_0000), (0x18, 0x20), (0x14, 0xa0), (0x1c, 0xbf80_0000), (0x24, 0x6b8), (0x20, 0x6a0)] {
        e.w32(uStack_70 + o, v);
    }
    APIgameobj::SetClippingParams(e, param_1, uStack_70, 1);
}

/// 0x000114c0 edit the clipping of `param_1`: 2 = no clipping, 4 = default flags 0x6bf, 8/0x10/0x20/0x40
/// add flags 0x20/0x80/0x100/0x400; `param_3` (5 floats) overrides the distances (0 = keep).
pub fn MiscFunc_ChangeClipping(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr, param_4: u32) {
    let pg = g(e, pGameObj);
    if e.r32(pg) != 0 {
        let s = e.alloca(0x40);
        APIgameobj::GetClippingParams(e, param_1, s);
        if param_2 & 2 == 0 {
            if param_2 & 4 != 0 {
                e.w32(s + 4, 0xbf80_0000);
                e.w32(s + 0xc, 0x6bf);
                e.w32(s + 8, 0x6bf);
                e.w32(s, 0);
            }
        } else {
            e.w32(s + 4, 0xbf80_0000);
            e.w32(s + 0xc, 0);
            e.w32(s + 8, 0);
            e.w32(s, 0);
        }
        for (bit, f) in [(8, 0x20), (0x10, 0x80), (0x20, 0x100), (0x40, 0x400)] {
            if param_2 & bit != 0 {
                let v = e.r32(s + 0xc) | f;
                e.w32(s + 0xc, v);
                let v = e.r32(s + 8) | f;
                e.w32(s + 8, v);
            }
        }
        if param_3 != NULL {
            let v = e.r32(param_3);
            e.w32(s + 4, v);
            for (k, o) in [(1, 0x10), (2, 0x1c), (3, 0x28), (4, 0x34)] {
                if e.rf(param_3 + k * 4) != 0.0 {
                    let v = e.r32(param_3 + k * 4);
                    e.w32(s + o, v);
                }
            }
        }
        APIgameobj::SetClippingParams(e, param_1, s, param_4);
    }
}

/// 0x00011720 ray from object `param_1` (0.5 above its position) to the point `param_2`.
/// `param_3` 1 = static geometry blocks, 2 = objects block (other than `param_1`). 1 = nothing in the way.
pub fn MiscFunc_ValidateTarget(e: &mut Engine, param_1: u32, param_2: Ptr, param_3: u32) -> u32 {
    let DStack_80 = e.alloca(0x10);
    let ray = e.alloca(0x34);
    e.w32(ray + 0x30, 0);
    APIgameobj::GetPosition(e, param_1, 0x7fff_ffff, DStack_80);
    let p = e.rv3(DStack_80);
    let t = e.rv3(param_2);
    let d = [t[0] - p[0], t[1] - p[1], t[2] - p[2]];
    e.wv3(ray, [p[0], p[1] + 0.5, p[2]]);
    if param_3 & 2 == 0 {
        e.w32(ray + 0x18, 0);
        e.w32(ray + 0x1c, 0);
        e.w32(ray + 0x28, 0);
    } else {
        e.w32(ray + 0x18, 0xffff_ffff);
        e.w32(ray + 0x1c, 0xffff_ffff);
        e.w32(ray + 0x28, param_1);
    }
    if param_3 & 1 == 0 {
        e.w32(ray + 0x20, 0);
        e.w32(ray + 0x24, 0);
    } else {
        e.w32(ray + 0x20, 0xffff_ffff);
        e.w32(ray + 0x24, 0xffff_ffff);
    }
    e.wv3(ray + 0xc, d);
    let out = local(e) + 0x40;
    let iVar1 = APIphysic::CheckRayIntersection(e, ray, out);
    if iVar1 == 2 && param_3 & 1 != 0 {
        0
    } else if iVar1 == 1 && param_3 & 2 != 0 {
        0
    } else {
        1
    }
}

/// 0x00011918 blocked -> copy the hit point (pLocalDataMiscFunc+0x40) to `param_3` and return its address;
/// clear -> return `param_2`
pub fn MiscFunc_FindIntersect(e: &mut Engine, param_1: u32, param_2: Ptr, param_3: Ptr) -> Ptr {
    let mut param_2 = param_2;
    if MiscFunc_ValidateTarget(e, param_1, param_2, 3) == 0 {
        let l = local(e);
        for k in 0..3 {
            let v = e.r32(l + 0x40 + k * 4);
            e.w32(param_3 + k * 4, v);
        }
        param_2 = l + 0x40;
    }
    param_2
}

/// 0x000119e8 table sine, 400 steps per turn. Rust `as i32` truncates and saturates like the R5900's
/// cvt.w.s. A negative angle gives a negative index: the original then reads the words BEFORE the
/// table (the module's API pointers, then whatever precedes the module image). The port reads the
/// same offsets of the library image in Mem, so the result is garbage there as well.
pub fn MiscFunc_Sin(e: &mut Engine, param_1: f32) -> f32 {
    let a = ((param_1 * 100000.0) as i32).wrapping_rem(0x9965e);
    let i = (((a as f32) / 628318.0) * 400.0) as i32 % 400;
    let t = g(e, preCalcSin);
    e.rf(t.wrapping_add((i as u32).wrapping_mul(4)))
}

/// 0x00011ad0 launch latent `param_1`, send it message 0x16f (sub `param_3`, data `param_2`); returns its id
pub fn MiscFunc_CreateGenericFX(e: &mut Engine, param_1: u32, param_2: Ptr, param_3: u32) -> u32 {
    let uVar1 = APIgameobj::LaunchLatant(e, param_1);
    APIgameobj::SendMessageTo(e, uVar1, NULL, 0x16f, param_3, param_2);
    uVar1
}

/// 0x00011b58 messages 0x171 then 0x170 to the FX object
pub fn MiscFunc_PlayGenericFX(e: &mut Engine, param_1: i32) {
    if param_1 != 0 {
        APIgameobj::SendMessageTo(e, param_1 as u32, NULL, 0x171, 0, NULL);
        APIgameobj::SendMessageTo(e, param_1 as u32, NULL, 0x170, 0, NULL);
    }
}

/// 0x00011be8
pub fn MiscFunc_DestroyGenericFX(e: &mut Engine, param_1: i32) {
    if param_1 != 0 {
        APIgameobj::KillObject(e, param_1 as u32);
    }
}

/// 0x00011c40 set (`param_4` == 0) or clear the `param_3` flags of emitter `param_2`. `param_1` is an
/// API table: the original calls its slots 0x58 (get) and 0x54 (set). Every shipped caller passes
/// pParticle (EmitterGetFlags / EmitterSetFlags); any other table is outside what the port supports
/// and panics.
pub fn MiscFunc_EmitterUpdateFlags(e: &mut Engine, param_1: i32, param_2: u32, param_3: u32, param_4: i32) {
    assert_eq!(param_1 as u32, cons::GetApiParticle(e), "MiscFunc_EmitterUpdateFlags: table is not APIparticle");
    let uStack_30 = e.alloca(4);
    e.w32(uStack_30, 0);
    APIparticle::EmitterGetFlags(e, param_2 as i32, uStack_30);
    let mut f = e.r32(uStack_30);
    if param_4 == 0 {
        f |= param_3;
    } else {
        f &= !param_3;
    }
    e.w32(uStack_30, f);
    APIparticle::EmitterSetFlags(e, param_2 as i32, f);
}

/// 0x00011cf0 zero `param_3` bytes, copy at most `param_3 - 1` chars; returns the number copied
pub fn MiscFunc_StrncpySafe(e: &mut Engine, param_1: Ptr, param_2: Ptr, param_3: i32) -> i32 {
    let mut i = 0;
    while i < param_3 {
        e.w8(param_1.wrapping_add(i as u32), 0);
        i += 1;
    }
    i = 0;
    while e.r8(param_2.wrapping_add(i as u32)) != 0 && i < param_3 - 1 {
        let c = e.r8(param_2.wrapping_add(i as u32));
        e.w8(param_1.wrapping_add(i as u32), c);
        i += 1;
    }
    i
}

/// 0x00011e00 physics flag 1 on, clipping off
pub fn MiscFunc_InitForCutscene(e: &mut Engine, param_1: u32) {
    APIphysic::SetFlags(e, param_1, 1);
    MiscFunc_ChangeClipping(e, param_1, 2, NULL, 1);
}

/// 0x00011e70 physics flag 1 off, clipping restored from `param_2`
pub fn MiscFunc_EndCutscene(e: &mut Engine, param_1: u32, param_2: Ptr) {
    APIphysic::ResetFlags(e, param_1, 1);
    APIgameobj::SetClippingParams(e, param_1, param_2, 0);
}

/// 0x00011ee0 fire displacement-modifier `param_1` at the foot joint `param_2` of the running object
/// (offset in object space, 0.15 up), `param_3` stored at +0x78
pub fn MiscFunc_LaunchFootStep(e: &mut Engine, param_1: u32, param_2: u32, param_3: u32) {
    let DStack_d0 = e.alloca(0x10);
    let DStack_c0 = e.alloca(0x10);
    let DStack_b0 = e.alloca(0x10);
    let afStack_a0 = e.alloca(0x80);
    APIphysic::DMGetParams(e, param_1, afStack_a0);
    APIgameobj::GetPosition(e, SELF, 0x7fff_ffff, DStack_d0);
    APIgameobj::GetPosition(e, SELF, param_2, DStack_c0);
    let (a, b) = (e.rv3(DStack_c0), e.rv3(DStack_d0));
    e.wv3(DStack_c0, [a[0] - b[0], a[1] - b[1], a[2] - b[2]]);
    APIgameobj::InvertTransformVector(e, SELF, DStack_c0, DStack_b0);
    let l = e.rv3(DStack_b0);
    e.wf(afStack_a0, l[0]);
    e.wf(afStack_a0 + 4, 0.15);
    e.wf(afStack_a0 + 8, l[2]);
    for (o, v) in [(0xc, 0), (0x10, 0xbf00_0000), (0x14, 0), (0x18, 0), (0x1c, 0), (0x20, 0xffff_ffff), (0x24, 0), (0x28, SELF), (0x6c, 0x3f00_0000), (0x70, 0), (0x74, 0x3f00_0000), (0x78, param_3)] {
        e.w32(afStack_a0 + o, v);
    }
    APIphysic::DMSetParams(e, param_1, afStack_a0);
    APIphysic::DMSetActive(e, param_1, 0xffff_ffff);
}

/// 0x00012090 read the running object's clipping into `param_1` (left untouched afterwards), clear the
/// `param_2` flags in a copy (each entry until distance 0, or through the -1.0 one) and apply the copy
pub fn MiscFunc_RemoveBitClipping(e: &mut Engine, param_1: Ptr, param_2: u32) {
    APIgameobj::GetClippingParams(e, SELF, param_1);
    let s = e.alloca(0x40);
    e.memcpy(s, param_1, 0x40);
    let mut i = 0;
    while i < 5 && e.rf(s + i * 0xc + 4) != 0.0 {
        let v = e.r32(s + i * 0xc + 0xc) & !param_2;
        e.w32(s + i * 0xc + 0xc, v);
        let v = e.r32(s + i * 0xc + 8) & !param_2;
        e.w32(s + i * 0xc + 8, v);
        if e.rf(s + i * 0xc + 4) == -1.0 {
            break;
        }
        i += 1;
    }
    APIgameobj::SetClippingParams(e, SELF, s, 0);
}

/// 0x000122b0
pub fn MiscFunc_ReplaceMyClipping(e: &mut Engine, param_1: Ptr) {
    APIgameobj::SetClippingParams(e, SELF, param_1, 0);
}

/// 0x00012300 additive "glow" sprite group with colour (r, g, b, a); group id written to `*param_5`
pub fn MiscFunc_CreateStandardGlow(e: &mut Engine, param_1: f32, param_2: f32, param_3: f32, param_4: f32, param_5: Ptr) -> u32 {
    let v = APIgraphics::SpriteAddGroup(e);
    e.w32(param_5, v);
    let t = e.cstr("glow");
    let grp = e.r32(param_5);
    APIgraphics::SpriteSetGroupTexture(e, grp, t);
    let grp = e.r32(param_5);
    APIgraphics::SpriteSetGroupBlendMode(e, grp, 1);
    let grp = e.r32(param_5);
    APIgraphics::SpriteSetGroupFlags(e, grp, 0x14);
    let grp = e.r32(param_5);
    APIgraphics::SpriteSetGroupColor(e, grp, param_1, param_2, param_3, param_4)
}

/// 0x000123f0 store the API tables, then RegisterLib("MiscFunc", 0x40)
pub fn miscfuncRegister(e: &mut Engine, api: Ptr) {
    let v = cons::GetApiExtra(e);
    set_global(e, &MODULE, pExtra, v);
    let v = cons::GetApiGameObj(e);
    set_global(e, &MODULE, pGameObj, v);
    let v = cons::GetApiParticle(e);
    set_global(e, &MODULE, pParticle, v);
    let v = cons::GetApiPhysic(e);
    set_global(e, &MODULE, pPhysic, v);
    let v = cons::GetApiNewSound(e);
    set_global(e, &MODULE, pSound, v);
    let v = cons::GetApiStdlib(e);
    set_global(e, &MODULE, pStdlib, v);
    let n = e.cstr("MiscFunc");
    let v = cons::GetApiCustom(e, n);
    set_global(e, &MODULE, pMiscFunc, v);
    let v = cons::GetApiRegLib(e);
    set_global(e, &MODULE, pRegLib, v);
    let v = cons::GetApiGraphics(e);
    set_global(e, &MODULE, pGraphic, v);
    let v = APIRegLib::RegisterLib(e, n, 0x40);
    set_global(e, &MODULE, pMiscFunc, v);
}

/// 0x00012670 global data = game buffer 0; local data 0xb8 bytes; game buffer 0 +0x4c = 0x10
pub fn miscfuncConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    let gs = g(e, pGlobalDataMiscFunc);
    APIcons::GetGlobalData(e, gs, 4);
    let b = APIgameobj::GetGameBuffer(e, 0, 0x8f8, 1);
    let gd = e.r32(gs);
    e.w32(gd, b);
    let ls = g(e, pLocalDataMiscFunc);
    APIcons::GetLocalData(e, ls, 0xb8);
    let gd = e.r32(gs);
    let b = e.r32(gd);
    e.w32(b + 0x4c, 0x10);
    Ok(())
}

/// 0x00012718 UnRegisterLib("MiscFunc")
pub fn miscfuncDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    let n = e.cstr("MiscFunc");
    APIRegLib::UnRegisterLib(e, n);
    Ok(())
}


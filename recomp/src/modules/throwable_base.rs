//! `throwable_base`: generic throwable prop (crates, barrels, ...): picked up by a thrower (0x113 /
//! 0x115), thrown with a JumpOnCoor arc (0x110), dropped (0x114), respawned at its initial matrix when
//! it falls below y = -50 (0xc9). Port of
//! `Game Logic Modules/Globals/Objects/THROWABLES/throwable_base__*/Throwable_Base.c`.
//!
//! The 6 builds share the code; they differ only by goCustomDataThrowableType (object id -> type row,
//! per-level, emitted by `build_init`) and by the 3 floor-hit sound ids (`SOUNDS`).
//! goCustomDataThrowableBase (17 x 0x1c: type, weight, +0xb4, +0xb8, +0xbc, +0xc0 pick height, attack
//! kind) is identical in all builds.
//!
//! pLocalDataThrowableBase (0xcc): +4/+8/+0xc/+0x10 DM handles (kind 4/0/8/7), +0x14 holder,
//! +0x18 relative height, +0x1c..+0x28 attack params, +0x2c, +0x30 held, +0x34 fly phase,
//! +0x38 spin angle, +0x3c thrower id, +0x40 JumpOnCoorParam, +0x68 spin axis, +0x74 look-at,
//! +0x80/+0x8c/+0x98/+0xa4 initial up/at/right/pos, +0xb0..+0xc4 type row, +0xc8 collided.
#![allow(non_snake_case, non_upper_case_globals, unused_variables)]

use crate::engine::*;
use crate::logic::{ExtFn, LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "throwable_base",
    // globals 0x10000..0x102a8 + gpoLanguagesNamesAndISOCodes at 0x102b0
    data_size: 0x2c0,
    ctor: throwable_baseConstructor,
    dtor: throwable_baseDestructor,
    register: None,
    init: &[(0x100a0, &CUSTOM_DATA_BASE)],
    build_init: Some(build_init),
};

const KEYS: [&str; 6] = ["86231c0e", "c3bdf103", "459893d3", "2c8cc342", "55e8562d", "3aba3c7c"];

pub fn build_index(key: &str) -> Option<usize> {
    KEYS.iter().position(|k| *k == key)
}

/// Floor-hit sounds (weight 50, weight 25, else), Throwable_BaseST_JUST_HIT_THE_FLOOR
const SOUNDS: [[u32; 3]; 6] = [
    [0x4000001c, 0x4000001b, 0x4000001a],
    [0x40000020, 0x4000001f, 0x4000001e],
    [0x4000001f, 0x4000001e, 0x4000001d],
    [0x40000028, 0x40000027, 0x40000026],
    [0x40000040, 0x4000003f, 0x4000003e],
    [0x4000001d, 0x4000001c, 0x4000001b],
];

/// goCustomDataThrowableType (0x10000, 20 x {object id, type}); rest zero
const TYPES: [&[(u32, u32)]; 6] = [
    &[(0x2a000159, 0xe), (0x2a00015a, 0xc), (0x2a00015b, 5), (0x2a00015c, 0xf), (0x2a00015d, 0xb), (0x2a00015e, 9)],
    &[(0x2a00014f, 1), (0x2a00014e, 1)],
    &[
        (0x2a0002fc, 0xe), (0x2a0002fe, 0xc), (0x2a000300, 0xc), (0x2a0002fd, 5), (0x2a000301, 5),
        (0x2a0002ff, 0xf), (0x2a0002da, 8), (0x2a0002db, 8), (0x2a0002dc, 7), (0x2a0002e0, 0xd),
        (0x2a0002df, 6), (0x2a0002d8, 2), (0x2a0002dd, 1), (0x2a0002d9, 0xb), (0x2a0002de, 9),
    ],
    &[(0x2a0000ee, 1), (0x2a0000ef, 1)],
    &[
        (0x2a0000f9, 8), (0x2a0000fa, 8), (0x2a0000fb, 7), (0x2a0000f6, 0xd), (0x2a0000f5, 6),
        (0x2a0000f7, 2), (0x2a0000fc, 1), (0x2a0000f8, 0xb), (0x2a0000fd, 9),
    ],
    &[(0x2a000134, 1), (0x2a000135, 1), (0x2a000136, 1), (0x2a000137, 4)],
];

/// goCustomDataThrowableBase (0x100a0, 17 x 7 words), same in every build
const CUSTOM_DATA_BASE: [u32; 119] = [
    0x00000000, 0x42480000, 0xc1a00000, 0x41c80000, 0x40e00000, 0x3f333333, 0x00000003,
    0x0000000e, 0x40a00000, 0xc1a00000, 0x41c80000, 0x40000000, 0x3eae147b, 0x00000000,
    0x00000005, 0x40a00000, 0xc1a00000, 0x41c80000, 0x40000000, 0x3eae147b, 0x00000000,
    0x0000000c, 0x41c80000, 0xc1a00000, 0x41c80000, 0x40a00000, 0x3f333333, 0x00000002,
    0x0000000f, 0x42480000, 0xc1a00000, 0x41c80000, 0x40e00000, 0x3f333333, 0x00000003,
    0x00000008, 0x40a00000, 0xc1a00000, 0x41c80000, 0x40000000, 0x3e19999a, 0x00000000,
    0x00000007, 0x41c80000, 0xc1a00000, 0x41c80000, 0x40a00000, 0x3e6b851f, 0x00000001,
    0x00000006, 0x42480000, 0xc1a00000, 0x41c80000, 0x40e00000, 0x3ea8f5c3, 0x00000003,
    0x0000000d, 0x42480000, 0xc1a00000, 0x41c80000, 0x40e00000, 0x3ea3d70a, 0x00000001,
    0x00000003, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000,
    0x00000009, 0x42480000, 0xc1a00000, 0x41c80000, 0x40e00000, 0x3ea8f5c3, 0x00000003,
    0x0000000a, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000,
    0x0000000b, 0x40a00000, 0xc1a00000, 0x41c80000, 0x40000000, 0x3e19999a, 0x00000000,
    0x00000010, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000,
    0x00000004, 0x40a00000, 0xc1a00000, 0x41c80000, 0x40000000, 0x3eae147b, 0x00000000,
    0x00000002, 0x40a00000, 0xc1a00000, 0x41c80000, 0x40000000, 0x3eae147b, 0x00000000,
    0x00000001, 0x41c80000, 0xc1a00000, 0x41c80000, 0x40a00000, 0x3f333333, 0x00000002,
];

fn build_init(m: &mut Mem, data: Ptr, b: usize) {
    for (i, &(id, t)) in TYPES[b].iter().enumerate() {
        m.w32(data + 8 * i as u32, id);
        m.w32(data + 8 * i as u32 + 4, t);
    }
}

pub const goCustomDataThrowableType: u32 = 0x10000;
pub const goCustomDataThrowableBase: u32 = 0x100a0;
pub const pExtra: u32 = 0x1027c;
pub const pGameObj: u32 = 0x10280;
pub const pPhysic: u32 = 0x10284;
pub const pDebug: u32 = 0x10288;
pub const pFunction: u32 = 0x1028c;
pub const pSound: u32 = 0x10290;
pub const pMath: u32 = 0x10294;
pub const pMiscFunc: u32 = 0x10298;
pub const pGlobalDataThrowableBase: u32 = 0x1029c;
pub const pLocalDataThrowableBase: u32 = 0x102a0;
pub const uiMyNewPriority: u32 = 0x102a4;

#[inline]
fn G(e: &Engine, a: u32) -> Ptr {
    e.rt.cur_data() + (a - 0x10000)
}
fn L(e: &Engine) -> Ptr {
    e.r32(G(e, pLocalDataThrowableBase))
}
/// `*(*pGlobalDataThrowableBase + 8)` (the thrower/hero id in the game buffer)
fn global8(e: &Engine) -> u32 {
    let p = e.r32(G(e, pGlobalDataThrowableBase));
    let b = e.r32(p);
    e.r32(b + 8)
}

/// Throwable_BaseST_FIRSTMOVE @0x000110f8
pub fn Throwable_BaseST_FIRSTMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    let l = L(e);
    let id = APIgameobj::GetID(e, SELF, NULL);
    let m = APIgameobj::GetLMat(e, SELF) as Ptr;
    APImath::CMatGetAt(e, l + 0x8c, m);
    APImath::CMatGetUp(e, l + 0x80, m);
    APImath::CMatGetRight(e, l + 0x98, m);
    APImath::CMatGetPos(e, l + 0xa4, m);
    let ty = G(e, goCustomDataThrowableType);
    let base = G(e, goCustomDataThrowableBase);
    'outer: for i in 0..0x14u32 {
        if e.r32(ty + i * 8) == id {
            let t = e.r32(ty + i * 8 + 4);
            for j in 0..=0x10u32 {
                let row = base + j * 0x1c;
                if e.r32(row) == t {
                    for k in 0..6 {
                        let v = e.r32(row + 4 + 4 * k);
                        e.w32(l + 0xb0 + 4 * k, v);
                    }
                    break;
                }
            }
            break 'outer;
        }
    }
    let c = e.alloca(0x40);
    APIgameobj::GetClippingParams(e, SELF, c);
    for (o, v) in [
        (4, 0x41c8_0000),
        (0x10, 0x428c_0000),
        (0xc, 0),
        (8, 7),
        (0x14, 0x27),
        (0x18, 0),
        (0x1c, 0xbf80_0000),
        (0x24, 0x6a7),
        (0x20, 0x6a7),
    ] {
        e.w32(c + o, v);
    }
    APIgameobj::SetClippingParams(e, SELF, c, 0);
    Throwable_baseFUNC_CreatePhysic(e);
    Throwable_BaseFUNC_SetInitalAttackParams(e);
    APIgameobj::SetContextFlag(e, SELF, 0xd);
    APIgameobj::SetGroupID(e, SELF, 0x14);
    APIgameobj::SetNextState(e, SELF, 1);
    Ok(0)
}

/// Throwable_BaseST_BASICMOVE_LOOP @0x00011530 (also ST_BEEN_DROPED_LOOP @0x00011928, same body)
pub fn Throwable_BaseST_BASICMOVE_LOOP(e: &mut Engine, _: u32) -> LResult<u64> {
    let v = e.alloca(0x10);
    APIgameobj::GetPosition(e, SELF, 0x7fff_ffff, v);
    if e.rf(v + 4) <= -50.0 {
        Throwable_baseFUNC_MessageFunction(e, 0xc9, 0, NULL)?;
    }
    Ok(0)
}

/// Throwable_BaseST_BEEN_DROPED_LOOP @0x00011928
pub fn Throwable_BaseST_BEEN_DROPED_LOOP(e: &mut Engine, a: u32) -> LResult<u64> {
    Throwable_BaseST_BASICMOVE_LOOP(e, a)
}

/// Throwable_BaseST_BEEN_PICKED @0x000115d0
pub fn Throwable_BaseST_BEEN_PICKED(e: &mut Engine, _: u32) -> LResult<u64> {
    let l = L(e);
    let v = e.alloca(0x10);
    let y = e.r32(l + 0xc0);
    e.w32(v, 0);
    e.w32(v + 4, y);
    e.w32(v + 8, 0);
    e.w32(l + 0x30, 1);
    let (thrower, holder) = (e.r32(l + 0x3c), e.r32(l + 0x14));
    APIgameobj::AddChild(e, thrower, holder, SELF, 1);
    let p = e.alloca(0x40);
    let h = e.r32(l + 0xc);
    APIphysic::DMGetParams(e, h, p);
    let w = e.r32(l + 0xbc);
    e.w32(p, w);
    APIphysic::DMSetParams(e, h, p);
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 5, 7);
    APIgameobj::SetTranslation(e, SELF, v);
    APIphysic::SetFlags(e, SELF, 1);
    Throwable_BaseFUNC_SendInfoToThrower(e);
    APIgameobj::SetNextState(e, SELF, 1);
    Ok(0)
}

/// Throwable_BaseST_BEEN_DROPED @0x00011758
pub fn Throwable_BaseST_BEEN_DROPED(e: &mut Engine, _: u32) -> LResult<u64> {
    let l = L(e);
    let v = e.alloca(0x10);
    e.wf(v, 0.5);
    e.wf(v + 4, 4.0);
    e.wf(v + 8, 0.5);
    e.w32(l + 0x3c, 0);
    APIphysic::ResetFlags(e, SELF, 1);
    APIgameobj::LeaveParent(e, SELF, 0);
    if e.r32(l + 0x30) != 0 {
        e.w32(l + 0x30, 0);
        Throwable_BaseFUNC_AlignWithFloor(e);
        APIgameobj::SetContextFlag(e, SELF, 0xf);
        APIphysic::PrimResetIntAttribBits(e, SELF, 0x32a, 0, 8);
        APIphysic::PrimResetIntAttribBits(e, SELF, 0x32a, 2, 4);
        APIphysic::PrimResetIntAttribBits(e, SELF, 0x32a, 8, 2);
        APIphysic::PrimResetIntAttribBits(e, SELF, 0x32a, 8, 8);
        APIphysic::Impulse(e, SELF, v);
    }
    e.w32(l + 0x34, 3);
    APIgameobj::SetNextState(e, SELF, 4);
    Ok(0)
}

/// Throwable_BaseST_BEEN_THROWN @0x000119c8
pub fn Throwable_BaseST_BEEN_THROWN(e: &mut Engine, _: u32) -> LResult<u64> {
    let l = L(e);
    APIphysic::ResetFlags(e, SELF, 1);
    APIphysic::PrimSetIntAttribBits(e, SELF, 0x32a, 0, 8);
    let bits = e.r32(l + 0x20);
    APIphysic::PrimSetIntAttribBits(e, SELF, 0x32a, 7, bits);
    APIphysic::PrimSetIntAttribBits(e, SELF, 0x32a, 8, 10);
    APIphysic::PrimSetIntAttribBits(e, SELF, 0x32a, 2, 4);
    APIgameobj::LeaveParent(e, SELF, 0);
    APIgameobj::SetContextFlag(e, SELF, 0xd);
    APIphysic::SetStateChangeNotifyBits(e, SELF, 1);
    Throwable_BaseFUNC_SetFlyingParams(e);
    let h = Throwable_BaseFUNC_GetRelativeHeight(e);
    e.w32(l + 0x18, h);
    let v = e.alloca(0x10);
    let tgt = global8(e);
    APIgameobj::GetPosition(e, tgt, 0x7fff_ffff, v);
    for k in 0..3 {
        let f = e.r32(v + 4 * k);
        e.w32(l + 0x40 + 4 * k, f);
    }
    e.w32(l + 0x4c, 0);
    e.w32(l + 0x50, 0);
    e.w32(l + 0x54, 0);
    let a = e.r32(l + 0x1c);
    e.w32(l + 0x58, a);
    e.w32(l + 0x5c, 2);
    let a = e.r32(l + 0x18);
    e.w32(l + 0x60, a);
    let a = e.r32(l + 8);
    e.w32(l + 0x64, a);
    APIphysic::JumpOnCoor(e, SELF, l + 0x40);
    APIgameobj::SetNextState(e, SELF, 4);
    Ok(0)
}

/// Throwable_BaseST_BEEN_THROWN_LOOP @0x00011c60
pub fn Throwable_BaseST_BEEN_THROWN_LOOP(e: &mut Engine, _: u32) -> LResult<u64> {
    let l = L(e);
    let zero = e.alloca(0x10);
    e.w32(zero, 0);
    e.w32(zero + 4, 0);
    e.w32(zero + 8, 0);
    e.w32(l + 200, 0);
    let ph = e.r32(l + 0x34);
    if ph == 1 || ph == 2 {
        let a = e.rf(l + 0x38);
        APIgameobj::Rotate(e, SELF, l + 0x68, a);
    }
    let vel = e.alloca(0x10);
    APIphysic::GetVelocity(e, SELF, vel);
    let eq = APImath::CVectorAreEqualEpsilon(e, vel, zero, 2);
    if eq != 0 && e.r32(l + 0x34) == 3 {
        let s = e.cstr("THROWABLE-> RESETER VEOCITY = 0.0F\n");
        APIextra::null_printf(e, s, 0);
        e.w32(l + 0x34, 1);
        let p = e.alloca(0x40);
        let h = e.r32(l + 0xc);
        APIphysic::DMGetParams(e, h, p);
        e.w32(p, 0x447a_0000);
        APIphysic::DMSetParams(e, h, p);
        APIphysic::SetVelocity(e, SELF, 0.0, 0.0, 0.0);
        APIgameobj::UnregisterNotify(e, SELF, 0xffff_ffff, 5);
        APIgameobj::SetGroupID(e, SELF, 0x14);
        APIgameobj::SetNextState(e, SELF, 1);
    }
    Ok(0)
}

/// Throwable_BaseST_JUST_HIT_THE_FLOOR @0x00011e80 (state-change notify, param = {new bits, changed bits})
pub fn Throwable_BaseST_JUST_HIT_THE_FLOOR(e: &mut Engine, p: u32) -> LResult<u64> {
    if e.r32(p + 4) & 1 != 0 && e.r32(p) & 1 != 0 {
        let l = L(e);
        let ph = e.r32(l + 0x34);
        if ph == 1 {
            if e.r32(l + 0x2c) == 0 {
                let t = e.r32(l + 0x3c);
                APIgameobj::SendMessageTo(e, t, NULL, 0x118, 0, NULL);
            }
            APIphysic::PrimResetIntAttribBits(e, SELF, 0x32a, 0, 8);
            APIphysic::PrimResetIntAttribBits(e, SELF, 0x32a, 2, 4);
            APIphysic::PrimResetIntAttribBits(e, SELF, 0x32a, 8, 2);
            APIphysic::PrimResetIntAttribBits(e, SELF, 0x32a, 8, 8);
            e.w32(l + 0x34, 2);
            e.w32(l + 0x38, 0x3e49_0fdb);
            let snd = SOUNDS[e.rt.cur_build()];
            let w = e.rf(l + 0xb0);
            // quirk: no `else` after the 50.0 test, so a weight-50 object plays snd[0] AND snd[2]
            if w == 50.0 {
                APInewSound::Play(e, SELF, snd[0], 0);
            }
            if e.rf(l + 0xb0) == 25.0 {
                APInewSound::Play(e, SELF, snd[1], 0);
            } else {
                APInewSound::Play(e, SELF, snd[2], 0);
            }
        } else if ph == 2 {
            e.w32(l + 0x34, 3);
            Throwable_BaseFUNC_AlignWithFloor(e);
        }
    }
    Ok(0)
}

/// Throwable_baseCB_NOTIFY_COLLIDE_OBJ @0x00012108
pub fn Throwable_baseCB_NOTIFY_COLLIDE_OBJ(e: &mut Engine, p: u32) -> LResult<u64> {
    let l = L(e);
    e.w32(l + 200, 1);
    Ok((e.r32(p + 0xc) == 0x32a) as u64)
}

/// Throwable_baseFUNC_CreatePhysic @0x00012170
fn Throwable_baseFUNC_CreatePhysic(e: &mut Engine) {
    let l = L(e);
    // one frame mirroring the C stack layout (sp-0x80 .. sp-0x30): the param buffers overlap
    // in the original if DMGetParams writes more than 16 bytes, keep the same placement.
    let f = e.alloca(0x60);
    APIphysic::CreatePhysic(e, SELF, 0);
    APIphysic::SetFlags(e, SELF, 6);
    let pr = e.r32(G(e, uiMyNewPriority)).wrapping_add(1);
    e.w32(G(e, uiMyNewPriority), pr);
    let w = e.rf(l + 0xb0);
    APIphysic::SetWeight(e, SELF, w);
    APIphysic::SetPriority(e, SELF, pr);
    APIphysic::SetPrimaryPrim(e, SELF, 0x32a);
    APIphysic::SetHardness(e, SELF, 0.9);
    APIphysic::SetRebound(e, SELF, 0.5);
    let h = APIphysic::DMAdd(e, SELF, 4);
    e.w32(l + 4, h);
    APIphysic::DMGetParams(e, h, f);
    for (o, v) in [(0, 0), (4, 0), (8, 0), (0xc, 1)] {
        e.w32(f + o, v);
    }
    APIphysic::DMSetParams(e, h, f);
    APIphysic::DMSetContextMask(e, h, 2);
    let h = APIphysic::DMAdd(e, SELF, 8);
    e.w32(l + 0xc, h);
    let (a, b) = (e.r32(l + 0xb8), e.r32(l + 0xbc));
    e.w32(f + 0x14, a);
    e.w32(f + 0x10, b);
    APIphysic::DMSetParams(e, h, f + 0x10);
    APIphysic::DMSetContextMask(e, h, 4);
    let h = APIphysic::DMAdd(e, SELF, 7);
    e.w32(l + 0x10, h);
    e.w32(f + 0x40, 0);
    e.w32(f + 0x44, 5);
    // quirk: params read back but never set
    APIphysic::DMGetParams(e, h, f + 0x40);
    APIphysic::DMSetContextMask(e, h, 1);
    let h = APIphysic::DMAdd(e, SELF, 0);
    e.w32(l + 8, h);
    APIphysic::DMGetParams(e, h, f + 0x20);
    let b4 = e.r32(l + 0xb4);
    e.w32(f + 0x20, 0);
    e.w32(f + 0x24, b4);
    e.w32(f + 0x28, 0);
    APIphysic::DMSetParams(e, h, f + 0x20);
    APIphysic::DMSetContextMask(e, h, 8);
}

/// Throwable_BaseFUNC_SetFlyingParams @0x000124f8
fn Throwable_BaseFUNC_SetFlyingParams(e: &mut Engine) {
    let l = L(e);
    e.w32(l + 0x34, 1);
    e.w32(l + 0x38, 0x3dc9_0fdb);
    e.w32(l + 0x2c, 0);
    APIgameobj::GetLookAt(e, SELF, 0x7fff_ffff, l + 0x74);
    let v = if 0.0 <= e.rf(l + 0x74) { 0x3e80_0000 } else { 0xbe80_0000 };
    e.w32(l + 0x68, v);
    e.w32(l + 0x6c, 0);
    APIgameobj::GetLookAt(e, SELF, 0x7fff_ffff, l + 0x74);
    let v = if 0.0 <= e.rf(l + 0x7c) { 0x3f80_0000 } else { 0xbf80_0000 };
    e.w32(l + 0x70, v);
}

/// Throwable_BaseFUNC_SetInitalAttackParams @0x00012640
fn Throwable_BaseFUNC_SetInitalAttackParams(e: &mut Engine) {
    let l = L(e);
    let (a, b, c, d, bits) = match e.r32(l + 0xc4) {
        1 => (0x4040_0000, 0x4120_0000, 0x40c0_0000, 0x10, 0x40),
        0 => (0x4080_0000, 0x4170_0000, 0x4100_0000, 2, 0x20),
        2 => (0x4000_0000, 0x4100_0000, 0x4080_0000, 0x200, 0x80),
        3 => (0x3f80_0000, 0x4080_0000, 0x4000_0000, 0x80000, 0x80),
        _ => return,
    };
    e.w32(l + 0x24, a);
    e.w32(l + 0x28, b);
    e.w32(l + 0x1c, c);
    e.w32(l + 0x20, d);
    APIphysic::PrimSetIntAttribBits(e, SELF, 0x32a, 8, bits);
}

/// Throwable_BaseFUNC_GetRelativeHeight @0x00012890 (returns the float bits)
fn Throwable_BaseFUNC_GetRelativeHeight(e: &mut Engine) -> u32 {
    let l = L(e);
    let tgt = global8(e);
    let d = LIBApiCFunctions::GetDistance(e, SELF as i32, tgt, 0, 1);
    let div = match e.r32(l + 0xc4) {
        1 => Some(4.0),
        0 => Some(5.0),
        2 => Some(3.0),
        3 => Some(2.0),
        _ => None,
    };
    if let Some(div) = div {
        e.wf(l + 0x18, d / div);
    }
    e.r32(l + 0x18)
}

/// Throwable_BaseFUNC_SendInfoToThrower @0x00012a00
fn Throwable_BaseFUNC_SendInfoToThrower(e: &mut Engine) {
    let l = L(e);
    if e.r32(l + 0x30) != 0 {
        let t = e.r32(l + 0x3c);
        APIgameobj::SendMessageTo(e, t, NULL, 0x117, 0, l + 0x24);
        let t = e.r32(l + 0x3c);
        APIgameobj::SendMessageTo(e, t, NULL, 0x116, 0, l + 0x28);
    }
}

/// Throwable_BaseFUNC_AlignWithFloor @0x00012ab8 (empty)
fn Throwable_BaseFUNC_AlignWithFloor(_e: &mut Engine) {}

/// Throwable_baseFUNC_MessageFunction @0x00012ad8
pub fn Throwable_baseFUNC_MessageFunction(e: &mut Engine, m: u32, p2: u32, _p3: Ptr) -> LResult<u64> {
    let l = L(e);
    match m {
        199 => {
            e.w32(l + 0x2c, 1);
            let t = e.r32(l + 0x3c);
            APIgameobj::SendMessageTo(e, t, NULL, 199, 0, NULL);
        }
        0xc9 => {
            let mat = APIgameobj::GetLMat(e, SELF) as Ptr;
            APImath::CMatSetAt(e, mat, l + 0x8c);
            APImath::CMatSetUp(e, mat, l + 0x80);
            APImath::CMatSetPos(e, mat, l + 0xa4);
            APImath::CMatSetRight(e, mat, l + 0x98);
            APIgameobj::SetLMat(e, SELF, mat);
            APIgameobj::SetGroupID(e, SELF, 0x14);
            APIgameobj::SetNextState(e, SELF, 1);
        }
        0x110 => {
            APIgameobj::SetNextState(e, SELF, 3);
        }
        0x111 => {
            APIgameobj::SetGroupID(e, SELF, 0);
        }
        0x112 => {
            e.w32(l + 0x30, 0);
            APIgameobj::SetGroupID(e, SELF, 0x14);
        }
        0x113 => {
            e.w32(l + 0x14, p2);
            APIgameobj::SetNextState(e, SELF, 2);
        }
        0x114 => {
            APIgameobj::SetNextState(e, SELF, 5);
        }
        0x115 => {
            e.w32(l + 0x3c, p2);
        }
        _ => {}
    }
    Ok(0)
}

/// throwable_baseConstructor @0x00012d90
pub fn throwable_baseConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    if instanceCount == 1 {
        let fns: [(i32, ExtFn); 9] = [
            (0, Throwable_BaseST_FIRSTMOVE),
            (1, Throwable_BaseST_BASICMOVE_LOOP),
            (5, Throwable_BaseST_BEEN_DROPED),
            (6, Throwable_BaseST_BEEN_DROPED_LOOP),
            (2, Throwable_BaseST_BEEN_PICKED),
            (3, Throwable_BaseST_BEEN_THROWN),
            (4, Throwable_BaseST_BEEN_THROWN_LOOP),
            (7, Throwable_BaseST_JUST_HIT_THE_FLOOR),
            (9, Throwable_baseCB_NOTIFY_COLLIDE_OBJ),
        ];
        for (i, f) in fns {
            APIcons::SetExternalFunction(e, i, f);
        }
        for (a, v) in [
            (pExtra, APIcons::GetApiExtra(e)),
            (pGameObj, APIcons::GetApiGameObj(e)),
            (pPhysic, APIcons::GetApiPhysic(e)),
        ] {
            let a = G(e, a);
            e.w32(a, v);
        }
        for (a, n) in [(pDebug, "debug"), (pFunction, "ApiCFunctions")] {
            let s = e.cstr(n);
            let v = APIcons::GetApiCustom(e, s);
            let a = G(e, a);
            e.w32(a, v);
        }
        let v = APIcons::GetApiNewSound(e);
        let a = G(e, pSound);
        e.w32(a, v);
        let v = APIcons::GetApiMath(e);
        let a = G(e, pMath);
        e.w32(a, v);
        let s = e.cstr("MiscFunc");
        let v = APIcons::GetApiCustom(e, s);
        let a = G(e, pMiscFunc);
        e.w32(a, v);
        let slot = G(e, pGlobalDataThrowableBase);
        APIcons::GetGlobalData(e, slot, 4);
        let b = APIgameobj::GetGameBuffer(e, 0, 0x8f8, 1);
        let gd = e.r32(slot);
        e.w32(gd, b);
    }
    let slot = G(e, pLocalDataThrowableBase);
    APIcons::GetLocalData(e, slot, 0xcc);
    APIcons::SetMessageFunction(e, Throwable_baseFUNC_MessageFunction);
    APIcons::SetInitialState(e, 0);
    LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 2, NULL, 0);
    Ok(())
}

/// throwable_baseDestructor @0x00013060
pub fn throwable_baseDestructor(e: &mut Engine, _: i32) -> LResult<()> {
    Ok(())
}

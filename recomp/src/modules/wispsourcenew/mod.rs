//! `wispsourcenew`: the "huge wisp" source, a spinning smoky object that carries three wisps
//! (launched copies of the level's wisp latent). Touching one of its three collision volumes plays
//! a release animation and lets the book suck the matching wisp (message 0x165, 0x166 when the
//! animation ends); when a carried wisp is collected (message 0x120 from it) a new one is launched in
//! its place. Port of `Game Logic Modules/Globals/Collectables/Wisp/wispsourcenew__<sha8>/WispSourceNew.c`
//! (29 functions, source `Source/Globals/Collectables/Wisp/WispSourceNew.c`).
//! Reference build: wispsourcenew__1a407d93 (EP6L05). The 15 builds share the code and differ in 8
//! literals (book id, wisp latent id, the animation/notify indices of the volumes), see [`builds`].
//!
//! Local data (`pLocalDataHugeWisp2_base`, 0xf8 bytes): +0x00/+0x14/+0x28 three EFBlendParams (id
//! 0xbb00000N, 0.2, 1.0, 0, 0.5) for volumes 1/2/3; +0x40/+0x44/+0x48 "volume N animation playing";
//! +0x70 smoke 1, +0x74 smoke 3, +0x78 flash, +0x7c flash-anim emitters; +0x80/+0x84/+0x88 motion
//! trails; +0x8c..+0xdc 21 sprites; +0xec/+0xf0/+0xf4 the carried wisps (volumes 1/2/3).
//! Global data (`pGlobalDataHugeWisp2_base`, 0x2c): +0 game buffer 0, +4 game buffer 2, +8 game
//! buffer 1 (+4 book power); +0xc..+0x18 four sprite groups; +0x1c smoke 1, +0x20 smoke 3,
//! +0x24 flash, +0x28 flash-anim particle types.
//!
//! Ghidra shows `pLocalDataHugeWisp2_base` with two types: word-indexed in ST_FIRSTMOVE
//! (`[0x3b]` = +0xec) and byte offsets elsewhere; the port uses byte offsets throughout.
//! `null_printf` calls (debug output, compiled to nothing) are kept; where the original passes no
//! second argument the register's leftover value is unobservable and the port passes 0.
#![allow(non_snake_case, non_upper_case_globals, unused_variables)]

pub mod builds;
mod particle_settings;
use builds::*;
pub use particle_settings::*;

use crate::engine::*;
use crate::logic::{LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "wispsourcenew",
    // globals 0x10000..0x10038; gpoLanguagesNamesAndISOCodes (0x10038, 10 bytes, "Abkhazian") is
    // never read by the module and is not reproduced
    data_size: 0x48,
    ctor: wispsourcenewConstructor,
    dtor: wispsourcenewDestructor,
    register: None,
    init: &[],
    build_init: None,
};

/// Index of a build in [`builds::BUILDS`] by the first 8 hex digits of its SHA-1.
pub fn build_index(sha8: &str) -> Option<usize> {
    BUILDS.iter().position(|b| b.sha8 == sha8)
}

// ---- module globals (_globals.h) ----
pub const pAnimMngr: u32 = 0x10000;
pub const pMiscFunc: u32 = 0x10004;
pub const pGraphic: u32 = 0x10008;
pub const pExtra: u32 = 0x1000c;
pub const pGameObj: u32 = 0x10010;
pub const pPhysic: u32 = 0x10014;
pub const pAnim: u32 = 0x10018;
pub const pParticle: u32 = 0x1001c;
pub const pMath: u32 = 0x10020;
pub const pStdlib: u32 = 0x10024;
pub const pLight: u32 = 0x10028;
pub const pSound: u32 = 0x1002c;
pub const pLocalDataHugeWisp2_base: u32 = 0x10030;
pub const pGlobalDataHugeWisp2_base: u32 = 0x10034;

#[inline]
fn G(e: &Engine, a: u32) -> Ptr {
    e.rt.cur_data() + (a - 0x10000)
}
/// current value of `pLocalDataHugeWisp2_base`
#[inline]
fn L(e: &Engine) -> Ptr {
    e.r32(G(e, pLocalDataHugeWisp2_base))
}
/// current value of `pGlobalDataHugeWisp2_base`
#[inline]
fn GD(e: &Engine) -> Ptr {
    e.r32(G(e, pGlobalDataHugeWisp2_base))
}
/// per-build literal (see builds.rs)
#[inline]
fn k(e: &Engine, i: usize) -> u32 {
    BUILDS[e.rt.cur_build()].k[i]
}

/// 0x00010df0 state 0: three wisps, blend params, physics, smoke, trails; then BASICMOVE_SET
pub fn HugeWisp2_baseST_FIRSTMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    HugeWisp2_baseFUNC_CreateChildren(e);
    let l = L(e);
    for (i, id) in [0xbb00_0001u32, 0xbb00_0002, 0xbb00_0003].into_iter().enumerate() {
        let p = l + i as u32 * 0x14;
        e.w32(p, id);
        e.w32(p + 4, 0x3e4c_cccd);
        e.w32(p + 8, 0x3f80_0000);
        e.w32(p + 0xc, 0);
        e.w32(p + 0x10, 0x3f00_0000);
    }
    APIphysic::CreatePhysic(e, SELF, 0);
    APIphysic::SetFlags(e, SELF, 2);
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 3, 3);
    HugeWisp2_baseFUNC_StartSmoke(e);
    for off in [0xecu32, 0xf0, 0xf4] {
        let c = e.r32(L(e) + off);
        APIgameobj::SendMessageTo(e, c, NULL, 0x166, 0, NULL);
    }
    HugeWisp2_baseFUNC_CreateMotionTrail(e);
    HugeWisp2_baseFUNC_SetClipping(e);
    APIgameobj::SetGameObjectVisible(e, SELF, 1);
    APIgameobj::SetNextState(e, SELF, 1);
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 0xb, 7);
    Ok(0)
}

/// 0x000110e0 state 1: idle animation, hum sound 0x2d, then LOOP
pub fn HugeWisp2_baseST_BASICMOVE_SET(e: &mut Engine, _: u32) -> LResult<u64> {
    LIBAnimMngr::PlayAnim(e, 0.0, 1, 2, 0);
    APInewSound::Play(e, SELF, 0x2d, 0);
    APIgameobj::SetNextState(e, SELF, 2);
    Ok(0)
}

/// 0x00011178 state 2: spin 3 degrees per frame; jitter the flash emitter and fire it
pub fn HugeWisp2_baseST_BASICMOVE_LOOP(e: &mut Engine, _: u32) -> LResult<u64> {
    let DStack_50 = e.alloca(16);
    let DStack_40 = e.alloca(16);
    e.wv3(DStack_50, [0.0, 1.0, 0.0]);
    APIgameobj::Rotate(e, SELF, DStack_50, 0.05235988);
    let uVar1 = APIstdlib::MyRand(e);
    let x = ((uVar1 as i32) % 9) as f32 * 0.05 - 0.2;
    e.wf(DStack_40, x);
    let uVar1 = APIstdlib::MyRand(e);
    let y = ((uVar1 as i32) % 0xf) as f32 * 0.05 - 0.1;
    e.wf(DStack_40 + 4, y);
    let uVar1 = APIstdlib::MyRand(e);
    let z = ((uVar1 as i32) % 9) as f32 * 0.05 - 0.2;
    e.wf(DStack_40 + 8, z);
    let fl = e.r32(L(e) + 0x78);
    APIgameobj::SetTranslation(e, fl, DStack_40);
    APIparticle::EmitterStart(e, fl as i32, 0);
    Ok(0)
}

/// 0x00011348 callback 3 (collision notify 3; data +4 = volume 1..3, +8 = other object): touched
/// by the current hero or the book while the wisps are on, no volume busy and the power not full:
/// play that volume's release animation, let the book suck its wisp (0x165), notify at the end
pub fn HugeWisp2_baseCB_NOTIFY_COLLIDE_OBJ(e: &mut Engine, param_1: u32) -> LResult<u64> {
    let gd = GD(e);
    let l = L(e);
    let other = e.r32(param_1 + 8);
    if (other == e.r32(e.r32(gd) + 8) || other == k(e, ID_2a000009))
        && e.r32(e.r32(gd + 4)) & 1 != 0
        && e.r32(l + 0x40) == 0
        && e.r32(l + 0x44) == 0
        && e.r32(l + 0x48) == 0
        && e.rf(e.r32(gd + 8) + 4) < 180.0
    {
        let uVar1 = e.r32(param_1 + 4);
        // (volume, busy flag, blend params, carried wisp, animation literal, notify literal, callback)
        let v = match uVar1 {
            2 => Some(("#2\n", 0x44u32, 0x14u32, 0xf0u32, HugeWisp2_baseCB_NOTIFY_COLLIDE_OBJ_0_1, HugeWisp2_baseCB_NOTIFY_COLLIDE_OBJ_0_2, 5u32)),
            1 => Some(("#1\n", 0x40, 0x00, 0xec, HugeWisp2_baseCB_NOTIFY_COLLIDE_OBJ_0_3, HugeWisp2_baseCB_NOTIFY_COLLIDE_OBJ_0_4, 4)),
            3 => Some(("#3\n", 0x48, 0x28, 0xf4, HugeWisp2_baseCB_NOTIFY_COLLIDE_OBJ_0_5, HugeWisp2_baseCB_NOTIFY_COLLIDE_OBJ_0_6, 6)),
            _ => None,
        };
        if let Some((msg, busy, blend, child, anim, notify, cb)) = v {
            let s = e.cstr(msg);
            APIextra::null_printf(e, s, 0);
            let l = L(e);
            e.w32(l + busy, 1);
            let a = k(e, anim);
            APIanim::PlayBlend(e, SELF, a, 1, 0.0, l + blend);
            let c = e.r32(L(e) + child);
            APIgameobj::SendMessageTo(e, c, NULL, 0x165, 0, NULL);
            let n = k(e, notify);
            APIgameobj::RegisterNotify(e, SELF, n, 1, cb);
        }
    }
    Ok(0)
}

/// 0x000116e8 callback 4: volume 1 animation done, its wisp may no longer be sucked
pub fn HugeWisp2_baseCB_NOTIFY_ANIM_LF1(e: &mut Engine, _: u32) -> LResult<u64> {
    let l = L(e);
    e.w32(l + 0x40, 0);
    let c = e.r32(l + 0xec);
    APIgameobj::SendMessageTo(e, c, NULL, 0x166, 0, NULL);
    Ok(0)
}

/// 0x00011760 callback 5: volume 2 animation done
pub fn HugeWisp2_baseCB_NOTIFY_ANIM_LF2(e: &mut Engine, _: u32) -> LResult<u64> {
    let l = L(e);
    e.w32(l + 0x44, 0);
    let c = e.r32(l + 0xf0);
    APIgameobj::SendMessageTo(e, c, NULL, 0x166, 0, NULL);
    Ok(0)
}

/// 0x000117d8 callback 6: volume 3 animation done
pub fn HugeWisp2_baseCB_NOTIFY_ANIM_LF3(e: &mut Engine, _: u32) -> LResult<u64> {
    let l = L(e);
    e.w32(l + 0x48, 0);
    let c = e.r32(l + 0xf4);
    APIgameobj::SendMessageTo(e, c, NULL, 0x166, 0, NULL);
    Ok(0)
}

/// 0x00011850 callback 7 (notify 0xb, animation event): flash and sound 0x3b
pub fn HugeWisp2_baseCB_NOTIFY_FROM_ANIM(e: &mut Engine, _: u32) -> LResult<u64> {
    HugeWisp2_baseFUNC_StartFlash_Anim(e);
    APInewSound::Play(e, SELF, 0x3b, 0);
    Ok(0)
}

/// 0x000118c0 message 0x120 (a carried wisp was collected; sub = its id): replace it
pub fn HugeWisp2_base_MyMessageFunction(e: &mut Engine, param_1: u32, param_2: u32, _: Ptr) -> LResult<u64> {
    if param_1 == 0x120 {
        HugeWisp2_baseFUNC_ManageChildren(e, param_2 as i32);
    }
    Ok(0)
}

/// 0x00011948 the four shared particle types
pub fn HugeWisp2_baseFUNC_CreateSmokeType(e: &mut Engine) -> LResult<()> {
    type Set = fn(&mut Engine, u32, u32) -> LResult<()>;
    let types: [(u32, &str, &str, Set); 4] = [
        (0x1c, "broue", "HugeWisp2_base_Smoke1", HugeWisp2_baseFUNC_SetParticle_SMOKE1),
        (0x24, "flash2", "HugeWisp2_base_Flash", HugeWisp2_baseFUNC_SetParticle_FLASH),
        (0x20, "broue", "HugeWisp2_base_Smoke3", HugeWisp2_baseFUNC_SetParticle_SMOKE3),
        (0x28, "star_light", "HugeWisp2_base_Flash_Anim", HugeWisp2_baseFUNC_SetParticle_FLASH_ANIM),
    ];
    for (off, tex, name, set) in types {
        let uVar1 = APIparticle::TypeCreate(e);
        let gd = GD(e);
        e.w32(gd + off, uVar1);
        let t = e.ri32(gd + off);
        let s = e.cstr(tex);
        APIparticle::TypeSetTexture(e, t, s);
        let s = e.cstr(name);
        APIparticle::TypeSetName(e, t, s);
        let pp = e.r32(G(e, pParticle));
        let t = e.r32(GD(e) + off);
        set(e, pp, t)?;
    }
    Ok(())
}

/// 0x00011bc8 emitters: smoke 1 and 3 on the root, flash and flash-anim as child objects (the
/// flash-anim 0.25 up)
pub fn HugeWisp2_baseFUNC_CreateSmokeParticle(e: &mut Engine) {
    let DStack_30 = e.alloca(16);
    e.wv3(DStack_30, [0.0, 0.25, 0.0]);
    for (slot, toff, joint, child) in [(0x70u32, 0x1cu32, 0u32, false), (0x78, 0x24, 0x6fff_ffff, true), (0x74, 0x20, 0, false), (0x7c, 0x28, 0x6fff_ffff, true)] {
        let t = e.ri32(GD(e) + toff);
        let uVar1 = APIparticle::EmitterCreate(e, t);
        let l = L(e);
        e.w32(l + slot, uVar1);
        APIparticle::EmitterSetJoint(e, uVar1 as i32, SELF, joint);
        if child {
            let em = e.r32(L(e) + slot);
            APIgameobj::AddChild(e, SELF, 0, em, 1);
        }
    }
    let em = e.r32(L(e) + 0x7c);
    APIgameobj::SetTranslation(e, em, DStack_30);
}

/// 0x00011dd8
pub fn HugeWisp2_baseFUNC_StartSmoke(e: &mut Engine) {
    let a = e.ri32(L(e) + 0x70);
    APIparticle::EmitterStart(e, a, 1);
    let a = e.ri32(L(e) + 0x74);
    APIparticle::EmitterStart(e, a, 1);
}

/// 0x00011e48
pub fn HugeWisp2_baseFUNC_StopSmoke(e: &mut Engine) {
    let a = e.ri32(L(e) + 0x70);
    APIparticle::EmitterStop(e, a);
    let a = e.ri32(L(e) + 0x74);
    APIparticle::EmitterStop(e, a);
}

/// 0x00011eb0 the four emitters and (also) the four shared types. The destructor then destroys the
/// types again when the last instance goes: kept as in the original.
pub fn HugeWisp2_baseFUNC_DestroySmokeParticle(e: &mut Engine) {
    for off in [0x70u32, 0x74, 0x78, 0x7c] {
        let a = e.ri32(L(e) + off);
        APIparticle::EmitterDestroy(e, a);
    }
    for off in [0x1cu32, 0x20, 0x24, 0x28] {
        let t = e.ri32(GD(e) + off);
        APIparticle::TypeDestroy(e, t);
    }
}

/// 0x00011fd8
pub fn HugeWisp2_baseFUNC_DestroySmokeType(e: &mut Engine) {
    for off in [0x1cu32, 0x20, 0x24, 0x28] {
        let t = e.ri32(GD(e) + off);
        APIparticle::TypeDestroy(e, t);
    }
}

/// 0x00012080
pub fn HugeWisp2_baseFUNC_StartFlash_Anim(e: &mut Engine) {
    let a = e.ri32(L(e) + 0x7c);
    APIparticle::EmitterStart(e, a, 0);
}

/// child slot -> the joint it is attached to (0 -> 0x16, 1 -> 0xc, 2 -> 0x11), as the original's
/// if/else chain
fn child_joint(i: i32) -> Option<u32> {
    if i == 1 {
        Some(0xc)
    } else if i < 2 {
        if i == 0 { Some(0x16) } else { None }
    } else if i == 2 {
        Some(0x11)
    } else {
        None
    }
}

/// 0x00012da8 launch the three carried wisps, attach them to joints 0x16/0xc/0x11 and activate them
pub fn HugeWisp2_baseFUNC_CreateChildren(e: &mut Engine) {
    let mut iStack_30: i32 = 0;
    while iStack_30 < 3 {
        let slot = 0xec + iStack_30 as u32 * 4;
        let lat = k(e, ID_2a800069);
        let uVar1 = APIgameobj::LaunchLatant(e, lat);
        let l = L(e);
        e.w32(l + slot, uVar1);
        let c = e.r32(L(e) + slot);
        APIgameobj::SendMessageTo(e, c, NULL, 0x11f, 1, NULL);
        let s = e.cstr("Wisp2_baseFUNC_CreateChildren> Create Child Id: %u\n");
        let c = e.r32(L(e) + slot);
        APIextra::null_printf(e, s, c);
        if let Some(j) = child_joint(iStack_30) {
            let s = e.cstr(match iStack_30 {
                1 => "#1\n",
                0 => "#0\n",
                _ => "#2\n",
            });
            APIextra::null_printf(e, s, 0);
            let c = e.r32(L(e) + slot);
            APIgameobj::AddChild(e, SELF, j, c, 1);
        }
        let c = e.r32(L(e) + slot);
        APIgameobj::SendMessageTo(e, c, NULL, 0x98, 0, NULL);
        iStack_30 += 1;
    }
}

/// 0x00013098 replace the carried wisp `param_1` (collected) with a new launch on the same joint.
/// The new wisp is not sent 0x98 here (unlike CreateChildren): kept as in the original.
pub fn HugeWisp2_baseFUNC_ManageChildren(e: &mut Engine, param_1: i32) {
    let mut iStack_2c: i32 = 0;
    while iStack_2c < 3 {
        let slot = 0xec + iStack_2c as u32 * 4;
        if e.ri32(L(e) + slot) == param_1 {
            let lat = k(e, ID_2a800069);
            let uVar1 = APIgameobj::LaunchLatant(e, lat);
            let l = L(e);
            e.w32(l + slot, uVar1);
            let c = e.r32(L(e) + slot);
            APIgameobj::SendMessageTo(e, c, NULL, 0x11f, 1, NULL);
            if let Some(j) = child_joint(iStack_2c) {
                let c = e.r32(L(e) + slot);
                APIgameobj::AddChild(e, SELF, j, c, 1);
            }
        }
        iStack_2c += 1;
    }
}

/// (local slot, group offset in the global data, joint, size) in CreateSprite order
const SPRITES: [(u32, u32, u32, f32); 21] = [
    (0x8c, 0x14, 0x15, 0.75),
    (0x90, 0x14, 0xb, 0.75),
    (0x94, 0x14, 0x10, 0.75),
    (0x98, 0x18, 1, 0.12),
    (0x9c, 0x18, 4, 0.12),
    (0xa0, 0x18, 7, 0.12),
    (0xa4, 0x10, 1, 0.05),
    (0xa8, 0x10, 4, 0.05),
    (0xac, 0x10, 7, 0.05),
    (0xb0, 0xc, 0x17, 0.075),
    (0xb4, 0xc, 0x18, 0.075),
    (0xb8, 0xc, 0xd, 0.075),
    (0xbc, 0xc, 0xe, 0.075),
    (0xc0, 0xc, 0x12, 0.075),
    (0xc4, 0xc, 0x13, 0.075),
    (0xc8, 0x10, 0x17, 0.025),
    (0xcc, 0x10, 0x18, 0.025),
    (0xd0, 0x10, 0xd, 0.025),
    (0xd4, 0x10, 0xe, 0.025),
    (0xd8, 0x10, 0x12, 0.025),
    (0xdc, 0x10, 0x13, 0.025),
];

/// 0x000132d8 the 21 glow sprites on the joints
pub fn HugeWisp2_baseFUNC_CreateSprite(e: &mut Engine) {
    for (slot, group, joint, size) in SPRITES {
        let gr = e.r32(GD(e) + group);
        let uVar1 = APIgraphics::SpriteGroupAddSprite(e, gr, SELF, joint, size);
        let l = L(e);
        e.w32(l + slot, uVar1);
    }
}

/// 0x00013838 remove the sprites (the original's order: groups +0xc, +0x10 (small), +0x14, +0x18,
/// +0x10 (tiny))
pub fn HugeWisp2_baseFUNC_DestroySprite(e: &mut Engine) {
    for i in [9usize, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 0, 1, 2, 3, 4, 5, 6, 7, 8] {
        let (slot, group, _, _) = SPRITES[i];
        let gr = e.r32(GD(e) + group);
        let s = e.r32(L(e) + slot);
        APIgraphics::SpriteGroupRemoveSprite(e, gr, s);
    }
}

/// 0x00013c00 three 3-point motion trails (joints 3/1/2, 6/4/5, 9/7/8; green in the middle)
pub fn HugeWisp2_baseFUNC_CreateMotionTrail(e: &mut Engine) {
    for (slot, joints) in [(0x80u32, [3u32, 1, 2]), (0x84, [6, 4, 5]), (0x88, [9, 7, 8])] {
        let uVar1 = APIgraphics::MTCreateSystem(e, 3, 0x19, 0);
        let l = L(e);
        e.w32(l + slot, uVar1);
        let mt = e.r32(L(e) + slot);
        APIgraphics::MTSetFrequency(e, mt, 0.01);
        for (i, (j, (r, g, b), life)) in [(joints[0], (0.0, 0.0, 0.0), 0.1), (joints[1], (0.1, 0.45, 0.1), 0.2), (joints[2], (0.0, 0.0, 0.0), 0.1)]
            .into_iter()
            .enumerate()
        {
            let mt = e.r32(L(e) + slot);
            APIgraphics::MTSetJoint(e, mt, i as u32, SELF, j);
            APIgraphics::MTSetRGB(e, mt, i as u32, r, g, b);
            APIgraphics::MTSetLifeSpan(e, mt, i as u32, life);
        }
    }
    for slot in [0x80u32, 0x84, 0x88] {
        let mt = e.r32(L(e) + slot);
        APIgraphics::MTStart(e, mt);
    }
}

/// 0x000142e0
pub fn HugeWisp2_baseFUNC_DestroyMotionTrail(e: &mut Engine) {
    for slot in [0x80u32, 0x84, 0x88] {
        if e.ri32(L(e) + slot) != -2 {
            let mt = e.r32(L(e) + slot);
            APIgraphics::MTStopDraw(e, mt);
            APIgraphics::MTDestroySystem(e, mt);
            let l = L(e);
            e.w32(l + slot, 0xffff_fffe);
        }
    }
}

/// 0x00014458
pub fn HugeWisp2_baseFUNC_SetClipping(e: &mut Engine) {
    let p = e.alloca(0x28);
    APIgameobj::GetClippingParams(e, SELF, p);
    for (o, v) in [
        (0x00u32, 0x3f80_0000u32),
        (0x04, 0x41c8_0000),
        (0x0c, 0),
        (0x08, 7),
        (0x10, 0x4248_0000),
        (0x18, 0x2a0),
        (0x14, 0x6a7),
        (0x1c, 0xbf80_0000),
        (0x24, 0x6a7),
        (0x20, 0x6a7),
    ] {
        e.w32(p + o, v);
    }
    APIgameobj::SetClippingParams(e, SELF, p, 0);
}

/// 0x00014518 module constructor
pub fn wispsourcenewConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    if instanceCount == 1 {
        let fns: [crate::logic::ExtFn; 8] = [
            HugeWisp2_baseST_FIRSTMOVE,
            HugeWisp2_baseST_BASICMOVE_SET,
            HugeWisp2_baseST_BASICMOVE_LOOP,
            HugeWisp2_baseCB_NOTIFY_COLLIDE_OBJ,
            HugeWisp2_baseCB_NOTIFY_ANIM_LF1,
            HugeWisp2_baseCB_NOTIFY_ANIM_LF2,
            HugeWisp2_baseCB_NOTIFY_ANIM_LF3,
            HugeWisp2_baseCB_NOTIFY_FROM_ANIM,
        ];
        for (i, f) in fns.into_iter().enumerate() {
            APIcons::SetExternalFunction(e, i as i32, f);
        }
        let tables: [(u32, fn(&mut Engine) -> Ptr); 9] = [
            (pAnim, |e| APIcons::GetApiAnim(e)),
            (pGameObj, |e| APIcons::GetApiGameObj(e)),
            (pExtra, |e| APIcons::GetApiExtra(e)),
            (pPhysic, |e| APIcons::GetApiPhysic(e)),
            (pSound, |e| APIcons::GetApiNewSound(e)),
            (pStdlib, |e| APIcons::GetApiStdlib(e)),
            (pParticle, |e| APIcons::GetApiParticle(e)),
            (pMath, |e| APIcons::GetApiMath(e)),
            (pLight, |e| APIcons::GetApiLight(e)),
        ];
        for (slot, get) in tables {
            let v = get(e);
            let a = G(e, slot);
            e.w32(a, v);
        }
        for (slot, name) in [(pAnimMngr, "AnimMngr"), (pMiscFunc, "MiscFunc")] {
            let n = e.cstr(name);
            let v = APIcons::GetApiCustom(e, n);
            let a = G(e, slot);
            e.w32(a, v);
        }
        let v = APIcons::GetApiGraphics(e);
        let a = G(e, pGraphic);
        e.w32(a, v);
        let slot = G(e, pGlobalDataHugeWisp2_base);
        APIcons::GetGlobalData(e, slot, 0x2c);
        for (off, id, size) in [(0u32, 0u32, 0x8f8u32), (4, 2, 0x158), (8, 1, 0x68)] {
            let uVar1 = APIgameobj::GetGameBuffer(e, id, size, 1);
            let gd = GD(e);
            e.w32(gd + off, uVar1);
        }
        HugeWisp2_baseFUNC_CreateSmokeType(e)?;
        for (off, (r, g, b)) in [(0xcu32, (0.0, 1.0, 0.0)), (0x10, (1.0, 1.0, 1.0)), (0x14, (0.1, 0.45, 0.1)), (0x18, (0.1, 0.45, 0.1))] {
            let uVar1 = APIgraphics::SpriteAddGroup(e);
            let gd = GD(e);
            e.w32(gd + off, uVar1);
            let gr = e.r32(GD(e) + off);
            let s = e.cstr("glow");
            APIgraphics::SpriteSetGroupTexture(e, gr, s);
            APIgraphics::SpriteSetGroupBlendMode(e, gr, 1);
            APIgraphics::SpriteSetGroupFlags(e, gr, 0x14);
            APIgraphics::SpriteSetGroupColor(e, gr, r, g, b, 0.1);
        }
    }
    APIcons::SetMessageFunction(e, HugeWisp2_base_MyMessageFunction);
    let slot = G(e, pLocalDataHugeWisp2_base);
    APIcons::GetLocalData(e, slot, 0xf8);
    LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 2, NULL, 0);
    let l = L(e);
    for off in [0x80u32, 0x84, 0x88] {
        e.w32(l + off, 0xffff_fffe);
    }
    HugeWisp2_baseFUNC_CreateSmokeParticle(e);
    HugeWisp2_baseFUNC_CreateSprite(e);
    APIcons::SetInitialState(e, 0);
    Ok(())
}

/// 0x00014c48 module destructor
pub fn wispsourcenewDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    HugeWisp2_baseFUNC_StopSmoke(e);
    HugeWisp2_baseFUNC_DestroySmokeParticle(e);
    HugeWisp2_baseFUNC_DestroyMotionTrail(e);
    HugeWisp2_baseFUNC_DestroySprite(e);
    if instanceCount == 0 {
        HugeWisp2_baseFUNC_DestroySmokeType(e);
        for off in [0xcu32, 0x10, 0x14, 0x18] {
            let gr = e.r32(GD(e) + off);
            APIgraphics::SpriteRemoveGroup(e, gr);
        }
    }
    Ok(())
}

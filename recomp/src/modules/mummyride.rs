//! `mummyride`: the Ep3 L03 ride mummy (a scripted prop riding with the heroes, not a capturable
//! monster: no Monster_Base, no book handling). It is parented to object 0x2a000009 (bone 0xe) on 0xae,
//! plays scripted anims 2 / 3 and launches the latents 0x2a80000d / 0x2a80000e on its anim key frames.
//! Port of `Game Logic Modules/Ep3_FrontierLand/Characters/MUMMY/LEVEL_03/mummyride/{mummyRide.c,
//! Particle_FUNC.c}` (7 + 3 functions). One build (pack EP3L03).
//!
//! Local data (0x30 bytes): +0 the "Yeux" particle type, +4 / +8 the two eye-glow emitters,
//! +0xc / +0x18 their offsets, +0x24 the launched latent, +0x2c the current anim.
#![allow(non_snake_case, non_upper_case_globals, unused_variables)]

use crate::engine::*;
use crate::logic::{LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "mummyride",
    // globals 0x10000..0x10030; gpoLanguagesNamesAndISOCodes (0x10030, "Abkhazian") is never read
    data_size: 0x30,
    ctor: mummyrideConstructor,
    dtor: mummyrideDestructor,
    register: None,
    init: &[],
    build_init: None,
};

// ---- module globals (_globals.h) ----
pub const pExtra: u32 = 0x10000;
pub const pGameObj: u32 = 0x10004;
pub const pPhysic: u32 = 0x10008;
pub const pAnimMngr: u32 = 0x1000c;
pub const pDebug: u32 = 0x10010;
pub const pSound: u32 = 0x10014;
pub const pFunction: u32 = 0x10018;
pub const pParticle: u32 = 0x1001c;
pub const pMiscFunc: u32 = 0x10020;
pub const pAnim: u32 = 0x10024;
pub const pLocalDataMummyRide: u32 = 0x10028;
pub const pGlobalDataMummyRide: u32 = 0x1002c;

#[inline]
fn G(e: &Engine, a: u32) -> Ptr {
    e.rt.cur_data() + (a - 0x10000)
}
#[inline]
fn L(e: &Engine) -> Ptr {
    e.r32(G(e, pLocalDataMummyRide))
}

// ---- mummyRide.c ----

/// 0x00011428 external state 1 (initial state): -> state 0
pub fn MummyRideST_FIRSTMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    APIgameobj::SetNextState(e, SELF, 0);
    Ok(0)
}

/// 0x00011480 external state 0
pub fn MummyRideST_BASICMOVE_LOOP(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

/// 0x000114b0 external 2
pub fn MummyRideCB_ANIM_DONE(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

/// 0x000114f0 external 3: anim key frame notify {key, kind, value}; for key frames 2 / 3 of kind 1:
/// in anim 2, value 0 launches latent 0x2a80000e (told 0xa6 + our id) and value 1 tells it 0x110;
/// in any other anim the current latent is told 0x110.
pub fn MummyRideCB_ANIM(e: &mut Engine, param_1: u32) -> LResult<u64> {
    let (k0, k1, k2) = (e.r32(param_1), e.r32(param_1 + 4), e.r32(param_1 + 8));
    if k1 != 0 && k1 == 1 && k0 < 4 && 1 < k0 {
        if e.r32(L(e) + 0x2c) == 2 {
            if k2 == 0 {
                let uVar3 = APIgameobj::LaunchLatant(e, 0x2a80_000e);
                e.w32(L(e) + 0x24, uVar3);
                let iVar2 = L(e);
                let uVar4 = APIgameobj::GetID(e, SELF, NULL);
                let to = e.r32(iVar2 + 0x24);
                APIgameobj::SendMessageTo(e, to, NULL, 0xa6, uVar4, NULL);
            }
            if k2 == 1 {
                let to = e.r32(L(e) + 0x24);
                APIgameobj::SendMessageTo(e, to, NULL, 0x110, 0, NULL);
            }
        } else {
            let to = e.r32(L(e) + 0x24);
            APIgameobj::SendMessageTo(e, to, NULL, 0x110, 0, NULL);
        }
    }
    Ok(0)
}

/// rotation about +y by `a`
fn rot_y(e: &mut Engine, a: f32) {
    let v = e.alloca(0x10);
    e.wf(v, 0.0);
    e.wf(v + 4, 1.0);
    e.wf(v + 8, 0.0);
    APIgameobj::SetRotation(e, SELF, v, a);
}

/// 0x000116b8 0xae: ride on 0x2a000009 (bone 0xe), anim 1 through AnimMngr (its result -> L+0x2c);
/// 0xa6: anim 2; 0xaa: launch latent 0x2a80000d (told 0xaa + our id), anim 3; 0x160 / 0x161 / 0x188:
/// face -90 / -180 / -30 degrees about y.
pub fn MummyRideFUNC_MessageHandler(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    match param_1 {
        0xae => {
            APIgameobj::AddChild(e, 0x2a00_0009, 0xe, SELF, 1);
            let iVar2 = LIBAnimMngr::PlayAnim(e, 0.0, 1, 2, 0);
            e.w32(L(e) + 0x2c, iVar2 as u32);
        }
        0xa6 => {
            e.w32(L(e) + 0x2c, 2);
            APIanim::Play(e, SELF, 2, 5, 0.23);
        }
        0xaa => {
            let uVar3 = APIgameobj::LaunchLatant(e, 0x2a80_000d);
            e.w32(L(e) + 0x24, uVar3);
            let iVar2 = L(e);
            let uVar4 = APIgameobj::GetID(e, SELF, NULL);
            let to = e.r32(iVar2 + 0x24);
            APIgameobj::SendMessageTo(e, to, NULL, 0xaa, uVar4, NULL);
            e.w32(L(e) + 0x2c, 3);
            APIanim::Play(e, SELF, 3, 5, 0.23);
        }
        0x161 => rot_y(e, -3.1415927),
        0x160 => rot_y(e, -1.5707964),
        0x188 => rot_y(e, -0.5235988),
        _ => {}
    }
    Ok(0)
}

/// 0x000119c0 physics (flags 2), local data 0x30, every notify kind 0xb -> external 3, eye glows,
/// initial state 1.
/// NOTE (PS2 behaviour kept): pSound is first the new-sound API, then overwritten by "SndMngr".
pub fn mummyrideConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    if instanceCount == 1 {
        APIcons::SetExternalFunction(e, 1, MummyRideST_FIRSTMOVE);
        APIcons::SetExternalFunction(e, 0, MummyRideST_BASICMOVE_LOOP);
        APIcons::SetExternalFunction(e, 2, MummyRideCB_ANIM_DONE);
        APIcons::SetExternalFunction(e, 3, MummyRideCB_ANIM);
        fn custom(e: &mut Engine, n: &str) -> Ptr {
            let n = e.cstr(n);
            APIcons::GetApiCustom(e, n)
        }
        let tables: [(u32, fn(&mut Engine) -> Ptr); 11] = [
            (pExtra, |e| APIcons::GetApiExtra(e)),
            (pGameObj, |e| APIcons::GetApiGameObj(e)),
            (pPhysic, |e| APIcons::GetApiPhysic(e)),
            (pSound, |e| APIcons::GetApiNewSound(e)),
            (pAnimMngr, |e| custom(e, "AnimMngr")),
            (pDebug, |e| custom(e, "debug")),
            (pSound, |e| custom(e, "SndMngr")),
            (pFunction, |e| custom(e, "ApiCFunctions")),
            (pMiscFunc, |e| custom(e, "MiscFunc")),
            (pParticle, |e| APIcons::GetApiParticle(e)),
            (pAnim, |e| APIcons::GetApiAnim(e)),
        ];
        for (slot, get) in tables {
            let v = get(e);
            let a = G(e, slot);
            e.w32(a, v);
        }
        let slot = G(e, pGlobalDataMummyRide);
        APIcons::GetGlobalData(e, slot, 4);
        let uVar1 = APIgameobj::GetGameBuffer(e, 0, 0x8f8, 1);
        let gd = e.r32(G(e, pGlobalDataMummyRide));
        e.w32(gd, uVar1);
    }
    APIphysic::CreatePhysic(e, SELF, 0);
    APIphysic::SetFlags(e, SELF, 2);
    let slot = G(e, pLocalDataMummyRide);
    APIcons::GetLocalData(e, slot, 0x30);
    APIcons::SetMessageFunction(e, MummyRideFUNC_MessageHandler);
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 0xb, 3);
    Create_Particles(e)?;
    APIcons::SetInitialState(e, 1);
    Ok(())
}

/// 0x00011ca8
pub fn mummyrideDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    Destroy_Particles(e)
}

// ---- Particle_FUNC.c ----

/// 0x00010d30 the "Yeux" eye glow (`param_1` = the particle API, unused by the port's typed calls)
pub fn SetParticle_04(e: &mut Engine, param_1: u32, param_2: u32) -> LResult<()> {
    let t = param_2 as i32;
    let f = f32::from_bits;
    APIparticle::TypeSetInt(e, t, 8, 1);
    APIparticle::TypeSetInt(e, t, 1, 2);
    APIparticle::TypeSetInt(e, t, 4, 0);
    APIparticle::TypeSetInt(e, t, 0x10, 0);
    APIparticle::TypeSetInt(e, t, 0x20, 0);
    APIparticle::TypeSetInt(e, t, 0x40, 0);
    APIparticle::TypeSetInt(e, t, 0x80, 0);
    APIparticle::TypeSetInt(e, t, 0x100, 0);
    APIparticle::TypeSetFloat(e, t, 1, 0.0);
    APIparticle::TypeSetFloat(e, t, 4, f(0x3dcc_cccd));
    APIparticle::TypeSetFloat(e, t, 2, f(0x3de1_47ae));
    APIparticle::TypeSetFloat(e, t, 8, 0.0);
    APIparticle::TypeSetVec(e, t, 1, 0.0, 0.0, 0.0);
    APIparticle::TypeSetVec(e, t, 2, 0.0, 0.0, 0.0);
    APIparticle::TypeSetInter(e, t, 1, 0.0, 0.0, 0.0, 0.0);
    APIparticle::TypeSetInter(e, t, 2, 1.0, 1.0, 1.0, 1.0);
    APIparticle::TypeSetInter(e, t, 4, 0.0, 0.0, 0.0, 0.0);
    APIparticle::TypeSetInter(e, t, 8, 1.0, 1.0, 1.0, 0.0);
    APIparticle::TypeSetInter(e, t, 0x10, f(0x3d99_999a), f(0x3d99_999a), f(0x3d99_999a), 0.5);
    APIparticle::TypeSetInter(e, t, 0x20, 0.0, 0.0, 0.0, 0.0);
    Ok(())
}

/// one eye glow: emitter of the type into L+`slot`, joint 0x6fffffff, child of `bone`, offset L+`ofs`,
/// flags 0x80 on / 2 off, start
fn eye(e: &mut Engine, l: Ptr, slot: u32, bone: u32, ofs: u32) -> u32 {
    let t = e.r32(l) as i32;
    let em = APIparticle::EmitterCreate(e, t);
    e.w32(l + slot, em);
    APIparticle::EmitterSetJoint(e, em as i32, SELF, 0x6fff_ffff);
    APIgameobj::AddChild(e, SELF, bone, em, 1);
    APIgameobj::SetTranslation(e, em, l + ofs);
    let p = e.r32(G(e, pParticle)) as i32;
    LIBMiscFunc::MiscFunc_EmitterUpdateFlags(e, p, em, 0x80, 1);
    LIBMiscFunc::MiscFunc_EmitterUpdateFlags(e, p, em, 2, 0);
    APIparticle::EmitterStart(e, em as i32, 1)
}

/// 0x00011030 the two eye glows on bones 7 / 8 at x +-0.04. Returns the last EmitterStart result.
pub fn Create_Particles(e: &mut Engine) -> LResult<u32> {
    let l = L(e);
    for (o, v) in [(0xc, 0), (0x10, 0), (0x14, 0x3d23_d70a), (0x18, 0), (0x1c, 0), (0x20, 0xbd23_d70a)] {
        e.w32(l + o, v);
    }
    let t = APIparticle::TypeCreate(e);
    e.w32(l, t);
    let s = e.cstr("glow");
    APIparticle::TypeSetTexture(e, t as i32, s);
    let s = e.cstr("Yeux");
    APIparticle::TypeSetName(e, t as i32, s);
    let p = e.r32(G(e, pParticle));
    SetParticle_04(e, p, t)?;
    eye(e, l, 4, 7, 0xc);
    Ok(eye(e, l, 8, 8, 0x18))
}

/// 0x00011330
/// NOTE (PS2 behaviour kept): the emitter loop indexes from L+0, so it destroys the type slot and the
/// first emitter (L+0, L+4) as emitters; the second emitter (L+8) is never destroyed.
pub fn Destroy_Particles(e: &mut Engine) -> LResult<()> {
    let l = L(e);
    for i in 0..1 {
        let t = e.r32(l + i * 4) as i32;
        APIparticle::TypeDestroy(e, t);
    }
    for i in 0..2 {
        let em = e.r32(l + i * 4) as i32;
        APIparticle::EmitterDestroy(e, em);
    }
    Ok(())
}

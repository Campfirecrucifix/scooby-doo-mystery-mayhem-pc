//! `swampcreature`: EP2 swamp creature prop that pops out of the water when the leader steps on its
//! trigger prim 0x3b8 (water splash / leak particles on 7 joints). Port of
//! `Ep2_MovieStudio/Objects/ANIMATED_PROPS/swampcreature` (1 build).
#![allow(non_snake_case, non_upper_case_globals, unused_variables)]

use super::modkit::*;
use super::swampcreature_particle::*;
use crate::engine::*;
use crate::logic::{LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "swampcreature",
    data_size: 0x68,
    ctor: swampcreatureConstructor,
    dtor: swampcreatureDestructor,
    register: None,
    init: &[(iSwampCreatureJoints, &[4, 7, 8, 9, 0xc, 0xd, 0xe]), (moSwampCreatureDataMap, &[0x2a00012f, 0x2a000130])],
    build_init: None,
};

pub const iSwampCreatureJoints: u32 = 0x10000;
pub const moSwampCreatureDataMap: u32 = 0x10020;
pub const pMath: u32 = 0x10028;
pub const pParticle: u32 = 0x1002c;
pub const pAnimMngr: u32 = 0x10030;
pub const pText: u32 = 0x10034;
pub const pDebug: u32 = 0x10038;
pub const pFunction: u32 = 0x1003c;
pub const pMiscFunc: u32 = 0x10040;
pub const pExtra: u32 = 0x10044;
pub const pGameObj: u32 = 0x10048;
pub const pPhysic: u32 = 0x1004c;
pub const pAnim: u32 = 0x10050;
pub const pInput: u32 = 0x10054;
pub const pGame: u32 = 0x10058;
pub const pLocalDataSwampCreature: u32 = 0x1005c;
pub const pGlobalDataSwampCreature: u32 = 0x10060;

/// local word i (100 bytes): 0 timer, 1 touching, 2 triggered, 3..10 splash, 10..17 leak, 17..24 leak low,
/// 0x18 leak->leaklow switch time (f32, -1 = none)
fn lr(e: &Engine, i: u32) -> u32 {
    e.r32(pw(e, pLocalDataSwampCreature, i * 4))
}
fn lw(e: &mut Engine, i: u32, v: u32) {
    let a = pw(e, pLocalDataSwampCreature, i * 4);
    e.w32(a, v);
}
fn gr(e: &Engine, i: u32) -> u32 {
    e.r32(pw(e, pGlobalDataSwampCreature, i * 4))
}

/// state 0
pub fn fstate_FIRSTMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    APIgameobj::GetID(e, SELF, NULL);
    lw(e, 1, 0);
    lw(e, 2, 0);
    let t = APIextra::TimerCreate(e);
    lw(e, 0, t);
    lw(e, 0x18, 0xbf80_0000);
    APIextra::TimerReset(e, t);
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 3, 3);
    APIanim::Play(e, SELF, 1, 1, 0.0);
    APIgameobj::SetNextState(e, SELF, 1);
    let pp = e.r32(G(e, pParticle)) as i32;
    for i in 0..7 {
        let j = e.r32(G(e, iSwampCreatureJoints) + i * 4);
        for (k, base) in [(0, 3), (1, 10), (2, 0x11)] {
            let ty = gr(e, k) as i32;
            let em = APIparticle::EmitterCreate(e, ty);
            lw(e, i + base, em);
            LIBMiscFunc::MiscFunc_EmitterUpdateFlags(e, pp, em, 0xc0, 1);
            APIparticle::EmitterSetJoint(e, em as i32, SELF, j);
        }
    }
    SwampCreateFUNC_SetClipping(e);
    Ok(0)
}

/// state 1
pub fn fstate_BASICMOVE_SET(e: &mut Engine, _: u32) -> LResult<u64> {
    let sw = f32::from_bits(lr(e, 0x18));
    if 0.0 < sw && sw < APIextra::TimerGetTime(e, 0x9000_0000) {
        lw(e, 0x18, 0xbf80_0000);
        for i in 0..7 {
            let em = lr(e, i + 10) as i32;
            APIparticle::EmitterStop(e, em);
        }
        for i in 0..7 {
            let em = lr(e, i + 0x11) as i32;
            APIparticle::EmitterStart(e, em, 1);
        }
    }
    let t = lr(e, 0);
    if 0.5 < APIextra::TimerGetTime(e, t) && lr(e, 1) != 0 {
        lw(e, 1, 0);
    }
    Ok(0)
}

/// state 2
pub fn fstate_BASICMOVE_LOOP(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

/// callback 3 (collision notify)
pub fn fcback_NOTIFY_COLLIDE_OBJ(e: &mut Engine, p: u32) -> LResult<u64> {
    let gb0 = gr(e, 3);
    if e.r32(gb0 + 8) == e.r32(p + 8) && e.r32(gb0 + 0x14) & 0x4900_097c == 0 {
        if e.r32(p + 4) == 0x3b8 {
            let t = lr(e, 0);
            APIextra::TimerReset(e, t);
            if lr(e, 1) == 0 {
                lw(e, 1, 1);
                if lr(e, 2) == 0 {
                    lw(e, 2, 1);
                    LIBAnimMngr::PlayAnim(e, 0.0, 2, 1, 4);
                    let pad = e.r32(gb0 + 0x4c) as i32;
                    APIinput::InputSetVibration(e, pad, 0, 0.66, 0.0, 0.33, 0.0);
                    DisableOtherCreatures(e);
                    for i in 0..7 {
                        let a = lr(e, i + 3) as i32;
                        APIparticle::EmitterStart(e, a, 0);
                        let b = lr(e, i + 10) as i32;
                        APIparticle::EmitterStart(e, b, 1);
                    }
                    let id = APIgameobj::GetID(e, SELF, NULL);
                    APIgameobj::SendMessageTo(e, 0x2a000031, NULL, 0x16e, id, NULL);
                    let now = APIextra::TimerGetTime(e, 0x9000_0000);
                    lw(e, 0x18, (now + 1.0).to_bits());
                    APIextra::TimerReset(e, t);
                }
            }
        } else {
            APIphysic::SetFlags(e, SELF, 1);
        }
    }
    Ok(0)
}

/// callback 4. Original writes word 8 (= byte 0x20, a splash emitter slot!) as `pLocal + 8` on a uint* —
/// i.e. local word 8; kept as-is.
pub fn fcback_NOTIFY_END_ATTACK(e: &mut Engine, _: u32) -> LResult<u64> {
    lw(e, 8, 0);
    APIphysic::ResetFlags(e, SELF, 1);
    Ok(1)
}

pub fn MyMessageFunction(e: &mut Engine, m: u32, _: u32, _: Ptr) -> LResult<u64> {
    if m == 0xda {
        APIgameobj::UnregisterNotify(e, SELF, 0xffff_ffff, 3);
    }
    Ok(0)
}

pub fn DisableOtherCreatures(e: &mut Engine) {
    let id = APIgameobj::GetID(e, SELF, NULL);
    for i in 0..2 {
        let o = e.r32(G(e, moSwampCreatureDataMap) + i * 4);
        if o != id {
            APIgameobj::SendMessageTo(e, o, NULL, 0xda, 0, NULL);
        }
    }
}

pub fn SwampCreateFUNC_SetClipping(e: &mut Engine) {
    let p = e.alloca(0x28);
    APIgameobj::GetClippingParams(e, SELF, p);
    for (o, v) in [
        (0, 0x3dcccccd),
        (4, 0x41c80000),
        (0x10, 0x42480000),
        (0x1c, 0xbf800000),
        (0xc, 0),
        (8, 0x6c7),
        (0x18, 0x4c0),
        (0x14, 0x6a7),
        (0x24, 0x6a7),
        (0x20, 0x6a7),
    ] {
        e.w32(p + o, v);
    }
    APIgameobj::SetClippingParams(e, SELF, p, 0);
}

pub fn Create_Physic(e: &mut Engine) {
    APIphysic::CreatePhysic(e, SELF, 0);
    APIphysic::SetFlags(e, SELF, 2);
    APIphysic::SetWeight(e, SELF, 1.0);
    APIphysic::SetPriority(e, SELF, 100);
}

pub fn swampcreatureConstructor(e: &mut Engine, api: Ptr, n: i32) -> LResult<()> {
    if n == 1 {
        ext(e, &[
            (0, fstate_FIRSTMOVE),
            (1, fstate_BASICMOVE_SET),
            (2, fstate_BASICMOVE_LOOP),
            (3, fcback_NOTIFY_COLLIDE_OBJ),
            (4, fcback_NOTIFY_END_ATTACK),
        ]);
        let v = APIcons::GetApiAnim(e); gset(e, pAnim, v);
        let v = APIcons::GetApiGameObj(e); gset(e, pGameObj, v);
        let v = APIcons::GetApiExtra(e); gset(e, pExtra, v);
        let v = APIcons::GetApiPhysic(e); gset(e, pPhysic, v);
        let v = APIcons::GetApiInput(e); gset(e, pInput, v);
        let v = APIcons::GetApiGame(e); gset(e, pGame, v);
        custom(e, pDebug, "debug");
        custom(e, pAnimMngr, "AnimMngr");
        custom(e, pText, "ApiText");
        let v = APIcons::GetApiMath(e); gset(e, pMath, v);
        let v = APIcons::GetApiParticle(e); gset(e, pParticle, v);
        custom(e, pFunction, "ApiCFunctions");
        custom(e, pMiscFunc, "MiscFunc");
        let s = G(e, pGlobalDataSwampCreature);
        APIcons::GetGlobalData(e, s, 0x14);
        let g = pw(e, pGlobalDataSwampCreature, 0);
        let b = APIgameobj::GetGameBuffer(e, 0, 0x8f8, 1);
        e.w32(g + 0xc, b);
        let b = APIgameobj::GetGameBuffer(e, 1, 0x68, 1);
        e.w32(g + 0x10, b);
        let set: [(&str, &str, fn(&mut Engine, i32)); 3] = [
            ("swampcreat_watersplash", "WaterFall", SetParticleWaterSplash),
            ("swampcreat_waterleak", "water_drop", SetParticleWaterLeak),
            ("swampcreat_waterleaklow", "water_drop", SetParticleWaterLeakLow),
        ];
        for (i, (name, tex, f)) in set.into_iter().enumerate() {
            let t = APIparticle::TypeCreate(e);
            e.w32(g + i as u32 * 4, t);
            let s = e.cstr(name);
            APIparticle::TypeSetName(e, t as i32, s);
            let s = e.cstr(tex);
            APIparticle::TypeSetTexture(e, t as i32, s);
            f(e, t as i32);
        }
    }
    APIcons::SetMessageFunction(e, MyMessageFunction);
    let s = G(e, pLocalDataSwampCreature);
    APIcons::GetLocalData(e, s, 100);
    Create_Physic(e);
    LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 2, NULL, 0);
    APIcons::SetInitialState(e, 0);
    Ok(())
}

pub fn swampcreatureDestructor(e: &mut Engine, n: i32) -> LResult<()> {
    for i in 0..7 {
        for b in [3, 10, 0x11] {
            let em = lr(e, i + b) as i32;
            APIparticle::EmitterDestroy(e, em);
        }
    }
    let t = lr(e, 0);
    APIextra::TimerDestroy(e, t);
    if n == 0 {
        for i in 0..3 {
            let t = gr(e, i) as i32;
            APIparticle::TypeDestroy(e, t);
        }
    }
    Ok(())
}

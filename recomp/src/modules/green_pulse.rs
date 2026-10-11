//! `green_pulse`: EP6 force-field pulse FX. Plays anim 1 + a sound, then kills itself after 1 s
//! (instance timer, ext fn 2). Builds differ only in the sound id.
//! Port of `Game Logic Modules/Ep6_ShermanTech/Objects/FORCE_FIELD/green_pulse__*/Green_Pulse.c` (7 functions).
#![allow(non_snake_case, non_upper_case_globals, unused_variables)]

use crate::engine::*;
use crate::logic::{LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "green_pulse",
    // globals 0x10000..0x10030 (+ unused gpoLanguagesNamesAndISOCodes)
    data_size: 0x30,
    ctor: green_pulseConstructor,
    dtor: green_pulseDestructor,
    register: None,
    init: &[],
    build_init: None,
};

const KEYS: [&str; 4] = ["1fc9eabf", "e2a122be", "049ea89c", "7f4e105e"];
/// per-build sound played in FIRSTMOVE
const SOUND: [u32; 4] = [0x4000_004d, 0x4000_0019, 0x4000_0012, 0x4000_0035];

pub fn build_index(key: &str) -> Option<usize> {
    KEYS.iter().position(|k| *k == key)
}

pub const pExtra: u32 = 0x10000;
pub const pGameObj: u32 = 0x10004;
pub const pPhysic: u32 = 0x10008;
pub const pAnim: u32 = 0x1000c;
pub const pParticle: u32 = 0x10010;
pub const pMath: u32 = 0x10014;
pub const pSound: u32 = 0x10018;
pub const pAnimMngr: u32 = 0x1001c;
pub const pFunctions: u32 = 0x10020;
pub const pMiscFunc: u32 = 0x10024;
pub const pLocalDataGreen_Pulse: u32 = 0x10028;
pub const pGlobalDataGreen_Pulse: u32 = 0x1002c;

#[inline]
fn G(e: &Engine, a: u32) -> Ptr {
    e.rt.cur_data() + (a - 0x10000)
}
/// *pLocalDataGreen_Pulse: the instance timer
#[inline]
fn timer(e: &Engine) -> u32 {
    e.r32(e.r32(G(e, pLocalDataGreen_Pulse)))
}

/// Green_PulseST_FIRSTMOVE @0x00010d10
pub fn Green_PulseST_FIRSTMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    APIanim::Play(e, SELF, 1, 2, 0.0);
    APInewSound::Play(e, SELF, SOUND[e.rt.cur_build()], 0);
    let t = timer(e);
    APIextra::TimerReset(e, t);
    APIextra::TimerSetFrequency(e, t, 1.0);
    APIextra::TimerRegisterNotify(e, SELF, t, 2);
    Green_PulseFUNC_SetClipping(e);
    APIgameobj::SetNextState(e, SELF, 1);
    Ok(0)
}

/// Green_PulseST_BASICMOVE_LOOP @0x00010e38
pub fn Green_PulseST_BASICMOVE_LOOP(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

/// Green_PulseCB_TIMER @0x00010e68
pub fn Green_PulseCB_TIMER(e: &mut Engine, t: u32) -> LResult<u64> {
    if t == timer(e) {
        APIgameobj::KillObject(e, SELF);
    }
    Ok(0)
}

/// Green_PulseFUNC_MessageHandler @0x00010ed8
pub fn Green_PulseFUNC_MessageHandler(e: &mut Engine, m: u32, _s: u32, _d: Ptr) -> LResult<u64> {
    Ok(0)
}

/// Green_PulseFUNC_SetClipping @0x00010f10 (stack EFClippingParams)
pub fn Green_PulseFUNC_SetClipping(e: &mut Engine) {
    let c = e.alloca(0x40);
    APIgameobj::GetClippingParams(e, SELF, c);
    for (o, v) in [(4, 0x41a0_0000), (8, 0), (0xc, 0), (0x10, 0xbf80_0000), (0x14, 0x87), (0x18, 0)] {
        e.w32(c + o, v);
    }
    APIgameobj::SetClippingParams(e, SELF, c, 0);
}

/// green_pulseConstructor @0x00010fa0
pub fn green_pulseConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    if instanceCount == 1 {
        APIcons::SetExternalFunction(e, 0, Green_PulseST_FIRSTMOVE);
        APIcons::SetExternalFunction(e, 1, Green_PulseST_BASICMOVE_LOOP);
        APIcons::SetExternalFunction(e, 2, Green_PulseCB_TIMER);
        for (a, v) in [
            (pAnim, APIcons::GetApiAnim(e)),
            (pGameObj, APIcons::GetApiGameObj(e)),
            (pExtra, APIcons::GetApiExtra(e)),
            (pPhysic, APIcons::GetApiPhysic(e)),
            (pSound, APIcons::GetApiNewSound(e)),
            (pParticle, APIcons::GetApiParticle(e)),
            (pMath, APIcons::GetApiMath(e)),
        ] {
            let a = G(e, a);
            e.w32(a, v);
        }
        for (a, n) in [(pAnimMngr, "AnimMngr"), (pFunctions, "ApiCFunctions"), (pMiscFunc, "MiscFunc")] {
            let s = e.cstr(n);
            let v = APIcons::GetApiCustom(e, s);
            let a = G(e, a);
            e.w32(a, v);
        }
        let slot = G(e, pGlobalDataGreen_Pulse);
        APIcons::GetGlobalData(e, slot, 4);
        let b = APIgameobj::GetGameBuffer(e, 0, 0x8f8, 1);
        let gd = e.r32(slot);
        e.w32(gd, b);
    }
    APIcons::SetMessageFunction(e, Green_PulseFUNC_MessageHandler);
    let slot = G(e, pLocalDataGreen_Pulse);
    APIcons::GetLocalData(e, slot, 4);
    let t = APIextra::TimerCreate(e);
    let l = e.r32(slot);
    e.w32(l, t);
    APIcons::SetInitialState(e, 0);
    Ok(())
}

/// green_pulseDestructor @0x000111f8
pub fn green_pulseDestructor(e: &mut Engine, _: i32) -> LResult<()> {
    let t = timer(e);
    APIextra::TimerDestroy(e, t);
    Ok(())
}

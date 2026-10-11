//! `fearexplosion`: the fear-ball bang. Message 0x174 shows it, plays anim 1 and starts the 14
//! "Fearball_Bang" emitters (joints 2..15) every frame for 0.7667 s; the anim's last frame hides it.
//! Local +4 particle type, +8..+0x40 emitters, +0x40 end time.
//! Port of `Game Logic Modules/Globals/Objects/FEAREXPLOSION/fearexplosion/{FearExplosion,Fearball_Bang}.c`.
#![allow(non_snake_case, non_upper_case_globals, unused_variables)]

use crate::engine::*;
use crate::logic::{LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "fearexplosion",
    data_size: 0x28,
    ctor: fearexplosionConstructor,
    dtor: fearexplosionDestructor,
    register: None,
    init: &[],
    build_init: None,
};

pub const pExtra: u32 = 0x10000;
pub const pGameObj: u32 = 0x10004;
pub const pPhysic: u32 = 0x10008;
pub const pAnim: u32 = 0x1000c;
pub const pParticle: u32 = 0x10010;
pub const pDebug: u32 = 0x10014;
pub const pAnimMngr: u32 = 0x10018;
pub const pMiscFunc: u32 = 0x1001c;
pub const pLocalDataFearExplosion: u32 = 0x10020;
pub const pGlobalDataFearExplosion: u32 = 0x10024;

#[inline]
fn G(e: &Engine, a: u32) -> Ptr {
    e.rt.cur_data() + (a - 0x10000)
}
fn L(e: &Engine) -> Ptr {
    e.r32(G(e, pLocalDataFearExplosion))
}
fn emitter(e: &Engine, i: u32) -> i32 {
    e.r32(L(e) + 8 + i * 4) as i32
}

pub fn FearExplosionST_FIRSTMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    APIgameobj::SetNextState(e, SELF, 1);
    APIgameobj::EnableObject(e, SELF, 0);
    Ok(0)
}
pub fn FearExplosionST_BASICMOVE_LOOP(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}
pub fn FearExplosionST_EXPLODE(e: &mut Engine, _: u32) -> LResult<u64> {
    let end = e.rf(L(e) + 0x40);
    if end < APIextra::TimerGetTime(e, 0x9000_0000) {
        APIgameobj::SetNextState(e, SELF, 1);
    } else {
        for i in 0..14 {
            let em = emitter(e, i);
            APIparticle::EmitterStart(e, em, 0);
        }
    }
    Ok(0)
}
pub fn FearExplosionCB_NOTIFY_COLLIDE_OBJ(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}
pub fn FearExplosionCB_ANIM_LF(e: &mut Engine, _: u32) -> LResult<u64> {
    APIgameobj::EnableObject(e, SELF, 0);
    Ok(1)
}
pub fn FearExplosionFUNC_MessageHandler(e: &mut Engine, m: u32, _: u32, _: Ptr) -> LResult<u64> {
    if m == 0x174 {
        APIgameobj::EnableObject(e, SELF, 1);
        APIanim::Play(e, SELF, 1, 1, 0.0);
        APIgameobj::RegisterNotify(e, SELF, 1, 1, 4);
        let t = APIextra::TimerGetTime(e, 0x9000_0000);
        let l = L(e);
        e.wf(l + 0x40, t + 0.766_666_65);
        APIgameobj::SetNextState(e, SELF, 2);
    }
    Ok(0)
}
/// Fearball_Bang.c SetParticle_Fearball_Bang
fn SetParticle_Fearball_Bang(e: &mut Engine, t: i32) {
    let h = f32::from_bits;
    for (k, v) in [(8, 1), (1, 2), (4, 0x164), (0x10, 0), (0x20, 0), (0x40, 0), (0x80, 0), (0x100, 0)] {
        APIparticle::TypeSetInt(e, t, k, v);
    }
    for (k, v) in [(1, 0), (4, 0x3a83_126f), (2, 0x3e80_0000), (8, 0x4040_0000)] {
        APIparticle::TypeSetFloat(e, t, k, h(v));
    }
    APIparticle::TypeSetVec(e, t, 1, 0.0, 0.0, 0.0);
    APIparticle::TypeSetVec(e, t, 2, 0.0, 0.0, 0.0);
    for (k, a, b, c, d) in [
        (1, 0x3f80_0000, 0x3f40_0000, 0x3c23_d70a, 0x3e00_0000),
        (2, 0x3f80_0000, 0x3f40_0000, 0x3c23_d70a, 0x3e00_0000),
        (4, 0x3f80_0000, 0x3f40_0000, 0x3c23_d70a, 0x3e00_0000),
        (8, 0x3f33_3333, 0x3ecc_cccd, 0, 0x3ecc_cccd),
        (0x10, 0x3e80_0000, 0x3e00_0000, 0x3c23_d70a, 0x3e00_0000),
        (0x20, 0, 0, 0, 0x3f00_0000),
    ] {
        APIparticle::TypeSetInter(e, t, k, h(a), h(b), h(c), h(d));
    }
}
fn FearExplosionFUNC_Create_Particules(e: &mut Engine) {
    let t = APIparticle::TypeCreate(e);
    let l = L(e);
    e.w32(l + 4, t);
    let t = t as i32;
    let s = e.cstr("broue");
    APIparticle::TypeSetTexture(e, t, s);
    let s = e.cstr("Fearball_Bang");
    APIparticle::TypeSetName(e, t, s);
    SetParticle_Fearball_Bang(e, t);
    for i in 0..14u32 {
        let em = APIparticle::EmitterCreate(e, t);
        e.w32(l + 8 + i * 4, em);
        APIparticle::EmitterSetJoint(e, em as i32, SELF, i + 2);
    }
}

pub fn fearexplosionConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    if instanceCount == 1 {
        APIcons::SetExternalFunction(e, 0, FearExplosionST_FIRSTMOVE);
        APIcons::SetExternalFunction(e, 1, FearExplosionST_BASICMOVE_LOOP);
        APIcons::SetExternalFunction(e, 2, FearExplosionST_EXPLODE);
        APIcons::SetExternalFunction(e, 3, FearExplosionCB_NOTIFY_COLLIDE_OBJ);
        APIcons::SetExternalFunction(e, 4, FearExplosionCB_ANIM_LF);
        for (a, v) in [
            (pAnim, APIcons::GetApiAnim(e)),
            (pGameObj, APIcons::GetApiGameObj(e)),
            (pExtra, APIcons::GetApiExtra(e)),
            (pPhysic, APIcons::GetApiPhysic(e)),
            (pParticle, APIcons::GetApiParticle(e)),
        ] {
            let a = G(e, a);
            e.w32(a, v);
        }
        for (a, n) in [(pDebug, "debug"), (pAnimMngr, "AnimMngr"), (pMiscFunc, "MiscFunc")] {
            let s = e.cstr(n);
            let v = APIcons::GetApiCustom(e, s);
            let a = G(e, a);
            e.w32(a, v);
        }
        let slot = G(e, pGlobalDataFearExplosion);
        APIcons::GetGlobalData(e, slot, 4);
        let b = APIgameobj::GetGameBuffer(e, 0, 0x8f8, 1);
        let gd = e.r32(slot);
        e.w32(gd, b);
    }
    APIcons::SetMessageFunction(e, FearExplosionFUNC_MessageHandler);
    let slot = G(e, pLocalDataFearExplosion);
    APIcons::GetLocalData(e, slot, 0x44);
    FearExplosionFUNC_Create_Particules(e);
    LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 2, NULL, 1);
    APIcons::SetInitialState(e, 0);
    Ok(())
}
pub fn fearexplosionDestructor(e: &mut Engine, _: i32) -> LResult<()> {
    for i in 0..14 {
        let em = emitter(e, i);
        APIparticle::EmitterDestroy(e, em);
    }
    let t = e.r32(L(e) + 4) as i32;
    APIparticle::TypeDestroy(e, t);
    Ok(())
}

//! `velmaboss`: EP4 Velma boss stand-in (waves, throws three Scooby snacks at the leader every 8 s while in
//! range; swaps with its twin object 0x2a0000c0/c1 on messages). Port of
//! `Ep4_Bayou/Characters/VELMA_BOSS/velmaboss` (1 build).
#![allow(non_snake_case, non_upper_case_globals, unused_variables)]

use super::modkit::*;
use crate::engine::*;
use crate::logic::{LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "velmaboss",
    data_size: 0x30,
    ctor: velmabossConstructor,
    dtor: velmabossDestructor,
    register: None,
    init: &[],
    build_init: None,
};

pub const pExtra: u32 = 0x10000;
pub const pGameObj: u32 = 0x10004;
pub const pPhysic: u32 = 0x10008;
pub const pAnim: u32 = 0x1000c;
pub const pStdlib: u32 = 0x10010;
pub const pSound: u32 = 0x10014;
pub const pAnimMngr: u32 = 0x10018;
pub const pFunction: u32 = 0x1001c;
pub const pMisc: u32 = 0x10020;
pub const pLocalDatavelmaboss: u32 = 0x10024;
pub const pGlobalDatavelmaboss: u32 = 0x10028;

/// local (0x2c): +4 DM, +8 flag, +0xc snacks in flight, +0x10 launch timer, +0x14 stand timer,
/// +0x18 wave count, +0x1c snack home pos, +0x28 wave anim ended
fn L(e: &Engine, o: u32) -> Ptr {
    pw(e, pLocalDatavelmaboss, o)
}
fn lr(e: &Engine, o: u32) -> u32 {
    e.r32(L(e, o))
}
fn lw(e: &mut Engine, o: u32, v: u32) {
    let a = L(e, o);
    e.w32(a, v);
}
fn leader(e: &Engine) -> u32 {
    e.r32(e.r32(pw(e, pGlobalDatavelmaboss, 0)) + 8)
}
fn in_range(e: &mut Engine, r: f32) -> bool {
    let l = leader(e);
    LIBApiCFunctions::CheckRangeObject(e, r, SELF, l, 0) != 0
}

pub fn velmabossST_FIRSTMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    lw(e, 0xc, 0);
    lw(e, 8, 1);
    velmabossFUNC_CreatePhysic(e);
    let p = L(e, 0x1c);
    APIgameobj::GetPosition(e, 0x2a0000c0, 0x7fff_ffff, p);
    let y = e.rf(p + 8) + 15.0;
    e.wf(p + 8, y);
    APIgameobj::SetNextState(e, SELF, 1);
    Ok(0)
}

pub fn velmabossST_STAND_SET(e: &mut Engine, _: u32) -> LResult<u64> {
    APIanim::Play(e, SELF, 2, 6, 0.8);
    let t = lr(e, 0x14);
    APIextra::TimerReset(e, t);
    APIgameobj::SetNextState(e, SELF, 2);
    Ok(0)
}

pub fn velmabossST_STAND_LOOP(e: &mut Engine, _: u32) -> LResult<u64> {
    if !in_range(e, 4.0) {
        let t = lr(e, 0x14);
        if 5.0 < APIextra::TimerGetTime(e, t) {
            APIgameobj::SetNextState(e, SELF, 3);
        }
    }
    velmaboss_FUNC_Launch_ScoobyScacks(e);
    Ok(0)
}

pub fn velmabossST_WAVE_SET(e: &mut Engine, _: u32) -> LResult<u64> {
    LIBAnimMngr::PlayAnim(e, 0.5, 1, 1, 6);
    lw(e, 0x18, 0);
    lw(e, 0x28, 0);
    APIgameobj::SetNextState(e, SELF, 4);
    Ok(0)
}

pub fn velmabossST_WAVE_LOOP(e: &mut Engine, _: u32) -> LResult<u64> {
    if in_range(e, 4.0) || lr(e, 0x18) >= 2 {
        APIgameobj::SetNextState(e, SELF, 1);
    } else if lr(e, 0x28) != 0 {
        lw(e, 0x28, 0);
        LIBAnimMngr::PlayAnim(e, 0.0, 1, 1, 6);
    }
    velmaboss_FUNC_Launch_ScoobyScacks(e);
    Ok(0)
}

pub fn velmabossCB_NOTIFY_COLLIDE_OBJ(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

pub fn velmabossCB_ANIM_LF(e: &mut Engine, p: u32) -> LResult<u64> {
    if p == 1 {
        let n = lr(e, 0x18) + 1;
        lw(e, 0x18, n);
        lw(e, 0x28, 1);
    }
    Ok(1)
}

fn dec_snacks(e: &mut Engine) {
    if lr(e, 0xc) != 0 {
        let n = lr(e, 0xc) - 1;
        lw(e, 0xc, n);
    }
}

pub fn velmabossFUNC_MessageHandler(e: &mut Engine, m: u32, p2: u32, _: Ptr) -> LResult<u64> {
    let v = e.alloca(0x10);
    match m {
        0x211 => {
            e.wf(v, 0.0);
            e.wf(v + 4, 0.0);
            e.wf(v + 8, 0.0);
            APIgameobj::SetTranslation(e, SELF, v);
            APIgameobj::SetGameObjectVisible(e, SELF, 0);
        }
        0x212 => {
            APIgameobj::GetPosition(e, 0x2a0000c1, 0x7fff_ffff, v);
            APIgameobj::TakeOrientation(e, 0x2a0000c0, 0x2a0000c1, 0x7fff_ffff);
            APIgameobj::SetTranslation(e, 0x2a0000c0, v);
            APIgameobj::SetGameObjectVisible(e, 0x2a0000c0, 1);
        }
        0x149 => {
            if (0x2a000020..=0x2a000022).contains(&p2) {
                let h = L(e, 0x1c);
                APIgameobj::SetTranslation(e, p2, h);
            }
            dec_snacks(e);
            if lr(e, 0xc) == 0 {
                lw(e, 8, 1);
            }
        }
        0xc1 => {
            dec_snacks(e);
            let t = lr(e, 0x10);
            APIextra::TimerReset(e, t);
        }
        0x15 | 0xc9 => {
            APIgameobj::GetPosition(e, 0x2a0000c0, 0x7fff_ffff, v);
            APIgameobj::TakeOrientation(e, 0x2a0000c1, 0x2a0000c0, 0x7fff_ffff);
            velmabossFUNC_MessageHandler(e, 0x211, 0, NULL)?;
            APIgameobj::SetGameObjectVisible(e, 0x2a0000c1, 1);
            APIgameobj::SetTranslation(e, 0x2a0000c1, v);
        }
        _ => {}
    }
    Ok(0)
}

pub fn velmaboss_FUNC_LaunchItem(e: &mut Engine, item: u32) {
    let v = e.alloca(0x10);
    let o = APIgameobj::LaunchLatant(e, 0x2a80000e);
    APIgameobj::GetPosition(e, SELF, 0x7fff_ffff, v);
    let y = e.rf(v + 4) + 1.5;
    e.wf(v + 4, y);
    APIgameobj::SetTranslation(e, o, v);
    APIgameobj::AddChild(e, o, 0x7fff_ffff, item, 1);
    let n = lr(e, 0xc) + 1;
    lw(e, 0xc, n);
}

pub fn velmaboss_FUNC_SetClipping(e: &mut Engine) {
    let p = e.alloca(0x28);
    APIgameobj::GetClippingParams(e, SELF, p);
    for (o, v) in [
        (0, 0x3dcccccd),
        (4, 0x41700000),
        (0x10, 0x41d00000),
        (0x1c, 0xbf800000),
        (0xc, 0),
        (8, 0x47),
        (0x18, 0),
        (0x14, 0x6a7),
        (0x24, 0x6a7),
        (0x20, 0x6a7),
    ] {
        e.w32(p + o, v);
    }
    APIgameobj::SetClippingParams(e, SELF, p, 0);
}

pub fn velmaboss_FUNC_Launch_ScoobyScacks(e: &mut Engine) {
    if lr(e, 0xc) != 0 {
        return;
    }
    let t = lr(e, 0x10);
    if !(8.0 < APIextra::TimerGetTime(e, t)) {
        return;
    }
    let gb1 = e.r32(pw(e, pGlobalDatavelmaboss, 4));
    if !(e.rf(gb1) < 1.0) || !in_range(e, 12.0) {
        return;
    }
    APInewSound::Play(e, 0x2a000008, 0xa000_0000, 0);
    for i in [0x2a000020, 0x2a000021, 0x2a000022] {
        velmaboss_FUNC_LaunchItem(e, i);
    }
    APIextra::TimerReset(e, t);
}

pub fn velmabossFUNC_CreatePhysic(e: &mut Engine) {
    APIphysic::CreatePhysic(e, SELF, 0);
    APIphysic::SetWeight(e, SELF, 1.0);
    APIphysic::SetPriority(e, SELF, 100);
    let h = APIphysic::DMAdd(e, SELF, 1);
    lw(e, 4, h);
    let p = e.alloca(0x40);
    APIphysic::DMGetParams(e, h, p);
    let l = leader(e);
    for (o, v) in [(4, 0x40f00000), (0, l), (8, 0), (0xc, 0), (0x10, 0), (0x14, 5)] {
        e.w32(p + o, v);
    }
    APIphysic::DMSetParams(e, h, p);
    APIphysic::DMSetContextMask(e, h, 1);
    APIgameobj::SetContextFlag(e, SELF, 1);
}

pub fn velmabossConstructor(e: &mut Engine, api: Ptr, n: i32) -> LResult<()> {
    if n == 1 {
        ext(e, &[
            (0, velmabossST_FIRSTMOVE),
            (1, velmabossST_STAND_SET),
            (2, velmabossST_STAND_LOOP),
            (3, velmabossST_WAVE_SET),
            (4, velmabossST_WAVE_LOOP),
            (5, velmabossCB_NOTIFY_COLLIDE_OBJ),
            (6, velmabossCB_ANIM_LF),
        ]);
        let v = APIcons::GetApiAnim(e); gset(e, pAnim, v);
        let v = APIcons::GetApiGameObj(e); gset(e, pGameObj, v);
        let v = APIcons::GetApiExtra(e); gset(e, pExtra, v);
        let v = APIcons::GetApiPhysic(e); gset(e, pPhysic, v);
        let v = APIcons::GetApiNewSound(e); gset(e, pSound, v);
        let v = APIcons::GetApiStdlib(e); gset(e, pStdlib, v);
        custom(e, pAnimMngr, "AnimMngr");
        custom(e, pMisc, "MiscFunc");
        custom(e, pFunction, "ApiCFunctions");
        let s = G(e, pGlobalDatavelmaboss);
        APIcons::GetGlobalData(e, s, 8);
        let g = pw(e, pGlobalDatavelmaboss, 0);
        let b = APIgameobj::GetGameBuffer(e, 0, 0x8f8, 1);
        e.w32(g, b);
        let b = APIgameobj::GetGameBuffer(e, 1, 0x68, 1);
        e.w32(g + 4, b);
    }
    APIcons::SetMessageFunction(e, velmabossFUNC_MessageHandler);
    let s = G(e, pLocalDatavelmaboss);
    APIcons::GetLocalData(e, s, 0x2c);
    let t = APIextra::TimerCreate(e);
    lw(e, 0x10, t);
    let t = APIextra::TimerCreate(e);
    lw(e, 0x14, t);
    let l = leader(e);
    LIBMiscFunc::MiscFunc_SetClipping(e, SELF, l, 0);
    velmaboss_FUNC_SetClipping(e);
    velmabossFUNC_MessageHandler(e, 0x211, 0, NULL)?;
    APIcons::SetInitialState(e, 0);
    Ok(())
}

pub fn velmabossDestructor(e: &mut Engine, n: i32) -> LResult<()> {
    for o in [0x10, 0x14] {
        let t = lr(e, o);
        APIextra::TimerDestroy(e, t);
    }
    Ok(())
}

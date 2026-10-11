//! `redknight`: EP2L01C red knight (cut-scene actor walking waypoint path 2). Port of
//! `Ep2_MovieStudio/Characters/L01C_RedKnight/redknight` (12 functions, 1 build).
#![allow(non_snake_case, non_upper_case_globals, unused_variables)]

use super::modkit::*;
use crate::engine::*;
use crate::logic::{LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "redknight",
    data_size: 0x28,
    ctor: redknightConstructor,
    dtor: redknightDestructor,
    register: None,
    init: &[],
    build_init: None,
};

pub const pExtra: u32 = 0x10000;
pub const pGameObj: u32 = 0x10004;
pub const pPhysic: u32 = 0x10008;
pub const pAnim: u32 = 0x1000c;
pub const pInput: u32 = 0x10010;
pub const pAnimMngr: u32 = 0x10014;
pub const pFunction: u32 = 0x10018;
pub const pMiscFunc: u32 = 0x1001c;
pub const pLocalDataRedKnight: u32 = 0x10020;
pub const pGlobalDataRedKnight: u32 = 0x10024;

/// local (0x38): +4..+0x14 DMs, +0x18 target, +0x24 waypoint idx, +0x28 waypoint count, +0x2c start pos
fn L(e: &Engine, o: u32) -> Ptr {
    pw(e, pLocalDataRedKnight, o)
}
fn lr(e: &Engine, o: u32) -> u32 {
    e.r32(L(e, o))
}
fn lw(e: &mut Engine, o: u32, v: u32) {
    let a = L(e, o);
    e.w32(a, v);
}

/// state 0
pub fn RedKnightST_FIRSTMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    RedKnightFUNC_CreatePhysic(e);
    APIanim::Play(e, SELF, 1, 1, 0.0);
    let p = L(e, 0x2c);
    APIgameobj::GetPosition(e, SELF, 0x7fff_ffff, p);
    LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 4, NULL, 0);
    APIgameobj::SetNextState(e, SELF, 1);
    Ok(0)
}
/// state 1
pub fn RedKnightST_BASICMOVE_SET(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}
/// state 2
pub fn RedKnightST_BASICMOVE_LOOP(e: &mut Engine, _: u32) -> LResult<u64> {
    RedKnightFUNC_CheckPointPos(e);
    Ok(0)
}
/// callback 3
pub fn RedKnightCB_NOTIFY_COLLIDE_OBJ(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}
/// callback 4
pub fn RedKnightCB_ANIM_LF(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(1)
}
/// callback 5 (cut-scene notify 0x16 {id, started})
pub fn RedKnightCB_CUTSCENE(e: &mut Engine, p: u32) -> LResult<u64> {
    if e.r32(p) == 1 {
        let s = e.r32(p + 4);
        if s == 1 {
            LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 2, NULL, 0);
            APIgameobj::SendMessageTo(e, 0x2a00004b, NULL, 0x1d3, 0, NULL);
            APIphysic::SetFlags(e, SELF, 1);
            APIinput::InputConfigActivate(e, 2, 0);
        } else if s == 0 {
            APIphysic::ResetFlags(e, SELF, 1);
            APIinput::InputConfigActivate(e, 2, 0);
            APIgameobj::SendMessageTo(e, 0x2a00004b, NULL, 0x1d3, 0, NULL);
            APIgameobj::SendMessageTo(e, 0x2a000029, NULL, 0xde, 0, NULL);
        }
    }
    Ok(0)
}

fn start_walk(e: &mut Engine) {
    let n = APIextra::WaypointGetNb(e, 2);
    lw(e, 0x28, n);
    let (i, t) = (lr(e, 0x24), L(e, 0x18));
    APIextra::WaypointGetPos(e, 2, i, t);
    RedKnightFUNC_UpdateTarget(e, t);
    APIanim::Play(e, SELF, 2, 2, 0.0);
    APIgameobj::SetContextFlag(e, SELF, 0x3e);
}

pub fn RedKnightFUNC_MessageHandler(e: &mut Engine, m: u32, _: u32, _: Ptr) -> LResult<u64> {
    if m == 0x21 {
        let p = L(e, 0x2c);
        APIgameobj::SetTranslation(e, SELF, p);
        lw(e, 0x24, 0);
        start_walk(e);
        APIphysic::SetVelocity(e, SELF, 0.0, 0.0, 0.0);
        APIgameobj::SetNextState(e, SELF, 2);
    } else if m == 0xde {
        APIgameobj::SetGroupBitMask(e, SELF, 0x100001);
        APIgameobj::SendMessageTo(e, 0x2a0000d3, NULL, 0xeb, 2, NULL);
        start_walk(e);
        LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 2, NULL, 0);
        APIgameobj::SetNextState(e, SELF, 2);
    }
    Ok(0)
}

fn dm(e: &mut Engine, slot: u32, kind: u32, mask: u32, get: bool, words: &[(u32, u32)]) {
    let h = APIphysic::DMAdd(e, SELF, kind);
    lw(e, slot, h);
    let p = e.alloca(0x40);
    if get {
        APIphysic::DMGetParams(e, h, p);
    }
    for &(o, v) in words {
        e.w32(p + o, v);
    }
    APIphysic::DMSetParams(e, h, p);
    APIphysic::DMSetContextMask(e, h, mask);
}

pub fn RedKnightFUNC_CreatePhysic(e: &mut Engine) {
    APIphysic::CreatePhysic(e, SELF, 0);
    APIphysic::SetFlags(e, SELF, 6);
    APIphysic::SetPrimaryPrim(e, SELF, 0x38e);
    APIphysic::SetWeight(e, SELF, 0.75);
    APIphysic::SetPriority(e, SELF, 10);
    dm(e, 4, 1, 4, true, &[(4, 0x41700000), (0, 0), (8, 0), (0xc, 0), (0x10, 0), (0x14, 5)]);
    dm(e, 8, 4, 2, true, &[(0xc, 1), (0, 0), (4, 0), (8, 0x40b9999a)]);
    // DM 8: no DMGetParams in the original (rest of the block is stack garbage; zero here)
    dm(e, 0xc, 8, 8, false, &[(4, 0x42340000), (0, 0x42960000)]);
    dm(e, 0x10, 0, 0x10, true, &[(0, 0), (4, 0xc1a00000), (8, 0)]);
    dm(e, 0x14, 7, 0x20, true, &[(0, 0x41200000), (4, 5)]);
    APIgameobj::SetContextFlag(e, SELF, 0);
}

pub fn RedKnightFUNC_CheckPointPos(e: &mut Engine) {
    let t = L(e, 0x18);
    if LIBApiCFunctions::CheckRangePoint(e, 0.7, SELF, t, 0) != 0 {
        if lr(e, 0x24) < lr(e, 0x28).wrapping_sub(1) {
            let i = lr(e, 0x24) + 1;
            lw(e, 0x24, i);
            APIextra::WaypointGetPos(e, 2, i, t);
            RedKnightFUNC_UpdateTarget(e, t);
        } else {
            APIanim::Play(e, SELF, 1, 1, 0.0);
            APIgameobj::SetContextFlag(e, SELF, 0);
            APIphysic::SetVelocity(e, SELF, 0.0, 0.0, 0.0);
            APIgameobj::SetNextState(e, SELF, 1);
        }
    }
}

pub fn RedKnightFUNC_UpdateTarget(e: &mut Engine, t: Ptr) {
    let h = lr(e, 4);
    let p = e.alloca(0x40);
    APIphysic::DMGetParams(e, h, p);
    let v = [e.r32(t), e.r32(t + 4), e.r32(t + 8)];
    for (o, w) in [(4, 0x41200000), (0, 0), (8, v[0]), (0xc, v[1]), (0x10, v[2]), (0x14, 5)] {
        e.w32(p + o, w);
    }
    APIphysic::DMSetParams(e, h, p);
}

pub fn redknightConstructor(e: &mut Engine, api: Ptr, n: i32) -> LResult<()> {
    if n == 1 {
        ext(e, &[
            (0, RedKnightST_FIRSTMOVE),
            (1, RedKnightST_BASICMOVE_SET),
            (2, RedKnightST_BASICMOVE_LOOP),
            (3, RedKnightCB_NOTIFY_COLLIDE_OBJ),
            (4, RedKnightCB_ANIM_LF),
            (5, RedKnightCB_CUTSCENE),
        ]);
        let v = APIcons::GetApiAnim(e); gset(e, pAnim, v);
        let v = APIcons::GetApiGameObj(e); gset(e, pGameObj, v);
        let v = APIcons::GetApiExtra(e); gset(e, pExtra, v);
        let v = APIcons::GetApiPhysic(e); gset(e, pPhysic, v);
        let v = APIcons::GetApiInput(e); gset(e, pInput, v);
        custom(e, pAnimMngr, "AnimMngr");
        custom(e, pFunction, "ApiCFunctions");
        custom(e, pMiscFunc, "MiscFunc");
        let s = G(e, pGlobalDataRedKnight);
        APIcons::GetGlobalData(e, s, 4);
        let b = APIgameobj::GetGameBuffer(e, 0, 0x8f8, 1);
        let g = pw(e, pGlobalDataRedKnight, 0);
        e.w32(g, b);
    }
    APIcons::SetMessageFunction(e, RedKnightFUNC_MessageHandler);
    let s = G(e, pLocalDataRedKnight);
    APIcons::GetLocalData(e, s, 0x38);
    APIgameobj::RegisterNotify(e, SELF, 1, 0x16, 5);
    LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 2, NULL, 0);
    APIcons::SetInitialState(e, 0);
    Ok(())
}

pub fn redknightDestructor(e: &mut Engine, n: i32) -> LResult<()> {
    Ok(())
}

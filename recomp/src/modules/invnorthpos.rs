//! `invnorthpos`: invisible radar north marker; first frame sets the clipping (MiscFunc
//! ChangeClipping(SELF, 4, NULL, 0)) and goes to state 1, whose functions are empty.
//! Port of `Game Logic Modules/Globals/Panel/Radar/invnorthpos/invNorthPos.c` (7 functions).
//! EP1L01 build 286dc9e0.
#![allow(non_snake_case, non_upper_case_globals, unused_variables)]

use crate::engine::*;
use crate::logic::{LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "invnorthpos",
    // globals 0x10000..0x1001c (+ unused gpoLanguagesNamesAndISOCodes)
    data_size: 0x1c,
    ctor: invnorthposConstructor,
    dtor: invnorthposDestructor,
    register: None,
    init: &[],
    build_init: None,
};

pub fn build_index(key: &str) -> Option<usize> {
    (key == "286dc9e0").then_some(0)
}

pub const pExtra: u32 = 0x10000;
pub const pGameObj: u32 = 0x10004;
pub const pPhysic: u32 = 0x10008;
pub const pAnim: u32 = 0x1000c;
pub const pMisc: u32 = 0x10010;
/// not in the symtab (the .sbss gap between pMisc and pGlobalDataNorthPos)
pub const pLocalDataNorthPos: u32 = 0x10014;
pub const pGlobalDataNorthPos: u32 = 0x10018;

#[inline]
fn G(e: &Engine, a: u32) -> Ptr {
    e.rt.cur_data() + (a - 0x10000)
}

/// 0x00010ce0 state 0
pub fn fstate_FIRSTMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 4, NULL, 0);
    APIgameobj::SetNextState(e, SELF, 1);
    Ok(0)
}

/// 0x00010d58
pub fn fstate_BASICMOVE_SET(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

/// 0x00010d88
pub fn fstate_BASICMOVE_LOOP(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

/// 0x00010db8
pub fn fcback_NOTIFY_COLLIDE_OBJ(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

/// 0x00010e08
pub fn MyMessageFunction(e: &mut Engine, m: u32, _s: u32, _d: Ptr) -> LResult<u64> {
    Ok(0)
}

/// 0x00010e40
pub fn invnorthposConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    if instanceCount == 1 {
        APIcons::SetExternalFunction(e, 0, fstate_FIRSTMOVE);
        APIcons::SetExternalFunction(e, 1, fstate_BASICMOVE_SET);
        APIcons::SetExternalFunction(e, 2, fstate_BASICMOVE_LOOP);
        APIcons::SetExternalFunction(e, 3, fcback_NOTIFY_COLLIDE_OBJ);
        let tables: [(u32, fn(&mut Engine) -> Ptr); 4] = [
            (pAnim, |e| APIcons::GetApiAnim(e)),
            (pGameObj, |e| APIcons::GetApiGameObj(e)),
            (pExtra, |e| APIcons::GetApiExtra(e)),
            (pPhysic, |e| APIcons::GetApiPhysic(e)),
        ];
        for (slot, get) in tables {
            let v = get(e);
            let a = G(e, slot);
            e.w32(a, v);
        }
        let n = e.cstr("MiscFunc");
        let v = APIcons::GetApiCustom(e, n);
        let a = G(e, pMisc);
        e.w32(a, v);
        let slot = G(e, pGlobalDataNorthPos);
        APIcons::GetGlobalData(e, slot, 4);
        let b = APIgameobj::GetGameBuffer(e, 0, 0x8f8, 1);
        let gd = e.r32(G(e, pGlobalDataNorthPos));
        e.w32(gd, b);
    }
    APIcons::SetMessageFunction(e, MyMessageFunction);
    let slot = G(e, pLocalDataNorthPos);
    APIcons::GetLocalData(e, slot, 4);
    APIcons::SetInitialState(e, 0);
    Ok(())
}

/// 0x00011008
pub fn invnorthposDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    Ok(())
}

//! `beamoflight`: Ep1 beam-of-light prop: an empty state machine (FIRSTMOVE -> 1, nothing else).
//! Port of `Game Logic Modules/Ep1_Library/Objects/BEAMOFLIGHT/beamoflight` (7 functions). Build 1cbc23a3 (EP1L01 only).
#![allow(non_snake_case, non_upper_case_globals, unused_variables)]

use crate::engine::*;
use crate::logic::{LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "beamoflight",
    data_size: 0x18,
    ctor: beamoflightConstructor,
    dtor: beamoflightDestructor,
    register: None,
    init: &[],
    build_init: None,
};

pub fn build_index(key: &str) -> Option<usize> {
    (key == "1cbc23a3").then_some(0)
}

pub const pExtra: u32 = 0x10000;
pub const pGameObj: u32 = 0x10004;
pub const pPhysic: u32 = 0x10008;
pub const pAnim: u32 = 0x1000c;
/// not in _globals.h (unnamed), between pAnim and pGlobalDatabeamoflight
pub const pLocalDatabeamoflight: u32 = 0x10010;
pub const pGlobalDatabeamoflight: u32 = 0x10014;

#[inline]
fn G(e: &Engine, a: u32) -> Ptr {
    e.rt.cur_data() + (a - 0x10000)
}

/// 0x00010cc8
pub fn fstate_FIRSTMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    APIgameobj::SetNextState(e, SELF, 1);
    Ok(0)
}
/// 0x00010d20
pub fn fstate_BASICMOVE_SET(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}
/// 0x00010d50
pub fn fstate_BASICMOVE_LOOP(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}
/// 0x00010d80
pub fn fcback_NOTIFY_COLLIDE_OBJ(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}
/// 0x00010dd0
pub fn MyMessageFunction(e: &mut Engine, _m: u32, _s: u32, _d: Ptr) -> LResult<u64> {
    Ok(0)
}

/// 0x00010e08
pub fn beamoflightConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
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
        let slot = G(e, pGlobalDatabeamoflight);
        APIcons::GetGlobalData(e, slot, 4);
        let b = APIgameobj::GetGameBuffer(e, 0, 0x8f8, 1);
        let gd = e.r32(G(e, pGlobalDatabeamoflight));
        e.w32(gd, b);
    }
    APIcons::SetMessageFunction(e, MyMessageFunction);
    let slot = G(e, pLocalDatabeamoflight);
    APIcons::GetLocalData(e, slot, 4);
    APIcons::SetInitialState(e, 0);
    Ok(())
}

/// 0x00010fb0
pub fn beamoflightDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    Ok(())
}

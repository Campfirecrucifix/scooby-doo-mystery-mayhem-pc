//! `switchdoor`: EP6 switch door, msg 0xde opens (anim 3), 0xdf closes (anim 1). Port of
//! `Ep6_ShermanTech/Objects/SWITCH_DOOR/switchdoor` (8 functions, 1 build).
//! pLocalDataSwitchDoor is not in the symtab; placed in the gap at 0x1002c.
#![allow(non_snake_case, non_upper_case_globals, unused_variables)]

use crate::engine::*;
use crate::logic::{LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "switchdoor",
    data_size: 0x34,
    ctor: switchdoorConstructor,
    dtor: switchdoorDestructor,
    register: None,
    init: &[],
    build_init: None,
};

pub const pMath: u32 = 0x10000;
pub const pParticle: u32 = 0x10004;
pub const pAnimMngr: u32 = 0x10008;
pub const pText: u32 = 0x1000c;
pub const pDebug: u32 = 0x10010;
pub const pExtra: u32 = 0x10014;
pub const pGameObj: u32 = 0x10018;
pub const pPhysic: u32 = 0x1001c;
pub const pAnim: u32 = 0x10020;
pub const pInput: u32 = 0x10024;
pub const pGame: u32 = 0x10028;
pub const pLocalDataSwitchDoor: u32 = 0x1002c;
pub const pGlobalDataSwitchDoor: u32 = 0x10030;

#[inline]
fn G(e: &Engine, a: u32) -> Ptr {
    e.rt.cur_data() + (a - 0x10000)
}

/// 0x00010d08 state 0
pub fn SwitchDoorST_FIRSTMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    Create_Physic(e);
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 3, 3);
    APIgameobj::SetNextState(e, SELF, 1);
    Ok(0)
}

/// 0x00010da0 state 1
pub fn SwitchDoorST_BASICMOVE_SET(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

/// 0x00010dd0 state 2
pub fn SwitchDoorST_BASICMOVE_LOOP(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

/// 0x00010e00 callback 3
pub fn SwitchDoorCB_NOTIFY_COLLIDE_OBJ(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

/// 0x00010e30
pub fn MyMessageFunction(e: &mut Engine, param_1: u32, _: u32, _: Ptr) -> LResult<u64> {
    if param_1 == 0xde {
        LIBAnimMngr::PlayAnim(e, 0.0, 3, 1, 0);
    } else if param_1 == 0xdf {
        LIBAnimMngr::PlayAnim(e, 0.0, 1, 1, 0);
    }
    Ok(0)
}

/// 0x00010ef0
pub fn Create_Physic(e: &mut Engine) {
    APIphysic::CreatePhysic(e, SELF, 0);
    APIphysic::SetFlags(e, SELF, 2);
    APIphysic::SetWeight(e, SELF, 1.0);
    APIphysic::SetPriority(e, SELF, 100);
}

/// 0x00010f90
pub fn switchdoorConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    if instanceCount == 1 {
        let fns: [crate::logic::ExtFn; 4] =
            [SwitchDoorST_FIRSTMOVE, SwitchDoorST_BASICMOVE_SET, SwitchDoorST_BASICMOVE_LOOP, SwitchDoorCB_NOTIFY_COLLIDE_OBJ];
        for (i, f) in fns.into_iter().enumerate() {
            APIcons::SetExternalFunction(e, i as _, f);
        }
        let tables: [(u32, fn(&mut Engine) -> Ptr); 6] = [
            (pAnim, |e| APIcons::GetApiAnim(e)),
            (pGameObj, |e| APIcons::GetApiGameObj(e)),
            (pExtra, |e| APIcons::GetApiExtra(e)),
            (pPhysic, |e| APIcons::GetApiPhysic(e)),
            (pInput, |e| APIcons::GetApiInput(e)),
            (pGame, |e| APIcons::GetApiGame(e)),
        ];
        for (slot, get) in tables {
            let v = get(e);
            let a = G(e, slot);
            e.w32(a, v);
        }
        for (slot, name) in [(pDebug, "debug"), (pAnimMngr, "AnimMngr"), (pText, "ApiText")] {
            let n = e.cstr(name);
            let v = APIcons::GetApiCustom(e, n);
            let a = G(e, slot);
            e.w32(a, v);
        }
        let v = APIcons::GetApiMath(e);
        let a = G(e, pMath);
        e.w32(a, v);
        let v = APIcons::GetApiParticle(e);
        let a = G(e, pParticle);
        e.w32(a, v);
        let slot = G(e, pGlobalDataSwitchDoor);
        APIcons::GetGlobalData(e, slot, 8);
        let b = APIgameobj::GetGameBuffer(e, 0, 0x8f8, 1);
        let g = e.r32(G(e, pGlobalDataSwitchDoor));
        e.w32(g, b);
        let b = APIgameobj::GetGameBuffer(e, 1, 0x68, 1);
        e.w32(g + 4, b);
    }
    APIcons::SetMessageFunction(e, MyMessageFunction);
    let slot = G(e, pLocalDataSwitchDoor);
    APIcons::GetLocalData(e, slot, 4);
    APIcons::SetInitialState(e, 0);
    Ok(())
}

/// 0x00011228
pub fn switchdoorDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    Ok(())
}

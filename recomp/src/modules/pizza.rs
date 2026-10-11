//! `pizza`: a pizza prop that plays its anim 1 once and kills itself when it ends (or on msg 0xd4).
//! Port of `Game Logic Modules/Globals/Objects/PIZZA/pizza__6e815558` (5 functions). Build 6e815558 (EP1L01).
#![allow(non_snake_case, non_upper_case_globals, unused_variables)]

use crate::engine::*;
use crate::logic::{LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "pizza",
    data_size: 0x10,
    ctor: pizzaConstructor,
    dtor: pizzaDestructor,
    register: None,
    init: &[],
    build_init: None,
};

pub fn build_index(key: &str) -> Option<usize> {
    matches!(key, "6e815558" | "ea8cb7c6").then_some(0)
}

pub const pGameObj: u32 = 0x10000;
pub const pPhysic: u32 = 0x10004;
pub const pDebug: u32 = 0x10008;
pub const pAnimMngr: u32 = 0x1000c;

#[inline]
fn G(e: &Engine, a: u32) -> Ptr {
    e.rt.cur_data() + (a - 0x10000)
}

/// 0x00010cd8
pub fn PizzaST_BASICMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

/// 0x00010d08 external 1
pub fn PizzaCB_ANIM_LF(e: &mut Engine, _: u32) -> LResult<u64> {
    APIgameobj::KillObject(e, SELF);
    Ok(1)
}

/// 0x00010d58
pub fn PizzaFUNC_MessageFunction(e: &mut Engine, param_1: u32, _s: u32, _d: Ptr) -> LResult<u64> {
    if param_1 == 0xd4 {
        APIgameobj::KillObject(e, SELF);
    }
    Ok(0)
}

/// 0x00010dd0
pub fn pizzaConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    if instanceCount == 1 {
        APIcons::SetExternalFunction(e, 0, PizzaST_BASICMOVE);
        APIcons::SetExternalFunction(e, 1, PizzaCB_ANIM_LF);
        let v = APIcons::GetApiGameObj(e);
        let a = G(e, pGameObj);
        e.w32(a, v);
        let v = APIcons::GetApiPhysic(e);
        let a = G(e, pPhysic);
        e.w32(a, v);
        for (slot, name) in [(pDebug, "debug"), (pAnimMngr, "AnimMngr")] {
            let n = e.cstr(name);
            let v = APIcons::GetApiCustom(e, n);
            let a = G(e, slot);
            e.w32(a, v);
        }
    }
    APIcons::SetMessageFunction(e, PizzaFUNC_MessageFunction);
    // (*pAnimMngr->[0])(0.0, 1, 1, 1) = PlayAnim
    LIBAnimMngr::PlayAnim(e, 0.0, 1, 1, 1);
    APIcons::SetInitialState(e, 0);
    Ok(())
}

/// 0x00010f10
pub fn pizzaDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    Ok(())
}

//! `costume`: the costume collectable placeholder: sleeps forever, dies on msg 0xd4.
//! Port of `Game Logic Modules/Globals/Collectables/COSTUME/costume__26b01e5f` (4 functions). Build 26b01e5f (EP1L01).
#![allow(non_snake_case, non_upper_case_globals, unused_variables)]

use crate::engine::*;
use crate::logic::{LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "costume",
    data_size: 0x8,
    ctor: costumeConstructor,
    dtor: costumeDestructor,
    register: None,
    init: &[],
    build_init: None,
};

pub fn build_index(key: &str) -> Option<usize> {
    ["26b01e5f", "c79ecd37"].iter().position(|k| *k == key) // c79ecd37 (EP1L03A): identical code
}

pub const pGameObj: u32 = 0x10000;
pub const pDebug: u32 = 0x10004;

#[inline]
fn G(e: &Engine, a: u32) -> Ptr {
    e.rt.cur_data() + (a - 0x10000)
}

/// 0x00010cc0
pub fn CostumeST_BASICMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    APIgameobj::Sleep(e, SELF, -1.0);
    Ok(0)
}

/// 0x00010d18
pub fn CostumeFUNC_MessageFunction(e: &mut Engine, param_1: u32, _s: u32, _d: Ptr) -> LResult<u64> {
    if param_1 == 0xd4 {
        APIgameobj::KillObject(e, SELF);
    }
    Ok(0)
}

/// 0x00010d90
pub fn costumeConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    if instanceCount == 1 {
        APIcons::SetExternalFunction(e, 0, CostumeST_BASICMOVE);
        let v = APIcons::GetApiGameObj(e);
        let a = G(e, pGameObj);
        e.w32(a, v);
        let n = e.cstr("debug");
        let v = APIcons::GetApiCustom(e, n);
        let a = G(e, pDebug);
        e.w32(a, v);
    }
    APIcons::SetMessageFunction(e, CostumeFUNC_MessageFunction);
    APIcons::SetInitialState(e, 0);
    Ok(())
}

/// 0x00010e58
pub fn costumeDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    Ok(())
}

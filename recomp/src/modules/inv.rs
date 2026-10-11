//! `inv`: invisible helper object (trigger volume / marker): clipping mode 2, then disables itself.
//! Port of `Game Logic Modules/Globals/Objects/Inv/inv__294d5726` (3 functions). Build 294d5726.
#![allow(non_snake_case, non_upper_case_globals, unused_variables)]

use crate::engine::*;
use crate::logic::{LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "inv",
    data_size: 0x8,
    ctor: invConstructor,
    dtor: invDestructor,
    register: None,
    init: &[],
    build_init: None,
};

pub fn build_index(key: &str) -> Option<usize> {
    matches!(key, "294d5726" | "d7bd4fce").then_some(0)
}

pub const pMiscFunc: u32 = 0x10000;
pub const pGameObj: u32 = 0x10004;

#[inline]
fn G(e: &Engine, a: u32) -> Ptr {
    e.rt.cur_data() + (a - 0x10000)
}

/// 0x00010cc8
pub fn null_main(e: &mut Engine, _: u32) -> LResult<u64> {
    APIgameobj::EnableObject(e, SELF, 0);
    Ok(0)
}

/// 0x00010d20
pub fn invConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    if instanceCount == 1 {
        APIcons::SetExternalFunction(e, 0, null_main);
        let v = APIcons::GetApiGameObj(e);
        let a = G(e, pGameObj);
        e.w32(a, v);
        let n = e.cstr("MiscFunc");
        let v = APIcons::GetApiCustom(e, n);
        let a = G(e, pMiscFunc);
        e.w32(a, v);
    }
    // (pMiscFunc + 4) = MiscFunc_ChangeClipping
    LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 2, NULL, 1);
    APIcons::SetInitialState(e, 0);
    Ok(())
}

/// 0x00010df8
pub fn invDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    Ok(())
}

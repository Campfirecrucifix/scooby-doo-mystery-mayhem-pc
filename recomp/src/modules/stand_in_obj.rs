//! `stand_in_obj`: a static stand-in prop; on its first frame it rewrites its clipping params
//! (first entry: distance -1.0, flags 0x6bf / 0x6b8) and sleeps forever. The constructor sets
//! the standard clipping through MiscFunc slot 1 (ChangeClipping(SELF, 2, NULL, 1)).
//! Port of `Game Logic Modules/Globals/Objects/Stand_In_Obj/stand_in_obj/Stand_In_Obj.c` (3 functions).
//! EP1L01 build a0c4f301.
#![allow(non_snake_case, non_upper_case_globals, unused_variables)]

use crate::engine::*;
use crate::logic::{LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "stand_in_obj",
    // globals 0x10000..0x10008 (+ unused gpoLanguagesNamesAndISOCodes)
    data_size: 0x8,
    ctor: stand_in_objConstructor,
    dtor: stand_in_objDestructor,
    register: None,
    init: &[],
    build_init: None,
};

pub fn build_index(key: &str) -> Option<usize> {
    (key == "a0c4f301").then_some(0)
}

pub const pMiscFunc: u32 = 0x10000;
pub const pGameObj: u32 = 0x10004;

#[inline]
fn G(e: &Engine, a: u32) -> Ptr {
    e.rt.cur_data() + (a - 0x10000)
}

/// 0x00010cc8 state 0
pub fn null_main(e: &mut Engine, _: u32) -> LResult<u64> {
    let p = e.alloca(0x40); // aEStack_60 (EFClippingParams)
    APIgameobj::GetClippingParams(e, SELF, p);
    e.w32(p + 4, 0xbf80_0000);
    e.w32(p + 0xc, 0x6b8);
    e.w32(p + 8, 0x6bf);
    APIgameobj::SetClippingParams(e, SELF, p, 0);
    APIgameobj::Sleep(e, SELF, -1.0);
    Ok(0)
}

/// 0x00010d80
pub fn stand_in_objConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
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
    LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 2, NULL, 1);
    APIcons::SetInitialState(e, 0);
    Ok(())
}

/// 0x00010e58
pub fn stand_in_objDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    Ok(())
}

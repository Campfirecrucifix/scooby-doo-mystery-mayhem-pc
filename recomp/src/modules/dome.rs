//! `dome`: state 0 kills the object; state 1 (never entered by the module itself) would glue it to the
//! per-build object ([`FOLLOW`]). Port of `Game Logic Modules/Globals/Objects/Dome/dome__*/Dome.c`.
#![allow(non_snake_case, non_upper_case_globals, unused_variables)]

use crate::engine::*;
use crate::logic::{LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "dome",
    data_size: 0x10,
    ctor: domeConstructor,
    dtor: domeDestructor,
    register: None,
    init: &[],
    build_init: None,
};

const KEYS: [&str; 6] = ["d373bc09", "58679171", "7bebb5f4", "85ad6cf6", "cee9cf63", "f0e5753f"];
const FOLLOW: [u32; 6] = [0x2a000004, 0x2a000006, 0x2a00000e, 0x2a00000f, 0x2a000012, 0x2a000037];

pub fn build_index(key: &str) -> Option<usize> {
    KEYS.iter().position(|k| *k == key)
}

pub const pGameObj: u32 = 0x10000;
pub const pMiscFunc: u32 = 0x10004;
pub const pGlobalDataDome: u32 = 0x10008;

#[inline]
fn G(e: &Engine, a: u32) -> Ptr {
    e.rt.cur_data() + (a - 0x10000)
}

pub fn null_platform(e: &mut Engine, _: u32) -> LResult<u64> {
    APIgameobj::KillObject(e, SELF);
    Ok(0)
}
pub fn null_main(e: &mut Engine, _: u32) -> LResult<u64> {
    let v = e.alloca(0x10);
    let o = FOLLOW[e.rt.cur_build()];
    let p = APIgameobj::GetPosition(e, o, 0x7fff_ffff, v);
    APIgameobj::SetTranslation(e, SELF, p);
    Ok(0)
}
pub fn domeConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    if instanceCount == 1 {
        APIcons::SetExternalFunction(e, 0, null_platform);
        APIcons::SetExternalFunction(e, 1, null_main);
        let v = APIcons::GetApiGameObj(e);
        let a = G(e, pGameObj);
        e.w32(a, v);
        let s = e.cstr("MiscFunc");
        let v = APIcons::GetApiCustom(e, s);
        let a = G(e, pMiscFunc);
        e.w32(a, v);
        let slot = G(e, pGlobalDataDome);
        APIcons::GetGlobalData(e, slot, 4);
        let b = APIgameobj::GetGameBuffer(e, 0, 0x8f8, 1);
        let gd = e.r32(slot);
        e.w32(gd, b);
    }
    LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 2, NULL, 1);
    APIcons::SetInitialState(e, 0);
    Ok(())
}
pub fn domeDestructor(e: &mut Engine, _: i32) -> LResult<()> {
    Ok(())
}

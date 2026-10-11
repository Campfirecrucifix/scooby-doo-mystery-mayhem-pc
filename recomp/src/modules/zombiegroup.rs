//! `zombiegroup`: static animated zombie group prop (two clipping set-ups, then sleeps forever). Port of
//! `Globals/Objects/ZombieGroup/zombiegroup` (3 functions, 1 build).
#![allow(non_snake_case, non_upper_case_globals, unused_variables)]

use crate::engine::*;
use crate::logic::{LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "zombiegroup",
    data_size: 0xc,
    ctor: zombiegroupConstructor,
    dtor: zombiegroupDestructor,
    register: None,
    init: &[],
    build_init: None,
};

pub const pGameObj: u32 = 0x10000;
pub const pAnim: u32 = 0x10004;
pub const pMiscFunc: u32 = 0x10008;

#[inline]
fn G(e: &Engine, a: u32) -> Ptr {
    e.rt.cur_data() + (a - 0x10000)
}

/// 0x00010cd0 state 0
pub fn null_main(e: &mut Engine, _: u32) -> LResult<u64> {
    let c = e.alloca(0x44);
    APIanim::Play(e, SELF, 1, 2, 0.0);
    APIgameobj::GetClippingParams(e, SELF, c);
    e.w32(c + 4, 0x4220_0000);
    e.w32(c + 0xc, 0x680);
    e.w32(c + 8, 0x6a7);
    APIgameobj::SetClippingParams(e, SELF, c, 0);
    APIgameobj::GetClippingParams(e, SELF, c);
    e.w32(c + 4, 0xbf80_0000);
    e.w32(c + 0xc, 0x6a0);
    e.w32(c + 8, 0x6a7);
    APIgameobj::SetClippingParams(e, SELF, c, 0);
    APIanim::Play(e, SELF, 1, 2, 0.2);
    APIgameobj::Sleep(e, SELF, -1.0);
    Ok(0)
}

/// 0x00010e38
pub fn zombiegroupConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    if instanceCount == 1 {
        APIcons::SetExternalFunction(e, 0, null_main);
        let v = APIcons::GetApiAnim(e);
        let a = G(e, pAnim);
        e.w32(a, v);
        let v = APIcons::GetApiGameObj(e);
        let a = G(e, pGameObj);
        e.w32(a, v);
        let n = e.cstr("MiscFunc");
        let v = APIcons::GetApiCustom(e, n);
        let a = G(e, pMiscFunc);
        e.w32(a, v);
    }
    // (*(pMiscFunc + 4)) = slot 1 MiscFunc_ChangeClipping
    LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 2, NULL, 1);
    APIcons::SetInitialState(e, 0);
    Ok(())
}

/// 0x00010f28
pub fn zombiegroupDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    Ok(())
}

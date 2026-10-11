//! `roomlocator`: invisible room-locator marker (group 0x13); sets its clipping and sleeps.
//! Msg 0xd5 only queries its own id (result unused).
//! Port of `Game Logic Modules/Globals/Objects/inv_RoomLocator/roomlocator__aa8a61d4/RoomLocator.c`
//! (5 functions). EP1L01 build aa8a61d4.
#![allow(non_snake_case, non_upper_case_globals, unused_variables)]

use crate::engine::*;
use crate::logic::{LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "roomlocator",
    // globals 0x10000..0x1000c (+ unused gpoLanguagesNamesAndISOCodes)
    data_size: 0xc,
    ctor: roomlocatorConstructor,
    dtor: roomlocatorDestructor,
    register: None,
    init: &[],
    build_init: None,
};

pub fn build_index(key: &str) -> Option<usize> {
    match key {
        "aa8a61d4" => Some(0),
        "fde7a04f" => Some(1),
        "8594eedb" => Some(2),
        _ => None,
    }
}

pub const pGameObj: u32 = 0x10000;
pub const pMisc: u32 = 0x10004;
pub const pGlobalDataRoomLocator: u32 = 0x10008;

#[inline]
fn G(e: &Engine, a: u32) -> Ptr {
    e.rt.cur_data() + (a - 0x10000)
}

/// 0x00010cd0 state 0
pub fn fstate_FIRSTMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    APIgameobj::SetGroupID(e, SELF, 0x13);
    APIgameobj::SetNextState(e, SELF, 1);
    Ok(0)
}

/// 0x00010d40 state 1
pub fn fstate_BASICMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 4, NULL, 0);
    APIgameobj::Sleep(e, SELF, -1.0);
    Ok(0)
}

/// 0x00010dc0 (aa8a61d4: the GetID result goes to a dead stack slot; fde7a04f / 8594eedb: if the
/// id is 0x2a0000b8 / 0x2a0000bb, set `*pGlobalDataRoomLocator + 0x24` = 1)
pub fn MyMessageFunction(e: &mut Engine, m: u32, _s: u32, _d: Ptr) -> LResult<u64> {
    if m == 0xd5 {
        let id = APIgameobj::GetID(e, SELF, NULL);
        let want = [0, 0x2a0000b8u32, 0x2a0000bb][e.rt.cur_build()];
        if want != 0 && id as u32 == want {
            let g = e.r32(e.rt.cur_data() + (pGlobalDataRoomLocator - 0x10000));
            e.w32(g + 0x24, 1);
        }
    }
    Ok(0)
}

/// 0x00010e48
pub fn roomlocatorConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    if instanceCount == 1 {
        APIcons::SetExternalFunction(e, 0, fstate_FIRSTMOVE);
        APIcons::SetExternalFunction(e, 1, fstate_BASICMOVE);
        let v = APIcons::GetApiGameObj(e);
        let a = G(e, pGameObj);
        e.w32(a, v);
        let n = e.cstr("MiscFunc");
        let v = APIcons::GetApiCustom(e, n);
        let a = G(e, pMisc);
        e.w32(a, v);
        let slot = G(e, pGlobalDataRoomLocator);
        APIcons::GetGlobalData(e, slot, 4);
        let b = APIgameobj::GetGameBuffer(e, 0, 0x8f8, 1);
        let gd = e.r32(G(e, pGlobalDataRoomLocator));
        e.w32(gd, b);
    }
    APIcons::SetMessageFunction(e, MyMessageFunction);
    LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 2, NULL, 0);
    APIcons::SetInitialState(e, 0);
    Ok(())
}

/// 0x00010f98
pub fn roomlocatorDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    if instanceCount == 0 {
        APIgameobj::FreeGameBuffer(e, 0);
    }
    Ok(())
}

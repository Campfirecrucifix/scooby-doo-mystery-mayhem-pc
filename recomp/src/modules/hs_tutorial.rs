//! `hs_tutorial`: EP1L01 tutorial hot spots (0x2a000021..25). Touching one (collide notify,
//! ext fn 3) while GB0+0x14 & 0xe20090c is clear sends the tutorial message to 0x2a000020:
//! 0x21 -> 0x100, 0x22 -> 0x108(1) + group 6, 0x23 -> 0x109, 0x24 -> 0x10d, 0x25 -> 0xb3.
//! Msg 0xc1 (reset, unless 0xd4 latched L+4) -> 0x108(0), group 0, re-arm; 0x96 -> L+0 = 1, re-arm.
//! Port of `Game Logic Modules/Ep1_Library/Objects/HOTSPOTS/Level_01/hs_tutorial/hs_tutorial.c`
//! (8 functions). EP1L01 build fb0ddd6c.
#![allow(non_snake_case, non_upper_case_globals, unused_variables)]

use crate::engine::*;
use crate::logic::{LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "hs_tutorial",
    // globals 0x10000..0x10014 (+ unused gpoLanguagesNamesAndISOCodes)
    data_size: 0x14,
    ctor: hs_tutorialConstructor,
    dtor: hs_tutorialDestructor,
    register: None,
    init: &[],
    build_init: None,
};

pub fn build_index(key: &str) -> Option<usize> {
    (key == "fb0ddd6c").then_some(0)
}

pub const pExtra: u32 = 0x10000;
pub const pGameObj: u32 = 0x10004;
pub const pPhysic: u32 = 0x10008;
pub const pLocalDatahs_tutorial: u32 = 0x1000c;
pub const pGlobalDatahs_tutorial: u32 = 0x10010;

#[inline]
fn G(e: &Engine, a: u32) -> Ptr {
    e.rt.cur_data() + (a - 0x10000)
}
/// pLocalDatahs_tutorial (0x48 bytes: +0 0x96 seen, +4 0xd4 latch, +8 EFClippingParams)
#[inline]
fn L(e: &Engine) -> Ptr {
    e.r32(G(e, pLocalDatahs_tutorial))
}
/// *pGlobalDatahs_tutorial: game buffer 0
#[inline]
fn GB0(e: &Engine) -> Ptr {
    e.r32(e.r32(G(e, pGlobalDatahs_tutorial)))
}

/// 0x00010ce8 state 0
pub fn fstate_FIRSTMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    APIphysic::CreatePhysic(e, SELF, 0);
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 3, 3);
    APIgameobj::SetNextState(e, SELF, 1);
    Ok(0)
}

/// 0x00010d80
pub fn fstate_BASICMOVE_SET(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

/// 0x00010db0
pub fn fstate_BASICMOVE_LOOP(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

/// 0x00010de0
pub fn fcback_NOTIFY_COLLIDE_OBJ(e: &mut Engine, _: u32) -> LResult<u64> {
    let id = APIgameobj::GetID(e, SELF, NULL);
    if e.r32(GB0(e) + 0x14) & 0x0e20_090c != 0 {
        return Ok(0);
    }
    let (m, p) = match id {
        0x2a00_0021 => (0x100, 0),
        0x2a00_0022 => (0x108, 1),
        0x2a00_0023 => (0x109, 0),
        0x2a00_0024 => (0x10d, 0),
        0x2a00_0025 => (0xb3, 0),
        _ => return Ok(0),
    };
    APIgameobj::SendMessageTo(e, 0x2a00_0020, NULL, m, p, NULL);
    if id == 0x2a00_0022 {
        APIgameobj::SetGroupID(e, SELF, 6);
    }
    Ok(1)
}

/// 0x00010fe0
pub fn MyMessageFunction(e: &mut Engine, m: u32, _s: u32, _d: Ptr) -> LResult<u64> {
    let l = L(e);
    match m {
        0xc1 => {
            if e.r32(l + 4) == 0 {
                APIgameobj::SendMessageTo(e, 0x2a00_0020, NULL, 0x108, 0, NULL);
                APIgameobj::SetGroupID(e, SELF, 0);
                APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 3, 3);
            }
        }
        0x96 => {
            e.w32(l, 1);
            APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 3, 3);
            return Ok(1);
        }
        0xd4 => e.w32(l + 4, 1),
        _ => {}
    }
    Ok(0)
}

/// 0x00011178
pub fn Hotspot_TutorialFUNC_SetClipping(e: &mut Engine) {
    let c = L(e) + 8;
    APIgameobj::GetClippingParams(e, SELF, c);
    for (o, v) in [(0, 0x3dcc_cccd), (4, 0x4160_0000), (0x10, 0x41c8_0000), (0x1c, 0xbf80_0000), (0xc, 0), (8, 0), (0x18, 0x80), (0x14, 0x6a7), (0x24, 0x6a0), (0x20, 0x6a7)] {
        e.w32(c + o, v);
    }
    APIgameobj::SetClippingParams(e, SELF, c, 0);
}

/// 0x000112b0
pub fn hs_tutorialConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    if instanceCount == 1 {
        APIcons::SetExternalFunction(e, 0, fstate_FIRSTMOVE);
        APIcons::SetExternalFunction(e, 1, fstate_BASICMOVE_SET);
        APIcons::SetExternalFunction(e, 2, fstate_BASICMOVE_LOOP);
        APIcons::SetExternalFunction(e, 3, fcback_NOTIFY_COLLIDE_OBJ);
        let tables: [(u32, fn(&mut Engine) -> Ptr); 3] = [
            (pGameObj, |e| APIcons::GetApiGameObj(e)),
            (pExtra, |e| APIcons::GetApiExtra(e)),
            (pPhysic, |e| APIcons::GetApiPhysic(e)),
        ];
        for (slot, get) in tables {
            let v = get(e);
            let a = G(e, slot);
            e.w32(a, v);
        }
        let slot = G(e, pGlobalDatahs_tutorial);
        APIcons::GetGlobalData(e, slot, 8);
        let gd = e.r32(slot);
        let b = APIgameobj::GetGameBuffer(e, 0, 0x8f8, 1);
        e.w32(gd, b);
        let b = APIgameobj::GetGameBuffer(e, 1, 0x68, 1);
        e.w32(gd + 4, b);
    }
    APIcons::SetMessageFunction(e, MyMessageFunction);
    let slot = G(e, pLocalDatahs_tutorial);
    APIcons::GetLocalData(e, slot, 0x48);
    let l = L(e);
    e.w32(l + 4, 0);
    Hotspot_TutorialFUNC_SetClipping(e);
    APIcons::SetInitialState(e, 0);
    Ok(())
}

/// 0x00011488
pub fn hs_tutorialDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    Ok(())
}

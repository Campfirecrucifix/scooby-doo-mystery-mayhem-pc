//! `pagecs`: EP1L03A tome-page cut-scene prop. Visible (ChangeClipping mode 2) while cut-scene 1
//! plays, hidden (mode 4) when it ends; otherwise idle. Port of
//! `Game Logic Modules/Ep1_Library/Objects/TodBeamCS/pagecs/PAGECS.c` (5 functions). Build 191622a8.
#![allow(non_snake_case, non_upper_case_globals, unused_variables)]

use crate::engine::*;
use crate::logic::{LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "pagecs",
    // globals 0x10000..0x10008 (+ unused gpoLanguagesNamesAndISOCodes)
    data_size: 0x8,
    ctor: pagecsConstructor,
    dtor: pagecsDestructor,
    register: None,
    init: &[],
    build_init: None,
};

pub fn build_index(key: &str) -> Option<usize> {
    (key == "191622a8").then_some(0)
}

pub const pMiscFunc: u32 = 0x10000;
pub const pGameObj: u32 = 0x10004;

#[inline]
fn G(e: &Engine, a: u32) -> Ptr {
    e.rt.cur_data() + (a - 0x10000)
}

/// 0x00010cc8 state 0
pub fn PAGECS_ST_FIRSTMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    APIgameobj::SetNextState(e, SELF, 1);
    Ok(0)
}

/// 0x00010d20 state 1
pub fn PAGECS_ST_BASICMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

/// 0x00010d50 callback 2: data {scene, 1 = start / 0 = end}; MiscFunc slot 1 = ChangeClipping
pub fn PAGECS_CB_CUTSCENE(e: &mut Engine, p: u32) -> LResult<u64> {
    let (scene, on) = (e.r32(p), e.r32(p + 4));
    if on == 1 && scene == 1 {
        LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 2, NULL, 0);
        APIgameobj::SetNextState(e, SELF, 1);
    } else if on == 0 && scene == 1 {
        LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 4, NULL, 0);
        APIgameobj::SetNextState(e, SELF, 1);
    }
    Ok(0)
}

/// 0x00010e68
pub fn pagecsConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    if instanceCount == 1 {
        APIcons::SetExternalFunction(e, 0, PAGECS_ST_FIRSTMOVE);
        APIcons::SetExternalFunction(e, 1, PAGECS_ST_BASICMOVE);
        APIcons::SetExternalFunction(e, 2, PAGECS_CB_CUTSCENE);
        let v = APIcons::GetApiGameObj(e);
        let a = G(e, pGameObj);
        e.w32(a, v);
        let n = e.cstr("MiscFunc");
        let v = APIcons::GetApiCustom(e, n);
        let a = G(e, pMiscFunc);
        e.w32(a, v);
    }
    LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 2, NULL, 0);
    APIcons::SetInitialState(e, 0);
    Ok(())
}

/// 0x00010f78
pub fn pagecsDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    Ok(())
}

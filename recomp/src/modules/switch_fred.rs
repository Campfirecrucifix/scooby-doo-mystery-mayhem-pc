//! `switch_fred`: decorative Fred switch (plays anim 1, clipping). Port of
//! `Ep2_MovieStudio/Objects/Switch_Fred/switch_fred` (6 functions, 1 build).
#![allow(non_snake_case, non_upper_case_globals, unused_variables)]

use crate::engine::*;
use crate::logic::{LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "switch_fred",
    data_size: 0x24,
    ctor: switch_fredConstructor,
    dtor: switch_fredDestructor,
    register: None,
    init: &[],
    build_init: None,
};

pub const pExtra: u32 = 0x10000;
pub const pGameObj: u32 = 0x10004;
pub const pPhysic: u32 = 0x10008;
pub const pAnim: u32 = 0x1000c;
pub const pDebug: u32 = 0x10010;
pub const pAnimMngr: u32 = 0x10014;
pub const pMiscFunc: u32 = 0x10018;
pub const pLocalDataSwitch_Fred: u32 = 0x1001c;
pub const pGlobalDataSwitch_Fred: u32 = 0x10020;

#[inline]
fn G(e: &Engine, a: u32) -> Ptr {
    e.rt.cur_data() + (a - 0x10000)
}

/// 0x00010d00 state 0
pub fn Switch_FredST_FIRSTMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    let v = LIBAnimMngr::PlayAnim(e, 0.0, 1, 1, 0);
    let l = e.r32(G(e, pLocalDataSwitch_Fred));
    e.w32(l, v as u32);
    LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 4, NULL, 1);
    APIgameobj::SetNextState(e, SELF, 2);
    Ok(0)
}

/// 0x00010da8 state 2
pub fn Switch_FredST_BASICMOVE_LOOP(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

/// 0x00010dd8 callback 5
pub fn Switch_FredCB_CUTSCENE(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

/// 0x00010e08
pub fn Switch_FredFUNC_MessageHandler(e: &mut Engine, _: u32, _: u32, _: Ptr) -> LResult<u64> {
    Ok(0)
}

/// 0x00010e40
pub fn switch_fredConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    if instanceCount == 1 {
        APIcons::SetExternalFunction(e, 0, Switch_FredST_FIRSTMOVE);
        APIcons::SetExternalFunction(e, 2, Switch_FredST_BASICMOVE_LOOP);
        APIcons::SetExternalFunction(e, 5, Switch_FredCB_CUTSCENE);
        let tables: [(u32, fn(&mut Engine) -> Ptr); 4] = [
            (pAnim, |e| APIcons::GetApiAnim(e)),
            (pGameObj, |e| APIcons::GetApiGameObj(e)),
            (pExtra, |e| APIcons::GetApiExtra(e)),
            (pPhysic, |e| APIcons::GetApiPhysic(e)),
        ];
        for (slot, get) in tables {
            let v = get(e);
            let a = G(e, slot);
            e.w32(a, v);
        }
        for (slot, name) in [(pDebug, "debug"), (pAnimMngr, "AnimMngr"), (pMiscFunc, "MiscFunc")] {
            let n = e.cstr(name);
            let v = APIcons::GetApiCustom(e, n);
            let a = G(e, slot);
            e.w32(a, v);
        }
        let slot = G(e, pGlobalDataSwitch_Fred);
        APIcons::GetGlobalData(e, slot, 4);
        let b = APIgameobj::GetGameBuffer(e, 0, 0x8f8, 1);
        let g = e.r32(G(e, pGlobalDataSwitch_Fred));
        e.w32(g, b);
    }
    APIcons::SetMessageFunction(e, Switch_FredFUNC_MessageHandler);
    let slot = G(e, pLocalDataSwitch_Fred);
    APIcons::GetLocalData(e, slot, 4);
    LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 2, NULL, 1);
    APIcons::SetInitialState(e, 0);
    Ok(())
}

/// 0x00011050
pub fn switch_fredDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    Ok(())
}

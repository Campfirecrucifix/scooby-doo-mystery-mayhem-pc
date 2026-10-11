//! `planefog`: EP1L03A fog plane. Transparent priority 0x20 at start, then spins about Y by
//! 0.0052359877 rad (0.3 deg) per frame (state 1 = BASICMOVE_SET runs every frame; it never
//! switches to state 2). Port of `Globals/Objects/PlaneFog/planefog/planefog.c` (8 functions).
//! Build 8eea67b4.
#![allow(non_snake_case, non_upper_case_globals, unused_variables)]

use crate::engine::*;
use crate::logic::{LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "planefog",
    // globals 0x10000..0x10020 (+ unused gpoLanguagesNamesAndISOCodes)
    data_size: 0x20,
    ctor: planefogConstructor,
    dtor: planefogDestructor,
    register: None,
    init: &[],
    build_init: None,
};

pub fn build_index(key: &str) -> Option<usize> {
    (key == "8eea67b4").then_some(0)
}

pub const pExtra: u32 = 0x10000;
pub const pGameObj: u32 = 0x10004;
pub const pPhysic: u32 = 0x10008;
pub const pAnim: u32 = 0x1000c;
pub const pAnimMngr: u32 = 0x10010;
pub const pMisc: u32 = 0x10014;
pub const pLocalDataplanefog: u32 = 0x10018;
pub const pGlobalDataplanefog: u32 = 0x1001c;

#[inline]
fn G(e: &Engine, a: u32) -> Ptr {
    e.rt.cur_data() + (a - 0x10000)
}

/// 0x00010cf0 state 0
pub fn planefogST_FIRSTMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    APIgameobj::SetTransparentPriority(e, SELF, 0, 0x20);
    APIgameobj::SetNextState(e, SELF, 1);
    Ok(0)
}

/// 0x00010d68 state 1
pub fn planefogST_BASICMOVE_SET(e: &mut Engine, _: u32) -> LResult<u64> {
    let v = e.alloca(0xc);
    e.wf(v, 0.0);
    e.wf(v + 4, 1.0);
    e.wf(v + 8, 0.0);
    let _id = APIgameobj::GetID(e, SELF, NULL);
    APIgameobj::Rotate(e, SELF, v, f32::from_bits(0x3bab_92a6)); // 0.0052359877
    Ok(0)
}

/// 0x00010e10 state 2
pub fn planefogST_BASICMOVE_LOOP(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

/// 0x00010e40 callback 3
pub fn planefogCB_NOTIFY_COLLIDE_OBJ(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

/// 0x00010e90 callback 4
pub fn planefogCB_ANIM_LF(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(1)
}

/// 0x00010ef0
pub fn planefogFUNC_MessageHandler(e: &mut Engine, _m: u32, _s: u32, _d: Ptr) -> LResult<u64> {
    Ok(0)
}

/// 0x00010f28
pub fn planefogConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    if instanceCount == 1 {
        APIcons::SetExternalFunction(e, 0, planefogST_FIRSTMOVE);
        APIcons::SetExternalFunction(e, 1, planefogST_BASICMOVE_SET);
        APIcons::SetExternalFunction(e, 2, planefogST_BASICMOVE_LOOP);
        APIcons::SetExternalFunction(e, 3, planefogCB_NOTIFY_COLLIDE_OBJ);
        APIcons::SetExternalFunction(e, 4, planefogCB_ANIM_LF);
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
        for (slot, name) in [(pAnimMngr, "AnimMngr"), (pMisc, "MiscFunc")] {
            let n = e.cstr(name);
            let v = APIcons::GetApiCustom(e, n);
            let a = G(e, slot);
            e.w32(a, v);
        }
        let slot = G(e, pGlobalDataplanefog);
        APIcons::GetGlobalData(e, slot, 4);
        let b = APIgameobj::GetGameBuffer(e, 0, 0x8f8, 1);
        let gd = e.r32(G(e, pGlobalDataplanefog));
        e.w32(gd, b);
    }
    APIcons::SetMessageFunction(e, planefogFUNC_MessageHandler);
    let slot = G(e, pLocalDataplanefog);
    APIcons::GetLocalData(e, slot, 4);
    APIcons::SetInitialState(e, 0);
    Ok(())
}

/// 0x00011130
pub fn planefogDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    Ok(())
}

//! `animated_water`: looping water anim, nudged 5 cm down, clipping overridden, enabled / disabled by the
//! sector manager (0x201/0x202; 0x203 registers {id, sector, 1} at local +0x40). Both builds are the
//! same code. Port of `Game Logic Modules/Ep4_Bayou/Objects/ANIMATED_WATER/animated_water__*/Animated_Water.c`.
#![allow(non_snake_case, non_upper_case_globals, unused_variables)]

use crate::engine::*;
use crate::logic::{LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "animated_water",
    data_size: 0x28,
    ctor: animated_waterConstructor,
    dtor: animated_waterDestructor,
    register: None,
    init: &[],
    build_init: None,
};

pub fn build_index(key: &str) -> Option<usize> {
    ["ac3d4ef2", "f3abef0d"].iter().position(|k| *k == key)
}

pub const pExtra: u32 = 0x10000;
pub const pGameObj: u32 = 0x10004;
pub const pPhysic: u32 = 0x10008;
pub const pAnim: u32 = 0x1000c;
pub const pAnimMngr: u32 = 0x10010;
pub const pFunctions: u32 = 0x10014;
pub const pMiscFunc: u32 = 0x10018;
pub const pSectorClip: u32 = 0x1001c;
pub const pLocalDataAnimated_Water: u32 = 0x10020;
pub const pGlobalDataAnimated_Water: u32 = 0x10024;

#[inline]
fn G(e: &Engine, a: u32) -> Ptr {
    e.rt.cur_data() + (a - 0x10000)
}
fn L(e: &Engine) -> Ptr {
    e.r32(G(e, pLocalDataAnimated_Water))
}

pub fn Animated_WaterST_FIRSTMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    SetClipping(e);
    APIanim::Play(e, SELF, 1, 2, 0.0);
    APIgameobj::SetNextState(e, SELF, 1);
    Ok(0)
}
pub fn Animated_WaterST_BASICMOVE_LOOP(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}
/// the clipping params live at local +0
fn SetClipping(e: &mut Engine) {
    let l = L(e);
    APIgameobj::GetClippingParams(e, SELF, l);
    for (o, v) in [(0, 0x3dcc_cccd), (4, 0xbf80_0000), (0xc, 0), (8, 0)] {
        e.w32(l + o, v);
    }
    APIgameobj::SetClippingParams(e, SELF, l, 1);
}
pub fn Animated_WaterFUNC_MessageHandler(e: &mut Engine, m: u32, _: u32, p3: Ptr) -> LResult<u64> {
    match m {
        0x202 => {
            APIgameobj::EnableObject(e, SELF, 0);
        }
        0x201 => {
            APIgameobj::EnableObject(e, SELF, 1);
        }
        0x203 => {
            let l = L(e);
            let id = APIgameobj::GetID(e, SELF, NULL);
            e.w32(l + 0x40, id);
            e.w16(l + 0x4a, 1);
            let s = e.r16(p3);
            e.w16(l + 0x48, s);
            LIBApiSectorClipping::ApiImpl_Register(e, l + 0x40);
        }
        _ => {}
    }
    Ok(0)
}
pub fn animated_waterConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    if instanceCount == 1 {
        APIcons::SetExternalFunction(e, 0, Animated_WaterST_FIRSTMOVE);
        APIcons::SetExternalFunction(e, 1, Animated_WaterST_BASICMOVE_LOOP);
        for (a, v) in [
            (pAnim, APIcons::GetApiAnim(e)),
            (pGameObj, APIcons::GetApiGameObj(e)),
            (pExtra, APIcons::GetApiExtra(e)),
            (pPhysic, APIcons::GetApiPhysic(e)),
        ] {
            let a = G(e, a);
            e.w32(a, v);
        }
        for (a, n) in [(pAnimMngr, "AnimMngr"), (pFunctions, "ApiCFunctions"), (pMiscFunc, "MiscFunc"), (pSectorClip, "ApiSectorClipping")] {
            let s = e.cstr(n);
            let v = APIcons::GetApiCustom(e, s);
            let a = G(e, a);
            e.w32(a, v);
        }
        let slot = G(e, pGlobalDataAnimated_Water);
        APIcons::GetGlobalData(e, slot, 4);
        let b = APIgameobj::GetGameBuffer(e, 0, 0x8f8, 1);
        let gd = e.r32(slot);
        e.w32(gd, b);
    }
    APIcons::SetMessageFunction(e, Animated_WaterFUNC_MessageHandler);
    let slot = G(e, pLocalDataAnimated_Water);
    APIcons::GetLocalData(e, slot, 0x4c);
    APIgameobj::SetTransparentPriority(e, SELF, 0, 0x1e);
    let v = e.alloca(0x10);
    APIgameobj::GetPosition(e, SELF, 0x7fff_ffff, v);
    let y = e.rf(v + 4) - 0.05;
    e.wf(v + 4, y);
    APIgameobj::SetTranslation(e, SELF, v);
    SetClipping(e);
    APIcons::SetInitialState(e, 0);
    Ok(())
}
pub fn animated_waterDestructor(e: &mut Engine, _: i32) -> LResult<()> {
    Ok(())
}

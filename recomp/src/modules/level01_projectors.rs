//! `level01_projectors`: the EP1L01 window projector ("lyb_fenetre_pro", 1x2 m, reach 6.25)
//! attached to the object frame; msg 0x98 toggles it (param 2 = visible, else hidden).
//! Port of `Game Logic Modules/Ep1_Library/Objects/LEVEL01_PROJECTORS/level01_projectors/level01_projectors.c`
//! (8 functions). EP1L01 build 7db649d8.
#![allow(non_snake_case, non_upper_case_globals, unused_variables)]

use crate::engine::*;
use crate::logic::{LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "level01_projectors",
    // globals 0x10000..0x10020 (+ unused gpoLanguagesNamesAndISOCodes)
    data_size: 0x20,
    ctor: level01_projectorsConstructor,
    dtor: level01_projectorsDestructor,
    register: None,
    init: &[],
    build_init: None,
};

pub fn build_index(key: &str) -> Option<usize> {
    (key == "7db649d8").then_some(0)
}

pub const pAnimMngr: u32 = 0x10000;
pub const pExtra: u32 = 0x10004;
pub const pGameObj: u32 = 0x10008;
pub const pPhysic: u32 = 0x1000c;
pub const pAnim: u32 = 0x10010;
pub const pProjector: u32 = 0x10014;
pub const pLocalDatalevel01_projectors: u32 = 0x10018;
pub const pGlobalDatalevel01_projectors: u32 = 0x1001c;

#[inline]
fn G(e: &Engine, a: u32) -> Ptr {
    e.rt.cur_data() + (a - 0x10000)
}
/// pLocalDatalevel01_projectors (8 bytes: +4 projector handle)
#[inline]
fn proj(e: &Engine) -> u32 {
    e.r32(e.r32(G(e, pLocalDatalevel01_projectors)) + 4)
}

/// 0x00010cf0 state 0
pub fn fstate_FIRSTMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    APIgameobj::SetNextState(e, SELF, 1);
    Ok(0)
}

/// 0x00010d48
pub fn fstate_BASICMOVE_SET(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

/// 0x00010d78
pub fn fstate_BASICMOVE_LOOP(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

/// 0x00010da8
pub fn fcback_NOTIFY_COLLIDE_OBJ(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

/// 0x00010df8
pub fn MyMessageFunction(e: &mut Engine, m: u32, p: u32, _d: Ptr) -> LResult<u64> {
    if m == 0x98 {
        let h = proj(e);
        APIprojector::SetVisible(e, h, (p == 2) as u32);
    }
    Ok(0)
}

/// 0x00010ed0
pub fn CreateProjectors(e: &mut Engine) -> u32 {
    let h = APIprojector::CreateProjector(e);
    let l = e.r32(G(e, pLocalDatalevel01_projectors));
    e.w32(l + 4, h);
    APIgameobj::AddChild(e, SELF, 0x7fff_ffff, h, 1);
    let t = e.cstr("lyb_fenetre_pro");
    APIprojector::SetTexture(e, h, t);
    APIprojector::SetColor(e, h, 1.0, 1.0, 1.0, 1.0);
    APIprojector::SetProjectionType(e, h, 0);
    APIprojector::SetReachDistance(e, h, 6.25);
    APIprojector::SetSize(e, h, 1.0, 2.0);
    APIprojector::SetBlendingFunction(e, h, 0);
    APIprojector::SetVisible(e, h, 1)
}

/// 0x00011080
pub fn level01_projectorsConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    if instanceCount == 1 {
        APIcons::SetExternalFunction(e, 0, fstate_FIRSTMOVE);
        APIcons::SetExternalFunction(e, 1, fstate_BASICMOVE_SET);
        APIcons::SetExternalFunction(e, 2, fstate_BASICMOVE_LOOP);
        APIcons::SetExternalFunction(e, 3, fcback_NOTIFY_COLLIDE_OBJ);
        let tables: [(u32, fn(&mut Engine) -> Ptr); 5] = [
            (pAnim, |e| APIcons::GetApiAnim(e)),
            (pGameObj, |e| APIcons::GetApiGameObj(e)),
            (pExtra, |e| APIcons::GetApiExtra(e)),
            (pPhysic, |e| APIcons::GetApiPhysic(e)),
            (pProjector, |e| APIcons::GetApiProjector(e)),
        ];
        for (slot, get) in tables {
            let v = get(e);
            let a = G(e, slot);
            e.w32(a, v);
        }
        let n = e.cstr("AnimMngr");
        let v = APIcons::GetApiCustom(e, n);
        let a = G(e, pAnimMngr);
        e.w32(a, v);
        let slot = G(e, pGlobalDatalevel01_projectors);
        APIcons::GetGlobalData(e, slot, 4);
        let gd = e.r32(slot);
        let b = APIgameobj::GetGameBuffer(e, 0, 0x8f8, 1);
        e.w32(gd, b);
    }
    APIcons::SetMessageFunction(e, MyMessageFunction);
    let slot = G(e, pLocalDatalevel01_projectors);
    APIcons::GetLocalData(e, slot, 8);
    CreateProjectors(e);
    APIcons::SetInitialState(e, 0);
    Ok(())
}

/// 0x00011270
pub fn level01_projectorsDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    let h = proj(e);
    APIprojector::DestroyProjector(e, h);
    Ok(())
}

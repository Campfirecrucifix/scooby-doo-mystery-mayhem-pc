//! `herospot`: the hero arrow in the middle of the radar. Msg 0x1a gives its HUD window, 0xe9 the hero
//! to follow (window placed at 0.4,0.42); each frame the arrow (object 0xf00066c) is rotated to the
//! hero's look-at direction seen from the camera.
//! Port of `Game Logic Modules/Globals/Panel/Radar/herospot__90ca0cbd/herospot.c` (7 functions).
//! One build matched (EP1L01, sha 90ca0cbd).
#![allow(non_snake_case, non_upper_case_globals, unused_variables)]

use crate::engine::*;
use crate::logic::{LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "herospot",
    data_size: 0x28,
    ctor: herospotConstructor,
    dtor: herospotDestructor,
    register: None,
    init: &[],
    build_init: None,
};

/// (build sha8, camera object, arrow object). All 37 builds share the source; only these ids differ (WP-L8).
pub static BUILDS: Table<(&'static str, u32, u32)> = Table::new(&BUILDS_K, builds_row);
fn builds_row(k: &TRow) -> (&'static str, u32, u32) {
    (k.0.builds[k.1], k.v(0), k.v(1))
}
static BUILDS_K: TSpec = TSpec {
    builds: &["043f09fc", "1279df19", "15806ed5", "26755258", "26bb84da", "2ba63f7e", "353d5815", "35e12ee0", "56e5d505", "76bbfb39", "77ca9baf", "78143773", "8873a911", "8f8a8bd4", "90ca0cbd", "99ca93e4", "9a3cf822", "9dfd22dd", "a02bdc05", "a6217532", "ab28c46e", "ae6c1459", "af4a33e6", "b37388ef", "b501c2fa", "b51f25f4", "b53ff036", "b6bc4a6f", "c04dd81f", "d40a29ce", "de0b4ed6", "df466fa3", "e5bf7e68", "e5eb9580", "f4f11235", "fc0885e1", "fd57d9bb"],
    fns: &["fstate_BASICMOVE_LOOP"],
    entries: &[
        TE::L(0, 3, 27, 1), TE::L(0, 3, 67, 1),
    ],
};

pub fn build_index(key: &str) -> Option<usize> {
    BUILDS.iter().position(|b| b.0 == key)
}

pub const pExtra: u32 = 0x10000;
pub const pGameObj: u32 = 0x10004;
pub const pPhysic: u32 = 0x10008;
pub const pAnim: u32 = 0x1000c;
pub const pHud: u32 = 0x10010;
pub const pMath: u32 = 0x10014;
pub const pMisc: u32 = 0x10018;
pub const pLocalDataHeroSpot: u32 = 0x1001c;
pub const pGlobalDataHeroSpot: u32 = 0x10020;

#[inline]
fn G(e: &Engine, a: u32) -> Ptr {
    e.rt.cur_data() + (a - 0x10000)
}
/// local data (8): +0 HUD window, +4 hero id
#[inline]
fn L(e: &Engine) -> Ptr {
    e.r32(G(e, pLocalDataHeroSpot))
}

/// shared body of the two rotating states; `first` = FIRSTMOVE, which (disasm-verified) feeds atan2f
/// the untransformed look-at (DStack_60) instead of the camera-space one.
fn rotate_arrow(e: &mut Engine, first: bool) {
    let p60 = e.alloca(0xc);
    let p50 = e.alloca(0xc);
    let axis = e.alloca(0xc);
    e.wf(axis, 0.0);
    e.wf(axis + 4, 0.0);
    e.wf(axis + 8, 1.0);
    let hero = e.r32(L(e) + 4);
    APIgameobj::GetLookAt(e, hero, 0x7fff_ffff, p60);
    let cam = BUILDS[e.rt.cur_build()].1;
    APIgameobj::InvertTransformVector(e, cam, p60, p50);
    let v = if first { p60 } else { p50 };
    let (z, x) = (e.rf(v + 8), e.rf(v));
    let a = APImath::atan2f(e, z, x);
    let q = a / 3.1415927;
    let ang = (q - (q as i32) as f32) * 3.1415927;
    let arrow = BUILDS[e.rt.cur_build()].2;
    APIgameobj::SetRotation(e, arrow, axis, ang);
}

/// 0x00010ce8 state 0
pub fn fstate_FIRSTMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    rotate_arrow(e, true);
    APIgameobj::SetNextState(e, SELF, 2);
    Ok(0)
}

/// 0x00010e50 state 1
pub fn fstate_BASICMOVE_SET(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

/// 0x00010e80 state 2
pub fn fstate_BASICMOVE_LOOP(e: &mut Engine, _: u32) -> LResult<u64> {
    rotate_arrow(e, false);
    Ok(0)
}

/// 0x00010fc8 external 3
pub fn fcback_NOTIFY_COLLIDE_OBJ(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

/// 0x00011018
pub fn MyMessageFunction(e: &mut Engine, m: u32, s: u32, _d: Ptr) -> LResult<u64> {
    if m == 0x1a {
        let l = L(e);
        e.w32(l, s);
    } else if m == 0xe9 {
        let w = e.r32(L(e));
        if w != 0 {
            APIhud::WindowSetPos(e, w, 0.4, 0.42);
        }
        LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 10, NULL, 0);
        let l = L(e);
        e.w32(l + 4, s);
    }
    Ok(0)
}

/// 0x00011128
pub fn herospotConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    if instanceCount == 1 {
        APIcons::SetExternalFunction(e, 0, fstate_FIRSTMOVE);
        APIcons::SetExternalFunction(e, 1, fstate_BASICMOVE_SET);
        APIcons::SetExternalFunction(e, 2, fstate_BASICMOVE_LOOP);
        APIcons::SetExternalFunction(e, 3, fcback_NOTIFY_COLLIDE_OBJ);
        let tables: [(u32, fn(&mut Engine) -> Ptr); 6] = [
            (pAnim, |e| APIcons::GetApiAnim(e)),
            (pGameObj, |e| APIcons::GetApiGameObj(e)),
            (pExtra, |e| APIcons::GetApiExtra(e)),
            (pPhysic, |e| APIcons::GetApiPhysic(e)),
            (pHud, |e| APIcons::GetApiHUD(e)),
            (pMath, |e| APIcons::GetApiMath(e)),
        ];
        for (slot, get) in tables {
            let v = get(e);
            let a = G(e, slot);
            e.w32(a, v);
        }
        let n = e.cstr("MiscFunc");
        let v = APIcons::GetApiCustom(e, n);
        let a = G(e, pMisc);
        e.w32(a, v);
        let slot = G(e, pGlobalDataHeroSpot);
        APIcons::GetGlobalData(e, slot, 4);
        let b = APIgameobj::GetGameBuffer(e, 0, 0x8f8, 1);
        let gd = e.r32(G(e, pGlobalDataHeroSpot));
        e.w32(gd, b);
    }
    APIcons::SetMessageFunction(e, MyMessageFunction);
    let slot = G(e, pLocalDataHeroSpot);
    APIcons::GetLocalData(e, slot, 8);
    LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 10, NULL, 0);
    APIcons::SetInitialState(e, 0);
    Ok(())
}

/// 0x00011348
pub fn herospotDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    Ok(())
}

#[allow(unused_imports)]
use crate::engine::gamedata::{KLoc, TRow, TSpec, Table, TE};

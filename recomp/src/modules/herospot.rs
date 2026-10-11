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
const BUILDS: [(&str, u32, u32); 37] = [
    ("043f09fc", 0x2a000027, 0xf00058d),
    ("1279df19", 0x2a000004, 0xf0004f7),
    ("15806ed5", 0x2a000006, 0xf0005e3),
    ("26755258", 0x2a00002a, 0xf000437),
    ("26bb84da", 0x2a00000e, 0xf000494),
    ("2ba63f7e", 0x2a000025, 0xf000a80),
    ("353d5815", 0x2a000004, 0xf000456),
    ("35e12ee0", 0x2a0000dd, 0xf000606),
    ("56e5d505", 0x2a000248, 0xf000501),
    ("76bbfb39", 0x2a000004, 0xf000454),
    ("77ca9baf", 0x2a000027, 0xf0004a9),
    ("78143773", 0x2a0000b3, 0xf0001ca),
    ("8873a911", 0x2a00001e, 0xf000657),
    ("8f8a8bd4", 0x2a000247, 0xf000506),
    ("90ca0cbd", 0x2a00008f, 0xf00066c),
    ("99ca93e4", 0x2a000004, 0xf000420),
    ("9a3cf822", 0x2a000045, 0xf000581),
    ("9dfd22dd", 0x2a00005c, 0xf00051b),
    ("a02bdc05", 0x2a000017, 0xf00061e),
    ("a6217532", 0x2a000006, 0xf000403),
    ("ab28c46e", 0x2a000037, 0xf000751),
    ("ae6c1459", 0x2a00000c, 0xf000483),
    ("af4a33e6", 0x2a000052, 0xf000616),
    ("b37388ef", 0x2a000004, 0xf00045b),
    ("b501c2fa", 0x2a000004, 0xf00037a),
    ("b51f25f4", 0x2a000004, 0xf0004c7),
    ("b53ff036", 0x2a00005c, 0xf000579),
    ("b6bc4a6f", 0x2a000004, 0xf0004c1),
    ("c04dd81f", 0x2a000020, 0xf00053e),
    ("d40a29ce", 0x2a000027, 0xf00040f),
    ("de0b4ed6", 0x2a000026, 0xf000550),
    ("df466fa3", 0x2a00000f, 0xf0004eb),
    ("e5bf7e68", 0x2a000012, 0xf000445),
    ("e5eb9580", 0x2a00004a, 0xf000618),
    ("f4f11235", 0x2a00001f, 0xf0005b3),
    ("fc0885e1", 0x2a000030, 0xf0005bc),
    ("fd57d9bb", 0x2a000004, 0xf000422),
];

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

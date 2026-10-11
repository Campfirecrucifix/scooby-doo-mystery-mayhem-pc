//! `coll_level3a`: EP1L03A collectables. Same Coll_base.c / Collectable.c / Init.c / coll_collect.c
//! as `coll_level1` (see [`super::coll_level1`], selected through its `Variant`), plus
//! Coll_level3A.c: two launchable pickups (0x2a000070, 0x2a000089) that are thrown out with an
//! impulse (msg 0xd3, from obj_launcher_level3a) and fall until they reach their spawn height, and a
//! cut-scene callback hiding the pickups during cut-scenes. Port of
//! `Game Logic Modules/Ep1_Library/COLLECTABLES/LEVEL03A/coll_level3a__b173298d/`.
#![allow(non_snake_case, non_upper_case_globals, unused_variables)]

use super::coll_level1::{self as base, write_tmp_map, Variant, G, L};
use crate::engine::*;
use crate::logic::{LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "coll_level3a",
    // globals 0x10000..0x104e4 (+ gpoLanguagesNamesAndISOCodes)
    data_size: 0x1d40,
    ctor: ctor_dispatch,
    dtor: dtor_dispatch,
    register: None,
    init: &[],
    build_init: Some(build_init_dispatch),
};

mod xb;

/// number of hand-ported builds (indices 0..N_OLD); generated builds (`xb`, c2rs) follow
pub(crate) const N_OLD: usize = 1;

fn ctor_dispatch(e: &mut Engine, api: Ptr, n: i32) -> LResult<()> {
    if e.rt.cur_build() < N_OLD { coll_level3aConstructor(e, api, n) } else { xb::ctor(e, api, n) }
}

fn dtor_dispatch(e: &mut Engine, n: i32) -> LResult<()> {
    if e.rt.cur_build() < N_OLD { coll_level3aDestructor(e, n) } else { xb::dtor(e, n) }
}

fn build_init_dispatch(m: &mut Mem, data: Ptr, b: usize) {
    if b < N_OLD {
        build_init(m, data, b);
    } else {
        xb::build_init(m, data, b - N_OLD);
    }
}

pub fn build_index(key: &str) -> Option<usize> {
    if let Some(i) = build_index_old(key) {
        return Some(i);
    }
    xb::build_index(key).map(|i| i + N_OLD)
}

fn build_index_old(key: &str) -> Option<usize> {
    (key == "b173298d").then_some(0)
}

pub(crate) const VARIANT: Variant = Variant {
    radar: 0x2a10_0000,
    fx_part: 0x2a80_00dc,
    voice_lo: 0x2a00_00a5,
    voice_hi: 0x2a00_00ab,
    hero_a: 0x2a00_00f0,
    hero_b: 0x2a00_00ef,
    voice_a: [0xa000_0003, 0x8000_004a],
    voice_b: [0xa000_0004, 0x8000_004b],
    tutorial: 0,
    n: 0x1a,
    gs: 0x268,
    gsize: 0x7e8,
    ms: 0x270,
    voice_gate: false,
    tutorial_on: false,
};

/// TmpMap (26 x 0x2c, module image 0x10030)
const TMP_MAP: [[u32; 11]; 26] = [
    [0x2a000070, 0, 0x4, 4, 5, 5, 4, 6, 0, 0, 0],
    [0x2a000072, 0, 0x20, 4, 5, 1, 3, 6, 0, 0, 0],
    [0x2a000073, 0, 0x40, 4, 5, 1, 3, 6, 0, 0, 0],
    [0x2a000074, 0, 0x80, 4, 5, 1, 3, 6, 0, 0, 0],
    [0x2a000075, 0, 0x100, 4, 5, 1, 3, 6, 0, 0, 0],
    [0x2a000076, 0, 0x200, 4, 5, 1, 3, 6, 0, 0, 0],
    [0x2a000077, 0, 0x400, 4, 5, 1, 3, 6, 0, 0, 0],
    [0x2a000078, 0, 0x800, 4, 5, 1, 3, 6, 0, 0, 0],
    [0x2a000079, 0, 0x1000, 4, 5, 1, 3, 6, 0, 0, 0],
    [0x2a00007a, 0, 0x2000, 4, 5, 1, 3, 6, 0, 0, 0],
    [0x2a00007b, 0, 0x4000, 4, 5, 1, 3, 6, 0, 0, 0],
    [0x2a00007c, 0, 0x8000, 4, 5, 1, 3, 6, 0, 0, 0],
    [0x2a00007d, 0, 0x10000, 4, 5, 1, 3, 6, 0, 0, 0],
    [0x2a00007e, 0, 0x20000, 4, 5, 1, 3, 6, 0, 0, 0],
    [0x2a00007f, 0, 0x40000, 4, 5, 1, 3, 6, 0, 0, 0],
    [0x2a000080, 0, 0x80000, 4, 5, 1, 3, 6, 0, 0, 0],
    [0x2a000081, 0, 0x100000, 4, 5, 1, 3, 6, 0, 0, 0],
    [0x2a000082, 0, 0x200000, 4, 5, 1, 3, 6, 0, 0, 0],
    [0x2a000083, 0, 0x400000, 4, 5, 1, 3, 6, 0, 0, 0],
    [0x2a000087, 1, 0x8, 2, 3, 0, 2, 0, 7, 0x2a000087, 0x85],
    [0x2a000088, 1, 0x1, 2, 3, 0, 2, 0, 7, 0x2a000088, 0x85],
    [0x2a000089, 1, 0x10, 2, 3, 0, 2, 0, 7, 0x2a000089, 0x129],
    [0x2a0000a6, 1, 0x100, 0xc, 0xd, 0, 8, 0, 0, 0, 0x14e],
    [0x2a0000aa, 1, 0x200, 0xc, 0xd, 0, 8, 0, 0, 0, 0x14e],
    [0x2a000069, 2, 0x4, 8, 9, 0, 5, 0, 0, 0, 0x156],
    [0x2a000048, 1, 0x2000000, 0xa, 0xb, 0, 1, 0, 0, 0x10, 0x15a],
];

fn build_init(m: &mut Mem, data: Ptr, _: usize) {
    write_tmp_map(m, data, &TMP_MAP);
}

/// pFunctions (0x104a8, "ApiCFunctions"; stored, never used)
const pFunctions: u32 = 0x104a8;

/// GB2+8 (pGlobalDataColl_Base[2] + 8): launched flags 0x8000000 (0x70) / 0x20000 (0x89)
fn gb2_8(e: &Engine) -> Ptr {
    let gd = e.r32(G(e, base::pGlobalDataColl_Base));
    e.r32(gd + 8) + 8
}
fn collected(e: &Engine) -> bool {
    let l = L(e);
    let i = e.r32(l);
    let s = e.r32(G(e, base::uiToStatusPointerMap) + 4 * i);
    e.r32(s) & e.r32(l + 4) != 0
}

/// ext fn 1
pub fn fstate_FIRSTMOVE_LOCAL(e: &mut Engine, p: u32) -> LResult<u64> {
    let id = APIgameobj::GetID(e, SELF, NULL);
    let l = L(e);
    e.w32(l + 0x88, (id == 0x2a00_0070 || id == 0x2a00_0089) as u32);
    base::fstate_FIRSTMOVE(e, p)?;
    let f = e.r32(gb2_8(e));
    if id == 0x2a00_0070 {
        if f & 0x800_0000 != 0 {
            LaunchObject(e, 0x2a00_0070);
        }
    } else if id == 0x2a00_0089 && f & 0x2_0000 != 0 {
        LaunchObject(e, 0x2a00_0089);
    }
    Ok(0)
}

/// ext fn 5
pub fn fstate_LAUNCH(e: &mut Engine, _: u32) -> LResult<u64> {
    let l = L(e);
    APIgameobj::GetPosition(e, SELF, 0x7fff_ffff, l + 0x7c);
    APIgameobj::GetID(e, SELF, NULL);
    if e.rf(l + 0x80) < e.rf(l + 0x68) {
        APIphysic::SetVelocity(e, SELF, 0.0, 0.0, 0.0);
        let gd = e.r32(G(e, base::pGlobalDataColl_Base));
        let hero = e.r32(e.r32(gd) + 8);
        base::Coll_BaseFUNC_SetProxyClipping(e, SELF, hero, 9);
        let y = e.rf(l + 0x68) - 0.35;
        e.wf(l + 0x80, y);
        let p = e.rv3(l + 0x7c);
        e.wv3(l + 0x94, p);
        APIgameobj::SetContextFlag(e, SELF, 0);
        APIgameobj::SetNextState(e, SELF, 2);
    }
    Ok(0)
}

/// ext fn 0xb (cut-scene notify {kind, on})
pub fn fcback_CUTSCENE(e: &mut Engine, p: u32) -> LResult<u64> {
    let (k, on) = (e.r32(p), e.r32(p + 4));
    let l = L(e);
    if on == 1 && k == 1 {
        let id = APIgameobj::GetID(e, SELF, NULL);
        if id != 0x2a00_00aa {
            if e.r32(l + 0xa4) != 0 {
                let g = e.r32(G(e, base::uiGlowSpriteType));
                let s = e.r32(l + 0x8c);
                APIgraphics::SpriteGroupRemoveSprite(e, g, s);
                e.w32(l + 0xa4, 0);
            }
            APIgameobj::SetGameObjectVisible(e, SELF, 0);
            APIgameobj::EnableObject(e, SELF, 0);
        }
    } else if on == 0 && k == 1 && APIgameobj::GetID(e, SELF, NULL) != 0x2a00_00aa && e.r32(l + 0xa4) == 0 && !collected(e) {
        base::Coll_BaseFUNC_CreateGlow(e);
        APIgameobj::SetGameObjectVisible(e, SELF, 1);
        APIgameobj::EnableObject(e, SELF, 1);
    }
    Ok(0)
}

pub fn MyLocalMessageFunction(e: &mut Engine, m: u32, p: u32, d: Ptr) -> LResult<u64> {
    if m == 0xc1 {
        let id = APIgameobj::GetID(e, SELF, NULL);
        let a = gb2_8(e);
        if id == 0x2a00_0070 {
            let v = e.r32(a) & 0xf7ff_ffff;
            e.w32(a, v);
        } else if id == 0x2a00_0089 {
            let v = e.r32(a) & 0xfffd_ffff;
            e.w32(a, v);
        }
    } else if m == 0xd3 {
        LaunchObject(e, p);
    }
    base::MyMessageFunction(e, m, p, d)
}

pub fn LaunchObject(e: &mut Engine, from: u32) {
    if collected(e) {
        return;
    }
    let id = APIgameobj::GetID(e, SELF, NULL);
    let bit = match id {
        0x2a00_0070 => 0x800_0000,
        0x2a00_0089 => 0x2_0000,
        _ => return,
    };
    let a = gb2_8(e);
    let v = e.r32(a) | bit;
    e.w32(a, v);
    SetForLaunch(e, from);
}

pub fn SetForLaunch(e: &mut Engine, from: u32) {
    APIgameobj::SetGroupID(e, SELF, 6);
    let l = L(e);
    APIgameobj::SetTranslation(e, SELF, l + 0x64);
    LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 2, NULL, 0);
    let v = e.alloca(0x10);
    e.wv3(v, [0.0, 4.0, 5.0]);
    APIgameobj::TakeOrientation(e, SELF, from, 0x7fff_ffff);
    APIgameobj::TransformVector(e, SELF, v, v);
    APIphysic::Impulse(e, SELF, v);
    APIgameobj::SetContextFlag(e, SELF, 3);
    APIgameobj::SetNextState(e, SELF, 5);
}

pub fn coll_level3aConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    base::coll_baseConstructor(e, api, instanceCount);
    if instanceCount == 1 {
        APIcons::SetExternalFunction(e, 1, fstate_FIRSTMOVE_LOCAL);
        APIcons::SetExternalFunction(e, 0xb, fcback_CUTSCENE);
        APIcons::SetExternalFunction(e, 5, fstate_LAUNCH);
        base::init(e);
        let n = e.cstr("ApiCFunctions");
        let v = APIcons::GetApiCustom(e, n);
        let a = e.rt.cur_data() + (pFunctions - 0x10000);
        e.w32(a, v);
    }
    APIcons::SetMessageFunction(e, MyLocalMessageFunction);
    APIgameobj::GetID(e, SELF, NULL);
    APIphysic::CreatePhysic(e, SELF, 0);
    APIphysic::SetFlags(e, SELF, 2);
    LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 2, NULL, 0);
    let l = L(e);
    let dm = APIphysic::DMAdd(e, SELF, 7);
    e.w32(l + 0x40, dm);
    let p = e.alloca(0x20); // uStack_70: {strength 0.7, 7}
    APIphysic::DMGetParams(e, dm, p);
    e.w32(p, 0x3f33_3333);
    e.w32(p + 4, 7);
    APIphysic::DMSetParams(e, dm, p);
    APIphysic::DMSetContextMask(e, dm, 1);
    let dm = APIphysic::DMAdd(e, SELF, 0);
    e.w32(l + 0x38, dm);
    let p = e.alloca(0x20); // uStack_60: gravity (0, -10, 0)
    APIphysic::DMGetParams(e, dm, p);
    e.wv3(p, [0.0, -10.0, 0.0]);
    APIphysic::DMSetParams(e, dm, p);
    APIphysic::DMSetContextMask(e, dm, 2);
    APIgameobj::SetContextFlag(e, SELF, 0);
    APIcons::SetInitialState(e, 1);
    Ok(())
}

pub fn coll_level3aDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    base::coll_baseDestructor(e, instanceCount);
    Ok(())
}

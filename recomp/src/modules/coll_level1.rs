//! `coll_level1`: EP1L01 collectables (12 entries in `TmpMap`: clue/key items 0x2a0000a1..ba, health
//! snacks 0x2a0000bc/bd, map 0x2a0000d1, book pages 0x2a0000db..dd). Each instance finds its row by
//! object id, bobs + spins with a glow sprite; on proximity (ext 9) it is picked up: status bit set in
//! the save buffer, sounds/text, radar msg (0x2a100006), tutorial hints (0x2a000020), FX.
//! Port of `Game Logic Modules/Ep1_Library/COLLECTABLES/LEVEL01/coll_level1__d7fb0e48/`
//! (Coll_level1.c, Collectable.c, Init.c, Coll_base.c, coll_collect.c). EP1L01 build d7fb0e48.
//!
//! pLocalDataColl_Base (0x108): [0] status idx, [1] bit, [2] set msg, [3] reset msg, [4] health
//! kind, [5..7] sounds, [8] msg param, [9] text, +0x28 flags (1 in proxy, 2 can collect), +0x2c id,
//! +0x44..+0x5c timers, +0x64 spawn pos, +0x88 local flag, +0x8c glow sprite, +0x94 bob pos,
//! +0xa0 pos valid, +0xa4 glow on, +0xa8 collected, +0xb4 clipping, +0xf8 sector clip reg, +0x104.
#![allow(non_snake_case, non_upper_case_globals, unused_variables)]

use crate::engine::*;
use crate::logic::{CodeFn, LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "coll_level1",
    // globals 0x10000..0x10274 (+ gpoLanguagesNamesAndISOCodes)
    data_size: 0x1bf0,
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
    if e.rt.cur_build() < N_OLD { coll_level1Constructor(e, api, n) } else { xb::ctor(e, api, n) }
}

fn dtor_dispatch(e: &mut Engine, n: i32) -> LResult<()> {
    if e.rt.cur_build() < N_OLD { coll_level1Destructor(e, n) } else { xb::dtor(e, n) }
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
    (key == "d7fb0e48").then_some(0)
}

/// TmpMap (12 x 0x2c, .data at 0x10030): id, status idx, bit, set msg, reset msg, health kind,
/// snd1..3, msg param, text
const TMP_MAP_L01: [[u32; 11]; 12] = [
    [0x2a0000a1, 0, 1, 4, 5, 5, 4, 6, 0, 0, 0],
    [0x2a0000a2, 0, 2, 4, 5, 5, 4, 6, 0, 0, 0],
    [0x2a0000a3, 0, 4, 4, 5, 5, 4, 6, 0, 0, 0],
    [0x2a0000a7, 0, 0x20, 4, 5, 1, 3, 6, 0, 0, 0],
    [0x2a0000b9, 0, 0x4000, 4, 5, 1, 3, 6, 0, 0, 0],
    [0x2a0000ba, 0, 0x8000, 4, 5, 1, 3, 6, 0, 0, 0],
    [0x2a0000bd, 1, 2, 2, 3, 0, 2, 0, 7, 0x2a0000bd, 0x126],
    [0x2a0000bc, 1, 4, 2, 3, 0, 2, 0, 7, 0x2a0000bc, 0x127],
    [0x2a0000d1, 1, 0x800, 0xa, 0xb, 0, 1, 0, 0, 2, 0x162],
    [0x2a0000db, 1, 0x20, 0xc, 0xd, 0, 8, 0, 0, 0, 0x15a],
    [0x2a0000dc, 1, 0x40, 0xc, 0xd, 0, 8, 0, 0, 0, 0x15a],
    [0x2a0000dd, 1, 0x80, 0xc, 0xd, 0, 8, 0, 0, 0, 0x15a],
];

fn build_init(m: &mut Mem, data: Ptr, _: usize) {
    write_tmp_map(m, data, &TMP_MAP_L01);
}

pub(crate) fn write_tmp_map(m: &mut Mem, data: Ptr, map: &[[u32; 11]]) {
    for (i, row) in map.iter().enumerate() {
        for (j, &v) in row.iter().enumerate() {
            m.w32(data + 0x30 + 0x2c * i as u32 + 4 * j as u32, v);
        }
    }
}

pub const pExtra: u32 = 0x10000;
pub const pGameObj: u32 = 0x10004;
pub const pPhysic: u32 = 0x10008;
pub const pAnim: u32 = 0x1000c;
pub const pStdlib: u32 = 0x10010;
pub const pSound: u32 = 0x10014;
pub const pHud: u32 = 0x10018;
pub const pSndMngr: u32 = 0x1001c;
pub const pText: u32 = 0x10020;
pub const pGraphic: u32 = 0x10024;
pub const pMiscFunc: u32 = 0x10028;
pub const pSectorClip: u32 = 0x1002c;
pub const TmpMap: u32 = 0x10030;
pub const uiToStatusPointerMap: u32 = 0x10240;
pub const pLocalDataColl_Base: u32 = 0x1024c;
pub const pGlobalDataColl_Base: u32 = 0x10250;
pub const pParticle: u32 = 0x10258;
pub const pMath: u32 = 0x1025c;
pub const uiGlowSpriteType: u32 = 0x10260;
pub const oMyRotAxe: u32 = 0x10268;

/// Per-level literals of Coll_base.c
pub(crate) struct Variant {
    pub(crate) radar: u32,
    pub(crate) fx_part: u32,
    /// Coll_BaseFUNC_PlayVoice: id range (exclusive) and heroes
    pub(crate) voice_lo: u32,
    pub(crate) voice_hi: u32,
    pub(crate) hero_a: u32,
    pub(crate) hero_b: u32,
    pub(crate) voice_a: [u32; 2],
    pub(crate) voice_b: [u32; 2],
    pub(crate) tutorial: u32,
    /// TmpMap rows; pGlobalDataColl_Base offset shift after the table; its size; module-globals
    /// shift after TmpMap (incl. L03A pFunctions)
    pub(crate) n: u32,
    pub(crate) gs: u32,
    pub(crate) gsize: i32,
    pub(crate) ms: u32,
    /// PlayVoice also requires GB0+0x40 bit 0x40000 (L01 only); Tutorial body present (L01 only)
    pub(crate) voice_gate: bool,
    pub(crate) tutorial_on: bool,
}
const L01: Variant = Variant {
    n: 0xc,
    gs: 0,
    gsize: 0x580,
    ms: 0,
    voice_gate: true,
    tutorial_on: true,
    radar: 0x2a10_0006,
    fx_part: 0x2a80_0138,
    voice_lo: 0x2a00_00da,
    voice_hi: 0x2a00_00e0,
    hero_a: 0x2a00_0091,
    hero_b: 0x2a00_0092,
    voice_a: [0xa000_0000, 0x8000_0022],
    voice_b: [0xa000_0001, 0x8000_0023],
    tutorial: 0x2a00_0020,
};
pub(crate) fn V(e: &Engine) -> &'static Variant {
    match e.rt.codes[e.rt.current_code().expect("no logic running")].def.name {
        "coll_level3a" => &super::coll_level3a::VARIANT,
        _ => &L01,
    }
}

#[inline]
pub(crate) fn G(e: &Engine, a: u32) -> Ptr {
    let s = if a >= uiToStatusPointerMap { V(e).ms } else { 0 };
    e.rt.cur_data() + (a - 0x10000) + s
}
/// pGlobalDataColl_Base past the TmpMap copy
fn GX(e: &Engine) -> Ptr {
    GD(e) + V(e).gs
}
#[inline]
pub(crate) fn L(e: &Engine) -> Ptr {
    e.r32(G(e, pLocalDataColl_Base))
}
#[inline]
fn lr(e: &Engine, o: u32) -> u32 {
    e.r32(L(e) + o)
}
#[inline]
fn lw(e: &mut Engine, o: u32, v: u32) {
    let a = L(e) + o;
    e.w32(a, v);
}
/// pGlobalDataColl_Base (0x580): [0] GB0, [1] health float*, [2] GB2, +0xc entries (12 x 0x2c),
/// +0x21c FX, +0x220.. generic FX params
#[inline]
fn GD(e: &Engine) -> Ptr {
    e.r32(G(e, pGlobalDataColl_Base))
}
fn GB0(e: &Engine) -> Ptr {
    e.r32(GD(e))
}
fn glow_type(e: &Engine) -> u32 {
    e.r32(G(e, uiGlowSpriteType))
}
/// `(&uiToStatusPointerMap)[local[0]]`
fn status_ptr(e: &Engine) -> Ptr {
    let i = lr(e, 0);
    e.r32(G(e, uiToStatusPointerMap) + 4 * i)
}
fn send(e: &mut Engine, to: u32, m: u32, p: u32) {
    APIgameobj::SendMessageTo(e, to, NULL, m, p, NULL);
}
fn remove_glow(e: &mut Engine) {
    let g = glow_type(e);
    let s = lr(e, 0x8c);
    APIgraphics::SpriteGroupRemoveSprite(e, g, s);
    lw(e, 0x8c, 0);
    lw(e, 0xa4, 0);
}
fn add_glow(e: &mut Engine) {
    let g = glow_type(e);
    let s = APIgraphics::SpriteGroupAddSprite(e, g, SELF, 0x7fff_ffff, 0.7);
    lw(e, 0x8c, s);
    lw(e, 0xa4, 1);
}
fn show_text(e: &mut Engine, text: u32) {
    if text != 0 && LIBApiText::IsTextDisplayed(e) == 0 {
        let b = e.alloca(0x40);
        LIBApiText::InitStruct(e, b);
        e.w32(b, 0x4000_0000);
        LIBApiText::ShowText(e, text, b);
    }
}
fn play_sounds(e: &mut Engine) {
    let s = lr(e, 0x14) as i32;
    LIBSndMngr::PlaySound(e, s);
    for o in [0x18, 0x1c] {
        let s = lr(e, o) as i32;
        if s != 0 {
            LIBSndMngr::PlaySound(e, s);
        }
    }
}

// ---- Coll_level1.c / Collectable.c / Init.c

/// ext fn 1
pub fn fstate_FIRSTMOVE_LOCAL(e: &mut Engine, p: u32) -> LResult<u64> {
    lw(e, 0x88, 0);
    fstate_FIRSTMOVE(e, p)
}

pub fn coll_level1Constructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    coll_baseConstructor(e, api, instanceCount);
    if instanceCount == 1 {
        APIcons::SetExternalFunction(e, 1, fstate_FIRSTMOVE_LOCAL);
        init(e);
    }
    let t = APIextra::TimerCreate(e);
    lw(e, 0x90, t);
    APIcons::SetInitialState(e, 1);
    Ok(())
}

pub fn coll_level1Destructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    coll_baseDestructor(e, instanceCount);
    let t = lr(e, 0x90);
    APIextra::TimerDestroy(e, t);
    Ok(())
}

/// Init.c: copies TmpMap into the global table, builds the 3 status pointers into GB2.
pub fn init(e: &mut Engine) -> u32 {
    let gd = GD(e);
    let tm = G(e, TmpMap);
    for i in 0..V(e).n {
        for j in 0..11 {
            let v = e.r32(tm + i * 0x2c + 4 * j);
            e.w32(gd + 0xc + i * 0x2c + 4 * j, v);
        }
    }
    let gb2 = e.r32(gd + 8);
    let m = G(e, uiToStatusPointerMap);
    e.w32(m, gb2 + 0x18);
    e.w32(m + 4, gb2 + 0xc);
    e.w32(m + 8, gb2);
    gd
}

// ---- coll_collect.c

pub fn SetParticleCollectablePickup_spark(e: &mut Engine, _: u32, t: u32) -> LResult<()> {
    let t = t as i32;
    for (a, v) in [(8, 5), (1, 2), (4, 0x96), (0x10, 0), (0x20, 0), (0x40, 0), (0x80, 0), (0x100, 0)] {
        APIparticle::TypeSetInt(e, t, a, v);
    }
    for (a, v) in [(1, 0), (4, 0), (2, 0x3fc0_0000), (8, 0x4000_0000)] {
        APIparticle::TypeSetFloat(e, t, a, f32::from_bits(v));
    }
    APIparticle::TypeSetVec(e, t, 1, 0.0, -3.0, 0.0);
    APIparticle::TypeSetVec(e, t, 2, 0.0, 0.0, 0.0);
    inter(e, t, &[(1, [0x3f80_0000, 0x3f00_0000, 0, 0x3f00_0000]), (2, [0x3f80_0000, 0, 0, 0x3f00_0000]), (4, [0x3f80_0000, 0x3f80_0000, 0, 0x3f00_0000]), (8, [0; 4]), (0x10, [0x3ecc_cccd, 0x3e80_0000, 0x3dcc_cccd, 0x3f40_0000]), (0x20, [0, 0x3f00_0000, 0x3f80_0000, 0x3f00_0000])]);
    Ok(())
}

pub fn SetParticleCollectablePickup_glow(e: &mut Engine, _: u32, t: u32) -> LResult<()> {
    let t = t as i32;
    for (a, v) in [(8, 1), (1, 2), (4, 0x14a), (0x10, 0), (0x20, 0), (0x40, 0), (0x80, 0), (0x100, 0)] {
        APIparticle::TypeSetInt(e, t, a, v);
    }
    for (a, v) in [(1, 0), (4, 0x3f26_6666), (2, 0x3f00_0000), (8, 0)] {
        APIparticle::TypeSetFloat(e, t, a, f32::from_bits(v));
    }
    APIparticle::TypeSetVec(e, t, 1, 0.0, 0.0, 0.0);
    APIparticle::TypeSetVec(e, t, 2, 0.0, 0.0, 0.0);
    inter(e, t, &[(1, [0x3f00_0000, 0x3e80_0000, 0x3c23_d70a, 0x3e80_0000]), (2, [0x3f00_0000, 0, 0x3c23_d70a, 0x3e80_0000]), (4, [0x3f80_0000, 0x3e80_0000, 0x3c23_d70a, 0x3e80_0000]), (8, [0, 0x3f00_0000, 0, 0]), (0x10, [0x4020_0000, 0x3fe0_0000, 0x3f80_0000, 0x3e80_0000]), (0x20, [0x3e00_0000, 0x3e00_0000, 0x3e00_0000, 0x3f80_0000])]);
    Ok(())
}

fn inter(e: &mut Engine, t: i32, v: &[(u32, [u32; 4])]) {
    let f = f32::from_bits;
    for &(a, [x, y, z, w]) in v {
        APIparticle::TypeSetInter(e, t, a, f(x), f(y), f(z), f(w));
    }
}

// ---- Coll_base.c

/// ext fn 0
pub fn fstate_FIRSTMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    let l = L(e);
    let id = APIgameobj::GetID(e, SELF, NULL);
    e.w32(l + 0xf4, 1);
    e.w32(l + 0x104, 1);
    let gd = GD(e);
    for i in 0..V(e).n {
        let row = gd + 0xc + i * 0x2c;
        if e.r32(row) == id {
            e.w32(l + 0x2c, id);
            // local[0..10] = row words 1..11 (local[10] untouched)
            for j in 0..10 {
                let v = e.r32(row + 4 + 4 * j);
                e.w32(l + 4 * j, v);
            }
            break;
        }
    }
    if AmIDead(e) == 1 {
        APIgameobj::KillObject(e, SELF);
    }
    if e.r32(l + 0xa8) == 0 {
        let sp = e.alloca(0x30); // uStack_60 _ShadowParams
        for (o, v) in [(0, 2), (4, 0), (8, 0x3ecc_cccd), (0xc, 0x4040_0000), (0x10, 0x3f80_0000), (0x14, 0x40), (0x24, 0x80), (0x28, 0x3f40_0000)] {
            e.w32(sp + o, v);
        }
        APIgameobj::SetShadowParams(e, SELF, sp);
        APIgameobj::SetLogicFlag(e, SELF, 0x8000);
        e.w32(l + 0xa4, 0);
        Coll_BaseFUNC_CreateGlow(e);
        Coll_BaseFUNC_SetClipping(e);
        if e.r32(l + 0x88) == 1 {
            APIgameobj::SetNextState(e, SELF, 4);
        } else {
            let hero = e.r32(GB0(e) + 8);
            Coll_BaseFUNC_SetProxyClipping(e, SELF, hero, 9);
            APIgameobj::SetNextState(e, SELF, 2);
        }
        if e.r32(l + 0xf8) != id {
            e.w32(l + 0xf8, id);
            e.w16(l + 0x102, 2);
            e.w16(l + 0x100, 0);
            LIBApiSectorClipping::ApiImpl_Register(e, l + 0xf8);
        }
    } else {
        SetBit(e);
        APIgameobj::KillObject(e, SELF);
    }
    Ok(0)
}

/// ext fn 2
pub fn fstate_BASICMOVE_SET(e: &mut Engine, _: u32) -> LResult<u64> {
    Coll_BaseFUNC_CheckEvents(e, 1);
    Coll_baseFUNC_RunBob(e);
    Coll_BaseFUNC_Spin(e, 4.0);
    Ok(0)
}

/// ext fn 3
pub fn fstate_BASICMOVE_LOOP(e: &mut Engine, _: u32) -> LResult<u64> {
    Coll_BaseFUNC_CheckEvents(e, 2);
    Coll_baseFUNC_RunBob(e);
    Coll_BaseFUNC_Spin(e, 20.0);
    Ok(0)
}

/// ext fn 4
pub fn fstate_BLACKHOLE_LOOP(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

pub fn Coll_BaseFUNC_CheckEvents(e: &mut Engine, param_1: u32) -> u32 {
    let v = V(e);
    let l = L(e);
    let health = e.r32(GD(e) + 4);
    let gb0 = GB0(e);
    if param_1 & 1 != 0 && e.r32(l + 0x28) & 1 != 0 {
        let busy = e.r32(gb0 + 0x14) & 0x0800_090c;
        if busy != 0 {
            return busy;
        }
        if e.r32(l + 0x10) != 0 && 1.0 <= e.rf(health) {
            // health already full: bounce, retry in 1 s
            APInewSound::Play(e, SELF, 9, 0);
            let t = e.r32(l + 0x44);
            APIextra::TimerReset(e, t);
            APIextra::TimerSetFrequency(e, t, 1.0);
            APIextra::TimerRegisterNotify(e, SELF, t, 8);
            APIgameobj::SetNextState(e, SELF, 3);
            let f = e.r32(l + 0x28) & 0xffff_fffd;
            e.w32(l + 0x28, f);
            return l;
        }
        if e.r32(l + 0xa4) != 0 && e.r32(l + 0x8c) != 0 {
            remove_glow(e);
        }
        let id = APIgameobj::GetID(e, SELF, NULL);
        let p = e.alloca(0x10);
        APIgameobj::GetPosition(e, id, 0x7fff_ffff, p);
        let pos = e.rv3(p);
        let gd = GD(e);
        let gd = gd + V(e).gs;
        for base in [0x2a8, 0x354] {
            e.wv3(gd + base, pos);
            for o in (0xc..0x24).step_by(4) {
                e.w32(gd + base + o, 0);
            }
        }
        let fx = e.ri32(gd + 0x21c);
        LIBMiscFunc::MiscFunc_PlayGenericFX(e, fx);
        play_sounds(e);
        Coll_BaseFUNC_PlayVoice(e);
        let text = e.r32(l + 0x24);
        show_text(e, text);
        let f = e.r32(l + 0x28) & 0xffff_fffe;
        e.w32(l + 0x28, f);
        APIgameobj::SetGroupID(e, SELF, 6);
        APIgameobj::EnableObject(e, SELF, 0);
        APIgameobj::UnregisterNotify(e, SELF, 0xffff_ffff, 10);
        SetBit(e);
        send(e, SELF, 0x14b, 0);
        let id = APIgameobj::GetID(e, SELF, NULL);
        send(e, v.radar, 0x33, id);
        let t = e.r32(l + 0x5c);
        APIextra::TimerReset(e, t);
        APIextra::TimerSetFrequency(e, t, 1.25);
        APIextra::TimerRegisterNotify(e, SELF, t, 8);
        Coll_BaseFUNC_Tutorial(e);
        APIgameobj::SetNextState(e, SELF, 4);
    }
    if param_1 & 2 != 0 && e.r32(l + 0x28) & 2 != 0 {
        return APIgameobj::SetNextState(e, SELF, 2);
    }
    0
}

pub fn Coll_BaseFUNC_PlayVoice(e: &mut Engine) {
    let v = V(e);
    let gb0 = GB0(e);
    let id = lr(e, 0x2c);
    if (!v.voice_gate || e.r32(gb0 + 0x40) & 0x40000 != 0) && id < v.voice_hi && v.voice_lo < id {
        let two = (e.r32(gb0 + 0x34) & 0x101 != 0) as usize;
        match e.r32(gb0 + 4) {
            1 | 3 | 5 | 7 | 9 | 0xe | 0x10 => {
                APInewSound::Play(e, v.hero_a, v.voice_a[two], 0);
            }
            2 | 4 | 6 | 8 | 10 | 0xb | 0xc | 0xd | 0xf | 0x11 => {
                APInewSound::Play(e, v.hero_b, v.voice_b[two], 0);
            }
            _ => {}
        }
    }
}

pub fn Coll_BaseFUNC_Tutorial(e: &mut Engine) {
    if !V(e).tutorial_on {
        return;
    }
    let tut = V(e).tutorial;
    let g40 = e.r32(GB0(e) + 0x40);
    let text = lr(e, 0x24);
    if 0x124 < text {
        if text < 0x12a {
            if g40 & 0x80 == 0 {
                send(e, tut, 0x103, 0);
            }
        } else if text == 0x162 && g40 & 0x2000 == 0 {
            send(e, tut, 0x106, 0);
        }
    }
    let id = lr(e, 0x2c);
    match id {
        0x2a0000a1 | 0x2a0000a2 | 0x2a0000a3 | 0x2a0000a7 | 0x2a0000b9 | 0x2a0000ba => {
            if g40 & 0x0800_0000 == 0 {
                send(e, tut, 0x10b, 0);
            }
        }
        0x2a0000db | 0x2a0000dc | 0x2a0000dd => {
            send(e, 0x2a00_0097, 0x107, id);
            send(e, tut, 0x107, 0);
        }
        _ => {}
    }
}

/// ext fn 8
pub fn fcback_TIMER(e: &mut Engine, p: u32) -> LResult<u64> {
    if p == lr(e, 0x44) {
        let (a, b) = (lr(e, 0x48), lr(e, 0x4c));
        APIextra::TimerPause(e, a, 0);
        APIextra::TimerPause(e, b, 0);
        let f = lr(e, 0x28) | 2;
        lw(e, 0x28, f);
    }
    Ok(1)
}

/// ext fn 9 (proximity notify, +4 inside)
pub fn fcback_NOTIFY_PROXYCLIPING(e: &mut Engine, p: u32) -> LResult<u64> {
    let f = lr(e, 0x28);
    let f = if e.r32(p + 4) == 0 { f & 0xffff_fffe } else { f | 1 };
    lw(e, 0x28, f);
    Ok(0)
}

fn proxy_and_state2(e: &mut Engine) {
    let hero = e.r32(GB0(e) + 8);
    Coll_BaseFUNC_SetProxyClipping(e, SELF, hero, 9);
}

pub fn MyMessageFunction(e: &mut Engine, m: u32, p: u32, _d: Ptr) -> LResult<u64> {
    let v = V(e);
    let l = L(e);
    match m {
        0xd5 => {
            let id = APIgameobj::GetID(e, SELF, NULL);
            let pp = e.alloca(0x10); // DStack_70
            APIgameobj::GetPosition(e, id, 0x7fff_ffff, pp);
            let [x, y, z] = e.rv3(pp);
            let r = e.alloca(0x30); // fStack_e0
            e.wv3(r, [x, y, z]);
            e.wv3(r + 0xc, [x, y - 1.0, z]);
            e.w32(r + 0x18, 0);
            e.w32(r + 0x20, 8);
            e.w32(r + 0x24, 0);
            let out = e.alloca(0x50); // fStack_b0 (hit only feeds an unused delta)
            APIphysic::CheckRayIntersection(e, r, out);
            e.w32(l + 0xa0, 1);
            e.wv3(l + 0x94, [x, y, z]);
            proxy_and_state2(e);
            APIgameobj::SetNextState(e, SELF, 2);
        }
        0xc1 => {
            if e.r32(l + 0x104) != 0 {
                APIgameobj::SetTranslation(e, SELF, l + 0x64);
                let f = e.r32(l + 0x28) & 0xffff_fffe;
                e.w32(l + 0x28, f);
                proxy_and_state2(e);
                APIgameobj::SetGroupID(e, SELF, 0);
                APIgameobj::SetGameObjectVisible(e, SELF, 1);
                APIgameobj::Sleep(e, SELF, 0.0);
                APIgameobj::EnableObject(e, SELF, 1);
                ResetBit(e);
                if e.r32(l + 0xa4) == 0 && glow_type(e) != 0 && e.r32(l + 0x8c) == 0 {
                    add_glow(e);
                }
                if e.r32(l + 0x88) == 1 {
                    let s = e.rv3(l + 0x64);
                    e.wv3(l + 0x94, s);
                    APIgameobj::UnregisterNotify(e, SELF, 0xffff_ffff, 9);
                    APIgameobj::SetNextState(e, SELF, 4);
                } else {
                    if e.r32(l + 0xa0) == 0 {
                        e.w32(l + 0xa0, 1);
                        APIgameobj::GetPosition(e, SELF, 0x7fff_ffff, l + 0x94);
                    }
                    APIgameobj::SetNextState(e, SELF, 2);
                }
            }
        }
        0x12 => {
            if LIBApiText::IsTextDisplayed(e) != 0 {
                LIBApiText::KillText(e);
                send(e, v.radar, 0x33, 0);
            }
        }
        0xcd => {
            if e.r32(l + 0xa0) == 0 {
                e.w32(l + 0xa0, 1);
                APIgameobj::GetPosition(e, SELF, 0x7fff_ffff, l + 0x94);
            }
            let st = APIgameobj::GetCurrentStateID(e, SELF);
            if 1 < st && e.r32(status_ptr(e)) & e.r32(l + 4) == 0 {
                Coll_BaseFUNC_SetClipping(e);
                proxy_and_state2(e);
            }
        }
        0xd4 => {
            if e.r32(l + 0xa4) != 0 && e.r32(l + 0x8c) != 0 {
                remove_glow(e);
            }
            APIgameobj::SetGroupID(e, SELF, 0);
            APIgameobj::KillObject(e, SELF);
        }
        0x14a => e.w32(l + 0xa8, 1),
        0x149 => {
            SetBit(e);
            play_sounds(e);
            let text = e.r32(l + 0x24);
            show_text(e, text);
            APIgameobj::SetNextState(e, SELF, 2);
            if e.r32(l + 0xa4) != 0 && e.r32(l + 0x8c) != 0 {
                remove_glow(e);
            }
            APIgameobj::SetGroupID(e, SELF, 6);
            APIgameobj::SetGameObjectVisible(e, SELF, 0);
            APIgameobj::Sleep(e, SELF, -1.0);
            APIgameobj::EnableObject(e, SELF, 0);
        }
        0x201 => {
            if e.r32(status_ptr(e)) & e.r32(l + 4) == 0 {
                APIgameobj::EnableObject(e, SELF, 1);
                if e.r32(l + 0x8c) == 0 && glow_type(e) != 0 {
                    add_glow(e);
                }
            }
        }
        0x202 => {
            APIgameobj::EnableObject(e, SELF, 0);
            if e.r32(l + 0x8c) != 0 && e.r32(l + 0xa4) != 0 {
                remove_glow(e);
            }
        }
        _ => {}
    }
    Ok(0)
}

pub fn Coll_baseFUNC_RunBob(e: &mut Engine) {
    let l = L(e);
    let t = e.r32(l + 0x4c);
    let tm = APIextra::TimerGetTime(e, t);
    let s = LIBMiscFunc::MiscFunc_Sin(e, tm * 3.0);
    let p = e.alloca(0x10);
    let [x, y, z] = e.rv3(l + 0x94);
    e.wv3(p, [x, y + s * 0.1, z]);
    APIgameobj::SetTranslation(e, SELF, p);
}

pub fn Coll_BaseFUNC_Spin(e: &mut Engine, deg: f32) {
    let a = G(e, oMyRotAxe);
    APIgameobj::Rotate(e, SELF, a, deg * 0.017453292);
}

pub fn Coll_BaseFUNC_SetProxyClipping(e: &mut Engine, obj: u32, target: u32, notify: u32) -> u64 {
    let p = e.alloca(0x10);
    APIgameobj::GetProximityClippingParams(e, obj, p);
    e.w32(p, target);
    e.w32(p + 4, 0x3fa6_6666);
    e.w32(p + 8, 0x3dcc_cccd);
    e.w32(p + 0xc, 0x1000_0000);
    if notify != 0 {
        APIgameobj::RegisterNotify(e, obj, 0xffff_ffff, 10, notify);
    }
    APIgameobj::SetProximityClippingParams(e, obj, p);
    0
}

pub fn Coll_BaseFUNC_SetClipping(e: &mut Engine) -> u64 {
    let c = L(e) + 0xb4;
    APIgameobj::GetClippingParams(e, SELF, c);
    for (o, v) in [(0, 0x3dcc_cccd), (4, 0x4160_0000), (0x10, 0x41c8_0000), (0x1c, 0xbf80_0000), (0xc, 0), (8, 0x2a7), (0x18, 0x80), (0x14, 0x6a7), (0x24, 0x6a0), (0x20, 0x6a7)] {
        e.w32(c + o, v);
    }
    APIgameobj::SetClippingParams(e, SELF, c, 0);
    0
}

pub fn Coll_BaseFUNC_CreateGlow(e: &mut Engine) {
    if lr(e, 0x8c) == 0 {
        let g = glow_type(e);
        let s = APIgraphics::SpriteGroupAddSprite(e, g, SELF, 0x7fff_ffff, 0.7);
        lw(e, 0x8c, s);
    }
    lw(e, 0xa4, 1);
}

pub fn Coll_BaseFUNC_InitParticles(e: &mut Engine) {
    let g = GX(e);
    e.w32(g + 0x220, 0x4000_0000);
    for (o, s) in [(0x264, "spark_collect"), (0x224, "spark")] {
        let p = e.cstr(s);
        LIBMiscFunc::MiscFunc_StrncpySafe(e, g + o, p, 0x40);
    }
    let t = e.code_ptr(CodeFn::Particle(SetParticleCollectablePickup_spark));
    e.w32(g + 0x2a4, t);
    e.w32(g + 0x2cc, 0);
    for (o, s) in [(0x310, "glow_collect"), (0x2d0, "star_light")] {
        let p = e.cstr(s);
        LIBMiscFunc::MiscFunc_StrncpySafe(e, g + o, p, 0x40);
    }
    let t = e.code_ptr(CodeFn::Particle(SetParticleCollectablePickup_glow));
    e.w32(g + 0x350, t);
    e.w32(g + 0x378, 0);
}

pub fn AmIDead(e: &mut Engine) -> u32 {
    let s = status_ptr(e);
    if s == 0 || e.r32(s) & lr(e, 4) == 0 {
        0
    } else {
        APIgameobj::KillObject(e, SELF);
        1
    }
}

pub fn SetBit(e: &mut Engine) {
    APIgameobj::GetID(e, SELF, NULL);
    let s = status_ptr(e);
    let v = e.r32(s) | lr(e, 4);
    e.w32(s, v);
    let h = e.r32(GD(e) + 4);
    let kind = lr(e, 0x10);
    if kind != 0 && e.rf(h) < 1.0 {
        if kind == 1 {
            let n = e.rf(h) + 0.2;
            e.wf(h, n);
            if 1.0 < e.rf(h) {
                e.wf(h, 1.0);
            }
        } else if kind == 5 {
            e.wf(h, 1.0);
        }
    }
    let (m, p) = (lr(e, 8), lr(e, 0x20));
    let r = V(e).radar;
    send(e, r, m, p);
}

pub fn ResetBit(e: &mut Engine) {
    APIgameobj::GetID(e, SELF, NULL);
    let s = status_ptr(e);
    let bit = lr(e, 4);
    let v = e.r32(s) & !bit;
    e.w32(s, v);
    let (m, p) = (lr(e, 0xc), lr(e, 0x20));
    let r = V(e).radar;
    send(e, r, m, p);
    if m == 0xd {
        send(e, r, 0x36, 0);
        let h = e.r32(GD(e) + 4) + 0x10;
        let v = e.r32(h) & !bit;
        e.w32(h, v);
    }
}

pub fn Coll_BaseFUNC_CreateGameObject(e: &mut Engine) {
    APIphysic::CreatePhysic(e, SELF, 0);
    APIphysic::PrimResetIntAttribBits(e, SELF, 0, 1, 2);
    let l = L(e);
    APIgameobj::GetPosition(e, SELF, 0x7fff_ffff, l + 0x64);
    e.w32(l + 0xa8, 0);
    e.w32(l + 0x28, 0);
}

pub fn coll_baseConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) {
    if instanceCount == 1 {
        let fns: [(i32, crate::logic::ExtFn); 6] = [
            (0, fstate_FIRSTMOVE),
            (2, fstate_BASICMOVE_SET),
            (3, fstate_BASICMOVE_LOOP),
            (4, fstate_BLACKHOLE_LOOP),
            (9, fcback_NOTIFY_PROXYCLIPING),
            (8, fcback_TIMER),
        ];
        for (i, f) in fns {
            APIcons::SetExternalFunction(e, i, f);
        }
        let tables: [(u32, fn(&mut Engine) -> Ptr); 10] = [
            (pAnim, |e| APIcons::GetApiAnim(e)),
            (pGameObj, |e| APIcons::GetApiGameObj(e)),
            (pExtra, |e| APIcons::GetApiExtra(e)),
            (pPhysic, |e| APIcons::GetApiPhysic(e)),
            (pSound, |e| APIcons::GetApiNewSound(e)),
            (pStdlib, |e| APIcons::GetApiStdlib(e)),
            (pParticle, |e| APIcons::GetApiParticle(e)),
            (pMath, |e| APIcons::GetApiMath(e)),
            (pHud, |e| APIcons::GetApiHUD(e)),
            (pGraphic, |e| APIcons::GetApiGraphics(e)),
        ];
        for (slot, get) in tables {
            let v = get(e);
            let a = G(e, slot);
            e.w32(a, v);
        }
        for (slot, name) in [(pSndMngr, "SndMngr"), (pText, "ApiText"), (pMiscFunc, "MiscFunc"), (pSectorClip, "ApiSectorClipping")] {
            let n = e.cstr(name);
            let v = APIcons::GetApiCustom(e, n);
            let a = G(e, slot);
            e.w32(a, v);
        }
        let slot = G(e, pGlobalDataColl_Base);
        APIcons::GetGlobalData(e, slot, V(e).gsize);
        let gd = e.r32(slot);
        for (w, size) in [(0u32, 0x8f8u32), (1, 0x68), (2, 0x158)] {
            let b = APIgameobj::GetGameBuffer(e, w, size, 1);
            e.w32(gd + 4 * w, b);
        }
        Coll_BaseFUNC_InitParticles(e);
        let gd = gd + V(e).gs;
        e.w32(gd + 0x21c, 0);
        let part = V(e).fx_part;
        let h = LIBMiscFunc::MiscFunc_CreateGenericFX(e, part, gd + 0x220, 2);
        e.w32(gd + 0x21c, h);
        // colour from $f12..$f15 (hidden by Ghidra): 0.375, 0, 0.75, 0.1
        let g = G(e, uiGlowSpriteType);
        LIBMiscFunc::MiscFunc_CreateStandardGlow(e, 0.375, 0.0, 0.75, 0.1, g);
        let a = G(e, oMyRotAxe);
        e.wv3(a, [0.0, 1.0, 0.0]);
    }
    APIcons::SetMessageFunction(e, MyMessageFunction);
    let slot = G(e, pLocalDataColl_Base);
    APIcons::GetLocalData(e, slot, 0x108);
    Coll_BaseFUNC_CreateGameObject(e);
    let l = L(e);
    for o in [0x5c, 0x50, 0x48, 0x4c, 0x44] {
        let t = APIextra::TimerCreate(e);
        e.w32(l + o, t);
    }
    for o in [0xa0, 0x94, 0x98, 0x9c] {
        e.w32(l + o, 0);
    }
    LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 2, NULL, 0);
}

pub fn coll_baseDestructor(e: &mut Engine, instanceCount: i32) {
    for o in [0x48, 0x4c, 0x44, 0x5c, 0x50] {
        let t = lr(e, o);
        APIextra::TimerDestroy(e, t);
    }
    let r = L(e) + 0xf8;
    LIBApiSectorClipping::ApiImpl_Unregister(e, r);
    let s = lr(e, 0x8c);
    let g = glow_type(e);
    if s != 0 {
        APIgraphics::SpriteGroupRemoveSprite(e, g, s);
    }
    if instanceCount == 0 {
        APIgraphics::SpriteRemoveGroup(e, g);
        let fx = e.ri32(GX(e) + 0x21c);
        LIBMiscFunc::MiscFunc_DestroyGenericFX(e, fx);
    }
}

//! `poltergeist`: the poltergeists of Ep2 (Movie Studio: levels 1A/1B/1C/2B/9) and Ep3 (levels 1/2). One
//! shared C source (`Monster_Base.c`, `PoltergeistII_Base_Utils.c`, `PoltergeistII_base.c`,
//! `ep2_poltergeist.c` or `ep3_poltergeist.c`) compiled with a per-level file (`Poltergeist.c`) into
//! one module per level pack.
//! Port of `Game Logic Modules/Globals/Characters/Poltergeist_base/..`. Reference build: 0bb93729
//! (EP2L01C). The 7 builds are listed in `tools/rustport/poltergeist_family.txt`; per-build literals
//! and code shapes are in [`builds`] (tools/rustport/gen_builds.py), the initialised tables in
//! [`maps`] (tools/rustport/gen_data_tables.py).
//!
//! The monster side of the Tome of Doom: a poltergeist stands until the leader hero comes within
//! 10 m, picks the nearest throwable object (GetID 0xff000080), walks (or path-finds) to it, picks it
//! up, chases the hero with it and throws it (attack anims), then rests 2.5 s. Hooked by the book's
//! beam (0xd6/0x65) it drops what it holds, turns to face the book and backs away from it
//! (DM target = the book, mode 0x11) until it is pulled in (0x67) and captured (0x68). A
//! full-body effect (FB 2 = an outline on the poltergeist) is shown within 10 m of the hero (50 m
//! once the clip-change callback has fired).
//!
//! PoltergeistII_base keeps its current sub-state as a CODE POINTER at L+0x160, called every frame
//! by external state 0xf (Poltergeist_baseST_Entry_Point) and compared by the pre-logic callback.
//! The port stores the reference build's function address (see [`addr`]) and dispatches with
//! [`call_state`]; the value is private to the instance's local data, so any consistent token works.
//!
//! Module globals are shared by every instance (the engine swaps `pLocalData*` per instance), as on
//! the PS2. `pLocalDataMonster_Base` and `pLocalDataPoltergeistII_Base` point at the instance's one
//! local data block (GetLocalData grows and shares it), likewise the two global data pointers.
#![allow(non_snake_case, non_upper_case_globals, unused_variables, unused_mut, unused_assignments)]
#![allow(clippy::all)]

pub mod builds;
pub mod maps;

mod level;
mod monster_base;
pub mod poltergeist_base;
mod poltergeist_base_utils;

pub use level::*;
pub use monster_base::*;
pub use poltergeist_base::*;
pub use poltergeist_base_utils::*;

use crate::engine::{Engine, Mem, Ptr};
use crate::logic::ModuleDef;

pub static MODULE: ModuleDef = ModuleDef {
    name: "poltergeist",
    data_size: DATA_SIZE,
    ctor: poltergeist_levelConstructor,
    dtor: poltergeist_levelDestructor,
    register: None,
    // the initialised data the code reads (goCustomDataPoltergeist, TmpMonsterMap) is written per
    // build by poltergeist_build_init; string literals are interned with `e.cstr`.
    init: &[],
    build_init: Some(poltergeist_build_init),
};

/// Index of a build in [`builds::BUILDS`] by its sha8.
pub fn build_index(sha8: &str) -> Option<usize> {
    builds::BUILDS.iter().position(|b| b.sha8 == sha8)
}

fn poltergeist_build_init(m: &mut Mem, data: Ptr, build: usize) {
    for (i, &w) in maps::GO_CUSTOM_DATA_POLTERGEIST[build].iter().enumerate() {
        m.w32(data + (goCustomDataPoltergeist - 0x10000) + i as u32 * 4, w);
    }
    for (i, &w) in maps::TMP_MONSTER_MAP[build].iter().enumerate() {
        m.w32(data + (TmpMonsterMap - 0x10000) + i as u32 * 4, w);
    }
}

// ---- module globals ----
// The builds place their globals at different addresses (the two tables have per-level sizes); the
// image is private to the module, so the port uses one layout for every build: the tables at their
// largest size, then the pointers.
/// per-poltergeist data: records of 0x4c bytes {object id, +4 (L+0x1a0), kill mask (L+0x40), uses a
/// path (L+0x1a4), path name [0x3c] (L+0x1a8)}; 304 bytes max (4 records)
pub const goCustomDataPoltergeist: u32 = 0x10000;
/// per-monster-type data: records of 0x3c bytes {type, speeds and ranges...}; 300 bytes max
pub const TmpMonsterMap: u32 = 0x10130;
pub const pFBEffects: u32 = 0x10260;
pub const pMath: u32 = 0x10264;
pub const pPathFinding: u32 = 0x10268;
pub const pSound: u32 = 0x1026c;
pub const pSectorClip: u32 = 0x10270;
pub const pAnimMngr: u32 = 0x10274;
pub const pFunction: u32 = 0x10278;
pub const pMiscFunc: u32 = 0x1027c;
pub const pDebug: u32 = 0x10280;
pub const pPhysic: u32 = 0x10284;
pub const pExtra: u32 = 0x10288;
pub const pGameObj: u32 = 0x1028c;
pub const pAnim: u32 = 0x10290;
pub const pLocalDataMonster_Base: u32 = 0x10294;
pub const pGlobalDataMonster_Base: u32 = 0x10298;
pub const pLocalDataPoltergeistII_Base: u32 = 0x1029c;
pub const pGlobalDataPoltergeist_Base: u32 = 0x102a0;
pub const DATA_SIZE: u32 = 0x2a4;

/// `ALL` joint / bone selector (0x7fffffff)
pub(crate) const ALL: u32 = 0x7fff_ffff;

/// Address of a module global.
#[inline]
pub(crate) fn G(e: &Engine, a: u32) -> Ptr {
    e.rt.cur_data() + (a - 0x10000)
}
/// Current value of `pLocalDataPoltergeistII_Base` (this instance's local data, 0x218 bytes).
#[inline]
pub(crate) fn L(e: &Engine) -> Ptr {
    e.r32(G(e, pLocalDataPoltergeistII_Base))
}
/// Current value of `pLocalDataMonster_Base` (the same block as [`L`] once both are registered).
#[inline]
pub(crate) fn LM(e: &Engine) -> Ptr {
    e.r32(G(e, pLocalDataMonster_Base))
}
/// Current value of `pGlobalDataMonster_Base` (byte-addressed: +0 game buffer 0, +4 game buffer 2,
/// +8 game buffer 1 (monster_baseConstructor)).
#[inline]
pub(crate) fn GDM(e: &Engine) -> Ptr {
    e.r32(G(e, pGlobalDataMonster_Base))
}
/// Current value of `pGlobalDataPoltergeist_Base` (the same block as [`GDM`]; +0x10 = "first
/// throwable not yet picked" flag set by Poltergeist_baseST_Initial).
#[inline]
pub(crate) fn GDP(e: &Engine) -> Ptr {
    e.r32(G(e, pGlobalDataPoltergeist_Base))
}
/// Per-build literal (see builds.rs).
#[inline]
pub(crate) fn k(e: &Engine, i: usize) -> u32 {
    builds::BUILDS[e.rt.cur_build()].k[i]
}
/// Per-build code shape group of a function (see builds.rs).
#[inline]
pub(crate) fn g(e: &Engine, i: usize) -> u8 {
    builds::BUILDS[e.rt.cur_build()].g[i]
}
/// The current build's sha8.
#[inline]
pub(crate) fn build_key(e: &Engine) -> &'static str {
    builds::BUILDS[e.rt.cur_build()].sha8
}

/// `*a |= bits`
#[inline]
pub(crate) fn or32(e: &mut Engine, a: Ptr, bits: u32) {
    let v = e.r32(a);
    e.w32(a, v | bits);
}
/// `*a &= mask`
#[inline]
pub(crate) fn and32(e: &mut Engine, a: Ptr, mask: u32) {
    let v = e.r32(a);
    e.w32(a, v & mask);
}

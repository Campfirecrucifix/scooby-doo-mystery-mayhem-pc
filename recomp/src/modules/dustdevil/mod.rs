//! `dustdevil`: the Ep3 (Frontier Land) dust devils. One shared C source (`Monster_Base.c`,
//! `DustDevil_Base.c`, `DustDevilGlowInit.c`, `DustDevilEpisodeThree.c`) compiled with a per-level file
//! (`DustDevilLevel<N>.c`) into one module per level pack.
//! Port of `Game Logic Modules/Ep3_FrontierLand/Characters/DUST_DEVIL/..`. Reference build:
//! dustdevilleveltwo. The 4 builds are listed in `tools/rustport/dustdevil_family.txt`; per-build
//! literals and code shapes are in [`builds`] (tools/rustport/gen_builds.py), the initialised tables in
//! [`maps`] (tools/rustport/gen_data_tables.py).
//!
//! The monster side of the Tome of Doom: a dust devil zig-zags along its waypoint path, chases the
//! leader hero within 10 m (CheckIfInSector), hits him (attack anim); hooked by the beam (0xd6/0x65)
//! it flees along its path and every 5 s turns to face the book (SCARED_ATTACK, 3 s), is pulled in
//! (0xd6/0x67: glow destroyed) and captured (0xd6/0x68). The glow is a ring of "dust_blur" sprites on
//! ten joints, a "glow" sprite pair and rock debris / dust emitters (DustDevilGlowInit.c). Level 5's
//! boss (dustdevillevelfiveboss) splits off two small dust devils at each of three stages and loads
//! the episode-end movie once captured.
//!
//! Module globals are shared by every instance (the engine swaps `pLocalData*` per instance), as on
//! the PS2. `pLocalDataMonster_Base` and `pLocalDataDustDevil_Base` point at the instance's one local
//! data block (GetLocalData grows and shares it), likewise the two global data pointers.
#![allow(non_snake_case, non_upper_case_globals, unused_variables, unused_mut, unused_assignments)]
#![allow(clippy::all)]

pub mod builds;
pub mod maps;

mod dustdevil_base;
mod dustdevil_glow_init;
mod dustdevil_particle;
mod level;
mod monster_base;

pub use dustdevil_base::*;
pub use dustdevil_glow_init::*;
pub use dustdevil_particle::*;
pub use level::*;
pub use monster_base::*;

use crate::engine::{Engine, Mem, Ptr};
use crate::logic::ModuleDef;

pub static MODULE: ModuleDef = ModuleDef {
    name: "dustdevil",
    data_size: DATA_SIZE,
    ctor: dustdevil_levelConstructor,
    dtor: dustdevil_levelDestructor,
    register: None,
    // the initialised data the code reads (goCustomDataDustDevil, TmpMonsterMap) is written per build
    // by dustdevil_build_init; string literals are interned with `e.cstr`.
    init: &[],
    build_init: Some(dustdevil_build_init),
};

/// Index of a build in [`builds::BUILDS`] by its directory name.
pub fn build_index(sha8: &str) -> Option<usize> {
    builds::BUILDS.iter().position(|b| b.sha8 == sha8)
}

fn dustdevil_build_init(m: &mut Mem, data: Ptr, build: usize) {
    for (i, &w) in maps::GO_CUSTOM_DATA_DUST_DEVIL[build].iter().enumerate() {
        m.w32(data + (goCustomDataDustDevil - 0x10000) + i as u32 * 4, w);
    }
    for (i, &w) in maps::TMP_MONSTER_MAP[build].iter().enumerate() {
        m.w32(data + (TmpMonsterMap - 0x10000) + i as u32 * 4, w);
    }
}

// ---- module globals ----
// The builds place their globals at different addresses (the two tables have per-level sizes); the
// image is private to the module, so the port uses one layout for every build: the tables at their
// largest size, then the pointers, then the boss build's own globals (bLoading, bHalfHealth and the
// static `moCameraParams` of DustDevilLevelFiveBoss_MessageFunction, all 0 in the image).
/// per-dust-devil data: records {object id, waypoint path (L+0x168), kill mask (L+0x40), sector
/// attribute mask (L+0xefc)} of 0x10 bytes; 112 bytes max
pub const goCustomDataDustDevil: u32 = 0x10000;
/// per-monster-type data: records of 0x3c bytes {type, speeds and ranges...}; 240 bytes max
pub const TmpMonsterMap: u32 = 0x10070;
pub const pAnimMngr: u32 = 0x10160;
pub const pFunction: u32 = 0x10164;
pub const pMiscFunc: u32 = 0x10168;
pub const pDebug: u32 = 0x1016c;
pub const pExtra: u32 = 0x10170;
pub const pGameObj: u32 = 0x10174;
pub const pPhysic: u32 = 0x10178;
pub const pParticle: u32 = 0x1017c;
pub const pMath: u32 = 0x10180;
pub const pStdlib: u32 = 0x10184;
pub const pLight: u32 = 0x10188;
pub const pSound: u32 = 0x1018c;
pub const pGraphic: u32 = 0x10190;
pub const pGame: u32 = 0x10194;
pub const pAnim: u32 = 0x10198;
pub const pLocalDataMonster_Base: u32 = 0x1019c;
pub const pGlobalDataMonster_Base: u32 = 0x101a0;
pub const pLocalDataDustDevil_Base: u32 = 0x101a4;
pub const pGlobalDataDustDevil_Base: u32 = 0x101a8;
/// (dustdevillevelfiveboss) the boss was captured: the next 0x4f loads the end-of-episode movie
pub const bLoading: u32 = 0x101ac;
/// (dustdevillevelfiveboss) the half-health message 0x3a was sent to the HUD
pub const bHalfHealth: u32 = 0x101b0;
/// (dustdevillevelfiveboss) static `moCameraParams` (0x4c-byte stCameraParams, C .text+0x128) sent
/// with message 0x54 to the camera (Camera_BaseFUNC_SetCameraUser reads P[0..0x12]).
pub const moCameraParams_198: u32 = 0x101b4;
pub const DATA_SIZE: u32 = 0x101b4 + 0x4c - 0x10000;

/// `ALL` joint / bone selector (0x7fffffff)
pub(crate) const ALL: u32 = 0x7fff_ffff;

/// Address of a module global.
#[inline]
pub(crate) fn G(e: &Engine, a: u32) -> Ptr {
    e.rt.cur_data() + (a - 0x10000)
}
/// Current value of `pLocalDataDustDevil_Base` (this instance's local data).
#[inline]
pub(crate) fn L(e: &Engine) -> Ptr {
    e.r32(G(e, pLocalDataDustDevil_Base))
}
/// Current value of `pLocalDataMonster_Base` (the same block as [`L`] once both are registered).
#[inline]
pub(crate) fn LM(e: &Engine) -> Ptr {
    e.r32(G(e, pLocalDataMonster_Base))
}
/// Current value of `pGlobalDataMonster_Base` (byte-addressed: +0 game buffer 0, +4 game buffer 2,
/// +8 game buffer 1 (monster_baseConstructor), +0xc / +0x10 the Debris / Poussiere particle types).
#[inline]
pub(crate) fn GDM(e: &Engine) -> Ptr {
    e.r32(G(e, pGlobalDataMonster_Base))
}
/// Current value of `pGlobalDataDustDevil_Base` (the same block as [`GDM`]).
#[inline]
pub(crate) fn GDD(e: &Engine) -> Ptr {
    e.r32(G(e, pGlobalDataDustDevil_Base))
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
/// The current build's directory name.
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

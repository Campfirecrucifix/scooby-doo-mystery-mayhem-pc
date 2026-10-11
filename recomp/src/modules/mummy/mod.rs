//! `mummy`: the Ep2 mummy family (5 builds: mummyleveltwoa / twob / twoc, mummylevelthreea,
//! mummylevelnine). Port of `Game Logic Modules/Ep2_MovieStudio/Characters/MUMMY/<level>/<build>`
//! (Monster_Base.c, Mummy_Base.c, Particle.c, Particle_Func.c, MummyEpisodeTwo.c, MummyLevel<N>.c).
//!
//! Global data: Monster_Base and Mummy_Base share one global block (GetGlobalData grows it):
//! monster_baseConstructor stores game buffer 0 at +0 and game buffer 1 at +8, mummy_baseConstructor
//! game buffer 0 at +0, game buffer 2 at +4 and 0 at +0xc. Local data: one block (0x160 bytes asked by
//! Monster_Base, grown to 0x330 by Mummy_Base); `pLocalDataMonster_Base == pLocalDataMummy_Base`.
//!
//! The capture: hooked by the book (0xd6/0x65) the mummy flees to its sarcophagus or away from the
//! book; pulled in (0xd6/0x67) it hides and launches the binding latent (`mummy_binding`, which plays
//! the capture anim); Monster_Base then handles the release / capture bookkeeping.
#![allow(non_snake_case, non_upper_case_globals, unused_variables, unused_mut, unused_assignments)]
#![allow(clippy::all)]

pub mod builds;
pub mod maps;

mod level;
mod monster_base;
pub mod mummy_base;
mod particle;
mod particle_func;

pub use level::*;
pub use monster_base::*;
pub use mummy_base::*;
pub use particle::*;
pub use particle_func::*;

use crate::engine::{Engine, Mem, Ptr};
use crate::logic::ModuleDef;

pub static MODULE: ModuleDef = ModuleDef {
    name: "mummy",
    data_size: DATA_SIZE,
    ctor: mummy_levelConstructor,
    dtor: mummy_levelDestructor,
    register: None,
    // goCustomDataMummy / TmpMonsterMap are written per build by mummy_build_init; string literals
    // are interned with `e.cstr`.
    init: &[],
    build_init: Some(mummy_build_init),
};

/// Index of a build in [`builds::BUILDS`] by its directory name.
pub fn build_index(sha8: &str) -> Option<usize> {
    builds::BUILDS.iter().position(|b| b.sha8 == sha8)
}

fn mummy_build_init(m: &mut Mem, data: Ptr, build: usize) {
    for (i, &w) in maps::GO_CUSTOM_DATA_MUMMY[build].iter().enumerate() {
        m.w32(data + (goCustomDataMummy - 0x10000) + i as u32 * 4, w);
    }
    for (i, &w) in maps::TMP_MONSTER_MAP[build].iter().enumerate() {
        m.w32(data + (TmpMonsterMap - 0x10000) + i as u32 * 4, w);
    }
}

// ---- module globals ----
// The builds place TmpMonsterMap at different addresses (goCustomDataMummy has per-level sizes); the
// image is private to the module, so the port uses one layout for every build: the tables at their
// largest size, then the pointers in _globals.h order.
/// per-mummy records of 0x50 bytes {id, sarcophagus, behaviour, kill mask, has a path, path name[0x3c]};
/// 480 bytes max
pub const goCustomDataMummy: u32 = 0x10000;
/// per-monster-type records of 0x3c bytes; 300 bytes max
pub const TmpMonsterMap: u32 = 0x101e0;
pub const pAnimMngr: u32 = 0x10310;
pub const pFunction: u32 = 0x10314;
pub const pMiscFunc: u32 = 0x10318;
pub const pDebug: u32 = 0x1031c;
pub const pAnim: u32 = 0x10320;
pub const pGameObj: u32 = 0x10324;
pub const pPhysic: u32 = 0x10328;
pub const pExtra: u32 = 0x1032c;
pub const pParticle: u32 = 0x10330;
pub const pMath: u32 = 0x10334;
pub const pStdlib: u32 = 0x10338;
pub const pPathFinding: u32 = 0x1033c;
pub const pSound: u32 = 0x10340;
pub const pSectorClip: u32 = 0x10344;
pub const pLocalDataMonster_Base: u32 = 0x10348;
pub const pGlobalDataMonster_Base: u32 = 0x1034c;
pub const pLocalDataMummy_Base: u32 = 0x10350;
pub const pGlobalDataMummy_Base: u32 = 0x10354;
pub const DATA_SIZE: u32 = 0x358;

/// `ALL` joint / bone selector (0x7fffffff)
pub(crate) const ALL: u32 = 0x7fff_ffff;

/// Address of a module global.
#[inline]
pub(crate) fn G(e: &Engine, a: u32) -> Ptr {
    e.rt.cur_data() + (a - 0x10000)
}
/// Current value of `pLocalDataMummy_Base` (this instance's local data).
#[inline]
pub(crate) fn L(e: &Engine) -> Ptr {
    e.r32(G(e, pLocalDataMummy_Base))
}
/// Current value of `pLocalDataMonster_Base` (the same block as [`L`]).
#[inline]
pub(crate) fn LM(e: &Engine) -> Ptr {
    e.r32(G(e, pLocalDataMonster_Base))
}
/// Current value of `pGlobalDataMonster_Base` (+0 game buffer 0, +4 game buffer 2, +8 game buffer 1).
#[inline]
pub(crate) fn GDM(e: &Engine) -> Ptr {
    e.r32(G(e, pGlobalDataMonster_Base))
}
/// Current value of `pGlobalDataMummy_Base` (the same block as [`GDM`]).
#[inline]
pub(crate) fn GDMu(e: &Engine) -> Ptr {
    e.r32(G(e, pGlobalDataMummy_Base))
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
/// `*a &= mask`
#[inline]
pub(crate) fn and32(e: &mut Engine, a: Ptr, mask: u32) {
    let v = e.r32(a);
    e.w32(a, v & mask);
}

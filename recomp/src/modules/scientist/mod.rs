//! `scientist`: the Ep6 scientists (2 builds: scientist__c66ebe98 = EP6L02, scientist__955aa35f =
//! EP6L09). Port of `Game Logic Modules/Ep6_ShermanTech/Characters/Scientist/<level>/<build>`
//! (Monster_Base.c, Scientist_Base.c, ScientistEpisodeSix.c, Scientist.c).
//!
//! A scientist stands, walks a waypoint path, hears / follows / spots the leader hero and throws its
//! weapon latent at him (the latent is parented to the hand and told 0xdc, then 0xd8 on key frame 0xb).
//! Scared (0xd8 in L09 / message 0x1d0 / ST_ENTRY_POINT event 0x4000) it falls asleep. It is a
//! Monster_Base monster (type 0xe; L02 also has the type 0xc record), so the book handling is the
//! shared Monster_Base code. L02 adds the key-card / captured-scientist puzzle and force fields.
//!
//! Global data: Monster_Base and Scientist_Base share one block (Monster_Base: +0 game buffer 0, +8 game
//! buffer 1; Scientist_Base: +0 game buffer 0, +4 game buffer 2, +0xc 0). Local data: one block (0x160
//! asked by Monster_Base, grown to 0x284); `pLocalDataMonster_Base == pLocalDatascientist_base`.
#![allow(non_snake_case, non_upper_case_globals, unused_variables, unused_mut, unused_assignments)]
#![allow(clippy::all)]

pub mod builds;
pub mod maps;

mod monster_base;
pub mod scientist;
pub mod scientist_base;

pub use monster_base::*;
pub use scientist::*;
pub use scientist_base::*;

use crate::engine::{Engine, Mem, Ptr};
use crate::logic::ModuleDef;

pub static MODULE: ModuleDef = ModuleDef {
    name: "scientist",
    data_size: DATA_SIZE,
    ctor: scientistConstructor,
    dtor: scientistDestructor,
    register: None,
    // goCustomDataScientist / TmpMonsterMap are written per build by scientist_build_init
    init: &[],
    build_init: Some(scientist_build_init),
};

/// Index of a build in [`builds::BUILDS`] by its directory name.
pub fn build_index(sha8: &str) -> Option<usize> {
    builds::BUILDS.iter().position(|b| b.sha8 == sha8)
}

fn scientist_build_init(m: &mut Mem, data: Ptr, build: usize) {
    for (i, &w) in maps::GO_CUSTOM_DATA_SCIENTIST[build].iter().enumerate() {
        m.w32(data + (goCustomDataScientist - 0x10000) + i as u32 * 4, w);
    }
    for (i, &w) in maps::TMP_MONSTER_MAP[build].iter().enumerate() {
        m.w32(data + (TmpMonsterMap - 0x10000) + i as u32 * 4, w);
    }
}

// ---- module globals ----
// One layout for both builds (the image is private to the module): the tables at their largest size,
// then the pointers in _globals.h order (c66ebe98's addresses).
/// per-scientist records of 0x58 bytes {id, path waypoint set -> L+0x1a4, -> L+0x1a0, -> L+0x170,
/// -> L+0x16c, -> L+0x1c0, has a path-finding path -> L+0x214, path name[0x3c] -> L+0x1d8}; 4 records
pub const goCustomDataScientist: u32 = 0x10000;
/// per-monster-type records of 0x3c bytes; 2 records max
pub const TmpMonsterMap: u32 = 0x10160;
pub const pAnimMngr: u32 = 0x101d8;
pub const pFunction: u32 = 0x101dc;
pub const pMiscFunc: u32 = 0x101e0;
pub const pDebug: u32 = 0x101e4;
pub const pAnim: u32 = 0x101e8;
pub const pGameObj: u32 = 0x101ec;
pub const pPhysic: u32 = 0x101f0;
pub const pExtra: u32 = 0x101f4;
pub const pParticle: u32 = 0x101f8;
pub const pMath: u32 = 0x101fc;
pub const pStdlib: u32 = 0x10200;
pub const pSound: u32 = 0x10204;
pub const pPathFinding: u32 = 0x10208;
pub const pLocalDataMonster_Base: u32 = 0x1020c;
pub const pGlobalDataMonster_Base: u32 = 0x10210;
pub const pLocalDatascientist_base: u32 = 0x10214;
pub const pGlobalDatascientist_base: u32 = 0x10218;
pub const DATA_SIZE: u32 = 0x220;

/// `ALL` joint / bone selector (0x7fffffff)
pub(crate) const ALL: u32 = 0x7fff_ffff;

/// Address of a module global.
#[inline]
pub(crate) fn G(e: &Engine, a: u32) -> Ptr {
    e.rt.cur_data() + (a - 0x10000)
}
/// Current value of `pLocalDatascientist_base` (this instance's local data).
#[inline]
pub(crate) fn L(e: &Engine) -> Ptr {
    e.r32(G(e, pLocalDatascientist_base))
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
/// Current value of `pGlobalDatascientist_base` (the same block as [`GDM`]).
#[inline]
pub(crate) fn GDS(e: &Engine) -> Ptr {
    e.r32(G(e, pGlobalDatascientist_base))
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

/// `if (pDebug != NULL) (*pDebug->PrintDebugInfo)("Scientist", msg);`
pub(crate) fn debug_info(e: &mut Engine, msg: &str) {
    if e.r32(G(e, pDebug)) != 0 {
        let n = e.cstr("Scientist");
        let m = e.cstr(msg);
        crate::engine::LIBdebug::PrintDebugInfo(e, n, m);
    }
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

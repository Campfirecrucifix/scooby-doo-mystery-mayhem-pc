//! `hero_ghost`: Scooby / Shaggy in the ghost costume (costume chest transformation). Port of
//! `Game Logic Modules/Ep1_Library/Characters/{ScoobyGhost,ShaggyGhost}/<Level>/<scooby|shaggy>_ghost__<sha8>/`
//! (8 builds, EP1 L01/L02/L03A/L03B). Reference build: scooby_ghost__08a011d0.
//!
//! The ghost modules are an older revision of the hero source (`hero_*.c`, no book, crouch, ladder,
//! edge or cut-scene code) compiled per character (`scooby_ghost.c` / `shaggy_ghost.c`). Per-build
//! literals and code shapes: [`builds`] (tools/rustport/gen_builds.py over hero_ghost_family.txt; the
//! debug-print character name "ScoobyGhost"/"ShaggyGhost" is normalised so both characters share one
//! shape). Functions whose code is token-identical to the hero reference (scooby__402420b4) were
//! transplanted from the hero port by tools/rustport/hero_ghost_transplant.py with the literal slots
//! remapped; the others are ported from the ghost source.
#![allow(non_snake_case, non_upper_case_globals, unused_variables, unused_mut, unused_assignments)]
#![allow(clippy::all)]

pub mod builds;
pub mod maps;
pub mod skeleton;

mod hero_actions;
mod hero_blend;
mod hero_callbacks;
mod hero_constructor;
mod hero_core;
mod hero_follow;
mod hero_inputs;
mod hero_misc;
mod hero_movements;
mod hero_msgfunc;
mod hero_particles;
mod hero_scared;
mod hero_sounds;
mod hero_stands;
mod hero_switch;
mod scooby_ghost;

pub use hero_actions::*;
pub use hero_blend::*;
pub use hero_callbacks::*;
pub use hero_constructor::*;
pub use hero_core::*;
pub use hero_follow::*;
pub use hero_inputs::*;
pub use hero_misc::*;
pub use hero_movements::*;
pub use hero_msgfunc::*;
pub use hero_particles::*;
pub use hero_scared::*;
pub use hero_sounds::*;
pub use hero_stands::*;
pub use hero_switch::*;
pub use scooby_ghost::*;
pub use skeleton::globals::{
    goGenericSoundScoobyGhost, goSurfaceSoundScoobyGhost, gpoLanguagesNamesAndISOCodes, pAnim, pAnimMngr, pDebug, pExtra,
    pFunctions, pGame, pGameObj, pGlobalDataHero, pGraphics, pHud, pInput, pLocalDataHero, pMath, pMiscFunc, pParticle,
    pPhysic, pSndMngr, pSound, pStdlib, pText, TmpSpeedMap,
};
pub use skeleton::{addr, call_code};

use crate::engine::{Engine, Mem, Ptr, LIBdebug};
use crate::logic::ModuleDef;

// ---- data layout ----
// The module image is the reference build's (skeleton.rs) plus an extension: every build's own
// initialised tables are written by `build_init` at the reference addresses (code reaches them only
// by name). goNextRoomInfo (up to 336 bytes) does not fit the reference build's 304-byte slot and
// moves to the extension; the Shaggy builds' sound tables use the Scooby slots (same sizes).
/// `goNextRoomInfo`: relocated to the image extension (336 bytes)
pub const goNextRoomInfo: u32 = 0x10000 + skeleton::DATA_SIZE;
pub const goSurfaceSoundShaggyGhost: u32 = goSurfaceSoundScoobyGhost;
pub const goGenericSoundShaggyGhost: u32 = goGenericSoundScoobyGhost;
const DATA_EXT: u32 = 0x150;

fn build_init(m: &mut Mem, data: Ptr, build: usize) {
    skeleton::write_init(m, data); // the reference image (formerly ModuleDef::init)
    let put = |m: &mut Mem, at: u32, slot: u32, w: &[u32]| {
        assert!(w.len() as u32 * 4 <= slot, "hero_ghost table does not fit its slot");
        for i in 0..slot / 4 {
            m.w32(data + (at - 0x10000) + i * 4, *w.get(i as usize).unwrap_or(&0));
        }
    };
    let pick = |a: &'static [u32], b: &'static [u32]| if a.is_empty() { b } else { a };
    put(m, goNextRoomInfo, DATA_EXT, maps::GO_NEXT_ROOM_INFO[build]);
    put(m, TmpSpeedMap, 0x60, maps::TMP_SPEED_MAP[build]);
    put(m, goSurfaceSoundScoobyGhost, 0x258, pick(maps::GO_SURFACE_SOUND_SCOOBY_GHOST[build], maps::GO_SURFACE_SOUND_SHAGGY_GHOST[build]));
    put(m, goGenericSoundScoobyGhost, 0xc0, pick(maps::GO_GENERIC_SOUND_SCOOBY_GHOST[build], maps::GO_GENERIC_SOUND_SHAGGY_GHOST[build]));
}

pub static MODULE: ModuleDef = ModuleDef {
    name: "scooby_ghost",
    data_size: skeleton::DATA_SIZE + DATA_EXT,
    ctor: ghostConstructor,
    dtor: ghostDestructor,
    register: None,
    init: &[],
    build_init: Some(build_init),
};

/// `0x7fffffff`: the whole object (no bone) in GetPosition / GetLookAt / TakeOrientation.
pub(crate) const ALL: u32 = 0x7fff_ffff;

/// Index of a build in [`builds::BUILDS`] by the first 8 hex digits of its SHA-1.
pub fn build_index(sha8: &str) -> Option<usize> {
    builds::BUILDS.iter().position(|b| b.sha8 == sha8)
}

/// Address of a module global.
#[inline]
pub(crate) fn G(e: &Engine, a: u32) -> Ptr {
    e.rt.cur_data() + (a - 0x10000)
}
/// Current value of `pLocalDataHero` (this instance's local data).
#[inline]
pub(crate) fn L(e: &Engine) -> Ptr {
    e.r32(G(e, pLocalDataHero))
}
/// Current value of `pGlobalDataHero` (an array of game-buffer pointers).
#[inline]
pub(crate) fn GD(e: &Engine) -> Ptr {
    e.r32(G(e, pGlobalDataHero))
}
/// `*pGlobalDataHero`: game buffer 0
#[inline]
pub(crate) fn GB0(e: &Engine) -> Ptr {
    e.r32(GD(e))
}
/// `pGlobalDataHero[2]`: game buffer 2
#[inline]
pub(crate) fn GB2(e: &Engine) -> Ptr {
    e.r32(GD(e) + 8)
}
/// Shaggy build (shaggy_ghost.c), else Scooby (scooby_ghost.c).
#[inline]
pub(crate) fn is_shaggy(e: &Engine) -> bool {
    g(e, builds::G_shaggy_ghostConstructor) == 0
}
/// `pDebug != NULL` (every ghost build has its debug prints compiled in)
#[inline]
pub(crate) fn dprint(e: &Engine) -> bool {
    e.r32(G(e, pDebug)) != 0
}
/// The character name the build passes to its debug prints, interned.
pub(crate) fn char_name(e: &mut Engine) -> Ptr {
    let n = if is_shaggy(e) { "ShaggyGhost" } else { "ScoobyGhost" };
    e.cstr(n)
}
/// `if (pDebug != NULL) (*pDebug->PrintDebugInfo)(<char name>, msg)`
pub(crate) fn debug_info(e: &mut Engine, msg: &str) {
    if dprint(e) {
        let n = char_name(e);
        let m = e.cstr(msg);
        LIBdebug::PrintDebugInfo(e, n, m);
    }
}
/// `if (pDebug != NULL) (*pDebug->PrintDebugInfoWithValue)(<char name>, msg, value, kind)`
pub(crate) fn debug_info_value(e: &mut Engine, msg: &str, value: Ptr, kind: u32) {
    if dprint(e) {
        let n = char_name(e);
        let m = e.cstr(msg);
        LIBdebug::PrintDebugInfoWithValue(e, n, m, value, kind);
    }
}
/// Per-build literal (see builds.rs).
#[inline]
pub(crate) fn k(e: &Engine, i: usize) -> u32 {
    builds::BUILDS[e.rt.cur_build()].k[i]
}
/// Per-build literal as f32.
#[inline]
pub(crate) fn kf(e: &Engine, i: usize) -> f32 {
    f32::from_bits(k(e, i))
}
/// Per-build code shape group of a function (see builds.rs).
#[inline]
pub(crate) fn g(e: &Engine, i: usize) -> u8 {
    builds::BUILDS[e.rt.cur_build()].g[i]
}
/// Module entry points: scooby_ghost.c or shaggy_ghost.c by build.
fn ghostConstructor(e: &mut Engine, api: Ptr, n: i32) -> crate::logic::LResult<()> {
    if is_shaggy(e) { shaggy_ghostConstructor(e, api, n) } else { scooby_ghostConstructor(e, api, n) }
}
fn ghostDestructor(e: &mut Engine, n: i32) -> crate::logic::LResult<()> {
    if is_shaggy(e) { shaggy_ghostDestructor(e, n) } else { scooby_ghostDestructor(e, n) }
}
/// `call_code` fallback: the shaggy_ghost entry points are not in the reference build.
pub(crate) fn call_code_extra(e: &mut Engine, f: u32, a0: Ptr) -> crate::logic::LResult<()> {
    panic!("call through code pointer {f:#x}: not a function of this module")
}
#[cold]
pub(crate) fn unported_shape(e: &Engine, f: &str, gi: usize) -> ! {
    let b = &builds::BUILDS[e.rt.cur_build()];
    panic!("{f}: code shape group {} of build {} is not ported yet", b.g[gi], b.sha8)
}

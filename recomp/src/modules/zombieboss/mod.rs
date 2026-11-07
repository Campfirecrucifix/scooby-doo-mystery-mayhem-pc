//! `zombieboss`: the Ep4 zombie boss (EP4L04). Port of
//! `Game Logic Modules/Globals/Characters/ZOMBIE_BOSS/zombieboss` (Monster_Base.c, ZombieBoss.c,
//! ZombieBossParticle.c, ZombieBossParticle_Func.c). One build.
//!
//! The fight: the boss chases the leader hero (path finding through the arena graph), gasses him
//! (anim 5) and stomps (anim 2: shock-wave latent 0x2a8000be, stomp gas and dirt particles, camera
//! shake, rumble; a stomp while the book is open knocks it shut, msg 0x92 to the hero). With the book
//! open nearby he keeps his distance; once hooked (0xd6/0x65) he struggles: the book reports its pull
//! strength (0x125 -> L+0x1fc) and twice he breaks free when it falls to 200 / 100 (L+0x198 phases,
//! each new phase drops wisps through 0x2a0000c3 to refill the book). The third time he is pulled in
//! (0xd6/0x67): the boss hides and hands over to the binding latent 0x2a8000bd
//! (`zombieboss_binding`), which plays the capture; 0xd6/0x68 then queues the Ep4 end movie (0x4f).
//!
//! Global data: Monster_Base and ZombieBoss share one global block (GetGlobalData grows it):
//! monster_baseConstructor stores game buffer 0 at +0 and game buffer 1 at +8, zombiebossConstructor
//! game buffer 0 at +0 and game buffer 2 at +4. Local data: one block (0x160 bytes asked by
//! Monster_Base, grown to 0x280 by ZombieBoss); `pLocalDataMonster_Base == pLocalDataZombieBoss`.
//!
//! ZombieBoss keeps its sub-state as a CODE POINTER at L+0x178 (see zombieboss::addr / call_state).
#![allow(non_snake_case, non_upper_case_globals, unused_variables, unused_mut, unused_assignments)]
#![allow(clippy::all)]

mod monster_base;
pub mod zombieboss;
mod zombieboss_particle;
mod zombieboss_particle_func;

pub use monster_base::*;
pub use zombieboss::*;
pub use zombieboss_particle::*;
pub use zombieboss_particle_func::*;

use crate::engine::{Engine, Ptr};
use crate::logic::ModuleDef;

pub static MODULE: ModuleDef = ModuleDef {
    name: "zombieboss",
    data_size: DATA_SIZE,
    ctor: zombiebossConstructor,
    dtor: zombiebossDestructor,
    register: None,
    // TmpMonsterMap (one record: type 0xc), read from the module on the disc (WP-X4). The 0x4f movie
    // parameters (DAT_00010f18) are built on the stack by ZombieBossMessageFonction; strings use `e.cstr`.
    init: &[],
    build_init: Some(|m, d, _| crate::engine::gamedata::init_syms(m, d, "a064c25c", &["TmpMonsterMap"])),
};

/// The only build.
pub fn build_index(key: &str) -> Option<usize> {
    (key == "zombieboss").then_some(0)
}

// ---- module globals (module-relative addresses from _globals.h) ----
/// one 0x3c-byte record {type, speeds and ranges...}
pub const TmpMonsterMap: u32 = 0x10000;
pub const pAnimMngr: u32 = 0x1003c;
pub const pFunction: u32 = 0x10040;
pub const pMiscFunc: u32 = 0x10044;
pub const pDebug: u32 = 0x10048;
pub const pAnim: u32 = 0x1004c;
pub const pGameObj: u32 = 0x10050;
pub const pPhysic: u32 = 0x10054;
pub const pExtra: u32 = 0x10058;
pub const pParticle: u32 = 0x1005c;
pub const pMath: u32 = 0x10060;
pub const pStdlib: u32 = 0x10064;
pub const pInput: u32 = 0x10068;
pub const pPathFinding: u32 = 0x1006c;
pub const pGame: u32 = 0x10070;
pub const pSound: u32 = 0x10074;
/// `moCameraParams.174` (static of ZombieBossMessageFonction, camera parameters of msg 0x54)
pub const moCameraParams_174: u32 = 0x10078;
pub const pLocalDataMonster_Base: u32 = 0x100c4;
pub const pGlobalDataMonster_Base: u32 = 0x100c8;
pub const pLocalDataZombieBoss: u32 = 0x100cc;
pub const pGlobalDataZombieBoss: u32 = 0x100d0;
/// End of the globals the code uses. The image also holds gpoLanguagesNamesAndISOCodes (0x100e0,
/// 3243 bytes, from a shared header), which no zombieboss function reads: not carried over.
pub const DATA_SIZE: u32 = 0xe0;

/// `ALL` joint / bone selector (0x7fffffff)
pub(crate) const ALL: u32 = 0x7fff_ffff;

/// Address of a module global.
#[inline]
pub(crate) fn G(e: &Engine, a: u32) -> Ptr {
    e.rt.cur_data() + (a - 0x10000)
}
/// Current value of `pLocalDataZombieBoss` (this instance's local data, 0x280 bytes).
#[inline]
pub(crate) fn L(e: &Engine) -> Ptr {
    e.r32(G(e, pLocalDataZombieBoss))
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
/// Current value of `pGlobalDataZombieBoss` (the same block as [`GDM`]).
#[inline]
pub(crate) fn GDZ(e: &Engine) -> Ptr {
    e.r32(G(e, pGlobalDataZombieBoss))
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

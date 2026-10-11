//! `fireghost`: the fire ghosts of Ep6 (levels 4A, 4B and 5). One shared C source (`Monster_Base.c`,
//! `FireGhost_Base.c`, `FireGhost_Base_particle.c`) compiled with a per-level file (`FireGhost.c`)
//! into one module per level pack.
//! Port of `Game Logic Modules/Globals/Characters/FIRE_GHOST/..`. Reference build: 6f63c7c0
//! (EP6L04A); 3a7b55ab = EP6L05, 818e89e1 = EP6L04B. The builds only differ in literals (object
//! ids, anims, sounds, timer periods, ranges), which the port keeps in [`Cfg`] (one per build,
//! in [`builds::BUILDS`] order); [`builds`] also carries the generated shape table, [`maps`] the
//! initialised tables (tools/rustport/gen_data_tables.py).
//!
//! A fire ghost is a burning shell (the "fire" object) around an ordinary ghost (object id in its
//! goCustomDataFireGhost record, kept at L+0x1a4): it floats along its waypoint path (bobbing,
//! fireGhostBaseFUNC_Wave), is surprised when the leader hero comes within L+0x1dc, turns towards
//! him and throws a fire latent (msg 0x1af, released by the attack anim event: 0x110). Water
//! (msgs 0xd8 / 0x1a9) puts the fire out: the shell is moved away to (35, 35, 35) and the ghost
//! inside is put in its place in state 0xe, so the ghost (ghost module) is what the Tome of Doom
//! captures. Unless the ghost has been captured (msg 0x68 to the shell), the fire timer L+0x1e8
//! relights the shell on the ghost's position (fireGhostBaseFUNC_CatchOnFire), with smoke
//! particles while extinguished. The level-5 builds' type-0x12 fire ghosts wait (external state 0)
//! until msg 0x1b2 gives them a spawn position (at most 3 at a time: L+0x1d0).
//!
//! The fire ghost replaces Monster_Base's message function with its own (fireGhostFUNC_MessageHandler
//! does not chain to Monster_Base_MessageFunction): Monster_Base's capture code is compiled in but
//! never runs. FireGhost_Base keeps its sub-state as a CODE POINTER at L+0x160 (see
//! fireghost_base::addr / call_state, as in the poltergeist port).
//!
//! Global data: Monster_Base and FireGhost_Base share one global block (GetGlobalData grows it):
//! monster_baseConstructor stores game buffer 0 at +0 and game buffer 1 at +8, then
//! fireghost_baseConstructor stores game buffer 0 at +4 and game buffer 2 at +8 -- overwriting
//! Monster_Base's game buffer 1. This is what the PS2 does (Monster_Base code that reads +4 / +8
//! as game buffers 2 / 1 only runs in its unused message function) and the port keeps it.
#![allow(non_snake_case, non_upper_case_globals, unused_variables, unused_mut, unused_assignments)]
#![allow(clippy::all)]

pub mod builds;
pub mod maps;

pub mod fireghost_base;
mod fireghost_base_particle;
mod level;
mod monster_base;

pub use fireghost_base::*;
pub use fireghost_base_particle::*;
pub use level::*;
pub use monster_base::*;

use crate::engine::{Engine, Mem, Ptr};
use crate::logic::ModuleDef;

pub static MODULE: ModuleDef = ModuleDef {
    name: "fireghost",
    data_size: DATA_SIZE,
    ctor: fireghostConstructor,
    dtor: fireghostDestructor,
    register: None,
    // goCustomDataFireGhost and TmpMonsterMap are written per build by fireghost_build_init;
    // string literals are interned with `e.cstr`.
    init: &[],
    build_init: Some(fireghost_build_init),
};

/// Index of a build in [`builds::BUILDS`] by its sha8.
pub fn build_index(sha8: &str) -> Option<usize> {
    builds::BUILDS.iter().position(|b| b.sha8 == sha8)
}

fn fireghost_build_init(m: &mut Mem, data: Ptr, build: usize) {
    for (i, &w) in maps::GO_CUSTOM_DATA_FIRE_GHOST[build].iter().enumerate() {
        m.w32(data + (goCustomDataFireGhost - 0x10000) + i as u32 * 4, w);
    }
    for (i, &w) in maps::TMP_MONSTER_MAP[build].iter().enumerate() {
        m.w32(data + (TmpMonsterMap - 0x10000) + i as u32 * 4, w);
    }
}

// ---- module globals ----
// The reference build's layout (its tables have the largest size): every build uses it.
/// per-fire-ghost records of 0x18 bytes {object id (L+0x198), +4, waypoint path (L+0x1f0), kill mask
/// (L+0x19c), ghost inside (L+0x1a4), +0x14}; 192 bytes max (8 records)
pub const goCustomDataFireGhost: u32 = 0x10000;
/// per-monster-type data: records of 0x3c bytes {type, speeds and ranges...}; 240 bytes max
pub const TmpMonsterMap: u32 = 0x100c0;
pub const pAnimMngr: u32 = 0x101b0;
pub const pFunction: u32 = 0x101b4;
pub const pMiscFunc: u32 = 0x101b8;
pub const pDebug: u32 = 0x101bc;
pub const pExtra: u32 = 0x101c0;
pub const pGameObj: u32 = 0x101c4;
pub const pPhysic: u32 = 0x101c8;
pub const pStdlib: u32 = 0x101cc;
pub const pSound: u32 = 0x101d0;
pub const pAnim: u32 = 0x101d4;
pub const pMath: u32 = 0x101d8;
pub const pParticle: u32 = 0x101dc;
pub const pLocalDataFireGhostBase: u32 = 0x101e0;
pub const pGlobalDataFireGhostBase: u32 = 0x101e4;
pub const pLocalDataMonster_Base: u32 = 0x101e8;
pub const pGlobalDataMonster_Base: u32 = 0x101ec;
pub const DATA_SIZE: u32 = 0x1f0;

/// `ALL` joint / bone selector (0x7fffffff)
pub(crate) const ALL: u32 = 0x7fff_ffff;

/// The literals that differ between the three builds (everything else is identical code).
pub struct Cfg {
    // ---- Monster_Base.c ----
    /// TmpMonsterMap records scanned by Monster_Base_Func_Initialise
    pub mb_records: u32,
    /// L+0xec (line-of-sight range) of Monster_Base_Func_Initialise / InitInLineOfSight
    pub mb_sight: u32,
    /// receiver of 0x14e (Monster_BaseST_CAPTURED)
    pub mb_manager: u32,
    /// the suck-beam latent (0xd6/0x65)
    pub mb_suck_latent: u32,
    /// the book tracked when pulled in (0xd6/0x67)
    pub mb_book: u32,
    /// receiver of 0xea / 0xeb (also sent by FireGhost_Base)
    pub ea_target: u32,
    /// move-DM targets re-pointed at the new leader (0xcd)
    pub mb_retarget: &'static [u32],
    // ---- FireGhost_Base.c ----
    /// sound of fireGhostBaseST_INITIAL (non-0x12 types)
    pub spawn_sound: u32,
    /// fireGhostBaseFUNC_Extinguishe plays sound 0x80000009 on object 0x2a000017 (EP6L05 only)
    pub extinguish_sound: bool,
    pub anim_search: i32,
    /// the thrown fire latent
    pub attack_latent: u32,
    pub anim_attack: i32,
    pub anim_scared: i32,
    pub anim_scared_loop: i32,
    /// turn DM speed of fireGhostBaseST_Scared_Execute
    pub scared_turn: u32,
    /// turn DM speed of fireGhostBaseFUNC_CreatePhysic
    pub turn_speed: u32,
    /// relight timer L+0x1e8 period (s)
    pub relight_period: f32,
    /// scared timer L+0x1e4 period (s)
    pub scared_period: f32,
    /// goCustomDataFireGhost records scanned by Monster_BaseFUNC_FillLocalDataParam
    pub fg_records: u32,
    /// L+0x1e0 and L+0x1dc/0x1d8/0x1d4 written by Monster_BaseFUNC_FillLocalDataParam
    pub fill_ranges: (u32, u32),
}

/// Per-build literals, in [`builds::BUILDS`] order (3a7b55ab, 6f63c7c0, 818e89e1).
pub static CFG: [Cfg; 3] = [
    // 3a7b55ab (EP6L05)
    Cfg {
        mb_records: 2,
        mb_sight: 0x4130_0000,
        mb_manager: 0x2a00_000c,
        mb_suck_latent: 0x2a80_009c,
        mb_book: 0x2a00_0009,
        ea_target: 0x2a00_009a,
        mb_retarget: &[0x2a00_0007, 0x2a00_0008],
        spawn_sound: 0x4000_0028,
        extinguish_sound: true,
        anim_search: 3,
        attack_latent: 0x2a80_0011,
        anim_attack: 4,
        anim_scared: 5,
        anim_scared_loop: 6,
        scared_turn: 0x40e0_0000,
        turn_speed: 0x40e0_0000,
        relight_period: 4.0,
        scared_period: 3.0,
        fg_records: 2,
        fill_ranges: (0x4140_0000, 0x4130_0000),
    },
    // 6f63c7c0 (EP6L04A, reference)
    Cfg {
        mb_records: 4,
        mb_sight: 0x4100_0000,
        mb_manager: 0x2a00_003a,
        mb_suck_latent: 0x2a80_00c3,
        mb_book: 0x2a00_0037,
        ea_target: 0x2a00_00c1,
        mb_retarget: &[0x2a00_0035, 0x2a00_0036, 0x2a00_00f0, 0x2a00_00f1],
        spawn_sound: 0x4000_0017,
        extinguish_sound: false,
        anim_search: 5,
        attack_latent: 0x2a80_000c,
        anim_attack: 6,
        anim_scared: 4,
        anim_scared_loop: 2,
        scared_turn: 0x4020_0000,
        turn_speed: 0x4000_0000,
        relight_period: 8.0,
        scared_period: 6.0,
        fg_records: 8,
        fill_ranges: (0x4110_0000, 0x4100_0000),
    },
    // 818e89e1 (EP6L04B)
    Cfg {
        mb_records: 4,
        mb_sight: 0x4100_0000,
        mb_manager: 0x2a00_007f,
        mb_suck_latent: 0x2a80_0106,
        mb_book: 0x2a00_007c,
        ea_target: 0x2a00_0104,
        mb_retarget: &[0x2a00_007a, 0x2a00_007b, 0x2a00_0136, 0x2a00_0137, 0x2a00_013a, 0x2a00_013b],
        spawn_sound: 0x4000_0011,
        extinguish_sound: false,
        anim_search: 4,
        attack_latent: 0x2a80_0084,
        anim_attack: 5,
        anim_scared: 3,
        anim_scared_loop: 6,
        scared_turn: 0x4020_0000,
        turn_speed: 0x4000_0000,
        relight_period: 8.0,
        scared_period: 6.0,
        fg_records: 5,
        fill_ranges: (0x4110_0000, 0x4100_0000),
    },
];

/// The current build's literals.
#[inline]
pub(crate) fn cfg(e: &Engine) -> &'static Cfg {
    &CFG[e.rt.cur_build()]
}

/// Address of a module global.
#[inline]
pub(crate) fn G(e: &Engine, a: u32) -> Ptr {
    e.rt.cur_data() + (a - 0x10000)
}
/// Current value of `pLocalDataFireGhostBase` (this instance's local data, 0x240 bytes).
#[inline]
pub(crate) fn L(e: &Engine) -> Ptr {
    e.r32(G(e, pLocalDataFireGhostBase))
}
/// Current value of `pLocalDataMonster_Base` (the same block as [`L`]).
#[inline]
pub(crate) fn LM(e: &Engine) -> Ptr {
    e.r32(G(e, pLocalDataMonster_Base))
}
/// Current value of `pGlobalDataMonster_Base` (+0 game buffer 0; see the module doc for +4 / +8).
#[inline]
pub(crate) fn GDM(e: &Engine) -> Ptr {
    e.r32(G(e, pGlobalDataMonster_Base))
}
/// Current value of `pGlobalDataFireGhostBase` (the same block as [`GDM`]): +4 game buffer 0, +8
/// game buffer 2, +0x10..+0x1c fire particle types, +0x20..+0x2c smoke particle types.
#[inline]
pub(crate) fn GDF(e: &Engine) -> Ptr {
    e.r32(G(e, pGlobalDataFireGhostBase))
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

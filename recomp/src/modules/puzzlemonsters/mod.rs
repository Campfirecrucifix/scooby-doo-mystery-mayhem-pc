//! `puzzleghost`, `puzzlemedghost`, `puzzlebigghost`, `puzzlemummy`, `puzzlemindy` (EP3L01 puzzle
//! "monster" props): Ep3_FrontierLand/Objects/L01_PUZZLEMONSTERS/{PuzzleMonsters.c, Puzzle<X>.c,
//! PuzzleGhostGlowInit.c, Glow.c}, 1 build each. The builds differ only in the monster type at L+8
//! (1 small, 2 medium, 3 big, 5 mummy, mindy leaves it 0 = no glow).
//!
//! PuzzleGhostGlowInit.c is the ghosts' GhostGlowInit.c with local offsets shifted by -0x198 (verified
//! store for store, CreateEmitterPos tables included) plus a type-5 mummy branch, and Glow.c is identical
//! to the ghosts'. So the ghost files are compiled again here (`#[path]`) against a *virtual* local base
//! `L = real - 0x198` (and `LM + 0x44` = real + 8). The mummy branch is ported in [`mummy_glow`].
#![allow(non_snake_case, non_upper_case_globals, unused_variables, dead_code, unused_imports)]

#[path = "../ghost/emitter_pos.rs"]
mod emitter_pos;
#[path = "../ghost/ghost_glow_init.rs"]
mod ghost_glow_init;
mod mum_particle;

use super::modkit::{custom, ext, gset};
use crate::engine::*;
use crate::logic::{LResult, ModuleDef};
pub(crate) use crate::modules::ghost::{SetParticle4Big, SetParticle5, SetParticle5Big, SetParticle5Sml, SetParticle6, SetParticle6Big};
pub(crate) use super::modkit::G;

macro_rules! def {
    ($id:ident, $name:literal, $ctor:ident) => {
        pub static $id: ModuleDef = ModuleDef {
            name: $name,
            data_size: 0x28,
            ctor: $ctor,
            dtor: puzzlemonstersDestructor,
            register: None,
            init: &[],
            build_init: None,
        };
    };
}
def!(MODULE_GHOST, "puzzleghost", puzzleghostConstructor);
def!(MODULE_MEDGHOST, "puzzlemedghost", puzzlemedghostConstructor);
def!(MODULE_BIGGHOST, "puzzlebigghost", puzzlebigghostConstructor);
def!(MODULE_MUMMY, "puzzlemummy", puzzlemummyConstructor);
def!(MODULE_MINDY, "puzzlemindy", puzzlemindyConstructor);

pub const pGameObj: u32 = 0x10000;
pub const pAnim: u32 = 0x10004;
pub const pExtra: u32 = 0x10008;
pub const pPhysic: u32 = 0x1000c;
pub const pParticle: u32 = 0x10010;
pub const pGraphic: u32 = 0x10014;
pub const pAnimMngr: u32 = 0x10018;
pub const pMiscFunc: u32 = 0x1001c;
pub const pLocalDatapuzzlemonsters: u32 = 0x10020;
pub const pGlobalDatapuzzlemonsters: u32 = 0x10024;

/// the real local block (0x718 bytes)
fn R(e: &Engine) -> Ptr {
    e.r32(G(e, pLocalDatapuzzlemonsters))
}
/// virtual ghost-layout base for the shared glow code
pub(crate) fn L(e: &Engine) -> Ptr {
    R(e).wrapping_sub(0x198)
}
pub(crate) fn LM(e: &Engine) -> Ptr {
    R(e).wrapping_sub(0x3c)
}
fn ty(e: &Engine) -> u32 {
    e.r32(R(e) + 8)
}

fn PuzzCreateGlow(e: &mut Engine) -> LResult<()> {
    ghost_glow_init::CreateEmitterPos(e)?;
    if ty(e) == 5 {
        APIgameobj::GetID(e, SELF, NULL);
        mummy_glow::PuzzCreate_Mummy_Particles(e)?;
    } else {
        ghost_glow_init::InitParticules(e)?;
    }
    Ok(())
}

fn PuzzDestroyGlow(e: &mut Engine) -> LResult<()> {
    if ty(e) == 5 {
        mummy_glow::PuzzDestroy_Mummy_Particles(e);
        Ok(())
    } else {
        ghost_glow_init::DestroyGlow(e)
    }
}

/// PuzzleGhostGlowInit.c type-5 branch: the mummy's arm/foot dust and eye glows (mummy
/// Particle_Func.c with offsets +0x480, no "created" flag and no shock-wave emitters; the Drop
/// functions start emitters with 1 and have stop counterparts).
pub mod mummy_glow {
    use super::mum_particle::*;
    use super::*;

    fn P(e: &Engine) -> u32 {
        e.r32(G(e, pParticle))
    }

    fn emitter(e: &mut Engine, l: Ptr, slot: u32, ty: u32, bone: u32) {
        let t = e.r32(l + ty) as i32;
        let em = APIparticle::EmitterCreate(e, t);
        e.w32(l + slot, em);
        let p = P(e) as i32;
        LIBMiscFunc::MiscFunc_EmitterUpdateFlags(e, p, em, 0x80, 1);
        APIparticle::EmitterSetJoint(e, em as i32, SELF, 0x6fff_ffff);
        APIgameobj::AddChild(e, SELF, bone, em, 1);
    }

    fn eye(e: &mut Engine, l: Ptr, slot: u32, bone: u32, ofs: u32) -> u32 {
        let t = e.r32(l + 0x68c) as i32;
        let em = APIparticle::EmitterCreate(e, t);
        e.w32(l + slot, em);
        APIparticle::EmitterSetJoint(e, em as i32, SELF, 0x6fff_ffff);
        APIgameobj::AddChild(e, SELF, bone, em, 1);
        APIgameobj::SetTranslation(e, em, l + ofs);
        let p = P(e) as i32;
        LIBMiscFunc::MiscFunc_EmitterUpdateFlags(e, p, em, 0x80, 1);
        LIBMiscFunc::MiscFunc_EmitterUpdateFlags(e, p, em, 2, 0);
        APIparticle::EmitterStart(e, em as i32, 1)
    }

    pub fn PuzzCreate_Mummy_Particles(e: &mut Engine) -> LResult<u32> {
        let l = R(e);
        for (o, v) in [(0x6fc, 0), (0x700, 0), (0x704, 0x3d23_d70a), (0x708, 0), (0x70c, 0), (0x710, 0xbd23_d70a)] {
            e.w32(l + o, v);
        }
        let types: [(u32, &str, &str, fn(&mut Engine, u32, u32) -> LResult<()>); 7] = [
            (0x680, "dust", "BrasDust", SetParticle_Mum_01),
            (0x684, "dust_blur", "BrasDustBlur", SetParticle_Mum_02),
            (0x688, "dust_blur", "PiedDustBlur", SetParticle_Mum_03),
            (0x68c, "glow", "Yeux", SetParticle_Mum_04),
            (0x690, "dust", "PiedDust", SetParticle_Mum_05),
            (0x694, "dust", "ShockWave_Dust", SetParticle_Mum_06),
            (0x698, "dust_blur", "ShockWave_DustBlur", SetParticle_Mum_07),
        ];
        for (o, tex, name, set) in types {
            let t = APIparticle::TypeCreate(e);
            e.w32(l + o, t);
            let s = e.cstr(tex);
            APIparticle::TypeSetTexture(e, t as i32, s);
            let s = e.cstr(name);
            APIparticle::TypeSetName(e, t as i32, s);
            let p = P(e);
            set(e, p, t)?;
        }
        // (slot, type, bone): arm dust 0xc..0xf / 0x12..0x15, foot dust 0x1b / 0x20
        let mut slot = 0x69c;
        for (ty, bones) in [(0x680, 0xc), (0x684, 0xc), (0x680, 0x12), (0x684, 0x12)] {
            for b in bones..bones + 4 {
                emitter(e, l, slot, ty, b);
                slot += 4;
            }
        }
        for (ty, b) in [(0x688, 0x1b), (0x690, 0x1b), (0x688, 0x20), (0x690, 0x20)] {
            emitter(e, l, slot, ty, b);
            slot += 4;
        }
        eye(e, l, 0x6ec, 7, 0x6fc);
        Ok(eye(e, l, 0x6f0, 8, 0x708))
    }

    /// NOTE: as in the original, the emitter loop walks 0x18 slots from the *type* slots (+0x680).
    pub fn PuzzDestroy_Mummy_Particles(e: &mut Engine) {
        let l = R(e);
        for i in 0..7 {
            let t = e.r32(l + 0x680 + i * 4) as i32;
            APIparticle::TypeDestroy(e, t);
        }
        for i in 0..0x18 {
            let t = e.r32(l + 0x680 + i * 4) as i32;
            APIparticle::EmitterDestroy(e, t);
        }
    }

    const RT: [u32; 8] = [0x69c, 0x6a0, 0x6a4, 0x6a8, 0x6ac, 0x6b0, 0x6b4, 0x6b8];
    const LT: [u32; 8] = [0x6bc, 0x6c0, 0x6c4, 0x6c8, 0x6cc, 0x6d0, 0x6d4, 0x6d8];
    fn go(e: &mut Engine, s: &[u32], start: bool) {
        let l = R(e);
        for &o in s {
            let em = e.r32(l + o) as i32;
            if start {
                APIparticle::EmitterStart(e, em, 1);
            } else {
                APIparticle::EmitterStop(e, em);
            }
        }
    }
    pub fn Particles_Drop_RT(e: &mut Engine, start: bool) {
        go(e, &RT, start);
        go(e, &[0x6dc, 0x6e0], start);
    }
    pub fn Particles_Drop_LT(e: &mut Engine, start: bool) {
        go(e, &LT, start);
        go(e, &[0x6e4, 0x6e8], start);
    }
    pub fn Particles_LeftArm(e: &mut Engine, start: bool) {
        go(e, &LT, start);
    }
    pub fn Particles_RightArm(e: &mut Engine, start: bool) {
        go(e, &RT, start);
    }
}

pub fn puzzlemonstersST_FIRSTMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    LIBAnimMngr::PlayAnim(e, 0.0, 1, 2, 0);
    APIgameobj::SetGroupID(e, SELF, 0x15);
    PuzzCreateGlow(e)?;
    let l = R(e);
    e.w32(l + 0x714, 1);
    APIgameobj::SetNextState(e, SELF, 1);
    APIgameobj::EnableObject(e, SELF, 0);
    Ok(0)
}
pub fn puzzlemonstersST_NOP(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}
pub fn puzzlemonstersCB_ANIM_LF(e: &mut Engine, _: u32) -> LResult<u64> {
    APIgameobj::EnableObject(e, SELF, 0);
    if ty(e) == 5 {
        mummy_glow::Particles_LeftArm(e, false);
        mummy_glow::Particles_RightArm(e, false);
    }
    Ok(1)
}

pub fn puzzlemonstersFUNC_MessageHandler(e: &mut Engine, m: u32, _: u32, _: Ptr) -> LResult<u64> {
    match m {
        0x141 => {
            let v = e.alloca(0xc);
            for i in 0..3 {
                e.w32(v + i * 4, 0);
            }
            APIanim::StopAllAnim(e, SELF);
            APIgameobj::EnableObject(e, SELF, 1);
            APIgameobj::SetTranslation(e, SELF, v);
            APIgameobj::TakeOrientation(e, SELF, 0x2a00012e, 0x7fff_ffff);
            if ty(e) == 5 {
                mummy_glow::Particles_Drop_RT(e, true);
                mummy_glow::Particles_Drop_LT(e, true);
            }
        }
        0xd8 => {
            APIgameobj::GetID(e, SELF, NULL);
            APIgameobj::GetID(e, SELF, NULL);
            APIgameobj::EnableObject(e, SELF, 1);
            APIanim::SetSpeed(e, SELF, 2, 0.85);
            LIBAnimMngr::PlayAnim(e, 0.0, 2, 1, 4);
            let g = e.r32(G(e, pGlobalDatapuzzlemonsters));
            let to = e.r32(e.r32(g) + 8);
            APIgameobj::SendMessageTo(e, to, NULL, 0x68, 0x10, NULL);
            if ty(e) == 5 {
                mummy_glow::Particles_LeftArm(e, true);
                mummy_glow::Particles_RightArm(e, true);
            }
        }
        0x142 => {
            APIgameobj::EnableObject(e, SELF, 0);
            if ty(e) == 5 {
                mummy_glow::Particles_Drop_RT(e, false);
                mummy_glow::Particles_Drop_LT(e, false);
            }
        }
        _ => {}
    }
    Ok(0)
}

fn puzzlemonstersConstructor(e: &mut Engine, n: i32) -> LResult<()> {
    if n == 1 {
        ext(e, &[
            (0, puzzlemonstersST_FIRSTMOVE),
            (1, puzzlemonstersST_NOP),
            (2, puzzlemonstersST_NOP),
            (3, puzzlemonstersST_NOP),
            (4, puzzlemonstersCB_ANIM_LF),
        ]);
        let v = APIcons::GetApiAnim(e); gset(e, pAnim, v);
        let v = APIcons::GetApiGameObj(e); gset(e, pGameObj, v);
        let v = APIcons::GetApiExtra(e); gset(e, pExtra, v);
        let v = APIcons::GetApiPhysic(e); gset(e, pPhysic, v);
        let v = APIcons::GetApiParticle(e); gset(e, pParticle, v);
        let v = APIcons::GetApiGraphics(e); gset(e, pGraphic, v);
        custom(e, pAnimMngr, "AnimMngr");
        custom(e, pMiscFunc, "MiscFunc");
        let s = G(e, pGlobalDatapuzzlemonsters);
        APIcons::GetGlobalData(e, s, 4);
        let b = APIgameobj::GetGameBuffer(e, 0, 0x8f8, 1);
        let g = e.r32(G(e, pGlobalDatapuzzlemonsters));
        e.w32(g, b);
    }
    APIcons::SetMessageFunction(e, puzzlemonstersFUNC_MessageHandler);
    let s = G(e, pLocalDatapuzzlemonsters);
    APIcons::GetLocalData(e, s, 0x718);
    let l = R(e);
    e.w32(l + 0x714, 0xffff_ffff);
    APIcons::SetInitialState(e, 0);
    Ok(())
}

/// Puzzle<X>.c: the shared constructor, then the monster type (none for mindy); the wrappers
/// re-registering state 0 / the message function are equivalent to the shared ones.
fn build(e: &mut Engine, n: i32, t: u32) -> LResult<()> {
    puzzlemonstersConstructor(e, n)?;
    if t != 0 {
        let l = R(e);
        e.w32(l + 8, t);
    }
    APIcons::SetMessageFunction(e, puzzlemonstersFUNC_MessageHandler);
    APIcons::SetInitialState(e, 0);
    Ok(())
}
pub fn puzzleghostConstructor(e: &mut Engine, _: Ptr, n: i32) -> LResult<()> {
    build(e, n, 1)
}
pub fn puzzlemedghostConstructor(e: &mut Engine, _: Ptr, n: i32) -> LResult<()> {
    build(e, n, 2)
}
pub fn puzzlebigghostConstructor(e: &mut Engine, _: Ptr, n: i32) -> LResult<()> {
    build(e, n, 3)
}
pub fn puzzlemummyConstructor(e: &mut Engine, _: Ptr, n: i32) -> LResult<()> {
    build(e, n, 5)
}
pub fn puzzlemindyConstructor(e: &mut Engine, _: Ptr, n: i32) -> LResult<()> {
    build(e, n, 0)
}

pub fn puzzlemonstersDestructor(e: &mut Engine, _: i32) -> LResult<()> {
    if e.r32(R(e) + 0x714) != 0xffff_ffff {
        PuzzDestroyGlow(e)?;
    }
    Ok(())
}

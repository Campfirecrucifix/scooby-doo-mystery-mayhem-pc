//! TITLE screen monsters: `smallghost`, `medghost`, `bigghost`, `mummy` (TITLE/MonstersSources, 1 build
//! each). Static posed monsters: message 0xde (param 0/1) restores clipping and plays anim 1 (or 2).
//! The ghosts build their glow in CreateEmitterPos: the ghosts' CreateEmitterPos tables (identical,
//! local offsets -0x1a0, generated data in ghost/emitter_pos.rs) then the glow of ghost InitParticules
//! without emitter clipping. The mummy uses Mummy_Base's Create_Particles with offsets -0x1fc, no
//! "created" flag, and its own particle settings (mummy_particle.rs, generated).
#![allow(non_snake_case, non_upper_case_globals, unused_variables, dead_code)]

#[path = "../ghost/emitter_pos.rs"]
mod emitter_pos;
mod mummy_particle;

use super::modkit::{custom, ext, gset, G};
use crate::engine::*;
use crate::logic::{LResult, ModuleDef};
use crate::modules::ghost::{SetParticle4Big, SetParticle5, SetParticle5Big, SetParticle5Sml, SetParticle6, SetParticle6Big};

macro_rules! def {
    ($id:ident, $name:literal, $ctor:ident, $dtor:ident) => {
        pub static $id: ModuleDef = ModuleDef {
            name: $name,
            data_size: 0x2c,
            ctor: $ctor,
            dtor: $dtor,
            register: None,
            init: &[],
            build_init: None,
        };
    };
}
def!(MODULE_SMALL, "smallghost", smallghostConstructor, smallghostDestructor);
def!(MODULE_MED, "medghost", medghostConstructor, medghostDestructor);
def!(MODULE_BIG, "bigghost", bigghostConstructor, bigghostDestructor);
def!(MODULE_MUMMY, "mummy", mummyConstructor, mummyDestructor);

pub const pAnim: u32 = 0x10000;
pub const pGameObj: u32 = 0x10004;
pub const pExtra: u32 = 0x10008;
pub const pPhysic: u32 = 0x1000c;
pub const pSound: u32 = 0x10010;
pub const pParticle: u32 = 0x10014;
pub const pMath: u32 = 0x10018;
pub const pGraphic: u32 = 0x1001c;
pub const pAnimMngr: u32 = 0x10020;
pub const pMiscFunc: u32 = 0x10024;
pub const pLocalData: u32 = 0x10028;

fn L(e: &Engine) -> Ptr {
    e.r32(G(e, pLocalData))
}
fn P(e: &Engine) -> u32 {
    e.r32(G(e, pParticle))
}

fn apis(e: &mut Engine, graphics: bool) {
    let v = APIcons::GetApiAnim(e); gset(e, pAnim, v);
    let v = APIcons::GetApiGameObj(e); gset(e, pGameObj, v);
    let v = APIcons::GetApiExtra(e); gset(e, pExtra, v);
    let v = APIcons::GetApiPhysic(e); gset(e, pPhysic, v);
    let v = APIcons::GetApiNewSound(e); gset(e, pSound, v);
    let v = APIcons::GetApiParticle(e); gset(e, pParticle, v);
    let v = APIcons::GetApiMath(e); gset(e, pMath, v);
    if graphics {
        let v = APIcons::GetApiGraphics(e); gset(e, pGraphic, v);
    }
    custom(e, pAnimMngr, "AnimMngr");
    custom(e, pMiscFunc, "MiscFunc");
}

fn shadow(e: &mut Engine) {
    let s = e.alloca(0x40);
    APIgameobj::GetShadowParams(e, SELF, s);
    for (o, v) in [(0, 2), (4, 0), (0x14, 0x80), (0x1c, 0x3e19_999a), (0x20, 0x40f0_0000), (0x24, 0x80), (0x28, 0x3f00_0000)] {
        e.w32(s + o, v);
    }
    APIgameobj::SetShadowParams(e, SELF, s);
}

pub fn ST_BASICMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

/// Small/Med ghost message function (also Mummy/BigGhost with `alt`): 0xde -> ChangeClipping(2), anim
fn on_de(e: &mut Engine, p: u32, alt: bool, notify: bool) {
    LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 2, NULL, 0);
    if alt && p != 0 {
        APIanim::Play(e, SELF, 2, 1, 0.0);
        if notify {
            APIgameobj::RegisterNotify(e, SELF, 2, 1, 1);
        }
    } else {
        APIanim::Play(e, SELF, 1, 1, 0.0);
    }
}
pub fn Ghost_MessageFunction(e: &mut Engine, m: u32, p: u32, _: Ptr) -> LResult<u64> {
    if m == 0xde {
        on_de(e, p, false, false);
    }
    Ok(0)
}
pub fn BigGhost_MessageFunction(e: &mut Engine, m: u32, p: u32, _: Ptr) -> LResult<u64> {
    if m == 0xde {
        on_de(e, p, true, true);
    }
    Ok(0)
}
pub fn Mummy_MessageFunction(e: &mut Engine, m: u32, p: u32, _: Ptr) -> LResult<u64> {
    if m == 0xde {
        on_de(e, p, true, false);
    }
    Ok(0)
}
pub fn bigghostCB_ANIM_LF(e: &mut Engine, _: u32) -> LResult<u64> {
    let v = e.alloca(0xc);
    for i in 0..3 {
        e.w32(v + i * 4, 0);
    }
    APIgameobj::SetTranslation(e, SELF, v);
    Ok(1)
}

/// The glow (ghost layout -0x1a0: group +0x1c, types +0x10..+0x18, sprite/emitter i at +0x30 + i*4,
/// joint +0x13c + i*4, offset +0x248 + i*0xc).
struct Glow {
    table: &'static [(u32, u32)],
    rgb: (f32, f32, f32),
    types: &'static [(u32, &'static str, fn(&mut Engine, u32, u32) -> LResult<()>)],
    sprites: u32,
    /// (index, type slot); bigghost's 4th (index 0x3f) is never stored (uses L+300 = 0)
    emitters: &'static [(u32, u32)],
    big: bool,
}

fn CreateEmitterPos(e: &mut Engine, g: &Glow) -> LResult<()> {
    let l = L(e);
    for &(o, v) in g.table {
        e.w32(l + o - 0x1a0, v);
    }
    let gr = APIgraphics::SpriteAddGroup(e);
    e.w32(l + 0x1c, gr);
    let s = e.cstr("glow");
    APIgraphics::SpriteSetGroupTexture(e, gr, s);
    APIgraphics::SpriteSetGroupColor(e, gr, g.rgb.0, g.rgb.1, g.rgb.2, 0.0);
    APIgraphics::SpriteSetGroupBlendMode(e, gr, 1);
    APIgraphics::SpriteSetGroupFlags(e, gr, 6);
    for &(slot, name, set) in g.types {
        let t = APIparticle::TypeCreate(e);
        e.w32(l + slot, t);
        let s = e.cstr("glow");
        APIparticle::TypeSetTexture(e, t as i32, s);
        let s = e.cstr(name);
        APIparticle::TypeSetName(e, t as i32, s);
        let p = P(e);
        set(e, p, t)?;
    }
    for i in 0..g.sprites {
        let sp = APIgraphics::SpriteGroupAddSprite(e, gr, SELF, 0x6fff_ffff, 0.45);
        e.w32(l + 0x30 + i * 4, sp);
        let j = e.r32(l + 0x13c + i * 4);
        APIgameobj::AddChild(e, SELF, j, sp, 1);
        APIgameobj::SetTranslation(e, sp, i * 0xc + 0x248 + l);
    }
    for &(i, ty) in g.emitters {
        let t = e.r32(l + ty);
        let em = APIparticle::EmitterCreate(e, t as i32);
        if !(g.big && i == 0x3f) {
            e.w32(l + 0x30 + i * 4, em);
        }
        let em = e.r32(l + 0x30 + i * 4);
        APIparticle::EmitterSetJoint(e, em as i32, SELF, 0x6fff_ffff);
        let j = e.r32(l + 0x13c + i * 4);
        APIgameobj::AddChild(e, SELF, j, em, 1);
        APIgameobj::SetTranslation(e, em, l + 0x248 + i * 0xc);
        APIparticle::EmitterStart(e, em as i32, 1);
        if g.big {
            APIparticle::EmitterSetFlags(e, em as i32, 0xc0);
        }
    }
    Ok(())
}

static SMALL: Glow = Glow {
    table: emitter_pos::EMITTER_POS_1,
    rgb: (0.0, 0.15, 0.0),
    types: &[(0x14, "TRAINEE_SMLGHOST", SetParticle5Sml)],
    sprites: 0x21,
    emitters: &[(0x21, 0x14)],
    big: false,
};
static MED: Glow = Glow {
    table: emitter_pos::EMITTER_POS_2,
    rgb: (0.0, 0.0, 0.3),
    types: &[(0x14, "TRAINEE_MID_MEDGHOST", SetParticle5), (0x18, "TRAINEE_SIDE_MEDGHOST", SetParticle6)],
    sprites: 0x2f,
    emitters: &[(0x2f, 0x14), (0x30, 0x18), (0x31, 0x14)],
    big: false,
};
static BIG: Glow = Glow {
    table: emitter_pos::EMITTER_POS_3,
    rgb: (0.0, 0.3, 0.0),
    types: &[
        (0x14, "TRAINEE_FARSIDE_BIG_GHOST", SetParticle5Big),
        (0x18, "TRAINEE_SIDE_BIG_GHOST", SetParticle6Big),
        (0x10, "TRAINEE_MID_BIG_GHOST", SetParticle4Big),
    ],
    sprites: 0x3c,
    emitters: &[(0x3c, 0x14), (0x3d, 0x14), (0x3e, 0x18), (0x3f, 0x18), (0x40, 0x10)],
    big: true,
};

fn ghost_ctor(e: &mut Engine, n: i32, g: &Glow, mf: crate::logic::MsgFn, big: bool) -> LResult<()> {
    if n == 1 {
        if big {
            ext(e, &[(0, ST_BASICMOVE), (1, bigghostCB_ANIM_LF)]);
        } else {
            ext(e, &[(0, ST_BASICMOVE)]);
        }
        apis(e, true);
    }
    APIcons::SetMessageFunction(e, mf);
    let s = G(e, pLocalData);
    APIcons::GetLocalData(e, s, 0x56c);
    CreateEmitterPos(e, g)?;
    APIcons::SetInitialState(e, 0);
    if !big {
        shadow(e);
    }
    Ok(())
}
pub fn smallghostConstructor(e: &mut Engine, _: Ptr, n: i32) -> LResult<()> {
    ghost_ctor(e, n, &SMALL, Ghost_MessageFunction, false)
}
pub fn medghostConstructor(e: &mut Engine, _: Ptr, n: i32) -> LResult<()> {
    ghost_ctor(e, n, &MED, Ghost_MessageFunction, false)
}
pub fn bigghostConstructor(e: &mut Engine, _: Ptr, n: i32) -> LResult<()> {
    ghost_ctor(e, n, &BIG, BigGhost_MessageFunction, true)
}
pub fn smallghostDestructor(e: &mut Engine, _: i32) -> LResult<()> {
    Ok(())
}
/// NOTE: as in the original, EmitterDestroy runs on sprite handles too and types +8/+0xc are never created.
fn ghost_dtor(e: &mut Engine, n: u32, types: &[u32]) {
    let l = L(e);
    for i in 0..n {
        let em = e.r32(l + 0x30 + i * 4) as i32;
        APIparticle::EmitterDestroy(e, em);
    }
    let gr = e.r32(l + 0x1c);
    APIgraphics::SpriteRemoveGroup(e, gr);
    for &o in types {
        let t = e.r32(l + o) as i32;
        APIparticle::TypeDestroy(e, t);
    }
}
pub fn medghostDestructor(e: &mut Engine, _: i32) -> LResult<()> {
    ghost_dtor(e, 0x32, &[8, 0xc, 0x10, 0x14, 0x18]);
    Ok(())
}
pub fn bigghostDestructor(e: &mut Engine, _: i32) -> LResult<()> {
    ghost_dtor(e, 0x41, &[8, 0xc, 0x14, 0x18, 0x10]);
    Ok(())
}

// ---- mummy (Particle_Func.c: types +4..+0x1c, emitters +0x20..+0x7c, eye offsets +0x80 / +0x8c)

fn m_emitter(e: &mut Engine, l: Ptr, slot: u32, ty: u32, bone: u32) -> u32 {
    let t = e.r32(l + ty) as i32;
    let em = APIparticle::EmitterCreate(e, t);
    e.w32(l + slot, em);
    let p = P(e) as i32;
    LIBMiscFunc::MiscFunc_EmitterUpdateFlags(e, p, em, 0x80, 1);
    APIparticle::EmitterSetJoint(e, em as i32, SELF, 0x6fff_ffff);
    APIgameobj::AddChild(e, SELF, bone, em, 1)
}
fn m_eye(e: &mut Engine, l: Ptr, slot: u32, bone: u32, ofs: u32) {
    let t = e.r32(l + 0x10) as i32;
    let em = APIparticle::EmitterCreate(e, t);
    e.w32(l + slot, em);
    APIparticle::EmitterSetJoint(e, em as i32, SELF, 0x6fff_ffff);
    APIgameobj::AddChild(e, SELF, bone, em, 1);
    APIgameobj::SetTranslation(e, em, l + ofs);
    let p = P(e) as i32;
    LIBMiscFunc::MiscFunc_EmitterUpdateFlags(e, p, em, 0x80, 1);
    LIBMiscFunc::MiscFunc_EmitterUpdateFlags(e, p, em, 2, 0);
    APIparticle::EmitterStart(e, em as i32, 1);
}
pub fn Create_Particles(e: &mut Engine) -> LResult<u32> {
    use mummy_particle::*;
    let l = L(e);
    for (o, v) in [(0x80, 0), (0x84, 0), (0x88, 0x3d23_d70a), (0x8c, 0), (0x90, 0), (0x94, 0xbd23_d70a)] {
        e.w32(l + o, v);
    }
    let types: [(u32, &str, &str, fn(&mut Engine, u32, u32) -> LResult<()>); 7] = [
        (4, "dust", "BrasDust", SetParticle_01),
        (8, "dust_blur", "BrasDustBlur", SetParticle_02),
        (0xc, "dust_blur", "PiedDustBlur", SetParticle_03),
        (0x10, "glow", "Yeux", SetParticle_04),
        (0x14, "dust", "PiedDust", SetParticle_05),
        (0x18, "dust", "ShockWave_Dust", SetParticle_06),
        (0x1c, "dust_blur", "ShockWave_DustBlur", SetParticle_07),
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
    let mut slot = 0x20;
    for (ty, b0) in [(4, 0xc), (8, 0xc), (4, 0x12), (8, 0x12)] {
        for b in b0..b0 + 4 {
            m_emitter(e, l, slot, ty, b);
            slot += 4;
        }
    }
    for (ty, b) in [(0xc, 0x1b), (0x14, 0x1b), (0xc, 0x20), (0x14, 0x20)] {
        m_emitter(e, l, slot, ty, b);
        slot += 4;
    }
    m_eye(e, l, 0x70, 7, 0x80);
    m_eye(e, l, 0x74, 8, 0x8c);
    m_emitter(e, l, 0x78, 0x18, 0x1d);
    Ok(m_emitter(e, l, 0x7c, 0x1c, 0x1d))
}
/// NOTE: the emitter loop walks 0x18 slots from the type slots (+4), as in the original.
pub fn Destroy_Particles(e: &mut Engine) {
    let l = L(e);
    for i in 0..7 {
        let t = e.r32(l + 4 + i * 4) as i32;
        APIparticle::TypeDestroy(e, t);
    }
    for i in 0..0x18 {
        let t = e.r32(l + 4 + i * 4) as i32;
        APIparticle::EmitterDestroy(e, t);
    }
}
pub fn mummyConstructor(e: &mut Engine, _: Ptr, n: i32) -> LResult<()> {
    if n == 1 {
        ext(e, &[(0, ST_BASICMOVE)]);
        apis(e, false);
    }
    APIcons::SetMessageFunction(e, Mummy_MessageFunction);
    let s = G(e, pLocalData);
    APIcons::GetLocalData(e, s, 0x98);
    Create_Particles(e)?;
    shadow(e);
    APIcons::SetInitialState(e, 0);
    Ok(())
}
pub fn mummyDestructor(e: &mut Engine, _: i32) -> LResult<()> {
    Destroy_Particles(e);
    Ok(())
}

//! `tbcs`: EP1L03A Tome-of-Doom beam cut-scene prop (the beam shown while Velma hands the book over).
//! Visible during cut-scene 1 (ChangeClipping 2 / 4 at its end); msg 0x1d5 creates the beam's spirit
//! particles (8 spiral emitters + 3 flying ones) and 14 sprites, 0x1d6 destroys them.
//! Port of `Game Logic Modules/Ep1_Library/Objects/TodBeamCS/tbcs/TBCS.c` (10 functions). Build 2a2fc044.
//! TBCS_Set_Particule / _FlyingSpirit are byte-identical to tod_beam's (same call sequence and
//! constants) -> the tod_beam ports are called.
#![allow(non_snake_case, non_upper_case_globals, unused_variables)]

use crate::engine::*;
use crate::logic::{LResult, ModuleDef};
use crate::modules::tod_beam::{ToD_Beam_Set_Particule, ToD_Beam_Set_Particule_FlyingSpirit};

pub static MODULE: ModuleDef = ModuleDef {
    name: "tbcs",
    // globals 0x10000..0x10014 (+ unused gpoLanguagesNamesAndISOCodes)
    data_size: 0x14,
    ctor: tbcsConstructor,
    dtor: tbcsDestructor,
    register: None,
    init: &[],
    build_init: None,
};

pub fn build_index(key: &str) -> Option<usize> {
    (key == "2a2fc044").then_some(0)
}

pub const pMiscFunc: u32 = 0x10000;
pub const pGraphic: u32 = 0x10004;
pub const pParticle: u32 = 0x10008;
pub const pGameObj: u32 = 0x1000c;
pub const pLocalDataTBCS: u32 = 0x10010;

#[inline]
fn G(e: &Engine, a: u32) -> Ptr {
    e.rt.cur_data() + (a - 0x10000)
}
/// pLocalDataTBCS (0x134): +0 sprite group, +4.. 14 sprites, +0x3c.. their joints, +0x74/+0x78
/// particle types, +0x7c.. 11 emitters, +0xa8.. emitter joints
#[inline]
fn L(e: &Engine) -> Ptr {
    e.r32(G(e, pLocalDataTBCS))
}

/// 0x00010d10 state 0
pub fn TBCS_ST_FIRSTMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    APIgameobj::SetNextState(e, SELF, 1);
    Ok(0)
}

/// 0x00010d68 state 1
pub fn TBCS_ST_BASICMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

/// 0x00010d98 callback 2 (notify 0x16): data {scene, 1 = start / 0 = end}
pub fn TBCS_CB_CUTSCENE(e: &mut Engine, p: u32) -> LResult<u64> {
    let (scene, on) = (e.r32(p), e.r32(p + 4));
    if on == 1 && scene == 1 {
        LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 2, NULL, 0);
        APIgameobj::SetNextState(e, SELF, 1);
    } else if on == 0 && scene == 1 {
        LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 4, NULL, 0);
        APIgameobj::SetNextState(e, SELF, 1);
    }
    Ok(0)
}

/// 0x00010eb0
pub fn TBCSFUNC_MessageHandler(e: &mut Engine, m: u32, _s: u32, _d: Ptr) -> LResult<u64> {
    if m == 0x1d5 {
        let l = L(e);
        TBCS_Create_Particules(e, l);
        TBCS_Create_Sprites(e, l);
    } else if m == 0x1d6 {
        TBCD_Destroy_effect(e);
    }
    Ok(0)
}

/// 0x00010f70
pub fn TBCS_Create_Particules(e: &mut Engine, p: Ptr) {
    for (i, j) in [3u32, 4, 6, 7, 9, 10, 0xc, 0xd].into_iter().enumerate() {
        e.w32(p + 0xa8 + 4 * i as u32, j);
    }
    for (slot, name, range, set) in [
        (0x74u32, "TOD_BEAM_SPIRALSPIRIT", 0..8u32, ToD_Beam_Set_Particule as fn(&mut Engine, Ptr, u32)),
        (0x78, "TOD_BEAM_FLYINGSPIRIT", 8..0xb, ToD_Beam_Set_Particule_FlyingSpirit),
    ] {
        if slot == 0x78 {
            e.w32(p + 0xc8, 0x20);
            e.w32(p + 0xcc, 0x24);
            e.w32(p + 0xd0, 0x28);
        }
        let t = APIparticle::TypeCreate(e);
        e.w32(p + slot, t);
        let s = e.cstr("spirit");
        APIparticle::TypeSetTexture(e, t as i32, s);
        let s = e.cstr(name);
        APIparticle::TypeSetName(e, t as i32, s);
        APIparticle::TypeSetInt(e, t as i32, 1, 1);
        let api = e.r32(G(e, pParticle));
        set(e, api, t);
        for i in range {
            let em = APIparticle::EmitterCreate(e, t as i32);
            e.w32(p + 0x7c + 4 * i, em);
            let j = e.r32(p + 0xa8 + 4 * i);
            APIparticle::EmitterSetJoint(e, em as i32, SELF, j);
            APIparticle::EmitterStart(e, em as i32, 1);
        }
    }
}

/// 0x00011380
pub fn TBCS_Create_Sprites(e: &mut Engine, p: Ptr) {
    let g = APIgraphics::SpriteAddGroup(e);
    e.w32(p, g);
    let s = e.cstr("spirit");
    APIgraphics::SpriteSetGroupTexture(e, g, s);
    APIgraphics::SpriteSetGroupColor(e, g, 0.0, f32::from_bits(0x3d4c_cccd), f32::from_bits(0x3ecc_cccd), 0.0);
    APIgraphics::SpriteSetGroupBlendMode(e, g, 1);
    APIgraphics::SpriteSetGroupFlags(e, g, 6);
    for i in 0..0xeu32 {
        e.w32(p + 4 * (i + 0xf), i + 0xe);
        let g = e.r32(p);
        let sp = APIgraphics::SpriteGroupAddSprite(e, g, SELF, 0x6fff_ffff, f32::from_bits(0x3ee6_6666));
        e.w32(p + 4 * (i + 1), sp);
        APIgameobj::AddChild(e, SELF, i + 0xe, sp, 1);
    }
}

/// 0x00011578
pub fn TBCD_Destroy_effect(e: &mut Engine) {
    let l = L(e);
    for i in 0..0xbu32 {
        let em = e.r32(l + 0x7c + 4 * i);
        APIparticle::EmitterDestroy(e, em as i32);
    }
    for o in [0x74, 0x78] {
        let t = e.r32(l + o);
        if t != 0 {
            APIparticle::TypeDestroy(e, t as i32);
        }
    }
    let g = e.r32(l);
    if g != 0 {
        APIgraphics::SpriteRemoveGroup(e, g);
    }
    e.w32(l + 0x7c, 0xffff_ffff);
}

/// 0x00011ce8
pub fn tbcsConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    if instanceCount == 1 {
        APIcons::SetExternalFunction(e, 0, TBCS_ST_FIRSTMOVE);
        APIcons::SetExternalFunction(e, 1, TBCS_ST_BASICMOVE);
        APIcons::SetExternalFunction(e, 2, TBCS_CB_CUTSCENE);
        let v = APIcons::GetApiGameObj(e);
        let a = G(e, pGameObj);
        e.w32(a, v);
        let n = e.cstr("MiscFunc");
        let v = APIcons::GetApiCustom(e, n);
        let a = G(e, pMiscFunc);
        e.w32(a, v);
        let v = APIcons::GetApiParticle(e);
        let a = G(e, pParticle);
        e.w32(a, v);
        let v = APIcons::GetApiGraphics(e);
        let a = G(e, pGraphic);
        e.w32(a, v);
    }
    let c = e.alloca(0x40); // uStack_70 (EFClippingParams)
    APIgameobj::GetClippingParams(e, SELF, c);
    for (o, v) in [(0, 0x3dcc_cccd), (4, 0x4160_0000), (0x10, 0xbf80_0000), (0xc, 0), (8, 0x47), (0x18, 0x40), (0x14, 0x6a7)] {
        e.w32(c + o, v);
    }
    let slot = G(e, pLocalDataTBCS);
    APIcons::GetLocalData(e, slot, 0x134);
    APIcons::SetMessageFunction(e, TBCSFUNC_MessageHandler);
    APIgameobj::SetClippingParams(e, SELF, c, 0);
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 0x16, 2);
    APIcons::SetInitialState(e, 0);
    Ok(())
}

/// 0x00011ee8
pub fn tbcsDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    if e.r32(L(e) + 0x7c) != 0xffff_ffff {
        TBCD_Destroy_effect(e);
    }
    Ok(())
}

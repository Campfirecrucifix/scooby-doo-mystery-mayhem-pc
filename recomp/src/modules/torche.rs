//! `torche`: wall torch with a "torche" fire emitter that starts / stops with the object's clipping
//! notify. Build 0 (8b7f1380, EP3/EP4) applies the particle settings twice (second pass wins: float 2
//! 1.35, inter 2 xy 0.75, inter 0x10 0.08/0.4/0.05) and stops the emitter on clip-out; build 1
//! (a0a1be5d, EP2) applies only the first pass and never stops it.
//! Port of `Game Logic Modules/Ep2_MovieStudio/Objects/TORCHES/torche__*/{Torche,Torches}.c`.
#![allow(non_snake_case, non_upper_case_globals, unused_variables)]

use crate::engine::*;
use crate::logic::{LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "torche",
    data_size: 0x20,
    ctor: torcheConstructor,
    dtor: torcheDestructor,
    register: None,
    init: &[],
    build_init: None,
};

pub fn build_index(key: &str) -> Option<usize> {
    ["8b7f1380", "a0a1be5d"].iter().position(|k| *k == key)
}

pub const pExtra: u32 = 0x10000;
pub const pGameObj: u32 = 0x10004;
pub const pPhysic: u32 = 0x10008;
pub const pAnim: u32 = 0x1000c;
pub const pParticle: u32 = 0x10010;
pub const pAnimMngr: u32 = 0x10014;
pub const pLocalDataTorche: u32 = 0x10018;
pub const pGlobalDataTorche: u32 = 0x1001c;

#[inline]
fn G(e: &Engine, a: u32) -> Ptr {
    e.rt.cur_data() + (a - 0x10000)
}
fn emitter(e: &Engine) -> i32 {
    e.r32(e.r32(G(e, pLocalDataTorche)) + 4) as i32
}
fn ptype(e: &Engine) -> i32 {
    e.r32(e.r32(G(e, pGlobalDataTorche)) + 4) as i32
}

pub fn TorcheST_FIRSTMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    APIgameobj::SetNextState(e, SELF, 1);
    APIgameobj::SetGroupID(e, SELF, 0x15);
    Ok(0)
}
pub fn TorcheST_NOP(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}
pub fn TorcheCB_ANIM_LF(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(1)
}
pub fn TorcheCB_NOTIFY_CLIPING(e: &mut Engine, p: u32) -> LResult<u64> {
    let em = emitter(e);
    if e.r32(p + 4) != 0 {
        APIparticle::EmitterStart(e, em, 1);
    } else if e.rt.cur_build() == 0 {
        APIparticle::EmitterStop(e, em);
    }
    Ok(0)
}
pub fn TorcheFUNC_MessageHandler(e: &mut Engine, _: u32, _: u32, _: Ptr) -> LResult<u64> {
    Ok(0)
}
fn torcheFUNC_SetClipping(e: &mut Engine) {
    let c = e.alloca(0x28);
    APIgameobj::GetClippingParams(e, SELF, c);
    for (o, v) in [(0, 0x3dcc_cccd), (4, 0x4270_0000), (0x10, 0xbf80_0000), (0xc, 0x1000_0000), (8, 0x1000_06a7), (0x18, 0x6a7), (0x14, 0x6a7)] {
        e.w32(c + o, v);
    }
    APIgameobj::SetClippingParams(e, SELF, c, 0);
}
/// Torches.c SetParticle_Torches; one pass (f2, inter2 y, inter 0x10 x/y/z)
fn pass(e: &mut Engine, t: i32, f2: f32, i2: f32, i10: [f32; 3]) {
    for (k, v) in [(8, 1), (1, 2), (4, 0x3c), (0x10, 0), (0x20, 0), (0x40, 0), (0x80, 0), (0x100, 0)] {
        APIparticle::TypeSetInt(e, t, k, v);
    }
    for (k, v) in [(1, 0.0), (4, 0.05), (2, f2), (8, 0.25)] {
        APIparticle::TypeSetFloat(e, t, k, v);
    }
    APIparticle::TypeSetVec(e, t, 1, 0.0, 0.5, 0.0);
    APIparticle::TypeSetVec(e, t, 2, 0.0, 0.0, 0.0);
    APIparticle::TypeSetInter(e, t, 1, 0.01, 1.0, 0.5, 0.5);
    APIparticle::TypeSetInter(e, t, 2, 0.01, i2, 0.01, 0.5);
    APIparticle::TypeSetInter(e, t, 4, 0.01, 0.25, 0.01, 0.5);
    APIparticle::TypeSetInter(e, t, 8, 1.0, 1.0, 1.0, 1.0);
    APIparticle::TypeSetInter(e, t, 0x10, i10[0], i10[1], i10[2], 0.15);
    APIparticle::TypeSetInter(e, t, 0x20, 0.1, 0.3, 0.8, 0.5);
}

pub fn torcheConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    if instanceCount == 1 {
        APIcons::SetExternalFunction(e, 0, TorcheST_FIRSTMOVE);
        APIcons::SetExternalFunction(e, 1, TorcheST_NOP);
        APIcons::SetExternalFunction(e, 2, TorcheST_NOP);
        APIcons::SetExternalFunction(e, 5, TorcheCB_NOTIFY_CLIPING);
        APIcons::SetExternalFunction(e, 3, TorcheST_NOP);
        APIcons::SetExternalFunction(e, 4, TorcheCB_ANIM_LF);
        for (a, v) in [
            (pAnim, APIcons::GetApiAnim(e)),
            (pGameObj, APIcons::GetApiGameObj(e)),
            (pExtra, APIcons::GetApiExtra(e)),
            (pPhysic, APIcons::GetApiPhysic(e)),
            (pParticle, APIcons::GetApiParticle(e)),
        ] {
            let a = G(e, a);
            e.w32(a, v);
        }
        let s = e.cstr("AnimMngr");
        let v = APIcons::GetApiCustom(e, s);
        let a = G(e, pAnimMngr);
        e.w32(a, v);
        let slot = G(e, pGlobalDataTorche);
        APIcons::GetGlobalData(e, slot, 8);
        let gd = e.r32(slot);
        let b = APIgameobj::GetGameBuffer(e, 0, 0x8f8, 1);
        e.w32(gd, b);
        let t = APIparticle::TypeCreate(e);
        e.w32(gd + 4, t);
        let t = t as i32;
        let s = e.cstr("torche");
        APIparticle::TypeSetName(e, t, s);
        let s = e.cstr("Firetorche");
        APIparticle::TypeSetTexture(e, t, s);
        pass(e, t, 0.65, 1.0, [0.01, 0.2, 0.01]);
        if e.rt.cur_build() == 0 {
            pass(e, t, 1.35, 0.75, [0.08, 0.4, 0.05]);
        }
    }
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 9, 5);
    APIcons::SetMessageFunction(e, TorcheFUNC_MessageHandler);
    torcheFUNC_SetClipping(e);
    let slot = G(e, pLocalDataTorche);
    APIcons::GetLocalData(e, slot, 8);
    // TorcheFUNC_CreateParticles
    let t = ptype(e);
    let em = APIparticle::EmitterCreate(e, t);
    let l = e.r32(slot);
    e.w32(l + 4, em);
    let em = em as i32;
    APIparticle::EmitterSetType(e, em, t);
    APIparticle::EmitterSetJoint(e, em, SELF, 0x7fff_ffff);
    APIparticle::EmitterSetFlags(e, em, 0x20);
    APIcons::SetInitialState(e, 0);
    Ok(())
}
pub fn torcheDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    let em = emitter(e);
    APIparticle::EmitterDestroy(e, em);
    if instanceCount == 0 {
        let t = ptype(e);
        APIparticle::TypeDestroy(e, t);
    }
    Ok(())
}

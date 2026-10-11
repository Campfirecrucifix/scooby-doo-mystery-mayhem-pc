//! `force_field_zombie_local` (EP6L02, 1 build): a zombie-shaped force field that blocks the leader. It
//! toggles on/off from switch messages (0x1ed/0xde/0xdf, 0x165/0x166), pushes the leader back with a
//! latent effect on contact, and shows four green motion trails. Port of
//! `Ep6_ShermanTech/Objects/FORCE_FIELD/LEVEL02/force_field_zombie_local` (Force_Field_Zombie.c + _Local.c).
//! ForceFieldZombieMap (one row): {0x2a0001ac, type 3, period 0.0, anims 2/2/1}.
#![allow(non_snake_case, non_upper_case_globals, unused_variables)]

use super::modkit::*;
use crate::engine::*;
use crate::logic::{LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "force_field_zombie_local",
    data_size: 0x40,
    ctor: force_field_zombie_localConstructor,
    dtor: force_field_zombie_localDestructor,
    register: None,
    init: &[],
    build_init: None,
};

pub const pAnim: u32 = 0x10000;
pub const pGameObj: u32 = 0x10004;
pub const pExtra: u32 = 0x10008;
pub const pPhysic: u32 = 0x1000c;
pub const pAnimMngr: u32 = 0x10010;
pub const pSound: u32 = 0x10014;
pub const pParticle: u32 = 0x10018;
pub const pDebug: u32 = 0x1001c;
pub const pFunctions: u32 = 0x10020;
pub const pMiscFunc: u32 = 0x10024;
pub const pGraphic: u32 = 0x10028;
pub const pLocalData: u32 = 0x1002c;
pub const pGlobalData: u32 = 0x10030;

/// {id, type, period (f32 bits), start anim, loop anim, off anim}
const MAP: [u32; 6] = [0x2a0001ac, 3, 0, 2, 2, 1];

// local (0x60): +0x18 event flags (1 off, 2 on), +0x1c on, +0x20 anim, +0x24/+0x28 timers,
// +0x2c period, +0x34 type, +0x38..+0x44 motion trails, +0x50 blocking, +0x54/+0x58/+0x5c anims
fn lr(e: &Engine, o: u32) -> u32 {
    e.r32(pw(e, pLocalData, o))
}
fn lw(e: &mut Engine, o: u32, v: u32) {
    let a = pw(e, pLocalData, o);
    e.w32(a, v);
}
fn gb(e: &Engine, i: u32) -> Ptr {
    e.r32(pw(e, pGlobalData, i * 4))
}
fn anim(e: &mut Engine, a: u32, n: i32, m: i32) {
    let r = LIBAnimMngr::PlayAnim(e, 0.0, lr(e, a) as i32, n, m);
    lw(e, 0x20, r as u32);
}
fn next(e: &mut Engine, s: u32) -> LResult<()> {
    APIgameobj::SetNextState(e, SELF, s);
    crate::modules::ghost::call_external_now(e, s)
}

pub fn ST_START(e: &mut Engine, _: u32) -> LResult<u64> {
    TR_START_NONE_ON(e)?;
    lw(e, 0x18, 0);
    APInewSound::Play(e, SELF, 0x4000_004b, 0);
    Ok(0)
}
pub fn ST_OFF(e: &mut Engine, _: u32) -> LResult<u64> {
    lw(e, 0x1c, 0);
    if lr(e, 0x18) & 2 != 0 {
        TR_OFF_TURN_ON(e)?;
    }
    lw(e, 0x18, 0);
    Ok(0)
}
pub fn ST_ON(e: &mut Engine, _: u32) -> LResult<u64> {
    lw(e, 0x1c, 1);
    if lr(e, 0x18) & 1 != 0 {
        TR_ON_TURN_OFF(e)?;
    }
    lw(e, 0x18, 0);
    Ok(0)
}
/// the map scan: row 0 matched or not, the loop exits at index 1
fn TR_START_NONE_ON(e: &mut Engine) -> LResult<()> {
    let id = APIgameobj::GetID(e, SELF, NULL);
    if id == MAP[0] {
        for (o, v) in [(0x34, MAP[1]), (0x2c, MAP[2]), (0x54, MAP[3]), (0x58, MAP[4]), (0x5c, MAP[5])] {
            lw(e, o, v);
        }
    }
    APIanim::Play(e, SELF, 3, 2, 0.0);
    CreateMotionTrail(e);
    anim(e, 0x54, 1, 4);
    APIgameobj::SetNextState(e, SELF, 2);
    SetClipping(e);
    crate::modules::ghost::call_external_now(e, 2)
}
fn TR_OFF_TURN_ON(e: &mut Engine) -> LResult<()> {
    APIgameobj::GetID(e, SELF, NULL);
    let t = lr(e, 0x24);
    APIextra::TimerUnregisterNotify(e, SELF, t);
    anim(e, 0x54, 1, 4);
    lw(e, 0x18, 0);
    lw(e, 0x50, 1);
    APIgameobj::SetGroupID(e, SELF, 0);
    next(e, 2)
}
fn TR_ON_TURN_OFF(e: &mut Engine) -> LResult<()> {
    APIgameobj::GetID(e, SELF, NULL);
    anim(e, 0x5c, 1, 0);
    let p = f32::from_bits(lr(e, 0x2c));
    if p != 0.0 {
        let t = lr(e, 0x24);
        APIextra::TimerReset(e, t);
        APIextra::TimerSetFrequency(e, t, p);
        APIextra::TimerRegisterNotify(e, SELF, t, 6);
    }
    lw(e, 0x18, 0);
    lw(e, 0x50, 0);
    APIgameobj::SetGroupID(e, SELF, 6);
    next(e, 1)
}
pub fn CB_COLLIDE_OBJ(e: &mut Engine, p: u32) -> LResult<u64> {
    if e.r32(p + 8) == e.r32(gb(e, 1) + 8) && lr(e, 0x50) == 1 {
        if lr(e, 0x34) != 3 {
            LaunchEffect(e);
        }
        let t = lr(e, 0x28);
        APIextra::TimerReset(e, t);
        APIextra::TimerSetFrequency(e, t, 0.5);
        APIextra::TimerRegisterNotify(e, SELF, t, 6);
        return Ok(1);
    }
    Ok(0)
}
pub fn CB_TIMER(e: &mut Engine, p: u32) -> LResult<u64> {
    if p == lr(e, 0x28) {
        APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 3, 3);
    }
    if p == lr(e, 0x24) {
        let v = lr(e, 0x18) | 2;
        lw(e, 0x18, v);
    }
    Ok(0)
}
pub fn CB_ANIM_LF(e: &mut Engine, _: u32) -> LResult<u64> {
    anim(e, 0x58, 2, 0);
    Ok(1)
}
fn turn(e: &mut Engine, on: bool) {
    let v = lr(e, 0x18) | if on { 2 } else { 1 };
    lw(e, 0x18, v);
    if on {
        APInewSound::Play(e, SELF, 0x4000_004b, 0);
    } else {
        APInewSound::Play(e, SELF, 0x4000_004c, 0);
        APInewSound::StopSound(e, SELF, 0x4000_004b);
    }
}
pub fn MessageHandler(e: &mut Engine, m: u32, p: u32, _: Ptr) -> LResult<u64> {
    match m {
        0xdf => turn(e, false),
        0xde => turn(e, true),
        0xd4 => {
            APIgameobj::KillObject(e, SELF);
        }
        0xc9 | 0x165 => TR_OFF_TURN_ON(e)?,
        0x166 => TR_ON_TURN_OFF(e)?,
        0x1ed => {
            let on = lr(e, 0x1c) == 0;
            turn(e, on);
        }
        0x1ee => {
            let v = lr(e, 0x1c);
            APIgameobj::SendMessageTo(e, p, NULL, 0x1ee, v, NULL);
        }
        _ => {}
    }
    Ok(0)
}
fn LaunchEffect(e: &mut Engine) {
    let pos = e.alloca(0x10);
    let right = e.alloca(0x10);
    APIgameobj::GetLookRight(e, SELF, 0x7fff_ffff, right);
    let leader = e.r32(gb(e, 1) + 8);
    if leader == 0x2a000041 {
        APIgameobj::GetPosition(e, leader, 0xd, pos);
    } else if leader == 0x2a000042 {
        APIgameobj::GetPosition(e, leader, 10, pos);
    }
    let o = APIgameobj::LaunchLatant(e, 0x2a8001d1);
    APIgameobj::SetTranslation(e, o, pos);
    APIgameobj::AlignAtWith(e, o, right);
}
fn SetClipping(e: &mut Engine) {
    let c = e.alloca(0x44);
    APIgameobj::GetClippingParams(e, SELF, c);
    for (o, v) in [
        (0, 0x3dcc_cccd), (4, 0x4170_0000), (0xc, 0), (8, 7), (0x10, 0x41c8_0000), (0x18, 0),
        (0x14, 0x6a7), (0x1c, 0xbf80_0000), (0x24, 0x6a7), (0x20, 0x6a7),
    ] {
        e.w32(c + o, v);
    }
    APIgameobj::SetClippingParams(e, SELF, c, 0);
}
fn CreateMotionTrail(e: &mut Engine) {
    if lr(e, 0x38) != 0xffff_ffff {
        return;
    }
    for i in 0..4 {
        let mt = APIgraphics::MTCreateSystem(e, 2, 10, 0);
        lw(e, 0x38 + i * 4, mt);
        APIgraphics::MTSetFrequency(e, mt, 0.05);
        for k in 0..2 {
            APIgraphics::MTSetJoint(e, mt, k, SELF, i * 2 + k + 1);
            APIgraphics::MTSetRGB(e, mt, k, 0.0, 1.0, 0.0);
            APIgraphics::MTSetLifeSpan(e, mt, k, 0.35);
        }
    }
    for i in 0..4 {
        let mt = lr(e, 0x38 + i * 4);
        APIgraphics::MTStart(e, mt);
    }
}
pub fn force_field_zombie_localConstructor(e: &mut Engine, api: Ptr, n: i32) -> LResult<()> {
    if n == 1 {
        ext(e, &[(0, ST_START), (1, ST_OFF), (2, ST_ON), (4, CB_ANIM_LF), (6, CB_TIMER), (3, CB_COLLIDE_OBJ)]);
        let v = APIcons::GetApiAnim(e); gset(e, pAnim, v);
        let v = APIcons::GetApiGameObj(e); gset(e, pGameObj, v);
        let v = APIcons::GetApiExtra(e); gset(e, pExtra, v);
        let v = APIcons::GetApiPhysic(e); gset(e, pPhysic, v);
        custom(e, pAnimMngr, "AnimMngr");
        let v = APIcons::GetApiNewSound(e); gset(e, pSound, v);
        let v = APIcons::GetApiParticle(e); gset(e, pParticle, v);
        custom(e, pDebug, "debug");
        custom(e, pFunctions, "ApiCFunctions");
        custom(e, pMiscFunc, "MiscFunc");
        let v = APIcons::GetApiGraphics(e); gset(e, pGraphic, v);
        let s = G(e, pGlobalData);
        APIcons::GetGlobalData(e, s, 0x10);
        let g = e.r32(G(e, pGlobalData));
        let b = APIgameobj::GetGameBuffer(e, 0, 0x8f8, 1);
        e.w32(g + 4, b);
        let b = APIgameobj::GetGameBuffer(e, 2, 0x158, 1);
        e.w32(g + 8, b);
        e.w32(g, api);
    }
    APIcons::SetMessageFunction(e, MessageHandler);
    let s = G(e, pLocalData);
    APIcons::GetLocalData(e, s, 0x60);
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 3, 3);
    lw(e, 0x50, 1);
    for i in 0..4 {
        lw(e, 0x38 + i * 4, 0xffff_ffff);
    }
    APIphysic::CreatePhysic(e, SELF, 0);
    APIphysic::SetFlags(e, SELF, 2);
    APIphysic::SetWeight(e, SELF, 1.0);
    APIphysic::SetPriority(e, SELF, 100);
    let t = APIextra::TimerCreate(e);
    lw(e, 0x24, t);
    let t = APIextra::TimerCreate(e);
    lw(e, 0x28, t);
    LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 2, NULL, 1);
    if e.r32(gb(e, 2) + 8) & 0x200 != 0 {
        APIgameobj::KillObject(e, SELF);
    }
    APIcons::SetInitialState(e, 0);
    Ok(())
}
pub fn force_field_zombie_localDestructor(e: &mut Engine, _: i32) -> LResult<()> {
    if lr(e, 0x38) != 0xffff_ffff {
        for i in 0..4 {
            let mt = lr(e, 0x38 + i * 4);
            APIgraphics::MTDestroySystem(e, mt);
        }
    }
    let t = lr(e, 0x24);
    APIextra::TimerDestroy(e, t);
    let t = lr(e, 0x28);
    APIextra::TimerDestroy(e, t);
    Ok(())
}

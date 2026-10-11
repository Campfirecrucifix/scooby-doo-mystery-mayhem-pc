//! `fakezombie` / `fakezombieb`: EP2 prop zombie that bursts out of the ground (dirt particles, pad
//! vibration, msg 0x68 to the leader) when the leader touches it with collision type 0x3b9. Port of
//! `Ep2_MovieStudio/Objects/ANIMATED_PROPS/fakezombie` (FakeZombie.c, 10 fns) and `fakezombieb`
//! (identical code; only symbol names and the particle-type names differ).
#![allow(non_snake_case, non_upper_case_globals, unused_variables)]

use super::fakezombie_particle::*;
use crate::engine::*;
use crate::logic::{LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "fakezombie",
    data_size: 0x4c,
    ctor: fakezombieConstructor,
    dtor: fakezombieDestructor,
    register: None,
    init: &[(iFakeZombieJoints, &[9, 5, 7, 3])],
    build_init: None,
};
pub static MODULE_B: ModuleDef = ModuleDef {
    name: "fakezombieb",
    data_size: 0x4c,
    ctor: fakezombiebConstructor,
    dtor: fakezombieDestructor,
    register: None,
    init: &[(iFakeZombieJoints, &[9, 5, 7, 3])],
    build_init: None,
};

pub const iFakeZombieJoints: u32 = 0x10000;
pub const pMath: u32 = 0x10010;
pub const pParticle: u32 = 0x10014;
pub const pAnimMngr: u32 = 0x10018;
pub const pText: u32 = 0x1001c;
pub const pDebug: u32 = 0x10020;
pub const pFunction: u32 = 0x10024;
pub const pMiscFunc: u32 = 0x10028;
pub const pExtra: u32 = 0x1002c;
pub const pGameObj: u32 = 0x10030;
pub const pPhysic: u32 = 0x10034;
pub const pAnim: u32 = 0x10038;
pub const pInput: u32 = 0x1003c;
pub const pGame: u32 = 0x10040;
pub const pLocalDataFakeZombie: u32 = 0x10044;
pub const pGlobalDataFakeZombie: u32 = 0x10048;

#[inline]
fn G(e: &Engine, a: u32) -> Ptr {
    e.rt.cur_data() + (a - 0x10000)
}
/// local word i (0 timer, 1 send-0x68 flag, 2 busy, 3..7 big dirt emitters, 7..9 small dirt emitters)
fn L(e: &Engine, i: u32) -> Ptr {
    e.r32(G(e, pLocalDataFakeZombie)) + i * 4
}
fn lr(e: &Engine, i: u32) -> u32 {
    e.r32(L(e, i))
}
fn lw(e: &mut Engine, i: u32, v: u32) {
    let a = L(e, i);
    e.w32(a, v);
}
/// global word i (0 big dirt type, 1 small dirt type, 2 game buffer 0, 3 game buffer 1)
fn gr(e: &Engine, i: u32) -> u32 {
    e.r32(e.r32(G(e, pGlobalDataFakeZombie)) + i * 4)
}
fn joint(e: &Engine, i: u32) -> u32 {
    e.r32(G(e, iFakeZombieJoints) + i * 4)
}

/// 0x00011418 state 0
pub fn fstate_FIRSTMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    lw(e, 1, 0);
    lw(e, 2, 0);
    let t = APIextra::TimerCreate(e);
    lw(e, 0, t);
    APIextra::TimerReset(e, t);
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 3, 3);
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 0xb, 5);
    APIanim::Play(e, SELF, 1, 1, 0.0);
    APIgameobj::SetNextState(e, SELF, 1);
    let pp = e.r32(G(e, pParticle)) as i32;
    for i in 0..4 {
        let ty = gr(e, 0) as i32;
        let em = APIparticle::EmitterCreate(e, ty);
        lw(e, i + 3, em);
        LIBMiscFunc::MiscFunc_EmitterUpdateFlags(e, pp, em, 0xc0, 1);
        let j = joint(e, i);
        APIparticle::EmitterSetJoint(e, em as i32, SELF, j);
        if i < 2 {
            let ty = gr(e, 1) as i32;
            let em = APIparticle::EmitterCreate(e, ty);
            lw(e, i + 7, em);
            LIBMiscFunc::MiscFunc_EmitterUpdateFlags(e, pp, em, 0xc0, 1);
            let j = joint(e, i);
            APIparticle::EmitterSetJoint(e, em as i32, SELF, j);
        }
    }
    FakeZombieFUNC_SetClipping(e);
    Ok(0)
}

/// 0x00011768 state 1
pub fn fstate_BASICMOVE_SET(e: &mut Engine, _: u32) -> LResult<u64> {
    let t = lr(e, 0);
    if 0.5 < APIextra::TimerGetTime(e, t) && lr(e, 1) != 0 {
        lw(e, 1, 0);
    }
    Ok(0)
}

/// 0x000117f8 state 2
pub fn fstate_BASICMOVE_LOOP(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

/// 0x00011828 callback 3 (collision notify 3)
pub fn fcback_NOTIFY_COLLIDE_OBJ(e: &mut Engine, param_1: u32) -> LResult<u64> {
    let gb0 = gr(e, 2);
    if e.r32(gb0 + 8) == e.r32(param_1 + 8) && e.r32(param_1 + 4) == 0x3b9 {
        let t = lr(e, 0);
        APIextra::TimerReset(e, t);
        if lr(e, 2) == 0 && e.r32(gb0 + 0x14) & 0x4900097c == 0 {
            lw(e, 1, 1);
            lw(e, 2, 1);
            LIBAnimMngr::PlayAnim(e, 0.0, 2, 1, 4);
            let pad = e.r32(gb0 + 0x4c) as i32;
            APIinput::InputSetVibration(e, pad, 0, 0.33, 0.0, 0.33, 0.0);
            for i in 0..4 {
                let em = lr(e, i + 3) as i32;
                APIparticle::EmitterStart(e, em, 0);
                if i < 2 {
                    let em = lr(e, i + 7) as i32;
                    APIparticle::EmitterStart(e, em, 1);
                }
            }
        }
    }
    Ok(0)
}

/// 0x00011a60 callback 4
pub fn fcback_NOTIFY_ANIM_END(e: &mut Engine, _: u32) -> LResult<u64> {
    lw(e, 2, 0);
    Ok(1)
}

/// 0x00011aa0 callback 5 (notify 0xb)
pub fn fcback_NOTIFY_ANIM(e: &mut Engine, _: u32) -> LResult<u64> {
    if lr(e, 1) != 0 {
        let leader = e.r32(gr(e, 2) + 8);
        APIgameobj::SendMessageTo(e, leader, NULL, 0x68, 3, NULL);
    }
    for i in 0..2 {
        let em = lr(e, i + 7) as i32;
        APIparticle::EmitterStop(e, em);
    }
    Ok(0)
}

/// 0x00011ba8
pub fn MyMessageFunction(e: &mut Engine, param_1: u32, _: u32, _: Ptr) -> LResult<u64> {
    if param_1 == 0xda {
        APIgameobj::UnregisterNotify(e, SELF, 0xffff_ffff, 3);
    }
    Ok(0)
}

/// 0x00011c30
pub fn FakeZombieFUNC_SetClipping(e: &mut Engine) {
    let c = e.alloca(0x44);
    APIgameobj::GetClippingParams(e, SELF, c);
    for (o, v) in [
        (0, 0x3dcc_cccd),
        (4, 0x420c_0000),
        (0x10, 0x4248_0000),
        (0x1c, 0xbf80_0000),
        (0xc, 0),
        (8, 0x687),
        (0x18, 0x480),
        (0x14, 0x6a7),
        (0x24, 0x6a7),
        (0x20, 0x6a7),
    ] {
        e.w32(c + o, v);
    }
    APIgameobj::SetClippingParams(e, SELF, c, 0);
}

/// 0x00011cf8
pub fn Create_Physic(e: &mut Engine) {
    APIphysic::CreatePhysic(e, SELF, 0);
    APIphysic::SetFlags(e, SELF, 2);
    APIphysic::SetWeight(e, SELF, 1.0);
    APIphysic::SetPriority(e, SELF, 100);
}

fn ctor(e: &mut Engine, instanceCount: i32, prefix: &str) {
    if instanceCount == 1 {
        let fns: [crate::logic::ExtFn; 6] = [
            fstate_FIRSTMOVE,
            fstate_BASICMOVE_SET,
            fstate_BASICMOVE_LOOP,
            fcback_NOTIFY_COLLIDE_OBJ,
            fcback_NOTIFY_ANIM_END,
            fcback_NOTIFY_ANIM,
        ];
        for (i, f) in fns.into_iter().enumerate() {
            APIcons::SetExternalFunction(e, i as _, f);
        }
        let tables: [(u32, fn(&mut Engine) -> Ptr); 6] = [
            (pAnim, |e| APIcons::GetApiAnim(e)),
            (pGameObj, |e| APIcons::GetApiGameObj(e)),
            (pExtra, |e| APIcons::GetApiExtra(e)),
            (pPhysic, |e| APIcons::GetApiPhysic(e)),
            (pInput, |e| APIcons::GetApiInput(e)),
            (pGame, |e| APIcons::GetApiGame(e)),
        ];
        for (slot, get) in tables {
            let v = get(e);
            let a = G(e, slot);
            e.w32(a, v);
        }
        for (slot, name) in [(pDebug, "debug"), (pAnimMngr, "AnimMngr"), (pText, "ApiText")] {
            let n = e.cstr(name);
            let v = APIcons::GetApiCustom(e, n);
            let a = G(e, slot);
            e.w32(a, v);
        }
        let v = APIcons::GetApiMath(e);
        let a = G(e, pMath);
        e.w32(a, v);
        let v = APIcons::GetApiParticle(e);
        let a = G(e, pParticle);
        e.w32(a, v);
        for (slot, name) in [(pFunction, "ApiCFunctions"), (pMiscFunc, "MiscFunc")] {
            let n = e.cstr(name);
            let v = APIcons::GetApiCustom(e, n);
            let a = G(e, slot);
            e.w32(a, v);
        }
        let slot = G(e, pGlobalDataFakeZombie);
        APIcons::GetGlobalData(e, slot, 0x10);
        let g = e.r32(slot);
        let b = APIgameobj::GetGameBuffer(e, 0, 0x8f8, 1);
        e.w32(g + 8, b);
        let b = APIgameobj::GetGameBuffer(e, 1, 0x68, 1);
        e.w32(g + 0xc, b);
        let tex = e.cstr("dark_dirt");
        for (i, kind, set) in [(0u32, "bigdirt", SetParticleBigDirt as fn(&mut Engine, i32)), (1, "smalldirt", SetParticleSmallDirt)] {
            let ty = APIparticle::TypeCreate(e);
            e.w32(g + i * 4, ty);
            let n = e.cstr(&format!("{prefix}_{kind}"));
            APIparticle::TypeSetName(e, ty as i32, n);
            APIparticle::TypeSetTexture(e, ty as i32, tex);
            set(e, ty as i32);
        }
    }
    APIcons::SetMessageFunction(e, MyMessageFunction);
    let slot = G(e, pLocalDataFakeZombie);
    APIcons::GetLocalData(e, slot, 0x2c);
    Create_Physic(e);
    LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 2, NULL, 0);
    APIcons::SetInitialState(e, 0);
}

/// 0x00011d98
pub fn fakezombieConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    ctor(e, instanceCount, "fakezombie");
    Ok(())
}
/// fakezombieb build
pub fn fakezombiebConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    ctor(e, instanceCount, "FakeZombieB");
    Ok(())
}

/// 0x00012200
pub fn fakezombieDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    for i in 0..4 {
        let em = lr(e, i + 3) as i32;
        APIparticle::EmitterDestroy(e, em);
        if i < 2 {
            let em = lr(e, i + 7) as i32;
            APIparticle::EmitterDestroy(e, em);
        }
    }
    let t = lr(e, 0);
    APIextra::TimerDestroy(e, t);
    if instanceCount == 0 {
        let (a, b) = (gr(e, 0) as i32, gr(e, 1) as i32);
        APIparticle::TypeDestroy(e, a);
        APIparticle::TypeDestroy(e, b);
    }
    Ok(())
}

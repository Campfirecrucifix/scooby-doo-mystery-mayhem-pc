//! `generic_fx`: a one-shot particle effect object driven by a descriptor that another module owns
//! (the wisps' collect sparkles). Launched as a latent copy by `MiscFunc_CreateGenericFX`, which
//! sends 0x16f (sub = entry count, data = descriptor); `MiscFunc_PlayGenericFX` sends 0x171 (re-run
//! the settings functions) then 0x170 (place and start the emitters).
//! Port of `Game Logic Modules/Globals/Objects/GENERIC_FX/generic_fx/generic_fx.c` (13 functions,
//! one build, SHA-1 c05bbaaa).
//!
//! Descriptor (owned by the sender, `L+0x2c`): entry i at `desc + i*0xac`: +0x04 texture (0x40
//! chars), +0x44 type name, +0x84 particle-settings function pointer (a [`CodeFn`] token in the port),
//! +0x88 position, +0x94 direction, +0xa0 up, +0xac emitter-start count.
//! Local data (0x40 bytes): +0x00 death time (never set by this module), +0x04 five particle types,
//! +0x18 five emitters, +0x2c descriptor, +0x30 entry count (max 5), +0x34 ready, +0x38 never set,
//! +0x3c start the emitters next frame.
#![allow(non_snake_case, non_upper_case_globals, unused_variables)]

use crate::engine::*;
use crate::logic::{CodeFn, LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "generic_fx",
    data_size: 0x34,
    ctor: generic_fxConstructor,
    dtor: generic_fxDestructor,
    register: None,
    init: &[],
    build_init: None,
};

// ---- module globals (_globals.h) ----
pub const pExtra: u32 = 0x10000;
pub const pGameObj: u32 = 0x10004;
pub const pPhysic: u32 = 0x10008;
pub const pAnim: u32 = 0x1000c;
pub const pParticle: u32 = 0x10010;
pub const pMath: u32 = 0x10014;
pub const pAnimMngr: u32 = 0x10018;
pub const pMiscFunc: u32 = 0x1001c;
pub const pLocalDataGeneric_fx: u32 = 0x10020;
pub const pGlobalDataGeneric_fx: u32 = 0x10024;

#[inline]
fn G(e: &Engine, a: u32) -> Ptr {
    e.rt.cur_data() + (a - 0x10000)
}
/// current value of `pLocalDataGeneric_fx`
#[inline]
fn L(e: &Engine) -> Ptr {
    e.r32(G(e, pLocalDataGeneric_fx))
}
/// `*(int *)(pLocalDataGeneric_fx + 0x2c)`: the descriptor
#[inline]
fn D(e: &Engine) -> Ptr {
    e.r32(L(e) + 0x2c)
}
/// `*(uint *)(pLocalDataGeneric_fx + 0x30)`: the entry count
#[inline]
fn N(e: &Engine) -> u32 {
    e.r32(L(e) + 0x30)
}

/// `(**(code **)(desc + 0x84 + i * 0xac))(pParticle, type_i)`
fn call_settings(e: &mut Engine, i: u32) -> LResult<()> {
    let f = e.r32(D(e) + 0x84 + i * 0xac);
    let pp = e.r32(G(e, pParticle));
    let t = e.r32(L(e) + 4 + i * 4);
    match e.code_fn(f) {
        CodeFn::Particle(f) => f(e, pp, t),
        _ => panic!("particle settings callback is not a particle function"),
    }
}

/// 0x00010cf8 state 0: types and emitters, ready, then state 1
pub fn Generic_fxST_FIRSTMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    Generic_fxFUNC_CreateTypes(e)?;
    Generic_fxFUNC_CreateEmitters(e);
    let l = L(e);
    e.w32(l + 0x34, 1);
    APIgameobj::SetNextState(e, SELF, 1);
    Ok(0)
}

/// 0x00010d88 state 1: after RunFX, start each entry's emitter once
pub fn Generic_fxST_BASICMOVE_SET(e: &mut Engine, _: u32) -> LResult<u64> {
    let l = L(e);
    if e.r32(l + 0x3c) != 0 {
        e.w32(l + 0x3c, 0);
        let mut uStack_2c = 0u32;
        while uStack_2c < N(e) {
            let em = e.ri32(L(e) + 0x18 + uStack_2c * 4);
            let n = e.r32(D(e) + 0xac + uStack_2c * 0xac);
            APIparticle::EmitterStart(e, em, n);
            uStack_2c += 1;
        }
    }
    Ok(0)
}

/// 0x00010e80 state 2: start every emitter, back to state 1
pub fn Generic_fxST_RUNFX(e: &mut Engine, _: u32) -> LResult<u64> {
    let mut uStack_2c = 0u32;
    while uStack_2c < N(e) {
        let em = e.ri32(L(e) + 0x18 + uStack_2c * 4);
        let n = e.r32(D(e) + 0xac + uStack_2c * 0xac);
        APIparticle::EmitterStart(e, em, n);
        uStack_2c += 1;
    }
    APIgameobj::SetNextState(e, SELF, 1);
    Ok(0)
}

/// 0x00010f70 state 3: die once the game clock passes L+0
pub fn Generic_fxST_BASICMOVE_DIE(e: &mut Engine, _: u32) -> LResult<u64> {
    let pfVar1 = L(e);
    let fVar2 = APIextra::TimerGetTime(e, 0x9000_0000);
    if e.rf(pfVar1) < fVar2 {
        APIgameobj::KillObject(e, SELF);
    }
    Ok(0)
}

/// 0x00010ff8 callback 4
pub fn Generic_fxCB_NOTIFY_COLLIDE_OBJ(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

/// 0x00011028 callback 5
pub fn Generic_fxCB_ANIM_LF(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(1)
}

/// 0x00011058 one particle type per entry, named/textured from the descriptor, configured by the
/// entry's settings function
pub fn Generic_fxFUNC_CreateTypes(e: &mut Engine) -> LResult<()> {
    let mut uStack_40 = 0u32;
    while uStack_40 < N(e) {
        let uVar1 = APIparticle::TypeCreate(e);
        let a = L(e) + 4 + uStack_40 * 4;
        e.w32(a, uVar1);
        let t = e.ri32(a);
        let name = uStack_40 * 0xac + D(e) + 0x44;
        APIparticle::TypeSetName(e, t, name);
        let tex = uStack_40 * 0xac + 4 + D(e);
        APIparticle::TypeSetTexture(e, t, tex);
        call_settings(e, uStack_40)?;
        uStack_40 += 1;
    }
    Ok(())
}

/// 0x00011218 re-run every entry's settings function (the sender may have swapped them)
pub fn Generic_fxFUNC_UpdateTypes(e: &mut Engine) -> LResult<()> {
    let mut uStack_30 = 0u32;
    while uStack_30 < N(e) {
        call_settings(e, uStack_30)?;
        uStack_30 += 1;
    }
    Ok(())
}

/// 0x000112e0 five emitters (always five, whatever the entry count: unused types are 0), flags 0xc0
/// set, not attached to any object
pub fn Generic_fxFUNC_CreateEmitters(e: &mut Engine) {
    for uStack_30 in 0..5u32 {
        let t = e.ri32(L(e) + 4 + uStack_30 * 4);
        let uVar1 = APIparticle::EmitterCreate(e, t);
        let a = L(e) + 0x18 + uStack_30 * 4;
        e.w32(a, uVar1);
        let pp = e.ri32(G(e, pParticle));
        LIBMiscFunc::MiscFunc_EmitterUpdateFlags(e, pp, uVar1, 0xc0, 1);
        let em = e.ri32(a);
        APIparticle::EmitterSetJoint(e, em, 0, 0x6fff_ffff);
    }
}

/// 0x00011428 place each emitter at its entry's position and orient it when direction and up are
/// non-zero; the emitters start on the next state-1 frame
pub fn Generic_fxFUNC_RunFX(e: &mut Engine) {
    if e.r32(L(e) + 0x34) == 0 {
        return;
    }
    let mut uStack_30 = 0u32;
    while uStack_30 < N(e) {
        let em = e.r32(L(e) + 0x18 + uStack_30 * 4);
        let ent = uStack_30 * 0xac + D(e);
        APIgameobj::SetTranslation(e, em, ent + 0x88);
        let len2 = |e: &Engine, a: Ptr| {
            let (x, y, z) = (e.rf(a), e.rf(a + 4), e.rf(a + 8));
            x * x + y * y + z * z
        };
        let s = len2(e, ent + 0x94);
        let fVar1 = APImath::sqrtf(e, s);
        if 0.0 < fVar1 {
            let s = len2(e, ent + 0xa0);
            let fVar1 = APImath::sqrtf(e, s);
            if 0.0 < fVar1 {
                APIgameobj::LookAtAlignUp(e, em, ent + 0x94, ent + 0xa0);
            }
        }
        uStack_30 += 1;
    }
    let l = L(e);
    e.w32(l + 0x3c, 1);
}

/// 0x000117f8 0x16f set descriptor (sub = count, max 5); 0x170 run; 0x171 update types; 0x172 kill
pub fn Generic_fxFUNC_MessageHandler(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    let l = L(e);
    match param_1 {
        0x170 => {
            if e.r32(l + 0x38) == 0 {
                Generic_fxFUNC_RunFX(e);
            }
        }
        0x16f => {
            e.w32(l + 0x30, if param_2 < 6 { param_2 } else { 5 });
            e.w32(l + 0x2c, param_3);
        }
        0x171 => Generic_fxFUNC_UpdateTypes(e)?,
        0x172 => {
            APIgameobj::KillObject(e, SELF);
        }
        _ => {}
    }
    Ok(0)
}

/// 0x00011950 module constructor. Note the order: the 0x16f message (descriptor) arrives after the
/// constructor and before state 0 runs (LaunchLatant + SendMessageTo in the same call).
pub fn generic_fxConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    let aEStack_70 = e.alloca(0x28);
    if instanceCount == 1 {
        let fns: [(i32, crate::logic::ExtFn); 6] = [
            (0, Generic_fxST_FIRSTMOVE),
            (1, Generic_fxST_BASICMOVE_SET),
            (3, Generic_fxST_BASICMOVE_DIE),
            (2, Generic_fxST_RUNFX),
            (4, Generic_fxCB_NOTIFY_COLLIDE_OBJ),
            (5, Generic_fxCB_ANIM_LF),
        ];
        for (i, f) in fns {
            APIcons::SetExternalFunction(e, i, f);
        }
        let tables: [(u32, fn(&mut Engine) -> Ptr); 6] = [
            (pAnim, |e| APIcons::GetApiAnim(e)),
            (pGameObj, |e| APIcons::GetApiGameObj(e)),
            (pExtra, |e| APIcons::GetApiExtra(e)),
            (pPhysic, |e| APIcons::GetApiPhysic(e)),
            (pParticle, |e| APIcons::GetApiParticle(e)),
            (pMath, |e| APIcons::GetApiMath(e)),
        ];
        for (slot, get) in tables {
            let v = get(e);
            let a = G(e, slot);
            e.w32(a, v);
        }
        for (slot, name) in [(pAnimMngr, "AnimMngr"), (pMiscFunc, "MiscFunc")] {
            let n = e.cstr(name);
            let v = APIcons::GetApiCustom(e, n);
            let a = G(e, slot);
            e.w32(a, v);
        }
        let slot = G(e, pGlobalDataGeneric_fx);
        APIcons::GetGlobalData(e, slot, 4);
        let uVar1 = APIgameobj::GetGameBuffer(e, 0, 0x8f8, 1);
        let gd = e.r32(G(e, pGlobalDataGeneric_fx));
        e.w32(gd, uVar1);
    }
    APIcons::SetMessageFunction(e, Generic_fxFUNC_MessageHandler);
    let slot = G(e, pLocalDataGeneric_fx);
    APIcons::GetLocalData(e, slot, 0x40);
    APIcons::SetInitialState(e, 0);
    APIgameobj::GetClippingParams(e, SELF, aEStack_70);
    e.w32(aEStack_70 + 4, 0xbf80_0000);
    e.w32(aEStack_70 + 0xc, 0);
    e.w32(aEStack_70 + 8, 0);
    APIgameobj::SetClippingParams(e, SELF, aEStack_70, 0);
    let l = L(e);
    e.w32(l + 0x34, 0);
    Ok(())
}

/// 0x00011c08 module destructor
pub fn generic_fxDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    let mut uStack_2c = 0u32;
    while uStack_2c < N(e) {
        let t = e.ri32(L(e) + 4 + uStack_2c * 4);
        APIparticle::TypeDestroy(e, t);
        uStack_2c += 1;
    }
    let mut uStack_2c = 0u32;
    while uStack_2c < N(e) {
        let em = e.ri32(L(e) + 0x18 + uStack_2c * 4);
        APIparticle::EmitterDestroy(e, em);
        uStack_2c += 1;
    }
    Ok(())
}

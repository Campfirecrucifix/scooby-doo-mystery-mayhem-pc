//! `zombieboss_binding`: the binding (chains / ropes) that holds the Ep4 zombie boss. When the
//! Tome of Doom pulls the boss in (message 0x67) the binding tracks the leader hero, blends its
//! anim 1 -> 2 and kills itself when that anim ends.
//! Port of `Game Logic Modules/Globals/Characters/ZOMBIE_BOSS/zombieboss_binding/ZombieBoss_binding.c`
//! (6 functions). One build (pack LEVEL/EP4/L04/GEN/EP4L04.P2R).
#![allow(non_snake_case, non_upper_case_globals, unused_variables)]

use crate::engine::*;
use crate::logic::{LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "zombieboss_binding",
    // globals 0x10000..0x10030; gpoLanguagesNamesAndISOCodes (0x10030, "Abkhazian") is never read
    data_size: 0x30,
    ctor: zombieboss_bindingConstructor,
    dtor: zombieboss_bindingDestructor,
    register: None,
    init: &[],
    build_init: None,
};

// ---- module globals (_globals.h; pLocalData is unnamed there: 0x10028, between pSound and pGlobalData) ----
pub const pAnim: u32 = 0x10000;
pub const pAnimMngr: u32 = 0x10004;
pub const pGameObj: u32 = 0x10008;
pub const pPhysic: u32 = 0x1000c;
pub const pExtra: u32 = 0x10010;
pub const pParticle: u32 = 0x10014;
pub const pMath: u32 = 0x10018;
pub const pStdlib: u32 = 0x1001c;
pub const pFunction: u32 = 0x10020;
pub const pSound: u32 = 0x10024;
pub const pLocalData: u32 = 0x10028;
pub const pGlobalData: u32 = 0x1002c;

#[inline]
fn G(e: &Engine, a: u32) -> Ptr {
    e.rt.cur_data() + (a - 0x10000)
}

/// 0x00010d00 external state 0: no context, group 2, then state 1
pub fn ZombieBoss_BindingST_FIRSTMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    APIgameobj::SetContextFlag(e, SELF, 0);
    APIgameobj::SetGroupID(e, SELF, 2);
    APIgameobj::SetNextState(e, SELF, 1);
    Ok(0)
}

/// 0x00010d90 external state 1
pub fn ZombieBoss_BindingST_BASICMOVE_LOOP(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

/// 0x00010dc0 external 2: the anim registered by MyMessageFonction ended
pub fn ZombieBoss_BindingCB_ANIM_LF(e: &mut Engine, _: u32) -> LResult<u64> {
    APIgameobj::KillObject(e, SELF);
    Ok(0)
}

/// 0x00010e10 0x67 (the boss is pulled into the book): track the leader hero (game buffer 0 +8),
/// play anim 1 blended into anim 2 (blend 0xbb000001, weight 0.5, time 0.5), notify the end of
/// anim 1 to external 2.
pub fn MyMessageFonction(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    if param_1 == 0x67 {
        let gd = e.r32(G(e, pGlobalData));
        let hero = e.r32(e.r32(gd) + 8);
        LIBApiCFunctions::TrackID(e, 3.1415927, SELF as i32, hero);
        APIanim::BeginTree(e, SELF);
        APIanim::Play(e, SELF, 1, 1, 0.0);
        let b = e.alloca(0x14);
        for (o, w) in [(0, 0xbb00_0001), (4, 0), (0xc, 0), (8, 0x3f00_0000), (0x10, 0x3f00_0000)] {
            e.w32(b + o, w);
        }
        APIanim::PlayBlendConnect(e, SELF, 2, 1, 1, b);
        APIanim::EndTree(e, SELF);
        APIgameobj::RegisterNotify(e, SELF, 1, 1, 2);
        APIgameobj::SetNextState(e, SELF, 1);
    }
    Ok(0)
}

/// 0x00010f90
pub fn zombieboss_bindingConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    if instanceCount == 1 {
        APIcons::SetExternalFunction(e, 0, ZombieBoss_BindingST_FIRSTMOVE);
        APIcons::SetExternalFunction(e, 1, ZombieBoss_BindingST_BASICMOVE_LOOP);
        APIcons::SetExternalFunction(e, 2, ZombieBoss_BindingCB_ANIM_LF);
        let tables: [(u32, fn(&mut Engine) -> Ptr); 9] = [
            (pAnim, |e| APIcons::GetApiAnim(e)),
            (pAnimMngr, |e| {
                let n = e.cstr("AnimMngr");
                APIcons::GetApiCustom(e, n)
            }),
            (pGameObj, |e| APIcons::GetApiGameObj(e)),
            (pExtra, |e| APIcons::GetApiExtra(e)),
            (pPhysic, |e| APIcons::GetApiPhysic(e)),
            (pSound, |e| APIcons::GetApiNewSound(e)),
            (pStdlib, |e| APIcons::GetApiStdlib(e)),
            (pParticle, |e| APIcons::GetApiParticle(e)),
            (pMath, |e| APIcons::GetApiMath(e)),
        ];
        for (slot, get) in tables {
            let v = get(e);
            let a = G(e, slot);
            e.w32(a, v);
        }
        let n = e.cstr("ApiCFunctions");
        let v = APIcons::GetApiCustom(e, n);
        let a = G(e, pFunction);
        e.w32(a, v);
        let slot = G(e, pGlobalData);
        APIcons::GetGlobalData(e, slot, 4);
        let uVar1 = APIgameobj::GetGameBuffer(e, 0, 0x8f8, 1);
        let gd = e.r32(G(e, pGlobalData));
        e.w32(gd, uVar1);
    }
    APIcons::SetMessageFunction(e, MyMessageFonction);
    let slot = G(e, pLocalData);
    APIcons::GetLocalData(e, slot, 4);
    APIcons::SetInitialState(e, 0);
    Ok(())
}

/// 0x000111c0
pub fn zombieboss_bindingDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    Ok(())
}

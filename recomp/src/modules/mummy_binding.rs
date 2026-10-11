//! `mummy_binding`: the bandages that wrap the Ep2 mummy when the Tome of Doom pulls it in
//! (message 0x67): the binding tracks the leader hero, blends its anim 1 -> 2 and kills itself when
//! that anim ends. Launched as latent by Monster_Base_MessageFunction (0xd6/0x67).
//! Port of `Game Logic Modules/Globals/Characters/Mummy_Base/mummy_binding/Mummy_binding.c`
//! (6 functions). One build (packs EP2L02A/B/C, EP2L03A, EP2L09).
#![allow(non_snake_case, non_upper_case_globals, unused_variables)]

use crate::engine::*;
use crate::logic::{LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "mummy_binding",
    // globals 0x10000..0x10034; gpoLanguagesNamesAndISOCodes (0x10038, "Abkhazian") is never read
    data_size: 0x38,
    ctor: mummy_bindingConstructor,
    dtor: mummy_bindingDestructor,
    register: None,
    init: &[],
    build_init: None,
};

// ---- module globals (_globals.h; pLocalData is unnamed there: 0x1002c, between pMiscFunc and pGlobalData) ----
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
pub const pMiscFunc: u32 = 0x10028;
pub const pLocalData: u32 = 0x1002c;
pub const pGlobalData: u32 = 0x10030;

#[inline]
fn G(e: &Engine, a: u32) -> Ptr {
    e.rt.cur_data() + (a - 0x10000)
}

/// 0x00010d18 external state 0: context 9, group 2, then state 1
pub fn MummyST_FIRSTMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    APIgameobj::SetContextFlag(e, SELF, 9);
    APIgameobj::SetGroupID(e, SELF, 2);
    APIgameobj::SetNextState(e, SELF, 1);
    Ok(0)
}

/// 0x00010da8 external state 1
pub fn MummyST_BASICMOVE_LOOP(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

/// 0x00010dd8 external 2: the anim registered by MyMessageFonction ended
pub fn MummyCB_ANIM_LF(e: &mut Engine, _: u32) -> LResult<u64> {
    APIgameobj::KillObject(e, SELF);
    Ok(0)
}

/// 0x00010e28 0x67 (the mummy is pulled into the book): track the leader hero (game buffer 0 +8),
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

/// 0x00010fa8 local data 0x14 bytes (unused); clipping mode 2
pub fn mummy_bindingConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    if instanceCount == 1 {
        APIcons::SetExternalFunction(e, 0, MummyST_FIRSTMOVE);
        APIcons::SetExternalFunction(e, 1, MummyST_BASICMOVE_LOOP);
        APIcons::SetExternalFunction(e, 2, MummyCB_ANIM_LF);
        let tables: [(u32, fn(&mut Engine) -> Ptr); 11] = [
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
            (pFunction, |e| {
                let n = e.cstr("ApiCFunctions");
                APIcons::GetApiCustom(e, n)
            }),
            (pMiscFunc, |e| {
                let n = e.cstr("MiscFunc");
                APIcons::GetApiCustom(e, n)
            }),
        ];
        for (slot, get) in tables {
            let v = get(e);
            let a = G(e, slot);
            e.w32(a, v);
        }
        let slot = G(e, pGlobalData);
        APIcons::GetGlobalData(e, slot, 4);
        let uVar1 = APIgameobj::GetGameBuffer(e, 0, 0x8f8, 1);
        let gd = e.r32(G(e, pGlobalData));
        e.w32(gd, uVar1);
    }
    APIcons::SetMessageFunction(e, MyMessageFonction);
    let slot = G(e, pLocalData);
    APIcons::GetLocalData(e, slot, 0x14);
    LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 2, NULL, 0);
    APIcons::SetInitialState(e, 0);
    Ok(())
}

/// 0x00011218
pub fn mummy_bindingDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    Ok(())
}

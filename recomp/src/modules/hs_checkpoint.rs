//! `hs_checkpoint`: EP4 L03/L09 checkpoint hot spots. Touched by the player (0x2a00000d, collide
//! notify, ext fn 2): stores its id at GD+4, sends 0xa3 to 0x2a00000b and, for the 3 known
//! checkpoint ids, shows the "checkpoint" text and sends 0xa3(checkpoint) to the player + 0x1f9.
//! Port of `Game Logic Modules/Ep4_Bayou/Objects/HOTSPOTS/Level_03/hs_checkpoint__*/hs_Checkpoint.c`
//! (6 functions). Builds differ only in the id/checkpoint table, the extra 0xc3 target and the text id.
#![allow(non_snake_case, non_upper_case_globals, unused_variables)]

use crate::engine::*;
use crate::logic::{LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "hs_checkpoint",
    // globals 0x10000..0x10024 (+ unused gpoLanguagesNamesAndISOCodes)
    data_size: 0x24,
    ctor: hs_checkpointConstructor,
    dtor: hs_checkpointDestructor,
    register: None,
    init: &[],
    build_init: None,
};

const KEYS: [&str; 4] = ["583ac3b4", "f40092d0", "e0b016ee", "751a9264"];

pub fn build_index(key: &str) -> Option<usize> {
    KEYS.iter().position(|k| *k == key)
}

/// per-build: [(object id, checkpoint id); 3], extra 0xc3 target sent for the 3rd id, ShowText id
struct Build {
    ids: [(u32, u32); 3],
    c3_target: Option<u32>,
    text: u32,
}
const BUILDS: [Build; 4] = [
    Build { ids: [(0x2a00_0128, 0x2a00_010a), (0x2a00_0129, 0x2a00_010b), (0x2a00_012a, 0x2a00_010c)], c3_target: Some(0x2a00_004f), text: 0x15c },
    Build { ids: [(0x2a00_012c, 0x2a00_0112), (0x2a00_012d, 0x2a00_0113), (0x2a00_012e, 0x2a00_0114)], c3_target: None, text: 0x15c },
    Build { ids: [(0x2a00_0128, 0x2a00_010a), (0x2a00_0129, 0x2a00_010b), (0x2a00_012a, 0x2a00_010c)], c3_target: Some(0x2a00_004e), text: 0x176 },
    Build { ids: [(0x2a00_012c, 0x2a00_0112), (0x2a00_012d, 0x2a00_0113), (0x2a00_012e, 0x2a00_0114)], c3_target: None, text: 0x176 },
];

pub const pExtra: u32 = 0x10000;
pub const pGameObj: u32 = 0x10004;
pub const pPhysic: u32 = 0x10008;
pub const pMiscFunc: u32 = 0x1000c;
pub const pGame: u32 = 0x10010;
pub const pHud: u32 = 0x10014;
pub const pText: u32 = 0x10018;
/// not in _globals.h (only written); placed in the gap before pGlobalDataHSCheckPoint
pub const pLocalDataHSCheckPoint: u32 = 0x1001c;
pub const pGlobalDataHSCheckPoint: u32 = 0x10020;

#[inline]
fn G(e: &Engine, a: u32) -> Ptr {
    e.rt.cur_data() + (a - 0x10000)
}
/// pGlobalDataHSCheckPoint (0x40: +4 last checkpoint id, +8 text struct, +0x3c game buffer 0)
#[inline]
fn GD(e: &Engine) -> Ptr {
    e.r32(G(e, pGlobalDataHSCheckPoint))
}

/// hs_CheckPointST_FirstMove @0x00010cf0
pub fn hs_CheckPointST_FirstMove(e: &mut Engine, _: u32) -> LResult<u64> {
    let t = GD(e) + 8;
    LIBApiText::InitStruct(e, t);
    e.w32(t, 0x4060_0000);
    APIgameobj::SetGroupID(e, SELF, 0x17);
    APIgameobj::SetNextState(e, SELF, 1);
    Ok(0)
}

/// hs_CheckPointST_BasicMove_Loop @0x00010d98
pub fn hs_CheckPointST_BasicMove_Loop(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

/// hs_CheckPointFUNC_MessageFunction @0x00010dc8
pub fn hs_CheckPointFUNC_MessageFunction(e: &mut Engine, m: u32, _s: u32, _d: Ptr) -> LResult<u64> {
    Ok(1)
}

/// hs_CheckPointCB_NotifyColide @0x00010e18 (param_1 = notify info, +8 = other object id)
pub fn hs_CheckPointCB_NotifyColide(e: &mut Engine, p: u32) -> LResult<u64> {
    if e.r32(p + 8) == 0x2a00_000d {
        let id = APIgameobj::GetID(e, SELF, NULL);
        let gd = GD(e);
        e.w32(gd + 4, id);
        let id = APIgameobj::GetID(e, SELF, NULL);
        let b = &BUILDS[e.rt.cur_build()];
        let mut cp = 0;
        for (i, &(oid, c)) in b.ids.iter().enumerate() {
            if id == oid {
                if i == 2 {
                    if let Some(t) = b.c3_target {
                        APIgameobj::SendMessageTo(e, t, NULL, 0xc3, 0, NULL);
                    }
                }
                cp = c;
            }
        }
        APIgameobj::SendMessageTo(e, 0x2a00_000b, NULL, 0xa3, 0, NULL);
        if cp != 0 {
            LIBApiText::KillText(e);
            let t = GD(e) + 8;
            LIBApiText::ShowText(e, b.text, t);
            APIgameobj::SendMessageTo(e, 0x2a00_000d, NULL, 0xa3, cp, NULL);
            APIgameobj::SendMessageTo(e, 0x2a00_000b, NULL, 0x1f9, 0, NULL);
        }
    }
    Ok(1)
}

/// hs_checkpointConstructor @0x00011058 (0x00011028 in f40092d0/751a9264)
pub fn hs_checkpointConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    if instanceCount == 1 {
        APIcons::SetExternalFunction(e, 0, hs_CheckPointST_FirstMove);
        APIcons::SetExternalFunction(e, 1, hs_CheckPointST_BasicMove_Loop);
        APIcons::SetExternalFunction(e, 2, hs_CheckPointCB_NotifyColide);
        for (a, v) in [
            (pExtra, APIcons::GetApiExtra(e)),
            (pGameObj, APIcons::GetApiGameObj(e)),
            (pPhysic, APIcons::GetApiPhysic(e)),
            (pGame, APIcons::GetApiGame(e)),
            (pHud, APIcons::GetApiHUD(e)),
        ] {
            let a = G(e, a);
            e.w32(a, v);
        }
        for (a, n) in [(pMiscFunc, "MiscFunc"), (pText, "ApiText")] {
            let s = e.cstr(n);
            let v = APIcons::GetApiCustom(e, s);
            let a = G(e, a);
            e.w32(a, v);
        }
        let slot = G(e, pGlobalDataHSCheckPoint);
        APIcons::GetGlobalData(e, slot, 0x40);
        let b = APIgameobj::GetGameBuffer(e, 0, 0x8f8, 1);
        let gd = GD(e);
        e.w32(gd + 0x3c, b);
    }
    let slot = G(e, pLocalDataHSCheckPoint);
    APIcons::GetLocalData(e, slot, 4);
    APIphysic::CreatePhysic(e, SELF, 0);
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 3, 2);
    APIcons::SetMessageFunction(e, hs_CheckPointFUNC_MessageFunction);
    APIcons::SetInitialState(e, 0);
    LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 2, NULL, 0);
    APIphysic::PrimSetIntAttribBits(e, SELF, 0, 0, 4);
    APIphysic::PrimSetIntAttribBits(e, SELF, 0, 2, 0x800);
    Ok(())
}

/// hs_checkpointDestructor @0x000112f0 (0x000112c0 in f40092d0/751a9264)
pub fn hs_checkpointDestructor(e: &mut Engine, _: i32) -> LResult<()> {
    Ok(())
}

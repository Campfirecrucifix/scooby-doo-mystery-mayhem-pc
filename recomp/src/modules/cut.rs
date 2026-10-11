//! `cut`, `cut1`..`cut8`: OVERALL_STATUS/CUTSCENE stand-ins that hide themselves (clipping) during a
//! cut-scene and restore it afterwards. Port of `Game Logic Modules/OVERALL_STATUS/CUTSCENE/cut*`
//! (6 functions each). Identical code except `cut` additionally sends msg 0xdf to 0x2a10000c at the end
//! of a cut-scene. Quirk kept: the clipping-params pointer read from L+4 is never set (zero-filled local),
//! so the original Get/SetClippingParams go through a NULL pointer.
#![allow(non_snake_case, non_upper_case_globals, unused_variables)]

use super::modkit::*;
use crate::engine::*;
use crate::logic::{LResult, ModuleDef};

macro_rules! def {
    ($id:ident, $name:literal) => {
        pub static $id: ModuleDef = ModuleDef {
            name: $name,
            data_size: 0x20,
            ctor: cutConstructor,
            dtor: cutDestructor,
            register: None,
            init: &[],
            build_init: None,
        };
    };
}
def!(MODULE, "cut");
def!(MODULE1, "cut1");
def!(MODULE2, "cut2");
def!(MODULE3, "cut3");
def!(MODULE4, "cut4");
def!(MODULE5, "cut5");
def!(MODULE6, "cut6");
def!(MODULE7, "cut7");
def!(MODULE8, "cut8");

pub const pExtra: u32 = 0x10000;
pub const pGameObj: u32 = 0x10004;
pub const pPhysic: u32 = 0x10008;
pub const pAnim: u32 = 0x1000c;
pub const pAnimMngr: u32 = 0x10010;
pub const pMiscFunc: u32 = 0x10014;
pub const pLocalDataCut: u32 = 0x10018;
pub const pGlobalDataCut: u32 = 0x1001c;

pub fn CutST_FIRSTMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 0x16, 5);
    APIgameobj::SetNextState(e, SELF, 1);
    Ok(0)
}
pub fn CutST_BASICMOVE_SET(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}
pub fn CutST_BASICMOVE_LOOP(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}
pub fn CutCB_CUTSCENE(e: &mut Engine, p: u32) -> LResult<u64> {
    let clip = e.r32(pw(e, pLocalDataCut, 4));
    match e.r32(p + 4) {
        1 => {
            APIgameobj::GetClippingParams(e, SELF, clip);
            LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 2, NULL, 1);
            APIgameobj::SetNextState(e, SELF, 1);
        }
        0 => {
            if e.rt.codes[e.rt.current_code().unwrap()].def.name == "cut" {
                APIgameobj::SendMessageTo(e, 0x2a10000c, NULL, 0xdf, 0, NULL);
            }
            APIgameobj::SetClippingParams(e, SELF, clip, 0);
            APIgameobj::SetNextState(e, SELF, 1);
        }
        _ => {}
    }
    Ok(0)
}
pub fn CutFUNC_MessageHandler(e: &mut Engine, _: u32, _: u32, _: Ptr) -> LResult<u64> {
    Ok(0)
}
pub fn cutConstructor(e: &mut Engine, api: Ptr, n: i32) -> LResult<()> {
    if n == 1 {
        ext(e, &[(0, CutST_FIRSTMOVE), (1, CutST_BASICMOVE_SET), (2, CutST_BASICMOVE_LOOP), (5, CutCB_CUTSCENE)]);
        let v = APIcons::GetApiAnim(e); gset(e, pAnim, v);
        let v = APIcons::GetApiGameObj(e); gset(e, pGameObj, v);
        let v = APIcons::GetApiExtra(e); gset(e, pExtra, v);
        let v = APIcons::GetApiPhysic(e); gset(e, pPhysic, v);
        custom(e, pAnimMngr, "AnimMngr");
        custom(e, pMiscFunc, "MiscFunc");
        let s = G(e, pGlobalDataCut);
        APIcons::GetGlobalData(e, s, 4);
        let b = APIgameobj::GetGameBuffer(e, 0, 0x8f8, 1);
        let g = pw(e, pGlobalDataCut, 0);
        e.w32(g, b);
    }
    APIcons::SetMessageFunction(e, CutFUNC_MessageHandler);
    let s = G(e, pLocalDataCut);
    APIcons::GetLocalData(e, s, 8);
    let leader = e.r32(e.r32(pw(e, pGlobalDataCut, 0)) + 8);
    LIBMiscFunc::MiscFunc_SetClipping(e, SELF, leader, 0);
    APIcons::SetInitialState(e, 0);
    Ok(())
}
pub fn cutDestructor(e: &mut Engine, n: i32) -> LResult<()> {
    Ok(())
}

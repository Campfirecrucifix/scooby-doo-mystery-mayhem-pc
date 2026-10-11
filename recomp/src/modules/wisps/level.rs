//! The per-level source file of each wisps build (`Source/<episode>/COLLECTABLES/<level>/LNN_Wisps.c`):
//! the module entry points `<level>_wispsConstructor` / `<level>_wispsDestructor`. 19 builds only
//! forward to the shared constructor/destructor; l03a_wisps__44cbb300 and l04_wisps__ed7e7a13 add
//! functions. The port has one entry pair that dispatches on the build.
use super::*;
use crate::engine::*;
use crate::logic::LResult;

/// The module entry point (`<level>_wispsConstructor` of the loaded build).
pub fn wisps_levelConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    match builds::BUILDS[e.rt.cur_build()].sha8 {
        "44cbb300" => l03a_wispsConstructor_44cbb300(e, api, instanceCount),
        "ed7e7a13" => l04_wispsConstructor_ed7e7a13(e, api, instanceCount),
        _ => lNN_wispsConstructor(e, api, instanceCount),
    }
}

/// The module exit point (`<level>_wispsDestructor`): the same in every build.
pub fn wisps_levelDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    lNN_wispsDestructor(e, instanceCount)
}

/// `l01_wispsConstructor`, `l01a_wispsConstructor`, ... (every build but the two below)
pub fn lNN_wispsConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    wisp_baseConstructor(e, api, instanceCount)
}

/// `l01_wispsDestructor`, ... (every build)
pub fn lNN_wispsDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    wisp_baseDestructor(e, instanceCount)
}

// ---- l03a_wisps__44cbb300 (Ep3 level 03a, L03A_Wisps.c) ----

/// 0x00017d68 message 0xe7: deactivate (clear 0x10000000, go Inactive) before the shared handler
pub fn L03A_WispsFUNC_MyMessageFunction(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    let puVar1 = L(e);
    if param_1 == 0xe7 {
        let f = e.r32(puVar1 + 4);
        e.w32(puVar1 + 4, f & 0xefff_ffff);
        Wisp_baseState_Inactive_In(e, puVar1);
    }
    Wisp_baseFUNC_MyMessageFunction(e, param_1, param_2, param_3)
}

/// 0x00017e38 callback 3 (notify 0x16, cut scene {1, end?}): wisp 0x2a0000c0 is prepared for / restored
/// after the cut scene (its clipping params pointer at L+0x74); the others are disabled / enabled
pub fn Wisp_baseCB_CUT_SCENE(e: &mut Engine, param_1: u32) -> LResult<u64> {
    let iVar2 = L(e);
    if e.r32(param_1) == 1 {
        if e.r32(param_1 + 4) == 0 {
            if e.r32(iVar2 + 0x94) == 0x2a00_00c0 {
                let uVar3 = APIgameobj::GetID(e, SELF, NULL);
                let c = e.r32(iVar2 + 0x74);
                LIBMiscFunc::MiscFunc_EndCutscene(e, uVar3, c);
                let f = e.r32(iVar2 + 4);
                e.w32(iVar2 + 4, f | 0x200_0000);
            } else {
                APIgameobj::EnableObject(e, SELF, 1);
            }
        } else if e.r32(iVar2 + 0x94) == 0x2a00_00c0 {
            let c = e.r32(iVar2 + 0x74);
            APIgameobj::GetClippingParams(e, SELF, c);
            let uVar3 = APIgameobj::GetID(e, SELF, NULL);
            LIBMiscFunc::MiscFunc_InitForCutscene(e, uVar3);
            let f = e.r32(iVar2 + 4);
            e.w32(iVar2 + 4, f | 0x100_0000);
        } else {
            APIgameobj::EnableObject(e, SELF, 0);
        }
    }
    Ok(0)
}

/// 0x00017ff0 `l03a_wispsConstructor` of build 44cbb300
pub fn l03a_wispsConstructor_44cbb300(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    wisp_baseConstructor(e, api, instanceCount)?;
    if instanceCount == 1 {
        APIcons::SetExternalFunction(e, 3, Wisp_baseCB_CUT_SCENE);
    }
    APIcons::SetMessageFunction(e, L03A_WispsFUNC_MyMessageFunction);
    APIgameobj::RegisterNotify(e, SELF, 1, 0x16, 3);
    Ok(())
}

// ---- l04_wisps__ed7e7a13 (Ep1 level 04, L04_Wisps.c) ----

/// 0x000176b0 message 0xc9: kill every launched wisp still alive and empty the list
pub fn Wisp_L04FUNC_MyMessageFunction(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    if param_1 == 0xc9 {
        for iStack_34 in 0..0x10u32 {
            let a = GD(e) + 0x26c + iStack_34 * 4;
            let id = e.r32(a);
            let iVar1 = APIgameobj::GetID(e, id, NULL);
            if iVar1 != 0x7fff_fffe {
                let a = GD(e) + 0x26c + iStack_34 * 4;
                let id = e.r32(a);
                APIgameobj::KillObject(e, id);
                e.w32(a, 0);
            }
        }
        let gd = GD(e);
        e.w32(gd + 0x10, 0);
    }
    Wisp_baseFUNC_MyMessageFunction(e, param_1, param_2, param_3)
}

/// 0x00017820 `l04_wispsConstructor` of build ed7e7a13
pub fn l04_wispsConstructor_ed7e7a13(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    wisp_baseConstructor(e, api, instanceCount)?;
    APIcons::SetMessageFunction(e, Wisp_L04FUNC_MyMessageFunction);
    Ok(())
}

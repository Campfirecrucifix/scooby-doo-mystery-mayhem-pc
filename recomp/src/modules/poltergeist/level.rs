//! The per-level source files of each poltergeist build (`ep2_poltergeist.c` / `ep3_poltergeist.c` +
//! `Poltergeist.c`): the module entry points, the level's message function and init. The port has
//! one entry pair ([`poltergeist_levelConstructor`] / [`poltergeist_levelDestructor`]) that
//! dispatches on the loaded build. Addresses are module-relative (.text at 0x10000) of the build
//! named.
use super::poltergeist_base::addr;
use super::*;
use crate::engine::*;
use crate::logic::LResult;

/// The module entry point (`poltergeistConstructor` of the loaded build).
pub fn poltergeist_levelConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    match build_key(e) {
        "0bb93729" => poltergeistConstructor_EP2L01C(e, api, instanceCount),
        "04b00cff" | "7d96cf48" => poltergeistConstructor_ep3(e, api, instanceCount),
        _ => poltergeistConstructor_ep2(e, api, instanceCount),
    }
}

/// The module exit point (`poltergeistDestructor` of the loaded build).
pub fn poltergeist_levelDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    match build_key(e) {
        "04b00cff" | "7d96cf48" => ep3_poltergeistDestructor(e),
        _ => ep2_poltergeistDestructor(e),
    }
}

// ---- ep2_poltergeist.c / ep3_poltergeist.c (identical) ----

pub fn ep2_poltergeistConstructor(e: &mut Engine, api: Ptr, param_2: i32) -> LResult<()> {
    poltergeist_baseConstructor(e, api, param_2)
}

/// The decompiled destructor passes an uninitialised register as the instance count;
/// poltergeist_baseDestructor does not read it.
pub fn ep2_poltergeistDestructor(e: &mut Engine) -> LResult<()> {
    poltergeist_baseDestructor(e, 0)
}

pub fn ep3_poltergeistConstructor(e: &mut Engine, api: Ptr, param_2: i32) -> LResult<()> {
    poltergeist_baseConstructor(e, api, param_2)
}

pub fn ep3_poltergeistDestructor(e: &mut Engine) -> LResult<()> {
    poltergeist_baseDestructor(e, 0)
}

/// `poltergeist_Func_Initialisation` of the builds whose Poltergeist.c only sets the monster type:
/// 6 (fa6a1f63: 7, the flying poltergeists that attack from GoGrab; 7d96cf48 also allows the jump on
/// the throwable, L+400 = 1).
pub fn poltergeist_Func_Initialisation(e: &mut Engine) -> LResult<()> {
    let l = L(e);
    match build_key(e) {
        "0bb93729" => return poltergeist_Func_Initialisation_EP2L01C(e),
        "fa6a1f63" => e.w32(l + 0x44, 7),
        "7d96cf48" => {
            e.w32(l + 0x44, 6);
            e.w32(l + 400, 1);
        }
        _ => e.w32(l + 0x44, 6),
    }
    Ok(())
}

/// `poltergeist_FUNC_MessageFunction` of the loaded build.
pub fn poltergeist_FUNC_MessageFunction(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    match build_key(e) {
        "0bb93729" => poltergeist_FUNC_MessageFunction_EP2L01C(e, param_1, param_2, param_3),
        "1c9d41ec" => poltergeist_FUNC_MessageFunction_EP2L09(e, param_1, param_2, param_3),
        "312143a4" => poltergeist_FUNC_MessageFunction_EP2L01A(e, param_1, param_2, param_3),
        "b8041da2" => poltergeist_FUNC_MessageFunction_EP2L01B(e, param_1, param_2, param_3),
        "fa6a1f63" => poltergeist_FUNC_MessageFunction_EP2L02B(e, param_1, param_2, param_3),
        // 04b00cff (EP3L01), 7d96cf48 (EP3L02): straight to the base
        _ => Poltergeist_BaseFUNC_MessageFunction(e, param_1, param_2, param_3),
    }
}

/// `poltergeistConstructor` of every build but 0bb93729 (ep2 entry)
pub fn poltergeistConstructor_ep2(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    ep2_poltergeistConstructor(e, api, instanceCount)?;
    APIcons::SetMessageFunction(e, poltergeist_FUNC_MessageFunction);
    poltergeist_Func_Initialisation(e)?;
    Monster_Base_Func_Initialise(e)
}

/// `poltergeistConstructor` of 04b00cff / 7d96cf48 (ep3 entry)
pub fn poltergeistConstructor_ep3(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    ep3_poltergeistConstructor(e, api, instanceCount)?;
    APIcons::SetMessageFunction(e, poltergeist_FUNC_MessageFunction);
    poltergeist_Func_Initialisation(e)?;
    Monster_Base_Func_Initialise(e)
}

/// "drop what I ride" check shared by EP2L01C: `L+0x184` is a real object and I am on it (L+0x174)
fn riding(e: &Engine, l: Ptr) -> bool {
    let o = e.r32(l + 0x184);
    o != 0 && o != 0x7fff_fffe && e.r32(l + 0x174) == 1
}

// ---- LEVEL_01C/Poltergeist.c (0bb93729, EP2L01C: the gliding poltergeists) ----
// While game buffer 2 +0 bit 8 is clear the poltergeists are "gliders" (type 8): they wait in
// poltergeist_ST_BlackHoleLoop, then (0x119) go grab a throwable (prop 0x184 = param_2), ride it
// (0x113: AddChild, 0xe0 to the object) and wait again. Once the bit is set (or on 0x11a) they are
// normal ground poltergeists (type 6, game buffer 2 +0xc |= 0x400000).

/// 0x000167e8
pub fn poltergeist_Func_Initialisation_EP2L01C(e: &mut Engine) -> LResult<()> {
    let iVar1 = L(e);
    let gb2 = e.r32(GDP(e) + 4);
    if e.r32(gb2) & 8 == 0 {
        APIphysic::PrimResetIntAttribBits(e, SELF, 800, 5, 8);
        APIphysic::PrimSetIntAttribBits(e, SELF, 800, 5, 0x40_0000);
        e.w32(iVar1 + 0x160, addr::poltergeist_ST_BlackHoleLoop);
        e.w32(iVar1 + 0x44, 8);
        APIgameobj::UnregisterNotify(e, SELF, 0xffff_ffff, 0x14);
        e.w32(iVar1 + 0x1a4, 0);
    } else {
        let l = L(e);
        e.w32(l + 0x44, 6);
        let gb2 = e.r32(GDP(e) + 4);
        or32(e, gb2 + 0xc, 0x40_0000);
        poltergeist_FUNC_MessageFunction_EP2L01C(e, 0x11a, 0, NULL)?;
    }
    call_state(e)
}

/// 0x00016938
pub fn poltergeist_ST_BlackHoleLoop(e: &mut Engine) -> LResult<()> {
    Ok(())
}

/// 0x00016958 callback 0x16 (cutscene notification 0x16): start (+4 == 1) freeze and wait; end (0)
/// back at the start position, pre-logic again, context 7. `param_1` points at the notification data.
pub fn Poltergeist_CB_CUTSCENE(e: &mut Engine, param_1: u32) -> LResult<u64> {
    let v = e.r32(param_1 + 4);
    if v == 1 {
        APIphysic::SetFlags(e, SELF, 1);
        set_state(e, addr::poltergeist_ST_BlackHoleLoop);
    } else if v == 0 {
        let p = L(e) + 0x178;
        APIgameobj::SetTranslation(e, SELF, p);
        APIphysic::ResetFlags(e, SELF, 1);
        APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 0x14, 0x15);
        APIgameobj::SetContextFlag(e, SELF, 7);
    }
    Ok(0)
}

/// 0x00016a88 0x113 ride the object; 0xc9 restart (HUD 0x2c/4 outside levels 10..12 of game buffer
/// 0 +0x3c; gliders reset); 0x119 go grab object `param_2` (gliders only, returns 0); 0x11a become a
/// ground poltergeist (returns 0). Then Poltergeist_BaseFUNC_MessageFunction.
pub fn poltergeist_FUNC_MessageFunction_EP2L01C(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    let iVar1 = L(e);
    if param_1 == 0x113 {
        e.w32(iVar1 + 0x174, 1);
        let o = e.r32(iVar1 + 0x184);
        APIgameobj::AddChild(e, o, 6, SELF, 1);
        LIBAnimMngr::PlayAnim(e, 0.1, 0x12, 2, 0);
        let o = e.r32(iVar1 + 0x184);
        APIgameobj::SendMessageTo(e, o, NULL, 0xe0, 0, NULL);
        APIgameobj::SetContextFlag(e, SELF, 0x60);
        APIgameobj::UnregisterNotify(e, SELF, 0xffff_ffff, 0x14);
        e.w32(iVar1 + 0x160, addr::poltergeist_ST_BlackHoleLoop);
    } else if param_1 < 0x114 {
        if param_1 == 0xc9 {
            let lvl = e.r32(e.r32(GDP(e)) + 0x3c);
            if lvl < 10 || 0xc < lvl {
                APIgameobj::SendMessageTo(e, 0x2a10_0006, NULL, 0x2c, 4, NULL);
            }
            let gb2 = e.r32(GDP(e) + 4);
            if e.r32(gb2 + 0xc) & 0x40_0000 == 0 {
                let s = e.cstr(" --------------------- RESET POLTERGEISTS GLIDE2 ----------------------- \n");
                APIextra::null_printf(e, s, 0);
                e.w32(iVar1 + 0x44, 8);
                and32(e, gb2 + 0x1c, 0xffff_fffb);
                if riding(e, iVar1) {
                    APIgameobj::LeaveParent(e, SELF, 0);
                    let o = e.r32(iVar1 + 0x184);
                    APIgameobj::SendMessageTo(e, o, NULL, 0xda, 0, NULL);
                }
                APIgameobj::SetTranslation(e, SELF, iVar1 + 0x178);
                APIphysic::PrimResetIntAttribBits(e, SELF, 800, 5, 8);
                APIphysic::PrimSetIntAttribBits(e, SELF, 800, 5, 0x40_0000);
                APIgameobj::SetGroupBitMask(e, SELF, 0x20_0409);
            }
        }
    } else if param_1 == 0x119 {
        if e.r32(L(e) + 0x44) != 8 {
            return Ok(0);
        }
        APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 0x14, 0x15);
        if riding(e, iVar1) {
            let o = e.r32(iVar1 + 0x184);
            APIgameobj::SendMessageTo(e, o, NULL, 0xda, 0, NULL);
        }
        e.w32(iVar1 + 0x174, 0);
        e.w32(iVar1 + 0x184, param_2);
        APIgameobj::SetContextFlag(e, SELF, 7);
        APIgameobj::LeaveParent(e, SELF, 0);
        e.w32(iVar1 + 0x160, addr::Poltergeist_baseST_GoGrabThrowable_In);
        return Ok(0);
    } else if param_1 == 0x11a {
        let l = L(e);
        if riding(e, l) {
            let o = e.r32(l + 0x184);
            APIgameobj::SendMessageTo(e, o, NULL, 0xda, 0, NULL);
        }
        APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 0x14, 0x15);
        e.w32(iVar1 + 0x1a4, 1);
        APIgameobj::LeaveParent(e, SELF, 0);
        e.w32(iVar1 + 0x44, 6);
        APIgameobj::SetGroupBitMask(e, SELF, 0x20_0609);
        APIphysic::PrimResetIntAttribBits(e, SELF, 800, 5, 0x40_0000);
        APIphysic::PrimSetIntAttribBits(e, SELF, 800, 5, 8);
        e.w32(iVar1 + 0x160, addr::Poltergeist_baseST_Standing_In);
        return Ok(0);
    }
    Poltergeist_BaseFUNC_MessageFunction(e, param_1, param_2, param_3)
}

/// 0x000170a8 cutscene callback 0x16 (first instance); notification 0x16 from object 0
pub fn poltergeistConstructor_EP2L01C(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    ep2_poltergeistConstructor(e, api, instanceCount)?;
    if instanceCount == 1 {
        APIcons::SetExternalFunction(e, 0x16, Poltergeist_CB_CUTSCENE);
    }
    APIcons::SetMessageFunction(e, poltergeist_FUNC_MessageFunction);
    poltergeist_Func_Initialisation(e)?;
    Monster_Base_Func_Initialise(e)?;
    APIgameobj::RegisterNotify(e, SELF, 0, 0x16, 0x16);
    Ok(())
}

// ---- LEVEL_09/Poltergeist.c (1c9d41ec, EP2L09) ----

/// 0x00016868 0x143 / 0x144 sleep / wake; 0xc1 restart: base restart, then reset (returns 0);
/// 0xd6/0x68 captured: 0x11d to 0x2a00000d; 0x1d2 new path (`param_3` = its name). Then
/// Poltergeist_BaseFUNC_MessageFunction.
pub fn poltergeist_FUNC_MessageFunction_EP2L09(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    if param_1 == 0x143 {
        APIgameobj::Sleep(e, SELF, -1.0);
    } else if param_1 < 0x144 {
        if param_1 == 0xc1 {
            Poltergeist_BaseFUNC_MessageFunction(e, 0xc1, param_2, param_3)?;
            APIphysic::PrimSetIntAttribBits(e, SELF, 800, 5, 8);
            APIphysic::PrimSetIntAttribBits(e, SELF, 800, 5, 0x40_0000);
            APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 3, 0xc);
            APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 0xb, 0x12);
            APIgameobj::SetGroupID(e, SELF, 1);
            APIgameobj::SetGroupBitMask(e, SELF, 0x20_0609);
            APIgameobj::SetContextFlag(e, SELF, 0x60);
            let l = L(e);
            let v = e.r32(l + 0xb0);
            e.w32(l + 0x5c, v);
            e.w32(l + 0x188, 0);
            e.w32(l + 0x184, 0x7fff_fffe);
            e.w32(l + 0x160, addr::Poltergeist_baseST_Standing_In);
            return Ok(0);
        }
        if param_1 == 0xd6 && param_2 == 0x68 {
            APIgameobj::SendMessageTo(e, 0x2a00_000d, NULL, 0x11d, 0, NULL);
        }
    } else if param_1 == 0x144 {
        APIgameobj::Sleep(e, SELF, 0.0);
    } else if param_1 == 0x1d2 {
        let l = L(e);
        LIBApiPathFinding::SetPath(e, param_3 as i32, (l + 0x1e4) as i32);
        set_state(e, addr::Poltergeist_baseST_Standing_In);
    }
    Poltergeist_BaseFUNC_MessageFunction(e, param_1, param_2, param_3)
}

// ---- LEVEL_01A/Poltergeist.c (312143a4, EP2L01A) ----

/// 0x000167c0 0xb3 (the first poltergeist to engage, see Standing_Execute): laugh sound
pub fn poltergeist_FUNC_MessageFunction_EP2L01A(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    if param_1 == 0xb3 {
        APInewSound::Play(e, 0x2a00_004c, 0x8000_0003, 0);
    }
    Poltergeist_BaseFUNC_MessageFunction(e, param_1, param_2, param_3)
}

// ---- LEVEL_01B/Poltergeist.c (b8041da2, EP2L01B) ----

/// 0x000166f0
pub fn PoltergeistST_BlackLoop_In(e: &mut Engine) -> LResult<()> {
    LIBAnimMngr::PlayAnim(e, 0.1, 8, 2, 0);
    set_state(e, addr::PoltergeistST_BlackLoop_Execute);
    call_state(e)
}

/// 0x00016778
pub fn PoltergeistST_BlackLoop_Execute(e: &mut Engine) -> LResult<()> {
    Ok(())
}

/// 0x00016798 0xde: engage (Standing, pre-logic, context 0x60); 0x143: wait in the black loop (no
/// pre-logic). Then Poltergeist_BaseFUNC_MessageFunction.
pub fn poltergeist_FUNC_MessageFunction_EP2L01B(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    if param_1 == 0xde {
        set_state(e, addr::Poltergeist_baseST_Standing_In);
        APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 0x14, 0x15);
        APIgameobj::SetContextFlag(e, SELF, 0x60);
    } else if param_1 == 0x143 {
        APIgameobj::SetContextFlag(e, SELF, 0x60);
        set_state(e, addr::PoltergeistST_BlackLoop_In);
        APIgameobj::UnregisterNotify(e, SELF, 0xffff_ffff, 0x14);
    }
    Poltergeist_BaseFUNC_MessageFunction(e, param_1, param_2, param_3)
}

// ---- LEVEL_02B/Poltergeist.c (fa6a1f63, EP2L02B) ----

/// 0x00016798
pub fn PoltergeistST_BlackLoopOfDoom_In(e: &mut Engine) -> LResult<()> {
    LIBAnimMngr::PlayAnim(e, 0.1, 6, 2, 0);
    set_state(e, addr::PoltergeistST_BlackLoopOfDoom_Execute);
    call_state(e)
}

/// 0x00016820
pub fn PoltergeistST_BlackLoopOfDoom_Execute(e: &mut Engine) -> LResult<()> {
    Ok(())
}

/// 0x00016840 0xdf: wait in the black loop; 0xde: engage (Standing); 0x136: collision attributes
/// (prim 800 attrib 5: 8 -> 0x400000). Then Poltergeist_BaseFUNC_MessageFunction.
pub fn poltergeist_FUNC_MessageFunction_EP2L02B(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    if param_1 == 0xdf {
        APIgameobj::SetContextFlag(e, SELF, 0x60);
        set_state(e, addr::PoltergeistST_BlackLoopOfDoom_In);
    } else if param_1 < 0xe0 {
        if param_1 == 0xde {
            set_state(e, addr::Poltergeist_baseST_Standing_In);
            APIgameobj::SetContextFlag(e, SELF, 0x60);
        }
    } else if param_1 == 0x136 {
        APIphysic::PrimResetIntAttribBits(e, SELF, 800, 5, 8);
        APIphysic::PrimSetIntAttribBits(e, SELF, 800, 5, 0x40_0000);
    }
    Poltergeist_BaseFUNC_MessageFunction(e, param_1, param_2, param_3)
}

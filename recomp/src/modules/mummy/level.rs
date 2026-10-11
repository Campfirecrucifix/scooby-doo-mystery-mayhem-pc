//! The per-level source files of each mummy build (`MummyEpisodeTwo.c` + `MummyLevel<N>.c`): the
//! module entry points, the level's message function and init. The port has one entry pair
//! ([`mummy_levelConstructor`] / [`mummy_levelDestructor`]) that dispatches on the loaded build.
//! Addresses are module-relative (.text at 0x10000) of the build named.
use super::*;
use crate::engine::*;
use crate::logic::LResult;

/// The module entry point (`<level>Constructor` of the loaded build).
pub fn mummy_levelConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    match build_key(e) {
        "mummyleveltwoa" => mummyleveltwoaConstructor(e, api, instanceCount),
        "mummyleveltwob" => mummyleveltwobConstructor(e, api, instanceCount),
        "mummyleveltwoc" => mummyleveltwocConstructor(e, api, instanceCount),
        "mummylevelthreea" => mummylevelthreeaConstructor(e, api, instanceCount),
        "mummylevelnine" => mummylevelnineConstructor(e, api, instanceCount),
        k => panic!("mummy: no level constructor for build {k}"),
    }
}

/// The module exit point. Every level destructor is `mummyepisodetwoDestructor()`.
pub fn mummy_levelDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    match build_key(e) {
        "mummyleveltwoa" => mummyleveltwoaDestructor(e, instanceCount),
        "mummyleveltwob" => mummyleveltwobDestructor(e, instanceCount),
        "mummyleveltwoc" => mummyleveltwocDestructor(e, instanceCount),
        "mummylevelthreea" => mummylevelthreeaDestructor(e, instanceCount),
        "mummylevelnine" => mummylevelnineDestructor(e, instanceCount),
        k => panic!("mummy: no level destructor for build {k}"),
    }
}

// ---- MummyEpisodeTwo.c ----

/// 0x0001a928 (L02A)
pub fn mummyepisodetwoConstructor(e: &mut Engine, api: Ptr, param_2: i32) -> LResult<()> {
    mummy_baseConstructor(e, api, param_2)
}

/// 0x0001a978 (L02A)
pub fn mummyepisodetwoDestructor(e: &mut Engine) -> LResult<()> {
    mummy_baseDestructor(e)
}

/// gb2 +8 & 0x4000000 (an Ep2 story flag: the L02A coliseum mummies 0x2a000100 / 0x2a000101 are awake)
fn coliseum_awake(e: &Engine) -> bool {
    e.r32(e.r32(GDMu(e) + 4) + 8) & 0x400_0000 != 0
}

// ---- MummyLevelTwoA.c (mummyleveltwoa) ----

/// 0x0001a9d0 external 0x10: the two coliseum mummies, once awake, start visible at the path node
/// nearest their sarcophagus (-> the start position L+0xf8)
pub fn Mummy_LevelTwoAST_FIRSTMOVE_SET(e: &mut Engine, p: u32) -> LResult<u64> {
    let iVar1 = L(e);
    let iVar2 = APIgameobj::GetID(e, SELF, NULL);
    if (iVar2 == 0x2a00_0100 || iVar2 == 0x2a00_0101) && coliseum_awake(e) {
        APIgameobj::SetGameObjectVisible(e, SELF, 1);
        let l = L(e);
        LIBApiPathFinding::SetPath(e, (iVar1 + 0x294) as i32, (l + 0x310) as i32);
        let DStack_40 = e.alloca(0x10);
        APIgameobj::GetPosition(e, SELF, ALL, DStack_40);
        let l = L(e);
        let s = e.r32(l + 0x194);
        APIgameobj::GetPosition(e, s, ALL, l + 0x1e4);
        let l = L(e);
        let lm = LM(e);
        LIBApiPathFinding::FindPath(e, DStack_40, l + 0x1e4, lm + 0xf8, (l + 0x310) as i32);
        let lm = LM(e);
        APIgameobj::SetTranslation(e, SELF, lm + 0xf8);
    }
    Mummy_BaseST_FIRSTMOVE_SET(e, p)
}

/// 0x0001abb0
pub fn MummyLevelTwoA_Func_Initialisation(e: &mut Engine) -> LResult<()> {
    e.w32(L(e) + 0x44, 5);
    Ok(())
}

/// 0x0001abe0 external 0x18: cut-scene 2 for the coliseum mummies: start: save the clipping,
/// cut-scene mode; end: restore and wake both (0xe6)
pub fn MummyLevelTwoA_CBCutScene(e: &mut Engine, param_1: u32) -> LResult<u64> {
    let iVar2 = APIgameobj::GetID(e, SELF, NULL);
    let (id, start) = (e.r32(param_1), e.r32(param_1 + 4));
    let ours = iVar2 == 0x2a00_0100 || iVar2 == 0x2a00_0101;
    if id == 2 && ours && start == 1 {
        let p = e.r32(L(e) + 800);
        APIgameobj::GetClippingParams(e, SELF, p);
        let uVar3 = APIgameobj::GetID(e, SELF, NULL);
        LIBMiscFunc::MiscFunc_InitForCutscene(e, uVar3);
    } else if id == 2 && ours && start == 0 {
        let uVar3 = APIgameobj::GetID(e, SELF, NULL);
        let p = e.r32(L(e) + 800);
        LIBMiscFunc::MiscFunc_EndCutscene(e, uVar3, p);
        APIgameobj::SendMessageTo(e, 0x2a00_0100, NULL, 0xe6, 0, NULL);
        APIgameobj::SendMessageTo(e, 0x2a00_0101, NULL, 0xe6, 0, NULL);
    }
    Ok(0)
}

/// 0x0001adf0 0x165: behaviour "asleep"
pub fn MummyLevelTwoA_MessageFunction(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    if param_1 == 0x165 {
        e.w32(L(e) + 0x198, 10);
    }
    Mummy_BaseMessageFonction(e, param_1, param_2, param_3)
}

/// 0x0001ae90
pub fn mummyleveltwoaConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    mummyepisodetwoConstructor(e, api, instanceCount)?;
    if instanceCount == 1 {
        APIcons::SetExternalFunction(e, 0x18, MummyLevelTwoA_CBCutScene);
        APIcons::SetExternalFunction(e, 0x10, Mummy_LevelTwoAST_FIRSTMOVE_SET);
    }
    APIcons::SetMessageFunction(e, MummyLevelTwoA_MessageFunction);
    let iVar1 = APIgameobj::GetID(e, SELF, NULL);
    if (iVar1 == 0x2a00_0100 || iVar1 == 0x2a00_0101) && coliseum_awake(e) {
        e.w32(L(e) + 0x198, 10);
    }
    MummyLevelTwoA_Func_Initialisation(e)?;
    Monster_Base_Func_Initialise(e)?;
    mummy_base_Func_Initialisation(e)?;
    APIphysic::PrimSetIntAttribBits(e, SELF, 700, 5, 0x200_0000);
    APIphysic::PrimSetIntAttribBits(e, SELF, 700, 5, 0x1000);
    APIgameobj::RegisterNotify(e, SELF, 3, 0x16, 0x18);
    Ok(())
}

/// 0x0001b060
pub fn mummyleveltwoaDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    mummyepisodetwoDestructor(e)
}

// ---- MummyLevelTwoB.c (mummyleveltwob) ----

/// 0x0001a960
pub fn MummyLevelTwoB_Func_Initialisation(e: &mut Engine) -> LResult<()> {
    e.w32(L(e) + 0x44, 5);
    Ok(())
}

/// 0x0001a990 0x141 / 0x142: physics response off / on; 0xdf: context 0x60
pub fn MummyLevelTwoB_MessageFunction(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    match param_1 {
        0x141 => {
            APIphysic::SetFlags(e, SELF, 1);
        }
        0xdf => {
            APIgameobj::SetContextFlag(e, SELF, 0x60);
        }
        0x142 => {
            APIphysic::ResetFlags(e, SELF, 1);
        }
        _ => {}
    }
    Mummy_BaseMessageFonction(e, param_1, param_2, param_3)
}

/// 0x0001aab0
pub fn mummyleveltwobConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    mummyepisodetwoConstructor(e, api, instanceCount)?;
    APIcons::SetMessageFunction(e, MummyLevelTwoB_MessageFunction);
    MummyLevelTwoB_Func_Initialisation(e)?;
    Monster_Base_Func_Initialise(e)?;
    mummy_base_Func_Initialisation(e)?;
    APIphysic::PrimSetIntAttribBits(e, SELF, 700, 5, 0x10_0000);
    Ok(())
}

/// 0x0001ab80
pub fn mummyleveltwobDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    mummyepisodetwoDestructor(e)
}

// ---- MummyLevelTwoC.c (mummyleveltwoc) ----

/// 0x0001a998
pub fn MummyLevelTwoC_Func_Initialisation(e: &mut Engine) -> LResult<()> {
    e.w32(L(e) + 0x44, 5);
    Ok(())
}

/// 0x0001a9c8 external 0x18: cut-scene 1: start: save the clipping, cut-scene mode, state 5; end:
/// restore, state 0x11, prim bit 8 off, walk out of the sarcophagus
pub fn Mummy_Local_CB_CUT_SCENE(e: &mut Engine, param_1: u32) -> LResult<u64> {
    let (id, start) = (e.r32(param_1), e.r32(param_1 + 4));
    if start == 1 && id == 1 {
        let p = e.r32(L(e) + 800);
        APIgameobj::GetClippingParams(e, SELF, p);
        let uVar2 = APIgameobj::GetID(e, SELF, NULL);
        LIBMiscFunc::MiscFunc_InitForCutscene(e, uVar2);
        APIgameobj::SetNextState(e, SELF, 5);
    } else if start == 0 && id == 1 {
        let uVar2 = APIgameobj::GetID(e, SELF, NULL);
        let p = e.r32(L(e) + 800);
        LIBMiscFunc::MiscFunc_EndCutscene(e, uVar2, p);
        APIgameobj::SetNextState(e, SELF, 0x11);
        APIphysic::PrimResetIntAttribBits(e, SELF, 700, 5, 8);
        set_state(e, addr::Mummy_BaseST_GET_OUT_SET);
    }
    Ok(0)
}

/// 0x0001ab70 0xde: visible, wake up; 0xc9: into a sarcophagus (behaviour 11, the path, no timers,
/// sarcophagus by id: 0x2a0000d0 -> 0x2a0000cf, 0x2a0000d1 -> 0x2a0000d6, 0x2a0000d2 -> 0x2a0000d7);
/// 0x1d2: moved to 0x2a0000d4 on "Path_Mummy_Tomb02" (written to L+0x2d4, set from there), asleep
/// with no sarcophagus, msg 0xeb/1 to 0x2a00002b, chase.
/// NOTE (PS2 behaviour kept): the path name is copied to L+0x2d4, which overlaps the end of the
/// L+0x294 name buffer and the "has a path" flag L+0x2d0 is left as it was.
pub fn MummyLevelTwoC_MessageFunction(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    match param_1 {
        0xde => {
            APIgameobj::SetGameObjectVisible(e, SELF, 1);
            APIgameobj::Sleep(e, SELF, 0.0);
        }
        0xc9 => {
            e.w32(L(e) + 0x198, 0xb);
            let l = L(e);
            LIBApiPathFinding::SetPath(e, (l + 0x294) as i32, (l + 0x310) as i32);
            let t = e.r32(L(e) + 0x18c);
            APIextra::TimerUnregisterNotify(e, SELF, t);
            let t = e.r32(L(e) + 0x184);
            APIextra::TimerUnregisterNotify(e, SELF, t);
            let uStack_40 = APIgameobj::GetID(e, SELF, NULL);
            let s = match uStack_40 {
                0x2a00_00d1 => Some(0x2a00_00d6),
                0x2a00_00d0 => Some(0x2a00_00cf),
                0x2a00_00d2 => Some(0x2a00_00d7),
                _ => None,
            };
            if let Some(s) = s {
                e.w32(L(e) + 0x194, s);
            }
        }
        0x1d2 => {
            APIphysic::ResetFlags(e, SELF, 1);
            APIphysic::PrimSetIntAttribBits(e, SELF, 700, 5, 8);
            let n = e.cstr("Path_Mummy_Tomb02");
            let l = L(e);
            LIBMiscFunc::MiscFunc_StrncpySafe(e, l + 0x2d4, n, 0x3c);
            let DStack_50 = e.alloca(0x10);
            APIgameobj::GetPosition(e, 0x2a00_00d4, ALL, DStack_50);
            APIgameobj::SetTranslation(e, SELF, DStack_50);
            APIgameobj::TakeOrientation(e, SELF, 0x2a00_00d4, ALL);
            let l = L(e);
            LIBApiPathFinding::SetPath(e, (l + 0x2d4) as i32, (l + 0x310) as i32);
            e.w32(L(e) + 0x198, 10);
            e.w32(L(e) + 0x194, 0);
            APIgameobj::SetGroupBitMask(e, SELF, 0x200611);
            APIgameobj::SendMessageTo(e, 0x2a00_002b, NULL, 0xeb, 1, NULL);
            set_state(e, addr::Mummy_BaseST_FOLLOW_HERO_SET);
        }
        _ => {}
    }
    Mummy_BaseMessageFonction(e, param_1, param_2, param_3)
}

/// 0x0001af60 hearing range 25 m; 0x2a0000d0 gets the cut-scene notify
pub fn mummyleveltwocConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    mummyepisodetwoConstructor(e, api, instanceCount)?;
    if instanceCount == 1 {
        APIcons::SetExternalFunction(e, 0x18, Mummy_Local_CB_CUT_SCENE);
    }
    APIcons::SetMessageFunction(e, MummyLevelTwoC_MessageFunction);
    MummyLevelTwoC_Func_Initialisation(e)?;
    Monster_Base_Func_Initialise(e)?;
    mummy_base_Func_Initialisation(e)?;
    let iVar1 = APIgameobj::GetID(e, SELF, NULL);
    if iVar1 == 0x2a00_00d0 {
        APIgameobj::RegisterNotify(e, SELF, 1, 0x16, 0x18);
    }
    e.w32(L(e) + 0x1f8, 0x41c8_0000);
    Ok(())
}

/// 0x0001b090
pub fn mummyleveltwocDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    mummyepisodetwoDestructor(e)
}

// ---- MummyLevelThreeA.c (mummylevelthreea) ----

/// 0x0001aa90
pub fn MummyLevelThreeA_Func_Initialisation(e: &mut Engine) -> LResult<()> {
    e.w32(L(e) + 0x44, 5);
    Ok(())
}

/// 0x0001aac0 out of the sarcophagus: the path, tell it 0xfc/1, stand at the path node nearest it,
/// prim bit 8, group mask 0x200611, visible
pub fn MummyLevelThreeA_Set(e: &mut Engine) -> LResult<()> {
    let l = L(e);
    LIBApiPathFinding::SetPath(e, (l + 0x294) as i32, (l + 0x310) as i32);
    let DStack_30 = e.alloca(0x10);
    APIgameobj::GetPosition(e, SELF, ALL, DStack_30);
    let l = L(e);
    let s = e.r32(l + 0x194);
    APIgameobj::GetPosition(e, s, ALL, l + 0x1e4);
    let s = e.r32(L(e) + 0x194);
    APIgameobj::SendMessageTo(e, s, NULL, 0xfc, 1, NULL);
    let l = L(e);
    LIBApiPathFinding::FindPath(e, DStack_30, l + 0x1e4, l + 0x1e4, (l + 0x310) as i32);
    let l = L(e);
    APIgameobj::SetTranslation(e, SELF, l + 0x1e4);
    APIphysic::PrimSetIntAttribBits(e, SELF, 700, 5, 8);
    APIgameobj::SetGroupBitMask(e, SELF, 0x200611);
    APIgameobj::SetGameObjectVisible(e, SELF, 1);
    Ok(())
}

/// 0x0001ac68 start position = the path node nearest the sarcophagus (from its own position), visible
pub fn MummyLevelThreeA_Set_InitialPos(e: &mut Engine) -> LResult<()> {
    let l = L(e);
    let s = e.r32(l + 0x194);
    APIgameobj::GetPosition(e, s, ALL, l + 0x1e4);
    let l = L(e);
    LIBApiPathFinding::FindPath(e, l + 0x1e4, l + 0x1e4, l + 0x1e4, (l + 0x310) as i32);
    copy_start_pos(e);
    APIgameobj::SetGameObjectVisible(e, SELF, 1);
    Ok(())
}

/// LM+0xf8..0x100 = L+0x1e4..0x1ec
fn copy_start_pos(e: &mut Engine) {
    for o in [0, 4, 8] {
        let v = e.r32(L(e) + 0x1e4 + o);
        let lm = LM(e);
        e.w32(lm + 0xf8 + o, v);
    }
}

/// 0x0001ad70 0xde: asleep at the initial position; 0x165: asleep, out of the sarcophagus (-> start
/// position), re-initialise
pub fn MummyLevelThreeA_MessageFunction(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    if param_1 == 0xde {
        e.w32(L(e) + 0x198, 10);
        MummyLevelThreeA_Set_InitialPos(e)?;
    } else if param_1 == 0x165 {
        e.w32(L(e) + 0x198, 10);
        MummyLevelThreeA_Set(e)?;
        copy_start_pos(e);
        mummy_base_Func_Initialisation(e)?;
    }
    Mummy_BaseMessageFonction(e, param_1, param_2, param_3)
}

/// 0x0001aeb0 external 0x18: cut-scene 0 only; at its start step out of the sarcophagus first
pub fn MummyLevelThreeA_CB_CUT_SCENE(e: &mut Engine, param_1: u32) -> LResult<u64> {
    if e.r32(param_1) == 0 {
        if e.r32(param_1 + 4) == 1 {
            MummyLevelThreeA_Set(e)?;
        }
        Mummy_BaseCB_CUT_SCENE(e, param_1)?;
    }
    Ok(0)
}

/// 0x0001af40 start position = the sarcophagus; cut-scene notify; clipping mode 2
pub fn mummylevelthreeaConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    mummyepisodetwoConstructor(e, api, instanceCount)?;
    if instanceCount == 1 {
        APIcons::SetExternalFunction(e, 0x18, MummyLevelThreeA_CB_CUT_SCENE);
    }
    APIcons::SetMessageFunction(e, MummyLevelThreeA_MessageFunction);
    MummyLevelThreeA_Func_Initialisation(e)?;
    Monster_Base_Func_Initialise(e)?;
    mummy_base_Func_Initialisation(e)?;
    let s = e.r32(L(e) + 0x194);
    let lm = LM(e);
    APIgameobj::GetPosition(e, s, ALL, lm + 0xf8);
    APIphysic::PrimSetIntAttribBits(e, SELF, 700, 5, 0x200_0000);
    APIphysic::PrimSetIntAttribBits(e, SELF, 700, 5, 0x1000);
    APIgameobj::RegisterNotify(e, SELF, 0, 0x16, 0x18);
    LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 2, NULL, 0);
    Ok(())
}

/// 0x0001b0c8
pub fn mummylevelthreeaDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    mummyepisodetwoDestructor(e)
}

// ---- MummyLevelNine.c (mummylevelnine) ----

/// 0x0001a958 monster type 5; start at 0x2a000006
pub fn MummyLevelNine_Func_Initialisation(e: &mut Engine) -> LResult<()> {
    e.w32(L(e) + 0x44, 5);
    let DStack_30 = e.alloca(0x10);
    APIgameobj::GetPosition(e, 0x2a00_0006, ALL, DStack_30);
    APIgameobj::SetTranslation(e, SELF, DStack_30);
    Ok(())
}

/// 0x0001a9d8 0xd6/0x68 (captured): msg 0x11d to 0x2a00000d (then on to the base); 0xc1: here is the
/// new start position, handled by the base first, then speed profiles, mask 0x200611, prim bit
/// 0x1000000, physics flags 6 on / 1 off, and NOT passed on again; 0x1d2: path set from `param_3`.
/// NOTE (PS2 behaviour kept): L+0xf8 here is written through pLocalDataMummy_Base, the same block as
/// Monster_Base's start position.
pub fn MummyLevelNine_MessageFunction(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    if param_1 == 0xd6 {
        if param_2 == 0x68 {
            APIgameobj::SendMessageTo(e, 0x2a00_000d, NULL, 0x11d, 0, NULL);
        }
    } else if param_1 == 0xc1 {
        let p = L(e) + 0xf8;
        APIgameobj::GetPosition(e, SELF, ALL, p);
        Mummy_BaseMessageFonction(e, 0xc1, param_2, param_3)?;
        SetSneakSpeedParams(e)?;
        SetAttackSpeedParams(e)?;
        SetRunFromTomeSpeedParams(e)?;
        APIgameobj::SetGroupBitMask(e, SELF, 0x200611);
        APIphysic::PrimSetIntAttribBits(e, SELF, 700, 5, 0x100_0000);
        APIphysic::SetFlags(e, SELF, 6);
        APIphysic::ResetFlags(e, SELF, 1);
        return Ok(0);
    } else if param_1 == 0x1d2 {
        let l = L(e);
        LIBApiPathFinding::SetPath(e, param_3 as i32, (l + 0x310) as i32);
    }
    Mummy_BaseMessageFonction(e, param_1, param_2, param_3)
}

/// 0x0001ac10 gb2 +4 = 0 after the base init
pub fn mummylevelnineConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    mummyepisodetwoConstructor(e, api, instanceCount)?;
    APIcons::SetMessageFunction(e, MummyLevelNine_MessageFunction);
    MummyLevelNine_Func_Initialisation(e)?;
    Monster_Base_Func_Initialise(e)?;
    let gb2 = e.r32(GDMu(e) + 4);
    e.w32(gb2 + 4, 0);
    mummy_base_Func_Initialisation(e)?;
    Ok(())
}

/// 0x0001acc8
pub fn mummylevelnineDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    mummyepisodetwoDestructor(e)
}

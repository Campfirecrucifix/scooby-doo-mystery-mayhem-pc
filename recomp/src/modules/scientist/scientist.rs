//! ScientistEpisodeSix.c (2 functions) + Scientist.c (c66ebe98: 15 functions; 955aa35f: 3). Addresses
//! are module-relative in scientist__c66ebe98.
//!
//! EP6L02 puzzle (c66ebe98 only; the object ids are literals because only this build has the code):
//! scientist 0x2a0000ff and 0x2a000100 patrol behind force fields (L+0x228 the field controller, told
//! 0x1ee with our id; it answers 0x1ee 0/1 -> L+0x22c), switching waypoint sets L+0x234 / 0x238 /
//! 0x23c by which side of the field (floor bits L+0x230) they and the hero are on. 0x2a000100 can be
//! captured into the cage (game buffer 2 +8 bit 0x1000000, position of 0x2a000112). 0x2a000101 gives
//! the key card in a cut-scene (bit 0x10000; the card 0x2a0000c6 appears at 0x2a0000c3) and then fades
//! out the level when the hero comes near with game flag 0x40.
use super::builds::*;
use super::*;
use crate::engine::*;
use crate::logic::LResult;

const ID_FF: u32 = 0x2a00_00ff;
const ID_100: u32 = 0x2a00_0100;
const ID_101: u32 = 0x2a00_0101;
const ID_102: u32 = 0x2a00_0102;
/// the HUD / tome-meter object told 0xeb (k(ID_2a0000ce) in Monster_Base)
const HUD_CE: u32 = 0x2a00_00ce;
const KEY_CARD: u32 = 0x2a00_00c6;

/// game buffer 2 +8 (the level's puzzle bits)
fn gb2_bits(e: &Engine) -> Ptr {
    e.r32(GDS(e) + 4) + 8
}
/// game buffer 0 +0x14 (game flags)
fn gb0_flags(e: &Engine) -> Ptr {
    e.r32(GDS(e)) + 0x14
}

// ---- ScientistEpisodeSix.c ----

/// 0x00016e30
pub fn scientistepisodesixConstructor(e: &mut Engine, api: Ptr, param_2: i32) -> LResult<()> {
    scientist_baseConstructor(e, api, param_2)
}

/// 0x00016e80
pub fn scientistepisodesixDestructor(e: &mut Engine) -> LResult<()> {
    scientist_baseDestructor(e)
}

// ---- Scientist.c ----

/// 0x00016ed8 external 0x14 (c66ebe98, replaces scientist_baseCB_CUT_SCENE): only for our cut-scenes.
/// End: 0x2a000101 has given the key card (bit 0x10000, Set_GaveKeyCard, 0x2a000027 told 0x55, mask
/// 0x800001, HUD 0xeb 0, game flag 0x100 off); the others come back (0x165). Start: the base handler,
/// then us and the heroes 0x2a000042 / 0x2a000041 visible.
pub fn scientist_localCB_CUT_SCENE(e: &mut Engine, param_1: u32) -> LResult<u64> {
    let cs = e.r32(param_1);
    if APIgameobj::IsInCutScene(e, SELF, cs) != 0 {
        if e.r32(param_1 + 4) == 0 {
            let iVar1 = APIgameobj::GetID(e, SELF, NULL);
            if iVar1 == ID_101 {
                let a = gb2_bits(e);
                or32(e, a, 0x10000);
                scientistFUNC_Set_GaveKeyCard(e)?;
                APIgameobj::SendMessageTo(e, 0x2a00_0027, NULL, 0x55, 0, NULL);
                APIgameobj::SetGroupBitMask(e, SELF, 0x800001);
                APIgameobj::SendMessageTo(e, HUD_CE, NULL, 0xeb, 0, NULL);
                let a = gb0_flags(e);
                and32(e, a, 0xffff_feff);
            } else {
                scientistFUNC_MessageHandler(e, 0x165, 0, NULL)?;
            }
        } else {
            scientist_baseCB_CUT_SCENE(e, param_1)?;
            APIgameobj::SetGameObjectVisible(e, SELF, 1);
            APIgameobj::SetGameObjectVisible(e, 0x2a00_0042, 1);
            APIgameobj::SetGameObjectVisible(e, 0x2a00_0041, 1);
        }
    }
    Ok(0)
}

/// 0x00017108 (c66ebe98) / 0x00016f08 (955aa35f: just the base handler).
/// c66ebe98: 0x165 come back (visible, physics, Stand, mask 0x800401, HUD 0xeb 1, state 0xf); 0x4c
/// ignored by 0x2a000100; 0xc1 / 0xc9 reset the puzzle unless solved (L+0x1c8); 0xd4 marks it solved
/// from the save bits (and is never passed on); 0x1ee force field answer; 0x166 vanish asleep (state
/// 5); 0x19f go to the hero; 0x215 game flag 0x100 + fade; 0x2711 (0x2a000101) 0x2a000027 told 0x1de.
/// Everything else (and the above, except the early returns) goes to scientist_baseMessageFonction,
/// whose result is dropped.
pub fn scientistFUNC_MessageHandler(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    if g(e, G_scientistFUNC_MessageHandler) == 0 {
        match param_1 {
            0x165 => {
                APIgameobj::SetGameObjectVisible(e, SELF, 1);
                APIphysic::ResetFlags(e, SELF, 1);
                set_state(e, addr::scientist_baseST_Stand_In);
                APIgameobj::SetGroupBitMask(e, SELF, 0x800401);
                APIgameobj::SendMessageTo(e, HUD_CE, NULL, 0xeb, 1, NULL);
                APIgameobj::SetNextState(e, SELF, 0xf);
            }
            0x4c => {
                if APIgameobj::GetID(e, SELF, NULL) == ID_100 {
                    return Ok(0);
                }
            }
            0xd4 => {
                let iVar1 = APIgameobj::GetID(e, SELF, NULL);
                let bit = match iVar1 {
                    ID_100 => 0x100_0000,
                    ID_101 => 0x10000,
                    _ => return Ok(0),
                };
                if e.r32(gb2_bits(e)) & bit != 0 {
                    e.w32(L(e) + 0x1c8, 1);
                }
                return Ok(0);
            }
            0xc1 | 0xc9 => {
                let uVar2 = APIgameobj::GetID(e, SELF, NULL);
                if uVar2 == ID_100 {
                    if e.r32(L(e) + 0x1c8) == 0 {
                        let a = gb2_bits(e);
                        and32(e, a, 0xfeff_ffff);
                        scientistFUNC_Reset_ScientistCaptured(e)?;
                    }
                } else if uVar2 == ID_FF {
                    let p = L(e) + 0xf8;
                    APIgameobj::SetTranslation(e, SELF, p);
                } else if uVar2 == ID_101 {
                    if e.r32(L(e) + 0x1c8) != 0 {
                        return Ok(0);
                    }
                    let a = gb2_bits(e);
                    and32(e, a, 0xfffe_ffff);
                    scientistFUNC_Reset_GaveKeyCard(e, 0)?;
                }
            }
            0x1ee => {
                e.w32(L(e) + 0x22c, (param_2 == 0) as u32);
            }
            0x166 => {
                APIgameobj::SetGameObjectVisible(e, SELF, 0);
                APIphysic::SetFlags(e, SELF, 1);
                e.w32(L(e) + 0x164, 0);
                go(e, addr::scientist_baseST_Sleep_In)?;
                APIgameobj::SetGroupBitMask(e, SELF, 0);
                APIgameobj::SendMessageTo(e, HUD_CE, NULL, 0xeb, 1, NULL);
                APIgameobj::SetNextState(e, SELF, 5);
            }
            0x19f => {
                e.w32(L(e) + 0x164, 0);
                go(e, addr::scientist_localST_GoToHero_In)?;
            }
            0x215 => {
                let a = gb0_flags(e);
                or32(e, a, 0x100);
                scientistFUNC_DoFade(e)?;
            }
            0x2711 => {
                if APIgameobj::GetID(e, SELF, NULL) == ID_101 {
                    let uStack_34 = e.alloca(0x10);
                    e.w32(uStack_34, 0x8000_0004);
                    APIgameobj::SendMessageTo(e, 0x2a00_0027, NULL, 0x1de, 0, uStack_34);
                }
            }
            _ => {}
        }
    }
    scientist_baseMessageFonction(e, param_1, param_2, param_3)?;
    Ok(0)
}

/// 0x00017758 fade {1, 0.5, 0, 0, 0}
pub fn scientistFUNC_DoFade(e: &mut Engine) -> LResult<()> {
    let b = e.alloca(0x20);
    for (o, v) in [(0x10, 0), (4, 0x3f00_0000), (8, 0), (0xc, 0), (0, 1)] {
        e.w32(b + o, v);
    }
    LIBApiCFunctions::DoFade(e, b);
    Ok(())
}

/// 0x000177b8 ask the field controller (L+0x228) about the field; pick the waypoint set: field off
/// (L+0x22c) -> L+0x234 and detection on; field on -> by our floor side (bits L+0x230): L+0x238 (on
/// the bits; detect when the hero is too) or L+0x23c (off; detect when the hero is too). A new set
/// re-counts its waypoints and turns a path walk into HearingNoise.
pub fn scientistFUNC_Handle_ForceFieldGamePlay(e: &mut Engine) -> LResult<()> {
    let mut iStack_80 = e.r32(L(e) + 0x1a4);
    let (to, id) = (e.r32(L(e) + 0x228), e.r32(L(e) + 0x174));
    APIgameobj::SendMessageTo(e, to, NULL, 0x1ee, id, NULL);
    let off = e.r32(L(e) + 0x22c);
    if off == 0 || e.r32(L(e) + 0x1a4) == e.r32(L(e) + 0x234) {
        if off == 0 {
            let a70 = e.alloca(0x20);
            let a50 = e.alloca(0x20);
            APIphysic::GetFloorAttribs(e, SELF, a70);
            let h = hero(e);
            APIphysic::GetFloorAttribs(e, h, a50);
            let m = e.r32(L(e) + 0x230);
            let (us, him) = (e.r32(a70 + 8) & m, e.r32(a50 + 8) & m);
            if us == 0 {
                iStack_80 = e.r32(L(e) + 0x23c);
                e.w32(L(e) + 0x1d0, (him == 0) as u32);
            } else {
                iStack_80 = e.r32(L(e) + 0x238);
                e.w32(L(e) + 0x1d0, (him != 0) as u32);
            }
        }
    } else {
        iStack_80 = e.r32(L(e) + 0x234);
        e.w32(L(e) + 0x1d0, 1);
    }
    if iStack_80 != e.r32(L(e) + 0x1a4) {
        e.w32(L(e) + 0x1a4, iStack_80);
        let uVar1 = APIextra::WaypointGetNb(e, iStack_80);
        e.w32(L(e) + 0x19c, uVar1);
        let s = e.r32(L(e) + 0x160);
        if s == addr::scientist_baseST_WalkOnPath_In || s == addr::scientist_baseST_WalkOnPath {
            go(e, addr::scientist_localST_HearingNoise_In)?;
        }
    }
    Ok(())
}

/// 0x00017a88 in the cage: group 6, at 0x2a000112
pub fn scientistFUNC_Set_ScientistCaptured(e: &mut Engine) -> LResult<()> {
    let d = e.alloca(0x10);
    APIgameobj::GetPosition(e, 0x2a00_0112, ALL, d);
    APIgameobj::SetGroupID(e, SELF, 6);
    APIgameobj::SetTranslation(e, SELF, d);
    Ok(())
}

/// 0x00017b10 out of the cage: group 0, back at the start, force field check, Stand next
pub fn scientistFUNC_Reset_ScientistCaptured(e: &mut Engine) -> LResult<()> {
    APIgameobj::SetGroupID(e, SELF, 0);
    let p = L(e) + 0xf8;
    APIgameobj::SetTranslation(e, SELF, p);
    scientistFUNC_Handle_ForceFieldGamePlay(e)?;
    set_state(e, addr::scientist_baseST_Stand_In);
    Ok(())
}

/// 0x00017ba8 gone after giving the card: group 6, 0x2a000101 hidden, mask 0, HUD 0xeb 1, physics off,
/// the card (if it exists) reset (0xc1), put at 0x2a0000c3 and shown (0xd5); state 5
pub fn scientistFUNC_Set_GaveKeyCard(e: &mut Engine) -> LResult<()> {
    APIgameobj::SetGroupID(e, SELF, 6);
    APIgameobj::SetGameObjectVisible(e, ID_101, 0);
    APIgameobj::SetGroupBitMask(e, SELF, 0);
    APIgameobj::SendMessageTo(e, HUD_CE, NULL, 0xeb, 1, NULL);
    APIphysic::SetFlags(e, SELF, 1);
    if APIgameobj::GetID(e, KEY_CARD, NULL) != 0x7fff_fffe {
        let d = e.alloca(0x10);
        APIgameobj::GetPosition(e, 0x2a00_00c3, ALL, d);
        APIgameobj::SendMessageTo(e, KEY_CARD, NULL, 0xc1, 0, NULL);
        APIgameobj::SetTranslation(e, KEY_CARD, d);
        APIgameobj::SendMessageTo(e, KEY_CARD, NULL, 0xd5, 0, NULL);
    }
    APIgameobj::SetNextState(e, SELF, 5);
    Ok(())
}

/// 0x00017d60 card not given yet: 0x2a000046 told 0xdf 4, the card parked at (100, 0.25, 100), us back
/// (physics, group 0, visible, mask 0x800401 twice, HUD 0xeb 1), 0x2a000024 told 0x165, Stand next;
/// `param_1 == 0`: clipping + state 0xf
pub fn scientistFUNC_Reset_GaveKeyCard(e: &mut Engine, param_1: i32) -> LResult<()> {
    let d = e.alloca(0x10);
    e.wf(d, 100.0);
    e.wf(d + 4, 0.25);
    e.wf(d + 8, 100.0);
    APIgameobj::SendMessageTo(e, 0x2a00_0046, NULL, 0xdf, 4, NULL);
    APIgameobj::SendMessageTo(e, KEY_CARD, NULL, 0xc1, 0, NULL);
    APIgameobj::SetTranslation(e, KEY_CARD, d);
    APIgameobj::SendMessageTo(e, KEY_CARD, NULL, 0xd5, 0, NULL);
    APIphysic::ResetFlags(e, SELF, 1);
    APIgameobj::SetGroupID(e, SELF, 0);
    APIgameobj::SetGameObjectVisible(e, ID_101, 1);
    APIgameobj::SetGroupBitMask(e, SELF, 0x800401);
    APIgameobj::SendMessageTo(e, HUD_CE, NULL, 0xeb, 1, NULL);
    APIgameobj::SetGroupBitMask(e, SELF, 0x800401);
    APIgameobj::SendMessageTo(e, 0x2a00_0024, NULL, 0x165, 0, NULL);
    set_state(e, addr::scientist_baseST_Stand_In);
    if param_1 == 0 {
        Scientist_BaseFUNC_SetClippingScientist(e)?;
        APIgameobj::SetNextState(e, SELF, 0xf);
    }
    Ok(())
}

/// 0x00017fa0 external 0xf (c66ebe98): 0x2a000101 fades the level out when the hero is within 3 m
/// with game flag 0x40; 0x2a0000ff / 0x2a000100 check the force field; then the base entry point
pub fn scientist_localST_EntryPoint(e: &mut Engine, param_1: u32) -> LResult<u64> {
    let id = e.r32(L(e) + 0x174);
    if id == ID_101 {
        if e.r32(gb0_flags(e)) & 0x40 != 0 {
            let h = hero(e);
            if LIBApiCFunctions::CheckRangeObject(e, 3.0, SELF, h, 1) != 0 {
                let a = gb0_flags(e);
                or32(e, a, 0x100);
                scientistFUNC_DoFade(e)?;
            }
        }
    } else if id == ID_FF || id == ID_100 {
        scientistFUNC_Handle_ForceFieldGamePlay(e)?;
    }
    scientist_baseST_ENTRY_POINT(e, param_1)
}

/// 0x000180f8 run (anim 5 loop) to the hero along the path graph
pub fn scientist_localST_GoToHero_In(e: &mut Engine) -> LResult<()> {
    let d50 = e.alloca(0x10);
    let d40 = e.alloca(0x10);
    let h = hero(e);
    APIgameobj::GetPosition(e, h, ALL, d50);
    APIgameobj::GetPosition(e, SELF, ALL, d40);
    APIgameobj::SetContextFlag(e, SELF, 0x67);
    LIBAnimMngr::PlayAnim(e, 0.0, 5, 2, 0);
    scientist_baseFUNC_SetSpeed(e, 0x6f)?;
    let l = L(e);
    LIBApiPathFinding::FindPath(e, d40, d50, l + 0x218, (l + 0x248) as i32);
    Scientist_BaseFUNC_TrackPos(e, 0x40a0_0000, l + 0x218)?;
    go(e, addr::scientist_localST_GoToHero)
}

/// 0x00018260 path end or within 2 m: 0x2a000152 told 0x19f, spotting forced (L+0x1cc), Spotting next
pub fn scientist_localST_GoToHero(e: &mut Engine) -> LResult<()> {
    let iVar2 = Scientist_BaseFUNC_UpdatePathFinding(e)?;
    let near = iVar2 == 2 || {
        let h = hero(e);
        LIBApiCFunctions::CheckRangeObject(e, 2.0, SELF, h, 0) != 0
    };
    if near {
        APIgameobj::SendMessageTo(e, 0x2a00_0152, NULL, 0x19f, 0, NULL);
        e.w32(L(e) + 0x1cc, 1);
        set_state(e, addr::scientist_baseST_Spotting_In);
    }
    Ok(())
}

/// 0x00018340 anim 0x13 (loop, end notified), no conditions
pub fn scientist_localST_HearingNoise_In(e: &mut Engine) -> LResult<()> {
    e.w32(L(e) + 0x178, 0);
    e.w32(L(e) + 0x164, 0);
    APIgameobj::SetContextFlag(e, SELF, 0x60);
    LIBAnimMngr::PlayAnim(e, 0.3, 0x13, 2, 0x12);
    set_state(e, addr::scientist_localST_HearingNoise);
    Ok(())
}

/// 0x000183e0 anim ended -> WalkOnPath
pub fn scientist_localST_HearingNoise(e: &mut Engine) -> LResult<()> {
    if e.r32(L(e) + 0x178) & 0x800 != 0 {
        set_state(e, addr::scientist_baseST_WalkOnPath_In);
        let a = L(e) + 0x178;
        and32(e, a, 0xffff_f7ff);
    }
    Ok(())
}

/// 0x00018450 (c66ebe98) / 0x00016f70 (955aa35f): base constructors, monster type 0xe, message handler;
/// c66ebe98: entry point / cut-scene externals, cut-scene notify for 0x2a000101 / 0x2a000102, the
/// force-field data of 0x2a0000ff / 0x2a000100 and the saved puzzle state; then the TmpMonsterMap
/// record, initial state 0xe.
pub fn scientistConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    scientistepisodesixConstructor(e, api, instanceCount)?;
    e.w32(LM(e) + 0x44, 0xe);
    APIcons::SetMessageFunction(e, scientistFUNC_MessageHandler);
    if g(e, G_scientistConstructor) == 0 {
        APIcons::SetExternalFunction(e, 0xf, scientist_localST_EntryPoint);
        APIcons::SetExternalFunction(e, 0x14, scientist_localCB_CUT_SCENE);
        let iVar1 = APIgameobj::GetID(e, SELF, NULL);
        if iVar1 == ID_101 || iVar1 == ID_102 {
            APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 0x16, 0x14);
        }
        let l = L(e);
        if iVar1 == ID_FF {
            for (o, v) in [(0x230, 0x800), (0x234, 0x9c), (0x238, 0x92), (0x23c, 0x9b), (0x228, 0x2a00_0126)] {
                e.w32(l + o, v);
            }
        } else if iVar1 == ID_100 {
            for (o, v) in [(0x230, 0x800), (0x234, 0xcf), (0x238, 0xdb), (0x23c, 0xda), (0x228, 0x2a00_015e), (0x240, 0x80_0020)] {
                e.w32(l + o, v);
            }
            APIphysic::PrimSetIntAttribBits(e, SELF, 0x3ed, 6, 0x20);
            if e.r32(gb2_bits(e)) & 0x100_0000 == 0 {
                e.w32(L(e) + 0x1c8, 0);
                scientistFUNC_Reset_ScientistCaptured(e)?;
            } else {
                e.w32(L(e) + 0x1c8, 1);
                scientistFUNC_Set_ScientistCaptured(e)?;
            }
        } else if iVar1 == ID_101 {
            if e.r32(gb2_bits(e)) & 0x10000 == 0 {
                e.w32(L(e) + 0x1c8, 0);
                scientistFUNC_Reset_GaveKeyCard(e, 1)?;
            } else {
                e.w32(L(e) + 0x1c8, 1);
                e.w32(L(e) + 0x1d4, 1);
                scientistFUNC_Set_GaveKeyCard(e)?;
            }
        }
    }
    Monster_Base_Func_Initialise(e)?;
    APIcons::SetInitialState(e, 0xe);
    Ok(())
}

/// 0x000187a8
pub fn scientistDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    scientistepisodesixDestructor(e)
}

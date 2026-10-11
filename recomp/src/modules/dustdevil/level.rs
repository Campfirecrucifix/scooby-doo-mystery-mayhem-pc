//! The per-level source files of each dust devil build (`DustDevilEpisodeThree.c` +
//! `DustDevilLevel<N>.c`): the module entry points, the level's message function and init. The port
//! has one entry pair ([`dustdevil_levelConstructor`] / [`dustdevil_levelDestructor`]) that dispatches
//! on the loaded build. Addresses are module-relative (.text at 0x10000) of the build named.
use super::*;
use crate::engine::*;
use crate::logic::LResult;

/// The module entry point (`<level>Constructor` of the loaded build).
pub fn dustdevil_levelConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    match build_key(e) {
        "dustdevilleveltwo" => dustdevilleveltwoConstructor(e, api, instanceCount),
        "dustdevillevelfour" => dustdevillevelfourConstructor(e, api, instanceCount),
        "dustdevillevelfive" => dustdevillevelfiveConstructor(e, api, instanceCount),
        "dustdevillevelfiveboss" => dustdevillevelfivebossConstructor(e, api, instanceCount),
        k => panic!("dustdevil: no level constructor for build {k}"),
    }
}

/// The module exit point (`<level>Destructor` of the loaded build).
pub fn dustdevil_levelDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    match build_key(e) {
        "dustdevilleveltwo" => dustdevilleveltwoDestructor(e, instanceCount),
        "dustdevillevelfour" => dustdevillevelfourDestructor(e, instanceCount),
        "dustdevillevelfive" => dustdevillevelfiveDestructor(e, instanceCount),
        "dustdevillevelfiveboss" => dustdevillevelfivebossDestructor(e, instanceCount),
        k => panic!("dustdevil: no level destructor for build {k}"),
    }
}

// ---- DustDevilEpisodeThree.c ----

/// (dustdevilleveltwo)
pub fn dustdevilepisodethreeConstructor(e: &mut Engine, api: Ptr, param_2: i32) -> LResult<()> {
    dustdevil_baseConstructor(e, api, param_2)?;
    Ok(())
}

/// (dustdevilleveltwo)
pub fn dustdevilepisodethreeDestructor(e: &mut Engine, param_1: i32) -> LResult<()> {
    dustdevil_baseDestructor(e, param_1)
}

/// `L+0x44 = 9` (monster type: a plain dust devil)
fn init_type_9(e: &mut Engine) {
    let l = L(e);
    e.w32(l + 0x44, 9);
}

/// near 0.1, far 40, fade 50, sector mask 0x6a7 (SetDustDevilLocalClipping / SetDustDevilBossClipping
/// and dustdevillevelfiveConstructor's inline copy)
const CLIP_L5: [u32; 10] = [0x3dcc_cccd, 0x4220_0000, 0x47, 0, 0x4248_0000, 0x6a7, 0, 0xbf80_0000, 0x6a7, 0x6a7];

// ---- DustDevilLevelTwo.c (dustdevilleveltwo, Ep3 L02) ----

/// 0x00017620
pub fn DustDevilLevelTwo_Func_Initialisation(e: &mut Engine) -> LResult<()> {
    init_type_9(e);
    Ok(())
}

pub fn DustDevilLevelTwo_MessageFunction(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    DustDevil_BaseMessageFonction(e, param_1, param_2, param_3)
}

/// cutscene notifications (4, 0x16) go to callback 0x1f
pub fn dustdevilleveltwoConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    dustdevilepisodethreeConstructor(e, api, instanceCount)?;
    APIgameobj::RegisterNotify(e, SELF, 4, 0x16, 0x1f);
    APIcons::SetMessageFunction(e, DustDevilLevelTwo_MessageFunction);
    DustDevilLevelTwo_Func_Initialisation(e)?;
    Monster_Base_Func_Initialise(e)?;
    DustDevil_Base_Func_Initialisation(e)?;
    DustDevilFUNC_Create_Physic(e)
}

pub fn dustdevilleveltwoDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    dustdevilepisodethreeDestructor(e, instanceCount)
}

// ---- DustDevilLevelFour.c (dustdevillevelfour, Ep3 L04) ----

pub fn DustDevilLevelFour_Func_Initialisation(e: &mut Engine) -> LResult<()> {
    init_type_9(e);
    Ok(())
}

pub fn DustDevilLevelFour_MessageFunction(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    DustDevil_BaseMessageFonction(e, param_1, param_2, param_3)
}

pub fn dustdevillevelfourConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    dustdevilepisodethreeConstructor(e, api, instanceCount)?;
    APIcons::SetMessageFunction(e, DustDevilLevelFour_MessageFunction);
    DustDevilLevelFour_Func_Initialisation(e)?;
    Monster_Base_Func_Initialise(e)?;
    DustDevil_Base_Func_Initialisation(e)?;
    DustDevilFUNC_Create_Physic(e)
}

pub fn dustdevillevelfourDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    dustdevilepisodethreeDestructor(e, instanceCount)
}

// ---- DustDevilLevelFive.c (dustdevillevelfive, Ep3 L05: the boss's split-offs) ----

/// remember the context flags (L+0x54), then hide, freeze and disable until the boss sends 0xd3
fn l5_hide(e: &mut Engine) {
    let uVar1 = APIgameobj::GetContextFlag(e, SELF);
    e.w32(L(e) + 0x54, uVar1);
    APIgameobj::SetContextFlag(e, SELF, 0);
}

/// type 9, start position L+0xf8; hidden, asleep, frozen and disabled until 0xd3
pub fn DustDevilLevelFive_Func_Initialisation(e: &mut Engine) -> LResult<()> {
    let p = L(e) + 0xf8;
    APIgameobj::GetPosition(e, SELF, ALL, p);
    init_type_9(e);
    l5_hide(e);
    APIgameobj::SetGameObjectVisible(e, SELF, 0);
    APIgameobj::Sleep(e, SELF, -1.0);
    APIphysic::SetFlags(e, SELF, 1);
    let p = L(e) + 0xf8;
    APIgameobj::SetTranslation(e, SELF, p);
    APIgameobj::EnableObject(e, SELF, 0);
    Ok(())
}

/// 0xd3 (released by the boss): enable, clipping, glow, physics, context, show, wake, roam (0x10);
/// 0xc9 restart: reset health / path state, clear my kill bit in game buffer 2 +4, groups, hide again
/// at the start (handled: no base call); 0xd6/0x68 captured: always clipped in. Then
/// DustDevil_BaseMessageFonction.
pub fn DustDevilLevelFive_MessageFunction(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    let iVar1 = L(e);
    if param_1 == 0xd3 {
        APIgameobj::EnableObject(e, SELF, 1);
        SetDustDevilLocalClipping(e)?;
        CreateGlow(e, 0.25)?;
        APIphysic::ResetFlags(e, SELF, 1);
        let c = e.r32(L(e) + 0x54);
        APIgameobj::SetContextFlag(e, SELF, c);
        APIgameobj::SetGameObjectVisible(e, SELF, 1);
        APIgameobj::Sleep(e, SELF, 0.0);
        APIgameobj::SetNextState(e, SELF, 0x10);
    } else if param_1 < 0xd4 {
        if param_1 == 0xc9 {
            let v = e.r32(L(e) + 0xb0);
            e.w32(L(e) + 0x5c, v);
            e.w32(iVar1 + 0x174, 1);
            e.w32(iVar1 + 0x170, 1);
            APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 3, 0x1b);
            let gb2 = e.r32(GDD(e) + 4);
            let v = e.r32(gb2 + 4) & !e.r32(iVar1 + 0x40);
            e.w32(gb2 + 4, v);
            APIgameobj::SetContextFlag(e, SELF, 0x66);
            SetWalkSpeedParams(e)?;
            APIgameobj::SetGroupBitMask(e, SELF, 0x200621);
            APIgameobj::SetGroupID(e, SELF, 1);
            l5_hide(e);
            APIphysic::SetFlags(e, SELF, 1);
            APIgameobj::SetGameObjectVisible(e, SELF, 0);
            APIgameobj::Sleep(e, SELF, -1.0);
            APIgameobj::SetTranslation(e, SELF, iVar1 + 0xf8);
            APIgameobj::EnableObject(e, SELF, 0);
            return Ok(0);
        }
    } else if param_1 == 0xd6 && param_2 == 0x68 {
        LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 2, NULL, 0);
    }
    DustDevil_BaseMessageFonction(e, param_1, param_2, param_3)
}

pub fn SetDustDevilLocalClipping(e: &mut Engine) -> LResult<()> {
    set_clipping(e, CLIP_L5);
    Ok(())
}

pub fn dustdevillevelfiveConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    dustdevilepisodethreeConstructor(e, api, instanceCount)?;
    APIcons::SetMessageFunction(e, DustDevilLevelFive_MessageFunction);
    set_clipping(e, CLIP_L5);
    DustDevilLevelFive_Func_Initialisation(e)?;
    Monster_Base_Func_Initialise(e)?;
    DustDevil_Base_Func_Initialisation(e)?;
    SetDustDevilLocalClipping(e)?;
    DustDevilFUNC_Create_Physic(e)
}

pub fn dustdevillevelfiveDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    dustdevilepisodethreeDestructor(e, instanceCount)
}

// ---- DustDevilLevelFiveBoss.c (dustdevillevelfiveboss, Ep3 L05 boss) ----
// The boss (type 10) homes on the hero. Its stage timer (L+0x180, 5 s, callback 0x1d) releases two
// small dust devils per stage (0x2a00009e/9f, a0/a1, a2/a3, oriented on 0x2a00009d) and tells the
// arena 0x2a0000aa (0xd3); a stage ends when both of its small ones are captured (game buffer 2 +4
// bits 0x1000..0x20000). After the third stage it becomes catchable: roams (0x10) and reports half
// health (0x3a to the HUD) once. Captured: the camera (0x2a000006) gets 0x54, the HUD 0x1e, and the
// next 0x4f loads the episode-end movie through the LOADER level.

/// type 10, stage timer, groups 0x100001; returns SetGroupBitMask's result
pub fn DustDevilLevelFiveBoss_Func_Initialisation(e: &mut Engine) -> LResult<u32> {
    e.w32(L(e) + 0x44, 10);
    let t = APIextra::TimerCreate(e);
    e.w32(L(e) + 0x180, t);
    stage_timer(e, 5.0);
    Ok(APIgameobj::SetGroupBitMask(e, SELF, 0x100001))
}

/// `TimerReset(L+0x180); TimerSetFrequency(f); TimerRegisterNotify(SELF, L+0x180, 0x1d)`
fn stage_timer(e: &mut Engine, f: f32) {
    let t = e.r32(L(e) + 0x180);
    APIextra::TimerReset(e, t);
    APIextra::TimerSetFrequency(e, t, f);
    APIextra::TimerRegisterNotify(e, SELF, t, 0x1d);
}

/// state 0 (replaces Monster_BaseST_REINITIALISE): big glow, full health, stages reset, home on the
/// hero at 8 m/s, back at the start, stage timer 5 s, boss idle (0x1a)
pub fn DustDevilST_REINITIALISE(e: &mut Engine, _: u32) -> LResult<u64> {
    let iVar1 = L(e);
    CreateGlow(e, 0.89)?;
    let v = e.r32(iVar1 + 0xb0);
    e.w32(iVar1 + 0x5c, v);
    e.w32(iVar1 + 0x174, 1);
    e.w32(iVar1 + 0x170, 1);
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 3, 0x1b);
    for m in [0xffff_fffe, 0xffff_fffd, 0xffff_fffb, 0xffff_fff7] {
        let v = e.r32(iVar1 + 0x184) & m;
        e.w32(iVar1 + 0x184, v);
    }
    let dm = e.r32(iVar1 + 4);
    let b = e.alloca(0x80);
    APIphysic::DMGetParams(e, dm, b);
    let h = e.r32(e.r32(GDM(e)) + 8);
    for (o, v) in [(0, h), (8, 0), (0xc, 0), (0x10, 0), (4, 0x4100_0000), (0x14, 1)] {
        e.w32(b + o, v);
    }
    APIphysic::DMSetParams(e, dm, b);
    LIBAnimMngr::PlayAnim(e, 0.2, 3, 2, 0);
    APIgameobj::SetContextFlag(e, SELF, 0x62);
    APIgameobj::SetTranslation(e, SELF, iVar1 + 0xf8);
    stage_timer(e, 5.0);
    SetDustDevilBossClipping(e)?;
    APIgameobj::SetNextState(e, SELF, 0x1a);
    Ok(0)
}

/// 0xc9 restart (0x3b to the HUD, state 0); 0x4f with the boss captured: load the episode-end movie
/// (stleveltransitionparams {2, "\MOVIES\EP03\EP03A", 0...}, game buffer 2 +0x28 |= 0x2000);
/// 0xd6/0x68 captured: camera 0x54 {[0] 0x62, [0x10] fade 1}, HUD 0x1e; 0x14e (a small one captured): next stage
/// check in 3 s, 0x3a to the HUD. Then DustDevil_BaseMessageFonction.
pub fn DustDevilLevelFiveBoss_MessageFunction(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    if param_1 == 0xc9 {
        APIgameobj::SendMessageTo(e, 0x2a10_0006, NULL, 0x3b, 0, NULL);
        APIgameobj::SetNextState(e, SELF, 0);
    } else if param_1 < 0xca {
        if param_1 == 0x4f && e.r32(G(e, bLoading)) != 0 {
            // memcpy(asStack_810, &DAT_00010fe0, 0x7dc): only words +0 / +4 are non-zero
            let asStack_810 = e.alloca(0x7dc);
            for o in (0..0x7dc).step_by(4) {
                e.w32(asStack_810 + o, 0);
            }
            e.w32(asStack_810, 2);
            let movie = e.cstr("\\MOVIES\\EP03\\EP03A");
            e.w32(asStack_810 + 4, movie);
            let gb2 = e.r32(GDD(e) + 4);
            let v = e.r32(gb2 + 0x28) | 0x2000;
            e.w32(gb2 + 0x28, v);
            let l = e.cstr("LOADER");
            APIgame::LoadLevel(e, l, l, asStack_810);
        }
    } else if param_1 == 0xd6 {
        if param_2 == 0x68 {
            let a = G(e, bLoading);
            e.w32(a, 1);
            let c = G(e, moCameraParams_198);
            // WP-R7: `static stCameraParams moCameraParams` is one 0x4c-byte zero-initialised static
            // (.o symbol moCameraParams.198 at .text+0x128, up to pAnim at +0x174); the C writes
            // [0] = 0x62 and DAT_00010164/68/6c = [0xf] = 0, [0x10] = 1 (fade: FadeInOut(1.0, 0) ->
            // camera timer +0xe8 == 1 -> broadcast 0x4f -> LoadLevel above), [0x11] flags = 0.
            // (R6 read the DAT_ words as [1..3] of a block at 0x10160 and filled [0x10] = 0: no fade.)
            for o in (0..0x4c).step_by(4) {
                e.w32(c + o, 0);
            }
            e.w32(c, 0x62);
            e.w32(c + 0x3c, 0);
            e.w32(c + 0x40, 1);
            e.w32(c + 0x44, 0);
            APIgameobj::SendMessageTo(e, 0x2a00_0006, NULL, 0x54, 0, c);
            APIgameobj::SendMessageTo(e, 0x2a10_0006, NULL, 0x1e, 0, NULL);
        }
    } else if param_1 == 0x14e {
        stage_timer(e, 3.0);
        APIgameobj::SendMessageTo(e, 0x2a10_0006, NULL, 0x3a, 0, NULL);
    }
    DustDevil_BaseMessageFonction(e, param_1, param_2, param_3)
}

/// release a pair of small dust devils at `pos`, facing the way of 0x2a00009d. `freeze` are the ids
/// SetFlags(1) is called on (the third stage freezes a0/a1 again, not a2/a3, as in the original).
fn release_pair(e: &mut Engine, freeze: [u32; 2], pair: [u32; 2], pos: Ptr) {
    for id in freeze {
        APIphysic::SetFlags(e, id, 1);
    }
    for id in pair {
        APIgameobj::SetTranslation(e, id, pos);
        APIgameobj::TakeOrientation(e, id, 0x2a00_009d, ALL);
    }
    for id in pair {
        APIgameobj::SendMessageTo(e, id, NULL, 0xd3, 0, NULL);
    }
}

/// callback 0x1d: the stage timer (L+0x180) runs the stages (see above); the half-health timer
/// (L+0xf00) sends 0x3a to the HUD once health is at most half. Returns 0 when that timer fired
/// without reporting, else 1.
pub fn DustDevilCB_NOTIFY_TIMER_LOCAL(e: &mut Engine, param_1: u32) -> LResult<u64> {
    if param_1 == e.r32(L(e) + 0x180) {
        let DStack_60 = e.alloca(0xc);
        APIgameobj::GetPosition(e, SELF, ALL, DStack_60);
        let bits = |e: &Engine| e.r32(e.r32(GDD(e) + 4) + 4);
        let st = |e: &Engine| e.r32(L(e) + 0x184);
        if bits(e) & 0x3f000 == 0 && st(e) & 1 == 0 {
            or32(e, L(e) + 0x184, 1);
            APIgameobj::SendMessageTo(e, 0x2a00_00aa, NULL, 0xd3, 0, NULL);
            release_pair(e, [0x2a00_009e, 0x2a00_009f], [0x2a00_009e, 0x2a00_009f], DStack_60);
        }
        if bits(e) & 0x1000 != 0 && bits(e) & 0x2000 != 0 && st(e) & 1 != 0 && st(e) & 2 == 0 {
            or32(e, L(e) + 0x184, 2);
            APIgameobj::SendMessageTo(e, 0x2a00_00aa, NULL, 0xd3, 0, NULL);
            DestroyGlow(e)?;
            APInewSound::Play(e, 0x2a00_0009, 0x8000_0006, 0);
            CreateGlow(e, 0.58)?;
            LIBAnimMngr::PlayAnim(e, 0.2, 4, 2, 0);
            release_pair(e, [0x2a00_00a0, 0x2a00_00a1], [0x2a00_00a0, 0x2a00_00a1], DStack_60);
        }
        if bits(e) & 0x4000 != 0 && bits(e) & 0x8000 != 0 && st(e) & 2 != 0 && st(e) & 4 == 0 {
            or32(e, L(e) + 0x184, 4);
            APIgameobj::SendMessageTo(e, 0x2a00_00aa, NULL, 0xd3, 0, NULL);
            DestroyGlow(e)?;
            APInewSound::Play(e, 0x2a00_0009, 0x8000_0007, 0);
            CreateGlow(e, 0.25)?;
            LIBAnimMngr::PlayAnim(e, 0.2, 2, 2, 0);
            release_pair(e, [0x2a00_00a0, 0x2a00_00a1], [0x2a00_00a2, 0x2a00_00a3], DStack_60);
        }
        if bits(e) & 0x10000 != 0 && bits(e) & 0x20000 != 0 && st(e) & 4 != 0 && st(e) & 8 == 0 {
            or32(e, L(e) + 0x184, 8);
            APIgameobj::SendMessageTo(e, 0x2a00_00aa, NULL, 0xdf, 0, NULL);
            APIgameobj::SendMessageTo(e, 0x2a00_00aa, NULL, 0xd3, 0, NULL);
            APInewSound::Play(e, 0x2a00_0009, 0x8000_0008, 0);
            let dm = e.r32(L(e) + 4);
            let b = e.alloca(0x80);
            APIphysic::DMGetParams(e, dm, b);
            for (o, v) in [(0, 0), (8, 0), (0xc, 0), (0x10, 0), (4, 0x4100_0000), (0x14, 5)] {
                e.w32(b + o, v);
            }
            let dm = e.r32(L(e) + 4);
            APIphysic::DMSetParams(e, dm, b);
            let t = e.r32(L(e) + 0xf00);
            APIextra::TimerReset(e, t);
            APIextra::TimerSetFrequency(e, t, 1.0);
            APIextra::TimerRegisterNotify(e, SELF, t, 0x1d);
            APIgameobj::SetContextFlag(e, SELF, 0x66);
            let a = G(e, bHalfHealth);
            e.w32(a, 0);
            SetWalkSpeedParams(e)?;
            APIgameobj::SetNextState(e, SELF, 0x10);
        }
    }
    if param_1 == e.r32(L(e) + 0xf00) {
        if e.rf(L(e) + 0xb0) / 2.0 < e.rf(L(e) + 0x5c) || e.r32(G(e, bHalfHealth)) != 0 {
            Ok(0)
        } else {
            let a = G(e, bHalfHealth);
            e.w32(a, 1);
            APIgameobj::SendMessageTo(e, 0x2a10_0006, NULL, 0x3a, 0, NULL);
            Ok(1)
        }
    } else {
        Ok(1)
    }
}

pub fn SetDustDevilBossClipping(e: &mut Engine) -> LResult<()> {
    set_clipping(e, CLIP_L5);
    Ok(())
}

/// stage callback 0x1d and state 0 (first instance), half-health timer L+0xf00; clears the six
/// small-dust-devil kill bits (game buffer 2 +4 & 0xfffc0fff)
pub fn dustdevillevelfivebossConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    dustdevilepisodethreeConstructor(e, api, instanceCount)?;
    if instanceCount == 1 {
        APIcons::SetExternalFunction(e, 0x1d, DustDevilCB_NOTIFY_TIMER_LOCAL);
        APIcons::SetExternalFunction(e, 0, DustDevilST_REINITIALISE);
    }
    APIcons::SetMessageFunction(e, DustDevilLevelFiveBoss_MessageFunction);
    DustDevilLevelFiveBoss_Func_Initialisation(e)?;
    Monster_Base_Func_Initialise(e)?;
    DustDevil_Base_Func_Initialisation(e)?;
    SetDustDevilBossClipping(e)?;
    DustDevilFUNC_Create_Physic(e)?;
    let t = APIextra::TimerCreate(e);
    e.w32(L(e) + 0xf00, t);
    let gb2 = e.r32(GDD(e) + 4);
    let v = e.r32(gb2 + 4) & 0xfffc_0fff;
    e.w32(gb2 + 4, v);
    Ok(())
}

pub fn dustdevillevelfivebossDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    for o in [0x180, 0xf00] {
        let t = e.r32(L(e) + o);
        APIextra::TimerDestroy(e, t);
    }
    dustdevilepisodethreeDestructor(e, instanceCount)
}

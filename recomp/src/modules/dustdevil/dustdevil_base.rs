//! DustDevil_Base.c (the dust devil AI). Addresses are module-relative (.text at 0x10000) in
//! dustdevilleveltwo.
//!
//! States: 0xe FIRSTMOVE, 0x10 NORMAL_SET (zig-zag along the waypoint path L+0x168), 0x11 NORMAL_LOOP
//! (hero within 10 m and in the sector -> 0x13), 0x12 BOSS_SET, 0x13 ALERTED_SET (chase the leader hero),
//! 0x14 ALERTED_LOOP (lost him beyond 13 m -> 0x10), 0x15 ATTACK_SET (from 199 / collide), 0x16
//! ATTACK_LOOP (anim end -> CB_ANIM_LF), 0x17 SCARED_FLEE (hooked: flee along the path), 0x18
//! SCARED_FLEE_LOOP, 0x19 SCARED_ATTACK (every 5 s of fleeing: turn to the book for 3 s), 0x1a
//! BLACK_HOLE_LOOP / boss idle; callbacks 0x1b collide, 0x1c timer, 0x1e anim end, 0x1f cutscene.
//! Local data (L = pLocalDataDustDevil_Base, the Monster_Base block grown to 0xf04):
//! +0x34 mode (0 roaming, 1 fleeing, 2 scared attack, 3 attacking), +0x160 / +0x164 waypoint count /
//! index, +0x168 path, +0x170 path direction, +0x174 may chase, +0x178 chase-cooldown timer (3.5 s),
//! +0x17c flee timer, +0x188 hooked, +0x18c glow created, +400..+0x19c zig-zag (step, steps, size,
//! side), +0x1a0 target waypoint, +0x1b8 cutscene clipping pointer, +0x1c0.. the glow
//! (DustDevilGlowInit.c), +0xefc sector mask.
use super::builds::*;
use super::*;
use crate::engine::*;
use crate::logic::LResult;

/// the leader hero's object id (game buffer 0 +8)
fn hero(e: &Engine) -> u32 {
    e.r32(e.r32(GDM(e)) + 8)
}

/// `DMGetParams(dm, b)` into a fresh 0x80-byte block
fn dm_params(e: &mut Engine, dm: u32) -> Ptr {
    let b = e.alloca(0x80);
    APIphysic::DMGetParams(e, dm, b);
    b
}

/// `DMGetParams(dm); words (offset, value); DMSetParams(dm)`
fn dm_set(e: &mut Engine, dm: u32, words: &[(u32, u32)]) {
    let b = dm_params(e, dm);
    for &(o, v) in words {
        e.w32(b + o, v);
    }
    APIphysic::DMSetParams(e, dm, b);
}

/// `GetClippingParams(SELF, b); b = words; SetClippingParams(SELF, b, 0)` (EFClippingParams, 10 words)
pub(crate) fn set_clipping(e: &mut Engine, words: [u32; 10]) {
    let b = e.alloca(0x28);
    APIgameobj::GetClippingParams(e, SELF, b);
    for (i, w) in words.iter().enumerate() {
        e.w32(b + i as u32 * 4, *w);
    }
    APIgameobj::SetClippingParams(e, SELF, b, 0);
}

/// `{type 1, 0x20}`: the ObjectRetrieverParams of the broadcast 0x155 ("a dust devil attacks")
fn send_155(e: &mut Engine) {
    let p = e.alloca(8);
    e.w32(p, 1);
    e.w32(p + 4, 0x20);
    APIgameobj::SendMessageTo(e, 0xff00_0010, p, 0x155, 0, NULL);
}

/// 0x00014bc0 remember the start (L+0xf8), full health, timers 3.5 s / 5 s, groups, collide notify,
/// clipping, physics on; first state 0xe.
pub fn DustDevil_Base_Func_Initialisation(e: &mut Engine) -> LResult<()> {
    let iVar1 = L(e);
    APIgameobj::GetPosition(e, SELF, ALL, iVar1 + 0xf8);
    let v = e.r32(iVar1 + 0xb0);
    e.w32(iVar1 + 0x5c, v);
    e.w32(iVar1 + 0x174, 1);
    e.w32(iVar1 + 0x170, 1);
    e.w32(iVar1 + 0x188, 0);
    let t = e.r32(iVar1 + 0x178);
    APIextra::TimerSetFrequency(e, t, 3.5);
    let t = e.r32(iVar1 + 0x17c);
    APIextra::TimerSetFrequency(e, t, 5.0);
    let t = e.r32(L(e) + 0x17c);
    APIextra::TimerUnregisterNotify(e, SELF, t);
    APIgameobj::SetGroupBitMask(e, SELF, 0x200621);
    APIgameobj::SetGroupID(e, SELF, 1);
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 3, 0x1b);
    SetDustDevilClipping(e)?;
    APIphysic::ResetFlags(e, SELF, 1);
    APIgameobj::SetNextState(e, SELF, 0xe);
    Ok(())
}

/// 0x00014d70 near 0.1, far 25, fade 80, sector mask 199
pub fn SetDustDevilClipping(e: &mut Engine) -> LResult<()> {
    set_clipping(e, [0x3dcc_cccd, 0x41c8_0000, 0x47, 0, 0x42a0_0000, 0x6a7, 0, 0xbf80_0000, 199, 199]);
    Ok(())
}

/// 0x00014e30 whirl sound + anim; a type-10 dust devil (boss) homes on the hero at 8 m/s (state 0x12),
/// any other walks the path (0x10). The glow is bigger on the boss.
pub fn DustDevil_BaseST_FIRSTMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    let iVar1 = L(e);
    let snd = k(e, DustDevil_BaseST_FIRSTMOVE_0_1);
    APInewSound::Play(e, SELF, snd, 0);
    let a = k(e, DustDevil_BaseST_FIRSTMOVE_0_2);
    APIanim::Play(e, SELF, a, 2, 0.0);
    if e.r32(iVar1 + 0x44) == 9 {
        CreateGlow(e, 0.25)?;
        APIgameobj::SetContextFlag(e, SELF, 0x66);
        SetWalkSpeedParams(e)?;
        APIgameobj::SetNextState(e, SELF, 0x10);
    } else if e.r32(iVar1 + 0x44) == 10 {
        CreateGlow(e, 0.89)?;
        let dm = e.r32(iVar1 + 4);
        let h = hero(e);
        dm_set(e, dm, &[(0, h), (8, 0), (0xc, 0), (0x10, 0), (4, 0x4100_0000), (0x14, 1)]);
        APIgameobj::SetContextFlag(e, SELF, 0x62);
        APIgameobj::SetNextState(e, SELF, 0x12);
    } else {
        CreateGlow(e, 0.25)?;
        APIgameobj::SetContextFlag(e, SELF, 0x66);
        SetWalkSpeedParams(e)?;
        APIgameobj::SetNextState(e, SELF, 0x10);
    }
    Ok(0)
}

/// 0x00015090
pub fn DustDevil_BaseST_BOSS_SET(e: &mut Engine, _: u32) -> LResult<u64> {
    LIBAnimMngr::PlayAnim(e, 0.2, 3, 2, 0);
    APIgameobj::SetNextState(e, SELF, 0x1a);
    Ok(0)
}

/// 0x00015110 head for the nearest waypoint, zig-zag 0.1 m a frame for 20 frames a side
pub fn DustDevil_BaseST_NORMAL_SET(e: &mut Engine, _: u32) -> LResult<u64> {
    let iVar1 = L(e);
    APIgameobj::SetContextFlag(e, SELF, 0x66);
    let a = k(e, DustDevil_BaseST_NORMAL_SET_0_1) as i32;
    LIBAnimMngr::PlayAnim(e, 0.2, a, 2, 0);
    let p = e.r32(iVar1 + 0x168);
    FindNearestPointOnPath(e, p)?;
    e.w32(iVar1 + 400, 0);
    e.w32(iVar1 + 0x19c, 1);
    e.w32(iVar1 + 0x198, 0x3dcc_cccd);
    let s = e.rf(iVar1 + 0x198);
    e.w32(iVar1 + 0x194, (2.0 / s) as i32 as u32);
    APIgameobj::SetNextState(e, SELF, 0x11);
    Ok(0)
}

/// `hero within r && game buffer 0 +0x14 has none of 0x4900094c (hero hidden / busy) && in my sector`
fn can_chase(e: &mut Engine, r: f32) -> LResult<bool> {
    let h = hero(e);
    if LIBApiCFunctions::CheckRangeObject(e, r, SELF, h, 0) == 0 {
        return Ok(false);
    }
    if e.r32(e.r32(GDD(e)) + 0x14) & 0x4900_094c != 0 {
        return Ok(false);
    }
    let s = L(e) + 0xefc;
    Ok(CheckIfInSector(e, s)? != 0)
}

/// 0x00015230 zig-zag; chase the hero when allowed, else next waypoint once within 2 m of it
pub fn DustDevil_BaseST_NORMAL_LOOP(e: &mut Engine, _: u32) -> LResult<u64> {
    let iVar1 = L(e);
    ZigZag(e)?;
    if e.r32(iVar1 + 0x174) == 0 || !can_chase(e, 10.0)? {
        if LIBApiCFunctions::CheckRangePoint(e, 2.0, SELF, iVar1 + 0x1a0, 0) != 0 {
            let p = e.r32(iVar1 + 0x168);
            WayPointTrack(e, p)?;
        }
    } else {
        APIgameobj::SetNextState(e, SELF, 0x13);
    }
    Ok(0)
}

/// 0x00015398 hit flag on, move DM onto the hero
pub fn DustDevil_BaseST_ALERTED_SET(e: &mut Engine, _: u32) -> LResult<u64> {
    let iStack_30 = L(e);
    APIphysic::PrimSetIntAttribBits(e, SELF, 0x3a2, 0, 8);
    let dm = e.r32(iStack_30 + 4);
    let h = hero(e);
    dm_set(e, dm, &[(0, h), (8, 0), (0xc, 0), (0x10, 0)]);
    let a = k(e, DustDevil_BaseST_ALERTED_SET_0_1) as i32;
    LIBAnimMngr::PlayAnim(e, 0.2, a, 2, 0);
    APIgameobj::SetNextState(e, SELF, 0x14);
    Ok(0)
}

/// 0x000154a8 hero lost (beyond 13 m, hidden or out of the sector): hit flag off, back to the path
pub fn DustDevil_BaseST_ALERTED_LOOP(e: &mut Engine, _: u32) -> LResult<u64> {
    if !can_chase(e, 13.0)? {
        APIphysic::PrimResetIntAttribBits(e, SELF, 0x3a2, 0, 8);
        APIgameobj::SetNextState(e, SELF, 0x10);
    }
    Ok(0)
}

/// 0x000155b8 run along the path at 3 m/s
pub fn DustDevil_BaseST_SCARED_FLEE(e: &mut Engine, _: u32) -> LResult<u64> {
    let iVar1 = L(e);
    let p = e.r32(L(e) + 0x168);
    FindNearestPointOnPath(e, p)?;
    let dm = e.r32(iVar1 + 8);
    dm_set(e, dm, &[(0, 0), (4, 0), (8, 0x4040_0000)]);
    APIgameobj::SetContextFlag(e, SELF, 0x66);
    e.w32(iVar1 + 0x34, 1);
    APIgameobj::SetNextState(e, SELF, 0x18);
    Ok(0)
}

/// 0x000156b8
pub fn DustDevil_BaseST_SCARED_FLEE_LOOP(e: &mut Engine, _: u32) -> LResult<u64> {
    let iVar1 = L(e);
    ZigZag(e)?;
    if LIBApiCFunctions::CheckRangePoint(e, 2.0, SELF, iVar1 + 0x1a0, 0) != 0 {
        let p = e.r32(iVar1 + 0x168);
        WayPointTrack(e, p)?;
    }
    Ok(0)
}

/// 0x00015768 turn to face the book (k ID_2a000041), 4 m/s
pub fn DustDevil_BaseST_SCARED_ATTACK(e: &mut Engine, _: u32) -> LResult<u64> {
    let iVar1 = L(e);
    let dm = e.r32(L(e) + 8);
    dm_set(e, dm, &[(0, 0), (4, 0), (8, 0x4080_0000)]);
    APIgameobj::SetContextFlag(e, SELF, 100);
    let book = k(e, ID_2a000041) as i32;
    LIBApiCFunctions::TakeRotation(e, std::f32::consts::PI, SELF, book);
    e.w32(iVar1 + 0x34, 2);
    APIgameobj::SetNextState(e, SELF, 0x1a);
    Ok(0)
}

/// 0x00015870 hit flag off, attack anim (ends in CB_ANIM_LF), stop
pub fn DustDevil_BaseST_ATTACK_SET(e: &mut Engine, _: u32) -> LResult<u64> {
    APIphysic::PrimResetIntAttribBits(e, SELF, 0x3a2, 0, 8);
    let a = k(e, DustDevil_BaseST_ATTACK_SET_0_1) as i32;
    LIBAnimMngr::PlayAnim(e, 0.2, a, 1, 0x1e);
    APIphysic::SetVelocity(e, SELF, 0.0, 0.0, 0.0);
    APIgameobj::SetContextFlag(e, SELF, 0x62);
    APIgameobj::SetNextState(e, SELF, 0x16);
    Ok(0)
}

/// 0x00015958
pub fn DustDevil_BaseST_ATTACK_LOOP(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

/// 0x00015988
pub fn DustDevil_BaseST_BLACK_HOLE_LOOP(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

/// 0x000159b8
pub fn Monster_BaseST_IN_CUT_LOOP(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

/// 0x000159e8
pub fn Monster_BaseST_REINITIALISE(e: &mut Engine, _: u32) -> LResult<u64> {
    DustDevil_Base_Func_Initialisation(e)?;
    Ok(0)
}

/// 0x00015a38 199 (hit the hero): attack (broadcast 0x155, ChangeState 0x15); the book (0xd6): 100
/// link broken -> back to roaming, 0x65 hooked -> flee (5 s flee timer, broadcast 0x155), 0x67 pulled
/// in -> track the book, glow off, black hole (5), 0x68 captured -> hidden; 0xc9 restart (type 9);
/// 0x155 another dust devil attacks -> stop chasing for 3.5 s. Then Monster_Base_MessageFunction.
pub fn DustDevil_BaseMessageFonction(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    let iVar1 = L(e);
    if param_1 == 199 {
        let t = e.r32(L(e) + 0x17c);
        APIextra::TimerUnregisterNotify(e, SELF, t);
        e.w32(iVar1 + 0x188, 1);
        e.w32(iVar1 + 0x34, 3);
        send_155(e);
        APIgameobj::ChangeState(e, 0x15)?;
    } else if 199 < param_1 {
        if param_1 == 0xd6 {
            match param_2 {
                100 => {
                    if e.r32(L(e) + 0x34) != 3 {
                        e.w32(L(e) + 0x188, 0);
                        let t = e.r32(L(e) + 0x17c);
                        APIextra::TimerUnregisterNotify(e, SELF, t);
                        let t = e.r32(iVar1 + 0x178);
                        APIextra::TimerRegisterNotify(e, SELF, t, 0x1c);
                        APIextra::TimerReset(e, t);
                        e.w32(iVar1 + 0x34, 0);
                        SetWalkSpeedParams(e)?;
                        APIgameobj::SetNextState(e, SELF, 0x10);
                    }
                }
                0x65 => {
                    if g(e, G_DustDevil_BaseMessageFonction) == 1 {
                        let (o, s) = (k(e, DustDevil_BaseMessageFonction_1_1), k(e, DustDevil_BaseMessageFonction_1_2));
                        APInewSound::Play(e, o, s, 0);
                    }
                    e.w32(L(e) + 0x188, 1);
                    APIgameobj::SetContextFlag(e, SELF, 0x60);
                    let t = e.r32(L(e) + 0x178);
                    APIextra::TimerUnregisterNotify(e, SELF, t);
                    let t = e.r32(L(e) + 0x17c);
                    APIextra::TimerRegisterNotify(e, SELF, t, 0x1c);
                    APIextra::TimerSetFrequency(e, t, 5.0);
                    APIextra::TimerReset(e, t);
                    APIphysic::PrimSetIntAttribBits(e, SELF, 0x3a2, 0, 8);
                    send_155(e);
                    APIgameobj::SetNextState(e, SELF, 0x17);
                }
                0x67 => {
                    APIphysic::SetFlags(e, SELF, 1);
                    let book = k(e, ID_2a000041);
                    LIBApiCFunctions::TrackID(e, std::f32::consts::PI, SELF as i32, book);
                    let t = e.r32(L(e) + 0x17c);
                    APIextra::TimerUnregisterNotify(e, SELF, t);
                    APIgameobj::SetContextFlag(e, SELF, 0x60);
                    if g(e, G_DustDevil_BaseMessageFonction) == 0 {
                        let (o, s) = (k(e, DustDevil_BaseMessageFonction_0_1), k(e, DustDevil_BaseMessageFonction_0_2));
                        APInewSound::StopSound(e, o, s);
                        let (o, s) = (k(e, DustDevil_BaseMessageFonction_0_3), k(e, ID_4000005d));
                        APInewSound::Play(e, o, s, 0);
                    } else {
                        let s = k(e, DustDevil_BaseMessageFonction_1_3);
                        APInewSound::StopSound(e, SELF, s);
                        let s = k(e, DustDevil_BaseMessageFonction_0_1);
                        APInewSound::Play(e, SELF, s, 0);
                        let (o, s) = (k(e, DustDevil_BaseMessageFonction_0_3), k(e, ID_4000005d));
                        APInewSound::Play(e, o, s, 0);
                    }
                    DestroyGlow(e)?;
                    APIgameobj::SetNextState(e, SELF, 5);
                }
                0x68 => {
                    APIgameobj::SetGameObjectVisible(e, SELF, 0);
                }
                _ => {}
            }
        } else if param_1 < 0xd7 {
            if param_1 == 0xc9 && e.r32(L(e) + 0x44) == 9 {
                APIgameobj::SetNextState(e, SELF, 0);
                APIgameobj::SetGameObjectVisible(e, SELF, 1);
            }
        } else if param_1 == 0x155 && e.r32(L(e) + 0x44) == 9 && e.r32(L(e) + 0x188) == 0 {
            e.w32(L(e) + 0x34, 0);
            e.w32(L(e) + 0x174, 0);
            let t = e.r32(iVar1 + 0x178);
            APIextra::TimerRegisterNotify(e, SELF, t, 0x1c);
            APIextra::TimerReset(e, t);
            APIgameobj::SetNextState(e, SELF, 0x10);
        }
    }
    Monster_Base_MessageFunction(e, param_1, param_2, param_3)
}

/// 0x000160b8
pub fn DustDevil_BaseCB_NOTIFY_COLLIDE_OBJ(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

/// 0x00016110
pub fn Monster_BaseCB_NOTIFY_COLLIDE_OBJ(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

/// 0x00016140 flee timer (L+0x17c): fleeing -> turn on the book for 3 s (0x19), scared attack -> flee
/// again for 5 s (0x17); the chase cooldown (L+0x178) while roaming: reverse along the path, may chase
/// again, collide notify and hit flags back on.
pub fn DustDevil_BaseCB_NOTIFY_TIMER(e: &mut Engine, param_1: u32) -> LResult<u64> {
    let iVar1 = L(e);
    if param_1 == e.r32(L(e) + 0x17c) {
        if e.r32(L(e) + 0x34) == 1 {
            let t = e.r32(L(e) + 0x17c);
            APIextra::TimerSetFrequency(e, t, 3.0);
            APIextra::TimerReset(e, t);
            APIgameobj::SetNextState(e, SELF, 0x19);
            return Ok(0);
        }
        if e.r32(L(e) + 0x34) == 2 {
            let t = e.r32(L(e) + 0x17c);
            APIextra::TimerSetFrequency(e, t, 5.0);
            APIextra::TimerReset(e, t);
            APIgameobj::SetNextState(e, SELF, 0x17);
            return Ok(0);
        }
    } else if e.r32(L(e) + 0x34) == 0 {
        let v = (e.r32(L(e) + 0x170) == 0) as u32;
        e.w32(L(e) + 0x170, v);
        let t = e.r32(iVar1 + 0x178);
        APIextra::TimerUnregisterNotify(e, SELF, t);
        APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 3, 0x1b);
        e.w32(iVar1 + 0x174, 1);
        APIphysic::PrimSetIntAttribBits(e, SELF, 0x3a2, 8, 0xc);
    }
    Ok(1)
}

/// 0x00016370 end of the attack anim: back to roaming, no chase for 3.5 s
pub fn DustDevil_BaseCB_ANIM_LF(e: &mut Engine, param_1: u32) -> LResult<u64> {
    let iVar1 = L(e);
    if param_1 == k(e, DustDevil_BaseCB_ANIM_LF_0_1) {
        e.w32(L(e) + 0x188, 0);
        e.w32(iVar1 + 0x34, 0);
        e.w32(L(e) + 0x174, 0);
        let t = e.r32(iVar1 + 0x178);
        APIextra::TimerRegisterNotify(e, SELF, t, 0x1c);
        APIextra::TimerReset(e, t);
        SetWalkSpeedParams(e)?;
        APIgameobj::SetNextState(e, SELF, 0x10);
    }
    Ok(1)
}

/// 0x00016470 cutscene `{?, on}`: on -> back to the start, saved clipping, cutscene mode (0x1a); off ->
/// end the cutscene, roam (0x10). L+0x1b8 is never set (NULL): the clipping goes to address 0 as on
/// the PS2 (Engine::zero_page).
pub fn DustDevil_BaseCB_CUT_SCENE(e: &mut Engine, param_1: u32) -> LResult<u64> {
    if e.r32(param_1 + 4) == 1 {
        let p = L(e) + 0xf8;
        APIgameobj::SetTranslation(e, SELF, p);
        let c = e.r32(L(e) + 0x1b8);
        APIgameobj::GetClippingParams(e, SELF, c);
        let uVar2 = APIgameobj::GetID(e, SELF, NULL);
        LIBMiscFunc::MiscFunc_InitForCutscene(e, uVar2);
        APIgameobj::SetNextState(e, SELF, 0x1a);
    } else if e.r32(param_1 + 4) == 0 {
        let uVar2 = APIgameobj::GetID(e, SELF, NULL);
        let c = e.r32(L(e) + 0x1b8);
        LIBMiscFunc::MiscFunc_EndCutscene(e, uVar2, c);
        APIgameobj::SetNextState(e, SELF, 0x10);
    }
    Ok(0)
}

/// 0x000165e0 sidestep L+0x198 m a frame for L+0x194 frames, then swap sides
pub fn ZigZag(e: &mut Engine) -> LResult<()> {
    let iVar2 = L(e);
    let uVar1 = e.r32(L(e) + 400);
    e.w32(L(e) + 400, uVar1.wrapping_add(1));
    if e.r32(iVar2 + 0x194) < uVar1 {
        e.w32(iVar2 + 400, 0);
        let v = (e.r32(iVar2 + 0x19c) == 0) as u32;
        e.w32(iVar2 + 0x19c, v);
    } else {
        let d = e.alloca(0xc);
        e.wf(d + 4, 0.0);
        e.wf(d + 8, 0.0);
        let s = e.rf(iVar2 + 0x198);
        e.wf(d, if e.r32(iVar2 + 0x19c) == 0 { -s } else { s });
        APIgameobj::Translate(e, SELF, d);
    }
    Ok(())
}

/// 0x000166c8 the waypoint of path `param_1` nearest in the ground plane -> L+0x1a0 / L+0x164; move
/// DM towards it
pub fn FindNearestPointOnPath(e: &mut Engine, param_1: u32) -> LResult<()> {
    let iVar1 = L(e);
    let mut fStack_2c: f32 = 1e6;
    let DStack_60 = e.alloca(0xc);
    let DStack_50 = e.alloca(0xc);
    APIgameobj::GetPosition(e, SELF, ALL, DStack_60);
    let mut uStack_30: u32 = 0;
    while (uStack_30 as i32) < e.r32(iVar1 + 0x160) as i32 {
        APIextra::WaypointGetPos(e, param_1, uStack_30, DStack_50);
        let fStack_40 = e.rf(DStack_60) - e.rf(DStack_50);
        let fStack_38 = e.rf(DStack_60 + 8) - e.rf(DStack_50 + 8);
        let fStack_28 = APImath::powf(e, fStack_40 * fStack_40 + fStack_38 * fStack_38, 0.5);
        if fStack_28 <= fStack_2c {
            for o in [0, 4, 8] {
                let v = e.r32(DStack_50 + o);
                e.w32(iVar1 + 0x1a0 + o, v);
            }
            e.w32(iVar1 + 0x164, uStack_30);
            fStack_2c = fStack_28;
        }
        uStack_30 += 1;
    }
    let dm = e.r32(iVar1 + 4);
    let (x, y, z) = (e.r32(iVar1 + 0x1a0), e.r32(iVar1 + 0x1a4), e.r32(iVar1 + 0x1a8));
    dm_set(e, dm, &[(0, 0), (8, x), (0xc, y), (0x10, z), (0x14, 5)]);
    APIgameobj::SetContextFlag(e, SELF, 0x66);
    Ok(())
}

/// 0x000168e0 wrap the waypoint index, move DM towards that waypoint, step the index along the path
/// direction (L+0x170)
pub fn WayPointTrack(e: &mut Engine, param_1: u32) -> LResult<()> {
    let iVar1 = L(e);
    let iStack_30 = L(e);
    let i = e.r32(L(e) + 0x164) as i32;
    let n = e.r32(L(e) + 0x160) as i32;
    if i < n {
        if i < 0 {
            e.w32(L(e) + 0x164, (n - 1) as u32);
        }
    } else {
        e.w32(L(e) + 0x164, 0);
    }
    let i = e.r32(iVar1 + 0x164);
    APIextra::WaypointGetPos(e, param_1, i, iVar1 + 0x1a0);
    let dm = e.r32(iStack_30 + 4);
    let (x, y, z) = (e.r32(iStack_30 + 0x1a0), e.r32(iStack_30 + 0x1a4), e.r32(iStack_30 + 0x1a8));
    dm_set(e, dm, &[(0, 0), (8, x), (0xc, y), (0x10, z), (0x14, 5)]);
    APIgameobj::SetContextFlag(e, SELF, 0x66);
    let i = e.r32(iStack_30 + 0x164) as i32;
    let d = if e.r32(iStack_30 + 0x170) == 0 { -1 } else { 1 };
    e.w32(iStack_30 + 0x164, i.wrapping_add(d) as u32);
    Ok(())
}

/// 0x00016a68 physics: prim 0x3a2, weight 0.75, priority 10; DMs: L+4 move (1, onto the hero, ctx 2),
/// L+8 speed (4, ctx 4), L+0 turn (7, 10 rad/s, ctx 1), L+0x24 gravity (0, -20, ctx 0x40), L+0x20
/// slope (8, 75 / 45 deg, ctx 0x20)
pub fn DustDevilFUNC_Create_Physic(e: &mut Engine) -> LResult<()> {
    let p = L(e);
    APIphysic::SetFlags(e, SELF, 6);
    APIphysic::SetPrimaryPrim(e, SELF, 0x3a2);
    APIphysic::SetWeight(e, SELF, 0.75);
    APIphysic::SetPriority(e, SELF, 10);
    APIphysic::PrimResetIntAttribBits(e, SELF, 0x3a2, 0, 4);
    APIphysic::PrimResetIntAttribBits(e, SELF, 0x3a2, 2, 2);
    APIphysic::PrimSetIntAttribBits(e, SELF, 0x3a2, 7, 0x20);
    APIphysic::PrimSetIntAttribBits(e, SELF, 0x3a2, 8, 0xc);
    let dm = APIphysic::DMAdd(e, SELF, 1);
    e.w32(p + 4, dm);
    let (a, h) = (e.r32(p + 0x84), hero(e));
    dm_set(e, dm, &[(4, a), (0, h), (8, 0), (0xc, 0), (0x10, 0), (0x14, 5)]);
    APIphysic::DMSetContextMask(e, dm, 2);
    let dm = APIphysic::DMAdd(e, SELF, 4);
    e.w32(p + 8, dm);
    let a = e.r32(p + 0x98);
    dm_set(e, dm, &[(0, 0), (4, 0), (8, a), (0xc, 1)]);
    APIphysic::DMSetContextMask(e, dm, 4);
    let dm = APIphysic::DMAdd(e, SELF, 7);
    e.w32(p, dm);
    dm_set(e, dm, &[(0, 0x4120_0000), (4, 5)]);
    APIphysic::DMSetContextMask(e, dm, 1);
    let dm = APIphysic::DMAdd(e, SELF, 0);
    e.w32(p + 0x24, dm);
    dm_set(e, dm, &[(0, 0), (4, 0xc1a0_0000), (8, 0)]);
    APIphysic::DMSetContextMask(e, dm, 0x40);
    let dm = APIphysic::DMAdd(e, SELF, 8);
    e.w32(p + 0x20, dm);
    dm_set(e, dm, &[(0, 0x4296_0000), (4, 0x4234_0000)]);
    APIphysic::DMSetContextMask(e, dm, 0x20);
    Ok(())
}

/// 0x00016ec8 states / callbacks and API tables (first instance), the particle types; local data
/// 0xf04; the goCustomDataDustDevil record of this object (path, kill mask, sector mask); timers;
/// srand; a dead dust devil is killed. Returns SetInitialState(0xe)'s result.
/// Ghidra shows `srand((int)fVar4)` with fVar4 the float return register: it is TimerGetTime's
/// result, so the seed is the global timer (as in the zombie family).
pub fn dustdevil_baseConstructor(e: &mut Engine, api: Ptr, param_2: i32) -> LResult<u32> {
    monster_baseConstructor(e, api, param_2)?;
    if param_2 == 1 {
        let fns: [(i32, crate::logic::ExtFn); 16] = [
            (0xe, DustDevil_BaseST_FIRSTMOVE),
            (0x10, DustDevil_BaseST_NORMAL_SET),
            (0x11, DustDevil_BaseST_NORMAL_LOOP),
            (0x12, DustDevil_BaseST_BOSS_SET),
            (0x13, DustDevil_BaseST_ALERTED_SET),
            (0x14, DustDevil_BaseST_ALERTED_LOOP),
            (0x15, DustDevil_BaseST_ATTACK_SET),
            (0x16, DustDevil_BaseST_ATTACK_LOOP),
            (0x17, DustDevil_BaseST_SCARED_FLEE),
            (0x18, DustDevil_BaseST_SCARED_FLEE_LOOP),
            (0x19, DustDevil_BaseST_SCARED_ATTACK),
            (0x1a, DustDevil_BaseST_BLACK_HOLE_LOOP),
            (0x1b, DustDevil_BaseCB_NOTIFY_COLLIDE_OBJ),
            (0x1c, DustDevil_BaseCB_NOTIFY_TIMER),
            (0x1e, DustDevil_BaseCB_ANIM_LF),
            (0x1f, DustDevil_BaseCB_CUT_SCENE),
        ];
        for (i, f) in fns {
            APIcons::SetExternalFunction(e, i, f);
        }
        let tables: [(u32, fn(&mut Engine) -> Ptr); 12] = [
            (pAnim, |e| APIcons::GetApiAnim(e)),
            (pGameObj, |e| APIcons::GetApiGameObj(e)),
            (pExtra, |e| APIcons::GetApiExtra(e)),
            (pPhysic, |e| APIcons::GetApiPhysic(e)),
            (pStdlib, |e| APIcons::GetApiStdlib(e)),
            (pParticle, |e| APIcons::GetApiParticle(e)),
            (pMath, |e| APIcons::GetApiMath(e)),
            (pLight, |e| APIcons::GetApiLight(e)),
            (pGraphic, |e| APIcons::GetApiGraphics(e)),
            (pSound, |e| APIcons::GetApiNewSound(e)),
            (pStdlib, |e| APIcons::GetApiStdlib(e)),
            (pGame, |e| APIcons::GetApiGame(e)),
        ];
        for (i, (slot, get)) in tables.into_iter().enumerate() {
            let v = get(e);
            let a = G(e, slot);
            e.w32(a, v);
            if i == 9 {
                // pFunction is fetched between pSound and the second pStdlib
                let n = e.cstr("ApiCFunctions");
                let v = APIcons::GetApiCustom(e, n);
                let a = G(e, pFunction);
                e.w32(a, v);
            }
        }
        let slot = G(e, pGlobalDataDustDevil_Base);
        APIcons::GetGlobalData(e, slot, 0x14);
        let uVar2 = APIgameobj::GetGameBuffer(e, 0, 0x8f8, 1);
        let gd = GDD(e);
        e.w32(gd, uVar2);
        let uVar2 = APIgameobj::GetGameBuffer(e, 2, 0x158, 1);
        let gd = GDD(e);
        e.w32(gd + 4, uVar2);
        CreateType(e)?;
    }
    let slot = G(e, pLocalDataDustDevil_Base);
    APIcons::GetLocalData(e, slot, 0xf04);
    e.w32(L(e) + 0x18c, 0);
    APIphysic::CreatePhysic(e, SELF, 0);
    let iVar3 = APIgameobj::GetID(e, SELF, NULL);
    let last = k(e, dustdevil_baseConstructor_0_1) as i32;
    let mut iStack_38: i32 = 0;
    while iStack_38 <= last {
        let r = G(e, goCustomDataDustDevil) + iStack_38 as u32 * 0x10;
        if e.r32(r) == iVar3 {
            let v = e.r32(r + 0xc);
            e.w32(L(e) + 0xefc, v);
            let v = e.r32(r + 4);
            e.w32(L(e) + 0x168, v);
            let v = e.r32(r + 8);
            e.w32(L(e) + 0x40, v);
            let p = e.r32(L(e) + 0x168);
            let n = APIextra::WaypointGetNb(e, p);
            e.w32(L(e) + 0x160, n);
            break;
        }
        iStack_38 += 1;
    }
    let t = APIextra::TimerCreate(e);
    e.w32(L(e) + 0x178, t);
    let t = APIextra::TimerCreate(e);
    e.w32(L(e) + 0x17c, t);
    let fVar4 = APIextra::TimerGetTime(e, 0x9000_0000);
    APIstdlib::srand(e, fVar4 as i32 as u32);
    if AmIDead(e)? == 1 {
        APIgameobj::KillObject(e, SELF);
    }
    Ok(APIcons::SetInitialState(e, 0xe))
}

/// 0x00017550 `param_1` = instances left (0: also the particle types)
pub fn dustdevil_baseDestructor(e: &mut Engine, param_1: i32) -> LResult<()> {
    let t = e.r32(L(e) + 0x178);
    APIextra::TimerDestroy(e, t);
    let t = e.r32(L(e) + 0x17c);
    APIextra::TimerDestroy(e, t);
    monster_baseDestructor(e)?;
    DestroyGlow(e)?;
    if param_1 == 0 {
        DestroyType(e)?;
    }
    APIgameobj::SetGroupID(e, SELF, 0);
    Ok(())
}

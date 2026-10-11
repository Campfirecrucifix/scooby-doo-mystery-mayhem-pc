//! Shadows_base.c (41 functions): the shadow AI. States (external function index): 0xe START,
//! 0xf WAITING_HERO, 0x10 SNEAKING_ON_HERO, 0x11 WALK_ON_HERO, 0x12 ATTACK_HERO, 0x13 WHAT_IS_NEXT,
//! 0x14 BEINGSUCKED (hooked by the book), 0x17 FLEE_TOME (back to the post while the book is open),
//! 0x18 STAY_AWAY_FROM_TOME; callbacks 0x19 ANIM_LF (anim finished), 0x1a ANIM (anim events: sounds,
//! collision prims), 0x1b TIMER.
//! Each state polls its checks, which OR event bits into L+0x184; the transitions (TR_*) consume
//! them. Event bits: 1 hero in my sector, 2 hero sees me, 4 hero within 2 (and not hiding),
//! 8 scare again, 0x10 hero within 2, 0x20 at my post, 0x40 hooked by the book, 0x80 pulled in,
//! 0x100 link broken, 0x200 book open, 0x400 book closed, 0x800 hero hiding (game flag 0x40),
//! 0x80000 attack anim done, 0x100000 bouga (scare) anim done, 0x200000 defensive attack anim done,
//! 0x400000 attack pause over.
//! L+0x188 = the anim manager handle of the last PlayAnim, L+0x18c = hooked (1) / free (0),
//! L+0x180 = sector mask (goCustomDataShadows), timers L+0x170 attack pause, +0x174 attack pause 2
//! (hero within 3 -> scare), +0x178 hooked lash-out delay, +0x17c next scare.
//! Addresses are module-relative (.text at 0x10000) in shadowslevelone.
//!
//! Per-build differences (see builds.rs): anim ids and sound ids are literals (`k`); shape group 1
//! of the hooked / pulled-in transitions (shadowslevelthree, shadowslevelthreeold) plays a sound on
//! the book holder first; Shadows_BaseCB_ANIM's three shapes test the same three anims (attack,
//! defensive attack, bouga) in a different order, and group 2 takes the defensive sound from its own
//! literal.
use super::builds::*;
use super::*;
use crate::engine::*;
use crate::logic::LResult;

/// `*(uint *)(pLocalDataShadows_Base + off)`
#[inline]
fn lv(e: &Engine, off: u32) -> u32 {
    e.r32(L(e) + off)
}
#[inline]
fn set_lv(e: &mut Engine, off: u32, v: u32) {
    let l = L(e);
    e.w32(l + off, v);
}
/// `(*(uint *)(pLocalDataShadows_Base + 0x184) & bit) != 0`
#[inline]
fn ev(e: &Engine, bit: u32) -> bool {
    lv(e, 0x184) & bit != 0
}
/// `*(uint *)(pLocalDataShadows_Base + 0x184) |= bit`
#[inline]
fn set_ev(e: &mut Engine, bit: u32) {
    let a = L(e) + 0x184;
    or32(e, a, bit);
}
/// `*(int *)(pLocalDataShadows_Base + 0x188) = PlayAnim(0.5, anim, mode, flags)`
fn play(e: &mut Engine, anim: u32, mode: i32, flags: i32) {
    let r = LIBAnimMngr::PlayAnim(e, 0.5, anim as i32, mode, flags);
    set_lv(e, 0x188, r as u32);
}
fn timer_unreg(e: &mut Engine, off: u32) {
    let t = lv(e, off);
    APIextra::TimerUnregisterNotify(e, SELF, t);
}
/// TimerReset, TimerSetFrequency(f), TimerRegisterNotify(0x1b)
fn timer_start(e: &mut Engine, off: u32, f: f32) {
    let t = lv(e, off);
    APIextra::TimerReset(e, t);
    let t = lv(e, off);
    APIextra::TimerSetFrequency(e, t, f);
    let t = lv(e, off);
    APIextra::TimerRegisterNotify(e, SELF, t, 0x1b);
}
/// as [`timer_start`] with `FloatRandom(3.0, 5.0, 0.5)` (the timer id is read before the call)
fn timer_start_random(e: &mut Engine, off: u32) {
    let t = lv(e, off);
    APIextra::TimerReset(e, t);
    let t = lv(e, off);
    let f = LIBApiCFunctions::FloatRandom(e, 3.0, 5.0, 0.5);
    APIextra::TimerSetFrequency(e, t, f);
    let t = lv(e, off);
    APIextra::TimerRegisterNotify(e, SELF, t, 0x1b);
}
/// `PrimSet/ResetIntAttribBits(SELF, 0x28b, {0,8}, {7,8}, {8,1})` (the collision prim 0x28b that
/// hurts the hero during the attacks)
fn attack_prims(e: &mut Engine, set: bool) {
    for (a, b) in [(0, 8), (7, 8), (8, 1)] {
        if set {
            APIphysic::PrimSetIntAttribBits(e, SELF, 0x28b, a, b);
        } else {
            APIphysic::PrimResetIntAttribBits(e, SELF, 0x28b, a, b);
        }
    }
}
/// `SetNextState(SELF, s); (*GetExternalFunction(s))(0);`
fn go_now(e: &mut Engine, s: u32) -> LResult<()> {
    APIgameobj::SetNextState(e, SELF, s);
    call_external_now(e, s)
}
/// `*(int *)(*pGlobalDataMonster_Base + 8)`: the leader hero
fn hero(e: &Engine) -> u32 {
    e.r32(e.r32(GDM(e)) + 8)
}
/// `*(uint *)(*pGlobalDataShadows_Base + 0x14) & 0x40` (game flags: the hero is hiding)
fn hero_hiding(e: &Engine) -> bool {
    e.r32(e.r32(GDS(e)) + 0x14) & 0x40 != 0
}

/// 0x00013b70 state 0xe
pub fn Shadows_BaseST_START(e: &mut Engine, _: u32) -> LResult<u64> {
    Shadows_BaseTR_START_NONE_WAITING_HERO(e)?;
    Shadows_BaseFUNC_ResetEventFlag(e)?;
    Ok(0)
}

/// 0x00013bd0 -> 0xf, at once
pub fn Shadows_BaseTR_START_NONE_WAITING_HERO(e: &mut Engine) -> LResult<()> {
    Shadows_BaseFUNC_SetClipping(e, 0)?;
    APIgameobj::SetContextFlag(e, SELF, 0x60);
    let a = k(e, Shadows_BaseTR_START_NONE_WAITING_HERO_0_1);
    play(e, a, 2, 0);
    go_now(e, 0xf)
}

/// 0x00013ca8 state 0xf
pub fn Shadows_BaseST_WAITING_HERO(e: &mut Engine, _: u32) -> LResult<u64> {
    Shadows_BaseFUNC_LookOutForHeroFunction(e)?;
    TomeIsOpen(e)?;
    if ev(e, 0x40) {
        Shadows_BaseTR_EVENT_SHADOWS_IS_HOOKED_BEINGSUCKED(e)?;
    }
    if ev(e, 0x200) {
        Shadows_BaseTR_RETURN_TO_STARTING_POSITION(e)?;
    }
    if ev(e, 1) {
        Shadows_BaseTR_WAITING_EVENT_HERO_IN_SECTOR_SNEAKING_ON_HERO(e)?;
    }
    Shadows_BaseFUNC_ResetEventFlag(e)?;
    Ok(0)
}

/// 0x00013d90 -> 0x10, at once
pub fn Shadows_BaseTR_WAITING_EVENT_HERO_IN_SECTOR_SNEAKING_ON_HERO(e: &mut Engine) -> LResult<()> {
    InitHeroInLineOfSight(e)?;
    SetSneakSpeedParams(e)?;
    Shadow_Base_Func_SetTarget(e, 2, 1)?;
    let a = k(e, Shadows_BaseTR_WAITING_EVENT_HERO_IN_SECTOR_SNEAKING_ON_HERO_0_1);
    play(e, a, 2, 0);
    APIgameobj::SetContextFlag(e, SELF, 0x66);
    go_now(e, 0x10)
}

/// 0x00013e90 state 0x10
pub fn Shadows_BaseST_SNEAKING_ON_HERO(e: &mut Engine, _: u32) -> LResult<u64> {
    Shadows_BaseFUNC_LookOutForHeroFunction(e)?;
    TomeIsOpen(e)?;
    if ev(e, 0x40) {
        Shadows_BaseTR_EVENT_SHADOWS_IS_HOOKED_BEINGSUCKED(e)?;
    }
    if ev(e, 0x200) {
        Shadows_BaseTR_RETURN_TO_STARTING_POSITION(e)?;
    }
    if ev(e, 4) {
        Shadows_BaseTR_SNEAKING_EVENT_HERO_IN_ATTACK_RANGE_ATTACK_HERO(e)?;
    }
    if ev(e, 2) {
        Shadows_BaseTR_SNEAKING_EVENT_HERO_SEE_ME(e)?;
    }
    if ev(e, 0x800) {
        Shadow_BaseTR_TELEPORT_TO_STARTING_POS(e)?;
    }
    Shadows_BaseFUNC_ResetEventFlag(e)?;
    Ok(0)
}

/// 0x00013fc8 -> 0x12, at once
pub fn Shadows_BaseTR_SNEAKING_EVENT_HERO_IN_ATTACK_RANGE_ATTACK_HERO(e: &mut Engine) -> LResult<()> {
    let a = k(e, Shadows_BaseTR_SNEAKING_EVENT_HERO_IN_ATTACK_RANGE_ATTACK_HERO_0_1);
    play(e, a, 1, 0x19);
    APIgameobj::SetContextFlag(e, SELF, 0x62);
    Shadows_BaseFUNC_ResetEventFlag(e)?;
    go_now(e, 0x12)
}

/// 0x000140a0 -> 0x11, at once
pub fn Shadows_BaseTR_SNEAKING_EVENT_HERO_SEE_ME(e: &mut Engine) -> LResult<()> {
    SetWalkSpeedParams(e)?;
    Shadow_Base_Func_SetTarget(e, 2, 1)?;
    let a = k(e, Shadows_BaseTR_SNEAKING_EVENT_HERO_SEE_ME_0_1);
    play(e, a, 2, 0);
    APIgameobj::SetContextFlag(e, SELF, 0x66);
    go_now(e, 0x11)
}

/// 0x00014190 back to the post at once -> 0xf, at once
pub fn Shadow_BaseTR_TELEPORT_TO_STARTING_POS(e: &mut Engine) -> LResult<()> {
    let p = L(e) + 0xf8;
    APIgameobj::SetTranslation(e, SELF, p);
    APIgameobj::SetContextFlag(e, SELF, 0x60);
    let a = k(e, Shadow_BaseTR_TELEPORT_TO_STARTING_POS_0_1);
    play(e, a, 2, 0);
    go_now(e, 0xf)
}

/// 0x00014278 state 0x11
pub fn Shadows_BaseST_WALK_ON_HERO(e: &mut Engine, _: u32) -> LResult<u64> {
    Shadows_BaseFUNC_LookOutForHeroFunction(e)?;
    TomeIsOpen(e)?;
    if ev(e, 0x40) {
        Shadows_BaseTR_EVENT_SHADOWS_IS_HOOKED_BEINGSUCKED(e)?;
    }
    if ev(e, 0x200) {
        Shadows_BaseTR_RETURN_TO_STARTING_POSITION(e)?;
    }
    if ev(e, 4) {
        Shadows_BaseTR_SNEAKING_EVENT_HERO_IN_ATTACK_RANGE_ATTACK_HERO(e)?;
    }
    if ev(e, 0x800) {
        Shadow_BaseTR_TELEPORT_TO_STARTING_POS(e)?;
    }
    Shadows_BaseFUNC_ResetEventFlag(e)?;
    Ok(0)
}

/// 0x00014388 state 0x12
pub fn Shadows_BaseST_ATTACK_HERO(e: &mut Engine, _: u32) -> LResult<u64> {
    TomeIsOpen(e)?;
    if ev(e, 0x40) {
        Shadows_BaseTR_EVENT_SHADOWS_IS_HOOKED_BEINGSUCKED(e)?;
    }
    if ev(e, 0x200) {
        Shadows_BaseTR_RETURN_TO_STARTING_POSITION(e)?;
    }
    if ev(e, 0x80000) {
        Shadows_BaseTR_ATTACK_HERO_EVENT_ANIM_LF_WHAT_IS_NEXT(e)?;
    }
    if ev(e, 0x400000) {
        Shadows_BaseTR_WAITING_EVENT_HERO_IN_SECTOR_SNEAKING_ON_HERO(e)?;
    }
    if ev(e, 0x800) {
        Shadow_BaseTR_TELEPORT_TO_STARTING_POS(e)?;
    }
    Shadows_BaseFUNC_ResetEventFlag(e)?;
    Ok(0)
}

/// 0x000144b8 attack done: pause 2 s (timer L+0x170 -> event 0x400000)
pub fn Shadows_BaseTR_ATTACK_HERO_EVENT_ANIM_LF_WHAT_IS_NEXT(e: &mut Engine) -> LResult<()> {
    let a = k(e, Shadows_BaseTR_ATTACK_HERO_EVENT_ANIM_LF_WHAT_IS_NEXT_0_1);
    play(e, a, 2, 0);
    APIgameobj::SetContextFlag(e, SELF, 0x60);
    timer_start(e, 0x170, 2.0);
    Ok(())
}

/// 0x000145a0 (never called) as the attack-done transition, with timer L+0x174
pub fn Shadows_BaseTR_BOUGA_ATTACK_HERO_EVENT_ANIM_LF_WHAT_IS_NEXT(e: &mut Engine) -> LResult<()> {
    let a = k(e, Shadows_BaseTR_BOUGA_ATTACK_HERO_EVENT_ANIM_LF_WHAT_IS_NEXT_0_1);
    play(e, a, 2, 0);
    APIgameobj::SetContextFlag(e, SELF, 0x60);
    timer_start(e, 0x174, 2.0);
    Ok(())
}

/// 0x00014688 state 0x13 (after a lash-out / scare): sneak again, or back to struggling if hooked
pub fn Shadows_BaseST_WHAT_IS_NEXT(e: &mut Engine, _: u32) -> LResult<u64> {
    if ev(e, 0x100) {
        Shadows_BaseTR_BEINGSUCKED_LINK_BROKEN(e)?;
    }
    if ev(e, 0x200000) || ev(e, 0x100000) {
        if lv(e, 0x18c) == 0 {
            Shadows_BaseTR_WAITING_EVENT_HERO_IN_SECTOR_SNEAKING_ON_HERO(e)?;
        } else {
            Shadows_BaseTR_EVENT_SHADOWS_IS_HOOKED_BEINGSUCKED(e)?;
        }
    }
    if ev(e, 0x800) {
        Shadow_BaseTR_TELEPORT_TO_STARTING_POS(e)?;
    }
    Shadows_BaseFUNC_ResetEventFlag(e)?;
    Ok(0)
}

/// 0x000147a8 state 0x14 (hooked by the book)
pub fn Shadows_BaseST_BEINGSUCKED(e: &mut Engine, _: u32) -> LResult<u64> {
    Shadows_BaseFUNC_LookOutForHeroFunction(e)?;
    if ev(e, 0x80) {
        Shadows_BaseTR_BEINGSUCKED_EVENT_PULLED_IN(e)?;
    }
    if ev(e, 0x100) {
        Shadows_BaseTR_BEINGSUCKED_LINK_BROKEN(e)?;
    }
    if ev(e, 0x20) {
        Shadows_BaseTR_BEINGSUCKED_EVENT_NEAR_ORIGINAL_POSITION_BOUGA_ATTACK(e)?;
    }
    if ev(e, 0x10) {
        Shadows_BaseTR_BEINGSUCKED_EVENT_HERO_IN_ATTACK_RANGE_DEFENSIVE_ATTACK(e)?;
    }
    if ev(e, 0x800) {
        Shadow_BaseTR_TELEPORT_TO_STARTING_POS(e)?;
    }
    Shadows_BaseFUNC_ResetEventFlag(e)?;
    Ok(0)
}

/// 0x000148d0 -> 0x13 (scare anim, facing the hero)
pub fn Shadows_BaseTR_BEINGSUCKED_EVENT_NEAR_ORIGINAL_POSITION_BOUGA_ATTACK(e: &mut Engine) -> LResult<()> {
    Shadow_Base_Func_SetTarget(e, 2, 1)?;
    let a = k(e, Shadows_BaseTR_BEINGSUCKED_EVENT_NEAR_ORIGINAL_POSITION_BOUGA_ATTACK_0_1);
    play(e, a, 1, 0x19);
    APIgameobj::SetContextFlag(e, SELF, 0x62);
    APIgameobj::SetNextState(e, SELF, 0x13);
    Ok(())
}

/// 0x00014988 -> 0x13 (defensive attack anim)
pub fn Shadows_BaseTR_BEINGSUCKED_EVENT_HERO_IN_ATTACK_RANGE_DEFENSIVE_ATTACK(e: &mut Engine) -> LResult<()> {
    let a = k(e, Shadows_BaseTR_BEINGSUCKED_EVENT_HERO_IN_ATTACK_RANGE_DEFENSIVE_ATTACK_0_1);
    play(e, a, 1, 0x19);
    APIgameobj::SetContextFlag(e, SELF, 0x62);
    APIgameobj::SetNextState(e, SELF, 0x13);
    Ok(())
}

/// 0x00014a20 hooked by the book: glow on, hooked flag, lash-out timer L+0x178 (3..5 s), pull back
/// towards the post at run-from-tome speed -> 0x14, at once.
/// (shape group 1: sound k_1_2 on object k_1_1 first)
pub fn Shadows_BaseTR_EVENT_SHADOWS_IS_HOOKED_BEINGSUCKED(e: &mut Engine) -> LResult<()> {
    if g(e, G_Shadows_BaseTR_EVENT_SHADOWS_IS_HOOKED_BEINGSUCKED) != 0 {
        let (o, s) = (
            k(e, Shadows_BaseTR_EVENT_SHADOWS_IS_HOOKED_BEINGSUCKED_1_1),
            k(e, Shadows_BaseTR_EVENT_SHADOWS_IS_HOOKED_BEINGSUCKED_1_2),
        );
        APInewSound::Play(e, o, s, 0);
    }
    Shadows_BaseFUNC_SetClipping(e, 1)?;
    set_lv(e, 0x104, 0);
    set_lv(e, 0x10c, 0);
    set_lv(e, 0x108, 0x4020_0000);
    set_lv(e, 0x18c, 1);
    timer_unreg(e, 0x170);
    timer_unreg(e, 0x174);
    timer_start_random(e, 0x178);
    Shadow_Base_Func_SetTarget(e, 1, 1)?;
    SetRunFromTomeSpeedParams(e)?;
    let a = k(e, Shadows_BaseTR_EVENT_SHADOWS_IS_HOOKED_BEINGSUCKED_0_1);
    play(e, a, 2, 0);
    APIgameobj::SetContextFlag(e, SELF, 0x66);
    Shadows_BaseFUNC_ResetEventFlag(e)?;
    go_now(e, 0x14)
}

/// 0x00014c58 pulled in: harmless again -> 0x13, at once (the capture itself is Monster_Base's).
/// (shape group 1: sound k_1_2 on object k_1_1 first)
pub fn Shadows_BaseTR_BEINGSUCKED_EVENT_PULLED_IN(e: &mut Engine) -> LResult<()> {
    if g(e, G_Shadows_BaseTR_BEINGSUCKED_EVENT_PULLED_IN) != 0 {
        let (o, s) = (
            k(e, Shadows_BaseTR_BEINGSUCKED_EVENT_PULLED_IN_1_1),
            k(e, Shadows_BaseTR_BEINGSUCKED_EVENT_PULLED_IN_1_2),
        );
        APInewSound::Play(e, o, s, 0);
    }
    attack_prims(e, false);
    SetRunFromTomeSpeedParams(e)?;
    timer_unreg(e, 0x170);
    timer_unreg(e, 0x174);
    go_now(e, 0x13)
}

/// 0x00014d90 the beam let go: glow off, free -> 0x10, at once
pub fn Shadows_BaseTR_BEINGSUCKED_LINK_BROKEN(e: &mut Engine) -> LResult<()> {
    Shadows_BaseFUNC_SetClipping(e, 0)?;
    set_lv(e, 0x18c, 0);
    attack_prims(e, false);
    timer_unreg(e, 0x174);
    go_now(e, 0x10)
}

/// 0x00014eb8 state 0x17 (book open: going back to the post)
pub fn Shadows_BaseST_FLEE_TOME(e: &mut Engine, _: u32) -> LResult<u64> {
    Shadows_BaseFUNC_LookOutForHeroFunction(e)?;
    TomeIsOpen(e)?;
    if ev(e, 0x40) {
        Shadows_BaseTR_EVENT_SHADOWS_IS_HOOKED_BEINGSUCKED(e)?;
    }
    if ev(e, 0x400) {
        Shadows_BaseTR_START_NONE_WAITING_HERO(e)?;
    }
    if ev(e, 0x20) {
        Shadows_BaseTR_AT_ORIGINAL_POSITION_SCARE_HERO(e)?;
    }
    if ev(e, 0x800) {
        Shadow_BaseTR_TELEPORT_TO_STARTING_POS(e)?;
    }
    Shadows_BaseFUNC_ResetEventFlag(e)?;
    Ok(0)
}

/// 0x00014fc8 state 0x18 (book open, at the post: scare the hero now and then)
pub fn Shadows_BaseST_STAY_AWAY_FROM_TOME(e: &mut Engine, _: u32) -> LResult<u64> {
    Shadows_BaseFUNC_LookOutForHeroFunction(e)?;
    TomeIsOpen(e)?;
    if ev(e, 0x400) {
        Shadows_BaseTR_START_NONE_WAITING_HERO(e)?;
    }
    if ev(e, 0x100000) {
        Shadows_BaseTR_WAIT_FOR_NEXT_BOUGA_ATTACK(e)?;
    }
    if ev(e, 8) {
        Shadows_BaseTR_AT_ORIGINAL_POSITION_SCARE_HERO(e)?;
    }
    if ev(e, 0x800) {
        Shadow_BaseTR_TELEPORT_TO_STARTING_POS(e)?;
    }
    Shadows_BaseFUNC_ResetEventFlag(e)?;
    Ok(0)
}

/// 0x000150e0 scare anim, facing the hero -> 0x18
pub fn Shadows_BaseTR_AT_ORIGINAL_POSITION_SCARE_HERO(e: &mut Engine) -> LResult<()> {
    Shadow_Base_Func_SetTarget(e, 2, 1)?;
    APIgameobj::SetContextFlag(e, SELF, 0x62);
    let a = k(e, Shadows_BaseTR_AT_ORIGINAL_POSITION_SCARE_HERO_0_1);
    play(e, a, 1, 0x19);
    APIgameobj::SetNextState(e, SELF, 0x18);
    Ok(())
}

/// 0x00015198 next scare in 3..5 s (timer L+0x17c -> event 8) -> 0x18
pub fn Shadows_BaseTR_WAIT_FOR_NEXT_BOUGA_ATTACK(e: &mut Engine) -> LResult<()> {
    timer_start_random(e, 0x17c);
    let a = k(e, Shadows_BaseTR_WAIT_FOR_NEXT_BOUGA_ATTACK_0_1);
    play(e, a, 2, 0);
    APIgameobj::SetNextState(e, SELF, 0x18);
    Ok(())
}

/// 0x000152b8 state 0 (released from the book / restart): back at the post -> 0xe
pub fn Monster_BaseST_REINITIALISE(e: &mut Engine, _: u32) -> LResult<u64> {
    timer_unreg(e, 0x170);
    timer_unreg(e, 0x174);
    let p = L(e) + 0xf8;
    APIgameobj::SetTranslation(e, SELF, p);
    SetSneakSpeedParams(e)?;
    APIgameobj::SetContextFlag(e, SELF, 0x66);
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 3, 0xc);
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 0xb, 0x1a);
    APIgameobj::SetGroupBitMask(e, SELF, 0x0020_0605);
    APIgameobj::SetGroupID(e, SELF, 1);
    set_lv(e, 0x18c, 0);
    APIgameobj::SetNextState(e, SELF, 0xe);
    Ok(0)
}

/// 0x00015448 state 8 (cut scene)
pub fn Monster_BaseST_IN_CUT_LOOP(e: &mut Engine, _: u32) -> LResult<u64> {
    let t = lv(e, 0x170);
    APIextra::TimerReset(e, t);
    let t = lv(e, 0x174);
    APIextra::TimerReset(e, t);
    Ok(0)
}

/// 0x000154c0 the book opened: back to the post -> 0x17
pub fn Shadows_BaseTR_RETURN_TO_STARTING_POSITION(e: &mut Engine) -> LResult<()> {
    Shadow_Base_Func_SetTarget(e, 1, 1)?;
    SetSneakSpeedParams(e)?;
    let a = k(e, Shadows_BaseTR_RETURN_TO_STARTING_POSITION_0_1);
    play(e, a, 2, 0);
    APIgameobj::SetContextFlag(e, SELF, 0x66);
    APIgameobj::SetNextState(e, SELF, 0x17);
    Ok(())
}

/// 0x00015588 callback 0xc
pub fn Monster_BaseCB_NOTIFY_COLLIDE_OBJ(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

/// 0x000155d8 callback 0x19 (anim finished, `param_1` = anim): attack -> 0x80000, defensive attack
/// -> 0x200000, bouga -> 0x100000; each notify is one-shot
pub fn Shadows_BaseCB_ANIM_LF(e: &mut Engine, param_1: u32) -> LResult<u64> {
    if param_1 == k(e, Shadows_BaseCB_ANIM_LF_0_1) {
        let b = k(e, Shadows_BaseCB_ANIM_LF_0_2);
        set_ev(e, b);
        let a = k(e, Shadows_BaseCB_ANIM_LF_0_3);
        APIgameobj::UnregisterNotify(e, SELF, a, 1);
    } else if param_1 < k(e, Shadows_BaseCB_ANIM_LF_0_4) {
        if param_1 == k(e, Shadows_BaseCB_ANIM_LF_0_5) {
            let b = k(e, Shadows_BaseCB_ANIM_LF_0_6);
            set_ev(e, b);
            let a = k(e, Shadows_BaseCB_ANIM_LF_0_7);
            APIgameobj::UnregisterNotify(e, SELF, a, 1);
        }
    } else if param_1 == k(e, Shadows_BaseCB_ANIM_LF_0_8) {
        set_ev(e, 0x100000);
        let a = k(e, Shadows_BaseCB_ANIM_LF_0_9);
        APIgameobj::UnregisterNotify(e, SELF, a, 1);
    }
    Ok(0)
}

/// 0x00015740 callback 0x1a (anim event `{anim, type, value}`): type 0 = sound (attack: value 0/1 ->
/// two sounds; defensive attack; bouga), type 1 = attack collision prims on (0) / off (1) for the
/// three attack anims. The three code shapes test the anims in different orders; the anims are
/// distinct in every build, so one test serves them all.
pub fn Shadows_BaseCB_ANIM(e: &mut Engine, param_1: u32) -> LResult<u64> {
    let atk = k(e, Shadows_BaseCB_ANIM_0_1);
    let def = k(e, Shadows_BaseCB_ANIM_0_3);
    let bouga = k(e, Shadows_BaseCB_ANIM_0_4);
    let (uVar1, ty, v) = (e.r32(param_1), e.r32(param_1 + 4), e.r32(param_1 + 8));
    if ty == 0 {
        let snd = if uVar1 == atk {
            match v {
                0 => Some(k(e, ID_40000011)),
                1 => Some(k(e, ID_4000000e)),
                _ => None,
            }
        } else if uVar1 == def {
            Some(match g(e, G_Shadows_BaseCB_ANIM) {
                2 => k(e, Shadows_BaseCB_ANIM_2_1),
                _ => k(e, ID_4000000f),
            })
        } else if uVar1 == bouga {
            Some(k(e, ID_40000012))
        } else {
            None
        };
        if let Some(s) = snd {
            APInewSound::Play(e, SELF, s, 0);
        }
    } else if ty == 1 && (uVar1 == atk || uVar1 == def || uVar1 == bouga) {
        match v {
            0 => attack_prims(e, true),
            1 => attack_prims(e, false),
            _ => {}
        }
    }
    Ok(0)
}

/// 0x00015b60 callback 0x1b (timers)
pub fn Shadows_BaseCB_TIMER(e: &mut Engine, param_1: u32) -> LResult<u64> {
    Monster_BaseCB_NOTIFY_TIMER(e, param_1)?;
    if param_1 == lv(e, 0x170) {
        timer_unreg(e, 0x170);
        timer_unreg(e, 0x174);
        set_ev(e, 0x400000);
    }
    if param_1 == lv(e, 0x174) {
        let h = hero(e);
        if LIBApiCFunctions::CheckRangeObject(e, 3.0, SELF, h, 0) != 0 {
            set_lv(e, 0x18c, 1);
            timer_unreg(e, 0x174);
            set_ev(e, 8);
        }
    }
    if param_1 == lv(e, 0x178) {
        timer_unreg(e, 0x178);
        set_ev(e, 0x10);
    }
    if param_1 == lv(e, 0x17c) {
        timer_unreg(e, 0x17c);
        set_ev(e, 8);
    }
    Ok(0)
}

/// 0x00015dd0 shadow messages, then Monster_Base's. 0xd6 = from the book: 100 link broken (0x100),
/// 0x65 hooked (0x40), 0x67 pulled in (0x80), 0x68 captured (glow off, harmless). 0xc9/0xc1
/// (re)start: reinitialise -> 0xe.
pub fn Shadows_BaseFUNC_MessageHandler(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    if param_1 == 0xc9 || param_1 == 0xc1 {
        Shadow_Base_Func_Initialisation(e)?;
        APIgameobj::SetNextState(e, SELF, 0xe);
    } else if param_1 == 0xd6 {
        match param_2 {
            100 => set_ev(e, 0x100),
            0x65 => set_ev(e, 0x40),
            0x67 => set_ev(e, 0x80),
            0x68 => {
                Shadows_BaseFUNC_SetClipping(e, 0)?;
                set_lv(e, 0x18c, 0);
                attack_prims(e, false);
            }
            _ => {}
        }
    }
    Monster_Base_MessageFunction(e, param_1, param_2, param_3)
}

/// 0x00015ff8 the checks: 1 hero in my sector (0x800 instead if he hides), 2 he sees me, 4 within
/// 2 and not hiding, 0x20 at my post (within 1), 0x10 within 2
pub fn Shadows_BaseFUNC_LookOutForHeroFunction(e: &mut Engine) -> LResult<()> {
    let p = L(e) + 0x180;
    let iVar2 = CheckIfInSector(e, p)?;
    if iVar2 == 0 || hero_hiding(e) {
        if hero_hiding(e) {
            set_ev(e, 0x800);
        }
    } else {
        set_ev(e, 1);
    }
    let p = L(e) + 0xb8;
    if LIBApiCFunctions::InLineOfSight(e, p) != 0 && !hero_hiding(e) {
        set_ev(e, 2);
    }
    let h = hero(e);
    if LIBApiCFunctions::CheckRangeObject(e, 2.0, SELF, h, 0) != 0 && !hero_hiding(e) {
        set_ev(e, 4);
    }
    let p = L(e) + 0xf8;
    if LIBApiCFunctions::CheckRangePoint(e, 1.0, SELF, p, 0) != 0 {
        set_ev(e, 0x20);
    }
    let h = hero(e);
    if LIBApiCFunctions::CheckRangeObject(e, 2.0, SELF, h, 0) != 0 {
        set_ev(e, 0x10);
    }
    Ok(())
}

/// 0x00016250 event 0x200 book open (and powered), 0x400 closed
pub fn TomeIsOpen(e: &mut Engine) -> LResult<()> {
    if IsTomeOpened(e)? == 0 {
        set_ev(e, 0x400);
    } else {
        set_ev(e, 0x200);
    }
    Ok(())
}

/// 0x000162d8 physics: prim 0x28a, weight 50; DMs: L+4 move (type 1), L+8 turn (type 4), L+0x20
/// type 8, L+0x24 gravity (type 0, -20)
pub fn Shadows_BaseFUNC_Create_Physic(e: &mut Engine) -> LResult<()> {
    APIphysic::CreatePhysic(e, SELF, 0);
    APIphysic::SetFlags(e, SELF, 6);
    APIphysic::SetPrimaryPrim(e, SELF, 0x28a);
    for (a, b) in [(0, 8), (7, 8), (8, 1)] {
        APIphysic::PrimSetIntAttribBits(e, SELF, 0x28a, a, b);
    }
    APIphysic::SetWeight(e, SELF, 50.0);
    let b = e.alloca(0x80);
    let uVar1 = APIphysic::DMAdd(e, SELF, 1);
    set_lv(e, 4, uVar1);
    let dm = lv(e, 4);
    APIphysic::DMGetParams(e, dm, b);
    let s = lv(e, 0x80);
    for (o, v) in [(0, 0), (4, s), (8, 0), (0xc, 0), (0x10, 0), (0x14, 5)] {
        e.w32(b + o, v);
    }
    APIphysic::DMSetParams(e, dm, b);
    APIphysic::DMSetContextMask(e, dm, 2);
    let b = e.alloca(0x80);
    let uVar1 = APIphysic::DMAdd(e, SELF, 4);
    set_lv(e, 8, uVar1);
    let dm = lv(e, 8);
    APIphysic::DMGetParams(e, dm, b);
    let s = lv(e, 0x94);
    for (o, v) in [(0, 0), (4, 0), (8, s), (0xc, 1)] {
        e.w32(b + o, v);
    }
    APIphysic::DMSetParams(e, dm, b);
    APIphysic::DMSetContextMask(e, dm, 4);
    // the type 8 DM's parameters are not read first: only the two words set are defined
    let b = e.alloca(0x80);
    let uVar1 = APIphysic::DMAdd(e, SELF, 8);
    set_lv(e, 0x20, uVar1);
    e.w32(b + 4, 0x4234_0000);
    e.w32(b, 0x4296_0000);
    let dm = lv(e, 0x20);
    APIphysic::DMSetParams(e, dm, b);
    APIphysic::DMSetContextMask(e, dm, 0x20);
    let b = e.alloca(0x80);
    let uVar1 = APIphysic::DMAdd(e, SELF, 0);
    set_lv(e, 0x24, uVar1);
    let dm = lv(e, 0x24);
    APIphysic::DMGetParams(e, dm, b);
    for (o, v) in [(0, 0), (4, 0xc1a0_0000), (8, 0)] {
        e.w32(b + o, v);
    }
    APIphysic::DMSetParams(e, dm, b);
    APIphysic::DMSetContextMask(e, dm, 0x40);
    Ok(())
}

/// 0x00016680
pub fn Shadows_BaseFUNC_ResetEventFlag(e: &mut Engine) -> LResult<()> {
    set_lv(e, 0x184, 0);
    Ok(())
}

/// 0x000166b0 the blue glow: off (0) / on (hooked)
pub fn Shadows_BaseFUNC_SetClipping(e: &mut Engine, param_1: i32) -> LResult<()> {
    let a = if param_1 == 0 { 0.0 } else { 1.0 };
    APIgraphics::SetGameObjBlendColor(e, SELF, 0, 0.079, 0.093, 0.28, a);
    Ok(())
}

/// 0x00016780
pub fn Shadow_Base_Func_Initialisation(e: &mut Engine) -> LResult<()> {
    let v = lv(e, 0xb0);
    set_lv(e, 0x5c, v);
    set_lv(e, 0x18c, 0);
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 3, 0xc);
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 0xb, 0x1a);
    APIgameobj::SetGroupBitMask(e, SELF, 0x0020_0605);
    APIgameobj::SetGroupID(e, SELF, 1);
    Ok(())
}

/// 0x00016858 move DM (L+4) target: 1 my post, 2 the leader hero, 3 the book holder (k ID_2a000093);
/// mode (+0x14) 0x11 if `param_2 == 2`, else 5. Any other `param_1` keeps the current target.
pub fn Shadow_Base_Func_SetTarget(e: &mut Engine, param_1: u32, param_2: i32) -> LResult<()> {
    let b = e.alloca(0x80);
    let dm = lv(e, 4);
    APIphysic::DMGetParams(e, dm, b);
    if param_1 == 2 {
        let h = hero(e);
        e.w32(b, h);
        for o in [8, 0xc, 0x10] {
            e.w32(b + o, 0);
        }
    } else if param_1 < 3 {
        if param_1 == 1 {
            e.w32(b, 0);
            for (o, s) in [(8, 0xf8), (0xc, 0xfc), (0x10, 0x100)] {
                let v = lv(e, s);
                e.w32(b + o, v);
            }
        }
    } else if param_1 == 3 {
        let book = k(e, ID_2a000093);
        e.w32(b, book);
        for o in [8, 0xc, 0x10] {
            e.w32(b + o, 0);
        }
    }
    e.w32(b + 0x14, if param_2 == 2 { 0x11 } else { 5 });
    let dm = lv(e, 4);
    APIphysic::DMSetParams(e, dm, b);
    Ok(())
}

/// 0x000169c8 the InLineOfSight block at L+0xb8: does the hero see me (prim 0x28a)
pub fn InitHeroInLineOfSight(e: &mut Engine) -> LResult<()> {
    let h = hero(e);
    set_lv(e, 0xb8, h);
    let uVar1 = APIgameobj::GetID(e, SELF, NULL);
    set_lv(e, 0xbc, uVar1);
    for (o, v) in [
        (0xc0, 0xf00f), (0xd8, 0x28a), (0xdc, 0), (0xc4, 2), (200, 0), (0xcc, 2), (0xd0, 0), (0xd4, 0),
        (0xe4, 0x3f80_0000), (0xe8, 0x3f80_0000),
    ] {
        set_lv(e, o, v);
    }
    Ok(())
}

/// 0x00016ad0
pub fn shadows_baseConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    monster_baseConstructor(e, api, instanceCount)?;
    if instanceCount == 1 {
        let fns: [(i32, crate::logic::ExtFn); 12] = [
            (0xe, Shadows_BaseST_START),
            (0xf, Shadows_BaseST_WAITING_HERO),
            (0x10, Shadows_BaseST_SNEAKING_ON_HERO),
            (0x11, Shadows_BaseST_WALK_ON_HERO),
            (0x12, Shadows_BaseST_ATTACK_HERO),
            (0x13, Shadows_BaseST_WHAT_IS_NEXT),
            (0x14, Shadows_BaseST_BEINGSUCKED),
            (0x17, Shadows_BaseST_FLEE_TOME),
            (0x18, Shadows_BaseST_STAY_AWAY_FROM_TOME),
            (0x19, Shadows_BaseCB_ANIM_LF),
            (0x1a, Shadows_BaseCB_ANIM),
            (0x1b, Shadows_BaseCB_TIMER),
        ];
        for (i, f) in fns {
            APIcons::SetExternalFunction(e, i, f);
        }
        let tables: [(u32, fn(&mut Engine) -> Ptr); 7] = [
            (pAnim, |e| APIcons::GetApiAnim(e)),
            (pGameObj, |e| APIcons::GetApiGameObj(e)),
            (pExtra, |e| APIcons::GetApiExtra(e)),
            (pPhysic, |e| APIcons::GetApiPhysic(e)),
            (pSound, |e| APIcons::GetApiNewSound(e)),
            (pStdlib, |e| APIcons::GetApiStdlib(e)),
            (pParticle, |e| APIcons::GetApiParticle(e)),
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
        let v = APIcons::GetApiGraphics(e);
        let a = G(e, pGraphic);
        e.w32(a, v);
        let slot = G(e, pGlobalDataShadows_Base);
        APIcons::GetGlobalData(e, slot, 0x24);
        let uVar1 = APIgameobj::GetGameBuffer(e, 0, 0x8f8, 1);
        let gd = GDS(e);
        e.w32(gd, uVar1);
        let uVar1 = APIgameobj::GetGameBuffer(e, 2, 0x158, 1);
        let gd = GDS(e);
        e.w32(gd + 4, uVar1);
        let gd = GDS(e);
        e.w32(gd + 0xc, api);
    }
    APIcons::SetMessageFunction(e, Shadows_BaseFUNC_MessageHandler);
    let slot = G(e, pLocalDataShadows_Base);
    APIcons::GetLocalData(e, slot, 0x1ac);
    Shadows_BaseFUNC_Create_Physic(e)?;
    let iStack_3c = APIgameobj::GetID(e, SELF, NULL);
    // records 0..=k are searched (signed compare); an id not found leaves L+0x180 / L+0x40 alone
    let last = k(e, shadows_baseConstructor_0_1) as i32;
    let mut iStack_40: i32 = 0;
    while iStack_40 <= last {
        let r = G(e, goCustomDataShadows) + iStack_40 as u32 * 0xc;
        if e.r32(r) == iStack_3c {
            let v = e.r32(r + 4);
            set_lv(e, 0x180, v);
            let v = e.r32(r + 8);
            set_lv(e, 0x40, v);
            break;
        }
        iStack_40 += 1;
    }
    if AmIDead(e)? == 1 {
        // AmIDead has killed the object already; the second kill is the original's
        APIgameobj::KillObject(e, SELF);
    } else {
        let b = e.alloca(0x40);
        for (o, v) in [(0, 2), (4, 0), (0x14, 0x80), (0x1c, 0x3e19_999a), (0x20, 0x4170_0000), (0x24, 0x100), (0x28, 0x3f40_0000)] {
            e.w32(b + o, v);
        }
        APIgameobj::SetShadowParams(e, SELF, b);
        APIgameobj::SetLogicFlag(e, SELF, 0x800);
        InitHeroInLineOfSight(e)?;
        for o in [0x170, 0x174, 0x178, 0x17c] {
            let t = APIextra::TimerCreate(e);
            set_lv(e, o, t);
        }
        APIcons::SetInitialState(e, 0xe);
    }
    Ok(())
}

/// 0x00017098
pub fn shadows_baseDestructor(e: &mut Engine) -> LResult<()> {
    monster_baseDestructor(e)?;
    for o in [0x170, 0x174, 0x178, 0x17c] {
        let t = lv(e, o);
        APIextra::TimerDestroy(e, t);
    }
    Ok(())
}

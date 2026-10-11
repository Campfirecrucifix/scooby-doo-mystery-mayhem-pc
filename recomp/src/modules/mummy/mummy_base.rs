//! Mummy_Base.c (45 functions): the Ep2 mummy. Addresses are module-relative (.text at 0x10000) in
//! mummyleveltwoa.
//!
//! The mummy sleeps (behaviour L+0x198 == 10) or waits in a sarcophagus (== 11; L+0x194 = the
//! sarcophagus object, which answers 0xe6 when it has opened and is told 0xfc / 0xe6 / 0xe7). Woken by
//! the leader hero nearby (or a noise), it is surprised, follows him along the level's path graph
//! (L+0x294 path name, L+0x310 path block), swipes when in reach (arm dust particles), and now and
//! then stamps (the "defensive attack": shock-wave latent + dust + camera shake, msg 0x56). With the
//! Tome of Doom open within 20 m it flees: back to its sarcophagus when it has one (and hides there,
//! the sarcophagus re-opens 7 s later), else away from the book. Hooked (0xd6/0x65) it is scared;
//! pulled in (0xd6/0x67) it hides and hands over to the binding latent (`mummy_binding`).
//!
//! The sub-state is a CODE POINTER at L+0x178 (see [`addr`] / [`call_state`]); L+0x180 says which
//! conditions [`Mummy_BaseFUNC_GameEventSignaler`] evaluates each frame and L+0x17c receives the
//! event bits:
//! 2 the hero is out of 6 m (with 8) / 4 within, 0x10 within L+0x1f8 (12 m, 25 m in L02C) "heard",
//! 0x20 in swipe reach (1.5 m, facing), 0x40 surprised anim ended, 0x100 path node reached / hero off
//! the path, 0x200 swipe anim ended, 0x400 stamp anim ended, 0x800 time to stamp, 0x1000 / 0x2000
//! within / out of 3.5 m, 0x4000 / 0x8000 the book is / is not open, 0x10000 the sarcophagus opened
//! (msg 0xe6 or the 7 s timer), 0x20000 the sarcophagus is open (0xfb), 0x40000 path end reached.
//!
//! Per-build anim numbers: every build plays the same five looping anims through
//! [`Mummy_BaseFUNC_SetAnim`] (walk, near-walk, sleep, swipe-ready, sarcophagus) and four one-shot
//! anims through AnimMngr (seek, surprised, swipe, stamp), with build-specific numbers. The switch
//! cases of SetAnim / CB_ANIM_LF / CB_NOTIFY_ANIRUN are those numbers in every build (checked against
//! the five sources), so they are written as comparisons against the per-build literals of the
//! states that play them (builds.rs).
#![allow(non_snake_case)]
use super::builds::*;
use super::*;
use crate::engine::*;
use crate::logic::{ExtFn, LResult};

/// Sub-state function addresses (mummyleveltwoa), the values stored at L+0x178.
pub mod addr {
    pub const Mummy_BaseST_IN_SARCOPHAGE_SET: u32 = 0x162d8;
    pub const Mummy_BaseST_IN_SARCOPHAGE_LOOP: u32 = 0x16370;
    pub const Mummy_BaseST_OPEN_SARCOPHAGE_SET: u32 = 0x16450;
    pub const Mummy_BaseST_OPEN_SARCOPHAGE_LOOP: u32 = 0x16518;
    pub const Mummy_BaseST_GET_OUT_SET: u32 = 0x16598;
    pub const Mummy_BaseST_GET_OUT_LOOP: u32 = 0x16760;
    pub const Mummy_BaseST_DEFENSIVE_ATTACK_SET: u32 = 0x16848;
    pub const Mummy_BaseST_DEFENSIVE_ATTACK_LOOP: u32 = 0x168f0;
    pub const Mummy_BaseST_SEEK_SET: u32 = 0x16990;
    pub const Mummy_BaseST_SEEK_LOOP: u32 = 0x16a78;
    pub const Mummy_BaseST_SURPRISED_SET: u32 = 0x16bb8;
    pub const Mummy_BaseST_SURPRISED_LOOP: u32 = 0x16ca8;
    pub const Mummy_BaseST_SLEEP_SET: u32 = 0x16d28;
    pub const Mummy_BaseST_SLEEP_LOOP: u32 = 0x16dd0;
    pub const Mummy_BaseST_FOLLOW_HERO_SET: u32 = 0x16f60;
    pub const Mummy_BaseST_FOLLOW_HERO_LOOP: u32 = 0x17060;
    pub const Mummy_BaseST_FOLLOW_HERO_NEAR_SET: u32 = 0x17280;
    pub const Mummy_BaseST_FOLLOW_HERO_NEAR_LOOP: u32 = 0x17338;
    pub const Mummy_BaseST_FOLLOW_NOISE_SET: u32 = 0x17460;
    pub const Mummy_BaseST_FOLLOW_NOISE_LOOP: u32 = 0x17538;
    pub const Mummy_BaseST_ATTACK_SET: u32 = 0x17788;
    pub const Mummy_BaseST_ATTACK_WAIT: u32 = 0x17838;
    pub const Mummy_BaseST_ATTACK_LOOP: u32 = 0x178f0;
    pub const Mummy_BaseST_SCARED_SET: u32 = 0x179e8;
    pub const Mummy_BaseST_SCARED_LOOP: u32 = 0x17b60;
    pub const Mummy_BaseST_SCARED_FLEE_SET: u32 = 0x17c98;
    pub const Mummy_BaseST_SCARED_FLEE_LOOP: u32 = 0x17d18;
    pub const Mummy_BaseST_ENTER_SARCOPHAGE_SET: u32 = 0x17df8;
    pub const Mummy_BaseST_ENTER_SARCOPHAGE_LOOP: u32 = 0x17f90;
}

/// `(**(code **)(L + 0x178))()`: run the current sub-state.
pub fn call_state(e: &mut Engine) -> LResult<()> {
    use addr::*;
    let f = e.r32(L(e) + 0x178);
    match f {
        Mummy_BaseST_IN_SARCOPHAGE_SET => self::Mummy_BaseST_IN_SARCOPHAGE_SET(e),
        Mummy_BaseST_IN_SARCOPHAGE_LOOP => self::Mummy_BaseST_IN_SARCOPHAGE_LOOP(e),
        Mummy_BaseST_OPEN_SARCOPHAGE_SET => self::Mummy_BaseST_OPEN_SARCOPHAGE_SET(e),
        Mummy_BaseST_OPEN_SARCOPHAGE_LOOP => self::Mummy_BaseST_OPEN_SARCOPHAGE_LOOP(e),
        Mummy_BaseST_GET_OUT_SET => self::Mummy_BaseST_GET_OUT_SET(e),
        Mummy_BaseST_GET_OUT_LOOP => self::Mummy_BaseST_GET_OUT_LOOP(e),
        Mummy_BaseST_DEFENSIVE_ATTACK_SET => self::Mummy_BaseST_DEFENSIVE_ATTACK_SET(e),
        Mummy_BaseST_DEFENSIVE_ATTACK_LOOP => self::Mummy_BaseST_DEFENSIVE_ATTACK_LOOP(e),
        Mummy_BaseST_SEEK_SET => self::Mummy_BaseST_SEEK_SET(e),
        Mummy_BaseST_SEEK_LOOP => self::Mummy_BaseST_SEEK_LOOP(e),
        Mummy_BaseST_SURPRISED_SET => self::Mummy_BaseST_SURPRISED_SET(e),
        Mummy_BaseST_SURPRISED_LOOP => self::Mummy_BaseST_SURPRISED_LOOP(e),
        Mummy_BaseST_SLEEP_SET => self::Mummy_BaseST_SLEEP_SET(e),
        Mummy_BaseST_SLEEP_LOOP => self::Mummy_BaseST_SLEEP_LOOP(e),
        Mummy_BaseST_FOLLOW_HERO_SET => self::Mummy_BaseST_FOLLOW_HERO_SET(e),
        Mummy_BaseST_FOLLOW_HERO_LOOP => self::Mummy_BaseST_FOLLOW_HERO_LOOP(e),
        Mummy_BaseST_FOLLOW_HERO_NEAR_SET => self::Mummy_BaseST_FOLLOW_HERO_NEAR_SET(e),
        Mummy_BaseST_FOLLOW_HERO_NEAR_LOOP => self::Mummy_BaseST_FOLLOW_HERO_NEAR_LOOP(e),
        Mummy_BaseST_FOLLOW_NOISE_SET => self::Mummy_BaseST_FOLLOW_NOISE_SET(e),
        Mummy_BaseST_FOLLOW_NOISE_LOOP => self::Mummy_BaseST_FOLLOW_NOISE_LOOP(e),
        Mummy_BaseST_ATTACK_SET => self::Mummy_BaseST_ATTACK_SET(e),
        Mummy_BaseST_ATTACK_WAIT => self::Mummy_BaseST_ATTACK_WAIT(e),
        Mummy_BaseST_ATTACK_LOOP => self::Mummy_BaseST_ATTACK_LOOP(e),
        Mummy_BaseST_SCARED_SET => self::Mummy_BaseST_SCARED_SET(e),
        Mummy_BaseST_SCARED_LOOP => self::Mummy_BaseST_SCARED_LOOP(e),
        Mummy_BaseST_SCARED_FLEE_SET => self::Mummy_BaseST_SCARED_FLEE_SET(e),
        Mummy_BaseST_SCARED_FLEE_LOOP => self::Mummy_BaseST_SCARED_FLEE_LOOP(e),
        Mummy_BaseST_ENTER_SARCOPHAGE_SET => self::Mummy_BaseST_ENTER_SARCOPHAGE_SET(e),
        Mummy_BaseST_ENTER_SARCOPHAGE_LOOP => self::Mummy_BaseST_ENTER_SARCOPHAGE_LOOP(e),
        // the PS2 would jump to whatever L+0x178 holds (0 when FIRSTMOVE_SET saw a behaviour other
        // than 10 / 11; every record of every build has one of the two): a fault
        _ => panic!("mummy: sub-state code pointer {f:#x} is not a known function"),
    }
}

/// `*(code **)(L + 0x178) = f`
#[inline]
pub(crate) fn set_state(e: &mut Engine, f: u32) {
    let l = L(e);
    e.w32(l + 0x178, f);
}

#[inline]
fn ev(e: &Engine) -> u32 {
    e.r32(L(e) + 0x17c)
}

#[inline]
fn clr(e: &mut Engine, bits: u32) {
    let a = L(e) + 0x17c;
    and32(e, a, !bits);
}

/// leader hero id (game buffer 0 +8)
#[inline]
fn hero(e: &Engine) -> u32 {
    e.r32(e.r32(GDM(e)) + 8)
}

/// game buffer 0 +0x14 (the leader hero's state bits)
#[inline]
fn hero_state(e: &Engine) -> u32 {
    e.r32(e.r32(GDMu(e)) + 0x14)
}

// per-build anim numbers (see the file doc)
fn anim_walk(e: &Engine) -> u32 {
    k(e, Mummy_BaseST_FOLLOW_HERO_SET_0_1)
}
fn anim_near(e: &Engine) -> u32 {
    k(e, Mummy_BaseST_FOLLOW_HERO_NEAR_SET_0_1)
}
fn anim_sleep(e: &Engine) -> u32 {
    k(e, Mummy_BaseST_SLEEP_SET_0_1)
}
fn anim_swipe_ready(e: &Engine) -> u32 {
    k(e, Mummy_BaseST_ATTACK_SET_0_1)
}
fn anim_sarcophagus(e: &Engine) -> u32 {
    k(e, Mummy_BaseST_ENTER_SARCOPHAGE_LOOP_0_1)
}
fn anim_seek(e: &Engine) -> u32 {
    k(e, Mummy_BaseST_SEEK_SET_0_1)
}
fn anim_surprised(e: &Engine) -> u32 {
    k(e, Mummy_BaseST_SURPRISED_SET_0_1)
}
fn anim_swipe(e: &Engine) -> u32 {
    k(e, Mummy_BaseST_ATTACK_WAIT_0_1)
}
fn anim_stamp(e: &Engine) -> u32 {
    k(e, Mummy_BaseST_DEFENSIVE_ATTACK_SET_0_1)
}

/// 0x00015ea8 (re)start: no events, monster type 5, L+0x5c = L+0xb0, not hooked, line of sight,
/// anim-run notify (0xb -> 0x15), group 2, particles; asleep: context 0x60, group mask 0x200611, solid;
/// in a sarcophagus: mask 0x200411, prim bits 0x1000000 on / 8 off, context 0x60, no physics response.
/// State 0x10 (FIRSTMOVE_SET).
pub fn mummy_base_Func_Initialisation(e: &mut Engine) -> LResult<()> {
    let iVar1 = L(e);
    e.w32(L(e) + 0x180, 0);
    e.w32(iVar1 + 0x17c, 0);
    e.w32(L(e) + 0x44, 5);
    let v = e.r32(iVar1 + 0xb0);
    e.w32(iVar1 + 0x5c, v);
    e.w32(iVar1 + 0x1f0, 0);
    InitInLineOfSight(e)?;
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 0xb, 0x15);
    APIgameobj::SetGroupID(e, SELF, 2);
    Create_Particles(e)?;
    let b = e.r32(iVar1 + 0x198);
    if b == 10 {
        APIgameobj::SetContextFlag(e, SELF, 0x60);
        APIgameobj::SetGroupBitMask(e, SELF, 0x200611);
        APIphysic::ResetFlags(e, SELF, 1);
    } else if b == 0xb {
        APIgameobj::SetGroupBitMask(e, SELF, 0x200411);
        APIphysic::PrimSetIntAttribBits(e, SELF, 700, 5, 0x100_0000);
        APIphysic::PrimResetIntAttribBits(e, SELF, 700, 5, 8);
        APIgameobj::SetContextFlag(e, SELF, 0x60);
        APIphysic::SetFlags(e, SELF, 1);
    }
    APIgameobj::SetNextState(e, SELF, 0x10);
    Ok(())
}

/// 0x000160d8 external 0x10: the path (if any); asleep: tell the sarcophagus (if any) 0xfc/1, sub-state
/// SLEEP_SET; in a sarcophagus: 0xfc/0, IN_SARCOPHAGE_SET. Clipping, state 0x11.
pub fn Mummy_BaseST_FIRSTMOVE_SET(e: &mut Engine, _: u32) -> LResult<u64> {
    let iVar1 = L(e);
    if e.r32(L(e) + 0x2d0) != 0 {
        let l = L(e);
        LIBApiPathFinding::SetPath(e, (l + 0x294) as i32, (l + 0x310) as i32);
    }
    let b = e.r32(iVar1 + 0x198);
    if b == 10 {
        let s = e.r32(iVar1 + 0x194);
        if s != 0 {
            APIgameobj::SendMessageTo(e, s, NULL, 0xfc, 1, NULL);
        }
        set_state(e, addr::Mummy_BaseST_SLEEP_SET);
    } else if b == 0xb {
        let s = e.r32(iVar1 + 0x194);
        if s != 0 {
            APIgameobj::SendMessageTo(e, s, NULL, 0xfc, 0, NULL);
        }
        set_state(e, addr::Mummy_BaseST_IN_SARCOPHAGE_SET);
    }
    SetClipping(e)?;
    APIgameobj::SetNextState(e, SELF, 0x11);
    Ok(0)
}

/// 0x00016268 external 0x11: evaluate the conditions, run the sub-state
pub fn Mummy_BaseST_ENTRY_POINT(e: &mut Engine, _: u32) -> LResult<u64> {
    Mummy_BaseFUNC_GameEventSignaler(e)?;
    call_state(e)?;
    Ok(0)
}

/// 0x000162d8 sarcophagus anim (mode 2), watch the book
pub fn Mummy_BaseST_IN_SARCOPHAGE_SET(e: &mut Engine) -> LResult<()> {
    e.w32(L(e) + 0x17c, 0);
    let a = k(e, Mummy_BaseST_IN_SARCOPHAGE_SET_0_1);
    APIanim::Play(e, SELF, a, 2, 0.0);
    Mummy_BaseFUNC_SetConditionsToCheck(e, 0x80)?;
    set_state(e, addr::Mummy_BaseST_IN_SARCOPHAGE_LOOP);
    Ok(())
}

/// 0x00016370 the sarcophagus has opened and the book is not open: open it. The anim is stopped every
/// frame (held on its first frame).
pub fn Mummy_BaseST_IN_SARCOPHAGE_LOOP(e: &mut Engine) -> LResult<()> {
    if ev(e) & 0x10000 != 0 && ev(e) & 0x8000 != 0 {
        clr(e, 0x10000);
        clr(e, 0x8000);
        set_state(e, addr::Mummy_BaseST_OPEN_SARCOPHAGE_SET);
    }
    let a = k(e, Mummy_BaseST_IN_SARCOPHAGE_LOOP_0_1);
    APIanim::StopAnim(e, SELF, a);
    Ok(())
}

/// 0x00016450 ask the sarcophagus to open (0xe6 with our id)
pub fn Mummy_BaseST_OPEN_SARCOPHAGE_SET(e: &mut Engine) -> LResult<()> {
    e.w32(L(e) + 0x17c, 0);
    Mummy_BaseFUNC_SetConditionsToCheck(e, 0)?;
    let uVar4 = APIgameobj::GetID(e, SELF, NULL);
    let s = e.r32(L(e) + 0x194);
    APIgameobj::SendMessageTo(e, s, NULL, 0xe6, uVar4, NULL);
    set_state(e, addr::Mummy_BaseST_OPEN_SARCOPHAGE_LOOP);
    Ok(())
}

/// 0x00016518 wait for 0xfb (open)
pub fn Mummy_BaseST_OPEN_SARCOPHAGE_LOOP(e: &mut Engine) -> LResult<()> {
    if ev(e) & 0x20000 != 0 {
        clr(e, 0x20000);
        set_state(e, addr::Mummy_BaseST_GET_OUT_SET);
    }
    Ok(())
}

/// 0x00016598 walk out of the sarcophagus: target = the path node nearest the sarcophagus, physics
/// response on, walk; re-register the sector clipping block (L+0x324: id, sector 0, type 2)
pub fn Mummy_BaseST_GET_OUT_SET(e: &mut Engine) -> LResult<()> {
    let l = L(e);
    let s = e.r32(l + 0x194);
    APIgameobj::GetPosition(e, s, ALL, l + 0x1e4);
    let DStack_40 = e.alloca(0x10);
    APIgameobj::GetPosition(e, SELF, ALL, DStack_40);
    APIphysic::ResetFlags(e, SELF, 1);
    let l = L(e);
    LIBApiPathFinding::FindPath(e, DStack_40, l + 0x1e4, l + 0x1e4, (l + 0x310) as i32);
    Mummy_BaseFUNC_TrackPos(e, 3.0)?;
    let a = k(e, Mummy_BaseST_GET_OUT_SET_0_1);
    Mummy_BaseFUNC_SetAnim(e, a)?;
    let l = L(e);
    LIBApiSectorClipping::ApiImpl_Unregister(e, l + 0x324);
    let uVar2 = APIgameobj::GetID(e, SELF, NULL);
    let l = L(e);
    e.w32(l + 0x324, uVar2);
    e.w16(l + 0x32e, 2);
    e.w16(l + 0x32c, 0);
    LIBApiSectorClipping::ApiImpl_Register(e, l + 0x324);
    set_state(e, addr::Mummy_BaseST_GET_OUT_LOOP);
    Ok(())
}

/// 0x00016760 out (within 0.3 m of the target): prim bit 8 on, group mask 0x200611, SEEK now
pub fn Mummy_BaseST_GET_OUT_LOOP(e: &mut Engine) -> LResult<()> {
    let p = L(e) + 0x1e4;
    if LIBApiCFunctions::CheckRangePoint(e, 0.3, SELF, p, 0) != 0 {
        APIphysic::PrimSetIntAttribBits(e, SELF, 700, 5, 8);
        APIgameobj::SetGroupBitMask(e, SELF, 0x200611);
        set_state(e, addr::Mummy_BaseST_SEEK_SET);
        call_state(e)?;
    }
    Ok(())
}

/// 0x00016848 the stamp (AnimMngr, blend 0.2, notify 0x17)
pub fn Mummy_BaseST_DEFENSIVE_ATTACK_SET(e: &mut Engine) -> LResult<()> {
    let iVar1 = L(e);
    e.w32(L(e) + 0x17c, 0);
    e.w32(iVar1 + 0x180, 0);
    APIgameobj::SetContextFlag(e, SELF, 0x60);
    let a = anim_stamp(e) as i32;
    LIBAnimMngr::PlayAnim(e, 0.2, a, 1, 0x17);
    set_state(e, addr::Mummy_BaseST_DEFENSIVE_ATTACK_LOOP);
    Ok(())
}

/// 0x000168f0 stamp ended: restart the stamp timer, follow the hero
pub fn Mummy_BaseST_DEFENSIVE_ATTACK_LOOP(e: &mut Engine) -> LResult<()> {
    if ev(e) & 0x400 != 0 {
        clr(e, 0x400);
        let t = e.r32(L(e) + 0x18c);
        APIextra::TimerReset(e, t);
        set_state(e, addr::Mummy_BaseST_FOLLOW_HERO_SET);
    }
    Ok(())
}

/// 0x00016990 look around (seek anim), watch for the hero (2 / 4)
pub fn Mummy_BaseST_SEEK_SET(e: &mut Engine) -> LResult<()> {
    e.w32(L(e) + 0x17c, 0);
    let t = e.r32(L(e) + 0x18c);
    APIextra::TimerReset(e, t);
    APIgameobj::SetContextFlag(e, SELF, 0x60);
    let a = anim_seek(e) as i32;
    LIBAnimMngr::PlayAnim(e, 0.2, a, 1, 0x17);
    Mummy_BaseFUNC_SetConditionsToCheck(e, 6)?;
    set_state(e, addr::Mummy_BaseST_SEEK_LOOP);
    call_state(e)?;
    Ok(())
}

/// 0x00016a78 hero near: follow him (target = his position); heard: follow the noise; seek anim
/// ended: back to sleep
pub fn Mummy_BaseST_SEEK_LOOP(e: &mut Engine) -> LResult<()> {
    if ev(e) & 4 == 0 {
        if ev(e) & 0x10 == 0 {
            if ev(e) & 2 != 0 {
                clr(e, 2);
                set_state(e, addr::Mummy_BaseST_SLEEP_SET);
            }
        } else {
            set_state(e, addr::Mummy_BaseST_FOLLOW_NOISE_SET);
        }
    } else {
        let h = hero(e);
        let p = L(e) + 0x1e4;
        APIgameobj::GetPosition(e, h, ALL, p);
        clr(e, 4);
        set_state(e, addr::Mummy_BaseST_FOLLOW_HERO_SET);
    }
    Ok(())
}

/// 0x00016bb8 surprised: face the hero (speed 7.5), context 0x62
pub fn Mummy_BaseST_SURPRISED_SET(e: &mut Engine) -> LResult<()> {
    let t = e.r32(L(e) + 0x18c);
    APIextra::TimerReset(e, t);
    APIgameobj::SetContextFlag(e, SELF, 0x62);
    Mummy_BaseFUNC_TrackHero(e, 7.5)?;
    let a = anim_surprised(e) as i32;
    LIBAnimMngr::PlayAnim(e, 0.2, a, 1, 0x17);
    Mummy_BaseFUNC_SetConditionsToCheck(e, 6)?;
    set_state(e, addr::Mummy_BaseST_SURPRISED_LOOP);
    call_state(e)?;
    Ok(())
}

/// 0x00016ca8 surprised anim ended: follow the noise
pub fn Mummy_BaseST_SURPRISED_LOOP(e: &mut Engine) -> LResult<()> {
    if ev(e) & 0x40 != 0 {
        clr(e, 0x40);
        set_state(e, addr::Mummy_BaseST_FOLLOW_NOISE_SET);
    }
    Ok(())
}

/// 0x00016d28 sleep anim, context 0x60, watch for the hero
pub fn Mummy_BaseST_SLEEP_SET(e: &mut Engine) -> LResult<()> {
    let a = anim_sleep(e);
    Mummy_BaseFUNC_SetAnim(e, a)?;
    APIgameobj::SetContextFlag(e, SELF, 0x60);
    Mummy_BaseFUNC_SetConditionsToCheck(e, 6)?;
    set_state(e, addr::Mummy_BaseST_SLEEP_LOOP);
    call_state(e)?;
    Ok(())
}

/// 0x00016dd0 hero near or heard (both bits cleared): surprised if he is on our level. NOTE (PS2
/// behaviour kept): the test is (dy * dy) < 3.0, the x / z differences are computed and dropped.
pub fn Mummy_BaseST_SLEEP_LOOP(e: &mut Engine) -> LResult<()> {
    let iVar1 = L(e);
    if ev(e) & 4 != 0 || ev(e) & 0x10 != 0 {
        clr(e, 0x14);
        let a = e.alloca(0x10);
        let b = e.alloca(0x10);
        APIgameobj::GetPosition(e, SELF, ALL, a);
        let h = e.r32(e.r32(GDMu(e)) + 8);
        APIgameobj::GetPosition(e, h, ALL, b);
        let dy = e.rf(b + 4) - e.rf(a + 4);
        if dy * dy < 3.0 {
            let h = hero(e);
            APIgameobj::GetPosition(e, h, ALL, iVar1 + 0x1e4);
            set_state(e, addr::Mummy_BaseST_SURPRISED_SET);
        }
    }
    Ok(())
}

/// 0x00016f60 chase: path to the hero, walk, conditions 0x1fe
pub fn Mummy_BaseST_FOLLOW_HERO_SET(e: &mut Engine) -> LResult<()> {
    let iVar1 = L(e);
    e.w32(L(e) + 0x17c, 0);
    APIgameobj::SetContextFlag(e, SELF, 0x66);
    let h = hero(e);
    APIgameobj::GetPosition(e, h, ALL, iVar1 + 0x1e4);
    Mummy_BaseFUNC_SetPathFinding(e, iVar1 + 0x1e4)?;
    let a = anim_walk(e);
    Mummy_BaseFUNC_SetAnim(e, a)?;
    Mummy_BaseFUNC_SetConditionsToCheck(e, 0x1fe)?;
    set_state(e, addr::Mummy_BaseST_FOLLOW_HERO_LOOP);
    call_state(e)?;
    Ok(())
}

/// 0x00017060 book open: scared; in reach: swipe; within 3.5 m: near; lost (8): follow the noise if
/// heard, else seek; time to stamp: stamp
pub fn Mummy_BaseST_FOLLOW_HERO_LOOP(e: &mut Engine) -> LResult<()> {
    if ev(e) & 0x4000 == 0 {
        if ev(e) & 0x20 == 0 {
            if ev(e) & 0x1000 == 0 {
                if ev(e) & 8 == 0 {
                    if ev(e) & 0x800 != 0 {
                        clr(e, 0x800);
                        set_state(e, addr::Mummy_BaseST_DEFENSIVE_ATTACK_SET);
                    }
                } else {
                    clr(e, 8);
                    if ev(e) & 0x10 == 0 {
                        set_state(e, addr::Mummy_BaseST_SEEK_SET);
                    } else {
                        clr(e, 0x10);
                        set_state(e, addr::Mummy_BaseST_FOLLOW_NOISE_SET);
                    }
                }
            } else {
                clr(e, 0x1000);
                set_state(e, addr::Mummy_BaseST_FOLLOW_HERO_NEAR_SET);
            }
        } else {
            clr(e, 0x20);
            set_state(e, addr::Mummy_BaseST_ATTACK_SET);
        }
    } else {
        clr(e, 0x4000);
        set_state(e, addr::Mummy_BaseST_SCARED_SET);
    }
    Ok(())
}

/// 0x00017280 close: walk straight at the hero (speed 3), near-walk anim (turn speed L+0x9c)
pub fn Mummy_BaseST_FOLLOW_HERO_NEAR_SET(e: &mut Engine) -> LResult<()> {
    e.w32(L(e) + 0x17c, 0);
    Mummy_BaseFUNC_TrackHero(e, 3.0)?;
    APIgameobj::SetContextFlag(e, SELF, 0x66);
    let a = anim_near(e);
    Mummy_BaseFUNC_SetAnim(e, a)?;
    Mummy_BaseFUNC_SetConditionsToCheck(e, 200)?;
    set_state(e, addr::Mummy_BaseST_FOLLOW_HERO_NEAR_LOOP);
    Ok(())
}

/// 0x00017338 book open: scared; in reach: swipe; out of 3.5 m: follow on the path again
pub fn Mummy_BaseST_FOLLOW_HERO_NEAR_LOOP(e: &mut Engine) -> LResult<()> {
    if ev(e) & 0x4000 == 0 {
        if ev(e) & 0x20 == 0 {
            if ev(e) & 0x2000 != 0 {
                clr(e, 0x2000);
                set_state(e, addr::Mummy_BaseST_FOLLOW_HERO_SET);
            }
        } else {
            clr(e, 0x20);
            set_state(e, addr::Mummy_BaseST_ATTACK_SET);
        }
    } else {
        clr(e, 0x4000);
        set_state(e, addr::Mummy_BaseST_SCARED_SET);
    }
    Ok(())
}

/// 0x00017460 path to where the hero is now, walk, conditions 0xba
pub fn Mummy_BaseST_FOLLOW_NOISE_SET(e: &mut Engine) -> LResult<()> {
    let iVar1 = L(e);
    e.w32(L(e) + 0x17c, 0);
    let h = hero(e);
    APIgameobj::GetPosition(e, h, ALL, iVar1 + 0x1e4);
    Mummy_BaseFUNC_SetPathFinding(e, iVar1 + 0x1e4)?;
    let a = anim_walk(e);
    Mummy_BaseFUNC_SetAnim(e, a)?;
    Mummy_BaseFUNC_SetConditionsToCheck(e, 0xba)?;
    set_state(e, addr::Mummy_BaseST_FOLLOW_NOISE_LOOP);
    Ok(())
}

/// 0x00017538 book open: scared; in reach: swipe; near: follow; at the end of the path: listen again
/// (still within L+0x1f8 and not hiding: re-path, else seek); time to stamp: stamp
pub fn Mummy_BaseST_FOLLOW_NOISE_LOOP(e: &mut Engine) -> LResult<()> {
    let iVar1 = L(e);
    if ev(e) & 0x4000 == 0 {
        if ev(e) & 0x20 == 0 {
            if ev(e) & 4 == 0 {
                if ev(e) & 0x100 == 0 {
                    if ev(e) & 0x800 != 0 {
                        clr(e, 0x800);
                        set_state(e, addr::Mummy_BaseST_DEFENSIVE_ATTACK_SET);
                    }
                } else {
                    clr(e, 0x100);
                    let r = e.rf(L(e) + 0x1f8);
                    let h = hero(e);
                    let bVar2 = LIBApiCFunctions::CheckRangeObject(e, r, SELF, h, 0);
                    if bVar2 != 0 && hero_state(e) & 0x4900_097c == 0 {
                        set_state(e, addr::Mummy_BaseST_FOLLOW_NOISE_SET);
                    } else {
                        set_state(e, addr::Mummy_BaseST_SEEK_SET);
                    }
                }
            } else {
                clr(e, 4);
                set_state(e, addr::Mummy_BaseST_FOLLOW_HERO_SET);
            }
        } else {
            clr(e, 0x20);
            set_state(e, addr::Mummy_BaseST_ATTACK_SET);
        }
    } else {
        clr(e, 0x4000);
        set_state(e, addr::Mummy_BaseST_SCARED_SET);
    }
    let _ = iVar1;
    Ok(())
}

/// 0x00017788 swipe-ready anim, context 0x60, conditions 8 (reach)
pub fn Mummy_BaseST_ATTACK_SET(e: &mut Engine) -> LResult<()> {
    e.w32(L(e) + 0x17c, 0);
    let a = anim_swipe_ready(e);
    Mummy_BaseFUNC_SetAnim(e, a)?;
    APIgameobj::SetContextFlag(e, SELF, 0x60);
    Mummy_BaseFUNC_SetConditionsToCheck(e, 8)?;
    set_state(e, addr::Mummy_BaseST_ATTACK_WAIT);
    call_state(e)?;
    Ok(())
}

/// 0x00017838 swipe once 0.5 s have passed since the last one (timer L+0x184)
pub fn Mummy_BaseST_ATTACK_WAIT(e: &mut Engine) -> LResult<()> {
    let t = e.r32(L(e) + 0x184);
    let fVar2 = APIextra::TimerGetTime(e, t);
    if 0.5 <= fVar2 {
        let a = anim_swipe(e) as i32;
        LIBAnimMngr::PlayAnim(e, 0.0, a, 1, 0x17);
        set_state(e, addr::Mummy_BaseST_ATTACK_LOOP);
        call_state(e)?;
    }
    Ok(())
}

/// 0x000178f0 swipe ended: restart the timer; still in reach: wait for the next, else follow
pub fn Mummy_BaseST_ATTACK_LOOP(e: &mut Engine) -> LResult<()> {
    if ev(e) & 0x200 != 0 {
        clr(e, 0x200);
        let t = e.r32(L(e) + 0x184);
        APIextra::TimerReset(e, t);
        if ev(e) & 0x20 == 0 {
            set_state(e, addr::Mummy_BaseST_FOLLOW_HERO_SET);
        } else {
            clr(e, 0x20);
            set_state(e, addr::Mummy_BaseST_ATTACK_WAIT);
        }
    }
    Ok(())
}

/// 0x000179e8 flee from the book: with a path and a sarcophagus, path to the sarcophagus (1: walk
/// there; 0: no path, flee; 2: already there, enter it); else flee straight away
pub fn Mummy_BaseST_SCARED_SET(e: &mut Engine) -> LResult<()> {
    let iVar1 = L(e);
    e.w32(L(e) + 0x17c, 0);
    APIgameobj::SetGroupBitMask(e, SELF, 0x200611);
    if e.r32(iVar1 + 0x2d0) == 0 || e.r32(iVar1 + 0x194) == 0 {
        set_state(e, addr::Mummy_BaseST_SCARED_FLEE_SET);
    } else {
        let DStack_50 = e.alloca(0x10);
        let s = e.r32(iVar1 + 0x194);
        APIgameobj::GetPosition(e, s, ALL, DStack_50);
        let iStack_40 = Mummy_BaseFUNC_SetPathFinding(e, DStack_50)?;
        if iStack_40 == 1 {
            let a = k(e, Mummy_BaseST_SCARED_SET_0_1);
            Mummy_BaseFUNC_SetAnim(e, a)?;
            Mummy_BaseFUNC_SetConditionsToCheck(e, 0x2a0)?;
            set_state(e, addr::Mummy_BaseST_SCARED_LOOP);
        } else if iStack_40 == 0 {
            set_state(e, addr::Mummy_BaseST_SCARED_FLEE_SET);
        } else {
            set_state(e, addr::Mummy_BaseST_ENTER_SARCOPHAGE_SET);
        }
    }
    Ok(())
}

/// 0x00017b60 book closed (and not hooked): follow the hero; path end: enter the sarcophagus; time to
/// stamp: stamp
pub fn Mummy_BaseST_SCARED_LOOP(e: &mut Engine) -> LResult<()> {
    if ev(e) & 0x8000 == 0 || e.r32(L(e) + 0x1f0) != 0 {
        if ev(e) & 0x40000 == 0 {
            if ev(e) & 0x800 != 0 {
                clr(e, 0x800);
                set_state(e, addr::Mummy_BaseST_DEFENSIVE_ATTACK_SET);
            }
        } else {
            clr(e, 0x40000);
            set_state(e, addr::Mummy_BaseST_ENTER_SARCOPHAGE_SET);
        }
    } else {
        clr(e, 0x8000);
        set_state(e, addr::Mummy_BaseST_FOLLOW_HERO_SET);
    }
    Ok(())
}

/// 0x00017c98 run away from the book (0x2a000034 in L02A)
pub fn Mummy_BaseST_SCARED_FLEE_SET(e: &mut Engine) -> LResult<()> {
    let book = k(e, ID_2a000034);
    Mummy_BaseFUNC_FleeFrom(e, book)?;
    Mummy_BaseFUNC_SetConditionsToCheck(e, 0xa0)?;
    set_state(e, addr::Mummy_BaseST_SCARED_FLEE_LOOP);
    Ok(())
}

/// 0x00017d18 book closed (and not hooked): follow the hero; time to stamp: stamp
pub fn Mummy_BaseST_SCARED_FLEE_LOOP(e: &mut Engine) -> LResult<()> {
    if ev(e) & 0x8000 == 0 || e.r32(L(e) + 0x1f0) != 0 {
        if ev(e) & 0x800 != 0 {
            clr(e, 0x800);
            set_state(e, addr::Mummy_BaseST_DEFENSIVE_ATTACK_SET);
        }
    } else {
        clr(e, 0x8000);
        set_state(e, addr::Mummy_BaseST_FOLLOW_HERO_SET);
    }
    Ok(())
}

/// 0x00017df8 walk into the sarcophagus: mask 0x200411; hooked while the hero holds the book: msg
/// 0x92 to the hero (the hook breaks); prim bits 0x1000000 on / 8 off; walk
pub fn Mummy_BaseST_ENTER_SARCOPHAGE_SET(e: &mut Engine) -> LResult<()> {
    let l = L(e);
    let s = e.r32(l + 0x194);
    APIgameobj::GetPosition(e, s, ALL, l + 0x1e4);
    Mummy_BaseFUNC_TrackPos(e, 3.0)?;
    Mummy_BaseFUNC_SetConditionsToCheck(e, 0)?;
    APIgameobj::SetGroupBitMask(e, SELF, 0x200411);
    if hero_state(e) & 1 != 0 && e.r32(L(e) + 0x1f0) == 1 {
        let h = e.r32(e.r32(GDMu(e)) + 8);
        APIgameobj::SendMessageTo(e, h, NULL, 0x92, 0, NULL);
    }
    APIphysic::PrimSetIntAttribBits(e, SELF, 700, 5, 0x100_0000);
    APIphysic::PrimResetIntAttribBits(e, SELF, 700, 5, 8);
    let a = k(e, Mummy_BaseST_ENTER_SARCOPHAGE_SET_0_1);
    Mummy_BaseFUNC_SetAnim(e, a)?;
    set_state(e, addr::Mummy_BaseST_ENTER_SARCOPHAGE_LOOP);
    Ok(())
}

/// 0x00017f90 within 0.5 m of the sarcophagus: sarcophagus anim, no physics response, msg 0xe7 (our
/// id) to it, the 7 s re-open timer (L+0x188 -> notify 0x16), IN_SARCOPHAGE_SET
pub fn Mummy_BaseST_ENTER_SARCOPHAGE_LOOP(e: &mut Engine) -> LResult<()> {
    let iVar2 = L(e);
    let DStack_50 = e.alloca(0x10);
    APIgameobj::GetPosition(e, SELF, ALL, DStack_50);
    let s = e.r32(iVar2 + 0x194);
    if LIBApiCFunctions::CheckRangeObject(e, 0.5, SELF, s, 0) != 0 {
        APIgameobj::SetContextFlag(e, SELF, 0x60);
        let a = anim_sarcophagus(e);
        Mummy_BaseFUNC_SetAnim(e, a)?;
        APIphysic::SetFlags(e, SELF, 1);
        let uVar4 = APIgameobj::GetID(e, SELF, NULL);
        let s = e.r32(iVar2 + 0x194);
        APIgameobj::SendMessageTo(e, s, NULL, 0xe7, uVar4, NULL);
        let t = e.r32(iVar2 + 0x188);
        APIextra::TimerRegisterNotify(e, SELF, t, 0x16);
        APIextra::TimerSetFrequency(e, t, 7.0);
        APIextra::TimerReset(e, t);
        set_state(e, addr::Mummy_BaseST_IN_SARCOPHAGE_SET);
    }
    Ok(())
}

/// 0x00018148 external 8 (cut-scene): hold both timers
pub fn Monster_BaseST_IN_CUT_LOOP(e: &mut Engine, _: u32) -> LResult<u64> {
    let iVar1 = L(e);
    let t = e.r32(L(e) + 0x184);
    APIextra::TimerReset(e, t);
    let t = e.r32(iVar1 + 0x188);
    APIextra::TimerReset(e, t);
    Ok(0)
}

/// 0x000181c8 external 0: visible, back at the start position (L+0xf8), re-initialise
pub fn Monster_BaseST_REINITIALISE(e: &mut Engine, _: u32) -> LResult<u64> {
    APIgameobj::SetGameObjectVisible(e, SELF, 1);
    let p = LM(e) + 0xf8;
    APIgameobj::SetTranslation(e, SELF, p);
    mummy_base_Func_Initialisation(e)?;
    Ok(0)
}

/// 0x00018260 0xfb: the sarcophagus is open; 0xe6: it opened; 0xd6 (book): 100 released -> scared,
/// 0x65 hooked (sound in all but L02B) -> scared, 0x67 pulled in (sound in L02C / L03A / L09) ->
/// hidden, binding latent at our position gets 0x67 + the book's data, particles destroyed, state 5;
/// 0x68 captured: no anim-run notify, sleep / sarcophagus anim, context 0x60. 0x15c: chase;
/// 0x201 / 0x202: enable / disable. Every message then goes to Monster_Base.
pub fn Mummy_BaseMessageFonction(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    let iVar1 = L(e);
    match param_1 {
        0xfb => or32(e, iVar1 + 0x17c, 0x20000),
        0xd6 => match param_2 {
            100 => {
                e.w32(iVar1 + 0x1f0, 0);
                set_state(e, addr::Mummy_BaseST_SCARED_SET);
            }
            0x65 => {
                if g(e, G_Mummy_BaseMessageFonction) != 2 {
                    let (o, s) = (k(e, ID_2a000032), k(e, ID_80000031));
                    APInewSound::Play(e, o, s, 0);
                }
                e.w32(iVar1 + 0x1f0, 1);
                set_state(e, addr::Mummy_BaseST_SCARED_SET);
            }
            0x67 => {
                if g(e, G_Mummy_BaseMessageFonction) == 1 {
                    let (o, s) = (k(e, Mummy_BaseMessageFonction_1_1), k(e, Mummy_BaseMessageFonction_1_2));
                    APInewSound::Play(e, o, s, 0);
                }
                APIphysic::SetFlags(e, SELF, 1);
                let book = k(e, ID_2a000034);
                LIBApiCFunctions::TrackID(e, 3.1415927, SELF as i32, book);
                let a = k(e, Mummy_BaseMessageFonction_0_1);
                Mummy_BaseFUNC_SetAnim(e, a)?;
                APIgameobj::SetContextFlag(e, SELF, 0x60);
                APIgameobj::SetGameObjectVisible(e, SELF, 0);
                let lat = k(e, ID_2a80010c);
                let uVar2 = APIgameobj::LaunchLatant(e, lat);
                e.w32(iVar1 + 400, uVar2);
                let DStack_40 = e.alloca(0x10);
                APIgameobj::GetPosition(e, SELF, ALL, DStack_40);
                APIgameobj::SetTranslation(e, uVar2, DStack_40);
                let b = e.r32(iVar1 + 400);
                APIgameobj::SendMessageTo(e, b, NULL, 0x67, 0, param_3);
                Destroy_Particles(e)?;
                APIgameobj::SetNextState(e, SELF, 5);
            }
            0x68 => {
                APIgameobj::UnregisterNotify(e, SELF, 0xffff_ffff, 0xb);
                let b = e.r32(iVar1 + 0x198);
                if b == 10 {
                    let a = k(e, Mummy_BaseMessageFonction_0_2);
                    APIanim::Play(e, SELF, a, 2, 0.0);
                } else if b == 0xb {
                    let a = k(e, Mummy_BaseMessageFonction_0_3);
                    APIanim::Play(e, SELF, a, 2, 0.0);
                }
                APIgameobj::SetContextFlag(e, SELF, 0x60);
            }
            _ => {} // 0x69 and the rest: nothing
        },
        0xe6 => or32(e, L(e) + 0x17c, 0x10000),
        0x201 => {
            APIgameobj::EnableObject(e, SELF, 1);
        }
        0x15c => set_state(e, addr::Mummy_BaseST_FOLLOW_HERO_SET),
        0x202 => {
            APIgameobj::EnableObject(e, SELF, 0);
        }
        _ => {}
    }
    Monster_Base_MessageFunction(e, param_1, param_2, param_3)
}

/// 0x00018740 external 0xc
pub fn Monster_BaseCB_NOTIFY_COLLIDE_OBJ(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

/// 0x000187d0 external 0x17 (AnimMngr end notify, `param_1` = anim): seek -> 2, swipe -> 0x200,
/// surprised -> 0x40, stamp -> 0x400
pub fn Mummy_BaseCB_ANIM_LF(e: &mut Engine, param_1: u32) -> LResult<u64> {
    let a = L(e) + 0x17c;
    if param_1 == anim_seek(e) {
        or32(e, a, 2);
    } else if param_1 == anim_swipe(e) {
        or32(e, a, 0x200);
    } else if param_1 == anim_surprised(e) {
        or32(e, a, 0x40);
    } else if param_1 == anim_stamp(e) {
        or32(e, a, 0x400);
    }
    Ok(0)
}

/// 0x000188e0 external 0x18 (cut-scene notify, `param_1` -> {?, 1 start / 0 end}): start: save the
/// clipping (into *(L+0x320)), cut-scene mode, state 5; end: restore, state 0x11, sleep
pub fn Mummy_BaseCB_CUT_SCENE(e: &mut Engine, param_1: u32) -> LResult<u64> {
    let w = e.r32(param_1 + 4);
    if w == 1 {
        let p = e.r32(L(e) + 800);
        APIgameobj::GetClippingParams(e, SELF, p);
        let uVar2 = APIgameobj::GetID(e, SELF, NULL);
        LIBMiscFunc::MiscFunc_InitForCutscene(e, uVar2);
        APIgameobj::SetNextState(e, SELF, 5);
    } else if w == 0 {
        let uVar2 = APIgameobj::GetID(e, SELF, NULL);
        let p = e.r32(L(e) + 800);
        LIBMiscFunc::MiscFunc_EndCutscene(e, uVar2, p);
        APIgameobj::SetNextState(e, SELF, 0x11);
        set_state(e, addr::Mummy_BaseST_SLEEP_SET);
    }
    Ok(0)
}

/// 0x00018a38 external 0x15 (anim-run keyframe, `param_1` -> {anim, ?, side}): walk / near-walk: foot
/// and arm dust; swipe: dust off the swinging arm (side 0 right, 1 left); stamp: shock-wave latent
/// 0x2a80010d at our root + (-0.243, 0.03, 0.413) (msg 0xfe), shock-wave dust, camera shake
/// parameters into game buffer 0 +0x7c8 {0, 1.2, 0.01, 0.1, 0.3, 10.0} and msg 0x56 to the camera.
pub fn Mummy_BaseCB_NOTIFY_ANIRUN(e: &mut Engine, param_1: u32) -> LResult<u64> {
    let uVar1 = e.r32(param_1);
    if uVar1 == anim_walk(e) || uVar1 == anim_near(e) {
        Particles_Drop_RT(e)?;
        Particles_Drop_LT(e)?;
    } else if uVar1 == anim_swipe(e) {
        let side = e.r32(param_1 + 8);
        if side == 0 {
            Particles_RightArm(e)?;
        } else if side == 1 {
            Particles_LeftArm(e)?;
        }
    } else if uVar1 == anim_stamp(e) {
        let puStack_40 = e.r32(GDMu(e)) + 0x7c8;
        let lat = k(e, ID_2a80010d);
        let uVar2 = APIgameobj::LaunchLatant(e, lat);
        e.w32(L(e) + 0x1a0, uVar2);
        let d = e.alloca(0x10);
        APIgameobj::GetPosition(e, SELF, 0, d);
        let x = e.rf(d) + -0.243;
        let y = e.rf(d + 4) + 0.03;
        let z = e.rf(d + 8) + 0.413;
        e.wf(d, x);
        e.wf(d + 4, y);
        e.wf(d + 8, z);
        let l = e.r32(L(e) + 0x1a0);
        APIgameobj::SendMessageTo(e, l, NULL, 0xfe, 0, d);
        let em = e.r32(L(e) + 0x274) as i32;
        APIparticle::EmitterStart(e, em, 0);
        let em = e.r32(L(e) + 0x278) as i32;
        APIparticle::EmitterStart(e, em, 0);
        for (i, v) in [0, 0x3f99_999a, 0x3c23_d70a, 0x3dcc_cccd, 0x3e99_999a, 0x4120_0000].into_iter().enumerate() {
            e.w32(puStack_40 + i as u32 * 4, v);
        }
        let cam = k(e, ID_2a000025);
        APIgameobj::SendMessageTo(e, cam, NULL, 0x56, 0, NULL);
    }
    Ok(0)
}

/// 0x00018d60 external 0x16: the sarcophagus timer (L+0x188) fired: one-shot, "opened"
pub fn Mummy_BaseCB_TIMER(e: &mut Engine, param_1: u32) -> LResult<u64> {
    if param_1 == e.r32(L(e) + 0x188) {
        let t = e.r32(L(e) + 0x188);
        APIextra::TimerUnregisterNotify(e, SELF, t);
        or32(e, L(e) + 0x17c, 0x10000);
    }
    Ok(0)
}

/// 0x00018df8 turn DM (L+8): {0, 0, turn speed L+0x98 (L+0x9c for the near-walk)}; play `param_1`
/// (looping, mode 6, blend 0.2; the sarcophagus anim mode 2, no blend) and remember it in L+0x38.
/// Anims other than the build's five play nothing. Context 0x66 in every case.
pub fn Mummy_BaseFUNC_SetAnim(e: &mut Engine, param_1: u32) -> LResult<()> {
    let iStack_30 = L(e);
    let b = e.alloca(0x40);
    let dm = e.r32(iStack_30 + 8);
    APIphysic::DMGetParams(e, dm, b);
    e.w32(b, 0);
    e.w32(b + 4, 0);
    let v = e.r32(iStack_30 + 0x98);
    e.w32(b + 8, v);
    if param_1 == anim_near(e) {
        APIanim::Play(e, SELF, param_1, 6, 0.2);
        e.w32(iStack_30 + 0x38, param_1);
        let v = e.r32(iStack_30 + 0x9c);
        e.w32(b + 8, v);
    } else if param_1 == anim_walk(e) || param_1 == anim_sleep(e) || param_1 == anim_swipe_ready(e) {
        APIanim::Play(e, SELF, param_1, 6, 0.2);
        e.w32(iStack_30 + 0x38, param_1);
    } else if param_1 == anim_sarcophagus(e) {
        APIanim::Play(e, SELF, param_1, 2, 0.0);
        e.w32(iStack_30 + 0x38, param_1);
    }
    let dm = e.r32(iStack_30 + 8);
    APIphysic::DMSetParams(e, dm, b);
    APIgameobj::SetContextFlag(e, SELF, 0x66);
    Ok(())
}

/// 0x00019018 the hero holds the book (gb0 +0x14 & 1), it is open (gb2 & 0x10) and he is within 20 m
pub fn Mummy_BaseFUNC_IsTomeOpen(e: &mut Engine) -> LResult<u32> {
    if hero_state(e) & 1 == 0 || e.r32(e.r32(GDMu(e) + 4)) & 0x10 == 0 {
        return Ok(0);
    }
    let h = hero(e);
    Ok((LIBApiCFunctions::CheckRangeObject(e, 20.0, SELF, h, 0) != 0) as u32)
}

/// 0x000190d0 move DM (L+4) at the leader hero, speed `param_1`, mode 5
pub fn Mummy_BaseFUNC_TrackHero(e: &mut Engine, param_1: f32) -> LResult<()> {
    let iStack_30 = L(e);
    let b = e.alloca(0x40);
    let dm = e.r32(iStack_30 + 4);
    APIphysic::DMGetParams(e, dm, b);
    let h = hero(e);
    e.w32(b, h);
    e.w32(b + 0x14, 5);
    e.w32(b + 8, 0);
    e.w32(b + 0xc, 0);
    e.w32(b + 0x10, 0);
    e.wf(b + 4, param_1);
    APIphysic::DMSetParams(e, dm, b);
    Ok(())
}

/// 0x00019178 move DM (L+4) at the path node L+0x1e4 (y = 0.5), speed `param_1`, mode 5
pub fn Mummy_BaseFUNC_TrackPos(e: &mut Engine, param_1: f32) -> LResult<()> {
    let iStack_30 = L(e);
    let b = e.alloca(0x40);
    let dm = e.r32(iStack_30 + 4);
    APIphysic::DMGetParams(e, dm, b);
    e.w32(b, 0);
    e.w32(b + 0x14, 5);
    let x = e.r32(iStack_30 + 0x1e4);
    e.w32(b + 8, x);
    e.w32(b + 0xc, 0x3f00_0000);
    let z = e.r32(iStack_30 + 0x1ec);
    e.w32(b + 0x10, z);
    e.wf(b + 4, param_1);
    APIphysic::DMSetParams(e, dm, b);
    Ok(())
}

/// 0x00019228 walk anim, move DM (L+4) away from object `param_1` (mode 0x11, speed kept)
pub fn Mummy_BaseFUNC_FleeFrom(e: &mut Engine, param_1: u32) -> LResult<()> {
    let iStack_40 = L(e);
    let a = k(e, Mummy_BaseFUNC_FleeFrom_0_1);
    Mummy_BaseFUNC_SetAnim(e, a)?;
    let b = e.alloca(0x40);
    let dm = e.r32(iStack_40 + 4);
    APIphysic::DMGetParams(e, dm, b);
    e.w32(b + 8, 0);
    e.w32(b + 0xc, 0);
    e.w32(b + 0x10, 0);
    e.w32(b + 0x14, 0x11);
    e.w32(b, param_1);
    APIphysic::DMSetParams(e, dm, b);
    APIgameobj::SetContextFlag(e, SELF, 0x66);
    Ok(())
}

/// 0x000192f8 with a path graph (L+0x2d0): path from us to `param_1`, first node -> L+0x1e4; returns
/// NextPathNode's 2 (arrived) / 0 (no path) as is. Otherwise (or for any other result) walk at
/// L+0x1e4 (speed 3) and return the result, 2 without a graph. NOTE (PS2 behaviour kept): without a
/// graph L+0x1e4 is not updated here; the callers have already stored the target there, except
/// SCARED_SET (which only gets here with a graph).
pub fn Mummy_BaseFUNC_SetPathFinding(e: &mut Engine, param_1: Ptr) -> LResult<i32> {
    let uStack_70 = e.alloca(0x10);
    for o in [0, 4, 8] {
        let v = e.r32(param_1 + o);
        e.w32(uStack_70 + o, v);
    }
    let mut iStack_50 = 2i32;
    if e.r32(L(e) + 0x2d0) != 0 {
        let DStack_60 = e.alloca(0x10);
        APIgameobj::GetPosition(e, SELF, ALL, DStack_60);
        let l = L(e);
        LIBApiPathFinding::FindPath(e, DStack_60, uStack_70, l + 0x1e4, (l + 0x310) as i32);
        let l = L(e);
        iStack_50 = LIBApiPathFinding::NextPathNode(e, l + 0x1e4, (l + 0x310) as i32) as i32;
        if iStack_50 == 2 {
            return Ok(2);
        }
        if iStack_50 == 0 {
            return Ok(0);
        }
    }
    Mummy_BaseFUNC_TrackPos(e, 3.0)?;
    Ok(iStack_50)
}

/// 0x00019468 within 0.5 m of the current node: next node (2 at the end; always 2 without a graph)
pub fn Mummy_BaseFUNC_UpdatePathFinding(e: &mut Engine) -> LResult<i32> {
    let mut iStack_40 = -1;
    let p = L(e) + 0x1e4;
    if LIBApiCFunctions::CheckRangePoint(e, 0.5, SELF, p, 0) != 0 {
        if e.r32(L(e) + 0x2d0) == 0 {
            iStack_40 = 2;
        } else {
            let l = L(e);
            iStack_40 = LIBApiPathFinding::NextPathNode(e, l + 0x1e4, (l + 0x310) as i32) as i32;
            if iStack_40 != 2 && iStack_40 != 0 {
                Mummy_BaseFUNC_TrackPos(e, 3.0)?;
            }
        }
    }
    Ok(iStack_40)
}

/// 0x00019578
pub fn Mummy_BaseFUNC_SetConditionsToCheck(e: &mut Engine, param_1: u32) -> LResult<()> {
    e.w32(L(e) + 0x180, 0);
    let a = L(e) + 0x180;
    or32(e, a, param_1);
    Ok(())
}

/// 0x000195c8 physics: prim 700, weight 0.75, priority 10; move DM 1 (L+4: at the hero, speed L+0x84,
/// mode 5, mask 2), turn DM 4 (L+8: speed L+0x98, mask 4), DM 7 (L+0: {10, 5}, mask 1), gravity DM 0
/// (L+0x24: -20, mask 0x40), DM 8 (L+0x20: {75, 45}, mask 0x20)
pub fn Mummy_BaseFUNC_Create_Physic(e: &mut Engine) -> LResult<()> {
    let puVar1 = L(e);
    APIphysic::CreatePhysic(e, SELF, 0);
    APIphysic::SetFlags(e, SELF, 6);
    APIphysic::SetPrimaryPrim(e, SELF, 700);
    APIphysic::SetWeight(e, SELF, 0.75);
    APIphysic::SetPriority(e, SELF, 10);
    let dm = |e: &mut Engine, slot: u32, ty: u32, words: &[(u32, u32)], mask: u32| {
        let uVar2 = APIphysic::DMAdd(e, SELF, ty);
        e.w32(puVar1 + slot, uVar2);
        let b = e.alloca(0x40);
        APIphysic::DMGetParams(e, uVar2, b);
        for &(o, v) in words {
            e.w32(b + o, v);
        }
        APIphysic::DMSetParams(e, uVar2, b);
        APIphysic::DMSetContextMask(e, uVar2, mask);
    };
    let speed = e.r32(puVar1 + 0x84);
    let h = hero(e);
    dm(e, 4, 1, &[(4, speed), (0, h), (8, 0), (0xc, 0), (0x10, 0), (0x14, 5)], 2);
    let turn = e.r32(puVar1 + 0x98);
    dm(e, 8, 4, &[(0, 0), (4, 0), (8, turn), (0xc, 1)], 4);
    dm(e, 0, 7, &[(0, 0x4120_0000), (4, 5)], 1);
    dm(e, 0x24, 0, &[(0, 0), (4, 0xc1a0_0000), (8, 0)], 0x40);
    dm(e, 0x20, 8, &[(0, 0x4296_0000), (4, 0x4234_0000)], 0x20);
    Ok(())
}

/// 0x000199b0 set the L+0x17c event bits asked for in L+0x180 (see the file doc). `fStack_3c` = distance
/// to the leader hero, `fStack_40` = vertical gap, `iStack_38` = he is on our path graph (1 without
/// one); these are only measured when a condition in 0x6e is asked for (0 / 0 / 1 otherwise).
/// gb0 +0x14 & 0x4900094c (0x4900097c for "heard") = the hero is hiding / in a state the mummy ignores.
/// NOTE (PS2 behaviour kept): condition 0x100 re-paths to the hero position read by the 0x6e block;
/// asked for without any 0x6e bit, the position is whatever the stack held (zeroed here).
pub fn Mummy_BaseFUNC_GameEventSignaler(e: &mut Engine) -> LResult<()> {
    let iVar1 = L(e);
    let mut fStack_40 = 0.0f32;
    let mut fStack_3c = 0.0f32;
    let mut iStack_38 = 1i32;
    let DStack_60 = e.alloca(0x10);
    let DStack_50 = e.alloca(0x10);
    let cond = |e: &Engine| e.r32(iVar1 + 0x180);
    let evp = iVar1 + 0x17c;
    if e.r32(L(e) + 0x180) == 0 {
        return Ok(());
    }
    if cond(e) & 0x6e != 0 {
        let h = hero(e);
        fStack_3c = LIBApiCFunctions::GetDistance(e, SELF as i32, h, ALL as i32, 0);
        APIgameobj::GetPosition(e, SELF, ALL, DStack_60);
        let h = hero(e);
        APIgameobj::GetPosition(e, h, ALL, DStack_50);
        let dy = e.rf(DStack_60 + 4) - e.rf(DStack_50 + 4);
        fStack_40 = APImath::sqrtf(e, dy * dy);
        if e.r32(iVar1 + 0x2d0) == 0 {
            iStack_38 = 1;
        } else {
            let h = hero(e);
            iStack_38 = LIBApiPathFinding::IsOnPath(e, h, (iVar1 + 0x310) as i32) as i32;
        }
    }
    if cond(e) & 8 != 0 {
        let mut out = 1.5 < fStack_3c;
        if !out {
            let h = hero(e);
            out = LIBApiCFunctions::CheckPositionOrientation(e, SELF, h, 0xe007) == 0;
        }
        if out || hero_state(e) & 0x4900_094c != 0 || 0.5 < fStack_40 || iStack_38 == 0 {
            and32(e, evp, 0xffff_ffdf);
        } else {
            or32(e, evp, 0x20);
        }
    }
    if cond(e) & 2 != 0 {
        if 6.0 < fStack_3c || hero_state(e) & 0x4900_094c != 0 || 0.5 < fStack_40 || iStack_38 == 0 {
            or32(e, evp, 8);
            and32(e, evp, 0xffff_fffb);
        } else {
            or32(e, evp, 4);
            and32(e, evp, 0xffff_fff7);
        }
    }
    if cond(e) & 4 != 0 {
        let r = e.rf(L(e) + 0x1f8);
        if r < fStack_3c || hero_state(e) & 0x4900_097c != 0 || 0.5 < fStack_40 || iStack_38 == 0 {
            and32(e, evp, 0xffff_ffef);
        } else {
            or32(e, evp, 0x10);
        }
    }
    if cond(e) & 0x10 != 0 && (iStack_38 == 0 || Mummy_BaseFUNC_UpdatePathFinding(e)? == 2) {
        or32(e, evp, 0x100);
    }
    if cond(e) & 0x200 != 0 && Mummy_BaseFUNC_UpdatePathFinding(e)? == 2 {
        or32(e, evp, 0x40000);
    }
    if cond(e) & 0x40 != 0 {
        if 3.5 < fStack_3c || hero_state(e) & 0x4900_094c != 0 || 0.5 < fStack_40 || iStack_38 == 0 {
            or32(e, evp, 0x2000);
            and32(e, evp, 0xffff_efff);
        } else {
            or32(e, evp, 0x1000);
            and32(e, evp, 0xffff_dfff);
        }
    }
    if cond(e) & 0x20 != 0 && fStack_3c <= 13.0 {
        let t = e.r32(iVar1 + 0x18c);
        let fVar4 = APIextra::TimerGetTime(e, t);
        if e.rf(iVar1 + 0x1fc) <= fVar4 && fStack_40 <= 0.5 {
            or32(e, evp, 0x800);
            let fVar4 = LIBApiCFunctions::FloatRandom(e, 0.0, 5.0, 0.05);
            e.wf(L(e) + 0x1fc, fVar4 + 7.5);
        }
    }
    if cond(e) & 0x80 != 0 {
        if Mummy_BaseFUNC_IsTomeOpen(e)? == 0 {
            or32(e, evp, 0x8000);
        } else {
            or32(e, evp, 0x4000);
        }
    }
    if cond(e) & 0x100 != 0
        && e.r32(L(e) + 0x2d0) != 0
        && LIBApiPathFinding::IsTargetInSameNode(e, DStack_50, (iVar1 + 0x310) as i32) == 0
    {
        for o in [0, 4, 8] {
            let v = e.r32(DStack_50 + o);
            e.w32(iVar1 + 0x1e4 + o, v);
        }
        Mummy_BaseFUNC_SetPathFinding(e, iVar1 + 0x1e4)?;
    }
    Ok(())
}

/// 0x0001a178 Monster_Base first, then (first instance) externals 0x10 / 0x11 / 0x15..0x18, API tables,
/// global data (+0 game buffer 0, +4 game buffer 2, +0xc 0); local data 0x330 bytes, physics, the
/// AnimMngr end notify (3 -> 0xc); the goCustomDataMummy record of this object (records 0..=n of 0x50
/// bytes: {id, sarcophagus -> L+0x194, behaviour -> L+0x198, kill mask -> L+0x19c / L+0x40, has a
/// path -> L+0x2d0, path name -> L+0x294}); a dead mummy is killed (not in L09). Particles not created
/// (L+500), path block (L+0x310 our id, L+0x31c 1), hearing range 12 m, three timers, srand, the next
/// stamp time (7.5..12.5 s), sector clipping block (L+0x324).
/// Ghidra shows `fVar5 = 12.0; ... srand((int)fVar5 + param_2)` with fVar5 the float return register:
/// it is TimerGetTime(0x90000000)'s result (the call just before), as in the zombie modules.
pub fn mummy_baseConstructor(e: &mut Engine, api: Ptr, param_2: i32) -> LResult<()> {
    monster_baseConstructor(e, api, param_2)?;
    if param_2 == 1 {
        let fns: [(i32, ExtFn); 6] = [
            (0x10, Mummy_BaseST_FIRSTMOVE_SET),
            (0x11, Mummy_BaseST_ENTRY_POINT),
            (0x15, Mummy_BaseCB_NOTIFY_ANIRUN),
            (0x16, Mummy_BaseCB_TIMER),
            (0x17, Mummy_BaseCB_ANIM_LF),
            (0x18, Mummy_BaseCB_CUT_SCENE),
        ];
        for (i, f) in fns {
            APIcons::SetExternalFunction(e, i, f);
        }
        let tables: [(u32, fn(&mut Engine) -> Ptr); 8] = [
            (pAnim, |e| APIcons::GetApiAnim(e)),
            (pGameObj, |e| APIcons::GetApiGameObj(e)),
            (pExtra, |e| APIcons::GetApiExtra(e)),
            (pPhysic, |e| APIcons::GetApiPhysic(e)),
            (pSound, |e| APIcons::GetApiNewSound(e)),
            (pStdlib, |e| APIcons::GetApiStdlib(e)),
            (pParticle, |e| APIcons::GetApiParticle(e)),
            (pMath, |e| APIcons::GetApiMath(e)),
        ];
        for (slot, get) in tables {
            let v = get(e);
            let a = G(e, slot);
            e.w32(a, v);
        }
        for (slot, name) in [(pFunction, "ApiCFunctions"), (pPathFinding, "ApiPathFinding"), (pSectorClip, "ApiSectorClipping")] {
            let n = e.cstr(name);
            let v = APIcons::GetApiCustom(e, n);
            let a = G(e, slot);
            e.w32(a, v);
        }
        let slot = G(e, pGlobalDataMummy_Base);
        APIcons::GetGlobalData(e, slot, 0x10);
        let uVar2 = APIgameobj::GetGameBuffer(e, 0, 0x8f8, 1);
        let gd = GDMu(e);
        e.w32(gd, uVar2);
        let uVar2 = APIgameobj::GetGameBuffer(e, 2, 0x158, 1);
        let gd = GDMu(e);
        e.w32(gd + 4, uVar2);
        e.w32(gd + 0xc, 0);
    }
    let slot = G(e, pLocalDataMummy_Base);
    APIcons::GetLocalData(e, slot, 0x330);
    Mummy_BaseFUNC_Create_Physic(e)?;
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 3, 0xc);
    let iVar3 = APIgameobj::GetID(e, SELF, NULL);
    let n = k(e, mummy_baseConstructor_0_1) as i32;
    let mut iStack_34 = 0i32;
    while iStack_34 <= n {
        let rec = G(e, goCustomDataMummy) + iStack_34 as u32 * 0x50;
        if e.r32(rec) == iVar3 {
            let l = L(e);
            for (src, dst) in [(4, 0x194), (8, 0x198), (0xc, 0x19c)] {
                let v = e.r32(rec + src);
                e.w32(l + dst, v);
            }
            let v = e.r32(l + 0x19c);
            e.w32(l + 0x40, v);
            let v = e.r32(rec + 0x10);
            e.w32(l + 0x2d0, v);
            LIBMiscFunc::MiscFunc_StrncpySafe(e, l + 0x294, rec + 0x14, 0x3c);
            break;
        }
        iStack_34 += 1;
    }
    if g(e, G_mummy_baseConstructor) != 1 && AmIDead(e)? == 1 {
        let s = e.cstr("***************************************************************JE KILL LA MOMMY!");
        APIextra::null_printf(e, s, 0);
        APIgameobj::KillObject(e, SELF);
    }
    let l = L(e);
    e.w32(l + 500, 1);
    e.w32(l + 0x31c, 1);
    e.w32(l + 0x310, iVar3);
    e.w32(l + 0x1f8, 0x4140_0000);
    for o in [0x184, 0x188, 0x18c] {
        let uVar2 = APIextra::TimerCreate(e);
        e.w32(L(e) + o, uVar2);
    }
    let fVar5 = APIextra::TimerGetTime(e, 0x9000_0000);
    APIstdlib::srand(e, (fVar5 as i32).wrapping_add(param_2) as u32);
    let fVar5 = LIBApiCFunctions::FloatRandom(e, 0.0, 5.0, 0.05);
    e.wf(L(e) + 0x1fc, fVar5 + 7.5);
    e.w32(L(e) + 0x1a0, 0);
    let uVar2 = APIgameobj::GetID(e, SELF, NULL);
    let l = L(e);
    e.w32(l + 0x324, uVar2);
    e.w16(l + 0x32e, 2);
    e.w16(l + 0x32c, 0);
    LIBApiSectorClipping::ApiImpl_Register(e, l + 0x324);
    Ok(())
}

/// 0x0001a848
pub fn mummy_baseDestructor(e: &mut Engine) -> LResult<()> {
    let iVar1 = L(e);
    monster_baseDestructor(e)?;
    let t = e.r32(iVar1 + 0x184);
    APIextra::TimerDestroy(e, t);
    let t = e.r32(iVar1 + 0x188);
    APIextra::TimerDestroy(e, t);
    let t = e.r32(L(e) + 0x18c);
    APIextra::TimerDestroy(e, t);
    let l = L(e);
    LIBApiSectorClipping::ApiImpl_Unregister(e, l + 0x324);
    Ok(())
}

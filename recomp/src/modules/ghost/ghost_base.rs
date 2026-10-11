//! Ghost_base.c (53 functions): the ghost AI. States (external function index): 0xe START,
//! 0xf ROAMING_ON_PATH, 0x10 WAITING, 0x11 SEARCH_MODE, 0x12 CHASSING_HERO, 0x13 ATTACK_HERO,
//! 0x15 BEINGSUCKED_LOOP (hooked by the book), 0x16 REGANING_CONFIDANCE, 0x17 TURN_ON_HERO_LOOP,
//! 0x18 TOME_IS_OPEN_FLEE; callbacks 0x1b ANIM_LF, 0x1d NOTIFY_TIMER, 0x1e NOTIFY_STATE_CHANGE.
//! Each state polls its checks, which OR event bits into L+0x19c; the transitions (TR_*) consume
//! them. Event bits: 8 waypoint reached, 0x10 hero seen, 0x20 noise heard, 0x40 waited, 0x80 noise
//! elsewhere, 0x100 hero close, 0x200 search point reached, 0x400 hero in attack range, 0x800 attack
//! anim done, 0x1000 hooked by the book, 0x2000 pulled in, 0x4000 link broken, 0x8000 turn on the
//! hero, 0x10000 book open, 0x20000 book closed, 0x40000 hero out of sector.
//! Timers: L+0x160 back to roaming after an attack, +0x164 idle sounds, +0x168 confidence regained,
//! +0x170 turn on the hero while hooked, +0x878 looking-around step.
//! Addresses are module-relative (.text at 0x10000) in smallghostlevelone__757255de.
//!
//! Code shape group 1 of several functions is the Ep6 ghost boss (ghostbosslevel5, 1 build): it
//! never flees the open book (no TOME_IS_OPEN checks), always chases (L+0x850 = 1 in its level
//! constructor) and keeps its level's ranges (no 0x70/0x74/0x78 reset).
use super::builds::*;
use super::*;
use crate::engine::*;
use crate::logic::LResult;

/// `*(uint *)(pLocalDataGhost_Base + off)`
#[inline]
fn lv(e: &Engine, off: u32) -> u32 {
    e.r32(L(e) + off)
}
/// `(*(uint *)(pLocalDataGhost_Base + 0x19c) & bit) != 0`
#[inline]
fn ev(e: &Engine, bit: u32) -> bool {
    lv(e, 0x19c) & bit != 0
}
/// `*(uint *)(pLocalDataGhost_Base + 0x19c) |= bit`
#[inline]
fn set_ev(e: &mut Engine, bit: u32) {
    let a = L(e) + 0x19c;
    or32(e, a, bit);
}
/// `*(int *)(pLocalDataGhost_Base + 0x1a0) = PlayAnim(t, anim, mode, flags)`
fn play(e: &mut Engine, t: f32, anim: u32, mode: i32, flags: i32) {
    let r = LIBAnimMngr::PlayAnim(e, t, anim as i32, mode, flags);
    let a = L(e) + 0x1a0;
    e.w32(a, r as u32);
}
fn timer_reset(e: &mut Engine, off: u32) {
    let t = lv(e, off);
    APIextra::TimerReset(e, t);
}
fn timer_freq(e: &mut Engine, off: u32, f: f32) {
    let t = lv(e, off);
    APIextra::TimerSetFrequency(e, t, f);
}
fn timer_reg(e: &mut Engine, off: u32) {
    let t = lv(e, off);
    APIextra::TimerRegisterNotify(e, SELF, t, 0x1d);
}
fn timer_unreg(e: &mut Engine, off: u32) {
    let t = lv(e, off);
    APIextra::TimerUnregisterNotify(e, SELF, t);
}
/// `TimerSetFrequency(L+off, (float)((int)MyRand() % m + a))` (the timer id is read before MyRand)
fn timer_freq_rand(e: &mut Engine, off: u32, m: i32, a: i32) {
    let t = lv(e, off);
    let r = APIstdlib::MyRand(e) as i32;
    APIextra::TimerSetFrequency(e, t, (r % m + a) as f32);
}
/// `SetNextState(SELF, s); (*GetExternalFunction(s))(0);`
fn go_now(e: &mut Engine, s: u32) -> LResult<()> {
    APIgameobj::SetNextState(e, SELF, s);
    call_external_now(e, s)
}

/// 0x000193c0 state 0xe
pub fn Ghost_BaseST_START(e: &mut Engine, _: u32) -> LResult<u64> {
    let s = e.cstr("GHOST");
    APIgameobj::DisplayInfo(e, 7, s, SELF, 0xf);
    Ghost_BaseTR_START_NONE_ROAMING_ON_PATH(e)?;
    Ghost_BaseFUNC_ResetEventFlag(e)?;
    Ok(0)
}

/// 0x00019448
pub fn Ghost_BaseTR_START_NONE_ROAMING_ON_PATH(e: &mut Engine) -> LResult<()> {
    let a = k(e, Ghost_BaseTR_START_NONE_ROAMING_ON_PATH_0_1);
    play(e, 0.1, a, 2, 0);
    let p = lv(e, 0x82c);
    CheckNearestPointOnPath(e, p)?;
    SetSneakSpeedParams(e)?;
    APIgameobj::SetContextFlag(e, SELF, 7);
    go_now(e, 0xf)
}

/// 0x00019540 state 0xf
/// (ghostbosslevel5: with L+0x850 set, straight to search mode instead)
pub fn Ghost_BaseST_ROAMING_ON_PATH(e: &mut Engine, _: u32) -> LResult<u64> {
    Waving(e)?;
    CheckPathFunction(e)?;
    LookOutForHeroFunction(e)?;
    TomeIsOpen(e)?;
    if g(e, G_Ghost_BaseST_ROAMING_ON_PATH) != 0 && lv(e, 0x850) != 0 {
        Ghost_BaseTR_WAITING_GOINGTO_SEARCH_MODE(e)?;
        return Ok(0);
    }
    if ev(e, 0x1000) {
        Ghost_BaseTR_HOOKED_BY_BOOK_SET(e)?;
        Ghost_BaseFUNC_ResetEventFlag(e)?;
    }
    if ev(e, 0x10000) {
        Ghost_BaseTR_TOME_IS_OPEN_SET(e)?;
        Ghost_BaseFUNC_ResetEventFlag(e)?;
    }
    if ev(e, 8) {
        Ghost_BaseTR_REACH_NEXTWP_ON_PATH(e)?;
        Ghost_BaseFUNC_ResetEventFlag(e)?;
    }
    if ev(e, 0x10) {
        Ghost_BaseTR_INLINEOFSIGHT(e)?;
        Ghost_BaseFUNC_ResetEventFlag(e)?;
    }
    if ev(e, 0x20) {
        Ghost_BaseTR_I_EAR_SOME_NOISE(e)?;
        Ghost_BaseFUNC_ResetEventFlag(e)?;
    }
    if ev(e, 0x40000) {
        let a = L(e) + 0x19c;
        let v = e.r32(a);
        e.w32(a, v & 0xfffb_ffff);
    }
    Ok(0)
}

/// 0x00019720
pub fn Ghost_BaseTR_REACH_NEXTWP_ON_PATH(e: &mut Engine) -> LResult<()> {
    if lv(e, 0x848) == 0 {
        let p = lv(e, 0x82c);
        WayPointTrack(e, 0, p)
    } else {
        let p = lv(e, 0x828);
        WayPointTrack(e, 1, p)
    }
}

/// 0x000197b0
pub fn Ghost_BaseTR_INLINEOFSIGHT(e: &mut Engine) -> LResult<()> {
    let l = L(e);
    e.w32(l + 0x840, 1);
    let a = k(e, Ghost_BaseTR_INLINEOFSIGHT_0_1);
    play(e, 0.2, a, 1, 0x1b);
    APIgameobj::SetContextFlag(e, SELF, 1);
    Ghost_BaseFUNC_ResetEventFlag(e)?;
    go_now(e, 0x10)
}

/// 0x00019898
pub fn Ghost_BaseTR_I_EAR_SOME_NOISE(e: &mut Engine) -> LResult<()> {
    let l = L(e);
    e.w32(l + 0x840, 2);
    SetPosOfNoise(e)?;
    let l = L(e);
    e.w32(l + 0x83c, 0);
    let a = k(e, Ghost_BaseTR_I_EAR_SOME_NOISE_0_1);
    play(e, 0.2, a, 1, 0x1b);
    APIgameobj::SetContextFlag(e, SELF, 1);
    go_now(e, 0x10)
}

/// 0x00019990 state 0x10
/// (ghostbosslevel5: no book-open check)
pub fn Ghost_BaseST_WAITING(e: &mut Engine, _: u32) -> LResult<u64> {
    let boss = g(e, G_Ghost_BaseST_WAITING) != 0;
    TomeIsOpen(e)?;
    if ev(e, 0x1000) {
        Ghost_BaseTR_HOOKED_BY_BOOK_SET(e)?;
        Ghost_BaseFUNC_ResetEventFlag(e)?;
    }
    if !boss && ev(e, k(e, Ghost_BaseST_WAITING_0_1)) {
        Ghost_BaseTR_TOME_IS_OPEN_SET(e)?;
        Ghost_BaseFUNC_ResetEventFlag(e)?;
    }
    if ev(e, 0x40) {
        Ghost_BaseTR_WAITING_GOINGTO_SEARCH_MODE(e)?;
        Ghost_BaseFUNC_ResetEventFlag(e)?;
    }
    Ok(0)
}

/// 0x00019a90
pub fn Ghost_BaseTR_WAITING_GOINGTO_SEARCH_MODE(e: &mut Engine) -> LResult<()> {
    if lv(e, 0x840) == 1 {
        SetTrackOnHero(e)?;
    } else if lv(e, 0x840) == 2 {
        SetPosOfNoise(e)?;
    }
    SetWalkSpeedParams(e)?;
    timer_freq_rand(e, 0x164, 2, 1);
    timer_reg(e, 0x164);
    let a = k(e, Ghost_BaseTR_WAITING_GOINGTO_SEARCH_MODE_0_1);
    play(e, 0.2, a, 1, 0x1b);
    APIgameobj::SetContextFlag(e, SELF, 7);
    go_now(e, 0x11)
}

/// 0x00019c58 state 0x11
/// (ghostbosslevel5: always "hero close" -> chase; only the hooked and chase checks)
pub fn Ghost_BaseST_SEARCH_MODE(e: &mut Engine, _: u32) -> LResult<u64> {
    Waving(e)?;
    SearchNoisePosFunction(e)?;
    LookOutForHeroFunction(e)?;
    TomeIsOpen(e)?;
    if g(e, G_Ghost_BaseST_SEARCH_MODE) != 0 {
        set_ev(e, 0x100);
        if ev(e, 0x1000) {
            Ghost_BaseTR_HOOKED_BY_BOOK_SET(e)?;
            Ghost_BaseFUNC_ResetEventFlag(e)?;
        }
        if ev(e, 0x100) {
            Ghost_BaseTR_SETTING_GHOST_FOR_CHASSE(e)?;
            Ghost_BaseFUNC_ResetEventFlag(e)?;
        }
        return Ok(0);
    }
    if ev(e, 0x1000) {
        Ghost_BaseTR_HOOKED_BY_BOOK_SET(e)?;
        Ghost_BaseFUNC_ResetEventFlag(e)?;
    }
    if ev(e, k(e, Ghost_BaseST_SEARCH_MODE_0_1)) {
        Ghost_BaseTR_TOME_IS_OPEN_SET(e)?;
        Ghost_BaseFUNC_ResetEventFlag(e)?;
    }
    if ev(e, 0x100) {
        Ghost_BaseTR_SETTING_GHOST_FOR_CHASSE(e)?;
        Ghost_BaseFUNC_ResetEventFlag(e)?;
    }
    if ev(e, 0x200) {
        Ghost_BaseTR_LOOKING_AROUND(e)?;
        Ghost_BaseFUNC_ResetEventFlag(e)?;
    }
    if ev(e, 0x80) {
        Ghost_BaseTR_NOISE_SOMEWHERE_ELSE(e)?;
        Ghost_BaseFUNC_ResetEventFlag(e)?;
    }
    if ev(e, 0x40000) {
        let l = L(e);
        e.w32(l + 0x858, 1);
        Ghost_BaseTR_LOOKING_AROUND(e)?;
        Ghost_BaseFUNC_ResetEventFlag(e)?;
    }
    Ok(0)
}

/// 0x00019e40
pub fn Ghost_BaseTR_NOISE_SOMEWHERE_ELSE(e: &mut Engine) -> LResult<()> {
    SetPosOfNoise(e)?;
    let l = L(e);
    e.w32(l + 0x83c, 0);
    Ok(())
}

/// 0x00019e90
pub fn Ghost_BaseTR_SETTING_GHOST_FOR_CHASSE(e: &mut Engine) -> LResult<()> {
    SetTrackOnHero(e)?;
    SetAttackSpeedParams(e)?;
    play(e, 0.1, 2, 2, 0);
    go_now(e, 0x12)
}

/// 0x00019f60 walk a star of 8 points around the noise position (L+0x110), 2 s per point; after
/// the 8th go back to the path.
pub fn Ghost_BaseTR_LOOKING_AROUND(e: &mut Engine) -> LResult<()> {
    let mut fStack_40: f32 = 0.0;
    let mut fStack_3c: f32 = 0.0;
    if lv(e, 0x858) != 0 {
        let l = L(e);
        e.w32(l + 0x858, 0);
        let hero = e.r32(e.r32(GDM(e)) + 8);
        let p = LM(e) + 0x110;
        APIgameobj::GetPosition(e, hero, ALL, p);
    }
    let b = e.alloca(0x80);
    let dm = lv(e, 4);
    APIphysic::DMGetParams(e, dm, b);
    if lv(e, 0x83c) == 8 {
        timer_unreg(e, 0x878);
        let l = L(e);
        e.w32(l + 0x83c, 0);
        e.w32(l + 0x844, 0x4000_0000);
        let p = lv(e, 0x82c);
        CheckNearestPointOnPath(e, p)?;
        go_now(e, 0xf)?;
    } else {
        let l = L(e);
        e.w32(l + 0x844, 0x3f80_0000);
        match lv(e, 0x83c) {
            0 => (fStack_40, fStack_3c) = (0.0, 1.5),
            1 => (fStack_40, fStack_3c) = (-1.2, 1.2),
            2 => (fStack_40, fStack_3c) = (-1.5, 0.0),
            3 => (fStack_40, fStack_3c) = (-1.2, -1.2),
            4 => (fStack_40, fStack_3c) = (0.0, -1.5),
            5 => (fStack_40, fStack_3c) = (1.2, -1.2),
            6 => (fStack_40, fStack_3c) = (1.5, 0.0),
            7 => (fStack_40, fStack_3c) = (1.2, 1.2),
            _ => {}
        }
        let x = e.rf(l + 0x110) + fStack_40;
        e.wf(l + 0x110, x);
        let z = e.rf(l + 0x118) + fStack_3c;
        e.wf(l + 0x118, z);
        for (o, s) in [(8, 0x110), (0xc, 0x114), (0x10, 0x118)] {
            let v = e.r32(l + s);
            e.w32(b + o, v);
        }
        let n = e.r32(l + 0x83c).wrapping_add(1);
        e.w32(l + 0x83c, n);
        e.w32(l + 0x844, 0x3f00_0000);
        e.w32(b, 0);
        e.w32(b + 0x14, 5);
        APIphysic::DMSetParams(e, dm, b);
        SetSneakSpeedParams(e)?;
        if lv(e, 0x1a0) != k(e, Ghost_BaseTR_LOOKING_AROUND_0_1) {
            let a = k(e, Ghost_BaseTR_LOOKING_AROUND_0_2);
            play(e, 0.1, a, 2, 0);
        }
        timer_reset(e, 0x878);
        timer_freq(e, 0x878, 2.0);
        timer_reg(e, 0x878);
    }
    Ok(())
}

/// 0x0001a3f0 state 0x12
/// (ghostbosslevel5: no book-open check)
pub fn Ghost_BaseST_CHASSING_HERO(e: &mut Engine, _: u32) -> LResult<u64> {
    let boss = g(e, G_Ghost_BaseST_CHASSING_HERO) != 0;
    Waving(e)?;
    LookOutForHeroFunction(e)?;
    SpecialCheckChassing(e)?;
    TomeIsOpen(e)?;
    if ev(e, 0x1000) {
        Ghost_BaseTR_HOOKED_BY_BOOK_SET(e)?;
        Ghost_BaseFUNC_ResetEventFlag(e)?;
    }
    if !boss && ev(e, 0x10000) {
        Ghost_BaseTR_TOME_IS_OPEN_SET(e)?;
        Ghost_BaseFUNC_ResetEventFlag(e)?;
    }
    if ev(e, 0x400) {
        Ghost_BaseTR_ATTACK_HERO(e)?;
        Ghost_BaseFUNC_ResetEventFlag(e)?;
    }
    if ev(e, 0x40000) {
        let l = L(e);
        e.w32(l + 0x858, 1);
        Ghost_BaseTR_LOOKING_AROUND(e)?;
        Ghost_BaseFUNC_ResetEventFlag(e)?;
    }
    Ok(0)
}

/// 0x0001a568
pub fn Ghost_BaseTR_ATTACK_HERO(e: &mut Engine) -> LResult<()> {
    ReSinc(e)?;
    SetAttackSpeedParams(e)?;
    timer_unreg(e, 0x878);
    APIgameobj::SetContextFlag(e, SELF, 0x20d);
    let dm = lv(e, 0x18);
    APIphysic::DMResetStateBits(e, dm, 1);
    let a = k(e, Ghost_BaseTR_ATTACK_HERO_0_1);
    play(e, 0.1, a, 1, 0x1b);
    let s = k(e, ID_40000017);
    APInewSound::Play(e, SELF, s, 0);
    timer_unreg(e, 0x168);
    go_now(e, 0x13)
}

/// 0x0001a6e0 state 0x13
pub fn Ghost_BaseST_ATTACK_HERO(e: &mut Engine, _: u32) -> LResult<u64> {
    LookForWalls(e)?;
    if ev(e, 0x2000) {
        Ghost_BaseTR_EVENT_PULLED_IN(e)?;
        Ghost_BaseFUNC_ResetEventFlag(e)?;
    }
    if ev(e, 0x1000) {
        Ghost_BaseTR_HOOKED_BY_BOOK_SET(e)?;
        Ghost_BaseFUNC_ResetEventFlag(e)?;
    }
    if ev(e, 0x4000) {
        Ghost_BaseTR_EVENT_LINK_BROKEN(e)?;
        Ghost_BaseFUNC_ResetEventFlag(e)?;
    }
    if ev(e, 0x800) {
        Ghost_BaseTR_ATTACK_LF_WHAT_DO_I_DO_KNOW(e)?;
        Ghost_BaseFUNC_ResetEventFlag(e)?;
    }
    Ok(0)
}

/// 0x0001a810 attack anim finished: still hooked -> back to being sucked, else wait 1 s
pub fn Ghost_BaseTR_ATTACK_LF_WHAT_DO_I_DO_KNOW(e: &mut Engine) -> LResult<()> {
    if lv(e, 0x84c) == 1 {
        timer_reset(e, 0x170);
        // ghostbosslevel5: 1.3
        let f = if g(e, G_Ghost_BaseTR_ATTACK_LF_WHAT_DO_I_DO_KNOW) == 0 { 4.0 } else { 1.3 };
        timer_freq(e, 0x170, f);
        timer_reg(e, 0x170);
        Ghost_BaseTR_HOOKED_BY_BOOK_SET(e)?;
    } else {
        let a = k(e, Ghost_BaseTR_ATTACK_LF_WHAT_DO_I_DO_KNOW_0_1);
        play(e, 0.2, a, 2, 0);
        timer_reset(e, 0x160);
        timer_freq(e, 0x160, 1.0);
        timer_reg(e, 0x160);
    }
    Ok(())
}

/// 0x0001a988 state 0x15: hooked by the book's beam, fleeing along the path while being pulled
pub fn Ghost_BaseST_BEINGSUCKED_LOOP(e: &mut Engine, _: u32) -> LResult<u64> {
    Waving(e)?;
    CheckPathFunction(e)?;
    if ev(e, 0x2000) {
        Ghost_BaseTR_EVENT_PULLED_IN(e)?;
        Ghost_BaseFUNC_ResetEventFlag(e)?;
    }
    if ev(e, 0x4000) {
        Ghost_BaseTR_EVENT_LINK_BROKEN(e)?;
        Ghost_BaseFUNC_ResetEventFlag(e)?;
    }
    if ev(e, 0x8000) {
        Ghost_BaseTR_EVENT_TURN_ON_HERO(e)?;
        Ghost_BaseFUNC_ResetEventFlag(e)?;
    }
    if ev(e, 8) {
        Ghost_BaseTR_REACH_NEXTWP_ON_PATH(e)?;
        Ghost_BaseFUNC_ResetEventFlag(e)?;
    }
    Ok(0)
}

/// 0x0001aac8 state 0x17: hooked ghost turns to face the book holder
pub fn Ghost_BaseST_TURN_ON_HERO_LOOP(e: &mut Engine, _: u32) -> LResult<u64> {
    HeroInHookAttackRange(e)?;
    let book = k(e, ID_2a000093);
    LIBApiCFunctions::TrackID(e, 0.0, SELF as i32, book);
    if ev(e, 0x2000) {
        Ghost_BaseTR_EVENT_PULLED_IN(e)?;
        Ghost_BaseFUNC_ResetEventFlag(e)?;
    }
    if ev(e, 0x400) {
        Ghost_BaseTR_ATTACK_HERO(e)?;
        Ghost_BaseFUNC_ResetEventFlag(e)?;
    }
    Ok(0)
}

/// 0x0001abb0 state 0x16 (ghostbosslevel5 has no book-open check here)
pub fn Ghost_BaseST_REGANING_CONFIDANCE(e: &mut Engine, _: u32) -> LResult<u64> {
    let boss = g(e, G_Ghost_BaseST_REGANING_CONFIDANCE) != 0;
    Waving(e)?;
    CheckPathFunction(e)?;
    TomeIsOpen(e)?;
    if ev(e, 0x1000) {
        Ghost_BaseTR_HOOKED_BY_BOOK_SET(e)?;
        Ghost_BaseFUNC_ResetEventFlag(e)?;
    }
    if !boss && ev(e, k(e, Ghost_BaseST_REGANING_CONFIDANCE_0_1)) {
        Ghost_BaseTR_TOME_IS_OPEN_SET(e)?;
        Ghost_BaseFUNC_ResetEventFlag(e)?;
    }
    if ev(e, 8) {
        Ghost_BaseTR_REACH_NEXTWP_ON_PATH(e)?;
        Ghost_BaseFUNC_ResetEventFlag(e)?;
    }
    Ok(0)
}

/// 0x0001acd0
pub fn Ghost_BaseTR_EVENT_TURN_ON_HERO(e: &mut Engine) -> LResult<()> {
    ReSinc(e)?;
    let b = e.alloca(0x80);
    let dm = lv(e, 4);
    APIphysic::DMGetParams(e, dm, b);
    let book = k(e, ID_2a000093);
    for (o, v) in [(0, book), (8, 0), (0xc, 0), (0x10, 0)] {
        e.w32(b + o, v);
    }
    APIphysic::DMSetParams(e, dm, b);
    SetWalkSpeedParams(e)?;
    play(e, 0.1, 2, 2, 0);
    APIgameobj::SetNextState(e, SELF, 0x17);
    Ok(())
}

/// 0x0001add8 the book's beam caught us: flee along path B (L+0x828), turn on the hero after 4 s
pub fn Ghost_BaseTR_HOOKED_BY_BOOK_SET(e: &mut Engine) -> LResult<()> {
    let l = L(e);
    e.w32(l + 0x84c, 1);
    e.w32(l + 0x848, 1);
    SetRunFromTomeSpeedParams(e)?;
    let p = lv(e, 0x828);
    CheckNearestPointOnPath(e, p)?;
    let a = k(e, Ghost_BaseTR_HOOKED_BY_BOOK_SET_0_1);
    play(e, 0.1, a, 2, 0);
    APIgameobj::SetContextFlag(e, SELF, 0x87);
    timer_unreg(e, 0x168);
    timer_unreg(e, 0x160);
    timer_reset(e, 0x170);
    // ghostbosslevel5: 2.5
    let f = if g(e, G_Ghost_BaseTR_HOOKED_BY_BOOK_SET) == 0 { 4.0 } else { 2.5 };
    timer_freq(e, 0x170, f);
    timer_reg(e, 0x170);
    Ghost_BaseFUNC_ResetEventFlag(e)?;
    APIgameobj::SetNextState(e, SELF, 0x15);
    Ok(())
}

/// 0x0001af90
pub fn Ghost_BaseTR_EVENT_PULLED_IN(e: &mut Engine) -> LResult<()> {
    SetRunFromTomeSpeedParams(e)?;
    let l = L(e);
    e.w32(l + 0x848, 0);
    timer_unreg(e, 0x168);
    timer_unreg(e, 0x170);
    timer_unreg(e, 0x164);
    Ok(())
}

/// 0x0001b048
pub fn Ghost_BaseTR_EVENT_LINK_BROKEN(e: &mut Engine) -> LResult<()> {
    let l = L(e);
    e.w32(l + 0x84c, 0);
    ReSinc(e)?;
    timer_reset(e, 0x168);
    // ghostbosslevel5: 1.0
    let f = if g(e, G_Ghost_BaseTR_EVENT_LINK_BROKEN) == 0 { 3.0 } else { 1.0 };
    timer_freq(e, 0x168, f);
    timer_reg(e, 0x168);
    timer_unreg(e, 0x170);
    APIgameobj::SetNextState(e, SELF, 0x16);
    Ok(())
}

/// 0x0001b148 state 0x18: run along path B while the book is open
pub fn Ghost_BaseST_TOME_IS_OPEN_FLEE(e: &mut Engine, _: u32) -> LResult<u64> {
    TomeIsOpen(e)?;
    CheckPathFunction(e)?;
    if ev(e, 0x1000) {
        Ghost_BaseTR_HOOKED_BY_BOOK_SET(e)?;
        Ghost_BaseFUNC_ResetEventFlag(e)?;
    }
    if ev(e, 0x20000) {
        Ghost_BaseTR_TOME_IS_CLOSE_SET(e)?;
        Ghost_BaseFUNC_ResetEventFlag(e)?;
    }
    if ev(e, 8) {
        Ghost_BaseTR_REACH_NEXTWP_ON_PATH(e)?;
        Ghost_BaseFUNC_ResetEventFlag(e)?;
    }
    Ok(0)
}

/// 0x0001b258
pub fn Ghost_BaseTR_TOME_IS_OPEN_SET(e: &mut Engine) -> LResult<()> {
    let l = L(e);
    e.w32(l + 0x848, 1);
    SetWalkSpeedParams(e)?;
    let p = lv(e, 0x828);
    CheckNearestPointOnPath(e, p)?;
    let a = k(e, Ghost_BaseTR_TOME_IS_OPEN_SET_0_1);
    play(e, 0.1, a, 2, 0);
    APIgameobj::SetContextFlag(e, SELF, 0x87);
    timer_unreg(e, 0x168);
    timer_unreg(e, 0x160);
    APIgameobj::SetNextState(e, SELF, 0x18);
    Ok(())
}

/// 0x0001b380
pub fn Ghost_BaseTR_TOME_IS_CLOSE_SET(e: &mut Engine) -> LResult<()> {
    timer_reset(e, 0x168);
    // ghostbosslevel5: 0.5
    let f = if g(e, G_Ghost_BaseTR_TOME_IS_CLOSE_SET) == 0 { 2.0 } else { 0.5 };
    timer_freq(e, 0x168, f);
    timer_reg(e, 0x168);
    APIgameobj::SetNextState(e, SELF, 0x16);
    Ok(())
}

/// 0x0001b438 state 8 (cut scene): keep the timers from firing
pub fn Monster_BaseST_IN_CUT_LOOP(e: &mut Engine, _: u32) -> LResult<u64> {
    for o in [0x170, 0x168, 0x164, 0x160] {
        timer_reset(e, o);
    }
    Ok(0)
}

/// 0x0001b4f0 state 0
pub fn Monster_BaseST_REINITIALISE(e: &mut Engine, _: u32) -> LResult<u64> {
    let s = k(e, Monster_BaseST_REINITIALISE_0_1);
    APIgameobj::SetNextState(e, SELF, s);
    Ok(0)
}

/// 0x0001b548 callback 0x1e: the wall DM (L+0x18) hit something (state 2): turn away from it
pub fn Ghost_BaseCB_NOTIFY_STATE_CHANGE(e: &mut Engine, _: u32) -> LResult<u64> {
    let b = e.alloca(0x80);
    let dm = lv(e, 0x18);
    APIphysic::DMGetParams(e, dm, b);
    if e.r32(b + 0x2c) as i32 == 2 {
        let up = e.alloca(0xc);
        e.wf(up, 0.0);
        e.wf(up + 4, 1.0);
        e.wf(up + 8, 0.0);
        let right = e.alloca(0xc);
        APIgameobj::GetLookRight(e, SELF, ALL, right);
        let mut f = APImath::CVectorGetAngleTo(e, b + 0x3c, right);
        if f < 1.5707964 {
            f = 3.1415927 - f;
        } else {
            f = (3.1415927 - f) * -1.0;
        }
        APIgameobj::Rotate(e, SELF, up, f);
    }
    Ok(0)
}

/// 0x0001b6a8 callback 0xc
pub fn Monster_BaseCB_NOTIFY_COLLIDE_OBJ(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

/// 0x0001b6f8 callback 0x1b: anim key-frame events (attack hit, idle variations, wait done).
/// MyRand is always drawn first (`(char)(r - r/2*2)`).
pub fn Ghost_BaseCB_ANIM_LF(e: &mut Engine, param_1: u32) -> LResult<u64> {
    let r = APIstdlib::MyRand(e) as i32;
    let cVar1 = r.wrapping_sub((r / 2).wrapping_mul(2)) as u8;
    let idle = |e: &mut Engine, a0: u32, a1: u32| {
        if cVar1 == 0 {
            play(e, 0.2, a0, 1, 0x1b);
        } else if cVar1 == 1 {
            play(e, 0.2, a1, 1, 0x1b);
        }
    };
    match g(e, G_Ghost_BaseCB_ANIM_LF) {
        0 => {
            if param_1 < k(e, Ghost_BaseCB_ANIM_LF_0_1) {
                if param_1 < k(e, Ghost_BaseCB_ANIM_LF_0_2) {
                    if param_1 == k(e, Ghost_BaseCB_ANIM_LF_0_3) {
                        set_ev(e, k(e, Ghost_BaseCB_ANIM_LF_0_4));
                    }
                } else if cVar1 as u32 == k(e, Ghost_BaseCB_ANIM_LF_0_5) {
                    play(e, 0.2, k(e, Ghost_BaseCB_ANIM_LF_0_6), 1, 0x1b);
                } else if cVar1 == 1 {
                    play(e, 0.2, k(e, Ghost_BaseCB_ANIM_LF_0_7), 1, 0x1b);
                }
            } else if param_1 == k(e, Ghost_BaseCB_ANIM_LF_0_8) {
                set_ev(e, k(e, Ghost_BaseCB_ANIM_LF_0_9));
            }
        }
        // ghostbosslevel5 (the random draw is unused)
        1 => {
            if param_1 == 6 {
                set_ev(e, 0x800);
            } else {
                if param_1 < 7 {
                    if param_1 != 4 {
                        return Ok(0);
                    }
                } else if param_1 != 0x29b {
                    if param_1 != 0x29c {
                        return Ok(0);
                    }
                    set_ev(e, 0x40);
                    return Ok(0);
                }
                play(e, 0.2, 4, 1, 0x1b);
            }
        }
        // medghostlevelfour, de27b0f0
        2 => {
            if param_1 == 7 {
                set_ev(e, 0x800);
            } else if param_1 < 8 {
                if param_1 < 6 && 3 < param_1 {
                    idle(e, 4, 5);
                }
            } else if param_1 == 8 {
                set_ev(e, 0x40);
            }
        }
        // 4324ff1c, d0275f8b
        _ => {
            if param_1 != 5 {
                if 5 < param_1 {
                    if param_1 == 6 {
                        set_ev(e, 0x800);
                        return Ok(0);
                    }
                    if param_1 != 7 {
                        return Ok(0);
                    }
                    set_ev(e, 0x40);
                    return Ok(0);
                }
                if param_1 != 1 {
                    return Ok(0);
                }
            }
            idle(e, 5, 1);
        }
    }
    Ok(0)
}

/// 0x0001b8a8 callback 0x1d: the five timers
pub fn Ghost_BaseCB_NOTIFY_TIMER(e: &mut Engine, param_1: u32) -> LResult<u64> {
    if param_1 == lv(e, 0x878) {
        set_ev(e, 0x200);
    }
    if param_1 == lv(e, 0x164) {
        let r = APIstdlib::MyRand(e) as i32;
        let iVar3 = r % 16;
        let s = if iVar3 < 0 || 7 < iVar3 { k(e, ID_40000014) } else { k(e, ID_40000015) };
        APInewSound::Play(e, SELF, s, 0);
        timer_reset(e, 0x164);
        timer_freq_rand(e, 0x164, 3, 2);
        timer_reg(e, 0x164);
    }
    if param_1 == lv(e, 0x168) {
        let l = L(e);
        e.w32(l + 0x848, 0);
        timer_unreg(e, 0x168);
        SetSneakSpeedParams(e)?;
        let p = lv(e, 0x82c);
        CheckNearestPointOnPath(e, p)?;
        let p = lv(e, 0x82c);
        WayPointTrack(e, 0, p)?;
        let a = k(e, Ghost_BaseCB_NOTIFY_TIMER_0_1);
        play(e, 0.1, a, 2, 0);
        APIgameobj::SetContextFlag(e, SELF, 7);
        go_now(e, 0xf)?;
    }
    if param_1 == lv(e, 0x160) {
        Monster_BaseCB_NOTIFY_TIMER(e, param_1)?;
        SetSneakSpeedParams(e)?;
        let p = lv(e, 0x82c);
        CheckNearestPointOnPath(e, p)?;
        let p = lv(e, 0x82c);
        WayPointTrack(e, 0, p)?;
        timer_unreg(e, 0x160);
        APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 3, 0xc);
        APIgameobj::SetContextFlag(e, SELF, 7);
        go_now(e, 0xf)?;
    }
    if param_1 == lv(e, 0x170) {
        set_ev(e, 0x8000);
        timer_unreg(e, 0x170);
    }
    Ok(0)
}

/// 0x0001bd38 ghost messages, then [`Monster_Base_MessageFunction`]. Shape group 1 (27 builds) also
/// plays the heroes' voice reactions when hooked / pulled in (0xd6/0x65, 0xd6/0x67).
pub fn Ghost_BaseFUNC_MessageHandler(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    let voices = g(e, G_Ghost_BaseFUNC_MessageHandler) != 0;
    if param_1 == 0xd6 {
        match param_2 {
            100 => set_ev(e, 0x4000),
            0x65 => {
                set_ev(e, 0x1000);
                if voices {
                    let (o, s) = if IsScoobyPullingAlone(e)? == 0 {
                        (k(e, Ghost_BaseFUNC_MessageHandler_1_1), k(e, Ghost_BaseFUNC_MessageHandler_1_2))
                    } else {
                        (k(e, Ghost_BaseFUNC_MessageHandler_1_3), k(e, Ghost_BaseFUNC_MessageHandler_1_4))
                    };
                    APInewSound::Play(e, o, s, 0);
                }
            }
            0x67 => {
                set_ev(e, 0x2000);
                SetRunFromTomeSpeedParams(e)?;
                let l = L(e);
                e.w32(l + 0x848, 0);
                timer_unreg(e, 0x168);
                timer_unreg(e, 0x170);
                timer_unreg(e, 0x164);
                let alone = IsScoobyPullingAlone(e)?;
                if voices {
                    let (o, s) = if alone == 0 {
                        (k(e, Ghost_BaseFUNC_MessageHandler_1_5), k(e, Ghost_BaseFUNC_MessageHandler_1_6))
                    } else {
                        (k(e, Ghost_BaseFUNC_MessageHandler_1_7), k(e, Ghost_BaseFUNC_MessageHandler_1_8))
                    };
                    APInewSound::Play(e, o, s, 0);
                }
            }
            0x68 => {
                for o in [0x164, 0x168, 0x170, 0x160] {
                    timer_unreg(e, o);
                }
                let l = L(e);
                e.w32(l + 0x84c, 0);
                DestroyGlow(e)?;
                let l = L(e);
                e.w32(l + 0x874, 0);
                let s = k(e, ID_40000013);
                APInewSound::StopSound(e, SELF, s);
                if lv(e, 0x838) != 0 {
                    let to = lv(e, 0x838);
                    APIgameobj::SendMessageTo(e, to, NULL, 0x68, 0, NULL);
                }
            }
            0x69 => {
                let s = k(e, ID_40000013);
                APInewSound::Play(e, SELF, s, 0);
            }
            _ => {}
        }
    } else if param_1 < 0xd7 {
        if param_1 == 0xc9 {
            if e.r32(e.r32(GDG(e) + 4) + 4) & lv(e, 0x40) == 0 {
                Ghost_Base_Func_Initialisation(e)?;
                timer_reset(e, 0x164);
                timer_freq_rand(e, 0x164, 3, 2);
                timer_reg(e, 0x164);
                let s = k(e, Ghost_BaseFUNC_MessageHandler_0_1);
                APIgameobj::SetNextState(e, SELF, s);
            } else {
                if lv(e, 0x874) == 0 {
                    CreateGlow(e)?;
                    let l = L(e);
                    e.w32(l + 0x874, 1);
                }
                Ghost_Base_Func_Initialisation(e)?;
                APIgameobj::Sleep(e, SELF, 0.0);
                let l = L(e);
                e.w32(l + 0x178, 1);
                e.w32(l + 0x188, 0);
                let p = lv(e, 0x82c);
                InitPath(e, p)?;
                SetSneakSpeedParams(e)?;
                APIgameobj::SetContextFlag(e, SELF, 7);
                let a = k(e, Ghost_BaseFUNC_MessageHandler_0_2) as i32;
                LIBAnimMngr::PlayAnim(e, 0.0, a, 2, 0);
            }
        } else if param_1 < 0xca {
            if param_1 == 0xc1 {
                Ghost_Base_Func_Initialisation(e)?;
                let s = k(e, Ghost_BaseFUNC_MessageHandler_0_3);
                APIgameobj::SetNextState(e, SELF, s);
            }
        } else if param_1 == 0xd4 && lv(e, 0x838) != 0 {
            let to = lv(e, 0x838);
            APIgameobj::SendMessageTo(e, to, NULL, 0xd4, 0, NULL);
        }
    } else if param_1 == 0x201 {
        APIgameobj::EnableObject(e, SELF, 1);
        StartGhostEmetter(e)?;
    } else if param_1 < 0x202 {
        if param_1 == 0x1b3 {
            let d = e.alloca(0xc);
            APIgameobj::GetPosition(e, param_2, ALL, d);
            let p = L(e) + 0xf8;
            APIgameobj::GetPosition(e, SELF, ALL, p);
            let l = L(e);
            let y = e.r32(d + 4);
            e.w32(l + 0xfc, y);
            e.w32(l + 0x838, param_2);
        }
    } else if param_1 == 0x202 {
        APIgameobj::EnableObject(e, SELF, 0);
        StopGhostEmetter(e)?;
    }
    Monster_Base_MessageFunction(e, param_1, param_2, param_3)
}

/// 0x0001c488 DMs: L+0 type 7, L+4 type 1 (move to target), L+8 type 4 (turn), L+0x18 type 0x1b
/// (wall probe from bone 3, notifies state changes through callback 0x1e)
pub fn Ghost_BaseFUNC_Create_Physic(e: &mut Engine) -> LResult<()> {
    APIphysic::CreatePhysic(e, SELF, 0);
    APIphysic::SetFlags(e, SELF, 6);
    let dm_with = |e: &mut Engine, slot: u32, ty: u32, words: &[(u32, u32)], mask: u32| -> u32 {
        let id = APIphysic::DMAdd(e, SELF, ty);
        let l = L(e);
        e.w32(l + slot, id);
        let b = e.alloca(0x80);
        APIphysic::DMGetParams(e, id, b);
        for &(o, v) in words {
            e.w32(b + o, v);
        }
        APIphysic::DMSetParams(e, id, b);
        APIphysic::DMSetContextMask(e, id, mask);
        id
    };
    dm_with(e, 0, 7, &[(0, 0x41a0_0000), (4, 7)], 1);
    dm_with(e, 4, 1, &[(0, 0), (4, 0x40c0_0000), (8, 0), (0xc, 0), (0x10, 0), (0x14, 5)], 2);
    dm_with(e, 8, 4, &[(0, 0), (4, 0), (8, 0x40c0_0000), (0xc, 1)], 4);
    let id = APIphysic::DMAdd(e, SELF, 0x1b);
    let l = L(e);
    e.w32(l + 0x18, id);
    let b = e.alloca(0x80);
    APIphysic::DMGetParams(e, id, b);
    let off = bone3_offset(e);
    for i in 0..3 {
        let v = e.r32(off + i * 4);
        e.w32(b + i * 4, v);
    }
    for (o, v) in [(0xc, 0), (0x10, 0), (0x14, 0x3f59_999a), (0x18, 0), (0x1c, 0), (0x20, 0x1000), (0x24, 0), (0x28, 0)] {
        e.w32(b + o, v);
    }
    APIphysic::DMSetParams(e, id, b);
    APIphysic::DMSetContextMask(e, id, 8);
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 6, 0x1e);
    APIphysic::DMSetStateChangeNotifyBits(e, id, 1);
    Ok(())
}

/// bone 3 position minus the object position, in object space (Create_Physic / LookForWalls)
fn bone3_offset(e: &mut Engine) -> Ptr {
    let d50 = e.alloca(0xc);
    let d40 = e.alloca(0xc);
    let d30 = e.alloca(0xc);
    APIgameobj::GetPosition(e, SELF, 3, d50);
    APIgameobj::GetPosition(e, SELF, ALL, d40);
    for i in 0..3 {
        let v = e.rf(d50 + i * 4) - e.rf(d40 + i * 4);
        e.wf(d30 + i * 4, v);
    }
    APIgameobj::InvertTransformVector(e, SELF, d30, d30);
    d30
}

/// 0x0001c8c8 reset the AI (ranges 3 / 4.5 / 8, flags, group, idle-sound timer, home position)
/// (ghostbosslevel5: the ranges and L+0x850 are left alone)
pub fn Ghost_Base_Func_Initialisation(e: &mut Engine) -> LResult<()> {
    let l = L(e);
    if g(e, G_Ghost_Base_Func_Initialisation) == 0 {
        for (o, v) in [(0x70, 0x4040_0000), (0x74, 0x4090_0000), (0x78, 0x4100_0000), (0x850, 0)] {
            e.w32(l + o, v);
        }
    }
    for (o, v) in [
        (0x840, 0), (0x848, 0),
        (0x844, 0x4000_0000), (0x838, 0), (0x83c, 0), (0x820, 0), (0x84c, 0), (0x178, 0), (0x198, 1), (0x188, 0),
    ] {
        e.w32(l + o, v);
    }
    let v = e.r32(l + 0xb0);
    e.w32(l + 0x5c, v);
    let m = if lv(e, 0x44) == 0x13 { 0x30_0203 } else { 0x20_0603 };
    APIgameobj::SetGroupBitMask(e, SELF, m);
    APIgameobj::SetGroupID(e, SELF, 1);
    APIgameobj::SetContextFlag(e, SELF, 7);
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 3, 0xc);
    timer_reset(e, 0x164);
    timer_freq_rand(e, 0x164, 3, 2);
    timer_reg(e, 0x164);
    let p = L(e) + 0xf8;
    APIgameobj::SetTranslation(e, SELF, p);
    let l = L(e);
    e.w32(l + 0x858, 0);
    Ok(())
}

/// 0x0001cb88 event 0x10000 (book open with power) or 0x20000
pub fn TomeIsOpen(e: &mut Engine) -> LResult<()> {
    if IsTomeOpened(e)? == 0 {
        set_ev(e, 0x20000);
    } else {
        set_ev(e, 0x10000);
    }
    Ok(())
}

/// 0x0001cc18 the book holder is within 4 units: event 0x400
pub fn HeroInHookAttackRange(e: &mut Engine) -> LResult<()> {
    let book = k(e, ID_2a000093);
    if LIBApiCFunctions::CheckRangeObject(e, 4.0, SELF, book, 0) != 0 {
        set_ev(e, 0x400);
    }
    Ok(())
}

/// 0x0001cc90 aim the wall probe DM (L+0x18) from bone 3
pub fn LookForWalls(e: &mut Engine) -> LResult<()> {
    let off = bone3_offset(e);
    let b = e.alloca(0x80);
    let dm = lv(e, 0x18);
    APIphysic::DMGetParams(e, dm, b);
    for i in 0..3 {
        let v = e.r32(off + i * 4);
        e.w32(b + i * 4, v);
    }
    APIphysic::DMSetParams(e, dm, b);
    Ok(())
}

/// 0x0001cdc0
pub fn LookOutForHeroFunction(e: &mut Engine) -> LResult<()> {
    if lv(e, 0x198) == 0 {
        return Ok(());
    }
    let flags = |e: &Engine| e.r32(e.r32(GDG(e)) + 0x14);
    let sector = L(e) + 0x834;
    let iVar2 = CheckIfInSector(e, sector)?;
    if iVar2 == 0 || flags(e) & 0x40 != 0 {
        if flags(e) & 0x40 != 0 || flags(e) & 0x800 != 0 || {
            let sector = L(e) + 0x834;
            CheckIfInSector(e, sector)? == 0
        } {
            set_ev(e, 0x40000);
        }
    } else {
        let p = L(e) + 0xb8;
        if LIBApiCFunctions::InLineOfSight(e, p) != 0 {
            let hero = e.r32(e.r32(GDM(e)) + 8);
            let r = e.rf(L(e) + 0x74);
            if LIBApiCFunctions::CheckRangeObject(e, r, SELF, hero, 0) != 0 {
                set_ev(e, 0x100);
            } else {
                set_ev(e, 0x10);
            }
            let hero = e.r32(e.r32(GDM(e)) + 8);
            let r = e.rf(L(e) + 0x70);
            if LIBApiCFunctions::CheckRangeObject(e, r, SELF, hero, 0) != 0 {
                set_ev(e, 0x400);
            }
        }
        if DoIEarSomeNoise(e)? != 0 {
            set_ev(e, 0x20);
        }
    }
    Ok(())
}

/// 0x0001d018
pub fn SpecialCheckChassing(e: &mut Engine) -> LResult<()> {
    let hero = e.r32(e.r32(GDM(e)) + 8);
    let r = e.rf(L(e) + 0x70);
    if LIBApiCFunctions::CheckRangeObject(e, r, SELF, hero, 0) != 0 {
        set_ev(e, 0x400);
    }
    Ok(())
}

/// 0x0001d0a0 within 1.5 of the current waypoint (L+0x17c): event 8
pub fn CheckPathFunction(e: &mut Engine) -> LResult<()> {
    let p = L(e) + 0x17c;
    if LIBApiCFunctions::CheckRangePoint(e, 1.5, SELF, p, 0) != 0 {
        set_ev(e, 8);
    }
    Ok(())
}

/// 0x0001d120
pub fn SearchNoisePosFunction(e: &mut Engine) -> LResult<()> {
    let sector = L(e) + 0x834;
    if CheckIfInSector(e, sector)? != 0 && DoIEarSomeNoise(e)? != 0 {
        set_ev(e, 0x80);
    }
    let r = e.rf(L(e) + 0x844);
    let p = LM(e) + 0x110;
    if LIBApiCFunctions::CheckRangePoint(e, r, SELF, p, 0) != 0 {
        set_ev(e, 0x200);
    }
    Ok(())
}

/// 0x0001d208 move DM -> the leader hero, speed 6
pub fn SetTrackOnHero(e: &mut Engine) -> LResult<()> {
    let l = L(e);
    e.w32(l + 0x840, 1);
    let b = e.alloca(0x80);
    let dm = e.r32(LM(e) + 4);
    APIphysic::DMGetParams(e, dm, b);
    let hero = e.r32(e.r32(GDM(e)) + 8);
    for (o, v) in [(4, 0x40c0_0000), (0, hero), (8, 0), (0xc, 0), (0x10, 0), (0x14, 5)] {
        e.w32(b + o, v);
    }
    APIphysic::DMSetParams(e, dm, b);
    Ok(())
}

/// 0x0001d2c0 back to the floating height L+0xfc
pub fn ReSinc(e: &mut Engine) -> LResult<()> {
    let d = e.alloca(0xc);
    for i in 0..3 {
        e.wf(d + i * 4, 0.0);
    }
    APIgameobj::GetPosition(e, SELF, ALL, d);
    let y = e.r32(L(e) + 0xfc);
    e.w32(d + 4, y);
    APIgameobj::SetTranslation(e, SELF, d);
    Ok(())
}

/// 0x0001d348 start path `param_1` at its waypoint 0
pub fn InitPath(e: &mut Engine, param_1: u32) -> LResult<()> {
    let n = APIextra::WaypointGetNb(e, param_1);
    let l = L(e);
    e.w32(l + 0x174, n);
    APIextra::WaypointGetPos(e, param_1, 0, l + 0x17c);
    set_move_target(e, None);
    Ok(())
}

/// move DM (L+4) target = no object, position L+0x17c [, mode]
fn set_move_target(e: &mut Engine, mode: Option<u32>) {
    let b = e.alloca(0x80);
    let dm = lv(e, 4);
    APIphysic::DMGetParams(e, dm, b);
    e.w32(b, 0);
    let l = L(e);
    for (o, s) in [(8, 0x17c), (0xc, 0x180), (0x10, 0x184)] {
        let v = e.r32(l + s);
        e.w32(b + o, v);
    }
    if let Some(m) = mode {
        e.w32(b + 0x14, m);
    }
    APIphysic::DMSetParams(e, dm, b);
}

/// 0x0001d440 the floating bob: phase L+0x86c moves by L+0x870 between -1 and 1 (random speed at
/// each end), height = L+0xfc +/- (1 - phase^2) * 0.15
pub fn Waving(e: &mut Engine) -> LResult<u32> {
    let l = L(e);
    let x = e.rf(l + 0x86c) + e.rf(l + 0x870);
    e.wf(l + 0x86c, x);
    if 1.0 <= e.rf(l + 0x86c) {
        e.w32(l + 0x86c, 0x3f80_0000);
        let r = APIstdlib::MyRand(e) as i32;
        let v = -(((r as f32) / 32768.0) * 0.1 + 0.05);
        e.wf(L(e) + 0x870, v);
    } else if e.rf(l + 0x86c) <= -1.0 {
        e.w32(l + 0x86c, 0xbf80_0000);
        let r = APIstdlib::MyRand(e) as i32;
        let v = ((r as f32) / 32768.0) * 0.1 + 0.05;
        e.wf(L(e) + 0x870, v);
    }
    let l = L(e);
    let ph = e.rf(l + 0x86c);
    let fStack_40 = (1.0 - ph * ph) * 0.15;
    let d = e.alloca(0xc);
    APIgameobj::GetPosition(e, SELF, ALL, d);
    let l = L(e);
    let y = if 0.0 < e.rf(l + 0x870) { e.rf(l + 0xfc) + fStack_40 } else { e.rf(l + 0xfc) - fStack_40 };
    e.wf(d + 4, y);
    Ok(APIgameobj::SetTranslation(e, SELF, d))
}

/// 0x0001d6a0 nearest waypoint of path `param_1` (in xz) becomes the target
pub fn CheckNearestPointOnPath(e: &mut Engine, param_1: u32) -> LResult<()> {
    let mut fStack_2c: f32 = 1e6;
    let me = e.alloca(0xc);
    let wp = e.alloca(0xc);
    APIgameobj::GetPosition(e, SELF, ALL, me);
    let n = APIextra::WaypointGetNb(e, param_1);
    let l = L(e);
    e.w32(l + 0x174, n);
    let mut i = 0u32;
    while i < lv(e, 0x174) {
        APIextra::WaypointGetPos(e, param_1, i, wp);
        let dx = e.rf(me) - e.rf(wp);
        let dz = e.rf(me + 8) - e.rf(wp + 8);
        let d = APImath::powf(e, dx * dx + dz * dz, 0.5);
        if d <= fStack_2c {
            let l = L(e);
            for o in 0..3 {
                let v = e.r32(wp + o * 4);
                e.w32(l + 0x17c + o * 4, v);
            }
            e.w32(l + 0x178, i);
            fStack_2c = d;
        }
        i += 1;
    }
    set_move_target(e, None);
    Ok(())
}

/// 0x0001d8f8 advance to the next waypoint. param_1 0 = loop (path A), 1 = ping-pong (path B);
/// L+0x188 = direction.
pub fn WayPointTrack(e: &mut Engine, param_1: i32, param_2: u32) -> LResult<()> {
    let l = L(e);
    if param_1 == 0 {
        if lv(e, 0x178) == lv(e, 0x174) {
            e.w32(l + 0x178, 0);
        } else if lv(e, 0x178) == 0 {
            let v = lv(e, 0x174).wrapping_sub(1);
            e.w32(l + 0x178, v);
        }
    } else if param_1 == 1 {
        if lv(e, 0x178) == lv(e, 0x174) {
            ChangeDirectionOnPath(e)?;
        } else if lv(e, 0x178) == 0 {
            e.w32(l + 0x178, 1);
            ChangeDirectionOnPath(e)?;
        }
    }
    if lv(e, 0x188) == 0 {
        let i = lv(e, 0x178);
        APIextra::WaypointGetPos(e, param_2, i, l + 0x17c);
        set_move_target(e, Some(5));
        let v = lv(e, 0x178).wrapping_add(1);
        e.w32(l + 0x178, v);
    }
    if lv(e, 0x188) == 1 {
        let v = lv(e, 0x178).wrapping_sub(1);
        e.w32(l + 0x178, v);
        APIextra::WaypointGetPos(e, param_2, v, l + 0x17c);
        set_move_target(e, Some(5));
    }
    Ok(())
}

/// 0x0001dc18
pub fn ChangeDirectionOnPath(e: &mut Engine) -> LResult<()> {
    let l = L(e);
    if lv(e, 0x188) == 0 {
        e.w32(l + 0x188, 1);
    } else if lv(e, 0x188) == 1 {
        e.w32(l + 0x188, 0);
    }
    Ok(())
}

/// 0x0001dc88
pub fn Ghost_BaseFUNC_ResetEventFlag(e: &mut Engine) -> LResult<()> {
    let l = L(e);
    e.w32(l + 0x19c, 0);
    Ok(())
}

/// 0x0001dcb8 Shaggy leads and the team-pull flags (game buffer 0 +0x34 & 0x101) are set
pub fn IsScoobyPullingAlone(e: &mut Engine) -> LResult<u32> {
    let gb0 = e.r32(GDG(e));
    if e.r32(gb0 + 8) == k(e, ID_2a000091) {
        Ok(0)
    } else if e.r32(gb0 + 0x34) & 0x101 == 0 {
        Ok(0)
    } else {
        Ok(1)
    }
}

/// 0x0001dd48
pub fn ghost_baseConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    monster_baseConstructor(e, api, instanceCount)?;
    if instanceCount == 1 {
        let fns: [(u32, crate::logic::ExtFn); 13] = [
            (0xe, Ghost_BaseST_START),
            (0xf, Ghost_BaseST_ROAMING_ON_PATH),
            (0x10, Ghost_BaseST_WAITING),
            (0x11, Ghost_BaseST_SEARCH_MODE),
            (0x12, Ghost_BaseST_CHASSING_HERO),
            (0x13, Ghost_BaseST_ATTACK_HERO),
            (0x15, Ghost_BaseST_BEINGSUCKED_LOOP),
            (0x16, Ghost_BaseST_REGANING_CONFIDANCE),
            (0x17, Ghost_BaseST_TURN_ON_HERO_LOOP),
            (0x18, Ghost_BaseST_TOME_IS_OPEN_FLEE),
            (0x1b, Ghost_BaseCB_ANIM_LF),
            (0x1d, Ghost_BaseCB_NOTIFY_TIMER),
            (0x1e, Ghost_BaseCB_NOTIFY_STATE_CHANGE),
        ];
        for (i, f) in fns {
            APIcons::SetExternalFunction(e, i as i32, f);
        }
        let tables: [(u32, fn(&mut Engine) -> Ptr); 10] = [
            (pGameObj, |e| APIcons::GetApiGameObj(e)),
            (pExtra, |e| APIcons::GetApiExtra(e)),
            (pPhysic, |e| APIcons::GetApiPhysic(e)),
            (pStdlib, |e| APIcons::GetApiStdlib(e)),
            (pParticle, |e| APIcons::GetApiParticle(e)),
            (pMath, |e| APIcons::GetApiMath(e)),
            (pLight, |e| APIcons::GetApiLight(e)),
            (pGraphic, |e| APIcons::GetApiGraphics(e)),
            (pAnim, |e| APIcons::GetApiAnim(e)),
            (pSound, |e| APIcons::GetApiNewSound(e)),
        ];
        for (slot, get) in tables {
            let v = get(e);
            let a = G(e, slot);
            e.w32(a, v);
        }
        let n = e.cstr("ApiSectorClipping");
        let v = APIcons::GetApiCustom(e, n);
        let a = G(e, pSectorClip);
        e.w32(a, v);
        let slot = G(e, pGlobalDataGhost_Base);
        APIcons::GetGlobalData(e, slot, 0x14);
        let uVar1 = APIgameobj::GetGameBuffer(e, 0, 0x8f8, 1);
        let gd = GDG(e);
        e.w32(gd, uVar1);
        let uVar1 = APIgameobj::GetGameBuffer(e, 2, 0x158, 1);
        let gd = GDG(e);
        e.w32(gd + 4, uVar1);
        e.w32(gd + 0xc, api);
    }
    let slot = G(e, pLocalDataGhost_Base);
    APIcons::GetLocalData(e, slot, 0x884);
    let id = APIgameobj::GetID(e, SELF, NULL);
    // match our id in goCustomDataGhost (record size 0x14, 0x18 in 4 builds): paths, kill mask, sector
    let last = k(e, ghost_baseConstructor_0_1) as i32;
    let rs = k(e, ghost_baseConstructor_0_2);
    let mut i: i32 = 0;
    loop {
        if last < i {
            break;
        }
        let r = G(e, goCustomDataGhost) + i as u32 * rs;
        if e.r32(r) == id {
            let l = L(e);
            for (d, s) in [(0x828, 4), (0x82c, 8), (0x40, 0xc), (0x834, k(e, ghost_baseConstructor_0_7))] {
                let v = e.r32(r + s);
                e.w32(l + d, v);
            }
            break;
        }
        i += 1;
    }
    if AmIDead(e)? == 1 {
        APIgameobj::KillObject(e, SELF);
    }
    for o in [0x160, 0x164, 0x168, 0x170, 0x878] {
        let t = APIextra::TimerCreate(e);
        let l = L(e);
        e.w32(l + o, t);
    }
    Ghost_BaseFUNC_Create_Physic(e)?;
    let p = lv(e, 0x82c);
    InitPath(e, p)?;
    APIphysic::PrimSetIntAttribBits(e, SELF, 600, 5, 0x1000);
    APIphysic::PrimResetIntAttribBits(e, SELF, 600, 0, 2);
    APIphysic::PrimSetIntAttribBits(e, SELF, 600, 0, 0x80);
    APIcons::SetMessageFunction(e, Ghost_BaseFUNC_MessageHandler);
    let id = APIgameobj::GetID(e, SELF, NULL);
    let l = L(e);
    e.w32(l + 0x860, id);
    e.w16(l + 0x86a, 2);
    e.w16(l + 0x868, 0);
    LIBApiSectorClipping::ApiImpl_Register(e, l + 0x860);
    let l = L(e);
    e.w32(l + 0x870, 0x3dcc_cccd);
    Ok(())
}

/// 0x0001e3f8
pub fn ghost_baseDestructor(e: &mut Engine) -> LResult<()> {
    monster_baseDestructor(e)?;
    let p = L(e) + 0x860;
    LIBApiSectorClipping::ApiImpl_Unregister(e, p);
    for o in [0x160, 0x164, 0x168, 0x170, 0x878] {
        let t = lv(e, o);
        APIextra::TimerDestroy(e, t);
    }
    Ok(())
}

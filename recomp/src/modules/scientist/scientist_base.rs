//! Scientist_Base.c (46 functions): the scientist's behaviour. Addresses are module-relative (.text at
//! 0x10000) in scientist__c66ebe98 (955aa35f: same code shifted, see the build notes per function).
//!
//! Local data (one block with Monster_Base, 0x284 bytes): +0 / +4 / +8 / +0x20 / +0x24 the DMs (type 7,
//! track 1, speed 4, 8, gravity 0), +0x160 sub-state code pointer ([`addr`]), +0x164 the conditions
//! GameEventSignaler evaluates (bit 2 sight, 4 hearing, 8 attack range, 0x10 path, 0x20 point reached,
//! 0x40 stand time-out, 0x80 detection lost), +0x168..0x170 / +0x1a0 / +0x1c0 record fields, +0x174 our
//! id, +0x178 events (2 spotted, 4 lost sight, 8 heard, 0x10 not heard, 0x20 in reach, 0x40 out of
//! reach, 0x80 path end, 0x100 stood long enough, 0x200 hero moved, 0x400 cannot detect, 0x800 anim
//! ended, 0x1000 msg 0x173/0x174, 0x2000 point reached, 0x4000 scared, 0x8000 wake up), +0x17c the
//! weapon latent, +0x180 sub-step, +0x184 / +0x188 / +0x18c / +400 / +0x194 timers (stand, attack,
//! scared 7 s, hero-moved, spotting), +0x198 / +0x19c waypoint index / count, +0x1a4 waypoint set,
//! +0x1a8 saved position, +0x1b4 target point, +0x1c8 / +0x1cc / +0x1d0 / +0x1d4 flags (puzzle done,
//! skip spotting, can detect, start hidden), +0x1d8 path name, +0x214 has a path-finding path, +0x218
//! current path node, +0x224 path length, +0x228..0x23c force-field data (Scientist.c), +0x240 floor
//! mask hiding the hero, +0x244 clipping save pointer (never set: NULL, as on the PS2), +0x248 path
//! block, +600 ray block.
use super::builds::*;
use super::*;
use crate::engine::*;
use crate::logic::LResult;

/// Module-relative addresses (c66ebe98) of the sub-state functions stored at L+0x160.
pub mod addr {
    pub const scientist_baseST_First_Move: u32 = 0x131d8;
    pub const scientist_baseST_Stand_In: u32 = 0x13238;
    pub const scientist_baseST_Stand: u32 = 0x13300;
    pub const scientist_baseST_HearingSound_In: u32 = 0x13478;
    pub const scientist_baseST_HearingSound: u32 = 0x13558;
    pub const scientist_baseST_FollowSound_In: u32 = 0x13620;
    pub const scientist_baseST_FollowSound: u32 = 0x13700;
    pub const scientist_baseST_Spotting_In: u32 = 0x13948;
    pub const scientist_baseST_Spotting: u32 = 0x13ab0;
    pub const scientist_baseST_FollowHero_In: u32 = 0x13bd8;
    pub const scientist_baseST_FollowHero: u32 = 0x13c98;
    pub const scientist_baseST_Attack_In: u32 = 0x13d60;
    pub const scientist_baseST_Attack: u32 = 0x13e08;
    pub const scientist_baseST_WalkOnPath_In: u32 = 0x13fd8;
    pub const scientist_baseST_WalkOnPath: u32 = 0x140e0;
    pub const scientist_baseST_Scared_In: u32 = 0x14350;
    pub const scientist_baseST_Scared: u32 = 0x14470;
    pub const scientist_baseST_Sleep_In: u32 = 0x14540;
    pub const scientist_baseST_Sleep: u32 = 0x145c8;
    // Scientist.c (c66ebe98 only)
    pub const scientist_localST_GoToHero_In: u32 = 0x180f8;
    pub const scientist_localST_GoToHero: u32 = 0x18260;
    pub const scientist_localST_HearingNoise_In: u32 = 0x18340;
    pub const scientist_localST_HearingNoise: u32 = 0x183e0;
}

/// `(**(code **)(L + 0x160))()`: run the current sub-state.
pub fn call_state(e: &mut Engine) -> LResult<()> {
    use addr::*;
    let f = e.r32(L(e) + 0x160);
    match f {
        scientist_baseST_First_Move => self::scientist_baseST_First_Move(e),
        scientist_baseST_Stand_In => self::scientist_baseST_Stand_In(e),
        scientist_baseST_Stand => self::scientist_baseST_Stand(e),
        scientist_baseST_HearingSound_In => self::scientist_baseST_HearingSound_In(e),
        scientist_baseST_HearingSound => self::scientist_baseST_HearingSound(e),
        scientist_baseST_FollowSound_In => self::scientist_baseST_FollowSound_In(e),
        scientist_baseST_FollowSound => self::scientist_baseST_FollowSound(e),
        scientist_baseST_Spotting_In => self::scientist_baseST_Spotting_In(e),
        scientist_baseST_Spotting => self::scientist_baseST_Spotting(e),
        scientist_baseST_FollowHero_In => self::scientist_baseST_FollowHero_In(e),
        scientist_baseST_FollowHero => self::scientist_baseST_FollowHero(e),
        scientist_baseST_Attack_In => self::scientist_baseST_Attack_In(e),
        scientist_baseST_Attack => self::scientist_baseST_Attack(e),
        scientist_baseST_WalkOnPath_In => self::scientist_baseST_WalkOnPath_In(e),
        scientist_baseST_WalkOnPath => self::scientist_baseST_WalkOnPath(e),
        scientist_baseST_Scared_In => self::scientist_baseST_Scared_In(e),
        scientist_baseST_Scared => self::scientist_baseST_Scared(e),
        scientist_baseST_Sleep_In => self::scientist_baseST_Sleep_In(e),
        scientist_baseST_Sleep => self::scientist_baseST_Sleep(e),
        scientist_localST_GoToHero_In => super::scientist::scientist_localST_GoToHero_In(e),
        scientist_localST_GoToHero => super::scientist::scientist_localST_GoToHero(e),
        scientist_localST_HearingNoise_In => super::scientist::scientist_localST_HearingNoise_In(e),
        scientist_localST_HearingNoise => super::scientist::scientist_localST_HearingNoise(e),
        // the PS2 would jump through whatever L+0x160 holds: a fault
        _ => panic!("scientist: sub-state code pointer {f:#x} is not a known function"),
    }
}

/// `*(code **)(L + 0x160) = f`
#[inline]
pub(crate) fn set_state(e: &mut Engine, f: u32) {
    let l = L(e);
    e.w32(l + 0x160, f);
}
/// `*(code **)(L + 0x160) = f; (**(code **)(L + 0x160))();`
pub(crate) fn go(e: &mut Engine, f: u32) -> LResult<()> {
    set_state(e, f);
    call_state(e)
}
#[inline]
fn ev(e: &Engine) -> u32 {
    e.r32(L(e) + 0x178)
}
#[inline]
fn clr(e: &mut Engine, bits: u32) {
    let a = L(e) + 0x178;
    and32(e, a, !bits);
}
#[inline]
fn set(e: &mut Engine, bits: u32) {
    let a = L(e) + 0x178;
    or32(e, a, bits);
}
/// the leader hero (game buffer 0 +8)
#[inline]
pub(crate) fn hero(e: &Engine) -> u32 {
    e.r32(e.r32(GDM(e)) + 8)
}
/// the game flags (game buffer 0 +0x14)
#[inline]
pub(crate) fn game_flags(e: &Engine) -> u32 {
    e.r32(e.r32(GDS(e)) + 0x14)
}
/// `if (L+0x178 & bit) { L+0x178 &= ~bit; L+0x160 = f; return true }`
fn take(e: &mut Engine, bit: u32, f: u32) -> bool {
    if ev(e) & bit != 0 {
        clr(e, bit);
        set_state(e, f);
        true
    } else {
        false
    }
}

/// 0x00012f50 external 0xe (initial state): mask 0x800401, the weapon latent (told 0x169 + our id),
/// the path, clipping; start hidden (L+0x1d4): mask 0, HUD-ish object told 0xeb 1, state 5; else
/// state 0xf.
pub fn scientist_baseST_INITIAL(e: &mut Engine, _: u32) -> LResult<u64> {
    APIgameobj::SetGroupBitMask(e, SELF, 0x800401);
    let uVar3 = APIgameobj::LaunchLatant(e, k(e, ID_2a800109));
    e.w32(L(e) + 0x17c, uVar3);
    let iVar2 = L(e);
    let uVar4 = APIgameobj::GetID(e, SELF, NULL);
    let to = e.r32(iVar2 + 0x17c);
    APIgameobj::SendMessageTo(e, to, NULL, 0x169, uVar4, NULL);
    if e.r32(L(e) + 0x214) != 0 {
        let l = L(e);
        LIBApiPathFinding::SetPath(e, (l + 0x1d8) as i32, (l + 0x248) as i32);
    }
    Scientist_BaseFUNC_SetClippingScientist(e)?;
    if e.r32(L(e) + 0x1d4) == 0 {
        APIgameobj::SetNextState(e, SELF, 0xf);
    } else {
        APIgameobj::SetGroupBitMask(e, SELF, 0);
        APIgameobj::SendMessageTo(e, k(e, ID_2a0000ce), NULL, 0xeb, 1, NULL);
        APIgameobj::SetNextState(e, SELF, 5);
    }
    Ok(0)
}

/// 0x00013120 external 0xf: evaluate the conditions, scared event -> Scared_In, run the sub-state
pub fn scientist_baseST_ENTRY_POINT(e: &mut Engine, _: u32) -> LResult<u64> {
    scientist_baseFUNC_GameEventSignaler(e)?;
    take(e, 0x4000, addr::scientist_baseST_Scared_In);
    call_state(e)?;
    Ok(0)
}

/// 0x000131d8
pub fn scientist_baseST_First_Move(e: &mut Engine) -> LResult<()> {
    go(e, addr::scientist_baseST_Stand_In)
}

/// 0x00013238 stand: anim 0xd (loop), stand timer reset, watch sight / hearing / reach / time-out
pub fn scientist_baseST_Stand_In(e: &mut Engine) -> LResult<()> {
    e.w32(L(e) + 0x178, 0);
    APIgameobj::SetContextFlag(e, SELF, 0x60);
    LIBAnimMngr::PlayAnim(e, 0.25, 0xd, 2, 0);
    let t = e.r32(L(e) + 0x184);
    APIextra::TimerReset(e, t);
    scientist_baseFUNC_SetCondition(e, 0x4e)?;
    set_state(e, addr::scientist_baseST_Stand);
    Ok(())
}

/// 0x00013300 in reach -> Attack, spotted -> Spotting, heard -> HearingSound, stood 5 s -> WalkOnPath
pub fn scientist_baseST_Stand(e: &mut Engine) -> LResult<()> {
    use addr::*;
    let _ = take(e, 0x20, scientist_baseST_Attack_In)
        || take(e, 2, scientist_baseST_Spotting_In)
        || take(e, 8, scientist_baseST_HearingSound_In)
        || take(e, 0x100, scientist_baseST_WalkOnPath_In);
    Ok(())
}

/// 0x00013478 anim 0x13 (once, end -> ext 0x12), remember the hero's position as the noise
pub fn scientist_baseST_HearingSound_In(e: &mut Engine) -> LResult<()> {
    e.w32(L(e) + 0x178, 0);
    APIgameobj::SetContextFlag(e, SELF, 0x60);
    LIBAnimMngr::PlayAnim(e, 0.25, 0x13, 1, 0x12);
    scientist_baseFUNC_SetCondition(e, 8)?;
    let h = hero(e);
    let p = L(e) + 0x1b4;
    APIgameobj::GetPosition(e, h, ALL, p);
    set_state(e, addr::scientist_baseST_HearingSound);
    Ok(())
}

/// 0x00013558 in reach -> Attack, anim ended -> FollowSound
pub fn scientist_baseST_HearingSound(e: &mut Engine) -> LResult<()> {
    use addr::*;
    let _ = take(e, 0x20, scientist_baseST_Attack_In) || take(e, 0x800, scientist_baseST_FollowSound_In);
    Ok(())
}

/// 0x00013620 walk (anim 0x10 loop) to the noise along the path graph
pub fn scientist_baseST_FollowSound_In(e: &mut Engine) -> LResult<()> {
    e.w32(L(e) + 0x178, 0);
    APIgameobj::SetContextFlag(e, SELF, 0x67);
    scientist_baseFUNC_SetSpeed(e, 0xb)?;
    LIBAnimMngr::PlayAnim(e, 0.25, 0x10, 2, 0);
    let p = L(e) + 0x1b4;
    Scientist_BaseFUNC_SetPathFinding(e, p)?;
    scientist_baseFUNC_SetCondition(e, 0x9e)?;
    set_state(e, addr::scientist_baseST_FollowSound);
    Ok(())
}

/// 0x00013700 detection lost (event or floor) -> Stand; in reach -> Attack; spotted -> Spotting;
/// hero moved -> re-path to him; path end -> Stand
pub fn scientist_baseST_FollowSound(e: &mut Engine) -> LResult<()> {
    use addr::*;
    let can = if ev(e) & 0x400 == 0 { Scientist_BaseFUNC_CanDetectHero_OnFloor(e)? } else { 0 };
    if ev(e) & 0x400 == 0 && can != 0 {
        if take(e, 0x20, scientist_baseST_Attack_In) || take(e, 2, scientist_baseST_Spotting_In) {
        } else if ev(e) & 0x200 != 0 {
            clr(e, 0x200);
            let h = hero(e);
            let p = L(e) + 0x1b4;
            APIgameobj::GetPosition(e, h, ALL, p);
            Scientist_BaseFUNC_SetPathFinding(e, p)?;
        } else {
            take(e, 0x80, scientist_baseST_Stand_In);
        }
    } else {
        clr(e, 0x400);
        set_state(e, scientist_baseST_Stand_In);
    }
    Ok(())
}

/// 0x00013948 face the hero, anim 0x17 (once); the "spotted" anim pair plays at most every 10 s
/// (spotting timer) unless L+0x1cc asks for it, else straight to FollowHero
pub fn scientist_baseST_Spotting_In(e: &mut Engine) -> LResult<()> {
    e.w32(L(e) + 0x178, 0);
    APIgameobj::SetContextFlag(e, SELF, 0x62);
    Scientist_BaseFUNC_TrackHero(e, 0x4040_0000)?;
    LIBAnimMngr::PlayAnim(e, 0.25, 0x17, 1, 0x12);
    scientist_baseFUNC_SetCondition(e, 0)?;
    let t = e.r32(L(e) + 0x194);
    let fVar1 = APIextra::TimerGetTime(e, t);
    if 10.0 <= fVar1 || e.r32(L(e) + 0x1cc) != 0 {
        let t = e.r32(L(e) + 0x194);
        APIextra::TimerReset(e, t);
        set_state(e, addr::scientist_baseST_Spotting);
    } else {
        set_state(e, addr::scientist_baseST_FollowHero_In);
    }
    e.w32(L(e) + 0x1cc, 0);
    e.w32(L(e) + 0x180, 0);
    Ok(())
}

/// 0x00013ab0 anim 0x17 ended -> anim 0x16 (once); that ended -> FollowHero
pub fn scientist_baseST_Spotting(e: &mut Engine) -> LResult<()> {
    let step = e.r32(L(e) + 0x180);
    if step == 0 {
        if ev(e) & 0x800 != 0 {
            clr(e, 0x800);
            LIBAnimMngr::PlayAnim(e, 0.25, 0x16, 1, 0x12);
            e.w32(L(e) + 0x180, 1);
        }
    } else if step == 1 {
        take(e, 0x800, addr::scientist_baseST_FollowHero_In);
    }
    Ok(())
}

/// 0x00013bd8 run (anim 6 loop) at the hero
pub fn scientist_baseST_FollowHero_In(e: &mut Engine) -> LResult<()> {
    e.w32(L(e) + 0x178, 0);
    APIgameobj::SetContextFlag(e, SELF, 0x67);
    scientist_baseFUNC_SetSpeed(e, 0x6f)?;
    LIBAnimMngr::PlayAnim(e, 0.25, 6, 2, 0);
    scientist_baseFUNC_SetCondition(e, 10)?;
    set_state(e, addr::scientist_baseST_FollowHero);
    Ok(())
}

/// 0x00013c98 in reach -> Attack, lost sight -> Stand
pub fn scientist_baseST_FollowHero(e: &mut Engine) -> LResult<()> {
    use addr::*;
    let _ = take(e, 0x20, scientist_baseST_Attack_In) || take(e, 4, scientist_baseST_Stand_In);
    Ok(())
}

/// 0x00013d60 face the hero, watch the reach
pub fn scientist_baseST_Attack_In(e: &mut Engine) -> LResult<()> {
    e.w32(L(e) + 0x178, 0);
    APIgameobj::SetContextFlag(e, SELF, 0x62);
    Scientist_BaseFUNC_TrackHero(e, 0x4040_0000)?;
    scientist_baseFUNC_SetCondition(e, 10)?;
    e.w32(L(e) + 0x180, 0);
    set_state(e, addr::scientist_baseST_Attack);
    Ok(())
}

/// 0x00013e08 out of reach -> FollowHero; every 4.5 s (attack timer, never reset here) throw: anim 0xb
/// (once), the weapon latent in hand (joint 0x2401000) told 0xdc; the anim's end re-arms
pub fn scientist_baseST_Attack(e: &mut Engine) -> LResult<()> {
    let step = e.r32(L(e) + 0x180);
    if step == 0 {
        if ev(e) & 0x40 == 0 {
            let t = e.r32(L(e) + 0x188);
            let fVar1 = APIextra::TimerGetTime(e, t);
            if 4.5 <= fVar1 {
                LIBAnimMngr::PlayAnim(e, 0.25, 0xb, 1, 0x12);
                let w = e.r32(L(e) + 0x17c);
                APIgameobj::AddChild(e, SELF, 0x240_1000, w, 1);
                let w = e.r32(L(e) + 0x17c);
                APIgameobj::SendMessageTo(e, w, NULL, 0xdc, 0, NULL);
                e.w32(L(e) + 0x180, 1);
            }
        } else {
            clr(e, 0x40);
            set_state(e, addr::scientist_baseST_FollowHero_In);
        }
    } else if step == 1 && ev(e) & 0x800 != 0 {
        clr(e, 0x800);
        e.w32(L(e) + 0x180, 0);
    }
    Ok(())
}

/// 0x00013fd8 walk (anim 1 loop) to the nearest waypoint of the set
pub fn scientist_baseST_WalkOnPath_In(e: &mut Engine) -> LResult<()> {
    e.w32(L(e) + 0x178, 0);
    APIgameobj::SetContextFlag(e, SELF, 0x67);
    scientist_baseFUNC_SetSpeed(e, 0xb)?;
    LIBAnimMngr::PlayAnim(e, 0.25, 1, 2, 0);
    scientist_baseFUNC_SetCondition(e, 0x1e)?;
    let w = e.r32(L(e) + 0x1a4);
    Scientist_BaseFUNC_FindNearestPointOnPath(e, w)?;
    let p = L(e) + 0x1b4;
    Scientist_BaseFUNC_SetPathFinding(e, p)?;
    e.w32(L(e) + 0x180, 0);
    set_state(e, addr::scientist_baseST_WalkOnPath);
    Ok(())
}

/// 0x000140e0 in reach / spotted / heard leave the walk (the walk code below still runs this frame);
/// path end -> walk the waypoints directly (condition 0x2e), each one reached -> the next
pub fn scientist_baseST_WalkOnPath(e: &mut Engine) -> LResult<()> {
    use addr::*;
    let _ = take(e, 0x20, scientist_baseST_Attack_In)
        || take(e, 2, scientist_baseST_Spotting_In)
        || take(e, 8, scientist_baseST_HearingSound_In);
    let step = e.r32(L(e) + 0x180);
    if step == 0 {
        if ev(e) & 0x80 != 0 {
            clr(e, 0x80);
            scientist_baseFUNC_SetCondition(e, 0x2e)?;
            let p = L(e) + 0x1b4;
            Scientist_BaseFUNC_TrackPos(e, 0x40a0_0000, p)?;
            e.w32(L(e) + 0x180, 1);
        }
    } else if step == 1 && ev(e) & 0x2000 != 0 {
        clr(e, 0x2000);
        let w = e.r32(L(e) + 0x1a4);
        Scientist_BaseFUNC_FindNextPointOnPath(e, w)?;
        let p = L(e) + 0x1b4;
        Scientist_BaseFUNC_TrackPos(e, 0x40a0_0000, p)?;
    }
    Ok(())
}

/// 0x00014350 scared: the 7 s timer notifies ext 0x11 (a no-op), anim 10 (once), the weapon told 0xda
pub fn scientist_baseST_Scared_In(e: &mut Engine) -> LResult<()> {
    e.w32(L(e) + 0x178, 0);
    let t = e.r32(L(e) + 0x18c);
    APIextra::TimerRegisterNotify(e, SELF, t, 0x11);
    let t = e.r32(L(e) + 0x18c);
    APIextra::TimerReset(e, t);
    APIgameobj::SetContextFlag(e, SELF, 0x60);
    LIBAnimMngr::PlayAnim(e, 0.2, 10, 1, 0x12);
    let w = e.r32(L(e) + 0x17c);
    if w != 0 {
        APIgameobj::SendMessageTo(e, w, NULL, 0xda, 0, NULL);
    }
    set_state(e, addr::scientist_baseST_Scared);
    Ok(())
}

/// 0x00014470 anim ended: Sleep (directly, not Sleep_In), anim 0xe (loop), snore sound 0x40, tell
/// k(0x2a0001a2) 0x1d0. NOTE (PS2 behaviour kept): the 0x800 event is not cleared.
pub fn scientist_baseST_Scared(e: &mut Engine) -> LResult<()> {
    if ev(e) & 0x800 != 0 {
        set_state(e, addr::scientist_baseST_Sleep);
        LIBAnimMngr::PlayAnim(e, 0.3, 0xe, 2, 0);
        APInewSound::Play(e, SELF, 0x40, 0);
        APIgameobj::SendMessageTo(e, k(e, ID_2a0001a2), NULL, 0x1d0, 0, NULL);
    }
    Ok(())
}

/// 0x00014540 anim 0xc (loop)
pub fn scientist_baseST_Sleep_In(e: &mut Engine) -> LResult<()> {
    APIgameobj::SetContextFlag(e, SELF, 0x60);
    LIBAnimMngr::PlayAnim(e, 0.3, 0xc, 2, 0);
    set_state(e, addr::scientist_baseST_Sleep);
    Ok(())
}

/// 0x000145c8 c66ebe98: wake event 0x8000 -> Stand. 955aa35f: empty (never wakes).
pub fn scientist_baseST_Sleep(e: &mut Engine) -> LResult<()> {
    if g(e, G_scientist_baseST_Sleep) == 0 && ev(e) & 0x8000 != 0 {
        set_state(e, addr::scientist_baseST_Stand_In);
        clr(e, 0x8000);
    }
    Ok(())
}

/// 0x00014638 external 0x11
pub fn scientist_baseCB_TIMER(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

/// 0x00014668 external 0x12: AnimMngr anim ended
pub fn scientist_baseCB_ANIM_LF(e: &mut Engine, _: u32) -> LResult<u64> {
    set(e, 0x800);
    Ok(0)
}

/// 0x000146b8 external 0x13: key frame {0xb, 1} of the throw releases the weapon (told 0xd8)
pub fn scientist_baseCB_ANIMATION(e: &mut Engine, param_1: u32) -> LResult<u64> {
    if e.r32(param_1 + 4) == 1 && e.r32(param_1) == 0xb {
        APIgameobj::RemoveAllChildren(e, SELF, 0);
        let w = e.r32(L(e) + 0x17c);
        APIgameobj::SendMessageTo(e, w, NULL, 0xd8, 0, NULL);
    }
    Ok(0)
}

/// 0x00014770 external 0x14: cut-scene start: save the clipping into *(L+0x244), cut-scene mode,
/// state 5; end: restore, Stand, state 0xf.
/// NOTE (PS2 behaviour kept): L+0x244 is never written, so the clipping goes through a NULL pointer.
pub fn scientist_baseCB_CUT_SCENE(e: &mut Engine, param_1: u32) -> LResult<u64> {
    let w = e.r32(param_1 + 4);
    if w == 1 {
        let p = e.r32(L(e) + 0x244);
        APIgameobj::GetClippingParams(e, SELF, p);
        let uVar2 = APIgameobj::GetID(e, SELF, NULL);
        LIBMiscFunc::MiscFunc_InitForCutscene(e, uVar2);
        APIgameobj::SetNextState(e, SELF, 5);
    } else if w == 0 {
        let uVar2 = APIgameobj::GetID(e, SELF, NULL);
        let p = e.r32(L(e) + 0x244);
        LIBMiscFunc::MiscFunc_EndCutscene(e, uVar2, p);
        set_state(e, addr::scientist_baseST_Stand_In);
        APIgameobj::SetNextState(e, SELF, 0xf);
    }
    Ok(0)
}

/// 0x000148c8 speed DM (L+8): 0xb = walk (L+0x98), else run (L+0x9c)
pub fn scientist_baseFUNC_SetSpeed(e: &mut Engine, param_1: i32) -> LResult<()> {
    let b = e.alloca(0x40);
    let dm = e.r32(L(e) + 8);
    APIphysic::DMGetParams(e, dm, b);
    let speed = e.r32(L(e) + if param_1 == 0xb { 0x98 } else { 0x9c });
    e.w32(b, 0);
    e.w32(b + 4, 0);
    e.w32(b + 8, speed);
    e.w32(b + 0xc, 1);
    let dm = e.r32(L(e) + 8);
    APIphysic::DMSetParams(e, dm, b);
    Ok(())
}

/// 0x000149e0
pub fn scientist_baseFUNC_SetCondition(e: &mut Engine, param_1: u32) -> LResult<()> {
    let a = L(e) + 0x164;
    e.w32(a, 0);
    or32(e, a, param_1);
    Ok(())
}

/// 0x00014a30 clear line from `param_2` (raised by `param_1`) horizontally towards `param_3`? The ray
/// block `param_4` = {origin (y + param_1), direction (dx, 0, dz)}; true when nothing is hit.
pub fn Scientist_baseFUNC_RayIntersect(e: &mut Engine, param_1: f32, param_2: Ptr, param_3: Ptr, param_4: Ptr) -> LResult<bool> {
    let (x0, y0, z0) = (e.rf(param_2), e.rf(param_2 + 4), e.rf(param_2 + 8));
    let (x1, z1) = (e.rf(param_3), e.rf(param_3 + 8));
    e.wf(param_4, x0);
    e.wf(param_4 + 4, y0 + param_1);
    e.wf(param_4 + 8, z0);
    e.wf(param_4 + 0xc, x1 - x0);
    e.wf(param_4 + 0x10, 0.0);
    e.wf(param_4 + 0x14, z1 - z0);
    let out = e.alloca(0x40);
    let iVar2 = APIphysic::CheckRayIntersection(e, param_4, out);
    Ok(iVar2 == 0)
}

/// 0x00014b98 turn the active conditions (L+0x164) into events (L+0x178). The hero only counts on
/// a detectable floor and within 0.5 m of our height.
pub fn scientist_baseFUNC_GameEventSignaler(e: &mut Engine) -> LResult<()> {
    let mut iStack_60 = Scientist_BaseFUNC_CanDetectHero_OnFloor(e)?;
    let h = hero(e);
    let fVar3 = LIBApiCFunctions::GetDistance(e, SELF as i32, h, ALL as i32, 0);
    let DStack_50 = e.alloca(0x10);
    let DStack_40 = e.alloca(0x10);
    APIgameobj::GetPosition(e, SELF, ALL, DStack_50);
    let h = hero(e);
    APIgameobj::GetPosition(e, h, ALL, DStack_40);
    let dy = e.rf(DStack_50 + 4) - e.rf(DStack_40 + 4);
    if 0.25 < dy * dy {
        iStack_60 = 0;
    }
    let cond = |e: &Engine| e.r32(L(e) + 0x164);
    if cond(e) & 4 != 0 {
        if iStack_60 == 0 || game_flags(e) & 0x4900_097c != 0 || 10.0 < fVar3 || e.r32(L(e) + 0x214) == 0 {
            set(e, 0x10);
            clr(e, 8);
        } else {
            set(e, 8);
            clr(e, 0x10);
            if Scientist_BaseFUNC_HeroMoved(e)? {
                set(e, 0x200);
            }
        }
    }
    if cond(e) & 2 != 0 {
        let seen = iStack_60 != 0 && game_flags(e) & 0x4900_094c == 0 && fVar3 <= 15.0 && {
            let ray = L(e) + 600;
            Scientist_baseFUNC_RayIntersect(e, 0.35, DStack_50, DStack_40, ray)?
        };
        if seen {
            set(e, 2);
            clr(e, 4);
        } else {
            set(e, 4);
            clr(e, 2);
        }
    }
    if cond(e) & 8 != 0 {
        let reach = iStack_60 != 0 && game_flags(e) & 0x4900_094c == 0 && fVar3 <= 4.0 && {
            let ray = L(e) + 600;
            Scientist_baseFUNC_RayIntersect(e, 1.6, DStack_50, DStack_40, ray)?
        };
        if reach {
            set(e, 0x20);
            clr(e, 0x40);
        } else {
            set(e, 0x40);
            clr(e, 0x20);
        }
    }
    if cond(e) & 0x10 != 0 && Scientist_BaseFUNC_UpdatePathFinding(e)? == 2 {
        set(e, 0x80);
    }
    if cond(e) & 0x20 != 0 {
        let p = L(e) + 0x1b4;
        if LIBApiCFunctions::CheckRangePoint(e, 1.3, SELF, p, 0) != 0 {
            set(e, 0x2000);
        }
    }
    if cond(e) & 0x40 != 0 && e.r32(L(e) + 0x1a4) != 0 {
        let t = e.r32(L(e) + 0x184);
        if 5.0 <= APIextra::TimerGetTime(e, t) {
            set(e, 0x100);
        }
    }
    if cond(e) & 0x80 != 0 && e.r32(L(e) + 0x1d0) == 0 {
        set(e, 0x400);
    }
    Ok(())
}

/// 0x000151b0 walk speed record value, line of sight, capture class 2 (LM+0xc4), group 2, mask
/// 0x600, the first waypoint, key-frame notifies -> ext 0x13, sub-state First_Move
pub fn scientist_base_Func_Initialisation(e: &mut Engine) -> LResult<()> {
    let iVar1 = L(e);
    let v = e.r32(iVar1 + 0xb0);
    e.w32(iVar1 + 0x5c, v);
    InitInLineOfSight(e)?;
    e.w32(LM(e) + 0xc4, 2);
    APIgameobj::SetGroupID(e, SELF, 2);
    APIgameobj::SetGroupBitMask(e, SELF, 0x600);
    APIgameobj::SetContextFlag(e, SELF, 0x60);
    e.w32(iVar1 + 0x198, 0);
    if e.r32(L(e) + 0x1a4) != 0 {
        let w = e.r32(iVar1 + 0x1a4);
        let uVar2 = APIextra::WaypointGetNb(e, w);
        e.w32(iVar1 + 0x19c, uVar2);
        let i = e.r32(L(e) + 0x198);
        let p = L(e) + 0x1b4;
        APIextra::WaypointGetPos(e, w, i, p);
    }
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 0xb, 0x13);
    set_state(e, addr::scientist_baseST_First_Move);
    Ok(())
}

/// the sub-states during which L09's 0xd8 does not scare
fn is_scared_or_asleep(e: &Engine) -> bool {
    use addr::*;
    matches!(
        e.r32(L(e) + 0x160),
        scientist_baseST_Scared_In | scientist_baseST_Scared | scientist_baseST_Sleep_In | scientist_baseST_Sleep
    )
}

/// 0x00015320 0xe6 mask 0x600 + state 5; 0xc9 weapon told 0xda; 199 sleep 1 s; 0xcd re-target the
/// track DM at the new leader; 0xde state 0xf; 0x173 / 0x174 event 0x1000; 0x1cf (`param_2` ->
/// L+0x170) stop the snore, weapon 0xda, Stand now, save the position; 0x1d0 Sleep now.
/// 955aa35f also: 0xd8 (water) scares unless scared / asleep (event 0x4000), 0x197 with `param_2`
/// forwards 0x197 to k(0x2a00024f) (ID slot 7). Then always Monster_Base_MessageFunction.
pub fn scientist_baseMessageFonction(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    let iVar1 = L(e);
    let l09 = g(e, G_scientist_baseMessageFonction) == 1;
    match param_1 {
        0xe6 => {
            APIgameobj::SetGroupBitMask(e, SELF, 0x600);
            APIgameobj::SetNextState(e, SELF, 5);
        }
        0xc9 => {
            let w = e.r32(L(e) + 0x17c);
            if w != 0 {
                APIgameobj::SendMessageTo(e, w, NULL, 0xda, 0, NULL);
            }
        }
        199 => {
            APIgameobj::Sleep(e, SELF, 1.0);
        }
        0xd8 if l09 => {
            if !is_scared_or_asleep(e) {
                set(e, 0x4000);
            }
        }
        0xcd => {
            let aiStack_60 = e.alloca(0x40);
            let iStack_40 = L(e);
            let dm = e.r32(iStack_40 + 4);
            APIphysic::DMGetParams(e, dm, aiStack_60);
            if e.r32(aiStack_60) != 0 {
                let h = hero(e);
                e.w32(aiStack_60, h);
            }
            APIphysic::DMSetParams(e, dm, aiStack_60);
        }
        0xde => {
            APIgameobj::SetNextState(e, SELF, 0xf);
        }
        0x197 if l09 => {
            if param_2 != 0 {
                // the build's 8th id slot (0x2a0001a2 in c66ebe98, 0x2a00024f in 955aa35f)
                APIgameobj::SendMessageTo(e, k(e, ID_2a0001a2), NULL, 0x197, 0, NULL);
            }
        }
        0x173 | 0x174 => set(e, 0x1000),
        0x1cf => {
            e.w32(L(e) + 0x170, param_2);
            APInewSound::StopSound(e, SELF, 0x40);
            let w = e.r32(iVar1 + 0x17c);
            if w != 0 {
                APIgameobj::SendMessageTo(e, w, NULL, 0xda, 0, NULL);
            }
            set_state(e, addr::scientist_baseST_Stand_In);
            APIgameobj::GetPosition(e, SELF, ALL, iVar1 + 0x1a8);
            call_state(e)?;
        }
        0x1d0 => {
            go(e, addr::scientist_baseST_Sleep_In)?;
        }
        _ => {}
    }
    Monster_Base_MessageFunction(e, param_1, param_2, param_3)
}

/// 0x00015738 external 0 (Monster_Base reinitialise): walk speed, waypoint count, index 0, Stand now,
/// state 0xf
pub fn Monster_BaseST_REINITIALISE(e: &mut Engine, _: u32) -> LResult<u64> {
    let v = e.r32(L(e) + 0xb0);
    e.w32(L(e) + 0x5c, v);
    if e.r32(L(e) + 0x1a4) != 0 {
        let w = e.r32(L(e) + 0x1a4);
        let uVar1 = APIextra::WaypointGetNb(e, w);
        e.w32(L(e) + 0x19c, uVar1);
    }
    e.w32(L(e) + 0x198, 0);
    go(e, addr::scientist_baseST_Stand_In)?;
    APIgameobj::SetNextState(e, SELF, 0xf);
    Ok(0)
}

/// 0x00015820 external 8
pub fn Monster_BaseST_IN_CUT_LOOP(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

/// 0x00015850 external 0xc
pub fn Monster_BaseCB_NOTIFY_COLLIDE_OBJ(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

/// add a DM of `kind` to SELF, run `fill` on its parameter block, set its context mask
fn dm_add(e: &mut Engine, kind: u32, mask: u32, fill: impl FnOnce(&mut Engine, Ptr)) -> u32 {
    let dm = APIphysic::DMAdd(e, SELF, kind);
    let b = e.alloca(0x40);
    APIphysic::DMGetParams(e, dm, b);
    fill(e, b);
    APIphysic::DMSetParams(e, dm, b);
    APIphysic::DMSetContextMask(e, dm, mask);
    dm
}

/// 0x000158e0 physics (flags 6, prim 0x3ed, weight 0.5, priority 10) and the DMs: L+4 track (type 1,
/// the hero, 3, mode 5, context 2), L+8 speed (4, walk speed, context 4), L+0 (7: 20, 5, context 1),
/// L+0x24 gravity (0: (0, -20, 0), context 0x40), L+0x20 (8: 175, 45, context 0x20); block L+0x270 =
/// {2, 0, 8, 0, our id}
pub fn scientist_baseFUNC_Create_Physic(e: &mut Engine) -> LResult<()> {
    let p = L(e);
    APIphysic::CreatePhysic(e, SELF, 0);
    APIphysic::SetFlags(e, SELF, 6);
    APIphysic::SetPrimaryPrim(e, SELF, 0x3ed);
    APIphysic::SetWeight(e, SELF, 0.5);
    APIphysic::SetPriority(e, SELF, 10);
    let h = hero(e);
    let dm = dm_add(e, 1, 2, |e, b| {
        for (o, v) in [(0, h), (4, 0x4040_0000), (8, 0), (0xc, 0), (0x10, 0), (0x14, 5)] {
            e.w32(b + o, v);
        }
    });
    e.w32(p + 4, dm);
    let walk = e.r32(p + 0x98);
    let dm = dm_add(e, 4, 4, |e, b| {
        for (o, v) in [(0, 0), (4, 0), (8, walk), (0xc, 1)] {
            e.w32(b + o, v);
        }
    });
    e.w32(p + 8, dm);
    let dm = dm_add(e, 7, 1, |e, b| {
        e.w32(b, 0x41a0_0000);
        e.w32(b + 4, 5);
    });
    e.w32(p, dm);
    let dm = dm_add(e, 0, 0x40, |e, b| {
        for (o, v) in [(0, 0), (4, 0xc1a0_0000), (8, 0)] {
            e.w32(b + o, v);
        }
    });
    e.w32(p + 0x24, dm);
    let dm = dm_add(e, 8, 0x20, |e, b| {
        e.w32(b, 0x432f_0000);
        e.w32(b + 4, 0x4234_0000);
    });
    e.w32(p + 0x20, dm);
    for (o, v) in [(0x270, 2), (0x274, 0), (0x278, 8), (0x27c, 0)] {
        e.w32(p + o, v);
    }
    let id = APIgameobj::GetID(e, SELF, NULL);
    e.w32(p + 0x280, id);
    Ok(())
}

/// 0x00015d18 head for `param_1`: straight (no path-finding path: the node = the point, result 2) or
/// through the path graph (FindPath result; L+0x224 = the path length); result != 0 -> track the node
pub fn Scientist_BaseFUNC_SetPathFinding(e: &mut Engine, param_1: Ptr) -> LResult<i32> {
    let tgt = e.alloca(0x10);
    for o in [0, 4, 8] {
        let v = e.r32(param_1 + o);
        e.w32(tgt + o, v);
    }
    let mut iStack_50 = 2i32;
    let l = L(e);
    if e.r32(l + 0x214) == 0 {
        for o in [0, 4, 8] {
            let v = e.r32(tgt + o);
            e.w32(l + 0x218 + o, v);
        }
    } else {
        let DStack_60 = e.alloca(0x10);
        APIgameobj::GetPosition(e, SELF, ALL, DStack_60);
        iStack_50 = LIBApiPathFinding::FindPath(e, DStack_60, tgt, l + 0x218, (l + 0x248) as i32) as i32;
        let n = e.r16(l + 0x250) as i16 as i32;
        e.w32(l + 0x224, n as u32);
    }
    if iStack_50 != 0 {
        Scientist_BaseFUNC_TrackPos(e, 0x40a0_0000, l + 0x218)?;
    }
    Ok(iStack_50)
}

/// 0x00015e90 node reached (1.3 m): 2 without a path-finding path, else the next node (tracked).
/// -1 while walking.
pub fn Scientist_BaseFUNC_UpdatePathFinding(e: &mut Engine) -> LResult<i32> {
    let mut iStack_40 = -1i32;
    let l = L(e);
    if LIBApiCFunctions::CheckRangePoint(e, 1.3, SELF, l + 0x218, 0) != 0 {
        if e.r32(l + 0x214) == 0 {
            iStack_40 = 2;
        } else {
            iStack_40 = LIBApiPathFinding::NextPathNode(e, l + 0x218, (l + 0x248) as i32) as i32;
            Scientist_BaseFUNC_TrackPos(e, 0x40a0_0000, l + 0x218)?;
        }
    }
    Ok(iStack_40)
}

/// 0x00015fc0 track DM (L+4) at the point `param_2` with speed bits `param_1`, mode 5.
/// NOTE (PS2 behaviour kept): the target's y is 0.5, not the point's y.
pub fn Scientist_BaseFUNC_TrackPos(e: &mut Engine, param_1: u32, param_2: Ptr) -> LResult<()> {
    let (x, z) = (e.r32(param_2), e.r32(param_2 + 8));
    let b = e.alloca(0x40);
    let dm = e.r32(L(e) + 4);
    APIphysic::DMGetParams(e, dm, b);
    for (o, v) in [(0, 0), (0x14, 5), (8, x), (0xc, 0x3f00_0000), (4, param_1), (0x10, z)] {
        e.w32(b + o, v);
    }
    APIphysic::DMSetParams(e, dm, b);
    Ok(())
}

/// 0x00016098 track DM at the leader hero, speed bits `param_1`, mode 5
pub fn Scientist_BaseFUNC_TrackHero(e: &mut Engine, param_1: u32) -> LResult<()> {
    let b = e.alloca(0x40);
    let dm = e.r32(L(e) + 4);
    APIphysic::DMGetParams(e, dm, b);
    let h = hero(e);
    for (o, v) in [(0, h), (0x14, 5), (8, 0), (0xc, 0), (0x10, 0), (4, param_1)] {
        e.w32(b + o, v);
    }
    APIphysic::DMSetParams(e, dm, b);
    Ok(())
}

/// 0x00016140 the waypoint of `param_1` nearest in xz (ties: the later one) -> L+0x1b4 / L+0x198
pub fn Scientist_BaseFUNC_FindNearestPointOnPath(e: &mut Engine, param_1: u32) -> LResult<()> {
    let iVar1 = L(e);
    let mut fStack_2c = 1e6f32;
    let DStack_60 = e.alloca(0x10);
    let DStack_50 = e.alloca(0x10);
    APIgameobj::GetPosition(e, SELF, ALL, DStack_60);
    let mut i = 0u32;
    while i < e.r32(iVar1 + 0x19c) {
        APIextra::WaypointGetPos(e, param_1, i, DStack_50);
        let dx = e.rf(DStack_60) - e.rf(DStack_50);
        let dz = e.rf(DStack_60 + 8) - e.rf(DStack_50 + 8);
        let d = APImath::powf(e, dx * dx + dz * dz, 0.5);
        if d <= fStack_2c {
            for o in [0, 4, 8] {
                let v = e.r32(DStack_50 + o);
                e.w32(iVar1 + 0x1b4 + o, v);
            }
            e.w32(iVar1 + 0x198, i);
            fStack_2c = d;
        }
        i += 1;
    }
    Ok(())
}

/// 0x000162c8 the next waypoint (wrapping) -> L+0x1b4
pub fn Scientist_BaseFUNC_FindNextPointOnPath(e: &mut Engine, param_1: u32) -> LResult<()> {
    let iVar1 = L(e);
    let v = e.r32(iVar1 + 0x198) + 1;
    e.w32(iVar1 + 0x198, v);
    if e.r32(iVar1 + 0x19c) <= e.r32(iVar1 + 0x198) {
        e.w32(iVar1 + 0x198, 0);
    }
    let i = e.r32(iVar1 + 0x198);
    APIextra::WaypointGetPos(e, param_1, i, iVar1 + 0x1b4);
    Ok(())
}

/// 0x00016388 clipping 0.1 / 40 / 65 m, flags 0x6a7, mode 7
pub fn Scientist_BaseFUNC_SetClippingScientist(e: &mut Engine) -> LResult<()> {
    let b = e.alloca(0x40);
    APIgameobj::GetClippingParams(e, SELF, b);
    for (o, v) in [
        (0, 0x3dcc_cccd),
        (4, 0x4220_0000),
        (0x10, 0x4282_0000),
        (0x1c, 0xbf80_0000),
        (0xc, 0),
        (8, 7),
        (0x18, 0x40),
        (0x14, 0x6a7),
        (0x24, 0x6a7),
        (0x20, 0x6a7),
    ] {
        e.w32(b + o, v);
    }
    APIgameobj::SetClippingParams(e, SELF, b, 0);
    Ok(())
}

/// 0x00016450 every 1.5 s (with a path-finding path): has the hero left our path node?
pub fn Scientist_BaseFUNC_HeroMoved(e: &mut Engine) -> LResult<bool> {
    if e.r32(L(e) + 0x214) == 0 {
        return Ok(false);
    }
    let t = e.r32(L(e) + 400);
    if APIextra::TimerGetTime(e, t) < 1.5 {
        return Ok(false);
    }
    let t = e.r32(L(e) + 400);
    APIextra::TimerReset(e, t);
    let DStack_30 = e.alloca(0x10);
    APIgameobj::GetPosition(e, SELF, ALL, DStack_30);
    let l = L(e);
    let iVar2 = LIBApiPathFinding::IsTargetInSameNode(e, DStack_30, (l + 0x248) as i32);
    Ok(iVar2 == 0)
}

/// 0x00016550 1 when detection is on (L+0x1d0) and the hero's floor has none of the hiding bits
/// L+0x240 (no mask: always)
pub fn Scientist_BaseFUNC_CanDetectHero_OnFloor(e: &mut Engine) -> LResult<u32> {
    let (mask, on) = (e.r32(L(e) + 0x240), e.r32(L(e) + 0x1d0));
    if mask == 0 && on != 0 {
        Ok(1)
    } else if on == 0 {
        Ok(0)
    } else {
        let b = e.alloca(0x20);
        let h = hero(e);
        APIphysic::GetFloorAttribs(e, h, b);
        Ok((e.r32(b + 8) & e.r32(L(e) + 0x240) == 0) as u32)
    }
}

/// 0x00016628 Monster_Base first; first instance: externals 0xe..0x14, API tables, global data
/// (+0 game buffer 0, +4 game buffer 2, +0xc 0); local data 0x284, start position (L+0xf8), physics,
/// AnimMngr end notify (3 -> 0xc), path block, five timers (scared 7 s), this object's
/// goCustomDataScientist record (4 records of 0x58 bytes), srand, our id, detection on, initialisation.
/// Ghidra shows `fVar4 = ChangeClipping(...); TimerGetTime(0x90000000); srand((int)fVar4)`: the float
/// in $f0 at srand is TimerGetTime's result, as in the zombie / mummy modules.
pub fn scientist_baseConstructor(e: &mut Engine, api: Ptr, param_2: i32) -> LResult<()> {
    monster_baseConstructor(e, api, param_2)?;
    if param_2 == 1 {
        let fns: [(i32, crate::logic::ExtFn); 6] = [
            (0xe, scientist_baseST_INITIAL),
            (0xf, scientist_baseST_ENTRY_POINT),
            (0x11, scientist_baseCB_TIMER),
            (0x12, scientist_baseCB_ANIM_LF),
            (0x13, scientist_baseCB_ANIMATION),
            (0x14, scientist_baseCB_CUT_SCENE),
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
        for (slot, name) in [(pFunction, "ApiCFunctions"), (pPathFinding, "ApiPathFinding")] {
            let n = e.cstr(name);
            let v = APIcons::GetApiCustom(e, n);
            let a = G(e, slot);
            e.w32(a, v);
        }
        let slot = G(e, pGlobalDatascientist_base);
        APIcons::GetGlobalData(e, slot, 0x10);
        let uVar2 = APIgameobj::GetGameBuffer(e, 0, 0x8f8, 1);
        let gd = GDS(e);
        e.w32(gd, uVar2);
        let uVar2 = APIgameobj::GetGameBuffer(e, 2, 0x158, 1);
        e.w32(gd + 4, uVar2);
        e.w32(gd + 0xc, 0);
    }
    let slot = G(e, pLocalDatascientist_base);
    APIcons::GetLocalData(e, slot, 0x284);
    let p = L(e) + 0xf8;
    APIgameobj::GetPosition(e, SELF, ALL, p);
    scientist_baseFUNC_Create_Physic(e)?;
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 3, 0xc);
    let iVar3 = APIgameobj::GetID(e, SELF, NULL);
    let l = L(e);
    for (o, v) in [(0x240, 0), (0x254, 1), (0x248, iVar3), (0x1cc, 0), (0x1d4, 0)] {
        e.w32(l + o, v);
    }
    for o in [0x184, 0x188, 0x18c, 400, 0x194] {
        let t = APIextra::TimerCreate(e);
        e.w32(L(e) + o, t);
    }
    let t = e.r32(L(e) + 0x18c);
    APIextra::TimerSetFrequency(e, t, 7.0);
    let gocd = G(e, goCustomDataScientist);
    for i in 0..4u32 {
        let rec = gocd + i * 0x58;
        if e.r32(rec) == iVar3 {
            let l = L(e);
            for (dst, src) in [(0x168, 0), (0x16c, 0x10), (0x1a4, 4), (0x1a0, 8), (0x170, 0xc), (0x1c0, 0x14), (0x214, 0x18)] {
                let v = e.r32(rec + src);
                e.w32(l + dst, v);
            }
            LIBMiscFunc::MiscFunc_StrncpySafe(e, l + 0x1d8, rec + 0x1c, 0x3c);
            break;
        }
    }
    LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 2, NULL, 0);
    let fVar4 = APIextra::TimerGetTime(e, 0x9000_0000);
    APIstdlib::srand(e, fVar4 as i32 as u32);
    let uVar2 = APIgameobj::GetID(e, SELF, NULL);
    e.w32(L(e) + 0x174, uVar2);
    e.w32(L(e) + 0x1d0, 1);
    scientist_base_Func_Initialisation(e)
}

/// 0x00016d38
pub fn scientist_baseDestructor(e: &mut Engine) -> LResult<()> {
    monster_baseDestructor(e)?;
    for o in [0x184, 0x188, 0x18c, 400, 0x194] {
        let t = e.r32(L(e) + o);
        APIextra::TimerDestroy(e, t);
    }
    Ok(())
}

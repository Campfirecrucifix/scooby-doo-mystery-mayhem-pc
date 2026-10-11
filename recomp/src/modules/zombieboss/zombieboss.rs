//! ZombieBoss.c (41 functions): the Ep4 zombie boss. Addresses are module-relative (.text at 0x10000).
//!
//! External states: 0 REINITIALISE (Monster_BaseST_REINITIALISE -> ZombieBoss_Func_Initialisation ->
//! state 5), 5 = waiting (Monster_Base's empty BLACK_HOLE_LOOP: the initial state) until the intro
//! cut-scene ends (ZombieBossCB_CUT_SCENE -> state 0x10), 0x10 FIRSTMOVE_SET (path
//! "Path_ZombieBossFight", sub-state STAND_SET, state 0x11),
//! 0x11 ENTRY_POINT (ZombieBossFUNC_GameEventSignaler sets the L+0x17c event bits it was asked to
//! check (L+0x180, ZombieBossFUNC_SetConditionsToCheck), then the sub-state code pointer L+0x178 runs).
//!
//! Sub-states: STAND (anim 3) -> FOLLOW_HERO (anim 1, chases the leader hero) -> ATTACK (wait 0.5 s on
//! timer L+0x184, then anim 5 = gas attack) -> FOLLOW_HERO ...; the book open near him (event 0x4000)
//! -> SCARED_FOLLOW (keeps 3 m away from the hero, tracking him); hooked by the book (msg 0xd6/0x65)
//! -> SCARED (anim 4) and the struggle (L+0x198 phases 0..2: each time the boss breaks free, event
//! 0x20000, L+0x198 grows; the third time he is caught). DEFENSIVE_ATTACK (anim 2) when the hero comes
//! too close (event 0x800).
//!
//! L+0x17c event bits (set by the signaler for the conditions in L+0x180 / by callbacks):
//! 2 / 4 hero in range (start following), 8 hero lost, 0x20 hero within attack range, 0x200 attack
//! anim done, 0x400 defensive attack done, 0x800 hero too close, 0x1000 hero hiding, 0x4000 book open,
//! 0x8000 book closed / link broken, 0x10000 scared anim done, 0x20000 broke free.
use super::*;
use crate::engine::*;
use crate::logic::LResult;

/// Module-relative addresses of the sub-state functions stored at L+0x178.
pub mod addr {
    pub const ZombieBossST_DEFENSIVE_ATTACK_SET: u32 = 0x15708;
    pub const ZombieBossST_DEFENSIVE_ATTACK_LOOP: u32 = 0x157b0;
    pub const ZombieBossST_STAND_SET: u32 = 0x15850;
    pub const ZombieBossST_STAND_LOOP: u32 = 0x15970;
    pub const ZombieBossST_FOLLOW_HERO_SET: u32 = 0x15a58;
    pub const ZombieBossST_FOLLOW_HERO_LOOP: u32 = 0x15b98;
    pub const ZombieBossST_ATTACK_SET: u32 = 0x15d68;
    pub const ZombieBossST_ATTACK_WAIT: u32 = 0x15e30;
    pub const ZombieBossST_ATTACK_LOOP: u32 = 0x15ef0;
    pub const ZombieBossST_SCARED_SET: u32 = 0x16010;
    pub const ZombieBossST_SCARED_LOOP: u32 = 0x160c8;
    pub const ZombieBossST_SCARED_FOLLOW_SET: u32 = 0x161b0;
    pub const ZombieBossST_SCARED_FOLLOW_LOOP: u32 = 0x162e8;
}

/// `(**(code **)(L + 0x178))()`: run the current sub-state.
pub fn call_state(e: &mut Engine) -> LResult<()> {
    let f = e.r32(L(e) + 0x178);
    match f {
        addr::ZombieBossST_DEFENSIVE_ATTACK_SET => ZombieBossST_DEFENSIVE_ATTACK_SET(e),
        addr::ZombieBossST_DEFENSIVE_ATTACK_LOOP => ZombieBossST_DEFENSIVE_ATTACK_LOOP(e),
        addr::ZombieBossST_STAND_SET => ZombieBossST_STAND_SET(e),
        addr::ZombieBossST_STAND_LOOP => ZombieBossST_STAND_LOOP(e),
        addr::ZombieBossST_FOLLOW_HERO_SET => ZombieBossST_FOLLOW_HERO_SET(e),
        addr::ZombieBossST_FOLLOW_HERO_LOOP => ZombieBossST_FOLLOW_HERO_LOOP(e),
        addr::ZombieBossST_ATTACK_SET => ZombieBossST_ATTACK_SET(e),
        addr::ZombieBossST_ATTACK_WAIT => ZombieBossST_ATTACK_WAIT(e),
        addr::ZombieBossST_ATTACK_LOOP => ZombieBossST_ATTACK_LOOP(e),
        addr::ZombieBossST_SCARED_SET => ZombieBossST_SCARED_SET(e),
        addr::ZombieBossST_SCARED_LOOP => ZombieBossST_SCARED_LOOP(e),
        addr::ZombieBossST_SCARED_FOLLOW_SET => ZombieBossST_SCARED_FOLLOW_SET(e),
        addr::ZombieBossST_SCARED_FOLLOW_LOOP => ZombieBossST_SCARED_FOLLOW_LOOP(e),
        // the PS2 would jump to whatever L+0x178 holds (0 before FIRSTMOVE_SET): a fault
        _ => panic!("zombieboss: sub-state code pointer {f:#x} is not a known function"),
    }
}

/// `*(code **)(L + 0x178) = f`
#[inline]
fn set_state(e: &mut Engine, f: u32) {
    let l = L(e);
    e.w32(l + 0x178, f);
}
/// L+0x17c (event bits)
#[inline]
fn ev(e: &Engine) -> u32 {
    e.r32(L(e) + 0x17c)
}
#[inline]
fn clr_ev(e: &mut Engine, bits: u32) {
    let a = L(e) + 0x17c;
    and32(e, a, !bits);
}
/// `L+0x198` (struggle phase 0..2) -> conditions
fn conditions_by_phase(e: &mut Engine, c: [u32; 3]) -> LResult<()> {
    match e.r32(L(e) + 0x198) {
        1 => ZombieBossFUNC_SetConditionsToCheck(e, c[1]),
        0 => ZombieBossFUNC_SetConditionsToCheck(e, c[0]),
        2 => ZombieBossFUNC_SetConditionsToCheck(e, c[2]),
        _ => Ok(()),
    }
}

/// 0x00015460 reset the events, conditions and struggle; capture parameters (weight 300, 2 s, class 3)
pub fn ZombieBoss_Func_Initialisation(e: &mut Engine) -> LResult<()> {
    let iVar1 = L(e);
    e.w32(L(e) + 0x180, 0);
    e.w32(iVar1 + 0x17c, 0);
    let v = e.r32(iVar1 + 0xb0);
    e.w32(iVar1 + 0x5c, v);
    e.w32(iVar1 + 0x1f0, 0);
    InitInLineOfSight(e)?;
    e.w32(LM(e) + 0xb0, 0x4396_0000);
    e.w32(iVar1 + 0x1fc, 0x4396_0000);
    e.w32(LM(e) + 0xb4, 0x4000_0000);
    e.w32(LM(e) + 0x5c, 0x4396_0000);
    e.w32(LM(e) + 0x68, 0x3e99_999a);
    e.w32(LM(e) + 0x6c, 0x3f33_3333);
    e.w32(LM(e) + 0x58, 3);
    e.w32(iVar1 + 0x198, 0);
    e.w32(iVar1 + 0x19c, 0);
    e.w32(iVar1 + 0x1a0, 0);
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 0xb, 0x15);
    APIgameobj::SetGroupID(e, SELF, 2);
    ZombieBossFUNC_Reset(e)?;
    APIgameobj::SetNextState(e, SELF, 5);
    Ok(())
}

/// 0x000155e0 external state 0x10
pub fn ZombieBossST_FIRSTMOVE_SET(e: &mut Engine, _: u32) -> LResult<u64> {
    let iVar1 = L(e);
    let p = e.cstr("Path_ZombieBossFight");
    LIBApiPathFinding::SetPath(e, p as i32, (iVar1 + 0x260) as i32);
    e.w32(iVar1 + 0x178, addr::ZombieBossST_STAND_SET);
    SetClipping(e)?;
    APIgameobj::SetNextState(e, SELF, 0x11);
    Ok(0)
}

/// 0x00015698 external state 0x11
pub fn ZombieBossST_ENTRY_POINT(e: &mut Engine, _: u32) -> LResult<u64> {
    ZombieBossFUNC_GameEventSignaler(e)?;
    call_state(e)?;
    Ok(0)
}

/// 0x00015708 anim 2 (once, notified)
pub fn ZombieBossST_DEFENSIVE_ATTACK_SET(e: &mut Engine) -> LResult<()> {
    let iVar1 = L(e);
    e.w32(L(e) + 0x17c, 0);
    e.w32(iVar1 + 0x180, 0);
    APIgameobj::SetContextFlag(e, SELF, 0x60);
    LIBAnimMngr::PlayAnim(e, 0.2, 2, 1, 0x17);
    set_state(e, addr::ZombieBossST_DEFENSIVE_ATTACK_LOOP);
    Ok(())
}

/// 0x000157b0
pub fn ZombieBossST_DEFENSIVE_ATTACK_LOOP(e: &mut Engine) -> LResult<()> {
    if ev(e) & 0x400 != 0 {
        clr_ev(e, 0x400);
        let t = e.r32(L(e) + 0x188);
        APIextra::TimerReset(e, t);
        set_state(e, addr::ZombieBossST_FOLLOW_HERO_SET);
    }
    Ok(())
}

/// 0x00015850
pub fn ZombieBossST_STAND_SET(e: &mut Engine) -> LResult<()> {
    LIBAnimMngr::PlayAnim(e, 0.2, 3, 2, 0);
    APIgameobj::SetContextFlag(e, SELF, 0x60);
    conditions_by_phase(e, [2, 0x62, 2])?;
    set_state(e, addr::ZombieBossST_STAND_LOOP);
    Ok(())
}

/// 0x00015970
pub fn ZombieBossST_STAND_LOOP(e: &mut Engine) -> LResult<()> {
    if ev(e) & 4 == 0 || ev(e) & 0x1000 != 0 {
        if ev(e) & 0x800 != 0 {
            clr_ev(e, 0x800);
            set_state(e, addr::ZombieBossST_DEFENSIVE_ATTACK_SET);
        }
    } else {
        clr_ev(e, 4);
        set_state(e, addr::ZombieBossST_FOLLOW_HERO_SET);
    }
    Ok(())
}

/// 0x00015a58
pub fn ZombieBossST_FOLLOW_HERO_SET(e: &mut Engine) -> LResult<()> {
    e.w32(L(e) + 0x17c, 0);
    APIgameobj::SetContextFlag(e, SELF, 0x66);
    ZombieBossFUNC_FollowHero(e)?;
    LIBAnimMngr::PlayAnim(e, 0.2, 1, 2, 0);
    conditions_by_phase(e, [0x9a, 0xf2, 0xba])?;
    set_state(e, addr::ZombieBossST_FOLLOW_HERO_LOOP);
    Ok(())
}

/// 0x00015b98
pub fn ZombieBossST_FOLLOW_HERO_LOOP(e: &mut Engine) -> LResult<()> {
    if ev(e) & 0x4000 == 0 {
        if ev(e) & 0x20 == 0 {
            if ev(e) & 0x1000 == 0 {
                if ev(e) & 8 == 0 {
                    if ev(e) & 0x800 != 0 {
                        clr_ev(e, 0x800);
                        set_state(e, addr::ZombieBossST_DEFENSIVE_ATTACK_SET);
                    }
                } else {
                    clr_ev(e, 8);
                    set_state(e, addr::ZombieBossST_STAND_SET);
                }
            } else {
                clr_ev(e, 0x1000);
                set_state(e, addr::ZombieBossST_STAND_SET);
            }
        } else {
            clr_ev(e, 0x20);
            set_state(e, addr::ZombieBossST_ATTACK_SET);
        }
    } else {
        clr_ev(e, 0x4000);
        set_state(e, addr::ZombieBossST_SCARED_FOLLOW_SET);
    }
    Ok(())
}

/// 0x00015d68 anim 3, then wait (runs ATTACK_WAIT at once)
pub fn ZombieBossST_ATTACK_SET(e: &mut Engine) -> LResult<()> {
    e.w32(L(e) + 0x17c, 0);
    LIBAnimMngr::PlayAnim(e, 0.2, 3, 2, 0);
    APIgameobj::SetContextFlag(e, SELF, 0x60);
    ZombieBossFUNC_SetConditionsToCheck(e, 8)?;
    set_state(e, addr::ZombieBossST_ATTACK_WAIT);
    call_state(e)
}

/// 0x00015e30 0.5 s on the attack timer L+0x184 -> anim 5 (gas attack), ATTACK_LOOP at once
pub fn ZombieBossST_ATTACK_WAIT(e: &mut Engine) -> LResult<()> {
    let t = e.r32(L(e) + 0x184);
    let fVar2 = APIextra::TimerGetTime(e, t);
    if 0.5 <= fVar2 {
        LIBAnimMngr::PlayAnim(e, 0.2, 5, 1, 0x17);
        set_state(e, addr::ZombieBossST_ATTACK_LOOP);
        return call_state(e);
    }
    Ok(())
}

/// 0x00015ef0 attack anim done: gas off; hero still in range (0x20) -> wait again, else follow
pub fn ZombieBossST_ATTACK_LOOP(e: &mut Engine) -> LResult<()> {
    if ev(e) & 0x200 != 0 {
        clr_ev(e, 0x200);
        ZombieBossFUNC_Particles_Stop_Gaz(e)?;
        ZombieBossFUNC_Particles_Stop_Grumeaux(e)?;
        let t = e.r32(L(e) + 0x184);
        APIextra::TimerReset(e, t);
        if ev(e) & 0x20 == 0 {
            set_state(e, addr::ZombieBossST_FOLLOW_HERO_SET);
        } else {
            clr_ev(e, 0x20);
            set_state(e, addr::ZombieBossST_ATTACK_WAIT);
        }
    }
    Ok(())
}

/// 0x00016010 hooked: anim 4 (once, notified)
pub fn ZombieBossST_SCARED_SET(e: &mut Engine) -> LResult<()> {
    e.w32(L(e) + 0x17c, 0);
    LIBAnimMngr::PlayAnim(e, 0.2, 4, 1, 0x17);
    APIgameobj::SetContextFlag(e, SELF, 0x60);
    ZombieBossFUNC_SetConditionsToCheck(e, 0x80)?;
    set_state(e, addr::ZombieBossST_SCARED_LOOP);
    Ok(())
}

/// 0x000160c8 book closed (0x8000, not being pulled: L+0x1f0) -> follow; anim done -> scared follow
pub fn ZombieBossST_SCARED_LOOP(e: &mut Engine) -> LResult<()> {
    if ev(e) & 0x8000 == 0 || e.r32(L(e) + 0x1f0) != 0 {
        if ev(e) & 0x10000 != 0 {
            clr_ev(e, 0x10000);
            set_state(e, addr::ZombieBossST_SCARED_FOLLOW_SET);
        }
    } else {
        clr_ev(e, 0x8000);
        set_state(e, addr::ZombieBossST_FOLLOW_HERO_SET);
    }
    Ok(())
}

/// 0x000161b0 walk keeping 3 m from the hero
pub fn ZombieBossST_SCARED_FOLLOW_SET(e: &mut Engine) -> LResult<()> {
    LIBAnimMngr::PlayAnim(e, 0.2, 1, 2, 0);
    APIgameobj::SetContextFlag(e, SELF, 0x66);
    ZombieBossFUNC_TrackHero(e, 3.0)?;
    conditions_by_phase(e, [0x188, 0x188, 0xa8])?;
    set_state(e, addr::ZombieBossST_SCARED_FOLLOW_LOOP);
    Ok(())
}

/// 0x000162e8
pub fn ZombieBossST_SCARED_FOLLOW_LOOP(e: &mut Engine) -> LResult<()> {
    let iVar1 = L(e);
    if ev(e) & 0x8000 == 0 {
        if ev(e) & 0x20000 == 0 {
            if ev(e) & 0x20 == 0 {
                if ev(e) & 0x800 != 0 {
                    clr_ev(e, 0x800);
                    set_state(e, addr::ZombieBossST_DEFENSIVE_ATTACK_SET);
                }
            } else {
                clr_ev(e, 0x20);
                set_state(e, addr::ZombieBossST_ATTACK_SET);
            }
        } else {
            clr_ev(e, 0x20000);
            let v = e.r32(iVar1 + 0x198).wrapping_add(1);
            e.w32(iVar1 + 0x198, v);
            set_state(e, addr::ZombieBossST_DEFENSIVE_ATTACK_SET);
        }
    } else {
        clr_ev(e, 0x8000);
        set_state(e, addr::ZombieBossST_FOLLOW_HERO_SET);
    }
    Ok(())
}

/// 0x00016480 external state 8 (in a cut-scene): hold the attack timer
pub fn Monster_BaseST_IN_CUT_LOOP(e: &mut Engine, _: u32) -> LResult<u64> {
    let t = e.r32(L(e) + 0x184);
    APIextra::TimerReset(e, t);
    Ok(0)
}

/// 0x000164e0 external state 0
pub fn Monster_BaseST_REINITIALISE(e: &mut Engine, _: u32) -> LResult<u64> {
    ZombieBoss_Func_Initialisation(e)?;
    Ok(0)
}

/// 0x00016530 0xd6 from the book: 100 link broken (stop being pulled), 0x65 hooked (gas off, SCARED),
/// 0x67 pulled in (hide, launch the binding latent 0x2a8000bd at our position and pass it the message,
/// HUD 0x3a, state 5), 0x68 captured (camera 0x54 {0x62,..}, HUD 0x2b/9 and 0x1e, L+0x1f4 = loading).
/// 0x4f with L+0x1f4: load the Ep4 movie (stleveltransitionparams {2, "\MOVIES\EP04\EP04D"}, game
/// buffer 2 +0x28 |= 0x80000). 0xc9 restart: reset and return (no chaining). 0x125 pull strength
/// (L+0x1fc) while pulled. 0xde: the boss appears. 0x15c: start following the hero. Everything else
/// (and every case but 0xc9) then goes to Monster_Base_MessageFunction.
pub fn ZombieBossMessageFonction(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    let iVar1 = L(e);
    match param_1 {
        0xd6 => match param_2 {
            100 => e.w32(L(e) + 0x1f0, 0),
            0x65 => {
                ZombieBossFUNC_Particles_Stop_Gaz(e)?;
                ZombieBossFUNC_Particles_Stop_Grumeaux(e)?;
                e.w32(iVar1 + 0x1f0, 1);
                e.w32(iVar1 + 0x178, addr::ZombieBossST_SCARED_SET);
            }
            0x67 => {
                APIphysic::ResetFlags(e, SELF, 6);
                APIphysic::SetFlags(e, SELF, 1);
                LIBApiCFunctions::TrackID(e, 3.1415927, SELF as i32, 0x2a00_000a);
                APIgameobj::SetContextFlag(e, SELF, 0x60);
                APIgameobj::SetGameObjectVisible(e, SELF, 0);
                APIanim::StopAllAnim(e, SELF);
                let stomp = e.r32(L(e) + 0x194);
                let iVar2 = APIgameobj::GetID(e, stomp, NULL);
                if iVar2 != 0x7fff_fffe {
                    let stomp = e.r32(L(e) + 0x194);
                    APIgameobj::KillObject(e, stomp);
                }
                let uVar3 = APIgameobj::LaunchLatant(e, 0x2a80_00bd);
                e.w32(iVar1 + 0x18c, uVar3);
                let pos = e.alloca(0x10);
                APIgameobj::GetPosition(e, SELF, ALL, pos);
                let binding = e.r32(iVar1 + 0x18c);
                APIgameobj::SetTranslation(e, binding, pos);
                APIgameobj::SendMessageTo(e, binding, NULL, 0x67, 0, param_3);
                APIgameobj::SendMessageTo(e, 0x2a10_0006, NULL, 0x3a, 0, NULL);
                APIgameobj::SetNextState(e, SELF, 5);
            }
            0x68 => {
                APIgameobj::UnregisterNotify(e, SELF, 0xffff_ffff, 0xb);
                APIgameobj::SetContextFlag(e, SELF, 0x60);
                APIgameobj::SendMessageTo(e, 0x2a10_0006, NULL, 0x2b, 9, NULL);
                e.w32(iVar1 + 500, 1);
                // moCameraParams.174 (0x10078) +0 / +0x3c / +0x40 / +0x44: the C writes these four words only
                let c = G(e, moCameraParams_174);
                for (o, v) in [(0, 0x62), (0x3c, 0), (0x40, 1), (0x44, 0)] {
                    e.w32(c + o, v);
                }
                APIgameobj::SendMessageTo(e, 0x2a00_0004, NULL, 0x54, 0, c);
                APIgameobj::SendMessageTo(e, 0x2a10_0006, NULL, 0x1e, 0, NULL);
            }
            _ => {} // 0x69 and others: nothing here
        },
        0x4f => {
            if e.r32(L(e) + 500) != 0 {
                // memcpy(asStack_810, &DAT_00010f18, 0x7dc): only words +0 / +4 are non-zero
                let asStack_810 = e.alloca(0x7dc);
                for o in (0..0x7dc).step_by(4) {
                    e.w32(asStack_810 + o, 0);
                }
                e.w32(asStack_810, 2);
                let movie = e.cstr("\\MOVIES\\EP04\\EP04D");
                e.w32(asStack_810 + 4, movie);
                let gb2 = e.r32(GDZ(e) + 4);
                or32(e, gb2 + 0x28, 0x80000);
                let l = e.cstr("LOADER");
                APIgame::LoadLevel(e, l, l, asStack_810);
            }
        }
        0xc9 => {
            ZombieBoss_Func_Initialisation(e)?;
            ZombieBossFUNC_Particles_Stop_Gaz(e)?;
            ZombieBossFUNC_Particles_Stop_Grumeaux(e)?;
            return Ok(0);
        }
        0x125 => {
            if e.r32(L(e) + 0x1f0) != 0 {
                let v = e.r32(param_3);
                e.w32(L(e) + 0x1fc, v);
            }
        }
        0xde => {
            // auStack_820[1..3] are uninitialised stack words on the PS2; the port zeroes them
            let b = e.alloca(0x10);
            for o in [0, 4, 8, 0xc] {
                e.w32(b + o, 0);
            }
            e.w32(b, 0x8000_000b);
            APIgameobj::SetGameObjectVisible(e, SELF, 1);
            APIgameobj::SetGroupBitMask(e, SELF, 0x30_0241);
            APIgameobj::SendMessageTo(e, 0x2a00_0090, NULL, 0xeb, 2, NULL);
            APIgameobj::SendMessageTo(e, 0x2a00_0004, NULL, 0x1de, 0, b);
        }
        0x15c => set_state(e, addr::ZombieBossST_FOLLOW_HERO_SET),
        _ => {}
    }
    Monster_Base_MessageFunction(e, param_1, param_2, param_3)
}

/// 0x00016bd0 external 0xc
pub fn Monster_BaseCB_NOTIFY_COLLIDE_OBJ(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

/// 0x00016c60 external 0x17: anim 4 / 2 / 5 ended -> event 0x10000 / 0x400 / 0x200
pub fn ZombieBossCB_ANIM_LF(e: &mut Engine, param_1: u32) -> LResult<u64> {
    let a = L(e) + 0x17c;
    match param_1 {
        4 => or32(e, a, 0x10000),
        2 => or32(e, a, 0x400),
        5 => or32(e, a, 0x200),
        _ => {}
    }
    Ok(0)
}

/// 0x00016d48 external 0x18: cut-scene start (+4 == 1: save the clipping into *(L+0x270), state 5)
/// / end (+4 == 0: restore, state 0x10, drop the first wisps, sub-state STAND).
/// NOTE: L+0x270 is a pointer this module never sets (it lies in the path-finding block at L+0x260).
pub fn ZombieBossCB_CUT_SCENE(e: &mut Engine, param_1: u32) -> LResult<u64> {
    match e.r32(param_1 + 4) {
        1 => {
            let p = e.r32(L(e) + 0x270);
            APIgameobj::GetClippingParams(e, SELF, p);
            let uVar2 = APIgameobj::GetID(e, SELF, NULL);
            LIBMiscFunc::MiscFunc_InitForCutscene(e, uVar2);
            APIgameobj::SetNextState(e, SELF, 5);
        }
        0 => {
            let uVar2 = APIgameobj::GetID(e, SELF, NULL);
            let p = e.r32(L(e) + 0x270);
            LIBMiscFunc::MiscFunc_EndCutscene(e, uVar2, p);
            APIgameobj::SetNextState(e, SELF, 0x10);
            ZombieBossFUNC_DropWisps(e, 0)?;
            set_state(e, addr::ZombieBossST_STAND_SET);
        }
        _ => {}
    }
    Ok(0)
}

/// 0x00016eb8 external 0x15 (anim key frames). Anim 2 (defensive stomp) key 0: rumble, wisps for a
/// new struggle phase (L+0x19c != L+0x198), stomp gas, the shock-wave latent 0x2a8000be (L+0x194, msg
/// 0xfe with our position + (-0.243, 0, 0.413)), camera shake 0x56 (parameters in game buffer 0
/// +0x7c8), and if the book is open (game buffer 0 +0x14 & 1) msg 0x92 to the leader hero. Anim 5
/// (gas attack) key 0: rumble and gas on; other keys: gas off.
pub fn ZombieBossCB_NOTIFY_ANI_KEYFRAME(e: &mut Engine, param_1: u32) -> LResult<u64> {
    let gb0 = |e: &Engine| e.r32(GDZ(e));
    match e.r32(param_1) {
        2 => {
            if e.r32(param_1 + 8) == 0 {
                let puStack_40 = gb0(e) + 0x7c8;
                let port = e.r32(gb0(e) + 0x4c) as i32;
                APIinput::InputSetVibration(e, port, 0, 1.0, 0.0, 0.33, 0.0);
                let l = L(e);
                if e.r32(l + 0x19c) != e.r32(l + 0x198) {
                    let v = e.r32(l + 0x198);
                    e.w32(l + 0x19c, v);
                    ZombieBossFUNC_DropWisps(e, v as i32)?;
                }
                ZombieBossFUNC_Particles_Start_GazStomp(e)?;
                let uVar1 = APIgameobj::LaunchLatant(e, 0x2a80_00be);
                e.w32(L(e) + 0x194, uVar1);
                let d = e.alloca(0x10);
                APIgameobj::GetPosition(e, SELF, 0, d);
                let x = e.rf(d) + -0.243;
                let y = e.rf(d + 4) + 0.0;
                let z = e.rf(d + 8) + 0.413;
                e.wf(d, x);
                e.wf(d + 4, y);
                e.wf(d + 8, z);
                let to = e.r32(L(e) + 0x194);
                APIgameobj::SendMessageTo(e, to, NULL, 0xfe, 0, d);
                for (i, v) in [0, 0x3f99_999a, 0x3c23_d70a, 0x3dcc_cccd, 0x3e99_999a, 0x4120_0000].into_iter().enumerate() {
                    e.w32(puStack_40 + i as u32 * 4, v);
                }
                APIgameobj::SendMessageTo(e, 0x2a00_0004, NULL, 0x56, 0, NULL);
                if e.r32(gb0(e) + 0x14) & 1 != 0 {
                    let hero = e.r32(gb0(e) + 8);
                    APIgameobj::SendMessageTo(e, hero, NULL, 0x92, 0, NULL);
                }
            }
        }
        5 => {
            if e.r32(param_1 + 8) == 0 {
                let port = e.r32(gb0(e) + 0x4c) as i32;
                APIinput::InputSetVibration(e, port, 0, 0.66, 0.0, 0.33, 0.0);
                ZombieBossFUNC_Particles_Start_Gaz(e)?;
                ZombieBossFUNC_Particles_Start_Grumeaux(e)?;
            } else {
                ZombieBossFUNC_Particles_Stop_Gaz(e)?;
                ZombieBossFUNC_Particles_Stop_Grumeaux(e)?;
            }
        }
        _ => {}
    }
    Ok(0)
}

/// 0x00017258 external 0x16
pub fn ZombieBossCB_TIMER(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

/// 0x00017288 the book is open (game buffer 0 +0x14 & 1) and the leader hero within 20 m
pub fn ZombieBossFUNC_IsTomeOpen(e: &mut Engine) -> LResult<u32> {
    if e.r32(e.r32(GDZ(e)) + 0x14) & 1 == 0 {
        return Ok(0);
    }
    let hero = e.r32(e.r32(GDM(e)) + 8);
    let bVar1 = LIBApiCFunctions::CheckRangeObject(e, 20.0, SELF, hero, 0);
    Ok(if bVar1 == 0 { 0 } else { 1 })
}

/// 0x00017320 chase the leader hero: straight at him (TrackHero 3 m) when within 3 m or no path
/// finding is needed, else along the path graph at L+0x260 (L+0x1a0 = following a path).
pub fn ZombieBossFUNC_FollowHero(e: &mut Engine) -> LResult<()> {
    let iVar2 = L(e);
    let hero = |e: &Engine| e.r32(e.r32(GDM(e)) + 8);
    let d = e.alloca(0x10);
    if e.r32(L(e) + 0x1a0) == 0 {
        let h = hero(e);
        let bVar1 = LIBApiCFunctions::CheckRangeObject(e, 3.0, SELF, h, 0);
        if bVar1 == 0 {
            let h = hero(e);
            let need = LIBApiPathFinding::NeedPathFinding(e, h, (iVar2 + 0x260) as i32);
            if need != 0 {
                e.w32(iVar2 + 0x1a0, 1);
                let h = hero(e);
                APIgameobj::GetPosition(e, h, ALL, d);
                ZombieBossFUNC_SetPathFinding(e, d)?;
            }
        }
    } else {
        let h = hero(e);
        let mut bVar1 = LIBApiCFunctions::CheckRangeObject(e, 3.0, SELF, h, 0) != 0;
        if !bVar1 {
            let h = hero(e);
            bVar1 = LIBApiPathFinding::NeedPathFinding(e, h, (iVar2 + 0x260) as i32) == 0;
        }
        if bVar1 {
            e.w32(iVar2 + 0x1a0, 0);
            ZombieBossFUNC_TrackHero(e, 3.0)?;
        }
        let h = hero(e);
        APIgameobj::GetPosition(e, h, ALL, d);
        let same = LIBApiPathFinding::IsTargetInSameNode(e, d, (iVar2 + 0x260) as i32);
        if same == 0 {
            ZombieBossFUNC_SetPathFinding(e, d)?;
        } else {
            ZombieBossFUNC_UpdatePathFinding(e)?;
        }
    }
    Ok(())
}

/// 0x00017570 move DM (L+4) at the leader hero, speed `param_1`, mode 5
pub fn ZombieBossFUNC_TrackHero(e: &mut Engine, param_1: f32) -> LResult<()> {
    let b = e.alloca(0x80);
    let dm = e.r32(L(e) + 4);
    APIphysic::DMGetParams(e, dm, b);
    let hero = e.r32(e.r32(GDM(e)) + 8);
    e.w32(b, hero);
    e.w32(b + 0x14, 5);
    e.w32(b + 8, 0);
    e.w32(b + 0xc, 0);
    e.w32(b + 0x10, 0);
    e.wf(b + 4, param_1);
    APIphysic::DMSetParams(e, dm, b);
    Ok(())
}

/// 0x00017618 move DM (L+4) at the path node position L+0x1e4 (y = 0.5), speed `param_1`, mode 5
pub fn ZombieBossFUNC_TrackPos(e: &mut Engine, param_1: f32) -> LResult<()> {
    let iStack_30 = L(e);
    let b = e.alloca(0x80);
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

/// 0x000176c8 find a path from us to `param_1` (node -> L+0x1e4) and walk to its first node
pub fn ZombieBossFUNC_SetPathFinding(e: &mut Engine, param_1: Ptr) -> LResult<i32> {
    let target = e.alloca(0x10);
    for o in [0, 4, 8] {
        let v = e.r32(param_1 + o);
        e.w32(target + o, v);
    }
    let me = e.alloca(0x10);
    APIgameobj::GetPosition(e, SELF, ALL, me);
    let l = L(e);
    LIBApiPathFinding::FindPath(e, me, target, l + 0x1e4, (l + 0x260) as i32);
    let l = L(e);
    let iStack_50 = LIBApiPathFinding::NextPathNode(e, l + 0x1e4, (l + 0x260) as i32) as i32;
    if iStack_50 != 2 && iStack_50 != 0 {
        ZombieBossFUNC_TrackPos(e, 3.0)?;
    }
    Ok(iStack_50)
}

/// 0x00017828 within 0.5 m of the current node: on to the next one
pub fn ZombieBossFUNC_UpdatePathFinding(e: &mut Engine) -> LResult<i32> {
    let mut iStack_40 = -1;
    let p = L(e) + 0x1e4;
    let bVar1 = LIBApiCFunctions::CheckRangePoint(e, 0.5, SELF, p, 0);
    if bVar1 != 0 {
        let l = L(e);
        iStack_40 = LIBApiPathFinding::NextPathNode(e, l + 0x1e4, (l + 0x260) as i32) as i32;
        if iStack_40 != 2 && iStack_40 != 0 {
            ZombieBossFUNC_TrackPos(e, 3.0)?;
        }
    }
    Ok(iStack_40)
}

/// 0x00017918 msg 0xf7 to the wisp dropper 0x2a0000c3 with {1, n, 1} at L+0x274 (n = 1 for struggle
/// phases 0 and 1, 2 for phase 2; nothing for other values)
pub fn ZombieBossFUNC_DropWisps(e: &mut Engine, param_1: i32) -> LResult<()> {
    let iVar1 = L(e);
    let n = match param_1 {
        0 | 1 => 1,
        2 => 2,
        _ => return Ok(()),
    };
    e.w32(iVar1 + 0x274, 1);
    e.w32(iVar1 + 0x278, n);
    e.w32(iVar1 + 0x27c, 1);
    APIgameobj::SendMessageTo(e, 0x2a00_00c3, NULL, 0xf7, 0, iVar1 + 0x274);
    Ok(())
}

/// 0x00017aa0 hidden, no physics response, no group, back at the start position (L+0xf8)
pub fn ZombieBossFUNC_Reset(e: &mut Engine) -> LResult<()> {
    APIphysic::SetFlags(e, SELF, 1);
    APIgameobj::SetGameObjectVisible(e, SELF, 0);
    APIgameobj::SetGroupBitMask(e, SELF, 0);
    APIgameobj::SendMessageTo(e, 0x2a00_0090, NULL, 0xeb, 2, NULL);
    APIgameobj::SetContextFlag(e, SELF, 0x60);
    e.w32(L(e) + 0x180, 0);
    let p = L(e) + 0xf8;
    APIgameobj::SetTranslation(e, SELF, p);
    Ok(())
}

/// 0x00017b98
pub fn ZombieBossFUNC_SetConditionsToCheck(e: &mut Engine, param_1: u32) -> LResult<()> {
    e.w32(L(e) + 0x180, 0);
    let a = L(e) + 0x180;
    or32(e, a, param_1);
    Ok(())
}

/// 0x00017be8 physics: move DM 1 (L+4, at the hero, mask 2), turn DM 4 (L+8, mask 4), DM 7 (L+0,
/// mask 1), gravity DM 0 (L+0x24, mask 0x40), DM 8 (L+0x20, mask 0x20)
pub fn ZombieBossFUNC_Create_Physic(e: &mut Engine) -> LResult<()> {
    let puVar1 = L(e);
    APIphysic::CreatePhysic(e, SELF, 0);
    APIphysic::SetFlags(e, SELF, 6);
    APIphysic::SetPrimaryPrim(e, SELF, 999);
    APIphysic::SetWeight(e, SELF, 0.75);
    APIphysic::SetPriority(e, SELF, 10);
    let dm = |e: &mut Engine, slot: u32, ty: u32, words: &[(u32, u32)], mask: u32| {
        let uVar2 = APIphysic::DMAdd(e, SELF, ty);
        e.w32(puVar1 + slot, uVar2);
        let b = e.alloca(0x80);
        APIphysic::DMGetParams(e, uVar2, b);
        for &(o, v) in words {
            e.w32(b + o, v);
        }
        APIphysic::DMSetParams(e, uVar2, b);
        APIphysic::DMSetContextMask(e, uVar2, mask);
    };
    let speed = e.r32(puVar1 + 0x84);
    let hero = e.r32(e.r32(GDM(e)) + 8);
    dm(e, 4, 1, &[(4, speed), (0, hero), (8, 0), (0xc, 0), (0x10, 0), (0x14, 5)], 2);
    dm(e, 8, 4, &[(0, 0), (4, 0), (8, 0x4000_0000), (0xc, 1)], 4);
    dm(e, 0, 7, &[(0, 0x4120_0000), (4, 5)], 1);
    dm(e, 0x24, 0, &[(0, 0), (4, 0xc1a0_0000), (8, 0)], 0x40);
    dm(e, 0x20, 8, &[(0, 0x4296_0000), (4, 0x4234_0000)], 0x20);
    Ok(())
}

/// 0x00017fd0 set the L+0x17c event bits asked for in L+0x180 (see the file doc). Distances are to
/// the leader hero; `fStack_40` is the vertical gap (no event while the hero is 0.5 m above/below);
/// game buffer 0 +0x14 & 0x4900094c = the hero is hiding / in a state the boss ignores.
pub fn ZombieBossFUNC_GameEventSignaler(e: &mut Engine) -> LResult<()> {
    let iVar2 = L(e);
    let mut fStack_40 = 0.0f32;
    let mut fStack_3c = 0.0f32;
    let cond = |e: &Engine| e.r32(iVar2 + 0x180);
    let hidden = |e: &Engine| e.r32(e.r32(GDZ(e)) + 0x14) & 0x4900_094c != 0;
    let ev = iVar2 + 0x17c;
    if e.r32(L(e) + 0x180) == 0 {
        return Ok(());
    }
    if cond(e) & 0x6a != 0 {
        let hero = e.r32(e.r32(GDM(e)) + 8);
        fStack_3c = LIBApiCFunctions::GetDistance(e, SELF as i32, hero, ALL as i32, 0);
        let a = e.alloca(0x10);
        let b = e.alloca(0x10);
        APIgameobj::GetPosition(e, SELF, ALL, a);
        let hero = e.r32(e.r32(GDM(e)) + 8);
        APIgameobj::GetPosition(e, hero, ALL, b);
        let dy = e.rf(a + 4) - e.rf(b + 4);
        fStack_40 = APImath::sqrtf(e, dy * dy);
    }
    if cond(e) & 8 != 0 {
        let mut out = 5.5 < fStack_3c;
        if !out {
            let hero = e.r32(e.r32(GDM(e)) + 8);
            out = LIBApiCFunctions::CheckPositionOrientation(e, SELF, hero, 0x8001) == 0;
        }
        if out || hidden(e) || 0.5 < fStack_40 {
            and32(e, ev, 0xffff_ffdf);
        } else {
            or32(e, ev, 0x20);
        }
    }
    if cond(e) & 2 != 0 {
        if 30.0 < fStack_3c || hidden(e) || 0.5 < fStack_40 {
            or32(e, ev, 8);
        } else {
            or32(e, ev, 4);
        }
    }
    if cond(e) & 0x10 != 0 {
        ZombieBossFUNC_FollowHero(e)?;
    }
    if cond(e) & 0x40 != 0 {
        if 2.0 < fStack_3c || hidden(e) || 0.5 < fStack_40 {
            or32(e, ev, 0x2000);
        } else {
            or32(e, ev, 0x1000);
        }
    }
    if cond(e) & 0x20 != 0 && fStack_3c <= 13.0 {
        let t = e.r32(iVar2 + 0x188);
        let fVar5 = APIextra::TimerGetTime(e, t);
        if e.rf(iVar2 + 0x1f8) <= fVar5 && fStack_40 <= 0.5 {
            or32(e, ev, 0x800);
            let fVar5 = LIBApiCFunctions::FloatRandom(e, 0.0, 1.0, 0.05);
            e.wf(L(e) + 0x1f8, fVar5 + 4.5);
        }
    }
    if cond(e) & 0x80 != 0 {
        if ZombieBossFUNC_IsTomeOpen(e)? == 0 {
            or32(e, ev, 0x8000);
        } else {
            or32(e, ev, 0x4000);
        }
    }
    if cond(e) & 0x100 != 0 {
        // ask the book (0x2a00000a) for the pull strength: it answers with 0x125 (-> L+0x1fc)
        let uVar4 = APIgameobj::GetID(e, SELF, NULL);
        APIgameobj::SendMessageTo(e, 0x2a00_000a, NULL, 0x125, uVar4, NULL);
        let phase = e.r32(iVar2 + 0x198);
        let pull = e.rf(iVar2 + 0x1fc);
        if (phase == 0 && pull <= 200.0) || (phase == 1 && pull <= 100.0) {
            APIgameobj::SendMessageTo(e, 0x2a10_0006, NULL, 0x3a, 0, NULL);
            or32(e, ev, 0x20000);
        }
    }
    Ok(())
}

/// 0x00018610 Monster_Base first, then the boss: externals 0x10/0x11/0/0x15..0x18, API tables,
/// global data (+0 game buffer 0, +4 game buffer 2), local data 0x280 bytes, physics, particles,
/// notifies (3 -> 0xc, 0x16 -> 0x18), path-finding block (L+0x260 = our id, L+0x26c = 1), two timers,
/// srand, the next defensive-attack time, then both initialisations; initial state 5 (wait for the cut-scene).
/// Ghidra shows `srand((int)fVar3 + instanceCount)` with fVar3 the float return register: it is
/// TimerGetTime(0x90000000)'s result (the call just before).
pub fn zombiebossConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    monster_baseConstructor(e, api, instanceCount)?;
    if instanceCount == 1 {
        let fns: [(i32, crate::logic::ExtFn); 7] = [
            (0x10, ZombieBossST_FIRSTMOVE_SET),
            (0x11, ZombieBossST_ENTRY_POINT),
            (0, Monster_BaseST_REINITIALISE),
            (0x15, ZombieBossCB_NOTIFY_ANI_KEYFRAME),
            (0x16, ZombieBossCB_TIMER),
            (0x17, ZombieBossCB_ANIM_LF),
            (0x18, ZombieBossCB_CUT_SCENE),
        ];
        for (i, f) in fns {
            APIcons::SetExternalFunction(e, i, f);
        }
        let tables: [(u32, fn(&mut Engine) -> Ptr); 9] = [
            (pAnim, |e| APIcons::GetApiAnim(e)),
            (pGameObj, |e| APIcons::GetApiGameObj(e)),
            (pExtra, |e| APIcons::GetApiExtra(e)),
            (pPhysic, |e| APIcons::GetApiPhysic(e)),
            (pInput, |e| APIcons::GetApiInput(e)),
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
        let v = APIcons::GetApiGame(e);
        let a = G(e, pGame);
        e.w32(a, v);
        let slot = G(e, pGlobalDataZombieBoss);
        APIcons::GetGlobalData(e, slot, 0xc);
        let uVar2 = APIgameobj::GetGameBuffer(e, 0, 0x8f8, 1);
        let gd = GDZ(e);
        e.w32(gd, uVar2);
        let uVar2 = APIgameobj::GetGameBuffer(e, 2, 0x158, 1);
        let gd = GDZ(e);
        e.w32(gd + 4, uVar2);
    }
    let slot = G(e, pLocalDataZombieBoss);
    APIcons::GetLocalData(e, slot, 0x280);
    ZombieBossFUNC_Create_Physic(e)?;
    for o in [0x200, 0x210, 0x214] {
        let a = L(e) + o;
        e.w32(a, 0xffff_ffff);
    }
    ZombieBossFUNC_CreateParticlesTypes(e)?;
    ZombieBossFUNC_CreateParticlesEmitters(e)?;
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 3, 0xc);
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 0x16, 0x18);
    e.w32(L(e) + 0x26c, 1);
    let uVar2 = APIgameobj::GetID(e, SELF, NULL);
    e.w32(L(e) + 0x260, uVar2);
    let uVar2 = APIextra::TimerCreate(e);
    e.w32(L(e) + 0x184, uVar2);
    let uVar2 = APIextra::TimerCreate(e);
    e.w32(L(e) + 0x188, uVar2);
    let fVar3 = APIextra::TimerGetTime(e, 0x9000_0000);
    APIstdlib::srand(e, (fVar3 as i32).wrapping_add(instanceCount) as u32);
    let fVar3 = LIBApiCFunctions::FloatRandom(e, 0.0, 1.0, 0.05);
    e.wf(L(e) + 0x1f8, fVar3 + 4.5);
    e.w32(L(e) + 0x194, 0);
    APIcons::SetMessageFunction(e, ZombieBossMessageFonction);
    Monster_Base_Func_Initialise(e)?;
    ZombieBoss_Func_Initialisation(e)?;
    APIcons::SetInitialState(e, 5);
    Ok(())
}

/// 0x00018b80 (the particle types are shared: destroyed with the last instance)
pub fn zombiebossDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    let iVar1 = L(e);
    monster_baseDestructor(e)?;
    let t = e.r32(iVar1 + 0x184);
    APIextra::TimerDestroy(e, t);
    let t = e.r32(L(e) + 0x188);
    APIextra::TimerDestroy(e, t);
    if instanceCount == 0 {
        ZombieBossFUNC_DestroyTypes(e)?;
    }
    Ok(())
}

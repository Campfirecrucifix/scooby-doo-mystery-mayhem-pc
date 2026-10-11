//! Zombie_Base.c (the zombie AI). Addresses are module-relative (.text at 0x10000) in zombieleveltwoa.
//!
//! States: 0x10 ATTACK (anim done -> END_ATTACK), 0x11 FIRSTMOVE (zombie_base_Func_Initialisation),
//! 0x12 WAITING_LOOP, 0x13 SLEEP_SET, 0x14 SLEEP_LOOP, 0x15 chase (ATTACK), 0x16 DEFENSIVE_ATTACK_SET,
//! 0x17 WANDERING_SET, 0x18 WANDERING_LOOP, 0x1b SCARED_SET (book open), 0x1c SCARED_LOOP,
//! 0x1d SCARED_NOTHOOKED_SET, 0x1e SCARED_NOTHOOKED_LOOP, 0x1f DEFENSIVE_ATTACK_LOOP,
//! 0x20 RETURN_TO_START, 0x21 FIRSTMOVE_BOSS, 0x22 RESPAWN_BOSS (see zombie_baseConstructor for the
//! table). Local data (L = pLocalDatazombie_base, the Monster_Base block grown to 0x1f8):
//! +0x194 behaviour (1 sleep, 2 wander, 3 guard, 4 teleport home), +0x1a4 starting position,
//! +0x1cc "ignore the book" (boss), +0x184/+0x188 wander turn steps / direction.
//! Per-build literals (anim ids) come from [`super::builds`].
use super::builds::*;
use super::*;
use crate::engine::*;
use crate::logic::LResult;

/// `(*pPhysic->DMGetParams)(dm, b)` into a fresh 0x80-byte block
fn dm_params(e: &mut Engine, dm: u32) -> Ptr {
    let b = e.alloca(0x80);
    APIphysic::DMGetParams(e, dm, b);
    b
}

/// `GetPosition(SELF)` into a fresh vector
fn tmp_vec(e: &mut Engine) -> Ptr {
    e.alloca(0xc)
}

/// `Rotate(SELF, {0, 1, 0}, angle)`
fn rotate_y(e: &mut Engine, angle: f32) {
    let v = tmp_vec(e);
    e.wf(v, 0.0);
    e.wf(v + 4, 1.0);
    e.wf(v + 8, 0.0);
    APIgameobj::Rotate(e, SELF, v, angle);
}

/// the leader hero's object id (game buffer 0 +8)
fn hero(e: &Engine) -> u32 {
    e.r32(e.r32(GDM(e)) + 8)
}

/// `CheckRangeObject(r, SELF, hero, 0)`
fn hero_in_range(e: &mut Engine, r: f32) -> bool {
    let h = hero(e);
    LIBApiCFunctions::CheckRangeObject(e, r, SELF, h, 0) != 0
}

/// 0x00012e28 copy the hook radius, line of sight, notifications, physics flags and groups, remember
/// the starting position (L+0x1a4); first state 0x11.
pub fn zombie_base_Func_Initialisation(e: &mut Engine) -> LResult<()> {
    let iVar1 = L(e);
    let v = e.r32(L(e) + 0xb0);
    e.w32(L(e) + 0x5c, v);
    e.w32(iVar1 + 0x1cc, 0);
    InitInLineOfSight(e)?;
    e.w32(iVar1 + 0x1d0, 1);
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 0xb, 0x25);
    if e.r32(iVar1 + 8) == 0 {
        zombie_baseFUNC_Create_Physic(e)?;
    }
    APIphysic::ResetFlags(e, SELF, 1);
    APIphysic::SetFlags(e, SELF, 6);
    APIgameobj::SetGroupID(e, SELF, 2);
    APIgameobj::SetGroupBitMask(e, SELF, 0x200641);
    APIgameobj::SetContextFlag(e, SELF, 0x60);
    APIgameobj::GetPosition(e, SELF, ALL, iVar1 + 0x1a4);
    APIgameobj::SetNextState(e, SELF, 0x11);
    Ok(())
}

/// 0x00012fb8 register with the sector clipping (record at L+0x1ec), stop the move DM, check the hero
pub fn zombie_baseST_FIRSTMOVE_BOSS(e: &mut Engine, _: u32) -> LResult<u64> {
    let iStack_40 = L(e);
    let uVar1 = APIgameobj::GetID(e, SELF, NULL);
    e.w32(L(e) + 0x1ec, uVar1);
    e.w16(L(e) + 0x1f6, 2);
    e.w16(L(e) + 500, 0);
    let r = L(e) + 0x1ec;
    LIBApiSectorClipping::ApiImpl_Register(e, r);
    let dm = e.r32(iStack_40 + 4);
    let b = dm_params(e, dm);
    let t = k(e, zombie_baseST_FIRSTMOVE_BOSS_0_1);
    for (o, v) in [(0, t), (0x14, 1), (4, 0x40a0_0000), (8, 0), (0xc, 0), (0x10, 0)] {
        e.w32(b + o, v);
    }
    APIphysic::DMSetParams(e, dm, b);
    APIgameobj::SetContextFlag(e, SELF, 0x62);
    CheckHero(e)?;
    Ok(0)
}

/// 0x00013100 after 1.5 s on timer L+0x16c: chase the leader hero (state 0x15)
pub fn zombie_baseST_RESPAWN_BOSS(e: &mut Engine, _: u32) -> LResult<u64> {
    let t = e.r32(L(e) + 0x16c);
    let fVar1 = APIextra::TimerGetTime(e, t);
    if 1.5 < fVar1 {
        let iStack_30 = L(e);
        let dm = e.r32(L(e) + 4);
        let b = dm_params(e, dm);
        let h = hero(e);
        for (o, v) in [(0, h), (0x14, 5), (8, 0), (0xc, 0), (0x10, 0)] {
            e.w32(b + o, v);
        }
        let dm = e.r32(iStack_30 + 4);
        APIphysic::DMSetParams(e, dm, b);
        APIgameobj::SetNextState(e, SELF, 0x15);
    }
    Ok(0)
}

/// `Play(SELF, anim, 6, blend); L+0x38 = anim; SetContextFlag(ctx); SetNextState(next)`
fn play_set(e: &mut Engine, anim: u32, blend: f32, ctx: u32, next: u32) {
    let iVar1 = L(e);
    APIanim::Play(e, SELF, anim, 6, blend);
    e.w32(iVar1 + 0x38, anim);
    APIgameobj::SetContextFlag(e, SELF, ctx);
    APIgameobj::SetNextState(e, SELF, next);
}

/// 0x00013200 pick the idle behaviour from L+0x194: 2 wander, 1 sleep, 3 guard (return to start),
/// 4 teleport to the start and sleep.
pub fn zombie_baseST_WAITING_SET(e: &mut Engine, _: u32) -> LResult<u64> {
    match e.r32(L(e) + 0x194) {
        2 => play_set(e, 1, 0.2, 0x6c, 0x18),
        1 => {
            let a = k(e, zombie_baseST_WAITING_SET_0_1);
            debug_assert_eq!(a, k(e, zombie_baseST_WAITING_SET_0_2));
            play_set(e, a, 0.2, 0x60, 0x12)
        }
        3 => {
            let a = k(e, zombie_baseST_WAITING_SET_0_3);
            debug_assert_eq!(a, k(e, zombie_baseST_WAITING_SET_0_4));
            play_set(e, a, 0.4, 0x60, 0x20)
        }
        4 => {
            let p = L(e) + 0xf8;
            APIgameobj::SetTranslation(e, SELF, p);
            APIgameobj::SetNextState(e, SELF, 0x13);
        }
        _ => {}
    }
    Ok(0)
}

/// 0x00013440 guard: now and then (1 in 400) start the walk anim; check the hero
pub fn zombie_baseST_WAITING_LOOP(e: &mut Engine, _: u32) -> LResult<u64> {
    let iVar1 = L(e);
    let a = k(e, zombie_baseST_WAITING_LOOP_0_1);
    if e.r32(L(e) + 0x194) == 3 && e.r32(L(e) + 0x38) != a {
        let uVar2 = APIstdlib::MyRand(e);
        let iStack_40 = (uVar2 as i32) % 400;
        if iStack_40 == 0x40 {
            let a = k(e, zombie_baseST_WAITING_LOOP_0_2);
            e.w32(iVar1 + 0x38, a);
            APIgameobj::SetContextFlag(e, SELF, 0x60);
            let a = k(e, zombie_baseST_WAITING_LOOP_0_3) as i32;
            LIBAnimMngr::PlayAnim(e, 0.4, a, 1, 0x24);
        }
    }
    let DStack_50 = tmp_vec(e);
    APIgameobj::GetPosition(e, SELF, ALL, DStack_50);
    e.w32(iVar1 + 0x34, 0);
    CheckHero(e)?;
    Ok(0)
}

/// 0x00013598
pub fn zombie_baseST_SLEEP_SET(e: &mut Engine, _: u32) -> LResult<u64> {
    let a = k(e, zombie_baseST_SLEEP_SET_0_1);
    debug_assert_eq!(a, k(e, zombie_baseST_SLEEP_SET_0_2));
    play_set(e, a, 0.2, 0x60, 0x14);
    Ok(0)
}

/// 0x00013650
pub fn zombie_baseST_SLEEP_LOOP(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

/// 0x00013680 chase: book open -> scared (0x1b) unless a boss; guard on a slope floor (attribute
/// 0x1000000) turns on the spot; hero within 1 m -> attack (0x10) or defend (0x16 if the last
/// state was 0x15 already this frame's chase, L+0x34 == 3); hero beyond 6 m -> wander / go back.
pub fn zombie_baseST_ATTACK(e: &mut Engine, _: u32) -> LResult<u64> {
    let iVar1 = L(e);
    let iVar3 = IsTomeOpen(e)?;
    if iVar3 == 1 && e.r32(iVar1 + 0x1cc) == 0 {
        APIgameobj::SetNextState(e, SELF, 0x1b);
        return Ok(0);
    }
    if e.r32(iVar1 + 0x194) == 3 {
        let a = e.alloca(0x20);
        APIphysic::GetFloorAttribs(e, SELF, a);
        if e.r32(a + 4) & 0x100_0000 == 0 {
            APIgameobj::SetContextFlag(e, SELF, 0x66);
        } else {
            rotate_y(e, -0.017453292);
            APIgameobj::SetContextFlag(e, SELF, 0x6c);
        }
    }
    if hero_in_range(e, 1.0) && e.r32(e.r32(GDZ(e)) + 0x14) & 0x4900_094c == 0 {
        if e.r32(iVar1 + 0x34) == 3 {
            APIgameobj::SetNextState(e, SELF, 0x16);
            return Ok(0);
        }
        APIgameobj::SetNextState(e, SELF, 0x10);
        APIgameobj::SetContextFlag(e, SELF, 0x62);
        let a = k(e, zombie_baseST_ATTACK_0_1);
        LIBAnimMngr::PlayAnim(e, 0.4, a as i32, 1, 0x16);
        let a = k(e, zombie_baseST_ATTACK_0_2);
        e.w32(iVar1 + 0x38, a);
    } else if hero_in_range(e, 6.0) || e.r32(iVar1 + 0x194) == 3 {
        if hero_in_range(e, 6.0) || e.r32(iVar1 + 0x194) != 3 {
            if e.r32(iVar1 + 0x38) != 1 {
                TrackHero(e)?;
            }
        } else {
            APIanim::Play(e, SELF, 1, 6, 0.4);
            e.w32(iVar1 + 0x38, 1);
            APIgameobj::SetContextFlag(e, SELF, 0x66);
            APIgameobj::SetNextState(e, SELF, 0x12);
        }
    } else {
        APIgameobj::SetContextFlag(e, SELF, 0x60);
        let a = k(e, zombie_baseST_ATTACK_0_3);
        e.w32(iVar1 + 0x38, a);
        let a = k(e, zombie_baseST_ATTACK_0_4);
        LIBAnimMngr::PlayAnim(e, 0.4, a as i32, 1, 0x24);
        APIgameobj::SetNextState(e, SELF, 0x17);
    }
    e.w32(iVar1 + 0x34, 3);
    Ok(0)
}

/// 0x00013ac0 unless returning to the start (0x20): idle anim, sleep 1 s, back to the chase
pub fn zombie_baseST_END_ATTACK(e: &mut Engine, _: u32) -> LResult<u64> {
    let iVar1 = L(e);
    let iVar2 = APIgameobj::GetCurrentStateID(e, SELF);
    if iVar2 != 0x20 {
        let a = k(e, zombie_baseST_END_ATTACK_0_1);
        APIanim::Play(e, SELF, a, 6, 0.4);
        let a = k(e, zombie_baseST_END_ATTACK_0_2);
        e.w32(iVar1 + 0x38, a);
        APIgameobj::SetContextFlag(e, SELF, 0x60);
        APIgameobj::Sleep(e, SELF, 1.0);
        e.w32(L(e) + 0x34, 4);
        APIgameobj::SetNextState(e, SELF, 0x15);
    }
    Ok(0)
}

/// 0x00013bc8 wander timer L+0x168 (3 s, notify 0x23), state 0x18
pub fn zombie_baseST_WANDERING_SET(e: &mut Engine, _: u32) -> LResult<u64> {
    e.w32(L(e) + 0x1a0, 0);
    let t = e.r32(L(e) + 0x168);
    APIextra::TimerReset(e, t);
    let t = e.r32(L(e) + 0x168);
    APIextra::TimerRegisterNotify(e, SELF, t, 0x23);
    let t = e.r32(L(e) + 0x168);
    APIextra::TimerSetFrequency(e, t, 3.0);
    APIgameobj::SetNextState(e, SELF, 0x18);
    Ok(0)
}

/// 0x00013c98 wander: book open -> scared (0x1b) unless a boss; finish a turn in progress (L+0x184
/// steps of 1 degree, L+0x188 1 left / 2 right), else roll a random turn or (1 in 3 of some rolls)
/// stop and go back to the start (0x17).
pub fn zombie_baseST_WANDERING_LOOP(e: &mut Engine, _: u32) -> LResult<u64> {
    let iVar1 = L(e);
    let iVar2 = IsTomeOpen(e)?;
    if iVar2 == 1 && e.r32(iVar1 + 0x1cc) == 0 {
        APIgameobj::SetNextState(e, SELF, 0x1b);
        return Ok(0);
    }
    CheckHero(e)?;
    if e.r32(iVar1 + 0x184) != 0 {
        if e.r32(iVar1 + 0x184) != 0 && e.r32(iVar1 + 0x188) == 1 {
            rotate_y(e, 0.017453292);
            let n = e.r32(iVar1 + 0x184).wrapping_sub(1);
            e.w32(iVar1 + 0x184, n);
            if e.r32(iVar1 + 0x184) != 0 {
                return Ok(0);
            }
            e.w32(iVar1 + 0x184, 0);
            return Ok(0);
        }
        if e.r32(iVar1 + 0x184) == 0 || e.r32(iVar1 + 0x188) != 2 {
            return Ok(0);
        }
        rotate_y(e, -0.017453292);
        let n = e.r32(iVar1 + 0x184).wrapping_sub(1);
        e.w32(iVar1 + 0x184, n);
        if e.r32(iVar1 + 0x184) != 0 {
            return Ok(0);
        }
        e.w32(iVar1 + 0x184, 0);
        return Ok(0);
    }
    let uVar3 = APIstdlib::MyRand(e);
    let uVar3 = ((uVar3 as i32) % 200) as u32;
    let (steps, dir) = match uVar3 {
        0x82 | 0x1e => (0x1e, 1),
        1 | 0x65 => (0x1e, 2),
        0x8c => (0x5a, 2),
        0x96 => (0x5a, 1),
        0xc6 => (0x78, 2),
        199 => (0x78, 1),
        10 | 0x46 | 0xb1 => {
            // LAB_00013eec
            let uVar3 = APIstdlib::MyRand(e);
            if (uVar3 as i32) % 3 != 2 {
                return Ok(0);
            }
            e.w32(iVar1 + 0x184, 0);
            e.w32(iVar1 + 0x188, 0);
            let a = k(e, zombie_baseST_WANDERING_LOOP_0_1);
            e.w32(iVar1 + 0x38, a);
            APIgameobj::SetContextFlag(e, SELF, 0x60);
            let a = k(e, zombie_baseST_WANDERING_LOOP_0_2) as i32;
            LIBAnimMngr::PlayAnim(e, 0.4, a, 1, 0x24);
            APIgameobj::SetNextState(e, SELF, 0x17);
            return Ok(0);
        }
        _ => return Ok(0),
    };
    e.w32(iVar1 + 0x184, steps);
    e.w32(iVar1 + 0x188, dir);
    Ok(0)
}

/// 0x00014128 the book opened: scared anim, defensive attack in 3 s, turn around, back away from the
/// book holder (move DM mode 0x11 on k ID_2a000045), stop the gas particles; state 0x1a.
pub fn zombie_baseST_SCARED_SET(e: &mut Engine, _: u32) -> LResult<u64> {
    let iVar1 = L(e);
    // DStack_40 = {0.0, 1.0, 0.0} (DAT_00010ee8 / DAT_00010ef0)
    let DStack_40 = tmp_vec(e);
    e.wf(DStack_40, 0.0);
    e.wf(DStack_40 + 4, 1.0);
    e.wf(DStack_40 + 8, 0.0);
    APIanim::Play(e, SELF, 2, 6, 0.2);
    e.w32(iVar1 + 0x38, 2);
    SetDefensiveAttack(e, 3.0)?;
    let v = e.r32(L(e) + 0x1a0) & 0xffff_fffb;
    e.w32(L(e) + 0x1a0, v);
    APIgameobj::Rotate(e, SELF, DStack_40, 3.1415927);
    let dm = e.r32(L(e) + 4);
    let b = dm_params(e, dm);
    let book = k(e, ID_2a000045);
    e.w32(b, book);
    e.w32(b + 0x14, 0x11);
    APIphysic::DMSetParams(e, dm, b);
    let dm = e.r32(L(e) + 8);
    let b = dm_params(e, dm);
    e.w32(b, 0);
    e.w32(b + 4, 0);
    e.w32(b + 8, 0x3fc0_0000);
    APIphysic::DMSetParams(e, dm, b);
    APIgameobj::SetContextFlag(e, SELF, 0x66);
    Particles_Stop_Gaz(e)?;
    Particles_Stop_Grumeaux(e)?;
    APIgameobj::SetNextState(e, SELF, 0x1a);
    Ok(0)
}

/// 0x00014348 the book closed: walk back towards the hero (0x17), or guard (0x12); defensive attack
/// timer fired (L+0x1a0 bit 2) -> 0x1d.
pub fn zombie_baseST_SCARED_LOOP(e: &mut Engine, _: u32) -> LResult<u64> {
    let iVar1 = L(e);
    let iVar2 = IsTomeOpen(e)?;
    if iVar2 == 0 && e.r32(iVar1 + 0x1cc) == 0 && e.r32(iVar1 + 0x194) != 3 {
        APIgameobj::SetContextFlag(e, SELF, 0x60);
        let a = k(e, zombie_baseST_SCARED_LOOP_0_1);
        e.w32(iVar1 + 0x38, a);
        let a = k(e, zombie_baseST_SCARED_LOOP_0_2) as i32;
        LIBAnimMngr::PlayAnim(e, 0.0, a, 1, 0x24);
        let dm = e.r32(iVar1 + 4);
        let b = dm_params(e, dm);
        let sp = e.r32(iVar1 + 0x84);
        let h = hero(e);
        for (o, v) in [(4, sp), (0, h), (8, 0), (0xc, 0), (0x10, 0), (0x14, 5)] {
            e.w32(b + o, v);
        }
        APIphysic::DMSetParams(e, dm, b);
        APIgameobj::SetNextState(e, SELF, 0x17);
    } else {
        let iVar2 = IsTomeOpen(e)?;
        if iVar2 == 0 && e.r32(iVar1 + 0x1cc) == 0 && e.r32(iVar1 + 0x194) == 3 {
            APIgameobj::SetNextState(e, SELF, 0x12);
        } else if e.r32(L(e) + 0x1a0) & 4 != 0 {
            let v = e.r32(L(e) + 0x1a0) & 0xffff_fffb;
            e.w32(L(e) + 0x1a0, v);
            APIgameobj::SetNextState(e, SELF, 0x1d);
        }
    }
    Ok(0)
}

/// 0x00014570
pub fn zombie_baseST_SCARED_NOTHOOKED_SET(e: &mut Engine, _: u32) -> LResult<u64> {
    let iVar1 = L(e);
    let t = e.r32(L(e) + 0x174);
    APIextra::TimerReset(e, t);
    e.w32(iVar1 + 0x184, 0);
    e.w32(iVar1 + 0x38, 0xffff_ffff);
    APIgameobj::SetNextState(e, SELF, 0x1c);
    Ok(0)
}

/// 0x00014608 the book is open but the beam missed: wander about (turning L+0x1dc degrees a frame)
/// until it closes (0x17); a boss goes to 0x19 instead.
pub fn zombie_baseST_SCARED_NOTHOOKED_LOOP(e: &mut Engine, _: u32) -> LResult<u64> {
    let iVar2 = IsTomeOpen(e)?;
    let iVar1 = L(e);
    if iVar2 == 0 {
        APIgameobj::SetNextState(e, SELF, 0x17);
    } else if iVar2 == 0 || e.r32(L(e) + 0x1cc) == 0 {
        let iVar2 = e.r32(L(e) + 0x184);
        e.w32(L(e) + 0x184, iVar2.wrapping_sub(1));
        if iVar2 == 0 {
            GetNextWanderMove(e)?;
            SetNextWanderMove(e)?;
        } else {
            let a = e.rf(iVar1 + 0x1dc) * 0.017453292;
            rotate_y(e, a);
        }
    } else {
        APIgameobj::SetNextState(e, SELF, 0x19);
    }
    Ok(0)
}

/// 0x00014780
pub fn zombie_baseST_DEFENSIVE_ATTACK_SET(e: &mut Engine, _: u32) -> LResult<u64> {
    let a = k(e, zombie_baseST_DEFENSIVE_ATTACK_SET_0_1) as i32;
    LIBAnimMngr::PlayAnim(e, 0.4, a, 1, 0x24);
    APIgameobj::SetContextFlag(e, SELF, 0x60);
    APIgameobj::SetNextState(e, SELF, 0x1e);
    Ok(0)
}

/// 0x00014820 attack anim done (L+0x1a0 bit 1): turn around, state 0x19
pub fn zombie_baseST_DEFENSIVE_ATTACK_LOOP(e: &mut Engine, _: u32) -> LResult<u64> {
    if e.r32(L(e) + 0x1a0) & 2 != 0 {
        rotate_y(e, 3.1415927);
        APIgameobj::SetNextState(e, SELF, 0x19);
        let v = e.r32(L(e) + 0x1a0) & 0xffff_fffd;
        e.w32(L(e) + 0x1a0, v);
    }
    Ok(0)
}

/// 0x000148f0
pub fn Monster_BaseST_IN_CUT_LOOP(e: &mut Engine, _: u32) -> LResult<u64> {
    let t = e.r32(L(e) + 0x160);
    APIextra::TimerReset(e, t);
    Ok(0)
}

/// 0x00014950
pub fn zombie_baseST_BASICMOVE_LOOP(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

/// 0x00014980
pub fn Monster_BaseST_REINITIALISE(e: &mut Engine, _: u32) -> LResult<u64> {
    zombie_base_Func_Initialisation(e)?;
    Ok(0)
}

/// 0x000149d0 within 7 m of the starting position (L+0x1a4): walk anim, stop the wander timer, 0x17
pub fn zombie_baseST_RETURN_TO_START(e: &mut Engine, _: u32) -> LResult<u64> {
    CheckHero(e)?;
    let p = L(e) + 0x1a4;
    let bVar1 = LIBApiCFunctions::CheckRangePoint(e, 7.0, SELF, p, 0);
    if bVar1 != 0 {
        APIgameobj::SetContextFlag(e, SELF, 0x60);
        let a = k(e, zombie_baseST_RETURN_TO_START_0_1);
        e.w32(L(e) + 0x38, a);
        let a = k(e, zombie_baseST_RETURN_TO_START_0_2) as i32;
        LIBAnimMngr::PlayAnim(e, 0.0, a, 1, 0x24);
        let t = e.r32(L(e) + 0x168);
        APIextra::TimerUnregisterNotify(e, SELF, t);
        APIgameobj::SetNextState(e, SELF, 0x17);
    }
    Ok(0)
}

/// 0x00014af0 zombie messages, then Monster_Base's. 0xc9 restart (back to state 0, 0xeb to the
/// level manager), 0xc1 guard back at its post, 199 pause 1 s, 0xe6 / 0xd6-0x67 into the book's black
/// hole (state 5), 0xd6 from the book: 100 link broken / 0x65 hooked (L+0x1cc = 0 / 1, state 0x19
/// SCARED), 0x68 captured (anim). Group 1 (Ep6 builds) and group 2 (Ep4 level 4) also play the hooked /
/// pulled-in sounds; group 2 forwards 199 to the object 0x2a0000c7.
pub fn zombie_baseMessageFonction(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    let grp = g(e, G_zombie_baseMessageFonction);
    // sounds: the emitter is the build's ID_2a000043 slot, the pulled-in sound sits in slot 0
    // (ID_2a000000) for group 1; group 2's three builds share their literals.
    let (snd_obj, snd_hooked, snd_pulled) = match grp {
        1 => (k(e, ID_2a000043), k(e, zombie_baseMessageFonction_1_2), k(e, ID_2a000000)),
        _ => (k(e, ID_2a000043), 0x8000_002e, 0x8000_0036),
    };
    let iVar1 = L(e);
    if param_1 == 0xc9 {
        let t = e.r32(L(e) + 0x168);
        APIextra::TimerUnregisterNotify(e, SELF, t);
        APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 3, 0xc);
        let m = e.r32(LM(e) + 0x4c);
        APIgameobj::SetGroupBitMask(e, SELF, m);
        let to = k(e, ID_2a0000cc);
        APIgameobj::SendMessageTo(e, to, NULL, 0xeb, 1, NULL);
        let id = e.r32(LM(e) + 0x48);
        APIgameobj::SetGroupID(e, SELF, id);
        APIgameobj::SetGameObjectVisible(e, SELF, 1);
        APIgameobj::Sleep(e, SELF, 0.0);
        APIgameobj::SetNextState(e, SELF, 0);
    } else if param_1 < 0xca {
        if param_1 != 0xa4 && 0xa4 < param_1 {
            if param_1 == 0xc1 {
                if e.r32(L(e) + 0x194) == 3 {
                    let p = L(e) + 0x1a4;
                    APIgameobj::SetTranslation(e, SELF, p);
                    APIgameobj::SetGameObjectVisible(e, SELF, 1);
                    return Ok(0);
                }
            } else if param_1 == 199 {
                APIgameobj::Sleep(e, SELF, 1.0);
                if grp == 2 {
                    APIgameobj::SendMessageTo(e, 0x2a00_00c7, NULL, 199, 0, NULL);
                }
            }
        }
    } else if param_1 == 0xe6 {
        APIgameobj::SetGroupBitMask(e, SELF, 0x200641);
        APIgameobj::SetNextState(e, SELF, 5);
    } else if param_1 < 0xe7 {
        if param_1 == 0xd6 {
            match param_2 {
                100 => {
                    e.w32(L(e) + 0x1cc, 0);
                    APIgameobj::SetNextState(e, SELF, 0x19);
                }
                0x65 => {
                    if grp != 0 {
                        APInewSound::Play(e, snd_obj, snd_hooked, 0);
                    }
                    e.w32(iVar1 + 0x1cc, 1);
                    APIgameobj::SetNextState(e, SELF, 0x19);
                }
                0x67 => {
                    APIphysic::ResetFlags(e, SELF, 6);
                    APIphysic::SetFlags(e, SELF, 1);
                    Particles_Stop_Gaz(e)?;
                    Particles_Stop_Grumeaux(e)?;
                    APIgameobj::SetNextState(e, SELF, 5);
                    if grp != 0 {
                        APInewSound::Play(e, snd_obj, snd_pulled, 0);
                    }
                }
                0x68 => {
                    let a = k(e, zombie_baseMessageFonction_0_2);
                    APIanim::Play(e, SELF, a, 2, 0.0);
                }
                _ => {}
            }
        } else if param_1 == 0xde {
            APIgameobj::SetNextState(e, SELF, 0x17);
            return Ok(0);
        }
    } else if param_1 == 0x201 {
        APIgameobj::EnableObject(e, SELF, 1);
    } else if param_1 == 0x202 {
        APIgameobj::EnableObject(e, SELF, 0);
    }
    Monster_Base_MessageFunction(e, param_1, param_2, param_3)
}

/// 0x00015038 callback 0xc. Group 1 (Ep4 level 4): touching object 0x2a000008 / 0x2a000009 sends 199
/// to 0x2a0000c7. `param_1` = the collision record (+8 the other object).
pub fn Monster_BaseCB_NOTIFY_COLLIDE_OBJ(e: &mut Engine, param_1: u32) -> LResult<u64> {
    if g(e, G_Monster_BaseCB_NOTIFY_COLLIDE_OBJ) == 1 {
        let o = e.r32(param_1 + 8);
        if o < 0x2a00_000a && 0x2a00_0007 < o {
            APIgameobj::SendMessageTo(e, 0x2a00_00c7, NULL, 199, 0, NULL);
        }
    }
    Ok(0)
}

/// 0x000150c8 callback 0x24 (anim finished `param_1`): the wake-up anim -> wander (not a guard);
/// the defensive attack -> L+0x1a0 bit 1; any other -> walk and chase (0x15).
pub fn zombie_baseCB_ANIM_LF(e: &mut Engine, param_1: u32) -> LResult<u64> {
    let a3 = k(e, zombie_baseCB_ANIM_LF_0_1);
    let a8 = k(e, zombie_baseCB_ANIM_LF_0_2);
    if param_1 == a3 && e.r32(L(e) + 0x194) != 3 {
        e.w32(L(e) + 0x38, 1);
        LIBAnimMngr::PlayAnim(e, 0.4, 1, 2, 0);
        APIgameobj::SetContextFlag(e, SELF, 0x6c);
        APIgameobj::SetNextState(e, SELF, 0x17);
    } else if param_1 == a8 {
        or32(e, L(e) + 0x1a0, 2);
    } else {
        e.w32(L(e) + 0x38, 1);
        LIBAnimMngr::PlayAnim(e, 0.4, 1, 2, 0);
        APIgameobj::SetContextFlag(e, SELF, 0x66);
        APIgameobj::SetNextState(e, SELF, 0x15);
    }
    Ok(0)
}

/// 0x00015250 callback 0x25 (anim event `{anim, type, value}`): the defensive attack anim spews gas
/// and lumps (value 0 start, else stop)
pub fn zombie_baseCB_ANIM(e: &mut Engine, param_1: u32) -> LResult<u64> {
    if e.r32(param_1) == k(e, zombie_baseCB_ANIM_0_1) {
        if e.r32(param_1 + 8) == 0 {
            Particles_Start_Gaz(e)?;
            Particles_Start_Grumeaux(e)?;
        } else {
            Particles_Stop_Gaz(e)?;
            Particles_Stop_Grumeaux(e)?;
        }
    }
    Ok(0)
}

/// 0x00015318 callback 0x26 (the ray DM L+0x18 changed state): on a hit (state 2) turn away from the
/// wall normal; reset the wander turn (L+0x184/0x188). The cleared state is only written to the
/// local copy (no DMSetParams), as in the original.
pub fn zombie_baseDM_RAY_INTERSCT(e: &mut Engine, _: u32) -> LResult<u64> {
    let dm = e.r32(L(e) + 0x18);
    let auStack_c0 = dm_params(e, dm);
    if e.r32(auStack_c0 + 0x2c) == 2 {
        let DStack_50 = tmp_vec(e);
        e.wf(DStack_50, 0.0);
        e.wf(DStack_50 + 4, 1.0);
        e.wf(DStack_50 + 8, 0.0);
        let DStack_40 = tmp_vec(e);
        APIgameobj::GetLookRight(e, SELF, ALL, DStack_40);
        let mut fStack_30 = APImath::CVectorGetAngleTo(e, auStack_c0 + 0x3c, DStack_40);
        if fStack_30 < 1.5707964 {
            fStack_30 = 3.1415927 - fStack_30;
        } else {
            fStack_30 = (3.1415927 - fStack_30) * -1.0;
        }
        e.w32(auStack_c0 + 0x2c, 0);
        APIgameobj::Rotate(e, SELF, DStack_50, fStack_30);
    }
    e.w32(L(e) + 0x184, 0);
    e.w32(L(e) + 0x188, 0);
    Ok(0)
}

/// 0x00015490 callback 0x23 (timers): wander timer -> CheckStartingPos; L+0x170 -> L+0x1a0 bit 2
pub fn zombie_baseCB_TIMER(e: &mut Engine, param_1: u32) -> LResult<u64> {
    if param_1 == e.r32(L(e) + 0x168) {
        CheckStartingPos(e)?;
    } else if param_1 == e.r32(L(e) + 0x170) {
        or32(e, L(e) + 0x1a0, 4);
    }
    Ok(0)
}

/// 0x00015538 hero within 6 m and not hidden / busy (game buffer 0 +0x14 & 0x4900097c): chase (0x15)
pub fn CheckHero(e: &mut Engine) -> LResult<()> {
    let bVar1 = hero_in_range(e, 6.0);
    if bVar1 && e.r32(e.r32(GDZ(e)) + 0x14) & 0x4900_097c == 0 {
        let t = e.r32(L(e) + 0x168);
        APIextra::TimerUnregisterNotify(e, SELF, t);
        TrackHero(e)?;
        APIgameobj::SetNextState(e, SELF, 0x15);
    }
    Ok(())
}

/// 0x00015618 wandered more than 15 m from the start: walk back to it (move DM to the point, 0x1f)
pub fn CheckStartingPos(e: &mut Engine) -> LResult<()> {
    let p = L(e) + 0x1a4;
    let bVar1 = LIBApiCFunctions::CheckRangePoint(e, 15.0, SELF, p, 0) != 0;
    if !bVar1 {
        let dm = e.r32(L(e) + 4);
        let b = dm_params(e, dm);
        e.w32(b, 0);
        for i in 0..3 {
            let v = e.r32(L(e) + 0x1a4 + i * 4);
            e.w32(b + 8 + i * 4, v);
        }
        e.w32(b + 0x14, 5);
        let dm = e.r32(L(e) + 4);
        APIphysic::DMSetParams(e, dm, b);
        APIgameobj::SetContextFlag(e, SELF, 0x66);
        APIgameobj::SetNextState(e, SELF, 0x1f);
    }
    Ok(())
}

/// 0x00015730 start the defensive-attack timer L+0x170 (`param_1` s, notify 0x23)
pub fn SetDefensiveAttack(e: &mut Engine, param_1: f32) -> LResult<()> {
    let t = e.r32(L(e) + 0x170);
    APIextra::TimerRegisterNotify(e, SELF, t, 0x23);
    APIextra::TimerSetFrequency(e, t, param_1);
    APIextra::TimerReset(e, t);
    Ok(())
}

/// 0x000157c8 the two particle types (once per module: global data +0x10 / +0x14, -1 = none)
pub fn CreateParticlesTypes(e: &mut Engine) -> LResult<()> {
    let iVar1 = GDZ(e);
    if e.ri32(GDZ(e) + 0x10) == -1 {
        let pp = e.r32(G(e, pParticle));
        let types: [(u32, &str, fn(&mut Engine, u32, u32) -> LResult<()>); 2] =
            [(0x10, "Gaz", SetParticle_Gaz), (0x14, "Grumeaux", SetParticle_Grumeaux)];
        for (o, name, set) in types {
            let uVar2 = APIparticle::TypeCreate(e);
            e.w32(iVar1 + o, uVar2);
            let t = e.ri32(iVar1 + o);
            let tex = e.cstr("dust2");
            APIparticle::TypeSetTexture(e, t, tex);
            let n = e.cstr(name);
            APIparticle::TypeSetName(e, t, n);
            let t = e.r32(iVar1 + o);
            set(e, pp, t)?;
        }
    }
    Ok(())
}

/// 0x00015928
pub fn DestroyParticlesTypes(e: &mut Engine) -> LResult<()> {
    let iVar1 = GDZ(e);
    if e.ri32(GDZ(e) + 0x10) != -1 {
        let t = e.ri32(iVar1 + 0x10);
        APIparticle::TypeDestroy(e, t);
        let t = e.ri32(iVar1 + 0x14);
        APIparticle::TypeDestroy(e, t);
    }
    Ok(())
}

/// 0x000159b0 this zombie's gas (L+0x1e4) and lump (L+0x1e8) emitters: on every joint, floor
/// collision plane 0.75 / 0.5 m up, children of the object
pub fn CreateParticlesEmitters(e: &mut Engine) -> LResult<()> {
    let iVar2 = GDZ(e);
    let iVar1 = L(e);
    for (to, eo, y) in [(0x10u32, 0x1e4u32, 0.75f32), (0x14, 0x1e8, 0.5)] {
        let t = e.ri32(iVar2 + to);
        let uVar3 = APIparticle::EmitterCreate(e, t);
        e.w32(iVar1 + eo, uVar3);
        let pp = e.ri32(G(e, pParticle));
        let t = e.r32(iVar2 + to);
        LIBMiscFunc::MiscFunc_EmitterUpdateFlags(e, pp, t, 0x80, 1);
        let em = e.ri32(iVar1 + eo);
        APIparticle::EmitterSetJoint(e, em, SELF, 0x6fff_ffff);
        let DStack_40 = tmp_vec(e);
        e.wf(DStack_40, 0.0);
        e.wf(DStack_40 + 4, 1.0);
        e.wf(DStack_40 + 8, 0.0);
        let DStack_30 = tmp_vec(e);
        e.wf(DStack_30, 0.0);
        e.wf(DStack_30 + 4, y);
        e.wf(DStack_30 + 8, 0.0);
        APIparticle::EmitterSetCollisionPlane(e, em, 1, DStack_40, DStack_30, 0.5);
        let em = e.r32(iVar1 + eo);
        APIgameobj::AddChild(e, SELF, 0x240_1000, em, 1);
    }
    Ok(())
}

/// 0x00015bf0
pub fn DestroyParticlesEmitters(e: &mut Engine) -> LResult<()> {
    let iVar1 = L(e);
    let em = e.ri32(iVar1 + 0x1e4);
    APIparticle::EmitterDestroy(e, em);
    let em = e.ri32(iVar1 + 0x1e8);
    APIparticle::EmitterDestroy(e, em);
    Ok(())
}

/// 0x000162d8
pub fn Particles_Start_Gaz(e: &mut Engine) -> LResult<()> {
    let em = e.ri32(L(e) + 0x1e4);
    APIparticle::EmitterSetFlags(e, em, 0x24);
    APIparticle::EmitterStart(e, em, 1);
    Ok(())
}

/// 0x00016358
pub fn Particles_Start_Grumeaux(e: &mut Engine) -> LResult<()> {
    let em = e.ri32(L(e) + 0x1e8);
    APIparticle::EmitterStart(e, em, 1);
    Ok(())
}

/// 0x000163b8
pub fn Particles_Stop_Gaz(e: &mut Engine) -> LResult<()> {
    let em = e.ri32(L(e) + 0x1e4);
    APIparticle::EmitterStop(e, em);
    Ok(())
}

/// 0x00016410
pub fn Particles_Stop_Grumeaux(e: &mut Engine) -> LResult<()> {
    let em = e.ri32(L(e) + 0x1e8);
    APIparticle::EmitterStop(e, em);
    Ok(())
}

/// `MyRand() % 100 < 50 ? 1.0 : -1.0` as bits
fn rand_sign(e: &mut Engine) -> u32 {
    let uVar3 = APIstdlib::MyRand(e);
    if (uVar3 as i32).wrapping_rem(100) < 0x32 { 0x3f80_0000 } else { 0xbf80_0000 }
}

/// 0x00016468 next wander turn: hero within 5 m -> turn away from him (L+0x184 = steps, L+0x1dc =
/// direction +-1 deg); else 11 %: stand still 40..119 steps (direction 0), else turn 30..159 steps.
/// Ghidra's operator order `a - b * 57.29578` is the instruction order and is kept.
pub fn GetNextWanderMove(e: &mut Engine) -> LResult<()> {
    let iVar1 = L(e);
    if hero_in_range(e, 5.0) {
        let DStack_90 = tmp_vec(e);
        let DStack_a0 = tmp_vec(e);
        let DStack_80 = tmp_vec(e);
        APIgameobj::GetPosition(e, SELF, ALL, DStack_90);
        let h = hero(e);
        APIgameobj::GetPosition(e, h, ALL, DStack_a0);
        APIgameobj::GetLookAt(e, SELF, ALL, DStack_80);
        let fStack_70 = e.rf(DStack_a0) - e.rf(DStack_90);
        let fStack_68 = e.rf(DStack_a0 + 8) - e.rf(DStack_90 + 8);
        let fVar5 = APImath::atan2f(e, fStack_70, fStack_68);
        let (lx, lz) = (e.rf(DStack_80), e.rf(DStack_80 + 8));
        let fVar6 = APImath::atan2f(e, lx, lz);
        let fStack_60 = fVar5 - fVar6 * 57.29578;
        let iStack_5c = (fStack_60 as i32).wrapping_rem(0xb4) + 0xb4;
        let uVar3 = APIstdlib::MyRand(e);
        e.w32(iVar1 + 0x184, ((uVar3 as i32).wrapping_rem(0x14) + iStack_5c) as u32);
        let s = rand_sign(e);
        e.w32(iVar1 + 0x1dc, s);
    } else {
        let uVar3 = APIstdlib::MyRand(e);
        if ((uVar3 as i32).wrapping_rem(100) as u32) < 0xb {
            let uVar3 = APIstdlib::MyRand(e);
            e.w32(iVar1 + 0x184, ((uVar3 as i32).wrapping_rem(0x50) + 0x28) as u32);
            e.w32(iVar1 + 0x1dc, 0);
        } else {
            let uVar3 = APIstdlib::MyRand(e);
            e.w32(iVar1 + 0x184, ((uVar3 as i32).wrapping_rem(0x82) + 0x1e) as u32);
            let s = rand_sign(e);
            e.w32(iVar1 + 0x1dc, s);
        }
    }
    Ok(())
}

/// 0x000167d0 direction 0 -> idle anim; else walk anim; turn by L+0x1dc degrees
pub fn SetNextWanderMove(e: &mut Engine) -> LResult<()> {
    let iVar1 = L(e);
    let idle = k(e, SetNextWanderMove_0_1);
    debug_assert_eq!(idle, k(e, SetNextWanderMove_0_2));
    let dir = e.rf(L(e) + 0x1dc);
    if dir == 0.0 && e.r32(L(e) + 0x38) != idle {
        e.w32(L(e) + 0x38, idle);
        APIgameobj::SetContextFlag(e, SELF, 0x60);
        let a = k(e, SetNextWanderMove_0_3) as i32;
        LIBAnimMngr::PlayAnim(e, 0.4, a, 1, 0);
    } else if dir != 0.0 && e.r32(L(e) + 0x38) != 1 {
        APIanim::Play(e, SELF, 1, 6, 0.2);
        e.w32(iVar1 + 0x38, 1);
        APIgameobj::SetContextFlag(e, SELF, 0x6c);
    }
    let f = e.rf(iVar1 + 0x1dc) * 0.017453292;
    rotate_y(e, f);
    Ok(())
}

/// 0x00016960 the book is open (game buffer 0 +0x14 bit 0) and the hero within 20 m
pub fn IsTomeOpen(e: &mut Engine) -> LResult<u32> {
    if e.r32(e.r32(GDZ(e)) + 0x14) & 1 == 0 || !hero_in_range(e, 20.0) {
        Ok(0)
    } else {
        Ok(1)
    }
}

/// 0x000169f8 walk anim; move DM L+4 to the leader hero
pub fn TrackHero(e: &mut Engine) -> LResult<()> {
    let iStack_30 = L(e);
    APIanim::Play(e, SELF, 1, 6, 0.2);
    e.w32(iStack_30 + 0x38, 1);
    let dm = e.r32(iStack_30 + 4);
    let b = dm_params(e, dm);
    let h = hero(e);
    for (o, v) in [(0, h), (0x14, 5), (8, 0), (0xc, 0), (0x10, 0)] {
        e.w32(b + o, v);
    }
    APIphysic::DMSetParams(e, dm, b);
    APIgameobj::SetContextFlag(e, SELF, 0x66);
    Ok(())
}

/// `dm = DMAdd(SELF, ty); L+slot = dm; DMGetParams; fields; DMSetParams; DMSetContextMask(ctx)`
fn add_dm(e: &mut Engine, slot: u32, ty: u32, fields: &[(u32, u32)], ctx: u32) -> u32 {
    let dm = APIphysic::DMAdd(e, SELF, ty);
    e.w32(L(e) + slot, dm);
    let b = dm_params(e, dm);
    for &(o, v) in fields {
        e.w32(b + o, v);
    }
    APIphysic::DMSetParams(e, dm, b);
    APIphysic::DMSetContextMask(e, dm, ctx);
    dm
}

/// 0x00016ae8 physics: prim 0xbe (a guard's slides on attribute 0x1000000 floors), weight 0.5,
/// priority 10; DMs: L+4 move to the hero (speed L+0x84, ctx 2), L+8 (type 4, L+0x98, ctx 4), L+0
/// (type 7, 20.0, ctx 1), L+0x24 gravity -20 (ctx 0x40), L+0x20 (type 8, 175/45, ctx 0x20), L+0x18 the
/// wall ray (type 0x1b, ctx 8, state change -> callback 0x26).
pub fn zombie_baseFUNC_Create_Physic(e: &mut Engine) -> LResult<()> {
    let p = L(e);
    APIphysic::SetFlags(e, SELF, 6);
    APIphysic::SetPrimaryPrim(e, SELF, 0xbe);
    APIphysic::SetWeight(e, SELF, 0.5);
    APIphysic::SetPriority(e, SELF, 10);
    if e.r32(p + 0x194) == 3 {
        APIphysic::PrimSetIntAttribBits(e, SELF, 0xbe, 5, 0x100_0000);
    }
    let h = hero(e);
    let sp = e.r32(p + 0x84);
    add_dm(e, 4, 1, &[(4, sp), (0, h), (8, 0), (0xc, 0), (0x10, 0), (0x14, 5)], 2);
    let v = e.r32(p + 0x98);
    add_dm(e, 8, 4, &[(0, 0), (4, 0), (8, v), (0xc, 1)], 4);
    add_dm(e, 0, 7, &[(0, 0x41a0_0000), (4, 5)], 1);
    add_dm(e, 0x24, 0, &[(0, 0), (4, 0xc1a0_0000), (8, 0)], 0x40);
    add_dm(e, 0x20, 8, &[(0, 0x432f_0000), (4, 0x4234_0000)], 0x20);
    let ray = [
        (0, 0), (4, 0x3f00_0000), (8, 0), (0xc, 0), (0x10, 0), (0x14, 0x3f40_0000),
        (0x20, 8), (0x18, 0), (0x1c, 0), (0x24, 0), (0x28, 0),
    ];
    let dm = add_dm(e, 0x18, 0x1b, &ray, 8);
    APIphysic::DMSetStateChangeNotifyBits(e, dm, 1);
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 6, 0x26);
    Ok(())
}

/// 0x00017010 the states / callbacks and API tables (first instance), the particle types, the
/// per-zombie record (goCustomDataZombie: +4 -> L+0x1e0, +0xc kill mask -> L+400 and L+0x40, +8
/// behaviour -> L+0x194; records 0..=k are searched), five timers (+ the respawn timer L+0x16c for a
/// guard), srand, the emitters; a dead zombie is killed. Returns AmIDead / KillObject's result.
/// Ghidra shows `srand((int)fVar4)` with fVar4 the float return register: it is TimerGetTime's
/// result (the calls in between return nothing in $f0), so the seed is the global timer.
pub fn zombie_baseConstructor(e: &mut Engine, api: Ptr, param_2: i32) -> LResult<i32> {
    monster_baseConstructor(e, api, param_2)?;
    if param_2 == 1 {
        let fns: [(i32, crate::logic::ExtFn); 23] = [
            (0x10, zombie_baseST_BASICMOVE_LOOP),
            (0x11, zombie_baseST_WAITING_SET),
            (0x12, zombie_baseST_WAITING_LOOP),
            (0x13, zombie_baseST_SLEEP_SET),
            (0x14, zombie_baseST_SLEEP_LOOP),
            (0x15, zombie_baseST_ATTACK),
            (0x16, zombie_baseST_END_ATTACK),
            (0x17, zombie_baseST_WANDERING_SET),
            (0x18, zombie_baseST_WANDERING_LOOP),
            (0x19, zombie_baseST_SCARED_SET),
            (0x1a, zombie_baseST_SCARED_LOOP),
            (0x1b, zombie_baseST_SCARED_NOTHOOKED_SET),
            (0x1c, zombie_baseST_SCARED_NOTHOOKED_LOOP),
            (0x1d, zombie_baseST_DEFENSIVE_ATTACK_SET),
            (0x1e, zombie_baseST_DEFENSIVE_ATTACK_LOOP),
            (0x1f, zombie_baseST_RETURN_TO_START),
            (0x20, zombie_baseST_FIRSTMOVE_BOSS),
            (0x21, zombie_baseST_RESPAWN_BOSS),
            (0, Monster_BaseST_REINITIALISE),
            (0x23, zombie_baseCB_TIMER),
            (0x24, zombie_baseCB_ANIM_LF),
            (0x25, zombie_baseCB_ANIM),
            (0x26, zombie_baseDM_RAY_INTERSCT),
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
        for (slot, name) in [(pFunction, "ApiCFunctions"), (pSectorClip, "ApiSectorClipping")] {
            let n = e.cstr(name);
            let v = APIcons::GetApiCustom(e, n);
            let a = G(e, slot);
            e.w32(a, v);
        }
        let slot = G(e, pGlobalDatazombie_base);
        APIcons::GetGlobalData(e, slot, 0x18);
        let uVar2 = APIgameobj::GetGameBuffer(e, 0, 0x8f8, 1);
        let gd = GDZ(e);
        e.w32(gd, uVar2);
        let uVar2 = APIgameobj::GetGameBuffer(e, 2, 0x158, 1);
        let gd = GDZ(e);
        e.w32(gd + 4, uVar2);
        e.w32(gd + 0xc, 0);
        e.w32(gd + 0x10, 0xffff_ffff);
        CreateParticlesTypes(e)?;
    }
    let slot = G(e, pLocalDatazombie_base);
    APIcons::GetLocalData(e, slot, 0x1f8);
    APIphysic::CreatePhysic(e, SELF, 0);
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 3, 0xc);
    let iVar3 = APIgameobj::GetID(e, SELF, NULL);
    let last = k(e, zombie_baseConstructor_0_1) as i32;
    let mut iStack_34: i32 = 0;
    while iStack_34 <= last {
        let r = G(e, goCustomDataZombie) + iStack_34 as u32 * 0x10;
        if e.r32(r) == iVar3 {
            let v = e.r32(r + 4);
            e.w32(L(e) + 0x1e0, v);
            let v = e.r32(r + 0xc);
            e.w32(L(e) + 400, v);
            e.w32(L(e) + 0x40, v);
            let v = e.r32(r + 8);
            e.w32(L(e) + 0x194, v);
            break;
        }
        iStack_34 += 1;
    }
    for o in [0x160, 0x168, 0x164, 0x170, 0x174] {
        let t = APIextra::TimerCreate(e);
        e.w32(L(e) + o, t);
    }
    if e.r32(L(e) + 0x194) == 3 {
        let t = APIextra::TimerCreate(e);
        e.w32(L(e) + 0x16c, t);
    }
    let fVar4 = APIextra::TimerGetTime(e, 0x9000_0000);
    APIstdlib::srand(e, fVar4 as i32 as u32);
    CreateParticlesEmitters(e)?;
    let mut iVar3 = AmIDead(e)? as i32;
    if iVar3 == 1 {
        iVar3 = APIgameobj::KillObject(e, SELF) as i32;
    }
    Ok(iVar3)
}

/// 0x000177d8 `param_1` = instances left (0: also the particle types)
pub fn zombie_baseDestructor(e: &mut Engine, param_1: i32) -> LResult<()> {
    monster_baseDestructor(e)?;
    if e.r32(L(e) + 0x194) == 3 {
        let t = e.r32(L(e) + 0x16c);
        APIextra::TimerDestroy(e, t);
    }
    for o in [0x164, 0x160, 0x168, 0x170, 0x174] {
        let t = e.r32(L(e) + o);
        APIextra::TimerDestroy(e, t);
    }
    DestroyParticlesEmitters(e)?;
    if param_1 == 0 {
        DestroyParticlesTypes(e)?;
    }
    Ok(())
}

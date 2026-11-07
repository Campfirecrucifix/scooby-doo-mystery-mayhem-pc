//! hero_core.c (17 functions). Addresses are module-relative (.text at 0x10000) in scooby__402420b4.
use super::builds::*;
use super::*;
use crate::engine::*;
use crate::logic::LResult;

/// 0x0001ce58 (164 bytes) `undefined8 heroST_FIRSTMOVE(void)`
pub fn heroST_FIRSTMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    let l = L(e);
    e.w32(l + 0x154, addr::heroInit_Stand);
    e.w32(l + 0x158, addr::heroST_Stand_Loop);
    e.w32(l + 0x15c, 0);
    heroST_ENTRY_POINT(e, 0)?;
    APIgameobj::SetNextState(e, SELF, 1);
    Ok(0)
}

/// 0x0001cf00 (672 bytes) `undefined8 heroST_ENTRY_POINT(void)`
pub fn heroST_ENTRY_POINT(e: &mut Engine, _: u32) -> LResult<u64> {
    // Builds differ only in which DoTestFor* functions take the local-data argument (the others read
    // pLocalDataHero themselves); the port always passes it.
    let l = L(e);
    heroFUNC_GetVelocity(e, l)?;
    heroFUNC_CheckControlEvents(e, l)?;
    heroFUNC_CheckActionEvents(e, l)?;
    if e.r32(l + 0x160) != 0 && e.r32(l + 0x154) == 0 {
        debug_info(e, "-- ST_ENTRY_POINT --- default state");
        let d = e.r32(l + 0x160);
        e.w32(l + 0x154, d);
    }
    if e.r32(l + 0x154) != 0 {
        let exit = e.r32(l + 0x15c);
        if exit != 0 {
            call_code(e, exit, l)?;
            e.w32(l + 0x15c, 0);
        }
        let init = e.r32(l + 0x154);
        call_code(e, init, l)?;
        e.w32(l + 0x160, 0);
        e.w32(l + 0x154, 0);
        e.w32(l + 0x164, 0);
    }
    e.w32(l + 0x174, 0);
    e.w32(l + 0x170, 0);
    let st = e.r32(l + 0x158);
    call_code(e, st, l)?;
    if leader(e) {
        heroDoTestForBump(e, l)?;
    }
    if leader(e) && e.r32(l + 0x158) != addr::heroST_Switch_Loop {
        heroDoTestForActionClimb(e, l)?;
        heroDoTestForActionCrouch(e, l)?;
        if e.r32(GB0(e) + 0x14) & 4 == 0 {
            heroDoTestForActionDiveOut(e)?;
            heroDoTestForActionDiveIn(e)?;
        }
        heroDoTestForOnFloor(e, l)?;
    }
    if e.r32(l + 0x17c) == e.r32(GB0(e) + 0xc) {
        heroDoTestForSlave(e, l)?;
    }
    Ok(0)
}

/// 0x0001d1a0 (1052 bytes) `void heroFUNC_CheckInputEvents(int param_1)`
pub fn heroFUNC_CheckInputEvents(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    // Shape groups: the switch-leader / tome parts are the same everywhere (release builds without the
    // prints); the tail differs: the "climb notification" branch (GB0+0x14 bit 0x1000 -> DM 0x1c0) is in
    // groups 0/1/4/6, the "crouch notification" branch (bits 0x4000|0x8000 -> DM 0x1c8) in 0/4/5.
    let gi = g(e, G_heroFUNC_CheckInputEvents);
    let has_climb = matches!(gi, 0 | 1 | 4 | 6);
    let has_crouch = matches!(gi, 0 | 4 | 5);
    let partner = e.r32(param_1 + 0x180);
    if e.r32(param_1 + 0x16c) & 0x1000_0000 != 0 && e.r32(param_1 + 0x170) & 1 == 0 && heroValidSwitchLeader(e, param_1)? != 0 {
        or_w(e, param_1 + 0x170, 1);
        APIgameobj::SendMessageTo(e, partner, NULL, 0x60, 1, NULL);
        return Ok(());
    }
    if e.r32(param_1 + 0x16c) & 8 != 0 && e.r32(param_1 + 0x170) & 4 == 0 {
        if e.r32(GB0(e) + 0x14) & 1 == 0 {
            if e.r32(param_1 + 0x170) & 0x4000 == 0 && heroValidOpenTome(e)? != 0 {
                and_w(e, param_1 + 0x16c, 0xffff_fff7);
                debug_time(e, "heroFUNC_CheckInputEvents event_ACTION_TOME", 0, 0)?;
                or_w(e, param_1 + 0x170, 0x4000);
                if IsInTeamMode(e)? == 0 {
                    return Ok(());
                }
                if APIgameobj::SendMessageTo(e, partner, NULL, 0x8d, 0, NULL) != 0 {
                    return Ok(());
                }
                APIgameobj::SendMessageTo(e, partner, NULL, 0x60, 0x4000, NULL);
                return Ok(());
            }
        } else if e.r32(param_1 + 0x170) & 0x2000 == 0 {
            debug_time(e, "heroFUNC_CheckInputEvents event_ACTION_TOME_OUT", 0, 0)?;
            or_w(e, param_1 + 0x170, 0x2000);
            if IsInTeamMode(e)? == 0 {
                return Ok(());
            }
            if APIgameobj::SendMessageTo(e, partner, NULL, 0x8d, 0, NULL) != 0 {
                return Ok(());
            }
            APIgameobj::SendMessageTo(e, partner, NULL, 0x60, 0x2000, NULL);
            return Ok(());
        }
    }
    let i16c = e.r32(param_1 + 0x16c);
    let f = e.r32(GB0(e) + 0x14);
    if has_climb && i16c & 0x10 != 0 && f & k(e, heroFUNC_CheckInputEvents_0_1) != 0 {
        debug_time(e, "--- FUNC_CheckInputEvents - OK", 0, 0)?;
        if e.r32(GB0(e) + 0x14) & 0x2000 != 0 {
            APIphysic::DMSetActive(e, e.r32(param_1 + 0x1c0), 0xffff_ffff);
        }
    } else if has_crouch && i16c & 0x10 != 0 && f & 0x4000 != 0 && f & 0x8000 != 0 {
        APIphysic::DMSetActive(e, e.r32(param_1 + 0x1c8), 0xffff_ffff);
    } else if i16c & 0x80 != 0 {
        or_w(e, param_1 + 0x170, 0x10000);
    }
    Ok(())
}

/// 0x0001d5c0 (828 bytes) `void heroFUNC_CheckControlEvents(int param_1)`
pub fn heroFUNC_CheckControlEvents(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    // Shaggy builds (groups 2/3) have no Sneak_In / Sneak_Out states to cancel
    let sneak_in_out = matches!(g(e, G_heroFUNC_CheckControlEvents), 0 | 1);
    let gb0 = GB0(e);
    if e.r32(param_1 + 0x174) & 8 != 0 && leader(e) {
        APIinput::InputConfigActivate(e, e.r32(param_1 + 0x188) as i32, 1);
        and_w(e, gb0 + 0x14, 0xfebf_ffff);
    }
    if e.r32(param_1 + 0x174) & 4 != 0 {
        if leader(e) {
            debug_time(e, "heroFUNC_CheckControlEvents -> event_CTRL_GOING_TO_CUT -> desable control/nophysicmask", 0, 0)?;
            or_w(e, gb0 + 0x14, 0x140_0000);
            APIinput::InputConfigActivate(e, e.r32(param_1 + 0x188) as i32, 0);
            APIgameobj::SetContextFlag(e, SELF, 0x50);
            e.w32(param_1 + 0x16c, 0);
            e.w32(param_1 + 0x160, addr::heroInit_Stand);
            let n = e.r32(param_1 + 0x154);
            let walking = n == addr::heroInit_Move
                || n == addr::heroInit_Sneak
                || n == addr::heroInit_Sneaking
                || (sneak_in_out && (n == addr::heroInit_Sneak_In || n == addr::heroInit_Sneak_Out));
            if walking {
                debug_info(e, "Force Stand");
                e.w32(param_1 + 0x154, 0);
            }
        } else {
            e.w32(param_1 + 0x16c, 0);
            e.w32(param_1 + 0x160, addr::heroInit_FollowStand);
        }
    }
    if e.r32(param_1 + 0x174) & 1 != 0 {
        heroSet_Leader(e, param_1)?;
        if e.r32(param_1 + 0x158) != addr::heroST_Switch_Loop {
            e.w32(param_1 + 0x154, addr::heroInit_Stand);
        }
    }
    if e.r32(param_1 + 0x174) & 2 != 0 {
        heroSet_Follower(e, param_1)?;
        if e.r32(param_1 + 0x158) != addr::heroST_Switch_Loop {
            e.w32(param_1 + 0x154, addr::heroInit_FollowStand);
        }
    }
    if e.r32(param_1 + 0x174) & 0x4000_0000 != 0 {
        or_w(e, param_1 + 0x164, 2);
    }
    Ok(())
}

/// `*(int *)(*pGlobalDataHero + 8) == *(int *)(pLocalDataHero + 0x17c)`: this hero is the leader.
fn leader(e: &Engine) -> bool {
    e.r32(GB0(e) + 8) == e.r32(L(e) + 0x17c)
}

pub(crate) fn or_w(e: &mut Engine, a: Ptr, v: u32) {
    let x = e.r32(a);
    e.w32(a, x | v);
}

pub(crate) fn and_w(e: &mut Engine, a: Ptr, v: u32) {
    let x = e.r32(a);
    e.w32(a, x & v);
}

/// 0x0001d900 (4088 bytes) `void heroFUNC_CheckActionEvents(int param_1)`
///
/// Turns the action events of local+0x170 into the next state (local+0x154). The original is one long
/// `if (!(ev & A)) { if (!(ev & B)) { ... } else B-handler } else A-handler` chain: the first event in
/// the chain that is set wins. Builds differ in which events their chain has (cut-scenes, edges,
/// crouch, ladder, power-up, use item) and in the objects of the door handler: both come from
/// `tables::CHECK_ACTION_EVENTS`; the handlers themselves are the same code in every build (apart from
/// the debug prints, which `debug_time`/`debug_info` compile out of release builds).
/// Book: event 0x4000 = open the book (leader: TomeIn, game flag 0x14 bit 0, 0xd1 to the book with the
/// prim's attribute 5; follower: TomeTrack), 0x2000 = close it (leader: TomeOut, flag off, 0xd0 to the
/// book; follower: prim attribute bit 4 set, TomeOut).
/// Note: the debug strings are the reference build's; a few builds word them differently (print only).
pub fn heroFUNC_CheckActionEvents(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    let t = &tables::CHECK_ACTION_EVENTS[e.rt.cur_build()];
    let ev = e.r32(param_1 + 0x170);
    if ev != 0 {
        let m = e.cstr("CheckActionEvents");
        heroFUNC_MultiBits(e, m, ev)?;
    }
    let mut hit = None;
    for &bit in t.chain {
        let set = e.r32(param_1 + 0x170) & bit != 0;
        let taken = match bit {
            // `((ev & 0x100000) == 0) || (GB0+0xc != local+0x17c)` -> else branch: set and I am the slave
            0x10_0000 => set && e.r32(GB0(e) + 0xc) == e.r32(param_1 + 0x17c),
            // `((ev & 0x10000) == 0) || (heroValidPowerUp() == 0)`
            0x1_0000 => set && heroValidPowerUp(e)? != 0,
            _ => set,
        };
        if taken {
            hit = Some(bit);
            break;
        }
    }
    let set_next = |e: &mut Engine, f: u32| e.w32(param_1 + 0x154, f);
    let Some(bit) = hit else {
        return heroFUNC_CheckActionEventsLocal(e);
    };
    let gb0 = GB0(e);
    match bit {
        0x400_0000 => set_next(e, addr::heroInit_CutScene),
        0x800_0000 => {
            and_w(e, gb0 + 0x14, 0xfff7_ffff);
            set_next(e, addr::heroInit_CutSceneOut);
        }
        1 => {
            and_w(e, gb0 + 0x14, 0xfffb_ffff);
            and_w(e, gb0 + 0x14, 0xffff_efff);
            if leader(e) {
                heroResetAction(e, param_1)?;
            }
            set_next(e, addr::heroInit_Switch);
        }
        2 => {
            if e.r32(GB0(e) + 0xc) == e.r32(param_1 + 0x17c) {
                set_next(e, addr::heroInit_Unhook);
            } else {
                debug_time(e, "heroFUNC_CheckActionEvents event_ACTION_HOOK_MODE to invalide hero", 0, 0)?;
            }
        }
        8 => {
            if leader(e) {
                if e.r32(gb0 + 0x34) & 0x10 != 0 {
                    and_w(e, gb0 + 0x34, 0xffff_fdff);
                }
                APIphysic::ResetFlags(e, SELF, 1);
                APIinput::InputConfigActivate(e, e.r32(param_1 + 0x188) as i32, 1);
                and_w(e, gb0 + 0x14, 0xffff_fffb);
                debug_time(e, "heroFUNC_CheckControlEvents -> event_ACTION_EXIT_DOOR heroactif", 0, 0)?;
                set_next(e, addr::heroInit_Stand);
            } else if e.r32(GB0(e) + 0xc) == e.r32(param_1 + 0x17c) {
                APIgameobj::LeaveParent(e, SELF, 0);
                APIphysic::ResetFlags(e, SELF, 1);
                set_next(e, addr::heroInit_FollowStand);
                debug_time(e, "heroFUNC_CheckControlEvents -> event_ACTION_EXIT_DOOR slave", 0, 0)?;
            }
        }
        4 => {
            if leader(e) {
                or_w(e, gb0 + 0x14, 4);
                and_w(e, gb0 + 0x14, 0xf9df_ffff);
                if APIgameobj::SendMessageTo(e, t.door_5d, NULL, 0x5d, 0, NULL) == 3 {
                    APIgameobj::SendMessageTo(e, t.door_5d, NULL, 0x5c, 1, NULL);
                }
                if !t.door_gate_34 || e.r32(GB0(e) + 0x34) & 1 == 0 {
                    let p = e.r32(param_1 + 0x180);
                    APIgameobj::SendMessageTo(e, p, NULL, 0x60, 4, NULL);
                }
                debug_time(e, "EventACTION event_ACTION_ENTER_DOOR", param_1 + 0x234, 1)?;
                let door = e.r32(param_1 + 0x234);
                APIgameobj::SendMessageTo(e, t.door_97, NULL, 0x97, door, NULL);
                APIinput::InputConfigActivate(e, e.r32(param_1 + 0x188) as i32, 0);
                APIphysic::SetVelocity(e, SELF, 0.0, 0.0, 0.0);
            } else {
                APIgameobj::LeaveParent(e, SELF, 0);
                APIphysic::ResetFlags(e, SELF, 1);
                APIphysic::PrimSetIntAttribBits(e, SELF, e.r32(param_1 + 0x18c), 2, 4);
            }
            if leader(e) && e.r32(GB0(e) + 0x14) & 0x200 != 0 {
                APIphysic::SetFlags(e, SELF, 1);
                debug_info(e, "do nothing");
            } else if e.r32(param_1 + 0x158) == addr::heroST_Panic_Loop {
                or_w(e, param_1 + 0x164, 0x80000);
                set_next(e, addr::heroInit_Busy);
            } else {
                or_w(e, param_1 + 0x164, 0x20000);
                set_next(e, addr::heroInit_Busy);
            }
            debug_time(e, "heroFUNC_CheckControlEvents -> event_ACTION_ENTER_DOOR", 0, 0)?;
        }
        0x10 => {
            debug_time(e, "heroFUNC_CheckActionEvents -> event_ACTION_DEAD", 0, 0)?;
            set_next(e, addr::heroInit_Dead);
        }
        0x20 => {
            debug_time(e, "heroFUNC_CheckActionEvents -> event_ACTION_HITTED", 0, 0)?;
            // both branches of the original's leader test store the same state
            set_next(e, addr::heroInit_Scare);
        }
        0x40 => set_next(e, addr::heroInit_PanicOut),
        0x10_0000 => {
            or_w(e, gb0 + 0x34, 0x100);
            and_w(e, gb0 + 0x34, 0xffff_fdff);
            set_next(e, addr::heroInit_Unhook);
        }
        0x200 => {
            if leader(e) {
                let p = e.r32(param_1 + 0x180);
                APIgameobj::SendMessageTo(e, p, NULL, 0x60, 0x200, NULL);
            }
            set_next(e, addr::heroInit_EdgeUp);
        }
        0x100 => {
            if leader(e) {
                and_w(e, gb0 + 0x14, 0xf9df_ffff);
                let f34 = e.r32(GB0(e) + 0x34);
                let solo = f34 & 0x10 == 0 || f34 & 0x100 != 0 || {
                    let partner = e.r32(L(e) + 0x180) as i32;
                    heroFUNC_IsAnythingBetweenUs(e, 0.15, SELF, partner, NULL, 0)? != 0
                };
                if solo {
                    set_next(e, addr::heroInit_EdgeSolo);
                } else {
                    let p = e.r32(param_1 + 0x180);
                    APIgameobj::SendMessageTo(e, p, NULL, 0x60, 0x100, NULL);
                    set_next(e, addr::heroInit_EdgeIn);
                }
            } else {
                set_next(e, addr::heroInit_EdgeJump);
            }
        }
        0x80 => {
            if leader(e) {
                set_next(e, addr::heroInit_EdgeDetect);
            }
        }
        0x400 => {
            debug_time(e, "heroFUNC_CheckActionEvents -> event_ACTION_DETECT_CROUCH", 0, 0)?;
            set_next(e, addr::heroInit_CrouchDetect);
        }
        0x800 => set_next(e, addr::heroInit_LadderIn),
        0x4000 => {
            debug_time(e, "heroFUNC_CheckActionEvents -> event_ACTION_TOME", 0, 0)?;
            if leader(e) {
                set_next(e, addr::heroInit_TomeIn);
                or_w(e, gb0 + 0x14, 1);
                let prim = e.r32(L(e) + 0x18c);
                let v = APIphysic::PrimGetIntAttrib(e, SELF, prim, 5);
                APIgameobj::SendMessageTo(e, t.tome, NULL, 0xd1, v, NULL);
            } else {
                set_next(e, addr::heroInit_TomeTrack);
            }
        }
        0x2000 => {
            debug_time(e, "heroFUNC_CheckActionEvents -> event_ACTION_TOME_OUT", 0, 0)?;
            if leader(e) {
                set_next(e, addr::heroInit_TomeOut);
                and_w(e, gb0 + 0x14, 0xffff_fffe);
                APIgameobj::SendMessageTo(e, t.tome, NULL, 0xd0, 0, NULL);
            } else {
                APIphysic::PrimSetIntAttribBits(e, SELF, e.r32(param_1 + 0x18c), 2, 4);
                set_next(e, addr::heroInit_TomeOut);
            }
        }
        0x8000 => {
            debug_time(e, "heroFUNC_CheckActionEvents -> event_ACTION_POWER_UP_PUT_AWAY", 0, 0)?;
            set_next(e, addr_pu::heroInit_PowerUpOut);
            and_w(e, gb0 + 0x14, 0xffff_fffd);
        }
        0x1_0000 => {
            debug_time(e, "heroFUNC_CheckActionEvents -> event_ACTION_POWER_UP_TAKE_OUT", 0, 0)?;
            set_next(e, addr_pu::heroInit_PowerUpIn);
            or_w(e, gb0 + 0x14, 2);
        }
        0x1000 => {
            debug_time(e, "heroFUNC_CheckActionEvents -> event_ACTION_USE_ITEM", 0, 0)?;
            set_next(e, addr::heroInit_Action);
        }
        0x4_0000 => set_next(e, addr::heroInit_Break),
        0x8_0000 => set_next(e, addr::heroInit_BumpByHero),
        0x100_0000 => {
            if leader(e) {
                APIinput::InputConfigActivate(e, e.r32(param_1 + 0x188) as i32, 0);
                if IsInTeamMode(e)? != 0 {
                    let p = e.r32(param_1 + 0x180);
                    APIgameobj::SendMessageTo(e, p, NULL, 0x60, 0x100_0000, NULL);
                }
            }
            set_next(e, addr::heroInit_Busy);
        }
        0x200_0000 => {
            if leader(e) {
                APIinput::InputConfigActivate(e, e.r32(param_1 + 0x188) as i32, 1);
                if IsInTeamMode(e)? != 0 {
                    let p = e.r32(param_1 + 0x180);
                    APIgameobj::SendMessageTo(e, p, NULL, 0x60, 0x200_0000, NULL);
                }
                set_next(e, addr::heroInit_Stand);
            } else {
                set_next(e, addr::heroInit_FollowStand);
            }
        }
        0x2_0000 => {
            if !leader(e) {
                set_next(e, addr::heroInit_FollowIdle);
            }
        }
        b => unreachable!("CHECK_ACTION_EVENTS: unknown event bit {b:#x}"),
    }
    Ok(())
}

/// 0x0001e978 (80 bytes) `void heroFUNC_GetVelocity(int param_1)`
pub fn heroFUNC_GetVelocity(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    let v = LIBApiCFunctions::GetVelocity(e, SELF as i32, 7);
    e.wf(param_1 + 0x2e0, v);
    Ok(())
}

/// 0x0001e9c8 (96 bytes) `undefined8 heroCB_PRELOGIC(void)`
pub fn heroCB_PRELOGIC(e: &mut Engine, _: u32) -> LResult<u64> {
    let l = L(e);
    heroFUNC_CheckInputEvents(e, l)?;
    Ok(0)
}

/// 0x0001ea28 (636 bytes) `undefined8 heroCB_POSTLOGIC(void)`
pub fn heroCB_POSTLOGIC(e: &mut Engine, _: u32) -> LResult<u64> {
    let l = L(e);
    if e.r32(l + 700) != 0 {
        return Ok(0);
    }
    if e.r32(l + 0x168) & 0x1000_0000 != 0 {
        debug_time(e, "POSTLOGIC event_CALLBACK_CHANGE_LEADER", 0, 0)?;
        let b = GB0(e);
        // b+0 / +4: current character (1 / 2), +8 leader id, +0xc follower id, +0x10 the other character
        let (cur, ids, other) = if e.r32(b + 8) == k(e, heroCB_POSTLOGIC_0_1) {
            (2, [k(e, heroCB_POSTLOGIC_0_2), k(e, heroCB_POSTLOGIC_0_3)], 1)
        } else {
            (1, [k(e, heroCB_POSTLOGIC_0_4), k(e, heroCB_POSTLOGIC_0_5)], 2)
        };
        e.w32(b + 4, cur);
        e.w32(b, cur);
        e.w32(b + 8, ids[0]);
        e.w32(b + 0xc, ids[1]);
        e.w32(b + 0x10, other);
        debug_info(e, "heroCB_POSTLOGIC -> change leader/follower");
        let b = GB0(e);
        let (ld, fl) = (e.r32(b + 8), e.r32(b + 0xc));
        APIgameobj::SendMessageTo(e, ld, NULL, 0x7c, 0, NULL);
        APIgameobj::SendMessageTo(e, fl, NULL, 0x7d, 0, NULL);
    }
    e.w32(l + 0x2cc, 0);
    e.w32(l + 0x2c8, 0);
    if leader(e) {
        let b = GB0(e);
        and_w(e, b + 0x14, 0x9fff_ffff);
    }
    e.w32(l + 0x168, 0);
    e.w32(l + 0x16c, 0);
    Ok(0)
}

/// 0x0001eca8 (1112 bytes) `undefined4 heroValidSwitchLeader(int param_1)`
pub fn heroValidSwitchLeader(e: &mut Engine, param_1: Ptr) -> LResult<u32> {
    // per group: the two "solo switch" voice sounds and the clearance radius (0.3 / 0.35)
    let (snd_a, snd_b, radius): (i32, i32, f32) = match g(e, G_heroValidSwitchLeader) {
        0 => (-0x7fff_ffcc, -0x7fff_ffca, 0.3),
        1 => (-0x7fff_ffed, -0x7fff_ffeb, 0.3),
        2 => (-0x7fff_ffed, -0x7fff_ffef, 0.3),
        3 => (-0x7fff_ffe2, -0x7fff_ffe0, 0.35),
        4 => (-0x7fff_ffee, -0x7fff_ffec, 0.35),
        _ => (-0x7fff_ffed, -0x7fff_ffef, 0.35),
    };
    debug_time(e, "heroValidSwitchLeader", 0, 0)?;
    let f34 = e.r32(GB0(e) + 0x34);
    let refused = f34 & 0x10 == 0
        || f34 & 0x100 != 0
        || {
            let p = e.r32(param_1 + 0x180);
            APIgameobj::SendMessageTo(e, p, NULL, 0x8d, 0, NULL) != 0
        }
        || e.r32(GB0(e) + 0x14) & 0xe20_090c != 0
        || APIgameobj::SendMessageTo(e, k(e, heroValidSwitchLeader_0_1), NULL, 0xc6, 0, NULL) != 0;
    if refused {
        if e.r32(GB0(e) + 0x34) & 0x101 != 0 {
            let me = e.r32(param_1 + 0x17c);
            if me == k(e, heroValidSwitchLeader_0_2) {
                debug_info(e, "PlaySound VO_SHAGGY_SOLOSWITCH");
                LIBSndMngr::PlaySound(e, snd_a);
            } else if me == k(e, heroValidSwitchLeader_0_3) {
                debug_info(e, "PlaySound VO_SCOOBY_SOLOSWITCH");
                LIBSndMngr::PlaySound(e, snd_b);
            }
        }
        return Ok(0);
    }
    let a80 = e.alloca(0x20);
    let a60 = e.alloca(0x20);
    let ld = e.r32(GB0(e) + 8);
    APIphysic::GetFloorAttribs(e, ld, a80);
    let partner = e.r32(L(e) + 0x180);
    APIphysic::GetFloorAttribs(e, partner, a60);
    let ld = e.r32(GB0(e) + 8);
    let s40 = APIphysic::GetPhysicState(e, ld);
    let partner = e.r32(L(e) + 0x180);
    let s3c = APIphysic::GetPhysicState(e, partner);
    let (f78, f58) = (e.r32(a80 + 8), e.r32(a60 + 8));
    if f78 & 1 == 0 && f58 & 1 == 0 && f78 & 0x40 == 0 && f58 & 0x40 == 0 && s40 & 1 != 0 && s3c & 1 != 0 {
        debug_info(e, "is not using cheat");
        let partner = e.r32(L(e) + 0x180) as i32;
        if heroFUNC_IsAnythingBetweenUs(e, radius, SELF, partner, NULL, 0)? == 0 {
            let b5 = e.r32(GD(e) + 0x14);
            e.w32(b5, 0);
            debug_info(e, "path clear");
            Ok(1)
        } else {
            debug_info(e, "path blocked");
            Ok(0)
        }
    } else {
        debug_info(e, "in stairs");
        Ok(0)
    }
}

/// 0x0001f110 (436 bytes) `undefined8 heroDoTestForBump(int param_1)`
pub fn heroDoTestForBump(e: &mut Engine, param_1: Ptr) -> LResult<u64> {
    // group 1 (release) folds the waypoint test into the condition: same behaviour without the print
    if e.r32(GB0(e) + 0x14) & 0x62c_0020 == 0
        && IsInTeamMode(e)? != 0
        && e.rf(param_1 + 0x2e0) < 2.0
        && 0.01 < e.rf(param_1 + 0x2c8) + e.rf(param_1 + 0x2cc)
        && IsSlaveInTheWay(e, param_1)? != 0
    {
        let fl = e.r32(GB0(e) + 0xc);
        if IsWaypointClear(e, fl)? == 0 {
            debug_info(e, "BLOCKED!!!! - DAMNIT!");
        } else {
            let p = e.r32(param_1 + 0x180);
            APIgameobj::SendMessageTo(e, p, NULL, 0x60, 0x80000, NULL);
            APIgameobj::SendMessageTo(e, p, NULL, 0x72, 0, param_1 + 0x2d0);
        }
    }
    Ok(0)
}

/// 0x0001f2c8 (1212 bytes) `undefined8 heroDoTestForActionClimb(int param_1)`
pub fn heroDoTestForActionClimb(e: &mut Engine, param_1: Ptr) -> LResult<u64> {
    // group 1 (20 builds): empty
    if g(e, G_heroDoTestForActionClimb) == 1 {
        return Ok(0);
    }
    notification_zone(e, param_1, 0x1000, 0x1, 0x2000, 3, None)
}

/// 0x0001f788 (1644 bytes) `undefined8 heroDoTestForActionCrouch(int param_1)`
pub fn heroDoTestForActionCrouch(e: &mut Engine, param_1: Ptr) -> LResult<u64> {
    // group 1 (44 builds): empty
    if g(e, G_heroDoTestForActionCrouch) == 1 {
        return Ok(0);
    }
    notification_zone(e, param_1, 0x4000, 0x201, 0x8000, 0x13, Some(1.22))
}

/// 0x0001fdf8 (48 bytes) `undefined8 heroDoTestForActionDiveIn(void)`
pub fn heroDoTestForActionDiveIn(e: &mut Engine) -> LResult<u64> {
    let param_1 = L(e);
    // only groups 1/2 (5 Ep4 builds with water) have a body: the "dive" panel notification 0x11 when
    // the leader stands on environment type 4 (water) within 0.75 m below
    if g(e, G_heroDoTestForActionDiveIn) == 0 {
        return Ok(0);
    }
    if !(leader(e) && e.r32(GB0(e) + 0x14) & 0x620_0000 == 0 && e.r32(GB2(e)) & 0x400 != 0) {
        return Ok(0);
    }
    let ld = e.r32(GB0(e) + 8);
    let on = IsCharacterOnThisFloorType(e, -0.75, 0.2, ld, 0, 4)?;
    let shown = |e: &Engine| e.r32(GB2(e)) & 0x2_0000 != 0;
    if on == 0 {
        if shown(e) && e.r32(param_1 + 0x214) == 0x11 {
            notification_out(e, param_1);
        }
    } else if e.r32(GB0(e) + 0x14) & 0x6e20_090d == 0 || !shown(e) || e.r32(param_1 + 0x214) != 0x11 {
        if !shown(e) && e.r32(GB0(e) + 0x14) & 0x6e20_090d == 0 {
            e.w32(param_1 + 0x214, 0x11);
            APIgameobj::SendMessageTo(e, 0x2a10_0006, NULL, 0x31, 0x11, NULL);
        }
    } else {
        notification_out(e, param_1);
    }
    Ok(0)
}

/// 0x0001fe28 (48 bytes) `undefined8 heroDoTestForActionDiveOut(void)`
pub fn heroDoTestForActionDiveOut(e: &mut Engine) -> LResult<u64> {
    // empty in every build
    Ok(0)
}

/// 0x0001fe58 (416 bytes) `undefined8 heroDoTestForSlave(int param_1)`
pub fn heroDoTestForSlave(e: &mut Engine, param_1: Ptr) -> LResult<u64> {
    let b = GB0(e);
    if e.r32(b + 0x34) & 0x10 != 0 && e.r32(b + 0x34) & 0x100 == 0 && e.r32(b + 0x14) & 0x620_010c == 0 {
        let d40 = e.alloca(0x10);
        let d30 = e.alloca(0x10);
        APIgameobj::GetPosition(e, SELF, ALL, d40);
        let ld = e.r32(GB0(e) + 8);
        APIgameobj::GetPosition(e, ld, ALL, d30);
        let c = e.r32(GB0(e) + 4);
        // the 2.1 is a double literal in the original
        if (((e.rf(d40 + 4) as f64) + 2.1) < e.rf(d30 + 4) as f64) && (c == 2 || c == 1) {
            debug_info(e, "heroDoTestForSlave -> deconnecting");
            or_w(e, param_1 + 0x170, 0x10_0000);
        }
    }
    Ok(0)
}

/// 0x0001fff8 (984 bytes) `undefined4 heroDoTestForOnFloor(int param_1)`
pub fn heroDoTestForOnFloor(e: &mut Engine, param_1: Ptr) -> LResult<u32> {
    // group 1 (df8bf53b): the partner is not tested while it is disconnected (GB0+0x34 bit 0x100)
    let partner_gate = g(e, G_heroDoTestForOnFloor) == 1;
    let t = APIextra::TimerGetTime(e, e.r32(param_1 + 0x210));
    if !(0.33 < t) {
        return Ok(0);
    }
    APIextra::TimerReset(e, e.r32(param_1 + 0x210));
    if !(e.r32(GB0(e) + 0x14) & 0x6e20_090c == 0 || e.r32(param_1 + 0x158) == addr::heroST_Switch_Loop) {
        return Ok(0);
    }
    let mut falling = false;
    let d90 = e.alloca(0x10);
    let d70 = e.alloca(0x10);
    APIphysic::GetVelocity(e, SELF, d90);
    let prim = e.r32(L(e) + 0x18c);
    let types = APIphysic::PrimGetIntAttrib(e, SELF, prim, 5) | 3;
    if e.rf(d90 + 4) < -25.0 {
        let ld = e.r32(GB0(e) + 8);
        if IsCharacterOnThisFloorType(e, -7.0, 1.0, ld, types as i32, 0)? == 0 {
            falling = true;
        }
    }
    if e.r32(GB0(e) + 0x34) & 0x10 != 0 {
        if !falling && (!partner_gate || e.r32(GB0(e) + 0x34) & 0x100 == 0) {
            let p = e.r32(param_1 + 0x180);
            APIphysic::GetVelocity(e, p, d70);
            let types = APIgameobj::SendMessageTo(e, p, NULL, 0x8b, 0, NULL) | 3;
            if e.rf(d70 + 4) < -25.0 && IsCharacterOnThisFloorType(e, -7.0, 1.0, p, types as i32, 0)? == 0 {
                falling = true;
            }
        }
        if e.r32(GB0(e) + 0x14) & 0x40 == 0 && falling {
            let p = e.r32(param_1 + 0x180);
            if e.r32(GB0(e) + 0x34) & 0x100 != 0 {
                let b = GB0(e);
                and_w(e, b + 0x34, 0xffff_feff);
                APIgameobj::SendMessageTo(e, p, NULL, 0x8c, 0, NULL);
            }
            APIgameobj::SendMessageTo(e, p, NULL, 0x60, 0x10, NULL);
        }
    }
    if falling {
        if std::env::var_os("GAME_DEADTRACE").is_some() { let p = APIgameobj::GetID(e, SELF, 0); eprintln!("FALLTRACE obj {p:#x} vy {} gb14 {:#x}", e.rf(d90 + 4), e.r32(GB0(e) + 0x14)); }
        if e.r32(GB0(e) + 0x14) & 0x40 == 0 {
            or_w(e, param_1 + 0x170, 0x10);
        } else {
            let orp = e.alloca(8);
            e.w32(orp, 1);
            e.w32(orp + 4, 0x25);
            APIgameobj::SendMessageTo(e, 0xff00_0008, orp, 0x1e8, 0, NULL);
        }
        return Ok(1);
    }
    Ok(0)
}

/// 0x000203d0 (216 bytes) `void heroResetAction(int param_1)`
pub fn heroResetAction(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    if e.r32(GB2(e)) & 0x2_0000 == 0 || e.r32(param_1 + 0x214) == 0 {
        e.w32(param_1 + 0x214, 0);
    } else {
        APIgameobj::SendMessageTo(e, 0x2a10_0006, NULL, 0x32, e.r32(param_1 + 0x214), NULL);
        e.w32(param_1 + 0x214, 0);
        debug_info(e, "-- MSG_PANEL_NOTIFICATION_OUT");
    }
    Ok(())
}

fn notification_zone(e: &mut Engine, p: Ptr, enable: u32, block: u32, flag: u32, id: u32, cone: Option<f32>) -> LResult<u64> {
    let f = e.r32(GB0(e) + 0x14);
    let shown = |e: &Engine| e.r32(GB2(e)) & 0x2_0000 != 0;
    if f & 0x620_0000 == 0 && f & enable != 0 && f & block == 0 {
        let d = e.alloca(0x10);
        APIgameobj::GetPosition(e, SELF, ALL, d);
        let (x, z) = (e.rf(d), e.rf(d + 8));
        let r = |o: u32| e.rf(p + o);
        let mut inside = if e.r32(p + 0x368) == 0 {
            r(0x350) * x + r(0x358) <= z
                && z <= r(0x350) * x + r(0x360)
                && r(0x354) * x + r(0x35c) <= z
                && z <= r(0x354) * x + r(0x364)
        } else {
            r(0x358) <= x && x <= r(0x360) && r(0x35c) <= z && z <= r(0x364)
        };
        if let Some(cone) = cone {
            if inside {
                let la = e.alloca(0x10);
                APIgameobj::GetLookAt(e, SELF, ALL, la);
                let mut a = APImath::atan2f(e, e.rf(la), e.rf(la + 8));
                let mut want = e.rf(p + 0x344);
                if (a < want && 3.1415927 < want - a) || (want < a && 3.1415927 < a - want) {
                    want += 3.1415927;
                    a += 3.1415927;
                    if 3.1415927 < want {
                        want = 6.2831855 - want;
                    }
                    if 3.1415927 < a {
                        a = 6.2831855 - a;
                    }
                }
                // 1.22 is a double literal: the sums are done in double precision
                let (w, a2) = (want as f64, a as f64);
                if w + 1.22 <= a2 || a2 <= w - 1.22 {
                    inside = false;
                }
                let _ = cone;
            }
        }
        // the climb zone also requires GB0+0x14 bit 0x800 clear to show the notification
        let extra_ok = cone.is_some() || e.r32(GB0(e) + 0x14) & 0x800 == 0;
        if inside && !shown(e) && extra_ok {
            debug_time(e, "NOTIFICATION_IN", 0, 0)?;
            or_w(e, GB0(e) + 0x14, flag);
            e.w32(p + 0x214, id);
            APIgameobj::SendMessageTo(e, 0x2a10_0006, NULL, 0x31, id, NULL);
            debug_info(e, "-- MSG_PANEL_NOTIFICATION_IN");
        } else if shown(e) && e.r32(p + 0x214) == id && !inside {
            and_w(e, GB0(e) + 0x14, !flag);
            notification_out(e, p);
        }
    } else if shown(e) && e.r32(p + 0x214) == id {
        and_w(e, GB0(e) + 0x14, !flag);
        notification_out(e, p);
    }
    Ok(0)
}

fn notification_out(e: &mut Engine, p: Ptr) {
    APIgameobj::SendMessageTo(e, 0x2a10_0006, NULL, 0x32, e.r32(p + 0x214), NULL);
    e.w32(p + 0x214, 0);
    debug_info(e, "-- MSG_PANEL_NOTIFICATION_OUT");
}

//! hero_core.c (17 functions). Addresses are module-relative (.text at 0x10000) in scooby_ghost__08a011d0.
use super::builds::*;
use super::*;
use crate::engine::*;
use crate::logic::LResult;

/// 0x00017e68 (164 bytes) `undefined8 heroST_FIRSTMOVE(void)`
pub fn heroST_FIRSTMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    let l = L(e);
    e.w32(l + 0x154, addr::heroInit_Stand);
    e.w32(l + 0x158, addr::heroST_Stand_Loop);
    e.w32(l + 0x15c, 0);
    heroST_ENTRY_POINT(e, 0)?;
    APIgameobj::SetNextState(e, SELF, 1);
    Ok(0)
}

/// 0x00017f10 (672 bytes) `undefined8 heroST_ENTRY_POINT(void)`
pub fn heroST_ENTRY_POINT(e: &mut Engine, _: u32) -> LResult<u64> {
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
        heroDoTestForActionCrouch(e)?;
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

/// 0x000181b0 (248 bytes) `void heroFUNC_CheckInputEvents(int param_1)`
pub fn heroFUNC_CheckInputEvents(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    if e.r32(param_1 + 0x16c) & 0x1000_0000 == 0 || e.r32(param_1 + 0x170) & 1 != 0 || heroValidSwitchLeader(e, param_1)? == 0 {
        if e.r32(param_1 + 0x16c) & 0x80 != 0 {
            or_w(e, param_1 + 0x170, 0x10000);
        }
    } else {
        or_w(e, param_1 + 0x170, 1);
        let partner = e.r32(param_1 + 0x180);
        APIgameobj::SendMessageTo(e, partner, NULL, 0x60, 1, NULL);
    }
    Ok(())
}

/// 0x000182a8 (780 bytes) `void heroFUNC_CheckControlEvents(int param_1)`
pub fn heroFUNC_CheckControlEvents(e: &mut Engine, param_1: Ptr) -> LResult<()> {
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
                || n == addr::heroInit_Sneaking;
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

/// 0x000185b8 (2760 bytes) `void heroFUNC_CheckActionEvents(int param_1)`
pub fn heroFUNC_CheckActionEvents(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    // group 1: no event_ACTION_USE_ITEM (0x1000) branch; the door / object ids are build literals
    let use_item = g(e, G_heroFUNC_CheckActionEvents) == 0;
    let ev0 = e.r32(param_1 + 0x170);
    if ev0 != 0 {
        let m = e.cstr("CheckActionEvents");
        heroFUNC_MultiBits(e, m, ev0)?;
    }
    let ev = |e: &Engine| e.r32(param_1 + 0x170);
    let set_next = |e: &mut Engine, f: u32| e.w32(param_1 + 0x154, f);
    if ev(e) & 1 != 0 {
        let b = GB0(e);
        and_w(e, b + 0x14, 0xfffb_ffff);
        and_w(e, b + 0x14, 0xffff_efff);
        if leader(e) {
            heroResetAction(e, param_1)?;
        }
        set_next(e, addr::heroInit_Switch);
    } else if ev(e) & 2 != 0 {
        if e.r32(GB0(e) + 0xc) == e.r32(param_1 + 0x17c) {
            set_next(e, addr::heroInit_Unhook);
        } else {
            debug_time(e, "heroFUNC_CheckActionEvents event_ACTION_HOOK_MODE to invalide hero", 0, 0)?;
        }
    } else if ev(e) & 8 != 0 {
        let gb0 = GB0(e);
        if leader(e) {
            if e.r32(gb0 + 0x34) & 0x10 != 0 {
                and_w(e, gb0 + 0x34, 0xffff_fdff);
            }
            APIphysic::ResetFlags(e, SELF, 1);
            APIinput::InputConfigActivate(e, e.r32(param_1 + 0x188) as i32, 1);
            and_w(e, gb0 + 0x14, 0xffff_fffb);
            debug_time(e, "heroFUNC_CheckControlEvents -> event_ACTION_EXIT_DOOR heroactif", 0, 0)?;
            set_next(e, addr::heroInit_Stand);
        } else if e.r32(gb0 + 0xc) == e.r32(param_1 + 0x17c) {
            APIgameobj::LeaveParent(e, SELF, 0);
            APIphysic::ResetFlags(e, SELF, 1);
            set_next(e, addr::heroInit_FollowStand);
            debug_time(e, "heroFUNC_CheckControlEvents -> event_ACTION_EXIT_DOOR slave", 0, 0)?;
        }
    } else if ev(e) & 4 != 0 {
        let gb0 = GB0(e);
        if leader(e) {
            or_w(e, gb0 + 0x14, 4);
            and_w(e, gb0 + 0x14, 0xf9df_ffff);
            if APIgameobj::SendMessageTo(e, k(e, ID_2a000096), NULL, 0x5d, 0, NULL) == 3 {
                APIgameobj::SendMessageTo(e, k(e, ID_2a000096), NULL, 0x5c, 1, NULL);
            }
            let p = e.r32(param_1 + 0x180);
            APIgameobj::SendMessageTo(e, p, NULL, 0x60, 4, NULL);
            debug_time(e, "EventACTION event_ACTION_ENTER_DOOR", param_1 + 0x234, 1)?;
            let door = e.r32(param_1 + 0x234);
            APIgameobj::SendMessageTo(e, k(e, ID_2a00008f), NULL, 0x97, door, NULL);
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
    } else if ev(e) & 0x10 != 0 {
        debug_time(e, "heroFUNC_CheckActionEvents -> event_ACTION_DEAD", 0, 0)?;
        set_next(e, addr::heroInit_Dead);
    } else if ev(e) & 0x20 != 0 {
        debug_time(e, "heroFUNC_CheckActionEvents -> event_ACTION_HITTED", 0, 0)?;
        // both branches of the original's leader test store the same state
        set_next(e, addr::heroInit_Scare);
    } else if ev(e) & 0x40 != 0 {
        set_next(e, addr::heroInit_PanicOut);
    } else if ev(e) & 0x10_0000 != 0 && e.r32(GB0(e) + 0xc) == e.r32(param_1 + 0x17c) {
        let gb0 = GB0(e);
        or_w(e, gb0 + 0x34, 0x100);
        and_w(e, gb0 + 0x34, 0xffff_fdff);
        set_next(e, addr::heroInit_Unhook);
    } else if use_item && ev(e) & 0x1000 != 0 {
        debug_time(e, "heroFUNC_CheckActionEvents -> event_ACTION_USE_ITEM", 0, 0)?;
        set_next(e, addr::heroInit_Action);
    } else if ev(e) & 0x4_0000 != 0 {
        set_next(e, addr::heroInit_Break);
    } else if ev(e) & 0x8_0000 != 0 {
        set_next(e, addr::heroInit_BumpByHero);
    } else if ev(e) & 0x100_0000 != 0 {
        if leader(e) {
            APIinput::InputConfigActivate(e, e.r32(param_1 + 0x188) as i32, 0);
            if IsInTeamMode(e)? != 0 {
                let p = e.r32(param_1 + 0x180);
                APIgameobj::SendMessageTo(e, p, NULL, 0x60, 0x100_0000, NULL);
            }
        }
        set_next(e, addr::heroInit_Busy);
    } else if ev(e) & 0x200_0000 != 0 {
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
    } else if ev(e) & 0x2_0000 != 0 {
        if !leader(e) {
            set_next(e, addr::heroInit_FollowIdle);
        }
    } else {
        heroFUNC_CheckActionEventsLocal(e)?;
    }
    Ok(())
}

/// 0x000190c8 (80 bytes) `void heroFUNC_GetVelocity(int param_1)`
pub fn heroFUNC_GetVelocity(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    let v = LIBApiCFunctions::GetVelocity(e, SELF as i32, 7);
    e.wf(param_1 + 0x2e0, v);
    Ok(())
}

/// 0x00019118 (96 bytes) `undefined8 heroCB_PRELOGIC(void)`
pub fn heroCB_PRELOGIC(e: &mut Engine, _: u32) -> LResult<u64> {
    let l = L(e);
    heroFUNC_CheckInputEvents(e, l)?;
    Ok(0)
}

/// 0x00019178 (636 bytes) `undefined8 heroCB_POSTLOGIC(void)`
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

/// 0x000193f8 (1112 bytes) `undefined4 heroValidSwitchLeader(int param_1)`
pub fn heroValidSwitchLeader(e: &mut Engine, param_1: Ptr) -> LResult<u32> {
    // groups differ only in the two debug strings (swapped)
    let swap = g(e, G_heroValidSwitchLeader) == 1;
    debug_time(e, "heroValidSwitchLeader", 0, 0)?;
    let f34 = e.r32(GB0(e) + 0x34);
    let refused = f34 & 0x10 == 0
        || f34 & 0x100 != 0
        || {
            let p = e.r32(param_1 + 0x180);
            APIgameobj::SendMessageTo(e, p, NULL, 0x8d, 0, NULL) != 0
        }
        || e.r32(GB0(e) + 0x14) & 0xe20_090c != 0
        || APIgameobj::SendMessageTo(e, k(e, ID_2a00008f), NULL, 0xc6, 0, NULL) != 0;
    if refused {
        if e.r32(GB0(e) + 0x34) & 0x101 != 0 {
            let me = e.r32(param_1 + 0x17c);
            let (sa, sb) = if swap {
                ("PlaySound VO_SCOOBY_SOLOSWITCH", "PlaySound VO_SHAGGY_SOLOSWITCH")
            } else {
                ("PlaySound VO_SHAGGY_SOLOSWITCH", "PlaySound VO_SCOOBY_SOLOSWITCH")
            };
            if me == k(e, heroValidSwitchLeader_0_1) {
                debug_info(e, sa);
                LIBSndMngr::PlaySound(e, k(e, ID_7fffffcc).wrapping_neg() as i32);
            } else if me == k(e, heroValidSwitchLeader_0_2) {
                debug_info(e, sb);
                LIBSndMngr::PlaySound(e, k(e, ID_7fffffca).wrapping_neg() as i32);
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
        if heroFUNC_IsAnythingBetweenUs(e, 0.35, SELF, partner, NULL, 0)? == 0 {
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

/// 0x00019860 (436 bytes) `undefined8 heroDoTestForBump(int param_1)`
pub fn heroDoTestForBump(e: &mut Engine, param_1: Ptr) -> LResult<u64> {
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

/// 0x00019a18 (1212 bytes) `undefined8 heroDoTestForActionClimb(int param_1)`
pub fn heroDoTestForActionClimb(e: &mut Engine, param_1: Ptr) -> LResult<u64> {
    let p = param_1;
    let f = e.r32(GB0(e) + 0x14);
    let shown = |e: &Engine| e.r32(GB2(e)) & 0x2_0000 != 0;
    if f & 0x620_0000 == 0 && f & 0x1000 != 0 && f & 1 == 0 {
        let d = e.alloca(0x10);
        APIgameobj::GetPosition(e, SELF, ALL, d);
        let (x, z) = (e.rf(d), e.rf(d + 8));
        let r = |o: u32| e.rf(p + o);
        let inside = if e.r32(p + 0x368) == 0 {
            r(0x350) * x + r(0x358) <= z
                && z <= r(0x350) * x + r(0x360)
                && r(0x354) * x + r(0x35c) <= z
                && z <= r(0x354) * x + r(0x364)
        } else {
            r(0x358) <= x && x <= r(0x360) && r(0x35c) <= z && z <= r(0x364)
        };
        if inside && !shown(e) && e.r32(GB0(e) + 0x14) & 0x800 == 0 {
            debug_time(e, "NOTIFICATION_IN", 0, 0)?;
            or_w(e, GB0(e) + 0x14, 0x2000);
            e.w32(p + 0x214, 3);
            APIgameobj::SendMessageTo(e, k(e, ID_2a100006), NULL, 0x31, 3, NULL);
            debug_info(e, "-- MSG_PANEL_NOTIFICATION_IN");
        } else if shown(e) && e.r32(p + 0x214) == 3 && !inside {
            and_w(e, GB0(e) + 0x14, 0xffff_dfff);
            notification_out(e, p);
        }
    } else if shown(e) && e.r32(p + 0x214) == 3 {
        and_w(e, GB0(e) + 0x14, 0xffff_dfff);
        notification_out(e, p);
    }
    Ok(0)
}

/// 0x00019ed8 (48 bytes) `undefined8 heroDoTestForActionCrouch(void)`
pub fn heroDoTestForActionCrouch(e: &mut Engine) -> LResult<u64> {
    // empty in every build
    Ok(0)
}

/// 0x00019f08 (48 bytes) `undefined8 heroDoTestForActionDiveIn(void)`
pub fn heroDoTestForActionDiveIn(e: &mut Engine) -> LResult<u64> {
    // empty in every build
    Ok(0)
}

/// 0x00019f38 (48 bytes) `undefined8 heroDoTestForActionDiveOut(void)`
pub fn heroDoTestForActionDiveOut(e: &mut Engine) -> LResult<u64> {
    // empty in every build
    Ok(0)
}

/// 0x00019f68 (416 bytes) `undefined8 heroDoTestForSlave(int param_1)`
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

/// 0x0001a108 (984 bytes) `undefined4 heroDoTestForOnFloor(int param_1)`
pub fn heroDoTestForOnFloor(e: &mut Engine, param_1: Ptr) -> LResult<u32> {
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
        if !falling {
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

/// 0x0001a4e0 (216 bytes) `void heroResetAction(int param_1)`
pub fn heroResetAction(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    if e.r32(GB2(e)) & 0x2_0000 == 0 || e.r32(param_1 + 0x214) == 0 {
        e.w32(param_1 + 0x214, 0);
    } else {
        APIgameobj::SendMessageTo(e, k(e, ID_2a100006), NULL, 0x32, e.r32(param_1 + 0x214), NULL);
        e.w32(param_1 + 0x214, 0);
        debug_info(e, "-- MSG_PANEL_NOTIFICATION_OUT");
    }
    Ok(())
}


// ---- helpers (from the hero port) ----

/// `*(int *)(*pGlobalDataHero + 8) == *(int *)(pLocalDataHero + 0x17c)`: this hero is the leader.
pub(crate) fn leader(e: &Engine) -> bool {
    e.r32(GB0(e) + 8) == e.r32(L(e) + 0x17c)
}

pub(crate) fn notification_out(e: &mut Engine, p: Ptr) {
    let id = e.r32(p + 0x214);
    APIgameobj::SendMessageTo(e, k(e, ID_2a100006), NULL, 0x32, id, NULL);
    e.w32(p + 0x214, 0);
    debug_info(e, "-- MSG_PANEL_NOTIFICATION_OUT");
}

//! hero_race.c plus the level part of scooby.c (c33ca2c4, EP3 L01) / shaggy.c (0117f115): the food race.
//! The heroes run along random waypoint paths (level local+0x388 path, +0x390 nb of waypoints, +0x394 current
//! waypoint pos, +0x3a0 index); +0x3a8 is the "fullness" factor (1 = full speed), +0x3a4 the resulting speed,
//! +0x3ac..0x3bc the food blend params (+0x3b4 the blend factor). Eating slows them down and makes them burp.
//!
//! Per-build differences (c33ca2c4 / 0117f115): wait anim 0x15/0xf, burp anim 0x40/0x37, blend anim
//! 0x50/0x40, UpdateBlend_Scooby/_Others; scooby only has heroInit_OpenDoor (message 300) and the
//! MSG_CUT_OBJ_IN_CUT_START/END messages and the 0x2a00002e prim in message 0x136.
//!
//! Code pointers: fake addresses 0x300000.. (see hero_level.rs).
use super::skeleton::addr;
use super::*;
use crate::engine::*;
use crate::logic::LResult;

const SELF: u32 = 0x2a00_0000;
const NULL: Ptr = 0;

#[allow(non_upper_case_globals)]
pub mod addr_race {
    pub const heroInit_RaceWaiting: u32 = 0x32c088;
    pub const heroST_RaceWaiting_Loop: u32 = 0x32c110;
    pub const heroInit_RaceBurping: u32 = 0x32c160;
    pub const heroST_RaceBurping_Loop: u32 = 0x32c350;
    pub const heroInit_RaceFollowerRunning: u32 = 0x32c580;
    pub const heroST_RaceFollowerRunning_Loop: u32 = 0x32c6c0;
    pub const heroInit_RaceLeaderStanding: u32 = 0x32c838;
    pub const heroST_RaceLeaderStanding_Loop: u32 = 0x32ca40;
    pub const heroInit_RaceLeaderMoving: u32 = 0x32cb98;
    pub const heroST_RaceLeaderMoving_Loop: u32 = 0x32cd18;
    pub const heroInit_OpenDoor: u32 = 0x32d9d0;
    pub const heroST_OpenDoor_Loop: u32 = 0x32daf0;
}

pub(crate) fn call_race(e: &mut Engine, f: u32, a0: Ptr) -> Option<LResult<()>> {
    use addr_race as a;
    Some(match f {
        a::heroInit_RaceWaiting => heroInit_RaceWaiting(e, a0),
        a::heroST_RaceWaiting_Loop => heroST_RaceWaiting_Loop(e, a0),
        a::heroInit_RaceBurping => heroInit_RaceBurping(e, a0),
        a::heroST_RaceBurping_Loop => heroST_RaceBurping_Loop(e, a0),
        a::heroInit_RaceFollowerRunning => heroInit_RaceFollowerRunning(e, a0),
        a::heroST_RaceFollowerRunning_Loop => heroST_RaceFollowerRunning_Loop(e, a0),
        a::heroInit_RaceLeaderStanding => heroInit_RaceLeaderStanding(e, a0),
        a::heroST_RaceLeaderStanding_Loop => heroST_RaceLeaderStanding_Loop(e, a0),
        a::heroInit_RaceLeaderMoving => heroInit_RaceLeaderMoving(e, a0),
        a::heroST_RaceLeaderMoving_Loop => heroST_RaceLeaderMoving_Loop(e, a0),
        a::heroInit_OpenDoor => heroInit_OpenDoor(e, a0),
        a::heroST_OpenDoor_Loop => heroST_OpenDoor_Loop(e, a0),
        _ => return None,
    })
}

pub(crate) const RACE_BUILDS: [&str; 2] = ["c33ca2c4", "0117f115"];

fn scooby(e: &Engine) -> bool {
    builds::BUILDS[e.rt.cur_build()].sha8 == "c33ca2c4"
}
/// (wait anim, burp anim, blend anim)
fn anims(e: &Engine) -> (i32, i32, u32) {
    if scooby(e) { (0x15, 0x40, 0x50) } else { (0xf, 0x37, 0x40) }
}
fn LL(e: &Engine) -> Ptr {
    e.r32(G(e, pLocalDataHeroLevel))
}
fn leader(e: &Engine) -> bool {
    e.r32(GB0(e) + 8) == e.r32(L(e) + 0x17c)
}
fn or_w(e: &mut Engine, a: Ptr, v: u32) {
    let x = e.r32(a) | v;
    e.w32(a, x);
}
fn and_w(e: &mut Engine, a: Ptr, v: u32) {
    let x = e.r32(a) & v;
    e.w32(a, x);
}
fn update_blend(e: &mut Engine, p: Ptr) -> LResult<()> {
    if scooby(e) {
        heroFUNC_UpdateBlend_Scooby(e, p, 1)?;
    } else {
        heroFUNC_UpdateBlend_Others(e, p, 1)?;
    }
    Ok(())
}
/// The food blend start shared by RaceBurping / RaceLeaderStanding / RaceLeaderMoving:
/// factor = min(2 * (1 - fullness), 1), PlayBlend(blend anim, mode), and (except Moving) the blend anim
/// speed = duration of the burp anim.
fn food_blend(e: &mut Engine, mode: u32, speed: bool) {
    let ll = LL(e);
    let f = 1.0 - e.rf(ll + 0x3a8);
    e.wf(ll + 0x3b4, f + f);
    if 1.0 < e.rf(ll + 0x3b4) {
        e.w32(ll + 0x3b4, 0x3f80_0000);
    }
    let (_, burp, blend) = anims(e);
    APIanim::PlayBlend(e, SELF, blend, mode, 0.0, LL(e) + 0x3ac);
    if speed {
        let d = APIanim::GetDuration(e, SELF, burp as u32);
        APIanim::SetSpeed(e, SELF, blend, d);
    }
}

pub fn heroInit_RaceWaiting(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    APIgameobj::SetContextFlag(e, SELF, 0x50);
    let w = anims(e).0 as u32;
    APIanim::Play(e, SELF, w, 2, 0.2);
    e.w32(param_1 + 0x158, addr_race::heroST_RaceWaiting_Loop);
    Ok(())
}

pub fn heroST_RaceWaiting_Loop(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    if e.r32(param_1 + 0x168) & 1 != 0 {
        e.w32(param_1 + 0x154, addr::heroInit_FollowStand);
    }
    Ok(())
}

pub fn heroInit_RaceBurping(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    APIgameobj::SetContextFlag(e, SELF, 0x50);
    let burp = anims(e).1;
    let r = heroFUNC_StartAnim(e, 0.1, burp, 1, 5)?;
    e.w32(param_1 + 0x218, r as u32);
    food_blend(e, 1, true);
    APIinput::InputConfigSetState(e, e.r32(L(e) + 0x188) as i32, i32::MIN);
    UpdateTimerOutOfTime(e, 0.5)?;
    if leader(e) {
        APIgameobj::SendMessageTo(e, 0x2a10_0006, NULL, 0x32, 0x17, NULL);
    }
    e.w32(param_1 + 0x158, addr_race::heroST_RaceBurping_Loop);
    Ok(())
}

pub fn heroST_RaceBurping_Loop(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    if e.r32(param_1 + 0x168) & 1 == 0 {
        if leader(e) && IsOutOfTime(e)? != 0 {
            UpdateTimerOutOfTime(e, 999.0)?;
            let v = f32::from_bits(0x3ea8_f5c3);
            heroFUNC_CallVibration(e, v, 0.0, v, 0.0)?;
        }
        let burp = anims(e).1 as u32;
        let t = APIanim::GetTime(e, SELF, burp);
        let d = APIanim::GetDuration(e, SELF, burp);
        let mut s = e.rf(LL(e) + 0x3a8) + t / d;
        if 1.0 < s {
            s = 1.0;
        }
        e.wf(LL(e) + 0x3b4, 1.0 - s);
        hero_localFUNC_UpdateFoodBlend(e, LL(e))?;
    } else {
        if leader(e) {
            e.w32(param_1 + 0x154, addr_race::heroInit_RaceLeaderStanding);
            e.w32(LL(e) + 0x3a8, 0x3f80_0000);
        } else {
            e.w32(param_1 + 0x154, addr_race::heroInit_RaceFollowerRunning);
            e.w32(LL(e) + 0x3a8, 0x3f59_999a);
        }
        hero_localFUNC_UpdatePhysicRace(e, LL(e), NULL, -1)?;
    }
    Ok(())
}

pub fn heroInit_RaceFollowerRunning(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    let ld = e.r32(GB0(e) + 8);
    APIgameobj::SendMessageTo(e, ld, NULL, 0x1e1, 0, NULL);
    APIgameobj::SetContextFlag(e, SELF, 0x3000_2050);
    APIinput::InputConfigSetState(e, e.r32(L(e) + 0x188) as i32, i32::MIN);
    or_w(e, GB0(e) + 0x14, 0x8_0000);
    let r = heroFUNC_StartBlend(e, 0.1, 1, 2, 0)?;
    e.w32(param_1 + 0x218, r as u32);
    let t = APIextra::TimerGetTime(e, 0x9000_0000);
    e.wf(param_1 + 0x2dc, t + 1.0);
    e.w32(param_1 + 0x158, addr_race::heroST_RaceFollowerRunning_Loop);
    Ok(())
}

/// Every second the follower may burp (random), and always once fullness drops below 0.3.
pub fn heroST_RaceFollowerRunning_Loop(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    hero_localFUNC_CheckPointPos(e, LL(e))?;
    update_blend(e, param_1)?;
    if e.rf(param_1 + 0x2dc) < APIextra::TimerGetTime(e, 0x9000_0000) {
        let t = APIextra::TimerGetTime(e, 0x9000_0000);
        e.wf(param_1 + 0x2dc, t + 1.0);
        let r = LIBApiCFunctions::FloatRandom(e, -0.3, 0.1, 0.025);
        if e.rf(LL(e) + 0x3a8) - r < 0.7 {
            e.w32(param_1 + 0x154, addr_race::heroInit_RaceBurping);
        }
    }
    if e.rf(LL(e) + 0x3a8) < 0.3 {
        e.w32(param_1 + 0x154, addr_race::heroInit_RaceBurping);
    }
    Ok(())
}

pub fn heroInit_RaceLeaderStanding(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    let w = anims(e).0;
    let r = LIBAnimMngr::PlayAnim(e, 0.1, w, 2, 0);
    e.w32(param_1 + 0x218, r as u32);
    food_blend(e, 1, true);
    debug_info_value(e, "Blend factor", LL(e) + 0x3b4, 2);
    and_w(e, GB0(e) + 0x14, 0xffff_ffdf);
    APIgameobj::SetContextFlag(e, SELF, 0x1001_0050);
    APIinput::InputConfigSetState(e, e.r32(param_1 + 0x188) as i32, -0x7fff_fbfe);
    e.w32(param_1 + 0x158, addr_race::heroST_RaceLeaderStanding_Loop);
    Ok(())
}

/// The burp request / "too full" message part shared by the two leader loops.
fn leader_burp_check(e: &mut Engine, param_1: Ptr, moving: bool) -> LResult<()> {
    if e.r32(param_1 + 0x16c) & 0x100 == 0 {
        if moving {
            update_blend(e, param_1)?;
        }
        // `**(uint **)(pGlobalDataHero + 8)`: the third global buffer's first word
        if e.rf(LL(e) + 0x3a8) <= 0.75 && e.r32(GB2(e)) & 0x2_0000 == 0 {
            APIgameobj::SendMessageTo(e, 0x2a10_0006, NULL, 0x31, 0x17, NULL);
        }
    } else {
        debug_info(e, "Burp request!");
        e.w32(param_1 + 0x154, addr_race::heroInit_RaceBurping);
    }
    Ok(())
}

pub fn heroST_RaceLeaderStanding_Loop(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    if e.r32(param_1 + 0x16c) & 2 == 0 {
        leader_burp_check(e, param_1, false)?;
    } else {
        super::hero_misc::debug_time(e, "heroST_RaceLeaderStanding_Loop to heroInit_RaceLeaderMoving", 0, 0)?;
        e.w32(param_1 + 0x154, addr_race::heroInit_RaceLeaderMoving);
    }
    Ok(())
}

pub fn heroInit_RaceLeaderMoving(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    let r = heroFUNC_StartBlend(e, 0.1, 1, 2, 0)?;
    e.w32(param_1 + 0x218, r as u32);
    APIgameobj::SetContextFlag(e, SELF, 0x1001_0052);
    APIinput::InputConfigSetState(e, e.r32(param_1 + 0x188) as i32, -0x7fff_fbfd);
    food_blend(e, 2, false);
    and_w(e, GB0(e) + 0x14, 0xffff_ffef);
    e.w32(param_1 + 0x158, addr_race::heroST_RaceLeaderMoving_Loop);
    Ok(())
}

pub fn heroST_RaceLeaderMoving_Loop(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    if e.r32(param_1 + 0x16c) & 1 == 0 {
        leader_burp_check(e, param_1, true)?;
    } else {
        super::hero_misc::debug_time(e, "heroST_RaceLeaderMoving_Loop to heroInit_RaceLeaderStanding", 0, 0)?;
        e.w32(param_1 + 0x154, addr_race::heroInit_RaceLeaderStanding);
    }
    Ok(())
}

/// Follower DMs: track DM (0x1d8) stopped at 15, walk DM (0x1a8) speed = +0x3a4, DM 400 speeds.
pub fn hero_localFUNC_PhysicRace(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    debug_info(e, "hero_localFUNC_PhysicRace");
    // frame: auStack_90 +0, uStack_80 +0x10, auStack_60 +0x30
    let f = e.alloca(0x50);
    let l = L(e);
    let b = f + 0x10;
    APIphysic::DMGetParams(e, e.r32(l + 0x1d8), b);
    e.w32(b + 4, 0x4170_0000);
    for o in [0, 8, 0xc, 0x10] {
        e.w32(b + o, 0);
    }
    e.w32(b + 0x14, 5);
    APIphysic::DMSetParams(e, e.r32(l + 0x1d8), b);
    APIphysic::DMGetParams(e, e.r32(l + 0x1a8), f);
    let v = e.r32(param_1 + 0x3a4);
    e.w32(f + 8, v);
    APIphysic::DMSetParams(e, e.r32(l + 0x1a8), f);
    let c = f + 0x30;
    APIphysic::DMGetParams(e, e.r32(l + 400), c);
    let s = e.rf(param_1 + 0x3a4);
    e.wf(c + 8, s);
    e.wf(c + 0xc, s * 0.333);
    APIphysic::DMSetParams(e, e.r32(l + 400), c);
    Ok(())
}

/// One of the 5 race paths (waypoint lists 0x60..100) at random.
pub fn hero_localFUNC_ChoosePath(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    let r = APIstdlib::MyRand(e) as i32 % 5;
    let p = match r {
        0 => 0x60,
        1 => 0x61,
        2 => 0x62,
        3 => 99,
        _ => 100,
    };
    e.w32(param_1 + 0x388, p);
    let n = APIextra::WaypointGetNb(e, p);
    e.w32(param_1 + 0x390, n);
    e.w32(param_1 + 0x3a0, 0);
    Ok(())
}

pub fn hero_localFUNC_CheckPointPos(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    if LIBApiCFunctions::CheckRangePoint(e, 0.5, SELF, param_1 + 0x394, 0) != 0 {
        if e.r32(param_1 + 0x3a0) < e.r32(param_1 + 0x390).wrapping_sub(1) {
            let i = e.r32(param_1 + 0x3a0) + 1;
            e.w32(param_1 + 0x3a0, i);
            APIextra::WaypointGetPos(e, e.r32(param_1 + 0x388), i, param_1 + 0x394);
            hero_localFUNC_UpdatePhysicRace(e, param_1, param_1 + 0x394, 0)?;
        } else {
            hero_localFUNC_ChoosePath(e, param_1)?;
        }
    }
    Ok(())
}

/// param_3 0: follow the waypoint param_2 at 25. Otherwise: 1 (-0.05) / 2 (-0.15) fullness (min 0.1),
/// speed = local+0x70 * fullness (DM 400 for the leader, DM 0x1a8 for the follower), food blend.
pub fn hero_localFUNC_UpdatePhysicRace(e: &mut Engine, param_1: Ptr, param_2: Ptr, param_3: i32) -> LResult<()> {
    let l = L(e);
    // frame: auStack_b0 +0, auStack_a0 +0x10, uStack_60 +0x50
    let f = e.alloca(0x70);
    if param_3 == 0 {
        let b = f + 0x50;
        APIphysic::DMGetParams(e, e.r32(l + 0x1d8), b);
        e.w32(b + 4, 0x41c8_0000);
        e.w32(b, 0);
        for i in 0..3 {
            let v = e.r32(param_2 + i * 4);
            e.w32(b + 8 + i * 4, v);
        }
        e.w32(b + 0x14, 5);
        APIphysic::DMSetParams(e, e.r32(l + 0x1d8), b);
    } else {
        let fu = param_1 + 0x3a8;
        if param_3 == 1 {
            e.wf(fu, e.rf(fu) - 0.05);
        } else if param_3 == 2 {
            e.wf(fu, e.rf(fu) - 0.15);
        }
        if e.rf(fu) < 0.1 {
            e.w32(fu, 0x3dcc_cccd);
        }
        let s = e.rf(l + 0x70) * e.rf(fu);
        e.wf(param_1 + 0x3a4, s);
        if APIgameobj::GetID(e, SELF, NULL) == e.r32(GB0(e) + 8) {
            let c = f + 0x10;
            APIphysic::DMGetParams(e, e.r32(l + 400), c);
            let s = e.rf(param_1 + 0x3a4);
            e.wf(c + 8, s);
            e.wf(c + 0xc, s * 0.333);
            APIphysic::DMSetParams(e, e.r32(l + 400), c);
            debug_info_value(e, "FUNC_UpdatePhysicRace", c + 8, 2);
        } else {
            APIphysic::DMGetParams(e, e.r32(l + 0x1a8), f);
            let v = e.r32(param_1 + 0x3a4);
            e.w32(f + 8, v);
            APIphysic::DMSetParams(e, e.r32(l + 0x1a8), f);
            debug_info_value(e, "FUNC_UpdatePhysicRace", f + 8, 2);
        }
        // reads pLocalDataHeroLevel, writes param_1 (always the same buffer)
        let x = 1.0 - e.rf(LL(e) + 0x3a8);
        e.wf(param_1 + 0x3b4, x + x);
        hero_localFUNC_UpdateFoodBlend(e, param_1)?;
    }
    Ok(())
}

pub fn hero_localFUNC_UpdateFoodBlend(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    let ll = LL(e);
    if 1.0 < e.rf(ll + 0x3b4) {
        e.w32(ll + 0x3b4, 0x3f80_0000);
    }
    APIanim::ChangeBlendParams(e, SELF, param_1 + 0x3ac);
    debug_info_value(e, "-------------------------Blend factor", param_1 + 0x3b4, 2);
    Ok(())
}

/// ext 0x1d (input action 0x61): burp button, only when fullness <= 0.75.
pub fn heroCB_INPUT_Burp(e: &mut Engine, _: u32) -> LResult<u64> {
    if e.rf(LL(e) + 0x3a8) <= 0.75 {
        or_w(e, L(e) + 0x16c, 0x100);
    }
    Ok(0)
}

pub fn hero_localFUNC_Init_Blend(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    e.w32(param_1 + 0x3ac, 0xbb00_0005);
    for o in [0x3b0, 0x3b4, 0x3b8, 0x3bc] {
        e.w32(param_1 + o, 0);
    }
    Ok(())
}

/// NOTE: called with pLocalDataHeroLevel, so the input config read at +0x188 is the LEVEL buffer's word
/// (as in the original), not the hero's.
pub fn hero_localFUNC_Create_Input(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    let c = e.r32(param_1 + 0x188) as i32;
    APIinput::InputActionAdd(e, c, 0x61, 0);
    let pad = e.r32(GB0(e) + 0x4c) as i32;
    APIinput::InputDeviceAdd(e, c, 0x61, 0x29, pad, 0, 2, 0, 0);
    APIinput::InputDeviceAdd(e, c, 0x61, 0x2a, 1, 1, 2, 0, 0x39);
    APIinput::InputContextAdd(e, SELF, 0x1d, c, 0x61, 0x12, 8);
    APIinput::InputContextSetState(e, c, 0x61, 0x12, 0x400);
    Ok(())
}

pub fn hero_localFUNC_Constructor(e: &mut Engine, _api: Ptr, param_2: i32) -> LResult<()> {
    if param_2 == 1 {
        APIcons::SetExternalFunction(e, 0x1d, heroCB_INPUT_Burp);
    }
    Ok(())
}

// ---- scooby.c / shaggy.c ----

pub(crate) fn check_action_events_local(e: &mut Engine) -> LResult<()> {
    let l = L(e);
    let ev = e.r32(l + 0x170);
    if ev & 0x1000_0000 != 0 {
        e.w32(l + 0x154, addr_race::heroInit_RaceFollowerRunning);
    } else if ev & 0x2000_0000 != 0 {
        e.w32(l + 0x154, addr_race::heroInit_RaceWaiting);
    } else if scooby(e) && ev & 0x4000_0000 != 0 {
        e.w32(l + 0x154, addr_race::heroInit_OpenDoor);
    } else if ev & 0x8000_0000 != 0 {
        e.w32(l + 0x154, addr_race::heroInit_RaceLeaderStanding);
    }
    Ok(())
}

/// c33ca2c4 only. Anim 0x44 at 0.85 when GB0+0x44 == 0x58, else 0x3d.
pub fn heroInit_OpenDoor(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    let r = if e.r32(GB0(e) + 0x44) == 0x58 {
        APIanim::SetSpeed(e, SELF, 0x44, 0.85);
        heroFUNC_StartAnim(e, 0.0, 0x44, 1, 5)?
    } else {
        heroFUNC_StartAnim(e, 0.0, 0x3d, 1, 5)?
    };
    e.w32(param_1 + 0x218, r as u32);
    APIinput::InputConfigSetState(e, e.r32(L(e) + 0x188) as i32, i32::MIN);
    APIgameobj::SetContextFlag(e, SELF, 0x50);
    e.w32(param_1 + 0x158, addr_race::heroST_OpenDoor_Loop);
    Ok(())
}

pub fn heroST_OpenDoor_Loop(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    if e.r32(param_1 + 0x168) & 1 != 0 {
        e.w32(param_1 + 0x154, addr::heroInit_Stand);
    }
    Ok(())
}

/// `scoobyFUNC_Create_Local` / `shaggyFUNC_Create_Local` of the race builds (replaces the common one).
pub fn race_create_local(e: &mut Engine) -> LResult<()> {
    let (l, ll) = (L(e), LL(e));
    let v = e.r32(l + 0x70);
    e.w32(ll + 0x3a4, v);
    e.w32(ll + 0x3a8, 0x3f80_0000);
    let (snd, gen_) = (G(e, goSurfaceSoundScooby), G(e, goGenericSoundScooby));
    e.w32(l, snd);
    e.w32(l + 4, gen_);
    let p = e.r32(l + 0x18c);
    APIphysic::PrimResetIntAttribBits(e, SELF, p, 5, 0x20_0000);
    Ok(())
}

/// MSG_ON (0xde, 0x1e2) / MSG_OFF (0xdf, 0x1e3) set fullness 0.85 / 1 and the speed.
fn set_speed(e: &mut Engine, full: u32) -> LResult<()> {
    let ll = LL(e);
    e.w32(ll + 0x3a8, full);
    let s = e.rf(L(e) + 0x70) * e.rf(ll + 0x3a8);
    e.wf(ll + 0x3a4, s);
    hero_localFUNC_PhysicRace(e, ll)
}

/// The race part of the message handler; always followed by heroFUNC_MessageHandler.
pub(crate) fn message(e: &mut Engine, param_1: u32, param_2: u32) -> LResult<()> {
    let l = L(e);
    match param_1 {
        0x13a => {
            debug_info(e, "MSG_HERO_PUSHEDBYFAN");
            hero_localFUNC_UpdatePhysicRace(e, LL(e), NULL, param_2 as i32)?;
        }
        0xdf | 0x1e3 => {
            debug_info(e, "MSG_OFF");
            set_speed(e, 0x3f80_0000)?;
            or_w(e, l + 0x170, 0x2000_0000);
        }
        300 if scooby(e) => {
            or_w(e, GB0(e) + 0x14, 0x4000_0000);
            or_w(e, l + 0x170, 0x4000_0000);
        }
        0x136 => {
            let p = e.r32(l + 0x18c);
            if param_2 == 0 {
                APIphysic::PrimResetIntAttribBits(e, SELF, p, 5, 0x20_0000);
                if scooby(e) {
                    APIphysic::PrimResetIntAttribBits(e, 0x2a00_002e, 0x84, 5, 0x20_0000);
                }
            } else if param_2 == 1 {
                APIphysic::PrimSetIntAttribBits(e, SELF, p, 5, 0x20_0000);
                if scooby(e) {
                    APIphysic::PrimSetIntAttribBits(e, 0x2a00_002e, 0x84, 5, 0x20_0000);
                }
            }
        }
        0x1e1 => {
            debug_info(e, "MSG_RACE_LEADER_START");
            or_w(e, l + 0x170, 0x8000_0000);
        }
        0x141 if scooby(e) => {
            debug_info(e, "MSG_CUT_OBJ_IN_CUT_START");
            or_w(e, l + 0x170, 0x400_0000);
        }
        0x142 if scooby(e) => {
            debug_info(e, "MSG_CUT_OBJ_IN_CUT_END");
            or_w(e, l + 0x170, 0x800_0000);
        }
        0xde | 0x1e2 => {
            // compares the current loop pointer with the Init function: never equal (original quirk)
            if e.r32(l + 0x158) != addr_race::heroInit_RaceFollowerRunning {
                debug_info(e, "MSG_ON");
                set_speed(e, 0x3f59_999a)?;
                let ll = LL(e);
                hero_localFUNC_ChoosePath(e, ll)?;
                APIextra::WaypointGetPos(e, e.r32(ll + 0x388), e.r32(ll + 0x3a0), ll + 0x394);
                hero_localFUNC_UpdatePhysicRace(e, ll, ll + 0x394, 0)?;
                or_w(e, l + 0x170, 0x1000_0000);
            }
        }
        _ => {}
    }
    Ok(())
}

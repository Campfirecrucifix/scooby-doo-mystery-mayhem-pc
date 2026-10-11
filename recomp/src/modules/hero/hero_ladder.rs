//! hero_ladder.c (7 functions). Addresses are module-relative (.text at 0x10000) in scooby__402420b4.
//! Present in 12 builds (ladder levels). local+0x18 = ladder object, +0x1c = ARG (0 up, 1 down,
//! else "out" with local+0x20 = 0x79 for up-out).
use super::builds::*;
use super::skeleton::addr;
use super::*;
use crate::engine::*;
use crate::logic::LResult;

const SELF: u32 = 0x2a00_0000;

fn gb14_or(e: &mut Engine, v: u32) {
    let a = GB0(e) + 0x14;
    let x = e.r32(a);
    e.w32(a, x | v);
}
fn gb14_and(e: &mut Engine, v: u32) {
    let a = GB0(e) + 0x14;
    let x = e.r32(a);
    e.w32(a, x & v);
}
fn start(e: &mut Engine, param_1: Ptr, blend: f32, anim: u32, mode: u32, fl: u32) -> LResult<()> {
    let a = heroFUNC_StartAnim(e, blend, anim as i32, mode as i32, fl as i32)?;
    e.w32(param_1 + 0x218, a as i32 as u32);
    Ok(())
}

/// 0x00016b90 (1024 bytes) `void heroInit_LadderIn(int param_1)`
pub fn heroInit_LadderIn(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    APIphysic::SetVelocity(e, SELF, 0.0, 0.0, 0.0);
    let arg = e.r32(param_1 + 0x1c);
    if arg == 0 || arg == 1 {
        let (msg, m, slot) = if arg == 0 {
            ("heroInit_LadderIn ARG_LADDER_UP", 0xe1, heroInit_LadderIn_0_1)
        } else {
            ("heroInit_LadderIn ARG_LADDER_DOWN", 0xe2, heroInit_LadderIn_0_2)
        };
        debug_time(e, msg, 0, 0)?;
        gb14_or(e, 0x100);
        APIgameobj::SendMessageTo(e, e.r32(param_1 + 0x18), 0, m, e.r32(param_1 + 0x17c), 0);
        start(e, param_1, 0.0, k(e, slot), 1, 5)?;
        APIphysic::SetFlags(e, SELF, 1);
        APIinput::InputConfigSetState(e, e.r32(param_1 + 0x188) as i32, i32::MIN);
        APIgameobj::SetContextFlag(e, SELF, 0x20);
        e.w32(param_1 + 0x158, addr::heroST_LadderIn_Loop);
    } else {
        APIphysic::SetFlags(e, SELF, 1);
        if e.r32(param_1 + 0x20) == 0x79 {
            debug_time(e, "heroInit_LadderIn ARG_LADDER_UP_OUT", 0, 0)?;
            gb14_or(e, 0x100);
            start(e, param_1, 0.0, k(e, heroInit_LadderIn_0_3), 1, 5)?;
        } else {
            debug_time(e, "heroInit_LadderIn ARG_LADDER_DOWN_OUT", 0, 0)?;
            gb14_or(e, 0x100);
            start(e, param_1, 0.0, k(e, heroInit_LadderIn_0_4), 1, 5)?;
            APIgameobj::SetContextFlag(e, SELF, 0x20);
            APIinput::InputConfigSetState(e, e.r32(param_1 + 0x188) as i32, i32::MIN);
        }
        e.w32(param_1 + 0x158, addr::heroST_LadderOut_Loop);
    }
    Ok(())
}

/// 0x00016f90 (80 bytes) `void heroST_LadderIn_Loop(int param_1)`
pub fn heroST_LadderIn_Loop(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    if e.r32(param_1 + 0x168) & 1 != 0 {
        e.w32(param_1 + 0x154, addr::heroInit_Ladder);
    }
    Ok(())
}

/// 0x00016fe0 (520 bytes) `void heroST_LadderOut_Loop(int param_1)`
/// Note: the original stores the Init functions in +0x158 (the loop slot), not +0x154; kept.
pub fn heroST_LadderOut_Loop(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    if e.r32(param_1 + 0x168) & 1 == 0 {
        return Ok(());
    }
    APIgameobj::LeaveParent(e, SELF, 0);
    APIphysic::ResetFlags(e, SELF, 1);
    let g34 = e.r32(GB0(e) + 0x34);
    let leader = e.r32(GB0(e) + 8) == e.r32(L(e) + 0x17c);
    if g34 & 0x10 == 0 || g34 & 0x100 != 0 {
        APIgameobj::SendMessageTo(e, e.r32(param_1 + 0x18), 0, 0xde, 0, 0);
        gb14_and(e, 0xffff_feff);
        let n = if leader { addr::heroInit_Stand } else { addr::heroInit_FollowStand };
        e.w32(param_1 + 0x158, n);
    } else if leader {
        e.w32(param_1 + 0x158, addr::heroInit_LadderWalkAway);
    } else {
        APIgameobj::SendMessageTo(e, e.r32(param_1 + 0x18), 0, 0xde, 0, 0);
        gb14_and(e, 0xffff_feff);
        e.w32(param_1 + 0x158, addr::heroInit_FollowStand);
    }
    Ok(())
}

/// 0x000171e8 (268 bytes) `void heroInit_Ladder(int param_1)`
/// local+0x254 = the climb-in anim; group 1 (2a9862df) swaps the two debug texts.
pub fn heroInit_Ladder(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    let swap = g(e, G_heroInit_Ladder) == 1;
    let (t1, t2) = ("heroInit_Ladder ANI_LADDER_DOWN_LOOP", "heroInit_Ladder ANI_LADDER_UP_LOOP");
    let (t1, t2) = if swap { (t2, t1) } else { (t1, t2) };
    let cur = e.r32(param_1 + 0x254);
    if cur == k(e, heroInit_Ladder_0_1) {
        debug_time(e, t1, 0, 0)?;
        start(e, param_1, 0.0, k(e, heroInit_Ladder_0_2), 2, 0)?;
    } else if cur == k(e, heroInit_Ladder_0_3) {
        debug_time(e, t2, 0, 0)?;
        start(e, param_1, 0.0, k(e, heroInit_Ladder_0_4), 2, 0)?;
    }
    e.w32(param_1 + 0x158, addr::heroST_Ladder_Loop);
    Ok(())
}

/// 0x000172f8 (36 bytes) `void heroST_Ladder_Loop(void)`
pub fn heroST_Ladder_Loop(_e: &mut Engine) -> LResult<()> {
    Ok(())
}

/// 0x00017320 (728 bytes) `void heroInit_LadderWalkAway(int param_1)`
/// Walks 2.0 (hero id slot, i.e. Scooby) or 1.75 forward off the ladder top via the track DM (+0x1d8).
pub fn heroInit_LadderWalkAway(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    debug_time(e, "heroInit_LadderWalkAway", 0, 0)?;
    let d40 = e.alloca(0x10);
    let d50 = e.alloca(0x10);
    let d60 = e.alloca(0x10);
    let z = if e.r32(GB0(e) + 8) == k(e, heroInit_LadderWalkAway_0_1) { 2.0 } else { 1.75 };
    e.wv3(d40, [0.0, 0.0, z]);
    APIgameobj::TransformVector(e, SELF, d40, d40);
    APIgameobj::GetPosition(e, SELF, ALL, d60);
    debug_info_value(e, "cur", d60, 4);
    APImath::CVectorAdd(e, d50, d60, d40);
    debug_info_value(e, "new", d50, 4);
    let p = e.alloca(0x18); // uStack_90..uStack_7c
    APIphysic::DMGetParams(e, e.r32(param_1 + 0x1d8), p);
    e.w32(p, 0);
    let v = e.rv3(d50);
    e.wv3(p + 8, v);
    e.w32(p + 4, 0x4148_0000);
    e.w32(p + 0x14, 5);
    APIphysic::DMSetParams(e, e.r32(param_1 + 0x1d8), p);
    let q = e.alloca(0x10); // auStack_70
    APIphysic::DMGetParams(e, e.r32(param_1 + 0x1a8), q);
    e.w32(q + 0xc, 1);
    e.w32(q + 8, e.r32(param_1 + 0x74));
    APIphysic::DMSetParams(e, e.r32(param_1 + 0x1a8), q);
    start(e, param_1, f32::from_bits(0x3ca3_d70a), k(e, heroInit_LadderWalkAway_0_2), 2, 0)?;
    APIinput::InputConfigSetState(e, e.r32(param_1 + 0x188) as i32, i32::MIN);
    APIgameobj::SetContextFlag(e, SELF, 0x3000_2050);
    e.w32(param_1 + 0x158, addr::heroST_LadderWalkAway_Loop);
    Ok(())
}

/// 0x000175f8 (112 bytes) `void heroST_LadderWalkAway_Loop(int param_1)`
pub fn heroST_LadderWalkAway_Loop(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    if heroIsTrackWPReached(e, 0.5, param_1)? != 0 {
        e.w32(param_1 + 0x154, addr::heroInit_Stand);
    }
    Ok(())
}

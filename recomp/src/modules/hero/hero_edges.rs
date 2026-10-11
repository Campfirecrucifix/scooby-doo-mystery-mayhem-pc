//! hero_edges.c (11 functions). Addresses are module-relative (.text at 0x10000) in scooby__402420b4.
//! Edge climbing (leader is pulled up by the partner). Absent from 20 builds.
use super::builds::*;
use super::skeleton::addr;
use super::*;
use crate::engine::*;
use crate::logic::LResult;

const SELF: u32 = 0x2a00_0000;
/// bone the follower hangs on (partner's hand joint)
const HAND: u32 = 0x250_0000;

fn start(e: &mut Engine, param_1: Ptr, anim: u32, fl: i32) -> LResult<()> {
    let a = heroFUNC_StartAnim(e, 0.0, anim as i32, 1, fl)?;
    e.w32(param_1 + 0x218, a as i32 as u32);
    Ok(())
}
fn gb14_and(e: &mut Engine, v: u32) {
    let a = GB0(e) + 0x14;
    let x = e.r32(a);
    e.w32(a, x & v);
}

/// 0x00014cc0 (188 bytes) `void heroInit_EdgeDetect(int param_1)`
pub fn heroInit_EdgeDetect(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    APIinput::InputConfigSetState(e, e.r32(param_1 + 0x188) as i32, i32::MIN);
    APIphysic::DMSetActive(e, e.r32(param_1 + 0x1c0), 1);
    APIgameobj::SetContextFlag(e, SELF, 0x2_0020);
    APIphysic::DMResetStateChangeNotifyBits(e, e.r32(param_1 + 0x1c0), 1);
    e.w32(param_1 + 0x158, addr::heroST_EdgeDetect);
    Ok(())
}

/// 0x00014d80 (36 bytes) `void heroST_EdgeDetect(void)`
pub fn heroST_EdgeDetect(_e: &mut Engine) -> LResult<()> {
    Ok(())
}

/// 0x00014da8 (280 bytes) `void heroInit_EdgeIn(int param_1)`
pub fn heroInit_EdgeIn(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    start(e, param_1, k(e, heroInit_EdgeIn_0_1), 0)?;
    gb14_and(e, 0xffff_efff);
    APIinput::InputConfigSetState(e, e.r32(param_1 + 0x188) as i32, i32::MIN);
    APIphysic::DMSetActive(e, e.r32(param_1 + 0x1c0), 1);
    APIgameobj::SetContextFlag(e, SELF, 0x2_0020);
    APIphysic::DMResetStateChangeNotifyBits(e, e.r32(param_1 + 0x1c0), 2);
    e.w32(param_1 + 0x158, addr::heroST_EdgeIn_Loop);
    Ok(())
}

/// 0x00014ec0 (36 bytes) `void heroST_EdgeIn_Loop(void)`
pub fn heroST_EdgeIn_Loop(_e: &mut Engine) -> LResult<()> {
    Ok(())
}

/// 0x00014ee8 (1128 bytes) `void heroInit_EdgeJump(int param_1)`
/// Jumps (JumpOnCoor, params at local+0x270..0x294) onto the leader's hand joint + 0.2 forward.
pub fn heroInit_EdgeJump(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    let d40 = e.alloca(0x10);
    e.wv3(d40, [0.0, 0.0, 0.2]);
    let leader = e.r32(GB0(e) + 8);
    APIgameobj::GetPosition(e, leader, HAND, param_1 + 0x298);
    let leader = e.r32(GB0(e) + 8);
    APIgameobj::InvertTransformVector(e, leader, d40, d40);
    debug_info_value(e, "0.joint", param_1 + 0x298, 4);
    debug_info_value(e, "1.comp", d40, 4);
    start(e, param_1, k(e, heroInit_EdgeJump_0_1), 0)?;
    let x = e.rf(param_1 + 0x298) + e.rf(d40);
    e.wf(param_1 + 0x270, x);
    e.w32(param_1 + 0x274, e.r32(param_1 + 0x29c));
    let z = e.rf(param_1 + 0x2a0) + e.rf(d40 + 8);
    e.wf(param_1 + 0x278, z);
    debug_info_value(e, "2.final", param_1 + 0x270, 4);
    APIphysic::PrimResetIntAttribBits(e, SELF, e.r32(param_1 + 0x18c), 2, 4);
    let p = e.alloca(0x18); // uStack_90..uStack_7c
    APIphysic::DMGetParams(e, e.r32(param_1 + 0x1d8), p);
    e.w32(p, 0);
    for i in 0..3 {
        e.w32(p + 8 + i * 4, e.r32(param_1 + 0x270 + i * 4));
    }
    e.w32(p + 4, 0x4148_0000);
    e.w32(p + 0x14, 5);
    APIphysic::DMSetParams(e, e.r32(param_1 + 0x1d8), p);
    let leader = e.r32(GB0(e) + 8);
    LIBApiCFunctions::TrackID(e, 0.0, SELF as i32, leader);
    let q = e.alloca(0x10); // auStack_70
    APIphysic::DMGetParams(e, e.r32(param_1 + 0x1b8), q);
    e.w32(q + 4, 0xc1a0_0000); // -20.0
    APIphysic::DMSetParams(e, e.r32(param_1 + 0x1b8), q);
    debug_info_value(e, "MaxSpeed", param_1 + 0x288, 2);
    debug_info_value(e, "Height", param_1 + 0x290, 2);
    debug_info_value(e, "uiFlags", param_1 + 0x28c, 1);
    APIphysic::JumpOnCoor(e, SELF, param_1 + 0x270);
    APIphysic::ResetStateChangeNotifyBits(e, SELF, 3);
    APIgameobj::SetContextFlag(e, SELF, 0x40);
    e.w32(param_1 + 0x158, addr::heroST_EdgeJump_Loop);
    Ok(())
}

/// 0x00015350 (812 bytes) `void heroST_EdgeJump_Loop(int param_1)`
/// Once falling (vy <= 0.25) within 0.15 of the hand: hook onto the leader's hand joint.
/// Both branches of the original's hero-id test call the same AddChild; the test has no effect.
pub fn heroST_EdgeJump_Loop(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    let vel = e.alloca(0x10);
    APIphysic::GetVelocity(e, SELF, vel);
    if e.rf(vel + 4) <= 0.25 {
        let leader = e.r32(GB0(e) + 8);
        APIgameobj::GetPosition(e, leader, HAND, param_1 + 0x298);
        let pos = e.alloca(0x10);
        APIgameobj::GetPosition(e, SELF, ALL, pos);
        let diff = e.alloca(4); // afStack_4c
        let d = e.rf(pos + 4) - e.rf(param_1 + 0x29c);
        e.wf(diff, d);
        if 0.15 < d {
            debug_info_value(e, "Height", diff, 2);
        } else {
            let q = e.alloca(0x10); // auStack_40
            APIphysic::DMGetParams(e, e.r32(param_1 + 0x1b8), q);
            e.w32(q + 4, 0xc248_0000); // -50.0
            APIphysic::DMSetParams(e, e.r32(param_1 + 0x1b8), q);
            let leader = e.r32(GB0(e) + 8);
            APIgameobj::GetPosition(e, leader, HAND, param_1 + 0x298);
            debug_info_value(e, "fDiffHeight", diff, 2);
            APIphysic::SetVelocity(e, SELF, 0.0, 0.0, 0.0);
            let leader = e.r32(GB0(e) + 8);
            APIgameobj::AddChild(e, leader, HAND, SELF, 1);
            APIphysic::SetFlags(e, SELF, 1);
            APIgameobj::SendMessageTo(e, e.r32(param_1 + 0x180), 0, 0x60, 0x200, 0);
        }
    }
    Ok(())
}

/// 0x000156a0 (316 bytes) `void heroInit_EdgeUp(int param_1)`
pub fn heroInit_EdgeUp(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    if e.r32(GB0(e) + 8) == e.r32(L(e) + 0x17c) {
        start(e, param_1, k(e, heroInit_EdgeUp_0_1), 5)?;
        APIinput::InputConfigSetState(e, e.r32(param_1 + 0x188) as i32, i32::MIN);
        APIgameobj::SetContextFlag(e, SELF, 0x12_0020);
    } else {
        start(e, param_1, k(e, heroInit_EdgeUp_0_2), 5)?;
        APIinput::InputConfigSetState(e, e.r32(param_1 + 0x188) as i32, i32::MIN);
        APIgameobj::SetContextFlag(e, SELF, 0);
    }
    e.w32(param_1 + 0x158, addr::heroST_EdgeUp_Loop);
    Ok(())
}

/// 0x000157e0 (524 bytes) `void heroST_EdgeUp_Loop(int param_1)`
pub fn heroST_EdgeUp_Loop(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    if e.r32(param_1 + 0x168) & 1 == 0 {
        return Ok(());
    }
    APIphysic::SetStateChangeNotifyBits(e, SELF, 2);
    APIphysic::ResetStateChangeNotifyBits(e, SELF, 1);
    if e.r32(GB0(e) + 8) == e.r32(L(e) + 0x17c) {
        debug_time(e, "ST_EdgeUp_Loop - lastframe", 0, 0)?;
        let dm = e.r32(param_1 + 0x1c0);
        APIphysic::DMSetActive(e, dm, 0);
        APIgameobj::LeaveParent(e, e.r32(param_1 + 0x180), 0);
        APIphysic::ResetFlags(e, e.r32(param_1 + 0x180), 1);
        APIphysic::DMSetStateChangeNotifyBits(e, dm, 1);
        APIphysic::DMSetStateChangeNotifyBits(e, dm, 2);
        e.w32(param_1 + 0x154, addr::heroInit_Stand);
    } else {
        APIphysic::DMDoAction(e, e.r32(param_1 + 0x1dc), 1, 0);
        APIphysic::PrimSetIntAttribBits(e, SELF, e.r32(param_1 + 0x18c), 2, 4);
        gb14_and(e, 0xffff_f7ff);
        e.w32(param_1 + 0x154, addr::heroInit_FollowStand);
    }
    Ok(())
}

/// 0x000159f0 (172 bytes) `void heroInit_EdgeSolo(int param_1)`
pub fn heroInit_EdgeSolo(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    start(e, param_1, k(e, heroInit_EdgeSolo_0_1), 5)?;
    APIinput::InputConfigSetState(e, e.r32(param_1 + 0x188) as i32, i32::MIN);
    APIgameobj::SetContextFlag(e, SELF, 0x12_0020);
    e.w32(param_1 + 0x158, addr::heroST_EdgeSolo_Loop);
    Ok(())
}

/// 0x00015aa0 (324 bytes) `void heroST_EdgeSolo_Loop(int param_1)`
pub fn heroST_EdgeSolo_Loop(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    if e.r32(param_1 + 0x168) & 1 != 0 {
        APIphysic::SetStateChangeNotifyBits(e, SELF, 2);
        APIphysic::ResetStateChangeNotifyBits(e, SELF, 1);
        debug_time(e, "ST_EdgeSolo_Loop - lastframe", 0, 0)?;
        let dm = e.r32(param_1 + 0x1c0);
        APIphysic::DMSetActive(e, dm, 0);
        APIphysic::DMSetStateChangeNotifyBits(e, dm, 1);
        APIphysic::DMSetStateChangeNotifyBits(e, dm, 2);
        gb14_and(e, 0xffff_f7ff);
        e.w32(param_1 + 0x154, addr::heroInit_Stand);
    }
    Ok(())
}

/// 0x00015be8 (424 bytes) `void heroFUNC_CreateEdgesPhysic(int param_1)`
/// Edge-detect DM 0x18 -> +0x1c0 (inactive) and the JumpOnCoor block at +0x270 (max speed 7.5, flags 2,
/// height 0.5, DM +0x1b8).
pub fn heroFUNC_CreateEdgesPhysic(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    let input = e.r32(param_1 + 0x188);
    let ctx = k(e, heroFUNC_CreateEdgesPhysic_0_1);
    dm(
        e,
        param_1,
        0x18,
        0x1c0,
        &[(0, 0x3fc9_0fdb), (4, 0x3f8c_cccd), (8, ctx), (0xc, input), (0x10, 0x2a), (0x14, 0x4060_0000), (0x18, 0x3f00_0000), (0x1c, 0x3f00_0000), (0x20, 0x3f00_0000)],
        0x2_0000,
    );
    APIphysic::DMSetActive(e, e.r32(param_1 + 0x1c0), 0);
    for o in [0x270, 0x274, 0x278, 0x27c, 0x280, 0x284] {
        e.w32(param_1 + o, 0);
    }
    e.w32(param_1 + 0x288, 0x40f0_0000);
    e.w32(param_1 + 0x28c, 2);
    e.w32(param_1 + 0x290, 0x3f00_0000);
    e.w32(param_1 + 0x294, e.r32(param_1 + 0x1b8));
    Ok(())
}

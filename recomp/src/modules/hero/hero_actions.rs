//! hero_actions.c (8 functions). Addresses are module-relative (.text at 0x10000) in scooby__402420b4.
//! Absent from 23 builds (no action states there); the dispatcher never reaches these in them.
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
fn is_leader(e: &Engine) -> bool {
    e.r32(GB0(e) + 8) == e.r32(L(e) + 0x17c)
}
fn one_shot(e: &mut Engine, param_1: Ptr) {
    e.w32(param_1 + 0x158, addr::heroST_ActionOneShot);
    e.w32(param_1 + 0x15c, addr::heroExit_ActionOneShot);
}
/// Snap the hero onto the item's dummy `node` (position + orientation).
fn snap(e: &mut Engine, param_1: Ptr, pos: Ptr, node: u32, rot: u32) {
    let item = e.r32(param_1 + 0x23c);
    APIgameobj::GetPosition(e, item, node, pos);
    APIgameobj::SetTranslation(e, SELF, pos);
    APIgameobj::TakeOrientation(e, SELF, item, rot);
}
fn start(e: &mut Engine, param_1: Ptr, blend: f32, anim: u32) -> LResult<()> {
    let a = heroFUNC_StartAnim(e, blend, anim as i32, 1, 5)?;
    e.w32(param_1 + 0x218, a as i32 as u32);
    Ok(())
}

/// 0x00013bc0 (2396 bytes) `void heroInit_Action(int param_1)`
/// local+0x240 = item kind: 0 push button, 1 carried/pushed object (loop), 2/3 use (msg 0xde/0xdf),
/// 4 two-hero item (dummies 0x2a000091/92 per build), 5/6 lever-like, 7 (group 1 only: node from +0x244/+0x248).
/// Anim ids of cases 2/3 are not slotted; per group: 0 = 0xfffff855/0xfffff856, 1 = 0x11/0x12, 2 = 0x10/0x12.
pub fn heroInit_Action(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    let gi = g(e, G_heroInit_Action);
    // case 2/3 (pull lever) anim ids are literals that vary inside a shape group (from each
    // build's hero_actions.c); a wrong id makes StartAnim fail, the one-shot never ends and the
    // hero keeps GB0+0x14 bit 0x100 (blocks e.g. the EP4/L01 pipes)
    let (a2, a3) = match (crate::modules::hero::builds::BUILDS[e.rt.cur_build()].sha8, gi) {
        ("11df665e", _) => (0x11, 0x12),
        ("4fb89b85", _) => (0x12, 0x13),
        ("7e698dff", _) => (0x13, 0x15),
        ("160eeb3e", _) => (0x15, 0x17),
        ("97105e68", _) => (0x11, 0x13),
        (_, 0) => (0xfffff855u32, 0xfffff856u32),
        (_, 1) => (0x11, 0x12),
        (_, 2) => (0x10, 0x12),
        _ => unported_shape(e, "heroInit_Action", G_heroInit_Action),
    };
    let pos = e.alloca(0x10); // DStack_60
    let fb = 0.1f32; // 0x3dcccccd
    let kind = e.r32(param_1 + 0x240);
    match kind {
        0 => {
            debug_time(e, "heroInit_Action -> ITEM_PUSH_BUTTON", 0, 0)?;
            snap(e, param_1, pos, 3, 3);
            gb14_or(e, 0x100);
            APIgameobj::SendMessageTo(e, e.r32(param_1 + 0x23c), 0, 0xde, 0, 0);
            start(e, param_1, fb, k(e, heroInit_Action_0_1))?;
            one_shot(e, param_1);
        }
        1 => {
            gb14_or(e, 0x100);
            APIgameobj::AddChild(e, e.r32(param_1 + 8), 2, SELF, 1);
            APIphysic::PrimResetIntAttribBits(e, SELF, e.r32(param_1 + 0x18c), 2, 2);
            start(e, param_1, fb, k(e, heroInit_Action_0_2))?;
            e.w32(param_1 + 0x25c, 0);
            e.w32(param_1 + 0x158, addr::heroST_ActionIn);
            e.w32(param_1 + 0x15c, addr::heroExit_ActionLoop);
        }
        2 | 3 => {
            snap(e, param_1, pos, 5, 5);
            start(e, param_1, fb, if kind == 2 { a2 } else { a3 })?;
            let id = APIgameobj::GetID(e, SELF, 0);
            let msg = if kind == 2 { 0xde } else { 0xdf };
            APIgameobj::SendMessageTo(e, e.r32(param_1 + 0x23c), 0, msg, 0, id);
            one_shot(e, param_1);
        }
        4 => {
            if is_leader(e) {
                let d50 = e.alloca(0x10);
                e.wf(d50, 1.0);
                e.wf(d50 + 4, 0.1);
                e.wf(d50 + 8, 0.0);
                let item = e.r32(param_1 + 0x23c);
                APIgameobj::GetPosition(e, item, 2, pos);
                APIgameobj::SetTranslation(e, k(e, heroInit_Action_0_3), pos);
                APIgameobj::TakeOrientation(e, k(e, heroInit_Action_0_4), item, 2);
                APIgameobj::TransformVector(e, item, d50, d50);
                for i in 0..3 {
                    let v = e.rf(d50 + i * 4) + e.rf(pos + i * 4);
                    e.wf(d50 + i * 4, v);
                }
                APIgameobj::SetTranslation(e, k(e, heroInit_Action_0_5), d50);
                APIgameobj::TakeOrientation(e, k(e, heroInit_Action_0_6), item, 2);
                gb14_or(e, 0x100);
                start(e, param_1, 0.0, k(e, heroInit_Action_0_7))?;
                APIgameobj::SendMessageTo(e, e.r32(param_1 + 0x23c), 0, 0x9c, 0, 0);
            } else {
                start(e, param_1, 0.0, k(e, heroInit_Action_0_8))?;
            }
            one_shot(e, param_1);
        }
        5 | 6 => {
            snap(e, param_1, pos, 2, 2);
            gb14_or(e, 0x100);
            let (slot, msg) = if kind == 5 { (heroInit_Action_0_9, 0xde) } else { (heroInit_Action_0_10, 0x9c) };
            start(e, param_1, 0.0, k(e, slot))?;
            APIgameobj::SendMessageTo(e, e.r32(param_1 + 0x23c), 0, msg, 0, 0);
            one_shot(e, param_1);
        }
        7 if gi == 1 => {
            let (n, r) = (e.r32(param_1 + 0x244), e.r32(param_1 + 0x248));
            snap(e, param_1, pos, n, r);
            gb14_or(e, 0x100);
            start(e, param_1, 0.0, k(e, heroInit_Action_1_1))?;
            APIgameobj::SendMessageTo(e, e.r32(param_1 + 0x23c), 0, 0x9c, 0, 0);
            one_shot(e, param_1);
        }
        _ => {
            debug_time(e, "heroInit_Action -> default", param_1 + 0x240, 1)?;
        }
    }
    APIphysic::SetVelocity(e, SELF, 0.0, 0.0, 0.0);
    APIgameobj::SetContextFlag(e, SELF, 0x50);
    APIinput::InputConfigSetState(e, e.r32(param_1 + 0x188) as i32, i32::MIN);
    Ok(())
}

/// 0x00014520 (140 bytes) `void heroST_ActionOneShot(int param_1)`
pub fn heroST_ActionOneShot(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    if e.r32(param_1 + 0x168) & 1 != 0 {
        let n = if is_leader(e) { addr::heroInit_Stand } else { addr::heroInit_FollowStand };
        e.w32(param_1 + 0x154, n);
    }
    Ok(())
}

/// 0x000145b0 (80 bytes) `void heroExit_ActionOneShot(void)`
pub fn heroExit_ActionOneShot(e: &mut Engine) -> LResult<()> {
    gb14_and(e, 0xffff_feff);
    Ok(())
}

/// 0x00014600 (320 bytes) `void heroST_ActionIn(int param_1)`
/// Note: the loop-state assignment happens on anim end regardless of kind (inside the && in C).
pub fn heroST_ActionIn(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    if e.r32(param_1 + 0x168) & 1 != 0 {
        e.w32(param_1 + 0x158, addr::heroST_ActionLoop);
        if e.r32(param_1 + 0x240) == 1 {
            let t = e.r32(param_1 + 0x1f8);
            APIextra::TimerReset(e, t);
            APIextra::TimerSetFrequency(e, t, 0.051);
            APIextra::TimerRegisterNotify(e, SELF, t, 10);
            let a = heroFUNC_StartAnim(e, 0.0, k(e, heroST_ActionIn_0_1) as i32, 2, 5)?;
            e.w32(param_1 + 0x218, a as i32 as u32);
            APIgameobj::SendMessageTo(e, e.r32(param_1 + 8), 0, 0xde, 0, 0);
        }
    }
    Ok(())
}

/// 0x00014740 (744 bytes) `void heroST_ActionLoop(int param_1)`
/// Pushing loop: repeats the anim local+0x14 times, then lets go. Event bit 2 = push step
/// (moves the object 0.042375 forward). Vibrates every 0.25 s.
pub fn heroST_ActionLoop(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    if e.r32(param_1 + 0x168) & 1 != 0 && e.r32(param_1 + 0x240) == 1 {
        let n = e.r32(param_1 + 0x25c);
        if n < e.r32(param_1 + 0x14) {
            e.w32(param_1 + 0x25c, n + 1);
            APIgameobj::RegisterNotify(e, SELF, e.r32(param_1 + 0x218), 1, 5);
        } else {
            e.w32(param_1 + 0x158, addr::heroST_ActionOut);
            APIgameobj::LeaveParent(e, SELF, 0);
            let a = heroFUNC_StartAnim(e, 0.0, k(e, heroST_ActionLoop_0_1) as i32, 1, 5)?;
            e.w32(param_1 + 0x218, a as i32 as u32);
            if is_leader(e) {
                APIgameobj::SendMessageTo(e, e.r32(param_1 + 8), 0, 0xda, 0, 0);
            }
            APIextra::TimerUnregisterNotify(e, SELF, e.r32(param_1 + 0x1f8));
        }
        debug_time(e, "heroST_ActionLoop event_CALLBACK_LASTFRAME", param_1 + 0x25c, 1)?;
    }
    if e.r32(param_1 + 0x168) & 2 != 0 {
        let f = e.r32(param_1 + 0x168);
        e.w32(param_1 + 0x168, f & 0xffff_fffd);
        let v = e.alloca(0x10);
        e.wf(v, 0.0);
        e.wf(v + 4, 0.0);
        e.w32(v + 8, 0x3d2d_9168); // 0.042375002
        APIgameobj::TransformVector(e, SELF, v, v);
        APIgameobj::Translate(e, e.r32(param_1 + 8), v);
    }
    if e.r32(param_1 + 0x240) == 1 && e.rf(param_1 + 0x378) < APIextra::TimerGetTime(e, 0x9000_0000) {
        let t = APIextra::TimerGetTime(e, 0x9000_0000);
        e.wf(param_1 + 0x378, t + 0.25);
        heroFUNC_CallVibration(e, f32::from_bits(0x3ea8_f5c3), 0.0, 0.25, 0.25)?;
    }
    Ok(())
}

/// 0x00014a28 (204 bytes) `void heroST_ActionOut(int param_1)`
pub fn heroST_ActionOut(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    if e.r32(param_1 + 0x168) & 1 != 0 {
        if e.r32(param_1 + 0x240) == 1 && is_leader(e) {
            APIgameobj::SendMessageTo(e, e.r32(param_1 + 8), 0, 0xdf, 0, 0);
        }
        e.w32(param_1 + 0x154, addr::heroInit_Stand);
    }
    Ok(())
}

/// 0x00014af8 (188 bytes) `void heroExit_ActionLoop(int param_1)`
pub fn heroExit_ActionLoop(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    APIphysic::PrimSetIntAttribBits(e, SELF, e.r32(param_1 + 0x18c), 2, 2);
    gb14_and(e, 0xffff_feff);
    APIextra::TimerUnregisterNotify(e, SELF, e.r32(param_1 + 0x1f8));
    APIgameobj::LeaveParent(e, SELF, 0);
    Ok(())
}

/// 0x00014bb8 (264 bytes) `void heroFUNC_UseItem(int param_1,int param_2,undefined4 *param_3)`
/// Item type 7 -> action kind 6, type 8 -> kind 4 (the leader also forwards it to the partner).
pub fn heroFUNC_UseItem(e: &mut Engine, param_1: Ptr, param_2: i32, param_3: Ptr) -> LResult<()> {
    let it = e.r32(param_3);
    e.w32(param_1 + 0x23c, it);
    if param_2 == 7 || param_2 == 8 {
        e.w32(param_1 + 0x240, if param_2 == 7 { 6 } else { 4 });
        let f = e.r32(param_1 + 0x170);
        e.w32(param_1 + 0x170, f | 0x1000);
        if param_2 == 8 && is_leader(e) {
            APIgameobj::SendMessageTo(e, e.r32(param_1 + 0x180), 0, 0x9c, 8, param_3);
        }
    }
    Ok(())
}

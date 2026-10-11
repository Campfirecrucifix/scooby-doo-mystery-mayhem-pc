//! hero_actions.c (8 functions). Addresses are module-relative (.text at 0x10000) in scooby_ghost__08a011d0.
use super::builds::*;
use super::*;
use crate::engine::*;
use crate::logic::LResult;

/// 0x00012f70 (2396 bytes) `void heroInit_Action(int param_1)`
/// local+0x240 = item kind: 0 push button, 1 carried/pushed object (loop), 2/3 use (msg 0xde/0xdf),
/// 4 two-hero item (dummies 0x2a000091/92 per build), 5/6 lever-like.
pub fn heroInit_Action(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    let pos = e.alloca(0x10); // DStack_60
    match e.r32(param_1 + 0x240) {
        0 => {
            debug_time(e, "heroInit_Action -> ITEM_PUSH_BUTTON", 0, 0)?;
            snap(e, param_1, pos, 3, 3);
            gb14_or(e, 0x100);
            APIgameobj::SendMessageTo(e, e.r32(param_1 + 0x23c), 0, 0xde, 0, 0);
            start(e, param_1, 0.1, 0xfffff857)?;
            one_shot(e, param_1);
        }
        1 => {
            gb14_or(e, 0x100);
            APIgameobj::AddChild(e, e.r32(param_1 + 8), 2, SELF, 1);
            APIphysic::PrimResetIntAttribBits(e, SELF, e.r32(param_1 + 0x18c), 2, 2);
            start(e, param_1, 0.1, 0xfffff85d)?;
            e.w32(param_1 + 0x25c, 0);
            e.w32(param_1 + 0x158, addr::heroST_ActionIn);
            e.w32(param_1 + 0x15c, addr::heroExit_ActionLoop);
        }
        kind @ (2 | 3) => {
            snap(e, param_1, pos, 5, 5);
            start(e, param_1, 0.1, if kind == 2 { 0xfffff855 } else { 0xfffff856 })?;
            let id = APIgameobj::GetID(e, SELF, 0);
            let msg = if kind == 2 { 0xde } else { 0xdf };
            APIgameobj::SendMessageTo(e, e.r32(param_1 + 0x23c), 0, msg, 0, id);
            one_shot(e, param_1);
        }
        4 => {
            if is_leader(e) {
                let v = e.alloca(0x10); // DStack_50
                e.wf(v, 1.0);
                e.wf(v + 4, 0.1);
                e.wf(v + 8, 0.0);
                let item = e.r32(param_1 + 0x23c);
                APIgameobj::GetPosition(e, item, 2, pos);
                APIgameobj::SetTranslation(e, k(e, heroInit_Action_0_1), pos);
                APIgameobj::TakeOrientation(e, k(e, heroInit_Action_0_2), item, 2);
                APIgameobj::TransformVector(e, item, v, v);
                for o in [0, 4, 8] {
                    let f = e.rf(v + o) + e.rf(pos + o);
                    e.wf(v + o, f);
                }
                APIgameobj::SetTranslation(e, k(e, heroInit_Action_0_3), v);
                APIgameobj::TakeOrientation(e, k(e, heroInit_Action_0_4), item, 2);
                gb14_or(e, 0x100);
                start(e, param_1, 0.0, 0xfffff85a)?;
                APIgameobj::SendMessageTo(e, item, 0, 0x9c, 0, 0);
            } else {
                start(e, param_1, 0.0, 0xfffff85a)?;
            }
            one_shot(e, param_1);
        }
        kind @ (5 | 6) => {
            snap(e, param_1, pos, 2, 2);
            gb14_or(e, 0x100);
            let (anim, msg) = if kind == 5 { (0xfffff858, 0xde) } else { (0xfffff859, 0x9c) };
            start(e, param_1, 0.0, anim)?;
            APIgameobj::SendMessageTo(e, e.r32(param_1 + 0x23c), 0, msg, 0, 0);
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

/// 0x000138d0 (140 bytes) `void heroST_ActionOneShot(int param_1)`
pub fn heroST_ActionOneShot(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    if e.r32(param_1 + 0x168) & 1 != 0 {
        let n = if is_leader(e) { addr::heroInit_Stand } else { addr::heroInit_FollowStand };
        e.w32(param_1 + 0x154, n);
    }
    Ok(())
}

/// 0x00013960 (80 bytes) `void heroExit_ActionOneShot(void)`
pub fn heroExit_ActionOneShot(e: &mut Engine) -> LResult<()> {
    gb14_and(e, 0xffff_feff);
    Ok(())
}

/// 0x000139b0 (320 bytes) `void heroST_ActionIn(int param_1)`
/// Note: the loop-state assignment happens on anim end regardless of kind (inside the && in C).
pub fn heroST_ActionIn(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    if e.r32(param_1 + 0x168) & 1 != 0 {
        e.w32(param_1 + 0x158, addr::heroST_ActionLoop);
        if e.r32(param_1 + 0x240) == 1 {
            let t = e.r32(param_1 + 0x1f8);
            APIextra::TimerReset(e, t);
            APIextra::TimerSetFrequency(e, t, 0.051);
            APIextra::TimerRegisterNotify(e, SELF, t, 10);
            let a = heroFUNC_StartAnim(e, 0.0, 0xfffff85eu32 as i32, 2, 5)?;
            e.w32(param_1 + 0x218, a as i32 as u32);
            APIgameobj::SendMessageTo(e, e.r32(param_1 + 8), 0, 0xde, 0, 0);
        }
    }
    Ok(())
}

/// 0x00013af0 (744 bytes) `void heroST_ActionLoop(int param_1)`
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
            start(e, param_1, 0.0, 0xfffff85f)?;
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

/// 0x00013dd8 (204 bytes) `void heroST_ActionOut(int param_1)`
pub fn heroST_ActionOut(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    if e.r32(param_1 + 0x168) & 1 != 0 {
        if e.r32(param_1 + 0x240) == 1 && is_leader(e) {
            APIgameobj::SendMessageTo(e, e.r32(param_1 + 8), 0, 0xdf, 0, 0);
        }
        e.w32(param_1 + 0x154, addr::heroInit_Stand);
    }
    Ok(())
}

/// 0x00013ea8 (188 bytes) `void heroExit_ActionLoop(int param_1)`
pub fn heroExit_ActionLoop(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    APIphysic::PrimSetIntAttribBits(e, SELF, e.r32(param_1 + 0x18c), 2, 2);
    gb14_and(e, 0xffff_feff);
    APIextra::TimerUnregisterNotify(e, SELF, e.r32(param_1 + 0x1f8));
    APIgameobj::LeaveParent(e, SELF, 0);
    Ok(())
}

/// 0x00013f68 (264 bytes) `void heroFUNC_UseItem(int param_1,int param_2,undefined4 *param_3)`
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


// ---- helpers (from the hero port) ----

/// Rust-side convenience for the `heroFUNC_DebugTime("<literal>", ptr, kind)` call sites: interns the
/// literal only in debug builds (release builds pass nothing).
pub(crate) fn debug_time(e: &mut Engine, msg: &str, p2: Ptr, p3: u32) -> LResult<u32> {
    if debug_build(e) {
        let m = e.cstr(msg);
        heroFUNC_DebugTime(e, m, p2, p3)
    } else {
        heroFUNC_DebugTime(e, 0, 0, 0)
    }
}

/// The build is a debug flavour (heroFUNC_DebugTime has a body, `pDebug` prints compiled in).
#[inline]
pub(crate) fn debug_build(e: &Engine) -> bool {
    true // every ghost build has the heroFUNC_DebugTime body
}

pub(crate) fn gb14_or(e: &mut Engine, v: u32) {
    let a = GB0(e) + 0x14;
    let x = e.r32(a);
    e.w32(a, x | v);
}

pub(crate) fn is_leader(e: &Engine) -> bool {
    e.r32(GB0(e) + 8) == e.r32(L(e) + 0x17c)
}

pub(crate) fn one_shot(e: &mut Engine, param_1: Ptr) {
    e.w32(param_1 + 0x158, addr::heroST_ActionOneShot);
    e.w32(param_1 + 0x15c, addr::heroExit_ActionOneShot);
}

/// Snap the hero onto the item's dummy `node` (position + orientation).
pub(crate) fn snap(e: &mut Engine, param_1: Ptr, pos: Ptr, node: u32, rot: u32) {
    let item = e.r32(param_1 + 0x23c);
    APIgameobj::GetPosition(e, item, node, pos);
    APIgameobj::SetTranslation(e, SELF, pos);
    APIgameobj::TakeOrientation(e, SELF, item, rot);
}

pub(crate) fn start(e: &mut Engine, param_1: Ptr, blend: f32, anim: u32) -> LResult<()> {
    let a = heroFUNC_StartAnim(e, blend, anim as i32, 1, 5)?;
    e.w32(param_1 + 0x218, a as i32 as u32);
    Ok(())
}

pub(crate) fn gb14_and(e: &mut Engine, v: u32) {
    let a = GB0(e) + 0x14;
    let x = e.r32(a);
    e.w32(a, x & v);
}

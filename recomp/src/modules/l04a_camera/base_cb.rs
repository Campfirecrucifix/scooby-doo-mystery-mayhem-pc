//! camera_base.c: states, CheckCameraMode, timer / anim / cut-scene / input callbacks (WP-L1).
#![allow(non_snake_case, unused_variables)]
use super::*;

/// `SendMessageTo(0xff000001, {1}, id, p2, NULL)` (object retriever "all objects of type 1")
pub(super) fn SendAll(e: &mut Engine, id: u32, p2: u32) {
    let r = e.alloca(8);
    e.w32(r, 1);
    APIgameobj::SendMessageTo(e, 0xff00_0001, r, id, p2, NULL);
}

/// 0x00017d08 state 1 (base): clip planes + first update
pub fn Camera_BaseST_FIRSTMOVE(e: &mut Engine) -> LResult<u64> {
    Camera_BaseFUNC_SetClipPlane(e);
    Camera_BaseFUNC_Update(e, 2, 0)?;
    Ok(0)
}

/// 0x00017d70 state 2 (follow)
pub fn Camera_BaseST_BASICMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    Camera_BaseFUNC_CheckCameraMode(e)?;
    if e.r32(LB(e) + 0x168) == 0 { cameraST_CAMERA_FOLLOW(e) } else { Ok(0) }
}

/// 0x00017df8 state 3 (user camera)
pub fn Camera_BaseST_USER(e: &mut Engine, _: u32) -> LResult<u64> {
    Camera_BaseFUNC_CheckCameraMode(e)?;
    Ok(0)
}

/// 0x00017e48 Camera_BaseFUNC_CheckCameraMode
pub fn Camera_BaseFUNC_CheckCameraMode(e: &mut Engine) -> LResult<u64> {
    let gdb = GDB(e);
    let l = LB(e);
    if e.r32(l + 0xf0) != 1 && e.r32(l + 0x4c) & 0x1000_0000 == 0 {
        Camera_BaseFUNC_GetCameraMode(e)?;
    }
    if e.ri32(l + 0x13c) != -1 {
        // debug text window (never created in retail: +0x13c stays -1). All 5 varargs passed
        // (WP-P4): mode, +0x14, (int) leader position x/y/z.
        let p = e.alloca(0x10);
        let lead = e.r32(e.r32(gdb) + 8);
        APIgameobj::GetPosition(e, lead, 0x7fff_ffff, p);
        let (w, a) = (e.r32(l + 0x13c), e.r32(e.r32(gdb) + 0x24));
        let b = e.r32(e.r32(gdb) + 0x14);
        let (x, y, z) = (e.rf(p) as i32 as u32, e.rf(p + 4) as i32 as u32, e.rf(p + 8) as i32 as u32);
        APIhud::TextWindowSetText(e, w, 0x87, &[a, b, x, y, z]);
    }
    if e.r32(l + 0xf4) == 2 && e.r32(e.r32(gdb) + 0x14) & 0x800_0000 == 0 {
        Camera_BaseFUNC_SetCameraBattle(e, l + 0x54)?;
    }
    let m = e.r32(l + 0xec);
    if m == 0 || m >= 9 {
        return Ok(0);
    }
    let fl = |e: &Engine| e.rf(l + 0x170);
    let wins = |e: &Engine| (e.r32(l + 0x130), e.r32(l + 0x134));
    match m {
        2 => {
            if e.r32(l + 0x130) == 0 {
                SendAll(e, 0x4e, 0);
                APIphysic::SetVelocity(e, SELF_CAM, 0.0, 0.0, 0.0);
                Camera_BaseFUNC_SetWindows(e, 1);
                e.w32(l + 0x170, 0x3ec0_0000);
                let (w0, w1) = wins(e);
                APIhud::WindowSetPos(e, w0, 0.0, fl(e) - 0.375);
                APIhud::WindowSetPos(e, w1, 0.0, 1.375 - fl(e));
            } else if fl(e) < e.rf(l + 0x174) {
                e.w32(l + 0x170, 0x3ec0_0000);
                let (w0, w1) = wins(e);
                APIhud::WindowSetPos(e, w0, 0.0, fl(e) - 0.375);
                APIhud::WindowSetPos(e, w1, 0.0, 1.375 - fl(e));
            }
        }
        1 => {
            if e.r32(l + 0x130) == 0 {
                SendAll(e, 0x4c, 0);
                APIphysic::SetVelocity(e, SELF_CAM, 0.0, 0.0, 0.0);
                e.w32(l + 0x170, 0);
                e.w32(l + 0x178, 0x3c4c_cccd);
                e.w32(l + 0x174, 0x3e00_0000);
                Camera_BaseFUNC_SetWindows(e, 1);
            } else if fl(e) < e.rf(l + 0x174) {
                let v = fl(e) + e.rf(l + 0x178);
                e.wf(l + 0x170, v);
                let (w0, w1) = wins(e);
                APIhud::WindowSetPos(e, w0, 0.0, fl(e) - 0.375);
                APIhud::WindowSetPos(e, w1, 0.0, 1.0 - fl(e));
                let p = e.alloca(8);
                APIhud::WindowGetPos(e, w1, p, p + 4);
            } else {
                SendAll(e, 0x4e, 0);
                e.w32(l + 0xec, 2);
            }
        }
        3 => {
            if 0.0 < fl(e) {
                let v = fl(e) - e.rf(l + 0x178);
                e.wf(l + 0x170, v);
                let (w0, w1) = wins(e);
                APIhud::WindowSetPos(e, w0, 0.0, fl(e) - 0.375);
                APIhud::WindowSetPos(e, w1, 0.0, 1.0 - fl(e));
            } else {
                e.w32(l + 0xec, 4);
            }
        }
        4 => {
            SendAll(e, 0x51, 0);
            Camera_BaseFUNC_SetWindows(e, 0);
            e.w32(l + 0x170, 0);
            e.w32(l + 0xec, 0);
        }
        _ => {}
    }
    Ok(0)
}

/// 0x000184d8 callback 0x10 (timers)
pub fn Camera_BaseCB_TIMER(e: &mut Engine, param_1: u32) -> LResult<u64> {
    let l = LB(e);
    if param_1 == e.r32(l + 0x118) {
        let t = e.r32(l + 0x118);
        if e.r32(l + 0xe8) == 6 {
            e.w32(l + 0xe8, 0);
            Camera_BaseFUNC_FadeInOut(e, 1.0, 1);
            return Ok(0);
        }
        let m = e.r32(l + 0xe8);
        if m == 1 || m == 5 {
            SendAll(e, 0x4f, 0);
        }
        let m = e.r32(l + 0xe8);
        if m == 3 {
            let g = e.r32(GDB(e));
            if e.r32(g + 0x7c) == 0 {
                let w = e.r32(l + 0x138);
                APIhud::WindowSetFlags(e, w, 0x100);
                APIextra::TimerSetFrequency(e, t, 0.0);
                Ok(1)
            } else {
                SendAll(e, 0x52, 1);
                e.w32(g + 0x7c, 0);
                APIextra::TimerSetFrequency(e, t, 1.0);
                Camera_BaseFUNC_SetClipPlane(e);
                Ok(0)
            }
        } else if m == 5 {
            e.w32(l + 0xe8, 6);
            APIextra::TimerReset(e, t);
            APIextra::TimerSetFrequency(e, t, 0.24);
            Ok(0)
        } else {
            Ok(1)
        }
    } else if param_1 == e.r32(l + 0x11c) {
        let t = e.r32(l + 0x11c);
        APIextra::TimerSetFrequency(e, t, 0.0);
        if e.r32(l + 0xec) != 0 {
            e.w32(l + 0xec, 4);
        }
        Camera_BaseFUNC_ResetMode(e, 0)?;
        SendAll(e, 0x50, 0);
        Ok(1)
    } else {
        if param_1 == e.r32(l + 0x124) {
            let t = e.r32(l + 0x124);
            APIextra::TimerSetFrequency(e, t, 0.0);
            if e.ri32(l + 0x13c) != -1 {
                let w = e.r32(l + 0x13c);
                APIhud::WindowDelete(e, w);
                e.w32(l + 0x13c, 0xffff_ffff);
            }
        } else if param_1 == e.r32(l + 300) {
            let t = e.r32(l + 300);
            APIextra::TimerUnregisterNotify(e, SELF_CAM, t);
            APIinput::InputConfigSetState(e, 4, 0x180);
            e.w32(l + 0x158, 1);
        }
        Ok(1)
    }
}

/// 0x00018910 callback 0xb (anim end)
pub fn Camera_BaseCB_ANIM_LF(e: &mut Engine, param_1: u32) -> LResult<u64> {
    APIanim::StopAnim(e, SELF_CAM, param_1);
    APIgameobj::SendMessageTo(e, 0x2a10_0006, NULL, 0x1f, 0, NULL);
    APIgameobj::SendMessageTo(e, SELF_CAM, NULL, 0x58, 0, NULL);
    Ok(1)
}

/// 0x000189b8 callback 0xd (cut scene start/end; param = {code, start flag})
pub fn Camera_BaseCB_CUT_SCENE(e: &mut Engine, param_1: u32) -> LResult<u64> {
    let l = LB(e);
    if e.r32(param_1 + 4) == 1 {
        e.w32(l + 0x168, 1);
        APIphysic::SetFlags(e, SELF_CAM, 1);
        APIgameobj::SendMessageTo(e, 0x2a00_003a, NULL, 0x5e, 1, NULL);
        SendAll(e, 0x4c, 0);
        Camera_BaseFUNC_SetCutSceneMode(e, 7)?;
    } else {
        e.w32(l + 0x168, 0);
        APIgameobj::SendMessageTo(e, 0x2a00_003a, NULL, 0x5e, 0, NULL);
        SendAll(e, 0x51, 0);
        if e.r32(l + 0x15c) != 1 {
            APIphysic::ResetFlags(e, SELF_CAM, 1);
            Camera_BaseFUNC_SetCutSceneMode(e, 8)?;
        }
        let c = e.r32(param_1);
        APIgameobj::SendMessageTo(e, SELF_CAM, NULL, 0x58, c, NULL);
        if IsAttractorReleased(e) || (IsAttractorSetted(e) && IsFlagsAreSetted(e, 3) != 0) {
            let pos = e.alloca(0x10);
            APIgameobj::GetPosition(e, SELF_CAM, 0x7fff_ffff, pos);
            // auStack_60 {0x28, 0, ?, &pos, 1.0, 1.0, 80.0}
            let a = e.alloca(0x20);
            e.w32(a, 0x28);
            e.w32(a + 4, 0);
            e.w32(a + 0xc, pos);
            e.w32(a + 0x10, 0x3f80_0000);
            e.w32(a + 0x14, 0x3f80_0000);
            e.w32(a + 0x18, 0x42a0_0000);
            Camera_BaseFUNC_SetCameraAttract(e, a)?;
        }
    }
    Ok(0)
}

/// 0x00018c70 Camera_BaseCB_INPUT_SWITCH
pub fn Camera_BaseCB_INPUT_SWITCH(e: &mut Engine, _: u32) -> LResult<u64> {
    let l = LB(e);
    e.w32(l + 8, 2);
    Camera_BaseFUNC_MessageHandler(e, 0x54, 0, l + 8)?;
    APIgameobj::SetNextState(e, SELF_CAM, 2);
    Ok(0)
}

/// 0x00018d10 Camera_BaseCB_INPUT_SELECT
pub fn Camera_BaseCB_INPUT_SELECT(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

/// 0x00018d40 Camera_BaseCB_INPUT_START
pub fn Camera_BaseCB_INPUT_START(e: &mut Engine, _: u32) -> LResult<u64> {
    if e.r32(LB(e) + 0x158) != 0 {
        APIgame::SkipCutScene(e);
        Camera_BaseFUNC_SetCutSceneMode(e, 8)?;
    }
    Ok(0)
}

/// 0x00018dc0 Camera_BaseCB_INPUT_STICK
pub fn Camera_BaseCB_INPUT_STICK(e: &mut Engine, _: u32) -> LResult<u64> {
    Camera_BaseFUNC_ResetMode(e, 0)?;
    Ok(1)
}

/// 0x00018e18 Camera_BaseCB_INPUT_RSTICK
pub fn Camera_BaseCB_INPUT_RSTICK(e: &mut Engine, _: u32) -> LResult<u64> {
    let l = LB(e);
    let p = e.alloca(0x80);
    let d0 = e.r32(l + 0xd0);
    APIphysic::DMGetParams(e, d0, p);
    e.w32(p + 0x30, 0x3f40_0000); // uStack_80
    let d1 = e.r32(l + 0xd4);
    APIphysic::DMSetParams(e, d1, p);
    if APIinput::InputContextGetState(e, 4, 0x38, 4) != 8 {
        APIinput::InputContextSetState(e, 4, 0x38, 4, 8);
        APIphysic::DMDoAction(e, d1, 1, NULL);
    }
    APIgameobj::SetContextFlag(e, SELF_CAM, 0x12);
    Ok(0)
}

/// 0x00018f38 Camera_BaseCB_INPUT_RSTICK_IDLE
pub fn Camera_BaseCB_INPUT_RSTICK_IDLE(e: &mut Engine, _: u32) -> LResult<u64> {
    APIinput::InputContextSetState(e, 4, 0x38, 4, 0);
    APIgameobj::SetContextFlag(e, SELF_CAM, 0x11);
    Ok(0)
}

/// shared body of the X/SQUARE/CIRCLE/TRIANGLE callbacks: combo bit by press index (+0x14c)
pub(super) fn InputCombo(e: &mut Engine, bits: [u32; 4]) {
    let l = LB(e);
    let i = e.r32(l + 0x14c);
    if i < 4 {
        let v = e.r32(l + 0x150) | bits[i as usize];
        e.w32(l + 0x150, v);
    }
    let v = e.r32(l + 0x14c).wrapping_add(1);
    e.w32(l + 0x14c, v);
}

/// 0x00018fb0 Camera_BaseCB_INPUT_X
pub fn Camera_BaseCB_INPUT_X(e: &mut Engine, _: u32) -> LResult<u64> {
    let l = LB(e);
    if e.r32(l + 0x158) != 0 {
        e.w32(l + 0x158, 0);
        APIgame::SkipCutScene(e);
    }
    InputCombo(e, [1, 2, 4, 8]);
    Ok(0)
}

/// 0x000190f8 Camera_BaseCB_INPUT_SQUARE
pub fn Camera_BaseCB_INPUT_SQUARE(e: &mut Engine, _: u32) -> LResult<u64> {
    InputCombo(e, [0x10, 0x20, 0x40, 0x80]);
    Ok(0)
}

//! camera_base.c: FUNC_* helpers (WP-L1).
#![allow(non_snake_case, unused_variables)]
use super::base_cb::SendAll;
use super::*;

/// 0x00019200 Camera_BaseCB_INPUT_CIRCLE
pub fn Camera_BaseCB_INPUT_CIRCLE(e: &mut Engine, _: u32) -> LResult<u64> {
    super::base_cb::InputCombo(e, [0x100, 0x200, 0x400, 0x800]);
    Ok(0)
}

/// 0x00019308 Camera_BaseCB_INPUT_TRIANGLE
pub fn Camera_BaseCB_INPUT_TRIANGLE(e: &mut Engine, _: u32) -> LResult<u64> {
    super::base_cb::InputCombo(e, [0x1000, 0x2000, 0x4000, 0x8000]);
    Ok(0)
}

/// 0x0001a440 helper: SetCameraTarget(range target, follow target)
pub fn Camera_BaseFUNC_SetCameraTarget(e: &mut Engine, param_1: u32, param_2: u32) {
    let l = LB(e);
    let p = e.alloca(0x80); // aiStack_b0
    let d = e.r32(l + 0xd0);
    APIphysic::DMGetParams(e, d, p);
    if e.r32(p + 8) != 5 {
        e.w32(p, param_2);
        if param_2 == 0 {
            let t = e.r32(GB0(e) + 8);
            e.w32(p, t);
        }
    }
    if param_1 != 0 {
        e.w32(p + 0x58, param_1); // iStack_58
    }
    APIphysic::DMSetParams(e, d, p);
}

/// 0x0001a510 cut-scene letterbox bars
pub fn Camera_BaseFUNC_SetWindows(e: &mut Engine, param_1: i32) -> u64 {
    let l = LB(e);
    if param_1 == 1 {
        for (off, name, y) in [(0x130, "uiPanelCutUp", -0.375f32), (0x134, "uiPanelCutDown", 1.375)] {
            let _ = y;
            let w = APIhud::WindowAdd(e, 1, 0xffff_ffff, 0x804, 0x35);
            e.w32(l + off, w);
            let n = e.cstr(name);
            APIhud::WindowSetName(e, w, n);
        }
        let (w0, w1) = (e.r32(l + 0x130), e.r32(l + 0x134));
        APIhud::WindowSetPos(e, w0, 0.0, -0.375);
        APIhud::WindowSetPos(e, w1, 0.0, 1.375);
        APIhud::WindowSetDims(e, w0, 1.5, 0.375);
        APIhud::WindowSetDims(e, w1, 1.5, 0.375);
        APIhud::WindowSetBackGround(e, w0, 1, 0.0, 0.0);
        APIhud::WindowSetBackGround(e, w1, 1, 0.0, 0.0);
        let v = e.r32(l + 0x4c) | 0x1000_0000;
        e.w32(l + 0x4c, v);
    } else {
        for off in [0x130, 0x134] {
            if e.r32(l + off) != 0 {
                let w = e.r32(l + off);
                APIhud::WindowDelete(e, w);
                e.w32(l + off, 0);
            }
        }
        let v = e.r32(l + 0x4c) & 0xefff_ffff;
        e.w32(l + 0x4c, v);
    }
    0
}

/// 0x0001a7b8 Camera_BaseFUNC_FadeInOut
pub fn Camera_BaseFUNC_FadeInOut(e: &mut Engine, param_1: f32, param_2: i32) {
    let l = LB(e);
    let w = e.r32(l + 0x138);
    APIhud::WindowResetFlags(e, w, 0x100);
    let t = e.r32(l + 0x118);
    if param_2 == 0 {
        if e.r32(l + 0xe8) != 5 {
            e.w32(l + 0xe8, 1);
        }
        APIhud::ColorFadeStart(e, 0, 0x8000_000b, 1, 0, param_1, 0xff);
        SendAll(e, 0x4d, 0);
        let g = GB0(e);
        e.w32(g + 0x7c, 1);
        if param_1 == 0.0 {
            APIextra::TimerReset(e, t);
            APIextra::TimerSetFrequency(e, t, 0.33);
            APIextra::TimerRegisterNotify(e, SELF_CAM, t, 6);
            return;
        }
    } else {
        e.w32(l + 0xe8, 3);
        let g = GB0(e);
        e.w32(g + 0x7c, 1);
        let c = e.r32(l + 200);
        APIcamera::SetRenderFlag(e, c, 0xffff_ffef);
        APIhud::ColorFadeStart(e, 0, 0x8000_000b, 0, 1, param_1 * 1.7, 0xff);
        SendAll(e, 0x52, 0);
        APIcamera::SetClearColor(e, c, l + 0xb8);
        if e.r32(l + 0x4c) & 0x1000_0000 == 0 {
            let m = e.r32(l + 0x110);
            let _ = Camera_BaseFUNC_Update(e, m, 0);
        }
        Camera_BaseFUNC_SetClipPlane(e);
    }
    APIextra::TimerReset(e, t);
    if e.r32(l + 0xe8) == 3 {
        APIextra::TimerSetFrequency(e, t, 0.45);
    } else {
        APIextra::TimerSetFrequency(e, t, param_1);
    }
    APIextra::TimerRegisterNotify(e, SELF_CAM, t, 6);
}

/// 0x0001ab28 CameraFlex (DM 0x12) default parameters on the leader
pub fn Camera_BaseFUNC_InitCamera(e: &mut Engine) {
    let l = LB(e);
    let p = e.alloca(0x80); // &uStack_b0
    let d = e.r32(l + 0xd0);
    APIphysic::DMGetParams(e, d, p);
    let gdb = GDB(e);
    let g0 = e.r32(gdb);
    let lead = e.r32(g0 + 8);
    let w = |e: &mut Engine, o: u32, v: u32| e.w32(p + o, v);
    w(e, 0x00, lead);
    w(e, 0x04, 0x2a00_001f);
    w(e, 0x08, 2);
    w(e, 0x0c, 0x40a0_0000);
    w(e, 0x10, 0);
    w(e, 0x14, 0x3f80_0000);
    w(e, 0x18, 0);
    w(e, 0x1c, 0);
    w(e, 0x20, 0x3fc0_0000);
    w(e, 0x24, 0);
    w(e, 0x28, 0x3f00_0000);
    w(e, 0x2c, 0x3f00_0000);
    w(e, 0x30, 0x3f00_0000);
    w(e, 0x3c, 0x459c_4000);
    w(e, 0x40, 0);
    w(e, 0x44, 0);
    w(e, 0x48, 0x3f4c_cccd);
    w(e, 0x4c, 0x3f4c_cccd);
    w(e, 0x50, 0x3f4c_cccd);
    w(e, 0x6c, 0x3e56_7750);
    w(e, 0x68, 0x3f80_0000);
    w(e, 0x74, 0);
    w(e, 0x54, 0x2d);
    let t = e.r32(g0 + 0x10);
    w(e, 0x58, t);
    let gb3 = e.r32(gdb + 8);
    let a = if e.r32(gb3 + 0x14) == 1 { 0xbfbd_e44e } else { 0x3fbd_e44e };
    w(e, 0x5c, a);
    w(e, 0x60, a);
    w(e, 0x64, 0x3f9c_61aa);
    APIphysic::DMSetParams(e, d, p);
}

/// 0x0001ad18 Camera_BaseFUNC_ResetMode
pub fn Camera_BaseFUNC_ResetMode(e: &mut Engine, param_1: i32) -> LResult<()> {
    let l = LB(e);
    e.w32(l + 0xf0, 0);
    e.w32(l + 0xf4, 0);
    if e.r32(l + 0xe8) == 1 {
        Camera_BaseFUNC_FadeInOut(e, 1.0, 1);
    }
    let m = e.r32(l + 0xec);
    if m != 3 && m != 0 {
        e.w32(l + 0xec, 3);
    }
    if e.r32(l + 0x160) == 1 {
        e.w32(l + 0x160, 0);
        APIgameobj::SetTranslation(e, SELF_CAM, l + 0xa0);
        LIBApiCFunctions::SetLookAt(e, 0.0, SELF_CAM, l + 0xac);
    }
    let c = e.r32(l + 200);
    let (a, b) = (e.rf(l), e.rf(l + 4));
    APIcamera::SetViewWindow(e, c, a, b);
    if param_1 != 8 {
        e.w32(l + 0x144, 0xffff_ffff);
        let a = G(e, i32ActualFloorEvent);
        e.w32(a, 0xffff_ffff);
        let v = e.r32(l + 0x108);
        e.w32(l + 0x10c, v);
        let v = e.r32(l + 0x100);
        e.w32(l + 0x104, v);
        ReleaseAttractor(e);
        let m = e.r32(l + 0x110);
        Camera_BaseFUNC_Update(e, m, 0)?;
        Camera_BaseFUNC_SetClipPlane(e);
    }
    Ok(())
}

/// 0x0001aee8 floor attribute under the leader -> floor event + clip planes
pub fn Camera_BaseFUNC_GetCameraMode(e: &mut Engine) -> LResult<u32> {
    let l = LB(e);
    let rin = e.alloca(0x30); // DStack_a0 .. uStack_78
    let rout = e.alloca(0x40); // aRStack_70
    let lead = e.r32(GB0(e) + 8);
    APIgameobj::GetPosition(e, lead, 0x7fff_ffff, rin);
    let y = e.rf(rin + 4) + 0.5;
    e.wf(rin + 4, y);
    e.w32(rin + 0xc, 0);
    e.w32(rin + 0x10, 0xbf80_0000);
    e.w32(rin + 0x14, 0);
    e.w32(rin + 0x18, 0);
    e.w32(rin + 0x1c, 0);
    e.w32(rin + 0x20, 9);
    e.w32(rin + 0x24, 0);
    e.w32(rin + 0x28, 0);
    let fa = G(e, pFloorAttribs);
    if APIphysic::CheckRayIntersection(e, rin, rout) == 2 {
        for (dst, src) in [(0, 0x1c), (0x10, 0x2c), (0x18, 0x34), (8, 0x24)] {
            let v = e.r64(rout + src);
            e.w64(fa + dst, v);
        }
        let v = e.r32(fa + 8);
        let g = GB0(e);
        e.w32(g + 100, v);
    }
    let ev = e.r32(GB0(e) + 100) >> 0x18;
    FloorEventManager(e, ev as i32)?;
    let cur = e.r32(fa + 8);
    if cur != e.r32(l + 0x144) {
        e.w32(l + 0x144, cur);
        Camera_BaseFUNC_SetClipPlane(e);
        CameraFUNC_SetLocalParams(e);
    }
    Ok(e.r32(fa + 8))
}

/// 0x0001b0c0 Camera_BaseFUNC_SetClipPlane
pub fn Camera_BaseFUNC_SetClipPlane(e: &mut Engine) {
    let c = e.r32(LB(e) + 200);
    // l04a: rooms 1..7
    let (n, f) = match e.r32(GB0(e) + 0x24) {
        1 => (0.05, 50.0),
        2 | 5 => (0.045, 45.0),
        3 | 4 | 6 => (0.04, 40.0),
        7 => (0.035, 35.0),
        _ => return,
    };
    APIcamera::SetNearClipPlane(e, c, n);
    APIcamera::SetFarClipPlane(e, c, f);
}

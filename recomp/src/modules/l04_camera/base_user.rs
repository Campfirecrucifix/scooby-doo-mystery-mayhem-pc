//! camera_base.c: SetCutSceneMode / SetCameraBattle / Death / Attract / SetCameraUser / Update (WP-L1).
#![allow(non_snake_case, unused_variables)]
use super::*;

/// 0x00019410 Camera_BaseCB_INPUT_R3
pub fn Camera_BaseCB_INPUT_R3(e: &mut Engine, _: u32) -> LResult<u64> {
    let n = G(e, uiNbPics);
    let f = G(e, szFileName);
    if e.r32(n) < 9 {
        let c = e.r8(f + 0x11).wrapping_add(1);
        e.w8(f + 0x11, c);
        let v = e.r32(n) + 1;
        e.w32(n, v);
    } else {
        e.w32(n, 0);
        let c = e.r8(f + 0x10).wrapping_add(1);
        e.w8(f + 0x10, c);
        e.w8(f + 0x11, b'0');
    }
    let c = e.r32(LB(e) + 200);
    APIcamera::TakeSnapShot(e, c, f);
    Ok(0)
}

/// 0x0001b170 Camera_BaseFUNC_ShowText
pub fn Camera_BaseFUNC_ShowText(e: &mut Engine, param_1: u32) -> u32 {
    let l = LB(e);
    if e.ri32(l + 0x13c) == -1 {
        let w = APIhud::WindowAdd(e, 1, 0xffff_ffff, 0x30004, 0x33);
        e.w32(l + 0x13c, w);
        APIhud::WindowSetPos(e, w, 0.0, 0.8);
        APIhud::WindowSetDims(e, w, 0.8, 0.2);
        APIhud::TextWindowSetColor(e, w, 6);
        APIhud::TextWindowSetFont(e, w, 0);
        let n = e.cstr("Camera_BaseFUNC_ShowText");
        APIhud::WindowSetName(e, w, n);
    }
    let w = e.r32(l + 0x13c);
    APIhud::TextWindowSetText(e, w, param_1, &[0]); // varargs: only the format is passed
    let t = e.r32(l + 0x124);
    APIextra::TimerReset(e, t);
    APIextra::TimerSetFrequency(e, t, 3.0);
    APIextra::TimerRegisterNotify(e, SELF_CAM, t, 6)
}

/// 0x0001b340 Camera_BaseFUNC_SetCutSceneMode
pub fn Camera_BaseFUNC_SetCutSceneMode(e: &mut Engine, param_1: i32) -> LResult<()> {
    let l = LB(e);
    if param_1 == 7 {
        e.w32(l + 8, 99);
        e.w32(l + 0x44, 1);
        e.w32(l + 0x48, 0);
        e.w32(l + 0x4c, 0x1000_0000);
        Camera_BaseFUNC_SetCameraUser(e, l + 8)?;
        APIgameobj::SendMessageTo(e, 0x2a10_0000, NULL, 0x1e, 0, NULL);
        let s = APIinput::InputConfigGetState(e, 4);
        e.w32(l + 0x148, s);
        let t = e.r32(l + 300);
        APIextra::TimerReset(e, t);
        APIextra::TimerSetFrequency(e, t, 1.0);
        APIextra::TimerRegisterNotify(e, SELF_CAM, t, 6);
    } else {
        let t = e.r32(l + 300);
        APIextra::TimerUnregisterNotify(e, SELF_CAM, t);
        Camera_BaseFUNC_ResetMode(e, 0)?;
        APIgameobj::SendMessageTo(e, 0x2a10_0000, NULL, 0x1f, 0, NULL);
        let s = e.r32(l + 0x148);
        APIinput::InputConfigSetState(e, 4, s as i32);
        e.w32(l + 0x158, 0);
    }
    Ok(())
}

/// 0x0001b538 battle camera: behind the leader on the leader->enemy line (param = LB+0x54 battle params)
pub fn Camera_BaseFUNC_SetCameraBattle(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    let l = LB(e);
    let a = e.alloca(0x10);
    let b = e.alloca(0x10);
    let lead = e.r32(GB0(e) + 8);
    APIgameobj::GetPosition(e, lead, 0x7fff_ffff, a);
    let en = e.r32(param_1 + 0x24);
    APIgameobj::GetPosition(e, en, 0x7fff_ffff, b);
    e.w32(l + 0x54, 5);
    e.w32(l + 0x78, en);
    let v = e.r32(param_1 + 0x30);
    e.w32(l + 0x84, v);
    let (p, q) = (e.rv3(a), e.rv3(b));
    let d = [p[0] - q[0], p[1] - q[1], p[2] - q[2]];
    let n2 = d[0] * d[0] + d[1] * d[1] + d[2] * d[2];
    let f3c = APImath::sqrtf(e, n2);
    let f38 = 1.0 / APImath::sqrtf(e, n2);
    let k = f3c + e.rf(param_1 + 0x30);
    let x = d[0] * f38 * k + q[0];
    let z = d[2] * f38 * k + q[2];
    let y = d[1] * f38 * k + q[1] + e.rf(l + 0x88);
    e.wv3(l + 0x58, [x, y, z]);
    let ty = q[1] + e.rf(l + 0x8c);
    e.wv3(l + 100, [q[0], ty, q[2]]);
    let f = e.r32(param_1 + 0x44) | 0x2000_0081;
    e.w32(l + 0x98, f);
    Camera_BaseFUNC_SetCameraUser(e, l + 0x54)
}

/// 0x0001b840 Camera_BaseFUNC_SetCameraDeath
pub fn Camera_BaseFUNC_SetCameraDeath(e: &mut Engine) -> LResult<()> {
    let l = LB(e);
    let v = e.alloca(0x10);
    e.wv3(v, [0.0, 1.8, -7.5]);
    APIgameobj::GetPosition(e, 0x2a00_0036, 0x7fff_ffff, l + 0x18);
    let y = e.rf(l + 0x1c) + 2.0;
    e.wf(l + 0x1c, y);
    APIgameobj::TransformVector(e, 0x2a00_0036, v, v);
    let (d, t) = (e.rv3(v), e.rv3(l + 0x18));
    e.wv3(l + 0xc, [d[0] + t[0], d[1] + t[1], d[2] + t[2]]);
    let c = e.r32(l + 200);
    APIcamera::SetNearClipPlane(e, c, 0.012);
    APIcamera::SetFarClipPlane(e, c, 12.0);
    e.w32(l + 8, 5);
    e.w32(l + 0x44, 1);
    e.w32(l + 0x48, 0);
    e.w32(l + 0x4c, 0x2181);
    Camera_BaseFUNC_SetCameraUser(e, l + 8)
}

/// 0x0001b9f8 Camera_BaseFUNC_SetCameraAttract
pub fn Camera_BaseFUNC_SetCameraAttract(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    let r = |i: u32| e.r32(param_1 + i * 4);
    let (a4, a5, a6, a0, a1, a2, a3) = (r(4), r(5), r(6), r(0), r(1), r(2), r(3));
    SetAttractorParam(e, f32::from_bits(a4), f32::from_bits(a5), f32::from_bits(a6), a0, a1 as i32, a2, a3);
    let l = LB(e);
    e.w32(l + 0x110, 2);
    Camera_BaseFUNC_Update(e, 2, 0)
}

/// 0x0001bab0 Camera user parameters (param = moCameraParams, 0x50 bytes: [0] mode, [1..3] pos, [4..6] target,
/// [7] pos object, [8] its bone, [9] target object, [10] its bone, [0xb] timer, [0xf] window mode,
/// [0x10] fade, [0x11] flags, [0x12] look-at object) -> CameraFlex DM params (+0xd0 / +0xd8).
pub fn Camera_BaseFUNC_SetCameraUser(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    let l = LB(e);
    let P = |e: &Engine, i: u32| e.r32(param_1 + i * 4);
    let fl = |e: &Engine| e.r32(param_1 + 0x44);
    if fl(e) & 0x8000_0000 == 0 {
        e.w32(l + 0xf0, 1);
    } else {
        let m = P(e, 0);
        e.w32(l + 0x110, m);
    }
    if P(e, 0) == 2 {
        let (m, f) = (P(e, 0), fl(e));
        Camera_BaseFUNC_Update(e, m, f)?;
    }
    if fl(e) & 0x4000_0000 != 0 {
        Camera_BaseFUNC_InitCamera(e);
    }
    let d = e.alloca(0x80); // auStack_d0
    let dm = e.r32(l + 0xd0);
    APIphysic::DMGetParams(e, dm, d);
    if P(e, 0) != 99 {
        let m = P(e, 0);
        e.w32(d + 8, m); // uStack_c8
    }
    let f = fl(e);
    let sp = if f & 0x2000 != 0 {
        Some(0)
    } else if f & 0x4000 != 0 {
        Some(0x3e4c_cccd)
    } else if f & 0x8000 != 0 {
        Some(0x3ecc_cccd)
    } else if f & 0x10000 != 0 {
        Some(0x3f40_0000)
    } else if f & 0x20000 != 0 {
        Some(0x3f80_0000)
    } else {
        None
    };
    if let Some(v) = sp {
        e.w32(d + 0x28, v);
        e.w32(d + 0x2c, v);
    }
    let sp = if f & 0x100 != 0 {
        Some(0)
    } else if f & 0x200 != 0 {
        Some(0x3e4c_cccd)
    } else if f & 0x400 != 0 {
        Some(0x3ecc_cccd)
    } else if f & 0x800 != 0 {
        Some(0x3f40_0000)
    } else if f & 0x1000 != 0 {
        Some(0x3f80_0000)
    } else {
        None
    };
    if let Some(v) = sp {
        for o in [0x48, 0x4c, 0x50, 0x30] {
            e.w32(d + o, v);
        }
    }
    let t = if f & 8 == 0 { e.r32(GB0(e) + 8) } else { P(e, 9) };
    e.w32(d, t);
    let cpy = |e: &mut Engine, dst: Ptr, i: u32| {
        for k in 0..3 {
            let v = P(e, i + k);
            e.w32(dst + k * 4, v);
        }
    };
    let (c0, b4) = (d + 0x10, d + 0x1c); // DStack_c0 (position), DStack_b4 (target)
    if f & 0x10 != 0 {
        cpy(e, b4, 4);
    }
    let m = P(e, 0);
    if m == 4 || m == 5 {
        if f & 1 == 0 {
            if f & 2 == 0 {
                APIgameobj::GetPosition(e, SELF_CAM, 0x7fff_ffff, c0);
            } else {
                let t40 = e.alloca(0x10);
                let o = P(e, 7);
                let bone = if f & 4 == 0 { 0x7fff_ffff } else { P(e, 8) };
                APIgameobj::GetPosition(e, o, bone, t40);
                e.memcpy(c0, t40, 12);
            }
        } else {
            cpy(e, c0, 1);
        }
        if m == 5 {
            if f & 0x20 == 0 {
                if f & 0x80 != 0 {
                    cpy(e, b4, 4);
                }
            } else {
                let o = P(e, 9);
                let bone = if f & 0x40 == 0 { 0x7fff_ffff } else { P(e, 10) };
                APIgameobj::GetPosition(e, o, bone, b4);
            }
        }
    }
    if m == 3 || m == 1 {
        cpy(e, c0, 1);
        cpy(e, b4, 4);
    }
    if m == 0 {
        let v = if f & 0x400_0000 == 0 { 0 } else { P(e, 7) };
        e.w32(l + 0xf8, v);
    }
    let t11c = e.r32(l + 0x11c);
    if f & 0x20_0000 != 0 {
        APIextra::TimerReset(e, t11c);
        APIextra::TimerSetFrequency(e, t11c, 0.1);
        APIextra::TimerRegisterNotify(e, SELF_CAM, t11c, 6);
    } else if f & 0x40_0000 != 0 {
        APIextra::TimerReset(e, t11c);
        // quirk: Ghidra `(float)param_1[0xb]`; the field holds a float (bit copy)
        let fr = f32::from_bits(P(e, 0xb));
        APIextra::TimerSetFrequency(e, t11c, fr);
        APIextra::TimerRegisterNotify(e, SELF_CAM, t11c, 6);
    }
    if f & 0x80_0000 != 0 {
        e.w32(l + 0x160, 1);
        APIgameobj::GetPosition(e, SELF_CAM, 0x7fff_ffff, l + 0xa0);
        APIgameobj::GetLookAt(e, SELF_CAM, 0x7fff_ffff, l + 0xac);
    } else if f & 0x100_0000 != 0 {
        e.w32(l + 0x160, 1);
        let o = P(e, 0x12);
        APIgameobj::GetPosition(e, o, 0x7fff_ffff, l + 0xa0);
        APIgameobj::GetLookAt(e, o, 0x7fff_ffff, l + 0xac);
    } else if f & 0x200_0000 != 0 {
        let o = P(e, 9);
        LIBApiCFunctions::TakeRotation(e, 0.0, SELF_CAM, o as i32);
    }
    match P(e, 0x10) {
        1 => Camera_BaseFUNC_FadeInOut(e, 1.0, 0),
        2 => Camera_BaseFUNC_FadeInOut(e, 0.0, 0),
        3 => Camera_BaseFUNC_FadeInOut(e, 1.0, 1),
        4 => Camera_BaseFUNC_FadeInOut(e, 0.0, 1),
        5 => {
            e.w32(l + 0xe8, 5);
            Camera_BaseFUNC_FadeInOut(e, 1.0, 0);
        }
        _ => {}
    }
    let wm = P(e, 0xf);
    if wm != 0 && wm < 9 {
        let v = fl(e) | 0x2000_0000;
        e.w32(param_1 + 0x44, v);
        e.w32(l + 0xec, wm);
    }
    if fl(e) & 0x2000_0000 == 0 {
        APIinput::InputContextSetState(e, 4, 0x38, 3, 4);
        let v = e.r32(GB0(e) + 0x10);
        e.w32(d + 0x58, v); // uStack_78
    } else {
        APIinput::InputContextSetState(e, 4, 0x38, 3, 0);
        e.w32(d + 0x58, 0);
    }
    let m = P(e, 0);
    if m == 0x62 {
        APIphysic::SetVelocity(e, SELF_CAM, 0.0, 0.0, 0.0);
        e.w32(l + 0x10c, 0);
        e.w32(l + 0x104, 2);
    } else if fl(e) & 0x8000_0000 == 0 {
        if m == 99 && fl(e) & 0x1000_0000 == 0 {
            APIphysic::SetVelocity(e, SELF_CAM, 0.0, 0.0, 0.0);
            e.w32(l + 0x10c, 0);
            e.w32(l + 0x104, 2);
        } else {
            let d8 = e.r32(l + 0xd8);
            APIphysic::DMSetParams(e, d8, d);
            let c = APIgameobj::GetContextFlag(e, SELF_CAM);
            e.w32(l + 0x108, c as u32);
            let s = APIgameobj::GetCurrentStateID(e, SELF_CAM);
            e.w32(l + 0x100, s as u32);
            e.w32(l + 0x10c, 0x14);
            e.w32(l + 0x104, 3);
        }
    } else {
        APIphysic::DMSetParams(e, dm, d);
        e.w32(l + 0x10c, 0x11);
        e.w32(l + 0x108, 0x11);
        e.w32(l + 0x104, 3);
        e.w32(l + 0x100, 3);
    }
    let (m, f) = (P(e, 0), fl(e));
    Camera_BaseFUNC_Update(e, m, f)
}

/// 0x0001c788 physic flags / sleep / next state for a camera mode
pub fn Camera_BaseFUNC_Update(e: &mut Engine, param_1: u32, param_2: u32) -> LResult<()> {
    let l = LB(e);
    match param_1 {
        1 | 3 => {
            APIphysic::SetFlags(e, SELF_CAM, 6);
            APIphysic::ResetFlags(e, SELF_CAM, 1);
            APIgameobj::Sleep(e, SELF_CAM, 0.0);
        }
        2 => {
            APIphysic::SetFlags(e, SELF_CAM, 6);
            APIphysic::ResetFlags(e, SELF_CAM, 1);
            APIgameobj::Sleep(e, SELF_CAM, 0.0);
            APIgameobj::SetContextFlag(e, SELF_CAM, 8);
            APIgameobj::SetNextState(e, SELF_CAM, 2);
            return Ok(());
        }
        0 => {
            APIphysic::ResetFlags(e, SELF_CAM, 5);
            APIphysic::SetFlags(e, SELF_CAM, 2);
            APIgameobj::Sleep(e, SELF_CAM, 0.0);
            let (d0, d8) = (e.r32(l + 0xd0), e.r32(l + 0xd8));
            if param_2 & 0x400_0000 == 0 {
                APIphysic::DMDoAction(e, d0, 1, NULL);
                APIphysic::DMDoAction(e, d8, 1, NULL);
            } else {
                APIphysic::DMDoAction(e, d0, 2, l + 0xf8);
                APIphysic::DMDoAction(e, d8, 2, l + 0xf8);
            }
        }
        4 | 5 => {
            APIphysic::ResetFlags(e, SELF_CAM, 5);
            APIphysic::SetFlags(e, SELF_CAM, 2);
            APIgameobj::Sleep(e, SELF_CAM, 0.0);
        }
        0x62 => {
            APIphysic::SetFlags(e, SELF_CAM, 1);
            APIgameobj::Sleep(e, SELF_CAM, -1.0);
        }
        99 => return Ok(()),
        _ => {}
    }
    let c = e.r32(l + 0x10c);
    APIgameobj::SetContextFlag(e, SELF_CAM, c);
    let s = e.r32(l + 0x104);
    APIgameobj::SetNextState(e, SELF_CAM, s);
    Ok(())
}

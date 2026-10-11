//! camera__1ac1b1b0: ManageEvent* (FloorEvent/BaseCam*.c) and camera.c
#![allow(non_snake_case, unused_variables, unused_imports, dead_code)]
use super::*;
use crate::engine::*;
use crate::logic::{ExtFn, LResult};

/// ManageEventDefault (0x00017f38)
pub fn ManageEventDefault(e: &mut Engine, param_1: i32) -> LResult<()> {
    if param_1 == 1 {
        { let a = sh(e, 0x103d0); e.w32(a, 2); }
        { let a = sh(e, 0x103f4); e.w32(a, lead(e)); }
        { let a = sh(e, 0x1040c); e.w32(a, 0); }
        { let a = sh(e, 0x10410); e.w32(a, 0); }
        { let a = sh(e, 0x10414); e.w32(a, 0xc0000000); }
        { let t = sh(e, 0x103d0); Camera_BaseFUNC_SetCameraUser(e, t)?; }
    }
    Ok(())
}

/// Events[] slot dispatch (0 = ManageEventDefault)
pub fn ME(e: &mut Engine, n: u8, s: i32) -> LResult<()> {
    match n {
        0 => ManageEventDefault(e, s),
        _ => Ok(()),
    }
}

/// GetColor: (0.11, 0.29, 0.25, 0)
fn GetColor(e: &mut Engine, p: Ptr) {
    ws(e, p, &[(0, 0x3de1_47ae), (4, 0x3e94_7ae1), (8, 0x3e80_0000), (0xc, 0)]);
}
/// GetColor255: GetColor() * 255
fn GetColor255(e: &mut Engine, p: Ptr) {
    GetColor(e, p);
    for i in 0..4 {
        let v = e.rf(p + i * 4) * 255.0;
        e.wf(p + i * 4, v);
    }
}
/// CameraST_FIRSTMOVE: linear fog in GetColor255
pub fn first(e: &mut Engine) -> LResult<u64> {
    EpN_CameraST_FIRSTMOVE(e)?;
    let c = e.alloca(0x10);
    GetColor255(e, c);
    fog(e, 0.1, Some(5.0));
    enable_fog(e, 1);
    let k = cam(e);
    APIcamera::SetFogColor(e, k, c);
    Ok(0)
}
/// 0x52 / 0x98 helper: colour (GetColor in the fog rooms, else black) sent with 0x5b to `to`
fn send_col(e: &mut Engine, fogroom: bool, to: u32) {
    let c = e.alloca(0x10);
    if fogroom {
        GetColor(e, c);
    } else {
        ws(e, c, &[(0, 0), (4, 0), (8, 0), (0xc, 0)]);
    }
    send(e, to, 0x5b, 0, c);
}
/// CameraFUNC_MessageHandler (0x0001ff78): fog in rooms 2 / 8
pub fn msg(e: &mut Engine, p1: u32, p2: u32, p3: Ptr) -> LResult<u64> {
    let fr = |e: &Engine| room(e) == 2 || room(e) == 8;
    if p1 == 0x98 {
        let r = fr(e);
        enable_fog(e, r as i32);
        send(e, 0x2a00_0047, 0xde, 0, NULL);
    } else if p1 == 0x52 && p2 == 0 {
        let r = fr(e);
        send_col(e, r, 0x2a00_0006);
    }
    std_msg(e, p1, p2, p3)
}
/// cameraConstructor (0x00020350)
pub fn ctor(e: &mut Engine, api: Ptr, n: i32) -> LResult<()> {
    std_ctor(e, api, n, 0x18c, true)
}

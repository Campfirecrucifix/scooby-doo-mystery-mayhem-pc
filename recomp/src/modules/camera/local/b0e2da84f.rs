//! camera__0e2da84f: ManageEvent* (FloorEvent/BaseCam*.c) and camera.c
#![allow(non_snake_case, unused_variables, unused_imports, dead_code)]
use super::*;
use crate::engine::*;
use crate::logic::{ExtFn, LResult};

/// ManageEventDefault (0x00017670)
pub fn ManageEventDefault(e: &mut Engine, param_1: i32) -> LResult<()> {
    if param_1 == 1 {
        { let a = sh(e, 0x103d8); e.w32(a, 2); }
        { let a = sh(e, 0x103fc); e.w32(a, lead(e)); }
        { let a = sh(e, 0x10414); e.w32(a, 0); }
        { let a = sh(e, 0x10418); e.w32(a, 0); }
        { let a = sh(e, 0x1041c); e.w32(a, 0xc0000000); }
        { let t = sh(e, 0x103d8); Camera_BaseFUNC_SetCameraUser(e, t)?; }
    }
    Ok(())
}

/// ManageEvent002 (0x000178b0)
pub fn ManageEvent002(e: &mut Engine, param_1: i32) -> LResult<()> {
    let fr = e.alloca(0x20);
    match param_1 {
        0 => {
            { let p = sh(e, 0x10438); APIgameobj::GetPosition(e, 0x2a00003e, 0x7fff_ffff, p); }
            APIgameobj::KillObject(e, 0x2a00003e);
        }
        1 => {
            { let a = fr + 0xc; e.w32(a, sh(e, 0x10438)); }
            { let a = fr + 0; e.w32(a, 0); }
            { let a = fr + 0x10; e.w32(a, 0x3fc00000); }
            { let a = fr + 0x14; e.w32(a, 0x3f800000); }
            { let a = fr + 0x18; e.w32(a, 0x41a00000); }
            Camera_BaseFUNC_SetCameraAttract(e, fr)?;
        }
        2 => {
        }
        3 => {
            ReleaseAttractor(e);
        }
        4 => {
        }
        _ => {}
    }
    Ok(())
}

/// 0e2da84f ManageEvent001 (0x00017758) / ManageEvent003 (0x000179e8): attractor on the killed
/// marker `id` unless GB2+8 & `bit`
fn me_attr_gb2(e: &mut Engine, param_1: i32, id: u32, pos: u32, bit: u32) -> LResult<()> {
    let fr = e.alloca(0x20);
    match param_1 {
        0 => {
            let p = sh(e, pos);
            APIgameobj::GetPosition(e, id, 0x7fff_ffff, p);
            APIgameobj::KillObject(e, id);
        }
        1 => {
            if e.r32(e.r32(GDB(e) + 0xc) + 8) & bit == 0 {
                e.w32(fr, 0);
                let p = sh(e, pos);
                e.w32(fr + 0xc, p);
                e.w32(fr + 0x10, 0x3fc0_0000);
                e.w32(fr + 0x14, 0x3f80_0000);
                e.w32(fr + 0x18, 0x4100_0000);
                Camera_BaseFUNC_SetCameraAttract(e, fr)?;
            }
        }
        3 => ReleaseAttractor(e),
        _ => {}
    }
    Ok(())
}
/// ManageEvent001 (0x00017758)
pub fn ManageEvent001(e: &mut Engine, param_1: i32) -> LResult<()> {
    me_attr_gb2(e, param_1, 0x2a00_003d, 0x10428, 0x100_0000)
}
/// ManageEvent003 (0x000179e8)
pub fn ManageEvent003(e: &mut Engine, param_1: i32) -> LResult<()> {
    me_attr_gb2(e, param_1, 0x2a00_0040, 0x10448, 0x8_0000)
}

/// Events[] slot dispatch (0 = ManageEventDefault)
pub fn ME(e: &mut Engine, n: u8, s: i32) -> LResult<()> {
    match n {
        0 => ManageEventDefault(e, s),
        1 => ManageEvent001(e, s),
        2 => ManageEvent002(e, s),
        3 => ManageEvent003(e, s),
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
/// CameraFUNC_MessageHandler (0x0001ecb0): fog in room 1
pub fn msg(e: &mut Engine, p1: u32, p2: u32, p3: Ptr) -> LResult<u64> {
    if p1 == 0x52 {
        if p2 == 0 {
            let r = room(e) == 1;
            send_col(e, r, 0x2a00_0037);
        }
    } else if p1 == 0x98 {
        let r = room(e) == 1;
        enable_fog(e, r as i32);
        if room(e) == 3 && e.r32(GCi(e, 3) + 0xc) & 0x8000 == 0 {
            send(e, 0x2a10_0006, 0x2a, 4, NULL);
        }
    }
    std_msg(e, p1, p2, p3)
}
/// cameraConstructor (0x0001f050)
pub fn ctor(e: &mut Engine, api: Ptr, n: i32) -> LResult<()> {
    EpN_CameraFUNC_Constructor(e, api, n)?;
    local_data(e, 0x180);
    if n == 1 {
        APIcons::SetExternalFunction(e, 1, FIRST);
        get_graphics(e);
        global_data(e, 0x24);
        global_buf(e, 0, 0, 0x8f8);
        global_buf(e, 8, 2, 0x158);
    }
    APIcons::SetMessageFunction(e, CameraFUNC_MessageHandler);
    APIcons::SetInitialState(e, 1);
    Ok(())
}

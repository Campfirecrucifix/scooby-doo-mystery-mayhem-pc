//! camera__6a8753d8: ManageEvent* (FloorEvent/BaseCam*.c) and camera.c
#![allow(non_snake_case, unused_variables, unused_imports, dead_code)]
use super::*;
use crate::engine::*;
use crate::logic::{ExtFn, LResult};

/// ManageEventDefault (0x00017d48)
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

/// Events[] slot dispatch (0 = ManageEventDefault)
pub fn ME(e: &mut Engine, n: u8, s: i32) -> LResult<()> {
    match n {
        0 => ManageEventDefault(e, s),
        _ => Ok(()),
    }
}

/// CameraST_FIRSTMOVE (0x0001f7d0)
pub fn first(e: &mut Engine) -> LResult<u64> {
    let a = e.alloca(0x10);
    e.w32(a, 0xd);
    EpN_CameraST_FIRSTMOVE(e)?;
    let s = e.alloca(0x30);
    ws(e, s, &[(0, 1), (4, 0), (8, 0x3f4c_cccd), (0xc, 0x4040_0000), (0x10, 0x3f33_3333), (0x14, 0x40),
        (0x18, 0x7fff_ffff), (0x1c, 0x3dcc_cccd), (0x20, 0x4080_0000), (0x24, 0x80), (0x28, 0x3f00_0000)]);
    for o in [0x2a00_00c0, 0x2a00_001b, 0x2a00_0021] {
        APIgameobj::SetShadowParams(e, o, s);
    }
    let l = lead(e);
    send(e, l, 0x7b, 0xe2, a);
    // CameraFUNC_SetFog (0x0001f960)
    set_col(e, 0x17c, [0x41a8_0000, 0x4208_0000, 0x4244_0000, 0]);
    let c = col255(e, 0x17c);
    send(e, 0x2a00_0012, 0x5b, 0, c);
    fog(e, 1.0, Some(30.0));
    fog_color(e, 0x17c);
    enable_fog(e, 1);
    Ok(0)
}
/// CameraFUNC_MessageHandler (0x0001fb78)
pub fn msg(e: &mut Engine, p1: u32, p2: u32, p3: Ptr) -> LResult<u64> {
    let g = GC0(e);
    if p1 == 0x98 {
        enable_fog(e, 1);
        if p2 == 2 {
            and32(e, g + 0x14, 0xffff_ffbf);
        }
    }
    Camera_BaseFUNC_MessageHandler(e, p1, p2, p3)
}
/// cameraConstructor (0x0001fc88)
pub fn ctor(e: &mut Engine, api: Ptr, n: i32) -> LResult<()> {
    EpN_CameraFUNC_Constructor(e, api, n)?;
    local_data(e, 400);
    if n == 1 {
        APIcons::SetExternalFunction(e, 1, FIRST);
        get_graphics(e);
        global_data(e, 0x20);
    }
    APIcons::SetMessageFunction(e, CameraFUNC_MessageHandler);
    let g = GC0(e);
    and32(e, g + 0x34, 0xffff_ffef);
    or32(e, g + 0x34, 1);
    ws(e, g, &[(8, 0x2a00_001b), (0xc, 0x2a00_001a), (0x10, 1), (4, 2)]);
    APIcons::SetInitialState(e, 1);
    Ok(())
}

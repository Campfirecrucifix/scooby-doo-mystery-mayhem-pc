//! camera__2500ac27: ManageEvent* (FloorEvent/BaseCam*.c) and camera.c
#![allow(non_snake_case, unused_variables, unused_imports, dead_code)]
use super::*;
use crate::engine::*;
use crate::logic::{ExtFn, LResult};

/// ManageEventDefault (0x00017620)
pub fn ManageEventDefault(e: &mut Engine, param_1: i32) -> LResult<()> {
    if param_1 == 1 {
        { let a = sh(e, 0x103d0); e.w32(a, 1); }
        { let a = sh(e, 0x103f8); e.w32(a, 0x7fffffff); }
        { let a = sh(e, 0x103f4); e.w32(a, 0x2a000004); }
        { let a = sh(e, 0x1040c); e.w32(a, 0); }
        { let a = sh(e, 0x10410); e.w32(a, 0); }
        { let a = sh(e, 0x103d4); e.w32(a, 0); }
        { let a = sh(e, 0x103d8); e.w32(a, 0x40500000); }
        { let a = sh(e, 0x103dc); e.w32(a, 0xc0800000); }
        { let a = sh(e, 0x103e0); e.w32(a, 0); }
        { let a = sh(e, 0x103e4); e.w32(a, 0x40000000); }
        { let a = sh(e, 0x103e8); e.w32(a, 0x40a00000); }
        { let a = sh(e, 0x10414); e.w32(a, 0x80004208); }
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

/// CameraST_FIRSTMOVE
pub fn first(e: &mut Engine) -> LResult<u64> {
    std_first(e)
}
/// CameraCB_INIT_LOCAL (0x0001e530, callback 0x23 on notify 0x14): Camera_BaseCB_INIT + fog
pub fn CameraCB_INIT_LOCAL(e: &mut Engine, p: u32) -> LResult<u64> {
    Camera_BaseCB_INIT(e, p)?;
    set_col(e, 0x17c, [0x41b8_0000, 0x4170_0000, 0x4100_0000, 0]);
    let c = col255(e, 0x17c);
    Camera_BaseFUNC_MessageHandler(e, 0x5b, 0, c)?;
    fog(e, 5.0, Some(5.0));
    enable_fog(e, 1);
    fog_color(e, 0x17c);
    Camera_BaseFUNC_SetClipPlane(e);
    Ok(1)
}
/// CameraFUNC_MessageHandler (0x0001e7d0)
pub fn msg(e: &mut Engine, p1: u32, p2: u32, p3: Ptr) -> LResult<u64> {
    if p1 == 0x52 {
        let c = col255(e, 0x17c);
        clear_color(e, c);
    } else if p1 == 0xc1 {
        cam_user(e, 0x10420, &[
            (0, 1), (0x28, 0x7fff_ffff), (0x24, 0x2a00_0004), (0x3c, 4), (0x40, 3), (4, 0), (8, 0x4050_0000),
            (0xc, 0xc080_0000), (0x10, 0), (0x14, 0x4000_0000), (0x18, 0x40a0_0000), (0x44, 0x8200_4208),
        ])?;
    }
    Camera_BaseFUNC_MessageHandler(e, p1, p2, p3)
}
/// cameraConstructor (0x0001e9e0)
pub fn ctor(e: &mut Engine, api: Ptr, n: i32) -> LResult<()> {
    EpN_CameraFUNC_Constructor(e, api, n)?;
    if n == 1 {
        APIcons::SetExternalFunction(e, 1, FIRST);
        APIcons::SetExternalFunction(e, 0x23, CameraCB_INIT_LOCAL);
        global_data(e, 0x20);
    }
    local_data(e, 0x18c);
    APIcons::SetMessageFunction(e, CameraFUNC_MessageHandler);
    APIgameobj::RegisterNotify(e, SELF_CAM, 0xffff_ffff, 0x14, 0x23);
    APIcons::SetInitialState(e, 1);
    Ok(())
}

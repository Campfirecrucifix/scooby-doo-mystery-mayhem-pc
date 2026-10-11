//! camera__78a0064b: ManageEvent* (FloorEvent/BaseCam*.c) and camera.c
#![allow(non_snake_case, unused_variables, unused_imports, dead_code)]
use super::*;
use crate::engine::*;
use crate::logic::{ExtFn, LResult};

/// ManageEventDefault (0x00017da8)
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

/// CameraST_FIRSTMOVE (0x0001f980)
pub fn first(e: &mut Engine) -> LResult<u64> {
    EpN_CameraST_FIRSTMOVE(e)?;
    set_col(e, 0x17c, [0x42fa_0000, 0x42c0_0000, 0x4220_0000, 0]);
    fog(e, 1.0, None);
    enable_fog(e, 1);
    fog_color(e, 0x17c);
    Ok(0)
}
/// CameraFUNC_MessageHandler (0x0001fae0)
pub fn msg(e: &mut Engine, p1: u32, p2: u32, p3: Ptr) -> LResult<u64> {
    if p1 == 0x52 && p2 == 0 {
        // CameraFUNC_SetFogColor (0x0001fb88)
        let c = col255(e, 0x17c);
        clear_color(e, c);
    }
    Camera_BaseFUNC_MessageHandler(e, p1, p2, p3)
}
/// cameraConstructor (0x0001fc68)
pub fn ctor(e: &mut Engine, api: Ptr, n: i32) -> LResult<()> {
    std_ctor(e, api, n, 0x18c, false)
}

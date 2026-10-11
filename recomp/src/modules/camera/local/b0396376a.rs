//! camera__0396376a: ManageEvent* (FloorEvent/BaseCam*.c) and camera.c
#![allow(non_snake_case, unused_variables, unused_imports, dead_code)]
use super::*;
use crate::engine::*;
use crate::logic::{ExtFn, LResult};

/// ManageEventDefault (0x000175d0)
pub fn ManageEventDefault(e: &mut Engine, param_1: i32) -> LResult<()> {
    if param_1 == 1 {
        { let a = sh(e, 0x103f4); e.w32(a, 0x2a0000aa); }
        { let a = sh(e, 0x10400); e.w32(a, 0x41000000); }
        { let a = sh(e, 0x10404); e.w32(a, 0x40600000); }
        { let a = sh(e, 0x10408); e.w32(a, 0x40f00000); }
        { let a = sh(e, 0x1040c); e.w32(a, 0); }
        { let a = sh(e, 0x10410); e.w32(a, 0); }
        { let a = sh(e, 0x10414); e.w32(a, 0x80004200); }
        Camera_BaseFUNC_MessageHandler(e, 0x54, 3, sh(e, 0x103d0))?;
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
/// CameraFUNC_MessageHandler -> EpN_CameraFUNC_MessageHandler
pub fn msg(e: &mut Engine, p1: u32, p2: u32, p3: Ptr) -> LResult<u64> {
    std_msg(e, p1, p2, p3)
}
/// cameraConstructor
pub fn ctor(e: &mut Engine, api: Ptr, n: i32) -> LResult<()> {
    std_ctor(e, api, n, 0x17c, false)
}

//! camera__f16e6791: ManageEvent* (FloorEvent/BaseCam*.c) and camera.c
#![allow(non_snake_case, unused_variables, unused_imports, dead_code)]
use super::*;
use crate::engine::*;
use crate::logic::{ExtFn, LResult};

/// ManageEventDefault (0x00017600)
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

/// CameraST_FIRSTMOVE
pub fn first(e: &mut Engine) -> LResult<u64> {
    std_first(e)
}
/// cameraConstructor
pub fn ctor(e: &mut Engine, api: Ptr, n: i32) -> LResult<()> {
    std_ctor(e, api, n, 0x17c, false)
}
/// CameraFUNC_MessageHandler (0x0001ea10): 0x59 moves the DM (LC+0xd8) target to object param_3[7]
pub fn msg(e: &mut Engine, p1: u32, p2: u32, p3: Ptr) -> LResult<u64> {
    if p1 == 0x59 {
        let v = e.alloca(0x10);
        let o = e.r32(p3 + 0x1c);
        APIgameobj::GetPosition(e, o, 0x7fff_ffff, v);
        let a = e.alloca(0x80);
        let dm = e.r32(LC(e) + 0xd8);
        APIphysic::DMGetParams(e, dm, a);
        for i in 0..3 {
            let x = e.r32(v + i * 4);
            e.w32(a + 0x10 + i * 4, x);
        }
        for (d, s) in [(0x1c, 0x10), (0x20, 0x14), (0x24, 0x18), (0, 0x24)] {
            let x = e.r32(p3 + s);
            e.w32(a + d, x);
        }
        APIphysic::DMSetParams(e, dm, a);
    }
    std_msg(e, p1, p2, p3)
}

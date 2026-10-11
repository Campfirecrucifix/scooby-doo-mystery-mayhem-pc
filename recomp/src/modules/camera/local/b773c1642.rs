//! camera__773c1642: ManageEvent* (FloorEvent/BaseCam*.c) and camera.c
#![allow(non_snake_case, unused_variables, unused_imports, dead_code)]
use super::*;
use crate::engine::*;
use crate::logic::{ExtFn, LResult};

/// ManageEventDefault (0x00017fd8)
pub fn ManageEventDefault(e: &mut Engine, param_1: i32) -> LResult<()> {
    if param_1 == 1 {
        { let a = sh(e, 0x103f0); e.w32(a, 2); }
        { let a = sh(e, 0x10414); e.w32(a, lead(e)); }
        { let a = sh(e, 0x1042c); e.w32(a, 0); }
        { let a = sh(e, 0x10430); e.w32(a, 0); }
        { let a = sh(e, 0x10434); e.w32(a, 0xc0000000); }
        { let t = sh(e, 0x103f0); Camera_BaseFUNC_SetCameraUser(e, t)?; }
    }
    Ok(())
}

/// ManageEvent001 (0x000180c0)
pub fn ManageEvent001(e: &mut Engine, param_1: i32) -> LResult<()> {
    if param_1 == 1 {
        { let a = sh(e, 0x10440); e.w32(a, 0); }
        { let a = sh(e, 0x1047c); e.w32(a, 0); }
        { let a = sh(e, 0x10480); e.w32(a, 0); }
        { let a = sh(e, 0x10484); e.w32(a, 0xc0000000); }
        { let t = sh(e, 0x10440); Camera_BaseFUNC_SetCameraUser(e, t)?; }
    }
    Ok(())
}

/// ManageEvent002 (0x00018188)
pub fn ManageEvent002(e: &mut Engine, param_1: i32) -> LResult<()> {
    if param_1 == 1 {
        { let a = sh(e, 0x10490); e.w32(a, 0); }
        { let a = sh(e, 0x10494); e.w32(a, 0x2a0000bd); }
        { let a = sh(e, 0x104a0); e.w32(a, 0); }
        { let a = sh(e, 0x104a4); e.w32(a, 0x3f800000); }
        { let a = sh(e, 0x104a8); e.w32(a, 0x41000000); }
        { let t = sh(e, 0x10490); Camera_BaseFUNC_SetCameraAttract(e, t)?; }
    }
    Ok(())
}

/// ManageEvent003 (0x00018270)
pub fn ManageEvent003(e: &mut Engine, param_1: i32) -> LResult<()> {
    if param_1 == 1 {
        { let a = sh(e, 0x104b0); e.w32(a, 3); }
        { let a = sh(e, 0x104b4); e.w32(a, 0xc0a00000); }
        { let a = sh(e, 0x104b8); e.w32(a, 0x40800000); }
        { let a = sh(e, 0x104bc); e.w32(a, 0); }
        { let a = sh(e, 0x104c0); e.w32(a, 0); }
        { let a = sh(e, 0x104c4); e.w32(a, 0x3f800000); }
        { let a = sh(e, 0x104c8); e.w32(a, 0); }
        { let a = sh(e, 0x104ec); e.w32(a, 0); }
        { let a = sh(e, 0x104f0); e.w32(a, 0); }
        { let a = sh(e, 0x104f4); e.w32(a, 0xe0002200); }
        { let t = sh(e, 0x104b0); Camera_BaseFUNC_SetCameraUser(e, t)?; }
    }
    Ok(())
}

/// ManageEvent004 (0x00018388)
pub fn ManageEvent004(e: &mut Engine, param_1: i32) -> LResult<()> {
    match param_1 {
        0 => {
        }
        1 => {
            { let a = sh(e, 0x10500); e.w32(a, 0); }
            { let a = sh(e, 0x10504); e.w32(a, 0x2a0000be); }
            { let a = sh(e, 0x10510); e.w32(a, 0x3f800000); }
            { let a = sh(e, 0x10514); e.w32(a, 0x3f800000); }
            { let a = sh(e, 0x10518); e.w32(a, 0x40a00000); }
            { let t = sh(e, 0x10500); Camera_BaseFUNC_SetCameraAttract(e, t)?; }
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

/// ManageEvent005 (0x00018488)
pub fn ManageEvent005(e: &mut Engine, param_1: i32) -> LResult<()> {
    match param_1 {
        0 => {
        }
        1 => {
            { let a = sh(e, 0x10520); e.w32(a, 0); }
            { let a = sh(e, 0x10524); e.w32(a, 0x2a0000bf); }
            { let a = sh(e, 0x10530); e.w32(a, 0x3f800000); }
            { let a = sh(e, 0x10534); e.w32(a, 0x3f800000); }
            { let a = sh(e, 0x10538); e.w32(a, 0x40a00000); }
            { let t = sh(e, 0x10520); Camera_BaseFUNC_SetCameraAttract(e, t)?; }
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

/// ManageEvent006 (0x00018588)
pub fn ManageEvent006(e: &mut Engine, param_1: i32) -> LResult<()> {
    match param_1 {
        0 => {
        }
        1 => {
            { let a = sh(e, 0x10540); e.w32(a, 0); }
            { let a = sh(e, 0x10544); e.w32(a, 0x2a0000c0); }
            { let a = sh(e, 0x10550); e.w32(a, 0x3f800000); }
            { let a = sh(e, 0x10554); e.w32(a, 0x3f800000); }
            { let a = sh(e, 0x10558); e.w32(a, 0x40c00000); }
            { let t = sh(e, 0x10540); Camera_BaseFUNC_SetCameraAttract(e, t)?; }
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

/// ManageEvent007 (0x00018688)
pub fn ManageEvent007(e: &mut Engine, param_1: i32) -> LResult<()> {
    match param_1 {
        0 => {
        }
        1 => {
            { let a = sh(e, 0x10560); e.w32(a, 0); }
            { let a = sh(e, 0x10564); e.w32(a, 0x2a0000c0); }
            { let a = sh(e, 0x10570); e.w32(a, 0); }
            { let a = sh(e, 0x10574); e.w32(a, 0x3f800000); }
            { let a = sh(e, 0x10578); e.w32(a, 0x40c00000); }
            { let t = sh(e, 0x10560); Camera_BaseFUNC_SetCameraAttract(e, t)?; }
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

/// Events[] slot dispatch (0 = ManageEventDefault)
pub fn ME(e: &mut Engine, n: u8, s: i32) -> LResult<()> {
    match n {
        0 => ManageEventDefault(e, s),
        1 => ManageEvent001(e, s),
        2 => ManageEvent002(e, s),
        3 => ManageEvent003(e, s),
        4 => ManageEvent004(e, s),
        5 => ManageEvent005(e, s),
        6 => ManageEvent006(e, s),
        7 => ManageEvent007(e, s),
        _ => Ok(()),
    }
}

/// CameraST_FIRSTMOVE (0x000202e0)
pub fn first(e: &mut Engine) -> LResult<u64> {
    let l = lead(e);
    send(e, l, 0x7b, 0xe4, NULL);
    EpN_CameraST_FIRSTMOVE(e)?;
    let c = LC(e);
    e.w32(c + 0x17c, 0);
    Ok(0)
}
/// moCameraParams in LC+8 aimed at `tgt`, context flag saved in LC+0x108, sent to 0x2a0000b3 (0x54)
fn cut(e: &mut Engine, tgt: u32) {
    let c = LC(e);
    ws(e, c, &[(8, 4), (0x24, 0x2a00_00b5), (0x2c, tgt), (0x44, 1), (0x48, 0), (0x4c, 0x400a)]);
    let f = APIgameobj::GetContextFlag(e, 0x2a00_0000);
    e.w32(c + 0x108, f);
    APIgameobj::SetContextFlag(e, 0x2a00_0000, 0x10);
    send(e, 0x2a00_00b3, 0x54, 0, c + 8);
}
/// CameraFUNC_MessageHandler (0x000203a0)
pub fn msg(e: &mut Engine, p1: u32, p2: u32, p3: Ptr) -> LResult<u64> {
    let c = LC(e);
    match p1 {
        0x147 if p2 == 0x2a00_00e4 => {
            send(e, 0x2a00_00e4, 0x13c, 0, NULL);
            send(e, 0x2a10_0006, 0x1e, 0, NULL);
            cut(e, 0x2a00_00e0);
        }
        0x57 => {
            let a = e.alloca(0x10);
            ws(e, a, &[(4, 0x8000_0002), (0, 0), (8, 1)]);
            send(e, 0x2a00_00b3, 0x1df, 0, a);
            return Ok(0);
        }
        0x58 => {
            send(e, 0x2a10_0006, 0x1f, 0, NULL);
            if p2 == 0 {
                send(e, 0x2a00_00af, 0x98, 99, NULL);
            }
            return Ok(0);
        }
        0x148 if p2 == 0x2a00_00e4 => {
            send(e, 0x2a00_00e4, 0x13b, 0, NULL);
            send(e, 0x2a10_0006, 0x1f, 0, NULL);
            send(e, 0x2a00_00b3, 0x55, 0, NULL);
        }
        0x2711 => match e.r32(c + 0x17c) {
            0 => {
                e.w32(c + 0x17c, 1);
                send(e, 0x2a10_0006, 0x1e, 0, NULL);
                cut(e, 0x2a00_00e4);
            }
            1 => {
                e.w32(c + 0x17c, 2);
                send(e, 0x2a10_0006, 0x1f, 0, NULL);
                send(e, 0x2a00_00b3, 0x55, 0, NULL);
                let f = e.r32(c + 0x108);
                APIgameobj::SetContextFlag(e, 0x2a00_0000, f);
            }
            _ => {}
        },
        _ => {}
    }
    Camera_BaseFUNC_MessageHandler(e, p1, p2, p3)
}
/// cameraConstructor (0x000208b8)
pub fn ctor(e: &mut Engine, api: Ptr, n: i32) -> LResult<()> {
    EpN_CameraFUNC_Constructor(e, api, n)?;
    local_data(e, 0x180);
    if n == 1 {
        APIcons::SetExternalFunction(e, 1, FIRST);
        let s = e.cstr("ApiCFunctions");
        let f = APIcons::GetApiCustom(e, s);
        let a = G(e, X_pFunction);
        e.w32(a, f);
        global_data(e, 0x20);
    }
    APIcons::SetMessageFunction(e, CameraFUNC_MessageHandler);
    APIcons::SetInitialState(e, 1);
    Ok(())
}

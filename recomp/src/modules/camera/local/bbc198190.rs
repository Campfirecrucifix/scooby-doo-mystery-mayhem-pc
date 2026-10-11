//! camera__bc198190: ManageEvent* (FloorEvent/BaseCam*.c) and camera.c
#![allow(non_snake_case, unused_variables, unused_imports, dead_code)]
use super::*;
use crate::engine::*;
use crate::logic::{ExtFn, LResult};

/// ManageEventDefault (0x000176f0)
pub fn ManageEventDefault(e: &mut Engine, param_1: i32) -> LResult<()> {
    if param_1 == 1 {
        { let a = sh(e, 0x103e0); e.w32(a, 2); }
        { let a = sh(e, 0x10404); e.w32(a, lead(e)); }
        { let a = sh(e, 0x1041c); e.w32(a, 0); }
        { let a = sh(e, 0x10420); e.w32(a, 0); }
        { let a = sh(e, 0x10424); e.w32(a, 0xc0000000); }
        { let t = sh(e, 0x103e0); Camera_BaseFUNC_SetCameraUser(e, t)?; }
    }
    Ok(())
}

/// ManageEvent001 (0x000177d8)
pub fn ManageEvent001(e: &mut Engine, param_1: i32) -> LResult<()> {
    match param_1 {
        0 => {
        }
        1 => {
            { let a = sh(e, 0x10430); e.w32(a, 0); }
            { let a = sh(e, 0x10434); e.w32(a, 0x2a00001e); }
            { let a = sh(e, 0x10440); e.w32(a, 0x3f800000); }
            { let a = sh(e, 0x10444); e.w32(a, 0x3f800000); }
            { let a = sh(e, 0x10448); e.w32(a, 0x40a00000); }
            { let t = sh(e, 0x10430); Camera_BaseFUNC_SetCameraAttract(e, t)?; }
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

/// ManageEvent002 (0x000178d8)
pub fn ManageEvent002(e: &mut Engine, param_1: i32) -> LResult<()> {
    match param_1 {
        0 => {
        }
        1 => {
            { let a = sh(e, 0x10450); e.w32(a, 0); }
            { let a = sh(e, 0x10454); e.w32(a, 0x2a00001f); }
            { let a = sh(e, 0x10460); e.w32(a, 0x3f800000); }
            { let a = sh(e, 0x10464); e.w32(a, 0x3f800000); }
            { let a = sh(e, 0x10468); e.w32(a, 0x40a00000); }
            { let t = sh(e, 0x10450); Camera_BaseFUNC_SetCameraAttract(e, t)?; }
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

/// ManageEvent003 (0x000179d8)
pub fn ManageEvent003(e: &mut Engine, param_1: i32) -> LResult<()> {
    match param_1 {
        0 => {
        }
        1 => {
            { let a = sh(e, 0x10470); e.w32(a, 0); }
            { let a = sh(e, 0x10474); e.w32(a, 0x2a000020); }
            { let a = sh(e, 0x10480); e.w32(a, 0x3f800000); }
            { let a = sh(e, 0x10484); e.w32(a, 0x3f800000); }
            { let a = sh(e, 0x10488); e.w32(a, 0x40a00000); }
            { let t = sh(e, 0x10470); Camera_BaseFUNC_SetCameraAttract(e, t)?; }
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

/// ManageEvent004 (0x00017ad8)
pub fn ManageEvent004(e: &mut Engine, param_1: i32) -> LResult<()> {
    if param_1 == 1 {
        { let a = sh(e, 0x10490); e.w32(a, 5); }
        { let a = sh(e, 0x104ac); e.w32(a, 0x2a00001a); }
        { let a = sh(e, 0x104b4); e.w32(a, 0x2a00001b); }
        { let a = sh(e, 0x104cc); e.w32(a, 0); }
        { let a = sh(e, 0x104d0); e.w32(a, 0); }
        { let a = sh(e, 0x104d4); e.w32(a, 0xc0002122); }
        { let t = sh(e, 0x10490); Camera_BaseFUNC_SetCameraUser(e, t)?; }
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
        _ => Ok(()),
    }
}

/// CameraST_FIRSTMOVE (0x0001ec40)
pub fn first(e: &mut Engine) -> LResult<u64> {
    let g = GB0(e);
    e.w32(g + 0x7c4, 0x2a00_00f2);
    if e.r32(g + 0x7ac) == 0 {
        e.w32(g + 0x7bc, 0x18);
        e.w32(g + 0x7c0, 0x18);
    } else {
        let v = e.r32(g + 0x7b8);
        e.w32(g + 0x7bc, v);
        e.w32(g + 0x7c0, v);
        if v & 0x20 != 0 {
            let s = e.alloca(0x10);
            let o = e.r32(g + 0x7c4);
            let p = APIgameobj::GetPosition(e, o, 0x7fff_ffff, s);
            APIgameobj::SetTranslation(e, 0x2a00_002c, p);
            let o = e.r32(g + 0x7c4);
            APIgameobj::TakeOrientation(e, 0x2a00_002c, o, 0x7fff_ffff);
        }
    }
    let l = lead(e);
    let v = e.r32(g + 0x7bc);
    send(e, l, 0x7b, v, NULL);
    std_first(e)
}
/// CameraLocal_CB_CUT_SCENE (0x0001edc8), ext fn 0x10
fn CameraLocal_CB_CUT_SCENE(e: &mut Engine, p: u32) -> LResult<u64> {
    if e.r32(p) < 6 {
        let a = e.alloca(0x10);
        e.w32(a, 1);
        if e.r32(p + 4) == 1 {
            APIphysic::SetFlags(e, SELF_CAM, 1);
            send(e, 0x2a00_0031, 0x5e, 1, NULL);
            APIgameobj::SendMessageTo(e, 0xff00_0001, a, 0x4c, 0, NULL);
            Camera_BaseFUNC_SetCutSceneMode(e, 7)?;
            let b = LB(e);
            let t = e.r32(b + 300);
            APIextra::TimerUnregisterNotify(e, SELF_CAM, t);
            APIextra::TimerReset(e, t);
            e.w32(b + 0x158, 0);
        } else {
            send(e, 0x2a00_0031, 0x5e, 0, NULL);
            APIgameobj::SendMessageTo(e, 0xff00_0001, a, 0x51, 0, NULL);
            if e.r32(LB(e) + 0x15c) != 1 {
                APIphysic::ResetFlags(e, SELF_CAM, 1);
                Camera_BaseFUNC_SetCutSceneMode(e, 8)?;
            }
            let v = e.r32(p);
            send(e, SELF_CAM, 0x58, v, NULL);
        }
    } else {
        Camera_BaseCB_CUT_SCENE(e, p)?;
    }
    Ok(0)
}
/// CameraFUNC_MessageHandler (0x0001f040)
pub fn msg(e: &mut Engine, p1: u32, p2: u32, p3: Ptr) -> LResult<u64> {
    if p1 == 0x98 {
        if p2 == 1 {
            send(e, 0x2a00_010e, 0x15a, 0, NULL);
        } else if p2 == 3 {
            send(e, 0x2a00_010e, 0x15a, 1, NULL);
        }
    }
    Camera_BaseFUNC_MessageHandler(e, p1, p2, p3)
}
/// cameraConstructor (0x0001f150)
pub fn ctor(e: &mut Engine, api: Ptr, n: i32) -> LResult<()> {
    EpN_CameraFUNC_Constructor(e, api, n)?;
    local_data(e, 0x17c);
    if n == 1 {
        APIcons::SetExternalFunction(e, 1, FIRST);
        APIcons::SetExternalFunction(e, 0x10, CameraLocal_CB_CUT_SCENE);
        global_data(e, 0x20);
    }
    APIcons::SetMessageFunction(e, CameraFUNC_MessageHandler);
    APIcons::SetInitialState(e, 1);
    Ok(())
}

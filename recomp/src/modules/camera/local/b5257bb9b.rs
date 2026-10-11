//! camera__5257bb9b: ManageEvent* (FloorEvent/BaseCam*.c) and camera.c
#![allow(non_snake_case, unused_variables, unused_imports, dead_code)]
use super::*;
use crate::engine::*;
use crate::logic::{ExtFn, LResult};

/// ManageEventDefault (0x00017e78)
pub fn ManageEventDefault(e: &mut Engine, param_1: i32) -> LResult<()> {
    if param_1 == 1 {
        { let a = sh(e, 0x10428); e.w32(a, 2); }
        { let a = sh(e, 0x1044c); e.w32(a, lead(e)); }
        { let a = sh(e, 0x10464); e.w32(a, 0); }
        { let a = sh(e, 0x10468); e.w32(a, 0); }
        { let a = sh(e, 0x1046c); e.w32(a, 0xc0000000); }
        { let t = sh(e, 0x10428); Camera_BaseFUNC_SetCameraUser(e, t)?; }
    }
    Ok(())
}

/// ManageEvent01 (0x00017f60)
pub fn ManageEvent01(e: &mut Engine, param_1: i32) -> LResult<()> {
    match param_1 {
        0 => {
        }
        1 => {
            { let p = sh(e, 0x10210); let v = e.rv3(p); CamSetCameaModeTour(e, v); }
        }
        2 => {
        }
        3 => {
            CamResetCameaModeTour(e);
        }
        4 => {
        }
        _ => {}
    }
    Ok(())
}

/// ManageEvent02 (0x00018018)
pub fn ManageEvent02(e: &mut Engine, param_1: i32) -> LResult<()> {
    match param_1 {
        0 => {
        }
        1 => {
            { let p = sh(e, 0x10220); let v = e.rv3(p); CamSetCameaModeTour(e, v); }
        }
        2 => {
        }
        3 => {
            CamResetCameaModeTour(e);
        }
        4 => {
        }
        _ => {}
    }
    Ok(())
}

/// ManageEvent03 (0x000180d0)
pub fn ManageEvent03(e: &mut Engine, param_1: i32) -> LResult<()> {
    match param_1 {
        0 => {
        }
        1 => {
            { let p = sh(e, 0x10230); let v = e.rv3(p); CamSetCameaModeTour(e, v); }
        }
        2 => {
        }
        3 => {
            CamResetCameaModeTour(e);
        }
        4 => {
        }
        _ => {}
    }
    Ok(())
}

/// ManageEvent04 (0x00018188)
pub fn ManageEvent04(e: &mut Engine, param_1: i32) -> LResult<()> {
    match param_1 {
        0 => {
        }
        1 => {
            { let p = sh(e, 0x10240); let v = e.rv3(p); CamSetCameaModeTour(e, v); }
        }
        2 => {
        }
        3 => {
            CamResetCameaModeTour(e);
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
        1 => ManageEvent01(e, s),
        2 => ManageEvent02(e, s),
        3 => ManageEvent03(e, s),
        4 => ManageEvent04(e, s),
        _ => Ok(()),
    }
}

/// CameraST_FIRSTMOVE (0x0001fc88)
pub fn first(e: &mut Engine) -> LResult<u64> {
    let a = e.alloca(0x10);
    e.w32(a, 0xd);
    let g = GB0(e);
    if e.r32(g + 0x7ac) != 0 {
        let v = e.r32(g + 0x7b8);
        e.w32(g + 0x7bc, v);
        e.w32(g + 0x7c0, v);
        let l = lead(e);
        send(e, l, 0x7b, v, a);
    }
    EpN_CameraST_FIRSTMOVE(e)?;
    let s = e.alloca(0x30);
    ws(e, s, &[(0, 1), (4, 0), (8, 0x3f4c_cccd), (0xc, 0x4040_0000), (0x10, 0x3f33_3333), (0x14, 0x40),
        (0x18, 0x7fff_ffff), (0x1c, 0x3dcc_cccd), (0x20, 0x4080_0000), (0x24, 0x80), (0x28, 0x3f00_0000)]);
    for o in [0x2a00_00e6, 0x2a00_0044, 0x2a00_004a] {
        APIgameobj::SetShadowParams(e, o, s);
    }
    // CameraFUNC_SetFog (0x0001fe58)
    set_col(e, 0x17c, [0x41a8_0000, 0x4208_0000, 0x4244_0000, 0]);
    let c = col255(e, 0x17c);
    send(e, 0x2a00_000f, 0x5b, 0, c);
    fog(e, 1.0, Some(30.0));
    fog_color(e, 0x17c);
    enable_fog(e, 1);
    Ok(0)
}
/// CameraFUNC_MessageHandler (0x00020070)
pub fn msg(e: &mut Engine, p1: u32, p2: u32, p3: Ptr) -> LResult<u64> {
    let g = GC0(e);
    let s60 = e.alloca(0x10);
    e.w32(s60, 0xd);
    let gb = GB0(e);
    if p1 == 0x7b {
        e.w32(gb + 0x7bc, 0xe2);
        e.w32(gb + 0x7c0, 0xe2);
        let to = e.r32(g + 8);
        send(e, to, 0x7b, 0xe2, s60);
    } else if p1 < 0x7c {
        let g3 = GCi(e, 3);
        if p1 == 0x52 && room(e) == 1 && e.r32(g3 + 0x1c) & 1 == 0 && p2 == 1 {
            let a = e.alloca(0x10);
            e.w32(a, 0x8000_0002);
            or32(e, g3 + 0x1c, 1);
            send(e, 0x2a00_000f, 0x1de, 1, a);
        }
    } else if p1 == 0x98 {
        enable_fog(e, 1);
        let v = e.alloca(0x10);
        if p2 == 2 {
            APIgameobj::LeaveParent(e, 0x2a00_004a, 0);
            send(e, 0x2a00_004a, 0xd2, 0x262, NULL);
            and32(e, g + 0x34, 0xffff_f8ee);
            or32(e, g + 0x34, 0x510);
            send(e, 0x2a00_000f, 0x47, 0xd, NULL);
            let p = APIgameobj::GetPosition(e, 0x2a00_0007, 0x7fff_ffff, v);
            APIgameobj::SetTranslation(e, 0x2a00_004a, p);
        } else if p2 != 3 {
            let p = APIgameobj::GetPosition(e, 0x2a00_0007, 0x7fff_ffff, v);
            APIgameobj::Translate(e, 0x2a00_004a, p);
        } else {
            APIgameobj::LeaveParent(e, 0x2a00_004a, 0);
            send(e, 0x2a00_004a, 0xd2, 0x262, NULL);
            send(e, 0x2a00_0044, 0xd2, 0x262, NULL);
            and32(e, g + 0x14, 0xffff_fffb);
            and32(e, g + 0x34, 0xffff_f8ee);
            or32(e, g + 0x34, 0x410);
            send(e, 0x2a00_000f, 0x47, 0xd, NULL);
            let p = APIgameobj::GetPosition(e, 0x2a00_0008, 0x7fff_ffff, v);
            APIgameobj::SetTranslation(e, 0x2a00_004a, p);
        }
    }
    Camera_BaseFUNC_MessageHandler(e, p1, p2, p3)
}
/// cameraConstructor (0x00020568) (null_printf trace dropped)
pub fn ctor(e: &mut Engine, api: Ptr, n: i32) -> LResult<()> {
    EpN_CameraFUNC_Constructor(e, api, n)?;
    local_data(e, 0x18c);
    if n == 1 {
        APIcons::SetExternalFunction(e, 1, FIRST);
        get_graphics(e);
        global_data(e, 0x20);
    }
    APIcons::SetMessageFunction(e, CameraFUNC_MessageHandler);
    let g = GC0(e);
    and32(e, g + 0x34, 0xffff_ffef);
    or32(e, g + 0x34, 1);
    ws(e, g, &[(8, 0x2a00_0044), (0xc, 0x2a00_004a), (0x10, 1), (4, 0xd)]);
    APIcons::SetInitialState(e, 1);
    Ok(())
}

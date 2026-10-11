//! camera__78c39c02: ManageEvent* (FloorEvent/BaseCam*.c) and camera.c
#![allow(non_snake_case, unused_variables, unused_imports, dead_code)]
use super::*;
use crate::engine::*;
use crate::logic::{ExtFn, LResult};

/// ManageEventDefault (0x00017f68)
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

/// ManageEvent001 (0x00018050)
pub fn ManageEvent001(e: &mut Engine, param_1: i32) -> LResult<()> {
    if param_1 == 1 {
        { let a = sh(e, 0x10430); e.w32(a, 3); }
        { let a = sh(e, 0x10434); e.w32(a, 0); }
        { let a = sh(e, 0x10438); e.w32(a, 0x40400000); }
        { let a = sh(e, 0x1043c); e.w32(a, 0x41100000); }
        { let a = sh(e, 0x10440); e.w32(a, 0); }
        { let a = sh(e, 0x10444); e.w32(a, 0x40000000); }
        { let a = sh(e, 0x10448); e.w32(a, 0); }
        { let a = sh(e, 0x10454); e.w32(a, lead(e)); }
        { let a = sh(e, 0x1046c); e.w32(a, 0); }
        { let a = sh(e, 0x10470); e.w32(a, 0); }
        { let a = sh(e, 0x10474); e.w32(a, 0x40002000); }
        { let t = sh(e, 0x10430); Camera_BaseFUNC_SetCameraUser(e, t)?; }
    }
    Ok(())
}

/// ManageEvent002 (0x00018180)
pub fn ManageEvent002(e: &mut Engine, param_1: i32) -> LResult<()> {
    if param_1 == 1 {
        { let a = sh(e, 0x10480); e.w32(a, 3); }
        { let a = sh(e, 0x10484); e.w32(a, 0x41100000); }
        { let a = sh(e, 0x10488); e.w32(a, 0x40400000); }
        { let a = sh(e, 0x1048c); e.w32(a, 0); }
        { let a = sh(e, 0x10490); e.w32(a, 0); }
        { let a = sh(e, 0x10494); e.w32(a, 0x40000000); }
        { let a = sh(e, 0x10498); e.w32(a, 0); }
        { let a = sh(e, 0x104a4); e.w32(a, lead(e)); }
        { let a = sh(e, 0x104bc); e.w32(a, 0); }
        { let a = sh(e, 0x104c0); e.w32(a, 0); }
        { let a = sh(e, 0x104c4); e.w32(a, 0xc0002000); }
        { let t = sh(e, 0x10480); Camera_BaseFUNC_SetCameraUser(e, t)?; }
    }
    Ok(())
}

/// ManageEvent004 (0x000183c0)
pub fn ManageEvent004(e: &mut Engine, param_1: i32) -> LResult<()> {
    if param_1 == 1 {
        { let a = sh(e, 0x10520); e.w32(a, 3); }
        { let a = sh(e, 0x10524); e.w32(a, 0x41100000); }
        { let a = sh(e, 0x10528); e.w32(a, 0x40400000); }
        { let a = sh(e, 0x1052c); e.w32(a, 0); }
        { let a = sh(e, 0x10530); e.w32(a, 0); }
        { let a = sh(e, 0x10534); e.w32(a, 0x40000000); }
        { let a = sh(e, 0x10538); e.w32(a, 0); }
        { let a = sh(e, 0x10544); e.w32(a, lead(e)); }
        { let a = sh(e, 0x1055c); e.w32(a, 0); }
        { let a = sh(e, 0x10560); e.w32(a, 0); }
        { let a = sh(e, 0x10564); e.w32(a, 0xc0008400); }
        { let t = sh(e, 0x10520); Camera_BaseFUNC_SetCameraUser(e, t)?; }
    }
    Ok(())
}

/// ManageEvent005 (0x000184f0)
pub fn ManageEvent005(e: &mut Engine, param_1: i32) -> LResult<()> {
    match param_1 {
        0 => {
        }
        1 => {
            { let a = sh(e, 0x10570); e.w32(a, 0); }
            { let a = sh(e, 0x10574); e.w32(a, 0x2a0000e1); }
            { let a = sh(e, 0x10580); e.w32(a, 0x40000000); }
            { let a = sh(e, 0x10584); e.w32(a, 0x3f800000); }
            { let a = sh(e, 0x10588); e.w32(a, 0x40e00000); }
            { let t = sh(e, 0x10570); Camera_BaseFUNC_SetCameraAttract(e, t)?; }
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

/// ManageEvent003 (0x000182b0)
pub fn ManageEvent003(e: &mut Engine, param_1: i32) -> LResult<()> {
    if param_1 == 1 {
        let m = sh(e, 0x104d0);
        e.w32(m, 0);
        let l = lead(e);
        e.w32(sh(e, 0x104f4), l);
        e.w32(sh(e, 0x1050c), 0);
        e.w32(sh(e, 0x10510), 0);
        e.w32(sh(e, 0x10514), 0xc000_0000);
        Camera_BaseFUNC_SetCameraUser(e, m)?;
        APIgameobj::SendMessageTo(e, 0x2a00_00e8, NULL, 0xd5, 0, NULL);
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
        _ => Ok(()),
    }
}

/// cameraConstructor
pub fn ctor(e: &mut Engine, api: Ptr, n: i32) -> LResult<()> {
    std_ctor(e, api, n, 0x17c, false)
}
/// CameraST_FIRSTMOVE (0x00020060)
pub fn first(e: &mut Engine) -> LResult<u64> {
    let l = lead(e);
    send(e, l, 0x7b, 0xe4, NULL);
    EpN_CameraST_FIRSTMOVE(e)?;
    Ok(0)
}
/// CameraFUNC_MessageHandler (0x00020110): 0x1a7/1000 MSG_CAMERA_ON: follow camera on the leader
pub fn msg(e: &mut Engine, p1: u32, p2: u32, p3: Ptr) -> LResult<u64> {
    if p1 == 0x1a7 && p2 == 1000 {
        dbg(e, "MSG_CAMERA_ON");
        let l = lead(e);
        cam_user(e, 0x10590, &[(0, 0), (0x24, l), (0x3c, 0), (0x40, 0), (0x44, 0xc000_0000)])?;
        return Ok(0);
    }
    std_msg(e, p1, p2, p3)
}

//! camera__fae3f0bb: ManageEvent* (FloorEvent/BaseCam*.c) and camera.c
#![allow(non_snake_case, unused_variables, unused_imports, dead_code)]
use super::*;
use crate::engine::*;
use crate::logic::{ExtFn, LResult};

/// ManageEventDefault (0x000177d0)
pub fn ManageEventDefault(e: &mut Engine, param_1: i32) -> LResult<()> {
    if param_1 == 1 {
        { let a = sh(e, 0x103f8); e.w32(a, 2); }
        { let a = sh(e, 0x1041c); e.w32(a, lead(e)); }
        { let a = sh(e, 0x10434); e.w32(a, 0); }
        { let a = sh(e, 0x10438); e.w32(a, 0); }
        { let a = sh(e, 0x1043c); e.w32(a, 0xc0000000); }
        { let t = sh(e, 0x103f8); Camera_BaseFUNC_SetCameraUser(e, t)?; }
    }
    Ok(())
}

/// ManageEvent001 (0x000178b8)
pub fn ManageEvent001(e: &mut Engine, param_1: i32) -> LResult<()> {
    if param_1 == 1 {
        { let a = sh(e, 0x10448); e.w32(a, 4); }
        { let a = sh(e, 0x10464); e.w32(a, 0x2a000013); }
        { let a = sh(e, 0x1046c); e.w32(a, 0x2a000125); }
        { let a = sh(e, 0x10458); e.w32(a, 0); }
        { let a = sh(e, 0x1045c); e.w32(a, 0x40000000); }
        { let a = sh(e, 0x10460); e.w32(a, 0); }
        { let a = sh(e, 0x10484); e.w32(a, 0); }
        { let a = sh(e, 0x10488); e.w32(a, 0); }
        { let a = sh(e, 0x1048c); e.w32(a, 0xa000421a); }
        { let t = sh(e, 0x10448); Camera_BaseFUNC_SetCameraUser(e, t)?; }
    }
    Ok(())
}

/// ManageEvent002 (0x000179c8)
pub fn ManageEvent002(e: &mut Engine, param_1: i32) -> LResult<()> {
    match param_1 {
        0 => {
        }
        1 => {
            { let a = sh(e, 0x10498); e.w32(a, 0); }
            { let a = sh(e, 0x1049c); e.w32(a, 0x2a000014); }
            { let a = sh(e, 0x104a8); e.w32(a, 0x3f800000); }
            { let a = sh(e, 0x104ac); e.w32(a, 0x3f800000); }
            { let a = sh(e, 0x104b0); e.w32(a, 0x41000000); }
            { let t = sh(e, 0x10498); Camera_BaseFUNC_SetCameraAttract(e, t)?; }
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

/// ManageEvent003 (0x00017ac8)
pub fn ManageEvent003(e: &mut Engine, param_1: i32) -> LResult<()> {
    match param_1 {
        0 => {
        }
        1 => {
            { let a = sh(e, 0x104b8); e.w32(a, 0); }
            { let a = sh(e, 0x104bc); e.w32(a, 0x2a000015); }
            { let a = sh(e, 0x104c8); e.w32(a, 0x3f800000); }
            { let a = sh(e, 0x104cc); e.w32(a, 0x3f800000); }
            { let a = sh(e, 0x104d0); e.w32(a, 0x41000000); }
            { let t = sh(e, 0x104b8); Camera_BaseFUNC_SetCameraAttract(e, t)?; }
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

/// ManageEvent006 (0x00017e68)
pub fn ManageEvent006(e: &mut Engine, param_1: i32) -> LResult<()> {
    match param_1 {
        0 => {
        }
        1 => {
            { let a = sh(e, 0x10518); e.w32(a, 0); }
            { let a = sh(e, 0x1051c); e.w32(a, 0x2a000018); }
            { let a = sh(e, 0x10528); e.w32(a, 0x3f800000); }
            { let a = sh(e, 0x1052c); e.w32(a, 0x3f800000); }
            { let a = sh(e, 0x10530); e.w32(a, 0x41000000); }
            { let t = sh(e, 0x10518); Camera_BaseFUNC_SetCameraAttract(e, t)?; }
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

/// ManageEvent007 (0x00017f68)
pub fn ManageEvent007(e: &mut Engine, param_1: i32) -> LResult<()> {
    match param_1 {
        0 => {
        }
        1 => {
            { let a = sh(e, 0x10538); e.w32(a, 0); }
            { let a = sh(e, 0x1053c); e.w32(a, 0x2a000019); }
            { let a = sh(e, 0x10548); e.w32(a, 0x3f800000); }
            { let a = sh(e, 0x1054c); e.w32(a, 0x3f800000); }
            { let a = sh(e, 0x10550); e.w32(a, 0x41000000); }
            { let t = sh(e, 0x10538); Camera_BaseFUNC_SetCameraAttract(e, t)?; }
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

/// ManageEvent004 (0x00017bc8) / ManageEvent005 (0x00017d18): attractor unless quest `q` completed
fn me_quest(e: &mut Engine, param_1: i32, q: u32, qflag: u32, m: u32, id: u32) -> LResult<()> {
    match param_1 {
        1 => {
            let p = sh(e, qflag);
            APIgameobj::SendMessageTo(e, q, NULL, 0x20a, 0, p);
            if e.r32(p) == 0 {
                let b = sh(e, m);
                e.w32(b, 0);
                e.w32(b + 4, id);
                e.w32(b + 0x10, 0x3f80_0000);
                e.w32(b + 0x14, 0x3f80_0000);
                e.w32(b + 0x18, 0x4100_0000);
                Camera_BaseFUNC_SetCameraAttract(e, b)?;
            }
        }
        3 => {
            if e.r32(sh(e, qflag)) == 0 {
                ReleaseAttractor(e);
            }
        }
        _ => {}
    }
    Ok(())
}
/// ManageEvent004 (0x00017bc8)
pub fn ManageEvent004(e: &mut Engine, param_1: i32) -> LResult<()> {
    me_quest(e, param_1, 0x2a00_0172, 0x104f4, 0x104d8, 0x2a00_0016)
}
/// ManageEvent005 (0x00017d18)
pub fn ManageEvent005(e: &mut Engine, param_1: i32) -> LResult<()> {
    me_quest(e, param_1, 0x2a00_0173, 0x10514, 0x104f8, 0x2a00_0017)
}
/// ManageEvent008 (0x00018068): objective bits 0x2000000 / 0x10000000 save (6) / restore (5)
pub fn ManageEvent008(e: &mut Engine, param_1: i32) -> LResult<()> {
    let os = sh(e, 0x1021c);
    let g2 = e.r32(GDB(e) + 0xc) + 8;
    let p = param_1 as u32;
    if p == 5 {
        if e.r32(os) & 0x200_0000 == 0 {
            let v = e.r32(g2) & 0xfdff_ffff;
            e.w32(g2, v);
            APIgameobj::SendMessageTo(e, 0x2a10_0006, NULL, 0x2d, 2, NULL);
            APIgameobj::SendMessageTo(e, 0x2a10_0006, NULL, 0x2c, 2, NULL);
        }
        if e.r32(os) & 0x1000_0000 == 0 {
            APIgameobj::SendMessageTo(e, 0x2a10_0006, NULL, 0x2d, 4, NULL);
            APIgameobj::SendMessageTo(e, 0x2a10_0006, NULL, 0x2c, 4, NULL);
        }
    } else if p < 6 {
        if p == 2 && e.r32(g2) & 0x200_0000 == 0 {
            let s = e.alloca(0x10);
            e.w32(s, 0x8000_0008);
            if e.r32(GB0(e) + 0x14) & 0x0800_090d == 0 {
                APIgameobj::SendMessageTo(e, 0x2a00_000e, NULL, 0x1de, 0, s);
                let v = e.r32(g2) | 0x200_0000;
                e.w32(g2, v);
            }
        }
    } else if p == 6 {
        if e.r32(g2) & 0x200_0000 != 0 {
            let v = e.r32(os) | 0x200_0000;
            e.w32(os, v);
        }
        if e.r32(g2) & 0x1000_0000 != 0 {
            let v = e.r32(os) | 0x1000_0000;
            e.w32(os, v);
        }
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
        8 => ManageEvent008(e, s),
        _ => Ok(()),
    }
}

/// CameraST_FIRSTMOVE (0x0001f348)
pub fn first(e: &mut Engine) -> LResult<u64> {
    EpN_CameraST_FIRSTMOVE(e)?;
    set_col(e, 0x17c, [0x41a8_0000, 0x4208_0000, 0x4244_0000, 0]);
    let c = col255(e, 0x17c);
    send(e, 0x2a00_000e, 0x5b, 0, c);
    fog(e, 1.0, None);
    fog_color(e, 0x17c);
    enable_fog(e, 1);
    Ok(0)
}
/// CameraCB_CUTSCENE_LOCAL (0x0001f540), ext fn 0x10
fn CameraCB_CUTSCENE_LOCAL(e: &mut Engine, p: u32) -> LResult<u64> {
    let v = e.r32(p);
    let l = LC(e);
    e.w32(l + 0x18c, v);
    if v != 3 || e.r32(p + 4) == 1 {
        Camera_BaseCB_CUT_SCENE(e, p)?;
    }
    Ok(0)
}
/// CameraFUNC_MessageHandler (0x0001f608)
pub fn msg(e: &mut Engine, p1: u32, p2: u32, p3: Ptr) -> LResult<u64> {
    match p1 {
        0x98 => match room(e) {
            1 | 3 => enable_fog(e, 1),
            2 => enable_fog(e, 0),
            _ => {}
        },
        0x4d => {
            if e.r32(LC(e) + 0x18c) == 3 {
                let b = LB(e);
                e.w32(b + 0x158, 0);
            }
        }
        0x52 => {
            let c = col255(e, 0x17c);
            clear_color(e, c);
        }
        0xde => enable_fog(e, 1),
        0xd4 => FloorEventForceState(e, 6)?,
        0xdf => enable_fog(e, 0),
        _ => {}
    }
    Camera_BaseFUNC_MessageHandler(e, p1, p2, p3)
}
/// cameraConstructor (0x0001f908)
pub fn ctor(e: &mut Engine, api: Ptr, n: i32) -> LResult<()> {
    EpN_CameraFUNC_Constructor(e, api, n)?;
    local_data(e, 400);
    if n == 1 {
        APIcons::SetExternalFunction(e, 1, FIRST);
        APIcons::SetExternalFunction(e, 0x10, CameraCB_CUTSCENE_LOCAL);
        get_graphics(e);
        global_data(e, 0x20);
    }
    APIcons::SetMessageFunction(e, CameraFUNC_MessageHandler);
    APIcons::SetInitialState(e, 1);
    Ok(())
}

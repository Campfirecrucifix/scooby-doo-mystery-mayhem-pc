//! camera__49127a51: ManageEvent* (FloorEvent/BaseCam*.c) and camera.c
#![allow(non_snake_case, unused_variables, unused_imports, dead_code)]
use super::*;
use crate::engine::*;
use crate::logic::{ExtFn, LResult};

/// ManageEventDefault (0x00017ed8)
pub fn ManageEventDefault(e: &mut Engine, param_1: i32) -> LResult<()> {
    if param_1 == 1 {
        { let a = sh(e, 0x103e8); e.w32(a, 2); }
        { let a = sh(e, 0x1040c); e.w32(a, lead(e)); }
        { let a = sh(e, 0x10424); e.w32(a, 0); }
        { let a = sh(e, 0x10428); e.w32(a, 0); }
        { let a = sh(e, 0x1042c); e.w32(a, 0xc0000000); }
        { let t = sh(e, 0x103e8); Camera_BaseFUNC_SetCameraUser(e, t)?; }
    }
    Ok(())
}

/// ManageEvent001 (0x00017fc0)
pub fn ManageEvent001(e: &mut Engine, param_1: i32) -> LResult<()> {
    let fr = e.alloca(0x20);
    match param_1 {
        0 => {
            { let p = sh(e, 0x10438); APIgameobj::GetPosition(e, 0x2a000036, 0x7fff_ffff, p); }
            APIgameobj::KillObject(e, 0x2a000036);
        }
        1 => {
            { let a = fr + 0; e.w32(a, 0); }
            { let a = fr + 4; e.w32(a, 0); }
            { let a = fr + 0xc; e.w32(a, sh(e, 0x10438)); }
            { let a = fr + 0x10; e.w32(a, 0x3f800000); }
            { let a = fr + 0x14; e.w32(a, 0x3f800000); }
            { let a = fr + 0x18; e.w32(a, 0x43480000); }
            Camera_BaseFUNC_SetCameraAttract(e, fr)?;
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

/// ManageEvent002 (0x000180f8)
pub fn ManageEvent002(e: &mut Engine, param_1: i32) -> LResult<()> {
    let fr = e.alloca(0x20);
    match param_1 {
        0 => {
            { let p = sh(e, 0x10448); APIgameobj::GetPosition(e, 0x2a000037, 0x7fff_ffff, p); }
            APIgameobj::KillObject(e, 0x2a000037);
        }
        1 => {
            { let a = fr + 0; e.w32(a, 0); }
            { let a = fr + 4; e.w32(a, 0); }
            { let a = fr + 0xc; e.w32(a, sh(e, 0x10448)); }
            { let a = fr + 0x10; e.w32(a, 0x3f800000); }
            { let a = fr + 0x14; e.w32(a, 0x3f800000); }
            { let a = fr + 0x18; e.w32(a, 0x43480000); }
            Camera_BaseFUNC_SetCameraAttract(e, fr)?;
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

/// ManageEvent003 (0x00018230)
pub fn ManageEvent003(e: &mut Engine, param_1: i32) -> LResult<()> {
    let fr = e.alloca(0x20);
    match param_1 {
        0 => {
            { let p = sh(e, 0x10458); APIgameobj::GetPosition(e, 0x2a000038, 0x7fff_ffff, p); }
            APIgameobj::KillObject(e, 0x2a000038);
        }
        1 => {
            { let a = fr + 0; e.w32(a, 0); }
            { let a = fr + 4; e.w32(a, 0); }
            { let a = fr + 0xc; e.w32(a, sh(e, 0x10458)); }
            { let a = fr + 0x10; e.w32(a, 0x3f800000); }
            { let a = fr + 0x14; e.w32(a, 0x3f800000); }
            { let a = fr + 0x18; e.w32(a, 0x43480000); }
            Camera_BaseFUNC_SetCameraAttract(e, fr)?;
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

/// ManageEvent004 (0x00018368)
pub fn ManageEvent004(e: &mut Engine, param_1: i32) -> LResult<()> {
    let fr = e.alloca(0x20);
    match param_1 {
        0 => {
            { let p = sh(e, 0x10468); APIgameobj::GetPosition(e, 0x2a000039, 0x7fff_ffff, p); }
            APIgameobj::KillObject(e, 0x2a000039);
        }
        1 => {
            { let a = fr + 0; e.w32(a, 0); }
            { let a = fr + 4; e.w32(a, 0); }
            { let a = fr + 0xc; e.w32(a, sh(e, 0x10468)); }
            { let a = fr + 0x10; e.w32(a, 0x3f800000); }
            { let a = fr + 0x14; e.w32(a, 0x3f800000); }
            { let a = fr + 0x18; e.w32(a, 0x43480000); }
            Camera_BaseFUNC_SetCameraAttract(e, fr)?;
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

/// ManageEvent005 (0x000184a0)
pub fn ManageEvent005(e: &mut Engine, param_1: i32) -> LResult<()> {
    let fr = e.alloca(0x20);
    match param_1 {
        0 => {
            { let p = sh(e, 0x10478); APIgameobj::GetPosition(e, 0x2a00003a, 0x7fff_ffff, p); }
            APIgameobj::KillObject(e, 0x2a00003a);
        }
        1 => {
            { let a = fr + 0; e.w32(a, 0); }
            { let a = fr + 4; e.w32(a, 0); }
            { let a = fr + 0xc; e.w32(a, sh(e, 0x10478)); }
            { let a = fr + 0x10; e.w32(a, 0x3f800000); }
            { let a = fr + 0x14; e.w32(a, 0x3f800000); }
            { let a = fr + 0x18; e.w32(a, 0x43480000); }
            Camera_BaseFUNC_SetCameraAttract(e, fr)?;
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

/// ManageEvent006 (0x000185d8)
pub fn ManageEvent006(e: &mut Engine, param_1: i32) -> LResult<()> {
    let fr = e.alloca(0x20);
    match param_1 {
        0 => {
            { let p = sh(e, 0x10488); APIgameobj::GetPosition(e, 0x2a00003b, 0x7fff_ffff, p); }
            APIgameobj::KillObject(e, 0x2a00003b);
        }
        1 => {
            { let a = fr + 0; e.w32(a, 0); }
            { let a = fr + 4; e.w32(a, 0); }
            { let a = fr + 0xc; e.w32(a, sh(e, 0x10488)); }
            { let a = fr + 0x10; e.w32(a, 0x3f800000); }
            { let a = fr + 0x14; e.w32(a, 0x3f800000); }
            { let a = fr + 0x18; e.w32(a, 0x43480000); }
            Camera_BaseFUNC_SetCameraAttract(e, fr)?;
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

/// ManageEvent007 (0x00018710)
pub fn ManageEvent007(e: &mut Engine, param_1: i32) -> LResult<()> {
    let fr = e.alloca(0x20);
    match param_1 {
        0 => {
        }
        1 => {
            { let a = fr + 0; e.w32(a, 4); }
            { let a = fr + 4; e.w32(a, 0x2a00003c); }
            { let a = fr + 0xc; e.w32(a, 0); }
            { let a = fr + 0x10; e.w32(a, 0x3f800000); }
            { let a = fr + 0x14; e.w32(a, 0x3f800000); }
            { let a = fr + 0x18; e.w32(a, 0x42a00000); }
            Camera_BaseFUNC_SetCameraAttract(e, fr)?;
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

const COL: [u32; 4] = [0x41c8_0000, 0x3f80_0000, 0x420c_0000, 0];
/// CameraST_FIRSTMOVE (0x00020268)
pub fn first(e: &mut Engine) -> LResult<u64> {
    EpN_CameraST_FIRSTMOVE(e)?;
    set_col(e, 0x17c, COL);
    fog(e, 1.0, Some(5.0));
    enable_fog(e, 0);
    fog_color(e, 0x17c);
    Ok(0)
}
/// CameraFUNC_MessageHandler (0x000203f0)
pub fn msg(e: &mut Engine, p1: u32, p2: u32, p3: Ptr) -> LResult<u64> {
    if p1 == 0x52 && p2 == 1 {
        match room(e) {
            1 => {
                enable_fog(e, 0);
                APIgraphics::DoStupidXboxShadowHack(e, 1);
            }
            2 => {
                APIgraphics::DoStupidXboxShadowHack(e, 0);
                set_col(e, 0x17c, COL);
                let c = col255(e, 0x17c);
                clear_color(e, c);
                fog(e, 1.0, Some(5.0));
                enable_fog(e, 0);
                fog_color(e, 0x17c);
                enable_fog(e, 1);
            }
            3 => {
                enable_fog(e, 0);
                APIgraphics::DoStupidXboxShadowHack(e, 0);
            }
            _ => {}
        }
    }
    Camera_BaseFUNC_MessageHandler(e, p1, p2, p3)
}
/// cameraConstructor (0x00020760)
pub fn ctor(e: &mut Engine, api: Ptr, n: i32) -> LResult<()> {
    EpN_CameraFUNC_Constructor(e, api, n)?;
    local_data(e, 0x18c);
    if n == 1 {
        APIcons::SetExternalFunction(e, 1, FIRST);
        get_graphics(e);
        global_data(e, 0x20);
        global_buf(e, 0, 0, 0x8f8);
    }
    APIcons::SetMessageFunction(e, CameraFUNC_MessageHandler);
    APIcons::SetInitialState(e, 1);
    Ok(())
}

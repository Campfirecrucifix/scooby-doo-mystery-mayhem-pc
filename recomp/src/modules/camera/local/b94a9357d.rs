//! camera__94a9357d: ManageEvent* (FloorEvent/BaseCam*.c) and camera.c
#![allow(non_snake_case, unused_variables, unused_imports, dead_code)]
use super::*;
use crate::engine::*;
use crate::logic::{ExtFn, LResult};

/// ManageEventDefault (0x00017dc8)
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

/// ManageEvent001 (0x00017eb0)
pub fn ManageEvent001(e: &mut Engine, param_1: i32) -> LResult<()> {
    let fr = e.alloca(0x20);
    match param_1 {
        0 => {
        }
        1 => {
            { let a = fr + 0; e.w32(a, 4); }
            { let a = fr + 4; e.w32(a, 0x2a00000e); }
            { let a = fr + 0xc; e.w32(a, 0); }
            { let a = fr + 0x10; e.w32(a, 0x3f800000); }
            { let a = fr + 0x14; e.w32(a, 0x3f800000); }
            { let a = fr + 0x18; e.w32(a, 0x41000000); }
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

/// ManageEvent003 (0x000181c8)
pub fn ManageEvent003(e: &mut Engine, param_1: i32) -> LResult<()> {
    let fr = e.alloca(0x20);
    match param_1 {
        0 => {
            { let p = sh(e, 0x10440); APIgameobj::GetPosition(e, 0x2a000010, 0x7fff_ffff, p); }
            APIgameobj::KillObject(e, 0x2a000010);
        }
        1 => {
            { let a = sh(e, 0x101f0); e.w32(a, 0); }
            { let a = fr + 0; e.w32(a, 0); }
            { let a = fr + 4; e.w32(a, 0); }
            { let a = fr + 0xc; e.w32(a, sh(e, 0x10440)); }
            { let a = fr + 0x10; e.w32(a, 0x3fc00000); }
            { let a = fr + 0x14; e.w32(a, 0x3f800000); }
            { let a = fr + 0x18; e.w32(a, 0x41000000); }
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

const ui32flagEvent002: u32 = 0x101f0;
/// ManageEvent002 (0x00017fa8)
pub fn ManageEvent002(e: &mut Engine, param_1: i32) -> LResult<()> {
    let fr = e.alloca(0x20);
    let f = sh(e, ui32flagEvent002);
    let gb2c = |e: &Engine| e.r32(e.r32(GDB(e) + 0xc) + 0xc) & 4 != 0;
    match param_1 {
        0 => {
            let v = gb2c(e) as u32;
            e.w32(f, v);
            let p = sh(e, 0x10430);
            APIgameobj::GetPosition(e, 0x2a00_000f, 0x7fff_ffff, p);
            APIgameobj::KillObject(e, 0x2a00_000f);
        }
        1 => {
            if !gb2c(e) {
                e.w32(f, 0);
            }
            if e.r32(f) == 0 {
                e.w32(fr, 0);
                e.w32(fr + 4, 0);
                let p = sh(e, 0x10430);
                e.w32(fr + 0xc, p);
                e.w32(fr + 0x10, 0x4000_0000);
                e.w32(fr + 0x14, 0x3f80_0000);
                e.w32(fr + 0x18, 0x4100_0000);
                Camera_BaseFUNC_SetCameraAttract(e, fr)?;
            }
        }
        2 => {
            if e.r32(GB0(e) + 0x14) & 0x200 != 0 && e.r32(f) == 1 {
                e.w32(f, 0);
                ManageEvent002(e, 1)?;
            }
        }
        3 => {
            if e.r32(f) == 0 {
                ReleaseAttractor(e);
            }
            if gb2c(e) {
                e.w32(f, 1);
            }
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
        _ => Ok(()),
    }
}

/// CameraST_FIRSTMOVE (0x0001fcc0)
pub fn first(e: &mut Engine) -> LResult<u64> {
    std_first(e)
}
/// CameraFUNC_MessageHandler (0x0001fd40)
pub fn msg(e: &mut Engine, p1: u32, p2: u32, p3: Ptr) -> LResult<u64> {
    let b = LB(e);
    if p1 == 0x59 {
        let s = e.alloca(0xb0);
        let d = e.r32(b + 0xd8);
        APIphysic::DMGetParams(e, d, s);
        APIgameobj::GetPosition(e, p2, 1, s + 0x1c);
        APIphysic::DMSetParams(e, d, s);
        return Ok(0);
    }
    if p1 == 0x98 {
        // (target, object, +0x30, flags)
        let t = match p2 {
            0x2a00_0105 => Some((0x2a00_000c, None, 0x4122)),
            0x2a00_0104 => Some((0x2a00_000a, None, 0x4122)),
            0x2a00_0106 => Some((0x2a00_000d, Some(5), 0x4162)),
            0x2a00_0107 => Some((0x2a00_000b, Some(6), 0x4162)),
            _ => None,
        };
        if let Some((o, x30, fl)) = t {
            ws(e, b, &[(8, 5), (0x24, o), (0x2c, p2)]);
            if let Some(v) = x30 {
                e.w32(b + 0x30, v);
            }
            ws(e, b, &[(0x44, 1), (0x48, 0), (0x4c, fl)]);
            Camera_BaseFUNC_MessageHandler(e, 0x54, 0, b + 8)?;
            return Ok(0);
        }
    }
    Camera_BaseFUNC_MessageHandler(e, p1, p2, p3)
}
/// cameraConstructor (0x000200c8)
pub fn ctor(e: &mut Engine, api: Ptr, n: i32) -> LResult<()> {
    EpN_CameraFUNC_Constructor(e, api, n)?;
    local_data(e, 0x17c);
    if n == 1 {
        APIcons::SetExternalFunction(e, 1, FIRST);
        global_data(e, 0x20);
        global_buf(e, 0, 0, 0x8f8);
    }
    APIcons::SetMessageFunction(e, CameraFUNC_MessageHandler);
    APIcons::SetInitialState(e, 1);
    Ok(())
}

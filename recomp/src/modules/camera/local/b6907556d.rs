//! camera__6907556d: ManageEvent* (FloorEvent/BaseCam*.c) and camera.c
#![allow(non_snake_case, unused_variables, unused_imports, dead_code)]
use super::*;
use crate::engine::*;
use crate::logic::{ExtFn, LResult};

/// ManageEventDefault (0x00017630)
pub fn ManageEventDefault(e: &mut Engine, param_1: i32) -> LResult<()> {
    if param_1 == 1 {
        { let a = sh(e, 0x103d8); e.w32(a, 1); }
        { let a = sh(e, 0x10400); e.w32(a, 0x7fffffff); }
        { let a = sh(e, 0x103fc); e.w32(a, 0x2a00000d); }
        { let a = sh(e, 0x10414); e.w32(a, 0); }
        { let a = sh(e, 0x10418); e.w32(a, 0); }
        { let a = sh(e, 0x103dc); e.w32(a, 0); }
        { let a = sh(e, 0x103e0); e.w32(a, 0x40800000); }
        { let a = sh(e, 0x103e4); e.w32(a, 0xc0a00000); }
        { let a = sh(e, 0x103e8); e.w32(a, 0); }
        { let a = sh(e, 0x103ec); e.w32(a, 0x40000000); }
        { let a = sh(e, 0x103f0); e.w32(a, 0x40a00000); }
        { let a = sh(e, 0x1041c); e.w32(a, 0x80004208); }
        { let t = sh(e, 0x103d8); Camera_BaseFUNC_SetCameraUser(e, t)?; }
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

/// CameraFUNC_InitFade (0x0001eb10): fog / light colour ramps
fn CameraFUNC_InitFade(e: &mut Engine) {
    let l = LC(e);
    ws(e, l, &[
        (0x18c, 0x4110_0000), (0x190, 0x4110_0000), (0x194, 0x41e8_0000), (0x198, 0),
        (0x19c, 0x42fa_0000), (0x1a0, 0x42c0_0000), (0x1a4, 0x4220_0000), (0x1a8, 0),
        (0x1cc, 0x3e80_8081), (0x1d0, 0x3e80_8081), (0x1d4, 0x3f3f_bfc0), (0x1d8, 0),
        (0x1dc, 0x3ee6_e6e7), (0x1e0, 0x3ee6_e6e7), (0x1e4, 0x3f59_d9da), (0x1e8, 0),
        (0x1ac, 0x3dcc_cccd), (0x1b0, 0x3e4c_cccd), (0x1b4, 0x3f00_0000), (0x1b8, 0),
        (0x1bc, 0x3ecc_cccd), (0x1c0, 0x3f00_0000), (0x1c4, 0x3f4c_cccd), (0x1c8, 0),
    ]);
    for i in 0..4 {
        let v = e.r32(l + 0x18c + i * 4);
        e.w32(l + 0x17c + i * 4, v);
    }
}
/// `(to - from) * t / 960 + from` for 4 floats
fn lerp4(e: &mut Engine, from: u32, to: u32, t: u32, out: Ptr) {
    let l = LC(e);
    for i in 0..4 {
        let a = e.rf(l + from + i * 4);
        let b = e.rf(l + to + i * 4);
        e.wf(out + i * 4, ((b - a) * t as f32) / 960.0 + a);
    }
}
/// CameraFUNC_FadingFog (0x0001ede8): message 0x1f8, t in 0..960
fn CameraFUNC_FadingFog(e: &mut Engine, t: u32) {
    let l = LC(e);
    lerp4(e, 0x18c, 0x19c, t, l + 0x17c);
    let c = col255(e, 0x17c);
    fog_color(e, 0x17c);
    clear_color(e, c);
    send(e, 0x2a00_0004, 0x5b, 0, c);
    let a = e.alloca(0x20);
    lerp4(e, 0x1ac, 0x1bc, t, a);
    lerp4(e, 0x1cc, 0x1dc, t, a + 0x10);
    APIlight::SetColor(e, 0x1280_0002, a);
    APIlight::SetColor(e, 0x1280_0000, a + 0x10);
}
/// CameraST_FIRSTMOVE (0x0001e6c8)
pub fn first(e: &mut Engine) -> LResult<u64> {
    EpN_CameraST_FIRSTMOVE(e)?;
    CameraFUNC_InitFade(e);
    let c = col255(e, 0x17c);
    send(e, 0x2a00_0004, 0x5b, 0, c);
    fog(e, 1.0, None);
    enable_fog(e, 1);
    fog_color(e, 0x17c);
    let g = GC0(e);
    e.w32(g + 8, 0x2a00_000d);
    Ok(0)
}
/// CameraFUNC_MessageHandler (0x0001e8c8)
pub fn msg(e: &mut Engine, p1: u32, p2: u32, p3: Ptr) -> LResult<u64> {
    if p1 == 0xc1 {
        cam_user(e, 0x10428, &[
            (0, 1), (0x28, 0x7fff_ffff), (0x24, 0x2a00_000d), (0x3c, 4), (0x40, 3), (4, 0), (8, 0x4080_0000),
            (0xc, 0xc0a0_0000), (0x10, 0), (0x14, 0x4000_0000), (0x18, 0x40a0_0000), (0x44, 0x8000_4208),
        ])?;
    } else if p1 == 0x52 {
        let c = col255(e, 0x17c);
        clear_color(e, c);
    } else if p1 == 0x1f8 {
        CameraFUNC_FadingFog(e, p2);
    }
    Camera_BaseFUNC_MessageHandler(e, p1, p2, p3)
}
/// cameraConstructor (0x0001f5b8)
pub fn ctor(e: &mut Engine, api: Ptr, n: i32) -> LResult<()> {
    EpN_CameraFUNC_Constructor(e, api, n)?;
    if n == 1 {
        APIcons::SetExternalFunction(e, 1, FIRST);
        get_light(e);
        get_graphics(e);
        global_data(e, 0x20);
    }
    local_data(e, 0x1ec);
    APIcons::SetMessageFunction(e, CameraFUNC_MessageHandler);
    APIcons::SetInitialState(e, 1);
    Ok(())
}

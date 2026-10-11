//! Per-build camera.c / FloorEvent.c / BaseCam*.c dispatch and shared helpers.
//! Each level build has its own `local/b<sha8>.rs` exposing `first` (CameraST_FIRSTMOVE), `msg`
//! (CameraFUNC_MessageHandler), `ctor` (cameraConstructor), `ManageEventDefault` and `ME`
//! (Events[] slot dispatch). Credits / Splash are in [`super::small`].
#![allow(non_snake_case, unused_variables, dead_code)]

use super::*;
use crate::logic::{ExtFn, LResult};

macro_rules! builds {
    ($($m:ident = $k:literal),* $(,)?) => {
        $(pub mod $m;)*
        /// camera.c CameraST_FIRSTMOVE (state 1)
        pub fn CameraST_FIRSTMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
            match KEY(e) { $($k => $m::first(e),)* _ => super::small::CameraST_FIRSTMOVE(e) }
        }
        /// camera.c CameraFUNC_MessageHandler
        pub fn CameraFUNC_MessageHandler(e: &mut Engine, p1: u32, p2: u32, p3: Ptr) -> LResult<u64> {
            match KEY(e) { $($k => $m::msg(e, p1, p2, p3),)* _ => super::small::CameraFUNC_MessageHandler(e, p1, p2, p3) }
        }
        /// camera.c cameraConstructor
        pub fn cameraConstructor(e: &mut Engine, api: Ptr, n: i32) -> LResult<()> {
            match KEY(e) { $($k => $m::ctor(e, api, n),)* _ => super::small::cameraConstructor(e, api, n) }
        }
        /// FloorEvent.c / BaseCam*.c ManageEventDefault
        pub fn ManageEventDefault(e: &mut Engine, s: i32) -> LResult<()> {
            match KEY(e) { $($k => $m::ManageEventDefault(e, s),)* _ => Ok(()) }
        }
        /// `(*Events[n])(state)`: ManageEvent number n of the current build
        pub fn ManageEvent(e: &mut Engine, n: u8, s: i32) -> LResult<()> {
            match KEY(e) { $($k => $m::ME(e, n, s),)* _ => Ok(()) }
        }
    };
}
builds!(
    b0396376a = "0396376a", b07e6c8a4 = "07e6c8a4", b0e2da84f = "0e2da84f", b1372a331 = "1372a331",
    b1ac1b1b0 = "1ac1b1b0", b2500ac27 = "2500ac27", b46dad128 = "46dad128", b49127a51 = "49127a51",
    b5257bb9b = "5257bb9b", b5bccc6a9 = "5bccc6a9", b68501e9f = "68501e9f", b6907556d = "6907556d",
    b6a8753d8 = "6a8753d8", b773c1642 = "773c1642", b78a0064b = "78a0064b", b78c39c02 = "78c39c02",
    b94a9357d = "94a9357d", bb2664a52 = "b2664a52", bba3dca02 = "ba3dca02", bbc198190 = "bc198190",
    bcd52e895 = "cd52e895", be50fa8e8 = "e50fa8e8", bf16e6791 = "f16e6791", bfae3f0bb = "fae3f0bb",
);

/// camera.c cameraDestructor -> EpN_CameraFUNC_Destructor (every build)
pub fn cameraDestructor(e: &mut Engine, n: i32) -> LResult<()> {
    if V2(e).ep == 0 {
        super::small::cameraDestructor(e, n);
    } else {
        EpN_CameraFUNC_Destructor(e, n);
    }
    Ok(())
}

/// camera.c CameraFUNC_SetLocalParams: empty in every build
pub fn CameraFUNC_SetLocalParams(e: &mut Engine) {}

// ---- helpers shared by the b_*.rs files ----

/// build-addressed function static (camera.c / ManageEvent statics, C address `a` >= 0x101f0)
/// relocated into the X_STATICS area after the l01 image
#[inline]
pub(crate) fn sh(e: &Engine, a: u32) -> Ptr {
    e.rt.cur_data() + (X_STATICS - 0x10000) + (a - 0x101f0)
}
/// leader (`*(GB0 + 8)`)
#[inline]
pub(crate) fn lead(e: &Engine) -> u32 {
    e.r32(GB0(e) + 8)
}
/// `pLocalDataCamera`
#[inline]
pub(crate) fn LC(e: &Engine) -> Ptr {
    e.r32(G(e, pLocalDataCamera))
}
/// `*pGlobalDataCamera` (game buffer 0)
#[inline]
pub(crate) fn GC0(e: &Engine) -> Ptr {
    e.r32(e.r32(G(e, pGlobalDataCamera)))
}
/// `pGlobalDataCamera[i]` / `pGlobalDataCameraBase[i]`
#[inline]
pub(crate) fn GCi(e: &Engine, i: u32) -> Ptr {
    e.r32(e.r32(G(e, pGlobalDataCamera)) + i * 4)
}
#[inline]
pub(crate) fn GBi(e: &Engine, i: u32) -> Ptr {
    e.r32(GDB(e) + i * 4)
}
/// camera handle `*(pLocalDataCameraBase + 200)`
#[inline]
pub(crate) fn cam(e: &Engine) -> u32 {
    e.r32(LB(e) + 200)
}
/// room `*(GB0 + 0x24)`
#[inline]
pub(crate) fn room(e: &Engine) -> u32 {
    e.r32(GC0(e) + 0x24)
}
pub(crate) fn or32(e: &mut Engine, a: Ptr, v: u32) {
    let x = e.r32(a) | v;
    e.w32(a, x);
}
pub(crate) fn and32(e: &mut Engine, a: Ptr, v: u32) {
    let x = e.r32(a) & v;
    e.w32(a, x);
}
/// write words at `p + off`
pub(crate) fn ws(e: &mut Engine, p: Ptr, w: &[(u32, u32)]) {
    for &(o, v) in w {
        e.w32(p + o, v);
    }
}
/// `CColor c = LC + off / 255.0` (4 floats in a fresh stack frame)
pub(crate) fn col255(e: &mut Engine, off: u32) -> Ptr {
    let s = e.alloca(0x10);
    let l = LC(e);
    for i in 0..4 {
        let v = e.rf(l + off + i * 4) / 255.0;
        e.wf(s + i * 4, v);
    }
    s
}
/// `LC[off..off+16] = c` (raw words)
pub(crate) fn set_col(e: &mut Engine, off: u32, c: [u32; 4]) {
    let l = LC(e);
    for (i, v) in c.into_iter().enumerate() {
        e.w32(l + off + i as u32 * 4, v);
    }
}
/// SetFogType(1) / SetFogDistance / SetFogDensity (when Some)
pub(crate) fn fog(e: &mut Engine, dist: f32, dens: Option<f32>) {
    let c = cam(e);
    APIcamera::SetFogType(e, c, 1);
    APIcamera::SetFogDistance(e, c, dist);
    if let Some(d) = dens {
        APIcamera::SetFogDensity(e, c, d);
    }
}
pub(crate) fn enable_fog(e: &mut Engine, on: i32) {
    let c = cam(e);
    APIcamera::EnableFog(e, c, on);
}
/// SetFogColor(cam, LC + off)
pub(crate) fn fog_color(e: &mut Engine, off: u32) {
    let c = cam(e);
    let p = LC(e) + off;
    APIcamera::SetFogColor(e, c, p);
}
pub(crate) fn clear_color(e: &mut Engine, p: Ptr) {
    let c = cam(e);
    APIcamera::SetClearColor(e, c, p);
}
pub(crate) fn send(e: &mut Engine, to: u32, msg: u32, p: u32, d: Ptr) {
    APIgameobj::SendMessageTo(e, to, NULL, msg, p, d);
}
/// `pGraphics = api->GetApiGraphics()`
pub(crate) fn get_graphics(e: &mut Engine) {
    let v = APIcons::GetApiGraphics(e);
    let a = G(e, X_pGraphics);
    e.w32(a, v);
}
/// `pLight = api->GetApiLight()`
pub(crate) fn get_light(e: &mut Engine) {
    let v = APIcons::GetApiLight(e);
    let a = G(e, X_pLight);
    e.w32(a, v);
}
pub(crate) fn local_data(e: &mut Engine, size: u32) {
    let slot = G(e, pLocalDataCamera);
    APIcons::GetLocalData(e, slot, size as i32);
}
pub(crate) fn global_data(e: &mut Engine, size: u32) {
    let slot = G(e, pGlobalDataCamera);
    APIcons::GetGlobalData(e, slot, size as i32);
}
/// `pGlobalDataCamera[i] = GetGameBuffer(b, size, 1)`
pub(crate) fn global_buf(e: &mut Engine, i: u32, b: u32, size: u32) {
    let v = APIgameobj::GetGameBuffer(e, b, size, 1);
    let g = e.r32(G(e, pGlobalDataCamera));
    e.w32(g + i * 4, v);
}
pub(crate) const FIRST: ExtFn = CameraST_FIRSTMOVE;

/// the standard cameraConstructor: EpN ctor; GetLocalData(lsize); if (n == 1) { SetExternalFunction(1,
/// CameraST_FIRSTMOVE); `inst1`; GetGlobalData(0x20) }; SetMessageFunction; SetInitialState(1)
pub(crate) fn std_ctor(e: &mut Engine, api: Ptr, n: i32, lsize: u32, graphics: bool) -> LResult<()> {
    EpN_CameraFUNC_Constructor(e, api, n)?;
    local_data(e, lsize);
    if n == 1 {
        APIcons::SetExternalFunction(e, 1, FIRST);
        if graphics {
            get_graphics(e);
        }
        global_data(e, 0x20);
    }
    APIcons::SetMessageFunction(e, CameraFUNC_MessageHandler);
    APIcons::SetInitialState(e, 1);
    Ok(())
}
/// standard CameraST_FIRSTMOVE: `EpN_CameraST_FIRSTMOVE(); return 0;`
pub(crate) fn std_first(e: &mut Engine) -> LResult<u64> {
    EpN_CameraST_FIRSTMOVE(e)?;
    Ok(0)
}
/// standard CameraFUNC_MessageHandler: forwards to EpN_CameraFUNC_MessageHandler
pub(crate) fn std_msg(e: &mut Engine, p1: u32, p2: u32, p3: Ptr) -> LResult<u64> {
    EpN_CameraFUNC_MessageHandler(e, p1, p2, p3)
}
/// fill a moCameraParams static at C address `base` and SetCameraUser on it
pub(crate) fn cam_user(e: &mut Engine, base: u32, w: &[(u32, u32)]) -> LResult<()> {
    let m = sh(e, base);
    ws(e, m, w);
    Camera_BaseFUNC_SetCameraUser(e, m)
}
/// stack moCameraParams (`auStack_80`) filled with `w`, SetCameraUser on it
pub(crate) fn cam_user_stack(e: &mut Engine, w: &[(u32, u32)]) -> LResult<()> {
    let m = e.alloca(0x60);
    ws(e, m, w);
    Camera_BaseFUNC_SetCameraUser(e, m)
}

//! Credits (`Credits/Camera/camera__81820030/camera.c`) and Splash
//! (`Splash/Camera/camera__e960d71e/camera.c`) single-file cameras. Their pLocalDataCamera /
//! pGlobalDataCamera are kept in the common-layout slots (internal only); the GetApi* pointer
//! fetches have no Rust equivalent (static dispatch).
#![allow(non_snake_case, unused_variables)]

use super::*;
use crate::logic::LResult;

fn credits(e: &Engine) -> bool {
    KEY(e) == "81820030"
}
fn lc(e: &Engine) -> Ptr {
    e.r32(G(e, pLocalDataCamera))
}

/// Credits CameraST_FIRSTMOVE (0x00010cc8), state 0
fn Credits_CameraST_FIRSTMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    APIphysic::CreatePhysic(e, SELF_CAM, 1);
    APIphysic::SetFlags(e, SELF_CAM, 2);
    let c = APIcamera::CreateCamera(e, SELF_CAM);
    let l = lc(e);
    e.w32(l + 0xc, c);
    APIcamera::SetRenderFlag(e, c, 0xffff_ffef);
    APIcamera::SetNearClipPlane(e, c, 1.0);
    APIcamera::SetFarClipPlane(e, c, 1000.0);
    let s = e.alloca(0x10);
    APIgameobj::GetPosition(e, 0x2a00_000e, 0x7fff_ffff, s);
    APIgameobj::SetTranslation(e, SELF_CAM, s);
    APIgameobj::SetNextState(e, SELF_CAM, 1);
    Ok(0)
}
/// Credits CameraST_Rolling (0x00010e40), state 1
fn Credits_CameraST_Rolling(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

/// Splash CameraST_INIT (0x00010cc8), state 0
fn Splash_CameraST_INIT(e: &mut Engine, _: u32) -> LResult<u64> {
    let l = lc(e);
    let c = APIcamera::CreateCamera(e, SELF_CAM);
    e.w32(l, c);
    APIphysic::CreatePhysic(e, SELF_CAM, 1);
    APIphysic::SetFlags(e, SELF_CAM, 2);
    APIcamera::SetRenderFlag(e, c, 0);
    APIcamera::SetNearClipPlane(e, c, 1.0);
    APIcamera::SetFarClipPlane(e, c, 1000.0);
    let t = APIextra::TimerCreate(e);
    e.w32(l + 0xc, t);
    APIgameobj::Sleep(e, SELF_CAM, -1.0);
    Ok(0)
}
/// Splash CameraCB_TIMER (0x00010e10), ext fn 1
fn Splash_CameraCB_TIMER(e: &mut Engine, _: u32) -> LResult<u64> {
    APIgameobj::SendMessageTo(e, 0x2a00_000d, NULL, 0x4f, 0, NULL);
    Ok(1)
}
/// Splash Camera_BaseFUNC_FadeInOut (0x00010e78)
fn Splash_FadeInOut(e: &mut Engine, out: bool) {
    let l = lc(e);
    let s = e.alloca(0x10);
    ws(e, s, &[(0, 0x3f80_0000), (4, 0x3f80_0000), (8, 0x3f80_0000)]);
    let c = e.r32(l);
    APIcamera::SetClearColor(e, c, s);
    if e.r32(l + 8) == 0 {
        let w = APIhud::WindowAdd(e, 1, 0xffff_ffff, 0x800, 1);
        e.w32(l + 8, w);
        APIhud::WindowSetPos(e, w, -0.25, -0.25);
        APIhud::WindowSetDims(e, w, 1.5, 1.5);
        APIhud::WindowSetBackGround(e, w, 4, 0.0, 0.0);
    }
    if !out {
        e.w32(l + 4, 1);
        APIhud::ColorFadeStart(e, 0, 0x8000_0004, 1, 0, 1.0, 0xff);
        let t = e.r32(l + 0xc);
        APIextra::TimerReset(e, t);
        APIextra::TimerSetFrequency(e, t, 1.0);
        APIextra::TimerRegisterNotify(e, SELF_CAM, t, 1);
    } else {
        e.w32(l + 4, 3);
        let c = e.r32(l);
        APIcamera::SetRenderFlag(e, c, 0xffff_ffef);
        APIhud::ColorFadeStart(e, 0, 0x8000_0004, 0, 1, 1.0, 0xff);
    }
}

/// state 1 fallback (Credits CameraST_Rolling)
pub fn CameraST_FIRSTMOVE(e: &mut Engine) -> LResult<u64> {
    Ok(0)
}
/// Credits CameraCB_Messages (0x00010e70) / Splash CameraCB_MessagesHandler (0x000110d8)
pub fn CameraFUNC_MessageHandler(e: &mut Engine, p1: u32, p2: u32, p3: Ptr) -> LResult<u64> {
    if !credits(e) && p1 == 0x54 {
        Splash_FadeInOut(e, p2 != 0);
    }
    Ok(0)
}
/// cameraConstructor (Credits 0x00010eb0 / Splash 0x00011180)
pub fn cameraConstructor(e: &mut Engine, api: Ptr, n: i32) -> LResult<()> {
    local_data(e, 0x10);
    if n == 1 {
        if credits(e) {
            APIcons::SetExternalFunction(e, 0, Credits_CameraST_FIRSTMOVE);
            APIcons::SetExternalFunction(e, 1, Credits_CameraST_Rolling);
            global_data(e, 0x18);
            global_buf(e, 0, 0, 0x8f8);
        } else {
            APIcons::SetExternalFunction(e, 0, Splash_CameraST_INIT);
            APIcons::SetExternalFunction(e, 1, Splash_CameraCB_TIMER);
        }
    }
    APIcons::SetMessageFunction(e, super::CameraFUNC_MessageHandler);
    APIcons::SetInitialState(e, 0);
    Ok(())
}
/// cameraDestructor (Credits 0x00011020: camera at +0xc / Splash 0x000112c0: camera at +0)
pub fn cameraDestructor(e: &mut Engine, n: i32) {
    let l = lc(e);
    let c = e.r32(l + if credits(e) { 0xc } else { 0 });
    APIcamera::DestroyCamera(e, c);
}

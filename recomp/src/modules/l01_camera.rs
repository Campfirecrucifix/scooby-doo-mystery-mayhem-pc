//! `l01_camera`: the EP1L01 camera object (l01_camera -> Ep1_Camera -> Camera_Base).
//! Port of `Game Logic Modules/Globals/camera/l01_camera/{L01_camera,ep1_camera,camera_base}.c`
//! (one build, SHA-1 0549a125, pack LEVEL/EP1/L01).
//!
//! FULL PORT (WP-L1, on top of WP-J4): every function of the five sources is ported 1:1:
//! camera_base.c (states 1/2/3, callbacks 4/5/6/0xe/0xf/0x10/0x11..0x1a/0x1e, message handler with
//! every id, SetCameraUser / Battle / Death / Attract / CutSceneMode / Update / CheckCameraMode /
//! SetClipPlane / FadeInOut / SetWindows / InitCamera / ResetMode), CameraFree.c (follow camera
//! `cameraST_CAMERA_FOLLOW` + attractor), BaseCamNewLibrary ManageEventDefault, FloorEventSystem,
//! ep1_camera.c and L01_camera.c (0xd3 / 0x59 / 0xda). Sub-files: l01_camera/{base_cb,base_fn,
//! base_user,msg,free,free2}.rs. ManageEvent001..008 are not ported: they are only reachable through
//! the floor-event table pEventBuffer, which is NULL/0 in l01 (see FloorEventConstructor).
#![allow(non_snake_case, non_upper_case_globals, unused_variables)]

use crate::engine::*;
use crate::logic::{ExtFn, LResult, ModuleDef};

mod base_cb;
mod base_fn;
mod base_user;
mod free;
mod free2;
mod msg;
pub use base_cb::*;
pub use base_fn::*;
pub use base_user::*;
pub use free::*;
pub use free2::*;
pub use msg::*;

pub static MODULE: ModuleDef = ModuleDef {
    name: "l01_camera",
    data_size: 0x1310,
    ctor: l01_cameraConstructor,
    dtor: l01_cameraDestructor,
    register: None,
    init: &[
        // goHeroesData[18] {id, leader, follower, team count/flags, alt id} (module image .text+0)
        (
            0x10000,
            &[
                1, 0x2a00_0091, 0x2a00_0092, 2, 0, //
                2, 0x2a00_0092, 0x2a00_0091, 1, 0, //
                3, 0x2a00_0143, 0x2a00_0142, 0x2a, 0, //
                4, 0x2a00_0142, 0x2a00_0143, 0x29, 0, //
                0xb, 0x2a00_0136, 0x2a00_0091, 0x2b, 2,
            ],
        ),
        // CameraFlags.. fSpeedFactor
        (0x10168, &[0, 0, 0x3f99_999a, 0x7fff_ffff, 0x3f40_0000, 0]),
        // fCameraCurve[6][2] {hauteur, distance} + the 3 CheckWall ray dirs (0x101b0/c0/d0), module image
        (
            0x10180,
            &[
                0, 0x4026_6666, 0x3f80_0000, 0x4000_0000, 0x4000_0000, 0x3fd9_999a, //
                0x4040_0000, 0x4019_999a, 0x4080_0000, 0x4040_0000, 0x40a0_0000, 0x409c_cccd, //
                0, 0, 0x3f80_0000, 0, 0x3f5d_b22d, 0, 0xbf00_0000, 0, 0xbf5d_b22d, 0, 0xbf00_0000, 0,
            ],
        ),
    ],
    build_init: None,
};

// ---- module globals (_globals.h, symtab) ----
pub const goHeroesData: u32 = 0x10000;
pub const CameraFlags: u32 = 0x10168;
pub const ui32Substate: u32 = 0x1016c;
pub const fHauterCameraFocus: u32 = 0x10170;
pub const ui32FollowJoint: u32 = 0x10174;
pub const fOuvertureCaptureActuel: u32 = 0x10178;
pub const fSpeedFactor: u32 = 0x1017c;
/// fCameraCurve[6][2] {hauteur, distance}
pub const fCameraCurve: u32 = 0x10180;
pub const Events: u32 = 0x101f0;
pub const pEventBuffer: u32 = 0x10214;
pub const i32EventBufferSize: u32 = 0x10218;
pub const pAnim: u32 = 0x10264;
pub const pStdlib: u32 = 0x10268;
pub const pExtra: u32 = 0x1026c;
pub const pGame: u32 = 0x10270;
pub const pCamera: u32 = 0x10274;
pub const pGameObj: u32 = 0x10278;
pub const pInput: u32 = 0x1027c;
pub const pPhysic: u32 = 0x10280;
pub const pConsole: u32 = 0x10284;
pub const pHud: u32 = 0x10288;
pub const pMath: u32 = 0x1028c;
pub const pSound: u32 = 0x10290;
pub const pDebug: u32 = 0x10294;
pub const pMiscFunc: u32 = 0x10298;
pub const pApiCFunc: u32 = 0x1029c;
pub const pText: u32 = 0x102a0;
pub const pLocalDataCameraBase: u32 = 0x102a4;
pub const pGlobalDataCameraBase: u32 = 0x102a8;
pub const oAttractorParam: u32 = 0x102b0;
pub const cvCamImpulse: u32 = 0x102f0;
pub const cvCamImpulseWall: u32 = 0x10300;
pub const cvLastFollowY: u32 = 0x10310;
pub const cvTrackEnAvant: u32 = 0x10320;
pub const fHauteurCameraFollow: u32 = 0x1032c;
pub const fHauteurCameraFollowTarget: u32 = 0x10330;
pub const fDistanceCameraFollow: u32 = 0x10334;
pub const fDistanceCameraFollowCollision: u32 = 0x10338;
pub const fDistanceCameraFollowTarget: u32 = 0x1033c;
pub const fTargetCameraFollow: u32 = 0x10340;
pub const fTargetCameraFollowWork: u32 = 0x10344;
pub const fOuvertureCaptureTarget: u32 = 0x10348;
pub const cvCamWallCheck: u32 = 0x10350;
pub const oEnvCollParams: u32 = 0x10360;
pub const oRayIntersectIn: u32 = 0x105d8;
pub const oRayIntersectOut: u32 = 0x10608;
pub const ui32ActualTargetID: u32 = 0x10644;
pub const cvCameraTourCenter: u32 = 0x10648;
pub const fCameraTourTimeIn: u32 = 0x10654;
// function statics (addresses from the code where visible, else chosen in free .bss gaps)
/// BaseCamNewLibrary moCameraParams_102 (fields +0x24/+0x3c/+0x40/+0x44 at DAT_0001041c/34/38/3c)
pub const moCameraParams_102: u32 = 0x103f8;
/// L01_camera moCameraParams_291 (+4.. DAT_000105a4/c4/c8/cc)
pub const moCameraParams_291: u32 = 0x10588;
pub const bInCamAnim_288: u32 = 0x10568;
pub const cmLookAt_290: u32 = 0x1056c;
pub const oCamPosStartCut_289: u32 = 0x10570;
/// camera_base statics: addresses not visible in the code; placed at the end of the 0x1310 image
pub const pFloorAttribs: u32 = 0x11280;
pub const uiNbPics: u32 = 0x112a0;
/// szFileName_204 (snapshot name, debug R3 only; the initial string is not recovered)
pub const szFileName: u32 = 0x112a4;
pub const i32ActualFloorEvent: u32 = 0x10398;
pub const i32FloorEventBufferSize: u32 = 0x1039c;
pub const pEventCall: u32 = 0x103a0;
pub const pLocalDataCameraEp1: u32 = 0x103a4;
pub const pGlobalDataCameraEp1: u32 = 0x103a8;
pub const pLocalDataCamera: u32 = 0x103ac;
pub const pGlobalDataCamera: u32 = 0x103b0;

#[inline]
pub(crate) fn G(e: &Engine, a: u32) -> Ptr {
    e.rt.cur_data() + (a - 0x10000)
}
/// `pLocalDataCameraBase`
#[inline]
pub(crate) fn LB(e: &Engine) -> Ptr {
    e.r32(G(e, pLocalDataCameraBase))
}
/// `pGlobalDataCameraBase` (int*: [0] GB0, [1] GB1, [2] GB3, [3] GB2, [4] GB4, [5] GB6)
#[inline]
pub(crate) fn GDB(e: &Engine) -> Ptr {
    e.r32(G(e, pGlobalDataCameraBase))
}
/// `*pGlobalDataCameraBase`: game buffer 0
#[inline]
pub(crate) fn GB0(e: &Engine) -> Ptr {
    e.r32(GDB(e))
}

pub(crate) const SELF_CAM: u32 = 0x2a00_0000;

/// 0x000184a8 callback 0xf (notify 0x14 = first EFGameLogicMgr::Update): camera init, team set-up
pub fn Camera_BaseCB_INIT(e: &mut Engine, _: u32) -> LResult<u64> {
    let piVar2 = GDB(e);
    let iVar1 = LB(e);
    let gb0 = e.r32(piVar2);
    e.w32(gb0 + 0x30, 0);
    e.w32(gb0 + 0x24, 1);
    e.w32(iVar1 + 0x144, 0xffff_ffff);
    e.w32(iVar1 + 0x108, 0);
    e.w32(iVar1 + 0x158, 0);
    e.w32(iVar1 + 0x100, 2);
    for o in [0xb8, 0xbc, 0xc0, 0xc4] {
        e.w32(iVar1 + o, 0);
    }
    if e.r32(gb0 + 4) != 0xd {
        let v = e.r32(gb0 + 0x34) & 0xffff_fffe;
        e.w32(gb0 + 0x34, v);
        e.w32(gb0 + 0x34, v | 0x10);
    }
    if e.r32(gb0 + 8) == 0 {
        Camera_BaseFUNC_MessageHandler(e, 0x47, 2, NULL)?;
    } else {
        match e.r32(gb0 + 4) {
            3 | 5 | 7 | 9 | 0xe | 0x10 => e.w32(gb0 + 4, 1),
            4 | 6 | 8 | 10 | 0xb | 0xf | 0x11 => e.w32(gb0 + 4, 2),
            0xc | 0xd => e.w32(gb0 + 4, 2),
            _ => {}
        }
        let p2 = e.r32(gb0 + 4);
        Camera_BaseFUNC_MessageHandler(e, 0x47, p2, NULL)?;
    }
    ManageEventDefault(e, 1)?;
    APIgameobj::RegisterNotify(e, SELF_CAM, 0xffff_ffff, 0x16, 0x10);
    APIgameobj::SetNextState(e, SELF_CAM, 1);
    Ok(1)
}

/// 0x00017900 BaseCamNewLibrary ManageEventDefault: follow camera on the leader
pub fn ManageEventDefault(e: &mut Engine, param_1: i32) -> LResult<()> {
    if param_1 == 1 {
        let m = G(e, moCameraParams_102);
        e.w32(m, 2);
        let t = e.r32(GB0(e) + 8);
        e.w32(m + 0x24, t);
        e.w32(m + 0x3c, 0);
        e.w32(m + 0x40, 0);
        e.w32(m + 0x44, 0xc000_0000);
        Camera_BaseFUNC_SetCameraUser(e, m)?;
    }
    Ok(())
}

/// 0x0001f080
pub fn Ep1_CameraST_FIRSTMOVE(e: &mut Engine) -> LResult<u64> {
    APIinput::InputConfigSetState(e, 4, 0xe);
    let l = e.r32(GB0(e) + 8);
    APIgameobj::SendMessageTo(e, l, NULL, 0x97, 0x2a00_003c, NULL);
    Camera_BaseST_FIRSTMOVE(e)?;
    Ok(0)
}

/// 0x0001f300 state 1 (l01)
pub fn CameraST_FIRSTMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    Ep1_CameraST_FIRSTMOVE(e)?;
    // *pGlobalDataCamera: word 0 of the module global block (= GB0, the block is shared by the
    // three GetGlobalData calls of this code)
    let g = e.r32(e.r32(G(e, pGlobalDataCamera)));
    let l = e.r32(GB0(e) + 8);
    if e.r32(g + 0x44) == 1 {
        e.w32(g + 0x44, 0);
        APIgameobj::SendMessageTo(e, l, NULL, 0x97, 0x2a00_0048, NULL);
    } else {
        APIgameobj::SendMessageTo(e, l, NULL, 0x97, 0x2a00_003c, NULL);
    }
    Ok(0)
}

/// 0x0001f128
pub fn Ep1_CameraFUNC_MessageHandler(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    Camera_BaseFUNC_MessageHandler(e, param_1, param_2, param_3)
}

/// 0x0001f420: cut-scene camera anims 0xd3 (start, anim 8 restores the pose / anim 7) / 0xda (end),
/// 0x59 (look at object param_2 when GB0+0x14 & 0x200)
pub fn CameraFUNC_MessageHandler(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    if param_1 == 0xd3 {
        APIphysic::SetFlags(e, SELF_CAM, 1);
        if param_2 == 4 {
            let p = G(e, oCamPosStartCut_289);
            APIgameobj::GetPosition(e, SELF_CAM, 0x7fff_ffff, p);
            let m = APImath::CMatCreateIdentity(e);
            let a = G(e, cmLookAt_290);
            e.w32(a, m);
            let l = APIgameobj::GetLMat(e, SELF_CAM);
            APImath::CMatCopy(e, l as u32, m);
            su(e, bInCamAnim_288, 1);
            let c = e.r32(LB(e) + 200);
            APIanim::Connect(e, SELF_CAM, 8, c);
            APIanim::Play(e, SELF_CAM, 8, 1, 0.0);
        } else if param_2 == 5 {
            su(e, bInCamAnim_288, 1);
            let c = e.r32(LB(e) + 200);
            APIanim::Connect(e, SELF_CAM, 7, c);
            APIanim::Play(e, SELF_CAM, 7, 1, 0.0);
        }
    } else if param_1 < 0xd4 {
        if param_1 == 0x59 {
            // *pGlobalDataCamera (word 0 of the shared global block = GB0)
            let g = e.r32(e.r32(G(e, pGlobalDataCamera)));
            if g != 0 && e.r32(g + 0x14) & 0x200 != 0 {
                let m = G(e, moCameraParams_291);
                e.w32(m, 4);
                e.w32(m + 0x3c, 0);
                e.w32(m + 0x40, 0);
                e.w32(m + 0x44, 0x20_2102);
                e.w32(m + 0x1c, param_2);
                Camera_BaseFUNC_SetCameraUser(e, m)?;
                return Ok(1);
            }
            return Ok(0);
        }
    } else if param_1 == 0xda && gu(e, bInCamAnim_288) != 0 {
        if param_2 == 4 {
            su(e, bInCamAnim_288, 0);
            APIanim::StopAnim(e, SELF_CAM, 8);
            let p = G(e, oCamPosStartCut_289);
            APIgameobj::SetTranslation(e, SELF_CAM, p);
            let m = gu(e, cmLookAt_290);
            APIgameobj::SetLMat(e, SELF_CAM, m);
            APImath::CMatDestroy(e, m);
            APIgameobj::SendMessageTo(e, SELF_CAM, NULL, 0x58, 0, NULL);
        } else if param_2 == 5 {
            su(e, bInCamAnim_288, 0);
            APIanim::StopAnim(e, SELF_CAM, 7);
            APIgameobj::SendMessageTo(e, SELF_CAM, NULL, 0x58, 0, NULL);
        }
        APIphysic::ResetFlags(e, SELF_CAM, 1);
    }
    Ep1_CameraFUNC_MessageHandler(e, param_1, param_2, param_3)
}

/// 0x0001a5f0
pub fn Camera_BaseFUNC_Create_Input(e: &mut Engine) {
    let p = |e: &Engine| e.ri32(GB0(e) + 0x4c);
    APIinput::InputConfigAdd(e, 4);
    APIinput::InputActionAdd(e, 4, 0x33, 0);
    let d = p(e);
    APIinput::InputDeviceAdd(e, 4, 0x33, 0x2f, d, 0, 2, 0, 10);
    APIinput::InputContextAdd(e, SELF_CAM, 0x11, 4, 0x33, 0x19, 8);
    APIinput::InputContextSetState(e, 4, 0x33, 0x19, 0x200);
    APIinput::InputActionAdd(e, 4, 0x37, 0);
    let d = p(e);
    for (a, b, c, f, g, h) in [(3, d, 0, 0, 0, 0x8000), (4, d, 0, 1, 0, 0x8001), (0xb, d, 1, 0, -100, 0xe), (0xc, d, 1, 0, 100, 0xf), (0xd, d, 1, 1, -100, 0xd), (0xe, d, 1, 1, 100, 0xc), (0xb, 1, 2, 0, -100, 0xcb), (0xc, 1, 2, 0, 100, 0xcd), (0xd, 1, 2, 1, -100, 200), (0xe, 1, 2, 1, 100, 0xd0)] {
        APIinput::InputDeviceAdd(e, 4, 0x37, a, b, c, f, g, h);
    }
    APIinput::InputContextSetState(e, 4, 0x37, 2, 0);
    APIinput::InputActionAdd(e, 4, 0x38, 0);
    for (a, b, c, f, g, h) in [(6, d, 0, 0, 0, 0x8003), (6, d, 0, 1, 0, 0x8004), (0x19, 1, 1, 0, -100, 0x24), (0x1b, 1, 1, 0, 100, 0x25), (0x1a, 1, 1, 1, -100, 0x17), (0x1c, 1, 1, 1, 100, 0x32)] {
        APIinput::InputDeviceAdd(e, 4, 0x38, a, b, c, f, g, h);
    }
    APIinput::InputContextSetState(e, 4, 0x38, 3, 0);
    // (action, device, button code, joy code, context cb, context id, state)
    for (act, dev, b0, b1, cb, ctx, st) in [(0x39, 0x3d, 0, 0x32, 0x17, 0x1e, 0x80), (0x3a, 0x3e, 2, 0x24, 0x18, 0x20, 0x40), (0x3b, 0x3f, 1, 0x25, 0x19, 0x22, 0x10), (0x3c, 0x40, 3, 0x17, 0x1a, 0x24, 0x20)] {
        APIinput::InputActionAdd(e, 4, act, 0);
        let d = p(e);
        APIinput::InputDeviceAdd(e, 4, act, dev, d, 0, 2, 0, b0);
        APIinput::InputDeviceAdd(e, 4, act, dev, 1, 1, 2, 0, b1);
        APIinput::InputContextAdd(e, SELF_CAM, cb, 4, act, ctx, 8);
        APIinput::InputContextSetState(e, 4, act, ctx, st);
    }
    APIinput::InputActionAdd(e, 4, 0x34, 0);
    let d = p(e);
    APIinput::InputDeviceAdd(e, 4, 0x34, 0x32, d, 0, 2, 0, 0xb);
    APIinput::InputDeviceAdd(e, 4, 0x34, 0x33, 1, 1, 2, 0, 1);
    APIinput::InputContextAdd(e, SELF_CAM, 0x12, 4, 0x34, 0x1c, 8);
    APIinput::InputContextSetState(e, 4, 0x34, 0x1c, 0x100);
    APIinput::InputConfigSetState(e, 4, 0x200);
    APIinput::InputConfigActivate(e, 4, 1);
}

/// 0x0001a940 camera physic: sphere 0.3, three CameraFlex (0x12) DMs (context 1/2/4), EnvColl (0x22)
pub fn Camera_BaseFUNC_Create_Physic(e: &mut Engine) {
    let iVar1 = LB(e);
    let auStack_40 = e.alloca(0x40);
    APIphysic::CreatePhysic(e, SELF_CAM, 1);
    APIphysic::SetFlags(e, SELF_CAM, 6);
    APIphysic::SetSpherePrimaryVolume(e, SELF_CAM, 0.3, 0.0, 0.0, 0.0);
    APIphysic::PrimSetIntAttribBits(e, SELF_CAM, 0, 0, 5);
    APIphysic::PrimSetIntAttribBits(e, SELF_CAM, 0, 1, 8);
    APIphysic::PrimSetIntAttribBits(e, SELF_CAM, 0, 2, 0x8000_0008);
    APIphysic::PrimSetIntAttribBits(e, SELF_CAM, 0, 5, 4);
    for (off, mask) in [(0xd0, 1), (0xd4, 2), (0xd8, 4)] {
        let dm = APIphysic::DMAdd(e, SELF_CAM, 0x12);
        e.w32(iVar1 + off, dm);
        APIphysic::DMSetContextMask(e, dm, mask);
    }
    let dm = APIphysic::DMAdd(e, SELF_CAM, 0x22);
    e.w32(iVar1 + 0x16c, dm);
    APIphysic::DMGetParams(e, dm, auStack_40);
    e.w32(auStack_40 + 0x10, 0x4080_0000); // uStack_30
    e.w32(auStack_40 + 0x1c, 4); // uStack_24
    e.w32(auStack_40, 0);
    APIphysic::DMSetParams(e, dm, auStack_40);
    APIphysic::DMSetContextMask(e, dm, 8);
    APIphysic::DMSetStateChangeNotifyBits(e, dm, 3);
    APIphysic::DMResetStateBits(e, dm, 3);
    APIgameobj::RegisterNotify(e, SELF_CAM, 0xffff_ffff, 6, 4);
    APIgameobj::RegisterNotify(e, SELF_CAM, 0xffff_ffff, 3, 5);
    APIgameobj::SetContextFlag(e, SELF_CAM, 0);
}

/// 0x0001ac88 view window, fade window, timers, floor events
pub fn Camera_BaseFUNC_Create_Game(e: &mut Engine) {
    // Ghidra types pLocalDataCameraBase as float* here; the `(float)`/`(uint)` casts on the id slots
    // (+200 camera, +0x138 fade window, +0x114.. timers) are register moves (mtc1/mfc1), not
    // conversions: every other function reads these slots as uint. Raw words are kept.
    let pfVar1 = LB(e);
    e.wf(pfVar1, 0.75);
    e.wf(pfVar1 + 4, 0.5625);
    let cam = e.r32(pfVar1 + 0x32 * 4);
    APIcamera::SetViewWindow(e, cam, 0.75, 0.5625);
    if e.r32(pfVar1 + 0x4e * 4) == 0 {
        let w = APIhud::WindowAdd(e, 1, 0xffff_ffff, 0x800, 1);
        e.w32(pfVar1 + 0x4e * 4, w);
        APIhud::WindowSetPos(e, w, -0.25, -0.25);
        APIhud::WindowSetDims(e, w, 1.5, 1.5);
        APIhud::WindowSetBackGround(e, w, 0xb, 0.0, 0.0);
        let n = e.cstr("uiPanelFade");
        APIhud::WindowSetName(e, w, n);
        APIhud::WindowSetFlags(e, w, 0x100);
    }
    for i in [0x45, 0x46, 0x47, 0x48, 0x49, 0x4b] {
        let t = APIextra::TimerCreate(e);
        e.w32(pfVar1 + i * 4, t);
    }
    let (b, n) = (e.r32(G(e, pEventBuffer)), e.ri32(G(e, i32EventBufferSize)));
    FloorEventConstructor(e, b, n);
}

/// 0x00018238 FloorEventManager. The event table (pEventCall as a function array) is empty in l01
/// (i32FloorEventBufferSize 0): only the bookkeeping is reachable.
pub fn FloorEventManager(e: &mut Engine, param_1: i32) {
    if param_1 < e.ri32(G(e, i32FloorEventBufferSize)) {
        // deviation: event function calls (none exist in l01) not dispatched
        let a = G(e, i32ActualFloorEvent);
        e.w32(a, param_1 as u32);
    }
}

/// 0x000183c8 (no events in l01)
pub fn FloorEventForceState(e: &mut Engine, param_1: i32) {}

/// 0x0001f3f8
pub fn CameraFUNC_SetLocalParams(e: &mut Engine) {}

/// 0x000182e8 FloorEventSystem: runs every event's init (pEventBuffer is NULL/0 in l01: no events)
pub fn FloorEventConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) {
    let a = G(e, i32FloorEventBufferSize);
    e.w32(a, instanceCount as u32);
    let a = G(e, pEventCall);
    e.w32(a, api);
    // deviation: the event function table is empty in l01 (i32EventBufferSize 0), loop not ported
    let a = G(e, i32ActualFloorEvent);
    e.w32(a, 0xffff_ffff);
}

/// 0x0001ed58
pub fn Camera_BaseFUNC_Constructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    if instanceCount == 1 {
        let st1: ExtFn = |e, _| Camera_BaseST_FIRSTMOVE(e);
        let phys: ExtFn = |e, p| Camera_BaseCB_PHYSIC(e, p);
        let tbl: [(i32, ExtFn); 20] = [
            (1, st1),
            (2, Camera_BaseST_BASICMOVE),
            (3, Camera_BaseST_USER),
            (6, Camera_BaseCB_TIMER),
            (0xe, Camera_BaseCB_ANIM_LF),
            (0xf, Camera_BaseCB_INIT),
            (0x10, Camera_BaseCB_CUT_SCENE),
            (0x13, Camera_BaseCB_INPUT_SWITCH),
            (0x12, Camera_BaseCB_INPUT_START),
            (0x11, Camera_BaseCB_INPUT_SELECT),
            (0x14, Camera_BaseCB_INPUT_STICK),
            (0x15, Camera_BaseCB_INPUT_RSTICK),
            (0x16, Camera_BaseCB_INPUT_RSTICK_IDLE),
            (0x17, Camera_BaseCB_INPUT_X),
            (0x18, Camera_BaseCB_INPUT_SQUARE),
            (0x19, Camera_BaseCB_INPUT_CIRCLE),
            (0x1a, Camera_BaseCB_INPUT_TRIANGLE),
            (0x1e, Camera_BaseCB_INPUT_R3),
            (4, phys),
            (5, Camera_BaseCB_COLLIDE),
        ];
        for (i, f) in tbl {
            APIcons::SetExternalFunction(e, i, f);
        }
        let tables: [(u32, fn(&mut Engine) -> Ptr); 12] = [
            (pAnim, |e| APIcons::GetApiAnim(e)),
            (pInput, |e| APIcons::GetApiInput(e)),
            (pMath, |e| APIcons::GetApiMath(e)),
            (pStdlib, |e| APIcons::GetApiStdlib(e)),
            (pGame, |e| APIcons::GetApiGame(e)),
            (pCamera, |e| APIcons::GetApiCamera(e)),
            (pGameObj, |e| APIcons::GetApiGameObj(e)),
            (pExtra, |e| APIcons::GetApiExtra(e)),
            (pPhysic, |e| APIcons::GetApiPhysic(e)),
            (pConsole, |e| APIcons::GetApiConsole(e)),
            (pHud, |e| APIcons::GetApiHUD(e)),
            (pSound, |e| APIcons::GetApiNewSound(e)),
        ];
        for (slot, get) in tables {
            let v = get(e);
            let a = G(e, slot);
            e.w32(a, v);
        }
        for (slot, name) in [(pDebug, "debug"), (pMiscFunc, "MiscFunc"), (pApiCFunc, "ApiCFunctions"), (pText, "ApiText")] {
            let n = e.cstr(name);
            let v = APIcons::GetApiCustom(e, n);
            let a = G(e, slot);
            e.w32(a, v);
        }
        let slot = G(e, pGlobalDataCameraBase);
        APIcons::GetGlobalData(e, slot, 0x20);
        for (i, (b, s, c)) in [(0u32, 0x8f8u32, 1u32), (1, 0x68, 1), (3, 0x40, 1), (2, 0x158, 1), (4, 8, 1), (6, 0x14, 0)].into_iter().enumerate() {
            let v = APIgameobj::GetGameBuffer(e, b, s, c);
            let gd = GDB(e);
            e.w32(gd + i as u32 * 4, v);
        }
    }
    let slot = G(e, pLocalDataCameraBase);
    APIcons::GetLocalData(e, slot, 0x17c);
    APIcons::SetMessageFunction(e, Camera_BaseFUNC_MessageHandler);
    let s = e.cstr("Camera> Camera_BaseFUNC_Constructor\n");
    APIextra::null_printf(e, s, 0);
    APIgameobj::RegisterNotify(e, SELF_CAM, 0xffff_ffff, 0x14, 0xf);
    let lb = LB(e);
    e.w32(lb + 0x13c, 0xffff_ffff);
    let c = APIcamera::CreateCamera(e, SELF_CAM);
    let lb = LB(e);
    e.w32(lb + 200, c);
    APIcamera::SetRenderFlag(e, c, 0);
    Ok(())
}

/// 0x0001f198
pub fn Ep1_CameraFUNC_Constructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    Camera_BaseFUNC_Constructor(e, api, instanceCount)?;
    let slot = G(e, pLocalDataCameraEp1);
    APIcons::GetLocalData(e, slot, 0x17c);
    if instanceCount == 1 {
        let slot = G(e, pGlobalDataCameraEp1);
        APIcons::GetGlobalData(e, slot, 0x20);
        let v = APIgameobj::GetGameBuffer(e, 0, 0x8f8, 1);
        let gd = e.r32(G(e, pGlobalDataCameraEp1));
        e.w32(gd, v);
    }
    APIcons::SetMessageFunction(e, Ep1_CameraFUNC_MessageHandler);
    Camera_BaseFUNC_Create_Physic(e);
    Camera_BaseFUNC_Create_Input(e);
    Camera_BaseFUNC_Create_Game(e);
    Ok(())
}

/// 0x0001f850
pub fn l01_cameraConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    Ep1_CameraFUNC_Constructor(e, api, instanceCount)?;
    let slot = G(e, pLocalDataCamera);
    APIcons::GetLocalData(e, slot, 0x17c);
    if instanceCount == 1 {
        APIcons::SetExternalFunction(e, 1, CameraST_FIRSTMOVE);
        let slot = G(e, pGlobalDataCamera);
        APIcons::GetGlobalData(e, slot, 0x20);
    }
    APIcons::SetMessageFunction(e, CameraFUNC_MessageHandler);
    APIcons::SetInitialState(e, 1);
    Ok(())
}

/// 0x0001f930 -> Ep1_CameraFUNC_Destructor 0x0001f2a8 -> Camera_BaseFUNC_Destructor
pub fn l01_cameraDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    Camera_BaseFUNC_Destructor(e, instanceCount);
    Ok(())
}

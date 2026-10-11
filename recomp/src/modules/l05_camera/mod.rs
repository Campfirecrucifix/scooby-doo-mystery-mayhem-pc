//! `l05_camera`: the EP6L05 / EP6L05B camera object (l05_camera -> Ep6_Camera -> Camera_Base).
//! Port of `Game Logic Modules/Globals/camera/l05_camera__{b76d2d71,c87e3daa}/{L05_camera,ep6_camera,
//! camera_base,CameraFree,FloorEvent,FloorEventSystem}.c` (builds b76d2d71 = LEVEL/EP6/L05,
//! c87e3daa = LEVEL/EP6/L05B; the two differ only in object ids, see `BuildConsts`).
//!
//! FULL PORT (WP-K4), derived from the l04a_camera port: same module globals (_globals.h identical),
//! camera_base / CameraFree / FloorEventSystem / ep6_camera identical up to constants. l05 differences
//! vs l04a: goHeroesData has 3 entries, SetClipPlane has room 1 only (0.25 / 250), ManageEventDefault
//! sends msg 0x54 sub 3 (track the boss: 0x2a0000c6 if its x >= 50 else 0x2a0000c0 / L05B 0x2a000012
//! else 0x2a00000c) instead of the leader follow, CameraFUNC_MessageHandler is a plain pass-through
//! (no WarpCameraBehind), constructor has no prim attrib bit and local data 0x17c.
//! Function addresses in the `///` lines of sub-files are those of the l04 build.
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

/// module address of ManageEventDefault: the word of the `Events` table (relocated code pointer)
const EV_DEFAULT: u32 = 0x175d0;

/// per-build object ids (b76d2d71 EP6L05, c87e3daa EP6L05B)
pub(crate) struct BuildConsts {
    /// camera_base 819 / CameraFree 1061: follow / ray-intersect object
    pub follow: u32,
    /// Camera_BaseFUNC_SetCutSceneMode msg 0x5e target
    pub cutscene: u32,
    /// Camera_BaseFUNC_SetCameraDeath
    pub death: u32,
    /// default team (MessageHandler 0x47 "target not yet supported")
    pub heroes: (u32, u32),
    /// Ep6_CameraST_FIRSTMOVE msg 0x97 param
    pub firstmove: u32,
    /// ManageEventDefault: tracked object (x >= 50) / else
    pub track: (u32, u32),
    /// goHeroesData (3 entries x {id, leader, follower, team, alt})
    pub heroes_data: [u32; 15],
}

pub(crate) static BUILDS: [BuildConsts; 2] = [
    BuildConsts {
        follow: 0x2a00_0004,
        cutscene: 0x2a00_000c,
        death: 0x2a00_011e,
        heroes: (0x2a00_0007, 0x2a00_0008),
        firstmove: 0x2a00_00bc,
        track: (0x2a00_00c6, 0x2a00_00c0),
        heroes_data: [1, 0x2a00_0007, 0x2a00_0008, 2, 0, 2, 0x2a00_0008, 0x2a00_0007, 1, 0, 0xb, 0x2a00_00b2, 0x2a00_0007, 0x2b, 2],
    },
    BuildConsts {
        follow: 0x2a00_0052,
        cutscene: 0x2a00_005a,
        death: 0x2a00_006a,
        heroes: (0x2a00_0055, 0x2a00_0056),
        firstmove: 0x2a00_010b,
        track: (0x2a00_0012, 0x2a00_000c),
        heroes_data: [1, 0x2a00_0055, 0x2a00_0056, 2, 0, 2, 0x2a00_0056, 0x2a00_0055, 1, 0, 0xb, 0x2a00_0101, 0x2a00_0055, 0x2b, 2],
    },
];

pub(crate) fn bc(e: &Engine) -> &'static BuildConsts {
    &BUILDS[e.rt.cur_build()]
}

pub static MODULE: ModuleDef = ModuleDef {
    name: "l05_camera",
    data_size: 0x1270,
    ctor: l05_cameraConstructor,
    dtor: l05_cameraDestructor,
    register: None,
    init: &[
        // CameraFlags.. fSpeedFactor
        (0x10168, &[0, 0, 0x3f99_999a, 0x7fff_ffff, 0x3f40_0000, 0]),
        // fCameraCurve[6][2] {hauteur, distance} + the 3 CheckWall ray dirs (0x101b0/c0/d0)
        (
            0x10180,
            &[
                0, 0x4026_6666, 0x3f80_0000, 0x4000_0000, 0x4000_0000, 0x3fd9_999a, //
                0x4040_0000, 0x4019_999a, 0x4080_0000, 0x4040_0000, 0x40a0_0000, 0x409c_cccd, //
                0, 0, 0x3f80_0000, 0, 0x3f5d_b22d, 0, 0xbf00_0000, 0, 0xbf5d_b22d, 0, 0xbf00_0000, 0,
            ],
        ),
        // Events[1] (code pointer, see EV_DEFAULT); pEventBuffer set by build_init; i32EventBufferSize 1
        (0x101f0, &[EV_DEFAULT, 0, 1]),
        // szFileName_172 = "ScoobyScreenShot00.bmp"
        (0x10200, &[0x6f6f_6353, 0x6353_7962, 0x6e65_6572, 0x746f_6853, 0x622e_3030, 0x0000_706d]),
    ],
    build_init: Some(l05_camera_build_init),
};

pub fn build_index(key: &str) -> Option<usize> {
    match key {
        "b76d2d71" => Some(0),
        "c87e3daa" => Some(1),
        _ => None,
    }
}

/// goHeroesData of the build (module image .text+0) and the relocated pointer pEventBuffer = &Events
fn l05_camera_build_init(m: &mut Mem, data: Ptr, build: usize) {
    for (i, v) in BUILDS[build].heroes_data.iter().enumerate() {
        m.w32(data + (goHeroesData - 0x10000) + 4 * i as u32, *v);
    }
    m.w32(data + (pEventBuffer - 0x10000), data + (Events - 0x10000));
}

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
pub const pEventBuffer: u32 = 0x101f4;
pub const i32EventBufferSize: u32 = 0x101f8;
/// szFileName.172 / uiNbPics.173 / pFloorAttribs.201
pub const szFileName: u32 = 0x10200;
pub const uiNbPics: u32 = 0x10218;
pub const pFloorAttribs: u32 = 0x10220;
pub const pAnim: u32 = 0x10240;
pub const pStdlib: u32 = 0x10244;
pub const pExtra: u32 = 0x10248;
pub const pGame: u32 = 0x1024c;
pub const pCamera: u32 = 0x10250;
pub const pGameObj: u32 = 0x10254;
pub const pInput: u32 = 0x10258;
pub const pPhysic: u32 = 0x1025c;
pub const pConsole: u32 = 0x10260;
pub const pHud: u32 = 0x10264;
pub const pMath: u32 = 0x10268;
pub const pSound: u32 = 0x1026c;
pub const pDebug: u32 = 0x10270;
pub const pMiscFunc: u32 = 0x10274;
pub const pApiCFunc: u32 = 0x10278;
pub const pText: u32 = 0x1027c;
pub const pLocalDataCameraBase: u32 = 0x10280;
pub const pGlobalDataCameraBase: u32 = 0x10284;
pub const oAttractorParam: u32 = 0x10288;
pub const cvCamImpulse: u32 = 0x102c8;
pub const cvCamImpulseWall: u32 = 0x102d8;
pub const cvLastFollowY: u32 = 0x102e8;
pub const cvTrackEnAvant: u32 = 0x102f8;
pub const fHauteurCameraFollow: u32 = 0x10304;
pub const fHauteurCameraFollowTarget: u32 = 0x10308;
pub const fDistanceCameraFollow: u32 = 0x1030c;
pub const fDistanceCameraFollowCollision: u32 = 0x10310;
pub const fDistanceCameraFollowTarget: u32 = 0x10314;
pub const fTargetCameraFollow: u32 = 0x10318;
pub const fTargetCameraFollowWork: u32 = 0x1031c;
pub const fOuvertureCaptureTarget: u32 = 0x10320;
pub const cvCamWallCheck: u32 = 0x10328;
pub const oEnvCollParams: u32 = 0x10338;
pub const i32ActualFloorEvent: u32 = 0x10370;
pub const i32FloorEventBufferSize: u32 = 0x10374;
pub const pEventCall: u32 = 0x10378;
pub const pLocalDataCameraEp6: u32 = 0x1037c;
pub const pGlobalDataCameraEp6: u32 = 0x10380;
pub const pLocalDataCamera: u32 = 0x10384;
pub const pGlobalDataCamera: u32 = 0x10388;
/// moCameraParams.102 (ManageEventDefault)
pub const moCameraParams_102: u32 = 0x103d0;
pub const oRayIntersectIn: u32 = 0x10420;
pub const oRayIntersectOut: u32 = 0x10450;
pub const ui32ActualTargetID: u32 = 0x1048c;
pub const cvCameraTourCenter: u32 = 0x10490;
pub const fCameraTourTimeIn: u32 = 0x1049c;

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

/// 0x00017ab8 callback 0xf (notify 0x14 = first EFGameLogicMgr::Update): camera init, team set-up
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

/// 0x000175d0 FloorEvent.c ManageEventDefault: l05 camera tracks the boss (msg 0x54 sub 3)
pub fn ManageEventDefault(e: &mut Engine, param_1: i32) -> LResult<()> {
    if param_1 == 1 {
        let (t0, t1) = bc(e).track;
        let p = e.alloca(0x10);
        APIgameobj::GetPosition(e, t0, 0x7fff_ffff, p);
        let m = G(e, moCameraParams_102);
        e.w32(m + 0x24, if 50.0 <= e.rf(p) { t0 } else { t1 });
        e.w32(m + 0x30, 0x4120_0000);
        e.w32(m + 0x34, 0x40d0_0000);
        e.w32(m + 0x38, 0x4040_0000);
        e.w32(m + 0x3c, 0);
        e.w32(m + 0x40, 0);
        e.w32(m + 0x44, 0x8000_4200);
        Camera_BaseFUNC_MessageHandler(e, 0x54, 3, m)?;
    }
    Ok(())
}

/// 0x0001e5b0
pub fn Ep6_CameraST_FIRSTMOVE(e: &mut Engine) -> LResult<u64> {
    APIinput::InputConfigSetState(e, 4, 0xe);
    let l = e.r32(GB0(e) + 8);
    let fm = bc(e).firstmove;
    APIgameobj::SendMessageTo(e, l, NULL, 0x97, fm, NULL);
    Camera_BaseST_FIRSTMOVE(e)?;
    Ok(0)
}

/// 0x0001e830 state 1
pub fn CameraST_FIRSTMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    Ep6_CameraST_FIRSTMOVE(e)?;
    Ok(0)
}

/// 0x0001e658 Ep6_CameraFUNC_MessageHandler
pub fn Ep6_CameraFUNC_MessageHandler(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    Camera_BaseFUNC_MessageHandler(e, param_1, param_2, param_3)
}

/// 0x0001e6f8 L05_camera.c CameraFUNC_MessageHandler
pub fn CameraFUNC_MessageHandler(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    Ep6_CameraFUNC_MessageHandler(e, param_1, param_2, param_3)
}

/// 0x000194d8
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

/// 0x00019ea0 camera physic: sphere 0.3, three CameraFlex (0x12) DMs (context 1/2/4), EnvColl (0x22)
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

/// 0x0001a1e8 view window, fade window, timers, floor events
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
    // ManageEventDefault(0) does nothing
    let _ = FloorEventConstructor(e, b, n);
}

/// calls the floor event function `f` (a word of the Events table) with state `st`
/// (0 init, 1 enter, 2 stay, 3 leave, 4 destroy, 5 force)
fn CallFloorEvent(e: &mut Engine, f: u32, st: i32) -> LResult<()> {
    match f {
        EV_DEFAULT => ManageEventDefault(e, st),
        _ => Ok(()),
    }
}

fn FloorEvent(e: &Engine, i: i32) -> u32 {
    e.r32(e.r32(G(e, pEventCall)) + (i as u32) * 4)
}

/// 0x000176d8 FloorEventManager: same event -> state 2; new event -> old state 3 (old -1 becomes 0),
/// new state 1
pub fn FloorEventManager(e: &mut Engine, param_1: i32) -> LResult<()> {
    if param_1 < e.ri32(G(e, i32FloorEventBufferSize)) {
        let act = e.ri32(G(e, i32ActualFloorEvent));
        if act == param_1 {
            let f = FloorEvent(e, act);
            if f != 0 {
                CallFloorEvent(e, f, 2)?;
            }
        } else {
            let mut act = act;
            if act == -1 {
                act = 0;
                let a = G(e, i32ActualFloorEvent);
                e.w32(a, 0);
            }
            let f = FloorEvent(e, act);
            if f != 0 {
                CallFloorEvent(e, f, 3)?;
            }
            let a = G(e, i32ActualFloorEvent);
            e.w32(a, param_1 as u32);
            let f = FloorEvent(e, param_1);
            if f != 0 {
                CallFloorEvent(e, f, 1)?;
            }
        }
    }
    Ok(())
}

/// 0x00017948 FloorEventForceState
pub fn FloorEventForceState(e: &mut Engine, param_1: i32) -> LResult<()> {
    let mut i = 0;
    while i < e.ri32(G(e, i32FloorEventBufferSize)) {
        let f = FloorEvent(e, i);
        if f != 0 {
            CallFloorEvent(e, f, param_1)?;
        }
        i += 1;
    }
    Ok(())
}

/// 0x0001e990 CameraFUNC_SetLocalParams
pub fn CameraFUNC_SetLocalParams(e: &mut Engine) {}

/// 0x00017868 FloorEventConstructor: runs every event's init (state 0)
pub fn FloorEventConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    let a = G(e, i32FloorEventBufferSize);
    e.w32(a, instanceCount as u32);
    let a = G(e, pEventCall);
    e.w32(a, api);
    let mut i = 0;
    while i < e.ri32(G(e, i32FloorEventBufferSize)) {
        let f = FloorEvent(e, i);
        if f != 0 {
            CallFloorEvent(e, f, 0)?;
        }
        i += 1;
    }
    let a = G(e, i32ActualFloorEvent);
    e.w32(a, 0xffff_ffff);
    Ok(())
}

/// 0x000179f8 FloorEventDestructor (module entry, no caller)
pub fn FloorEventDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    let mut i = 0;
    while i < e.ri32(G(e, i32FloorEventBufferSize)) {
        let f = FloorEvent(e, i);
        if f != 0 {
            CallFloorEvent(e, f, 4)?;
        }
        i += 1;
    }
    let a = G(e, i32ActualFloorEvent);
    e.w32(a, 0xffff_ffff);
    Ok(())
}

/// 0x0001dbb8
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

/// 0x0001e6c8
pub fn Ep6_CameraFUNC_Constructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    Camera_BaseFUNC_Constructor(e, api, instanceCount)?;
    let slot = G(e, pLocalDataCameraEp6);
    APIcons::GetLocalData(e, slot, 0x17c);
    if instanceCount == 1 {
        let slot = G(e, pGlobalDataCameraEp6);
        APIcons::GetGlobalData(e, slot, 0x20);
        let v = APIgameobj::GetGameBuffer(e, 0, 0x8f8, 1);
        let gd = e.r32(G(e, pGlobalDataCameraEp6));
        e.w32(gd, v);
    }
    APIcons::SetMessageFunction(e, Ep6_CameraFUNC_MessageHandler);
    Camera_BaseFUNC_Create_Physic(e);
    Camera_BaseFUNC_Create_Input(e);
    Camera_BaseFUNC_Create_Game(e);
    Ok(())
}

/// 0x0001e770
pub fn l05_cameraConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    Ep6_CameraFUNC_Constructor(e, api, instanceCount)?;
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

/// 0x0001e850 -> Ep6_CameraFUNC_Destructor 0x0001e7d8 -> Camera_BaseFUNC_Destructor
pub fn l05_cameraDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    Camera_BaseFUNC_Destructor(e, instanceCount);
    Ok(())
}

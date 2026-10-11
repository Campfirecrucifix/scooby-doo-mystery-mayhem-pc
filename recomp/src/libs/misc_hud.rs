//! MiscHUD custom library (`Game Logic Modules/Globals/Lib/mischud__01a68e40/MiscHUD.c`, 12 functions).
//! Registers the 9-slot table "MiscHUD" (0x24 bytes) used by the HUD panel: options volume bars,
//! language index, vibration / camera options, volume setters and game/episode buffer reset.
//! The other build (b6a2d2e3) is not in EP1L01; per PORT_NOTES all lib builds are the same code.
#![allow(non_snake_case, non_upper_case_globals, unused_variables)]
use super::set_global;
use crate::engine::core::apicons as cons;
use crate::engine::*;
use crate::logic::{LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "mischud",
    data_size: 0x18,
    ctor: mischudConstructor,
    dtor: mischudDestructor,
    register: Some(mischudRegister),
    init: &[],
    build_init: None,
};

pub fn build_index(key: &str) -> Option<usize> {
    matches!(key, "01a68e40" | "b6a2d2e3").then_some(0)
}

// module globals (_globals.h)
const pRegLib: u32 = 0x10000;
const pExtra: u32 = 0x10004;
const pGameObj: u32 = 0x10008;
const pSound: u32 = 0x1000c;
const pHud: u32 = 0x10010;
const pMiscHUD: u32 = 0x10014;

/// 0x00010cd0 options volume bar: stick x `param_1` > 0.33 raises the level `*param_3` by max/10
/// (snapped to `param_2` within 0.005), < -0.33 lowers it (snapped to 0 under 0.005); the bar window
/// `param_4` is widened/narrowed by 0.0675 (new width in `*param_5`, height `*param_6` = 0.1).
/// Returns 1 when the level changed.
pub fn miscHUDFUNC_VolumeBarUpdate(e: &mut Engine, param_1: f32, param_2: f32, param_3: Ptr, param_4: u32, param_5: Ptr, param_6: Ptr) -> u32 {
    let dims = e.alloca(8);
    e.wf(dims, 0.0);
    e.w32(param_6, 0x3dcc_cccd);
    e.wf(dims + 4, 0.1);
    if param_1 <= 0.33 || param_2 <= e.rf(param_3) {
        if -0.33 <= param_1 || e.rf(param_3) <= 0.01 {
            0
        } else {
            let mut v = e.rf(param_3) - param_2 / 10.0;
            e.wf(param_3, v);
            if v < 0.005 {
                v = 0.0;
                e.wf(param_3, v);
            }
            APIhud::WindowGetDims(e, param_4, dims, dims + 4);
            let w = e.rf(dims) + 0.0675;
            e.wf(param_5, w);
            let h = e.rf(param_6);
            APIhud::WindowSetDims(e, param_4, w, h);
            1
        }
    } else {
        let v = e.rf(param_3) + param_2 / 10.0;
        e.wf(param_3, v);
        if param_2 - 0.005 < v {
            e.wf(param_3, param_2);
        }
        APIhud::WindowGetDims(e, param_4, dims, dims + 4);
        let w = e.rf(dims) - 0.0675;
        e.wf(param_5, w);
        let h = e.rf(param_6);
        APIhud::WindowSetDims(e, param_4, w, h);
        1
    }
}

/// 0x00010f58 language code -> 0 (default), 1 (0x24), 2 (0x2a)
pub fn miscHUDFUNC_GetLanguage(e: &mut Engine, a0: u32) -> u32 {
    if a0 == 0x24 {
        1
    } else if a0 < 0x25 || a0 != 0x2a {
        0
    } else {
        2
    }
}

/// 0x00010fe8 toggles the vibration option `param_2+0x10` and applies it to pad `param_1+0x4c` through
/// the input API table `param_3` (+0x6c InputEnableVibration, +0x70 InputSetVibration on both motors).
pub fn miscHUDFUNC_SetOptionVibration(e: &mut Engine, a0: i32, a1: i32, a2: i32) {
    let (p1, p2) = (a0 as u32, a1 as u32);
    // every shipped caller passes pInput: dispatch through the APIinput trait
    assert_eq!(a2 as u32, cons::GetApiInput(e), "miscHUDFUNC_SetOptionVibration: table is not pInput");
    let v = (e.r32(p2 + 0x10) == 0) as u32;
    e.w32(p2 + 0x10, v);
    let pad = e.r32(p1 + 0x4c);
    APIinput::InputEnableVibration(e, pad as i32, v as i32);
    if e.r32(p2 + 0x10) != 0 {
        let pad = e.r32(p1 + 0x4c);
        APIinput::InputSetVibration(e, pad as i32, 0, 0.66, 0.2, 0.1, 0.2);
        let pad = e.r32(p1 + 0x4c);
        APIinput::InputSetVibration(e, pad as i32, 1, 0.66, 0.2, 0.1, 0.2);
    }
}

/// 0x00011100 toggles the camera option `param_1+0x14` and sends it (msg 0x1b) to camera `param_2`
pub fn miscHUDFUNC_SetOptionCamera(e: &mut Engine, a0: i32, a1: u32) {
    let p = a0 as u32;
    let v = (e.r32(p + 0x14) == 0) as u32;
    e.w32(p + 0x14, v);
    APIgameobj::SendMessageTo(e, a1, NULL, 0x1b, v, NULL);
}

/// 0x00011178 (the f32 arrives as raw bits through the table's u32 slot)
pub fn miscHUDFUNC_SetMusicVolume(e: &mut Engine, a0: u32) {
    APInewSound::SetVolumeByGroup(e, 0xa8, f32::from_bits(a0));
}

/// 0x000111c0
pub fn miscHUDFUNC_SetFXVolume(e: &mut Engine, a0: u32) {
    let v = f32::from_bits(a0);
    APInewSound::SetFXVolume(e, v);
    APInewSound::SetVolumeByGroup(e, 0x17, v);
}

/// 0x00011220
pub fn miscHUDFUNC_SetMoviesVolume(e: &mut Engine, a0: u32) {
    let v = f32::from_bits(a0);
    APInewSound::SetVolumeByGroup(e, 0x300, v);
    APInewSound::SetVideoVolume(e, v);
}

/// 0x00011280
pub fn miscHUDFUNC_InitGameBuffers(e: &mut Engine, a0: i32, a1: u32, a2: Ptr, a3: u32) {
    let p1 = a0 as u32;
    for o in [0x40, 0x68, 0x18, 0x98] {
        e.w32(p1 + o, 0);
    }
    for i in [10, 8, 0] {
        e.w32(a2 + i * 4, 0);
    }
    LIBMiscHUD::miscHUDFUNC_InitEpisodeBuffers(e, p1, a1, a2 as i32, a3);
}

/// 0x00011318
pub fn miscHUDFUNC_InitEpisodeBuffers(e: &mut Engine, a0: Ptr, a1: Ptr, a2: i32, a3: Ptr) {
    let (p1, p2, p3) = (a0, a1, a2 as u32);
    let w = |i: u32| p1 + i * 4;
    for i in 0..0xdu32 {
        e.w8(w(i * 2 + 0x1d1), 0);
        e.w32(w(i * 2 + 0x1d2), 0);
    }
    for i in [
        0x1eb, 0x1ed, 0x1ef, 0x1f0, 0x1ec, 0x1ee, 0x1f1, 0, 1, 2, 3, 4, 5, 7, 9, 10, 0xb, 0xc, 0xd, 0xe, 0x12, 0x19, 0x1d, 0x1e, 0x20,
        0x21, 0x22, 0x23, 0x1f2, 0x1f7, 500, 0x1f5, 0x1f6, 499,
    ] {
        e.w32(w(i), 0);
    }
    for i in 0..5u32 {
        e.w32(p2 + (i + 0x15) * 4, 0);
    }
    e.w32(p2, 0x3f80_0000);
    e.w32(p2 + 4, 0x4334_0000);
    for i in [2u32, 3, 4, 9, 0xf, 7, 0x11, 0xd, 0xb, 5, 0x13] {
        e.w32(p2 + i * 4, 0);
    }
    for i in 0..10u32 {
        for o in [0x2c, 0x30, 0x34] {
            e.w32(p3 + o + i * 0xc, 0);
        }
    }
    for o in [4, 8, 0xc, 0x10, 0x14, 0x18, 0x1c, 0x24] {
        e.w32(p3 + o, 0);
    }
    for i in 0..0xcu32 {
        for o in [0xa4, 0xac, 0xa8] {
            e.w32(p3 + o + i * 0xc, 0);
        }
    }
    e.w32(a3, 0);
    e.w32(a3 + 4, 0);
}

/// 0x000116f8
pub fn mischudRegister(e: &mut Engine, api: Ptr) {
    let tables: [(u32, fn(&mut Engine) -> Ptr); 5] = [
        (pRegLib, |e| cons::GetApiRegLib(e)),
        (pExtra, |e| cons::GetApiExtra(e)),
        (pGameObj, |e| cons::GetApiGameObj(e)),
        (pSound, |e| cons::GetApiNewSound(e)),
        (pHud, |e| cons::GetApiHUD(e)),
    ];
    for (a, f) in tables {
        let v = f(e);
        set_global(e, &MODULE, a, v);
    }
    let n = e.cstr("MiscHUD");
    let v = cons::GetApiCustom(e, n);
    set_global(e, &MODULE, pMiscHUD, v);
    // the table slots are filled with the functions above: the LIBMiscHUD trait dispatches to them
    let v = APIRegLib::RegisterLib(e, n, 0x24);
    set_global(e, &MODULE, pMiscHUD, v);
}

/// 0x000118a0
pub fn mischudConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    Ok(())
}

/// 0x000118d8
pub fn mischudDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    if instanceCount == 0 {
        let n = e.cstr("MiscHUD");
        APIRegLib::UnRegisterLib(e, n);
    }
    Ok(())
}

//! `panel` of the CREDITS level (build 73c662c1): credits roll. The text window (local +0x10)
//! scrolls up over 192 s (timer local +0x44, notify ext 5 at the end -> back to INTRO); start /
//! triangle skip to INTRO. GB0+0x44 = 0x22b unless it is 0x1bc (the "game finished" entry).
//! Messages 0x20d/0x20e pause/resume input, timer and window +0x14.
//! Port of `Game Logic Modules/Credits/Panel/panel__73c662c1/Panel.c` (11 functions).
#![allow(non_snake_case, non_upper_case_globals, unused_variables)]

use crate::engine::*;
use crate::logic::{LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "panel",
    // globals 0x10000..0x10038 (+ unused gpoLanguagesNamesAndISOCodes)
    data_size: 0x38,
    ctor: panelConstructor,
    dtor: panelDestructor,
    register: None,
    init: &[],
    build_init: None,
};

pub fn build_index(key: &str) -> Option<usize> {
    (key == "73c662c1").then_some(0)
}

pub const pPanelLocalData: u32 = 0x10000;
pub const pPanelGlobalData: u32 = 0x10004;
pub const pExtra: u32 = 0x10008;
pub const pGameObj: u32 = 0x1000c;
pub const pMath: u32 = 0x10010;
pub const pStdlib: u32 = 0x10014;
pub const pLight: u32 = 0x10018;
pub const pInput: u32 = 0x1001c;
pub const pConsole: u32 = 0x10020;
pub const pSound: u32 = 0x10024;
pub const pHud: u32 = 0x10028;
pub const pAnim: u32 = 0x1002c;
pub const pGame: u32 = 0x10030;
pub const pMiscFunc: u32 = 0x10034;

#[inline]
fn G(e: &Engine, a: u32) -> Ptr {
    e.rt.cur_data() + (a - 0x10000)
}
/// pPanelLocalData + byte offset
fn LD(e: &Engine, o: u32) -> Ptr {
    e.r32(G(e, pPanelLocalData)) + o
}
fn GB0(e: &Engine) -> Ptr {
    let g = e.r32(G(e, pPanelGlobalData));
    e.r32(g)
}
fn L(e: &Engine, o: u32) -> u32 {
    e.r32(LD(e, o))
}

/// 0x00010d08 state 0
pub fn PanelST_FIRSTLOGIC(e: &mut Engine, _: u32) -> LResult<u64> {
    let a = LD(e, 0x28);
    e.w32(a, 0);
    APInewSound::SetListener(e, 0x2a000004);
    PanelFUNC_Create_Windows(e);
    APInewSound::Play(e, SELF, 0x80000000, 0);
    let t = APIextra::TimerCreate(e);
    let a = LD(e, 0x44);
    e.w32(a, t);
    APIextra::TimerRegisterNotify(e, SELF, t, 5);
    APIextra::TimerSetFrequency(e, L(e, 0x44), 192.0);
    APIextra::TimerReset(e, L(e, 0x44));
    APIanim::Play(e, SELF, 2, 2, 0.0);
    let a = LD(e, 0x4c);
    e.w32(a, 1);
    APIinput::InputConfigActivate(e, 1, L(e, 0x4c) as i32);
    APIgameobj::SetNextState(e, SELF, 1);
    Ok(0)
}

/// 0x00010f50 state 1
pub fn PanelST_SECONDLOGIC(e: &mut Engine, _: u32) -> LResult<u64> {
    APIgameobj::SendMessageTo(e, 0x2a000015, NULL, 0x20b, 0, NULL);
    APIgameobj::SetNextState(e, SELF, 2);
    Ok(0)
}

/// 0x00010fd0 state 2: scroll the credits text
pub fn PanelST_BASICLOGIC(e: &mut Engine, _: u32) -> LResult<u64> {
    let fVar1 = APIextra::TimerGetTime(e, L(e, 0x44));
    APIhud::WindowSetPos(e, L(e, 0x10), 0.0, 1.0 - (fVar1 / 192.0) * 24.8);
    Ok(0)
}

/// 0x00011088 ext 6
pub fn PanelCB_TIMER_END(e: &mut Engine, param_1: u32) -> LResult<u64> {
    if param_1 == L(e, 0x48) {
        APIextra::TimerUnregisterNotify(e, SELF, L(e, 0x48));
    }
    Ok(0)
}

/// shared body of TIMER_SCROLL / INPUT_START / INPUT_TRIANGLE (three identical functions)
fn back_to_intro(e: &mut Engine) {
    let g = GB0(e);
    if e.r32(g + 0x44) != 0x1bc {
        e.w32(g + 0x44, 0x22b);
    }
    APInewSound::StopSound(e, SELF, 0x80000000);
    let s = e.cstr("INTRO");
    APIgame::LoadLevel(e, s, s, NULL);
}

/// 0x00011100 ext 5 (scroll timer done)
pub fn PanelCB_TIMER_SCROLL(e: &mut Engine, _: u32) -> LResult<u64> {
    back_to_intro(e);
    Ok(0)
}

/// 0x000111b0 ext 3
pub fn PanelCB_INPUT_START(e: &mut Engine, _: u32) -> LResult<u64> {
    back_to_intro(e);
    Ok(0)
}

/// 0x00011260 ext 4
pub fn PanelCB_INPUT_TRIANGLE(e: &mut Engine, _: u32) -> LResult<u64> {
    back_to_intro(e);
    Ok(0)
}

/// 0x00011310 input config 1: action 0x34 (start -> ext 3), 0x2b (triangle -> ext 4)
pub fn PanelFUNC_Create_Input(e: &mut Engine) {
    APIinput::InputConfigAdd(e, 1);
    APIinput::InputActionAdd(e, 1, 0x34, 0);
    let pad = e.r32(GB0(e) + 0x4c) as i32;
    APIinput::InputDeviceAdd(e, 1, 0x34, 0x32, pad, 0, 2, 0, 0xb);
    APIinput::InputDeviceAdd(e, 1, 0x34, 0x33, 1, 1, 2, 0, 1);
    APIinput::InputContextAdd(e, SELF, 3, 1, 0x34, 0x1c, 8);
    APIinput::InputContextSetState(e, 1, 0x34, 0x1c, 2);
    APIinput::InputActionAdd(e, 1, 0x2b, 0);
    let pad = e.r32(GB0(e) + 0x4c) as i32;
    APIinput::InputDeviceAdd(e, 1, 0x2b, 0x40, pad, 0, 2, 0, 3);
    APIinput::InputDeviceAdd(e, 1, 0x2b, 0x27, 1, 1, 2, 0, 0x39);
    APIinput::InputContextAdd(e, SELF, 4, 1, 0x2b, 0x12, 8);
    APIinput::InputContextSetState(e, 1, 0x2b, 0x12, 4);
    APIinput::InputConfigSetState(e, 1, 6);
}

fn add(e: &mut Engine, slot: u32, ty: u32, parent: u32, a2: u32, a3: u32) -> u32 {
    let w = APIhud::WindowAdd(e, ty, parent, a2, a3);
    let a = LD(e, slot);
    e.w32(a, w);
    w
}

/// 0x00011538
pub fn PanelFUNC_Create_Windows(e: &mut Engine) -> u32 {
    let w0 = add(e, 0, 0, 0xffffffff, 0xc, 0xff);
    APIhud::WindowSetDims(e, w0, 1.0, 1.0);
    APIhud::WindowResetFlags(e, w0, 0x100);
    let w1 = add(e, 4, 0, w0, 2, 9);
    APIhud::WindowSetPos(e, w1, 0.07, 0.05);
    APIhud::WindowSetDims(e, w1, 0.9, 0.9);
    APIhud::WindowResetFlags(e, w1, 0x100);
    let w3 = add(e, 0xc, 2, w0, 2, 10);
    APIhud::WindowSetPos(e, w3, 0.0, 0.0);
    APIhud::WindowSetDims(e, w3, 1.0, 1.0);
    let t = e.cstr("credits");
    APIhud::Window2DSetTexture(e, w3, t);
    let w = add(e, 8, 1, w1, 0x10001, 3);
    APIhud::WindowSetPos(e, w, 0.0, 0.0);
    APIhud::WindowSetDims(e, w, 0.9, 0.9);
    APIhud::TextWindowSetText(e, w, 0x138, &[6]);
    let w = add(e, 0x14, 1, w1, 0x90021, 3);
    APIhud::WindowSetPos(e, w, 0.0, 0.0);
    APIhud::WindowSetDims(e, w, 0.9, 0.9);
    if e.r32(GB0(e) + 0x44) == 0x1bc {
        APIhud::TextWindowSetText(e, w, 8, &[7]);
    } else {
        APIhud::TextWindowSetText(e, w, 4, &[7]);
    }
    // letterbox bars (+0x18/+0x1c) and their thin lines (+0x20/+0x24)
    for (slot, y, h, r, g, fl) in [
        (0x18, 0.0, 0.15, 0.0, 0.0, false),
        (0x1c, 0.85, 0.15, 0.0, 0.0, false),
        (0x20, 0.15, 0.005, 1.0, 0.25, true),
        (0x24, 0.845, 0.005, 1.0, 0.25, true),
    ] {
        let w = add(e, slot, 1, w3, 0x800, 8);
        APIhud::WindowSetPos(e, w, 0.0, y);
        APIhud::WindowSetDims(e, w, 1.0, h);
        APIhud::WindowSetBackGround(e, w, 1, r, g);
        if fl {
            APIhud::WindowSetFlags(e, w, 0x100);
        }
    }
    let w = add(e, 0x10, 1, w0, 0x30000, 9);
    APIhud::WindowSetPos(e, w, 0.0, 1.0);
    APIhud::WindowSetDims(e, w, 1.0, 24.0);
    APIhud::TextWindowSetFont(e, w, 0);
    APIhud::TextWindowSetColor(e, w, 2);
    APIhud::TextWindowSetText(e, w, 0x13a, &[7])
}

/// 0x00011db8 0x20d pause / 0x20e resume
pub fn PanelCB_Messages(e: &mut Engine, param_1: u32, _: u32, _: Ptr) -> LResult<u64> {
    if param_1 == 0x20d {
        APIinput::InputConfigActivate(e, 1, 0);
        APIextra::TimerPause(e, L(e, 0x44), 1);
        APIhud::WindowSetFlags(e, L(e, 0x14), 0x100);
    } else if 0x20d < param_1 && param_1 == 0x20e {
        APIinput::InputConfigActivate(e, 1, L(e, 0x4c) as i32);
        APIextra::TimerPause(e, L(e, 0x44), 0);
        APIhud::WindowResetFlags(e, L(e, 0x14), 0x100);
    }
    Ok(0)
}

/// 0x00011f30
pub fn panelConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    if instanceCount == 1 {
        APIcons::SetExternalFunction(e, 0, PanelST_FIRSTLOGIC);
        APIcons::SetExternalFunction(e, 1, PanelST_SECONDLOGIC);
        APIcons::SetExternalFunction(e, 2, PanelST_BASICLOGIC);
        APIcons::SetExternalFunction(e, 3, PanelCB_INPUT_START);
        APIcons::SetExternalFunction(e, 4, PanelCB_INPUT_TRIANGLE);
        APIcons::SetExternalFunction(e, 6, PanelCB_TIMER_END);
        APIcons::SetExternalFunction(e, 5, PanelCB_TIMER_SCROLL);
        for (a, v) in [
            (pAnim, APIcons::GetApiAnim(e)),
            (pGameObj, APIcons::GetApiGameObj(e)),
            (pExtra, APIcons::GetApiExtra(e)),
            (pInput, APIcons::GetApiInput(e)),
            (pSound, APIcons::GetApiNewSound(e)),
            (pStdlib, APIcons::GetApiStdlib(e)),
            (pMath, APIcons::GetApiMath(e)),
            (pLight, APIcons::GetApiLight(e)),
            (pConsole, APIcons::GetApiConsole(e)),
            (pHud, APIcons::GetApiHUD(e)),
            (pGame, APIcons::GetApiGame(e)),
        ] {
            let a = G(e, a);
            e.w32(a, v);
        }
        let n = e.cstr("MiscFunc");
        let v = APIcons::GetApiCustom(e, n);
        let a = G(e, pMiscFunc);
        e.w32(a, v);
        let slot = G(e, pPanelGlobalData);
        APIcons::GetGlobalData(e, slot, 8);
        let b = APIgameobj::GetGameBuffer(e, 0, 0x8f8, 1);
        let gd = e.r32(G(e, pPanelGlobalData));
        e.w32(gd, b);
        let b = APIgameobj::GetGameBuffer(e, 3, 0x40, 1);
        let gd = e.r32(G(e, pPanelGlobalData));
        e.w32(gd + 4, b);
    }
    APIcons::SetMessageFunction(e, PanelCB_Messages);
    let slot = G(e, pPanelLocalData);
    APIcons::GetLocalData(e, slot, 0x50);
    LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 2, NULL, 0);
    PanelFUNC_Create_Input(e);
    APIcons::SetInitialState(e, 0);
    Ok(())
}

/// 0x00012258
pub fn panelDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    APIhud::WindowDelete(e, L(e, 0));
    APIextra::TimerDestroy(e, L(e, 0x44));
    if L(e, 0x28) != 0 {
        APIlight::DestroyLight(e, L(e, 0x28));
    }
    if instanceCount == 0 {
        APIgameobj::FreeGameBuffer(e, 0);
    }
    Ok(())
}

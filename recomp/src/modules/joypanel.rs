//! `joypanel`: the "controller disconnected" panel. Local +0 event bits (1 activate, 2 deactivate,
//! 0x400 controller out, 0x800 start), +4 fullscreen window, +8 text window, +0xc active.
//! States 0 FIRST, 1 ACTIVE, 2 INACTIVE, 3 NOCONTROLLER. The 6 builds differ only in the object id
//! told 0x210 on (de)activation ([`PANEL`]).
//! Port of `Game Logic Modules/Globals/Panel/JoyPanel/joypanel__*/JoyPanel.c` (14 functions).
#![allow(non_snake_case, non_upper_case_globals, unused_variables)]

use crate::engine::*;
use crate::logic::{LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "joypanel",
    // globals 0x10000..0x10020 (+ unused gpoLanguagesNamesAndISOCodes)
    data_size: 0x20,
    ctor: joypanelConstructor,
    dtor: joypanelDestructor,
    register: None,
    init: &[],
    build_init: None,
};

const KEYS: [&str; 6] = ["514d037c", "24ba5d25", "b64facac", "3e615eec", "649957d6", "6d749458"];
/// SendMessageTo(.., 0x210, on/off) target per build (same order as [`KEYS`])
const PANEL: [u32; 6] = [0x2a100007, 0x2a100000, 0x2a100006, 0x2a10000c, 0x2a10001f, 0x2a000005];

pub fn build_index(key: &str) -> Option<usize> {
    KEYS.iter().position(|k| *k == key)
}

pub const pLocalData_JoyPanel: u32 = 0x10000;
pub const pGlobalData_JoyPanel: u32 = 0x10004;
pub const pExtra: u32 = 0x10008;
pub const pGameObj: u32 = 0x1000c;
pub const pHud: u32 = 0x10010;
pub const pInput: u32 = 0x10014;
pub const pMisc: u32 = 0x10018;
pub const pMiscHUD: u32 = 0x1001c;

#[inline]
fn G(e: &Engine, a: u32) -> Ptr {
    e.rt.cur_data() + (a - 0x10000)
}
fn LD(e: &Engine, o: u32) -> Ptr {
    e.r32(G(e, pLocalData_JoyPanel)) + o
}
fn L(e: &Engine, o: u32) -> u32 {
    e.r32(LD(e, o))
}
fn W(e: &mut Engine, o: u32, v: u32) {
    let a = LD(e, o);
    e.w32(a, v);
}
fn GB(e: &Engine, i: u32) -> Ptr {
    let g = e.r32(G(e, pGlobalData_JoyPanel));
    e.r32(g + i * 4)
}
fn pad(e: &Engine) -> u32 {
    e.r32(GB(e, 0) + 0x4c)
}
fn ev(e: &mut Engine, bits: u32) {
    let v = L(e, 0) | bits;
    W(e, 0, v);
}
fn clear_events(e: &mut Engine) {
    let v = L(e, 0) & 0xfffff000;
    W(e, 0, v);
}
fn panel(e: &Engine) -> u32 {
    PANEL[e.rt.cur_build()]
}

pub fn JoyPanelST_FIRST(e: &mut Engine, _: u32) -> LResult<u64> {
    APIgameobj::ResetLogicFlag(e, SELF, 0xe3a);
    JoyPanelFUNC_Init(e);
    clear_events(e);
    APIgameobj::SetNextState(e, SELF, 2);
    Ok(0)
}
pub fn JoyPanelST_INACTIVE(e: &mut Engine, _: u32) -> LResult<u64> {
    if L(e, 0) & 1 != 0 {
        JoyPanelFUNC_Activate(e);
    }
    clear_events(e);
    Ok(0)
}
pub fn JoyPanelST_ACTIVE(e: &mut Engine, _: u32) -> LResult<u64> {
    if L(e, 0) & 2 != 0 {
        JoyPanelFUNC_Deactivate(e);
    } else if L(e, 0) & 0x400 != 0 && APIhud::WindowGetFlags(e, L(e, 4)) & 0x100 != 0 {
        JoyPanelFUNC_Controller_Out(e);
    }
    clear_events(e);
    Ok(0)
}
pub fn JoyPanelST_NOCONTROLLER(e: &mut Engine, _: u32) -> LResult<u64> {
    if L(e, 0) & 2 != 0 {
        JoyPanelFUNC_Deactivate(e);
    } else if L(e, 0) & 0x400 == 0 && APIhud::WindowGetFlags(e, L(e, 4)) & 0x100 == 0 {
        JoyPanelFUNC_Controller_In(e);
    }
    clear_events(e);
    Ok(0)
}
pub fn JoyPanelCB_MessageHandler(e: &mut Engine, m: u32, p: u32, _: Ptr) -> LResult<u64> {
    match m {
        0x20b => ev(e, 1),
        0x20c => ev(e, 2),
        0x20f => {
            let r = if APIgameobj::GetCurrentStateID(e, SELF) == 3 { 0x20d } else { 0x20e };
            APIgameobj::SendMessageTo(e, p, NULL, r, 0, NULL);
        }
        _ => {}
    }
    Ok(0)
}
pub fn JoyPanelCB_START(e: &mut Engine, _: u32) -> LResult<u64> {
    ev(e, 0x800);
    Ok(0)
}
pub fn JoyPanelCB_INPUT_DEBUG(e: &mut Engine, _: u32) -> LResult<u64> {
    if L(e, 0xc) == 1 {
        ev(e, 0x400);
    }
    Ok(0)
}
pub fn JoyPanelCB_CONTROLLER_OUT(e: &mut Engine, p: u32) -> LResult<u64> {
    if L(e, 0xc) == 1 && p == pad(e) {
        ev(e, 0x400);
    }
    Ok(0)
}
fn JoyPanelFUNC_Activate(e: &mut Engine) {
    W(e, 0xc, 1);
    let t = panel(e);
    APIgameobj::SendMessageTo(e, t, NULL, 0x210, 1, NULL);
    let p = pad(e);
    APIinput::InputErrorRegisterNotify(e, p as i32, SELF, 4);
    APIgameobj::SetNextState(e, SELF, 1);
}
fn JoyPanelFUNC_Deactivate(e: &mut Engine) {
    W(e, 0xc, 0);
    if APIhud::WindowGetFlags(e, L(e, 8)) & 0x100 != 0 {
        JoyPanelFUNC_Controller_Out(e);
    }
    let t = panel(e);
    APIgameobj::SendMessageTo(e, t, NULL, 0x210, 0, NULL);
    APIgameobj::SetNextState(e, SELF, 2);
}
fn text_id(e: &Engine) -> u32 {
    if e.r32(GB(e, 2) + 0x20) == 0 { 0x55 } else { 0x56 }
}
fn send_all(e: &mut Engine, m: u32) {
    let r = e.alloca(0x10);
    e.w32(r, 1);
    APIgameobj::SendMessageTo(e, 0xff000001, r, m, 0, NULL);
}
fn JoyPanelFUNC_Controller_Out(e: &mut Engine) {
    let t = text_id(e);
    let p = pad(e);
    let arg = match p {
        0x10..=0x13 => p - 0xf,
        _ => p,
    };
    APIhud::TextWindowSetText(e, L(e, 8), t, &[arg]);
    APIhud::WindowResetFlags(e, L(e, 4), 0x100);
    send_all(e, 0x20d);
    APIgameobj::SetNextState(e, SELF, 3);
}
fn JoyPanelFUNC_Controller_In(e: &mut Engine) {
    APIhud::WindowSetFlags(e, L(e, 4), 0x100);
    send_all(e, 0x20e);
    APIgameobj::SetNextState(e, SELF, 1);
}
fn JoyPanelFUNC_Init(e: &mut Engine) {
    W(e, 0xc, 0);
    let g = GB(e, 0);
    if e.r32(g + 0x4c) == 0 {
        e.w32(g + 0x4c, 0x10);
    }
    APIinput::InputConfigActivate(e, 0x1e, 0);
    APIinput::InputConfigSetState(e, 0x1e, 0x1e);
    let w = APIhud::WindowAdd(e, 1, 0xffffffff, 0, 1);
    W(e, 4, w);
    APIhud::WindowSetDims(e, w, 1.0, 1.0);
    APIhud::WindowSetPos(e, w, 0.0, 0.0);
    APIhud::WindowSetFlags(e, w, 0x100);
    let t = APIhud::WindowAdd(e, 1, w, 0x3080c, 1);
    W(e, 8, t);
    APIhud::WindowSetDims(e, t, 0.5, 0.55);
    APIhud::WindowSetPos(e, t, 0.0, 0.0);
    let id = text_id(e);
    APIhud::TextWindowSetText(e, t, id, &[1]);
    APIhud::TextWindowSetColor(e, t, 6);
    APIhud::WindowSetBackGround(e, t, 1, 0.7, 0.4);
}

pub fn joypanelConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    if instanceCount == 1 {
        APIcons::SetExternalFunction(e, 0, JoyPanelST_FIRST);
        APIcons::SetExternalFunction(e, 2, JoyPanelST_INACTIVE);
        APIcons::SetExternalFunction(e, 1, JoyPanelST_ACTIVE);
        APIcons::SetExternalFunction(e, 3, JoyPanelST_NOCONTROLLER);
        APIcons::SetExternalFunction(e, 5, JoyPanelCB_START);
        APIcons::SetExternalFunction(e, 4, JoyPanelCB_CONTROLLER_OUT);
        APIcons::SetExternalFunction(e, 6, JoyPanelCB_INPUT_DEBUG);
        for (a, v) in [
            (pGameObj, APIcons::GetApiGameObj(e)),
            (pExtra, APIcons::GetApiExtra(e)),
            (pHud, APIcons::GetApiHUD(e)),
            (pInput, APIcons::GetApiInput(e)),
        ] {
            let a = G(e, a);
            e.w32(a, v);
        }
        for (a, n) in [(pMisc, "MiscFunc"), (pMiscHUD, "MiscHUD")] {
            let s = e.cstr(n);
            let v = APIcons::GetApiCustom(e, s);
            let a = G(e, a);
            e.w32(a, v);
        }
        let slot = G(e, pGlobalData_JoyPanel);
        APIcons::GetGlobalData(e, slot, 0x18);
        let gd = e.r32(slot);
        let b = APIgameobj::GetGameBuffer(e, 0, 0x8f8, 1);
        e.w32(gd, b);
        let b = APIgameobj::GetGameBuffer(e, 3, 0x40, 1);
        e.w32(gd + 8, b);
    }
    let slot = G(e, pLocalData_JoyPanel);
    APIcons::GetLocalData(e, slot, 0x10);
    APIcons::SetMessageFunction(e, JoyPanelCB_MessageHandler);
    LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 2, NULL, 0);
    APIcons::SetInitialState(e, 0);
    Ok(())
}
pub fn joypanelDestructor(e: &mut Engine, _: i32) -> LResult<()> {
    let w = L(e, 4);
    APIhud::WindowDelete(e, w);
    Ok(())
}

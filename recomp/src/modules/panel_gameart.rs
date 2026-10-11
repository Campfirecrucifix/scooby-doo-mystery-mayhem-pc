//! `panel` of the GAMEART levels (ART01..ART24, one build ccc02357): game-art viewer. Shows the
//! art image of GB0+0x44 (1..24) or "LockedImage_Sample" + the lock text when its unlock bit
//! (table +0xc) is not in GB2+0x20; left/right loads the previous/next ART level, triangle goes
//! back to INTRO (GB0+0x44 = 999). Messages 0x20d/0x20e hide/show the panel and its input.
//! Port of `Game Logic Modules/TITLE/Panel/GameArts/panel__ccc02357/Panel.c` (9 functions).
//! Quirk kept: the constructor registers PanelCB_NOINPUT (0x13) and PanelCB_TIMER_CONTROLLER
//! (0x14), which are undefined symbols of this object: EFLogicCode::GetExternalSymbol resolves
//! them to NULL, so the slots exist but are empty.
#![allow(non_snake_case, non_upper_case_globals, unused_variables)]

use crate::engine::*;
use crate::logic::{LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "panel",
    // globals 0x10000..0x10204 (+ unused gpoLanguagesNamesAndISOCodes)
    data_size: 0x208,
    ctor: panelConstructor,
    dtor: panelDestructor,
    register: None,
    init: &[],
    build_init: Some(build_init),
};

pub fn build_index(key: &str) -> Option<usize> {
    (key == "ccc02357").then_some(0)
}

pub const oGameArtTable: u32 = 0x10000;
pub const pPanelLocalData: u32 = 0x101e0;
pub const pPanelGlobalData: u32 = 0x101e4;
pub const pAnim: u32 = 0x101e8;
pub const pExtra: u32 = 0x101ec;
pub const pGame: u32 = 0x101f0;
pub const pGameObj: u32 = 0x101f4;
pub const pHud: u32 = 0x101f8;
pub const pInput: u32 = 0x101fc;
pub const pSound: u32 = 0x10200;

const ART_BITS: [u32; 24] = [
    0, 0, 0, 0, 1, 4, 8, 0x10, 0x40, 0x80, 0x100, 0x200, 0x400, 0x800, 0x2000, 0x4000, 0x8000, 0x10000, 0x20000,
    0x80000, 0x100000, 0x200000, 0x800000, 0x1000000,
];

/// oGameArtTable[24] (0x14 bytes each): level dir, level name, image, unlock bit, lock text id
fn build_init(m: &mut Mem, data: Ptr, _: usize) {
    for i in 1..=24u32 {
        let a = data + (i - 1) * 0x14;
        let bit = ART_BITS[i as usize - 1];
        let txt = [0, 0x177, 0x178, 0x179, 0x17a, 0x17b][(i as usize - 1) / 4];
        let s = [format!("GAMEART\\ART{i:02}"), format!("ART{i:02}"), format!("Image{i:02}_Sample")];
        for (j, s) in s.iter().enumerate() {
            let p = m.cstr(s);
            m.w32(a + 4 * j as u32, p);
        }
        m.w32(a + 0xc, bit);
        m.w32(a + 0x10, txt);
    }
}

#[inline]
fn G(e: &Engine, a: u32) -> Ptr {
    e.rt.cur_data() + (a - 0x10000)
}
/// pPanelLocalData + byte offset
fn LD(e: &Engine, o: u32) -> Ptr {
    e.r32(G(e, pPanelLocalData)) + o
}
/// *pPanelGlobalData (GB0)
fn GB0(e: &Engine) -> Ptr {
    let g = e.r32(G(e, pPanelGlobalData));
    e.r32(g)
}
/// oGameArtTable entry of the current art (GB0+0x44, 1-based)
fn Art(e: &Engine) -> Ptr {
    let n = e.r32(GB0(e) + 0x44);
    G(e, oGameArtTable).wrapping_add(n.wrapping_sub(1).wrapping_mul(0x14))
}

/// SetExternalFunction(i, NULL): undefined symbol resolved to 0 by EFLogicCode::LoadFile
fn SetExternalFunctionNull(e: &mut Engine, i: usize) {
    let c = e.rt.current_code().expect("SetExternalFunction outside logic");
    let f = &mut e.rt.codes[c].funcs;
    if f.len() <= i {
        f.resize(i + 1, None);
    }
    f[i] = None;
}

fn win(e: &mut Engine, slot: u32, ty: u32, parent: u32, a2: u32, a3: u32) -> u32 {
    let w = APIhud::WindowAdd(e, ty, parent, a2, a3);
    let a = LD(e, slot);
    e.w32(a, w);
    w
}
fn name(e: &mut Engine, w: u32, s: &str) {
    let p = e.cstr(s);
    APIhud::WindowSetName(e, w, p);
}

/// 0x00011320 state 0: windows, art image or locked image + lock text
pub fn PanelST_FIRSTLOGIC(e: &mut Engine, _: u32) -> LResult<u64> {
    let piVar1 = GB0(e) + 0x44;
    APIanim::Play(e, SELF, 3, 2, 0.0);
    APIanim::Play(e, SELF, 4, 2, 0.0);
    let w0 = win(e, 0, 0, 0xffffffff, 0xc, 10);
    APIhud::WindowSetDims(e, w0, 1.0, 1.0);
    APIhud::WindowSetPos(e, w0, 0.0, 0.0);
    name(e, w0, "uiMainWin");
    let w = win(e, 4, 2, w0, 1, 6);
    APIhud::WindowSetDims(e, w, 1.0, 1.0);
    APIhud::WindowSetPos(e, w, 0.0, 0.0);
    name(e, w, "uiWindowsImage");
    let w = win(e, 0x10, 1, w0, 0x30001, 3);
    APIhud::TextWindowSetText(e, w, 0x177, &[0]);
    APIhud::WindowSetDims(e, w, 0.5, 0.35);
    APIhud::WindowSetPos(e, w, 0.25, 0.5);
    APIhud::WindowSetFlags(e, w, 0x100);
    name(e, w, "uiWinLockedArtsText");
    let w = win(e, 0x14, 1, w0, 0x60001, 3);
    APIhud::TextWindowSetText(e, w, 0xf4, &[0]);
    APIhud::WindowSetDims(e, w, 0.1, 0.2);
    APIhud::WindowSetPos(e, w, 0.56, 0.815);
    name(e, w, "Left");
    let w = win(e, 0x18, 1, w0, 0x20001, 3);
    APIhud::TextWindowSetText(e, w, 0xf5, &[0]);
    APIhud::WindowSetDims(e, w, 0.1, 0.2);
    APIhud::WindowSetPos(e, w, 0.83, 0.815);
    name(e, w, "Right");
    let w = win(e, 0x1c, 1, w0, 0x30001, 3);
    let n = e.r32(GB0(e) + 0x44);
    APIhud::TextWindowSetText(e, w, 0x1a5, &[n]);
    APIhud::WindowSetDims(e, w, 0.25, 0.2);
    APIhud::WindowSetPos(e, w, 0.615, 0.81);
    name(e, w, "Change");
    let w = win(e, 0x20, 1, w0, 0x20001, 3);
    APIhud::TextWindowSetText(e, w, 4, &[8]);
    APIhud::WindowSetDims(e, w, 0.4, 0.2);
    APIhud::WindowSetPos(e, w, 0.17, 0.81);
    name(e, w, "Back");
    let f = e.cstr("GAME ART: %d, %d, %d, %s \n");
    let n = e.r32(piVar1);
    APIextra::null_printf(e, f, n);
    let art = Art(e);
    let bit = e.r32(art + 0xc);
    let gb2 = e.r32(e.r32(G(e, pPanelGlobalData)) + 4);
    if bit == 0 || e.r32(gb2 + 0x20) & bit != 0 {
        let w = e.r32(LD(e, 4));
        let img = e.r32(art + 8);
        APIhud::Window2DSetTexture(e, w, img);
    } else {
        let w = e.r32(LD(e, 0x10));
        let t = e.r32(art + 0x10);
        APIhud::TextWindowSetText(e, w, t, &[0]);
        APIhud::WindowResetFlags(e, w, 0x100);
        let w = e.r32(LD(e, 4));
        let img = e.cstr("LockedImage_Sample");
        APIhud::Window2DSetTexture(e, w, img);
    }
    APIgameobj::SetNextState(e, SELF, 1);
    Ok(0)
}

/// 0x00011c18 state 1
pub fn PanelST_SECONDLOGIC(e: &mut Engine, _: u32) -> LResult<u64> {
    APIgameobj::SendMessageTo(e, 0x2a000093, NULL, 0x20b, 0, NULL);
    APIinput::InputConfigActivate(e, 3000, 1);
    APIgameobj::SetNextState(e, SELF, 2);
    Ok(0)
}

/// 0x00011cb8 state 2
pub fn PanelST_BASICLOGIC(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

/// 0x00011ce8 back to the title
pub fn PanelCB_INPUT_TRIANGLE(e: &mut Engine, _: u32) -> LResult<u64> {
    let g = GB0(e);
    e.w32(g + 0x44, 999);
    APIinput::InputConfigActivate(e, 3000, 0);
    APInewSound::Play(e, SELF, 0x40000002, 0);
    let s = e.cstr("INTRO");
    APIgame::LoadLevel(e, s, s, NULL);
    Ok(0)
}

/// 0x00011da0 previous / next art level (1..24, wrapping)
pub fn PanelCB_INPUT_LR(e: &mut Engine, param_1: u32) -> LResult<u64> {
    let iVar1 = GB0(e);
    let f = e.cstr("INPUT LR \n");
    APIextra::null_printf(e, f, 0);
    APInewSound::Play(e, SELF, 0x40000001, 0);
    if APIgameobj::GetCurrentStateID(e, SELF) == 2 {
        let x = e.rf(param_1);
        let g = GB0(e);
        if x < -0.15 {
            let n = e.r32(g + 0x44).wrapping_sub(1);
            e.w32(g + 0x44, n);
            if n == 0 {
                e.w32(g + 0x44, 0x18);
            }
        } else if -0.15 < x {
            let n = e.r32(g + 0x44).wrapping_add(1);
            e.w32(g + 0x44, n);
            if 0x18 < n {
                e.w32(g + 0x44, 1);
            }
        }
        APIinput::InputConfigActivate(e, 3000, 0);
        let art = G(e, oGameArtTable).wrapping_add(e.r32(iVar1 + 0x44).wrapping_sub(1).wrapping_mul(0x14));
        let (dir, lvl) = (e.r32(art), e.r32(art + 4));
        APIgame::LoadLevel(e, dir, lvl, NULL);
    }
    Ok(1)
}

/// 0x00011fd8 input config 3000: action 0x3c (triangle -> cb 10), 0x40 (left/right -> cb 5)
pub fn PanelFUNC_Create_Input(e: &mut Engine) {
    let pad = e.r32(GB0(e) + 0x4c) as i32;
    APIinput::InputConfigAdd(e, 3000);
    APIinput::InputActionAdd(e, 3000, 0x3c, 0);
    APIinput::InputDeviceAdd(e, 3000, 0x3c, 0x26, pad, 0, 2, 0, 3);
    APIinput::InputDeviceAdd(e, 3000, 0x3c, 0x27, 1, 1, 2, 0, 1);
    APIinput::InputContextAdd(e, SELF, 10, 0xbb9, 0x3c, 0x1e, 8);
    APIinput::InputContextAdd(e, SELF, 10, 3000, 0x3c, 0x1e, 8);
    APIinput::InputContextSetState(e, 3000, 0x3c, 0x1e, 0x100);
    APIinput::InputContextSetState(e, 3000, 0x3c, 0x1e, 0x80);
    APIinput::InputActionAdd(e, 3000, 0x40, 0);
    let pad = e.r32(GB0(e) + 0x4c) as i32;
    APIinput::InputDeviceAdd(e, 3000, 0x40, 3, pad, 0, 0, 0, 0x8000);
    APIinput::InputDeviceAdd(e, 3000, 0x40, 0xb, pad, 1, 0, -100, 0xe);
    APIinput::InputDeviceAdd(e, 3000, 0x40, 0xc, pad, 1, 0, 100, 0xf);
    APIinput::InputDeviceAdd(e, 3000, 0x40, 0xb, 1, 2, 0, -100, 0xcb);
    APIinput::InputDeviceAdd(e, 3000, 0x40, 0xc, 1, 2, 0, 100, 0xcd);
    APIinput::InputContextAdd(e, SELF, 5, 3000, 0x40, 0x2b, 0xc00000);
    APIinput::InputContextSetState(e, 3000, 0x40, 0x2b, 0x20);
    APIinput::InputConfigSetState(e, 3000, 0xf2);
    APIinput::InputConfigActivate(e, 3000, 0);
}

/// 0x00012320 0x20d hide / 0x20e show
pub fn PanelFUNC_Messages(e: &mut Engine, param_1: u32, _: u32, _: Ptr) -> LResult<u64> {
    if param_1 == 0x20d {
        APIinput::InputConfigActivate(e, 3000, 0);
        let w = e.r32(LD(e, 0));
        APIhud::WindowSetFlags(e, w, 0x100);
    } else if param_1 == 0x20e {
        APIinput::InputConfigActivate(e, 3000, 1);
        let w = e.r32(LD(e, 0));
        APIhud::WindowResetFlags(e, w, 0x100);
    }
    Ok(0)
}

/// 0x00012418
pub fn panelConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    if instanceCount == 1 {
        APIcons::SetExternalFunction(e, 0, PanelST_FIRSTLOGIC);
        APIcons::SetExternalFunction(e, 1, PanelST_SECONDLOGIC);
        APIcons::SetExternalFunction(e, 2, PanelST_BASICLOGIC);
        APIcons::SetExternalFunction(e, 5, PanelCB_INPUT_LR);
        APIcons::SetExternalFunction(e, 10, PanelCB_INPUT_TRIANGLE);
        SetExternalFunctionNull(e, 0x13); // PanelCB_NOINPUT (undefined)
        SetExternalFunctionNull(e, 0x14); // PanelCB_TIMER_CONTROLLER (undefined)
        for (a, v) in [
            (pAnim, APIcons::GetApiAnim(e)),
            (pExtra, APIcons::GetApiExtra(e)),
            (pGame, APIcons::GetApiGame(e)),
            (pGameObj, APIcons::GetApiGameObj(e)),
            (pHud, APIcons::GetApiHUD(e)),
            (pInput, APIcons::GetApiInput(e)),
            (pSound, APIcons::GetApiNewSound(e)),
        ] {
            let a = G(e, a);
            e.w32(a, v);
        }
        let slot = G(e, pPanelGlobalData);
        APIcons::GetGlobalData(e, slot, 8);
        let b = APIgameobj::GetGameBuffer(e, 0, 0x8f8, 1);
        let gd = e.r32(G(e, pPanelGlobalData));
        e.w32(gd, b);
        let b = APIgameobj::GetGameBuffer(e, 2, 0x158, 1);
        let gd = e.r32(G(e, pPanelGlobalData));
        e.w32(gd + 4, b);
    }
    APIcons::SetMessageFunction(e, PanelFUNC_Messages);
    let slot = G(e, pPanelLocalData);
    APIcons::GetLocalData(e, slot, 0x38);
    let g = GB0(e);
    if e.r32(g + 0x4c) == 0 {
        e.w32(g + 0x4c, 0x10);
    }
    let t = APIextra::TimerCreate(e);
    let a = LD(e, 0x30);
    e.w32(a, t);
    let t = APIextra::TimerCreate(e);
    let a = LD(e, 0x2c);
    e.w32(a, t);
    let t = e.r32(LD(e, 0x30));
    APIextra::TimerReset(e, t);
    let t = e.r32(LD(e, 0x2c));
    APIextra::TimerReset(e, t);
    PanelFUNC_Create_Input(e);
    let g = GB0(e);
    if e.r32(g + 0x44) == 0 {
        e.w32(g + 0x44, 1);
    } else if 0x18 < e.r32(g + 0x44) {
        e.w32(g + 0x44, 0x18);
    }
    APIcons::SetInitialState(e, 0);
    Ok(())
}

/// 0x000127a8
pub fn panelDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    if instanceCount == 0 {
        APIgameobj::FreeGameBuffer(e, 0);
    }
    let t = e.r32(LD(e, 0x30));
    APIextra::TimerDestroy(e, t);
    let t = e.r32(LD(e, 0x2c));
    APIextra::TimerDestroy(e, t);
    Ok(())
}

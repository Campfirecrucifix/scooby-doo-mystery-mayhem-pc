//! ApiText custom library (`Game Logic Modules/Globals/Lib/apitext__ac7174a1/ApiText.c`, 13 functions).
//! Both builds (89583368, ac7174a1) are the same code. It registers the 6-slot table "ApiText" (0x18 bytes):
//! a queue of on-screen text boxes (the hint texts shown e.g. when the book is first used).
//!
//! Local data (`pLocalDataApiCText`, 0x1748 bytes, the lib object's own): +0 lib object id,
//! +4 HUD window (-2 = none), +8 display timer, +0xc 99 queue entries of 0x3c bytes, +0x1740 "text
//! displayed", +0x1744 object to notify (message 99999) when the text is killed.
//! Entry i, at local + i*0x3c (the settings words p[] are the 13-word struct filled by InitStruct):
//! +0xc in use, +0x10 text id, +0x14 duration p[0], +0x18/+0x1c position p[1..3], +0x20/+0x24 size
//! p[3..5], +0x28/+0x2c background params p[5..7], +0x30 colour p[7], +0x34 background type p[8],
//! +0x38 font p[9], +0x3c/+0x40 WindowAdd args 4/3 p[10]/p[11], +0x44 object to notify p[12].
#![allow(non_snake_case, non_upper_case_globals, unused_variables)]
use super::{lib_data, set_global};
use crate::engine::core::apicons as cons;
use crate::engine::*;
use crate::logic::{LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "apitext",
    data_size: 0x30,
    ctor: apitextConstructor,
    dtor: apitextDestructor,
    register: Some(apitextRegister),
    init: &[],
    build_init: None,
};

// module globals (_globals.h)
const pGameObj: u32 = 0x10000;
const pStdlib: u32 = 0x10004;
const pExtra: u32 = 0x10008;
const pRegLib: u32 = 0x1000c;
const pMath: u32 = 0x10010;
const pHud: u32 = 0x10014;
const pText: u32 = 0x10018;
const pLocalDataApiCText: u32 = 0x1001c;

/// value of pLocalDataApiCText
fn local(e: &Engine) -> Ptr {
    e.r32(lib_data(e, &MODULE, pLocalDataApiCText))
}
/// `pLocalDataApiCText[i]` (word index)
fn lw(e: &Engine, i: u32) -> u32 {
    let l = local(e);
    e.r32(l + i * 4)
}
fn sw(e: &mut Engine, i: u32, v: u32) {
    let l = local(e);
    e.w32(l + i * 4, v);
}

/// 0x00010cd8 queue text `param_1` with the 13-word settings `param_2` (see InitStruct) in the first
/// free of the 99 entries (queue full: dropped), then try to display the head of the queue
pub fn ShowText(e: &mut Engine, param_1: u32, param_2: Ptr) {
    for i in 0..=0x62u32 {
        let ent = local(e) + i * 0x3c;
        if e.r32(ent + 0xc) == 0 {
            e.w32(ent + 0xc, 1);
            e.w32(ent + 0x10, param_1);
            // the stores are in the original's order
            for (o, k) in [(0x14, 0), (0x2c, 6), (0x28, 5), (0x24, 4), (0x18, 1), (0x1c, 2), (0x20, 3), (0x40, 0xb), (0x34, 8), (0x38, 9), (0x30, 7), (0x3c, 10), (0x44, 0xc)] {
                let v = e.r32(param_2 + k * 4);
                e.w32(ent + o, v);
            }
            break;
        }
    }
    DisplayText(e);
}

/// 0x00011010 remove the displayed text (notify its object with 99999, delete the window) and show the
/// next queued one, if any
pub fn KillText(e: &mut Engine) {
    let l = local(e);
    e.w32(l + 0x1740, 0);
    if e.r32(l + 0x44) != 0 {
        let o = e.r32(l + 0x1744);
        APIgameobj::SendMessageTo(e, o, NULL, 99999, 0, NULL);
    }
    if e.ri32(l + 4) != -2 {
        let w = e.r32(l + 4);
        APIhud::WindowDelete(e, w);
        e.w32(l + 4, 0xffff_fffe);
    }
    if e.r32(l + 0x48) == 1 {
        SetArrayFirstPosAtNull(e);
        SortTheText(e);
        DisplayText(e);
    } else {
        SetArrayFirstPosAtNull(e);
    }
}

/// 0x00011148 default text settings
pub fn InitStruct(e: &mut Engine, param_1: Ptr) {
    for (i, v) in [(0, 0), (6, 0x3f80_0000), (5, 0x3f80_0000), (4, 0x3e4c_cccd), (1, 0), (2, 0x3f26_6666), (3, 0x3f4c_cccd), (0xb, 0x30004), (8, 0), (7, 6), (9, 0), (10, 2), (0xc, 0)] {
        e.w32(param_1 + i * 4, v);
    }
}

/// 0x00011218
pub fn IsTextDisplayed(e: &mut Engine) -> u8 {
    let l = local(e);
    (e.r32(l + 0x1740) != 0) as u8
}

/// 0x00011270 callback 0 of the lib object: the display timer fired
pub fn fcback_TIMERTEXT(e: &mut Engine, param_1: u32) -> LResult<u64> {
    if param_1 == lw(e, 2) {
        let (o, t) = (lw(e, 0), lw(e, 2));
        APIextra::TimerUnregisterNotify(e, o, t);
        KillText(e);
    }
    Ok(0)
}

/// 0x00011308 shift the queue down by one entry. Kept as in the original: the last iteration copies
/// "entry 99", which starts at +0x172c and runs 0x24 bytes past the 0x1748-byte local data (it covers
/// +0x1740/+0x1744 and whatever follows the allocation).
pub fn SortTheText(e: &mut Engine) {
    for i in 0..99u32 {
        let l = local(e);
        let (dst, src) = (l + i * 0x3c, l + (i + 1) * 0x3c);
        let v = e.r32(src + 0xc);
        e.w32(dst + 0xc, v);
        for o in (0x14..0x48).step_by(4) {
            let v = e.r32(src + o);
            e.w32(dst + o, v);
        }
        let v = e.r32(src + 0x10);
        e.w32(dst + 0x10, v);
    }
}

/// 0x000114a0 clear entry 0
pub fn SetArrayFirstPosAtNull(e: &mut Engine) {
    let l = local(e);
    for o in [0xc, 0x14, 0x2c, 0x28, 0x24, 0x18, 0x1c, 0x20, 0x40, 0x34, 0x38, 0x30, 0x3c, 0x10, 0x44] {
        e.w32(l + o, 0);
    }
}

/// 0x00011578 hide the text window
pub fn DisableWindow(e: &mut Engine) {
    if lw(e, 1) as i32 != -2 {
        let w = lw(e, 1);
        APIhud::WindowSetFlags(e, w, 0x100);
    }
}

/// 0x000115e0 show the text window
pub fn EnableWindow(e: &mut Engine) {
    if lw(e, 1) as i32 != -2 {
        let w = lw(e, 1);
        APIhud::WindowResetFlags(e, w, 0x100);
    }
}

/// 0x00011648 if nothing is displayed and entry 0 has a text: open its HUD text window, arm the timer
/// (duration > 0) and remember the object to notify.
/// TextWindowSetText is variadic (printf-style); ApiText passes no extra arguments, the port passes 0.
pub fn DisplayText(e: &mut Engine) {
    if lw(e, 0x5d0) != 0 {
        return;
    }
    if lw(e, 4) == 0 {
        sw(e, 0x5d0, 0);
        return;
    }
    sw(e, 0x5d0, 1);
    if lw(e, 1) != 0xffff_fffe {
        let w = lw(e, 1);
        APIhud::WindowDelete(e, w);
        sw(e, 1, 0xffff_fffe);
    }
    let (a, b) = (lw(e, 0x10), lw(e, 0xf));
    let uVar1 = APIhud::WindowAdd(e, 1, 0xffff_ffff, a, b);
    sw(e, 1, uVar1);
    let f = |e: &Engine, i: u32| f32::from_bits(lw(e, i));
    let w = lw(e, 1);
    let (x, y) = (f(e, 6), f(e, 7));
    APIhud::WindowSetPos(e, w, x, y);
    let (x, y) = (f(e, 8), f(e, 9));
    APIhud::WindowSetDims(e, w, x, y);
    let (t, x, y) = (lw(e, 0xd), f(e, 10), f(e, 0xb));
    APIhud::WindowSetBackGround(e, w, t, x, y);
    let c = lw(e, 0xc);
    APIhud::TextWindowSetColor(e, w, c);
    let fo = lw(e, 0xe);
    APIhud::TextWindowSetFont(e, w, fo);
    let t = lw(e, 4);
    APIhud::TextWindowSetText(e, w, t, &[0]);
    if 0.0 < f(e, 5) {
        let tm = lw(e, 2);
        APIextra::TimerReset(e, tm);
        let d = f(e, 5);
        APIextra::TimerSetFrequency(e, tm, d);
        let o = lw(e, 0);
        APIextra::TimerRegisterNotify(e, o, tm, 0);
    }
    if lw(e, 0x11) != 0 {
        let v = lw(e, 0x11);
        sw(e, 0x5d1, v);
    }
}

/// 0x00011950 store the API tables, then RegisterLib("ApiText", 0x18)
pub fn apitextRegister(e: &mut Engine, api: Ptr) {
    let tables: [(u32, fn(&mut Engine) -> Ptr); 5] = [
        (pStdlib, |e| cons::GetApiStdlib(e)),
        (pGameObj, |e| cons::GetApiGameObj(e)),
        (pExtra, |e| cons::GetApiExtra(e)),
        (pMath, |e| cons::GetApiMath(e)),
        (pHud, |e| cons::GetApiHUD(e)),
    ];
    for (a, f) in tables {
        let v = f(e);
        set_global(e, &MODULE, a, v);
    }
    let n = e.cstr("ApiText");
    let v = cons::GetApiCustom(e, n);
    set_global(e, &MODULE, pText, v);
    let v = cons::GetApiRegLib(e);
    set_global(e, &MODULE, pRegLib, v);
    let v = APIRegLib::RegisterLib(e, n, 0x18);
    set_global(e, &MODULE, pText, v);
}

/// 0x00011ad8 display timer, no window, empty queue
pub fn apitextConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    if instanceCount == 1 {
        cons::SetExternalFunction(e, 0, fcback_TIMERTEXT);
    }
    let slot = lib_data(e, &MODULE, pLocalDataApiCText);
    APIcons::GetLocalData(e, slot, 0x1748);
    let v = APIextra::TimerCreate(e);
    sw(e, 2, v);
    sw(e, 1, 0xffff_fffe);
    for i in 0..99u32 {
        for k in [3, 5, 0xb, 10, 9, 6, 7, 8, 0x10, 0xd, 0xe, 0xc, 0xf, 4, 0x11] {
            sw(e, i * 0xf + k, 0);
        }
    }
    let v = APIgameobj::GetID(e, SELF, NULL);
    sw(e, 0, v);
    Ok(())
}

/// 0x00011e08
pub fn apitextDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    let n = e.cstr("ApiText");
    APIRegLib::UnRegisterLib(e, n);
    let t = lw(e, 2);
    APIextra::TimerDestroy(e, t);
    if lw(e, 1) as i32 != -2 {
        let w = lw(e, 1);
        APIhud::WindowDelete(e, w);
        sw(e, 1, 0xffff_fffe);
    }
    Ok(())
}

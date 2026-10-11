//! `button`: EP1L01 wall button 0x2a00004f (goCustomDataButton: target 0x2a00004e, msg 0xe0,
//! GB2+8 bit 0x100000). Leader touching it shows the X prompt (0x31/4 to 0x2a100006); X toggles
//! the GB2+8/+0x10 bits, tells the leader (0x99, data = own id), moves the camera (0x54/4 at the
//! button's bone 4) and the press anim's end (anim notify) sends msg 0xe0 to the target with the
//! own id and &L[0x10]. Port of `Game Logic Modules/Ep1_Library/Objects/BUTTON/button__ecaaf7b4/`
//! (button.c + button_base.c, 14 functions). EP1L01 build ecaaf7b4.
#![allow(non_snake_case, non_upper_case_globals, unused_variables)]

use crate::engine::*;
use crate::logic::{ExtFn, LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "button",
    // globals 0x10000..0x1007c (+ unused gpoLanguagesNamesAndISOCodes)
    data_size: 0xe80,
    ctor: ctor_dispatch,
    dtor: dtor_dispatch,
    register: None,
    init: &[],
    build_init: Some(build_init_dispatch),
};

/// initialised data of the hand-ported build(s) (written by build_init_dispatch only for them)
const OLD_INIT: &[(u32, &[u32])] = &[(0x10000, &[0x2a00_004f, 0x2a00_004e, 0xe0, 0x10_0000, 0, 0])];

mod xb;

/// number of hand-ported builds (indices 0..N_OLD); generated builds (`xb`, c2rs) follow
pub(crate) const N_OLD: usize = 1;

fn ctor_dispatch(e: &mut Engine, api: Ptr, n: i32) -> LResult<()> {
    if e.rt.cur_build() < N_OLD { buttonConstructor(e, api, n) } else { xb::ctor(e, api, n) }
}

fn dtor_dispatch(e: &mut Engine, n: i32) -> LResult<()> {
    if e.rt.cur_build() < N_OLD { buttonDestructor(e, n) } else { xb::dtor(e, n) }
}

fn build_init_dispatch(m: &mut Mem, data: Ptr, b: usize) {
    if b < N_OLD {
        for &(a, ws) in OLD_INIT {
            for (i, &w) in ws.iter().enumerate() {
                m.w32(data + (a - 0x10000) + 4 * i as u32, w);
            }
        }
    } else {
        xb::build_init(m, data, b - N_OLD);
    }
}


pub fn build_index(key: &str) -> Option<usize> {
    if let Some(i) = build_index_old(key) {
        return Some(i);
    }
    xb::build_index(key).map(|i| i + N_OLD)
}

fn build_index_old(key: &str) -> Option<usize> {
    (key == "ecaaf7b4").then_some(0)
}

pub const goCustomDataButton: u32 = 0x10000;
pub const pAnim: u32 = 0x10018;
pub const pExtra: u32 = 0x1001c;
pub const pGame: u32 = 0x10020;
pub const pGameObj: u32 = 0x10024;
pub const pHud: u32 = 0x10028;
pub const pInput: u32 = 0x1002c;
pub const pPhysic: u32 = 0x10030;
pub const pSound: u32 = 0x10034;
pub const pAnimMngr: u32 = 0x10038;
pub const pDebug: u32 = 0x1003c;
pub const pText: u32 = 0x10040;
pub const pMiscFunc: u32 = 0x10044;
pub const pLocalDataButton: u32 = 0x10048;
pub const pGlobalDataButton: u32 = 0x1004c;
/// function statics of fcback_InputAction_X (not in the symtab; addresses from the disassembly)
pub const moCameraParams_24: u32 = 0x10050;
pub const oCameraPos_25: u32 = 0x10070;

#[inline]
fn G(e: &Engine, a: u32) -> Ptr {
    e.rt.cur_data() + (a - 0x10000)
}
/// pLocalDataButton (0x8c bytes, word index n = +4n): [0] input config, [2] own id, [3] DM,
/// [4] disabled, [5] already pressed, [6] input active, [7] prompt shown, [8] 0x4c pause,
/// [9] 0x12 latch, [0xa..0xc] position, [0xd] touched (post-collision), [0xe] target, [0xf] msg,
/// [0x10] GB2+8 bits, [0x11] one-shot, [0x12] GB2+0x10 bits, [0x13..] clipping
#[inline]
fn L(e: &Engine) -> Ptr {
    e.r32(G(e, pLocalDataButton))
}
#[inline]
fn lr(e: &Engine, n: u32) -> u32 {
    e.r32(L(e) + 4 * n)
}
#[inline]
fn lw(e: &mut Engine, n: u32, v: u32) {
    let a = L(e) + 4 * n;
    e.w32(a, v);
}
/// game buffer n (pGlobalDataButton[n])
#[inline]
fn GB(e: &Engine, n: u32) -> Ptr {
    e.r32(e.r32(G(e, pGlobalDataButton)) + 4 * n)
}
fn cfg(e: &Engine) -> i32 {
    lr(e, 0) as i32
}


pub fn buttonConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    button_baseConstructor(e, api, instanceCount);
    APIcons::SetInitialState(e, 0);
    Ok(())
}

pub fn buttonDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    Ok(())
}

/// state 0
pub fn fstate_FIRSTMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    let d = G(e, goCustomDataButton);
    if e.r32(d) == lr(e, 2) {
        for (n, o) in [(0x11, 16), (0xe, 4), (0xf, 8), (0x10, 12), (0x12, 20)] {
            let v = e.r32(d + o);
            lw(e, n, v);
        }
    }
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 3, 4);
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 0xb, 3);
    let c = cfg(e);
    APIinput::InputConfigAdd(e, c);
    APIinput::InputActionAdd(e, c, 0x39, 0);
    let port = e.r32(GB(e, 0) + 0x4c) as i32;
    APIinput::InputDeviceAdd(e, c, 0x39, 0x3d, port, 0, 2, 0, 0);
    APIinput::InputDeviceAdd(e, c, 0x39, 0x3d, 1, 1, 2, 0, 0x39);
    APIinput::InputContextAdd(e, SELF, 8, c, 0x39, 0x1e, 8);
    APIinput::InputContextSetState(e, c, 0x39, 0x1e, 1);
    APIinput::InputConfigSetState(e, c, 3);
    APIinput::InputConfigActivate(e, c, 0);
    lw(e, 6, 0);
    let gb2 = GB(e, 2);
    if e.r32(gb2 + 8) & lr(e, 0x10) == 0 && e.r32(gb2 + 0x10) & lr(e, 0x12) == 0 {
        lw(e, 5, 0);
    } else {
        lw(e, 5, 1);
        if lr(e, 0x11) != 0 {
            APIgameobj::UnregisterNotify(e, SELF, 0xffff_ffff, 3);
            APIinput::InputConfigActivate(e, c, 0);
            lw(e, 6, 0);
            lw(e, 4, 1);
        }
    }
    let p = L(e) + 0x28;
    APIgameobj::GetPosition(e, SELF, 0x7fff_ffff, p);
    LIBAnimMngr::PlayAnim(e, 0.0, 2, 1, 0);
    APIgameobj::SetNextState(e, SELF, 1);
    Ok(0)
}

/// state 1
pub fn fstate_BASICMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

/// ext fn 2
pub fn fcback_PUSH_LF(e: &mut Engine, _: u32) -> LResult<u64> {
    LIBAnimMngr::PlayAnim(e, 0.0, 2, 1, 0);
    APIgameobj::SetNextState(e, SELF, 1);
    Ok(0)
}

/// ext fn 3 (anim notify: +4 type, +8 key)
pub fn fcback_ANIM(e: &mut Engine, p: u32) -> LResult<u64> {
    if e.r32(p + 4) == 0 && e.r32(p + 8) == 0 {
        APInewSound::Play(e, SELF, 0x4000_0004, 0);
        let (t, m, id, d) = (lr(e, 0xe), lr(e, 0xf), lr(e, 2), L(e) + 0x40);
        APIgameobj::SendMessageTo(e, t, NULL, m, id, d);
    }
    Ok(0)
}

/// ext fn 4
pub fn fcback_CollisionDetection(e: &mut Engine, p: u32) -> LResult<u64> {
    let gb0 = GB(e, 0);
    if e.r32(gb0 + 8) == e.r32(p + 8) && e.r32(gb0 + 0x14) & 0x6e20_090d == 0 && lr(e, 4) != 1 {
        if matches!(e.r32(gb0 + 4), 1 | 2 | 5 | 6 | 0xe | 0xf | 0x10 | 0x11) {
            lw(e, 0xd, 1);
            let dead = e.r32(GB(e, 2)) & 0x20000 != 0;
            if lr(e, 7) == 0 && !dead {
                lw(e, 7, 1);
                let c = L(e) + 0x4c;
                LIBMiscFunc::MiscFunc_RemoveBitClipping(e, c, 0x80);
                APIgameobj::SetContextFlag(e, SELF, 3);
                let c = cfg(e);
                APIinput::InputConfigActivate(e, c, 1);
                lw(e, 6, 1);
                APIgameobj::SendMessageTo(e, 0x2a10_0006, NULL, 0x31, 4, NULL);
            } else if lr(e, 7) != 0 && !dead {
                APIgameobj::SendMessageTo(e, 0x2a10_0006, NULL, 0x31, 4, NULL);
            }
        }
    }
    Ok(0)
}

/// ext fn 5
pub fn fcback_CollisionDetectionOut(e: &mut Engine, _: u32) -> LResult<u64> {
    if e.r32(GB(e, 2)) & 0x20000 != 0 {
        let s = e.cstr("Bouton Killed\n");
        APIextra::null_printf(e, s, 0);
        APIgameobj::SendMessageTo(e, 0x2a10_0006, NULL, 0x32, 4, NULL);
    }
    let c = L(e) + 0x4c;
    LIBMiscFunc::MiscFunc_ReplaceMyClipping(e, c);
    let c = cfg(e);
    APIinput::InputConfigActivate(e, c, 0);
    lw(e, 6, 0);
    lw(e, 7, 0);
    APIgameobj::SetContextFlag(e, SELF, 0);
    Ok(0)
}

/// ext fn 6
pub fn fcback_POST_COLLISION(e: &mut Engine, _: u32) -> LResult<u64> {
    if lr(e, 0xd) == 0 {
        fcback_CollisionDetectionOut(e, 0)?;
    }
    lw(e, 0xd, 0);
    Ok(0)
}

/// ext fn 8
pub fn fcback_InputAction_X(e: &mut Engine, _: u32) -> LResult<u64> {
    if e.r32(GB(e, 0) + 0x14) & 0x6e20_090d != 0 || lr(e, 8) == 1 || lr(e, 9) == 1 {
        return Ok(0);
    }
    LIBApiText::KillText(e);
    let gb2 = GB(e, 2);
    let (b8, b10) = (lr(e, 0x10), lr(e, 0x12));
    if e.r32(gb2 + 8) & b8 == 0 && e.r32(gb2 + 0x10) & b10 == 0 {
        let v = e.r32(gb2 + 8) | b8;
        e.w32(gb2 + 8, v);
        let v = e.r32(gb2 + 0x10) | b10;
        e.w32(gb2 + 0x10, v);
    } else {
        let v = e.r32(gb2 + 8) & !b8;
        e.w32(gb2 + 8, v);
    }
    fcback_CollisionDetectionOut(e, 0)?;
    let leader = e.r32(GB(e, 0) + 8);
    let id = APIgameobj::GetID(e, SELF, NULL);
    APIgameobj::SendMessageTo(e, leader, NULL, 0x99, 0, id);
    let pos = G(e, oCameraPos_25);
    APIgameobj::GetPosition(e, SELF, 4, pos);
    let cp = G(e, moCameraParams_24);
    for (o, v) in [(0, 1), (4, 0), (0xc, pos), (0x10, 0x3f00_0000), (0x14, 0x3f80_0000), (0x18, 0x4120_0000)] {
        e.w32(cp + o, v);
    }
    APIgameobj::SendMessageTo(e, 0x2a00_008f, NULL, 0x54, 4, cp);
    Ok(1)
}

pub fn MyMessageFunction(e: &mut Engine, m: u32, _p: u32, _d: Ptr) -> LResult<u64> {
    let gb2 = GB(e, 2);
    match m {
        0xd4 => {
            if e.r32(gb2 + 8) & lr(e, 0x10) != 0 || e.r32(gb2 + 0x10) & lr(e, 0x12) != 0 {
                lw(e, 5, 1);
            }
        }
        0x4c => {
            lw(e, 8, 1);
            let c = cfg(e);
            APIinput::InputConfigActivate(e, c, 0);
        }
        0x12 => lw(e, 9, 1),
        0x13 => lw(e, 9, 0),
        0x51 => {
            if lr(e, 6) != 0 {
                let c = cfg(e);
                APIinput::InputConfigActivate(e, c, 1);
            }
            lw(e, 8, 0);
        }
        0xde => {
            APIgameobj::SetGroupID(e, SELF, 6);
            LIBAnimMngr::PlayAnim(e, 0.0, 1, 1, 2);
            if lr(e, 0x11) != 0 {
                APIgameobj::UnregisterNotify(e, SELF, 0xffff_ffff, 3);
                let c = cfg(e);
                APIinput::InputConfigActivate(e, c, 0);
                lw(e, 4, 1);
                fcback_CollisionDetectionOut(e, 0)?;
            }
        }
        0xdf | 0x166 => {
            APIgameobj::UnregisterNotify(e, SELF, 0xffff_ffff, 3);
            let c = cfg(e);
            APIinput::InputConfigActivate(e, c, 0);
            fcback_CollisionDetectionOut(e, 0)?;
        }
        99999 => {
            let c = cfg(e);
            APIinput::InputConfigActivate(e, c, 0);
            lw(e, 6, 0);
        }
        0xc9 | 0x165 => {
            if lr(e, 5) == 0 {
                APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 3, 4);
                let v = e.r32(gb2 + 8) & !lr(e, 0x10);
                e.w32(gb2 + 8, v);
                let v = e.r32(gb2 + 0x10) & !lr(e, 0x12);
                e.w32(gb2 + 0x10, v);
                lw(e, 4, 0);
            }
        }
        _ => {}
    }
    Ok(0)
}

pub fn CreatePhysic(e: &mut Engine) {
    APIphysic::CreatePhysic(e, SELF, 0);
    APIphysic::SetFlags(e, SELF, 2);
    APIphysic::SetWeight(e, SELF, 1.0);
    APIphysic::SetPriority(e, SELF, 100);
    APIphysic::PrimSetIntAttribBits(e, SELF, 0x32, 0, 4);
    let dm = APIphysic::DMAdd(e, SELF, 0x25);
    lw(e, 3, dm);
    let p = e.alloca(0x10); // auStack_30
    APIphysic::DMGetParams(e, dm, p);
    e.w32(p + 4, 6);
    APIphysic::DMSetParams(e, dm, p);
    APIphysic::DMSetContextMask(e, dm, 2);
    APIgameobj::SetContextFlag(e, SELF, 0);
}

pub fn button_baseConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) {
    if instanceCount == 1 {
        let fns: [(i32, ExtFn); 8] = [
            (0, fstate_FIRSTMOVE),
            (1, fstate_BASICMOVE),
            (2, fcback_PUSH_LF),
            (3, fcback_ANIM),
            (4, fcback_CollisionDetection),
            (5, fcback_CollisionDetectionOut),
            (6, fcback_POST_COLLISION),
            (8, fcback_InputAction_X),
        ];
        for (i, f) in fns {
            APIcons::SetExternalFunction(e, i, f);
        }
        let tables: [(u32, fn(&mut Engine) -> Ptr); 8] = [
            (pAnim, |e| APIcons::GetApiAnim(e)),
            (pExtra, |e| APIcons::GetApiExtra(e)),
            (pGame, |e| APIcons::GetApiGame(e)),
            (pGameObj, |e| APIcons::GetApiGameObj(e)),
            (pHud, |e| APIcons::GetApiHUD(e)),
            (pInput, |e| APIcons::GetApiInput(e)),
            (pPhysic, |e| APIcons::GetApiPhysic(e)),
            (pSound, |e| APIcons::GetApiNewSound(e)),
        ];
        for (slot, get) in tables {
            let v = get(e);
            let a = G(e, slot);
            e.w32(a, v);
        }
        for (slot, name) in [(pAnimMngr, "AnimMngr"), (pDebug, "debug"), (pText, "ApiText"), (pMiscFunc, "MiscFunc")] {
            let n = e.cstr(name);
            let v = APIcons::GetApiCustom(e, n);
            let a = G(e, slot);
            e.w32(a, v);
        }
        let slot = G(e, pGlobalDataButton);
        APIcons::GetGlobalData(e, slot, 0xc);
        let gd = e.r32(slot);
        for (n, size) in [(0u32, 0x8f8), (1, 0x68), (2, 0x158)] {
            let b = APIgameobj::GetGameBuffer(e, n, size, 1);
            e.w32(gd + 4 * n, b);
        }
    }
    let slot = G(e, pLocalDataButton);
    APIcons::GetLocalData(e, slot, 0x8c);
    let id = APIgameobj::GetID(e, SELF, NULL);
    lw(e, 2, id);
    LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 2, NULL, 0);
    APIcons::SetMessageFunction(e, MyMessageFunction);
    CreatePhysic(e);
    lw(e, 0, (instanceCount + 700) as u32);
}

//! `inv_control`: the per-level invisible controller (music switching/fading via msgs 0x5c..0x5f,
//! light-set visibility per room via msg 0x98, game-flow data reset at load, and the level's
//! objective radar messages). Port of
//! `Game Logic Modules/Globals/Objects/Inv_Control_Base/inv_control__056b6db0/` (Inv_Control_Base.c,
//! inv_control.c; EP1L01 build). Inv_Control_Base.c is shared with the EP1L03A build
//! (inv_control__7c7df8a6, build 1 here: globals shifted by +0x450 behind `goCustomDataInvCtrl`, own
//! local part with the ghost-trap objective timer).
#![allow(non_snake_case, non_upper_case_globals, unused_variables)]

use crate::engine::*;
use crate::logic::{LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "inv_control",
    // globals 0x10000..0x1004c (L03A: 0x10000..0x1049c) (+ gpoLanguagesNamesAndISOCodes)
    data_size: 0x4b0,
    ctor: inv_controlConstructor,
    dtor: inv_controlDestructor,
    register: None,
    init: &[],
    build_init: None,
};

pub fn build_index(key: &str) -> Option<usize> {
    ["056b6db0", "7c7df8a6"].iter().position(|k| *k == key)
}

pub const pGraphic: u32 = 0x10000;
pub const pLocalDataInvCtrlBase: u32 = 0x10004;
pub const pGlobalDataInvCtrlBase: u32 = 0x10008;
pub const pLocalDataInvCtrl: u32 = 0x1000c;
pub const pGlobalDataInvCtrl: u32 = 0x10010;
pub const oLtColor_12: u32 = 0x10018;
pub const pExtra: u32 = 0x10028;
pub const pGameObj: u32 = 0x1002c;
pub const pLight: u32 = 0x10030;
pub const pSound: u32 = 0x10034;
pub const pDebug: u32 = 0x10038;
pub const pHud: u32 = 0x1003c;
pub const pGame: u32 = 0x10040;
pub const pMisc: u32 = 0x10044;
pub const pGraphics: u32 = 0x10048;

const RADAR: u32 = 0x2a10_0006;

#[inline]
pub(crate) fn G(e: &Engine, a: u32) -> Ptr {
    // L03A: goCustomDataInvCtrl[0x450] precedes the same globals (pGraphics slot is pPhysic there)
    e.rt.cur_data() + (a - 0x10000) + [0, 0x450][e.rt.cur_build()]
}
/// pLocalDataInvCtrlBase (0xbe3c): 13 light sets x 0x30 {light, count, rooms[]}, +0xbe0c current
/// music, +0xbe10 saved music, +0xbe1c, +0xbe28 fade step, +0xbe2c fade dir, +0xbe30 fading,
/// +0xbe34 master volume, +0xbe38 flags (2 = music allowed)
#[inline]
pub(crate) fn LB(e: &Engine) -> Ptr {
    e.r32(G(e, pLocalDataInvCtrlBase))
}
/// pGlobalDataInvCtrlBase: [0] GB0, [1] GB2, [2] GB1
#[inline]
fn L03A(e: &Engine) -> bool {
    e.rt.cur_build() == 1
}
/// Inv_Control_Base local offset: L03A has 23 light sets (+10 x 0x30 = +0x1e0)
#[inline]
fn o(e: &Engine, x: u32) -> u32 {
    if L03A(e) { x + 0x1e0 } else { x }
}
/// L01 music event id -> this build's (panic 0x29/5, costume 0x2a/6, pause 0x28/4)
fn mu(e: &Engine, x: u32) -> u32 {
    if L03A(e) { x - 0x24 } else { x }
}
/// inverse of [`mu`] for the switch statements
fn norm(e: &Engine, x: u32) -> u32 {
    if L03A(e) && matches!(x, 0x8000_0004..=0x8000_0006) { x + 0x24 } else { x }
}
fn GDB(e: &Engine, i: u32) -> Ptr {
    let g = e.r32(G(e, pGlobalDataInvCtrlBase));
    e.r32(g + 4 * i)
}
/// pLocalDataInvCtrl (same instance block, 0xbe44): +0xbe40 objective flags
#[inline]
pub(crate) fn LC(e: &Engine) -> Ptr {
    e.r32(G(e, pLocalDataInvCtrl))
}
/// pGlobalDataInvCtrl: [0] GB0, [1] GB2
#[inline]
pub(crate) fn GDC(e: &Engine, i: u32) -> Ptr {
    let g = e.r32(G(e, pGlobalDataInvCtrl));
    e.r32(g + 4 * i)
}
fn dbg(e: &mut Engine, msg: &str) {
    if e.r32(G(e, pDebug)) != 0 {
        let (a, b) = (e.cstr("inv_control_base"), e.cstr(msg));
        LIBdebug::PrintDebugInfo(e, a, b);
    }
}
pub(crate) fn send(e: &mut Engine, to: u32, m: u32, p: u32) {
    APIgameobj::SendMessageTo(e, to, NULL, m, p, NULL);
}

// ---- Inv_Control_Base.c

pub fn inv_control_baseST_FIRSTMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    let cam = if L03A(e) { 0x2a00_004a } else { 0x2a00_008f };
    APInewSound::SetListener(e, cam);
    let l = LB(e);
    e.w32(l + o(e, 0xbe0c), 0x8000_0000);
    dbg(e, "-> INV_CONTROL_BASE");
    Ok(0)
}

pub fn inv_control_base_Update_Music_Volume(e: &mut Engine) {
    let l = LB(e);
    if e.r32(l + o(e, 0xbe30)) == 0 {
        return;
    }
    let vol = e.rf(l + o(e, 0xbe34));
    if e.r32(l + o(e, 0xbe2c)) == 0 {
        let s = e.ri32(l + o(e, 0xbe28)) - 1;
        e.w32(l + o(e, 0xbe28), s as u32);
        if s < 1 {
            e.w32(l + o(e, 0xbe28), 0);
            APInewSound::SetSoundEventMasterVolume(e, SELF, 0x8000_0000, vol);
            e.w32(l + o(e, 0xbe30), 0);
            return;
        }
    } else {
        let s = e.ri32(l + o(e, 0xbe28)) + 1;
        e.w32(l + o(e, 0xbe28), s as u32);
        if 0xe < s {
            e.w32(l + o(e, 0xbe28), 0xe);
            e.w32(l + o(e, 0xbe30), 0);
            return;
        }
    }
    let s = e.ri32(l + o(e, 0xbe28));
    APInewSound::SetSoundEventMasterVolume(e, SELF, 0x8000_0000, vol - s as f32 * 0.05);
}

pub fn inv_control_base_Change_Music(e: &mut Engine, param_1: u32) {
    let l = LB(e);
    let cur = e.r32(l + o(e, 0xbe0c));
    match param_1 {
        0 => {
            dbg(e, "Change_Music ---- MUS_STOP");
            APInewSound::StopSound(e, SELF, cur);
            e.w32(l + o(e, 0xbe0c), 0);
        }
        1 => {
            if e.r32(l + o(e, 0xbe38)) & 2 == 0 {
                APInewSound::StopSound(e, SELF, cur);
                e.w32(l + o(e, 0xbe0c), 0);
            } else {
                dbg(e, "Change_Music ---- NORMAL_MUSIC");
                match norm(e, cur) {
                    0x8000_0029 => {
                        APInewSound::StopSound(e, SELF, mu(e, 0x8000_0029));
                        APInewSound::Play(e, SELF, 0x8000_0000, 0);
                    }
                    0x8000_0001 => {
                        APInewSound::StopSound(e, SELF, 0x8000_0001);
                        APInewSound::ResumeSoundEvents(e, 8);
                    }
                    0 => {
                        APInewSound::Play(e, SELF, 0x8000_0000, 0);
                    }
                    0x8000_002a => {
                        APInewSound::StopSound(e, SELF, mu(e, 0x8000_002a));
                        APInewSound::Play(e, SELF, 0x8000_0000, 0);
                    }
                    _ => {}
                }
                e.w32(l + o(e, 0xbe0c), 0x8000_0000);
            }
        }
        2 => {
            if cur != 0x8000_0001 {
                dbg(e, "Change_Music ---- COMBAT_MUSIC");
                if cur == 0x8000_0000 {
                    APInewSound::PauseSoundEvents(e, 8);
                } else {
                    APInewSound::StopSound(e, SELF, cur);
                }
                APInewSound::Play(e, SELF, 0x8000_0001, 0);
                e.w32(l + o(e, 0xbe0c), 0x8000_0001);
            }
        }
        3 => {
            if cur != mu(e, 0x8000_0029) {
                dbg(e, "Change_Music ---- MU_MUSIC_PANIC");
                if cur != 0 {
                    APInewSound::StopSound(e, SELF, cur);
                }
                APInewSound::Play(e, SELF, mu(e, 0x8000_0029), 0);
                e.w32(l + o(e, 0xbe0c), mu(e, 0x8000_0029));
            }
        }
        4 => {
            if cur != mu(e, 0x8000_002a) {
                dbg(e, "Change_Music ---- MUS_COSTUME");
                if cur == 0x8000_0000 {
                    APInewSound::StopSound(e, SELF, 0x8000_0000);
                } else {
                    APInewSound::StopSound(e, SELF, cur);
                }
                APInewSound::Play(e, SELF, mu(e, 0x8000_002a), 0);
                e.w32(l + o(e, 0xbe0c), mu(e, 0x8000_002a));
            }
        }
        5 => {
            e.w32(l + o(e, 0xbe10), cur);
            match norm(e, cur) {
                0x8000_0001 => {
                    APInewSound::PauseSoundEvents(e, 0x20);
                }
                0x8000_0000 | 0x8000_0029 => {
                    APInewSound::PauseSoundEvents(e, 8);
                }
                0x8000_002a => {
                    APInewSound::PauseSoundEvents(e, 0x80);
                }
                _ => {}
            }
            APInewSound::Play(e, SELF, mu(e, 0x8000_0028), 0);
            e.w32(l + o(e, 0xbe0c), mu(e, 0x8000_0028));
        }
        6 => {
            let saved = e.r32(l + o(e, 0xbe10));
            match norm(e, saved) {
                0x8000_0001 => {
                    APInewSound::ResumeSoundEvents(e, 0x20);
                }
                0x8000_0000 | 0x8000_0029 => {
                    APInewSound::ResumeSoundEvents(e, 8);
                }
                0x8000_002a => {
                    APInewSound::ResumeSoundEvents(e, 0x80);
                }
                _ => {}
            }
            APInewSound::StopSound(e, SELF, mu(e, 0x8000_0028));
            e.w32(l + o(e, 0xbe0c), saved);
        }
        _ => {}
    }
}

/// (unreferenced) writes an RGBA colour into the static `oLtColor_12`
pub fn inv_controlFUNC_SetLights(e: &mut Engine, r: u32, g: u32, b: u32) -> Ptr {
    let a = G(e, oLtColor_12);
    for (i, v) in [r, g, b, 0].into_iter().enumerate() {
        e.w32(a + 4 * i as u32, v);
    }
    a
}

pub fn inv_control_baseFUNC_InitGameFlowData(e: &mut Engine) {
    let l = LB(e);
    let gb0 = GDB(e, 0);
    let (gb2, gb1) = (GDB(e, 1), GDB(e, 2));
    let old18 = e.r32(gb0 + 0x18);
    let old1c = e.r32(gb0 + 0x1c);
    let v = e.r32(gb0 + 0x20);
    e.w32(l + o(e, 0xbe1c), v);
    e.w32(gb0 + 0x18, 1);
    let (lvl, sub) = if L03A(e) { (3, 2) } else { (1, 0) };
    e.w32(gb0 + 0x1c, lvl);
    e.w32(gb0 + 0x20, sub);
    let f = e.r32(gb2) & 0xfffd_ffff;
    e.w32(gb2, f);
    e.w32(gb0 + 0x74, 0);
    if old18 != e.r32(gb0 + 0x18) {
        e.w32(gb2 + 0x14, 0);
        e.w32(gb1, 0x3f80_0000);
    }
    if old1c != e.r32(gb0 + 0x1c) {
        e.w32(gb2 + 0x14, 0);
        e.w32(gb2 + 0x18, 0);
    }
    if e.r32(l + o(e, 0xbe1c)) != e.r32(gb0 + 0x20) {
        e.w32(gb2 + 0x14, 0);
        e.w32(gb2 + 0x18, 0);
    }
    if e.r32(gb0 + 0x1c) == 9 {
        e.w32(gb1, 0x3f80_0000);
    }
    let same = (e.r32(gb0 + 0x20) == e.r32(gb0 + 0x7b0)) as u32;
    e.w32(gb0 + 0x7ac, same);
    for o in [0x7b0, 0x7bc, 0x7c0, 0x7c4] {
        e.w32(gb0 + o, 0);
    }
}

pub fn inv_control_baseFUNC_DisableGameObjects(_e: &mut Engine) {}

pub fn inv_control_baseFUNC_MessageHandler(e: &mut Engine, m: u32, p: u32) -> u64 {
    let l = LB(e);
    match m {
        0x5e => {
            e.w32(l + o(e, 0xbe2c), p);
            e.w32(l + o(e, 0xbe30), 1);
        }
        0x5c => {
            if p < 7 {
                inv_control_base_Change_Music(e, p);
            }
        }
        0x5d => {
            return match norm(e, e.r32(l + o(e, 0xbe0c))) {
                0x8000_0000 => 1,
                0x8000_0001 => 2,
                0x8000_0028 => 5,
                0x8000_0029 => 3,
                0x8000_002a => 4,
                _ => 0,
            };
        }
        0x5f => {
            let f = e.r32(l + o(e, 0xbe38));
            e.w32(l + o(e, 0xbe38), if p == 0 { f & 0xffff_fffd } else { f | 2 });
        }
        0x98 => {
            for on in [false, true] {
                for i in 0..if L03A(e) { 0x17 } else { 0xd } {
                    let set = l + i * 0x30;
                    for j in 0..e.r32(set + 4) {
                        let room = e.r32(set + 8 + j * 4);
                        if (room == p) == on {
                            let light = e.r32(set);
                            let s = e.cstr(if on { "Inv control> light %u turn ON\n" } else { "Inv control> light %u turn OFF\n" });
                            APIextra::null_printf(e, s, light);
                            APIlight::SetVisible(e, light, on as i32);
                            if on {
                                break;
                            }
                        }
                    }
                }
            }
            inv_control_baseFUNC_DisableGameObjects(e);
        }
        _ => {}
    }
    0
}

pub fn inv_control_baseConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) {
    if instanceCount == 1 {
        APIcons::SetExternalFunction(e, 0, inv_control_baseST_FIRSTMOVE);
        let tables: [(u32, fn(&mut Engine) -> Ptr); 6] = [
            (pGameObj, |e| APIcons::GetApiGameObj(e)),
            (pExtra, |e| APIcons::GetApiExtra(e)),
            (pSound, |e| APIcons::GetApiNewSound(e)),
            (pLight, |e| APIcons::GetApiLight(e)),
            (pHud, |e| APIcons::GetApiHUD(e)),
            (pGame, |e| APIcons::GetApiGame(e)),
        ];
        for (slot, get) in tables {
            let v = get(e);
            let a = G(e, slot);
            e.w32(a, v);
        }
        for (slot, name) in [(pDebug, "debug"), (pMisc, "MiscFunc")] {
            let n = e.cstr(name);
            let v = APIcons::GetApiCustom(e, n);
            let a = G(e, slot);
            e.w32(a, v);
        }
        let v = APIcons::GetApiGraphics(e);
        let a = G(e, pGraphic);
        e.w32(a, v);
        let slot = G(e, pGlobalDataInvCtrlBase);
        APIcons::GetGlobalData(e, slot, 0xc);
        let gd = e.r32(slot);
        for (w, n, size) in [(0u32, 0u32, 0x8f8u32), (1, 2, 0x158), (2, 1, 0x68)] {
            let b = APIgameobj::GetGameBuffer(e, n, size, 1);
            e.w32(gd + 4 * w, b);
        }
    }
    let slot = G(e, pLocalDataInvCtrlBase);
    let size = if L03A(e) { 0xc01c } else { 0xbe3c };
    APIcons::GetLocalData(e, slot, size);
    LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 2, NULL, 0);
    inv_control_baseFUNC_InitGameFlowData(e);
    APIgraphics::DoStupidXboxShadowHack(e, 0);
    let l = LB(e);
    e.w32(l + o(e, 0xbe38), 0xf);
    let v = APInewSound::GetSoundEventMasterVolume(e, SELF, 0x8000_0000);
    e.wf(l + o(e, 0xbe34), v);
    e.w32(l + o(e, 0xbe28), 0);
}

pub fn inv_control_baseDestructor(_e: &mut Engine, _instanceCount: i32) {}

// ---- inv_control.c (EP1L01)

/// state 7
pub fn inv_controlST_FIRSTMOVE(e: &mut Engine, p: u32) -> LResult<u64> {
    inv_control_baseST_FIRSTMOVE(e, p)?;
    APInewSound::Play(e, SELF, 0x8000_0000, 0);
    send(e, RADAR, 0x2a, 1);
    APIgameobj::SetNextState(e, SELF, 8);
    Ok(0)
}

fn flag(e: &Engine) -> u32 {
    e.r32(LC(e) + 0xbe40)
}
fn set_flag(e: &mut Engine, v: u32) {
    let a = LC(e) + 0xbe40;
    e.w32(a, v);
}

/// state 8
pub fn inv_controlST_BASICMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    inv_control_base_Update_Music_Volume(e);
    let gb2 = GDC(e, 1);
    if e.r32(gb2 + 0xc) & 0x800 == 0 {
        if flag(e) & 8 != 0 {
            set_flag(e, flag(e) & 0xffff_fff7);
            send(e, RADAR, 0x2d, 2);
            send(e, RADAR, 0x2c, 2);
        }
    } else if flag(e) & 8 == 0 {
        set_flag(e, flag(e) | 8);
        send(e, RADAR, 0x2a, 2);
        send(e, RADAR, 0x2b, 2);
    } else if e.r32(gb2 + 8) & 0x80 != 0 {
        send(e, RADAR, 0xb, 2);
    }
    Ok(0)
}

/// sets `bit` once; returns true when it was newly set
fn once(e: &mut Engine, bit: u32) -> bool {
    if flag(e) & bit == 0 {
        set_flag(e, flag(e) | bit);
        true
    } else {
        false
    }
}

pub fn inv_controlFUNC_MessageFunction(e: &mut Engine, m: u32, p: u32, _d: Ptr) -> LResult<u64> {
    match m {
        0xc9 => {
            let lvl = e.r32(GDC(e, 0) + 0x24);
            if lvl == 4 || lvl == 5 {
                APIgraphics::DoCrappyXboxSpewHackThingy(e, 0x2a00_0092, 0, 1);
                APIgraphics::DoCrappyXboxSpewHackThingy(e, 0x2a00_0091, 0, 1);
            }
        }
        0x98 => {
            APIgraphics::DoCrappyXboxSpewHackThingy(e, 0x2a00_0092, 0, 0);
            APIgraphics::DoCrappyXboxSpewHackThingy(e, 0x2a00_0091, 0, 0);
            match p {
                6 => {
                    once(e, 0x80);
                    once(e, 0x100);
                }
                5 => {
                    once(e, 0x40);
                }
                7 => {
                    once(e, 0x200);
                }
                _ => {}
            }
        }
        0xde => match p {
            2 => {
                if once(e, 4) {
                    send(e, RADAR, 0x2a, 2);
                }
            }
            1 => {
                if once(e, 2) {
                    send(e, RADAR, 0x2b, 1);
                }
            }
            3 => {
                if once(e, 0x10) {
                    send(e, RADAR, 0x2a, 3);
                }
            }
            4 => {
                if once(e, 0x20) {
                    send(e, RADAR, 0x2b, 3);
                }
            }
            _ => {}
        },
        0xdf => match p {
            3 if flag(e) & 0x10 != 0 => {
                set_flag(e, flag(e) & 0xffff_ffef);
                send(e, RADAR, 0x2c, 3);
            }
            4 if flag(e) & 0x20 != 0 => {
                set_flag(e, flag(e) & 0xffff_ffdf);
                send(e, RADAR, 0x2d, 3);
            }
            _ => {}
        },
        _ => {}
    }
    Ok(inv_control_baseFUNC_MessageHandler(e, m, p))
}

pub fn inv_controlConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    if e.rt.cur_build() == 1 {
        return l03a::inv_controlConstructor(e, api, instanceCount);
    }
    inv_control_baseConstructor(e, api, instanceCount);
    if instanceCount == 1 {
        APIcons::SetExternalFunction(e, 7, inv_controlST_FIRSTMOVE);
        APIcons::SetExternalFunction(e, 8, inv_controlST_BASICMOVE);
        inv_control_ctor_globals(e);
    }
    APIcons::SetMessageFunction(e, inv_controlFUNC_MessageFunction);
    inv_control_ctor_local(e, 0xbe44);
    APIcons::SetInitialState(e, 7);
    Ok(())
}

pub(crate) fn inv_control_ctor_globals(e: &mut Engine) {
    let v = APIcons::GetApiGraphics(e);
    let a = G(e, pGraphics);
    e.w32(a, v);
    let slot = G(e, pGlobalDataInvCtrl);
    APIcons::GetGlobalData(e, slot, 0xc);
    let gd = e.r32(slot);
    let b = APIgameobj::GetGameBuffer(e, 0, 0x8f8, 1);
    e.w32(gd, b);
    let b = APIgameobj::GetGameBuffer(e, 2, 0x158, 1);
    e.w32(gd + 4, b);
}

pub(crate) fn inv_control_ctor_local(e: &mut Engine, size: i32) {
    let slot = G(e, pLocalDataInvCtrl);
    APIcons::GetLocalData(e, slot, size);
    LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 2, NULL, 0);
}

pub fn inv_controlDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    if e.rt.cur_build() == 1 {
        let t = e.r32(LC(e) + 0xc024);
        APIextra::TimerDestroy(e, t);
    }
    inv_control_baseDestructor(e, instanceCount);
    Ok(())
}

/// inv_control.c of EP1L03A (inv_control__7c7df8a6)
pub mod l03a {
    use super::*;
    const RADAR: u32 = 0x2a10_0000;
    /// pPhysic (same slot as L01's pGraphics)
    pub const pPhysic: u32 = super::pGraphics;

    /// state 0xc
    pub fn inv_controlST_FIRSTMOVE(e: &mut Engine, p: u32) -> LResult<u64> {
        inv_control_baseST_FIRSTMOVE(e, p)?;
        APInewSound::Play(e, SELF, 0x8000_0000, 0);
        let t = APIextra::TimerCreate(e);
        let l = LC(e);
        e.w32(l + 0xc024, t);
        e.w32(l + 0xc02c, 1);
        APIgameobj::SetNextState(e, SELF, 0xd);
        Ok(0)
    }

    /// state 0xd
    pub fn inv_controlST_BASICMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
        let v = e.r32(GDC(e, 0) + 0x3c);
        if v < 10 || 0xc < v {
            send(e, RADAR, 0x2a, 5);
        }
        APIgameobj::SetNextState(e, SELF, 0xe);
        Ok(0)
    }

    /// state 0xe
    pub fn inv_controlST_BASICMOVE_LOOP(e: &mut Engine, _: u32) -> LResult<u64> {
        inv_control_base_Update_Music_Volume(e);
        Ok(0)
    }

    /// state 10
    pub fn inv_controlST_BLACK_HOLE_LOOP(e: &mut Engine, _: u32) -> LResult<u64> {
        inv_control_base_Update_Music_Volume(e);
        Ok(0)
    }

    fn g4(e: &Engine) -> u32 {
        e.r32(GDC(e, 1) + 4)
    }
    fn f20(e: &Engine) -> u32 {
        e.r32(LC(e) + 0xc020)
    }
    fn set20(e: &mut Engine, v: u32) {
        let a = LC(e) + 0xc020;
        e.w32(a, v);
    }

    /// ext fn 0x11 (param = timer)
    pub fn inv_controlCB_TIMER_END(e: &mut Engine, p: u32) -> LResult<u64> {
        let l = LC(e);
        if p == e.r32(l + 0xbff4) {
            APInewSound::StopSound(e, SELF, 0x8000_0000);
            for (o, v) in [(0xbfa0, 99), (0xbfdc, 0), (0xbfe0, 3), (0xbfe4, 0)] {
                e.w32(l + o, v);
            }
            APIgameobj::SendMessageTo(e, 0x2a00_004a, NULL, 0x54, 0, l + 0xbfa0);
            send(e, RADAR, 0x1e, 0);
            let w = e.r32(l + 0xbff8);
            APIhud::WindowResetFlags(e, w, 0x100);
            let t = e.r32(l + 0xbff4);
            APIextra::TimerUnregisterNotify(e, SELF, t);
        }
        if p == e.r32(l + 0xc024) {
            let g = g4(e);
            if g & 0x10 == 0 || g & 0x40 == 0 || g & 0x100 == 0 {
                set20(e, f20(e) & 0xffff_fffd);
            }
            if g & 0x20 == 0 || g & 0x80 == 0 {
                set20(e, f20(e) & 0xffff_fffb);
            }
            let t = e.r32(l + 0xc024);
            APIextra::TimerUnregisterNotify(e, SELF, t);
        }
        Ok(0)
    }

    fn arm(e: &mut Engine) {
        set20(e, 0);
        let t = e.r32(LC(e) + 0xc024);
        APIextra::TimerReset(e, t);
        APIextra::TimerSetFrequency(e, t, 5.0);
        APIextra::TimerRegisterNotify(e, SELF, t, 0x11);
    }

    pub fn inv_controlFUNC_MessageFunction(e: &mut Engine, m: u32, p: u32, _d: Ptr) -> LResult<u64> {
        match m {
            0xc9 | 0xc1 => arm(e),
            0x14e => {
                let g = g4(e);
                if g & 0x20 != 0 && g & 0x80 != 0 && g & 0x100 != 0 && f20(e) & 2 == 0 {
                    set20(e, f20(e) | 2);
                    send(e, 0x2a00_00e5, 0xdf, 0);
                    send(e, RADAR, 0x2a, 7);
                    send(e, RADAR, 0x2b, 7);
                }
                let g = g4(e);
                if g & 0x40 != 0 && g & 0x10 != 0 && f20(e) & 4 == 0 {
                    set20(e, f20(e) | 4);
                    send(e, 0x2a00_00e6, 0xdf, 0);
                }
            }
            0x200 => {
                let gb2 = GDC(e, 1);
                let v = e.r32(gb2) & 0xffff_fffe;
                e.w32(gb2, v);
                let r = e.alloca(0x20); // uStack_50: retriever {1, group 0x800}
                e.w32(r, 1);
                e.w32(r + 4, 0x800);
                APIgameobj::SendMessageTo(e, 0xff00_0010, r, 0xe7, 0, NULL);
            }
            0x52 => {
                let l = LC(e);
                if e.r32(l + 0xc02c) != 0 {
                    send(e, RADAR, 0x2a, 4);
                    send(e, RADAR, 0x2b, 4);
                    e.w32(l + 0xc02c, 0);
                }
            }
            _ => {}
        }
        Ok(inv_control_baseFUNC_MessageHandler(e, m, p))
    }

    pub fn inv_controlConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
        inv_control_baseConstructor(e, api, instanceCount);
        if instanceCount == 1 {
            let fns: [(i32, crate::logic::ExtFn); 5] = [
                (0xc, inv_controlST_FIRSTMOVE),
                (0xd, inv_controlST_BASICMOVE),
                (0xe, inv_controlST_BASICMOVE_LOOP),
                (10, inv_controlST_BLACK_HOLE_LOOP),
                (0x11, inv_controlCB_TIMER_END),
            ];
            for (i, f) in fns {
                APIcons::SetExternalFunction(e, i, f);
            }
            let v = APIcons::GetApiPhysic(e);
            let a = G(e, pPhysic);
            e.w32(a, v);
            let slot = G(e, pGlobalDataInvCtrl);
            APIcons::GetGlobalData(e, slot, 0xc);
            let gd = e.r32(slot);
            let b = APIgameobj::GetGameBuffer(e, 0, 0x8f8, 1);
            e.w32(gd, b);
            let b = APIgameobj::GetGameBuffer(e, 2, 0x158, 1);
            e.w32(gd + 4, b);
        }
        APIcons::SetMessageFunction(e, inv_controlFUNC_MessageFunction);
        let slot = G(e, pLocalDataInvCtrl);
        APIcons::GetLocalData(e, slot, 0xc030);
        // *(pGlobalDataInvCtrlBase + 4) is GB2 (int-indexed [1])
        let gb2 = GDB(e, 1);
        let v = e.r32(gb2 + 0x28) | 4;
        e.w32(gb2 + 0x28, v);
        let gb2c = GDC(e, 1);
        if e.r32(gb2c) & 1 != 0 && e.r32(gb2c + 8) & 0x80_0000 == 0 {
            let v = e.r32(gb2c) & 0xffff_fffe;
            e.w32(gb2c, v);
        }
        APIcons::SetInitialState(e, 0xc);
        Ok(())
    }
}

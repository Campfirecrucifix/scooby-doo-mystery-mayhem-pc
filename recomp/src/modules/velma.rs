//! `velma`: EP1L01 Velma (0x2a000000 = SELF), the clue giver. Touching her shows the X prompt; X plays
//! the clue VO (0x80000024/25). After msg 0x107 (GB0+0x40 bit 0x40000 clear) she plays intro VO
//! 0x8000000c; the clue cut-scene looks at 0x2a000097 (camera 0x2a00008f, texts 0x152/0x153).
//! Port of `Game Logic Modules/Ep1_Library/Characters/VELMA/LEVEL01/velma__0a5c1ccf/` (Velma.c 11 fns
//! + Velma_base.c 22 fns). Velma_base.c is the source of [`super::velmanew`] (EP1L03A) with level
//! literals (radar 0x2a100006, camera 0x2a00008f, music 0x2a000096, anims 1/3/2/4/6) and debug
//! prints; the L01 base X callback stops after marking clues (the clue VO is in _X_LOCAL).
#![allow(non_snake_case, non_upper_case_globals, unused_variables)]

use crate::engine::*;
use crate::logic::{ExtFn, LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "velma",
    // globals 0x10000..0x1004c (+ unused gpoLanguagesNamesAndISOCodes)
    data_size: 0x4c,
    ctor: velmaConstructor,
    dtor: velmaDestructor,
    register: None,
    init: &[],
    build_init: None,
};

pub fn build_index(key: &str) -> Option<usize> {
    (key == "0a5c1ccf").then_some(0)
}

pub const pExtra: u32 = 0x10000;
pub const pGameObj: u32 = 0x10004;
pub const pPhysic: u32 = 0x10008;
pub const pParticle: u32 = 0x1000c;
pub const pMath: u32 = 0x10010;
pub const pStdlib: u32 = 0x10014;
pub const pLight: u32 = 0x10018;
pub const pAnim: u32 = 0x1001c;
pub const pSound: u32 = 0x10020;
pub const pGraphic: u32 = 0x10024;
pub const pDebug: u32 = 0x10028;
pub const pAnimMngr: u32 = 0x1002c;
pub const pFunction: u32 = 0x10030;
pub const pText: u32 = 0x10034;
pub const pInput: u32 = 0x10038;
pub const pMiscFunc: u32 = 0x1003c;
pub const pSectorClip: u32 = 0x10040;
pub const pLocalDatavelma_base: u32 = 0x10044;
pub const pGlobalDatavelma_base: u32 = 0x10048;

#[inline]
fn G(e: &Engine, a: u32) -> Ptr {
    e.rt.cur_data() + (a - 0x10000)
}
/// pLocalDatavelma_base (0x18c bytes): +0 cut-scene clipping, +0x40 own clipping, +0x80 text struct,
/// +0xb4 camera params, +0x100 sector-clip record, +0x10c start pos, +0x118 DM, +0x11c idle timer,
/// +0x120 talk timer, +0x124 collide timer, +0x128/+0x130 local timers, +0x134 VO timer, +0x144 VO id,
/// +0x14c anim, +0x158.. flags
#[inline]
fn L(e: &Engine) -> Ptr {
    e.r32(G(e, pLocalDatavelma_base))
}
#[inline]
fn l(e: &Engine, o: u32) -> u32 {
    e.r32(L(e) + o)
}
#[inline]
fn lw(e: &mut Engine, o: u32, v: u32) {
    let a = L(e) + o;
    e.w32(a, v);
}
/// game buffer n (pGlobalDatavelma_base[n]): 0 = 0x8f8 (heroes), 1 = 0x68, 2 = 0x158 (GB2), 3 = 0x40
#[inline]
fn GB(e: &Engine, n: u32) -> Ptr {
    e.r32(e.r32(G(e, pGlobalDatavelma_base)) + 4 * n)
}
fn gb_or(e: &mut Engine, n: u32, o: u32, set: u32, clear: u32) {
    let a = GB(e, n) + o;
    let v = (e.r32(a) | set) & !clear;
    e.w32(a, v);
}
fn play(e: &mut Engine, blend: f32, anim: i32, mode: i32, f: i32) {
    let v = LIBAnimMngr::PlayAnim(e, blend, anim, mode, f);
    lw(e, 0x14c, v as u32);
}
fn send(e: &mut Engine, to: u32, msg: u32, a: u32, d: Ptr) {
    APIgameobj::SendMessageTo(e, to, NULL, msg, a, d);
}
/// `if (pDebug != NULL) pDebug->PrintDebugInfo[WithValue]("Velma", msg[, &local[o], 1])`
fn dbg(e: &mut Engine, msg: &str, o: Option<u32>) {
    if e.r32(G(e, pDebug)) == 0 {
        return;
    }
    let (a, b) = (e.cstr("Velma"), e.cstr(msg));
    match o {
        Some(o) => {
            let v = L(e) + o;
            LIBdebug::PrintDebugInfoWithValue(e, a, b, v, 1);
        }
        None => {
            LIBdebug::PrintDebugInfo(e, a, b);
        }
    }
}
fn rand_idle_freq(e: &mut Engine) {
    let t = l(e, 0x11c);
    APIextra::TimerReset(e, t);
    let f = LIBApiCFunctions::FloatRandom(e, 2.5, 3.5, 0.05);
    APIextra::TimerSetFrequency(e, t, f);
}

// ---------------------------------------------------------------- Velma_base.c

/// 0x00010d88 state 0
pub fn fstate_FIRSTMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    if l(e, 0x158) != 0 {
        let t = l(e, 0x11c);
        APIextra::TimerRegisterNotify(e, SELF, t, 0xc);
        velma_baseEnableDisableIdle(e, 1);
    }
    APIgameobj::SetNextState(e, SELF, 2);
    Ok(0)
}

/// 0x00010e38 state 2
pub fn fstate_BASICMOVE_SET(e: &mut Engine, _: u32) -> LResult<u64> {
    APIgameobj::SetNextState(e, SELF, 5);
    Ok(0)
}

/// 0x00010e90 state 5
pub fn fstate_BLACK_HOLE_LOOP(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

/// 0x00010ec0 callback 6 (base): leader touch -> X prompt (0x31/10 to 0x2a100000)
pub fn fcback_NOTIFY_COLLIDE_OBJ(e: &mut Engine, p: u32) -> LResult<u64> {
    let gb0 = GB(e, 0);
    if e.r32(gb0 + 8) != e.r32(p + 8) || e.r32(gb0 + 0x14) & 0x6e20094d != 0 {
        return Ok(0);
    }
    let t = l(e, 0x124);
    if !(1.0 <= APIextra::TimerGetTime(e, t)) {
        return Ok(0);
    }
    lw(e, 0x174, 1);
    if l(e, 0x178) == 0 && l(e, 0x17c) == 0 && e.r32(GB(e, 2)) & 0x20000 == 0 {
        let s = L(e) + 0x80;
        LIBApiText::InitStruct(e, s);
        lw(e, 0xa8, 0x37);
        let c = L(e) + 0x40;
        LIBMiscFunc::MiscFunc_RemoveBitClipping(e, c, 0x80);
        APIgameobj::SetContextFlag(e, SELF, 1);
        APIinput::InputConfigActivate(e, 0x12, 1);
        lw(e, 0x178, 1);
        send(e, 0x2a10_0006, 0x31, 10, NULL);
    }
    Ok(0)
}

/// 0x000110c8
pub fn ResetCollision(e: &mut Engine) {
    if e.r32(GB(e, 2)) & 0x20000 != 0 {
        send(e, 0x2a10_0006, 0x32, 10, NULL);
    }
    let c = L(e) + 0x40;
    LIBMiscFunc::MiscFunc_ReplaceMyClipping(e, c);
    APIinput::InputConfigActivate(e, 0x12, 0);
    lw(e, 0x178, 0);
    APIgameobj::SetContextFlag(e, SELF, 0);
}

/// 0x000111a0 callback 7
pub fn fcback_NOTIFY_COLLIDE_POST(e: &mut Engine, _: u32) -> LResult<u64> {
    if l(e, 0x174) == 0 && l(e, 0x178) == 1 {
        ResetCollision(e);
    }
    lw(e, 0x174, 0);
    Ok(0)
}

/// 0x00011230 callback 8 (base)
pub fn fcback_CUTSCENE(e: &mut Engine, p: u32) -> LResult<u64> {
    match e.r32(p + 4) {
        1 => {
            APIgameobj::SetNextState(e, SELF, 5);
        }
        0 => {
            APIgameobj::SetNextState(e, SELF, 2);
        }
        _ => {}
    }
    Ok(0)
}

/// 0x00011440 callback 9 (base): X pressed -> marks new clues (GB1+0x10) seen
pub fn fcback_InputActionVelma_X(e: &mut Engine, _: u32) -> LResult<u64> {
    lw(e, 0x138, 1);
    lw(e, 0x140, 0);
    LIBApiText::KillText(e);
    if l(e, 0x170) == 0 {
        play(e, 0.2, 1, 2, 0);
        velma_baseEnableDisableIdle(e, 0);
        for bit in [0x20u32, 0x40, 0x80, 0x100, 0x200] {
            let (g2, g1) = (GB(e, 2), GB(e, 1));
            if e.r32(g2 + 0xc) & bit != 0 && e.r32(g1 + 0x10) & bit == 0 {
                gb_or(e, 1, 0x10, bit, 0);
                lw(e, 0x140, 1);
            }
        }
    }
    // (the L03A build continues with the clue VO here; the L01 build does it in _X_LOCAL)
    Ok(1)
}

/// 0x000116c8 callback 0xb
pub fn fcback_LF_ANIMATION(e: &mut Engine, _: u32) -> LResult<u64> {
    play(e, 0.2, 1, 1, 0);
    velma_baseEnableDisableIdle(e, 1);
    Ok(1)
}

/// 0x00011758 callback 0xc (base)
pub fn fcback_TIMER(e: &mut Engine, p: u32) -> LResult<u64> {
    if p == l(e, 0x134) {
        if l(e, 0x188) != 0 {
            return Ok(0);
        }
        LIBApiText::KillText(e);
        velma_baseFadeMusVol(e, 1);
        let vo = l(e, 0x144);
        APInewSound::StopSound(e, SELF, vo);
        lw(e, 0x144, 0);
        let t = l(e, 0x124);
        APIextra::TimerReset(e, t);
        play(e, 0.1, 1, 2, 0);
        velma_baseFUNC_CutSceneManager(e, 0);
        velma_baseEnableDisableIdle(e, 1);
        let t = l(e, 0x120);
        APIextra::TimerUnregisterNotify(e, SELF, t);
        rand_idle_freq(e);
        Ok(1)
    } else if p == l(e, 0x120) {
        dbg(e, "fcback_TIMER - Sound Is not completed somehow...", Some(0x144));
        dbg(e, "fcback_TIMER - *******************************TimerVoice", Some(0x120));
        let t = l(e, 0x120);
        APIextra::TimerUnregisterNotify(e, SELF, t);
        let vo = l(e, 0x144);
        APInewSound::StopSound(e, SELF, vo);
        lw(e, 0x144, 0);
        let t = l(e, 0x124);
        APIextra::TimerReset(e, t);
        play(e, 0.1, 1, 2, 0);
        if l(e, 0x138) == 0 {
            lw(e, 0x178, 0);
            lw(e, 0x17c, 0);
        } else {
            rand_idle_freq(e);
            velma_baseEnableDisableIdle(e, 1);
            velma_baseFUNC_CutSceneManager(e, 0);
        }
        Ok(1)
    } else {
        if p == l(e, 0x11c) {
            dbg(e, "fcback_TIMER - *******************************uiTimerIdle", Some(0x11c));
            // pLVar2 (pAnimMngr) is read before the random pick; same table
            let a = velma_baseGetRandIdleAnim(e);
            play(e, 0.2, a, 1, 0xb);
            velma_baseEnableDisableIdle(e, 0);
        }
        Ok(0)
    }
}

/// 0x00011b78 callback 0xd
pub fn fcback_SOUNDCOMPLETION(e: &mut Engine, p: u32) -> LResult<u64> {
    if e.r32(p + 4) == l(e, 0x144) && l(e, 0x188) == 0 {
        dbg(e, "fcback_SOUNDCOMPLETION - ***********************************Sound completed", None);
        LIBApiText::KillText(e);
        velma_baseFadeMusVol(e, 1);
        let vo = l(e, 0x144);
        APInewSound::StopSound(e, SELF, vo);
        lw(e, 0x144, 0);
        let t = l(e, 0x124);
        APIextra::TimerReset(e, t);
        play(e, 0.1, 1, 2, 0);
        let t = l(e, 0x134);
        APIextra::TimerUnregisterNotify(e, SELF, t);
        velma_baseFUNC_CutSceneManager(e, 0);
        velma_baseEnableDisableIdle(e, 1);
        let t = l(e, 0x120);
        APIextra::TimerUnregisterNotify(e, SELF, t);
        rand_idle_freq(e);
    }
    if l(e, 0x188) != 0 {
        let t = l(e, 0x134);
        APIextra::TimerReset(e, t);
        APIextra::TimerSetFrequency(e, t, 0.5);
        APIextra::TimerRegisterNotify(e, SELF, t, 0xc);
    }
    dbg(e, "fcback_SOUNDCOMPLETION - Sound Failed to match ID", None);
    Ok(1)
}

/// 0x00011e10
pub fn velma_base_MessageFunction(e: &mut Engine, m: u32) -> LResult<u64> {
    match m {
        0x10c => {
            APIgameobj::SetNextState(e, SELF, 5);
        }
        0x12 => {
            lw(e, 0x188, 1);
            if APIinput::ConfigIsActive(e, 0x12) != 0 {
                lw(e, 0x168, 1);
                APIinput::InputConfigActivate(e, 0x12, 0);
            }
            let t = l(e, 0x120);
            APIextra::TimerPause(e, t, 1);
        }
        0x13 => {
            lw(e, 0x188, 0);
            if l(e, 0x168) != 0 {
                lw(e, 0x168, 0);
                APIinput::InputConfigActivate(e, 0x12, 1);
                ResetCollision(e);
                lw(e, 0x174, 0);
            }
            let t = l(e, 0x120);
            APIextra::TimerPause(e, t, 0);
        }
        0x201 => {
            APIgameobj::EnableObject(e, SELF, 1);
        }
        0x202 => {
            APIgameobj::EnableObject(e, SELF, 0);
        }
        _ => {}
    }
    Ok(0)
}

/// 0x00012048
pub fn velma_baseCreate_Physic(e: &mut Engine) {
    APIphysic::CreatePhysic(e, SELF, 0);
    APIphysic::SetFlags(e, SELF, 2);
    APIphysic::SetWeight(e, SELF, 1.0);
    APIphysic::SetPriority(e, SELF, 100);
    let dm = APIphysic::DMAdd(e, SELF, 0x25);
    lw(e, 0x118, dm);
    let p = e.alloca(0x10); // auStack_30
    APIphysic::DMGetParams(e, dm, p);
    e.w32(p + 4, 7);
    APIphysic::DMSetParams(e, dm, p);
    APIphysic::DMSetContextMask(e, dm, 1);
    APIgameobj::SetContextFlag(e, SELF, 0);
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 3, 6);
}

/// 0x000121c8
pub fn velma_baseCreate_Input(e: &mut Engine) {
    APIinput::InputConfigAdd(e, 0x12);
    APIinput::InputActionAdd(e, 0x12, 0x39, 0);
    let pad = e.r32(GB(e, 0) + 0x4c) as i32;
    APIinput::InputDeviceAdd(e, 0x12, 0x39, 0x3d, pad, 0, 2, 0, 0);
    APIinput::InputDeviceAdd(e, 0x12, 0x39, 0x3d, 1, 1, 2, 0, 0x39);
    APIinput::InputContextAdd(e, SELF, 9, 0x12, 0x39, 0x1e, 8);
    APIinput::InputContextSetState(e, 0x12, 0x39, 0x1e, 1);
    APIinput::InputConfigSetState(e, 0x12, 1);
    APIinput::InputConfigActivate(e, 0x12, 0);
}

/// 0x000124d0 (rand%100: <31 -> 3, 31..50 -> 2, 51..70 -> 4, else 6)
pub fn velma_baseGetRandIdleAnim(e: &mut Engine) -> i32 {
    let r = (APIstdlib::MyRand(e) as i32) % 100;
    if r < 0x1f {
        3
    } else if r <= 0x32 {
        2
    } else if r <= 0x46 {
        4
    } else {
        6
    }
}

/// 0x00012420
pub fn velma_baseEnableDisableIdle(e: &mut Engine, on: i32) {
    if on == 1 {
        rand_idle_freq(e);
    } else {
        let t = l(e, 0x11c);
        APIextra::TimerReset(e, t);
        APIextra::TimerSetFrequency(e, t, 0.0);
    }
}

/// 0x00012530
pub fn velma_baseFUNC_CutSceneManager(e: &mut Engine, which: u32) {
    let cp = L(e) + 0xb4;
    if which == 1 {
        lw(e, 0x160, 1);
        lw(e, 0xb4, 99);
        lw(e, 0xf0, 1);
        lw(e, 0xf4, 0);
        lw(e, 0xf8, 0);
        send(e, 0x2a00_008f, 0x54, 0, cp);
    } else if which == 0x2a00_0097 {
        lw(e, 0x160, 1);
        lw(e, 0xb4, 4);
        lw(e, 0xd0, 0x2a00_0097);
        lw(e, 0xd4, 0x2d);
        lw(e, 0xd8, 0x2a00_0097);
        lw(e, 0xc4, 0);
        lw(e, 0xc8, 0x3fa0_0000);
        lw(e, 0xcc, 0);
        lw(e, 0xf0, 1);
        lw(e, 0xf4, 0);
        lw(e, 0xf8, 0x2000_211e);
        send(e, 0x2a00_008f, 0x54, 0, cp);
        send(e, 0x2a10_0006, 0x1e, 0, NULL);
        let v = e.alloca(0x10); // DStack_50
        for (hero, bone) in [(8u32, 0x2bu32), (0xc, 0x2c)] {
            if hero == 0xc && e.r32(GB(e, 0) + 0x34) & 0x10 == 0 {
                break;
            }
            let h = e.r32(GB(e, 0) + hero);
            let p = APIgameobj::GetPosition(e, SELF, bone, v);
            APIgameobj::SetTranslation(e, h, p);
            let h = e.r32(GB(e, 0) + hero);
            APIgameobj::TakeOrientation(e, h, SELF, bone);
        }
    } else {
        lw(e, 0x160, 0);
        APIinput::InputConfigActivate(e, 0x12, 0);
        send(e, 0x2a00_008f, 0x55, 0, NULL);
        send(e, 0x2a10_0006, 0x1f, 0, NULL);
        send(e, 0x2a00_0020, 0x46, 0, NULL);
        lw(e, 0x178, 0);
        lw(e, 0x17c, 0);
    }
}

/// 0x00012900
pub fn velma_basePlayVO(e: &mut Engine) {
    dbg(e, "velma_basePlayVO", Some(0x144));
    let vo = l(e, 0x144);
    APIgameobj::RegisterNotify(e, SELF, vo, 0xf, 0xd);
    APInewSound::Play(e, SELF, vo, 0);
}

/// 0x00012980 (music fade via msg 0x5e to 0x2a0000ed; the return value is unused by callers)
pub fn velma_baseFadeMusVol(e: &mut Engine, dir: i32) {
    if dir == 0 && l(e, 0x164) == 1 {
        send(e, 0x2a00_0096, 0x5e, 1, NULL);
        lw(e, 0x164, 0);
        let s = e.cstr("\t\t\t\t\tFade <<<<<<<<<\n");
        APIextra::null_printf(e, s, 0);
    } else if dir == 1 && l(e, 0x164) == 0 {
        send(e, 0x2a00_0096, 0x5e, 0, NULL);
        lw(e, 0x164, 1);
        let s = e.cstr("\t\t\t\t\tFade >>>>>>>>>\n");
        APIextra::null_printf(e, s, 0);
    }
}

/// 0x00012aa8
pub fn velma_baseClues(e: &mut Engine) {
    let vo = if l(e, 0x140) == 1 { 0x8000_0025 } else { 0x8000_0024 };
    lw(e, 0x144, vo);
    velma_basePlayVO(e);
    lw(e, 0x140, 0);
}

/// 0x00012b50
pub fn velma_baseConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) {
    if instanceCount == 1 {
        let fns: [(i32, ExtFn); 10] = [
            (0, fstate_FIRSTMOVE),
            (2, fstate_BASICMOVE_SET),
            (5, fstate_BLACK_HOLE_LOOP),
            (6, fcback_NOTIFY_COLLIDE_OBJ),
            (7, fcback_NOTIFY_COLLIDE_POST),
            (8, fcback_CUTSCENE),
            (9, fcback_InputActionVelma_X),
            (0xb, fcback_LF_ANIMATION),
            (0xc, fcback_TIMER),
            (0xd, fcback_SOUNDCOMPLETION),
        ];
        for (i, f) in fns {
            APIcons::SetExternalFunction(e, i, f);
        }
        let tables: [(u32, fn(&mut Engine) -> Ptr); 11] = [
            (pGameObj, |e| APIcons::GetApiGameObj(e)),
            (pExtra, |e| APIcons::GetApiExtra(e)),
            (pPhysic, |e| APIcons::GetApiPhysic(e)),
            (pStdlib, |e| APIcons::GetApiStdlib(e)),
            (pParticle, |e| APIcons::GetApiParticle(e)),
            (pMath, |e| APIcons::GetApiMath(e)),
            (pLight, |e| APIcons::GetApiLight(e)),
            (pGraphic, |e| APIcons::GetApiGraphics(e)),
            (pSound, |e| APIcons::GetApiNewSound(e)),
            (pInput, |e| APIcons::GetApiInput(e)),
            (pAnim, |e| APIcons::GetApiAnim(e)),
        ];
        for (slot, get) in tables {
            let v = get(e);
            let a = G(e, slot);
            e.w32(a, v);
        }
        for (slot, name) in [
            (pDebug, "debug"),
            (pAnimMngr, "AnimMngr"),
            (pFunction, "ApiCFunctions"),
            (pText, "ApiText"),
            (pMiscFunc, "MiscFunc"),
            (pSectorClip, "ApiSectorClipping"),
        ] {
            let n = e.cstr(name);
            let v = APIcons::GetApiCustom(e, n);
            let a = G(e, slot);
            e.w32(a, v);
        }
        let slot = G(e, pGlobalDatavelma_base);
        APIcons::GetGlobalData(e, slot, 0x10);
        let gd = e.r32(slot);
        for (n, size) in [(0u32, 0x8f8), (1, 0x68), (2, 0x158), (3, 0x40)] {
            let b = APIgameobj::GetGameBuffer(e, n, size, 1);
            e.w32(gd + 4 * n, b);
        }
    }
    let slot = G(e, pLocalDatavelma_base);
    APIcons::GetLocalData(e, slot, 0x18c);
    velma_baseCreate_Physic(e);
    velma_baseCreate_Input(e);
    LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 2, NULL, 0);
    APIanim::Play(e, SELF, 1, 2, 0.0);
    lw(e, 0x15c, 0);
    lw(e, 0x158, 1);
    lw(e, 0x164, 1);
    for o in [0x11c, 0x120, 0x124, 0x134] {
        let t = APIextra::TimerCreate(e);
        lw(e, o, t);
    }
    // srand seed: Ghidra shows the ChangeClipping f0; the f0 at the cvt is TimerGetTime's result
    let t = APIextra::TimerGetTime(e, 0x9000_0000);
    APIstdlib::srand(e, t as i32 as u32);
    let id = APIgameobj::GetID(e, SELF, NULL);
    lw(e, 0x100, id);
    let a = L(e);
    e.w16(a + 0x10a, 2);
    e.w16(a + 0x108, 0);
    LIBApiSectorClipping::ApiImpl_Register(e, a + 0x100);
    lw(e, 0x188, 0);
    APIcons::SetInitialState(e, 0);
}

/// 0x00013138
pub fn velma_baseDestructor(e: &mut Engine) {
    for o in [0x11c, 0x120, 0x134] {
        let t = l(e, o);
        APIextra::TimerDestroy(e, t);
    }
    let a = L(e) + 0x100;
    LIBApiSectorClipping::ApiImpl_Unregister(e, a);
}


// ---------------------------------------------------------------- Velma.c

/// 0x000133f0 message function
pub fn velmaFUNC_MessageHandler(e: &mut Engine, m: u32, _s: u32, _d: Ptr) -> LResult<u64> {
    if m == 0x107 {
        lw(e, 0x140, 1);
        if e.r32(GB(e, 0) + 0x40) & 0x40000 == 0 {
            send(e, 0x2a10_0006, 0x183, 1, NULL);
            velma_baseFUNC_CutSceneManager(e, 1);
            velma_baseFadeMusVol(e, 0);
            lw(e, 0x144, 0x8000_000c);
            velma_basePlayVO(e);
            lw(e, 0x15c, 1);
            lw(e, 0x170, 1);
            let t = l(e, 0x128);
            APIextra::TimerReset(e, t);
            // GB3+0x24 = language id (0x24 / 0x2a get 10 s, others 9 s)
            let lang = e.r32(GB(e, 3) + 0x24);
            let f = if lang == 0x24 || lang == 0x2a { 10.0 } else { 9.0 };
            APIextra::TimerSetFrequency(e, t, f);
            APIextra::TimerRegisterNotify(e, SELF, t, 0xc);
            APIinput::InputConfigActivate(e, 0x12, 1);
        }
    }
    velma_base_MessageFunction(e, m)
}

/// 0x00013670
pub fn Reset_Velma_After_CluePicked(e: &mut Engine) {
    LIBApiText::KillText(e);
    send(e, 0x2a10_0006, 0x1e, 0, NULL);
    lw(e, 0x138, 0);
    velma_baseFadeMusVol(e, 1);
    velma_baseFUNC_CutSceneManager(e, 0);
    APIgameobj::UnregisterNotify(e, SELF, 0x8000_0024, 0xf);
    for o in [0x128, 0x120] {
        let t = l(e, o);
        APIextra::TimerUnregisterNotify(e, SELF, t);
    }
    if e.r32(GB(e, 0) + 0x14) & 0x40 != 0 {
        send(e, 0x2a00_013f, 0x1e7, 0, NULL);
        send(e, 0x2a10_0006, 0x183, 0, NULL);
    }
    if l(e, 0x144) == 0x8000_000c {
        APIgameobj::UnregisterNotify(e, SELF, 0x8000_000c, 0xf);
    }
    lw(e, 0x170, 0);
    lw(e, 0x15c, 0);
    let vo = l(e, 0x144);
    APInewSound::StopSound(e, SELF, vo);
    lw(e, 0x144, 0);
    let t = l(e, 0x124);
    APIextra::TimerReset(e, t);
    APIgameobj::SetNextState(e, SELF, 5);
}

/// 0x000138b8 callback 9
pub fn fcback_InputActionVelma_X_LOCAL(e: &mut Engine, p: u32) -> LResult<u64> {
    fcback_InputActionVelma_X(e, p)?;
    if l(e, 0x15c) == 0 || l(e, 0x170) == 0 {
        velma_LocalClues(e);
        lw(e, 0x17c, 1);
        let t = l(e, 0x120);
        APIextra::TimerReset(e, t);
        APIextra::TimerRegisterNotify(e, SELF, t, 0xc);
        send(e, 0x2a10_0006, 0x32, 10, NULL);
        APIinput::InputConfigActivate(e, 0x12, 0);
    } else {
        Reset_Velma_After_CluePicked(e);
    }
    Ok(0)
}

/// 0x00013a00 callback 10: skip the clue cut-scene (input config 0x13) after 2 s
pub fn fcback_InputActionVelma_X_Skipable(e: &mut Engine, _: u32) -> LResult<u64> {
    let t = l(e, 0x12c);
    if 2.0 <= APIextra::TimerGetTime(e, t) {
        if l(e, 0x17c) != 0 {
            LIBApiText::KillText(e);
            LIBApiText::KillText(e);
            velma_baseFadeMusVol(e, 1);
            let vo = l(e, 0x144);
            APInewSound::StopSound(e, SELF, vo);
            lw(e, 0x144, 0);
            let t = l(e, 0x124);
            APIextra::TimerReset(e, t);
            play(e, 0.1, 1, 2, 0);
            let t = l(e, 0x120);
            APIextra::TimerUnregisterNotify(e, SELF, t);
            rand_idle_freq(e);
            velma_baseEnableDisableIdle(e, 1);
            velma_baseFUNC_CutSceneManager(e, 0);
        }
        APIinput::InputConfigActivate(e, 0x13, 0);
    }
    Ok(0)
}

/// 0x00013c40
pub fn velma_LocalClues(e: &mut Engine) {
    let g40 = GB(e, 0) + 0x40;
    let f = e.r32(g40);
    if f & 0x40000 == 0 || f & 0x80000 != 0 {
        if f & 0x100000 == 0 && f & 0x80000 == 0 {
            e.w32(g40, f | 0x100000);
            let t = l(e, 0x120);
            APIextra::TimerSetFrequency(e, t, 13.5);
            lw(e, 0x144, 0x8000_0024);
            velma_baseFUNC_CutSceneManager(e, 0x2a00_0097);
            velma_basePlayVO(e);
            velma_baseFadeMusVol(e, 0);
            play(e, 0.1, 7, 1, 0xb);
            let t = l(e, 0x12c);
            APIextra::TimerReset(e, t);
            APIinput::InputConfigActivate(e, 0x13, 1);
        } else {
            let vo = if l(e, 0x140) == 1 { 0x8000_0025 } else { 0x8000_0024 };
            lw(e, 0x144, vo);
            let t = l(e, 0x120);
            APIextra::TimerSetFrequency(e, t, 10.8);
            velma_basePlayVO(e);
            lw(e, 0x140, 0);
        }
    } else {
        e.w32(g40, f | 0x80000);
        lw(e, 0x80, 0x40a0_0000);
        lw(e, 0x144, 0x8000_000d);
        velma_baseFUNC_CutSceneManager(e, 0x2a00_0097);
        velma_basePlayVO(e);
        velma_baseFadeMusVol(e, 0);
        let s = L(e) + 0x80;
        LIBApiText::ShowText(e, 0x152, s);
        LIBApiText::ShowText(e, 0x153, s);
        lw(e, 0x140, 0);
        let t = l(e, 0x120);
        APIextra::TimerSetFrequency(e, t, 13.5);
        play(e, 0.1, 7, 1, 0xb);
        let t = l(e, 0x12c);
        APIextra::TimerReset(e, t);
        APIinput::InputConfigActivate(e, 0x13, 1);
    }
}

/// 0x00014070 callback 0xb
pub fn fcback_LOCAL_LF_ANIMATION(e: &mut Engine, p: u32) -> LResult<u64> {
    let a = l(e, 0x14c);
    if a == 7 || a == 9 || a == 8 {
        let t = velma_GetRandTalkAnim(e);
        play(e, 0.2, t, 1, 0xb);
        Ok(0)
    } else {
        fcback_LF_ANIMATION(e, p)
    }
}

/// 0x00014180 (rand%100: <36 -> 7, 36..65 -> 9, else 8)
pub fn velma_GetRandTalkAnim(e: &mut Engine) -> i32 {
    let r = (APIstdlib::MyRand(e) as i32) % 100;
    if r < 0x24 {
        7
    } else if r > 0x41 {
        8
    } else {
        9
    }
}

/// 0x00014248 callback 0xc
pub fn fcback_TIMER_LOCAL(e: &mut Engine, p: u32) -> LResult<u64> {
    if p == l(e, 0x128) {
        Reset_Velma_After_CluePicked(e);
        Ok(1)
    } else {
        fcback_TIMER(e, p)
    }
}

/// 0x000142d8
pub fn velmaFUNC_Create_Input(e: &mut Engine) {
    APIinput::InputConfigAdd(e, 0x13);
    APIinput::InputActionAdd(e, 0x13, 0x39, 0);
    let pad = e.r32(GB(e, 0) + 0x4c) as i32;
    APIinput::InputDeviceAdd(e, 0x13, 0x39, 0x3d, pad, 0, 2, 0, 0);
    APIinput::InputDeviceAdd(e, 0x13, 0x39, 0x3d, 1, 1, 2, 0, 0x39);
    APIinput::InputContextAdd(e, SELF, 10, 0x13, 0x39, 0x1e, 8);
    APIinput::InputContextSetState(e, 0x13, 0x39, 0x1e, 1);
    APIinput::InputConfigSetState(e, 0x13, 1);
    APIinput::InputConfigActivate(e, 0x13, 0);
}

pub fn velmaConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    velma_baseConstructor(e, api, instanceCount);
    if instanceCount == 1 {
        let fns: [(i32, ExtFn); 4] = [
            (9, fcback_InputActionVelma_X_LOCAL),
            (10, fcback_InputActionVelma_X_Skipable),
            (0xc, fcback_TIMER_LOCAL),
            (0xb, fcback_LOCAL_LF_ANIMATION),
        ];
        for (i, f) in fns {
            APIcons::SetExternalFunction(e, i, f);
        }
    }
    LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 2, NULL, 0);
    APIcons::SetMessageFunction(e, velmaFUNC_MessageHandler);
    velmaFUNC_Create_Input(e);
    for o in [0x128, 0x12c] {
        let t = APIextra::TimerCreate(e);
        lw(e, o, t);
    }
    APIgameobj::SetNextState(e, SELF, 0);
    Ok(())
}

pub fn velmaDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    for o in [0x128, 0x12c] {
        let t = l(e, o);
        APIextra::TimerDestroy(e, t);
    }
    velma_baseDestructor(e);
    Ok(())
}

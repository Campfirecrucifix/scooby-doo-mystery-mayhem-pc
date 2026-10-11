//! `velmanew`: EP1L03A Velma (0x2a000000 = SELF). Hands the Tome of Doom over: the leader touching
//! her the first time (GB2+8 bit 0x400000 clear) starts cut-scene 0x80000002 (msg 0x1de to the camera
//! 0x2a00004a); once GB2+0xc bit 0x2000000 is set (and GB2+8 bit 0x800000 still clear) the next touch
//! sends 0xb/0x10 to 0x2a100000, **msg 0x70 to the book 0x2a0000ee**, sets GB2+8 bit 0x800000 and
//! starts cut-scene 0x80000003. Otherwise she runs the Velma_base talk/idle logic.
//! Port of `Game Logic Modules/Ep1_Library/Characters/VELMA/LEVEL03A/velmanew/` (VelmaNew.c 8 fns +
//! Velma_base.c 22 fns). EP1L03A build 10db7d06.
#![allow(non_snake_case, non_upper_case_globals, unused_variables)]

use crate::engine::*;
use crate::logic::{ExtFn, LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "velmanew",
    // globals 0x10000..0x1004c (+ unused gpoLanguagesNamesAndISOCodes)
    data_size: 0x4c,
    ctor: velmanewConstructor,
    dtor: velmanewDestructor,
    register: None,
    init: &[],
    build_init: None,
};

pub fn build_index(key: &str) -> Option<usize> {
    (key == "10db7d06").then_some(0)
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
        send(e, 0x2a10_0000, 0x31, 10, NULL);
    }
    Ok(0)
}

/// 0x000110c8
pub fn ResetCollision(e: &mut Engine) {
    if e.r32(GB(e, 2)) & 0x20000 != 0 {
        send(e, 0x2a10_0000, 0x32, 10, NULL);
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

/// 0x00011230 callback 8 (base; replaced by fcback_CUTSCENE_LOCAL in velmanew)
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

/// 0x000112d8 callback 9: X pressed -> clue VO, talk timer 10.8 s
pub fn fcback_InputActionVelma_X(e: &mut Engine, _: u32) -> LResult<u64> {
    lw(e, 0x138, 1);
    lw(e, 0x140, 0);
    LIBApiText::KillText(e);
    if l(e, 0x170) == 0 {
        play(e, 0.2, 2, 2, 0);
        velma_baseEnableDisableIdle(e, 0);
        for bit in [0x20u32, 0x40, 0x80, 0x100, 0x200] {
            let (g2, g1) = (GB(e, 2), GB(e, 1));
            if e.r32(g2 + 0xc) & bit != 0 && e.r32(g1 + 0x10) & bit == 0 {
                gb_or(e, 1, 0x10, bit, 0);
                lw(e, 0x140, 1);
            }
        }
    }
    velma_baseClues(e);
    let t = l(e, 0x120);
    APIextra::TimerReset(e, t);
    lw(e, 0x17c, 1);
    APIextra::TimerSetFrequency(e, t, 10.8);
    APIextra::TimerRegisterNotify(e, SELF, t, 0xc);
    send(e, 0x2a10_0000, 0x32, 10, NULL);
    APIinput::InputConfigActivate(e, 0x12, 0);
    let v = (l(e, 0x15c) == 0) as u32;
    lw(e, 0x15c, v);
    lw(e, 0x170, 0);
    Ok(1)
}

/// 0x000116c8 callback 0xb
pub fn fcback_LF_ANIMATION(e: &mut Engine, _: u32) -> LResult<u64> {
    play(e, 0.2, 2, 1, 0);
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
        play(e, 0.1, 2, 2, 0);
        velma_baseFUNC_CutSceneManager(e, 0);
        velma_baseEnableDisableIdle(e, 1);
        let t = l(e, 0x120);
        APIextra::TimerUnregisterNotify(e, SELF, t);
        rand_idle_freq(e);
        Ok(1)
    } else if p == l(e, 0x120) {
        let t = l(e, 0x120);
        APIextra::TimerUnregisterNotify(e, SELF, t);
        let vo = l(e, 0x144);
        APInewSound::StopSound(e, SELF, vo);
        lw(e, 0x144, 0);
        let t = l(e, 0x124);
        APIextra::TimerReset(e, t);
        play(e, 0.1, 2, 2, 0);
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
        LIBApiText::KillText(e);
        velma_baseFadeMusVol(e, 1);
        let vo = l(e, 0x144);
        APInewSound::StopSound(e, SELF, vo);
        lw(e, 0x144, 0);
        let t = l(e, 0x124);
        APIextra::TimerReset(e, t);
        play(e, 0.1, 2, 2, 0);
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

/// 0x00012328 (rand%100: <31 -> 10, 31..50 -> 9, 51..70 -> 6, else 7)
pub fn velma_baseGetRandIdleAnim(e: &mut Engine) -> i32 {
    let r = (APIstdlib::MyRand(e) as i32) % 100;
    if r < 0x1f {
        10
    } else if r <= 0x32 {
        9
    } else if r <= 0x46 {
        6
    } else {
        7
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
        send(e, 0x2a00_004a, 0x54, 0, cp);
        send(e, 0x2a10_0000, 0x1e, 0, NULL);
    } else if which == 0x2a00_0047 {
        lw(e, 0x160, 1);
        lw(e, 0xb4, 4);
        lw(e, 0xd0, 0x2a00_0047);
        lw(e, 0xd4, 0x2d);
        lw(e, 0xd8, 0x2a00_0047);
        lw(e, 0xc4, 0);
        lw(e, 0xc8, 0x3fa0_0000);
        lw(e, 0xcc, 0);
        lw(e, 0xf0, 1);
        lw(e, 0xf4, 0);
        lw(e, 0xf8, 0x2000_211e);
        send(e, 0x2a00_004a, 0x54, 0, cp);
        send(e, 0x2a10_0000, 0x1e, 0, NULL);
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
        send(e, 0x2a00_004a, 0x55, 0, NULL);
        send(e, 0x2a10_0000, 0x1f, 0, NULL);
        lw(e, 0x178, 0);
        lw(e, 0x17c, 0);
    }
}

/// 0x00012900
pub fn velma_basePlayVO(e: &mut Engine) {
    let vo = l(e, 0x144);
    APIgameobj::RegisterNotify(e, SELF, vo, 0xf, 0xd);
    APInewSound::Play(e, SELF, vo, 0);
}

/// 0x00012980 (music fade via msg 0x5e to 0x2a0000ed; the return value is unused by callers)
pub fn velma_baseFadeMusVol(e: &mut Engine, dir: i32) {
    if dir == 0 && l(e, 0x164) == 1 {
        send(e, 0x2a00_00ed, 0x5e, 1, NULL);
        lw(e, 0x164, 0);
        let s = e.cstr("\t\t\t\t\tFade <<<<<<<<<\n");
        APIextra::null_printf(e, s, 0);
    } else if dir == 1 && l(e, 0x164) == 0 {
        send(e, 0x2a00_00ed, 0x5e, 0, NULL);
        lw(e, 0x164, 1);
        let s = e.cstr("\t\t\t\t\tFade >>>>>>>>>\n");
        APIextra::null_printf(e, s, 0);
    }
}

/// 0x00012aa8
pub fn velma_baseClues(e: &mut Engine) {
    let vo = if l(e, 0x140) == 1 { 0x8000_0014 } else { 0x8000_0013 };
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
    APIanim::Play(e, SELF, 2, 2, 0.0);
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

// ---------------------------------------------------------------- VelmaNew.c

fn set_start_pos(e: &mut Engine) {
    lw(e, 0x10c, 0x43db_c7ae); // 439.56
    lw(e, 0x110, 0);
    lw(e, 0x114, 0xc080_0000); // -4.0
    let p = L(e) + 0x10c;
    APIgameobj::SetTranslation(e, SELF, p);
}

/// 0x000131f8 state 1
pub fn fstate_FIRSTMOVE_LOCAL(e: &mut Engine, _: u32) -> LResult<u64> {
    let g8 = e.r32(GB(e, 2) + 8);
    if g8 & 0x80_0000 == 0 {
        if g8 & 0x40_0000 == 0 {
            let p = L(e) + 0x10c;
            APIgameobj::GetPosition(e, SELF, 0x7fff_ffff, p);
            play(e, 0.0, 3, 2, 0);
            APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 0x16, 8);
            APIgameobj::SetNextState(e, SELF, 5);
        } else {
            lw(e, 0x184, 1);
            set_start_pos(e);
            let t = l(e, 0x11c);
            APIextra::TimerUnregisterNotify(e, SELF, t);
            play(e, 0.0, 1, 2, 0);
            gb_or(e, 2, 8, 0, 0x80_0000);
            APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 0x16, 8);
            APIgameobj::SetNextState(e, SELF, 5);
        }
    } else {
        lw(e, 0x16c, 1);
        set_start_pos(e);
        play(e, 0.0, 2, 2, 0);
        velma_FUNC_SetClipping(e);
        fstate_FIRSTMOVE(e, 0)?;
    }
    Ok(0)
}

/// 0x00013528 callback 6: cut-scene 2 on the first touch, book handover (msg 0x70) on the second
pub fn fcback_NOTIFY_COLLIDE_OBJ_LOCAL(e: &mut Engine, p: u32) -> LResult<u64> {
    let d = e.alloca(8); // auStack_38
    e.w32(d, 0);
    let gb0 = GB(e, 0);
    if e.r32(gb0 + 8) != e.r32(p + 8) || e.r32(gb0 + 0x14) & 0x6e20094d != 0 {
        return Ok(0);
    }
    let (g8, gc) = (e.r32(GB(e, 2) + 8), e.r32(GB(e, 2) + 0xc));
    if g8 & 0x40_0000 == 0 && gc & 0x200_0000 == 0 {
        for (obj, snd) in [(0x2a00_00ef, 0xa000_0004), (0x2a00_00f0, 0xa000_0003), (0x2a00_00f0, 0x8000_004a), (0x2a00_00ef, 0x8000_004b)] {
            if APInewSound::IsPlaying(e, obj, snd) != 0 {
                APInewSound::StopSound(e, obj, snd);
            }
        }
        gb_or(e, 2, 8, 0x40_0000, 0);
        lw(e, 0x180, 1);
        e.w32(d, 0x8000_0002);
        send(e, 0x2a00_004a, 0x1de, 0, d);
    } else if g8 & 0x80_0000 == 0 && gc & 0x200_0000 != 0 {
        send(e, 0x2a10_0000, 0xb, 0x10, NULL);
        gb_or(e, 2, 8, 0x80_0000, 0);
        lw(e, 0x180, 1);
        send(e, 0x2a00_00ee, 0x70, 0, NULL);
        e.w32(d, 0x8000_0003);
        send(e, 0x2a00_004a, 0x1de, 1, d);
    } else if g8 & 0x40_0000 != 0 && g8 & 0x80_0000 != 0 && l(e, 0x180) == 0 {
        let t = l(e, 0x128);
        if 1.0 < APIextra::TimerGetTime(e, t) {
            fcback_NOTIFY_COLLIDE_OBJ(e, p)?;
        }
    }
    Ok(0)
}

fn init_for_cutscene(e: &mut Engine) {
    APIgameobj::SetGroupID(e, SELF, 6);
    let a = L(e);
    APIgameobj::GetClippingParams(e, SELF, a);
    let id = APIgameobj::GetID(e, SELF, NULL);
    LIBMiscFunc::MiscFunc_InitForCutscene(e, id);
    APIgameobj::SetNextState(e, SELF, 5);
}
fn end_cutscene(e: &mut Engine, anim: i32) {
    play(e, 0.0, anim, 2, 0);
    let id = APIgameobj::GetID(e, SELF, NULL);
    let a = L(e);
    LIBMiscFunc::MiscFunc_EndCutscene(e, id, a);
}

/// 0x00013988 callback 8: data {scene index, 1 = start / 0 = end}
pub fn fcback_CUTSCENE_LOCAL(e: &mut Engine, p: u32) -> LResult<u64> {
    let (scene, on) = (e.r32(p), e.r32(p + 4));
    if on == 1 && scene == 0 {
        init_for_cutscene(e);
    } else if on == 0 && scene == 0 {
        send(e, 0x2a10_0000, 0x2a, 6, NULL);
        end_cutscene(e, 1);
        lw(e, 0x180, 0);
        let t = l(e, 0x128);
        APIextra::TimerReset(e, t);
        APIgameobj::SetNextState(e, SELF, 5);
    }
    let (scene, on) = (e.r32(p), e.r32(p + 4));
    if on == 1 && scene == 1 {
        init_for_cutscene(e);
    } else if on == 0 && scene == 1 {
        send(e, 0x2a10_0000, 0x2a, 7, NULL);
        send(e, 0x2a10_0000, 0x2a, 6, NULL);
        send(e, 0x2a10_0000, 0x2b, 6, NULL);
        end_cutscene(e, 2);
        velma_FUNC_SetClipping(e);
        fstate_FIRSTMOVE(e, 0)?;
        lw(e, 0x180, 0);
        let t = l(e, 0x128);
        APIextra::TimerReset(e, t);
    }
    Ok(0)
}

/// 0x00013da0
pub fn velma_FUNC_SetClipping(e: &mut Engine) {
    let c = L(e) + 0x40;
    APIgameobj::GetClippingParams(e, SELF, c);
    for (o, v) in [(0x40, 0x3dcc_cccd), (0x44, 0x4160_0000), (0x50, 0x41c8_0000), (0x5c, 0xbf80_0000), (0x4c, 0), (0x48, 0x27), (0x58, 0x80), (0x54, 0x6a7), (0x64, 0x6a0), (0x60, 0x6a7)] {
        lw(e, o, v);
    }
    APIgameobj::SetClippingParams(e, SELF, c, 0);
}

/// 0x00013ed8 callback 0xc
pub fn fcback_TIMER_LOCAL(e: &mut Engine, p: u32) -> LResult<u64> {
    if p == l(e, 0x130) {
        let m = if e.r32(GB(e, 2) + 0xc) & 0x200_0000 == 0 { 0xb } else { 10 };
        send(e, 0x2a10_0000, m, 0x10, NULL);
        velma_FUNC_SetClipping(e);
        Ok(1)
    } else {
        fcback_TIMER(e, p)
    }
}

fn reset_timer_130(e: &mut Engine) {
    let t = l(e, 0x130);
    APIextra::TimerReset(e, t);
    APIextra::TimerSetFrequency(e, t, 0.75);
    APIextra::TimerRegisterNotify(e, SELF, t, 0xc);
    LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 2, NULL, 0);
}

/// 0x00013ff8 (msg 0xc1 = checkpoint reset, 0xd4)
pub fn velmaFUNC_MessageHandler(e: &mut Engine, m: u32, _s: u32, _d: Ptr) -> LResult<u64> {
    if m == 0xc1 {
        if l(e, 0x16c) == 0 {
            if e.r32(GB(e, 2) + 8) & 0x80_0000 != 0 {
                send(e, 0x2a10_0000, 0x2d, 6, NULL);
                send(e, 0x2a10_0000, 0x2d, 7, NULL);
                send(e, 0x2a10_0000, 0x2c, 7, NULL);
                let t = l(e, 0x11c);
                APIextra::TimerUnregisterNotify(e, SELF, t);
                send(e, 0x2a00_00ed, 0x200, 0, NULL);
                play(e, 0.0, 1, 2, 0);
                gb_or(e, 2, 8, 0, 0x80_0000);
                reset_timer_130(e);
                APIgameobj::SetNextState(e, SELF, 5);
            }
            if e.r32(GB(e, 2) + 8) & 0x40_0000 != 0 && l(e, 0x184) == 0 {
                send(e, 0x2a10_0000, 0x2d, 7, NULL);
                send(e, 0x2a10_0000, 0x2d, 6, NULL);
                send(e, 0x2a10_0000, 0x2c, 6, NULL);
                let t = l(e, 0x11c);
                APIextra::TimerUnregisterNotify(e, SELF, t);
                let p = L(e) + 0x10c;
                APIgameobj::SetTranslation(e, SELF, p);
                reset_timer_130(e);
                play(e, 0.0, 3, 2, 0);
                gb_or(e, 2, 8, 0, 0x40_0000);
                APIgameobj::SetNextState(e, SELF, 5);
            }
        }
        APIgameobj::SetGroupID(e, SELF, 0);
    } else if m == 0xd4 {
        if e.r32(GB(e, 2) + 8) & 0x40_0000 != 0 {
            lw(e, 0x184, 1);
            if e.r32(GB(e, 2) + 8) & 0x80_0000 != 0 {
                lw(e, 0x16c, 1);
            }
        }
        APIgameobj::SetGroupID(e, SELF, 0);
    }
    velma_base_MessageFunction(e, m)
}

/// 0x00014568
pub fn velmanewConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    velma_baseConstructor(e, api, instanceCount);
    if instanceCount == 1 {
        APIcons::SetExternalFunction(e, 1, fstate_FIRSTMOVE_LOCAL);
        APIcons::SetExternalFunction(e, 8, fcback_CUTSCENE_LOCAL);
        APIcons::SetExternalFunction(e, 6, fcback_NOTIFY_COLLIDE_OBJ_LOCAL);
        APIcons::SetExternalFunction(e, 0xc, fcback_TIMER_LOCAL);
    }
    LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 2, NULL, 0);
    APIcons::SetMessageFunction(e, velmaFUNC_MessageHandler);
    for o in [0x128, 0x130] {
        let t = APIextra::TimerCreate(e);
        lw(e, o, t);
    }
    APIgameobj::SetNextState(e, SELF, 1);
    Ok(())
}

/// 0x000146d0 (destroys timer +0x140 = clue flag, not +0x128: C quirk kept)
pub fn velmanewDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    for o in [0x140, 0x130] {
        let t = l(e, o);
        APIextra::TimerDestroy(e, t);
    }
    velma_baseDestructor(e);
    Ok(())
}

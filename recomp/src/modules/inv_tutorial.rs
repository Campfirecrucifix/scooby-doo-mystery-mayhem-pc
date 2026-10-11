//! `inv_tutorial`: EP1L01 tutorial director. Reacts to room changes (msg 0x52 with the room set by
//! 0x98) and gameplay events (0x103..0x10f, 0xa3, 0xb3, 0x108/0x10a) with one-shot tutorial voice
//! overs, texts and short camera cuts; the X button skips (input config 5). Flags live in GB0+0x40.
//! Port of `Game Logic Modules/Ep1_Library/Objects/Inv_Tutorial/inv_tutorial/Inv_Tutorial.c`.
#![allow(non_snake_case, non_upper_case_globals, unused_variables)]

use crate::engine::*;
use crate::logic::{LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "inv_tutorial",
    // globals 0x10000..0x10034 (+ gpoLanguagesNamesAndISOCodes)
    data_size: 0x50,
    ctor: inv_tutorialConstructor,
    dtor: inv_tutorialDestructor,
    register: None,
    init: &[],
    build_init: None,
};

pub fn build_index(key: &str) -> Option<usize> {
    (key == "0b9206d9").then_some(0)
}

pub const pExtra: u32 = 0x10000;
pub const pGameObj: u32 = 0x10004;
pub const pPhysic: u32 = 0x10008;
pub const pAnim: u32 = 0x1000c;
pub const pHud: u32 = 0x10010;
pub const pInput: u32 = 0x10014;
pub const pSound: u32 = 0x10018;
pub const pDebug: u32 = 0x1001c;
pub const pText: u32 = 0x10020;
pub const pMisc: u32 = 0x10024;
pub const bIsInterupByInput: u32 = 0x10028;
pub const pLocalDataInv_Tutorial: u32 = 0x1002c;
pub const pGlobalDataInv_Tutorial: u32 = 0x10030;

const RADAR: u32 = 0x2a10_0006;
const CAMERA: u32 = 0x2a00_008f;
const CONTROL: u32 = 0x2a00_0096;

#[inline]
fn G(e: &Engine, a: u32) -> Ptr {
    e.rt.cur_data() + (a - 0x10000)
}
/// local (0xbc): +8/+0x18/+0x20/+0x98/+0x9c/+0xa8/+0xb8 timers, +0xc, +0x14 current VO, +0x24
/// camera msg 0x54 block, +0x74, +0x78 room, +0x80 count, +0x90 in cut, +0x94 music up,
/// +0xa0/+0xa4 cut targets, +0xac text shown, +0xb0, +0xb4
#[inline]
fn L(e: &Engine) -> Ptr {
    e.r32(G(e, pLocalDataInv_Tutorial))
}
fn lr(e: &Engine, o: u32) -> u32 {
    e.r32(L(e) + o)
}
fn lw(e: &mut Engine, o: u32, v: u32) {
    let a = L(e) + o;
    e.w32(a, v);
}
/// pGlobalDataInv_Tutorial[i]: 0 GB0, 1 GB1, 2 GB2
fn GB(e: &Engine, i: u32) -> Ptr {
    let g = e.r32(G(e, pGlobalDataInv_Tutorial));
    e.r32(g + 4 * i)
}
fn g40(e: &Engine) -> u32 {
    e.r32(GB(e, 0) + 0x40)
}
fn set40(e: &mut Engine, v: u32) {
    let a = GB(e, 0) + 0x40;
    e.w32(a, v);
}
fn send(e: &mut Engine, to: u32, m: u32, p: u32) {
    APIgameobj::SendMessageTo(e, to, NULL, m, p, NULL);
}
/// SendMessageTo(0xff000008, {1, kind}, m, 0)
fn send_kind(e: &mut Engine, to: u32, kind: u32, m: u32) {
    let r = e.alloca(0x10);
    e.w32(r, 1);
    e.w32(r + 4, kind);
    APIgameobj::SendMessageTo(e, to, r, m, 0, NULL);
}
fn interrupted(e: &Engine) -> u32 {
    e.r32(G(e, bIsInterupByInput))
}
fn set_interrupted(e: &mut Engine, v: u32) {
    let a = G(e, bIsInterupByInput);
    e.w32(a, v);
}
fn dbg(e: &mut Engine, s: &str) {
    if e.r32(G(e, pDebug)) != 0 {
        let (a, b) = (e.cstr("TUTORIAL"), e.cstr(s));
        LIBdebug::PrintDebugInfo(e, a, b);
    }
}
fn timer_start(e: &mut Engine, o: u32, f: f32) {
    let t = lr(e, o);
    APIextra::TimerReset(e, t);
    APIextra::TimerSetFrequency(e, t, f);
    APIextra::TimerRegisterNotify(e, SELF, t, 3);
}
fn unreg(e: &mut Engine, o: u32) {
    let t = lr(e, o);
    APIextra::TimerUnregisterNotify(e, SELF, t);
}
fn show(e: &mut Engine, text: u32, buf: Ptr) {
    LIBApiText::ShowText(e, text, buf);
    lw(e, 0xac, 1);
}
/// CutSceneManager(1) + VO + music down
fn start_vo(e: &mut Engine, vo: u32) {
    lw(e, 0x14, vo);
    Inv_Tutorial_FUNC_CutSceneManager(e, 1);
    PlayVO(e);
    FadeMusVol(e, 0);
}

/// ext fn 0
pub fn fstate_FIRSTMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    set_interrupted(e, 0);
    APIgameobj::SetNextState(e, SELF, 1);
    Ok(0)
}

/// ext fn 1
pub fn fstate_BLACKHOLE_LOOP(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

/// ext fn 3
pub fn fcback_TIMER(e: &mut Engine, p: u32) -> LResult<u64> {
    if p == lr(e, 0xb8) {
        lw(e, 0x14, 0x8000_0014);
        send(e, 0x2a00_0013, 0xc1, 0);
        lw(e, 0xa0, 0x2a00_008b);
        lw(e, 0xa4, 0x2a00_008a);
        timer_start(e, 0x9c, 6.0);
        timer_start(e, 0xa8, 6.0);
        Inv_Tutorial_FUNC_CutSceneManager(e, 1);
        PlayVO(e);
        FadeMusVol(e, 0);
    } else if p == lr(e, 0xa8) {
        send(e, 0x2a00_0013, 0x58, 0);
    } else if p == lr(e, 0x9c) {
        let (a, b) = (lr(e, 0xa0), lr(e, 0xa4));
        SetCameraForCut(e, a, b);
    } else {
        if p == lr(e, 0x98) && interrupted(e) == 0 {
            let cam = e.r32(GB(e, 0) + 0x24);
            send(e, CAMERA, 0xda, cam);
            Inv_Tutorial_FUNC_CutSceneManager(e, 0);
            FadeMusVol(e, 1);
            let vo = lr(e, 0x14);
            APInewSound::StopSound(e, SELF, vo);
        }
        if p == lr(e, 0x18) {
            PlayVO(e);
        } else {
            if p == lr(e, 0x20) {
                if e.r32(GB(e, 0) + 0x14) & 0x6e20_090d == 0 && e.r32(GB(e, 2)) & 0x2_0000 == 0 {
                    return Ok(0);
                }
                unreg(e, 0x20);
                Inv_Tutorial_FUNC_CutSceneManager(e, 0);
                return Ok(1);
            }
            if lr(e, 0xac) != 0 {
                LIBApiText::KillText(e);
                lw(e, 0xac, 0);
                send(e, RADAR, 0x33, 0);
            }
        }
    }
    Ok(1)
}

/// ext fn 4
pub fn fcback_InputAction_X(e: &mut Engine, _: u32) -> LResult<u64> {
    let cam = e.r32(GB(e, 0) + 0x24);
    send(e, CAMERA, 0xda, cam);
    set_interrupted(e, 1);
    lw(e, 0xc, 0);
    if lr(e, 0xac) != 0 {
        LIBApiText::KillText(e);
        lw(e, 0xac, 1);
        send(e, RADAR, 0x33, 0);
        LIBApiText::KillText(e);
    }
    for o in [8, 0x9c, 0x20, 0xb8] {
        unreg(e, o);
    }
    FadeMusVol(e, 1);
    let vo = lr(e, 0x14);
    APInewSound::StopSound(e, SELF, vo);
    Inv_Tutorial_FUNC_CutSceneManager(e, 0);
    send(e, CAMERA, 0x55, 0);
    if lr(e, 0x14) == 0x8000_000e {
        let leader = e.r32(GB(e, 0) + 8);
        send(e, leader, 0xd2, 0x200);
    }
    Ok(1)
}

/// ext fn 5
pub fn fcback_END_OF_SOUND(e: &mut Engine, _: u32) -> LResult<u64> {
    set_interrupted(e, 0);
    unreg(e, 0x9c);
    timer_start(e, 0x98, 3.0);
    Ok(1)
}

pub fn MyMessageFunction(e: &mut Engine, m: u32, p: u32, _d: Ptr) -> LResult<u64> {
    let buf = e.alloca(0x40); // auStack_a0
    LIBApiText::InitStruct(e, buf);
    match m {
        0x103 => {
            if g40(e) & 0x80 == 0 {
                lw(e, 0xb0, 1);
                Inv_Tutorial_FUNC_CutSceneManager(e, 1);
                set40(e, g40(e) | 0x80);
                timer_start(e, 0x20, 12.0);
                lw(e, 0x14, 0x8000_0012);
                PlayVO(e);
                FadeMusVol(e, 0);
                LIBApiText::InitStruct(e, buf);
                e.w32(buf, 0x4040_0000);
                show(e, 0x156, buf);
            }
        }
        0x52 => {
            match lr(e, 0x78) {
                1 => {
                    if g40(e) & 1 == 0 && p == 0 {
                        let s = e.alloca(0x10); // auStack_60
                        e.w32(s, 0x8000_0009);
                        APIgameobj::SendMessageTo(e, CAMERA, NULL, 0x1de, 0, s);
                        set40(e, g40(e) | 1);
                        APInewSound::Play(e, SELF, 0x4000_0002, 0);
                    }
                    return Ok(0);
                }
                2 => {
                    if g40(e) & 2 == 0 {
                        dbg(e, "WALKTOCHECKPOINT");
                        set40(e, g40(e) | 2);
                        start_vo(e, 0x8000_000a);
                        show(e, 0x14f, buf);
                    }
                }
                3 => {
                    if g40(e) & 0x1000_0000 == 0 {
                        dbg(e, "WALKTOCHECKPOINT");
                        set40(e, g40(e) | 0x1000_0000);
                        lw(e, 0xa0, 0x2a00_0086);
                        lw(e, 0xa4, 0);
                        timer_start(e, 0x9c, 4.0);
                        start_vo(e, 0x8000_000e);
                    }
                }
                4 => {
                    if g40(e) & 4 == 0 {
                        dbg(e, "PUSHOBJECT");
                        set40(e, g40(e) | 4);
                        lw(e, 0xa0, 0x2a00_0087);
                        lw(e, 0xa4, 0x2a00_0088);
                        timer_start(e, 0x9c, 4.0);
                        start_vo(e, 0x8000_0011);
                        show(e, 0x155, buf);
                    }
                }
                5 => {
                    if g40(e) & 8 == 0 {
                        set40(e, g40(e) | 8);
                        dbg(e, "SNEAKVO");
                        send(e, CAMERA, 0xd3, 5);
                        start_vo(e, 0x8000_0013);
                        show(e, 0x158, buf);
                    }
                }
                6 => {
                    if g40(e) & 0x8000 == 0 {
                        set40(e, g40(e) | 0x8000);
                        dbg(e, "ROOM_6A");
                        if e.r32(GB(e, 0) + 0x14) & 0x40 != 0 {
                            send_kind(e, 0xff00_0008, 0x25, 0x1e8);
                        }
                        timer_start(e, 0xb8, 0.1);
                    }
                }
                _ => return Ok(0),
            }
            lw(e, 0x78, 0);
        }
        0x13 => {
            if lr(e, 0x90) != 0 {
                APIinput::InputConfigActivate(e, 5, 1);
            }
        }
        0x12 => {
            APIinput::InputConfigActivate(e, 5, 0);
        }
        0x46 => {
            lw(e, 0xc, 0);
            if lr(e, 0xac) != 0 {
                LIBApiText::KillText(e);
                lw(e, 0xac, 0);
            }
            unreg(e, 8);
            FadeMusVol(e, 1);
            let vo = lr(e, 0x14);
            APInewSound::StopSound(e, SELF, vo);
            Inv_Tutorial_FUNC_CutSceneManager(e, 0);
            send(e, CAMERA, 0x55, 0);
            if lr(e, 0x14) == 0x8000_000c {
                send(e, RADAR, 0x11b, 0);
            }
            if lr(e, 0x14) == 0x8000_000e {
                let leader = e.r32(GB(e, 0) + 8);
                send(e, leader, 0xd2, 0x200);
            }
        }
        0xa3 => {
            if g40(e) & 0x2_0000 == 0 {
                set40(e, g40(e) | 0x2_0000);
                lw(e, 0x14, 0x8000_000b);
                Inv_Tutorial_FUNC_CutSceneManager(e, 1);
                show(e, 0x150, buf);
                PlayVO(e);
                FadeMusVol(e, 0);
            }
        }
        0x98 => lw(e, 0x78, p),
        0xb3 => {
            if g40(e) & 0x200 == 0 {
                set40(e, g40(e) | 0x200);
                lw(e, 0x14, 0x8000_0015);
                lw(e, 0xa0, 0x2a00_013f);
                lw(e, 0xa4, 0x2a00_0089);
                timer_start(e, 0x9c, 2.0);
                Inv_Tutorial_FUNC_CutSceneManager(e, 1);
                PlayVO(e);
                FadeMusVol(e, 0);
            }
        }
        0x10a => {
            if p == 0 {
                if g40(e) & 0x400_0000 != 0 {
                    let n = lr(e, 0x80).wrapping_sub(1);
                    lw(e, 0x80, n);
                    if n == 1 {
                        send(e, CONTROL, 0xdf, 4);
                    } else if n == 0 {
                        set40(e, g40(e) & 0xfbff_ffff);
                    }
                }
            } else if g40(e) & 0x400_0000 == 0 {
                // the counter only moves while the bit is clear (short-circuit &&)
                let n = lr(e, 0x80).wrapping_add(1);
                lw(e, 0x80, n);
                if n == 2 {
                    set40(e, g40(e) | 0x400_0000);
                    send(e, CONTROL, 0xde, 4);
                }
            }
        }
        0x107 => {
            lw(e, 0x74, 1);
            if g40(e) & 0x4_0000 == 0 {
                lw(e, 0xb4, 1);
                set40(e, g40(e) | 0x4_0000);
                Inv_Tutorial_FUNC_CutSceneManager(e, 1);
                FadeMusVol(e, 0);
                show(e, 0x151, buf);
                timer_start(e, 0x98, 8.0);
                APIgameobj::SetNextState(e, SELF, 2);
            }
        }
        0x104 => {
            if g40(e) & 0x100 == 0 {
                Inv_Tutorial_FUNC_CutSceneManager(e, 1);
                set40(e, g40(e) | 0x100);
                lw(e, 0x14, 0x8000_0006);
                PlayVO(e);
                FadeMusVol(e, 0);
            }
        }
        0x108 => {
            if p == 0 {
                if g40(e) & 0x100_0000 != 0 {
                    set40(e, g40(e) & 0xfeff_ffff);
                    send(e, CONTROL, 0xdf, 3);
                }
            } else if g40(e) & 0x100_0000 == 0 {
                set40(e, g40(e) | 0x100_0000);
                lw(e, 0x14, 0x8000_0008);
                send(e, CONTROL, 0xde, 3);
                send(e, CAMERA, 0xd3, 4);
                Inv_Tutorial_FUNC_CutSceneManager(e, 1);
                PlayVO(e);
                FadeMusVol(e, 0);
            }
        }
        0x10d => {
            if g40(e) & 0x2000_0000 == 0 {
                set40(e, g40(e) | 0x2000_0000);
                start_vo(e, 0x8000_0004);
                show(e, 0x157, buf);
            }
        }
        0x10b => {
            if g40(e) & 0x800_0000 == 0 {
                set40(e, g40(e) | 0x800_0000);
                start_vo(e, 0x8000_0007);
                show(e, 0x159, buf);
            }
        }
        0x10f => {
            lw(e, 0x14, 0x8000_0010);
            PlayVO(e);
            FadeMusVol(e, 0);
        }
        _ => {}
    }
    Ok(0)
}

/// camera msg 0x54 type 4: {0x195, target obj, look-at obj, pos*, 1.0, 1.0, 15.0}
pub fn SetCameraForCut(e: &mut Engine, look: u32, target: u32) {
    let pos = e.alloca(0x10); // DStack_30
    APIgameobj::GetPosition(e, CAMERA, 0x7fff_ffff, pos);
    let s = e.alloca(0x20); // uStack_50
    e.w32(s, 0x195);
    e.w32(s + 4, target);
    e.w32(s + 8, look);
    e.w32(s + 0xc, if target != 0 { 0 } else { pos });
    e.w32(s + 0x10, 0x3f80_0000);
    e.w32(s + 0x14, 0x3f80_0000);
    e.w32(s + 0x18, 0x4170_0000);
    APIgameobj::SendMessageTo(e, CAMERA, NULL, 0x54, 4, s);
}

pub fn PlayVO(e: &mut Engine) {
    let vo = lr(e, 0x14);
    APIgameobj::RegisterNotify(e, SELF, vo, 0xf, 5);
    APInewSound::Play(e, SELF, vo, 0);
}

pub fn FadeMusVol(e: &mut Engine, down_up: u32) {
    if down_up == 0 && lr(e, 0x94) == 1 {
        send(e, CONTROL, 0x5e, 1);
        lw(e, 0x94, 0);
        let s = e.cstr("\t\t\t\t\tFade <<<<<<<<<\n");
        APIextra::null_printf(e, s, 0);
    } else if down_up == 1 && lr(e, 0x94) == 0 {
        send(e, CONTROL, 0x5e, 0);
        lw(e, 0x94, 1);
        let s = e.cstr("\t\t\t\t\tFade >>>>>>>>>\n");
        APIextra::null_printf(e, s, 0);
    }
}

fn cam_block(e: &mut Engine, kind: u32, w68: u32) {
    let l = L(e);
    e.w32(l + 0x24, kind);
    e.w32(l + 0x60, 1);
    e.w32(l + 0x64, 0);
    e.w32(l + 0x68, w68);
    APIgameobj::SendMessageTo(e, CAMERA, NULL, 0x54, 0, l + 0x24);
}

pub fn Inv_Tutorial_FUNC_CutSceneManager(e: &mut Engine, mode: u32) {
    let gb0 = GB(e, 0);
    if mode == 1 {
        lw(e, 0x90, 1);
        set_interrupted(e, 0);
        for o in [8, 0xc] {
            let h = e.r32(gb0 + o);
            send(e, h, 0xd2, 0x104);
        }
        send(e, RADAR, 0x183, 1);
        for o in [0x2a00_013f, 0x2a00_0140, 0x2a00_0141] {
            send(e, o, 0x1e7, 1);
        }
        cam_block(e, 99, 0);
        APIinput::InputConfigActivate(e, 5, 1);
        let g2 = GB(e, 2);
        let v = e.r32(g2) | 0x2_0000;
        e.w32(g2, v);
        send(e, RADAR, 0x25, 0);
        if lr(e, 0xb4) == 0 {
            send(e, RADAR, 0x1e, 0);
        }
        if lr(e, 0xb4) != 0 {
            lw(e, 0xb4, 0);
        }
        send_kind(e, 0xff00_0008, 1, 0x57);
        send_kind(e, 0xff00_0008, 7, 0x57);
    } else if mode == 0x2a00_0097 {
        lw(e, 0x90, 1);
        send(e, RADAR, 0x183, 1);
        send(e, 0x2a00_013f, 0x1e7, 1);
        let l = L(e);
        e.w32(l + 0x40, 0x2a00_007c);
        e.w32(l + 0x34, 0);
        e.w32(l + 0x38, 0x3f80_0000);
        e.w32(l + 0x3c, 0);
        e.w32(l + 0x48, 0x2a00_0097);
        cam_block(e, 4, 0x421a);
        send(e, RADAR, 0x1e, 0);
    } else {
        if e.r32(gb0 + 0x14) & 0x904 == 0 {
            for o in [8, 0xc] {
                let h = e.r32(gb0 + o);
                send(e, h, 0xd2, 0x208);
            }
        }
        unreg(e, 0x98);
        send(e, RADAR, 0x183, 0);
        for o in [0x2a00_013f, 0x2a00_0140, 0x2a00_0141] {
            send(e, o, 0x1e7, 0);
        }
        send(e, RADAR, 0x25, 1);
        lw(e, 0x90, 0);
        let g2 = GB(e, 2);
        let v = e.r32(g2) & 0xfffd_ffff;
        e.w32(g2, v);
        APIinput::InputConfigActivate(e, 5, 0);
        send(e, CAMERA, 0x55, 0);
        send(e, RADAR, 0x1f, 0);
        send_kind(e, 0xff00_0008, 1, 0x58);
        send_kind(e, 0xff00_0008, 7, 0x58);
    }
}

pub fn Inv_Tutorial_FUNC_Create_Input(e: &mut Engine) {
    dbg(e, "Create Input");
    APIinput::InputConfigAdd(e, 5);
    APIinput::InputActionAdd(e, 5, 0x39, 0);
    let port = e.ri32(GB(e, 0) + 0x4c);
    APIinput::InputDeviceAdd(e, 5, 0x39, 0x3d, port, 0, 2, 0, 0);
    APIinput::InputDeviceAdd(e, 5, 0x39, 0x3d, 1, 1, 2, 0, 0x39);
    APIinput::InputContextAdd(e, SELF, 4, 5, 0x39, 0x1e, 8);
    APIinput::InputContextSetState(e, 5, 0x39, 0x1e, 2);
    APIinput::InputConfigSetState(e, 5, 2);
    APIinput::InputConfigActivate(e, 5, 0);
}

pub fn inv_tutorialConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    if instanceCount == 1 {
        let fns: [(i32, crate::logic::ExtFn); 5] = [
            (0, fstate_FIRSTMOVE),
            (1, fstate_BLACKHOLE_LOOP),
            (3, fcback_TIMER),
            (4, fcback_InputAction_X),
            (5, fcback_END_OF_SOUND),
        ];
        for (i, f) in fns {
            APIcons::SetExternalFunction(e, i, f);
        }
        let tables: [(u32, fn(&mut Engine) -> Ptr); 7] = [
            (pAnim, |e| APIcons::GetApiAnim(e)),
            (pGameObj, |e| APIcons::GetApiGameObj(e)),
            (pExtra, |e| APIcons::GetApiExtra(e)),
            (pPhysic, |e| APIcons::GetApiPhysic(e)),
            (pInput, |e| APIcons::GetApiInput(e)),
            (pSound, |e| APIcons::GetApiNewSound(e)),
            (pHud, |e| APIcons::GetApiHUD(e)),
        ];
        for (slot, get) in tables {
            let v = get(e);
            let a = G(e, slot);
            e.w32(a, v);
        }
        for (slot, name) in [(pDebug, "debug"), (pText, "ApiText"), (pMisc, "MiscFunc")] {
            let n = e.cstr(name);
            let v = APIcons::GetApiCustom(e, n);
            let a = G(e, slot);
            e.w32(a, v);
        }
        let slot = G(e, pGlobalDataInv_Tutorial);
        APIcons::GetGlobalData(e, slot, 0xc);
        let gd = e.r32(slot);
        for (w, size) in [(0u32, 0x8f8u32), (1, 0x68), (2, 0x158)] {
            let b = APIgameobj::GetGameBuffer(e, w, size, 1);
            e.w32(gd + 4 * w, b);
        }
    }
    APIcons::SetMessageFunction(e, MyMessageFunction);
    let slot = G(e, pLocalDataInv_Tutorial);
    APIcons::GetLocalData(e, slot, 0xbc);
    for o in [8, 0x18, 0x20, 0x98, 0x9c, 0xa8, 0xb8] {
        let t = APIextra::TimerCreate(e);
        lw(e, o, t);
    }
    Inv_Tutorial_FUNC_Create_Input(e);
    lw(e, 0x94, 1);
    lw(e, 0xac, 0);
    lw(e, 0xb0, 0);
    LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 2, NULL, 0);
    APIcons::SetInitialState(e, 0);
    Ok(())
}

pub fn inv_tutorialDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    for o in [8, 0x18, 0x20, 0x98, 0x9c, 0xa8, 0xb8] {
        let t = lr(e, o);
        APIextra::TimerDestroy(e, t);
    }
    Ok(())
}

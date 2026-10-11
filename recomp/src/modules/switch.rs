//! `switch`: wall/floor switches toggled with X by the leader (GB2+8 bit per switch); sends its on/off
//! message to a target object and animates via AnimMngr. Port of the 4 disc builds of
//! `.../SWITCH/switch__<sha8>` (switch_base.c + switch.c): one shared base, per-build table + the local
//! (switch.c) variants selected by build. pDebug prints are skipped (no "debug" lib -> pDebug == NULL).
#![allow(non_snake_case, non_upper_case_globals, unused_variables)]

use crate::engine::*;
use crate::logic::{LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "switch",
    // port layout: API pointers 0x10000.. (goCustomDataSwitch is read from BUILDS, it is never written)
    data_size: 0x40,
    ctor: switchConstructor,
    dtor: switchDestructor,
    register: None,
    init: &[],
    build_init: None,
};

pub const pText: u32 = 0x10000;
pub const pMiscFunc: u32 = 0x10004;
pub const pLocalDataSwitch: u32 = 0x10008;
pub const pGlobalDataSwitch: u32 = 0x1000c;
pub const pAnim: u32 = 0x10010;
pub const pExtra: u32 = 0x10014;
pub const pGame: u32 = 0x10018;
pub const pGameObj: u32 = 0x1001c;
pub const pHud: u32 = 0x10020;
pub const pInput: u32 = 0x10024;
pub const pLight: u32 = 0x10028;
pub const pPhysic: u32 = 0x1002c;
pub const pSound: u32 = 0x10030;
pub const pAnimMngr: u32 = 0x10034;
pub const pDebug: u32 = 0x10038;

/// switch.c variant
#[derive(PartialEq, Clone, Copy)]
enum Local {
    /// 05b58ac2: msg 0x13e reset, FIRSTMOVE_LOCAL, initial state 1
    Reset,
    /// bd24116c: msg 0x1ed sets on/off, initial state 0
    Set,
    /// d5430e80: paired switches 0x2a00010b/c (msg 0x16 / 0xc1), clipping, help text
    Pair,
    /// f112abee: msg 0xe2/0x166, cut-scene callback 10
    Cut,
    /// module `cutswitch` (704f1031): same base/data as f112abee but id +0x1f9, own X action,
    /// cut-scene and message function (msg 0xc9 restore)
    CutSw,
}

struct Build {
    sha8: &'static str,
    local: Local,
    /// goCustomDataSwitch: 11 words per switch {id, target, ?, ?, anim target, on msg, off msg, GB2+8 bit, ?, ?, start on}
    data: &'static [[u32; 11]],
    radar: u32,
    /// MyMessageFunction 0xde/0xdf hero ids and anims: (id a, id b, de a, de b, df a, df b)
    beam: (u32, u32, i32, i32, i32, i32),
    /// "off" animation (TURN_OFF_LF, 0xc1, init)
    off_anim: i32,
    /// fcback_ANIM second "on" anim key (6 or 5)
    on_key: u32,
}

static BUILDS: [Build; 5] = [
    Build {
        sha8: "05b58ac2",
        local: Local::Reset,
        data: &[[0x2a00013f, 0, 0, 0, 0x2a00012c, 0, 0x1a6, 0x200000, 0, 0, 1]],
        radar: 0x2a100006,
        beam: (0x2a00007c, 0x2a00007d, 1, 6, 4, 3),
        off_anim: 7,
        on_key: 6,
    },
    Build {
        sha8: "bd24116c",
        local: Local::Set,
        data: &[
            [0x2a000160, 0x2a000164, 0, 0, 0x2a000164, 0xe1, 0xe2, 0x100000, 0, 0, 1],
            [0x2a000161, 0x2a000164, 0, 0, 0x2a000164, 0xe1, 0xe2, 0x200000, 0, 0, 1],
            [0x2a000162, 0x2a000164, 0, 0, 0x2a000164, 0xe1, 0xe2, 0x400000, 0, 0, 0],
            [0x2a000163, 0x2a000164, 0, 0, 0x2a000164, 0xe1, 0xe2, 0x800000, 0, 0, 0],
        ],
        radar: 0x2a100013,
        beam: (0x2a00004c, 0x2a00004d, 1, 6, 4, 3),
        off_anim: 7,
        on_key: 6,
    },
    Build {
        sha8: "d5430e80",
        local: Local::Pair,
        data: &[
            [0x2a00010b, 0, 0, 0, 0x2a00010e, 0xde, 0, 0x10000000, 0, 0, 0],
            [0x2a00010c, 0, 0, 0, 0x2a00017b, 0xe2, 0, 0x400, 0, 0, 0],
        ],
        radar: 0x2a100006,
        beam: (0x2a000023, 0x2a000024, 6, 5, 4, 3),
        off_anim: 1,
        on_key: 6,
    },
    Build {
        sha8: "f112abee",
        local: Local::Cut,
        data: &[
            [0x2a0000e3, 0, 0, 0, 0x2a0000df, 0xe1, 0xe2, 0x200, 0, 0, 0],
            [0x2a0000e4, 0, 0, 0, 0x2a0000e0, 0xe1, 0xe2, 0x400, 0, 0, 0],
            [0x2a0000e5, 0x2a0000b3, 0x57, 0, 0, 0, 0, 0x10000000, 0, 0, 0],
        ],
        radar: 0x2a100006,
        beam: (0x2a0000aa, 0x2a0000ab, 1, 5, 4, 3),
        off_anim: 6,
        on_key: 5,
    },
    Build {
        sha8: "704f1031",
        local: Local::CutSw,
        data: &[
            [0x2a0000e3, 0, 0, 0, 0x2a0000df, 0xe1, 0xe2, 0x200, 0, 0, 0],
            [0x2a0000e4, 0, 0, 0, 0x2a0000e0, 0xe1, 0xe2, 0x400, 0, 0, 0],
            [0x2a0000e5, 0x2a0000b3, 0x57, 0, 0, 0, 0, 0x10000000, 0, 0, 0],
        ],
        radar: 0x2a100006,
        beam: (0x2a0000aa, 0x2a0000ab, 1, 5, 4, 3),
        off_anim: 6,
        on_key: 5,
    },
];

pub static MODULE_CUTSWITCH: ModuleDef = ModuleDef {
    name: "cutswitch",
    data_size: 0x40,
    ctor: switchConstructor,
    dtor: switchDestructor,
    register: None,
    init: &[],
    build_init: None,
};

pub fn build_index(key: &str) -> Option<usize> {
    BUILDS.iter().position(|b| b.sha8 == key)
}
fn B(e: &Engine) -> &'static Build {
    &BUILDS[e.rt.cur_build()]
}

#[inline]
fn G(e: &Engine, a: u32) -> Ptr {
    e.rt.cur_data() + (a - 0x10000)
}
/// local data (0x90) byte offset
fn L(e: &Engine, o: u32) -> Ptr {
    e.r32(G(e, pLocalDataSwitch)) + o
}
fn lr(e: &Engine, o: u32) -> u32 {
    e.r32(L(e, o))
}
fn lw(e: &mut Engine, o: u32, v: u32) {
    let a = L(e, o);
    e.w32(a, v);
}
/// game buffer 0 (+0) / 2 (+4) pointers
fn gb(e: &Engine, i: u32) -> Ptr {
    e.r32(e.r32(G(e, pGlobalDataSwitch)) + i * 4)
}
fn gb2_8(e: &Engine) -> u32 {
    e.r32(gb(e, 1) + 8)
}
fn set_gb2_8(e: &mut Engine, v: u32) {
    let a = gb(e, 1) + 8;
    e.w32(a, v);
}
fn play(e: &mut Engine, anim: i32, mode: i32) {
    LIBAnimMngr::PlayAnim(e, 0.0, anim, 1, mode);
}
fn send(e: &mut Engine, to: u32, msg: u32, p: u32, d: Ptr) {
    APIgameobj::SendMessageTo(e, to, NULL, msg, p, d);
}

/// 0x00010d70 state 0
pub fn fstate_FIRSTMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    APIphysic::SetPriority(e, SELF, 100);
    APIphysic::SetVelocity(e, SELF, 0.0, 0.0, 0.0);
    APIgameobj::SetNextState(e, SELF, 2);
    Ok(0)
}

/// 0x00010e08 state 2
pub fn fstate_BASICMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

/// callback 3
pub fn fcback_TURN_ON_LF(e: &mut Engine, _: u32) -> LResult<u64> {
    play(e, 2, 0);
    APIgameobj::SetNextState(e, SELF, 2);
    Ok(0)
}

/// callback 4
pub fn fcback_TURN_OFF_LF(e: &mut Engine, _: u32) -> LResult<u64> {
    let a = B(e).off_anim;
    play(e, a, 0);
    APIgameobj::SetNextState(e, SELF, 2);
    Ok(0)
}

/// callback 5 (anim key event {key, kind}). Keys 3/4 = turned off, 1 and 6 (5 in f112abee) = turned on;
/// d5430e80 has a different shape: 3/4 off, 5/6 on.
pub fn fcback_ANIM(e: &mut Engine, param_1: u32) -> LResult<u64> {
    let s = e.alloca(8);
    let id = APIgameobj::GetID(e, SELF, NULL);
    e.w32(s, id);
    if e.r32(param_1 + 4) != 1 {
        return Ok(0);
    }
    let k = e.r32(param_1);
    let on = if B(e).local == Local::Pair {
        if k <= 2 {
            return Ok(0);
        }
        if k >= 7 {
            return Ok(0);
        }
        k >= 5
    } else {
        if k == 3 || k == 4 {
            false
        } else if k == 1 || k == B(e).on_key {
            true
        } else {
            return Ok(0);
        }
    };
    if on {
        send(e, SELF, 0x13d, 0, NULL);
        if lr(e, 0x24) != 0 {
            let (t, m) = (lr(e, 0x24), lr(e, 0x28));
            send(e, t, m, 1, s);
        }
    } else {
        send(e, SELF, 0x13e, 0, NULL);
        if lr(e, 0x24) != 0 {
            let (t, m) = (lr(e, 0x24), lr(e, 0x2c));
            send(e, t, m, 0, s);
        }
    }
    Ok(0)
}

/// 0x00011130
pub fn CreatePhysic(e: &mut Engine) {
    APIphysic::CreatePhysic(e, SELF, 0);
    APIphysic::SetFlags(e, SELF, 2);
    APIphysic::SetWeight(e, SELF, 1.0);
    APIphysic::SetPriority(e, SELF, 100);
    APIphysic::PrimSetIntAttribBits(e, SELF, 0, 0, 4);
    let dm = APIphysic::DMAdd(e, SELF, 0x25);
    lw(e, 0x84, dm);
    let p = e.alloca(0x10);
    APIphysic::DMGetParams(e, dm, p);
    e.w32(p + 4, 7);
    APIphysic::DMSetParams(e, dm, p);
    APIphysic::DMSetContextMask(e, dm, 4);
}

/// 0x00011290
pub fn CreateInput(e: &mut Engine) {
    let c = lr(e, 0) as i32;
    APIinput::InputConfigAdd(e, c);
    APIinput::InputActionAdd(e, c, 0x39, 0);
    let pad = e.r32(gb(e, 0) + 0x4c) as i32;
    APIinput::InputDeviceAdd(e, c, 0x39, 0x3d, pad, 0, 2, 0, 0);
    APIinput::InputDeviceAdd(e, c, 0x39, 0x3d, 1, 1, 2, 0, 0x39);
    APIinput::InputContextAdd(e, SELF, 8, c, 0x39, 0x1e, 8);
    APIinput::InputContextSetState(e, c, 0x39, 0x1e, 1);
    APIinput::InputConfigSetState(e, c, 1);
    APIinput::InputConfigActivate(e, c, 0);
}

/// callback 6 (collision notify 3)
pub fn fcback_CollisionDetection(e: &mut Engine, param_1: u32) -> LResult<u64> {
    if B(e).local == Local::Pair && lr(e, 0xc) == 0 && gb2_8(e) & lr(e, 0x30) == 0 && lr(e, 0x88) == 0 {
        lw(e, 0x88, 1);
        let t = e.alloca(0x34);
        LIBApiText::InitStruct(e, t);
        e.w32(t, 0x4060_0000);
        let id = APIgameobj::GetID(e, SELF, NULL);
        e.w32(t + 0x30, id);
        LIBApiText::ShowText(e, 0x15c, t);
    }
    if lr(e, 0xc) == 1
        && e.r32(gb(e, 0) + 8) == e.r32(param_1 + 8)
        && e.r32(gb(e, 0) + 0x14) & 0x6e60094d == 0
    {
        lw(e, 0x14, 1);
        if lr(e, 0x10) == 0 && e.r32(gb(e, 1)) & 0x20000 == 0 {
            let c = L(e, 0x40);
            LIBMiscFunc::MiscFunc_RemoveBitClipping(e, c, 0x80);
            lw(e, 0x10, 1);
            let r = B(e).radar;
            send(e, r, 0x31, 5, NULL);
            let c = lr(e, 0) as i32;
            APIinput::InputConfigActivate(e, c, 1);
            APIgameobj::SetContextFlag(e, SELF, 4);
        }
    }
    Ok(0)
}

/// callback 8
pub fn fcback_InputAction_X(e: &mut Engine, _: u32) -> LResult<u64> {
    if e.r32(gb(e, 0) + 0x14) & 0x6e60094d == 0 {
        fcback_CollisionDetectionOut(e, 0)?;
        let bit = lr(e, 0x30);
        let on = gb2_8(e) & bit == 0;
        let v = if on { gb2_8(e) | bit } else { gb2_8(e) & !bit };
        set_gb2_8(e, v);
        let leader = e.r32(gb(e, 0) + 8);
        let id = APIgameobj::GetID(e, SELF, NULL);
        send(e, leader, 0x9a, on as u32, id);
        if lr(e, 0x18) != 0 {
            let (t, m) = (lr(e, 0x18), lr(e, if on { 0x1c } else { 0x20 }));
            send(e, t, m, on as u32, NULL);
        }
    }
    Ok(0)
}

/// cutswitch callback 8
pub fn fcback_InputAction_X_Local(e: &mut Engine, _: u32) -> LResult<u64> {
    let ic = lr(e, 0) as i32;
    APIinput::InputConfigActivate(e, ic, 0);
    APIgameobj::UnregisterNotify(e, SELF, 0xffff_ffff, 3);
    if lr(e, 0x10) != 0 && e.r32(gb(e, 1)) & 0x20000 != 0 {
        lw(e, 0x10, 0);
        send(e, 0x2a100006, 0x32, 0xd, NULL);
    }
    let bit = lr(e, 0x30);
    let on = gb2_8(e) & bit == 0;
    let v = if on { gb2_8(e) | bit } else { gb2_8(e) & !bit };
    set_gb2_8(e, v);
    if lr(e, 0x18) != 0 {
        let (t, m) = (lr(e, 0x18), lr(e, if on { 0x1c } else { 0x20 }));
        send(e, t, m, on as u32, NULL);
    }
    Ok(1)
}

/// callback 9
pub fn fcback_CollisionDetectionOut(e: &mut Engine, _: u32) -> LResult<u64> {
    if e.r32(gb(e, 1)) & 0x20000 != 0 {
        let r = B(e).radar;
        send(e, r, 0x32, 5, NULL);
    }
    lw(e, 0x10, 0);
    let c = L(e, 0x40);
    LIBMiscFunc::MiscFunc_ReplaceMyClipping(e, c);
    let c = lr(e, 0) as i32;
    APIinput::InputConfigActivate(e, c, 0);
    APIgameobj::SetContextFlag(e, SELF, 0);
    Ok(0)
}

/// callback 7 (notify 0xb)
pub fn fcback_POST_COLLISION(e: &mut Engine, _: u32) -> LResult<u64> {
    if lr(e, 0x14) == 0 {
        fcback_CollisionDetectionOut(e, 0)?;
    }
    lw(e, 0x14, 0);
    Ok(0)
}

/// 0x00011a18
pub fn switch_baseInitialisation(e: &mut Engine) {
    let id = APIgameobj::GetID(e, SELF, NULL);
    for (i, d) in B(e).data.iter().enumerate() {
        if d[0] == id {
            lw(e, 0, i as u32 + 500);
            for (o, w) in [(0x18, 1), (0x1c, 2), (0x20, 3), (0x24, 4), (0x28, 5), (0x2c, 6), (0x30, 7), (0x3c, 10)] {
                lw(e, o, d[w]);
            }
        }
    }
    CreatePhysic(e);
    CreateInput(e);
    lw(e, 0xc, 1);
    lw(e, 0x80, 0);
    APIgameobj::SetGroupID(e, SELF, 6);
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 3, 6);
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 0xb, 5);
    let off = B(e).off_anim;
    let bit = lr(e, 0x30);
    if B(e).local == Local::Set {
        if lr(e, 0x3c) == 0 {
            play(e, 7, 0);
        } else {
            let id = APIgameobj::GetID(e, SELF, NULL);
            let a = id == 0x2a000160 && e.r32(gb(e, 0) + 0x80) == 0;
            let b = !a && {
                let id = APIgameobj::GetID(e, SELF, NULL);
                id == 0x2a000161 && e.r32(gb(e, 0) + 0x84) == 0
            };
            if a || b {
                let v = gb2_8(e) | bit;
                set_gb2_8(e, v);
                play(e, 2, 0);
            } else {
                play(e, 7, 0);
            }
        }
    } else if lr(e, 0x3c) == 0 {
        play(e, if gb2_8(e) & bit == 0 { off } else { 2 }, 0);
    } else {
        if gb2_8(e) & bit == 0 {
            let v = gb2_8(e) | bit;
            set_gb2_8(e, v);
        }
        play(e, 2, 0);
    }
}

/// 0x00011e00
pub fn MyMessageFunction(e: &mut Engine, param_1: u32, param_2: u32, param_3: u32) -> LResult<u64> {
    let (ia, ib, da, db, fa, fb) = B(e).beam;
    match param_1 {
        0xde => {
            if param_3 == ia {
                play(e, da, 3);
            } else if param_3 == ib {
                play(e, db, 3);
            }
        }
        0x13 => {
            APIphysic::ResetFlags(e, SELF, 1);
            if lr(e, 0x10) != 0 {
                lw(e, 0x10, 0);
            }
        }
        0x12 => {
            fcback_CollisionDetectionOut(e, 0)?;
            APIphysic::SetFlags(e, SELF, 1);
        }
        0xc1 => {
            lw(e, 0xc, 1);
            let bit = lr(e, 0x30);
            if lr(e, 0x3c) == 0 {
                let v = gb2_8(e) & !bit;
                set_gb2_8(e, v);
                let a = B(e).off_anim;
                play(e, a, 0);
            } else {
                let v = gb2_8(e) | bit;
                set_gb2_8(e, v);
                play(e, 2, 0);
            }
        }
        0xd4 => lw(e, 0x80, 1),
        0x13b => lw(e, 0xc, 1),
        0xdf => {
            if param_3 == ia {
                play(e, fa, 4);
            } else if param_3 == ib {
                play(e, fb, 4);
            }
        }
        0x13c => lw(e, 0xc, 0),
        99999 => {
            lw(e, 0x88, 0);
            let c = lr(e, 0) as i32;
            APIinput::InputConfigActivate(e, c, 0);
        }
        _ => {}
    }
    Ok(0)
}

/// switch.c MyLocalMessageFunction (per build)
pub fn MyLocalMessageFunction(e: &mut Engine, param_1: u32, param_2: u32, d: Ptr) -> LResult<u64> {
    match B(e).local {
        Local::Reset => {
            if param_1 == 0x13e {
                APIanim::StopAllAnim(e, SELF);
                play(e, 7, 0);
                APIgameobj::UnregisterNotify(e, SELF, 0xffff_ffff, 3);
                APIgameobj::UnregisterNotify(e, SELF, 0xffff_ffff, 6);
                lw(e, 0x10, 0);
                let c = lr(e, 0) as i32;
                APIinput::InputConfigActivate(e, c, 0);
                send(e, 0x2a100006, 0x32, 5, NULL);
            }
        }
        Local::Set => {
            if param_1 == 0x1ed {
                play(e, if param_2 == 1 { 2 } else { 7 }, 0);
                lw(e, 0x3c, param_2);
            }
        }
        Local::Pair => {
            if param_1 == 0x16 {
                let v = (gb2_8(e) & lr(e, 0x30) == 0) as u32;
                lw(e, 0x8c, v);
            } else if param_1 == 0xc1 {
                let bit = lr(e, 0x30);
                if lr(e, 0x8c) == 0 {
                    let v = gb2_8(e) | bit;
                    set_gb2_8(e, v);
                    play(e, 2, 0);
                } else if lr(e, 0x8c) == 1 {
                    let v = gb2_8(e) & !bit;
                    set_gb2_8(e, v);
                    play(e, 1, 0);
                }
                let id = APIgameobj::GetID(e, SELF, NULL);
                if gb2_8(e) & 0x18 == 0x18 {
                    if id == 0x2a00010b {
                        if gb2_8(e) & 0x1000_0000 == 0 {
                            send(e, 0x2a00010b, 0x13b, 0, NULL);
                            send(e, 0x2a00019d, 0xde, 0, NULL);
                        }
                    } else if id == 0x2a00010c && gb2_8(e) & 2 != 0 && gb2_8(e) & 0x100 == 0 {
                        send(e, 0x2a00010c, 0x13b, 0, NULL);
                        send(e, 0x2a00019e, 0xde, 0, NULL);
                    }
                } else if id == 0x2a00010b {
                    if gb2_8(e) & 0x1000_0000 != 0 {
                        send(e, 0x2a00010b, 0x13c, 0, NULL);
                        send(e, 0x2a00019d, 0xdf, 0, NULL);
                        let v = gb2_8(e) & !lr(e, 0x30);
                        set_gb2_8(e, v);
                    }
                } else if id == 0x2a00010c && gb2_8(e) & 2 == 0 {
                    send(e, 0x2a00010c, 0x13c, 0, NULL);
                    send(e, 0x2a00019e, 0xdf, 0, NULL);
                    let v = gb2_8(e) & !lr(e, 0x30);
                    set_gb2_8(e, v);
                }
                return Ok(0);
            }
        }
        Local::CutSw => {
            if param_1 == 0xc1 {
                return Ok(0);
            }
            if param_1 == 0xc9 && lr(e, 0x80) == 0 {
                let bit = lr(e, 0x30);
                if lr(e, 0x3c) == 0 {
                    let v = gb2_8(e) & !bit;
                    set_gb2_8(e, v);
                    play(e, 6, 0);
                } else {
                    let v = gb2_8(e) | bit;
                    set_gb2_8(e, v);
                    play(e, 2, 0);
                }
                lw(e, 0xc, 1);
                APIgameobj::RegisterNotify(e, SELF, 0, 0x16, 10);
                APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 3, 6);
            }
        }
        Local::Cut => {
            if param_1 == 0xe2 {
                if gb2_8(e) == 0 && lr(e, 0x30) & 1 != 0 {
                    let v = gb2_8(e) & !lr(e, 0x30);
                    set_gb2_8(e, v);
                    play(e, 4, 4);
                }
            } else if param_1 == 0x166 {
                send(e, 0x2a0000e0, 0xda, 0, NULL);
            }
        }
    }
    MyMessageFunction(e, param_1, param_2, d)
}

/// state 1 (05b58ac2, d5430e80)
pub fn fstate_FIRSTMOVE_LOCAL(e: &mut Engine, s: u32) -> LResult<u64> {
    fstate_FIRSTMOVE(e, s)?;
    if B(e).local == Local::Pair {
        let id = APIgameobj::GetID(e, SELF, NULL);
        if id > 0x2a00010a && id < 0x2a00010d {
            lw(e, 0xc, 0);
            if gb2_8(e) & 0x1000_0000 != 0 {
                send(e, 0x2a00010b, 0x13c, 0, NULL);
                send(e, 0x2a00019d, 0xdf, 0, NULL);
            }
            if gb2_8(e) & 2 == 0 {
                send(e, 0x2a00010c, 0x13c, 0, NULL);
                send(e, 0x2a00019e, 0xdf, 0, NULL);
            }
        }
        let v = (gb2_8(e) & lr(e, 0x30) == 0) as u32;
        lw(e, 0x8c, v);
        let leader = e.r32(gb(e, 0) + 8);
        LIBMiscFunc::MiscFunc_SetClipping(e, SELF, leader, 0);
    }
    Ok(0)
}

/// callback 10 (f112abee, notify 0x16)
pub fn fcback_CUTSCENE(e: &mut Engine, param_1: u32) -> LResult<u64> {
    let (k, a) = (e.r32(param_1), e.r32(param_1 + 4));
    if a == 1 && APIgameobj::IsInCutScene(e, SELF, 0) == 1 && k == 0 {
        if B(e).local == Local::CutSw {
            lw(e, 0x80, 0);
            APIgameobj::SetGroupID(e, SELF, 6);
        }
        APIphysic::SetFlags(e, SELF, 1);
        APIgameobj::SetNextState(e, SELF, 2);
    } else if a == 0 && APIgameobj::IsInCutScene(e, SELF, 0) == 1 && k == 0 {
        APIphysic::ResetFlags(e, SELF, 1);
        APIgameobj::SetNextState(e, SELF, 2);
    }
    Ok(0)
}

/// 0x00012250
pub fn switch_baseConstructor(e: &mut Engine, api: Ptr, param_2: i32) {
    if param_2 == 1 {
        let fns: [(i32, crate::logic::ExtFn); 9] = [
            (0, fstate_FIRSTMOVE),
            (2, fstate_BASICMOVE),
            (3, fcback_TURN_ON_LF),
            (4, fcback_TURN_OFF_LF),
            (5, fcback_ANIM),
            (6, fcback_CollisionDetection),
            (7, fcback_POST_COLLISION),
            (8, fcback_InputAction_X),
            (9, fcback_CollisionDetectionOut),
        ];
        for (i, f) in fns {
            APIcons::SetExternalFunction(e, i, f);
        }
        let tables: [(u32, fn(&mut Engine) -> Ptr); 9] = [
            (pAnim, |e| APIcons::GetApiAnim(e)),
            (pExtra, |e| APIcons::GetApiExtra(e)),
            (pGame, |e| APIcons::GetApiGame(e)),
            (pGameObj, |e| APIcons::GetApiGameObj(e)),
            (pHud, |e| APIcons::GetApiHUD(e)),
            (pInput, |e| APIcons::GetApiInput(e)),
            (pLight, |e| APIcons::GetApiLight(e)),
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
        let slot = G(e, pGlobalDataSwitch);
        APIcons::GetGlobalData(e, slot, 8);
        let b = APIgameobj::GetGameBuffer(e, 0, 0x8f8, 1);
        let g = e.r32(G(e, pGlobalDataSwitch));
        e.w32(g, b);
        let b = APIgameobj::GetGameBuffer(e, 2, 0x158, 1);
        e.w32(g + 4, b);
    }
    let slot = G(e, pLocalDataSwitch);
    APIcons::GetLocalData(e, slot, 0x90);
    lw(e, 0, param_2 as u32 + if B(e).local == Local::CutSw { 0x1f9 } else { 500 });
    switch_baseInitialisation(e);
}

/// switchConstructor (per build)
pub fn switchConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    switch_baseConstructor(e, api, instanceCount);
    let local = B(e).local;
    if instanceCount == 1 {
        match local {
            Local::Reset | Local::Pair => {
                APIcons::SetExternalFunction(e, 1, fstate_FIRSTMOVE_LOCAL);
            }
            Local::Cut => {
                APIcons::SetExternalFunction(e, 10, fcback_CUTSCENE);
            }
            Local::CutSw => {
                APIcons::SetExternalFunction(e, 8, fcback_InputAction_X_Local);
                APIcons::SetExternalFunction(e, 10, fcback_CUTSCENE);
            }
            Local::Set => {}
        }
    }
    APIcons::SetMessageFunction(e, MyLocalMessageFunction);
    match local {
        Local::Pair => LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 2, NULL, 0),
        Local::Cut | Local::CutSw => {
            APIgameobj::RegisterNotify(e, SELF, 0, 0x16, 10);
        }
        _ => {}
    }
    APIcons::SetInitialState(e, if matches!(local, Local::Reset | Local::Pair) { 1 } else { 0 });
    Ok(())
}

pub fn switchDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    Ok(())
}

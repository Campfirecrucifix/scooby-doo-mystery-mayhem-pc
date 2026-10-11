//! `chest_level1`: the EP1L01 costume chest. The leader touching it opens it (prompt 0x31/7 to the
//! radar); X puts the level costume on both heroes (they are moved to the costume objects
//! 0x2a000143/142 via a ground ray, the plain heroes parked at the origin; msg 0xd2/0x595 + 0x6a,
//! camera 0x47, book 0x5c/4, sparkle FX) and starts a 15 s timer (radar 0x2e/0x30); X again or the
//! timer (ext 9 on timer +0x4c) takes it off. Sub-states run from ST_ENTRY_POINT via L+4.
//! Port of `Game Logic Modules/Ep1_Library/Objects/CHEST/LEVEL_1/chest_level1__189c2902/`
//! (chest_level1.c, ep1_chest.c, chest_base.c, Costume_PutOnFX.c; 37 functions). EP1L01 build 189c2902.
//! chest_base.c is shared with `chest_level3a` (EP1L03A, [`Variant`] selected by module name).
//!
//! Deviation: L+4 holds the current sub-state *function pointer* in the original; the port stores
//! a [`Sub`] index (compared/dispatched the same way).
#![allow(non_snake_case, non_upper_case_globals, unused_variables)]

use crate::engine::*;
use crate::logic::{CodeFn, LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "chest_level1",
    // globals 0x10000..0x10038 (+ unused gpoLanguagesNamesAndISOCodes)
    data_size: 0x2260,
    ctor: ctor_dispatch,
    dtor: dtor_dispatch,
    register: None,
    init: &[],
    build_init: Some(build_init_dispatch),
};

mod xb;

/// number of hand-ported builds (indices 0..N_OLD); generated builds (`xb`, c2rs) follow
pub(crate) const N_OLD: usize = 1;

fn ctor_dispatch(e: &mut Engine, api: Ptr, n: i32) -> LResult<()> {
    if e.rt.cur_build() < N_OLD { chest_level1Constructor(e, api, n) } else { xb::ctor(e, api, n) }
}

fn dtor_dispatch(e: &mut Engine, n: i32) -> LResult<()> {
    if e.rt.cur_build() < N_OLD { chest_level1Destructor(e, n) } else { xb::dtor(e, n) }
}

fn build_init_dispatch(m: &mut Mem, data: Ptr, b: usize) {
    if b < N_OLD {
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
    (key == "189c2902").then_some(0)
}

pub const pAnim: u32 = 0x10000;
pub const pExtra: u32 = 0x10004;
pub const pGame: u32 = 0x10008;
pub const pGameObj: u32 = 0x1000c;
pub const pHud: u32 = 0x10010;
pub const pInput: u32 = 0x10014;
pub const pPhysic: u32 = 0x10018;
pub const pSound: u32 = 0x1001c;
pub const pAnimMngr: u32 = 0x10020;
pub const pDebug: u32 = 0x10024;
pub const pSndMngr: u32 = 0x10028;
pub const pMiscFunc: u32 = 0x1002c;
pub const pLocalDataChestBase: u32 = 0x10030;
pub const pGlobalDataChestBase: u32 = 0x10034;

/// Per-level literals of chest_base.c
pub(crate) struct Variant {
    /// hero objects (Scooby, Shaggy)
    hero_a: u32,
    hero_b: u32,
    /// their costume objects
    cost_a: u32,
    cost_b: u32,
    snd_chest: u32,
    snd_a: u32,
    snd_b: u32,
    snd_both: u32,
    camera: u32,
    book: u32,
    fx_part: u32,
    latent: u32,
    radar: u32,
}
const L01: Variant = Variant {
    hero_a: 0x2a00_0091,
    hero_b: 0x2a00_0092,
    cost_a: 0x2a00_0143,
    cost_b: 0x2a00_0142,
    snd_chest: 0x4000_000c,
    snd_a: 0x8000_0055,
    snd_b: 0x8000_0056,
    snd_both: 0xa000_0004,
    camera: 0x2a00_008f,
    book: 0x2a00_0096,
    fx_part: 0x2a80_0138,
    latent: 0x2a80_0144,
    radar: 0x2a10_0006,
};
const L03A: Variant = Variant {
    hero_a: 0x2a00_00f0,
    hero_b: 0x2a00_00ef,
    cost_a: 0x2a00_00e9,
    cost_b: 0x2a00_00e8,
    snd_chest: 0x4000_000f,
    snd_a: 0x8000_0029,
    snd_b: 0x8000_002a,
    snd_both: 0xa000_0000,
    camera: 0x2a00_004a,
    book: 0x2a00_00ed,
    fx_part: 0x2a80_00dc,
    latent: 0x2a80_00ec,
    radar: 0x2a10_0000,
};
fn V(e: &Engine) -> &'static Variant {
    if e.rt.codes[e.rt.current_code().expect("no logic running")].def.name == "chest_level3a" { &L03A } else { &L01 }
}

/// L+4 sub-state (function pointer in the original)
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
enum Sub {
    BasicMove = 1,
    Opened,
    OpenedReadyToClose,
    Closed,
    ClosedReadyToOpen,
    Opening,
    Closing,
}

#[inline]
fn G(e: &Engine, a: u32) -> Ptr {
    e.rt.cur_data() + (a - 0x10000)
}
/// pLocalDataChestBase (0xa8): +4 sub-state, +8 pos, +0x14 touched, +0x18 prompt, +0x1c, +0x24,
/// +0x28 input cfg, +0x30 latent obj, +0x34 latent id, +0x38 prompt hidden, +0x48 DM, +0x4c costume
/// timer, +0x50 close timer, +0x54 open timer, +0x58 costume time (15.0), +0x5c/+0x60 paused,
/// +0x64 event flags, +0x68 clipping
#[inline]
fn L(e: &Engine) -> Ptr {
    e.r32(G(e, pLocalDataChestBase))
}
#[inline]
fn lr(e: &Engine, o: u32) -> u32 {
    e.r32(L(e) + o)
}
#[inline]
fn lw(e: &mut Engine, o: u32, v: u32) {
    let a = L(e) + o;
    e.w32(a, v);
}
fn fl(e: &Engine) -> u32 {
    lr(e, 100)
}
fn fset(e: &mut Engine, v: u32) {
    lw(e, 100, v);
}
fn sub(e: &Engine) -> u32 {
    lr(e, 4)
}
fn set_sub(e: &mut Engine, s: Sub) {
    lw(e, 4, s as u32);
}
/// *pGlobalDataChestBase (900 bytes): [0] GB1, [1] GB0, [2] GB2, [3] GB6, +0x10 vec, +0x1c/+0x20 FX,
/// +0x24.. generic FX params (two particle slots of 0xac)
#[inline]
fn GD(e: &Engine) -> Ptr {
    e.r32(G(e, pGlobalDataChestBase))
}
fn GB0(e: &Engine) -> Ptr {
    e.r32(GD(e) + 4)
}
fn GB2(e: &Engine) -> Ptr {
    e.r32(GD(e) + 8)
}
fn f14(e: &Engine) -> u32 {
    e.r32(GB0(e) + 0x14)
}
fn f34(e: &Engine) -> u32 {
    e.r32(GB0(e) + 0x34)
}
fn dead(e: &Engine) -> bool {
    e.r32(GB2(e)) & 0x20000 != 0
}
fn send(e: &mut Engine, to: u32, m: u32, p: u32, d: Ptr) {
    APIgameobj::SendMessageTo(e, to, NULL, m, p, d);
}

// ---- chest_level1.c / ep1_chest.c

pub fn chest_level1_FUNC_MessageFunction(e: &mut Engine, m: u32, p: u32, d: Ptr) -> LResult<u64> {
    ChestBaseCB_MessageFunction(e, m, p, d)
}

pub fn chest_level1Constructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    chest_baseConstructor(e, api, instanceCount);
    APIcons::SetMessageFunction(e, chest_level1_FUNC_MessageFunction);
    ChestBaseFUNC_CreateGameObject(e);
    APIcons::SetInitialState(e, 0);
    Ok(())
}

pub fn chest_level1Destructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    chest_baseDestructor(e, instanceCount);
    Ok(())
}

// ---- Costume_PutOnFX.c (param_1 = APIparticle table)

pub fn SetParticleCostume_spark(e: &mut Engine, _: u32, t: u32) -> LResult<()> {
    let t = t as i32;
    for (a, v) in [(8, 10), (1, 2), (4, 0x96), (0x10, 0), (0x20, 0), (0x40, 0), (0x80, 0), (0x100, 0)] {
        APIparticle::TypeSetInt(e, t, a, v);
    }
    for (a, v) in [(1, 0), (4, 0), (2, 0x3fc0_0000), (8, 0x4040_0000)] {
        APIparticle::TypeSetFloat(e, t, a, f32::from_bits(v));
    }
    APIparticle::TypeSetVec(e, t, 1, 0.0, -4.0, 0.0);
    APIparticle::TypeSetVec(e, t, 2, 0.0, 0.0, 0.0);
    inter(e, t, &[(1, [0, 0, 0, 0x3f00_0000]), (2, [0x3f80_0000, 0x3f80_0000, 0, 0x3f00_0000]), (4, [0x3f80_0000, 0x3f80_0000, 0, 0x3f00_0000]), (8, [0; 4]), (0x10, [0x3ecc_cccd, 0x3e80_0000, 0x3dcc_cccd, 0x3f40_0000]), (0x20, [0, 0x3f00_0000, 0x3f80_0000, 0x3f00_0000])]);
    Ok(())
}

pub fn SetParticleCostume_glow(e: &mut Engine, _: u32, t: u32) -> LResult<()> {
    let t = t as i32;
    for (a, v) in [(8, 1), (1, 2), (4, 0x14a), (0x10, 0), (0x20, 0), (0x40, 0), (0x80, 0), (0x100, 0)] {
        APIparticle::TypeSetInt(e, t, a, v);
    }
    for (a, v) in [(1, 0), (4, 0x3f26_6666), (2, 0x3f00_0000), (8, 0)] {
        APIparticle::TypeSetFloat(e, t, a, f32::from_bits(v));
    }
    APIparticle::TypeSetVec(e, t, 1, 0.0, 0.0, 0.0);
    APIparticle::TypeSetVec(e, t, 2, 0.0, 0.0, 0.0);
    inter(e, t, &[(1, [0x3f00_0000, 0x3e80_0000, 0x3c23_d70a, 0x3e80_0000]), (2, [0x3f00_0000, 0, 0x3c23_d70a, 0x3e80_0000]), (4, [0x3f80_0000, 0x3e80_0000, 0x3c23_d70a, 0x3e80_0000]), (8, [0, 0x3f00_0000, 0, 0]), (0x10, [0x40b0_0000, 0x4070_0000, 0x4000_0000, 0x3e80_0000]), (0x20, [0x3e00_0000, 0x3e00_0000, 0x3e00_0000, 0x3f80_0000])]);
    Ok(())
}

fn inter(e: &mut Engine, t: i32, v: &[(u32, [u32; 4])]) {
    for &(a, [x, y, z, w]) in v {
        let f = f32::from_bits;
        APIparticle::TypeSetInter(e, t, a, f(x), f(y), f(z), f(w));
    }
}

// ---- chest_base.c

/// state 0
pub fn ChestBaseST_FIRSTMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    let l = L(e);
    if e.r32(G(e, pDebug)) != 0 {
        let (a, b) = (e.cstr("Chest_Base"), e.cstr("fstate_FIRSTMOVE"));
        LIBdebug::PrintDebugInfo(e, a, b);
    }
    e.w32(l + 100, 0);
    APIgameobj::GetPosition(e, SELF, 0x7fff_ffff, l + 8);
    e.w32(l + 0x14, 0);
    e.w32(l + 0x38, 0);
    LIBAnimMngr::PlayAnim(e, 0.0, 1, 1, 0);
    APIgameobj::SetNextState(e, SELF, 1);
    e.w32(l + 4, Sub::BasicMove as u32);
    Ok(0)
}

/// state 1
pub fn ChestBaseST_ENTRY_POINT(e: &mut Engine, _: u32) -> LResult<u64> {
    let l = L(e);
    ChestBaseFUNC_AllStates_EventCheck(e);
    match e.r32(l + 4) {
        1 => ChestBaseST_BASICMOVE(e),
        2 => ChestBaseST_OPENED(e),
        3 => ChestBaseST_OPENED_READY_TO_CLOSE(e),
        4 => ChestBaseST_CLOSED(e),
        5 => ChestBaseST_CLOSED_READY_TO_OPEN(e),
        6 => ChestBaseST_OPENING(e),
        7 => ChestBaseST_CLOSING(e),
        s => panic!("chest: call through bad sub-state {s:#x}"),
    }
    Ok(0)
}

pub fn ChestBaseST_BASICMOVE(e: &mut Engine) {
    set_sub(e, Sub::ClosedReadyToOpen);
}

/// clears `bit`; true if it was set
fn take(e: &mut Engine, bit: u32) -> bool {
    let f = fl(e);
    if f & bit != 0 {
        fset(e, f & !bit);
        true
    } else {
        false
    }
}

pub fn ChestBaseST_OPENED(e: &mut Engine) {
    let c = lr(e, 0x28) as i32;
    APIinput::InputConfigActivate(e, c, 1);
    if take(e, 4) {
        ChestBaseFUNC_Put_Remove_Costume(e);
    } else if take(e, 0x20) {
        ChestBaseFUNC_GettingReadyToBeClosed(e);
    }
}

pub fn ChestBaseST_OPENED_READY_TO_CLOSE(e: &mut Engine) {
    if take(e, 4) {
        ChestBaseFUNC_Put_Remove_Costume(e);
    } else if take(e, 2) {
        ChestBaseFUNC_Close_Chest(e);
    }
}

pub fn ChestBaseST_CLOSED(e: &mut Engine) {
    if take(e, 0x10) {
        ChestBaseFUNC_GettingReadyToBeOpened(e);
    } else if take(e, 4) {
        ChestBaseFUNC_Put_Remove_Costume(e);
    }
}

pub fn ChestBaseST_CLOSED_READY_TO_OPEN(e: &mut Engine) {
    if fl(e) & 1 == 0 || f14(e) & 0x0620_0000 != 0 {
        if take(e, 4) {
            ChestBaseFUNC_Put_Remove_Costume(e);
        }
    } else {
        ChestBaseFUNC_Open_Chest(e);
    }
}

pub fn ChestBaseST_OPENING(e: &mut Engine) {
    let c = lr(e, 0x28) as i32;
    APIinput::InputConfigActivate(e, c, 1);
    if take(e, 0x200) {
        set_sub(e, Sub::Opened);
    } else if take(e, 4) {
        ChestBaseFUNC_Put_Remove_Costume(e);
    }
}

pub fn ChestBaseST_CLOSING(e: &mut Engine) {
    if take(e, 0x100) {
        set_sub(e, Sub::Closed);
    } else if take(e, 4) {
        ChestBaseFUNC_Put_Remove_Costume(e);
    }
}

/// ext fn 7 (collision notify, +8 other)
pub fn ChestBaseCB_COLLISION_IN(e: &mut Engine, p: u32) -> LResult<u64> {
    if e.r32(p + 8) == e.r32(GB0(e) + 8) {
        lw(e, 0x14, 1);
        if lr(e, 0x18) == 0 {
            let f = (fl(e) | 1) & !2 | 0x2000;
            fset(e, f);
            lw(e, 0x18, 1);
            APIgameobj::SetContextFlag(e, SELF, 4);
            let c = L(e) + 0x68;
            LIBMiscFunc::MiscFunc_RemoveBitClipping(e, c, 0x80);
        }
    }
    Ok(0)
}

fn untouch(e: &mut Engine) {
    lw(e, 0x18, 0);
    let f = (fl(e) | 2) & !1 | 0x4000;
    fset(e, f);
    let r = V(e).radar;
    send(e, r, 0x32, 7, NULL);
}

/// ext fn 2
pub fn ChestBaseCB_POST_COLLISION(e: &mut Engine, _: u32) -> LResult<u64> {
    if lr(e, 0x14) == 0 {
        if lr(e, 0x18) != 0 {
            untouch(e);
        }
        let c = L(e) + 0x68;
        LIBMiscFunc::MiscFunc_ReplaceMyClipping(e, c);
        APIgameobj::SetContextFlag(e, SELF, 0);
    }
    lw(e, 0x14, 0);
    Ok(0)
}

/// ext fn 6 (anim last frame, param = anim)
pub fn ChestBaseCB_LASTFRAME(e: &mut Engine, p: u32) -> LResult<u64> {
    match p {
        3 => fset(e, fl(e) | 0x200),
        4 => fset(e, fl(e) | 0x100),
        _ => {}
    }
    Ok(0)
}

/// ext fn 9 (param = timer)
pub fn ChestBaseCB_TIMER(e: &mut Engine, p: u32) -> LResult<u64> {
    if p == lr(e, 0x50) {
        fset(e, fl(e) | 0x20);
    } else if p == lr(e, 0x4c) {
        fset(e, fl(e) | 0x40);
    } else if p == lr(e, 0x54) {
        fset(e, fl(e) | 0x10);
    }
    Ok(0)
}

pub fn ChestBaseCB_MessageFunction(e: &mut Engine, m: u32, p: u32, _d: Ptr) -> LResult<u64> {
    let t = lr(e, 0x4c);
    match m {
        0x1e5 => {
            APIextra::TimerSetFrequency(e, t, 0.1);
            APIextra::TimerReset(e, t);
        }
        0x12 => {
            ResetCollision(e);
            APIphysic::SetFlags(e, SELF, 1);
        }
        0x13 => {
            APIphysic::ResetFlags(e, SELF, 1);
        }
        0x1e7 => {
            let on = (p != 0) as u32;
            lw(e, 0x5c, on);
            APIextra::TimerPause(e, t, on);
        }
        0x1e6 => {
            APIextra::TimerUnregisterNotify(e, SELF, t);
        }
        0x1e8 if f14(e) & 0x40 != 0 => {
            fset(e, fl(e) & 0xffff_ffbb);
            ChestBaseFUNC_Put_Remove_Costume(e);
        }
        _ => {}
    }
    Ok(0)
}

/// ext fn 10
pub fn ChestBaseCB_INPUT_X(e: &mut Engine, _: u32) -> LResult<u64> {
    if f14(e) & 0x40_0000 == 0 {
        fset(e, fl(e) | 4);
    }
    Ok(0)
}

pub fn ResetCollision(e: &mut Engine) {
    if lr(e, 0x18) != 0 {
        untouch(e);
    }
    let c = lr(e, 0x28) as i32;
    APIinput::InputConfigActivate(e, c, 0);
    let c = L(e) + 0x68;
    LIBMiscFunc::MiscFunc_ReplaceMyClipping(e, c);
    APIgameobj::SetContextFlag(e, SELF, 0);
    lw(e, 0x14, 0);
}

pub fn ChestBaseFUNC_AllStates_EventCheck(e: &mut Engine) {
    let radar = V(e).radar;
    if fl(e) & 0x40 == 0 {
        if fl(e) & 0x10000 != 0 && f14(e) & 0x40 != 0 {
            let t = lr(e, 0x4c);
            if lr(e, 0x60) == 0 {
                if f14(e) & 0x40_010c != 0 {
                    lw(e, 0x60, 1);
                    APIextra::TimerPause(e, t, 1);
                }
            } else if f14(e) & 0x40_010c == 0 {
                lw(e, 0x60, 0);
                APIextra::TimerPause(e, t, 0);
            }
            if lr(e, 0x5c) == 0 && lr(e, 0x60) == 0 {
                let d = L(e) + 0x58;
                send(e, radar, 0x30, 0, d);
            }
        }
    } else {
        ChestBaseFUNC_Ready_To_Remove_Costume(e);
    }
    let s = sub(e);
    let open = s == Sub::Opened as u32 || s == Sub::Opening as u32 || s == Sub::OpenedReadyToClose as u32;
    if dead(e) && fl(e) & 0x4000 != 0 && lr(e, 0x38) == 1 && (s == Sub::Closed as u32 || s == Sub::Closing as u32) {
        fset(e, fl(e) & 0xffff_bfff);
        lw(e, 0x38, 0);
        send(e, radar, 0x32, 7, NULL);
    } else if !dead(e) && f14(e) & 0x0620_0000 == 0 && fl(e) & 0x2000 != 0 && lr(e, 0x38) == 0 && open {
        fset(e, fl(e) & 0xffff_dfff);
        lw(e, 0x38, 1);
    }
    let s = sub(e);
    let open = s == Sub::Opened as u32 || s == Sub::Opening as u32 || s == Sub::OpenedReadyToClose as u32;
    if fl(e) & 1 != 0 && !dead(e) && f14(e) & 0x0620_0000 == 0 && open {
        let k = if f14(e) & 0x40 == 0 { 7 } else { 8 };
        send(e, radar, 0x31, k, NULL);
    }
}

pub fn ChestBaseFUNC_Open_Chest(e: &mut Engine) {
    let l = L(e);
    ChestBaseFUNC_StartTimer_ReadyToClose(e);
    let t = e.r32(l + 0x54);
    APIextra::TimerUnregisterNotify(e, SELF, t);
    LIBAnimMngr::PlayAnim(e, 0.0, 3, 1, 6);
    if f14(e) & 0x40 == 0 {
        let id = lr(e, 0x34);
        let o = APIgameobj::LaunchLatant(e, id);
        e.w32(l + 0x30, o);
        APIgameobj::AddChild(e, SELF, 6, o, 1);
    }
    let c = lr(e, 0x28) as i32;
    APIinput::InputConfigActivate(e, c, 1);
    e.w32(l + 4, Sub::Opening as u32);
}

pub fn ChestBaseFUNC_Close_Chest(e: &mut Engine) {
    let l = L(e);
    ChestBaseFUNC_StartTimer_ReadyToOpen(e);
    let t = e.r32(l + 0x50);
    APIextra::TimerUnregisterNotify(e, SELF, t);
    let c = e.r32(l + 0x28) as i32;
    APIinput::InputConfigActivate(e, c, 0);
    LIBAnimMngr::PlayAnim(e, 0.0, 4, 1, 6);
    let o = e.r32(l + 0x30);
    APIgameobj::KillObject(e, o);
    e.w32(l + 4, Sub::Closing as u32);
}

pub fn ChestBaseFUNC_GettingReadyToBeClosed(e: &mut Engine) {
    fset(e, fl(e) & 0xffff_ffdf);
    set_sub(e, Sub::OpenedReadyToClose);
}

pub fn ChestBaseFUNC_GettingReadyToBeOpened(e: &mut Engine) {
    fset(e, fl(e) & 0xffff_ffef);
    set_sub(e, Sub::ClosedReadyToOpen);
}

pub fn ChestBaseFUNC_Ready_To_Remove_Costume(e: &mut Engine) {
    if f14(e) & 0x6e20_090d == 0 {
        fset(e, fl(e) & 0xffff_ffbf);
        ChestBaseFUNC_Put_Remove_Costume(e);
    }
}

/// Drops `dst` onto the ground under `src` (ray 2.5 m down from src + 0.2) and gives it src's orientation.
pub fn ChestBaseFUNC_SetNewPos(e: &mut Engine, src: u32, dst: u32) {
    let p = e.alloca(0x10); // DStack_b0
    APIgameobj::GetPosition(e, src, 0x7fff_ffff, p);
    let [x, y, z] = e.rv3(p);
    let r = e.alloca(0x30); // fStack_90
    e.wv3(r, [x, y + 0.2, z]);
    e.wv3(r + 0xc, [0.0, -2.5, 0.0]);
    for (i, v) in [0, 0, 1, 0, 0].into_iter().enumerate() {
        e.w32(r + 0x18 + 4 * i as u32, v);
    }
    let out = e.alloca(0x50); // aDStack_60
    if APIphysic::CheckRayIntersection(e, r, out) == 2 {
        APIgameobj::SetTranslation(e, dst, out);
    } else {
        APIgameobj::SetTranslation(e, dst, p);
    }
    APIgameobj::TakeOrientation(e, dst, src, 0x7fff_ffff);
}

fn vfx_at(e: &mut Engine, hero: u32, fx_slot: u32) {
    let gd = GD(e);
    let v = e.alloca(0x10);
    let id = APIgameobj::GetID(e, hero, NULL);
    APIgameobj::GetPosition(e, id, 3, v);
    let p = e.rv3(v);
    for base in [0xac, 0x158] {
        e.wv3(gd + base, p);
        for o in (0xc..0x24).step_by(4) {
            e.w32(gd + base + o, 0);
        }
    }
    let fx = e.ri32(gd + fx_slot);
    LIBMiscFunc::MiscFunc_PlayGenericFX(e, fx);
}

pub fn ChestBaseFUNC_StartVFX(e: &mut Engine) {
    let h = e.r32(GB0(e) + 8);
    vfx_at(e, h, 0x1c);
    if f34(e) & 0x10 != 0 {
        let h = e.r32(GB0(e) + 0xc);
        vfx_at(e, h, 0x20);
    }
}

pub fn ChestBaseFUNC_Put_Remove_Costume(e: &mut Engine) {
    let v = V(e);
    let zero = e.alloca(0x10); // DStack_a0
    let mut cam = 0u32; // uStack_90
    if f14(e) & 0x6e60_090d != 0 {
        return;
    }
    let gb0 = GB0(e);
    if f14(e) & 0x40 == 0 {
        let s1 = APIphysic::GetPhysicState(e, e.r32(gb0 + 8));
        let s2 = APIphysic::GetPhysicState(e, e.r32(gb0 + 0xc));
        let (p1, p2) = (e.alloca(0x10), e.alloca(0x10));
        APIgameobj::GetPosition(e, e.r32(gb0 + 8), 0x7fff_ffff, p1);
        APIgameobj::GetPosition(e, e.r32(gb0 + 0xc), 0x7fff_ffff, p2);
        if s1 & 1 == 0 {
            return;
        }
        if f34(e) & 0x10 != 0 && f34(e) & 0x100 == 0 {
            let (y1, y2) = (e.rf(p1 + 4), e.rf(p2 + 4));
            if s2 & 1 == 0 || y1 + 0.5 <= y2 || y2 <= y1 - 0.5 {
                return;
            }
        }
    }
    ChestBaseFUNC_Close_Chest(e);
    let gb6 = e.r32(GD(e) + 0xc);
    e.w32(gb6, 1);
    let leader = e.r32(gb0 + 8);
    if f14(e) & 0x40 == 0 || fl(e) & 0x10000 == 0 {
        if f14(e) & 0x40 != 0 {
            return;
        }
        // put the costume on
        fset(e, fl(e) | 0x10000);
        let t = lr(e, 0x4c);
        APIextra::TimerReset(e, t);
        let secs = e.rf(L(e) + 0x58);
        APIextra::TimerSetFrequency(e, t, secs);
        APIextra::TimerRegisterNotify(e, SELF, t, 9);
        let d = L(e) + 0x58;
        send(e, v.radar, 0x2e, 0, d);
        if leader == v.hero_a {
            cam = 3;
        } else if leader == v.hero_b {
            cam = 4;
        }
        e.w32(gb0 + 0x14, f14(e) | 0x40);
        let gd10 = GD(e) + 0x10;
        if f34(e) & 0x10 == 0 {
            APIgameobj::GetPosition(e, e.r32(gb0 + 0xc), 0x7fff_ffff, gd10);
            let (h, c) = if leader == v.hero_a { (v.hero_a, v.cost_a) } else { (v.hero_b, v.cost_b) };
            ChestBaseFUNC_SetNewPos(e, h, c);
            send(e, c, 0xd2, 0x6a, NULL);
            APIgameobj::SetTranslation(e, h, zero);
        } else {
            ChestBaseFUNC_SetNewPos(e, v.hero_a, v.cost_a);
            ChestBaseFUNC_SetNewPos(e, v.hero_b, v.cost_b);
            APIgameobj::GetPosition(e, e.r32(gb0 + 0xc), 0x7fff_ffff, gd10);
            APIgameobj::SetTranslation(e, v.hero_a, zero);
            APIgameobj::SetTranslation(e, v.hero_b, zero);
            send(e, v.cost_a, 0xd2, 0x6a, NULL);
            send(e, v.cost_b, 0xd2, 0x6a, NULL);
        }
        send(e, v.hero_a, 0xd2, 0x595, NULL);
        send(e, v.hero_b, 0xd2, 0x595, NULL);
        APInewSound::Play(e, SELF, v.snd_chest, 0);
        let quiet = lr(e, 0x24) != 0;
        if f34(e) & 0x10 == 0 || f34(e) & 0x100 != 0 {
            if leader == v.hero_a && !quiet {
                APInewSound::Play(e, SELF, v.snd_a, 0);
            } else if leader == v.hero_b && !quiet {
                APInewSound::Play(e, SELF, v.snd_b, 0);
            }
        } else if !quiet {
            APInewSound::Play(e, SELF, v.snd_both, 0);
        }
        send(e, v.camera, 0x47, cam, NULL);
        if !quiet {
            send(e, v.book, 0x5c, 4, NULL);
        }
        ChestBaseFUNC_StartVFX(e);
        send(e, SELF, 0x1e9, 0, NULL);
    } else {
        // take it off
        fset(e, fl(e) & 0xfffe_ffff);
        e.w32(gb0 + 0x14, f14(e) & 0xffff_ffbf);
        let rp = e.alloca(0x20); // uStack_60: retriever {1, group 0x25}
        e.w32(rp, 1);
        e.w32(rp + 4, 0x25);
        APIgameobj::SendMessageTo(e, 0xff00_0008, rp, 0x1e6, 0, NULL);
        let (c88, c8c) = (e.r32(gb0 + 8), e.r32(gb0 + 0xc));
        let (h40, h3c) = match e.r32(gb0 + 4) {
            3 | 5 | 7 | 9 | 0xe | 0x10 => {
                cam = 1;
                (v.hero_a, v.hero_b)
            }
            _ => {
                cam = 2;
                (v.hero_b, v.hero_a)
            }
        };
        ChestBaseFUNC_SetNewPos(e, c88, h40);
        APIgameobj::SetTranslation(e, c88, zero);
        if f34(e) & 0x10 != 0 {
            if f34(e) & 0x100 == 0 {
                ChestBaseFUNC_SetNewPos(e, c8c, h3c);
                APIgameobj::TakeOrientation(e, h3c, c8c, 0x7fff_ffff);
                APIgameobj::SetTranslation(e, c8c, zero);
            } else {
                let gd10 = GD(e) + 0x10;
                APIgameobj::SetTranslation(e, h3c, gd10);
                APIgameobj::SetTranslation(e, c8c, zero);
            }
        }
        send(e, c88, 0xd2, 0x595, NULL);
        send(e, c8c, 0xd2, 0x595, NULL);
        send(e, v.hero_a, 0xd2, 0x6a, NULL);
        send(e, v.hero_b, 0xd2, 0x6a, NULL);
        APInewSound::Play(e, SELF, v.snd_chest, 0);
        send(e, v.camera, 0x47, cam, NULL);
        send(e, v.radar, 0x2f, 0, NULL);
        send(e, v.book, 0x5c, 1, NULL);
        ChestBaseFUNC_StartVFX(e);
    }
}

pub fn ChestBaseFUNC_StartTimer_ReadyToOpen(e: &mut Engine) {
    let t = lr(e, 0x54);
    APIextra::TimerReset(e, t);
    APIextra::TimerSetFrequency(e, t, 1.0);
    APIextra::TimerRegisterNotify(e, SELF, t, 9);
}

pub fn ChestBaseFUNC_StartTimer_ReadyToClose(e: &mut Engine) {
    let t = lr(e, 0x50);
    APIextra::TimerReset(e, t);
    APIextra::TimerSetFrequency(e, t, 0.5);
    APIextra::TimerRegisterNotify(e, SELF, t, 9);
}

pub fn ChestBaseFUNC_CreatePhysic(e: &mut Engine) {
    APIphysic::CreatePhysic(e, SELF, 0);
    APIphysic::SetFlags(e, SELF, 2);
    APIphysic::SetWeight(e, SELF, 1.0);
    APIphysic::SetPriority(e, SELF, 100);
    APIphysic::PrimSetIntAttribBits(e, SELF, 0x32, 0, 4);
    let dm = APIphysic::DMAdd(e, SELF, 0x25);
    lw(e, 0x48, dm);
    let p = e.alloca(0x10); // auStack_30
    APIphysic::DMGetParams(e, dm, p);
    e.w32(p + 4, 2);
    APIphysic::DMSetParams(e, dm, p);
    APIphysic::DMSetContextMask(e, dm, 4);
}

pub fn ChestBaseFUNC_CreateGameObject(e: &mut Engine) -> u32 {
    let l = L(e);
    ChestBaseFUNC_CreatePhysic(e);
    for o in [0x4c, 0x50, 0x54] {
        let t = APIextra::TimerCreate(e);
        e.w32(l + o, t);
    }
    APIgameobj::SetGroupID(e, SELF, 0x25);
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 3, 7);
    APIgameobj::SetContextFlag(e, SELF, 0);
    let c = e.r32(l + 0x28) as i32;
    APIinput::InputConfigAdd(e, c);
    APIinput::InputActionAdd(e, c, 0x39, 0);
    let port = e.r32(GB0(e) + 0x4c) as i32;
    APIinput::InputDeviceAdd(e, c, 0x39, 0x3d, port, 0, 2, 0, 0);
    APIinput::InputDeviceAdd(e, c, 0x39, 0x3d, 1, 1, 2, 0, 0x39);
    APIinput::InputContextAdd(e, SELF, 10, c, 0x39, 0x1e, 8);
    APIinput::InputContextSetState(e, c, 0x39, 0x1e, 2);
    APIinput::InputConfigSetState(e, c, 2);
    APIinput::InputConfigActivate(e, c, 0)
}

pub fn ChestBaseFUNC_InitParticles(e: &mut Engine) {
    let g = GD(e);
    e.w32(g + 0x24, 0x4000_0000);
    for (o, s) in [(0x68, "spark_collect"), (0x28, "spark")] {
        let p = e.cstr(s);
        LIBMiscFunc::MiscFunc_StrncpySafe(e, g + o, p, 0x40);
    }
    let t = e.code_ptr(CodeFn::Particle(SetParticleCostume_spark));
    e.w32(g + 0xa8, t);
    e.w32(g + 0xd0, 0);
    for (o, s) in [(0x114, "glow_collect"), (0xd4, "star_light")] {
        let p = e.cstr(s);
        LIBMiscFunc::MiscFunc_StrncpySafe(e, g + o, p, 0x40);
    }
    let t = e.code_ptr(CodeFn::Particle(SetParticleCostume_glow));
    e.w32(g + 0x154, t);
    e.w32(g + 0x17c, 0);
}

pub fn chest_baseConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) {
    if instanceCount == 1 {
        let fns: [(i32, crate::logic::ExtFn); 7] = [
            (0, ChestBaseST_FIRSTMOVE),
            (1, ChestBaseST_ENTRY_POINT),
            (2, ChestBaseCB_POST_COLLISION),
            (6, ChestBaseCB_LASTFRAME),
            (7, ChestBaseCB_COLLISION_IN),
            (9, ChestBaseCB_TIMER),
            (10, ChestBaseCB_INPUT_X),
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
        for (slot, name) in [(pAnimMngr, "AnimMngr"), (pDebug, "debug"), (pSndMngr, "SndMngr"), (pMiscFunc, "MiscFunc")] {
            let n = e.cstr(name);
            let v = APIcons::GetApiCustom(e, n);
            let a = G(e, slot);
            e.w32(a, v);
        }
        let slot = G(e, pGlobalDataChestBase);
        APIcons::GetGlobalData(e, slot, 900);
        let gd = e.r32(slot);
        for (w, n, size) in [(1u32, 0u32, 0x8f8u32), (0, 1, 0x68), (2, 2, 0x158), (3, 6, 0x14)] {
            let b = APIgameobj::GetGameBuffer(e, n, size, 1);
            e.w32(gd + 4 * w, b);
        }
        ChestBaseFUNC_InitParticles(e);
        e.w32(gd + 0x1c, 0);
        e.w32(gd + 0x20, 0);
        let part = V(e).fx_part;
        for o in [0x1c, 0x20] {
            let h = LIBMiscFunc::MiscFunc_CreateGenericFX(e, part, gd + 0x24, 2);
            e.w32(gd + o, h);
        }
    }
    let slot = G(e, pLocalDataChestBase);
    APIcons::GetLocalData(e, slot, 0xa8);
    let latent = V(e).latent;
    for (o, v) in [(0x58, 0x4170_0000), (0x34, latent), (0x28, (instanceCount + 1000) as u32), (0x1c, 1), (0x24, 0)] {
        lw(e, o, v);
    }
    LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 2, NULL, 1);
}

pub fn chest_baseDestructor(e: &mut Engine, instanceCount: i32) {
    for o in [0x4c, 0x50, 0x54] {
        let t = lr(e, o);
        APIextra::TimerDestroy(e, t);
    }
    if instanceCount == 0 {
        let gd = GD(e);
        for o in [0x1c, 0x20] {
            let h = e.ri32(gd + o);
            LIBMiscFunc::MiscFunc_DestroyGenericFX(e, h);
        }
    }
}

//! `inv_switchsolver_01a`: EP2L01A invisible solver for the four-switch puzzle (switch objects
//! 0x2a000160..163, GB2+8 bits 0x100000..0x800000). Each switch change re-evaluates the 16-state truth table
//! (sends a pose message to 0x2a000169/16a/167); 0101 solves it. Port of
//! `Ep2_MovieStudio/Objects/INV_SWITCHSOLVER/LEVEL_01A/inv_switchsolver_01a` (1 build). The decompiled
//! CheckSolution if-cascade was evaluated exhaustively (node) into `TABLE`; null_printf calls dropped.
#![allow(non_snake_case, non_upper_case_globals, unused_variables)]

use super::modkit::*;
use crate::engine::*;
use crate::logic::{LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "inv_switchsolver_01a",
    data_size: 0x20,
    ctor: inv_switchsolver_01aConstructor,
    dtor: inv_switchsolver_01aDestructor,
    register: None,
    init: &[],
    build_init: None,
};

pub const pGameObj: u32 = 0x10000;
pub const pLight: u32 = 0x10004;
pub const pSound: u32 = 0x10008;
pub const pExtra: u32 = 0x1000c;
pub const pDebug: u32 = 0x10010;
pub const pMiscFunc: u32 = 0x10014;
pub const pLocalDataSwitchSolver: u32 = 0x10018;
pub const pGlobalDataSwitchSolver: u32 = 0x1001c;

/// local (0x6c): 0..0x48 camera message block (msg 0x54), +0x4c first-change sound pending, +0x50 initialising,
/// +0x54..+0x60 switch 1..4 on, +100 solved, +0x68 saved status bits
fn L(e: &Engine, o: u32) -> Ptr {
    pw(e, pLocalDataSwitchSolver, o)
}
fn lr(e: &Engine, o: u32) -> u32 {
    e.r32(L(e, o))
}
fn lw(e: &mut Engine, o: u32, v: u32) {
    let a = L(e, o);
    e.w32(a, v);
}
fn gb(e: &Engine, i: u32) -> Ptr {
    e.r32(pw(e, pGlobalDataSwitchSolver, i * 4))
}
fn gb2_8(e: &Engine) -> u32 {
    e.r32(gb(e, 1) + 8)
}
fn set_gb2_8(e: &mut Engine, v: u32) {
    let a = gb(e, 1) + 8;
    e.w32(a, v);
}
fn send(e: &mut Engine, to: u32, m: u32, p: u32, d: Ptr) {
    APIgameobj::SendMessageTo(e, to, NULL, m, p, d);
}

/// state index (s1<<3|s2<<2|s3<<1|s4) -> message slot for 0x2a000169, 0x2a00016a, 0x2a000167
const TABLE: [[usize; 3]; 16] = [
    [5, 2, 0], [4, 2, 0], [4, 3, 0], [4, 3, 0],
    [5, 2, 1], [4, 2, 1], [5, 2, 0], [4, 2, 0],
    [4, 3, 1], [4, 3, 1], [5, 3, 1], [5, 3, 1],
    [5, 3, 0], [5, 3, 0], [5, 2, 0], [5, 2, 0],
];

pub fn inv_switchSolver_01AST_FIRSTMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    APIgameobj::Sleep(e, SELF, 1.0);
    inv_switchSolver_SavePuzzleStatus(e);
    inv_switchSolver_01AFUNC_Initialisation(e)?;
    lw(e, 0x4c, 1);
    APIgameobj::SetNextState(e, SELF, 1);
    Ok(0)
}

pub fn inv_switchSolver_01AST_BASICMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

pub fn inv_switchSolver_01AFUNC_CheckSolution(e: &mut Engine) -> LResult<u32> {
    let init = lr(e, 0x50) != 0;
    let msgs: [u32; 6] = if !init {
        [0x128, 0x127, 0x130, 0x12f, 300, 299]
    } else {
        [0x12a, 0x129, 0x131, 0x132, 0x12e, 0x12d]
    };
    let s = [lr(e, 0x54), lr(e, 0x58), lr(e, 0x5c), lr(e, 0x60)];
    let k = s.iter().fold(0, |a, &b| a << 1 | (b != 0) as usize);
    for (to, slot) in [0x2a000169, 0x2a00016a, 0x2a000167].into_iter().zip(TABLE[k]) {
        send(e, to, msgs[slot], 0, NULL);
    }
    if !init {
        inv_switchSolver_01AFUNC_MessageFunction(e, 0x150, 0, NULL)?;
    }
    let solved = k == 0b0101;
    if solved {
        lw(e, 100, 1);
    }
    for (i, v) in s.into_iter().enumerate() {
        send(e, 0x2a000160 + i as u32, 0x1ed, v, NULL);
    }
    Ok(solved as u32)
}

pub fn inv_switchSolver_01AFUNC_Initialisation(e: &mut Engine) -> LResult<()> {
    lw(e, 0x50, 1);
    for (i, bit) in [0x100000, 0x200000, 0x400000, 0x800000].into_iter().enumerate() {
        let v = (gb2_8(e) & bit != 0) as u32;
        lw(e, 0x54 + i as u32 * 4, v);
    }
    inv_switchSolver_01AFUNC_CheckSolution(e)?;
    lw(e, 0x50, 0);
    Ok(())
}

pub fn inv_switchSolver_SavePuzzleStatus(e: &mut Engine) {
    let mut v = 1;
    for (i, bit) in [0x100000, 0x200000, 0x400000, 0x800000].into_iter().enumerate() {
        if gb2_8(e) & bit != 0 {
            v |= 2 << i;
        }
    }
    if lr(e, 100) != 0 {
        v |= 0x20;
    }
    lw(e, 0x68, v);
}

fn switch_changed(e: &mut Engine, d: Ptr, on: u32) -> LResult<()> {
    let id = e.r32(d);
    if (0x2a000160..=0x2a000163).contains(&id) {
        lw(e, 0x54 + (id - 0x2a000160) * 4, on);
        if id <= 0x2a000161 {
            let a = gb(e, 0) + 0x80 + (id - 0x2a000160) * 4;
            e.w32(a, 1);
        }
    }
    if lr(e, 0x4c) != 0 {
        lw(e, 0x4c, 0);
        APInewSound::Play(e, 0x2a00004c, 0x8000_0007, 0);
    }
    if inv_switchSolver_01AFUNC_CheckSolution(e)? != 0 {
        send(e, 0x2a100013, 0x2b, 2, NULL);
        APInewSound::Play(e, SELF, 0x4000_0006, 0);
    }
    Ok(())
}

pub fn inv_switchSolver_01AFUNC_MessageFunction(e: &mut Engine, m: u32, _: u32, d: Ptr) -> LResult<u64> {
    match m {
        0xe1 => switch_changed(e, d, 1)?,
        0xe2 => switch_changed(e, d, 0)?,
        0x16 => inv_switchSolver_SavePuzzleStatus(e),
        0xc9 => {
            let st = lr(e, 0x68);
            let mut v = gb2_8(e) & 0xff0f_ffff;
            for i in 0..4 {
                if st & (2 << i) != 0 {
                    v |= 0x100000 << i;
                }
            }
            set_gb2_8(e, v);
            if st & 0x20 == 0 && lr(e, 100) != 0 {
                lw(e, 100, 0);
                send(e, 0x2a100013, 0x2d, 2, NULL);
            }
            inv_switchSolver_01AFUNC_Initialisation(e)?;
        }
        0x150 => {
            APIgameobj::TakeOrientation(e, 0x2a000027, 0x2a000033, 0x7fff_ffff);
            for (o, v) in [(0x24, 0x2a000167), (0x1c, 0x2a000033), (0, 4), (0x44, 0xc0200a), (0x2c, 0x40a00000), (0x3c, 1), (0x40, 0)] {
                lw(e, o, v);
            }
            let l = L(e, 0);
            send(e, 0x2a000027, 0x54, 0, l);
        }
        _ => {}
    }
    Ok(0)
}

pub fn inv_switchsolver_01aConstructor(e: &mut Engine, api: Ptr, n: i32) -> LResult<()> {
    if n == 1 {
        ext(e, &[(0, inv_switchSolver_01AST_FIRSTMOVE), (1, inv_switchSolver_01AST_BASICMOVE)]);
        let v = APIcons::GetApiGameObj(e); gset(e, pGameObj, v);
        let v = APIcons::GetApiNewSound(e); gset(e, pSound, v);
        let v = APIcons::GetApiExtra(e); gset(e, pExtra, v);
        let v = APIcons::GetApiLight(e); gset(e, pLight, v);
        custom(e, pDebug, "debug");
        custom(e, pMiscFunc, "MiscFunc");
        let s = G(e, pGlobalDataSwitchSolver);
        APIcons::GetGlobalData(e, s, 8);
        let g = pw(e, pGlobalDataSwitchSolver, 0);
        let b = APIgameobj::GetGameBuffer(e, 0, 0x8f8, 1);
        e.w32(g, b);
        let b = APIgameobj::GetGameBuffer(e, 2, 0x158, 1);
        e.w32(g + 4, b);
    }
    APIcons::SetMessageFunction(e, inv_switchSolver_01AFUNC_MessageFunction);
    let s = G(e, pLocalDataSwitchSolver);
    APIcons::GetLocalData(e, s, 0x6c);
    LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 2, NULL, 0);
    if e.r32(gb(e, 0) + 0x88) != 0 {
        lw(e, 100, 1);
        let v = gb2_8(e) & 0xffaf_ffff | 0xa00000;
        set_gb2_8(e, v);
    }
    APIcons::SetInitialState(e, 0);
    Ok(())
}

pub fn inv_switchsolver_01aDestructor(e: &mut Engine, n: i32) -> LResult<()> {
    Ok(())
}

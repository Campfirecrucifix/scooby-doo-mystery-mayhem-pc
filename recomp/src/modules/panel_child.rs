//! `panel_child`: an animated HUD sub-panel; sleeps forever, msg 0xe1 / 0xe2 play its show / hide
//! anims (0x2a,0x1a,0x1e,0x22,0x26 / 0x29,0x19,0x1d,0x21,0x25).
//! Port of `Game Logic Modules/Globals/Panel/panel_child__3402e01d/panel_child.c` (4 functions).
//! One build matched (EP1L01, sha 3402e01d).
#![allow(non_snake_case, non_upper_case_globals, unused_variables)]

use crate::engine::*;
use crate::logic::{LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "panel_child",
    data_size: 0x10,
    ctor: panel_childConstructor,
    dtor: panel_childDestructor,
    register: None,
    init: &[],
    build_init: None,
};

/// (build sha8, [anims played on msg 0xe1, on msg 0xe2]) — the 12 builds differ only in these ids (WP-L8).
const BUILDS: [(&str, [&[u32]; 2]); 12] = [
    ("0baac36f", [&[0x1a, 0x1e, 0x22, 0x26, 0x2a], &[0x19, 0x1d, 0x21, 0x25, 0x29]]),
    ("1fa10c29", [&[0x1d, 0x2d, 0x29, 0x21, 0x25], &[0x1c, 0x2c, 0x28, 0x20, 0x24]]),
    ("3402e01d", [&[0x2a, 0x1a, 0x1e, 0x22, 0x26], &[0x29, 0x19, 0x1d, 0x21, 0x25]]),
    ("441cbf46", [&[0x1e, 0x22, 0x26, 0x2a, 0x2e], &[0x1d, 0x21, 0x25, 0x29, 0x2d]]),
    ("6ded8a3a", [&[0x24, 0x28, 0x2c, 0x30, 0x34], &[0x23, 0x27, 0x2b, 0x2f, 0x33]]),
    ("a5ecccbf", [&[0x25, 0x35, 0x31, 0x2d, 0x29], &[0x24, 0x34, 0x30, 0x2c, 0x28]]),
    ("b736e4f8", [&[0x17, 0x19, 0x1f, 0x23, 0x25], &[0x16, 0x1a, 0x1e, 0x22, 0x26]]),
    ("be253c17", [&[0x1d, 0x2d, 0x29, 0x25, 0x21], &[0x1c, 0x2c, 0x28, 0x24, 0x20]]),
    ("c16732fc", [&[0x2f, 0x1f, 0x23, 0x27, 0x2b], &[0x2e, 0x1e, 0x22, 0x26, 0x2a]]),
    ("c305db92", [&[0x1d, 0x21, 0x25, 0x29, 0x2d], &[0x1c, 0x20, 0x24, 0x28, 0x2c]]),
    ("c42582d4", [&[0x17, 0x1b, 0x1f, 0x23, 0x27], &[0x16, 0x1a, 0x1e, 0x22, 0x26]]),
    ("d4e1cff3", [&[0x20, 0x24, 0x28, 0x2c, 0x30], &[0x1f, 0x23, 0x27, 0x2b, 0x2f]]),
];

pub fn build_index(key: &str) -> Option<usize> {
    BUILDS.iter().position(|b| b.0 == key)
}

pub const pMiscFunc: u32 = 0x10000;
pub const pGameObj: u32 = 0x10004;
pub const pAnim: u32 = 0x10008;

#[inline]
fn G(e: &Engine, a: u32) -> Ptr {
    e.rt.cur_data() + (a - 0x10000)
}

/// 0x00010cd0 state 0
pub fn null_main(e: &mut Engine, _: u32) -> LResult<u64> {
    APIgameobj::Sleep(e, SELF, -1.0);
    Ok(0)
}

/// 0x00010d28
pub fn panelchildFUNC_Message(e: &mut Engine, m: u32, _s: u32, _d: Ptr) -> LResult<u64> {
    let anims: &[u32] = match m {
        0xe1 => BUILDS[e.rt.cur_build()].1[0],
        0xe2 => BUILDS[e.rt.cur_build()].1[1],
        _ => &[],
    };
    for &a in anims {
        APIanim::Play(e, SELF, a, 1, 0.0);
    }
    Ok(0)
}

/// 0x00010f08 (quirk kept: only the first instance gets the message function)
pub fn panel_childConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    if instanceCount == 1 {
        APIcons::SetExternalFunction(e, 0, null_main);
        APIcons::SetMessageFunction(e, panelchildFUNC_Message);
        let v = APIcons::GetApiGameObj(e);
        let a = G(e, pGameObj);
        e.w32(a, v);
        let v = APIcons::GetApiAnim(e);
        let a = G(e, pAnim);
        e.w32(a, v);
        let n = e.cstr("MiscFunc");
        let v = APIcons::GetApiCustom(e, n);
        let a = G(e, pMiscFunc);
        e.w32(a, v);
    }
    APIcons::SetInitialState(e, 0);
    Ok(())
}

/// 0x00010fe8
pub fn panel_childDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    Ok(())
}

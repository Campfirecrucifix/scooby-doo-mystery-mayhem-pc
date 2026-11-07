//! `enmyspot`: one radar blip of a monster (HUD window given by msg 0x1a). The radar manager assigns
//! the tracked monster with msg 0xe9 (0x7ffffffe = none -> window parked at 100,100) and clears it
//! with 0xea. Each frame the blip = monster position relative to the leader hero, in camera space,
//! scaled 0.4/20 m; beyond sqrt(195) m it tells the HUD (0x1c to 0x2a100006) and hides at -1,-1.
//! Port of `Game Logic Modules/Globals/Panel/Radar/enmyspot__67dc9dcc/enmyspot.c` (6 functions).
//! One build matched (EP1L01, sha 67dc9dcc).
#![allow(non_snake_case, non_upper_case_globals, unused_variables)]

use crate::engine::*;
use crate::logic::{LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "enmyspot",
    data_size: 0x28,
    ctor: enmyspotConstructor,
    dtor: enmyspotDestructor,
    register: None,
    init: &[],
    build_init: None,
};

/// Per-build constants (WP-L8; all 32 builds share EnmySpot.c): camera object, HUD object of msg 0x1c,
/// the two "blue" ids tested in the 0xe9 handler (NONE = 0x7ffffffe), and whether BASICMOVE
/// uses the transformed y (1) instead of z (0) as the radar's vertical axis.
#[derive(Debug)]
pub struct B {
    sha8: &'static str,
    cam: u32,
    hud: u32,
    blue: [u32; 2],
    use_y: bool,
}
const fn b(sha8: &'static str, cam: u32, hud: u32, blue: [u32; 2], y: u32) -> B {
    B { sha8, cam, hud, blue, use_y: y != 0 }
}
pub static BUILDS: Table<B> = Table::new(&BUILDS_K, builds_row);
const BUILDS_G: [u8; 32] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 2, 3, 2, 3, 0, 3, 0, 0, 3, 0, 0, 3, 0, 0, 0];
fn builds_row(k: &TRow) -> B {
    match BUILDS_G[k.1] {
        0 => b(k.0.builds[k.1], k.v(0), k.v(1), [NONE,NONE], 0),
        1 => b(k.0.builds[k.1], k.v(2), k.v(3), [k.v(4),NONE], 1),
        2 => b(k.0.builds[k.1], k.v(2), k.v(5), [k.v(4),k.v(6)], 0),
        3 => b(k.0.builds[k.1], k.v(0), k.v(1), [k.v(4),NONE], 0),
        _ => unreachable!(),
    }
}
static BUILDS_K: TSpec = TSpec {
    builds: &["07b12697", "0d6649c3", "103be7a3", "1b5a60be", "1bbef259", "243d4c19", "2d6e54ff", "37de4250", "4ab217e2", "4f41fcaa", "53e8a602", "5c9d5a63", "67dc9dcc", "6b42f421", "75fbfb01", "7b0c03cd", "7d4ca0f7", "87465f5e", "8aa87f92", "90504888", "948fb5a9", "95847430", "a009f532", "b506e9eb", "c22276d8", "ca2097b8", "cd07996f", "cf6473cb", "d360a6c6", "efac9fa4", "f91df4c1", "fffa8012"],
    fns: &["enmyspotST_BASICMOVE", "enmyspotFUNC_MessageHandler"],
    entries: &[
        TE::M(&[KLoc(0, 0xf5deea48, 255, 3, 101, 1), KLoc(0, 0xbcb7e7d0, 255, 3, 101, 1)]), TE::M(&[KLoc(0, 0xf5deea48, 255, 3, 164, 1), KLoc(0, 0xbcb7e7d0, 255, 2, 164, 0)]), TE::L(0, 3, 101, 1), TE::L(0, 2, 164, 0), TE::L(1, 3, 45, 1), TE::L(0, 3, 164, 1), TE::L(1, 3, 45, 8),
    ],
};

pub fn build_index(key: &str) -> Option<usize> {
    BUILDS.iter().position(|b| b.sha8 == key)
}

pub const pGameObj: u32 = 0x10000;
pub const pPhysic: u32 = 0x10004;
pub const pExtra: u32 = 0x10008;
pub const pStdlib: u32 = 0x1000c;
pub const pHud: u32 = 0x10010;
pub const pMath: u32 = 0x10014;
pub const pMisc: u32 = 0x10018;
pub const pLocalDataEnmySpot: u32 = 0x1001c;
pub const pGlobalDataEnmySpot: u32 = 0x10020;
pub const pGraphic: u32 = 0x10024;

/// "no window yet" marker in L+0x10
const NO_WINDOW: u32 = 0x5f5e0f8;
const NONE: u32 = 0x7fff_fffe;

#[inline]
fn G(e: &Engine, a: u32) -> Ptr {
    e.rt.cur_data() + (a - 0x10000)
}
/// local data (0x14): +0 tracked monster id, +0x10 HUD window
#[inline]
fn L(e: &Engine) -> Ptr {
    e.r32(G(e, pLocalDataEnmySpot))
}
fn lr(e: &Engine, o: u32) -> u32 {
    e.r32(L(e) + o)
}
fn lw(e: &mut Engine, o: u32, v: u32) {
    let a = L(e) + o;
    e.w32(a, v);
}

/// 0x00010ce8 state 0
pub fn enmyspotST_FIRSTMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    lw(e, 0, NONE);
    APIgraphics::SetGameObjBlendColor(e, SELF, 0, 1.0, 0.0, 0.0, 1.0);
    APIgameobj::SetNextState(e, SELF, 1);
    Ok(0)
}

/// 0x00010d88 state 1
pub fn enmyspotST_BASICMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    let p60 = e.alloca(0xc);
    let p50 = e.alloca(0xc);
    let p40 = e.alloca(0xc);
    if lr(e, 0x10) == NO_WINDOW {
        APIgameobj::Sleep(e, SELF, -1.0);
    } else if lr(e, 0) == NONE {
        let w = lr(e, 0x10);
        APIhud::WindowSetPos(e, w, -100.0, -100.0);
        APIhud::WindowSetFlags(e, w, 0x100);
        APIgameobj::Sleep(e, SELF, -1.0);
    } else {
        let m = lr(e, 0);
        APIgameobj::GetPosition(e, m, 0x7fff_ffff, p60);
        let gd = e.r32(G(e, pGlobalDataEnmySpot));
        let leader = e.r32(e.r32(gd) + 8);
        APIgameobj::GetPosition(e, leader, 0x7fff_ffff, p50);
        let x = e.rf(p60) - e.rf(p50);
        let z = e.rf(p60 + 8) - e.rf(p50 + 8);
        e.wf(p40, x);
        e.wf(p40 + 4, 0.0);
        e.wf(p40 + 8, z);
        let cam = BUILDS[e.rt.cur_build()].cam;
        APIgameobj::InvertTransformVector(e, cam, p40, p40);
        let (x, z) = (e.rf(p40), e.rf(p40 + if BUILDS[e.rt.cur_build()].use_y { 4 } else { 8 }));
        let px = 0.4 - (x * 0.4) / 20.0;
        let pz = 0.4 - (z * 0.4) / 20.0;
        let w = lr(e, 0x10);
        if x * x + z * z < 195.0 {
            APIhud::WindowSetPos(e, w, px, pz);
        } else {
            let hud = BUILDS[e.rt.cur_build()].hud;
            APIgameobj::SendMessageTo(e, hud, NULL, 0x1c, 0, NULL);
            APIhud::WindowSetPos(e, w, -1.0, -1.0);
        }
    }
    Ok(0)
}

/// 0x00011090 (never registered)
pub fn enmyspotCB_TIMER(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

/// 0x000110c0
pub fn enmyspotFUNC_MessageHandler(e: &mut Engine, m: u32, s: u32, _d: Ptr) -> LResult<u64> {
    if m == 0xe9 {
        if lr(e, 0x10) != NO_WINDOW {
            if s == NONE {
                let w = lr(e, 0x10);
                APIhud::WindowSetPos(e, w, 100.0, 100.0);
            } else {
                lw(e, 0, s);
                let blue = BUILDS[e.rt.cur_build()].blue;
                if lr(e, 0) == blue[0] || lr(e, 0) == blue[1] {
                    APIgraphics::SetGameObjBlendColor(e, SELF, 0, 0.0, 0.0, 1.0, 1.0);
                } else {
                    APIgraphics::SetGameObjBlendColor(e, SELF, 0, 1.0, 0.0, 0.0, 1.0);
                }
                APIgameobj::Sleep(e, SELF, 0.0);
                let w = lr(e, 0x10);
                APIhud::WindowResetFlags(e, w, 0x100);
                APIgameobj::SetNextState(e, SELF, 1);
            }
        }
    } else if m < 0xea {
        if m == 0x1a {
            lw(e, 0x10, s);
            APIgameobj::Sleep(e, SELF, 0.0);
        }
    } else if m == 0xea {
        lw(e, 0, NONE);
        APIgameobj::SetNextState(e, SELF, 1);
    }
    Ok(0)
}

/// 0x00011350
pub fn enmyspotConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    if instanceCount == 1 {
        APIcons::SetExternalFunction(e, 0, enmyspotST_FIRSTMOVE);
        APIcons::SetExternalFunction(e, 1, enmyspotST_BASICMOVE);
        let tables: [(u32, fn(&mut Engine) -> Ptr); 6] = [
            (pGameObj, |e| APIcons::GetApiGameObj(e)),
            (pPhysic, |e| APIcons::GetApiPhysic(e)),
            (pExtra, |e| APIcons::GetApiExtra(e)),
            (pStdlib, |e| APIcons::GetApiStdlib(e)),
            (pHud, |e| APIcons::GetApiHUD(e)),
            (pMath, |e| APIcons::GetApiMath(e)),
        ];
        for (slot, get) in tables {
            let v = get(e);
            let a = G(e, slot);
            e.w32(a, v);
        }
        let n = e.cstr("MiscFunc");
        let v = APIcons::GetApiCustom(e, n);
        let a = G(e, pMisc);
        e.w32(a, v);
        let v = APIcons::GetApiGraphics(e);
        let a = G(e, pGraphic);
        e.w32(a, v);
        let slot = G(e, pGlobalDataEnmySpot);
        APIcons::GetGlobalData(e, slot, 4);
        let b = APIgameobj::GetGameBuffer(e, 0, 0x8f8, 1);
        let gd = e.r32(G(e, pGlobalDataEnmySpot));
        e.w32(gd, b);
    }
    let slot = G(e, pLocalDataEnmySpot);
    APIcons::GetLocalData(e, slot, 0x14);
    APIcons::SetMessageFunction(e, enmyspotFUNC_MessageHandler);
    LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 10, NULL, 0);
    lw(e, 0x10, NO_WINDOW);
    APIcons::SetInitialState(e, 0);
    Ok(())
}

/// 0x00011560
pub fn enmyspotDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    if instanceCount == 0 {
        APIgameobj::FreeGameBuffer(e, 0);
    }
    Ok(())
}

#[allow(unused_imports)]
use crate::engine::gamedata::{KLoc, TRow, TSpec, Table, TE};

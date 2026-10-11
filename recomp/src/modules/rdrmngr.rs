//! `rdrmngr`: the radar manager. Owns 9 radar point records {+0 spot object, +4 tracked id, +8} at
//! L+0x40 (6 enemy spots 0x2a10000a/0x2a000111..115, 3 unused, boss spot 0x2a000116 at L+0xac) and
//! the hero spot (L+0 = 0x2a10000f, L+4 = leader). Every update it collects the monsters in range of
//! the leader (retriever 0xff000100, group mask 0x400, 100 m), shows one spot per monster (msg 0xe9 to
//! the spot), and when none is found tells the HUD (0x1f to 0x2a100006). The nearest monster
//! (0xff000002, 100 m) gets 0xd5.
//! Port of `Game Logic Modules/Globals/Panel/Radar/rdrmngr__259af778/rdrmngr.c` (11 functions).
//! One build matched (EP1L01, sha 259af778).
#![allow(non_snake_case, non_upper_case_globals, unused_variables)]

use crate::engine::*;
use crate::logic::{LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "rdrmngr",
    data_size: 0x20,
    ctor: rdrmngrConstructor,
    dtor: rdrmngrDestructor,
    register: None,
    init: &[],
    build_init: None,
};

pub fn build_index(key: &str) -> Option<usize> {
    BUILDS.iter().position(|b| b.sha8 == key)
}

pub const pGameObj: u32 = 0x10000;
pub const pExtra: u32 = 0x10004;
pub const pStdlib: u32 = 0x10008;
pub const pFunctions: u32 = 0x1000c;
pub const pDebug: u32 = 0x10010;
pub const pMisc: u32 = 0x10014;
pub const pLocalDataRdrMngr: u32 = 0x10018;
pub const pGlobalDataRdrMngr: u32 = 0x1001c;

const NONE: u32 = 0x7fff_fffe;
/// per build: enemy spots, boss spot, hero spot, HUD, Shaggy
/// (WP-L8: all 34 builds share RdrMngr.c; 3 builds never install the message handler, msg_fn = false)
struct B { sha8: &'static str, spots: [u32; 6], boss: u32, hero: u32, hud: u32, shaggy: u32, msg_fn: bool }
const fn bd(sha8: &'static str, spots: [u32; 6], boss: u32, hero: u32, hud: u32, shaggy: u32, m: u32) -> B {
    B { sha8, spots, boss, hero, hud, shaggy, msg_fn: m != 0 }
}
const BUILDS: [B; 34] = [
    bd("172f2782", [0x2a10_0001, 0x2a10_000f, 0x2a10_0010, 0x2a10_0011, 0x2a10_0012, 0x2a10_0013], 0x2a10_0014, 0x2a10_0002, 0x2a10_0000, 0x2a00_00ef, 1),
    bd("258f0a74", [0x2a10_000a, 0x2a00_00cd, 0x2a00_00ce, 0x2a00_00cf, 0x2a00_00d0, 0x2a00_00d1], 0x2a00_00d2, 0x2a10_000f, 0x2a10_0006, 0x2a00_004c, 1),
    bd("259af778", [0x2a10_000a, 0x2a00_0111, 0x2a00_0112, 0x2a00_0113, 0x2a00_0114, 0x2a00_0115], 0x2a00_0116, 0x2a10_000f, 0x2a10_0006, 0x2a00_0092, 1),
    bd("2e960d87", [0x2a10_000a, 0x2a00_00bc, 0x2a00_00bd, 0x2a00_00be, 0x2a00_00bf, 0x2a00_00c0], 0x2a00_00c1, 0x2a10_000f, 0x2a10_0006, 0x2a00_0033, 1),
    bd("408ddff1", [0x2a10_000a, 0x2a00_00bb, 0x2a00_00bc, 0x2a00_00bd, 0x2a00_00be, 0x2a00_00bf], 0x2a00_00c0, 0x2a10_000f, 0x2a10_0006, 0x2a00_0036, 1),
    bd("438a5c4d", [0x2a10_000a, 0x2a00_00ad, 0x2a00_00ae, 0x2a00_00af, 0x2a00_00b0, 0x2a00_00b1], 0x2a00_00b2, 0x2a10_000f, 0x2a10_0006, 0x2a00_002d, 1),
    bd("4735e2a5", [0x2a10_000a, 0x2a00_00a0, 0x2a00_00a1, 0x2a00_00a2, 0x2a00_00a3, 0x2a00_00a4], 0x2a00_00a5, 0x2a10_000f, 0x2a10_0006, 0x2a00_001b, 1),
    bd("50353d58", [0x2a10_000a, 0x2a00_00c1, 0x2a00_00c2, 0x2a00_00c3, 0x2a00_00c4, 0x2a00_00c5], 0x2a00_00c6, 0x2a10_000f, 0x2a10_0006, 0x2a00_003d, 1),
    bd("568865d2", [0x2a10_0001, 0x2a10_000f, 0x2a10_0010, 0x2a10_0011, 0x2a10_0015, 0x2a10_0016], 0x2a10_0017, 0x2a10_0002, 0x2a10_0000, 0x2a00_0067, 1),
    bd("56b9d5d3", [0x2a10_0002, 0x2a10_000e, 0x2a10_000f, 0x2a10_0010, 0x2a10_0011, 0x2a10_0012], 0x2a10_0013, 0x2a10_0001, 0x2a10_0000, 0x2a00_0009, 1),
    bd("59eb32fb", [0x2a10_0021, 0x2a00_00bd, 0x2a00_00be, 0x2a00_00bf, 0x2a00_00c0, 0x2a00_00c1], 0x2a00_00c2, 0x2a10_0020, 0x2a10_001f, 0x2a00_0008, 0),
    bd("5f67890d", [0x2a10_000a, 0x2a00_0167, 0x2a00_0168, 0x2a00_0169, 0x2a00_016a, 0x2a00_016b], 0x2a00_016c, 0x2a10_000f, 0x2a10_0006, 0x2a00_00e4, 1),
    bd("6839aa4e", [0x2a10_000a, 0x2a10_0019, 0x2a10_001a, 0x2a10_001b, 0x2a10_001c, 0x2a10_001d], 0x2a10_001e, 0x2a10_000f, 0x2a10_0006, 0x2a00_00ab, 1),
    bd("6cff452e", [0x2a10_000a, 0x2a00_0092, 0x2a00_0093, 0x2a00_0094, 0x2a00_0095, 0x2a00_0096], 0x2a00_0097, 0x2a10_000f, 0x2a10_0006, 0x2a00_0013, 1),
    bd("70675c62", [0x2a10_000a, 0x2a00_0094, 0x2a00_0095, 0x2a00_0096, 0x2a00_0097, 0x2a00_0098], 0x2a00_0099, 0x2a10_000f, 0x2a10_0006, 0x2a00_0008, 1),
    bd("711abad9", [0x2a10_000a, 0x2a00_00e3, 0x2a00_00e4, 0x2a00_00e5, 0x2a00_00e6, 0x2a00_00e7], 0x2a00_00e8, 0x2a10_000f, 0x2a10_0006, 0x2a00_0056, 1),
    bd("7165aa4d", [0x2a10_000a, 0x2a00_00ae, 0x2a00_00af, 0x2a00_00b0, 0x2a00_00b1, 0x2a00_00b2], 0x2a00_00b3, 0x2a10_000f, 0x2a10_0006, 0x2a00_0026, 1),
    bd("75f5499d", [0x2a10_000a, 0x2a00_00fc, 0x2a00_00fd, 0x2a00_00fe, 0x2a00_00ff, 0x2a00_0100], 0x2a00_0101, 0x2a10_000f, 0x2a10_0006, 0x2a00_007d, 1),
    bd("772eef9d", [0x2a10_000a, 0x2a00_008a, 0x2a00_008b, 0x2a00_008c, 0x2a00_008d, 0x2a00_008e], 0x2a00_008f, 0x2a10_000f, 0x2a10_0006, 0x2a00_0009, 1),
    bd("7c58fdfc", [0x2a10_0001, 0x2a10_000e, 0x2a10_000f, 0x2a10_0010, 0x2a10_0011, 0x2a10_0012], 0x2a10_0013, 0x2a10_0002, 0x2a10_0000, 0x2a00_002f, 1),
    bd("84c1a6c2", [0x2a10_000a, 0x2a00_00c8, 0x2a00_00c9, 0x2a00_00ca, 0x2a00_00cb, 0x2a00_00cc], 0x2a00_00cd, 0x2a10_000f, 0x2a10_0006, 0x2a00_0042, 1),
    bd("8915034a", [0x2a10_000a, 0x2a00_00c6, 0x2a00_00c7, 0x2a00_00c8, 0x2a00_00c9, 0x2a00_00ca], 0x2a00_00cb, 0x2a10_000f, 0x2a10_0006, 0x2a00_0044, 1),
    bd("9b542156", [0x2a10_000a, 0x2a00_00fe, 0x2a00_00ff, 0x2a00_0100, 0x2a00_0101, 0x2a00_0102], 0x2a00_0103, 0x2a10_000f, 0x2a10_0006, 0x2a00_007b, 1),
    bd("a14c4b63", [0x2a10_0021, 0x2a00_00c5, 0x2a00_00c6, 0x2a00_00c7, 0x2a00_00c8, 0x2a00_00c9], 0x2a00_00ca, 0x2a10_0020, 0x2a10_001f, 0x2a00_0008, 0),
    bd("b179e57d", [0x2a10_0021, 0x2a00_0020, 0x2a00_0021, 0x2a00_0022, 0x2a00_0023, 0x2a00_0024], 0x2a00_0025, 0x2a10_0020, 0x2a10_001f, 0x2a00_0007, 0),
    bd("b3056a2d", [0x2a10_000a, 0x2a10_001d, 0x2a10_001e, 0x2a10_001f, 0x2a10_0020, 0x2a10_0021], 0x2a00_00a4, 0x2a10_000f, 0x2a10_0006, 0x2a00_0024, 1),
    bd("b4a4aa20", [0x2a10_000a, 0x2a10_001d, 0x2a10_001e, 0x2a10_001f, 0x2a00_0088, 0x2a10_0020], 0x2a10_0021, 0x2a10_000f, 0x2a10_0006, 0x2a00_000a, 1),
    bd("bc4279fe", [0x2a10_0001, 0x2a00_00b0, 0x2a00_00b1, 0x2a00_00b2, 0x2a00_00b3, 0x2a00_00b4], 0x2a00_00b5, 0x2a10_0002, 0x2a10_0000, 0x2a00_002e, 1),
    bd("c1e016dd", [0x2a10_000a, 0x2a00_0090, 0x2a00_0091, 0x2a00_0092, 0x2a00_0093, 0x2a00_0094], 0x2a00_0095, 0x2a10_000f, 0x2a10_0006, 0x2a00_000e, 1),
    bd("e0642538", [0x2a10_000a, 0x2a00_00b3, 0x2a00_00b4, 0x2a00_00b5, 0x2a00_00b6, 0x2a00_00b7], 0x2a00_00b8, 0x2a10_000f, 0x2a10_0006, 0x2a00_002c, 1),
    bd("e3b9673e", [0x2a10_000a, 0x2a10_0018, 0x2a10_0019, 0x2a10_001a, 0x2a10_001b, 0x2a10_001c], 0x2a10_001d, 0x2a10_000f, 0x2a10_0006, 0x2a00_0083, 1),
    bd("e5656207", [0x2a10_000a, 0x2a00_00d3, 0x2a00_00d4, 0x2a00_00d5, 0x2a00_00d6, 0x2a00_00d7], 0x2a00_00d8, 0x2a10_000f, 0x2a10_0013, 0x2a00_004d, 1),
    bd("f44552c6", [0x2a10_000a, 0x2a00_00c9, 0x2a00_00ca, 0x2a00_00cb, 0x2a00_00cc, 0x2a00_00cd], 0x2a00_00ce, 0x2a10_000f, 0x2a10_0006, 0x2a00_0040, 1),
    bd("f716798c", [0x2a10_000a, 0x2a10_001e, 0x2a10_001f, 0x2a10_0020, 0x2a10_0021, 0x2a10_0022], 0x2a10_0023, 0x2a10_000f, 0x2a10_0006, 0x2a00_0011, 1),
];
fn b(e: &Engine) -> &'static B {
    &BUILDS[e.rt.cur_build()]
}

#[inline]
fn G(e: &Engine, a: u32) -> Ptr {
    e.rt.cur_data() + (a - 0x10000)
}
/// local data (0x148): +0 hero spot, +4 hero id, +0x40.. 10 records of 0xc {spot, id, -} (last = boss),
/// +0x130 count, +0x140 nearest monster, +0x144 last found monster
#[inline]
fn L(e: &Engine) -> Ptr {
    e.r32(G(e, pLocalDataRdrMngr))
}
fn lr(e: &Engine, o: u32) -> u32 {
    e.r32(L(e) + o)
}
fn lw(e: &mut Engine, o: u32, v: u32) {
    let a = L(e) + o;
    e.w32(a, v);
}
/// pGlobalDataRdrMngr: +0 game buffer 0, +4 game buffer 1
fn gd(e: &Engine, o: u32) -> u32 {
    e.r32(e.r32(G(e, pGlobalDataRdrMngr)) + o)
}
fn send(e: &mut Engine, to: u32, msg: u32, sub: u32) {
    APIgameobj::SendMessageTo(e, to, NULL, msg, sub, NULL);
}

/// 0x00010d58 state 0
pub fn rdrmngrST_FIRSTMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

/// 0x00010d88 state 2
pub fn rdrmngrST_BASICMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 4, NULL, 0);
    Ok(0)
}

/// 0x00010de8 (never registered)
pub fn rdrmngrCB_TIMER(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

/// 0x00010e18
pub fn rdrmngrFUNC_InitRadarPoints(e: &mut Engine) {
    let S = b(e).spots;
    let spots = [S[0], S[1], S[2], S[3], S[4], S[5], 0, 0, 0, b(e).boss];
    // records at word index 0x10 + 3*i (the boss record [0x2b] is i = 9)
    for (i, s) in spots.into_iter().enumerate() {
        let o = (0x10 + 3 * i as u32) * 4;
        lw(e, o, s);
        lw(e, o + 4, NONE);
    }
    let h = b(e).hero;
    lw(e, 0, h);
    let leader = gd(e, 0);
    let leader = e.r32(leader + 8);
    lw(e, 4, leader);
    for w in [3u32, 6, 9, 0xc] {
        lw(e, w * 4, 0);
        lw(e, (w + 1) * 4, NONE);
    }
}

/// 0x00011060
pub fn rdrmngrFUNC_ConnectEnemiesToPoints(e: &mut Engine) {
    let p = e.alloca(8);
    e.w32(p, 1);
    e.w32(p + 4, 0x400);
    lw(e, 0x130, 0);
    let id = APIgameobj::GetID(e, 0xff00_0010, p);
    let n = lr(e, 0x130);
    lw(e, 0x44 + n * 0xc, id);
    while lr(e, 0x44 + lr(e, 0x130) * 0xc) != NONE && lr(e, 0x130) < 7 {
        let n = lr(e, 0x130) + 1;
        lw(e, 0x130, n);
        let id = APIgameobj::GetID(e, 0xff00_0010, p);
        lw(e, 0x44 + n * 0xc, id);
    }
    send_points(e);
}

fn send_points(e: &mut Engine) {
    let mut i = 0u32;
    while lr(e, 0x40 + i * 0xc) != 0 && i < 8 {
        let (s, id) = (lr(e, 0x40 + i * 0xc), lr(e, 0x44 + i * 0xc));
        send(e, s, 0xe9, id);
        i += 1;
    }
}

/// 0x00011298
pub fn rdrmngrFUNC_ConnectBossToPoints(e: &mut Engine) {
    let p = e.alloca(8);
    e.w32(p, 1);
    e.w32(p + 4, 0x10_0000);
    let id = APIgameobj::GetID(e, 0xff00_0010, p);
    lw(e, 0xb0, id);
    let s = lr(e, 0xac);
    if id == NONE {
        send(e, s, 0xea, 0);
    } else {
        send(e, s, 0xe9, id);
    }
}

/// 0x00011388
pub fn rdrmngrFUNC_ConnectHeroToPoints(e: &mut Engine) {
    let (s, h) = (lr(e, 0), lr(e, 4));
    send(e, s, 0xe9, h);
}

/// 0x000113e8
pub fn rdrmngrFUNC_UpdateEnemyPoints(e: &mut Engine) {
    let p70 = e.alloca(0x20);
    let p50 = e.alloca(0x20);
    let mut found = 0;
    let leader = e.r32(gd(e, 0) + 8);
    for (i, v) in [1, 0x13, leader, 0, 0, 0, 0x42c8_0000, 0x3f80_0000].into_iter().enumerate() {
        e.w32(p70 + 4 * i as u32, v);
    }
    let id = APIgameobj::GetID(e, 0xff00_0002, p70);
    lw(e, 0x140, id);
    if id == NONE {
        let leader = e.r32(gd(e, 0) + 8);
        lw(e, 0x140, leader);
    } else {
        send(e, id, 0xd5, 0);
    }
    let near = lr(e, 0x140);
    for (i, v) in [1, 0x400, near, 0, 0, 0, 0x42c8_0000, 0x3c23_d70a].into_iter().enumerate() {
        e.w32(p50 + 4 * i as u32, v);
    }
    let gb1 = gd(e, 4);
    e.w32(gb1 + 0xc, 1);
    for i in 0..9u32 {
        lw(e, 0x44 + i * 0xc, NONE);
        lw(e, 0x40 + i * 0xc, 0);
    }
    lw(e, 0x130, 0);
    for s in b(e).spots {
        LIBMiscFunc::MiscFunc_ChangeClipping(e, s, 4, NULL, 0);
    }
    for s in b(e).spots {
        APIgameobj::SetGameObjectVisible(e, s, 0);
    }
    while e.r32(p50) != NONE {
        let id = APIgameobj::GetID(e, 0xff00_0100, p50);
        lw(e, 0x144, id);
        if id == NONE {
            break;
        }
        if found == 0 {
            let gb1 = gd(e, 4);
            e.w32(gb1 + 0xc, 0);
            found = 1;
        }
        let n = lr(e, 0x130);
        if n < 6 {
            let s = b(e).spots[n as usize];
            lw(e, 0x40 + n * 0xc, s);
            lw(e, 0x44 + n * 0xc, id);
            LIBMiscFunc::MiscFunc_ChangeClipping(e, s, 2, NULL, 0);
            APIgameobj::SetGameObjectVisible(e, s, 1);
        }
        lw(e, 0x130, n + 1);
    }
    send_points(e);
    e.w32(p50, 1);
    e.w32(p50 + 4, 0x10_0000);
    let id = APIgameobj::GetID(e, 0xff00_0010, p50);
    lw(e, 0xb0, id);
    if id != NONE {
        let s = lr(e, 0xac);
        send(e, s, 0xe9, id);
    }
    if e.r32(gd(e, 4) + 0xc) == 1 {
        let hud = b(e).hud;
            send(e, hud, 0x1f, 0);
    }
}

/// 0x00011ea0
pub fn rdrmngrFUNC_MessageHandler(e: &mut Engine, m: u32, s: u32, _d: Ptr) -> LResult<u64> {
    if m == 0xcd {
        let h = b(e).hero;
    lw(e, 0, h);
        let leader = e.r32(gd(e, 0) + 8);
        lw(e, 4, leader);
        rdrmngrFUNC_ConnectHeroToPoints(e);
        return Ok(0);
    }
    if m < 0xce {
        if m == 0x4f {
            let gb0 = gd(e, 0);
            let sub = if e.r32(gb0 + 0x14) & 0x40 != 0 {
                3
            } else if e.r32(gb0 + 8) == b(e).shaggy {
                1
            } else {
                2
            };
            let hud = b(e).hud;
            send(e, hud, 0x1f, sub);
            return Ok(0);
        }
        if m != 0x52 || s != 0 {
            return Ok(0);
        }
        rdrmngrFUNC_ConnectEnemiesToPoints(e);
        rdrmngrFUNC_ConnectBossToPoints(e);
        rdrmngrFUNC_ConnectHeroToPoints(e);
        rdrmngrFUNC_UpdateEnemyPoints(e);
        APIgameobj::SetNextState(e, SELF, 2);
        return Ok(0);
    }
    if m == 0xea {
        for i in 0..8u32 {
            let o = (i * 3 + 0x10) * 4;
            if s == lr(e, o + 4) {
                let sp = lr(e, o);
                send(e, sp, 0xea, 0);
                lw(e, o, 0);
                lw(e, o + 4, NONE);
            }
        }
        rdrmngrFUNC_UpdateEnemyPoints(e);
        return Ok(0);
    }
    if m != 0xeb {
        return Ok(0);
    }
    match s {
        2 => rdrmngrFUNC_ConnectBossToPoints(e),
        1 => rdrmngrFUNC_ConnectEnemiesToPoints(e),
        3 => rdrmngrFUNC_ConnectHeroToPoints(e),
        _ => {
            let t = e.cstr("auiParam invalide... VERIFIE TON MSG_RADER_UPDATE_SPOTS, MON HOMME\n");
            APIextra::null_printf(e, t, 0);
        }
    }
    rdrmngrFUNC_UpdateEnemyPoints(e);
    Ok(0)
}

/// 0x000122b0
pub fn rdrmngrConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    if instanceCount == 1 {
        APIcons::SetExternalFunction(e, 0, rdrmngrST_FIRSTMOVE);
        APIcons::SetExternalFunction(e, 2, rdrmngrST_BASICMOVE);
        let tables: [(u32, fn(&mut Engine) -> Ptr); 3] = [
            (pGameObj, |e| APIcons::GetApiGameObj(e)),
            (pExtra, |e| APIcons::GetApiExtra(e)),
            (pStdlib, |e| APIcons::GetApiStdlib(e)),
        ];
        for (slot, get) in tables {
            let v = get(e);
            let a = G(e, slot);
            e.w32(a, v);
        }
        for (slot, name) in [(pFunctions, "ApiCFunctions"), (pDebug, "debug"), (pMisc, "MiscFunc")] {
            let n = e.cstr(name);
            let v = APIcons::GetApiCustom(e, n);
            let a = G(e, slot);
            e.w32(a, v);
        }
        let slot = G(e, pGlobalDataRdrMngr);
        APIcons::GetGlobalData(e, slot, 8);
        let b = APIgameobj::GetGameBuffer(e, 0, 0x8f8, 1);
        let g = e.r32(G(e, pGlobalDataRdrMngr));
        e.w32(g, b);
        let b = APIgameobj::GetGameBuffer(e, 1, 0x68, 1);
        e.w32(g + 4, b);
    }
    let slot = G(e, pLocalDataRdrMngr);
    APIcons::GetLocalData(e, slot, 0x148);
    if BUILDS[e.rt.cur_build()].msg_fn {
        APIcons::SetMessageFunction(e, rdrmngrFUNC_MessageHandler);
    }
    rdrmngrFUNC_InitRadarPoints(e);
    LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 2, NULL, 0);
    APIcons::SetInitialState(e, 0);
    Ok(())
}

/// 0x000124c8
pub fn rdrmngrDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    if instanceCount == 0 {
        APIgameobj::FreeGameBuffer(e, 0);
    }
    Ok(())
}

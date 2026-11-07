//! `fearwave`: the fear-ball projectile (Velma's / the fear gun's charge). Charge (0xdc) shows it with a
//! small ghost child, fire (0xd8) launches it for 2 s; collision with an enemy (group 1/2) or the hero
//! explodes it: the fearexplosion child bangs and every object in 3 m gets 0xd8 with a scare factor
//! by distance (the hero's own ghosts route a 0x68 to the leader instead). Local: +0/+4 DMs, +0x14
//! owner, +0x18 owner kind (0x167 = 0, 0x169 = 1), +0x1c, +0x20 hit enemy, +0x24 bounce end, +0x28
//! explosion child, +0x2c state mirror, +0x30 end time.
//! The 9 builds differ only in object ids and the charge anim ([`BUILDS`]).
//! Port of `Game Logic Modules/Globals/Objects/FEARWAVE/fearwave__*/FearWave.c` (24 functions).
#![allow(non_snake_case, non_upper_case_globals, unused_variables)]

use crate::engine::*;
use crate::logic::{LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "fearwave",
    data_size: 0x30,
    ctor: fearwaveConstructor,
    dtor: fearwaveDestructor,
    register: None,
    init: &[],
    build_init: None,
};

/// (sha8, explosion latent (+1 = small ghost latent), charge anim, hero id, 3 "own ghost" ids)
pub static BUILDS: Table<(&'static str, u32, u32, u32, [u32; 3])> = Table::new(&BUILDS_K, builds_row);
const BUILDS_G: [u8; 9] = [0, 0, 0, 0, 0, 1, 1, 0, 0];
fn builds_row(k: &TRow) -> (&'static str, u32, u32, u32, [u32; 3]) {
    match BUILDS_G[k.1] {
        0 => (k.0.builds[k.1], k.v(0), 2, k.v(1), [k.v(2), k.v(3), k.v(4)]),
        1 => (k.0.builds[k.1], k.v(0), 3, k.v(1), [k.v(2), k.v(3), k.v(4)]),
        _ => unreachable!(),
    }
}
static BUILDS_K: TSpec = TSpec {
    builds: &["12cac354", "26216801", "2f030c78", "3a0d3e17", "5b4dbe07", "6fed490c", "737c99f4", "7f990cb3", "84e0783f"],
    fns: &["FearWaveFUNC_CreateSmallGhosts", "FearWaveCB_NOTIFY_COLLIDE_OBJ", "FearWaveFUNC_ExplodeScare"],
    entries: &[
        TE::L(0, 13, 16, 1), TE::L(1, 3, 17, 1), TE::L(1, 13, 17, 1), TE::L(2, 3, 60, 6), TE::L(2, 3, 60, 11),
    ],
};

pub fn build_index(key: &str) -> Option<usize> {
    BUILDS.iter().position(|b| b.0 == key)
}

pub const pExtra: u32 = 0x10000;
pub const pGameObj: u32 = 0x10004;
pub const pPhysic: u32 = 0x10008;
pub const pAnim: u32 = 0x1000c;
pub const pStdlib: u32 = 0x10010;
pub const pLight: u32 = 0x10014;
pub const pDebug: u32 = 0x10018;
pub const pAnimMngr: u32 = 0x1001c;
pub const pFunctions: u32 = 0x10020;
pub const pMiscFunc: u32 = 0x10024;
pub const pLocalDataFearWave: u32 = 0x10028;
pub const pGlobalDataFearWave: u32 = 0x1002c;

#[inline]
fn G(e: &Engine, a: u32) -> Ptr {
    e.rt.cur_data() + (a - 0x10000)
}
fn L(e: &Engine, o: u32) -> u32 {
    e.r32(e.r32(G(e, pLocalDataFearWave)) + o)
}
fn W(e: &mut Engine, o: u32, v: u32) {
    let a = e.r32(G(e, pLocalDataFearWave)) + o;
    e.w32(a, v);
}
fn Lf(e: &Engine, o: u32) -> f32 {
    f32::from_bits(L(e, o))
}
fn GB0(e: &Engine) -> Ptr {
    e.r32(e.r32(G(e, pGlobalDataFearWave)))
}
fn B(e: &Engine) -> &'static (&'static str, u32, u32, u32, [u32; 3]) {
    &BUILDS[e.rt.cur_build()]
}
fn go(e: &mut Engine, s: u32) {
    W(e, 0x2c, s);
    APIgameobj::SetNextState(e, SELF, s);
}
fn now(e: &mut Engine) -> f32 {
    APIextra::TimerGetTime(e, 0x9000_0000)
}
fn hide(e: &mut Engine) {
    APIgameobj::SetGameObjectVisible(e, SELF, 0);
    APIphysic::SetFlags(e, SELF, 1);
    APIgameobj::SetContextFlag(e, SELF, 0);
}

pub fn FearWaveST_FIRSTMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    hide(e);
    W(e, 0x20, 0);
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 3, 0xb);
    let x = APIgameobj::LaunchLatant(e, B(e).1);
    W(e, 0x28, x);
    APIgameobj::AddChild(e, SELF, 0x7fff_ffff, x, 1);
    APIgameobj::SetGameObjectVisible(e, x, 0);
    W(e, 0x2c, 1);
    APIgameobj::SetNextState(e, SELF, 1);
    APIgameobj::EnableObject(e, SELF, 0);
    Ok(0)
}
pub fn FearWaveST_BASICMOVE_SET(e: &mut Engine, _: u32) -> LResult<u64> {
    go(e, 2);
    Ok(0)
}
pub fn FearWaveST_NOP(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}
pub fn FearWaveST_CHARGE(e: &mut Engine, _: u32) -> LResult<u64> {
    APIanim::Play(e, SELF, B(e).2, 1, 0.0);
    APIgameobj::SetGameObjectVisible(e, SELF, 1);
    // FearWaveFUNC_CreateSmallGhosts
    let g = APIgameobj::LaunchLatant(e, B(e).1 + 1);
    APIgameobj::AddChild(e, SELF, 4, g, 1);
    let id = APIgameobj::GetID(e, SELF, NULL);
    APIgameobj::SendMessageTo(e, g, NULL, 0x175, id, NULL);
    APIphysic::SetVelocity(e, SELF, 0.0, 0.0, 0.0);
    APIphysic::SetFlags(e, SELF, 1);
    APIgameobj::SetContextFlag(e, SELF, 0);
    go(e, 2);
    Ok(0)
}
pub fn FearWaveST_FIRE(e: &mut Engine, _: u32) -> LResult<u64> {
    APIgameobj::SetContextFlag(e, SELF, 0x18);
    APIphysic::ResetFlags(e, SELF, 1);
    APIgameobj::SetContextFlag(e, SELF, 0x18);
    let t = now(e) + 2.0;
    W(e, 0x30, t.to_bits());
    W(e, 0x24, 0);
    go(e, 5);
    Ok(0)
}
pub fn FearWaveST_MOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    if Lf(e, 0x30) < now(e) {
        let m = if L(e, 0x18) == 0 { 0x178 } else { 0x197 };
        APIgameobj::SendMessageTo(e, L(e, 0x14), NULL, m, 0, NULL);
        go(e, 9);
    } else if Lf(e, 0x24) != 0.0 && Lf(e, 0x24) < now(e) {
        W(e, 0x20, 0);
        go(e, 8);
    }
    Ok(0)
}
pub fn FearWaveST_COLLIDE(e: &mut Engine, _: u32) -> LResult<u64> {
    let t = now(e);
    let r = (APIstdlib::MyRand(e) as i32) % 100;
    W(e, 0x24, (t + (r as f32 * 0.5) / 100.0).to_bits());
    go(e, 5);
    Ok(0)
}
pub fn FearWaveST_EXPLODE(e: &mut Engine, _: u32) -> LResult<u64> {
    APIphysic::SetVelocity(e, SELF, 0.0, 0.0, 0.0);
    APIphysic::SetFlags(e, SELF, 1);
    APIgameobj::SetContextFlag(e, SELF, 0);
    APIgameobj::SetGameObjectVisible(e, SELF, 0);
    let x = L(e, 0x28);
    APIgameobj::SetGameObjectVisible(e, x, 1);
    APIgameobj::SendMessageTo(e, x, NULL, 0x174, 0, NULL);
    ExplodeScare(e);
    small_ghosts(e, 0x176);
    let t = now(e) + 2.0;
    W(e, 0x30, t.to_bits());
    go(e, 10);
    Ok(0)
}
pub fn FearWaveST_FADE(e: &mut Engine, _: u32) -> LResult<u64> {
    let t = now(e) + 2.0;
    W(e, 0x30, t.to_bits());
    small_ghosts(e, 0x176);
    go(e, 10);
    Ok(0)
}
pub fn FearWaveST_DIE(e: &mut Engine, _: u32) -> LResult<u64> {
    if Lf(e, 0x30) < now(e) {
        hide(e);
        APIgameobj::EnableObject(e, SELF, 0);
        let x = L(e, 0x28);
        APIgameobj::SetGameObjectVisible(e, x, 0);
        APIgameobj::SendMessageTo(e, L(e, 0x14), NULL, 0x173, 0, NULL);
        go(e, 2);
    }
    Ok(0)
}
pub fn FearWaveCB_NOTIFY_COLLIDE_OBJ(e: &mut Engine, p: u32) -> LResult<u64> {
    let o = e.r32(p + 8);
    let gb = GB0(e);
    if o == e.r32(gb + 8) || (o == B(e).3 && e.r32(gb + 0x14) & 1 != 0) {
        if L(e, 0x18) != 0 && L(e, 0x2c) == 5 && Lf(e, 0x24) == 0.0 {
            go(e, 6);
            return Ok(1);
        }
        return Ok(0);
    }
    let g = APIgameobj::GetGroupID(e, o);
    if g == 1 || g == 2 {
        go(e, 6);
        W(e, 0x20, 1);
        return Ok(1);
    }
    Ok(0)
}
pub fn FearWaveFUNC_MessageHandler(e: &mut Engine, m: u32, p: u32, _: Ptr) -> LResult<u64> {
    match m {
        0xdc => {
            APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 3, 0xb);
            go(e, 3);
            APIgameobj::EnableObject(e, SELF, 1);
        }
        0xd8 => {
            W(e, 0x1c, 0);
            go(e, 4);
        }
        0xd0 => {
            APIgameobj::SetGameObjectVisible(e, SELF, 0);
            small_ghosts(e, 0x177);
        }
        0xda => {
            APIgameobj::SetNextState(e, SELF, 9);
        }
        0x167 => {
            W(e, 0x14, p);
            W(e, 0x18, 0);
        }
        0xe5 => W(e, 0x1c, 1),
        0x169 => {
            W(e, 0x14, p);
            APIphysic::PrimResetIntAttribBits(e, SELF, 0, 2, 2);
            APIphysic::PrimSetIntAttribBits(e, SELF, 0, 2, 4);
            W(e, 0x18, 1);
        }
        _ => {}
    }
    Ok(0)
}
fn Create_Physic(e: &mut Engine) {
    APIphysic::CreatePhysic(e, SELF, 0);
    APIphysic::SetFlags(e, SELF, 6);
    APIphysic::SetWeight(e, SELF, 0.25);
    APIphysic::SetPriority(e, SELF, 0);
    for (i, b) in [(0, 7), (1, 1), (5, 8), (7, 0), (8, 9), (2, 2)] {
        APIphysic::PrimSetIntAttribBits(e, SELF, 0, i, b);
    }
    APIphysic::PrimResetIntAttribBits(e, SELF, 0, 2, 4);
    let p = e.alloca(0x20);
    let h = APIphysic::DMAdd(e, SELF, 4);
    W(e, 0, h);
    APIphysic::DMGetParams(e, h, p);
    for (o, v) in [(0, 0), (4, 0), (8, 0x4120_0000), (0xc, 1)] {
        e.w32(p + o, v);
    }
    APIphysic::DMSetParams(e, h, p);
    APIphysic::DMSetContextMask(e, h, 8);
    let h = APIphysic::DMAdd(e, SELF, 7);
    W(e, 4, h);
    APIphysic::DMGetParams(e, h, p);
    e.w32(p, 0x4120_0000);
    e.w32(p + 4, 5);
    APIphysic::DMSetParams(e, h, p);
    APIphysic::DMSetContextMask(e, h, 0x10);
}
/// DetachSmallGhosts (0x176) / KillSmallGhosts (0x177): SendMessageTo(0xff000008, {1, 0x20}, m, id)
fn small_ghosts(e: &mut Engine, m: u32) {
    let r = e.alloca(0x10);
    e.w32(r, 1);
    e.w32(r + 4, 0x20);
    let id = APIgameobj::GetID(e, SELF, NULL);
    APIgameobj::SendMessageTo(e, 0xff00_0008, r, m, id, NULL);
}
fn GetScareFactor(d: f32) -> u32 {
    if d < 0.24 {
        0x80000
    } else if d < 0.99 {
        0x200
    } else if d < 1.98 {
        0x10
    } else if d < 3.0 {
        2
    } else {
        1
    }
}
fn ExplodeScare(e: &mut Engine) {
    let r = e.alloca(0x20);
    let me = APIgameobj::GetID(e, SELF, NULL);
    for (o, v) in [(0, 1), (4, 0x801403), (8, me), (0xc, 0), (0x10, 0), (0x14, 0), (0x18, 0x4040_0000), (0x1c, 0x3dcc_cccd)] {
        e.w32(r + o, v);
    }
    let msg = e.alloca(0x18);
    let mut own = false;
    loop {
        let o = APIgameobj::GetID(e, 0xff00_0100, r);
        if o == 0x7fff_fffe {
            break;
        }
        let d = LIBApiCFunctions::GetDistance(e, me as i32, o, 0x7fff_ffff, 1);
        if B(e).4.contains(&o) {
            let id = APIgameobj::GetID(e, SELF, NULL);
            for (k, v) in [(8, id), (0x10, GetScareFactor(d)), (0x14, 0x21)] {
                e.w32(msg + k, v);
            }
            own = true;
            let ld = e.r32(GB0(e) + 8);
            APIgameobj::SendMessageTo(e, ld, NULL, 0x68, 0, msg);
        } else {
            APIgameobj::SendMessageTo(e, o, NULL, 0xd8, GetScareFactor(d), NULL);
        }
    }
    APIgameobj::SendMessageTo(e, L(e, 0x14), NULL, 0x197, own as u32, NULL);
}

pub fn fearwaveConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    if instanceCount == 1 {
        let fns: [fn(&mut Engine, u32) -> LResult<u64>; 12] = [
            FearWaveST_FIRSTMOVE,
            FearWaveST_BASICMOVE_SET,
            FearWaveST_NOP,
            FearWaveST_CHARGE,
            FearWaveST_FIRE,
            FearWaveST_MOVE,
            FearWaveST_COLLIDE,
            FearWaveST_NOP,
            FearWaveST_EXPLODE,
            FearWaveST_FADE,
            FearWaveST_DIE,
            FearWaveCB_NOTIFY_COLLIDE_OBJ,
        ];
        for (i, f) in fns.into_iter().enumerate() {
            APIcons::SetExternalFunction(e, i as i32, f);
        }
        for (a, v) in [
            (pAnim, APIcons::GetApiAnim(e)),
            (pGameObj, APIcons::GetApiGameObj(e)),
            (pExtra, APIcons::GetApiExtra(e)),
            (pPhysic, APIcons::GetApiPhysic(e)),
            (pStdlib, APIcons::GetApiStdlib(e)),
            (pLight, APIcons::GetApiLight(e)),
        ] {
            let a = G(e, a);
            e.w32(a, v);
        }
        for (a, n) in [(pDebug, "debug"), (pAnimMngr, "AnimMngr"), (pFunctions, "ApiCFunctions"), (pMiscFunc, "MiscFunc")] {
            let s = e.cstr(n);
            let v = APIcons::GetApiCustom(e, s);
            let a = G(e, a);
            e.w32(a, v);
        }
        let slot = G(e, pGlobalDataFearWave);
        APIcons::GetGlobalData(e, slot, 4);
        let b = APIgameobj::GetGameBuffer(e, 0, 0x8f8, 1);
        let gd = e.r32(slot);
        e.w32(gd, b);
    }
    APIcons::SetMessageFunction(e, FearWaveFUNC_MessageHandler);
    let slot = G(e, pLocalDataFearWave);
    APIcons::GetLocalData(e, slot, 0x34);
    Create_Physic(e);
    LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 2, NULL, 1);
    APIcons::SetInitialState(e, 0);
    Ok(())
}
pub fn fearwaveDestructor(e: &mut Engine, _: i32) -> LResult<()> {
    W(e, 8, 0);
    Ok(())
}

#[allow(unused_imports)]
use crate::engine::gamedata::{KLoc, TRow, TSpec, Table, TE};

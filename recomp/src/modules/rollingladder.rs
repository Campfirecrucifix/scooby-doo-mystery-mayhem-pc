//! `rollingladder`: the Ep1 library rolling ladder (ladder_base + rolling part). The hero climbs it
//! with X (hero msg 0x96 / MSG_UP 0xe1 / MSG_DOWN 0xe2), and a switch (msg 0xe0) rolls it between
//! x=38 and x=46 (camera msg 0x54 to 0x2a00008f, sounds 0x40000008/9).
//! Port of `Game Logic Modules/Globals/Objects/Ladders/rollingladder__2c0fd45d/{ladder_base.c,rollingladder.c}`
//! (25 functions). One build (EP1L01, sha 2c0fd45d).
//! Both GetLocalData slots (pLocalDataLadderBase 0x94, pLocalDataRollingLadder 0x108) end up on the
//! same per-instance block (InitLocalData grows it), so rolling fields live at +0xa0..+0x108.
#![allow(non_snake_case, non_upper_case_globals, unused_variables)]

use crate::engine::*;
use crate::logic::{LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "rollingladder",
    data_size: 0xf40,
    ctor: ctor_dispatch,
    dtor: dtor_dispatch,
    register: None,
    // goCustomDataLadder: one entry {object id, ?, ?, ?} (image words at 0x10000)
    init: &[],
    build_init: Some(build_init_dispatch),
};

/// initialised data of the hand-ported build(s) (written by build_init_dispatch only for them)
const OLD_INIT: &[(u32, &[u32])] = &[(0x10000, &[0x2a00_004e, 0x8000_0000, 0x3e8, 0x3e9])];

mod xb;

/// number of hand-ported builds (indices 0..N_OLD); generated builds (`xb`, c2rs) follow
pub(crate) const N_OLD: usize = 1;

fn ctor_dispatch(e: &mut Engine, api: Ptr, n: i32) -> LResult<()> {
    if e.rt.cur_build() < N_OLD { rollingladderConstructor(e, api, n) } else { xb::ctor(e, api, n) }
}

fn dtor_dispatch(e: &mut Engine, n: i32) -> LResult<()> {
    if e.rt.cur_build() < N_OLD { rollingladderDestructor(e, n) } else { xb::dtor(e, n) }
}

fn build_init_dispatch(m: &mut Mem, data: Ptr, b: usize) {
    if b < N_OLD {
        for &(a, ws) in OLD_INIT {
            for (i, &w) in ws.iter().enumerate() {
                m.w32(data + (a - 0x10000) + 4 * i as u32, w);
            }
        }
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
    (key == "2c0fd45d").then_some(0)
}

pub const goCustomDataLadder: u32 = 0x10000;
pub const pFunction: u32 = 0x10010;
pub const pText: u32 = 0x10014;
pub const pLocalDataLadderBase: u32 = 0x10018;
pub const pGlobalDataLadderBase: u32 = 0x1001c;
pub const pLocalDataRollingLadder: u32 = 0x10020;
/// camera params sent with msg 0x54 sub 4 by ladder_baseFUNC_SetHero (0x10028..0x10044)
pub const moCameraParams: u32 = 0x10028;
pub const pAnim: u32 = 0x10044;
pub const pExtra: u32 = 0x10048;
pub const pGame: u32 = 0x1004c;
pub const pGameObj: u32 = 0x10050;
pub const pHud: u32 = 0x10054;
pub const pInput: u32 = 0x10058;
pub const pPhysic: u32 = 0x1005c;
pub const pSound: u32 = 0x10060;
pub const pAnimMngr: u32 = 0x10064;
pub const pDebug: u32 = 0x10068;
pub const pMiscFunc: u32 = 0x1006c;

const SHAGGY: u32 = 0x2a00_0091;
const SCOOBY: u32 = 0x2a00_0092;

#[inline]
fn G(e: &Engine, a: u32) -> Ptr {
    e.rt.cur_data() + (a - 0x10000)
}
/// local data: +0 input config, +4 touched prim (0x79/0x7a), +8 anim, +0xc hero on the ladder,
/// +0x10 input active, +0x24 DM, +0x28 touched this frame, +0x30 position, +0x3c..+0x44 custom data,
/// +0x48 hero msg 0x96 data {ladder id, mode, prim}, +0x54 EFClippingParams;
/// rolling: +0xa0 side, +0xa4 locked, +0xa8 armed, +0xac moves, +0xb0 target, +0xbc camera msg data
#[inline]
fn L(e: &Engine) -> Ptr {
    e.r32(G(e, pLocalDataLadderBase))
}
fn lr(e: &Engine, o: u32) -> u32 {
    e.r32(L(e) + o)
}
fn lw(e: &mut Engine, o: u32, v: u32) {
    let a = L(e) + o;
    e.w32(a, v);
}
/// pLocalDataRollingLadder (same block)
fn rw(e: &mut Engine, o: u32, v: u32) {
    let a = e.r32(G(e, pLocalDataRollingLadder)) + o;
    e.w32(a, v);
}
fn rr(e: &Engine, o: u32) -> u32 {
    e.r32(e.r32(G(e, pLocalDataRollingLadder)) + o)
}
/// pGlobalDataLadderBase[i]: 0 game buffer 0, 1 game buffer 1, 2 game buffer 2
fn GB(e: &Engine, i: u32) -> Ptr {
    e.r32(e.r32(G(e, pGlobalDataLadderBase)) + i * 4)
}
fn dbg(e: &mut Engine, b: &str) {
    if e.r32(G(e, pDebug)) != 0 {
        let (a, b) = (e.cstr("Ladder"), e.cstr(b));
        LIBdebug::PrintDebugInfo(e, a, b);
    }
}

/// 0x00010f38
pub fn ladder_baseST_FIRSTMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    dbg(e, "ladder_baseST_FIRSTMOVE");
    let id = APIgameobj::GetID(e, SELF, NULL);
    let t = G(e, goCustomDataLadder);
    if e.r32(t) == id {
        lw(e, 0, 600);
        for i in 0..3 {
            let v = e.r32(t + 4 + i * 4);
            lw(e, 0x3c + i * 4, v);
        }
    }
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 3, 5);
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 0xb, 2);
    let cfg = lr(e, 0) as i32;
    APIinput::InputConfigAdd(e, cfg);
    let p = L(e) + 0x30;
    APIgameobj::GetPosition(e, SELF, 0x7fff_ffff, p);
    APIinput::InputActionAdd(e, cfg, 0x39, 0);
    let port = e.r32(GB(e, 0) + 0x4c) as i32;
    APIinput::InputDeviceAdd(e, cfg, 0x39, 0x3d, port, 0, 2, 0, 0);
    APIinput::InputDeviceAdd(e, cfg, 0x39, 0x3d, 1, 1, 2, 0, 0x39);
    APIinput::InputContextAdd(e, SELF, 9, cfg, 0x39, 0x1e, 8);
    APIinput::InputContextSetState(e, cfg, 0x39, 0x1e, 2);
    APIinput::InputConfigSetState(e, cfg, 2);
    APIinput::InputConfigActivate(e, cfg, 0);
    let a = LIBAnimMngr::PlayAnim(e, 0.0, 3, 1, 4);
    lw(e, 8, a as u32);
    APIgameobj::SetNextState(e, SELF, 1);
    Ok(0)
}

/// 0x000112f0
pub fn ladder_baseST_BASICMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

fn send_ladder_msg(e: &mut Engine, to: u32) {
    let id = APIgameobj::GetID(e, SELF, NULL);
    lw(e, 0x48, id);
    lw(e, 0x4c, 2);
    let p = lr(e, 4);
    lw(e, 0x50, p);
    let d = L(e) + 0x48;
    APIgameobj::SendMessageTo(e, to, NULL, 0x96, 0, d);
}

/// 0x00011320 external 2: anim event {+0 anim, +4 kind, +8 callback id}
pub fn ladder_baseCB_ANIM(e: &mut Engine, param_1: u32) -> LResult<u64> {
    dbg(e, "ladder_baseCB_ANIM");
    let a = e.r32(param_1);
    if e.r32(param_1 + 4) == 1 && a != 0 && (a < 3 || (a < 6 && 3 < a)) {
        match e.r32(param_1 + 8) {
            1 => {
                let to = e.r32(GB(e, 0) + 8);
                send_ladder_msg(e, to);
            }
            0 => {
                let f = e.r32(GB(e, 0) + 0x34);
                if f & 0x10 != 0 && f & 0x100 == 0 && e.r32(GB(e, 0) + 8) == lr(e, 0xc) {
                    if e.r32(GB(e, 0) + 8) == SHAGGY {
                        ladder_baseFUNC_SetHero(e, SCOOBY);
                    } else {
                        ladder_baseFUNC_SetHero(e, SHAGGY);
                    }
                }
            }
            2 => {
                if lr(e, 0xc) != e.r32(GB(e, 0) + 8) {
                    let to = lr(e, 0xc);
                    send_ladder_msg(e, to);
                }
            }
            _ => dbg(e, "unknown uiCBID"),
        }
    }
    Ok(0)
}

/// 0x000116b8 external 3
pub fn ladder_baseCB_ANIM_LF(e: &mut Engine, _: u32) -> LResult<u64> {
    dbg(e, "ladder_baseCB_ANIM_LF");
    APIgameobj::SendMessageTo(e, 0x2a00_008f, NULL, 0x55, 0, NULL);
    APIanim::StopAllAnim(e, SELF);
    ladder_baseFUNC_SetClipping(e, 2);
    Ok(1)
}

/// 0x00011788 external 4
pub fn ladder_baseCB_ANIM_STOP(e: &mut Engine, _: u32) -> LResult<u64> {
    APIanim::StopAllAnim(e, SELF);
    Ok(1)
}

/// 0x000117d8 external 5
pub fn ladder_baseCB_CollisionDetection(e: &mut Engine, param_1: u32) -> LResult<u64> {
    if e.r32(GB(e, 0) + 8) == e.r32(param_1 + 8) && e.r32(GB(e, 0) + 0x14) & 0x6e20_094d == 0 && {
        // quirk: L+0x28 set inside the condition
        lw(e, 0x28, 1);
        lr(e, 0x10) == 0
    } && e.r32(GB(e, 2)) & 0x20000 == 0
    {
        dbg(e, "ladder_baseCB_CollisionDetection");
        let p = e.r32(param_1 + 4);
        lw(e, 4, p);
        if ladder_baseFUNC_CheckLocalLadder(e) {
            let c = L(e) + 0x54;
            LIBMiscFunc::MiscFunc_RemoveBitClipping(e, c, 0x80);
            lw(e, 0x10, 1);
            let cfg = lr(e, 0) as i32;
            APIinput::InputConfigActivate(e, cfg, 1);
            APIgameobj::SetContextFlag(e, SELF, 0x20);
            APIgameobj::SendMessageTo(e, 0x2a10_0006, NULL, 0x31, 3, NULL);
        }
    }
    Ok(0)
}

/// 0x000119c8 external 6
pub fn ladder_baseCB_CollisionDetectionOut(e: &mut Engine, _: u32) -> LResult<u64> {
    if e.r32(GB(e, 2)) & 0x20000 != 0 {
        APIgameobj::SendMessageTo(e, 0x2a10_0006, NULL, 0x32, 3, NULL);
    }
    let c = L(e) + 0x54;
    LIBMiscFunc::MiscFunc_ReplaceMyClipping(e, c);
    lw(e, 0x10, 0);
    let cfg = lr(e, 0) as i32;
    APIinput::InputConfigActivate(e, cfg, 0);
    APIgameobj::SetContextFlag(e, SELF, 0);
    Ok(1)
}

/// 0x00011ac0 external 7
pub fn ladder_baseCB_POST_COLLISION(e: &mut Engine, _: u32) -> LResult<u64> {
    if lr(e, 0x28) == 0 && lr(e, 0x10) != 0 {
        ladder_baseCB_CollisionDetectionOut(e, 0)?;
    }
    lw(e, 0x28, 0);
    Ok(0)
}

/// 0x00011b48 external 9
pub fn ladder_baseCB_InputAction_X(e: &mut Engine, _: u32) -> LResult<u64> {
    dbg(e, "ladder_baseCB_InputAction_X");
    if e.r32(GB(e, 0) + 0x14) & 0x6e20_094d == 0 {
        LIBApiText::KillText(e);
        ladder_baseCB_CollisionDetectionOut(e, 0)?;
        let h = e.r32(GB(e, 0) + 8);
        ladder_baseFUNC_SetHero(e, h);
    }
    Ok(0)
}

/// 0x00011c38 hero msg 0x96 {ladder, 0 = prim 0x79 / 1 = prim 0x7a}, camera msg 0x54 sub 4
pub fn ladder_baseFUNC_SetHero(e: &mut Engine, param_1: u32) {
    let f = e.alloca(0x10);
    dbg(e, "ladder_baseFUNC_SetHero");
    let id = APIgameobj::GetID(e, SELF, NULL);
    lw(e, 0x48, id);
    match lr(e, 4) {
        0x79 => lw(e, 0x4c, 0),
        0x7a => lw(e, 0x4c, 1),
        _ => {}
    }
    let d = L(e) + 0x48;
    APIgameobj::SendMessageTo(e, param_1, NULL, 0x96, 0, d);
    let c = G(e, moCameraParams);
    e.w32(c, 0);
    e.w32(c + 4, 0);
    let r = APIgameobj::GetPosition(e, SELF, 5, f);
    // quirk: stores GetPosition's return value, not the position
    e.w32(c + 0xc, r);
    e.wf(c + 0x10, 5.0);
    e.wf(c + 0x14, 1.5);
    e.wf(c + 0x18, 8.0);
    APIgameobj::SendMessageTo(e, 0x2a00_008f, NULL, 0x54, 4, c);
}

fn climb(e: &mut Engine, param_2: u32, shaggy_anim: i32, scooby_anim: i32, tag: &str) {
    lw(e, 0xc, param_2);
    APIphysic::SetFlags(e, SELF, 1);
    let leader = e.r32(GB(e, 0) + 8);
    for (h, anim, n) in [(SHAGGY, shaggy_anim, "_SHAGGY"), (SCOOBY, scooby_anim, "_SCOOBYDOO")] {
        if leader == h && param_2 == h {
            dbg(e, &format!("MyMessageFunction --> {tag} -> {n}"));
            ladder_baseFUNC_SetClipping(e, 1);
            let a = LIBAnimMngr::PlayAnim(e, 0.0, anim, 1, 3);
            lw(e, 8, a as u32);
            APIanim::SetSpeed(e, SELF, anim as u32, 1.0);
            break;
        }
    }
    let leader = e.r32(GB(e, 0) + 8);
    APIgameobj::AddChild(e, SELF, if leader == param_2 { 2 } else { 3 }, param_2, 1);
}

/// 0x00011df8
pub fn ladder_baseFUNC_MessageHandler(e: &mut Engine, param_1: u32, param_2: u32, _d: Ptr) -> LResult<u64> {
    match param_1 {
        0xde => {
            dbg(e, "MyMessageFunction --> MSG_ON");
            APIgameobj::RemoveAllChildren(e, SELF, 0);
            lw(e, 0xc, 0);
            APIphysic::ResetFlags(e, SELF, 1);
            APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 3, 5);
            let a = LIBAnimMngr::PlayAnim(e, 0.0, 3, 1, 4);
            lw(e, 8, a as u32);
            APIgameobj::SendMessageTo(e, 0x2a00_008f, NULL, 0x55, 0, NULL);
        }
        0x12 => {
            ResetCollision(e)?;
            APIphysic::SetFlags(e, SELF, 1);
        }
        0x13 => {
            APIphysic::ResetFlags(e, SELF, 1);
        }
        0xe2 => {
            dbg(e, "MyMessageFunction --> MSG_DOWN");
            climb(e, param_2, 2, 5, "MSG_DOWN");
        }
        0xe1 => {
            dbg(e, "MyMessageFunction --> MSG_UP");
            climb(e, param_2, 1, 4, "MSG_UP");
        }
        99999 => {
            let cfg = lr(e, 0) as i32;
            APIinput::InputConfigActivate(e, cfg, 0);
        }
        _ => {}
    }
    Ok(0)
}

/// 0x00012540
pub fn ladder_baseFUNC_CreatePhysic(e: &mut Engine) {
    let f = e.alloca(0x10);
    APIphysic::CreatePhysic(e, SELF, 0);
    APIphysic::SetFlags(e, SELF, 2);
    APIphysic::SetWeight(e, SELF, 1.0);
    APIphysic::SetPriority(e, SELF, 100);
    APIphysic::PrimSetIntAttribBits(e, SELF, 0x79, 0, 4);
    APIphysic::PrimSetIntAttribBits(e, SELF, 0x7a, 0, 4);
    let m = APIphysic::DMAdd(e, SELF, 0x25);
    lw(e, 0x24, m);
    APIphysic::DMGetParams(e, m, f);
    e.w32(f + 4, 7);
    APIphysic::DMSetParams(e, m, f);
    APIphysic::DMSetContextMask(e, m, 0x20);
}

/// 0x000126c0
pub fn ladder_baseFUNC_SetClipping(e: &mut Engine, param_1: i32) {
    if param_1 == 1 {
        LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 2, NULL, 0);
    } else if param_1 == 0 || param_1 == 2 {
        let c = L(e) + 0x54;
        APIgameobj::GetClippingParams(e, SELF, c);
        for (o, v) in [(0, 0x3dcc_cccd), (4, 0x4160_0000), (0xc, 0), (8, 7), (0x10, 0x41c8_0000), (0x18, 0), (0x14, 0x6a7), (0x1c, 0xbf80_0000), (0x24, 0x6a0), (0x20, 0x6a7)] {
            e.w32(c + o, v);
        }
        APIgameobj::SetClippingParams(e, SELF, c, 0);
    }
}

/// 0x00012850
pub fn ResetCollision(e: &mut Engine) -> LResult<()> {
    ladder_baseCB_CollisionDetectionOut(e, 0)?;
    Ok(())
}

/// 0x00012898
pub fn ladder_baseConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    if instanceCount == 1 {
        APIcons::SetExternalFunction(e, 0, ladder_baseST_FIRSTMOVE);
        APIcons::SetExternalFunction(e, 1, ladder_baseST_BASICMOVE);
        APIcons::SetExternalFunction(e, 2, ladder_baseCB_ANIM);
        APIcons::SetExternalFunction(e, 3, ladder_baseCB_ANIM_LF);
        APIcons::SetExternalFunction(e, 4, ladder_baseCB_ANIM_STOP);
        APIcons::SetExternalFunction(e, 5, ladder_baseCB_CollisionDetection);
        APIcons::SetExternalFunction(e, 6, ladder_baseCB_CollisionDetectionOut);
        APIcons::SetExternalFunction(e, 7, ladder_baseCB_POST_COLLISION);
        APIcons::SetExternalFunction(e, 9, ladder_baseCB_InputAction_X);
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
        for (slot, name) in [(pAnimMngr, "AnimMngr"), (pDebug, "debug"), (pFunction, "ApiCFunctions"), (pText, "ApiText"), (pMiscFunc, "MiscFunc")] {
            let n = e.cstr(name);
            let v = APIcons::GetApiCustom(e, n);
            let a = G(e, slot);
            e.w32(a, v);
        }
        let slot = G(e, pGlobalDataLadderBase);
        APIcons::GetGlobalData(e, slot, 0xc);
        for (i, (id, sz)) in [(0u32, (0, 0x8f8)), (1, (1, 0x68)), (2, (2, 0x158))] {
            let b = APIgameobj::GetGameBuffer(e, id, sz, 1);
            let gd = e.r32(G(e, pGlobalDataLadderBase));
            e.w32(gd + i * 4, b);
        }
    }
    let slot = G(e, pLocalDataLadderBase);
    APIcons::GetLocalData(e, slot, 0x94);
    ladder_baseFUNC_CreatePhysic(e);
    ladder_baseFUNC_SetClipping(e, 0);
    Ok(())
}

/// 0x00012c20
pub fn ladder_baseDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    Ok(())
}

fn set_pos(e: &mut Engine, x: u32) {
    lw(e, 0x30, x);
    lw(e, 0x34, 0);
    lw(e, 0x38, 0x4114_cccd);
}

/// 0x00012c50 state 10
pub fn rollingladderST_FIRSTMOVE_LOCAL(e: &mut Engine, a: u32) -> LResult<u64> {
    ladder_baseST_FIRSTMOVE(e, a)?;
    rollingladderFUNC_CreatePhysic(e);
    if e.r32(GB(e, 2) + 8) & 0x10_0000 == 0 {
        rw(e, 0xa4, 1);
        rw(e, 0xa0, 0);
        set_pos(e, 0x423a_0000);
    } else {
        rw(e, 0xa4, 0);
        rw(e, 0xac, 2);
        rw(e, 0xa0, 1);
        set_pos(e, 0x4218_0000);
        let p = L(e) + 0x30;
        APIgameobj::SetTranslation(e, SELF, p);
    }
    APIgameobj::SetNextState(e, SELF, 0xd);
    Ok(0)
}

/// 0x00012dd8 state 11
pub fn rollingladderST_MOVE_SET(e: &mut Engine, _: u32) -> LResult<u64> {
    APIgameobj::SetNextState(e, SELF, 0xc);
    Ok(0)
}

/// 0x00012e30 state 12: rolling until within 0.5 of the target (+0xb0)
pub fn rollingladderST_MOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    let t = e.r32(G(e, pLocalDataRollingLadder)) + 0xb0;
    if LIBApiCFunctions::CheckRangePoint(e, 0.5, SELF, t, 0) != 0 {
        APIphysic::SetPriority(e, SELF, 100);
        APIphysic::SetVelocity(e, SELF, 0.0, 0.0, 0.0);
        APInewSound::StopSound(e, SELF, 0x4000_0008);
        APInewSound::Play(e, SELF, 0x4000_0009, 0);
        let s = rr(e, 0xa4);
        APIgameobj::SendMessageTo(e, 0x2a00_0020, NULL, 0x96, s, NULL);
        APIgameobj::SendMessageTo(e, 0x2a00_008f, NULL, 0x55, 0, NULL);
        APIgameobj::SetNextState(e, SELF, 0xd);
        let c = L(e) + 0x54;
        LIBMiscFunc::MiscFunc_ReplaceMyClipping(e, c);
    }
    Ok(0)
}

/// 0x00012fc8 state 13
pub fn rollingladderST_BLACK_HOLE_LOOP(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

/// 0x00012ff8
pub fn rollingladderFUNC_MessageHandler(e: &mut Engine, param_1: u32, param_2: u32, d: Ptr) -> LResult<u64> {
    match param_1 {
        0xc9 => {
            rw(e, 0xa0, 0);
            set_pos(e, 0x4238_0000);
            let p = L(e) + 0x30;
            APIgameobj::SetTranslation(e, SELF, p);
            rw(e, 0xa4, 1);
        }
        0x4e => {
            if rr(e, 0xa8) == 1 {
                rw(e, 0xa8, 0);
                APInewSound::Play(e, SELF, 0x4000_0008, 0);
                APIgameobj::ChangeState(e, 0xb)?;
            }
        }
        0xe0 => {
            rw(e, 0xa8, 1);
            for (o, v) in [(0xbc, 4), (0xd8, 0x2a00_0070), (0xcc, 0), (0xd0, 0x4060_0000), (0xd4, 0)] {
                rw(e, o, v);
            }
            let id = APIgameobj::GetID(e, SELF, NULL);
            rw(e, 0xe0, id);
            rw(e, 0xf8, 1);
            rw(e, 0xfc, 0);
            rw(e, 0x100, 0x421a);
            let cp = e.r32(G(e, pLocalDataRollingLadder)) + 0xbc;
            APIgameobj::SendMessageTo(e, 0x2a00_008f, NULL, 0x54, 0, cp);
            let side = rr(e, 0xa0);
            if side <= 1 {
                let n = rr(e, 0xac) + 1;
                rw(e, 0xac, n);
                let (vx, x, nside, lock) = if side == 0 { (-4.0, 0x4218_0000, 1, 0) } else { (4.0, 0x4238_0000, 0, 1) };
                APIphysic::SetVelocity(e, SELF, vx, 0.0, 0.0);
                let c = L(e) + 0x54;
                LIBMiscFunc::MiscFunc_RemoveBitClipping(e, c, 0x480);
                rw(e, 0xb0, x);
                rw(e, 0xb4, 0);
                rw(e, 0xb8, 0x4114_cccd);
                rw(e, 0xa0, nside);
                if side == 0 {
                    rw(e, 0xa4, lock);
                }
                set_pos(e, x);
                if side == 1 {
                    rw(e, 0xa4, lock);
                }
            }
        }
        99999 => rw(e, 0x10, 0),
        _ => {}
    }
    ladder_baseFUNC_MessageHandler(e, param_1, param_2, d)
}

/// 0x00013520
pub fn ladder_baseFUNC_CheckLocalLadder(e: &mut Engine) -> bool {
    rr(e, 0xa4) == 0
}

/// 0x00013578
pub fn rollingladderFUNC_CreatePhysic(e: &mut Engine) {
    APIphysic::SetWeight(e, SELF, 1.0);
    APIphysic::SetPriority(e, SELF, 100);
    APIphysic::SetPrimaryPrim(e, SELF, 0x78);
}

/// 0x000135f8
pub fn rollingladderConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    ladder_baseConstructor(e, api, instanceCount)?;
    APIcons::SetExternalFunction(e, 10, rollingladderST_FIRSTMOVE_LOCAL);
    APIcons::SetExternalFunction(e, 0xb, rollingladderST_MOVE_SET);
    APIcons::SetExternalFunction(e, 0xc, rollingladderST_MOVE);
    APIcons::SetExternalFunction(e, 0xd, rollingladderST_BLACK_HOLE_LOOP);
    APIcons::SetMessageFunction(e, rollingladderFUNC_MessageHandler);
    let slot = G(e, pLocalDataRollingLadder);
    APIcons::GetLocalData(e, slot, 0x108);
    APIcons::SetInitialState(e, 10);
    Ok(())
}

/// 0x00013700
pub fn rollingladderDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    ladder_baseDestructor(e, instanceCount)
}

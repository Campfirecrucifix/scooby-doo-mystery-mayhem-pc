//! `cartstype01`: the Ep1 library book carts (heavyobj_base): the hero pushes one with X (hero msg
//! 0x9d), its slot bit (custom data +4) goes into game buffer 2 +8; msg 0xc1 puts it back.
//! cartstype01 adds msg 0x10a to 0x2a000020 on 0xc1/0xdf.
//! Port of `Game Logic Modules/Ep1_Library/Objects/CARTS/Level_01/cartstype01__fd60b9a5/
//! {heavyobj_base.c, carts.c, cartstype01.c}` (19 functions). One build (EP1L01, sha fd60b9a5).
#![allow(non_snake_case, non_upper_case_globals, unused_variables)]

use crate::engine::*;
use crate::logic::{LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "cartstype01",
    data_size: 0xea0,
    ctor: ctor_dispatch,
    dtor: dtor_dispatch,
    register: None,
    // goCustomDataHeavyObj: 2 x {object id, slot bit, push data, 0}
    init: &[],
    build_init: Some(build_init_dispatch),
};

/// initialised data of the hand-ported build(s) (written by build_init_dispatch only for them)
const OLD_INIT: &[(u32, &[u32])] = &[(0x10000, &[0x2a00_0051, 0x8000, 3, 0, 0x2a00_0052, 0x10000, 2, 0])];

mod xb;

/// number of hand-ported builds (indices 0..N_OLD); generated builds (`xb`, c2rs) follow
pub(crate) const N_OLD: usize = 1;

fn ctor_dispatch(e: &mut Engine, api: Ptr, n: i32) -> LResult<()> {
    if e.rt.cur_build() < N_OLD { cartstype01Constructor(e, api, n) } else { xb::ctor(e, api, n) }
}

fn dtor_dispatch(e: &mut Engine, n: i32) -> LResult<()> {
    if e.rt.cur_build() < N_OLD { cartstype01Destructor(e, n) } else { xb::dtor(e, n) }
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
    (key == "fd60b9a5").then_some(0)
}

pub const goCustomDataHeavyObj: u32 = 0x10000;
pub const pMiscFunc: u32 = 0x10020;
pub const pLocalDataHeavyObj: u32 = 0x10024;
pub const pGlobalDataHeavyObj: u32 = 0x10028;
pub const pAnim: u32 = 0x1002c;
pub const pExtra: u32 = 0x10030;
pub const pGame: u32 = 0x10034;
pub const pGameObj: u32 = 0x10038;
pub const pHud: u32 = 0x1003c;
pub const pInput: u32 = 0x10040;
pub const pPhysic: u32 = 0x10044;
pub const pSound: u32 = 0x10048;
pub const pAnimMngr: u32 = 0x1004c;
pub const pDebug: u32 = 0x10050;
pub const pSndMngr: u32 = 0x10054;

#[inline]
fn G(e: &Engine, a: u32) -> Ptr {
    e.rt.cur_data() + (a - 0x10000)
}
/// local data (0x88): +0 id, +4 input config, +8 DM, +0xc panel sub, +0x10 touched this frame,
/// +0x14 panel shown, +0x18 can be pushed, +0x1c placed, +0x20 hero msg 0x9d data {id, .., .., +0x2c push data},
/// +0x30 clipping params (never filled: quirk), +0x70 slot bit, +0x74 offset, +0x78, +0x7c home position
#[inline]
fn L(e: &Engine) -> Ptr {
    e.r32(G(e, pLocalDataHeavyObj))
}
fn lr(e: &Engine, o: u32) -> u32 {
    e.r32(L(e) + o)
}
fn lw(e: &mut Engine, o: u32, v: u32) {
    let a = L(e) + o;
    e.w32(a, v);
}
fn GB(e: &Engine, i: u32) -> Ptr {
    e.r32(e.r32(G(e, pGlobalDataHeavyObj)) + i * 4)
}
fn dbg(e: &mut Engine, b: &str) {
    if e.r32(G(e, pDebug)) != 0 {
        // DAT_00010d10: the tag string (in .rodata, not recovered)
        let (a, b) = (e.cstr("HeavyObj"), e.cstr(b));
        LIBdebug::PrintDebugInfo(e, a, b);
    }
}

/// 0x00010e38
pub fn fstate_FIRSTMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    let f = e.alloca(0x40);
    dbg(e, "fstate_FIRSTMOVE");
    heavyobj_baseFUNC_CreatePhysic(e);
    for i in 0..2u32 {
        let t = G(e, goCustomDataHeavyObj) + i * 0x10;
        if e.r32(t) == lr(e, 0) {
            lw(e, 4, i + 900);
            for k in 0..3 {
                let v = e.r32(t + 4 + k * 4);
                lw(e, 0x70 + k * 4, v);
            }
        }
    }
    if e.r32(GB(e, 2) + 8) & lr(e, 0x70) == 0 || lr(e, 0x18) != 1 {
        APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 3, 3);
        lw(e, 0x1c, 0);
    } else {
        heavyobj_baseFUNC_SetPos(e);
        lw(e, 0x1c, 1);
    }
    // _ShadowParams on the stack (unset words: stack garbage, zero here)
    for (o, v) in [(0, 2), (4, 0), (0x14, 0x80), (0x1c, 0x3e80_0000), (0x20, 0x40a0_0000), (0x24, 0x80), (0x28, 0x3f40_0000)] {
        e.w32(f + o, v);
    }
    APIgameobj::SetShadowParams(e, SELF, f);
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 0xb, 2);
    let cfg = lr(e, 4) as i32;
    APIinput::InputConfigAdd(e, cfg);
    APIinput::InputActionAdd(e, cfg, 0x39, 0);
    let port = e.r32(GB(e, 0) + 0x4c) as i32;
    APIinput::InputDeviceAdd(e, cfg, 0x39, 0x3d, port, 0, 2, 0, 0);
    APIinput::InputDeviceAdd(e, cfg, 0x39, 0x3d, 1, 1, 2, 0, 0x39);
    APIinput::InputContextAdd(e, SELF, 6, cfg, 0x39, 0x1e, 8);
    APIinput::InputContextSetState(e, cfg, 0x39, 0x1e, 2);
    APIinput::InputConfigSetState(e, cfg, 2);
    APIinput::InputConfigActivate(e, cfg, 0);
    dbg(e, "InputConfigActivate - 0");
    LIBAnimMngr::PlayAnim(e, 0.0, 1, 1, 0);
    APIgameobj::SetNextState(e, SELF, 1);
    Ok(0)
}

/// 0x000112c0
pub fn fstate_BASICMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

/// 0x000112f0 external 3
pub fn fcback_CollisionDetection(e: &mut Engine, param_1: u32) -> LResult<u64> {
    if e.r32(GB(e, 0) + 8) == e.r32(param_1 + 8) && e.r32(GB(e, 0) + 0x14) & 0x6e20_090d == 0 {
        match e.r32(GB(e, 0) + 4) {
            1 | 2 | 7 | 8 | 9 | 10 | 0x10 | 0x11 => {
                if lr(e, 0x18) != 0 && {
                    lw(e, 0x10, 1);
                    lr(e, 0x14) == 0
                } {
                    dbg(e, "fcback_CollisionDetection");
                    lw(e, 0x14, 1);
                    APIgameobj::SetContextFlag(e, SELF, 2);
                    if e.r32(GB(e, 2)) & 0x20000 == 0 {
                        let s = lr(e, 0xc);
                        APIgameobj::SendMessageTo(e, 0x2a10_0006, NULL, 0x31, s, NULL);
                    }
                    let cfg = lr(e, 4) as i32;
                    APIinput::InputConfigActivate(e, cfg, 1);
                    let c = L(e) + 0x30;
                    LIBMiscFunc::MiscFunc_RemoveBitClipping(e, c, 0x80);
                }
            }
            _ => dbg(e, "can`t do it with this caracter..."),
        }
    }
    Ok(0)
}

/// 0x00011550 external 4
pub fn fcback_PostCollision(e: &mut Engine, _: u32) -> LResult<u64> {
    if lr(e, 0x10) == 0 {
        if lr(e, 0x14) != 0 && {
            lw(e, 0x14, 0);
            e.r32(GB(e, 2)) & 0x20000 != 0
        } {
            let s = lr(e, 0xc);
            APIgameobj::SendMessageTo(e, 0x2a10_0006, NULL, 0x32, s, NULL);
        }
        let cfg = lr(e, 4) as i32;
        APIinput::InputConfigActivate(e, cfg, 0);
        APIgameobj::SetContextFlag(e, SELF, 0);
        let c = L(e) + 0x30;
        LIBMiscFunc::MiscFunc_ReplaceMyClipping(e, c);
    }
    lw(e, 0x10, 0);
    Ok(0)
}

/// 0x00011680 external 2
pub fn fcback_ANIM(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

/// 0x00011700 external 6
pub fn fcback_InputAction_X(e: &mut Engine, _: u32) -> LResult<u64> {
    dbg(e, "fcback_InputAction_X");
    let b = e.r32(GB(e, 0) + 0x14) & 0x6e20_090d == 0;
    if b {
        APIgameobj::UnregisterNotify(e, SELF, 0xffff_ffff, 3);
        let cfg = lr(e, 4) as i32;
        APIinput::InputConfigActivate(e, cfg, 0);
        let g = GB(e, 2) + 8;
        let v = e.r32(g) | lr(e, 0x70);
        e.w32(g, v);
        let id = lr(e, 0);
        lw(e, 0x20, id);
        let d = lr(e, 0x74);
        lw(e, 0x2c, d);
        let hero = e.r32(GB(e, 0) + 8);
        let p = L(e) + 0x20;
        APIgameobj::SendMessageTo(e, hero, NULL, 0x9d, 0, p);
        APIgameobj::SetGroupID(e, SELF, 6);
    }
    Ok(b as u64)
}

/// 0x000118a0
pub fn MyMessageFunction(e: &mut Engine, param_1: u32, _s: u32, _d: Ptr) -> LResult<u64> {
    match param_1 {
        0xd4 => {
            if e.r32(GB(e, 2) + 8) & lr(e, 0x70) != 0 {
                lw(e, 0x1c, 1);
            }
        }
        0x13 => {
            APIphysic::ResetFlags(e, SELF, 1);
        }
        0x12 => {
            ResetCollision(e);
            APIphysic::SetFlags(e, SELF, 1);
        }
        0xc1 => {
            if lr(e, 0x1c) == 0 {
                heavyobj_baseFUNC_ResetPos(e);
            }
        }
        0xde => {
            APInewSound::Play(e, SELF, 0x4000_0007, 0);
            dbg(e, "MSG_ON playing sound");
        }
        0xda => {
            APInewSound::StopSound(e, SELF, 0x4000_0007);
        }
        0x164 => lw(e, 0x18, 1),
        99999 => {
            let cfg = lr(e, 4) as i32;
            APIinput::InputConfigActivate(e, cfg, 0);
            dbg(e, "InputConfigActivate - 0");
        }
        _ => {}
    }
    Ok(0)
}

/// 0x00011b60
pub fn heavyobj_baseFUNC_ResetPos(e: &mut Engine) {
    let p = L(e) + 0x7c;
    APIgameobj::SetTranslation(e, SELF, p);
    lw(e, 0x18, 1);
    let g = GB(e, 2) + 8;
    let v = e.r32(g) & !lr(e, 0x70);
    e.w32(g, v);
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 3, 3);
    APIgameobj::SetGroupID(e, SELF, 0);
    APIgameobj::SendMessageTo(e, SELF, NULL, 0x9f, 0, NULL);
}

/// 0x00011c60 moves the cart by (0, 0, L+0x74 * 1.507) in its own frame
pub fn heavyobj_baseFUNC_SetPos(e: &mut Engine) {
    let f = e.alloca(0x24);
    let (p50, p40, p30) = (f, f + 0xc, f + 0x18);
    e.wf(p40, 0.0);
    e.wf(p40 + 4, 0.0);
    e.wf(p40 + 8, 0.0);
    APIgameobj::GetPosition(e, SELF, 0x7fff_ffff, p50);
    // unsigned -> float (the < 0 branch is GCC's u32 conversion)
    let z = lr(e, 0x74) as f32 * 1.507;
    e.wf(p40 + 8, z);
    APIgameobj::TransformVector(e, SELF, p40, p40);
    for k in 0..3 {
        let v = e.rf(p50 + k * 4) + e.rf(p40 + k * 4);
        e.wf(p30 + k * 4, v);
    }
    APIgameobj::SetTranslation(e, SELF, p30);
    lw(e, 0x18, 0);
    APIgameobj::SetGroupID(e, SELF, 6);
    APIgameobj::SendMessageTo(e, SELF, NULL, 0x9e, 0, NULL);
}

/// 0x00011dd8 (EFClippingParams on the stack)
pub fn heavyobj_baseFUNC_SetClipping(e: &mut Engine) {
    let c = e.alloca(0x28);
    APIgameobj::GetClippingParams(e, SELF, c);
    for (o, v) in [(0, 0x3dcc_cccd), (4, 0x41a0_0000), (0x10, 0x420c_0000), (0x1c, 0xbf80_0000), (0xc, 0), (8, 7), (0x18, 0x80), (0x14, 0x287), (0x24, 0x280), (0x20, 0x287)] {
        e.w32(c + o, v);
    }
    APIgameobj::SetClippingParams(e, SELF, c, 0);
}

/// 0x00011ea0
pub fn heavyobj_baseFUNC_CreatePhysic(e: &mut Engine) {
    let f = e.alloca(0x10);
    APIphysic::CreatePhysic(e, SELF, 0);
    APIphysic::SetFlags(e, SELF, 2);
    let m = APIphysic::DMAdd(e, SELF, 0x25);
    lw(e, 8, m);
    APIphysic::DMGetParams(e, m, f);
    e.w32(f + 4, 4);
    APIphysic::DMSetParams(e, m, f);
    APIphysic::DMSetContextMask(e, m, 2);
}

/// 0x00011fa0
pub fn ResetCollision(e: &mut Engine) {
    if lr(e, 0x14) != 0 && {
        lw(e, 0x14, 0);
        e.r32(GB(e, 2)) & 0x20000 != 0
    } {
        let s = lr(e, 0xc);
        APIgameobj::SendMessageTo(e, 0x2a10_0006, NULL, 0x32, s, NULL);
    }
    let cfg = lr(e, 4) as i32;
    APIinput::InputConfigActivate(e, cfg, 0);
    APIgameobj::SetContextFlag(e, SELF, 0);
    lw(e, 0x10, 0);
}

/// 0x00012088
pub fn heavyobj_baseConstructor(e: &mut Engine, api: Ptr, param_2: i32) -> u32 {
    if param_2 == 1 {
        APIcons::SetExternalFunction(e, 0, fstate_FIRSTMOVE);
        APIcons::SetExternalFunction(e, 1, fstate_BASICMOVE);
        APIcons::SetExternalFunction(e, 2, fcback_ANIM);
        APIcons::SetExternalFunction(e, 3, fcback_CollisionDetection);
        APIcons::SetExternalFunction(e, 4, fcback_PostCollision);
        APIcons::SetExternalFunction(e, 6, fcback_InputAction_X);
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
        let slot = G(e, pGlobalDataHeavyObj);
        APIcons::GetGlobalData(e, slot, 0xc);
        for (i, (id, sz)) in [(0u32, (0, 0x8f8)), (1, (1, 0x68)), (2, (2, 0x158))] {
            let b = APIgameobj::GetGameBuffer(e, id, sz, 1);
            let gd = e.r32(G(e, pGlobalDataHeavyObj));
            e.w32(gd + i * 4, b);
        }
    }
    let slot = G(e, pLocalDataHeavyObj);
    APIcons::GetLocalData(e, slot, 0x88);
    lw(e, 0x18, 1);
    lw(e, 0xc, 4);
    let id = APIgameobj::GetID(e, SELF, NULL);
    lw(e, 0, id);
    let p = L(e) + 0x7c;
    APIgameobj::GetPosition(e, SELF, 0x7fff_ffff, p);
    heavyobj_baseFUNC_SetClipping(e);
    APIcons::SetMessageFunction(e, MyMessageFunction)
}

/// 0x00012418
pub fn heavyobj_baseDestructor(e: &mut Engine) {}

/// 0x00012448
pub fn cartsConstructor(e: &mut Engine, api: Ptr, param_2: i32) {
    heavyobj_baseConstructor(e, api, param_2);
    APIcons::SetInitialState(e, 0);
}

/// 0x000124b0
pub fn cartsDestructor(e: &mut Engine) {
    heavyobj_baseDestructor(e);
}

/// 0x00012508
pub fn cartstype01MessageFunction(e: &mut Engine, param_1: u32, s: u32, d: Ptr) -> LResult<u64> {
    if param_1 == 0xc1 {
        if lr(e, 0x1c) == 0 && e.r32(GB(e, 2) + 8) & lr(e, 0x70) != 0 {
            APIgameobj::SendMessageTo(e, 0x2a00_0020, NULL, 0x10a, 0, NULL);
        }
    } else if param_1 == 0xdf {
        APIgameobj::SendMessageTo(e, 0x2a00_0020, NULL, 0x10a, 1, NULL);
    }
    MyMessageFunction(e, param_1, s, d)?;
    Ok(0)
}

/// 0x00012640
pub fn cartstype01Constructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    cartsConstructor(e, api, instanceCount);
    APIcons::SetMessageFunction(e, cartstype01MessageFunction);
    APIcons::SetInitialState(e, 0);
    Ok(())
}

/// 0x000126c0
pub fn cartstype01Destructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    cartsDestructor(e);
    Ok(())
}

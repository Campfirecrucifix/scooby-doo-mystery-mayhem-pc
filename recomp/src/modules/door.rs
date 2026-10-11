//! `door`: EP1L03A door puzzle (code from Ep3_FrontierLand/Objects/L01_DOORPUZZLE). Doors
//! 0x2a00011e (carries 0x2a000089 to locator 0x2a000120) and 0x2a00011f (0x2a000070 -> 0x2a000121).
//! The leader touching it (GB0+0x14 & 0x6e20090d == 0, touching hero 0x2a0000ef/0x2a0000f0, GB2 bit
//! 0x20000 clear) activates input config 0x514+n and shows the panel prompt (0x2a100000 msg 0x31/9);
//! X plays the door anim; anim action 1 moves the carried object onto the locator (msg 0xd3).
//! Port of `Ep3_FrontierLand/Objects/L01_DOORPUZZLE/door__fc0d9f03/Door.c` (14 functions). Build fc0d9f03.
#![allow(non_snake_case, non_upper_case_globals, unused_variables)]

use crate::engine::*;
use crate::logic::{LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "door",
    // globals 0x10000..0x10028 (+ unused gpoLanguagesNamesAndISOCodes)
    data_size: 0xd00,
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
    if e.rt.cur_build() < N_OLD { doorConstructor(e, api, n) } else { xb::ctor(e, api, n) }
}

fn dtor_dispatch(e: &mut Engine, n: i32) -> LResult<()> {
    if e.rt.cur_build() < N_OLD { doorDestructor(e, n) } else { xb::dtor(e, n) }
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
    (key == "fc0d9f03").then_some(0)
}

pub const pExtra: u32 = 0x10000;
pub const pGameObj: u32 = 0x10004;
pub const pPhysic: u32 = 0x10008;
pub const pAnim: u32 = 0x1000c;
pub const pLight: u32 = 0x10010;
pub const pInput: u32 = 0x10014;
pub const pAnimMngr: u32 = 0x10018;
pub const pMiscFunc: u32 = 0x1001c;
pub const pLocalDatadoor: u32 = 0x10020;
pub const pGlobalDatadoor: u32 = 0x10024;

#[inline]
fn G(e: &Engine, a: u32) -> Ptr {
    e.rt.cur_data() + (a - 0x10000)
}
/// local data, byte offset
fn L(e: &Engine, o: u32) -> Ptr {
    e.r32(G(e, pLocalDatadoor)) + o
}
fn lr(e: &Engine, o: u32) -> u32 {
    e.r32(L(e, o))
}
fn lw(e: &mut Engine, o: u32, v: u32) {
    let a = L(e, o);
    e.w32(a, v);
}
/// global data, byte offset: +0 GB0, +4 GB2, +8 word
fn gr(e: &Engine, o: u32) -> u32 {
    e.r32(e.r32(G(e, pGlobalDatadoor)) + o)
}
fn gw(e: &mut Engine, o: u32, v: u32) {
    let a = e.r32(G(e, pGlobalDatadoor)) + o;
    e.w32(a, v);
}

/// 0x00010cf8 state 0
pub fn doorST_FIRSTMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    let id = APIgameobj::GetID(e, SELF, NULL);
    lw(e, 0x14, 0);
    lw(e, 0x10, 0);
    lw(e, 0x1c, 1);
    lw(e, 0x18, 1);
    lw(e, 0x2c, 1);
    // EFClippingParams at fp+0x10; the 25.0 local sits at fp+0x50 (= params+0x40) and is
    // re-read after GetClippingParams (disasm 0x10da4)
    let c = e.alloca(0x44);
    e.wf(c + 0x40, 25.0);
    APIgameobj::GetClippingParams(e, SELF, c);
    let f = e.rf(c + 0x40);
    e.wf(c + 4, f);
    e.w32(c + 0xc, 0);
    e.w32(c + 8, 0);
    e.wf(c + 0x10, f + f);
    e.w32(c + 0x18, 0x6a0);
    e.w32(c + 0x14, 0x6a0);
    APIgameobj::SetClippingParams(e, SELF, c, 0);
    if id < 0x2a00_0120 && 0x2a00_011d < id {
        APIanim::Play(e, SELF, 1, 2, 0.0);
        APIanim::SetTime(e, SELF, 1, 0.0);
        APIanim::SetSpeed(e, SELF, 1, 0.0);
        lw(e, 0, 1);
        if id == 0x2a00_011e {
            lw(e, 0x14, 0x2a00_0089);
            lw(e, 0x10, 0x2a00_0120);
        }
        if id == 0x2a00_011f {
            lw(e, 0x14, 0x2a00_0070);
            lw(e, 0x10, 0x2a00_0121);
        }
    }
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 0xb, 6);
    APIgameobj::SetGroupID(e, SELF, 0x15);
    APIgameobj::SetNextState(e, SELF, 1);
    Ok(0)
}

/// 0x00010fa8 state 1
pub fn doorST_BASICMOVE_SET(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

/// 0x00010fd8 state 2
pub fn doorST_BASICMOVE_LOOP(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

/// 0x00011008 callback 3
pub fn doorCB_NOTIFY_COLLIDE_OBJ(e: &mut Engine, p: u32) -> LResult<u64> {
    let who = e.r32(p + 8);
    if e.r32(gr(e, 0) + 8) == who && e.r32(gr(e, 0) + 0x14) & 0x6e20_090d == 0 {
        lw(e, 0x34, 1);
        if who < 0x2a00_00f1 && 0x2a00_00ee < who && lr(e, 0x30) == 0 && e.r32(gr(e, 4)) & 0x20000 == 0 {
            lw(e, 0x30, 1);
            APIgameobj::SetContextFlag(e, SELF, 4);
            let cfg = lr(e, 8) as i32;
            APIinput::InputConfigActivate(e, cfg, 1);
            APIgameobj::SendMessageTo(e, 0x2a10_0000, NULL, 0x31, 9, NULL);
            let c = L(e, 0x3c);
            LIBMiscFunc::MiscFunc_RemoveBitClipping(e, c, 0x80);
        }
    }
    Ok(0)
}

/// 0x000111c8
pub fn ResetCollision(e: &mut Engine) {
    if e.r32(gr(e, 4)) & 0x20000 != 0 {
        APIgameobj::SendMessageTo(e, 0x2a10_0000, NULL, 0x32, 9, NULL);
    }
    let c = L(e, 0x3c);
    LIBMiscFunc::MiscFunc_ReplaceMyClipping(e, c);
    lw(e, 0x30, 0);
    let cfg = lr(e, 8) as i32;
    APIinput::InputConfigActivate(e, cfg, 0);
    APIgameobj::SetContextFlag(e, SELF, 0);
}

/// 0x000112a8 callback 4
pub fn doorCB_NOTIFY_COLLIDE_OBJ_OUT(e: &mut Engine, _: u32) -> LResult<u64> {
    if lr(e, 0x30) != 0 && lr(e, 0x34) == 0 {
        ResetCollision(e);
        gw(e, 8, 0);
    }
    lw(e, 0x34, 0);
    Ok(0)
}

/// 0x00011338 callback 5
pub fn doorCB_ANIM_LF(e: &mut Engine, _: u32) -> LResult<u64> {
    let a = lr(e, 0);
    APIanim::SetSpeed(e, SELF, a, -0.05);
    let t = APIanim::GetTime(e, SELF, a);
    APIanim::SetTime(e, SELF, a, t);
    APIanim::Play(e, SELF, a, 1, 0.0);
    let cfg = lr(e, 8) as i32;
    APIinput::InputConfigSetState(e, cfg, 2);
    ResetCollision(e);
    Ok(1)
}

/// moves `obj` onto locator `loc` and hands it over (msg 0xd3)
fn place(e: &mut Engine, obj: u32, loc: u32) {
    let v = e.alloca(0xc);
    APIgameobj::GetPosition(e, loc, 0x7fff_ffff, v);
    APIgameobj::SetTranslation(e, obj, v);
    APIgameobj::SendMessageTo(e, obj, NULL, 0xd3, loc, NULL);
}

/// 0x00011458 callback 6: param {?, action}
pub fn doorCB_ANIM_ACTION(e: &mut Engine, p: u32) -> LResult<u64> {
    if e.r32(p + 4) == 1 {
        APIgameobj::GetID(e, SELF, NULL);
        let t = lr(e, 0x14);
        if t == 0x2a00_0089 {
            if lr(e, 0x18) != 0 && APIgameobj::GetID(e, 0x2a00_0089, NULL) != 0x7fff_fffe {
                place(e, 0x2a00_0089, 0x2a00_0120);
                lw(e, 0x18, 0);
            }
        } else {
            if t < 0x2a00_008a {
                if t == 0x2a00_0070 {
                    if lr(e, 0x2c) == 0 || APIgameobj::GetID(e, 0x2a00_0070, NULL) == 0x7fff_fffe {
                        return Ok(0);
                    }
                    place(e, 0x2a00_0070, 0x2a00_0121);
                    lw(e, 0x2c, 0);
                    return Ok(0);
                }
            } else if t == 0x2a00_00a2 {
                if lr(e, 0x1c) == 0 || APIgameobj::GetID(e, 0x2a00_00a2, NULL) == 0x7fff_fffe {
                    return Ok(0);
                }
                place(e, 0x2a00_00a2, 0x2a00_0120);
                lw(e, 0x1c, 0);
                return Ok(0);
            }
            let (t, s) = (lr(e, 0x14), lr(e, 0x10));
            APIgameobj::SendMessageTo(e, t, NULL, 300, s, NULL);
        }
    }
    Ok(0)
}

/// 0x000117f8 callback 7
pub fn doorCB_INPUT_X(e: &mut Engine, _: u32) -> LResult<u64> {
    let id = APIgameobj::GetID(e, SELF, NULL);
    lw(e, 4, id);
    let a = lr(e, 0);
    APIanim::SetTime(e, SELF, a, 0.0);
    LIBAnimMngr::PlayAnim(e, 0.0, a as i32, 1, 5);
    let (o, a) = (lr(e, 4), lr(e, 0));
    APIanim::SetSpeed(e, o, a, 1.0);
    ResetCollision(e);
    Ok(0)
}

/// 0x000118f8
pub fn doorFUNC_MessageHandler(e: &mut Engine, param_1: u32, _s: u32, _d: Ptr) -> LResult<u64> {
    if param_1 == 0xde {
        lw(e, 0x28, 0);
    } else if param_1 < 0xdf {
        if param_1 == 0xc9 {
            lw(e, 0x1c, 1);
            lw(e, 0x18, 1);
            lw(e, 0x2c, 1);
            ResetCollision(e);
            gw(e, 8, 0);
        }
    } else if param_1 == 0xdf {
        lw(e, 0x28, 1);
    } else if param_1 == 299 {
        let id = APIgameobj::GetID(e, SELF, NULL);
        gw(e, 8, 0);
        if id == 0x2a00_011e {
            APIanim::Play(e, SELF, 1, 2, 0.0);
            APIanim::SetTime(e, SELF, 1, 0.0);
            APIanim::SetSpeed(e, SELF, 1, 0.0);
            lw(e, 0, 1);
        }
    }
    Ok(0)
}

/// 0x00011ae0
pub fn doorFUNC_Create_Input(e: &mut Engine) {
    let c = lr(e, 8) as i32;
    APIinput::InputConfigAdd(e, c);
    APIinput::InputActionAdd(e, c, 0x39, 0);
    let pad = e.r32(gr(e, 0) + 0x4c) as i32;
    APIinput::InputDeviceAdd(e, c, 0x39, 0x3d, pad, 0, 2, 0, 0);
    APIinput::InputDeviceAdd(e, c, 0x39, 0x3d, 1, 1, 2, 0, 0x39);
    APIinput::InputContextAdd(e, SELF, 7, c, 0x39, 0x1e, 8);
    APIinput::InputContextSetState(e, c, 0x39, 0x1e, 2);
    APIinput::InputConfigSetState(e, c, 2);
    APIinput::InputConfigActivate(e, c, 0);
}

/// 0x00011c80
pub fn doorFUNC_Create_Physic(e: &mut Engine) {
    APIphysic::CreatePhysic(e, SELF, 0);
    APIphysic::SetFlags(e, SELF, 2);
    APIphysic::SetWeight(e, SELF, 1.0);
    APIphysic::SetPriority(e, SELF, 100);
    APIphysic::PrimSetIntAttribBits(e, SELF, 0, 0, 4);
    let dm = APIphysic::DMAdd(e, SELF, 0x25);
    lw(e, 0x38, dm);
    let s = e.alloca(0x40);
    APIphysic::DMGetParams(e, dm, s);
    e.w32(s + 4, 4);
    APIphysic::DMSetParams(e, dm, s);
    APIphysic::DMSetContextMask(e, dm, 4);
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 3, 3);
}

/// 0x00011e08
pub fn doorConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    if instanceCount == 1 {
        APIcons::SetExternalFunction(e, 0, doorST_FIRSTMOVE);
        APIcons::SetExternalFunction(e, 1, doorST_BASICMOVE_SET);
        APIcons::SetExternalFunction(e, 2, doorST_BASICMOVE_LOOP);
        APIcons::SetExternalFunction(e, 3, doorCB_NOTIFY_COLLIDE_OBJ);
        APIcons::SetExternalFunction(e, 4, doorCB_NOTIFY_COLLIDE_OBJ_OUT);
        APIcons::SetExternalFunction(e, 5, doorCB_ANIM_LF);
        APIcons::SetExternalFunction(e, 7, doorCB_INPUT_X);
        APIcons::SetExternalFunction(e, 6, doorCB_ANIM_ACTION);
        let tables: [(u32, fn(&mut Engine) -> Ptr); 6] = [
            (pAnim, |e| APIcons::GetApiAnim(e)),
            (pGameObj, |e| APIcons::GetApiGameObj(e)),
            (pExtra, |e| APIcons::GetApiExtra(e)),
            (pPhysic, |e| APIcons::GetApiPhysic(e)),
            (pInput, |e| APIcons::GetApiInput(e)),
            (pLight, |e| APIcons::GetApiLight(e)),
        ];
        for (slot, get) in tables {
            let v = get(e);
            let a = G(e, slot);
            e.w32(a, v);
        }
        for (slot, name) in [(pAnimMngr, "AnimMngr"), (pMiscFunc, "MiscFunc")] {
            let n = e.cstr(name);
            let v = APIcons::GetApiCustom(e, n);
            let a = G(e, slot);
            e.w32(a, v);
        }
        let slot = G(e, pGlobalDatadoor);
        APIcons::GetGlobalData(e, slot, 0xc);
        let b = APIgameobj::GetGameBuffer(e, 0, 0x8f8, 1);
        gw(e, 0, b);
        let b = APIgameobj::GetGameBuffer(e, 2, 0x158, 1);
        gw(e, 4, b);
    }
    APIcons::SetMessageFunction(e, doorFUNC_MessageHandler);
    let slot = G(e, pLocalDatadoor);
    APIcons::GetLocalData(e, slot, 0x7c);
    lw(e, 8, (instanceCount + 0x514) as u32);
    doorFUNC_Create_Input(e);
    doorFUNC_Create_Physic(e);
    let dm = APIphysic::DMAdd(e, SELF, 0x23);
    lw(e, 0xc, dm);
    APIphysic::DMSetContextMask(e, dm, 4);
    APIcons::SetInitialState(e, 0);
    Ok(())
}

/// 0x00012140
pub fn doorDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    Ok(())
}

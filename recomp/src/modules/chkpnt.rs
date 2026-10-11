//! `chkpnt`: the Ep1 checkpoint (save book stand). Glows, flashes on activation (msg 0x44 from the
//! save panel), takes a "Scooby_Shaggy_*" picture through the tex fader, and opens the save panel
//! (0x2a100006, msgs 0x31/0x32/0x41/0x42) when the leader hero touches it and presses X.
//! Port of `Game Logic Modules/Globals/Collectables/CheckPoint/chkpnt__14f72b78/ChkPnt.c`
//! (15 functions). Builds: 0 = 14f72b78 (EP1L01), 1 = be70538d (EP1L03A: panel 0x2a100000, no
//! tutorial branch msg 0xa3 to 0x2a000020 in ChkPntCB_NOTIFY_COLLIDE_OBJ).
#![allow(non_snake_case, non_upper_case_globals, unused_variables)]

use crate::engine::*;
use crate::logic::{LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "chkpnt",
    // globals 0x10000..0x10038 (+ unused gpoLanguagesNamesAndISOCodes)
    data_size: 0x38,
    ctor: chkpntConstructor,
    dtor: chkpntDestructor,
    register: None,
    init: &[],
    build_init: None,
};

pub fn build_index(key: &str) -> Option<usize> {
    // WP-L8: 1bd85f39 / 6de6c35c / 5e6eea89 = be70538d with another panel id; 5e6eea89 also has the
    // 10.5 s first-checkpoint timer (FIRSTMOVE + MyMessageFunction flag L+0xf0) and msg 0xd4 to 0x2a00000e
    ["14f72b78", "be70538d", "1bd85f39", "6de6c35c", "5e6eea89"].iter().position(|k| *k == key)
}
const B_5E6EEA89: usize = 4;
/// save panel object of the build
fn panel(e: &Engine) -> u32 {
    [0x2a10_0006, 0x2a10_0000, 0x2a10_0006, 0x2a10_0013, 0x2a10_0006][e.rt.cur_build()]
}

pub const pAnim: u32 = 0x10000;
pub const pExtra: u32 = 0x10004;
pub const pGameObj: u32 = 0x10008;
pub const pGraphics: u32 = 0x1000c;
pub const pInput: u32 = 0x10010;
pub const pSound: u32 = 0x10014;
pub const pPhysic: u32 = 0x10018;
pub const pAnimMngr: u32 = 0x1001c;
pub const pFunction: u32 = 0x10020;
pub const pMiscFunc: u32 = 0x10024;
pub const pDebug: u32 = 0x10028;
pub const pLocalDataCheckPoint: u32 = 0x1002c;
pub const pGlobalDataCheckPoint: u32 = 0x10030;
pub const bInputActive: u32 = 0x10034;

#[inline]
fn G(e: &Engine, a: u32) -> Ptr {
    e.rt.cur_data() + (a - 0x10000)
}
/// pLocalDataCheckPoint
#[inline]
fn L(e: &Engine) -> Ptr {
    e.r32(G(e, pLocalDataCheckPoint))
}
/// pGlobalDataCheckPoint: +0 current picture name, +4 game buffer 0, +8 game buffer 1, +0xc game buffer 2
#[inline]
fn GD(e: &Engine) -> Ptr {
    e.r32(G(e, pGlobalDataCheckPoint))
}
fn lr(e: &Engine, o: u32) -> u32 {
    e.r32(L(e) + o)
}
fn lw(e: &mut Engine, o: u32, v: u32) {
    let a = L(e) + o;
    e.w32(a, v);
}
/// `if (pDebug != 0) pDebug->PrintDebugInfo(a, b)`
fn dbg(e: &mut Engine, a: &str, b: &str) {
    if e.r32(G(e, pDebug)) != 0 {
        let (a, b) = (e.cstr(a), e.cstr(b));
        LIBdebug::PrintDebugInfo(e, a, b);
    }
}
fn timer_restart(e: &mut Engine, t: u32, f: f32) {
    APIextra::TimerReset(e, t);
    APIextra::TimerSetFrequency(e, t, f);
    APIextra::TimerRegisterNotify(e, SELF, t, 6);
}

/// 0x00010df8 state 0
pub fn ChkPntST_FIRSTMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    APIgameobj::SetGroupID(e, SELF, 7);
    LIBAnimMngr::PlayAnim(e, 0.0, 2, 1, 0);
    APIgameobj::SetContextFlag(e, SELF, 3);
    // _sTexFaderParams at L+0x2c
    let n = e.cstr("Scooby_Shaggy_Normal");
    for (o, v) in [(0x2c, 2), (0x34, n), (0x38, 0), (0x3c, 0), (0x40, n), (0x44, 0), (0x48, 0), (0x30, 0x3dcc_cccd), (0x4c, 0x3f80_0000), (0x50, 0), (0x54, 0), (0x58, 0x3f80_0000), (0xf0, 1)] {
        lw(e, o, v);
    }
    let t = lr(e, 0xb0);
    timer_restart(e, t, if e.rt.cur_build() == B_5E6EEA89 { 10.5 } else { 2.5 });
    APIgameobj::SetNextState(e, SELF, 1);
    Ok(0)
}

/// 0x00010fe8 state 1
pub fn ChkPntST_BASICMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

/// 0x00011018 state 2: the activation flash shrinks (size L+0xcc -0.75, colour L+0xd0 -0.0333333 per frame)
pub fn ChkPntST_FLASH(e: &mut Engine, _: u32) -> LResult<u64> {
    let l = L(e);
    if 0.0 <= e.rf(l + 0xcc) {
        if lr(e, 0xc4) != 0 {
            let (g, s) = (lr(e, 0xc8), lr(e, 0xc4));
            APIgraphics::SpriteGroupRemoveSprite(e, g, s);
        }
        let (g, sz) = (lr(e, 0xc8), e.rf(l + 0xcc));
        let s = APIgraphics::SpriteGroupAddSprite(e, g, SELF, ALL, sz);
        lw(e, 0xc4, s);
        let c = e.rf(l + 0xd0);
        APIgraphics::SpriteSetGroupColor(e, g, c, c, c, 0.0);
    } else {
        lw(e, 0xcc, 0x40e0_0000);
        lw(e, 0xd0, 0x3f80_0000);
        if lr(e, 0xc4) != 0 {
            let (g, s) = (lr(e, 0xc8), lr(e, 0xc4));
            APIgraphics::SpriteGroupRemoveSprite(e, g, s);
        }
        lw(e, 0xc4, 0);
        APIgameobj::SetNextState(e, SELF, 1);
    }
    let v = e.rf(l + 0xcc) - 0.75;
    e.wf(l + 0xcc, v);
    let v = e.rf(l + 0xd0) - 0.0333333;
    e.wf(l + 0xd0, v);
    Ok(0)
}

pub const ALL: u32 = 0x7fff_ffff;

/// 0x00011220 external 3: collision with an object (param = collision info, +8 = other object id)
pub fn ChkPntCB_NOTIFY_COLLIDE_OBJ(e: &mut Engine, param_1: u32) -> LResult<u64> {
    let gb0 = e.r32(GD(e) + 4);
    let other = e.r32(param_1 + 8);
    if e.rt.cur_build() == 0 && e.r32(gb0 + 8) == other && e.r32(gb0 + 0x40) & 0x20000 == 0 {
        APIgameobj::SendMessageTo(e, 0x2a00_0020, NULL, 0xa3, 0, NULL);
    } else if e.r32(gb0 + 8) == other && e.r32(gb0 + 0x14) & 0x6e20_090d == 0 && lr(e, 0xdc) != 0 && {
        let t = lr(e, 0xb4);
        1.0 <= APIextra::TimerGetTime(e, t)
    } && {
        // quirk: L+0xe4 is set as a side effect of the condition chain
        lw(e, 0xe4, 1);
        lr(e, 0xe8) == 0 && e.r32(e.r32(GD(e) + 0xc)) & 0x20000 == 0
    } && lr(e, 0xec) == 0
    {
        lw(e, 0xe8, 1);
        let c = L(e) + 0x60;
        LIBMiscFunc::MiscFunc_RemoveBitClipping(e, c, 0x80);
        APIgameobj::SetContextFlag(e, SELF, 7);
        let cfg = lr(e, 0xa4) as i32;
        APIinput::InputConfigActivate(e, cfg, 1);
        lw(e, 0xe0, 1);
        let sub = lr(e, 0xbc);
        let pn = panel(e);
        APIgameobj::SendMessageTo(e, pn, NULL, 0x31, sub, NULL);
    }
    Ok(0)
}

/// 0x00011498 external 4
pub fn ChkPntCB_POST_COLLISION(e: &mut Engine, _: u32) -> LResult<u64> {
    if lr(e, 0xe4) == 0 {
        dbg(e, "***CheckPoint", "CB_POST_COLLISION");
        let sub = lr(e, 0xbc);
        let pn = panel(e);
        APIgameobj::SendMessageTo(e, pn, NULL, 0x32, sub, NULL);
        let c = L(e) + 0x60;
        LIBMiscFunc::MiscFunc_ReplaceMyClipping(e, c);
        lw(e, 0xe8, 0);
        lw(e, 0xe0, 0);
        let a = G(e, bInputActive);
        e.w32(a, 0);
        let (cfg, on) = (lr(e, 0xa4) as i32, lr(e, 0xe0) as i32);
        APIinput::InputConfigActivate(e, cfg, on);
        APIgameobj::SetContextFlag(e, SELF, 3);
    }
    lw(e, 0xe4, 0);
    Ok(0)
}

/// 0x000115e8 external 5: X pressed at the checkpoint -> save panel msg 0x41 (0x42 after a failed save)
pub fn ChkPntCB_INPUT_X(e: &mut Engine, _: u32) -> LResult<u64> {
    let gb0 = e.r32(GD(e) + 4);
    if e.r32(gb0 + 0x14) & 0x6e20_090d == 0 || lr(e, 0xdc) == 0 {
        dbg(e, "***CheckPoint", "CB_INPUT_X");
        ChkPntCB_POST_COLLISION(e, 0)?;
        let id = lr(e, 0xa0);
        let msg = if lr(e, 0xbc) == 0xc { 0x42 } else { 0x41 };
        let pn = panel(e);
        APIgameobj::SendMessageTo(e, pn, NULL, msg, id, NULL);
        Ok(1)
    } else {
        Ok(0)
    }
}

/// 0x00011740 external 6
pub fn ChkPntCB_TIMER(e: &mut Engine, param_1: u32) -> LResult<u64> {
    if param_1 == lr(e, 0xb0) {
        if lr(e, 0xec) == 0 {
            lw(e, 0xdc, 1);
            APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 3, 3);
            SetClipping(e);
            return Ok(1);
        }
    }
    Ok(0)
}

/// 0x00011818
pub fn MyMessageFunction(e: &mut Engine, param_1: u32, param_2: u32, _data: Ptr) -> LResult<u64> {
    let l = L(e);
    // LAB_00011e60 (0x13 / 0x58) then LAB_00011f60
    let resume = |e: &mut Engine| {
        lw(e, 0xec, 0);
        if lr(e, 0xe0) != 0 {
            lw(e, 0xdc, 1);
            lw(e, 0xe0, 0);
            lw(e, 0xdc, 0);
            let t = lr(e, 0xb0);
            APIextra::TimerReset(e, t);
            APIextra::TimerSetFrequency(e, t, 0.5);
            APIextra::TimerRegisterNotify(e, SELF, t, 6);
            APIextra::TimerPause(e, t, 0);
            let t2 = lr(e, 0xb4);
            APIextra::TimerPause(e, t2, 0);
        }
    };
    match param_1 {
        0x4f | 0x52 => {}
        0x13 | 0x58 => resume(e),
        0x44 => {
            if param_2 != 0xb {
                return Ok(0);
            }
            lw(e, 0xbc, 0xc);
            e.w32(l + 0xdc, 0);
            let p = e.alloca(8);
            e.w32(p, 1);
            e.w32(p + 4, 6);
            APIgameobj::SendMessageTo(e, 0xff00_0008, p, 0xd4, 0, NULL);
            let id = lr(e, 0xa0);
            e.w32(l, id);
            APIgameobj::GetLookAt(e, SELF, ALL, l + 8);
            APIgameobj::GetLookUp(e, SELF, ALL, l + 0x14);
            APIgameobj::GetLookRight(e, SELF, ALL, l + 0x20);
            APInewSound::Play(e, SELF, 0, 0);
            ChkPntFUNC_ChangeMyPicture(e);
            LIBAnimMngr::PlayAnim(e, 0.0, 1, 2, 0);
            let t = lr(e, 0xb0);
            if e.rt.cur_build() == B_5E6EEA89 {
                APIextra::TimerReset(e, t);
                if lr(e, 0xf0) == 0 {
                    APIextra::TimerSetFrequency(e, t, 1.0);
                } else {
                    lw(e, 0xf0, 0);
                    APIextra::TimerSetFrequency(e, t, 10.5);
                }
                APIgameobj::SendMessageTo(e, 0x2a00_000e, NULL, 0xd4, 0, NULL);
                APIextra::TimerRegisterNotify(e, SELF, t, 6);
            } else {
                timer_restart(e, t, 1.0);
            }
            let gb0 = e.r32(GD(e) + 4);
            let id = lr(e, 0xa0);
            e.w32(gb0 + 0x28, id);
            e.w32(p, 1);
            e.w32(p + 4, 7);
            APIgameobj::SendMessageTo(e, 0xff00_0008, p, 0xa3, id, l);
            if lr(e, 0xd8) != 0 {
                let (g, s) = (lr(e, 0xd4), lr(e, 0xd8));
                APIgraphics::SpriteGroupRemoveSprite(e, g, s);
                lw(e, 0xd8, 0);
            }
            APIgameobj::SetNextState(e, SELF, 2);
            return Ok(0);
        }
        0x45 => {
            dbg(e, "***CheckPoint", "receiving MSG_MEMCARD_RETURN_FAILED");
            lw(e, 0xbc, param_2);
            lw(e, 0xdc, 0);
            let t = lr(e, 0xb0);
            timer_restart(e, t, 1.0);
            APIgameobj::SetNextState(e, SELF, 1);
            return Ok(0);
        }
        0xa3 => {
            dbg(e, "***CheckPoint", "receiving MSG_CHECKPOINT");
            if lr(e, 0xdc) != 0 {
                lw(e, 0xbc, 0xb);
                APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 3, 3);
                LIBAnimMngr::PlayAnim(e, 0.0, 2, 2, 0);
            }
            if param_2 == lr(e, 0xa0) || lr(e, 0xd8) != 0 {
                return Ok(0);
            }
            let g = lr(e, 0xd4);
            let s = APIgraphics::SpriteGroupAddSprite(e, g, SELF, ALL, 0.7);
            lw(e, 0xd8, s);
            return Ok(0);
        }
        0x12 | 0x57 => {
            let cfg = lr(e, 0xa4) as i32;
            APIinput::InputConfigActivate(e, cfg, 0);
            lw(e, 0xec, 1);
            if lr(e, 0xe0) != 0 {
                lw(e, 0xdc, 0);
                lw(e, 0xe4, 0);
                ChkPntCB_POST_COLLISION(e, 0)?;
                lw(e, 0xe0, 1);
                let (t, t2) = (lr(e, 0xb0), lr(e, 0xb4));
                APIextra::TimerPause(e, t, 1);
                APIextra::TimerPause(e, t2, 1);
            }
            return Ok(0);
        }
        _ => return Ok(0),
    }
    // LAB_00011f60
    let t2 = lr(e, 0xb4);
    APIextra::TimerReset(e, t2);
    Ok(0)
}

/// 0x00011fb8 EFClippingParams at L+0x60
pub fn SetClipping(e: &mut Engine) {
    let c = L(e) + 0x60;
    APIgameobj::GetClippingParams(e, SELF, c);
    for (o, v) in [(0, 0x3dcc_cccd), (4, 0x4160_0000), (0xc, 0), (8, 7), (0x10, 0x41c8_0000), (0x18, 0), (0x14, 0x6a7), (0x1c, 0xbf80_0000), (0x24, 0), (0x20, 0x6a7)] {
        e.w32(c + o, v);
    }
    APIgameobj::SetClippingParams(e, SELF, c, 0);
}

/// 0x000120e0 detection modules (10 = cone?, 9 = capsule around the stand, 0x25) + input config L+0xa4 (X)
pub fn ChkPntFUNC_Create_Physic(e: &mut Engine) {
    // stack frame: auStack_80 +0, uStack_70 +0x10, fStack_60 +0x20, DStack_30 +0x50
    let f = e.alloca(0x60);
    APIphysic::CreatePhysic(e, SELF, 0);
    APIphysic::SetFlags(e, SELF, 2);
    let m = APIphysic::DMAdd(e, SELF, 10);
    lw(e, 0xa8, m);
    APIphysic::DMGetParams(e, m, f + 0x10);
    for (o, v) in [(0x10, 0x4049_0fdb), (0x14, 0), (0x18, 0x3f80_0000), (0x1c, 0)] {
        e.w32(f + o, v);
    }
    APIphysic::DMSetParams(e, m, f + 0x10);
    APIphysic::DMSetContextMask(e, m, 1);
    APIgameobj::GetPosition(e, SELF, ALL, f + 0x50);
    let m = APIphysic::DMAdd(e, SELF, 9);
    lw(e, 0xac, m);
    APIphysic::DMGetParams(e, m, f + 0x20);
    let (x, y, z) = (e.rf(f + 0x50), e.rf(f + 0x54), e.rf(f + 0x58));
    e.wf(f + 0x20, x);
    e.wf(f + 0x24, y);
    e.wf(f + 0x28, z);
    e.wf(f + 0x2c, x);
    e.wf(f + 0x34, z);
    e.wf(f + 0x30, y + 0.09);
    e.w32(f + 0x38, 0x4000_0000);
    e.w32(f + 0x40, 0x4000_0000);
    e.w32(f + 0x44, 2);
    APIphysic::DMSetParams(e, m, f + 0x20);
    APIphysic::DMSetContextMask(e, m, 2);
    let m = APIphysic::DMAdd(e, SELF, 0x25);
    lw(e, 0xb8, m);
    APIphysic::DMGetParams(e, m, f);
    e.w32(f + 4, 4);
    APIphysic::DMSetParams(e, m, f);
    APIphysic::DMSetContextMask(e, m, 4);
    let cfg = lr(e, 0xa4) as i32;
    APIinput::InputConfigAdd(e, cfg);
    APIinput::InputActionAdd(e, cfg, 0x39, 0);
    let port = e.r32(e.r32(GD(e) + 4) + 0x4c) as i32;
    APIinput::InputDeviceAdd(e, cfg, 0x39, 0x3d, port, 0, 2, 0, 0);
    APIinput::InputDeviceAdd(e, cfg, 0x39, 0x3d, 1, 1, 2, 0, 0x39);
    APIinput::InputContextAdd(e, SELF, 5, cfg, 0x39, 0x1e, 8);
    APIinput::InputContextSetState(e, cfg, 0x39, 0x1e, 8);
    APIinput::InputConfigSetState(e, cfg, 8);
    APIinput::InputConfigActivate(e, cfg, 0);
    lw(e, 0xe0, 0);
}

/// 0x00012538 sprite group L+0xc8 "flash"
pub fn ChkPntFUNC_Create_Flash(e: &mut Engine) {
    lw(e, 0xcc, 0x40e0_0000);
    lw(e, 0xd0, 0x3f80_0000);
    let g = APIgraphics::SpriteAddGroup(e);
    lw(e, 0xc8, g);
    let t = e.cstr("flash");
    APIgraphics::SpriteSetGroupTexture(e, g, t);
    APIgraphics::SpriteSetGroupColor(e, g, 1.0, 1.0, 1.0, 0.0);
    APIgraphics::SpriteSetGroupBlendMode(e, g, 1);
    APIgraphics::SpriteSetGroupFlags(e, g, 8);
    lw(e, 0xc4, 0);
}

/// 0x00012668 sprite group L+0xd4 "glow", sprite L+0xd8
pub fn ChkPntFUNC_Create_Glow(e: &mut Engine) {
    let g = APIgraphics::SpriteAddGroup(e);
    lw(e, 0xd4, g);
    let t = e.cstr("glow");
    APIgraphics::SpriteSetGroupTexture(e, g, t);
    APIgraphics::SpriteSetGroupColor(e, g, 0.375, 0.0, 0.75, 0.1);
    APIgraphics::SpriteSetGroupBlendMode(e, g, 1);
    APIgraphics::SpriteSetGroupFlags(e, g, 0x14);
    let s = APIgraphics::SpriteGroupAddSprite(e, g, SELF, ALL, 0.7);
    lw(e, 0xd8, s);
}

/// 0x000127d8 picture by courage (game buffer 1 +0 float): <0.25 Scared, <0.75 Nervous, else Normal
pub fn ChkPntFUNC_ChangeMyPicture(e: &mut Engine) {
    let gd = GD(e);
    let cur = e.r32(gd);
    lw(e, 0x34, cur);
    let c = e.rf(e.r32(gd + 8));
    let n = if c < 0.25 {
        "Scooby_Shaggy_Scared"
    } else if c < 0.75 {
        "Scooby_Shaggy_Nervous"
    } else {
        "Scooby_Shaggy_Normal"
    };
    let n = e.cstr(n);
    lw(e, 0x40, n);
    e.w32(gd, n);
    let p = L(e) + 0x2c;
    APIgraphics::TexFaderPlay(e, SELF, p);
}

/// 0x000128f8
pub fn chkpntConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    if instanceCount == 1 {
        APIcons::SetExternalFunction(e, 0, ChkPntST_FIRSTMOVE);
        APIcons::SetExternalFunction(e, 1, ChkPntST_BASICMOVE);
        APIcons::SetExternalFunction(e, 2, ChkPntST_FLASH);
        APIcons::SetExternalFunction(e, 3, ChkPntCB_NOTIFY_COLLIDE_OBJ);
        APIcons::SetExternalFunction(e, 4, ChkPntCB_POST_COLLISION);
        APIcons::SetExternalFunction(e, 5, ChkPntCB_INPUT_X);
        APIcons::SetExternalFunction(e, 6, ChkPntCB_TIMER);
        let tables: [(u32, fn(&mut Engine) -> Ptr); 7] = [
            (pAnim, |e| APIcons::GetApiAnim(e)),
            (pExtra, |e| APIcons::GetApiExtra(e)),
            (pGameObj, |e| APIcons::GetApiGameObj(e)),
            (pGraphics, |e| APIcons::GetApiGraphics(e)),
            (pInput, |e| APIcons::GetApiInput(e)),
            (pPhysic, |e| APIcons::GetApiPhysic(e)),
            (pSound, |e| APIcons::GetApiNewSound(e)),
        ];
        for (slot, get) in tables {
            let v = get(e);
            let a = G(e, slot);
            e.w32(a, v);
        }
        for (slot, name) in [(pAnimMngr, "AnimMngr"), (pMiscFunc, "MiscFunc"), (pFunction, "ApiCFunctions"), (pDebug, "debug")] {
            let n = e.cstr(name);
            let v = APIcons::GetApiCustom(e, n);
            let a = G(e, slot);
            e.w32(a, v);
        }
        let slot = G(e, pGlobalDataCheckPoint);
        APIcons::GetGlobalData(e, slot, 0x10);
        for (o, (id, sz)) in [(4, (0, 0x8f8)), (8, (1, 0x68)), (0xc, (2, 0x158))] {
            let b = APIgameobj::GetGameBuffer(e, id, sz, 1);
            let gd = GD(e);
            e.w32(gd + o, b);
        }
        let n = e.cstr("Scooby_Shaggy_Normal");
        let gd = GD(e);
        e.w32(gd, n);
        let gb0 = e.r32(gd + 4);
        e.w32(gb0 + 0x28, 0);
    }
    APIcons::SetMessageFunction(e, MyMessageFunction);
    let slot = G(e, pLocalDataCheckPoint);
    APIcons::GetLocalData(e, slot, 0xf4);
    lw(e, 0xa4, (instanceCount + 0x3c) as u32);
    let id = APIgameobj::GetID(e, SELF, NULL);
    lw(e, 0xa0, id);
    ChkPntFUNC_Create_Physic(e);
    ChkPntFUNC_Create_Glow(e);
    ChkPntFUNC_Create_Flash(e);
    lw(e, 0xbc, 0xb);
    lw(e, 0xdc, 0);
    lw(e, 0xec, 0);
    APIgameobj::SetLogicFlag(e, SELF, 0x8000);
    let t = APIextra::TimerCreate(e);
    lw(e, 0xb0, t);
    let t = APIextra::TimerCreate(e);
    lw(e, 0xb4, t);
    LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 2, NULL, 0);
    APIcons::SetInitialState(e, 0);
    Ok(())
}

/// 0x00012d50
pub fn chkpntDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    if L(e) != 0 {
        let g = lr(e, 0xd4);
        if g != 0 {
            let s = lr(e, 0xd8);
            if s != 0 {
                APIgraphics::SpriteGroupRemoveSprite(e, g, s);
            }
            APIgraphics::SpriteRemoveGroup(e, g);
        }
        let g = lr(e, 0xc8);
        if g != 0 {
            APIgraphics::SpriteRemoveGroup(e, g);
        }
        let (t, t2) = (lr(e, 0xb0), lr(e, 0xb4));
        APIextra::TimerDestroy(e, t);
        APIextra::TimerDestroy(e, t2);
    }
    Ok(())
}

//! `hs_door_l01`: EP1L01 door hot spots (goCustomDataDoor: 21 doors 0x2a000027..3b). Touching a
//! door with the leader shows the "X" prompt (msg 0x31/9 to 0x2a100006); X enters it: locked doors
//! (GB2+0xc & mask clear) play 0x40000001 + text, unlocking ones set GB2+8 |= bits, a NULL level
//! sends 0x97 (ENTER_DOOR) to the leader, else camera msg 0x54 + 0x88 to both heroes and msg 0x4f
//! LoadLevel(EP1\L02). Local overrides: 0x2a00003a/2e also send 0xde to 0x2a000096; 0x2a00002a
//! sets GB2+0xc |= 0x1000400 on X, 0x2a00002d clears it on reset.
//! Port of `Game Logic Modules/Ep1_Library/Objects/HOTSPOTS/Level_01/hs_door_l01__b930f068/`
//! (hs_door_L01.c + hs_door_base.c, 17 functions). EP1L01 build b930f068.
#![allow(non_snake_case, non_upper_case_globals, unused_variables)]

use crate::engine::*;
use crate::logic::{LResult, ModuleDef};
pub(crate) use super::hs_door::Var;

pub static MODULE: ModuleDef = ModuleDef {
    name: "hs_door_l01",
    // globals 0x10000..0x1a8d0 (+ unused gpoLanguagesNamesAndISOCodes)
    data_size: 0xa8d0,
    ctor: hs_door_l01Constructor,
    dtor: hs_door_l01Destructor,
    register: None,
    init: &[],
    build_init: Some(build_init),
};

pub fn build_index(key: &str) -> Option<usize> {
    (key == "b930f068").then_some(0)
}

/// goCustomDataDoor (0x804-byte entries, string pointers relocated to interned C strings)
pub(crate) fn build_init_tab(m: &mut Mem, data: Ptr, words: &[(u32, u32)], strs: &[(u32, &str)]) {
    for &(a, w) in words {
        m.w32(data + (a - 0x10000), w);
    }
    for &(a, s) in strs {
        let p = m.cstr(s);
        m.w32(data + (a - 0x10000), p);
    }
}
fn build_init(m: &mut Mem, data: Ptr, _: usize) {
    for &(a, w) in WORDS {
        m.w32(data + (a - 0x10000), w);
    }
    for &(a, s) in STR_RELOCS {
        let p = m.cstr(s);
        m.w32(data + (a - 0x10000), p);
    }
}

pub const goCustomDataDoor: u32 = 0x10000;
pub const pGame: u32 = 0x1a854;
pub const pGameObj: u32 = 0x1a858;
pub const pInput: u32 = 0x1a85c;
pub const pPhysic: u32 = 0x1a860;
pub const pSound: u32 = 0x1a864;
pub const pDebug: u32 = 0x1a868;
pub const pText: u32 = 0x1a86c;
pub const pMiscFunc: u32 = 0x1a870;
/// function static of hs_door_baseCB_InputAction_X (camera params, 4 words)
pub const moCameraParams_27: u32 = 0x1a8b0;
pub const pExtra: u32 = 0x1a8c4;
pub const pLocalDataHSDoor: u32 = 0x1a8c8;
pub const pGlobalDataHSDoor: u32 = 0x1a8cc;

/// The shared hs_door_base.c is also linked by hs_door_l03 (EP1L03A) and by the 13 builds of
/// [`super::hs_door`]: they differ only in global layout / door count / ids / sounds / debug traces
/// ([`Var`]), picked from the running module (and build).
pub(crate) static L01: Var = Var { name: "hs_door_l01__b930f068", n: 0x15, radar: 0x2a10_0006, cam: 0x2a00_008f,
    snd_lock: 0x4000_0001, snd_open: 0x4000_0000, dbg: false, pgame: 0x1a854, cam27: 0x1a8b0, pextra: 0x1a8c4,
    pfunction: 0, plight: 0, size: 0xa8d0 };
pub(crate) static L03: Var = Var { name: "hs_door_l03__a80dc326", n: 0xb, radar: 0x2a10_0000, cam: 0x2a00_004a,
    snd_lock: 0x4000_0001, snd_open: 0x4000_0000, dbg: false, pgame: 0x1582c, cam27: 0x15888, pextra: 0x1589c,
    pfunction: 0, plight: 0, size: 0x58a8 };
pub(crate) fn var(e: &Engine) -> &'static Var {
    match e.rt.codes[e.rt.current_code().expect("no logic running")].def.name {
        "hs_door_l03" => &L03,
        "hs_door_l01" => &L01,
        _ => &super::hs_door::VARS[e.rt.cur_build()],
    }
}
/// module-relative address of a base global in the running build's layout
#[inline]
pub(crate) fn G(e: &Engine, a: u32) -> Ptr {
    let v = var(e);
    let o = if a >= pExtra {
        v.pextra + (a - pExtra)
    } else if a == moCameraParams_27 {
        // Var::cam27 holds DAT_first-4 (Ghidra's first DAT_ write); the static itself
        // (ELF symtab `moCameraParams.27`) starts 0x38 lower: the DAT_ writes are P[0xf..0x11].
        v.cam27 - 0x38
    } else if a >= pGame {
        v.pgame + (a - pGame)
    } else {
        a
    };
    e.rt.cur_data() + (o - 0x10000)
}
fn RADAR(e: &Engine) -> u32 {
    var(e).radar
}
/// `if (pDebug != NULL) pDebug->PrintDebugInfo("HS_Door", s)` (only in builds with the traces)
fn dbg(e: &mut Engine, s: &str) {
    if var(e).dbg && e.r32(G(e, pDebug)) != 0 {
        let (a, b) = (e.cstr("HS_Door"), e.cstr(s));
        LIBdebug::PrintDebugInfo(e, a, b);
    }
}
/// pLocalDataHSDoor (0x86c bytes): +0 dest obj, +4 sound, +8 lock mask, +0xc unlock bits, +0x10 text,
/// +0x14 no-input flag, +0x18/+0x1c level/pack names, +0x20 stleveltransitionparams, +0x7fc radar id,
/// +0x800 input config, +0x804 prompt shown, +0x80c loading, +0x810 0xd4 latch, +0x814 DM,
/// +0x818 touched, +0x81c clipping, +0x85c clip-refresh, +0x868 timer
#[inline]
pub(crate) fn L(e: &Engine) -> Ptr {
    e.r32(G(e, pLocalDataHSDoor))
}
#[inline]
pub(crate) fn lr(e: &Engine, o: u32) -> u32 {
    e.r32(L(e) + o)
}
#[inline]
pub(crate) fn lw(e: &mut Engine, o: u32, v: u32) {
    let a = L(e) + o;
    e.w32(a, v);
}
/// game buffer n (pGlobalDataHSDoor[n]: 0 = GB0, 2 = GB2)
#[inline]
pub(crate) fn GB(e: &Engine, n: u32) -> Ptr {
    e.r32(e.r32(G(e, pGlobalDataHSDoor)) + 4 * n)
}

fn printf(e: &mut Engine, s: &str) {
    let p = e.cstr(s);
    APIextra::null_printf(e, p, 0);
}

// ---- hs_door_L01.c

/// ext fn 4 (the base registers CollisionDetection as 2; slot 4 is only this one)
pub fn fcback_COLLISION_LOCAL(e: &mut Engine, p: u32) -> LResult<u64> {
    APIgameobj::SendMessageTo(e, 0x2a00_0020, NULL, 0x10e, 0, NULL);
    let id = APIgameobj::GetID(e, SELF, NULL);
    if id == 0x2a00_003a {
        APIgameobj::SendMessageTo(e, 0x2a00_0096, NULL, 0xde, 1, NULL);
    } else if APIgameobj::GetID(e, SELF, NULL) == 0x2a00_002e {
        APIgameobj::SendMessageTo(e, 0x2a00_0096, NULL, 0xde, 2, NULL);
    }
    hs_door_baseCB_CollisionDetection(e, p)
}

/// ext fn 6
pub fn hs_door_localCB_InputAction_X(e: &mut Engine, _: u32) -> LResult<u64> {
    if APIgameobj::GetID(e, SELF, NULL) == 0x2a00_002a {
        let a = GB(e, 2) + 0xc;
        let v = e.r32(a) | 0x400;
        e.w32(a, v);
        let v = e.r32(a) | 0x0100_0000;
        e.w32(a, v);
    }
    hs_door_baseCB_InputAction_X(e, 0)
}

pub fn hs_door_FUNC_MessageHandler(e: &mut Engine, m: u32, p: u32, d: Ptr) -> LResult<u64> {
    if (m == 0xc1 || m == 0xc9) && lr(e, 0x810) == 0 && APIgameobj::GetID(e, SELF, NULL) == 0x2a00_002d {
        let a = GB(e, 2) + 0xc;
        let v = e.r32(a) & 0xffff_fbff;
        e.w32(a, v);
        let v = e.r32(a) & 0xfeff_ffff;
        e.w32(a, v);
    }
    hs_door_baseFUNC_MessageHandler(e, m, p, d)
}

pub fn hs_door_l01Constructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    hs_door_baseConstructor(e, api, instanceCount);
    if instanceCount == 1 {
        APIcons::SetExternalFunction(e, 4, fcback_COLLISION_LOCAL);
        APIcons::SetExternalFunction(e, 6, hs_door_localCB_InputAction_X);
    }
    APIcons::SetMessageFunction(e, hs_door_FUNC_MessageHandler);
    APIcons::SetInitialState(e, 0);
    Ok(())
}

pub fn hs_door_l01Destructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    hs_door_baseDestructor(e);
    Ok(())
}

// ---- hs_door_base.c

pub fn CreatePhysic(e: &mut Engine) {
    APIphysic::CreatePhysic(e, SELF, 0);
    APIphysic::SetFlags(e, SELF, 2);
    APIphysic::SetWeight(e, SELF, 1.0);
    APIphysic::SetPriority(e, SELF, 100);
    APIphysic::PrimSetIntAttribBits(e, SELF, 0, 0, 4);
    let dm = APIphysic::DMAdd(e, SELF, 0x25);
    lw(e, 0x814, dm);
    let p = e.alloca(0x10); // auStack_30
    APIphysic::DMGetParams(e, dm, p);
    e.w32(p + 4, 3);
    APIphysic::DMSetParams(e, dm, p);
    APIphysic::DMSetContextMask(e, dm, 2);
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 3, 4);
}

pub fn CreateInput(e: &mut Engine) {
    if lr(e, 0x14) == 0 {
        let cfg = lr(e, 0x800) as i32;
        APIinput::InputConfigAdd(e, cfg);
        APIinput::InputActionAdd(e, cfg, 0x39, 0);
        let port = e.r32(GB(e, 0) + 0x4c) as i32;
        APIinput::InputDeviceAdd(e, cfg, 0x39, 0x3d, port, 0, 2, 0, 0);
        APIinput::InputDeviceAdd(e, cfg, 0x39, 0x3d, 1, 1, 2, 0, 0x39);
        APIinput::InputContextAdd(e, SELF, 6, cfg, 0x39, 0x1e, 8);
        APIinput::InputContextSetState(e, cfg, 0x39, 0x1e, 2);
        APIinput::InputConfigSetState(e, cfg, 2);
        APIinput::InputConfigActivate(e, cfg, 0);
        dbg(e, "InputConfigActivate - FALSE");
    }
}

pub fn CreateClipping(e: &mut Engine) {
    let leader = e.r32(GB(e, 0) + 8);
    LIBMiscFunc::MiscFunc_SetClipping(e, SELF, leader, 0);
    let c = e.alloca(0x40); // aEStack_70 (auStack_68 = c + 8)
    APIgameobj::GetClippingParams(e, SELF, c);
    for i in 0..5 {
        for o in [8 + i * 12 + 4, 8 + i * 12] {
            let v = e.r32(c + o) | 0x420;
            e.w32(c + o, v);
        }
    }
    APIgameobj::SetClippingParams(e, SELF, c, 1);
}

pub fn hs_door_baseST_FIRSTMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    let id = APIgameobj::GetID(e, SELF, NULL);
    for i in 0..var(e).n {
        let ent = G(e, goCustomDataDoor) + i * 0x804;
        if e.r32(ent) == id {
            let l = L(e);
            e.w32(l + 0x800, i + 400);
            for (lo, eo) in [(0, 4), (4, 8), (8, 0xc), (0xc, 0x10), (0x10, 0x14), (0x18, 0x1c), (0x1c, 0x20)] {
                let v = e.r32(ent + eo);
                e.w32(l + lo, v);
            }
            e.memcpy(l + 0x20, ent + 0x24, 0x7dc);
            let v = e.r32(ent + 0x18);
            e.w32(l + 0x14, v);
            let v = e.r32(ent + 0x800);
            e.w32(l + 0x7fc, v);
        }
    }
    CreatePhysic(e);
    CreateInput(e);
    CreateClipping(e);
    let gb2 = GB(e, 2);
    if e.r32(gb2 + 8) == 0 && lr(e, 0xc) & 1 != 0 && e.r32(gb2 + 0xc) & lr(e, 8) != 0 {
        let r = lr(e, 0x7fc);
        APIgameobj::SendMessageTo(e, RADAR(e), NULL, 10, r, NULL);
    }
    lw(e, 0x810, 0);
    APIgameobj::SetContextFlag(e, SELF, 0);
    lw(e, 0x85c, 1);
    APIgameobj::SetNextState(e, SELF, 1);
    Ok(0)
}

pub fn hs_door_baseST_BASICMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

/// ext fn 2 (param: notify payload, +8 = colliding object)
pub fn hs_door_baseCB_CollisionDetection(e: &mut Engine, p: u32) -> LResult<u64> {
    let gb0 = GB(e, 0);
    if e.r32(gb0 + 8) == e.r32(p + 8) && e.r32(gb0 + 0x14) & 0x0800_090d == 0 {
        if lr(e, 0x14) == 1 && e.r32(GB(e, 2)) & 0x20000 == 0 {
            hs_door_baseCB_InputAction_X(e, 0)?;
        } else {
            lw(e, 0x818, 1);
            if lr(e, 0x804) == 0 && e.r32(GB(e, 2)) & 0x20000 == 0 {
                dbg(e, "InputConfigActivate - TRUE");
                APIgameobj::SetContextFlag(e, SELF, 2);
                APIgameobj::SendMessageTo(e, RADAR(e), NULL, 0x31, 9, NULL);
                lw(e, 0x804, 1);
                let cfg = lr(e, 0x800) as i32;
                APIinput::InputConfigActivate(e, cfg, 1);
                let c = L(e) + 0x81c;
                LIBMiscFunc::MiscFunc_RemoveBitClipping(e, c, 0x80);
            }
        }
    }
    Ok(0)
}

pub fn ResetCollision(e: &mut Engine) {
    if e.r32(GB(e, 2)) & 0x20000 != 0 {
        APIgameobj::SendMessageTo(e, RADAR(e), NULL, 0x32, 9, NULL);
    }
    let c = L(e) + 0x81c;
    LIBMiscFunc::MiscFunc_ReplaceMyClipping(e, c);
    lw(e, 0x804, 0);
    let cfg = lr(e, 0x800) as i32;
    APIinput::InputConfigActivate(e, cfg, 0);
    APIgameobj::SetContextFlag(e, SELF, 0);
}

/// ext fn 3
pub fn hs_door_baseCB_CollisionDetectionOut(e: &mut Engine, _: u32) -> LResult<u64> {
    if lr(e, 0x804) != 0 && lr(e, 0x818) == 0 {
        ResetCollision(e);
    }
    lw(e, 0x818, 0);
    Ok(0)
}

/// ext fn 6
pub fn hs_door_baseCB_InputAction_X(e: &mut Engine, _: u32) -> LResult<u64> {
    let gb0 = GB(e, 0);
    if e.r32(gb0 + 0x14) & 0x0800_090d != 0 || e.r32(gb0 + 0x7c) == 1 {
        return Ok(0);
    }
    let f34 = e.r32(gb0 + 0x34);
    let leader = e.r32(gb0 + 8);
    if (!f34) & 0x110 == 0 && f34 & 0x1400 == 0 {
        APIgameobj::SendMessageTo(e, leader, NULL, 0x86, 1, NULL);
        return Ok(0);
    }
    APIgameobj::SendMessageTo(e, SELF, NULL, 0x214, 0, NULL);
    let gb2 = GB(e, 2);
    let (mask, bits) = (lr(e, 8), lr(e, 0xc));
    if mask == 0 || e.r32(gb2 + 8) & bits != 0 {
        let s = lr(e, 4);
        APInewSound::Play(e, SELF, s, 0);
    } else {
        if e.r32(gb2 + 0xc) & mask == 0 || mask == 0xffff_ffff {
            dbg(e, "Door is Locked - access denied");
            let t = lr(e, 0x10);
            if t != 0 && LIBApiText::IsTextDisplayed(e) == 0 {
                let s = e.alloca(0x40); // auStack_80
                LIBApiText::InitStruct(e, s);
                e.w32(s, 0x4060_0000);
                LIBApiText::ShowText(e, t, s);
            }
            let snd = var(e).snd_lock;
            APInewSound::Play(e, SELF, snd, 0);
            return Ok(0);
        }
        let snd = var(e).snd_open;
        APInewSound::Play(e, SELF, snd, 0);
        let v = e.r32(gb2 + 8) | bits;
        e.w32(gb2 + 8, v);
        let r = lr(e, 0x7fc);
        APIgameobj::SendMessageTo(e, RADAR(e), NULL, 0xb, r, NULL);
        APIgameobj::SetGroupID(e, SELF, 6);
    }
    let a = APIphysic::PrimGetIntAttrib(e, SELF, 0, 9);
    if a & 0x8000_0000 != 0 {
        e.w32(gb0 + 0x88, 1);
        printf(e, " HotSpot_base> *************************************** \n");
        printf(e, " HotSpot_base> >>>>> bLoadAtTheEndOfTheLevel = TRUE \n");
        printf(e, " HotSpot_base> *************************************** \n");
    }
    let cfg = lr(e, 0x800) as i32;
    APIinput::InputConfigActivate(e, cfg, 0);
    dbg(e, "InputConfigActivate - FALSE");
    // `(char *)L[6] == "NULL"`: pointer compare with the module's pooled literal (interned cstr)
    let null = e.cstr("NULL");
    if lr(e, 0x18) == null {
        let dst = lr(e, 0);
        APIgameobj::SendMessageTo(e, leader, NULL, 0x97, dst, NULL);
        printf(e, " HotSpot_base> ENTER_DOOR ..... \n");
    } else {
        dbg(e, "Invalid command - please correct");
        lw(e, 0x80c, 1);
        let cp = G(e, moCameraParams_27);
        // P[0]=0x62, P[0xf]=0 (no context), P[0x10]=1 (Camera_BaseFUNC_FadeInOut(1.0,0) -> timer
        // sends 0x4f -> LoadLevel), P[0x11]=0 (DAT_+0x3c/+0x40/+0x44 of moCameraParams.27)
        for (o, v) in [(0, 0x62), (0x3c, 0), (0x40, 1), (0x44, 0)] {
            e.w32(cp + o, v);
        }
        let cam = var(e).cam;
        APIgameobj::SendMessageTo(e, cam, NULL, 0x54, 0, cp);
        APIgameobj::SendMessageTo(e, RADAR(e), NULL, 0x1e, 0, NULL);
        APIgameobj::SendMessageTo(e, leader, NULL, 0x88, 0, NULL);
        let other = e.r32(gb0 + 0xc);
        APIgameobj::SendMessageTo(e, other, NULL, 0x88, 0, NULL);
        printf(e, " HotSpot_base> Loading ..... \n");
    }
    Ok(1)
}

pub fn hs_door_baseFUNC_MessageHandler(e: &mut Engine, m: u32, p: u32, _d: Ptr) -> LResult<u64> {
    match m {
        0x4f => {
            if lr(e, 0x80c) != 0 {
                let (a, b, c) = (lr(e, 0x18), lr(e, 0x1c), L(e) + 0x20);
                APIgame::LoadLevel(e, a, b, c);
            }
        }
        0x12 | 0x4c => {
            ResetCollision(e);
            APIphysic::SetFlags(e, SELF, 1);
        }
        0x13 | 0x51 => {
            APIphysic::ResetFlags(e, SELF, 1);
        }
        0xc1 => {
            if lr(e, 0x810) != 0 {
                return Ok(0);
            }
            lw(e, 0x810, 0);
            APIgameobj::SetGroupID(e, SELF, 0);
            let a = GB(e, 2) + 8;
            let v = e.r32(a) & !lr(e, 0xc);
            e.w32(a, v);
            let t = lr(e, 0x868);
            APIextra::TimerReset(e, t);
            APIextra::TimerSetFrequency(e, t, 1.0);
            APIextra::TimerRegisterNotify(e, SELF, t, 5);
            ResetCollision(e);
            APIphysic::ResetFlags(e, SELF, 1);
            APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 3, 4);
        }
        0xcd => {
            if lr(e, 0x85c) == 0 {
                return Ok(0);
            }
            CreateClipping(e);
            if lr(e, 0x804) != 0 {
                let c = L(e) + 0x81c;
                LIBMiscFunc::MiscFunc_RemoveBitClipping(e, c, 0x80);
            }
        }
        0xd4 => {
            APIgameobj::SetGroupID(e, SELF, 0);
            lw(e, 0x810, 1);
        }
        _ => {}
    }
    Ok(0)
}

/// ext fn 5
pub fn hs_door_baseCB_TIMER(e: &mut Engine, _: u32) -> LResult<u64> {
    if e.r32(GB(e, 2) + 0xc) & lr(e, 8) != 0 {
        let r = lr(e, 0x7fc);
        APIgameobj::SendMessageTo(e, RADAR(e), NULL, 10, r, NULL);
    }
    Ok(1)
}

pub fn hs_door_baseConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) {
    if instanceCount == 1 {
        APIcons::SetExternalFunction(e, 0, hs_door_baseST_FIRSTMOVE);
        APIcons::SetExternalFunction(e, 1, hs_door_baseST_BASICMOVE);
        APIcons::SetExternalFunction(e, 2, hs_door_baseCB_CollisionDetection);
        APIcons::SetExternalFunction(e, 3, hs_door_baseCB_CollisionDetectionOut);
        APIcons::SetExternalFunction(e, 6, hs_door_baseCB_InputAction_X);
        APIcons::SetExternalFunction(e, 5, hs_door_baseCB_TIMER);
        let tables: [(u32, fn(&mut Engine) -> Ptr); 6] = [
            (pExtra, |e| APIcons::GetApiExtra(e)),
            (pGame, |e| APIcons::GetApiGame(e)),
            (pGameObj, |e| APIcons::GetApiGameObj(e)),
            (pInput, |e| APIcons::GetApiInput(e)),
            (pPhysic, |e| APIcons::GetApiPhysic(e)),
            (pSound, |e| APIcons::GetApiNewSound(e)),
        ];
        for (slot, get) in tables {
            let v = get(e);
            let a = G(e, slot);
            e.w32(a, v);
        }
        for (slot, name) in [(pDebug, "debug"), (pText, "ApiText"), (pMiscFunc, "MiscFunc")] {
            let n = e.cstr(name);
            let v = APIcons::GetApiCustom(e, n);
            let a = G(e, slot);
            e.w32(a, v);
        }
        let slot = G(e, pGlobalDataHSDoor);
        APIcons::GetGlobalData(e, slot, 0xc);
        let gd = e.r32(slot);
        let b = APIgameobj::GetGameBuffer(e, 0, 0x8f8, 1);
        e.w32(gd, b);
        let b = APIgameobj::GetGameBuffer(e, 2, 0x158, 1);
        e.w32(gd + 8, b);
    }
    let slot = G(e, pLocalDataHSDoor);
    APIcons::GetLocalData(e, slot, 0x86c);
    APIcons::SetMessageFunction(e, hs_door_baseFUNC_MessageHandler);
    let t = APIextra::TimerCreate(e);
    lw(e, 0x868, t);
    LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 2, NULL, 1);
}

pub fn hs_door_baseDestructor(e: &mut Engine) {
    let t = lr(e, 0x868);
    APIextra::TimerDestroy(e, t);
}

const WORDS: &[(u32, u32)] = &[(0x10000, 0x2a000027), (0x10004, 0x2a00004b), (0x10008, 0x40000002), (0x10804, 0x2a000028), (0x10808, 0x2a00003d), (0x1080c, 0x40000002), (0x11008, 0x2a000029), (0x1100c, 0x2a00003e), (0x11010, 0x40000002), (0x1180c, 0x2a00002a), (0x11810, 0x2a00003f), (0x11814, 0x40000002), (0x12010, 0x2a00002b), (0x12014, 0x2a000040), (0x12018, 0x40000002), (0x12814, 0x2a00002c), (0x12818, 0x2a000041), (0x1281c, 0x40000002), (0x13018, 0x2a00002d), (0x1301c, 0x2a000042), (0x13020, 0x40000002), (0x13024, 0x1000000), (0x13028, 0x1), (0x1302c, 0x15f), (0x1381c, 0x2a00002e), (0x13820, 0x2a000043), (0x13824, 0x40000002), (0x13828, 0x800), (0x1382c, 0x80), (0x13830, 0x163), (0x1401c, 0x2), (0x14020, 0x2a00002f), (0x14024, 0x2a000044), (0x14028, 0x40000002), (0x14824, 0x2a000030), (0x14828, 0x2a000045), (0x1482c, 0x40000002), (0x15028, 0x2a000031), (0x1502c, 0x2a000046), (0x15030, 0x40000002), (0x1582c, 0x2a000032), (0x15830, 0x2a000047), (0x15834, 0x40000002), (0x16030, 0x2a000033), (0x16034, 0x2a000048), (0x16038, 0x40000002), (0x16834, 0x2a000034), (0x16838, 0x2a00004a), (0x1683c, 0x40000002), (0x16858, 0x2), (0x17038, 0x2a000036), (0x1703c, 0x2a000049), (0x17040, 0x40000002), (0x1783c, 0x2a000035), (0x17848, 0xffffffff), (0x17850, 0x161), (0x18040, 0x2a000037), (0x1804c, 0xffffffff), (0x18054, 0x15f), (0x18844, 0x2a000038), (0x18850, 0xffffffff), (0x18858, 0x161), (0x19048, 0x2a000039), (0x19054, 0xffffffff), (0x1905c, 0x160), (0x1984c, 0x2a00003a), (0x19870, 0x2), (0x1a050, 0x2a00003b), (0x1a05c, 0xffffffff), (0x1a064, 0x161)];
const STR_RELOCS: &[(u32, &str)] = &[(0x1001c, "NULL"), (0x10020, "NULL"), (0x10028, ""), (0x10820, "NULL"), (0x10824, "NULL"), (0x1082c, ""), (0x11024, "NULL"), (0x11028, "NULL"), (0x11030, ""), (0x11828, "NULL"), (0x1182c, "NULL"), (0x11834, ""), (0x1202c, "NULL"), (0x12030, "NULL"), (0x12038, ""), (0x12830, "NULL"), (0x12834, "NULL"), (0x1283c, ""), (0x13034, "NULL"), (0x13038, "NULL"), (0x13040, ""), (0x13838, "NULL"), (0x1383c, "NULL"), (0x13844, ""), (0x1403c, "NULL"), (0x14040, "NULL"), (0x14048, ""), (0x14840, "NULL"), (0x14844, "NULL"), (0x1484c, ""), (0x15044, "NULL"), (0x15048, "NULL"), (0x15050, ""), (0x15848, "NULL"), (0x1584c, "NULL"), (0x15854, ""), (0x1604c, "NULL"), (0x16050, "NULL"), (0x16058, ""), (0x16850, "EP1\\L02"), (0x16854, "EP1L02"), (0x1685c, "\\MOVIES\\EP01\\EP01C"), (0x17054, "NULL"), (0x17058, "NULL"), (0x17060, ""), (0x17858, "NULL"), (0x1785c, "NULL"), (0x17864, ""), (0x1805c, "NULL"), (0x18060, "NULL"), (0x18068, ""), (0x18860, "NULL"), (0x18864, "NULL"), (0x1886c, ""), (0x19064, "NULL"), (0x19068, "NULL"), (0x19070, ""), (0x19868, "EP1\\L02"), (0x1986c, "EP1L02"), (0x19874, "\\MOVIES\\EP01\\EP01C"), (0x1a06c, "NULL"), (0x1a070, "NULL"), (0x1a078, "")];

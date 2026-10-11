//! `falling_key`: the key that falls into place (Ep2 MovieStudio). 0xc1 hangs it (child of a holder,
//! glow), 0xe0 drops it: it jumps onto the destination object; near it (<= 1.5) it hides itself and
//! shows/enables the real key object at that spot, plays the leader's sound and stops. Port of
//! `Game Logic Modules/Ep2_MovieStudio/Objects/FALLING_KEY/falling_key__*/Falling_Key.c`. The 3 builds
//! differ only by object ids and two small code differences (see [`BUILDS`]).
#![allow(non_snake_case, non_upper_case_globals, unused_variables)]

use crate::engine::*;
use crate::logic::{LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "falling_key",
    // globals 0x10000..0x10044 (+ unused gpoLanguagesNamesAndISOCodes)
    data_size: 0x44,
    ctor: falling_keyConstructor,
    dtor: falling_keyDestructor,
    register: None,
    init: &[],
    build_init: None,
};

const KEYS: [&str; 3] = ["8fa4bb5e", "2c385321", "2c5ce370"];

pub fn build_index(key: &str) -> Option<usize> {
    KEYS.iter().position(|k| *k == key)
}

/// per-build constants of Falling_Key.c
struct Build {
    /// destination object (jump target, distance check)
    dest: u32,
    /// the real key object shown at the destination
    key: u32,
    /// holder the key hangs on (0xc1)
    holder: u32,
    /// leaders: (id, sound) checked in order
    snd: [(u32, u32); 2],
    /// message 0x51 handled (8fa4bb5e only)
    msg51: bool,
    /// ST_BEENTHROWN_LOOP moves itself to (25, 0, 35) after DestroyGlow (2c385321 only)
    park: bool,
}

const BUILDS: [Build; 3] = [
    Build { dest: 0x2a00_0055, key: 0x2a00_0094, holder: 0x2a00_016b, snd: [(0x2a00_004d, 0x4000_000d), (0x2a00_004c, 0x4000_000c)], msg51: true, park: false },
    Build { dest: 0x2a00_00f9, key: 0x2a00_0049, holder: 0x2a00_00f4, snd: [(0x2a00_0033, 0x4000_000d), (0x2a00_0032, 0x4000_000c)], msg51: false, park: true },
    Build { dest: 0x2a00_00f1, key: 0x2a00_0049, holder: 0x2a00_00ea, snd: [(0x2a00_002c, 0x4000_000d), (0x2a00_002b, 0x4000_000c)], msg51: false, park: false },
];

fn B(e: &Engine) -> &'static Build {
    &BUILDS[e.rt.cur_build()]
}

pub const pExtra: u32 = 0x10000;
pub const pGameObj: u32 = 0x10004;
pub const pPhysic: u32 = 0x10008;
pub const pAnim: u32 = 0x1000c;
pub const pInput: u32 = 0x10010;
pub const pGame: u32 = 0x10014;
pub const pMath: u32 = 0x10018;
pub const pParticle: u32 = 0x1001c;
pub const pAnimMngr: u32 = 0x10020;
pub const pText: u32 = 0x10024;
pub const pDebug: u32 = 0x10028;
pub const pFunction: u32 = 0x1002c;
pub const pGraphic: u32 = 0x10030;
pub const pMiscFunc: u32 = 0x10034;
pub const pSound: u32 = 0x10038;
pub const pLocalDataFallingKey: u32 = 0x1003c;
pub const pGlobalDataFallingKey: u32 = 0x10040;

#[inline]
fn G(e: &Engine, a: u32) -> Ptr {
    e.rt.cur_data() + (a - 0x10000)
}
/// local data (0x20): +0 DM gravity, +4 DM kind 4, +8 DM kind 7, +0xc DM kind 10, +0x10 motion
/// trail (-2 = none), +0x14 sprite, +0x18 sprite group (-2 = none), +0x1c dropped flag
fn L(e: &Engine) -> Ptr {
    e.r32(G(e, pLocalDataFallingKey))
}
/// global data (8): game buffers 0 and 1
fn GD(e: &Engine) -> Ptr {
    e.r32(G(e, pGlobalDataFallingKey))
}

/// FallingKey_ST_FIRSTMOVE @0x00010d40
pub fn FallingKey_ST_FIRSTMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    let l = L(e);
    e.w32(l + 0x1c, 0);
    APIgameobj::SetNextState(e, SELF, 2);
    Ok(0)
}
/// FallingKey_ST_BASICMOVE_LOOP @0x00010da0
pub fn FallingKey_ST_BASICMOVE_LOOP(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}
/// FallingKey_ST_BEENTHROWN_LOOP @0x00010dd0
pub fn FallingKey_ST_BEENTHROWN_LOOP(e: &mut Engine, _: u32) -> LResult<u64> {
    let b = B(e);
    let park = e.alloca(0x10);
    e.wf(park, 25.0);
    e.wf(park + 4, 0.0);
    e.wf(park + 8, 35.0);
    let me = e.alloca(0x10);
    let d = e.alloca(0x10);
    APIgameobj::GetPosition(e, SELF, 0x7fff_ffff, me);
    if FallingKey_FUNC_IsNearDestination(e) {
        APIgameobj::GetPosition(e, b.dest, 0x7fff_ffff, d);
        APIgameobj::SendMessageTo(e, b.key, NULL, 0xc1, 0, NULL);
        APIgameobj::SetTranslation(e, b.key, d);
        APIgameobj::SendMessageTo(e, b.key, NULL, 0xd5, 0, NULL);
        APIphysic::SetVelocity(e, SELF, 0.0, 0.0, 0.0);
        APIgameobj::SetGameObjectVisible(e, SELF, 0);
        APIgameobj::SetGameObjectVisible(e, b.key, 1);
        APIgameobj::EnableObject(e, b.key, 1);
        Falling_Key_FUNC_DestroyGlow(e);
        if b.park {
            APIgameobj::SetTranslation(e, SELF, park);
        }
        APIgameobj::SetContextFlag(e, SELF, 8);
        APIphysic::SetVelocity(e, SELF, 0.0, 0.0, 0.0);
        Falling_Key_FUNC_DestroyMotionTrail(e);
        let gb = e.r32(GD(e));
        let leader = e.r32(gb + 8);
        if leader == b.snd[0].0 {
            APInewSound::Play(e, SELF, b.snd[0].1, 0);
        } else if leader == b.snd[1].0 {
            APInewSound::Play(e, SELF, b.snd[1].1, 0);
        }
        APIgameobj::SetNextState(e, SELF, 2);
    }
    Ok(0)
}
/// FallingKey_CB_NOTIFY_COLLIDE_OBJ @0x000110a0
pub fn FallingKey_CB_NOTIFY_COLLIDE_OBJ(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}
/// FallingKey_FUNC_IsNearDestination @0x000110d0
fn FallingKey_FUNC_IsNearDestination(e: &mut Engine) -> bool {
    let d = B(e).dest;
    let f = LIBApiCFunctions::GetDistance(e, SELF as i32, d, 0x7fff_ffff, 1);
    f <= 1.5
}
/// Falling_Key_FUNC_CreateGlow @0x00011158
fn Falling_Key_FUNC_CreateGlow(e: &mut Engine) {
    let l = L(e);
    if e.r32(l + 0x18) as i32 == -2 {
        let g = APIgraphics::SpriteAddGroup(e);
        e.w32(l + 0x18, g);
        let s = e.cstr("glow");
        APIgraphics::SpriteSetGroupTexture(e, g, s);
        APIgraphics::SpriteSetGroupBlendMode(e, g, 1);
        APIgraphics::SpriteSetGroupFlags(e, g, 0x14);
        APIgraphics::SpriteSetGroupColor(e, g, 0.375, 0.0, 0.75, 0.1);
        let sp = APIgraphics::SpriteGroupAddSprite(e, g, SELF, 0x7fff_ffff, 0.7);
        e.w32(l + 0x14, sp);
    }
}
/// Falling_Key_FUNC_DestroyGlow @0x000112d8
fn Falling_Key_FUNC_DestroyGlow(e: &mut Engine) {
    let l = L(e);
    let g = e.r32(l + 0x18);
    if g as i32 != -2 {
        let sp = e.r32(l + 0x14);
        APIgraphics::SpriteGroupRemoveSprite(e, g, sp);
        APIgraphics::SpriteRemoveGroup(e, g);
        e.w32(l + 0x18, 0xffff_fffe);
    }
}
/// FallingKey_FUNC_Create_Physic @0x00011380
fn FallingKey_FUNC_Create_Physic(e: &mut Engine) {
    let l = L(e);
    let p = e.alloca(0x20);
    APIphysic::CreatePhysic(e, SELF, 0);
    APIphysic::SetFlags(e, SELF, 2);
    // (local slot, DM kind, written params, context mask)
    let dms: [(u32, u32, &[(u32, u32)], u32); 4] = [
        (8, 7, &[(0, 0), (4, 5)], 8),
        (4, 4, &[(0, 0), (4, 0), (8, 0), (0xc, 1)], 1),
        (0, 0, &[(0, 0), (4, 0xc1a0_0000), (8, 0)], 4),
        (0xc, 10, &[(0, 0x40c9_0fdb), (4, 0), (8, 0x3f80_0000), (0xc, 0)], 0x20),
    ];
    for (slot, kind, prm, mask) in dms {
        let h = APIphysic::DMAdd(e, SELF, kind);
        e.w32(l + slot, h);
        APIphysic::DMGetParams(e, h, p);
        for &(o, v) in prm {
            e.w32(p + o, v);
        }
        APIphysic::DMSetParams(e, h, p);
        APIphysic::DMSetContextMask(e, h, mask);
    }
}
/// Falling_Key_FUNC_CreateMotionTrail @0x00011688
fn Falling_Key_FUNC_CreateMotionTrail(e: &mut Engine) {
    let l = L(e);
    if e.r32(l + 0x10) as i32 == -2 {
        let m = APIgraphics::MTCreateSystem(e, 2, 0x19, 0);
        e.w32(l + 0x10, m);
        APIgraphics::MTSetFrequency(e, m, 0.1);
        for (i, j) in [(0, 2), (1, 3)] {
            APIgraphics::MTSetJoint(e, m, i, SELF, j);
            APIgraphics::MTSetRGB(e, m, i, 0.518, 0.158, 0.815);
            APIgraphics::MTSetLifeSpan(e, m, i, 0.1);
        }
        APIgraphics::MTStart(e, m);
    }
}
/// Falling_Key_FUNC_DestroyMotionTrail @0x00011890
fn Falling_Key_FUNC_DestroyMotionTrail(e: &mut Engine) {
    let l = L(e);
    let m = e.r32(l + 0x10);
    if m as i32 != -2 {
        APIgraphics::MTStopDraw(e, m);
        APIgraphics::MTStopTracking(e, m);
        APIgraphics::MTDestroySystem(e, m);
        e.w32(l + 0x10, 0xffff_fffe);
    }
}
/// FallingKey_FUNC_MyMessageFunction @0x00011948
pub fn FallingKey_FUNC_MyMessageFunction(e: &mut Engine, m: u32, _: u32, _: Ptr) -> LResult<u64> {
    let b = B(e);
    match m {
        0xd4 | 0x14c => {
            Falling_Key_FUNC_CreateGlow(e);
            APIgameobj::KillObject(e, SELF);
        }
        0xc1 => {
            let v = e.alloca(0x10);
            e.wf(v, 0.0);
            e.wf(v + 4, 3.4);
            e.wf(v + 8, 35.0);
            APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 3, 4);
            APIgameobj::SetContextFlag(e, SELF, 0x20);
            APIgameobj::SetTranslation(e, b.key, v);
            APIgameobj::AddChild(e, b.holder, 3, SELF, 1);
            APIphysic::PrimSetIntAttribBits(e, SELF, 0, 2, 2);
            Falling_Key_FUNC_CreateGlow(e);
        }
        0xc9 => {
            APIgameobj::SetNextState(e, SELF, 0);
            let l = L(e);
            e.w32(l + 0x1c, 0);
        }
        0x51 if b.msg51 => {
            let l = L(e);
            if e.r32(l + 0x1c) != 0 {
                let v = e.alloca(0x10);
                e.wf(v, 25.0);
                e.wf(v + 4, 0.0);
                e.wf(v + 8, 35.0);
                APIgameobj::SetTranslation(e, SELF, v);
            }
        }
        0x165 => Falling_Key_FUNC_CreateGlow(e),
        0xe0 => {
            APIgameobj::SetContextFlag(e, SELF, 0x2d);
            APIgameobj::LeaveParent(e, SELF, 0);
            let l = L(e);
            e.w32(l + 0x1c, 1);
            let pos = e.alloca(0x10);
            APIgameobj::GetPosition(e, b.dest, 0x7fff_ffff, pos);
            // JumpOnCoorParam
            let j = e.alloca(0x30);
            for i in 0..3 {
                let w = e.r32(pos + 4 * i);
                e.w32(j + 4 * i, w);
            }
            let dm = e.r32(l);
            for (o, v) in [(0xc, 0), (0x10, 0), (0x14, 0), (0x18, 0x41f0_0000), (0x1c, 2), (0x20, 0x4120_0000), (0x24, dm)] {
                e.w32(j + o, v);
            }
            APIphysic::JumpOnCoor(e, SELF, j);
            Falling_Key_FUNC_CreateMotionTrail(e);
            APIphysic::ResetFlags(e, SELF, 1);
            APIgameobj::SetNextState(e, SELF, 3);
        }
        _ => {}
    }
    Ok(0)
}
/// falling_keyConstructor @0x00011d30
pub fn falling_keyConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    if instanceCount == 1 {
        APIcons::SetExternalFunction(e, 0, FallingKey_ST_FIRSTMOVE);
        APIcons::SetExternalFunction(e, 2, FallingKey_ST_BASICMOVE_LOOP);
        APIcons::SetExternalFunction(e, 3, FallingKey_ST_BEENTHROWN_LOOP);
        APIcons::SetExternalFunction(e, 4, FallingKey_CB_NOTIFY_COLLIDE_OBJ);
        let custom = |e: &mut Engine, n: &str| {
            let s = e.cstr(n);
            APIcons::GetApiCustom(e, s)
        };
        let v = APIcons::GetApiAnim(e);
        e.w32(G(e, pAnim), v);
        let v = APIcons::GetApiGameObj(e);
        e.w32(G(e, pGameObj), v);
        let v = APIcons::GetApiExtra(e);
        e.w32(G(e, pExtra), v);
        let v = APIcons::GetApiPhysic(e);
        e.w32(G(e, pPhysic), v);
        let v = APIcons::GetApiInput(e);
        e.w32(G(e, pInput), v);
        let v = APIcons::GetApiGame(e);
        e.w32(G(e, pGame), v);
        let v = custom(e, "debug");
        e.w32(G(e, pDebug), v);
        let v = custom(e, "AnimMngr");
        e.w32(G(e, pAnimMngr), v);
        let v = custom(e, "ApiText");
        e.w32(G(e, pText), v);
        let v = APIcons::GetApiMath(e);
        e.w32(G(e, pMath), v);
        let v = APIcons::GetApiParticle(e);
        e.w32(G(e, pParticle), v);
        let v = custom(e, "ApiCFunctions");
        e.w32(G(e, pFunction), v);
        let v = APIcons::GetApiGraphics(e);
        e.w32(G(e, pGraphic), v);
        let v = custom(e, "MiscFunc");
        e.w32(G(e, pMiscFunc), v);
        let v = APIcons::GetApiNewSound(e);
        e.w32(G(e, pSound), v);
        let slot = G(e, pGlobalDataFallingKey);
        APIcons::GetGlobalData(e, slot, 8);
        let b0 = APIgameobj::GetGameBuffer(e, 0, 0x8f8, 1);
        let g = GD(e);
        e.w32(g, b0);
        let b1 = APIgameobj::GetGameBuffer(e, 1, 0x68, 1);
        let g = GD(e);
        e.w32(g + 4, b1);
    }
    APIcons::SetMessageFunction(e, FallingKey_FUNC_MyMessageFunction);
    LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 2, NULL, 0);
    let slot = G(e, pLocalDataFallingKey);
    APIcons::GetLocalData(e, slot, 0x20);
    FallingKey_FUNC_Create_Physic(e);
    let l = L(e);
    e.w32(l + 0x10, 0xffff_fffe);
    e.w32(l + 0x18, 0xffff_fffe);
    APIcons::SetInitialState(e, 0);
    Ok(())
}
/// falling_keyDestructor @0x00012090
pub fn falling_keyDestructor(e: &mut Engine, _: i32) -> LResult<()> {
    Falling_Key_FUNC_DestroyGlow(e);
    Falling_Key_FUNC_DestroyMotionTrail(e);
    Ok(())
}

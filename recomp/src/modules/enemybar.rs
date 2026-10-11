//! `enemybar`: the monster life bar shown above a hooked monster while the book sucks it. The bar is a
//! child of the monster (parent found through retriever 0xff000020), follows the camera (TrackID
//! 0x2a00008f) and its fill = blend weight current/max life from msg 0xec {max, current}; killed by
//! msg 0xd6/100 (book link broken).
//! Port of `Game Logic Modules/Globals/Panel/ENEMYBAR/enemybar__530d0457/EnemyBar.c` (7 functions).
//! 26 builds (see [`BUILDS`]).
#![allow(non_snake_case, non_upper_case_globals, unused_variables)]

use crate::engine::*;
use crate::logic::{LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "enemybar",
    data_size: 0x28,
    ctor: enemybarConstructor,
    dtor: enemybarDestructor,
    register: None,
    init: &[],
    build_init: None,
};

/// (build sha8, camera object, FIRSTMOVE anim variant: 0 = PlayAnim 2 + blend (1,2), 1 = PlayAnim 1 + blend (2,1)).
/// All 26 builds share EnemyBar.c; only these constants differ (WP-L8).
const BUILDS: [(&str, u32, u32); 26] = [
    ("14be3154", 0x2a00_001f, 0),
    ("26c66f9e", 0x2a00_0247, 0),
    ("2e1e9d2f", 0x2a00_0004, 0),
    ("37cbfaa1", 0x2a00_000c, 0),
    ("39f336a1", 0x2a00_0006, 1),
    ("3d98c754", 0x2a00_005c, 0),
    ("42ab147d", 0x2a00_0027, 0),
    ("46dc2fdb", 0x2a00_0030, 0),
    ("4ab90906", 0x2a00_002a, 0),
    ("530d0457", 0x2a00_008f, 0),
    ("685c1a49", 0x2a00_0020, 0),
    ("6a96af2e", 0x2a00_0026, 0),
    ("6e004941", 0x2a00_001e, 1),
    ("77972111", 0x2a00_00dd, 0),
    ("8e395d34", 0x2a00_0248, 0),
    ("94375486", 0x2a00_0052, 0),
    ("aa756721", 0x2a00_000f, 0),
    ("b9de54a4", 0x2a00_0025, 1),
    ("be6ba59e", 0x2a00_004a, 0),
    ("c606d6c5", 0x2a00_00b3, 0),
    ("c7ce8808", 0x2a00_0017, 0),
    ("d0ee12c7", 0x2a00_0045, 0),
    ("d4f1e62d", 0x2a00_0037, 0),
    ("ef61e885", 0x2a00_000e, 0),
    ("f8a12f7d", 0x2a00_0006, 0),
    ("f8ef2fbb", 0x2a00_0012, 0),
];

pub fn build_index(key: &str) -> Option<usize> {
    BUILDS.iter().position(|b| b.0 == key)
}

pub const pDebug: u32 = 0x10000;
pub const pAnimMngr: u32 = 0x10004;
pub const pFunctions: u32 = 0x10008;
pub const pExtra: u32 = 0x1000c;
pub const pGameObj: u32 = 0x10010;
pub const pPhysic: u32 = 0x10014;
pub const pAnim: u32 = 0x10018;
pub const pLocalDataenemybar: u32 = 0x1001c;
pub const pGlobalDataenemybar: u32 = 0x10020;

#[inline]
fn G(e: &Engine, a: u32) -> Ptr {
    e.rt.cur_data() + (a - 0x10000)
}
/// local data (0x40): +4 parent id, +8 own id, +0xc max life, +0x10 life, +0x28 blend params
/// {id, +0x2c, +0x30 weight, +0x34, +0x38}, +0x3c DM
#[inline]
fn L(e: &Engine) -> Ptr {
    e.r32(G(e, pLocalDataenemybar))
}
fn lw(e: &mut Engine, o: u32, v: u32) {
    let a = L(e) + o;
    e.w32(a, v);
}
fn dbg(e: &mut Engine, a: &str, b: &str) {
    if e.r32(G(e, pDebug)) != 0 {
        let (a, b) = (e.cstr(a), e.cstr(b));
        LIBdebug::PrintDebugInfo(e, a, b);
    }
}

/// 0x00010d50 state 0
pub fn fstate_FIRSTMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    let v = e.alloca(0xc);
    let params = e.alloca(8);
    e.w32(params, 0xff00_0020);
    let id = APIgameobj::GetID(e, SELF, NULL);
    lw(e, 8, id);
    e.w32(params + 4, id);
    let p = APIgameobj::GetID(e, 0xff00_0020, params);
    lw(e, 4, p);
    if e.r32(G(e, pDebug)) != 0 {
        let (a, b) = (e.cstr("EnemyBar"), e.cstr("\n\n\nID parent"));
        let pv = L(e) + 4;
        LIBdebug::PrintDebugInfoWithValue(e, a, b, pv, 1);
    }
    APIanim::BeginTree(e, SELF);
    let swap = BUILDS[e.rt.cur_build()].2 != 0;
    LIBAnimMngr::PlayAnim(e, 0.0, if swap { 1 } else { 2 }, 1, 0);
    lw(e, 0x28, 0xbb00_0001);
    lw(e, 0x2c, 0);
    lw(e, 0x34, 0);
    lw(e, 0x38, 0x3f00_0000);
    let bp = L(e) + 0x28;
    let (a, b) = if swap { (2, 1) } else { (1, 2) };
    APIanim::PlayBlendConnect(e, SELF, a, b, 1, bp);
    APIanim::EndTree(e, SELF);
    e.wf(v, 0.0);
    e.wf(v + 4, 3.0);
    e.wf(v + 8, 0.0);
    APIgameobj::SetTranslation(e, SELF, v);
    APIgameobj::SetNextState(e, SELF, 1);
    Ok(0)
}

/// 0x00010f60 state 1
pub fn fstate_BASICMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    let cam = BUILDS[e.rt.cur_build()].1;
    LIBApiCFunctions::TrackID(e, 0.0, SELF as i32, cam);
    Ok(0)
}

/// 0x00010fc0 external 2
pub fn fcback_NOTIFY_COLLIDE_OBJ(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

/// 0x00011010
pub fn CreatePhysic(e: &mut Engine) {
    let p = e.alloca(0x18);
    APIphysic::CreatePhysic(e, SELF, 0);
    let dm = APIphysic::DMAdd(e, SELF, 1);
    lw(e, 0x3c, dm);
    APIphysic::DMGetParams(e, dm, p);
    let cam = BUILDS[e.rt.cur_build()].1;
    for (i, v) in [cam, 0x42c8_0000, 0, 0, 0, 5].into_iter().enumerate() {
        e.w32(p + 4 * i as u32, v);
    }
    APIphysic::DMSetParams(e, dm, p);
    APIphysic::DMSetContextMask(e, dm, 2);
}

/// 0x00011118
pub fn MyMessageFunction(e: &mut Engine, m: u32, s: u32, d: Ptr) -> LResult<u64> {
    if m == 0xd6 {
        dbg(e, "EnemyBar", "MSG_SUCKING");
        if s == 100 {
            dbg(e, "EnemyBar", "MSG_SUCKING -> MSP_LINK_BROKEN");
            APIgameobj::KillObject(e, SELF);
        }
    } else if m == 0xec {
        let (a, b) = (e.r32(d), e.r32(d + 4));
        lw(e, 0xc, a);
        lw(e, 0x10, b);
        let l = L(e);
        let mut w = e.rf(l + 0x10) / e.rf(l + 0xc);
        if 1.0 < w {
            w = 1.0;
        } else if w < 0.0 {
            w = 0.0;
        }
        e.wf(l + 0x30, w);
        APIanim::ChangeBlendParams(e, SELF, l + 0x28);
    }
    Ok(0)
}

/// 0x00011310
pub fn enemybarConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    if instanceCount == 1 {
        APIcons::SetExternalFunction(e, 0, fstate_FIRSTMOVE);
        APIcons::SetExternalFunction(e, 1, fstate_BASICMOVE);
        APIcons::SetExternalFunction(e, 2, fcback_NOTIFY_COLLIDE_OBJ);
        let tables: [(u32, fn(&mut Engine) -> Ptr); 4] = [
            (pAnim, |e| APIcons::GetApiAnim(e)),
            (pGameObj, |e| APIcons::GetApiGameObj(e)),
            (pExtra, |e| APIcons::GetApiExtra(e)),
            (pPhysic, |e| APIcons::GetApiPhysic(e)),
        ];
        for (slot, get) in tables {
            let v = get(e);
            let a = G(e, slot);
            e.w32(a, v);
        }
        for (slot, name) in [(pDebug, "debug"), (pAnimMngr, "AnimMngr"), (pFunctions, "ApiCFunctions")] {
            let n = e.cstr(name);
            let v = APIcons::GetApiCustom(e, n);
            let a = G(e, slot);
            e.w32(a, v);
        }
        let slot = G(e, pGlobalDataenemybar);
        APIcons::GetGlobalData(e, slot, 4);
        let b = APIgameobj::GetGameBuffer(e, 0, 0x8f8, 1);
        let gd = e.r32(G(e, pGlobalDataenemybar));
        e.w32(gd, b);
    }
    APIcons::SetMessageFunction(e, MyMessageFunction);
    let slot = G(e, pLocalDataenemybar);
    APIcons::GetLocalData(e, slot, 0x40);
    CreatePhysic(e);
    lw(e, 0x28, 0);
    lw(e, 0x30, 0x3f80_0000);
    APIcons::SetInitialState(e, 0);
    Ok(())
}

/// 0x00011530
pub fn enemybarDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    Ok(())
}

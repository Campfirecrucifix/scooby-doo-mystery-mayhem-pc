//! `hs_launchenemy`: EP3 L03 hot-spots; when the cart 0x2a000004 enters one, sends msg 0xae (enemy
//! type, position of the spawn marker) to 0x2a000009. Port of
//! `Ep3_FrontierLand/Objects/Hotspots/Level_03/hs_launchenemy__<sha8>` (6 functions, 2 builds that differ
//! only in the hot-spot / marker ids). pLocalDataHSLaunchEnemy is not in the symtab (placed at 0x10014).
#![allow(non_snake_case, non_upper_case_globals, unused_variables)]

use crate::engine::*;
use crate::logic::{LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "hs_launchenemy",
    data_size: 0x1c,
    ctor: hs_launchenemyConstructor,
    dtor: hs_launchenemyDestructor,
    register: None,
    init: &[],
    build_init: None,
};

pub const pExtra: u32 = 0x10000;
pub const pGameObj: u32 = 0x10004;
pub const pPhysic: u32 = 0x10008;
pub const pMiscFunc: u32 = 0x1000c;
pub const pGame: u32 = 0x10010;
pub const pLocalDataHSLaunchEnemy: u32 = 0x10014;
pub const pGlobalDataHSLaunchEnemy: u32 = 0x10018;

struct Build {
    sha8: &'static str,
    /// first hot-spot id; hot-spot i -> marker `marker + i`, enemy type TYPES[i]
    first: u32,
    marker: u32,
}
static BUILDS: [Build; 2] = [
    Build { sha8: "aab46205", first: 0x2a000230, marker: 0x2a0002a6 },
    Build { sha8: "b7581c49", first: 0x2a00022f, marker: 0x2a0002a7 },
];
const TYPES: [u32; 5] = [0xf, 0x29, 0x28, 0x1f, 0x27];

pub fn build_index(key: &str) -> Option<usize> {
    BUILDS.iter().position(|b| b.sha8 == key)
}

#[inline]
fn G(e: &Engine, a: u32) -> Ptr {
    e.rt.cur_data() + (a - 0x10000)
}
fn gd(e: &Engine) -> Ptr {
    e.r32(G(e, pGlobalDataHSLaunchEnemy))
}

/// 0x00010cf8 state 0
pub fn hs_LaunchEnemyST_FirstMove(e: &mut Engine, _: u32) -> LResult<u64> {
    let c = e.alloca(0x44);
    APIgameobj::GetClippingParams(e, SELF, c);
    for (o, v) in [(4, 0x41c8_0000), (0xc, 0), (8, 0), (0x10, 0xbf80_0000), (0x18, 0x6a0), (0x14, 0x6a0)] {
        e.w32(c + o, v);
    }
    APIgameobj::SetClippingParams(e, SELF, c, 0);
    let g = gd(e) + 4;
    e.w32(g, 0);
    APIgameobj::SetGroupID(e, SELF, 0x15);
    APIgameobj::SetNextState(e, SELF, 1);
    Ok(0)
}

/// 0x00010df0 state 1
pub fn hs_LaunchEnemyST_BasicMove_Loop(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

/// 0x00010e20 callback 2 (collision notify 3)
pub fn hs_LaunchEnemyCB_NotifyColide(e: &mut Engine, param_1: u32) -> LResult<u64> {
    let id = APIgameobj::GetID(e, SELF, NULL);
    if id != e.r32(gd(e) + 4) && e.r32(param_1 + 8) == 0x2a000004 {
        let id = APIgameobj::GetID(e, SELF, NULL);
        let g = gd(e) + 4;
        e.w32(g, id);
        let id = APIgameobj::GetID(e, SELF, NULL);
        let b = &BUILDS[e.rt.cur_build()];
        let i = id.wrapping_sub(b.first);
        if i >= 5 {
            return Ok(1);
        }
        let p = e.alloca(0xc);
        APIgameobj::GetPosition(e, b.marker + i, 0x7fff_ffff, p);
        let t = TYPES[i as usize];
        if t != 0x29a {
            APIgameobj::SendMessageTo(e, 0x2a000009, NULL, 0xae, t, p);
        }
    }
    Ok(0)
}

/// 0x000110b8
pub fn hs_LaunchEnemyFUNC_MessageHandler(e: &mut Engine, param_1: u32, _: u32, _: Ptr) -> LResult<u64> {
    if param_1 == 0xc1 {
        let g = gd(e) + 4;
        e.w32(g, 0);
    } else if param_1 == 0xc2 {
        APIgameobj::UnregisterNotify(e, SELF, 0xffff_ffff, 3);
    }
    Ok(0)
}

/// 0x00011160
pub fn hs_launchenemyConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    if instanceCount == 1 {
        APIcons::SetExternalFunction(e, 0, hs_LaunchEnemyST_FirstMove);
        APIcons::SetExternalFunction(e, 1, hs_LaunchEnemyST_BasicMove_Loop);
        APIcons::SetExternalFunction(e, 2, hs_LaunchEnemyCB_NotifyColide);
        let tables: [(u32, fn(&mut Engine) -> Ptr); 4] = [
            (pExtra, |e| APIcons::GetApiExtra(e)),
            (pGameObj, |e| APIcons::GetApiGameObj(e)),
            (pPhysic, |e| APIcons::GetApiPhysic(e)),
            (pGame, |e| APIcons::GetApiGame(e)),
        ];
        for (slot, get) in tables {
            let v = get(e);
            let a = G(e, slot);
            e.w32(a, v);
        }
        let n = e.cstr("MiscFunc");
        let v = APIcons::GetApiCustom(e, n);
        let a = G(e, pMiscFunc);
        e.w32(a, v);
        let slot = G(e, pGlobalDataHSLaunchEnemy);
        APIcons::GetGlobalData(e, slot, 0xc);
        let b = APIgameobj::GetGameBuffer(e, 0, 0x8f8, 1);
        let g = gd(e) + 8;
        e.w32(g, b);
    }
    let slot = G(e, pLocalDataHSLaunchEnemy);
    APIcons::GetLocalData(e, slot, 0x10);
    APIphysic::CreatePhysic(e, SELF, 0);
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 3, 2);
    APIcons::SetMessageFunction(e, hs_LaunchEnemyFUNC_MessageHandler);
    APIcons::SetInitialState(e, 0);
    APIgameobj::SetGroupID(e, SELF, 0x15);
    APIphysic::PrimSetIntAttribBits(e, SELF, 0, 0, 4);
    APIphysic::PrimSetIntAttribBits(e, SELF, 0, 2, 0x800);
    Ok(())
}

/// 0x000113b8
pub fn hs_launchenemyDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    Ok(())
}

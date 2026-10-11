//! `obj_launcher_level3a`: EP1L03A launched physics object (bouncing sphere, radius 0.5). On start
//! gets a random x impulse (FloatRandom(-1,1)) + (6, 3.5) in object space; once it has landed
//! (state-change notify, both bit 0) and slowed below 0.3 m/s, or touches a floor with attrib bit
//! 0x200000, it hands its class-0x40 child (msgs 0xd5, 0xdd, physics reset, rotation 0) back and
//! kills itself. Port of `Ep1_Library/Objects/OBJ_LAUNCHER/Level03A/obj_launcher_level3a__453c51d6`
//! (obj_launcher.c + obj_launcher_level3a.c, 12 functions). Build 453c51d6.
#![allow(non_snake_case, non_upper_case_globals, unused_variables)]

use crate::engine::*;
use crate::logic::{LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "obj_launcher_level3a",
    // globals 0x10000..0x1002c (+ unused gpoLanguagesNamesAndISOCodes at 0x10030)
    data_size: 0x30,
    ctor: obj_launcher_level3aConstructor,
    dtor: obj_launcher_level3aDestructor,
    register: None,
    init: &[],
    build_init: None,
};

pub fn build_index(key: &str) -> Option<usize> {
    matches!(key, "453c51d6" | "f437ad09").then_some(0)
}

pub const pExtra: u32 = 0x10000;
pub const pGameObj: u32 = 0x10004;
pub const pPhysic: u32 = 0x10008;
pub const pAnim: u32 = 0x1000c;
pub const pMath: u32 = 0x10010;
pub const pAnimMngr: u32 = 0x10014;
pub const pFunction: u32 = 0x10018;
pub const pMiscFunc: u32 = 0x1001c;
pub const pLocalDataobj_launcher: u32 = 0x10020;
pub const pGlobalDataobj_launcher: u32 = 0x10024;
/// not in the symtab; slot address from the constructor disasm (0x119c4)
pub const pLocalDataobj_launcher_level3a: u32 = 0x10028;

#[inline]
fn G(e: &Engine, a: u32) -> Ptr {
    e.rt.cur_data() + (a - 0x10000)
}
fn L(e: &Engine, o: u32) -> Ptr {
    e.r32(G(e, pLocalDataobj_launcher)) + o
}

/// 0x00010d10 state 0 (also called by the level-3a state 7)
pub fn obj_launcherST_FIRSTMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    let f = LIBApiCFunctions::FloatRandom(e, -1.0, 1.0, 1.0);
    let a = L(e, 0x14);
    e.wf(a, f);
    e.wf(a + 4, 6.0);
    e.wf(a + 8, 3.5);
    APIphysic::SetHardness(e, SELF, 0.9);
    APIphysic::SetRebound(e, SELF, 0.5);
    let a = L(e, 0xc);
    e.w32(a, 0);
    let a = L(e, 0x20);
    e.w32(a, 0);
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 5, 5);
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 3, 3);
    APIgameobj::SetNextState(e, SELF, 1);
    Ok(0)
}

/// 0x00010e70 state 1
pub fn obj_launcherST_BASICMOVE_SET(e: &mut Engine, _: u32) -> LResult<u64> {
    let v = L(e, 0x14);
    APIgameobj::TransformVector(e, SELF, v, v);
    APIphysic::Impulse(e, SELF, v);
    APIphysic::SetStateChangeNotifyBits(e, SELF, 1);
    APIgameobj::SetNextState(e, SELF, 2);
    Ok(0)
}

/// Shared tail of BASICMOVE_LOOP and CB_STATECHANGE: release the carried class-0x40 child.
fn release_child(e: &mut Engine) {
    let r = e.alloca(8);
    e.w32(r, 1);
    e.w32(r + 4, SELF);
    let child = APIgameobj::GetID(e, 0xff00_0040, r);
    APIgameobj::RemoveAllChildren(e, SELF, 0);
    APIgameobj::SendMessageTo(e, child, NULL, 0xd5, 0, NULL);
    APIphysic::ResetFlags(e, child, 1);
    APIphysic::SetVelocity(e, child, 0.0, 0.0, 0.0);
    APIgameobj::SendMessageTo(e, child, NULL, 0xdd, 0, NULL);
    let v = e.alloca(0xc);
    e.wf(v, 1.0);
    e.wf(v + 4, 0.0);
    e.wf(v + 8, 0.0);
    APIgameobj::SetRotation(e, child, v, 0.0);
    APIgameobj::KillObject(e, SELF);
}

/// 0x00010f40 state 2
pub fn obj_launcherST_BASICMOVE_LOOP(e: &mut Engine, _: u32) -> LResult<u64> {
    if e.r32(L(e, 0x20)) == 1 {
        let v = e.alloca(0xc);
        APIphysic::GetVelocity(e, SELF, v);
        let (x, y, z) = (e.rf(v), e.rf(v + 4), e.rf(v + 8));
        let s = APImath::sqrtf(e, x * x + y * y + z * z);
        if s <= 0.3 {
            release_child(e);
            return Ok(1);
        }
    }
    Ok(0)
}

/// 0x00011158 callback 5 (state-change notify): param {old bits, new bits}
pub fn obj_launcherCB_STATECHANGE(e: &mut Engine, p: u32) -> LResult<u64> {
    let fa = e.alloca(0x40);
    APIphysic::GetFloorAttribs(e, SELF, fa);
    if e.r32(fa + 4) & 0x20_0000 == 0 {
        if e.r32(p) & 1 != 0 && e.r32(p + 4) & 1 != 0 && e.r32(L(e, 0x20)) != 1 {
            let a = L(e, 0x20);
            e.w32(a, 1);
        }
        Ok(0)
    } else {
        release_child(e);
        Ok(1)
    }
}

/// 0x00011368 callback 3
pub fn obj_launcherCB_NOTIFY_COLLIDE_OBJ(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

/// 0x00011398 callback 4
pub fn obj_launcherCB_ANIM_LF(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(1)
}

/// 0x000113f8
pub fn MyMessageFunction(e: &mut Engine, _m: u32, _s: u32, _d: Ptr) -> LResult<u64> {
    Ok(0)
}

/// 0x00011430
pub fn obj_launcherConstructor(e: &mut Engine, api: Ptr, param_2: i32) {
    if param_2 == 1 {
        APIcons::SetExternalFunction(e, 0, obj_launcherST_FIRSTMOVE);
        APIcons::SetExternalFunction(e, 1, obj_launcherST_BASICMOVE_SET);
        APIcons::SetExternalFunction(e, 2, obj_launcherST_BASICMOVE_LOOP);
        APIcons::SetExternalFunction(e, 3, obj_launcherCB_NOTIFY_COLLIDE_OBJ);
        APIcons::SetExternalFunction(e, 4, obj_launcherCB_ANIM_LF);
        APIcons::SetExternalFunction(e, 5, obj_launcherCB_STATECHANGE);
        let tables: [(u32, fn(&mut Engine) -> Ptr); 5] = [
            (pAnim, |e| APIcons::GetApiAnim(e)),
            (pGameObj, |e| APIcons::GetApiGameObj(e)),
            (pExtra, |e| APIcons::GetApiExtra(e)),
            (pPhysic, |e| APIcons::GetApiPhysic(e)),
            (pMath, |e| APIcons::GetApiMath(e)),
        ];
        for (slot, get) in tables {
            let v = get(e);
            let a = G(e, slot);
            e.w32(a, v);
        }
        for (slot, name) in [(pAnimMngr, "AnimMngr"), (pFunction, "ApiCFunctions"), (pMiscFunc, "MiscFunc")] {
            let n = e.cstr(name);
            let v = APIcons::GetApiCustom(e, n);
            let a = G(e, slot);
            e.w32(a, v);
        }
        let slot = G(e, pGlobalDataobj_launcher);
        APIcons::GetGlobalData(e, slot, 8);
        let b = APIgameobj::GetGameBuffer(e, 0, 0x8f8, 1);
        let gd = e.r32(G(e, pGlobalDataobj_launcher));
        e.w32(gd, b);
    }
    APIcons::SetMessageFunction(e, MyMessageFunction);
    let slot = G(e, pLocalDataobj_launcher);
    APIcons::GetLocalData(e, slot, 0x24);
    APIphysic::CreatePhysic(e, SELF, 0);
    APIphysic::SetFlags(e, SELF, 6);
    APIphysic::SetSpherePrimaryVolume(e, SELF, 0.5, 0.0, 0.0, 0.0);
    APIphysic::PrimSetIntAttribBits(e, SELF, 0, 0, 5);
    APIphysic::PrimSetIntAttribBits(e, SELF, 0, 5, 0x20_0008);
    // DM 0: params {0, -20, 0} (gravity), context mask 1
    let dm = APIphysic::DMAdd(e, SELF, 0);
    let a = L(e, 4);
    e.w32(a, dm);
    let s = e.alloca(0x40);
    APIphysic::DMGetParams(e, dm, s);
    e.w32(s, 0);
    e.w32(s + 4, 0xc1a0_0000);
    e.w32(s + 8, 0);
    APIphysic::DMSetParams(e, dm, s);
    APIphysic::DMSetContextMask(e, dm, 1);
    // DM 8: params {48, 90}, context mask 2
    let dm = APIphysic::DMAdd(e, SELF, 8);
    let a = L(e, 8);
    e.w32(a, dm);
    let s = e.alloca(0x40);
    APIphysic::DMGetParams(e, dm, s);
    e.w32(s + 4, 0x42b4_0000);
    e.w32(s, 0x4240_0000);
    APIphysic::DMSetParams(e, dm, s);
    APIphysic::DMSetContextMask(e, dm, 2);
    APIgameobj::SetContextFlag(e, SELF, 3);
    LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 2, NULL, 0);
    APIcons::SetInitialState(e, 0);
}

/// 0x00011900 state 7
pub fn obj_launcher_level3aST_FIRSTMOVE_LOCAL(e: &mut Engine, p: u32) -> LResult<u64> {
    obj_launcherST_FIRSTMOVE(e, p)?;
    Ok(0)
}

/// 0x00011958
pub fn obj_launcher_level3aConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    obj_launcherConstructor(e, api, instanceCount);
    if instanceCount == 1 {
        APIcons::SetExternalFunction(e, 7, obj_launcher_level3aST_FIRSTMOVE_LOCAL);
    }
    let slot = G(e, pLocalDataobj_launcher_level3a);
    APIcons::GetLocalData(e, slot, 0x24);
    APIcons::SetInitialState(e, 7);
    Ok(())
}

/// 0x00011a08
pub fn obj_launcher_level3aDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    Ok(())
}

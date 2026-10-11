//! `fearghost`: small fear ghost spawned (parented) by a monster; msg 0x175 binds it to an owner id,
//! 0x176 from that owner releases it (LeaveParent -> EXPLODE: physics + 3 s life), it then picks one of
//! up to 4 nearby heroes (retriever 0xff000100, weighted random over {45,70,85,95,100}), homes on it
//! (DM 1), spins around it for 3 s and dies; 0x177 from the owner kills it at once.
//! Port of `Game Logic Modules/Globals/Objects/FEARGHOST/fearghost/fearghost.c` (19 functions, 1 build).
#![allow(non_snake_case, non_upper_case_globals, unused_variables)]

use crate::engine::*;
use crate::logic::{LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "fearghost",
    data_size: 0x30,
    ctor: fearghostConstructor,
    dtor: fearghostDestructor,
    register: None,
    init: &[],
    build_init: None,
};

pub const pExtra: u32 = 0x10000;
pub const pGameObj: u32 = 0x10004;
pub const pPhysic: u32 = 0x10008;
pub const pAnim: u32 = 0x1000c;
pub const pGraphic: u32 = 0x10010;
pub const pMath: u32 = 0x10014;
pub const pStdlib: u32 = 0x10018;
pub const pAnimMngr: u32 = 0x1001c;
pub const pFunctions: u32 = 0x10020;
pub const pMiscFunc: u32 = 0x10024;
pub const pLocalDataFearghost: u32 = 0x10028;
pub const pGlobalDataFearghost: u32 = 0x1002c;

const T0: u32 = 0x9000_0000;

#[inline]
fn G(e: &Engine, a: u32) -> Ptr {
    e.rt.cur_data() + (a - 0x10000)
}
/// local data (0x40): +0 owner id, +4 state mirror, +8 death time, +0xc spin end time, +0x10 track time,
/// +0x1c motion trail, +0x20 target, +0x28 anim 2 played, +0x2c/+0x30/+0x34 DMs, +0x38 sprite group, +0x3c sprite
fn L(e: &Engine, o: u32) -> Ptr {
    e.r32(G(e, pLocalDataFearghost)) + o
}
fn lr(e: &Engine, o: u32) -> u32 {
    e.r32(L(e, o))
}
fn lw(e: &mut Engine, o: u32, v: u32) {
    let a = L(e, o);
    e.w32(a, v);
}
fn lf(e: &Engine, o: u32) -> f32 {
    e.rf(L(e, o))
}
fn lwf(e: &mut Engine, o: u32, v: f32) {
    let a = L(e, o);
    e.wf(a, v);
}
fn next(e: &mut Engine, s: u32) {
    lw(e, 4, s);
    APIgameobj::SetNextState(e, SELF, s);
}

/// 0x00010d30 state 0
pub fn FearghostST_FIRSTMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    APIgameobj::SetGroupID(e, SELF, 0x20);
    APIanim::Play(e, SELF, 1, 2, 0.0);
    FearghostFUNC_CreateGlow(e);
    lw(e, 0x28, 0);
    FearghostFUNC_CreateMotionTrail(e);
    next(e, 1);
    Ok(0)
}

/// 0x00010e08 state 1
pub fn FearghostST_BASICMOVE_SET(e: &mut Engine, _: u32) -> LResult<u64> {
    next(e, 2);
    Ok(0)
}

/// 0x00010e70 state 2
pub fn FearghostST_BASICMOVE_LOOP(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

/// 0x00010ea0 state 3
pub fn FearghostST_EXPLODE(e: &mut Engine, _: u32) -> LResult<u64> {
    let t = APIextra::TimerGetTime(e, T0);
    lwf(e, 8, t + 3.0);
    FearghostFUNC_SetMove(e);
    let t = APIextra::TimerGetTime(e, T0);
    lwf(e, 0x10, t + 0.3);
    next(e, 4);
    Ok(0)
}

/// 0x00010f80 state 4
pub fn FearghostST_FINDTARGET(e: &mut Engine, _: u32) -> LResult<u64> {
    let id = FearghostFUNC_GetTargetID(e);
    if id == 0x7fff_fffe {
        next(e, 5);
    } else {
        lw(e, 0x20, id);
        next(e, 6);
    }
    Ok(0)
}

/// 0x00011058 state 6
pub fn FearghostST_TRACKTARGET(e: &mut Engine, _: u32) -> LResult<u64> {
    let t0 = lf(e, 0x10);
    if t0 <= APIextra::TimerGetTime(e, T0) {
        let dm = APIphysic::DMAdd(e, SELF, 1);
        lw(e, 0x34, dm);
        let p = e.alloca(0x40);
        APIphysic::DMGetParams(e, dm, p);
        let tgt = lr(e, 0x20);
        for (i, v) in [tgt, 0x40a0_0000, 0, 0x3e80_0000, 0, 6].into_iter().enumerate() {
            e.w32(p + i as u32 * 4, v);
        }
        APIphysic::DMSetParams(e, dm, p);
        APIphysic::DMSetContextMask(e, dm, 2);
        APIgameobj::SetContextFlag(e, SELF, 7);
        lw(e, 0xc, 0);
        next(e, 7);
    }
    Ok(0)
}

/// 0x000111f8 state 7
pub fn FearghostST_SPINAROUND(e: &mut Engine, _: u32) -> LResult<u64> {
    let a = e.alloca(0xc);
    let b = e.alloca(0xc);
    let id = APIgameobj::GetID(e, SELF, NULL);
    APIgameobj::GetPosition(e, id, 0x7fff_ffff, a);
    let tgt = lr(e, 0x20);
    APIgameobj::GetPosition(e, tgt, 0x7fff_ffff, b);
    let (p, mut q) = (e.rv3(a), e.rv3(b));
    q[1] += 0.25;
    let d = [p[0] - q[0], p[1] - q[1], p[2] - q[2]];
    if lf(e, 0xc) == 0.0 {
        let l = APImath::sqrtf(e, d[0] * d[0] + d[1] * d[1] + d[2] * d[2]);
        if 1.0 < l {
            return Ok(0);
        }
        let t = APIextra::TimerGetTime(e, T0);
        lwf(e, 0xc, t + 3.0);
        let ax = e.alloca(0xc);
        e.wv3(ax, [0.0, 1.0, 0.0]);
        APIgameobj::Rotate(e, SELF, ax, 1.570_796_4);
    }
    let end = lf(e, 0xc);
    if APIextra::TimerGetTime(e, T0) < end {
        FearghostFUNC_Spin(e);
        if lr(e, 0x28) == 0 && lf(e, 0xc) - 1.0 < APIextra::TimerGetTime(e, T0) {
            lw(e, 0x28, 1);
            APIanim::Play(e, SELF, 2, 1, 0.0);
        }
    } else {
        lw(e, 8, 0);
        next(e, 5);
    }
    Ok(0)
}

fn remove_glow(e: &mut Engine) {
    if lr(e, 0x38) as i32 != -2 {
        let (g, s) = (lr(e, 0x38), lr(e, 0x3c));
        APIgraphics::SpriteGroupRemoveSprite(e, g, s);
        APIgraphics::SpriteRemoveGroup(e, g);
        lw(e, 0x38, 0xffff_fffe);
    }
}

/// 0x000114f8 state 5
pub fn FearghostST_DIE(e: &mut Engine, _: u32) -> LResult<u64> {
    let t = lf(e, 8);
    if t < APIextra::TimerGetTime(e, T0) {
        let mt = lr(e, 0x1c);
        APIgraphics::MTStopDraw(e, mt);
        remove_glow(e);
        APIgameobj::KillObject(e, SELF);
    }
    Ok(0)
}

/// 0x00011620 callback 8
pub fn FearghostCB_NOTIFY_COLLIDE_OBJ(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

/// 0x00011670
pub fn FearghostFUNC_MessageHandler(e: &mut Engine, param_1: u32, param_2: u32, _d: Ptr) -> LResult<u64> {
    if param_1 == 0x176 {
        if param_2 == lr(e, 0) && lf(e, 0xc) == 0.0 {
            APIgameobj::LeaveParent(e, SELF, 0);
            lw(e, 0, 0);
            APIgameobj::SetNextState(e, SELF, 3);
        }
    } else if param_1 < 0x177 {
        if param_1 == 0x175 {
            lw(e, 0, param_2);
        }
    } else if param_1 == 0x177 && param_2 == lr(e, 0) {
        APIgameobj::SetNextState(e, SELF, 5);
    }
    Ok(0)
}

/// 0x000117c8
pub fn FearghostFUNC_Create_Physic(e: &mut Engine) {}

/// 0x000117e8
pub fn FearghostFUNC_SetMove(e: &mut Engine) {
    APIphysic::CreatePhysic(e, SELF, 0);
    APIphysic::SetFlags(e, SELF, 2);
    let dm = APIphysic::DMAdd(e, SELF, 4);
    lw(e, 0x2c, dm);
    let p = e.alloca(0x40);
    APIphysic::DMGetParams(e, dm, p);
    e.w32(p + 0xc, 1);
    e.w32(p, 0);
    e.w32(p + 4, 0);
    e.w32(p + 8, 0x4080_0000);
    APIphysic::DMSetParams(e, dm, p);
    APIphysic::DMSetContextMask(e, dm, 4);
    let dm = APIphysic::DMAdd(e, SELF, 7);
    lw(e, 0x30, dm);
    let p = e.alloca(0x40);
    APIphysic::DMGetParams(e, dm, p);
    e.w32(p, 0x4120_0000);
    e.w32(p + 4, 7);
    APIphysic::DMSetParams(e, dm, p);
    APIphysic::DMSetContextMask(e, dm, 1);
    APIgameobj::SetContextFlag(e, SELF, 5);
}

/// 0x000119c0
pub fn FearghostFUNC_CreateMotionTrail(e: &mut Engine) {
    let mt = APIgraphics::MTCreateSystem(e, 5, 10, 0);
    lw(e, 0x1c, mt);
    APIgraphics::MTSetFrequency(e, mt, 0.01);
    for (i, joint, rgb, life) in [
        (0, 5, [0.3, 0.78, 0.82], 0.2),
        (1, 4, [0.0, 0.0, 0.0], 0.2),
        (2, 1, [1.0, 1.0, 1.0], 0.4),
        (3, 2, [0.0, 0.0, 0.0], 0.2),
        (4, 3, [0.3, 0.78, 0.82], 0.2),
    ] {
        let mt = lr(e, 0x1c);
        APIgraphics::MTSetJoint(e, mt, i, SELF, joint);
        APIgraphics::MTSetRGB(e, mt, i, rgb[0], rgb[1], rgb[2]);
        APIgraphics::MTSetLifeSpan(e, mt, i, life);
    }
    let mt = lr(e, 0x1c);
    APIgraphics::MTStart(e, mt);
}

/// rodata 0x10ce0: cumulative pick weights (percent) of the 1st..5th nearest hero
const WEIGHTS: [u32; 5] = [0x2d, 0x46, 0x55, 0x5f, 0x64];

/// 0x00011d58. Quirk kept: the ids are sorted by distance into a local array that is never used; the
/// weighted pick indexes the unsorted (retrieval-order) array.
pub fn FearghostFUNC_GetTargetID(e: &mut Engine) -> u32 {
    let rp = e.alloca(0x20);
    let me = APIgameobj::GetID(e, SELF, NULL);
    for (i, v) in [1, 0x0080_1400, me, 0, 0, 0, 0x4040_0000, 0x3dcc_cccd].into_iter().enumerate() {
        e.w32(rp + i as u32 * 4, v);
    }
    let mut ids = [0u32; 4];
    let mut dist = [0f32; 4];
    let mut n = 0usize;
    loop {
        let id = APIgameobj::GetID(e, 0xff00_0100, rp);
        if id == 0x7fff_fffe || 3 < n {
            break;
        }
        ids[n] = id;
        dist[n] = LIBApiCFunctions::GetDistance(e, me as i32, id, 0x7fff_ffff, 1);
        n += 1;
    }
    let mut sorted = [0u32; 4];
    let (mut best_id, mut best_i) = (0u32, 0usize);
    for i in 0..n {
        let mut best = 999.0f32;
        for j in 0..n {
            if dist[j] < best {
                best = dist[j];
                best_id = ids[j];
                best_i = j;
            }
        }
        sorted[i] = best_id;
        dist[best_i] = 9999.0;
    }
    let r = (APIstdlib::MyRand(e) as i32 % 100) as u32;
    for (k, &w) in WEIGHTS.iter().enumerate().take(n) {
        if r < w {
            return ids[k];
        }
    }
    if n == 0 { 0x7fff_fffe } else { ids[n - 1] }
}

/// 0x00012150
pub fn FearghostFUNC_Spin(e: &mut Engine) {
    let a = e.alloca(0x40);
    let dm = lr(e, 0x34);
    APIphysic::DMGetParams(e, dm, a);
    let v = e.rf(a + 4) + 0.075;
    e.wf(a + 4, v);
    let v = e.rf(a + 0xc) + 0.02;
    e.wf(a + 0xc, v);
    APIphysic::DMSetParams(e, dm, a);
    let b = e.alloca(0x40);
    let dm = lr(e, 0x2c);
    APIphysic::DMGetParams(e, dm, b);
    let v = e.rf(b + 8) + 0.05;
    e.wf(b + 8, v);
    APIphysic::DMSetParams(e, dm, b);
}

/// 0x00012250
pub fn FearghostFUNC_SetClipping(e: &mut Engine) {
    let c = e.alloca(0x44);
    APIgameobj::GetClippingParams(e, SELF, c);
    e.w32(c + 4, 0xbf80_0000);
    e.w32(c + 0xc, 0);
    e.w32(c + 8, 7);
    APIgameobj::SetClippingParams(e, SELF, c, 0);
}

/// 0x000122d0
pub fn FearghostFUNC_CreateGlow(e: &mut Engine) {
    if lr(e, 0x38) as i32 == -2 {
        let g = APIgraphics::SpriteAddGroup(e);
        lw(e, 0x38, g);
        let s = e.cstr("glow");
        APIgraphics::SpriteSetGroupTexture(e, g, s);
        APIgraphics::SpriteSetGroupBlendMode(e, g, 1);
        APIgraphics::SpriteSetGroupFlags(e, g, 4);
        APIgraphics::SpriteSetGroupColor(e, g, 0.3, 0.78, 0.82, 0.1);
        let sp = APIgraphics::SpriteGroupAddSprite(e, g, SELF, 0x7fff_ffff, 0.15);
        lw(e, 0x3c, sp);
    }
}

/// 0x00012440
pub fn fearghostConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    if instanceCount == 1 {
        let fns: [crate::logic::ExtFn; 9] = [
            FearghostST_FIRSTMOVE,
            FearghostST_BASICMOVE_SET,
            FearghostST_BASICMOVE_LOOP,
            FearghostST_EXPLODE,
            FearghostST_FINDTARGET,
            FearghostST_DIE,
            FearghostST_TRACKTARGET,
            FearghostST_SPINAROUND,
            FearghostCB_NOTIFY_COLLIDE_OBJ,
        ];
        for (i, f) in fns.into_iter().enumerate() {
            APIcons::SetExternalFunction(e, i as i32, f);
        }
        let tables: [(u32, fn(&mut Engine) -> Ptr); 7] = [
            (pAnim, |e| APIcons::GetApiAnim(e)),
            (pGameObj, |e| APIcons::GetApiGameObj(e)),
            (pExtra, |e| APIcons::GetApiExtra(e)),
            (pPhysic, |e| APIcons::GetApiPhysic(e)),
            (pGraphic, |e| APIcons::GetApiGraphics(e)),
            (pStdlib, |e| APIcons::GetApiStdlib(e)),
            (pMath, |e| APIcons::GetApiMath(e)),
        ];
        for (slot, get) in tables {
            let v = get(e);
            let a = G(e, slot);
            e.w32(a, v);
        }
        for (slot, name) in [(pAnimMngr, "AnimMngr"), (pFunctions, "ApiCFunctions"), (pMiscFunc, "MiscFunc")] {
            let n = e.cstr(name);
            let v = APIcons::GetApiCustom(e, n);
            let a = G(e, slot);
            e.w32(a, v);
        }
        let slot = G(e, pGlobalDataFearghost);
        APIcons::GetGlobalData(e, slot, 4);
        let b = APIgameobj::GetGameBuffer(e, 0, 0x8f8, 1);
        let g = e.r32(G(e, pGlobalDataFearghost));
        e.w32(g, b);
    }
    APIcons::SetMessageFunction(e, FearghostFUNC_MessageHandler);
    let slot = G(e, pLocalDataFearghost);
    APIcons::GetLocalData(e, slot, 0x40);
    lw(e, 0x38, 0xffff_fffe);
    lw(e, 0x1c, 0xffff_fffe);
    FearghostFUNC_Create_Physic(e);
    FearghostFUNC_SetClipping(e);
    APIcons::SetInitialState(e, 0);
    Ok(())
}

/// 0x00012768
pub fn fearghostDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    remove_glow(e);
    if lr(e, 0x1c) as i32 != -2 {
        let mt = lr(e, 0x1c);
        APIgraphics::MTDestroySystem(e, mt);
    }
    Ok(())
}

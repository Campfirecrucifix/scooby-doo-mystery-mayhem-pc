//! `candle`: Ep1 library candle: a flame particle type + a halo particle child whose frequency /
//! life span flicker with a random value redrawn by two timers.
//! Port of `Game Logic Modules/Ep1_Library/Objects/CANDLE/candle__46128d09/Candle.c` (9 functions).
//! One build matched (EP1L01, sha 46128d09).
#![allow(non_snake_case, non_upper_case_globals, unused_variables)]

use crate::engine::*;
use crate::logic::{LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "candle",
    data_size: 0xd40,
    ctor: ctor_dispatch,
    dtor: dtor_dispatch,
    register: None,
    init: &[],
    build_init: Some(build_init_dispatch),
};

mod xb;

/// number of hand-ported builds (indices 0..N_OLD); generated builds (`xb`) follow
pub(crate) const N_OLD: usize = 1;

fn ctor_dispatch(e: &mut Engine, api: Ptr, n: i32) -> LResult<()> {
    if e.rt.cur_build() < N_OLD { candleConstructor(e, api, n) } else { xb::ctor(e, api, n) }
}

fn dtor_dispatch(e: &mut Engine, n: i32) -> LResult<()> {
    if e.rt.cur_build() < N_OLD { candleDestructor(e, n) } else { xb::dtor(e, n) }
}

fn build_init_dispatch(m: &mut Mem, data: Ptr, b: usize) {
    if b >= N_OLD {
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
    matches!(key, "46128d09" | "f03b2ebb").then_some(0)
}

pub const pExtra: u32 = 0x10000;
pub const pGameObj: u32 = 0x10004;
pub const pParticle: u32 = 0x10008;
pub const pLight: u32 = 0x1000c;
pub const pFunction: u32 = 0x10010;
/// float, shared by every candle (module global)
pub const fRandValue: u32 = 0x10014;
pub const pLocalDataCandle: u32 = 0x10018;
pub const pGlobalDataCandle: u32 = 0x1001c;

#[inline]
fn G(e: &Engine, a: u32) -> Ptr {
    e.rt.cur_data() + (a - 0x10000)
}
/// local data (0x2c): +0 flame type, +0x10 flame emitter, +0x14 halo emitter, +0x18 halo type,
/// +0x24 timer (life span), +0x28 timer (halo)
fn lr(e: &Engine, o: u32) -> u32 {
    e.r32(e.r32(G(e, pLocalDataCandle)) + o)
}
fn lw(e: &mut Engine, o: u32, v: u32) {
    let a = e.r32(G(e, pLocalDataCandle)) + o;
    e.w32(a, v);
}
fn rearm(e: &mut Engine, t: u32, f: f32) {
    APIextra::TimerReset(e, t);
    APIextra::TimerSetFrequency(e, t, f);
    APIextra::TimerRegisterNotify(e, SELF, t, 2);
}

/// 0x00010d28
pub fn candleST_FIRSTMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    createParticles(e);
    let t = APIextra::TimerCreate(e);
    lw(e, 0x24, t);
    let t = APIextra::TimerCreate(e);
    lw(e, 0x28, t);
    rearm(e, lr(e, 0x28), 1.0);
    rearm(e, lr(e, 0x24), 1.1);
    APIgameobj::SetNextState(e, SELF, 1);
    Ok(0)
}

/// 0x00010eb8
pub fn candleST_BASICMOVE_LOOP(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

/// 0x00010ee8 external 2
pub fn candleCB_TIMER(e: &mut Engine, param_1: u32) -> LResult<u64> {
    if param_1 == lr(e, 0x28) {
        let r = LIBApiCFunctions::FloatRandom(e, 0.15, 0.9, 0.05);
        let a = G(e, fRandValue);
        e.wf(a, r);
        rearm(e, lr(e, 0x28), r);
        candleFUNC_AdjustHaloFrequency(e, r);
    }
    if param_1 == lr(e, 0x24) {
        // quirk: uses the shared fRandValue (0.0 until the halo timer first fired)
        let r = e.rf(G(e, fRandValue));
        rearm(e, lr(e, 0x24), r);
        candleFUNC_AdjustLifeSpanFrequency(e, r);
    }
    Ok(0)
}

/// 0x000110a8 (type +0x18 is the halo type)
pub fn candleFUNC_AdjustHaloFrequency(e: &mut Engine, param_1: f32) {
    let t = lr(e, 0x18) as i32;
    APIparticle::TypeSetFloat(e, t, 4, param_1);
}

/// 0x00011100
pub fn candleFUNC_AdjustLifeSpanFrequency(e: &mut Engine, param_1: f32) {
    let t = lr(e, 0x18) as i32;
    APIparticle::TypeSetFloat(e, t, 2, param_1);
}

/// 0x00011158
pub fn candleFUNC_MessageHandler(e: &mut Engine, _m: u32, _s: u32, _d: Ptr) -> LResult<u64> {
    Ok(0)
}

fn type_params(e: &mut Engine, t: i32, floats: [f32; 4], vec1: [f32; 3], inter: [[f32; 4]; 6]) {
    for (k, v) in [(8, 1), (1, 2), (4, 0x1e), (0x10, 0), (0x20, 0), (0x40, 0), (0x80, 0), (0x100, 0)] {
        APIparticle::TypeSetInt(e, t, k, v);
    }
    for (k, v) in [1, 4, 2, 8].into_iter().zip(floats) {
        APIparticle::TypeSetFloat(e, t, k, v);
    }
    APIparticle::TypeSetVec(e, t, 1, vec1[0], vec1[1], vec1[2]);
    APIparticle::TypeSetVec(e, t, 2, 0.0, 0.0, 0.0);
    for (k, i) in [1, 2, 4, 8, 0x10, 0x20].into_iter().zip(inter) {
        APIparticle::TypeSetInter(e, t, k, i[0], i[1], i[2], i[3]);
    }
}

/// 0x000111a0
pub fn createParticles(e: &mut Engine) -> u32 {
    // DTVector3d DStack_40 = DAT_00010cd0 = {0, 0.1, 0}
    let v = e.alloca(0xc);
    e.wf(v, 0.0);
    e.wf(v + 4, 0.1);
    e.wf(v + 8, 0.0);
    let t = APIparticle::TypeCreate(e);
    lw(e, 0, t);
    let em = APIparticle::EmitterCreate(e, t as i32);
    lw(e, 0x10, em);
    APIparticle::EmitterSetJoint(e, em as i32, SELF, 0x7fff_ffff);
    let s = e.cstr("spirit");
    APIparticle::TypeSetTexture(e, t as i32, s);
    let s = e.cstr("Candle_Flamme");
    APIparticle::TypeSetName(e, t as i32, s);
    type_params(
        e,
        t as i32,
        [0.0, 0.06, 0.21, 0.8],
        [0.0, 0.1, 0.0],
        [[1.0, 1.0, 0.25, 0.1], [1.0, 0.5, 0.0, 0.1], [1.0, 0.0, 0.0, 0.1], [0.0; 4], [0.1, 0.075, 0.01, 0.1], [0.0, 0.0, 0.0, 0.5]],
    );
    APIparticle::EmitterStart(e, em as i32, 1);
    let t = APIparticle::TypeCreate(e);
    lw(e, 0x18, t);
    let em = APIparticle::EmitterCreate(e, t as i32);
    lw(e, 0x14, em);
    APIparticle::EmitterSetJoint(e, em as i32, SELF, 0x6fff_ffff);
    APIgameobj::AddChild(e, SELF, 0x7fff_ffff, em, 1);
    APIgameobj::SetTranslation(e, em, v);
    let s = e.cstr("halo_candle");
    APIparticle::TypeSetTexture(e, t as i32, s);
    let s = e.cstr("Candle_Halo");
    APIparticle::TypeSetName(e, t as i32, s);
    type_params(
        e,
        t as i32,
        [0.0, 1.0, 1.1, 0.0],
        [0.0, 0.0, 0.0],
        [[0.05, 0.25, 0.05, 0.5], [0.05, 0.25, 0.05, 0.5], [0.05, 0.25, 0.05, 0.5], [0.0; 4], [0.4, 0.4, 0.4, 0.0], [0.125, 0.125, 0.125, 0.0]],
    );
    APIparticle::EmitterStart(e, em as i32, 1)
}

/// 0x00011c00
pub fn candleConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    if instanceCount == 1 {
        APIcons::SetExternalFunction(e, 0, candleST_FIRSTMOVE);
        APIcons::SetExternalFunction(e, 1, candleST_BASICMOVE_LOOP);
        APIcons::SetExternalFunction(e, 2, candleCB_TIMER);
        let tables: [(u32, fn(&mut Engine) -> Ptr); 4] = [
            (pGameObj, |e| APIcons::GetApiGameObj(e)),
            (pExtra, |e| APIcons::GetApiExtra(e)),
            (pParticle, |e| APIcons::GetApiParticle(e)),
            (pLight, |e| APIcons::GetApiLight(e)),
        ];
        for (slot, get) in tables {
            let v = get(e);
            let a = G(e, slot);
            e.w32(a, v);
        }
        let n = e.cstr("ApiCFunctions");
        let v = APIcons::GetApiCustom(e, n);
        let a = G(e, pFunction);
        e.w32(a, v);
        let slot = G(e, pGlobalDataCandle);
        APIcons::GetGlobalData(e, slot, 4);
        let b = APIgameobj::GetGameBuffer(e, 0, 0x8f8, 1);
        let gd = e.r32(G(e, pGlobalDataCandle));
        e.w32(gd, b);
    }
    APIcons::SetMessageFunction(e, candleFUNC_MessageHandler);
    let slot = G(e, pLocalDataCandle);
    APIcons::GetLocalData(e, slot, 0x2c);
    APIcons::SetInitialState(e, 0);
    Ok(())
}

/// 0x00011db0
pub fn candleDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    let (a, b, c, d) = (lr(e, 0x10), lr(e, 0), lr(e, 0x18), lr(e, 0x14));
    APIparticle::EmitterDestroy(e, a as i32);
    APIparticle::TypeDestroy(e, b as i32);
    APIparticle::TypeDestroy(e, c as i32);
    APIparticle::EmitterDestroy(e, d as i32);
    let (t1, t2) = (lr(e, 0x24), lr(e, 0x28));
    APIextra::TimerDestroy(e, t1);
    APIextra::TimerDestroy(e, t2);
    Ok(())
}

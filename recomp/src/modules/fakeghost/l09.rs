//! `fakeghost` build 4da56182 (EP1L09): Peabody disguised as a ghost, walking a tracked path. Messages set
//! the next track point (0xf9 / 0xe0), the ghost reports arrival (0xf8), being caught (0xf5 / 0xbd) or a wall
//! (0xed) to 0x2a000033. Port of `Ep1_Library/Characters/FakeGhost/Level09/FakeGhost.c` (18 functions).
use super::*;

pub const pExtra: u32 = 0x10000;
pub const pGameObj: u32 = 0x10004;
pub const pPhysic: u32 = 0x10008;
pub const pParticle: u32 = 0x1000c;
pub const pMath: u32 = 0x10010;
pub const pStdlib: u32 = 0x10014;
pub const pLight: u32 = 0x10018;
pub const pSound: u32 = 0x1001c;
pub const pGraphic: u32 = 0x10020;
pub const pDebug: u32 = 0x10024;
pub const pAnimMngr: u32 = 0x10028;
pub const pFunction: u32 = 0x1002c;
pub const pMiscFunc: u32 = 0x10030;
pub const pAnim: u32 = 0x10034;
pub const pLocalDataFakeGhost: u32 = 0x10038;
pub const pGlobalDataFakeGhost: u32 = 0x1003c;

fn L(e: &Engine) -> Ptr {
    e.r32(G(e, pLocalDataFakeGhost))
}
fn lr(e: &Engine, o: u32) -> u32 {
    e.r32(L(e) + o)
}
fn lw(e: &mut Engine, o: u32, v: u32) {
    let a = L(e) + o;
    e.w32(a, v);
}
fn gb(e: &Engine) -> Ptr {
    e.r32(e.r32(G(e, pGlobalDataFakeGhost)))
}
fn leader(e: &Engine) -> u32 {
    e.r32(gb(e) + 8)
}

pub fn fstate_FIRSTMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    let v = e.alloca(0xc);
    for i in 0..3 {
        e.wf(v + i * 4, -100.0);
    }
    Create_Physic(e);
    APIgameobj::GetID(e, SELF, NULL);
    let l = leader(e);
    APInewSound::SetListener(e, l);
    APIgameobj::SetGroupBitMask(e, SELF, 0x200003);
    APIgameobj::SetGroupBitMask(e, SELF, 0x100000);
    APIgameobj::SetGroupID(e, SELF, 1);
    APIgameobj::SetContextFlag(e, SELF, 7);
    lw(e, 0x7c, 1);
    lw(e, 0x80, 0);
    APIgameobj::SetGameObjectVisible(e, SELF, 0);
    APIgameobj::Translate(e, SELF, v);
    APIgameobj::SetNextState(e, SELF, 2);
    LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 4, NULL, 0);
    for (o, cb) in [(0x14, Some(4)), (0x18, None)] {
        let t = APIextra::TimerCreate(e);
        lw(e, o, t);
        APIextra::TimerSetFrequency(e, t, 0.1);
        if let Some(cb) = cb {
            APIextra::TimerReset(e, t);
            APIextra::TimerRegisterNotify(e, SELF, t, cb);
        }
    }
    InitParticles_FootSteps(e);
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 0xb, 7);
    Ok(0)
}
pub fn fstate_BASICMOVE_SET(e: &mut Engine, _: u32) -> LResult<u64> {
    APIgameobj::SetNextState(e, SELF, 2);
    Ok(0)
}
pub fn fstate_BASICMOVE_LOOP(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

pub fn MyMessageFunction(e: &mut Engine, m: u32, _: u32, d: Ptr) -> LResult<u64> {
    match m {
        0xf9 => SetTracking(e, d),
        0xc1 => {
            LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 4, NULL, 0);
            APIgameobj::SendMessageTo(e, 0x2a0000b9, NULL, 0xeb, 2, NULL);
            let v = e.alloca(0xc);
            e.wf(v, 549.0);
            e.wf(v + 4, 0.0);
            e.wf(v + 8, 1.2);
            APIgameobj::SetTranslation(e, SELF, v);
            for (o, x) in [(0x84, 0), (0x88, 1), (0x8c, 1), (0x90, 0), (0x94, 1), (0x98, 1), (0x9c, 0), (0xa0, 0), (0xa4, 0)] {
                lw(e, o, x);
            }
            SetSpeed(e, false);
            LIBAnimMngr::PlayAnim(e, 0.2, 7, 2, 0);
            APIgameobj::SetNextState(e, SELF, 1);
        }
        0xe0 => {
            LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 2, NULL, 0);
            APIgameobj::SetGameObjectVisible(e, SELF, 1);
            APIgameobj::SendMessageTo(e, 0x2a0000b9, NULL, 0xeb, 2, NULL);
            let t = lr(e, 0x14);
            APIextra::TimerRegisterNotify(e, SELF, t, 4);
            SetTracking(e, d);
            SetSpeed(e, true);
            APIgameobj::SetNextState(e, SELF, 1);
        }
        0xfa => {
            let t = lr(e, 0x14);
            APIextra::TimerUnregisterNotify(e, SELF, t);
            LIBAnimMngr::PlayAnim(e, 0.0, 6, 1, 0);
            SetSpeed(e, false);
        }
        0x11b => {
            LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 2, NULL, 0);
        }
        _ => {}
    }
    Ok(1)
}

pub fn fakeghostCB_TIMER_TRACKING(e: &mut Engine, _: u32) -> LResult<u64> {
    let l = L(e);
    if LIBApiCFunctions::CheckRangePoint(e, 1.0, SELF, l + 0x9c, 0) != 0 {
        for o in [0, 4, 8] {
            let v = lr(e, 0x90 + o);
            lw(e, 0x84 + o, v);
        }
        APIgameobj::SendMessageTo(e, 0x2a000033, NULL, 0xf8, 0, l + 0x90);
    } else if lr(e, 0x80) == 0 {
        let ld = leader(e);
        if LIBApiCFunctions::CheckRangeObject(e, 1.75, SELF, ld, 0) != 0
            && LIBApiCFunctions::CheckPositionOrientation(e, SELF, ld, 0xc003) == 1
        {
            if lr(e, 0x90) == 0 && lr(e, 0x94) == 1 && lr(e, 0x98) == 1 {
                APIgameobj::SendMessageTo(e, 0x2a000033, NULL, 0xf5, 0, NULL);
            } else {
                APIgameobj::SendMessageTo(e, 0x2a000033, NULL, 0xbd, 0, l + 0x90);
            }
        } else {
            for (o, v) in [(0x5c, 0), (0x60, 0), (100, 0), (0x68, 0), (0x6c, 0), (0x70, 0x4080_0000)] {
                lw(e, o, v);
            }
            RayIntersect(e);
            let d = fFrontWallDist(e);
            if d < 1.0 && d != 0.0 {
                lw(e, 0x80, 1);
                let t = lr(e, 0x18);
                APIextra::TimerReset(e, t);
                APIextra::TimerRegisterNotify(e, SELF, t, 5);
                APIgameobj::SendMessageTo(e, 0x2a000033, NULL, 0xed, 0, l + 0x90);
            }
        }
    }
    Ok(0)
}
pub fn fcback_TIMER_ENABLE_GHOSTTOUCHED(e: &mut Engine, _: u32) -> LResult<u64> {
    lw(e, 0x80, 0);
    Ok(1)
}
pub fn fcback_END_ANIBUMP(e: &mut Engine, _: u32) -> LResult<u64> {
    lw(e, 0x78, 1);
    Ok(1)
}
pub fn fstate_GHOSTPUSHED(e: &mut Engine, _: u32) -> LResult<u64> {
    if lr(e, 0x78) == 1 {
        SetSpeed(e, true);
        APIgameobj::SetNextState(e, SELF, 1);
    }
    Ok(0)
}
/// ray from the ghost (+0.5 up) along its local +0x68 direction; hit -> L+0x1c, result -> L+0x74
fn RayIntersect(e: &mut Engine) {
    let l = L(e);
    APIgameobj::GetPosition(e, SELF, 0x7fff_ffff, l + 0x5c);
    APIgameobj::TransformVector(e, SELF, l + 0x68, l + 0x68);
    let r = e.alloca(0x2c);
    let id = APIgameobj::GetID(e, SELF, NULL);
    let y = e.rf(l + 0x60) + 0.5;
    let vals = [lr(e, 0x5c), y.to_bits(), lr(e, 100), lr(e, 0x68), lr(e, 0x6c), lr(e, 0x70), 0x100_0000, 0, 0x80008, 0, id];
    for (i, v) in vals.into_iter().enumerate() {
        e.w32(r + i as u32 * 4, v);
    }
    let v = APIphysic::CheckRayIntersection(e, r, l + 0x1c);
    lw(e, 0x74, v);
}
fn SetSpeed(e: &mut Engine, on: bool) {
    let p = e.alloca(0x30);
    let dm = lr(e, 8);
    APIphysic::DMGetParams(e, dm, p);
    if !on {
        if e.rf(p + 8) != 0.0 {
            e.wf(p + 8, 0.0);
            LIBAnimMngr::PlayAnim(e, 0.2, 3, 2, 0);
        }
    } else {
        lw(e, 0x58, 1);
        let s = match e.r32(gb(e) + 0x38) {
            2 => 2.75,
            3 => 3.025,
            _ => 2.5,
        };
        e.wf(p + 8, s);
        LIBAnimMngr::PlayAnim(e, 0.2, 7, 2, 0);
    }
    APIphysic::DMSetParams(e, dm, p);
}
fn fFrontWallDist(e: &mut Engine) -> f32 {
    if lr(e, 0x74) == 0 {
        return 0.0;
    }
    let l = L(e);
    let dx = e.rf(l + 0x1c) - e.rf(l + 0x5c);
    let dz = e.rf(l + 0x24) - e.rf(l + 100);
    APImath::sqrtf(e, dx * dx + dz * dz)
}
fn SetTracking(e: &mut Engine, d: Ptr) {
    let p = e.alloca(0x30);
    let v = [e.r32(d), e.r32(d + 4), e.r32(d + 8)];
    for (i, x) in v.into_iter().enumerate() {
        lw(e, 0x9c + i as u32 * 4, x);
    }
    let dm = lr(e, 4);
    APIphysic::DMGetParams(e, dm, p);
    for (i, x) in v.into_iter().enumerate() {
        e.w32(p + 8 + i as u32 * 4, x);
    }
    APIphysic::DMSetParams(e, dm, p);
}
fn dm(e: &mut Engine, kind: u32, slot: u32, mask: u32, set: &[(u32, u32)], id_at: Option<u32>) {
    let h = APIphysic::DMAdd(e, SELF, kind);
    lw(e, slot, h);
    if !set.is_empty() || id_at.is_some() {
        let p = e.alloca(0x48);
        APIphysic::DMGetParams(e, h, p);
        for &(o, v) in set {
            e.w32(p + o, v);
        }
        if let Some(o) = id_at {
            let id = APIgameobj::GetID(e, SELF, NULL);
            e.w32(p + o, id);
        }
        APIphysic::DMSetParams(e, h, p);
    }
    APIphysic::DMSetContextMask(e, h, mask);
}
fn Create_Physic(e: &mut Engine) {
    APIphysic::CreatePhysic(e, SELF, 0);
    APIphysic::SetFlags(e, SELF, 6);
    APIphysic::SetPrimaryPrim(e, SELF, 900);
    APIphysic::SetWeight(e, SELF, 0.5);
    APIphysic::SetPriority(e, SELF, 3);
    dm(e, 7, 0, 1, &[(0, 0x41a0_0000), (4, 7)], None);
    dm(e, 4, 8, 4, &[(0, 0), (4, 0), (8, 0), (0xc, 1)], None);
    dm(e, 0x1b, 0xc, 8, &[(0x18, 2), (0x1c, 0), (0x20, 0x80008), (0x24, 0)], Some(0x28));
    dm(e, 1, 4, 2, &[(0, 0), (4, 0x41a0_0000), (8, 0), (0xc, 0), (0x10, 0), (0x14, 5)], None);
    let h = APIphysic::DMAdd(e, SELF, 0x1d);
    lw(e, 0x10, h);
    APIphysic::DMSetActive(e, h, 0);
    APIphysic::DMSetContextMask(e, h, 1);
}
fn InitParticles_FootSteps(e: &mut Engine) {
    let t = APIparticle::TypeCreate(e);
    lw(e, 0xa8, t);
    let em = APIparticle::EmitterCreate(e, t as i32);
    lw(e, 0xac, em);
    let t = t as i32;
    let s = e.cstr("FootStep Peabody");
    APIparticle::TypeSetName(e, t, s);
    let s = e.cstr("footprintbrun_invers");
    APIparticle::TypeSetTexture(e, t, s);
    APIparticle::TypeSetInter(e, t, 0x10, 1.0, 1.0, 1.0, 1.0);
    APIparticle::TypeSetFloat(e, t, 2, 15.0);
    APIparticle::TypeSetFloat(e, t, 8, 0.0);
    APIparticle::TypeSetInter(e, t, 0x20, 0.0, 0.0, 0.0, 0.0);
    for k in [1, 2, 4, 8] {
        APIparticle::TypeSetInter(e, t, k, 1.0, 1.0, 0.0, 10.0);
    }
    APIparticle::TypeSetInt(e, t, 4, 0x168);
    APIparticle::TypeSetInt(e, t, 8, 1);
    APIparticle::TypeSetInt(e, t, 1, 0x201);
    APIparticle::EmitterSetJoint(e, em as i32, SELF, 0x7fff_ffff);
}
/// only logs unsupported joints (null_printf dropped)
pub fn fcback_ANIM(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

pub fn ctor(e: &mut Engine, api: Ptr, n: i32) -> LResult<()> {
    if n == 1 {
        ext(e, &[
            (0, fstate_FIRSTMOVE),
            (1, fstate_BASICMOVE_SET),
            (2, fstate_BASICMOVE_LOOP),
            (3, fstate_GHOSTPUSHED),
            (4, fakeghostCB_TIMER_TRACKING),
            (5, fcback_TIMER_ENABLE_GHOSTTOUCHED),
            (6, fcback_END_ANIBUMP),
            (7, fcback_ANIM),
        ]);
        let v = APIcons::GetApiAnim(e); gset(e, pAnim, v);
        let v = APIcons::GetApiGameObj(e); gset(e, pGameObj, v);
        let v = APIcons::GetApiExtra(e); gset(e, pExtra, v);
        let v = APIcons::GetApiPhysic(e); gset(e, pPhysic, v);
        let v = APIcons::GetApiStdlib(e); gset(e, pStdlib, v);
        let v = APIcons::GetApiParticle(e); gset(e, pParticle, v);
        let v = APIcons::GetApiMath(e); gset(e, pMath, v);
        let v = APIcons::GetApiLight(e); gset(e, pLight, v);
        let v = APIcons::GetApiGraphics(e); gset(e, pGraphic, v);
        let v = APIcons::GetApiNewSound(e); gset(e, pSound, v);
        custom(e, pDebug, "debug");
        custom(e, pAnimMngr, "AnimMngr");
        custom(e, pFunction, "ApiCFunctions");
        custom(e, pMiscFunc, "MiscFunc");
        let s = G(e, pGlobalDataFakeGhost);
        APIcons::GetGlobalData(e, s, 0x34c);
        let b = APIgameobj::GetGameBuffer(e, 0, 0x8f8, 1);
        let g = e.r32(G(e, pGlobalDataFakeGhost));
        e.w32(g, b);
    }
    APIcons::SetMessageFunction(e, MyMessageFunction);
    let s = G(e, pLocalDataFakeGhost);
    APIcons::GetLocalData(e, s, 0xb0);
    // srand seed: the f0 at the cvt is TimerGetTime's result (see other ports)
    let t = APIextra::TimerGetTime(e, 0x9000_0000);
    APIstdlib::srand(e, t as i32 as u32);
    LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 2, NULL, 0);
    APIcons::SetInitialState(e, 0);
    Ok(())
}
pub fn dtor(e: &mut Engine, _: i32) -> LResult<()> {
    let t = lr(e, 0x18);
    APIextra::TimerDestroy(e, t);
    let t = lr(e, 0x14);
    APIextra::TimerDestroy(e, t);
    APIgameobj::SetGroupID(e, SELF, 0);
    Ok(())
}

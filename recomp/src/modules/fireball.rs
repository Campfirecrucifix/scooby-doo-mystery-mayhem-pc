//! `fireball`: a thrown fire ball (Ep6 ShermanTech). Waits for 0x1af (attach to the thrower), 0x110
//! (thrown at a target), then jumps onto the target position and explodes on floor contact / when it
//! hits the leader / an object of kind 0xd8. Port of
//! `Game Logic Modules/Ep6_ShermanTech/Objects/FIREBALL/fireball__*/FireBall.c`. The 4 builds share the
//! code; they differ only in the constants of [`BUILDS`] (target ids, sound, latent ids, jump tweaks).
//!
//! The local state function pointer at local +4 (`fireBallST_*_In/_Execute`) is only read by this
//! module; the port stores a small state number there instead of a code address ([`St`]).
#![allow(non_snake_case, non_upper_case_globals, unused_variables)]

use crate::engine::*;
use crate::logic::{CodeFn, LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "fireball",
    data_size: 0x40,
    ctor: fireballConstructor,
    dtor: fireballDestructor,
    register: None,
    init: &[],
    build_init: None,
};

const KEYS: [&str; 4] = ["43fa44d6", "b872b9bb", "3cf11c1e", "458e4c56"];

pub fn build_index(key: &str) -> Option<usize> {
    KEYS.iter().position(|k| *k == key)
}

/// per-build constants of FireBall.c
struct Build {
    /// target id that raises the jump target by 0.2 (and +0.1 with `extra`)
    tgt_hi: u32,
    /// target id that lowers it by 0.5 (and +0.05 with `extra`)
    tgt_lo: u32,
    /// pSound->Play id on impact
    snd: u32,
    /// LaunchLatant id of the explosion
    expl: u32,
    /// MiscFunc_CreateGenericFX id
    fx: u32,
    /// fireBallST_Thrown_In: speed 22.0 and the extra height tweaks (3cf11c1e, 458e4c56)
    extra: bool,
}

const BUILDS: [Build; 4] = [
    Build { tgt_hi: 0x2a00_0036, tgt_lo: 0x2a00_0035, snd: 0x4000_001a, expl: 0x2a80_001d, fx: 0x2a80_00e7, extra: false },
    Build { tgt_hi: 0x2a00_007b, tgt_lo: 0x2a00_007a, snd: 0x4000_0014, expl: 0x2a80_0155, fx: 0x2a80_0125, extra: false },
    Build { tgt_hi: 0x2a00_0008, tgt_lo: 0x2a00_0007, snd: 0x4000_0034, expl: 0x2a80_00c5, fx: 0x2a80_00ba, extra: true },
    Build { tgt_hi: 0x2a00_0056, tgt_lo: 0x2a00_0055, snd: 0x4000_0034, expl: 0x2a80_0011, fx: 0x2a80_0109, extra: true },
];

fn B(e: &Engine) -> &'static Build {
    &BUILDS[e.rt.cur_build()]
}

pub const pExtra: u32 = 0x10000;
pub const pGameObj: u32 = 0x10004;
pub const pPhysic: u32 = 0x10008;
pub const pAnimMngr: u32 = 0x1000c;
pub const pDebug: u32 = 0x10010;
pub const pInput: u32 = 0x10014;
pub const pGame: u32 = 0x10018;
pub const pStdlib: u32 = 0x1001c;
pub const pSound: u32 = 0x10020;
pub const pMiscFunc: u32 = 0x10024;
pub const pAnim: u32 = 0x10028;
pub const pMath: u32 = 0x1002c;
pub const pFunction: u32 = 0x10030;
pub const pParticle: u32 = 0x10034;
pub const pLocalDataFireBall: u32 = 0x10038;
pub const pGlobalDataFireBall: u32 = 0x1003c;

#[inline]
fn G(e: &Engine, a: u32) -> Ptr {
    e.rt.cur_data() + (a - 0x10000)
}
/// local data (0x90 bytes): +0 flags (2 attached, 4 thrown, 8 floor, 0x10 hit leader, 0x20 hit 0xd8),
/// +4 state, +8/+0xc/+0x10/+0x14 DM handles, +0x24 timer, +0x28 look-at, +0x3c floor normal,
/// +0x48 own id, +0x4c parent, +0x50 target, +0x54 hit object, +0x58 JumpOnCoorParam, +0x8c emitter
fn L(e: &Engine) -> Ptr {
    e.r32(G(e, pLocalDataFireBall))
}
/// global data (0x36c bytes): +0 game buffer, +4 particle type, +8 generic FX, +0xc FX descriptor
fn GD(e: &Engine) -> Ptr {
    e.r32(G(e, pGlobalDataFireBall))
}
/// `*(uint *)(*pGlobalDataFireBall + 8)`: the leader in game buffer 0
fn leader(e: &Engine) -> u32 {
    let gb = e.r32(GD(e));
    e.r32(gb + 8)
}
fn flags_and(e: &mut Engine, m: u32) {
    let l = L(e);
    let v = e.r32(l) & m;
    e.w32(l, v);
}
fn flags_or(e: &mut Engine, m: u32) {
    let l = L(e);
    let v = e.r32(l) | m;
    e.w32(l, v);
}

/// values of the state function pointer at local +4
#[derive(Clone, Copy)]
#[repr(u32)]
enum St {
    WaitingIn = 1,
    WaitingExecute,
    CreationIn,
    CreationExecute,
    ThrownIn,
    ThrownExecute,
}
fn set_state(e: &mut Engine, s: St) {
    let l = L(e);
    e.w32(l + 4, s as u32);
}
/// `(**(code **)(pLocalDataFireBall + 4))()`
fn call_state(e: &mut Engine) {
    let l = L(e);
    match e.r32(l + 4) {
        1 => fireBallST_Waiting_In(e),
        2 => fireBallST_Waiting_Execute(e),
        3 => fireBallST_Creation_In(e),
        4 => fireBallST_Creation_Execute(e),
        5 => fireBallST_Thrown_In(e),
        6 => fireBallST_Thrown_Execute(e),
        s => panic!("fireball: bad state function {s:#x}"),
    }
}

/// fireBallST_INITIAL @0x00010d88
pub fn fireBallST_INITIAL(e: &mut Engine, _: u32) -> LResult<u64> {
    let s = e.cstr("FireBall OF DOOM");
    APIgameobj::DisplayInfo(e, 8, s, SELF, 0xc1f);
    APIgameobj::SetContextFlag(e, SELF, 2);
    set_state(e, St::WaitingIn);
    APIgameobj::SetNextState(e, SELF, 1);
    Ok(1)
}
/// fireBallST_ENTRY_POINT @0x00010e38
pub fn fireBallST_ENTRY_POINT(e: &mut Engine, _: u32) -> LResult<u64> {
    call_state(e);
    Ok(0)
}
/// fireBallST_Waiting_In @0x00010e98
fn fireBallST_Waiting_In(e: &mut Engine) {
    set_state(e, St::WaitingExecute);
    call_state(e);
}
/// fireBallST_Waiting_Execute @0x00010ef8
fn fireBallST_Waiting_Execute(e: &mut Engine) {
    if e.r32(L(e)) & 2 != 0 {
        flags_and(e, 0xffff_fffd);
        set_state(e, St::CreationIn);
        call_state(e);
    }
}
/// fireBallST_Creation_In @0x00010f90
fn fireBallST_Creation_In(e: &mut Engine) {
    let l = L(e);
    let parent = e.r32(l + 0x4c);
    APIgameobj::AddChild(e, parent, 6, SELF, 1);
    let parent = e.r32(l + 0x4c);
    APIgameobj::TakeOrientation(e, SELF, parent, 0x7fff_ffff);
    APIphysic::SetFlags(e, SELF, 1);
    fireBallFUNC_StartFire(e);
    set_state(e, St::CreationExecute);
    call_state(e);
}
/// fireBallST_Creation_Execute @0x00011070
fn fireBallST_Creation_Execute(e: &mut Engine) {
    if e.r32(L(e)) & 4 != 0 {
        flags_and(e, 0xffff_fffb);
        set_state(e, St::ThrownIn);
        call_state(e);
    }
}
/// fireBallST_Thrown_In @0x00011108
fn fireBallST_Thrown_In(e: &mut Engine) {
    let l = L(e);
    APIphysic::ResetFlags(e, SELF, 1);
    APIphysic::SetFlags(e, SELF, 6);
    APIphysic::PrimSetIntAttribBits(e, SELF, 0x32a, 0, 8);
    APIphysic::PrimSetIntAttribBits(e, SELF, 0x32a, 0, 4);
    APIphysic::PrimSetIntAttribBits(e, SELF, 0x32a, 7, 0x100);
    APIphysic::PrimSetIntAttribBits(e, SELF, 0x32a, 8, 0x2a);
    APIphysic::PrimSetIntAttribBits(e, SELF, 0x32a, 0, 0x80);
    APIgameobj::LeaveParent(e, SELF, 0);
    APIgameobj::SetContextFlag(e, SELF, 0x16);
    APIgameobj::GetLookAt(e, SELF, 0x7fff_ffff, l + 0x28);
    APIphysic::SetStateChangeNotifyBits(e, SELF, 0xd);
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 5, 3);
    let v = e.alloca(0x10);
    let ld = leader(e);
    APIgameobj::GetPosition(e, ld, 5, v);
    let b = B(e);
    let (x, y, z) = (e.rf(v), e.rf(v + 4), e.rf(v + 8));
    e.wf(l + 0x58, x);
    e.wf(l + 0x5c, y);
    let tgt = e.r32(l + 0x50);
    if tgt == b.tgt_hi {
        let y = e.rf(l + 0x5c) + 0.2;
        e.wf(l + 0x5c, y);
    }
    if tgt == b.tgt_lo {
        let y = e.rf(l + 0x5c) + -0.5;
        e.wf(l + 0x5c, y);
    }
    e.wf(l + 0x60, z);
    e.w32(l + 0x64, 0);
    e.w32(l + 0x68, 0);
    e.w32(l + 0x6c, 0);
    e.w32(l + 0x70, 0x4188_0000);
    if b.extra {
        e.w32(l + 0x70, 0x41b0_0000);
        e.w32(l + 0x78, 0x3f00_0000);
        let tgt = e.r32(l + 0x50);
        if tgt == b.tgt_hi {
            let y = e.rf(l + 0x5c) + 0.1;
            e.wf(l + 0x5c, y);
        }
        if tgt == b.tgt_lo {
            let y = e.rf(l + 0x5c) + 0.05;
            e.wf(l + 0x5c, y);
        }
    }
    e.w32(l + 0x74, 1);
    e.w32(l + 0x78, 0x3f00_0000);
    let dm = e.r32(l + 0xc);
    e.w32(l + 0x7c, dm);
    APIphysic::JumpOnCoor(e, SELF, l + 0x58);
    set_state(e, St::ThrownExecute);
    call_state(e);
}
/// fireBallST_Thrown_Execute @0x00011438
fn fireBallST_Thrown_Execute(e: &mut Engine) {
    let l = L(e);
    let f = e.r32(l);
    if f & 0x10 == 0 {
        if f & 8 == 0 {
            if f & 0x20 != 0 {
                flags_and(e, 0xffff_ffdf);
                let hit = e.r32(l + 0x54);
                if hit != 0 {
                    APIgameobj::SendMessageTo(e, hit, NULL, 0x1ec, 0, NULL);
                }
                e.w32(l + 0x54, 0);
                fireBallFUNC_Explode(e);
            }
        } else {
            let s = B(e).snd;
            APInewSound::Play(e, SELF, s, 0);
            flags_and(e, 0xffff_ffe7);
            fireBallFUNC_ParticleShot(e);
            fireBallFUNC_Explode(e);
        }
    } else {
        flags_and(e, 0xffff_ffef);
        let s = B(e).snd;
        APInewSound::Play(e, SELF, s, 0);
        fireBallFUNC_Explode(e);
    }
}
/// fireBallCB_NotifyColide @0x00011608
pub fn fireBallCB_NotifyColide(e: &mut Engine, p: u32) -> LResult<u64> {
    let o = e.r32(p + 8);
    if o == leader(e) {
        flags_or(e, 0x10);
        Ok(1)
    } else if e.r32(p + 0xc) == 0xd8 {
        let l = L(e);
        e.w32(l + 0x54, o);
        flags_or(e, 0x20);
        Ok(1)
    } else {
        Ok(0)
    }
}
/// fireBallCB_PHYSIC_STATE_CHANGE @0x000116e0
pub fn fireBallCB_PHYSIC_STATE_CHANGE(e: &mut Engine, p: u32) -> LResult<u64> {
    let s = e.r32(p);
    if s & 1 != 0 && s & 0xc == 0 {
        let l = L(e);
        APIphysic::GetFloorNormal(e, SELF, l + 0x3c);
        flags_or(e, 8);
    }
    Ok(0)
}
/// fireBallCB_TIMER @0x00011790
pub fn fireBallCB_TIMER(e: &mut Engine, p: u32) -> LResult<u64> {
    let l = L(e);
    if p == e.r32(l + 0x24) {
        fireBallFUNC_StopFire(e);
        let t = e.r32(l + 0x24);
        APIextra::TimerUnregisterNotify(e, SELF, t);
        let id = APIgameobj::GetID(e, SELF, NULL);
        APIgameobj::KillObject(e, id);
    }
    Ok(0)
}
/// fireBallFUNC_Explode @0x00011860
fn fireBallFUNC_Explode(e: &mut Engine) {
    let v = e.alloca(0x10);
    APIgameobj::GetPosition(e, SELF, 0x7fff_ffff, v);
    let x = B(e).expl;
    let o = APIgameobj::LaunchLatant(e, x);
    APIgameobj::SendMessageTo(e, o, NULL, 0x1ea, 0, NULL);
    APIgameobj::SetTranslation(e, o, v);
    let id = APIgameobj::GetID(e, SELF, NULL);
    APIgameobj::KillObject(e, id);
}
/// fireBallFUNC_ParticleShot @0x00011958
fn fireBallFUNC_ParticleShot(e: &mut Engine) {
    let vel = e.alloca(0x10);
    let pos = e.alloca(0x10);
    let proj = e.alloca(0x10);
    let id = APIgameobj::GetID(e, SELF, NULL);
    APIphysic::GetVelocity(e, id, vel);
    let (x, y, z) = (e.rf(vel), e.rf(vel + 4), e.rf(vel + 8));
    let n = APImath::sqrtf(e, x * x + y * y + z * z);
    if 0.0 < n {
        let n = APImath::sqrtf(e, x * x + y * y + z * z);
        let n = 1.0 / n;
        e.wf(vel, x * n * 0.5);
        e.wf(vel + 4, y * n * 0.5);
        e.wf(vel + 8, z * n * 0.5);
    }
    APIgameobj::GetPosition(e, SELF, 0x7fff_ffff, pos);
    let (px, py, pz) = (e.rf(pos), e.rf(pos + 4) - 0.15, e.rf(pos + 8));
    let g = GD(e);
    e.wf(g + 0x94, px);
    e.wf(g + 0x98, py);
    e.wf(g + 0x9c, pz);
    let l = L(e);
    APImath::CVectorProjectOnto(e, proj, l + 0x28, l + 0x3c);
    let a = [e.rf(l + 0x28) - e.rf(proj), e.rf(l + 0x2c) - e.rf(proj + 4), e.rf(l + 0x30) - e.rf(proj + 8)];
    for i in 0..3 {
        let w = e.r32(l + 0x3c + 4 * i);
        e.w32(g + 0xa0 + 4 * i, w);
    }
    for (i, v) in a.into_iter().enumerate() {
        e.wf(g + 0xac + 4 * i as u32, v);
    }
    let fx = e.r32(g + 8);
    LIBMiscFunc::MiscFunc_PlayGenericFX(e, fx as i32);
}
/// fireBallFUNC_ParticlesInit @0x00011c78 (called by GENERIC_FX through the descriptor at global +0x90)
pub fn fireBallFUNC_ParticlesInit(e: &mut Engine, _pp: u32, t: u32) -> LResult<()> {
    let t = t as i32;
    APIparticle::TypeSetInter(e, t, 0x10, 2.0, 2.0, 0.2, 5.0);
    APIparticle::TypeSetFloat(e, t, 4, 0.01);
    APIparticle::TypeSetFloat(e, t, 2, 10.5);
    APIparticle::TypeSetFloat(e, t, 8, 0.0);
    APIparticle::TypeSetInter(e, t, 0x20, 0.0, 0.0, 0.0, 0.0);
    for w in [1, 2, 4, 8] {
        APIparticle::TypeSetInter(e, t, w, 1.0, 1.0, 1.0, 5.0);
    }
    APIparticle::TypeSetInt(e, t, 4, 0xb4);
    APIparticle::TypeSetInt(e, t, 8, 1);
    APIparticle::TypeSetInt(e, t, 1, 0x81);
    APIparticle::TypeSetVec(e, t, 1, 0.0, 0.0, 0.0);
    Ok(())
}
/// fireBallFUNC_SetClipping @0x00011ef8
fn fireBallFUNC_SetClipping(e: &mut Engine) {
    let c = e.alloca(0x30);
    APIgameobj::GetClippingParams(e, SELF, c);
    for (o, v) in [
        (0, 0x3dcc_cccd),
        (4, 0xbf80_0000),
        (0x10, 0xbf80_0000),
        (0x1c, 0xbf80_0000),
        (0xc, 0),
        (8, 0x47),
        (0x18, 0x40),
        (0x14, 0x6a7),
        (0x24, 0x6a7),
        (0x20, 0x6a7),
    ] {
        e.w32(c + o, v);
    }
    APIgameobj::SetClippingParams(e, SELF, c, 0);
}
/// fireBallFUNC_CreateFireType @0x00011fc0
fn fireBallFUNC_CreateFireType(e: &mut Engine) {
    let t = APIparticle::TypeCreate(e);
    let g = GD(e);
    e.w32(g + 4, t);
    let s = e.cstr("Fire_bubble_blue");
    APIparticle::TypeSetTexture(e, t as i32, s);
    let s = e.cstr("FireBall_FIRE");
    APIparticle::TypeSetName(e, t as i32, s);
    let t = e.r32(GD(e) + 4);
    fireBallFUNC_SetParticle_FIRE(e, t);
}
/// fireBallFUNC_CreateFireParticle @0x00012088
fn fireBallFUNC_CreateFireParticle(e: &mut Engine) {
    let t = e.r32(GD(e) + 4);
    let em = APIparticle::EmitterCreate(e, t as i32);
    let l = L(e);
    e.w32(l + 0x8c, em);
    APIparticle::EmitterSetJoint(e, em as i32, SELF, 0x7fff_ffff);
}
fn emitter(e: &Engine) -> i32 {
    e.r32(L(e) + 0x8c) as i32
}
/// fireBallFUNC_StartFire @0x00012108
fn fireBallFUNC_StartFire(e: &mut Engine) {
    let em = emitter(e);
    APIparticle::EmitterStart(e, em, 1);
}
/// fireBallFUNC_StopFire @0x00012158
fn fireBallFUNC_StopFire(e: &mut Engine) {
    let em = emitter(e);
    APIparticle::EmitterStop(e, em);
}
/// fireBallFUNC_DestroyFireParticle @0x000121a0
fn fireBallFUNC_DestroyFireParticle(e: &mut Engine) {
    let em = emitter(e);
    APIparticle::EmitterDestroy(e, em);
}
/// fireBallFUNC_DestroyFireType @0x000121e8
fn fireBallFUNC_DestroyFireType(e: &mut Engine) {
    let t = e.r32(GD(e) + 4) as i32;
    APIparticle::TypeDestroy(e, t);
}
/// fireBallFUNC_SetParticle_FIRE @0x00012230. The original takes the pParticle table and calls its
/// slots 0x10 (TypeSetInt), 0x18 (TypeSetFloat), 0x1c (TypeSetVec), 0xc (TypeSetInter); the only
/// caller passes pParticle, so the port calls the API directly.
fn fireBallFUNC_SetParticle_FIRE(e: &mut Engine, t: u32) {
    let t = t as i32;
    for (w, v) in [(8, 5), (1, 2), (4, 0x168), (0x10, 0), (0x20, 0), (0x40, 0), (0x80, 0), (0x100, 0)] {
        APIparticle::TypeSetInt(e, t, w, v);
    }
    for (w, v) in [(1, 0.0), (4, 0.05), (2, 0.65), (8, 3.0)] {
        APIparticle::TypeSetFloat(e, t, w, v);
    }
    APIparticle::TypeSetVec(e, t, 1, 0.0, -7.0, 0.0);
    APIparticle::TypeSetVec(e, t, 2, 0.0, 0.0, 0.0);
    let f = f32::from_bits;
    for (w, a, b, c, d) in [
        (1, 0x3c23_d70a, 0x3f00_0000, 0x3c23_d70a, 0x3e99_999a),
        (2, 0x3c23_d70a, 0x3f40_0000, 0x3c23_d70a, 0x3e99_999a),
        (4, 0x3c23_d70a, 0x3f40_0000, 0x3c23_d70a, 0x3e99_999a),
        (8, 0, 0, 0, 0x3e99_999a),
        (0x10, 0x3fa0_0000, 0x3f00_0000, 0x3d4c_cccd, 0x3e99_999a),
        (0x20, 0x3e4c_cccd, 0x3e4c_cccd, 0x3e4c_cccd, 0x3e99_999a),
    ] {
        APIparticle::TypeSetInter(e, t, w, f(a), f(b), f(c), f(d));
    }
}
/// fireBallFUNC_MessageHandler @0x00012598
pub fn fireBallFUNC_MessageHandler(e: &mut Engine, m: u32, p2: u32, _: Ptr) -> LResult<u64> {
    let l = L(e);
    if m == 0x110 {
        e.w32(l + 0x50, p2);
        flags_or(e, 4);
    } else if m == 0x1af {
        e.w32(l + 0x4c, p2);
        let id = APIgameobj::GetID(e, SELF, NULL);
        e.w32(l + 0x48, id);
        let c = e.alloca(0x30);
        APIgameobj::GetClippingParams(e, SELF, c);
        for o in [4, 0x10, 0x1c] {
            e.w32(c + o, 0xbf80_0000);
        }
        APIgameobj::SetClippingParams(e, SELF, c, 0);
        flags_or(e, 2);
    }
    Ok(0)
}
/// fireBallFUNC_CreatePhysic @0x000126f8
fn fireBallFUNC_CreatePhysic(e: &mut Engine) {
    let l = L(e);
    let p = e.alloca(0x20);
    APIphysic::CreatePhysic(e, SELF, 0);
    APIphysic::SetFlags(e, SELF, 6);
    APIphysic::SetPriority(e, SELF, 10);
    // (local slot, DM kind, written params, context mask)
    let dms: [(u32, u32, &[(u32, u32)], u32); 4] = [
        (0xc, 0, &[(0, 0), (4, 0xc120_0000), (8, 0)], 2),
        (8, 4, &[(0, 0), (4, 0), (8, 0x40a0_0000), (0xc, 1)], 8),
        (0x14, 7, &[(0, 0), (4, 5)], 0x10),
        (0x10, 8, &[(0, 0x4000_0000), (4, 0x4049_0fdb)], 4),
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
/// fireballConstructor @0x00012a08
pub fn fireballConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    if instanceCount == 1 {
        APIcons::SetExternalFunction(e, 0, fireBallST_INITIAL);
        APIcons::SetExternalFunction(e, 1, fireBallST_ENTRY_POINT);
        APIcons::SetExternalFunction(e, 2, fireBallCB_NotifyColide);
        APIcons::SetExternalFunction(e, 3, fireBallCB_PHYSIC_STATE_CHANGE);
        APIcons::SetExternalFunction(e, 4, fireBallCB_TIMER);
        let custom = |e: &mut Engine, n: &str| {
            let s = e.cstr(n);
            APIcons::GetApiCustom(e, s)
        };
        let v = custom(e, "AnimMngr");
        e.w32(G(e, pAnimMngr), v);
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
        let v = APIcons::GetApiStdlib(e);
        e.w32(G(e, pStdlib), v);
        let v = APIcons::GetApiNewSound(e);
        e.w32(G(e, pSound), v);
        let v = custom(e, "MiscFunc");
        e.w32(G(e, pMiscFunc), v);
        let v = APIcons::GetApiAnim(e);
        e.w32(G(e, pAnim), v);
        let v = APIcons::GetApiMath(e);
        e.w32(G(e, pMath), v);
        let v = APIcons::GetApiParticle(e);
        e.w32(G(e, pParticle), v);
        let v = custom(e, "ApiCFunctions");
        e.w32(G(e, pFunction), v);
        let slot = G(e, pGlobalDataFireBall);
        APIcons::GetGlobalData(e, slot, 0x36c);
        let b = APIgameobj::GetGameBuffer(e, 0, 0x8f8, 1);
        let g = GD(e);
        e.w32(g, b);
        e.w32(g + 0xc, 0x4000_0000);
        let t = e.code_ptr(CodeFn::Particle(fireBallFUNC_ParticlesInit));
        e.w32(g + 0x90, t);
        let s = e.cstr("Fire Scratch");
        LIBMiscFunc::MiscFunc_StrncpySafe(e, g + 0x50, s, 0x40);
        let s = e.cstr("FireCampB");
        LIBMiscFunc::MiscFunc_StrncpySafe(e, g + 0x10, s, 0x40);
        e.w32(g + 0xb8, 0);
        let fx = B(e).fx;
        let h = LIBMiscFunc::MiscFunc_CreateGenericFX(e, fx, g + 0xc, 1);
        e.w32(g + 8, h);
        fireBallFUNC_CreateFireType(e);
        let x = B(e).expl;
        let o = APIgameobj::LaunchLatant(e, x);
        // quirk: the translation vector is uninitialised stack in the original
        let v = e.alloca(0x10);
        APIgameobj::SetTranslation(e, o, v);
    }
    let slot = G(e, pLocalDataFireBall);
    APIcons::GetLocalData(e, slot, 0x90);
    fireBallFUNC_CreatePhysic(e);
    fireBallFUNC_SetClipping(e);
    fireBallFUNC_CreateFireParticle(e);
    let t = APIextra::TimerCreate(e);
    let l = L(e);
    e.w32(l + 0x24, t);
    APIextra::TimerSetFrequency(e, t, 5.0);
    APIcons::SetMessageFunction(e, fireBallFUNC_MessageHandler);
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 3, 2);
    APIcons::SetInitialState(e, 0);
    Ok(())
}
/// fireballDestructor @0x00012ea0
pub fn fireballDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    let t = e.r32(L(e) + 0x24);
    APIextra::TimerDestroy(e, t);
    fireBallFUNC_DestroyFireParticle(e);
    if instanceCount == 0 {
        let fx = e.r32(GD(e) + 8);
        LIBMiscFunc::MiscFunc_DestroyGenericFX(e, fx as i32);
        fireBallFUNC_DestroyFireType(e);
    }
    Ok(())
}

//! `ghost_lock`: EP1L03A ghost locks 0x2a0000e5 (GB2+0xc bit 0x2000) and 0x2a0000e6 (bit 0x4000).
//! Msg 0xdf (beam) -> state OFF: when no hero is busy (GB0+0x14 & 0x0e20090c == 0) sets its GB2+0xc bit,
//! fades out and sends msg 0x54 to the L03A camera 0x2a00004a. 0xc1 (checkpoint reload) undoes it,
//! 0xd4 latches it. Port of `Ep1_Library/Objects/GHOST_LOCK/Level3A/ghost_lock__cf7d0bf2`
//! (Ghost_Lock_Base.c + Ghost_Lock.c, 27 functions). Build cf7d0bf2.
#![allow(non_snake_case, non_upper_case_globals, unused_variables)]

use crate::engine::*;
use crate::logic::{LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "ghost_lock",
    // port layout: TmpGhostLockMap 0x10000..0x10030 (max 6 entries over all builds), then the API pointers
    data_size: 0x58,
    ctor: ghost_lockConstructor,
    dtor: ghost_lockDestructor,
    register: None,
    init: &[],
    build_init: Some(build_init),
};

/// Local (Ghost_Lock.c) variant of a build.
#[derive(Clone, Copy, PartialEq)]
enum Local {
    /// constructor only
    None,
    /// EP1L03A: msg 0xdf ChangeClipping; CB_ANIM_LF: disappear done in GB0+0x3c == 10 by 0x2a0000e5 -> 0x1f2 to 0x2a000125
    L03a,
    /// EP2L01C: same shape, cb test GB0+0x3c == 0xb (no id test) -> 0x1f2 to 0x2a0003a9; empty 0xc1 branch
    L01c,
    /// EP6L04A: msg 0x166 PrimResetIntAttribBits(0x3e4, 1, 2)
    L04a,
}

/// Per-build constants (all builds share Ghost_Lock_Base.c, WP-L10).
struct Build {
    sha8: &'static str,
    /// TmpGhostLockMap {object id, GB2+0xc bit} (= the first .text words of the carved .o; WP-P17: the
    /// single-entry builds had bit 0, so their lock never set/tested its door bit)
    map: &'static [(u32, u32)],
    /// msg 0x54 target (level camera)
    cam: u32,
    /// heroes hidden while the lock vanishes
    vis: [u32; 2],
    /// msg 0x14f target on checkpoint reload
    s14f: u32,
    /// anim ids: disappear (also GetDuration), attack, stand (AnimMngr), stand (APIanim::Play)
    dis: i32,
    atk: i32,
    stand: i32,
    play: i32,
    local: Local,
}

const fn b(sha8: &'static str, map: &'static [(u32, u32)], cam: u32, vis: [u32; 2], s14f: u32, dis: i32, atk: i32, stand: i32, play: i32, local: Local) -> Build {
    Build { sha8, map, cam, vis, s14f, dis, atk, stand, play, local }
}

static BUILDS: [Build; 10] = [
    b("cf7d0bf2", &[(0x2a0000e5, 0x2000), (0x2a0000e6, 0x4000)], 0x2a00004a, [0x2a0000ef, 0x2a0000f0], 0x2a0000ed, 3, 4, 2, 1, Local::L03a),
    b("3415d130", &[(0x2a000125, 0x20000), (0x2a000126, 0x10000), (0x2a000127, 0x8000)], 0x2a000020, [0x2a000083, 0x2a000084], 0x2a000087, 2, 3, 1, 4, Local::None),
    b("1d6c6630", &[(0x2a000025, 0x1000000)], 0x2a000027, [0x2a00004d, 0x2a00004c], 0x2a00004f, 3, 4, 2, 1, Local::None),
    b("c20b6677", &[(0x2a000136, 0x8000)], 0x2a000037, [0x2a00004c, 0x2a00004b], 0x2a000050, 2, 3, 1, 4, Local::L01c),
    b("635b7f3b", &[(0x2a000103, 0x40000)], 0x2a0000b3, [0x2a0000ab, 0x2a0000aa], 0x2a0000af, 2, 3, 1, 4, Local::None),
    b("1e7007c9", &[(0x2a0000f0, 0x200000)], 0x2a00001e, [0x2a00002c, 0x2a00002b], 0x2a000030, 2, 3, 1, 4, Local::None),
    b("8907fa5b", &[(0x2a000138, 0x80000)], 0x2a000017, [0x2a00002d, 0x2a00002c], 0x2a000031, 2, 3, 1, 4, Local::None),
    b("086cdde6", &[(0x2a00013d, 0x4000)], 0x2a000004, [0x2a000026, 0x2a000025], 0x2a00002a, 3, 4, 2, 1, Local::None),
    b("041f00a7", &[(0x2a0001c3, 0x10000), (0x2a0001c4, 0x20000), (0x2a0001c5, 0x80000), (0x2a0001c6, 0x100000), (0x2a0001c7, 0x200000), (0x2a0001c8, 0x8000000)], 0x2a00001f, [0x2a000036, 0x2a000035], 0x2a00003a, 2, 3, 1, 4, Local::L04a),
    b("43e3f9f6", &[(0x2a000140, 0x400000), (0x2a000141, 0x800000), (0x2a000142, 0x1000000), (0x2a000143, 0x80000000)], 0x2a00005c, [0x2a00007b, 0x2a00007a], 0x2a00007f, 2, 1, 3, 4, Local::None),
];

fn build_init(m: &mut Mem, data: Ptr, build: usize) {
    for (i, &(id, bit)) in BUILDS[build].map.iter().enumerate() {
        m.w32(data + i as u32 * 8, id);
        m.w32(data + i as u32 * 8 + 4, bit);
    }
}

fn B(e: &Engine) -> &'static Build {
    &BUILDS[e.rt.cur_build()]
}

pub fn build_index(key: &str) -> Option<usize> {
    BUILDS.iter().position(|b| b.sha8 == key)
}

pub const TmpGhostLockMap: u32 = 0x10000;
pub const pExtra: u32 = 0x10030;
pub const pGameObj: u32 = 0x10034;
pub const pPhysic: u32 = 0x10038;
pub const pAnim: u32 = 0x1003c;
pub const pParticle: u32 = 0x10040;
pub const pSound: u32 = 0x10044;
pub const pAnimMngr: u32 = 0x10048;
pub const pMiscFunc: u32 = 0x1004c;
pub const pLocalDataGhost_Lock_Base: u32 = 0x10050;
pub const pGlobalDataGhost_Lock_Base: u32 = 0x10054;

#[inline]
fn G(e: &Engine, a: u32) -> Ptr {
    e.rt.cur_data() + (a - 0x10000)
}
/// local data byte offset
fn L(e: &Engine, o: u32) -> Ptr {
    e.r32(G(e, pLocalDataGhost_Lock_Base)) + o
}
fn lr(e: &Engine, o: u32) -> u32 {
    e.r32(L(e, o))
}
fn lw(e: &mut Engine, o: u32, v: u32) {
    let a = L(e, o);
    e.w32(a, v);
}
/// global data word
fn gr(e: &Engine, o: u32) -> u32 {
    e.r32(e.r32(G(e, pGlobalDataGhost_Lock_Base)) + o)
}
fn gw(e: &mut Engine, o: u32, v: u32) {
    let a = e.r32(G(e, pGlobalDataGhost_Lock_Base)) + o;
    e.w32(a, v);
}
/// GB2+0xc
fn gb2c(e: &Engine) -> Ptr {
    gr(e, 4) + 0xc
}

/// 0x00010d80 state 0
pub fn Ghost_Lock_BaseST_FIRSTMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    lw(e, 0x68, 0);
    if e.r32(gb2c(e)) & lr(e, 8) != 0 {
        lw(e, 100, 1);
        APIgameobj::KillObject(e, SELF);
    }
    lw(e, 100, 0);
    let t = gr(e, 8);
    LIBMiscFunc::MiscFunc_SetClipping(e, SELF, t, 0);
    Ghost_Lock_BaseFUNC_BASICMOVE_SET(e);
    Ok(0)
}

/// 0x00010e68
pub fn Ghost_Lock_BaseFUNC_BASICMOVE_SET(e: &mut Engine) {
    Ghost_Lock_BaseFUNC_Start_Particle_Stand(e);
    let b = B(e);
    LIBAnimMngr::PlayAnim(e, 0.0, b.stand, 2, 0);
    APInewSound::Play(e, SELF, 0x32, 0);
    APIanim::Play(e, SELF, b.play as u32, 2, 0.0);
    APIgameobj::SetNextState(e, SELF, 1);
}

/// 0x00010f30 state 1
pub fn Ghost_Lock_BaseST_BASICMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

/// 0x00010f60 state 2
pub fn Ghost_Lock_BaseST_OFF(e: &mut Engine, _: u32) -> LResult<u64> {
    if e.r32(gr(e, 0) + 0x14) & 0xe20090c == 0 {
        let a = gb2c(e);
        let v = e.r32(a) | lr(e, 8);
        e.w32(a, v);
        APIgameobj::SetGroupID(e, SELF, 6);
        APInewSound::StopSound(e, SELF, 0x32);
        APInewSound::Play(e, SELF, 0x33, 0);
        Ghost_Lock_BaseFUNC_Disappear(e);
        let t = gr(e, 8);
        LIBMiscFunc::MiscFunc_SetClipping(e, SELF, t, 0);
        APIgameobj::SetNextState(e, SELF, 1);
    }
    Ok(0)
}

/// 0x000110a8 callback 3
pub fn Ghost_Lock_BaseCB_NOTIFY_COLLIDE_OBJ(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

/// 0x000110d8 (callback 4 of the base, called by the local one)
pub fn Ghost_Lock_BaseCB_ANIM_LF(e: &mut Engine, _: u32) -> LResult<u64> {
    // builds test disappear then attack, 43e3f9f6 attack then disappear (ids differ: same result)
    let a = lr(e, 0) as i32;
    let b = B(e);
    if a == b.dis {
        APInewSound::StopSound(e, SELF, 0x33);
        APIgameobj::Sleep(e, SELF, -1.0);
    } else if a == b.atk {
        LIBAnimMngr::PlayAnim(e, 0.0, b.stand, 2, 0);
    }
    Ok(1)
}

/// 0x000111b8
pub fn Ghost_Lock_BaseFUNC_MessageHandler(e: &mut Engine, param_1: u32, _s: u32, _d: Ptr) -> LResult<u64> {
    if param_1 == 199 {
        if e.r32(gb2c(e)) & lr(e, 8) == 0 {
            let r = LIBAnimMngr::PlayAnim(e, 0.1, B(e).atk, 1, 4);
            lw(e, 0, r as u32);
            Ghost_Lock_BaseFUNC_Start_Particle_Attack(e);
        }
    } else if param_1 < 200 {
        if param_1 == 0x50 {
            if lr(e, 0x68) != 0 {
                lw(e, 0x68, 0);
                let v = B(e).vis;
                APIgameobj::SetGameObjectVisible(e, v[0], 1);
                APIgameobj::SetGameObjectVisible(e, v[1], 1);
                // quirk kept: the particle type ids are used as object ids
                let (a, b) = (gr(e, 8), gr(e, 0xc));
                APIgameobj::SendMessageTo(e, a, NULL, 0xd2, 0x208, NULL);
                APIgameobj::SendMessageTo(e, b, NULL, 0xd2, 0x208, NULL);
            }
        } else if param_1 == 0xc1 && lr(e, 100) == 0 {
            APIgameobj::SendMessageTo(e, B(e).s14f, NULL, 0x14f, 0, NULL);
            lw(e, 100, 0);
            APIgameobj::SetGroupID(e, SELF, 0);
            APIgameobj::Sleep(e, SELF, 0.0);
            APIphysic::ResetFlags(e, SELF, 1);
            APIphysic::PrimResetIntAttribBits(e, SELF, 0x3e4, 0, 2);
            let a = gb2c(e);
            let v = e.r32(a) & !lr(e, 8);
            e.w32(a, v);
            Ghost_Lock_BaseFUNC_BASICMOVE_SET(e);
        }
    } else if param_1 == 0xd4 {
        if e.r32(gb2c(e)) & lr(e, 8) != 0 {
            lw(e, 100, 1);
            APIgameobj::SetGroupID(e, SELF, 0);
        }
    } else if param_1 == 0xdf && lr(e, 100) == 0 {
        let (a, b) = (gr(e, 8), gr(e, 0xc));
        APIgameobj::SendMessageTo(e, a, NULL, 0xd2, 0x104, NULL);
        APIgameobj::SendMessageTo(e, b, NULL, 0xd2, 0x104, NULL);
        LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 2, NULL, 0);
        APIgameobj::SetNextState(e, SELF, 2);
    }
    Ok(0)
}

/// 0x000115e0
pub fn Ghost_Lock_BaseFUNC_Create_Physic(e: &mut Engine) {
    APIphysic::CreatePhysic(e, SELF, 0);
    APIphysic::SetFlags(e, SELF, 2);
}

fn new_type(e: &mut Engine, slot: u32, tex: &str, name: &str, set: fn(&mut Engine, i32)) {
    let t = APIparticle::TypeCreate(e);
    gw(e, slot, t);
    let s = e.cstr(tex);
    APIparticle::TypeSetTexture(e, t as i32, s);
    let s = e.cstr(name);
    APIparticle::TypeSetName(e, t as i32, s);
    set(e, t as i32);
    APIparticle::TypeSetInt(e, t as i32, 1, 1);
}

/// 0x00011640
pub fn Ghost_Lock_BaseFUNC_Create_Type(e: &mut Engine) {
    new_type(e, 8, "water_splash", "Ghost_Lock_Base_Particle_Stand", Ghost_Lock_BaseFUNC_SetParticle_Particle_Stand);
    new_type(e, 0xc, "water_splash", "Ghost_Lock_Base_Particle_Attack", Ghost_Lock_BaseFUNC_SetParticle_Particle_Attack);
    new_type(e, 0x10, "flash2", "Ghost_Lock_Base_Particle_Fade", Ghost_Lock_BaseFUNC_SetParticle_Particle_Fade);
}

/// 0x000118a8
pub fn Ghost_Lock_BaseFUNC_Create_Particle(e: &mut Engine) {
    let v = e.alloca(0xc);
    e.wf(v, 0.0);
    e.wf(v + 4, 0.0);
    e.wf(v + 8, 0.0);
    for (g, l) in [(8, 0xc), (0xc, 0x10), (0x10, 0x14)] {
        let t = gr(e, g);
        let em = APIparticle::EmitterCreate(e, t as i32);
        lw(e, l, em);
        APIparticle::EmitterSetJoint(e, em as i32, SELF, 0x6fff_ffff);
        APIgameobj::AddChild(e, SELF, 1, em, 1);
        APIgameobj::SetTranslation(e, em, v);
    }
}

/// 0x00011ad8
pub fn Ghost_Lock_BaseFUNC_Start_Particle_Stand(e: &mut Engine) {
    let em = lr(e, 0xc);
    APIparticle::EmitterStart(e, em as i32, 1);
}
/// 0x00011b28
pub fn Ghost_Lock_BaseFUNC_Start_Particle_Attack(e: &mut Engine) {
    let em = lr(e, 0x10);
    APIparticle::EmitterStart(e, em as i32, 0);
}
/// 0x00011b78
pub fn Ghost_Lock_BaseFUNC_Start_Particle_Fade(e: &mut Engine) {
    let em = lr(e, 0x14);
    APIparticle::EmitterStart(e, em as i32, 0);
}
/// 0x00011bc8
pub fn Ghost_Lock_BaseFUNC_Stop_Particle_Stand(e: &mut Engine) {
    let em = lr(e, 0xc);
    APIparticle::EmitterStop(e, em as i32);
}
/// 0x00011c10
pub fn Ghost_Lock_BaseFUNC_Destroy_Particle(e: &mut Engine) {
    for o in [0xc, 0x10, 0x14] {
        let em = lr(e, o);
        APIparticle::EmitterDestroy(e, em as i32);
    }
}
/// 0x00011c98
pub fn Ghost_Lock_BaseFUNC_Destroy_Type(e: &mut Engine) {
    for o in [8, 0xc, 0x10] {
        let t = gr(e, o);
        APIparticle::TypeDestroy(e, t as i32);
    }
}

/// Shared shape of the three SetParticle functions (ints, floats(1,4,2,8), vecs, inters as raw bits).
fn set_particle(e: &mut Engine, t: i32, kind: i32, f4: u32, f2: u32, inter: [[u32; 4]; 6]) {
    for (k, v) in [(8, kind), (1, 1), (4, 0), (0x10, 0), (0x20, 0), (0x40, 0), (0x80, 0), (0x100, 0)] {
        APIparticle::TypeSetInt(e, t, k, v);
    }
    for (k, v) in [(1, 0), (4, f4), (2, f2), (8, 0)] {
        APIparticle::TypeSetFloat(e, t, k, f32::from_bits(v));
    }
    APIparticle::TypeSetVec(e, t, 1, 0.0, 0.0, 0.0);
    APIparticle::TypeSetVec(e, t, 2, 0.0, 0.0, 0.0);
    for (k, i) in [1, 2, 4, 8, 0x10, 0x20].into_iter().zip(inter) {
        let f = i.map(f32::from_bits);
        APIparticle::TypeSetInter(e, t, k, f[0], f[1], f[2], f[3]);
    }
}

/// 0x00011d20
pub fn Ghost_Lock_BaseFUNC_SetParticle_Particle_Stand(e: &mut Engine, t: i32) {
    set_particle(e, t, 2, 0x3f00_0000, 0x4040_0000, [
        [0x3c23d70a, 0x3f000000, 0, 0x3fc00000],
        [0x3c23d70a, 0x3f4ccccd, 0, 0x3fc00000],
        [0x3c23d70a, 0x3f800000, 0, 0x3fc00000],
        [0; 4],
        [0x3c23d70a, 0x40200000, 0x40c00000, 0x3fc00000],
        [0; 4],
    ]);
}
/// 0x00012028
pub fn Ghost_Lock_BaseFUNC_SetParticle_Particle_Attack(e: &mut Engine, t: i32) {
    set_particle(e, t, 3, 0x3fc0_0000, 0x4000_0000, [
        [0x3c23d70a, 0x3f000000, 0x3c23d70a, 0x3fc00000],
        [0x3c23d70a, 0x3f4ccccd, 0x3c23d70a, 0x3fc00000],
        [0x3c23d70a, 0x3f800000, 0x3c23d70a, 0x3fc00000],
        [0; 4],
        [0x3c23d70a, 0x40900000, 0x3c23d70a, 0x3e000000],
        [0; 4],
    ]);
}
/// 0x00012350
pub fn Ghost_Lock_BaseFUNC_SetParticle_Particle_Fade(e: &mut Engine, t: i32) {
    set_particle(e, t, 4, 0x4040_0000, 0x3f40_0000, [
        [0x3c23d70a, 0x3f800000, 0x3c23d70a, 0x3eb33333],
        [0x3c23d70a, 0x3f800000, 0x3c23d70a, 0x3eb33333],
        [0x3c23d70a, 0x3f800000, 0x3c23d70a, 0x3eb33333],
        [0; 4],
        [0x3c23d70a, 0x40b00000, 0x3c23d70a, 0x3dcccccd],
        [0; 4],
    ]);
}

/// 0x00012680
pub fn Ghost_Lock_BaseFUNC_Disappear(e: &mut Engine) {
    APIgameobj::SetGroupID(e, SELF, 6);
    APIphysic::PrimSetIntAttribBits(e, SELF, 0x3e4, 0, 2);
    LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 2, NULL, 0);
    let a = gb2c(e);
    let v = e.r32(a) | lr(e, 8);
    e.w32(a, v);
    lw(e, 0x18, 5);
    let p = L(e, 0x1c);
    APIgameobj::GetPosition(e, SELF, 4, p);
    let p = L(e, 0x28);
    APIgameobj::GetPosition(e, SELF, 2, p);
    let d = APIanim::GetDuration(e, SELF, B(e).dis as u32);
    lw(e, 0x44, (d + 1.5) as i32 as u32);
    lw(e, 0x54, 1);
    lw(e, 0x58, 0);
    lw(e, 0x5c, 0xc02181);
    let p = L(e, 0x18);
    let b = B(e);
    APIgameobj::SendMessageTo(e, b.cam, NULL, 0x54, 0, p);
    APIgameobj::SetGameObjectVisible(e, b.vis[0], 0);
    APIgameobj::SetGameObjectVisible(e, b.vis[1], 0);
    lw(e, 0x68, 1);
    APIphysic::SetFlags(e, SELF, 1);
    Ghost_Lock_BaseFUNC_Stop_Particle_Stand(e);
    let r = LIBAnimMngr::PlayAnim(e, 0.25, B(e).dis, 1, 4);
    lw(e, 0, r as u32);
    Ghost_Lock_BaseFUNC_Start_Particle_Fade(e);
    Ghost_Lock_BaseFUNC_Start_Particle_Attack(e);
}

/// 0x00012918
pub fn ghost_lock_baseConstructor(e: &mut Engine, api: Ptr, param_2: i32) {
    if param_2 == 1 {
        APIcons::SetExternalFunction(e, 0, Ghost_Lock_BaseST_FIRSTMOVE);
        APIcons::SetExternalFunction(e, 1, Ghost_Lock_BaseST_BASICMOVE);
        APIcons::SetExternalFunction(e, 2, Ghost_Lock_BaseST_OFF);
        APIcons::SetExternalFunction(e, 3, Ghost_Lock_BaseCB_NOTIFY_COLLIDE_OBJ);
        APIcons::SetExternalFunction(e, 4, Ghost_Lock_BaseCB_ANIM_LF);
        let tables: [(u32, fn(&mut Engine) -> Ptr); 6] = [
            (pAnim, |e| APIcons::GetApiAnim(e)),
            (pGameObj, |e| APIcons::GetApiGameObj(e)),
            (pExtra, |e| APIcons::GetApiExtra(e)),
            (pPhysic, |e| APIcons::GetApiPhysic(e)),
            (pSound, |e| APIcons::GetApiNewSound(e)),
            (pParticle, |e| APIcons::GetApiParticle(e)),
        ];
        for (slot, get) in tables {
            let v = get(e);
            let a = G(e, slot);
            e.w32(a, v);
        }
        for (slot, name) in [(pAnimMngr, "AnimMngr"), (pMiscFunc, "MiscFunc")] {
            let n = e.cstr(name);
            let v = APIcons::GetApiCustom(e, n);
            let a = G(e, slot);
            e.w32(a, v);
        }
        let slot = G(e, pGlobalDataGhost_Lock_Base);
        APIcons::GetGlobalData(e, slot, 0x14);
        let b = APIgameobj::GetGameBuffer(e, 0, 0x8f8, 1);
        gw(e, 0, b);
        let b = APIgameobj::GetGameBuffer(e, 2, 0x158, 1);
        gw(e, 4, b);
        Ghost_Lock_BaseFUNC_Create_Type(e);
    }
    let slot = G(e, pLocalDataGhost_Lock_Base);
    APIcons::GetLocalData(e, slot, 0x6c);
    APIcons::SetMessageFunction(e, Ghost_Lock_BaseFUNC_MessageHandler);
    Ghost_Lock_BaseFUNC_Create_Physic(e);
    Ghost_Lock_BaseFUNC_Create_Particle(e);
    let id = APIgameobj::GetID(e, SELF, NULL);
    for i in 0..B(e).map.len() as u32 {
        let m = G(e, TmpGhostLockMap) + i * 8;
        if id == e.r32(m) {
            let (a, b) = (e.r32(m), e.r32(m + 4));
            lw(e, 4, a);
            lw(e, 8, b);
        }
    }
    LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 2, NULL, 0);
    APIcons::SetInitialState(e, 0);
}

/// 0x00012c98
pub fn ghost_lock_baseDestructor(e: &mut Engine, param_1: i32) {
    Ghost_Lock_BaseFUNC_Stop_Particle_Stand(e);
    Ghost_Lock_BaseFUNC_Destroy_Particle(e);
    if param_1 == 0 {
        Ghost_Lock_BaseFUNC_Destroy_Type(e);
    }
}

/// 0x00012d08
pub fn Ghost_Lock_Local_FUNC_MessageHandler(e: &mut Engine, param_1: u32, s: u32, d: Ptr) -> LResult<u64> {
    // L01C: `if 0xc1 && GB2+0x1c & 0x10 {}` (empty) before the 0xdf test
    if param_1 == 0xdf {
        LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 2, NULL, 0);
    }
    Ghost_Lock_BaseFUNC_MessageHandler(e, param_1, s, d)
}

/// 0x00012dc0 callback 4
pub fn Ghost_Lock_Local_CB_ANIM_LF(e: &mut Engine, p: u32) -> LResult<u64> {
    if B(e).local == Local::L01c {
        if lr(e, 0) == 2 && e.r32(gr(e, 0) + 0x3c) == 0xb {
            APIgameobj::SendMessageTo(e, 0x2a00_03a9, NULL, 0x1f2, 0, NULL);
        }
    } else if lr(e, 0) == 3 && e.r32(gr(e, 0) + 0x3c) == 10 && APIgameobj::GetID(e, SELF, NULL) == 0x2a00_00e5 {
        APIgameobj::SendMessageTo(e, 0x2a00_0125, NULL, 0x1f2, 0, NULL);
    }
    Ghost_Lock_BaseCB_ANIM_LF(e, p)
}

/// 0x00012ec0 (not registered)
pub fn Ghost_Lock_LocalCB_TIMER_END_GHOSTLOCK(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

/// 0x00012ef0
pub fn ghost_lockConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    ghost_lock_baseConstructor(e, api, instanceCount);
    match B(e).local {
        Local::L03a | Local::L01c => {
            if instanceCount == 1 {
                APIcons::SetExternalFunction(e, 4, Ghost_Lock_Local_CB_ANIM_LF);
            }
            APIcons::SetMessageFunction(e, Ghost_Lock_Local_FUNC_MessageHandler);
        }
        Local::L04a => {
            APIcons::SetMessageFunction(e, Ghost_Lock_LocalFUNC_MessageHandler);
        }
        Local::None => {}
    }
    Ok(())
}

/// EP6L04A (041f00a7) 0x00012d28
pub fn Ghost_Lock_LocalFUNC_MessageHandler(e: &mut Engine, param_1: u32, s: u32, d: Ptr) -> LResult<u64> {
    if param_1 == 0x166 {
        APIphysic::PrimResetIntAttribBits(e, SELF, 0x3e4, 1, 2);
    }
    Ghost_Lock_BaseFUNC_MessageHandler(e, param_1, s, d)
}

/// 0x00012f88
pub fn ghost_lockDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    ghost_lock_baseDestructor(e, instanceCount);
    Ok(())
}

//! `tod_beam`: the beam the Tome of Doom shoots when it sucks a monster in. A spiral of "spirit"
//! particles and sprites runs along the beam's bones: it opens, hooks the monster, and closes; the
//! book is told about each step (messages 0xd6/0x68 and 0xd6/0x69).
//! Port of `Game Logic Modules/Globals/Objects/ToD_Beam/tod_beam__<sha8>/tod_beam.c` (17 functions).
//! Reference build: tod_beam__01ad949b (EP6L03). The 30 builds share the same code and differ in
//! 11 literals (the object id of the book, the animation indices), see [`builds`].
//!
//! Local data (`pLocalDataBeam`, 0x158 bytes): +0x00 EFBlendParams of the current blend (+0 blend id
//! 0xbb00000N: 1 hooking, 2 closing, 3 opening; +8 weight f32; +0x10 blend time f32); +0x18 current
//! state function; +0x1c flags (1 = blend params changed, 0x10000000 = animation reached its last
//! frame, 0x20000000 = the monster is hooked); +0x20 animation from PlayAnim; +0x24 sprite group;
//! +0x28 14 sprites; +0x60 their 14 child joint ids; +0x98/+0x9c particle types (spiral, flying);
//! +0xa0 11 emitters; +0xcc their 11 joints.
//!
//! The state function at +0x18 is a code pointer in the original. The port stores the function's
//! module-relative address (the same value up to relocation) and dispatches on it in [`call_state`].
#![allow(non_snake_case, non_upper_case_globals, unused_variables)]

pub mod builds;
use builds::*;

use crate::engine::*;
use crate::logic::{LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "tod_beam",
    // globals 0x10000..0x10028; gpoLanguagesNamesAndISOCodes (0x10028, 10 bytes, "Abkhazian") is
    // never read by the module and is not reproduced
    data_size: 0x34,
    ctor: tod_beamConstructor,
    dtor: tod_beamDestructor,
    register: None,
    init: &[],
    build_init: None,
};

/// Index of a build in [`builds::BUILDS`] by the first 8 hex digits of its SHA-1.
pub fn build_index(sha8: &str) -> Option<usize> {
    BUILDS.iter().position(|b| b.sha8 == sha8)
}

// ---- module globals (_globals.h) ----
pub const pAnim: u32 = 0x10000;
pub const pExtra: u32 = 0x10004;
pub const pGameObj: u32 = 0x10008;
pub const pGraphic: u32 = 0x1000c;
pub const pParticle: u32 = 0x10010;
pub const pPhysic: u32 = 0x10014;
pub const pDebug: u32 = 0x10018;
pub const pAnimMngr: u32 = 0x1001c;
pub const pMiscFunc: u32 = 0x10020;
pub const pLocalDataBeam: u32 = 0x10024;

#[inline]
fn G(e: &Engine, a: u32) -> Ptr {
    e.rt.cur_data() + (a - 0x10000)
}
/// current value of `pLocalDataBeam`
#[inline]
fn L(e: &Engine) -> Ptr {
    e.r32(G(e, pLocalDataBeam))
}
/// per-build literal (see builds.rs)
#[inline]
fn k(e: &Engine, i: usize) -> u32 {
    BUILDS[e.rt.cur_build()].k[i]
}

// ---- the state functions stored at +0x18 (their module-relative addresses) ----
const ADDR_Opening_In: u32 = 0x00010e20;
const ADDR_Opening_Execute: u32 = 0x00010f80;
const ADDR_Hooking_In: u32 = 0x00011050;
const ADDR_Hooking_Execute: u32 = 0x00011158;
const ADDR_Closing_In: u32 = 0x00011208;
const ADDR_Closing_Execute: u32 = 0x00011310;

/// `(**(code **)(param_1 + 0x18))(param_1)`. The local data is zeroed at allocation, so before
/// ST_FIRSTMOVE ran the pointer is NULL; the original would jump to 0 and crash. The port panics.
fn call_state(e: &mut Engine, param_1: Ptr) {
    match e.r32(param_1 + 0x18) {
        ADDR_Opening_In => ToD_BeamState_Opening_In(e, param_1),
        ADDR_Opening_Execute => ToD_BeamState_Opening_Execute(e, param_1),
        ADDR_Hooking_In => ToD_BeamState_Hooking_In(e, param_1),
        ADDR_Hooking_Execute => ToD_BeamState_Hooking_Execute(e, param_1),
        ADDR_Closing_In => ToD_BeamState_Closing_In(e, param_1),
        ADDR_Closing_Execute => ToD_BeamState_Closing_Execute(e, param_1),
        a => panic!("tod_beam: call through bad state pointer {a:#x}"),
    }
}

/// 0x00010d38 state 0: register for animation events (notify 0xb -> callback 3), go to Opening, run
/// the entry point from the next frame
pub fn ToD_BeamST_FIRSTMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 0xb, 3);
    let l = L(e);
    e.w32(l + 0x18, ADDR_Opening_In);
    APIgameobj::SetNextState(e, SELF, 1);
    Ok(0)
}

/// 0x00010dc8 state 1: run the current state function
pub fn ToD_BeamST_ENTRYPOINT(e: &mut Engine, _: u32) -> LResult<u64> {
    let l = L(e);
    call_state(e, l);
    Ok(0)
}

/// 0x00010e20 play the opening animation (last-frame callback 2) blended in over 0.5 s, show the
/// beam, create its particles and sprites
pub fn ToD_BeamState_Opening_In(e: &mut Engine, param_1: Ptr) {
    APIanim::BeginTree(e, SELF);
    let a = k(e, ToD_BeamState_Opening_In_0_1) as i32;
    let iVar1 = LIBAnimMngr::PlayAnim(e, 0.0, a, 1, 2);
    e.wi32(param_1 + 0x20, iVar1);
    e.w32(param_1, 0xbb00_0003);
    e.w32(param_1 + 4, 0);
    e.w32(param_1 + 0xc, 0);
    e.w32(param_1 + 8, 0);
    e.w32(param_1 + 0x10, 0x3f00_0000);
    let (a, b) = (k(e, ToD_BeamState_Opening_In_0_2), k(e, ToD_BeamState_Opening_In_0_3));
    APIanim::PlayBlendConnect(e, SELF, a, b, 1, param_1);
    APIanim::EndTree(e, SELF);
    APIgameobj::SetGameObjectVisible(e, SELF, 1);
    let l = L(e);
    ToD_Beam_Create_Particules(e, l);
    let l = L(e);
    ToD_Beam_Create_Sprites(e, l);
    e.w32(param_1 + 0x18, ADDR_Opening_Execute);
    call_state(e, param_1);
}

/// 0x00010f80 wait for the opening animation's last frame (-> Hooking); meanwhile apply the blend
/// weight that message 0xd5 sets
pub fn ToD_BeamState_Opening_Execute(e: &mut Engine, param_1: Ptr) {
    let f = e.r32(param_1 + 0x1c);
    if f & 0x1000_0000 == 0 {
        if f & 1 != 0 {
            e.w32(param_1 + 0x1c, f & 0xffff_fffe);
            APIanim::ChangeBlendParams(e, SELF, param_1);
        }
    } else {
        e.w32(param_1 + 0x1c, f & 0xefff_ffff);
        e.w32(param_1 + 0x18, ADDR_Hooking_In);
    }
}

/// 0x00011050 play the hooking (looping) animation; the blend weight is left as it was
pub fn ToD_BeamState_Hooking_In(e: &mut Engine, param_1: Ptr) {
    APIanim::BeginTree(e, SELF);
    let a = k(e, ToD_BeamState_Hooking_In_0_1) as i32;
    let iVar1 = LIBAnimMngr::PlayAnim(e, 0.0, a, 2, 0);
    e.wi32(param_1 + 0x20, iVar1);
    e.w32(param_1, 0xbb00_0001);
    e.w32(param_1 + 4, 0);
    e.w32(param_1 + 0xc, 0);
    e.w32(param_1 + 0x10, 0x3f00_0000);
    let (a, b) = (k(e, ToD_BeamState_Hooking_In_0_2), k(e, ToD_BeamState_Hooking_In_0_3));
    APIanim::PlayBlendConnect(e, SELF, a, b, 2, param_1);
    APIanim::EndTree(e, SELF);
    e.w32(param_1 + 0x18, ADDR_Hooking_Execute);
    call_state(e, param_1);
}

/// 0x00011158 wait until the monster is hooked (message 0xd6/0x67 -> Closing); meanwhile apply the
/// blend weight that message 0xd5 sets. The hooked flag is not cleared here (Closing_Execute only
/// tests 0x10000000).
pub fn ToD_BeamState_Hooking_Execute(e: &mut Engine, param_1: Ptr) {
    let f = e.r32(param_1 + 0x1c);
    if f & 0x2000_0000 == 0 {
        if f & 1 != 0 {
            e.w32(param_1 + 0x1c, f & 0xffff_fffe);
            APIanim::ChangeBlendParams(e, SELF, param_1);
        }
    } else {
        e.w32(param_1 + 0x18, ADDR_Closing_In);
    }
}

/// 0x00011208 play the closing animation (last-frame callback 2), blend 1 s
pub fn ToD_BeamState_Closing_In(e: &mut Engine, param_1: Ptr) {
    APIanim::BeginTree(e, SELF);
    let a = k(e, ToD_BeamState_Closing_In_0_1) as i32;
    let iVar1 = LIBAnimMngr::PlayAnim(e, 0.0, a, 1, 2);
    e.wi32(param_1 + 0x20, iVar1);
    e.w32(param_1, 0xbb00_0002);
    e.w32(param_1 + 4, 0);
    e.w32(param_1 + 0xc, 0);
    e.w32(param_1 + 0x10, 0x3f80_0000);
    let (a, b) = (k(e, ToD_BeamState_Closing_In_0_2), k(e, ToD_BeamState_Closing_In_0_3));
    APIanim::PlayBlendConnect(e, SELF, a, b, 1, param_1);
    APIanim::EndTree(e, SELF);
    e.w32(param_1 + 0x18, ADDR_Closing_Execute);
    call_state(e, param_1);
}

/// 0x00011310 at the closing animation's last frame: tell the book (0xd6/0x68) and kill the beam
pub fn ToD_BeamState_Closing_Execute(e: &mut Engine, param_1: Ptr) {
    let f = e.r32(param_1 + 0x1c);
    if f & 0x1000_0000 != 0 {
        e.w32(param_1 + 0x1c, f & 0xefff_ffff);
        let book = k(e, ID_2a000012);
        APIgameobj::SendMessageTo(e, book, NULL, 0xd6, 0x68, NULL);
        APIgameobj::KillObject(e, SELF);
    }
}

/// 0x000113c0 callback 2: the playing animation reached its last frame
pub fn ToD_BeamCB_ANIM_LF(e: &mut Engine, _: u32) -> LResult<u64> {
    let l = L(e);
    let f = e.r32(l + 0x1c);
    e.w32(l + 0x1c, f | 0x1000_0000);
    Ok(1)
}

/// 0x00011418 callback 3 (notify 0xb, animation event {id, sub}): event 3 of the closing
/// animation's track tells the book (0xd6/0x69)
pub fn ToD_BeamCB_ANIMATION(e: &mut Engine, param_1: Ptr) -> LResult<u64> {
    if e.r32(param_1 + 4) == 0 && e.r32(param_1) == k(e, ToD_BeamCB_ANIMATION_0_1) {
        let book = k(e, ID_2a000012);
        APIgameobj::SendMessageTo(e, book, NULL, 0xd6, 0x69, NULL);
    }
    Ok(0)
}

/// 0x000114a8 messages: 0xd6/0x67 the monster is hooked (payload +8: blend weight); 0xd5 new blend
/// weight (payload +8, clamped to 1.0) while opening or hooking; 0xdf kill the beam
pub fn ToD_BeamFUNC_MessageHandler(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    let piVar1 = L(e);
    if param_1 == 0xd6 {
        if param_2 == 0x67 {
            let f = e.r32(piVar1 + 0x1c);
            e.w32(piVar1 + 0x1c, f | 0x2000_0000);
            let v = e.r32(param_3 + 8);
            e.w32(piVar1 + 8, v);
        }
    } else if param_1 < 0xd7 {
        let b = e.r32(piVar1);
        if param_1 == 0xd5 && (b == 0xbb00_0001 || b == 0xbb00_0003) {
            let f = e.r32(piVar1 + 0x1c);
            e.w32(piVar1 + 0x1c, f | 1);
            if e.rf(param_3 + 8) < 1.0 {
                let v = e.r32(param_3 + 8);
                e.w32(piVar1 + 8, v);
            } else {
                e.w32(piVar1 + 8, 0x3f80_0000);
            }
        }
    } else if param_1 == 0xdf {
        APIgameobj::KillObject(e, SELF);
    }
    Ok(0)
}

/// 0x00011650 the "spirit" particle types and 11 emitters on the beam's joints: 8 spiral emitters on
/// joints 3,4,6,7,9,10,12,13 and 3 flying-spirit emitters on joints 0x20,0x24,0x28
pub fn ToD_Beam_Create_Particules(e: &mut Engine, param_1: Ptr) {
    for (i, j) in [3u32, 4, 6, 7, 9, 10, 0xc, 0xd].into_iter().enumerate() {
        e.w32(param_1 + 0xcc + i as u32 * 4, j);
    }
    let uVar1 = APIparticle::TypeCreate(e);
    e.w32(param_1 + 0x98, uVar1);
    let t = e.ri32(param_1 + 0x98);
    let s = e.cstr("spirit");
    APIparticle::TypeSetTexture(e, t, s);
    let s = e.cstr("TOD_BEAM_SPIRALSPIRIT");
    APIparticle::TypeSetName(e, t, s);
    APIparticle::TypeSetInt(e, t, 1, 1);
    let p = e.r32(G(e, pParticle));
    let t = e.r32(param_1 + 0x98);
    ToD_Beam_Set_Particule(e, p, t);
    for iStack_3c in 0..8u32 {
        let t = e.ri32(param_1 + 0x98);
        let uVar1 = APIparticle::EmitterCreate(e, t);
        e.w32(param_1 + 0xa0 + iStack_3c * 4, uVar1);
        let (em, j) = (e.ri32(param_1 + 0xa0 + iStack_3c * 4), e.r32(param_1 + 0xcc + iStack_3c * 4));
        APIparticle::EmitterSetJoint(e, em, SELF, j);
        APIparticle::EmitterStart(e, em, 1);
    }
    e.w32(param_1 + 0xec, 0x20);
    e.w32(param_1 + 0xf0, 0x24);
    e.w32(param_1 + 0xf4, 0x28);
    let uVar1 = APIparticle::TypeCreate(e);
    e.w32(param_1 + 0x9c, uVar1);
    let t = e.ri32(param_1 + 0x9c);
    let s = e.cstr("spirit");
    APIparticle::TypeSetTexture(e, t, s);
    let s = e.cstr("TOD_BEAM_FLYINGSPIRIT");
    APIparticle::TypeSetName(e, t, s);
    APIparticle::TypeSetInt(e, t, 1, 1);
    let p = e.r32(G(e, pParticle));
    let t = e.r32(param_1 + 0x9c);
    ToD_Beam_Set_Particule_FlyingSpirit(e, p, t);
    for iStack_3c in 8..0xbu32 {
        let t = e.ri32(param_1 + 0x9c);
        let uVar1 = APIparticle::EmitterCreate(e, t);
        e.w32(param_1 + 0xa0 + iStack_3c * 4, uVar1);
        let (em, j) = (e.ri32(param_1 + 0xa0 + iStack_3c * 4), e.r32(param_1 + 0xcc + iStack_3c * 4));
        APIparticle::EmitterSetJoint(e, em, SELF, j);
        APIparticle::EmitterStart(e, em, 1);
    }
}

/// 0x00011a60 a group of 14 blue additive "spirit" sprites, each attached to child joint 0xe + i
pub fn ToD_Beam_Create_Sprites(e: &mut Engine, param_1: Ptr) {
    let uVar1 = APIgraphics::SpriteAddGroup(e);
    e.w32(param_1 + 0x24, uVar1);
    let gr = e.r32(param_1 + 0x24);
    let s = e.cstr("spirit");
    APIgraphics::SpriteSetGroupTexture(e, gr, s);
    APIgraphics::SpriteSetGroupColor(e, gr, 0.0, f32::from_bits(0x3d4c_cccd), f32::from_bits(0x3ecc_cccd), 0.0);
    APIgraphics::SpriteSetGroupBlendMode(e, gr, 1);
    APIgraphics::SpriteSetGroupFlags(e, gr, 6);
    for iStack_3c in 0..0xeu32 {
        e.w32(param_1 + 0x60 + iStack_3c * 4, iStack_3c + 0xe);
        let gr = e.r32(param_1 + 0x24);
        let uVar1 = APIgraphics::SpriteGroupAddSprite(e, gr, SELF, 0x6fff_ffff, f32::from_bits(0x3ee6_6666));
        e.w32(param_1 + 0x28 + iStack_3c * 4, uVar1);
        let (j, sp) = (e.r32(param_1 + 0x60 + iStack_3c * 4), e.r32(param_1 + 0x28 + iStack_3c * 4));
        APIgameobj::AddChild(e, SELF, j, sp, 1);
    }
}

/// 0x00011c58 settings of the spiral-spirit particle type. `param_1` is the APIparticle table
/// (called through raw offsets in the original: +0xc TypeSetInter, +0x10 TypeSetInt, +0x18
/// TypeSetFloat, +0x1c TypeSetVec); the calls go through the [`APIparticle`] trait.
/// Float arguments are in $f12..$f15 in the order shown (checked with disasm.py).
pub fn ToD_Beam_Set_Particule(e: &mut Engine, param_1: Ptr, param_2: u32) {
    let t = param_2 as i32;
    let h = f32::from_bits;
    for (w, v) in [(8, 1), (1, 0), (4, 0), (0x10, 0), (0x20, 0), (0x40, 0), (0x80, 0), (0x100, 0)] {
        APIparticle::TypeSetInt(e, t, w, v);
    }
    APIparticle::TypeSetFloat(e, t, 1, 0.0);
    APIparticle::TypeSetFloat(e, t, 4, h(0x3a83_126f));
    APIparticle::TypeSetFloat(e, t, 2, 0.5);
    APIparticle::TypeSetFloat(e, t, 8, 0.0);
    APIparticle::TypeSetVec(e, t, 1, 0.0, 0.0, 0.0);
    APIparticle::TypeSetVec(e, t, 2, 0.0, 0.0, 0.0);
    APIparticle::TypeSetInter(e, t, 8, 0.0, 0.0, 0.0, 0.5);
    APIparticle::TypeSetInter(e, t, 0x10, h(0x3f0c_cccd), h(0x3dcc_cccd), h(0x3a83_126f), 0.5);
    APIparticle::TypeSetInter(e, t, 0x20, 0.0, 0.0, 0.0, 0.0);
    APIparticle::TypeSetInter(e, t, 1, 0.5, h(0x3e4c_cccd), h(0x3c23_d70a), 0.5);
    APIparticle::TypeSetInter(e, t, 2, 0.25, h(0x3dcc_cccd), 0.0, 0.5);
    APIparticle::TypeSetInter(e, t, 4, 1.0, 0.5, h(0x3c23_d70a), 0.5);
}

/// 0x00011f78 settings of the flying-spirit particle type (see [`ToD_Beam_Set_Particule`])
pub fn ToD_Beam_Set_Particule_FlyingSpirit(e: &mut Engine, param_1: Ptr, param_2: u32) {
    let t = param_2 as i32;
    let h = f32::from_bits;
    for (w, v) in [(8, 1), (1, 0), (4, 0), (0x10, 0), (0x20, 0), (0x40, 0), (0x80, 0), (0x100, 0)] {
        APIparticle::TypeSetInt(e, t, w, v);
    }
    APIparticle::TypeSetFloat(e, t, 1, 0.0);
    APIparticle::TypeSetFloat(e, t, 4, h(0x3a83_126f));
    APIparticle::TypeSetFloat(e, t, 2, 0.5);
    APIparticle::TypeSetFloat(e, t, 8, 0.0);
    APIparticle::TypeSetVec(e, t, 1, 0.0, 0.0, 0.0);
    APIparticle::TypeSetVec(e, t, 2, 0.0, 0.0, 0.0);
    APIparticle::TypeSetInter(e, t, 1, 0.0, 0.0, 0.0, 0.5);
    APIparticle::TypeSetInter(e, t, 2, 0.25, h(0x3dcc_cccd), 0.0, 0.5);
    APIparticle::TypeSetInter(e, t, 4, 1.0, 0.5, 0.0, 0.5);
    APIparticle::TypeSetInter(e, t, 8, 0.0, 0.0, 0.0, 0.5);
    APIparticle::TypeSetInter(e, t, 0x10, h(0x3ecc_cccd), h(0x3dcc_cccd), h(0x3a83_126f), 0.5);
    APIparticle::TypeSetInter(e, t, 0x20, 0.0, 0.0, 0.0, 0.0);
}

/// 0x00012280 module constructor
pub fn tod_beamConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    if instanceCount == 1 {
        let fns: [(i32, crate::logic::ExtFn); 4] = [
            (0, ToD_BeamST_FIRSTMOVE),
            (1, ToD_BeamST_ENTRYPOINT),
            (2, ToD_BeamCB_ANIM_LF),
            (3, ToD_BeamCB_ANIMATION),
        ];
        for (i, f) in fns {
            APIcons::SetExternalFunction(e, i, f);
        }
        // the API table pointers are stored like in the original (parity only: calls go through the traits)
        let tables: [(u32, fn(&mut Engine) -> Ptr); 6] = [
            (pAnim, |e| APIcons::GetApiAnim(e)),
            (pExtra, |e| APIcons::GetApiExtra(e)),
            (pGameObj, |e| APIcons::GetApiGameObj(e)),
            (pGraphic, |e| APIcons::GetApiGraphics(e)),
            (pParticle, |e| APIcons::GetApiParticle(e)),
            (pPhysic, |e| APIcons::GetApiPhysic(e)),
        ];
        for (slot, get) in tables {
            let v = get(e);
            let a = G(e, slot);
            e.w32(a, v);
        }
        for (slot, name) in [(pDebug, "debug"), (pAnimMngr, "AnimMngr"), (pMiscFunc, "MiscFunc")] {
            let n = e.cstr(name);
            let v = APIcons::GetApiCustom(e, n);
            let a = G(e, slot);
            e.w32(a, v);
        }
    }
    let slot = G(e, pLocalDataBeam);
    APIcons::GetLocalData(e, slot, 0x158);
    APIcons::SetMessageFunction(e, ToD_BeamFUNC_MessageHandler);
    let l = L(e);
    e.w32(l, 0);
    LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 2, NULL, 1);
    APIgameobj::SetGameObjectVisible(e, SELF, 0);
    APIcons::SetInitialState(e, 0);
    Ok(())
}

/// 0x000124c0 module destructor: emitters, particle types, sprite group
pub fn tod_beamDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    for iStack_2c in 0..0xbu32 {
        let em = e.ri32(L(e) + 0xa0 + iStack_2c * 4);
        if em != 0 {
            APIparticle::EmitterDestroy(e, em);
        }
    }
    let t = e.ri32(L(e) + 0x98);
    if t != 0 {
        APIparticle::TypeDestroy(e, t);
    }
    let t = e.ri32(L(e) + 0x9c);
    if t != 0 {
        APIparticle::TypeDestroy(e, t);
    }
    let gr = e.r32(L(e) + 0x24);
    if gr != 0 {
        APIgraphics::SpriteRemoveGroup(e, gr);
    }
    Ok(())
}

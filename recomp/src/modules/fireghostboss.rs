//! `fireghostboss`: the Ep6 fire ghost boss. Hovers (FUNC_Wave), turns to face the leader hero and
//! throws fireballs (latent 0x2a80001b, one or four per attack), is hit by water (0xd8). It has no
//! Monster_Base: the book side of the fight is the fire ghosts it spawns (see `fireghost`).
//! Port of `Game Logic Modules/Ep6_ShermanTech/Characters/FIRE_GHOST_BOSS/fireghostboss/
//! {FireGhostBoss.c, FireGhostBoss_particle.c}` (27 + 2 functions). One build.
//!
//! Local data (0xbc bytes): +0 sub-state code pointer (see [`addr`]), +8 start position (+0xc = hover
//! base height), +0x2c rotation axis (0,1,0), +0x40 event bits (4 hero in range, 8 anim ended),
//! +0x48..0x54 the fireball latents (+0x48 also the one launched by the first constructor), +0x58 that
//! first latent, +0x5c 1, +0x60 "four fireballs" flag, +0x64 the 0xd8 sender, +0x68 target angle,
//! +0x6c angle turned, +0x70 turn step, +0x74 turn direction, +0x94 last anim ended, +0x98..0xa0 fire
//! emitters, +0xb0 timer, +0xb4 / +0xb8 hover phase / speed.
//! Global data (0x18 bytes): +0 game buffer 0, +4 game buffer 2, +8 / +0xc the FIRE_ELBOW / FIRE_HEAD
//! particle types. (Ghidra prints some of these as word indexes: the offsets here are bytes,
//! checked with tools/rustport/disasm.py.)
#![allow(non_snake_case, non_upper_case_globals, unused_variables)]

use crate::engine::*;
use crate::logic::{LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "fireghostboss",
    // globals 0x10000..0x10038; gpoLanguagesNamesAndISOCodes (0x10038, "Abkhazian") is never read
    data_size: 0x38,
    ctor: fireghostbossConstructor,
    dtor: fireghostbossDestructor,
    register: None,
    init: &[],
    build_init: None,
};

// ---- module globals (_globals.h) ----
pub const pExtra: u32 = 0x10000;
pub const pGameObj: u32 = 0x10004;
pub const pPhysic: u32 = 0x10008;
pub const pAnimMngr: u32 = 0x1000c;
pub const pDebug: u32 = 0x10010;
pub const pStdlib: u32 = 0x10014;
pub const pSound: u32 = 0x10018;
pub const pAnim: u32 = 0x1001c;
pub const pMath: u32 = 0x10020;
pub const pMiscFunc: u32 = 0x10024;
pub const pFunction: u32 = 0x10028;
pub const pParticle: u32 = 0x1002c;
pub const pLocalDataFireGhostBoss: u32 = 0x10030;
pub const pGlobalDataFireGhostBoss: u32 = 0x10034;

const ALL: u32 = 0x7fff_ffff;

#[inline]
fn G(e: &Engine, a: u32) -> Ptr {
    e.rt.cur_data() + (a - 0x10000)
}
#[inline]
fn L(e: &Engine) -> Ptr {
    e.r32(G(e, pLocalDataFireGhostBoss))
}
#[inline]
fn GD(e: &Engine) -> Ptr {
    e.r32(G(e, pGlobalDataFireGhostBoss))
}
/// the leader hero: game buffer 0 +8
#[inline]
fn hero(e: &Engine) -> u32 {
    e.r32(e.r32(GD(e)) + 8)
}
#[inline]
fn or32(e: &mut Engine, a: Ptr, bits: u32) {
    let v = e.r32(a);
    e.w32(a, v | bits);
}
#[inline]
fn and32(e: &mut Engine, a: Ptr, mask: u32) {
    let v = e.r32(a);
    e.w32(a, v & mask);
}

/// Module-relative addresses of the sub-state functions stored in L+0.
pub mod addr {
    pub const fireGhostBossST_SearchingHero_In: u32 = 0x115c0;
    pub const fireGhostBossST_SearchingHero_Execute: u32 = 0x116d0;
    pub const fireGhostBossST_AttackPreparation_In: u32 = 0x117c0;
    pub const fireGhostBossST_AttackPreparation_Execute: u32 = 0x11a98;
    pub const fireGhostBossST_Attack_In: u32 = 0x11bb0;
    pub const fireGhostBossST_Attack_Execute: u32 = 0x11e18;
    pub const fireGhostBossST_BeingAttack_In: u32 = 0x11fa0;
    pub const fireGhostBossST_BeingAttack_Execute: u32 = 0x120b0;
    pub const fireGhostBossST_BlackHole_In: u32 = 0x12160;
    pub const fireGhostBossST_BlackHole_Execute: u32 = 0x121c0;
}

/// `(*(code *)*pLocalDataFireGhostBoss)()`
pub fn call_state(e: &mut Engine) -> LResult<()> {
    let f = e.r32(L(e));
    match f {
        addr::fireGhostBossST_SearchingHero_In => fireGhostBossST_SearchingHero_In(e),
        addr::fireGhostBossST_SearchingHero_Execute => fireGhostBossST_SearchingHero_Execute(e),
        addr::fireGhostBossST_AttackPreparation_In => fireGhostBossST_AttackPreparation_In(e),
        addr::fireGhostBossST_AttackPreparation_Execute => fireGhostBossST_AttackPreparation_Execute(e),
        addr::fireGhostBossST_Attack_In => fireGhostBossST_Attack_In(e),
        addr::fireGhostBossST_Attack_Execute => fireGhostBossST_Attack_Execute(e),
        addr::fireGhostBossST_BeingAttack_In => fireGhostBossST_BeingAttack_In(e),
        addr::fireGhostBossST_BeingAttack_Execute => fireGhostBossST_BeingAttack_Execute(e),
        addr::fireGhostBossST_BlackHole_In => fireGhostBossST_BlackHole_In(e),
        addr::fireGhostBossST_BlackHole_Execute => fireGhostBossST_BlackHole_Execute(e),
        // a jump through a null / foreign pointer would crash the PS2
        f => panic!("fireghostboss: bad sub-state pointer {f:#x}"),
    }
}
#[inline]
fn set_state(e: &mut Engine, f: u32) {
    let l = L(e);
    e.w32(l, f);
}
/// `*L = f; (*L)()`
fn go(e: &mut Engine, f: u32) -> LResult<()> {
    set_state(e, f);
    call_state(e)
}

/// tell the current fireball L+0x48 to burn out (0x1b1) if it still exists and is not the first one
fn end_fireball(e: &mut Engine) {
    let l = L(e);
    let fb = e.r32(l + 0x48);
    let iVar2 = APIgameobj::IsGameObjectVisible(e, fb);
    if iVar2 as i32 != -1 && fb != e.r32(L(e) + 0x58) {
        APIgameobj::SendMessageTo(e, fb, NULL, 0x1b1, 0, NULL);
    }
}

/// launch a fireball latent into L+`slot` and tell it 0x1af + our id
fn launch_fireball(e: &mut Engine, l: Ptr, slot: u32) {
    let uVar3 = APIgameobj::LaunchLatant(e, 0x2a80_001b);
    e.w32(l + slot, uVar3);
    let uVar4 = APIgameobj::GetID(e, SELF, NULL);
    let to = e.r32(l + slot);
    APIgameobj::SendMessageTo(e, to, NULL, 0x1af, uVar4, NULL);
}

// ---- FireGhostBoss.c ----

/// 0x000113e8 external state 0 (initial): context 0x1a, start position, axis +y, mask 0x100001,
/// sub-state BlackHole (waits for the cut-scene), -> state 1
pub fn fireGhostBossST_INITIAL(e: &mut Engine, _: u32) -> LResult<u64> {
    let puVar1 = L(e);
    APIgameobj::SetContextFlag(e, SELF, 0x1a);
    APIgameobj::GetPosition(e, SELF, ALL, puVar1 + 8);
    for (o, v) in [(0x2c, 0), (0x30, 0x3f80_0000), (0x34, 0), (0x5c, 1), (0x60, 0), (0x64, 0)] {
        e.w32(puVar1 + o, v);
    }
    APIgameobj::SetGroupBitMask(e, SELF, 0x100001);
    e.w32(puVar1, addr::fireGhostBossST_BlackHole_In);
    APIgameobj::SetNextState(e, SELF, 1);
    Ok(1)
}

/// 0x00011550 external state 1: hover, run the sub-state
pub fn fireGhostBossST_ENTRY_POINT(e: &mut Engine, _: u32) -> LResult<u64> {
    fireGhostBossFUNC_Wave(e)?;
    call_state(e)?;
    Ok(0)
}

/// 0x000115c0 anim 2 (loop), context 0x1a, end the current fireball
pub fn fireGhostBossST_SearchingHero_In(e: &mut Engine) -> LResult<()> {
    LIBAnimMngr::PlayAnim(e, 0.0, 2, 2, 0);
    APIgameobj::SetContextFlag(e, SELF, 0x1a);
    end_fireball(e);
    go(e, addr::fireGhostBossST_SearchingHero_Execute)
}

/// 0x000116d0 the hero within 30 m -> AttackPreparation
pub fn fireGhostBossST_SearchingHero_Execute(e: &mut Engine) -> LResult<()> {
    let h = hero(e);
    let bVar2 = LIBApiCFunctions::CheckRangeObject(e, 30.0, SELF, h, 1);
    if bVar2 != 0 {
        let a = L(e) + 0x40;
        or32(e, a, 4);
    }
    if e.r32(L(e) + 0x40) & 4 != 0 {
        let a = L(e) + 0x40;
        and32(e, a, 0xffff_fffb);
        go(e, addr::fireGhostBossST_AttackPreparation_In)?;
    }
    Ok(())
}

/// 0x000117c0 the hero within 30.5 m: angle to him (signed by the y of look x direction), first turn
/// step of 0.1257 rad; else back to SearchingHero
pub fn fireGhostBossST_AttackPreparation_In(e: &mut Engine) -> LResult<()> {
    let puVar1 = L(e);
    let h = hero(e);
    let bVar2 = LIBApiCFunctions::CheckRangeObject(e, 30.5, SELF, h, 1);
    if bVar2 != 0 {
        e.w32(puVar1 + 0x74, 1);
        e.w32(puVar1 + 0x70, 0x3e00_adfd);
        e.w32(puVar1 + 0x6c, 0);
        let d50 = e.alloca(0x10);
        let d60 = e.alloca(0x10);
        let d70 = e.alloca(0x10);
        let d80 = e.alloca(0x10);
        APIgameobj::GetLookAt(e, SELF, ALL, d50);
        APImath::CVectorNormalize(e, d50, d50);
        let h = hero(e);
        APIgameobj::GetPosition(e, h, ALL, d80);
        APIgameobj::GetPosition(e, SELF, ALL, d70);
        APImath::CVectorGetDirectionAndDistanceTo(e, d60, d70, d80);
        let fVar3 = APImath::CVectorGetAngleTo(e, d60, d50);
        e.wf(puVar1 + 0x68, fVar3);
        // only the y component of (look x dir) is used
        let fStack_3c = e.rf(d50 + 8) * e.rf(d60) - e.rf(d50) * e.rf(d60 + 8);
        if fStack_3c < 0.0 {
            let v = e.rf(puVar1 + 0x68) * -1.0;
            e.wf(puVar1 + 0x68, v);
            let v = e.rf(puVar1 + 0x70) * -1.0;
            e.wf(puVar1 + 0x70, v);
            e.w32(puVar1 + 0x74, 0);
        }
        let v = e.rf(puVar1 + 0x6c) + e.rf(puVar1 + 0x70);
        e.wf(puVar1 + 0x6c, v);
        let a = e.rf(puVar1 + 0x6c);
        APIgameobj::Rotate(e, SELF, puVar1 + 0x2c, a);
        go(e, addr::fireGhostBossST_AttackPreparation_Execute)
    } else {
        go(e, addr::fireGhostBossST_SearchingHero_In)
    }
}

/// 0x00011a98 keep turning by the step until the target angle is passed, then Attack
pub fn fireGhostBossST_AttackPreparation_Execute(e: &mut Engine) -> LResult<()> {
    let l = L(e);
    let (target, turned, dir) = (e.rf(l + 0x68), e.rf(l + 0x6c), e.r32(l + 0x74));
    if (turned <= target && dir != 0) || (target <= turned && dir == 0) {
        let v = turned + e.rf(l + 0x70);
        e.wf(l + 0x6c, v);
        let s = e.rf(l + 0x70);
        APIgameobj::Rotate(e, SELF, l + 0x2c, s);
        Ok(())
    } else {
        go(e, addr::fireGhostBossST_Attack_In)
    }
}

/// 0x00011bb0 anim 3 (once, end notified), one fireball, three more when L+0x60 is set
pub fn fireGhostBossST_Attack_In(e: &mut Engine) -> LResult<()> {
    let puVar2 = L(e);
    LIBAnimMngr::PlayAnim(e, 0.1, 3, 1, 3);
    launch_fireball(e, puVar2, 0x48);
    if e.r32(puVar2 + 0x60) == 1 {
        launch_fireball(e, puVar2, 0x4c);
        launch_fireball(e, puVar2, 0x50);
        launch_fireball(e, puVar2, 0x54);
    }
    go(e, addr::fireGhostBossST_Attack_Execute)
}

/// 0x00011e18 on an anim end: after anim 3 play anim 1 or 6 at random; after 1 / 6 forget the
/// fireball and prepare the next attack
pub fn fireGhostBossST_Attack_Execute(e: &mut Engine) -> LResult<()> {
    let puVar1 = L(e);
    if e.r32(L(e) + 0x40) & 8 != 0 {
        let a = L(e) + 0x40;
        and32(e, a, 0xffff_fff7);
        let last = e.r32(puVar1 + 0x94);
        if last == 3 {
            let iVar2 = LIBApiCFunctions::IntRandom(e, 0, 1);
            if iVar2 == 0 {
                LIBAnimMngr::PlayAnim(e, 0.1, 1, 1, 3);
            } else if iVar2 == 1 {
                LIBAnimMngr::PlayAnim(e, 0.1, 6, 1, 3);
            }
        } else if last == 1 || last == 6 {
            e.w32(puVar1 + 0x48, 0);
            go(e, addr::fireGhostBossST_AttackPreparation_In)?;
        }
    }
    Ok(())
}

/// 0x00011fa0 hit: anim 5 (once), end the fireball, from now on four fireballs per attack
pub fn fireGhostBossST_BeingAttack_In(e: &mut Engine) -> LResult<()> {
    let puVar1 = L(e);
    LIBAnimMngr::PlayAnim(e, 0.1, 5, 1, 3);
    end_fireball(e);
    e.w32(puVar1 + 0x48, 0);
    e.w32(puVar1 + 0x60, 1);
    go(e, addr::fireGhostBossST_BeingAttack_Execute)
}

/// 0x000120b0 anim 5 ended -> SearchingHero
pub fn fireGhostBossST_BeingAttack_Execute(e: &mut Engine) -> LResult<()> {
    if e.r32(L(e) + 0x40) & 8 != 0 {
        let a = L(e) + 0x40;
        and32(e, a, 0xffff_fff7);
        if e.r32(L(e) + 0x94) == 5 {
            go(e, addr::fireGhostBossST_SearchingHero_In)?;
        }
    }
    Ok(())
}

/// 0x00012160
pub fn fireGhostBossST_BlackHole_In(e: &mut Engine) -> LResult<()> {
    go(e, addr::fireGhostBossST_BlackHole_Execute)
}

/// 0x000121c0 idle
pub fn fireGhostBossST_BlackHole_Execute(e: &mut Engine) -> LResult<()> {
    Ok(())
}

/// 0x000121e0 external 3: anim `param_1` ended (L+0x94, event 8)
pub fn fireGhostBossCB_ANIMATION_LAST_FRAME(e: &mut Engine, param_1: u32) -> LResult<u64> {
    e.w32(L(e) + 0x94, param_1);
    let a = L(e) + 0x40;
    or32(e, a, 8);
    Ok(0)
}

/// 0x00012240 external 4: our timer -> SearchingHero next frame (not called now), timer off
pub fn fireGhostBossCB_TIMER(e: &mut Engine, param_1: u32) -> LResult<u64> {
    if param_1 == e.r32(L(e) + 0xb0) {
        set_state(e, addr::fireGhostBossST_SearchingHero_In);
        let t = e.r32(L(e) + 0xb0);
        APIextra::TimerUnregisterNotify(e, SELF, t);
    }
    Ok(0)
}

/// 0x000122d8 external 6: cut-scene start: fire on, sound 0x40000030, BlackHole; end: anim 2, the
/// timer (1.5 s) -> external 4, run the current sub-state
pub fn fireGhostBoss_baseCB_CUTSCENE(e: &mut Engine, param_1: u32) -> LResult<u64> {
    let start = e.r32(param_1 + 4);
    if start == 1 {
        fireGhostBossFUNC_StartFire(e)?;
        APInewSound::Play(e, SELF, 0x4000_0030, 0);
        go(e, addr::fireGhostBossST_BlackHole_In)?;
    } else if start == 0 {
        LIBAnimMngr::PlayAnim(e, 0.0, 2, 2, 0);
        let t = e.r32(L(e) + 0xb0);
        APIextra::TimerRegisterNotify(e, SELF, t, 4);
        call_state(e)?;
    }
    Ok(0)
}

/// 0x00012408 external 5: anim key frame {key, kind}: key 3 of kind 1 with a fireball out releases
/// it: with four fireballs, at the four targets of the brazier L+0x64 (0x2a0000ce..0x2a0000d1 ->
/// 0x2a00010c.. in fours) and the flag cleared; else at the leader hero.
pub fn fireGhostBossCB_ANIM(e: &mut Engine, param_1: u32) -> LResult<u64> {
    let iVar1 = L(e);
    if e.r32(param_1) == 3 && e.r32(param_1 + 4) == 1 && e.r32(L(e) + 0x48) != 0 {
        if e.r32(L(e) + 0x60) == 1 {
            let first = match e.r32(L(e) + 100) {
                0x2a00_00ce => Some(0x2a00_010c),
                0x2a00_00cf => Some(0x2a00_0110),
                0x2a00_00d0 => Some(0x2a00_0114),
                0x2a00_00d1 => Some(0x2a00_0118),
                _ => None,
            };
            if let Some(t) = first {
                for (i, o) in [0x48u32, 0x4c, 0x50, 0x54].into_iter().enumerate() {
                    let to = e.r32(iVar1 + o);
                    APIgameobj::SendMessageTo(e, to, NULL, 0x110, t + i as u32, NULL);
                }
            }
            e.w32(iVar1 + 0x60, 0);
        } else {
            let to = e.r32(L(e) + 0x48);
            let h = hero(e);
            APIgameobj::SendMessageTo(e, to, NULL, 0x110, h, NULL);
        }
    }
    Ok(0)
}

/// 0x00012860 clipping distances 25 / 50 m (modes 7)
pub fn fireGhostBossFUNC_SetClipping(e: &mut Engine) -> LResult<()> {
    let fStack_40: f32 = 25.0;
    let b = e.alloca(0x40);
    APIgameobj::GetClippingParams(e, SELF, b);
    e.wf(b + 4, fStack_40);
    e.w32(b + 0xc, 0);
    e.w32(b + 8, 7);
    e.wf(b + 0x10, fStack_40 + fStack_40);
    e.w32(b + 0x18, 0);
    e.w32(b + 0x14, 7);
    APIgameobj::SetClippingParams(e, SELF, b, 0);
    Ok(())
}

/// 0x00012908 hover: the phase L+0xb4 runs between -1 and 1 at a random speed L+0xb8 (0.05..0.15 per
/// frame); height = base L+0xc +- (1 - phase^2) * 0.15
pub fn fireGhostBossFUNC_Wave(e: &mut Engine) -> LResult<u32> {
    let iVar1 = L(e);
    let v = e.rf(iVar1 + 0xb4) + e.rf(iVar1 + 0xb8);
    e.wf(iVar1 + 0xb4, v);
    if 1.0 <= e.rf(iVar1 + 0xb4) {
        e.w32(iVar1 + 0xb4, 0x3f80_0000);
        let uVar2 = APIstdlib::MyRand(e);
        e.wf(iVar1 + 0xb8, -(((uVar2 as i32) as f32 / 32768.0) * 0.1 + 0.05));
    } else if e.rf(iVar1 + 0xb4) <= -1.0 {
        e.w32(iVar1 + 0xb4, 0xbf80_0000);
        let uVar2 = APIstdlib::MyRand(e);
        e.wf(iVar1 + 0xb8, ((uVar2 as i32) as f32 / 32768.0) * 0.1 + 0.05);
    }
    let ph = e.rf(iVar1 + 0xb4);
    let fStack_40 = (1.0 - ph * ph) * 0.15;
    let d50 = e.alloca(0x10);
    APIgameobj::GetPosition(e, SELF, ALL, d50);
    let y0 = e.rf(iVar1 + 0xc);
    if 0.0 < e.rf(iVar1 + 0xb8) {
        e.wf(d50 + 4, y0 + fStack_40);
    } else {
        e.wf(d50 + 4, y0 - fStack_40);
    }
    Ok(APIgameobj::SetTranslation(e, SELF, d50))
}

/// 0x00012b48 0xc9: SearchingHero next frame, one fireball; 0xd8 (water, from `param_2`): BeingAttack
/// next frame
pub fn fireGhostBossFUNC_MessageHandler(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    let puVar1 = L(e);
    if param_1 == 0xc9 {
        set_state(e, addr::fireGhostBossST_SearchingHero_In);
        e.w32(puVar1 + 0x60, 0);
    } else if param_1 == 0xd8 {
        set_state(e, addr::fireGhostBossST_BeingAttack_In);
        e.w32(puVar1 + 0x64, param_2);
    }
    Ok(0)
}

/// 0x00012c00
pub fn fireGhostBossFUNC_CreatePhysic(e: &mut Engine) -> LResult<()> {
    APIphysic::CreatePhysic(e, SELF, 0);
    Ok(())
}

/// 0x00012c48 the two fire types (texture "Firetorche_blue") into global data +8 / +0xc
pub fn fireGhostBossFUNC_CreateFireType(e: &mut Engine) -> LResult<()> {
    let t = APIparticle::TypeCreate(e);
    let gd = GD(e);
    e.w32(gd + 8, t);
    let t = APIparticle::TypeCreate(e);
    e.w32(gd + 0xc, t);
    let (elbow, head) = (e.r32(gd + 8) as i32, e.r32(gd + 0xc) as i32);
    let s = e.cstr("Firetorche_blue");
    APIparticle::TypeSetTexture(e, elbow, s);
    APIparticle::TypeSetTexture(e, head, s);
    let s = e.cstr("FIRE_ELBOW");
    APIparticle::TypeSetName(e, elbow, s);
    let s = e.cstr("FIRE_HEAD");
    APIparticle::TypeSetName(e, head, s);
    let p = e.r32(G(e, pParticle));
    fireGhostBossFUNC_SetParticle_FIRE_ELBOW(e, p, elbow as u32)?;
    fireGhostBossFUNC_SetParticle_FIRE_HEAD(e, p, head as u32)?;
    Ok(())
}

/// 0x00012da0 elbow fire on joints 0x800000 / 0x801000, head fire on 0x1200000
pub fn fireGhostBossFUNC_CreateFireParticle(e: &mut Engine) -> LResult<()> {
    let gd = GD(e);
    for (slot, ty) in [(0x98, 8), (0x9c, 0xc), (0xa0, 8)] {
        let t = e.r32(gd + ty) as i32;
        let em = APIparticle::EmitterCreate(e, t);
        e.w32(L(e) + slot, em);
    }
    for (slot, joint) in [(0x98, 0x80_0000), (0x9c, 0x120_0000), (0xa0, 0x80_1000)] {
        let em = e.r32(L(e) + slot) as i32;
        APIparticle::EmitterSetJoint(e, em, SELF, joint);
    }
    Ok(())
}

/// 0x00012ec8
pub fn fireGhostBossFUNC_StartFire(e: &mut Engine) -> LResult<()> {
    for slot in [0x98, 0x9c, 0xa0] {
        let em = e.r32(L(e) + slot) as i32;
        APIparticle::EmitterStart(e, em, 1);
    }
    Ok(())
}

/// 0x00012f60
pub fn fireGhostBossFUNC_DestroyFireParticle(e: &mut Engine) -> LResult<()> {
    for slot in [0x98, 0x9c, 0xa0] {
        let em = e.r32(L(e) + slot) as i32;
        APIparticle::EmitterDestroy(e, em);
    }
    Ok(())
}

/// 0x00012fe8
pub fn fireGhostBossFUNC_DestroyFireType(e: &mut Engine) -> LResult<()> {
    for o in [8, 0xc] {
        let t = e.r32(GD(e) + o) as i32;
        APIparticle::TypeDestroy(e, t);
    }
    Ok(())
}

/// 0x00013050 local data 0xbc; first instance: API tables, global data (0x18), the first fireball
/// latent parked at (50, -50, 0), the fire types; every instance: a 1.5 s timer, notifies (key frames
/// -> 5, cut-scenes -> 6), physics, clipping, fire emitters, anim 7 (loop).
/// NOTE (PS2 behaviour kept): local data is fetched before the API tables, and pFunction is fetched
/// twice. The first fireball latent is stored in the FIRST instance's local data only.
pub fn fireghostbossConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    let slot = G(e, pLocalDataFireGhostBoss);
    APIcons::GetLocalData(e, slot, 0xbc);
    if instanceCount == 1 {
        APIcons::SetExternalFunction(e, 0, fireGhostBossST_INITIAL);
        APIcons::SetExternalFunction(e, 1, fireGhostBossST_ENTRY_POINT);
        APIcons::SetExternalFunction(e, 3, fireGhostBossCB_ANIMATION_LAST_FRAME);
        APIcons::SetExternalFunction(e, 4, fireGhostBossCB_TIMER);
        APIcons::SetExternalFunction(e, 5, fireGhostBossCB_ANIM);
        APIcons::SetExternalFunction(e, 6, fireGhostBoss_baseCB_CUTSCENE);
        fn custom(e: &mut Engine, n: &str) -> Ptr {
            let n = e.cstr(n);
            APIcons::GetApiCustom(e, n)
        }
        let tables: [(u32, fn(&mut Engine) -> Ptr); 13] = [
            (pAnim, |e| APIcons::GetApiAnim(e)),
            (pAnimMngr, |e| custom(e, "AnimMngr")),
            (pGameObj, |e| APIcons::GetApiGameObj(e)),
            (pExtra, |e| APIcons::GetApiExtra(e)),
            (pPhysic, |e| APIcons::GetApiPhysic(e)),
            (pSound, |e| APIcons::GetApiNewSound(e)),
            (pStdlib, |e| APIcons::GetApiStdlib(e)),
            (pMath, |e| APIcons::GetApiMath(e)),
            (pFunction, |e| custom(e, "ApiCFunctions")),
            (pDebug, |e| custom(e, "debug")),
            (pMiscFunc, |e| custom(e, "MiscFunc")),
            (pFunction, |e| custom(e, "ApiCFunctions")),
            (pParticle, |e| APIcons::GetApiParticle(e)),
        ];
        for (slot, get) in tables {
            let v = get(e);
            let a = G(e, slot);
            e.w32(a, v);
        }
        let slot = G(e, pGlobalDataFireGhostBoss);
        APIcons::GetGlobalData(e, slot, 0x18);
        let uVar1 = APIgameobj::GetGameBuffer(e, 0, 0x8f8, 1);
        let gd = GD(e);
        e.w32(gd, uVar1);
        let uVar1 = APIgameobj::GetGameBuffer(e, 2, 0x158, 1);
        e.w32(gd + 4, uVar1);
        let uVar1 = APIgameobj::LaunchLatant(e, 0x2a80_001b);
        e.w32(L(e) + 0x48, uVar1);
        let v = e.r32(L(e) + 0x48);
        e.w32(L(e) + 0x58, v);
        let d40 = e.alloca(0x10);
        e.wf(d40, 50.0);
        e.wf(d40 + 4, -50.0);
        e.wf(d40 + 8, 0.0);
        let fb = e.r32(L(e) + 0x48);
        APIgameobj::SetTranslation(e, fb, d40);
        fireGhostBossFUNC_CreateFireType(e)?;
    }
    APIcons::SetMessageFunction(e, fireGhostBossFUNC_MessageHandler);
    let uVar1 = APIextra::TimerCreate(e);
    e.w32(L(e) + 0xb0, uVar1);
    let t = e.r32(L(e) + 0xb0);
    APIextra::TimerSetFrequency(e, t, 1.5);
    APIcons::SetInitialState(e, 0);
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 0xb, 5);
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 0x16, 6);
    fireGhostBossFUNC_CreatePhysic(e)?;
    fireGhostBossFUNC_SetClipping(e)?;
    fireGhostBossFUNC_CreateFireParticle(e)?;
    APIanim::Play(e, SELF, 7, 2, 0.0);
    Ok(())
}

/// 0x000134d8 the fire types go with the last instance
pub fn fireghostbossDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    fireGhostBossFUNC_DestroyFireParticle(e)?;
    let t = e.r32(L(e) + 0xb0);
    APIextra::TimerDestroy(e, t);
    if instanceCount == 0 {
        fireGhostBossFUNC_DestroyFireType(e)?;
    }
    Ok(())
}

// ---- FireGhostBoss_particle.c ----

/// 0x00010d50 blue head fire
pub fn fireGhostBossFUNC_SetParticle_FIRE_HEAD(e: &mut Engine, param_1: u32, param_2: u32) -> LResult<()> {
    let t = param_2 as i32;
    let f = f32::from_bits;
    for (k, v) in [(8, 1), (1, 2), (4, 0x32), (0x10, 0), (0x20, 0), (0x40, 0), (0x80, 0), (0x100, 0)] {
        APIparticle::TypeSetInt(e, t, k, v);
    }
    APIparticle::TypeSetFloat(e, t, 1, 0.0);
    APIparticle::TypeSetFloat(e, t, 4, f(0x3d4c_cccd));
    APIparticle::TypeSetFloat(e, t, 2, f(0x3f80_0000));
    APIparticle::TypeSetFloat(e, t, 8, f(0x4000_0000));
    APIparticle::TypeSetVec(e, t, 1, 0.0, f(0x3fe0_0000), 0.0);
    APIparticle::TypeSetVec(e, t, 2, 0.0, 0.0, 0.0);
    let c = (f(0x3c23_d70a), f(0x3f40_0000), f(0x3c23_d70a), f(0x3f00_0000));
    for k in [1, 2, 4] {
        APIparticle::TypeSetInter(e, t, k, c.0, c.1, c.2, c.3);
    }
    APIparticle::TypeSetInter(e, t, 8, 0.0, 0.0, 0.0, f(0x3f00_0000));
    APIparticle::TypeSetInter(e, t, 0x10, f(0x4000_0000), f(0x3f80_0000), f(0x3d4c_cccd), f(0x3f00_0000));
    APIparticle::TypeSetInter(e, t, 0x20, f(0x3e4c_cccd), f(0x3e4c_cccd), f(0x3e4c_cccd), f(0x3f00_0000));
    Ok(())
}

/// 0x00011098 blue elbow fire
pub fn fireGhostBossFUNC_SetParticle_FIRE_ELBOW(e: &mut Engine, param_1: u32, param_2: u32) -> LResult<()> {
    let t = param_2 as i32;
    let f = f32::from_bits;
    for (k, v) in [(8, 1), (1, 2), (4, 0x32), (0x10, 0), (0x20, 0), (0x40, 0), (0x80, 0), (0x100, 0)] {
        APIparticle::TypeSetInt(e, t, k, v);
    }
    APIparticle::TypeSetFloat(e, t, 1, 0.0);
    APIparticle::TypeSetFloat(e, t, 4, f(0x3dcc_cccd));
    APIparticle::TypeSetFloat(e, t, 2, f(0x3fc0_0000));
    APIparticle::TypeSetFloat(e, t, 8, f(0x3f00_0000));
    APIparticle::TypeSetVec(e, t, 1, 0.0, f(0x4000_0000), 0.0);
    APIparticle::TypeSetVec(e, t, 2, 0.0, 0.0, 0.0);
    let c = (f(0x3c23_d70a), f(0x3f40_0000), f(0x3c23_d70a), f(0x3f40_0000));
    for k in [1, 2, 4] {
        APIparticle::TypeSetInter(e, t, k, c.0, c.1, c.2, c.3);
    }
    APIparticle::TypeSetInter(e, t, 8, 0.0, 0.0, f(0x3c23_d70a), f(0x3f40_0000));
    APIparticle::TypeSetInter(e, t, 0x10, f(0x3fc0_0000), f(0x3f40_0000), f(0x3c23_d70a), f(0x3f40_0000));
    APIparticle::TypeSetInter(e, t, 0x20, f(0x3e4c_cccd), f(0x3e4c_cccd), f(0x3e4c_cccd), f(0x3f40_0000));
    Ok(())
}

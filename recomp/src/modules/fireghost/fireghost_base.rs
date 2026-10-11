//! FireGhost_Base.c (42 functions): the fire ghost AI. Addresses are module-relative (.text at
//! 0x10000) in fireghost__6f63c7c0.
//!
//! Sub-states (code pointer at L+0x160, run by external state 1 every frame after the bobbing
//! fireGhostBaseFUNC_Wave): SearchingHero (float along the waypoint path L+0x1f0; hero within
//! L+0x1dc and not hiding: L+0x1a0 bit 4; waypoint reached within 3 m: bit 2) -> BeingSurprised
//! (anim 1) -> AttackPreparation (hero within L+0x1e0: turn by L+0x1c8 per frame until facing him,
//! then launch the fire latent, msg 0x1af) -> Attack (anim; its event releases the latent: 0x110)
//! -> AttackPreparation ... Water (msg 0xd8, or 0x1a9 while burning: bit 0x40) -> Extinguished
//! (smoke; on anim 1's end fireGhostBaseFUNC_Extinguishe swaps in the ghost inside, L+0x1a4) ->
//! the relight timer L+0x1e8 (bit 0x80, unless the hero holds the book open: game buffer 0 +0x14
//! bit 0) -> fireGhostBaseFUNC_CatchOnFire -> SearchingHero. Scared_In/Execute (anim, then chase
//! the hero for the L+0x1e4 period: bit 0x200) is compiled in but never entered.
//! Other L+0x1a0 bits: 8 anim done (anim id in L+0x1fc).
//!
//! The builds differ only in literals (see [`Cfg`]) plus one sound in fireGhostBaseFUNC_Extinguishe
//! (3a7b55ab).
use super::*;
use crate::engine::*;
use crate::logic::LResult;

/// Reference-build addresses of the sub-state functions stored at L+0x160 (the port stores the
/// reference build's addresses in every build; only [`call_state`] reads them).
pub mod addr {
    pub const fireGhostBaseST_SearchingHero_In: u32 = 0x14c28;
    pub const fireGhostBaseST_SearchingHero_Execute: u32 = 0x14cf0;
    pub const fireGhostbaseST_BeingSurprised_In: u32 = 0x14ec8;
    pub const fireGhostbaseST_BeingSurprised_Execute: u32 = 0x14f68;
    pub const fireGhostBaseST_AttackPreparation_In: u32 = 0x15018;
    pub const fireGhostBaseST_AttackPreparation_Execute: u32 = 0x15308;
    pub const fireGhostBaseST_Attack_In: u32 = 0x154c0;
    pub const fireGhostBaseST_Attack_Execute: u32 = 0x15548;
    pub const fireGhostBaseST_Extinguished_In: u32 = 0x155f8;
    pub const fireGhostBaseST_Extinguished_Execute: u32 = 0x15730;
    pub const fireGhostBaseST_Scared_In: u32 = 0x15848;
    pub const fireGhostBaseST_Scared_Execute: u32 = 0x158f0;
}

/// `(**(code **)(L + 0x160))()`: run the current sub-state.
pub fn call_state(e: &mut Engine) -> LResult<()> {
    let f = e.r32(L(e) + 0x160);
    match f {
        addr::fireGhostBaseST_SearchingHero_In => fireGhostBaseST_SearchingHero_In(e),
        addr::fireGhostBaseST_SearchingHero_Execute => fireGhostBaseST_SearchingHero_Execute(e),
        addr::fireGhostbaseST_BeingSurprised_In => fireGhostbaseST_BeingSurprised_In(e),
        addr::fireGhostbaseST_BeingSurprised_Execute => fireGhostbaseST_BeingSurprised_Execute(e),
        addr::fireGhostBaseST_AttackPreparation_In => fireGhostBaseST_AttackPreparation_In(e),
        addr::fireGhostBaseST_AttackPreparation_Execute => fireGhostBaseST_AttackPreparation_Execute(e),
        addr::fireGhostBaseST_Attack_In => fireGhostBaseST_Attack_In(e),
        addr::fireGhostBaseST_Attack_Execute => fireGhostBaseST_Attack_Execute(e),
        addr::fireGhostBaseST_Extinguished_In => fireGhostBaseST_Extinguished_In(e),
        addr::fireGhostBaseST_Extinguished_Execute => fireGhostBaseST_Extinguished_Execute(e),
        addr::fireGhostBaseST_Scared_In => fireGhostBaseST_Scared_In(e),
        addr::fireGhostBaseST_Scared_Execute => fireGhostBaseST_Scared_Execute(e),
        // the PS2 would jump to whatever L+0x160 holds (0 before fireGhostBaseST_INITIAL has run
        // once): treat it as a fault
        _ => panic!("fireghost: sub-state code pointer {f:#x} is not a known function"),
    }
}

/// `*(code **)(L + 0x160) = f; (**(code **)(L + 0x160))();`
fn go(e: &mut Engine, f: u32) -> LResult<()> {
    let l = L(e);
    e.w32(l + 0x160, f);
    call_state(e)
}
/// L+0x1a0 (event bits)
#[inline]
fn ev(e: &Engine) -> u32 {
    e.r32(L(e) + 0x1a0)
}
#[inline]
fn set_ev(e: &mut Engine, bits: u32) {
    let a = L(e) + 0x1a0;
    or32(e, a, bits);
}
#[inline]
fn clr_ev(e: &mut Engine, bits: u32) {
    let a = L(e) + 0x1a0;
    and32(e, a, !bits);
}
/// the leader hero (`*(*pGlobalDataMonster_Base + 8)`)
#[inline]
fn hero(e: &Engine) -> u32 {
    e.r32(e.r32(GDM(e)) + 8)
}
/// game buffer 0 +0x14 (hero flags), through `pGlobalDataFireGhostBase + 4`
#[inline]
fn game_flags(e: &Engine) -> u32 {
    e.r32(e.r32(GDF(e) + 4) + 0x14)
}
/// a zeroed DTVector3d on the stack
fn vec3(e: &mut Engine, x: f32, y: f32, z: f32) -> Ptr {
    let v = e.alloca(0x10);
    e.wf(v, x);
    e.wf(v + 4, y);
    e.wf(v + 8, z);
    v
}

/// 0x000149d8 external state 0: type-0x12 fire ghosts wait for a spawn position (msg 0x1b2 sets
/// L+0x1bc); the others start at once (spawn sound). Then SearchingHero, external state 1.
pub fn fireGhostBaseST_INITIAL(e: &mut Engine, _: u32) -> LResult<u64> {
    let iVar1 = L(e);
    APIgameobj::SetContextFlag(e, SELF, 10);
    if e.r32(L(e) + 0x44) == 0x12 {
        if e.r32(iVar1 + 0x1bc) != 0 {
            APIgameobj::SetTranslation(e, SELF, iVar1 + 0x1a8);
            fireGhostBaseFUNC_Initialisation(e)?;
            e.w32(iVar1 + 0x160, addr::fireGhostBaseST_SearchingHero_In);
            APIgameobj::SetNextState(e, SELF, 1);
        }
    } else {
        fireGhostBaseFUNC_Initialisation(e)?;
        let s = cfg(e).spawn_sound;
        APInewSound::Play(e, SELF, s, 0);
        e.w32(iVar1 + 0x160, addr::fireGhostBaseST_SearchingHero_In);
        APIgameobj::SetNextState(e, SELF, 1);
    }
    let to = cfg(e).ea_target;
    APIgameobj::SendMessageTo(e, to, NULL, 0xeb, 1, NULL);
    Ok(1)
}

/// 0x00014b60 external state 1: put out (bit 0x40) -> Extinguished; bob; run the sub-state
pub fn fireGhostBaseST_ENTRY_POINT(e: &mut Engine, _: u32) -> LResult<u64> {
    if ev(e) & 0x40 != 0 {
        clr_ev(e, 0x40);
        go(e, addr::fireGhostBaseST_Extinguished_In)?;
    }
    fireGhostBaseFUNC_Wave(e)?;
    call_state(e)?;
    Ok(0)
}

/// 0x00014c28
pub fn fireGhostBaseST_SearchingHero_In(e: &mut Engine) -> LResult<()> {
    let iVar1 = L(e);
    let a = cfg(e).anim_search;
    LIBAnimMngr::PlayAnim(e, 0.0, a, 2, 0);
    APIgameobj::SetContextFlag(e, SELF, 0x3a);
    let path = e.r32(iVar1 + 0x1f0);
    fireGhostBaseFUNC_FollowPath(e, path, 0)?;
    fireGhostBaseFUNC_CreateFireParticle(e)?;
    go(e, addr::fireGhostBaseST_SearchingHero_Execute)
}

/// 0x00014cf0
pub fn fireGhostBaseST_SearchingHero_Execute(e: &mut Engine) -> LResult<()> {
    let iVar1 = L(e);
    let r = e.rf(L(e) + 0x1dc);
    let h = hero(e);
    let bVar2 = LIBApiCFunctions::CheckRangeObject(e, r, SELF, h, 1);
    if bVar2 != 0 && game_flags(e) & 0x4900_096c == 0 {
        set_ev(e, 4);
    }
    let bVar2 = LIBApiCFunctions::CheckRangePoint(e, 3.0, SELF, iVar1 + 0x180, 1);
    if bVar2 != 0 {
        set_ev(e, 2);
    }
    if ev(e) & 4 == 0 {
        if ev(e) & 2 != 0 {
            clr_ev(e, 2);
            let path = e.r32(L(e) + 0x1f0);
            fireGhostBaseFUNC_FollowPath(e, path, 1)?;
        }
        Ok(())
    } else {
        clr_ev(e, 4);
        go(e, addr::fireGhostbaseST_BeingSurprised_In)
    }
}

/// 0x00014ec8
pub fn fireGhostbaseST_BeingSurprised_In(e: &mut Engine) -> LResult<()> {
    APIgameobj::SetContextFlag(e, SELF, 10);
    LIBAnimMngr::PlayAnim(e, 0.0, 1, 1, 3);
    go(e, addr::fireGhostbaseST_BeingSurprised_Execute)
}

/// 0x00014f68
pub fn fireGhostbaseST_BeingSurprised_Execute(e: &mut Engine) -> LResult<()> {
    if ev(e) & 8 != 0 {
        clr_ev(e, 8);
        if e.r32(L(e) + 0x1fc) == 1 {
            return go(e, addr::fireGhostBaseST_AttackPreparation_In);
        }
    }
    Ok(())
}

/// 0x00015018 hero still within L+0x1e0: compute the angle to him (L+0x1c0), the turn step
/// L+0x1c8 (+/-0.1257 rad, sign from the cross product's y) and make the first step; else back to
/// SearchingHero.
pub fn fireGhostBaseST_AttackPreparation_In(e: &mut Engine) -> LResult<()> {
    let iVar1 = L(e);
    let r = e.rf(L(e) + 0x1e0);
    let h = hero(e);
    let bVar2 = LIBApiCFunctions::CheckRangeObject(e, r, SELF, h, 1);
    if bVar2 != 0 {
        e.w32(iVar1 + 0x1cc, 1);
        e.w32(iVar1 + 0x1c8, 0x3e00_adfd);
        e.w32(iVar1 + 0x1c4, 0);
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
        e.wf(iVar1 + 0x1c0, fVar3);
        // only the y component of (look x dir) is used
        let fStack_3c = e.rf(d50 + 8) * e.rf(d60) - e.rf(d50) * e.rf(d60 + 8);
        if fStack_3c < 0.0 {
            let v = e.rf(iVar1 + 0x1c0) * -1.0;
            e.wf(iVar1 + 0x1c0, v);
            let v = e.rf(iVar1 + 0x1c8) * -1.0;
            e.wf(iVar1 + 0x1c8, v);
            e.w32(iVar1 + 0x1cc, 0);
        }
        let v = e.rf(iVar1 + 0x1c4) + e.rf(iVar1 + 0x1c8);
        e.wf(iVar1 + 0x1c4, v);
        let a = e.rf(iVar1 + 0x1c4);
        APIgameobj::Rotate(e, SELF, iVar1 + 0x18c, a);
        APIgameobj::SetContextFlag(e, SELF, 10);
        go(e, addr::fireGhostBaseST_AttackPreparation_Execute)
    } else {
        go(e, addr::fireGhostBaseST_SearchingHero_In)
    }
}

/// 0x00015308 keep turning until the accumulated angle L+0x1c4 passes L+0x1c0; then, unless the
/// hero is hiding / busy (game flags 0x4900094c), launch the fire latent (L+0x1b4, msg 0x1af with
/// our id) and attack.
pub fn fireGhostBaseST_AttackPreparation_Execute(e: &mut Engine) -> LResult<()> {
    let iVar2 = L(e);
    let c = e.rf(iVar2 + 0x1c4);
    let a = e.rf(iVar2 + 0x1c0);
    let s = e.r32(iVar2 + 0x1cc);
    if (c <= a && s != 0) || (a <= c && s == 0) {
        let v = c + e.rf(iVar2 + 0x1c8);
        e.wf(iVar2 + 0x1c4, v);
        let step = e.rf(iVar2 + 0x1c8);
        APIgameobj::Rotate(e, SELF, iVar2 + 0x18c, step);
    } else if game_flags(e) & 0x4900_094c == 0 {
        let id = cfg(e).attack_latent;
        let uVar3 = APIgameobj::LaunchLatant(e, id);
        e.w32(iVar2 + 0x1b4, uVar3);
        let uVar4 = APIgameobj::GetID(e, SELF, NULL);
        let to = e.r32(iVar2 + 0x1b4);
        APIgameobj::SendMessageTo(e, to, NULL, 0x1af, uVar4, NULL);
        return go(e, addr::fireGhostBaseST_Attack_In);
    }
    Ok(())
}

/// 0x000154c0
pub fn fireGhostBaseST_Attack_In(e: &mut Engine) -> LResult<()> {
    let a = cfg(e).anim_attack;
    LIBAnimMngr::PlayAnim(e, 0.1, a, 1, 3);
    go(e, addr::fireGhostBaseST_Attack_Execute)
}

/// 0x00015548
pub fn fireGhostBaseST_Attack_Execute(e: &mut Engine) -> LResult<()> {
    if ev(e) & 8 != 0 {
        clr_ev(e, 8);
        if e.r32(L(e) + 0x1fc) == cfg(e).anim_attack as u32 {
            return go(e, addr::fireGhostBaseST_AttackPreparation_In);
        }
    }
    Ok(())
}

/// 0x000155f8 put out: no scared timer, anim 1, fire -> smoke (smoke timer L+0x1ec)
pub fn fireGhostBaseST_Extinguished_In(e: &mut Engine) -> LResult<()> {
    let iVar1 = L(e);
    clr_ev(e, 0x80);
    e.w32(L(e) + 0x1b8, 0);
    let t = e.r32(iVar1 + 0x1e4);
    APIextra::TimerUnregisterNotify(e, SELF, t);
    LIBAnimMngr::PlayAnim(e, 0.1, 1, 1, 3);
    fireGhostBaseFUNC_DestroyFireParticle(e)?;
    fireGhostBaseFUNC_CreateSmokeParticle(e)?;
    let t = e.r32(iVar1 + 0x1ec);
    APIextra::TimerReset(e, t);
    APIextra::TimerRegisterNotify(e, SELF, t, 4);
    go(e, addr::fireGhostBaseST_Extinguished_Execute)
}

/// 0x00015730 anim 1 done -> swap in the ghost; relight timer (bit 0x80) -> burn again
pub fn fireGhostBaseST_Extinguished_Execute(e: &mut Engine) -> LResult<()> {
    if ev(e) & 8 == 0 {
        if ev(e) & 0x80 != 0 {
            clr_ev(e, 0x80);
            fireGhostBaseFUNC_CatchOnFire(e)?;
            return go(e, addr::fireGhostBaseST_SearchingHero_In);
        }
    } else {
        clr_ev(e, 8);
        if e.r32(L(e) + 0x1fc) == 1 {
            fireGhostBaseFUNC_Extinguishe(e)?;
        }
    }
    Ok(())
}

/// 0x00015848 (never entered)
pub fn fireGhostBaseST_Scared_In(e: &mut Engine) -> LResult<()> {
    APIgameobj::SetContextFlag(e, SELF, 10);
    let a = cfg(e).anim_scared;
    LIBAnimMngr::PlayAnim(e, 0.1, a, 1, 3);
    go(e, addr::fireGhostBaseST_Scared_Execute)
}

/// 0x000158f0 (never entered) scared anim done: loop anim, chase the hero (move DM mode 0x11,
/// turn DM speed), scared timer L+0x1e4; its end (bit 0x200) -> SearchingHero
pub fn fireGhostBaseST_Scared_Execute(e: &mut Engine) -> LResult<()> {
    let iVar1 = L(e);
    if ev(e) & 8 == 0 {
        if ev(e) & 0x200 != 0 {
            clr_ev(e, 0x200);
            let t = e.r32(iVar1 + 0x1e4);
            APIextra::TimerUnregisterNotify(e, SELF, t);
            return go(e, addr::fireGhostBaseST_SearchingHero_In);
        }
    } else {
        clr_ev(e, 8);
        if e.r32(L(e) + 0x1fc) == cfg(e).anim_scared as u32 {
            let a = cfg(e).anim_scared_loop;
            LIBAnimMngr::PlayAnim(e, 0.0, a, 2, 0);
            APIgameobj::SetContextFlag(e, SELF, 0x3a);
            let b = e.alloca(0x80);
            let dm = e.r32(iVar1 + 4);
            APIphysic::DMGetParams(e, dm, b);
            let h = hero(e);
            e.w32(b, h);
            e.w32(b + 0x14, 0x11);
            APIphysic::DMSetParams(e, dm, b);
            let b = e.alloca(0x80);
            let dm = e.r32(iVar1 + 8);
            APIphysic::DMGetParams(e, dm, b);
            let turn = cfg(e).scared_turn;
            for (o, v) in [(0, 0), (4, 0), (8, turn), (0xc, 1)] {
                e.w32(b + o, v);
            }
            APIphysic::DMSetParams(e, dm, b);
            let t = e.r32(iVar1 + 0x1e4);
            APIextra::TimerReset(e, t);
            APIextra::TimerRegisterNotify(e, SELF, t, 4);
        }
    }
    Ok(())
}

/// 0x00015b40 external 3 (anim done): remember the anim (L+0x1fc), bit 8
pub fn fireGhostBaseCB_ANIMATION_LAST_FRAME(e: &mut Engine, param_1: u32) -> LResult<u64> {
    let l = L(e);
    e.w32(l + 0x1fc, param_1);
    set_ev(e, 8);
    Ok(0)
}

/// 0x00015ba0 external 4 (timers): relight L+0x1e8 (re-armed while the book is open), scared
/// L+0x1e4, smoke L+0x1ec
pub fn fireGhostBaseCB_TIMER(e: &mut Engine, param_1: u32) -> LResult<u64> {
    if param_1 == e.r32(L(e) + 0x1e8) {
        if game_flags(e) & 1 == 0 {
            set_ev(e, 0x80);
        } else {
            let t = e.r32(L(e) + 0x1e8);
            APIextra::TimerReset(e, t);
        }
    } else if param_1 == e.r32(L(e) + 0x1e4) {
        set_ev(e, 0x200);
    } else if param_1 == e.r32(L(e) + 0x1ec) {
        fireGhostBaseFUNC_DestroySmokeParticle(e)?;
    }
    Ok(0)
}

/// 0x00015cd0 external 5 (anim event {anim, event}): the attack anim's event 1 throws the fire
/// latent (msg 0x110)
pub fn fireGhostBaseCB_ANIM(e: &mut Engine, param_1: u32) -> LResult<u64> {
    let iVar1 = L(e);
    if e.r32(param_1) == cfg(e).anim_attack as u32 && e.r32(param_1 + 4) == 1 && e.r32(L(e) + 0x1b4) != 0 {
        let to = e.r32(L(e) + 0x1b4);
        APIgameobj::SendMessageTo(e, to, NULL, 0x110, 0, NULL);
        e.w32(iVar1 + 0x1b4, 0);
    }
    Ok(0)
}

/// 0x00015d88 one more fire ghost alive (L+0x1d0), tell the ghost inside who we are (0x1b3), last
/// waypoint index (L+0x1f8), rotation axis (0,1,0) at L+0x18c, burning (L+0x1b8)
pub fn fireGhostBaseFUNC_Initialisation(e: &mut Engine) -> LResult<()> {
    let iVar2 = L(e);
    let v = e.r32(L(e) + 0x1d0).wrapping_add(1);
    e.w32(L(e) + 0x1d0, v);
    APIgameobj::SetContextFlag(e, SELF, 0x3a);
    fireGhostBaseFUNC_SetClipping(e)?;
    let uVar3 = APIgameobj::GetID(e, SELF, NULL);
    let to = e.r32(iVar2 + 0x1a4);
    APIgameobj::SendMessageTo(e, to, NULL, 0x1b3, uVar3, NULL);
    let path = e.r32(L(e) + 0x1f0);
    let iVar4 = APIextra::WaypointGetNb(e, path);
    e.w32(iVar2 + 0x1f8, iVar4.wrapping_sub(1));
    e.w32(iVar2 + 0x18c, 0);
    e.w32(iVar2 + 400, 0x3f80_0000);
    e.w32(iVar2 + 0x194, 0);
    e.w32(L(e) + 0x1b8, 1);
    Ok(())
}

/// 0x00015eb8 clip distances 25 / 50
pub fn fireGhostBaseFUNC_SetClipping(e: &mut Engine) -> LResult<()> {
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

/// 0x00015f60 never clipped
pub fn fireGhostBaseFUNC_SetInfiniteClipping(e: &mut Engine) -> LResult<()> {
    let b = e.alloca(0x40);
    APIgameobj::GetClippingParams(e, SELF, b);
    e.w32(b + 4, 0xbf80_0000);
    e.w32(b + 0xc, 0xa7);
    e.w32(b + 8, 0xa7);
    APIgameobj::SetClippingParams(e, SELF, b, 0);
    Ok(())
}

/// 0x00015fe0 next waypoint (param_2 != 0: L+500 + 1, wrapping to 0 after the last; 0: the
/// nearest) -> L+0x180, move DM towards it (mode 5)
pub fn fireGhostBaseFUNC_FollowPath(e: &mut Engine, param_1: u32, param_2: i32) -> LResult<()> {
    let iStack_40 = L(e);
    if e.r32(L(e) + 500) < e.r32(L(e) + 0x1f8) && param_2 != 0 {
        let v = e.r32(L(e) + 500).wrapping_add(1);
        e.w32(L(e) + 500, v);
    } else if param_2 == 0 {
        let uVar1 = fireGhostBaseFUNC_GetNearestWP(e, param_1)?;
        e.w32(iStack_40 + 500, uVar1);
    } else {
        e.w32(L(e) + 500, 0);
    }
    let i = e.r32(iStack_40 + 500);
    APIextra::WaypointGetPos(e, param_1, i, iStack_40 + 0x180);
    let b = e.alloca(0x80);
    let dm = e.r32(iStack_40 + 4);
    APIphysic::DMGetParams(e, dm, b);
    for (o, s) in [(8, 0x180), (0xc, 0x184), (0x10, 0x188)] {
        let v = e.r32(iStack_40 + s);
        e.w32(b + o, v);
    }
    e.w32(b + 0x14, 5);
    e.w32(b, 0);
    APIphysic::DMSetParams(e, dm, b);
    Ok(())
}

/// 0x00016140 index of the waypoint nearest to us (xz distance) among 0..L+0x1f8 (the last
/// waypoint is never chosen). The bound is unsigned: a path without waypoints (L+0x1f8 = -1) makes
/// this loop ~4G times, as on the PS2.
pub fn fireGhostBaseFUNC_GetNearestWP(e: &mut Engine, param_1: u32) -> LResult<u32> {
    let iVar1 = L(e);
    let mut uStack_2c = 0u32;
    let mut fStack_28: f32 = 0.0;
    let mut fStack_24: f32 = 100000.0;
    let d60 = e.alloca(0x10);
    let d50 = e.alloca(0x10);
    let d40 = e.alloca(0x10);
    APIgameobj::GetPosition(e, SELF, ALL, d60);
    let mut uStack_30 = 0u32;
    while uStack_30 < e.r32(iVar1 + 0x1f8) {
        APIextra::WaypointGetPos(e, param_1, uStack_30, d50);
        let x = e.rf(d60) - e.rf(d50);
        let z = e.rf(d60 + 8) - e.rf(d50 + 8);
        e.wf(d40, x);
        e.wf(d40 + 4, 0.0);
        e.wf(d40 + 8, z);
        fStack_28 = APImath::CVectorLength(e, d40);
        if fStack_28 <= fStack_24 {
            uStack_2c = uStack_30;
            fStack_24 = fStack_28;
        }
        uStack_30 += 1;
    }
    Ok(uStack_2c)
}

/// 0x00016290 relight: the shell takes the ghost's place and orientation, the ghost goes to the
/// origin; visible, clipped, 0xeb to the level manager
pub fn fireGhostBaseFUNC_CatchOnFire(e: &mut Engine) -> LResult<()> {
    let iVar1 = L(e);
    let path = e.r32(L(e) + 0x1f0);
    let uVar2 = fireGhostBaseFUNC_GetNearestWP(e, path)?;
    e.w32(iVar1 + 500, uVar2);
    let t = e.r32(iVar1 + 0x1e8);
    APIextra::TimerUnregisterNotify(e, SELF, t);
    e.w32(iVar1 + 0x1b8, 1);
    let d40 = vec3(e, 0.0, 0.0, 0.0);
    let d50 = e.alloca(0x10);
    let ghost = e.r32(L(e) + 0x1a4);
    APIgameobj::GetPosition(e, ghost, ALL, d50);
    let ghost = e.r32(L(e) + 0x1a4);
    APIgameobj::TakeOrientation(e, SELF, ghost, ALL);
    APIgameobj::SetTranslation(e, SELF, d50);
    let ghost = e.r32(L(e) + 0x1a4);
    APIgameobj::SetTranslation(e, ghost, d40);
    APIgameobj::SetGameObjectVisible(e, SELF, 1);
    APIgameobj::SetContextFlag(e, SELF, 0x3a);
    fireGhostBaseFUNC_SetClipping(e)?;
    let to = cfg(e).ea_target;
    APIgameobj::SendMessageTo(e, to, NULL, 0xeb, 1, NULL);
    Ok(())
}

/// 0x00016438 (still burning flag L+0x1b8 clear) swap in the ghost: it takes our place and
/// orientation and enters state 0xe, the shell goes to (35, 35, 35); relight timer armed.
pub fn fireGhostBaseFUNC_Extinguishe(e: &mut Engine) -> LResult<()> {
    let iVar1 = L(e);
    if e.r32(L(e) + 0x1b8) == 0 {
        if cfg(e).extinguish_sound {
            APInewSound::Play(e, 0x2a00_0017, 0x8000_0009, 0);
        }
        fireGhostBaseFUNC_SetInfiniteClipping(e)?;
        let d50 = e.alloca(0x10);
        APIgameobj::GetPosition(e, SELF, ALL, d50);
        let ghost = e.r32(iVar1 + 0x1a4);
        APIgameobj::TakeOrientation(e, ghost, SELF, ALL);
        let d40 = vec3(e, 35.0, 35.0, 35.0);
        APIgameobj::SetTranslation(e, SELF, d40);
        APIgameobj::SetContextFlag(e, SELF, 10);
        APIgameobj::SetTranslation(e, ghost, d50);
        let to = cfg(e).ea_target;
        APIgameobj::SendMessageTo(e, to, NULL, 0xeb, 1, NULL);
        APIgameobj::SetNextState(e, ghost, 0xe);
        let t = e.r32(iVar1 + 0x1e8);
        APIextra::TimerReset(e, t);
        APIextra::TimerRegisterNotify(e, SELF, t, 4);
        let t = e.r32(L(e) + 0x1e8);
        let p = cfg(e).relight_period;
        APIextra::TimerSetFrequency(e, t, p);
    }
    Ok(())
}

/// 0x00016618 bob around the start height (L+0x16c): phase L+0x238 in [-1, 1] advanced by L+0x23c,
/// which is re-drawn at random (0.05..0.15) at each end
pub fn fireGhostBaseFUNC_Wave(e: &mut Engine) -> LResult<u32> {
    let iVar1 = L(e);
    let v = e.rf(L(e) + 0x238) + e.rf(L(e) + 0x23c);
    e.wf(L(e) + 0x238, v);
    if 1.0 <= e.rf(iVar1 + 0x238) {
        e.w32(iVar1 + 0x238, 0x3f80_0000);
        let uVar2 = APIstdlib::MyRand(e);
        e.wf(iVar1 + 0x23c, -(((uVar2 as i32) as f32 / 32768.0) * 0.1 + 0.05));
    } else if e.rf(iVar1 + 0x238) <= -1.0 {
        e.w32(iVar1 + 0x238, 0xbf80_0000);
        let uVar2 = APIstdlib::MyRand(e);
        e.wf(iVar1 + 0x23c, ((uVar2 as i32) as f32 / 32768.0) * 0.1 + 0.05);
    }
    let ph = e.rf(iVar1 + 0x238);
    let fStack_40 = (1.0 - ph * ph) * 0.15;
    let d50 = e.alloca(0x10);
    APIgameobj::GetPosition(e, SELF, ALL, d50);
    let y0 = e.rf(iVar1 + 0x16c);
    if 0.0 < e.rf(iVar1 + 0x23c) {
        e.wf(d50 + 4, y0 + fStack_40);
    } else {
        e.wf(d50 + 4, y0 - fStack_40);
    }
    Ok(APIgameobj::SetTranslation(e, SELF, d50))
}

/// 0x00016858 messages: 0xd4 kill; 0x68 the ghost inside was captured (one fewer alive, hide at
/// the origin, no timers); 0xc9 restart (relight, back to state 0); 0xd8 water / 0x1a9 water while
/// burning (bit 0x40, relight timer restarted); 0x1b2 spawn position for type 0x12 (max 3 alive).
pub fn fireGhostBaseFUNC_MessageHandler(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    let iVar1 = L(e);
    if param_1 == 0xd4 {
        APIgameobj::KillObject(e, SELF);
    } else if param_1 < 0xd5 {
        if param_1 == 0x68 {
            let v = e.r32(L(e) + 0x1d0).wrapping_sub(1);
            e.w32(L(e) + 0x1d0, v);
            e.w32(iVar1 + 0x1bc, 0);
            let d60 = vec3(e, 0.0, 0.0, 0.0);
            APIgameobj::SetTranslation(e, SELF, d60);
            APIgameobj::SetContextFlag(e, SELF, 10);
            let t = e.r32(iVar1 + 0x1e4);
            APIextra::TimerUnregisterNotify(e, SELF, t);
            let t = e.r32(iVar1 + 0x1e8);
            APIextra::TimerUnregisterNotify(e, SELF, t);
        } else if param_1 == 0xc9 {
            let iStack_40 = L(e);
            let d50 = vec3(e, 0.0, 0.0, 0.0);
            e.w32(L(e) + 0x1a0, 0);
            e.w32(iVar1 + 0x1d0, 0);
            e.w32(iVar1 + 0x1bc, 0);
            let t = e.r32(iVar1 + 0x1e8);
            APIextra::TimerUnregisterNotify(e, SELF, t);
            fireGhostBaseFUNC_CatchOnFire(e)?;
            if e.r32(iStack_40 + 0x44) == 0x12 {
                fireGhostBaseFUNC_SetInfiniteClipping(e)?;
                APIgameobj::SetTranslation(e, SELF, d50);
            } else {
                APIgameobj::SetTranslation(e, SELF, iStack_40 + 0xf8);
            }
            APIgameobj::SetNextState(e, SELF, 0);
        }
    } else if param_1 == 0x1a9 {
        let t = e.r32(L(e) + 0x1e8);
        APIextra::TimerReset(e, t);
        if e.r32(iVar1 + 0x1b8) != 0 {
            set_ev(e, 0x40);
        }
    } else if param_1 < 0x1aa {
        if param_1 == 0xd8 {
            let t = e.r32(L(e) + 0x1e8);
            APIextra::TimerReset(e, t);
            set_ev(e, 0x40);
        }
    } else if param_1 == 0x1b2 && e.r32(L(e) + 0x1d0) < 3 {
        let (x, y, z) = (e.r32(param_3), e.r32(param_3 + 4), e.r32(param_3 + 8));
        e.w32(L(e) + 0x1a8, x);
        e.w32(iVar1 + 0x1ac, y);
        e.w32(iVar1 + 0x1b0, z);
        e.w32(iVar1 + 0x1bc, 1);
        APIgameobj::SetNextState(e, SELF, 0);
    }
    Ok(0)
}

/// the APIparticle table (`pParticle`), passed to the generated SetParticle functions
#[inline]
fn particle_api(e: &Engine) -> u32 {
    e.r32(G(e, pParticle))
}

/// create 4 particle types at GDF+base.. with a texture and the names, then their settings
fn create_types(e: &mut Engine, base: u32, texture: &str, names: [&str; 4], set: [fn(&mut Engine, u32, u32) -> LResult<()>; 4]) -> LResult<()> {
    for i in 0..4 {
        let uVar1 = APIparticle::TypeCreate(e);
        let a = GDF(e) + base + i * 4;
        e.w32(a, uVar1);
    }
    for i in 0..4 {
        let t = e.r32(GDF(e) + base + i * 4) as i32;
        let s = e.cstr(texture);
        APIparticle::TypeSetTexture(e, t, s);
    }
    for (i, n) in names.iter().enumerate() {
        let t = e.r32(GDF(e) + base + i as u32 * 4) as i32;
        let s = e.cstr(n);
        APIparticle::TypeSetName(e, t, s);
    }
    for (i, f) in set.iter().enumerate() {
        let p = particle_api(e);
        let t = e.r32(GDF(e) + base + i as u32 * 4);
        f(e, p, t)?;
    }
    Ok(())
}

/// 0x00016c10 GDF+0x10..0x1c: FIRE_HAND, FIRE_ELBOW, FIRE_SLEEVE, FIRE_HEAD ("Firetorche_blue")
pub fn fireGhostBaseFUNC_CreateFireType(e: &mut Engine) -> LResult<()> {
    create_types(
        e,
        0x10,
        "Firetorche_blue",
        ["FIRE_HAND", "FIRE_ELBOW", "FIRE_SLEEVE", "FIRE_HEAD"],
        [
            fireGhostBaseFUNC_SetParticle_FIRE_HAND,
            fireGhostBaseFUNC_SetParticle_FIRE_ELBOW,
            fireGhostBaseFUNC_SetParticle_FIRE_SLEEVE,
            fireGhostBaseFUNC_SetParticle_FIRE_HEAD,
        ],
    )
}

/// emitter slots (L offsets), the GDF type each uses and its joint
const EMITTERS: [(u32, u32, u32); 7] = [
    (0, 0, 0x100_0000),
    (4, 4, 0x80_0000),
    (8, 8, 0x2c),
    (0xc, 0xc, 0x120_0000),
    (0x10, 8, 0x15),
    (0x14, 4, 0x80_1000),
    (0x18, 0, 0x100_1000),
];

/// create the 7 emitters at L+slot (types GDF+types), set their joints, [smoke: don't clip], start
fn create_emitters(e: &mut Engine, slot: u32, types: u32, dont_clip: bool) -> LResult<()> {
    for (o, t, _) in EMITTERS {
        let ty = e.r32(GDF(e) + types + t) as i32;
        let uVar1 = APIparticle::EmitterCreate(e, ty);
        let a = L(e) + slot + o;
        e.w32(a, uVar1);
    }
    for (o, _, j) in EMITTERS {
        let em = e.r32(L(e) + slot + o) as i32;
        APIparticle::EmitterSetJoint(e, em, SELF, j);
    }
    if dont_clip {
        fireGhostBaseFUNC_DontClipSmokeParticle(e)?;
    }
    for (o, _, _) in EMITTERS {
        let em = e.r32(L(e) + slot + o) as i32;
        APIparticle::EmitterStart(e, em, 1);
    }
    Ok(())
}

/// destroy the 7 emitters at L+slot (if created) and mark them free (-1)
fn destroy_emitters(e: &mut Engine, slot: u32) {
    if e.r32(L(e) + slot) as i32 != -1 {
        for (o, _, _) in EMITTERS {
            let em = e.r32(L(e) + slot + o) as i32;
            APIparticle::EmitterDestroy(e, em);
        }
        let a = L(e) + slot;
        e.w32(a, 0xffff_ffff);
    }
}

/// 0x00016e90 fire on the head, hands, elbows and sleeves (L+0x200..0x218), unless already burning
pub fn fireGhostBaseFUNC_CreateFireParticle(e: &mut Engine) -> LResult<()> {
    if e.r32(L(e) + 0x200) as i32 == -1 {
        create_emitters(e, 0x200, 0x10, false)?;
    }
    Ok(())
}

/// 0x00017228
pub fn fireGhostBaseFUNC_DestroyFireParticle(e: &mut Engine) -> LResult<()> {
    destroy_emitters(e, 0x200);
    Ok(())
}

/// 0x00017360
pub fn fireGhostBaseFUNC_DestroyFireType(e: &mut Engine) -> LResult<()> {
    for o in [0x10, 0x14, 0x18, 0x1c] {
        let t = e.r32(GDF(e) + o) as i32;
        APIparticle::TypeDestroy(e, t);
    }
    Ok(())
}

/// 0x00017408 GDF+0x20..0x2c: Smoke_HAND, Smoke_ELBOW, Smoke_SLEEVE, Smoke_HEAD ("broue")
pub fn fireGhostBaseFUNC_CreateSmokeType(e: &mut Engine) -> LResult<()> {
    create_types(
        e,
        0x20,
        "broue",
        ["Smoke_HAND", "Smoke_ELBOW", "Smoke_SLEEVE", "Smoke_HEAD"],
        [
            fireGhostBaseFUNC_SetParticle_Smoke_HAND,
            fireGhostBaseFUNC_SetParticle_Smoke_ELBOW,
            fireGhostBaseFUNC_SetParticle_Smoke_SLEEVE,
            fireGhostBaseFUNC_SetParticle_Smoke_HEAD,
        ],
    )
}

/// 0x00017688 smoke on the same joints (L+0x21c..0x234), never clipped
pub fn fireGhostBaseFUNC_CreateSmokeParticle(e: &mut Engine) -> LResult<()> {
    if e.r32(L(e) + 0x21c) as i32 == -1 {
        create_emitters(e, 0x21c, 0x20, true)?;
    }
    Ok(())
}

/// 0x00017a38
pub fn fireGhostBaseFUNC_DestroySmokeParticle(e: &mut Engine) -> LResult<()> {
    destroy_emitters(e, 0x21c);
    Ok(())
}

/// 0x00017b70
pub fn fireGhostBaseFUNC_DestroySmokeType(e: &mut Engine) -> LResult<()> {
    for o in [0x20, 0x24, 0x28, 0x2c] {
        let t = e.r32(GDF(e) + o) as i32;
        APIparticle::TypeDestroy(e, t);
    }
    Ok(())
}

/// 0x00017c18 the first smoke emitter's flags without bits 0x40/0x80 (clipping), on all 7
pub fn fireGhostBaseFUNC_DontClipSmokeParticle(e: &mut Engine) -> LResult<()> {
    let b = e.alloca(0x10);
    let em = e.r32(L(e) + 0x21c) as i32;
    APIparticle::EmitterGetFlags(e, em, b);
    let f = e.r32(b) & 0xffff_ff3f;
    e.w32(b, f);
    for (o, _, _) in EMITTERS {
        let em = e.r32(L(e) + 0x21c + o) as i32;
        APIparticle::EmitterSetFlags(e, em, f);
    }
    Ok(())
}

/// 0x00017d78 Monster_Base, local data 0x240; first instance: externals 0/1/3/4/5, API tables,
/// global data (game buffers 0 and 2), a parked fire latent at (35, 0, 0), the particle types.
/// Every instance: message function, initial state 0, anim notifications, physics, clipping, the
/// three timers, idle anim 7, its goCustomDataFireGhost record; killed if captured before.
pub fn fireghost_baseConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    monster_baseConstructor(e, api, instanceCount)?;
    let slot = G(e, pLocalDataFireGhostBase);
    APIcons::GetLocalData(e, slot, 0x240);
    if instanceCount == 1 {
        let d40 = vec3(e, 35.0, 0.0, 0.0);
        let fns: [(i32, crate::logic::ExtFn); 5] = [
            (0, fireGhostBaseST_INITIAL),
            (1, fireGhostBaseST_ENTRY_POINT),
            (3, fireGhostBaseCB_ANIMATION_LAST_FRAME),
            (4, fireGhostBaseCB_TIMER),
            (5, fireGhostBaseCB_ANIM),
        ];
        for (i, f) in fns {
            APIcons::SetExternalFunction(e, i, f);
        }
        let tables: [(u32, fn(&mut Engine) -> Ptr); 10] = [
            (pAnim, |e| APIcons::GetApiAnim(e)),
            (pAnimMngr, |e| {
                let n = e.cstr("AnimMngr");
                APIcons::GetApiCustom(e, n)
            }),
            (pGameObj, |e| APIcons::GetApiGameObj(e)),
            (pExtra, |e| APIcons::GetApiExtra(e)),
            (pPhysic, |e| APIcons::GetApiPhysic(e)),
            (pSound, |e| APIcons::GetApiNewSound(e)),
            (pStdlib, |e| APIcons::GetApiStdlib(e)),
            (pMath, |e| APIcons::GetApiMath(e)),
            (pFunction, |e| {
                let n = e.cstr("ApiCFunctions");
                APIcons::GetApiCustom(e, n)
            }),
            (pParticle, |e| APIcons::GetApiParticle(e)),
        ];
        for (slot, get) in tables {
            let v = get(e);
            let a = G(e, slot);
            e.w32(a, v);
        }
        let slot = G(e, pGlobalDataFireGhostBase);
        APIcons::GetGlobalData(e, slot, 0x30);
        let uVar1 = APIgameobj::GetGameBuffer(e, 0, 0x8f8, 1);
        let gd = GDF(e);
        e.w32(gd + 4, uVar1);
        let uVar1 = APIgameobj::GetGameBuffer(e, 2, 0x158, 1);
        let gd = GDF(e);
        e.w32(gd + 8, uVar1);
        let id = cfg(e).attack_latent;
        let uVar1 = APIgameobj::LaunchLatant(e, id);
        let l = L(e);
        e.w32(l + 0x1b4, uVar1);
        APIgameobj::SetTranslation(e, uVar1, d40);
        fireGhostBaseFUNC_CreateFireType(e)?;
        fireGhostBaseFUNC_CreateSmokeType(e)?;
    }
    APIcons::SetMessageFunction(e, fireGhostBaseFUNC_MessageHandler);
    APIcons::SetInitialState(e, 0);
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 0xb, 5);
    fireGhostBaseFUNC_CreatePhysic(e)?;
    fireGhostBaseFUNC_SetClipping(e)?;
    let (relight, scared) = (cfg(e).relight_period, cfg(e).scared_period);
    for (o, period) in [(0x1e8, relight), (0x1e4, scared), (0x1ec, 0.5)] {
        let uVar1 = APIextra::TimerCreate(e);
        let l = L(e);
        e.w32(l + o, uVar1);
        APIextra::TimerSetFrequency(e, uVar1, period);
    }
    let l = L(e);
    e.w32(l + 0x200, 0xffff_ffff);
    e.w32(l + 0x21c, 0xffff_ffff);
    APIanim::Play(e, SELF, 7, 2, 0.0);
    Monster_BaseFUNC_FillLocalDataParam(e)?;
    fireGhostBaseFUNC_SetInfiniteClipping(e)?;
    APIphysic::PrimResetIntAttribBits(e, SELF, 0xd7, 5, 8);
    APIphysic::PrimSetIntAttribBits(e, SELF, 0xd7, 0, 0x80);
    APIphysic::PrimSetIntAttribBits(e, SELF, 0xd7, 1, 0x8000_0000);
    if e.r32(e.r32(GDF(e) + 8) + 4) & e.r32(L(e) + 0x19c) != 0 {
        APIgameobj::KillObject(e, SELF);
    }
    Ok(())
}

/// 0x000182d8
pub fn fireghost_baseDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    monster_baseDestructor(e)?;
    for o in [0x1e8, 0x1e4, 0x1ec] {
        let t = e.r32(L(e) + o);
        APIextra::TimerDestroy(e, t);
    }
    fireGhostBaseFUNC_DestroyFireParticle(e)?;
    fireGhostBaseFUNC_DestroySmokeParticle(e)?;
    if instanceCount == 0 {
        fireGhostBaseFUNC_DestroyFireType(e)?;
        fireGhostBaseFUNC_DestroySmokeType(e)?;
    }
    Ok(())
}

/// 0x000183d0 physics (weight 0.5, priority 10); DMs: L+4 move (type 1, speed 4, mode 5, context
/// 0x20), L+8 turn (type 4, context 0x10), L+0 type 7 (20, 7; context 8), L+0x20 type 8 (175, 45;
/// context 4)
pub fn fireGhostBaseFUNC_CreatePhysic(e: &mut Engine) -> LResult<()> {
    let puVar1 = L(e);
    APIphysic::CreatePhysic(e, SELF, 0);
    APIphysic::SetFlags(e, SELF, 6);
    APIphysic::SetWeight(e, SELF, 0.5);
    APIphysic::SetPriority(e, SELF, 10);
    let turn = cfg(e).turn_speed;
    let dms: [(u32, u32, &[(u32, u32)], u32); 4] = [
        (4, 1, &[(4, 0x4080_0000), (0, 0), (8, 0), (0xc, 0), (0x10, 0), (0x14, 5)], 0x20),
        (8, 4, &[(0, 0), (4, 0), (8, turn), (0xc, 1)], 0x10),
        (0, 7, &[(0, 0x41a0_0000), (4, 7)], 8),
        (0x20, 8, &[(0, 0x432f_0000), (4, 0x4234_0000)], 4),
    ];
    for (slot, ty, params, mask) in dms {
        let uVar2 = APIphysic::DMAdd(e, SELF, ty);
        e.w32(puVar1 + slot, uVar2);
        let b = e.alloca(0x80);
        let dm = e.r32(puVar1 + slot);
        APIphysic::DMGetParams(e, dm, b);
        for &(o, v) in params {
            e.w32(b + o, v);
        }
        APIphysic::DMSetParams(e, dm, b);
        APIphysic::DMSetContextMask(e, dm, mask);
    }
    Ok(())
}

/// 0x000186f8 our goCustomDataFireGhost record (by object id): id L+0x198, kill mask L+0x19c, path
/// L+0x1f0, ghost inside L+0x1a4; no spawn position; ranges L+0x1e0 / L+0x1dc,0x1d8,0x1d4; start
/// position L+0x168; L+0x5c; group 1, group mask 0x401.
pub fn Monster_BaseFUNC_FillLocalDataParam(e: &mut Engine) -> LResult<()> {
    let iVar1 = L(e);
    let iVar2 = APIgameobj::GetID(e, SELF, NULL);
    let n = cfg(e).fg_records;
    for uStack_3c in 0..n {
        let r = G(e, goCustomDataFireGhost) + uStack_3c * 0x18;
        if e.r32(r) == iVar2 {
            let l = L(e);
            for (d, s) in [(0x198, 0), (0x19c, 0xc), (0x1f0, 8), (0x1a4, 0x10)] {
                let v = e.r32(r + s);
                e.w32(l + d, v);
            }
            break;
        }
    }
    let (r1, r2) = cfg(e).fill_ranges;
    for (o, v) in [(0x1bc, 0), (0x1a8, 0), (0x1ac, 0), (0x1b0, 0), (0x1e0, r1), (0x1dc, r2), (0x1d8, r2), (0x1d4, r2)] {
        e.w32(iVar1 + o, v);
    }
    APIgameobj::GetPosition(e, SELF, ALL, iVar1 + 0x168);
    let v = e.r32(iVar1 + 0xb0);
    e.w32(iVar1 + 0x5c, v);
    APIgameobj::SetGroupID(e, SELF, 1);
    APIgameobj::SetGroupBitMask(e, SELF, 0x401);
    Ok(())
}

/// 0x000189a8
pub fn Monster_BaseCB_NOTIFY_COLLIDE_OBJ(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}
/// 0x000189d8
pub fn Monster_BaseST_IN_CUT_LOOP(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}
/// 0x00018a08
pub fn Monster_BaseST_REINITIALISE(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

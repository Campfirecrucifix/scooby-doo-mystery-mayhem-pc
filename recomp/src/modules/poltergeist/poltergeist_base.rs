//! PoltergeistII_base.c (32 functions): the poltergeist AI. Addresses are module-relative (.text at
//! 0x10000) in poltergeist__0bb93729.
//!
//! Sub-states (code pointer at L+0x160, run by external state 0xf every frame): Standing (wait for
//! the hero within 10 m: L+0x164 bit 4) -> GoGrabThrowable (walk / path-find to the chosen throwable
//! L+0x184; within 1.5 m: bit 0x20) -> PickUpThrowable (anim; on its event bit 0x40 tell the object
//! 0x113/0x115: L+0x188 = holding) -> TrackHero (until within L+0x18c, set by message 0x116: bit 8)
//! -> Attack (wind-up anim, throw: 0x110 to the object, follow-through anim) -> AfterAttack (2.5 s
//! timer: bit 0x100) -> Standing. Type 7 poltergeists attack directly from GoGrab; type 8 ones ride
//! their object (JumpOnThrowable, see the level files). Hooked (0xd6/0x65: bit 0x400) -> Scared
//! (drop the object, face the book, back away from it, or grab a new throwable when one is free).
//! Other L+0x164 bits: 2 adjust the FB effect (clip change), 0x10 hero out of 12 m, 0x200 (msg 199)
//! play the taunt anim, 0x40 anim event, 0x80 anim done.
//!
//! Per-build anim ids are literal slots (builds.rs); shape groups: ST_Initial (04b00cff clears
//! L+0x1f0 again), Standing_Execute (312143a4 sends itself 0xb3 the first time), JumpOnThrowable_In
//! / ChooseWalkType (1c9d41ec: walk anim -1), BaseFUNC_MessageFunction (312143a4, b8041da2: no hook /
//! drop sounds).
use super::builds::*;
use super::*;
use crate::engine::*;
use crate::logic::LResult;

/// Reference-build addresses of the sub-state functions stored at L+0x160 (see mod.rs).
pub mod addr {
    pub const Poltergeist_baseST_Standing_In: u32 = 0x13958;
    pub const Poltergeist_baseST_Standing_Execute: u32 = 0x13a40;
    pub const Poltergeist_baseST_JumpOnThrowable_In: u32 = 0x13b28;
    pub const Poltergeist_baseST_JumpOnThrowable_Execute: u32 = 0x13ca8;
    pub const Poltergeist_baseST_GoGrabThrowable_In: u32 = 0x13d20;
    pub const Poltergeist_baseST_GoGrabThrowable_Execute: u32 = 0x13e88;
    pub const Poltergeist_baseST_PickUpThrowable_In: u32 = 0x13fb8;
    pub const Poltergeist_baseST_PickUpThrowable_Execute: u32 = 0x140b0;
    pub const Poltergeist_baseST_TrackHero_In: u32 = 0x14228;
    pub const Poltergeist_baseST_TrackHero_Execute: u32 = 0x142f0;
    pub const Poltergeist_baseST_Attack_In: u32 = 0x14368;
    pub const Poltergeist_baseST_Attack_Execute: u32 = 0x14460;
    pub const Poltergeist_baseST_AfterAttack_In: u32 = 0x14608;
    pub const Poltergeist_baseST_AfterAttack_Execute: u32 = 0x146d8;
    pub const Poltergeist_baseST_Scared_In: u32 = 0x147c8;
    pub const Poltergeist_baseST_Scared_Execute: u32 = 0x148a8;
    /// Poltergeist.c of 0bb93729
    pub const poltergeist_ST_BlackHoleLoop: u32 = 0x16938;
    /// Poltergeist.c of b8041da2 (its own address)
    pub const PoltergeistST_BlackLoop_In: u32 = 0x166f0;
    /// Poltergeist.c of b8041da2 (its own address)
    pub const PoltergeistST_BlackLoop_Execute: u32 = 0x16778;
    /// Poltergeist.c of fa6a1f63 (its own address)
    pub const PoltergeistST_BlackLoopOfDoom_In: u32 = 0x16798;
    /// Poltergeist.c of fa6a1f63 (its own address)
    pub const PoltergeistST_BlackLoopOfDoom_Execute: u32 = 0x16820;
}

/// `(**(code **)(L + 0x160))()`: run the current sub-state.
pub fn call_state(e: &mut Engine) -> LResult<()> {
    let f = e.r32(L(e) + 0x160);
    match f {
        addr::Poltergeist_baseST_Standing_In => Poltergeist_baseST_Standing_In(e),
        addr::Poltergeist_baseST_Standing_Execute => Poltergeist_baseST_Standing_Execute(e),
        addr::Poltergeist_baseST_JumpOnThrowable_In => Poltergeist_baseST_JumpOnThrowable_In(e),
        addr::Poltergeist_baseST_JumpOnThrowable_Execute => Poltergeist_baseST_JumpOnThrowable_Execute(e),
        addr::Poltergeist_baseST_GoGrabThrowable_In => Poltergeist_baseST_GoGrabThrowable_In(e),
        addr::Poltergeist_baseST_GoGrabThrowable_Execute => Poltergeist_baseST_GoGrabThrowable_Execute(e),
        addr::Poltergeist_baseST_PickUpThrowable_In => Poltergeist_baseST_PickUpThrowable_In(e),
        addr::Poltergeist_baseST_PickUpThrowable_Execute => Poltergeist_baseST_PickUpThrowable_Execute(e),
        addr::Poltergeist_baseST_TrackHero_In => Poltergeist_baseST_TrackHero_In(e),
        addr::Poltergeist_baseST_TrackHero_Execute => Poltergeist_baseST_TrackHero_Execute(e),
        addr::Poltergeist_baseST_Attack_In => Poltergeist_baseST_Attack_In(e),
        addr::Poltergeist_baseST_Attack_Execute => Poltergeist_baseST_Attack_Execute(e),
        addr::Poltergeist_baseST_AfterAttack_In => Poltergeist_baseST_AfterAttack_In(e),
        addr::Poltergeist_baseST_AfterAttack_Execute => Poltergeist_baseST_AfterAttack_Execute(e),
        addr::Poltergeist_baseST_Scared_In => Poltergeist_baseST_Scared_In(e),
        addr::Poltergeist_baseST_Scared_Execute => Poltergeist_baseST_Scared_Execute(e),
        addr::poltergeist_ST_BlackHoleLoop => poltergeist_ST_BlackHoleLoop(e),
        addr::PoltergeistST_BlackLoop_In => PoltergeistST_BlackLoop_In(e),
        addr::PoltergeistST_BlackLoop_Execute => PoltergeistST_BlackLoop_Execute(e),
        addr::PoltergeistST_BlackLoopOfDoom_In => PoltergeistST_BlackLoopOfDoom_In(e),
        addr::PoltergeistST_BlackLoopOfDoom_Execute => PoltergeistST_BlackLoopOfDoom_Execute(e),
        // the PS2 would jump to address 0 (L+0x160 is cleared by Monster_BaseST_REINITIALISE and
        // only set again by Poltergeist_baseST_Initial, which runs first): treat it as a fault
        _ => panic!("poltergeist: sub-state code pointer {f:#x} is not a known function"),
    }
}

/// `*(code **)(L + 0x160) = f`
#[inline]
pub(crate) fn set_state(e: &mut Engine, f: u32) {
    let l = L(e);
    e.w32(l + 0x160, f);
}
#[inline]
fn state(e: &Engine) -> u32 {
    e.r32(L(e) + 0x160)
}
/// L+0x164 (event bits) &= mask
#[inline]
fn clr(e: &mut Engine, mask: u32) {
    let a = L(e) + 0x164;
    and32(e, a, mask);
}
#[inline]
fn ev(e: &Engine) -> u32 {
    e.r32(L(e) + 0x164)
}

/// 0x00013740 external state 0x10: reset the sub-state data, remember the start position (L+0x178),
/// full health, group 1, the path (if any), pre-logic notification, Standing, FB effect; then 0xf.
pub fn Poltergeist_baseST_Initial(e: &mut Engine, _: u32) -> LResult<u64> {
    let iVar1 = L(e);
    for o in [0x204, 0x18c, 0x188, 0x16c, 0x174] {
        e.w32(iVar1 + o, 0);
    }
    e.w32(iVar1 + 0x1f0, 1);
    if g(e, G_Poltergeist_baseST_Initial) == 1 {
        e.w32(iVar1 + 0x1f0, 0);
    }
    let uVar2 = APIgameobj::GetID(e, SELF, NULL);
    e.w32(iVar1 + 0x1e4, uVar2);
    let gp = GDP(e);
    e.w32(gp + 0x10, 1);
    APIgameobj::GetPosition(e, SELF, ALL, iVar1 + 0x178);
    let v = e.r32(L(e) + 0xb0);
    e.w32(iVar1 + 0x5c, v);
    APIgameobj::SetGroupID(e, SELF, 1);
    if e.r32(L(e) + 0x1a4) != 0 {
        let l = L(e);
        LIBApiPathFinding::SetPath(e, (l + 0x1a8) as i32, (l + 0x1e4) as i32);
    }
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 0x14, 0x15);
    e.w32(iVar1 + 0x160, addr::Poltergeist_baseST_Standing_In);
    Poltergeist_BaseFUNC_ActivateFBEffect(e)?;
    APIgameobj::SetNextState(e, SELF, 0xf);
    Ok(1)
}

/// 0x000138f8 external state 0xf: run the sub-state
pub fn Poltergeist_baseST_Entry_Point(e: &mut Engine, _: u32) -> LResult<u64> {
    call_state(e)?;
    Ok(0)
}

/// 0x00013958 (not for type 8, the riders) idle anim, context 0x60, nothing chosen / held
pub fn Poltergeist_baseST_Standing_In(e: &mut Engine) -> LResult<()> {
    let iVar1 = L(e);
    if e.r32(L(e) + 0x44) != 8 {
        let a = k(e, Poltergeist_baseST_Standing_In_0_1) as i32;
        LIBAnimMngr::PlayAnim(e, 0.1, a, 2, 0);
        APIgameobj::SetContextFlag(e, SELF, 0x60);
        e.w32(iVar1 + 0x184, 0x7fff_fffe);
        for o in [0x188, 0x18c, 0x170] {
            e.w32(iVar1 + o, 0);
        }
        e.w32(iVar1 + 0x160, addr::Poltergeist_baseST_Standing_Execute);
        call_state(e)?;
    }
    Ok(())
}

/// 0x00013a40 hero near (bit 4), engageable and a throwable free: claim it (0x111), go grab it
pub fn Poltergeist_baseST_Standing_Execute(e: &mut Engine) -> LResult<()> {
    let iVar1 = L(e);
    if ev(e) & 4 != 0
        && Poltergeist_BaseFUNC_TestAtcivationCondition(e)? != 0
        && Poltergeist_BaseFUNC_ChooseThrowable(e)? != 0
    {
        if g(e, G_Poltergeist_baseST_Standing_Execute) == 1 && e.r32(GDP(e) + 0x10) != 0 {
            let gp = GDP(e);
            e.w32(gp + 0x10, 0);
            APIgameobj::SendMessageTo(e, SELF, NULL, 0xb3, 0, NULL);
        }
        let to = e.r32(iVar1 + 0x184);
        APIgameobj::SendMessageTo(e, to, NULL, 0x111, 0, NULL);
        clr(e, 0xffff_fffb);
        e.w32(iVar1 + 0x160, addr::Poltergeist_baseST_GoGrabThrowable_In);
    }
    Ok(())
}

/// 0x00013b28 jump onto the throwable (L+0x184): 15 m/s, 2.5 m high
pub fn Poltergeist_baseST_JumpOnThrowable_In(e: &mut Engine) -> LResult<()> {
    let iVar1 = L(e);
    let a = if g(e, G_Poltergeist_baseST_JumpOnThrowable_In) == 1 {
        -1
    } else {
        k(e, Poltergeist_baseST_JumpOnThrowable_In_0_1) as i32
    };
    let iVar2 = LIBAnimMngr::PlayAnim(e, 0.0, a, 2, 0);
    e.w32(iVar1 + 0x38, iVar2 as u32);
    Poltergeist_BaseFUNC_SetTrackingSpeed(e, 0x3f80_0000)?;
    APIgameobj::SetContextFlag(e, SELF, 0x60);
    APIphysic::ResetFlags(e, SELF, 1);
    let obj = e.r32(iVar1 + 0x184);
    APIgameobj::GetPosition(e, obj, ALL, iVar1 + 0x194);
    let b = e.alloca(0x28);
    for (o, s) in [(0, 0x194), (4, 0x198), (8, 0x19c)] {
        let v = e.r32(iVar1 + s);
        e.w32(b + o, v);
    }
    let gdm = e.r32(iVar1 + 0x24);
    for (o, v) in [(0xc, 0), (0x10, 0), (0x14, 0), (0x18, 0x4170_0000), (0x1c, 2), (0x20, 0x4020_0000), (0x24, gdm)] {
        e.w32(b + o, v);
    }
    APIphysic::JumpOnCoor(e, SELF, b);
    e.w32(iVar1 + 0x160, addr::Poltergeist_baseST_JumpOnThrowable_Execute);
    call_state(e)
}

/// 0x00013ca8 landed (within 3.5 m): pick it up
pub fn Poltergeist_baseST_JumpOnThrowable_Execute(e: &mut Engine) -> LResult<()> {
    let iVar1 = L(e);
    let obj = e.r32(L(e) + 0x184);
    let bVar2 = LIBApiCFunctions::CheckRangeObject(e, 3.5, SELF, obj, 0);
    if bVar2 != 0 {
        e.w32(iVar1 + 0x160, addr::Poltergeist_baseST_PickUpThrowable_In);
    }
    Ok(())
}

/// 0x00013d20 head for the throwable (directly, or along the path); no path to it but a jump
/// allowed (L+400): jump on it
pub fn Poltergeist_baseST_GoGrabThrowable_In(e: &mut Engine) -> LResult<()> {
    let iVar1 = L(e);
    let mut iStack_40 = -1;
    let DStack_50 = e.alloca(0xc);
    let obj = e.r32(L(e) + 0x184);
    APIgameobj::GetPosition(e, obj, ALL, DStack_50);
    let obj = e.r32(iVar1 + 0x184);
    let lVar2 = Poltergeist_BaseFUNC_Path_NeedPathFinding(e, obj)?;
    if lVar2 == 0 {
        let obj = e.r32(iVar1 + 0x184);
        Poltergeist_BaseFUNC_TrackObj(e, obj)?;
    } else {
        iStack_40 = Poltergeist_BaseFUNC_Path_SetPathFinding(e, DStack_50)?;
    }
    if iStack_40 == 0 && e.r32(iVar1 + 400) != 0 {
        e.w32(iVar1 + 0x160, addr::Poltergeist_baseST_JumpOnThrowable_In);
    } else {
        Poltergeist_BaseFUNC_ChooseWalkType(e)?;
        if e.r32(iVar1 + 0x44) != 8 {
            APIgameobj::SetContextFlag(e, SELF, 0x66);
        }
        e.w32(iVar1 + 0x160, addr::Poltergeist_baseST_GoGrabThrowable_Execute);
    }
    call_state(e)
}

/// 0x00013e88 reached it (bit 0x20): pick it up (type 8: tell myself 0x113, the level file mounts
/// it); follow the path, at its end track the object
pub fn Poltergeist_baseST_GoGrabThrowable_Execute(e: &mut Engine) -> LResult<()> {
    let iVar1 = L(e);
    if ev(e) & 0x20 != 0 {
        clr(e, 0xffff_ffdf);
        if e.r32(iVar1 + 0x44) == 8 {
            APIgameobj::SendMessageTo(e, SELF, NULL, 0x113, 0, NULL);
        } else {
            e.w32(iVar1 + 0x160, addr::Poltergeist_baseST_PickUpThrowable_In);
        }
    }
    let iVar2 = Poltergeist_BaseFUNC_Path_UpdatePathFinding(e)?;
    if iVar2 == 2 {
        let DStack_40 = e.alloca(0xc);
        let obj = e.r32(iVar1 + 0x184);
        APIgameobj::GetPosition(e, obj, ALL, DStack_40);
        Poltergeist_BaseFUNC_TrackPos(e, DStack_40)?;
    }
    Ok(())
}

/// 0x00013fb8 pick-up anim (L+0x168)
pub fn Poltergeist_baseST_PickUpThrowable_In(e: &mut Engine) -> LResult<()> {
    let iVar1 = L(e);
    let a = k(e, Poltergeist_baseST_PickUpThrowable_In_0_1) as i32;
    let iVar2 = LIBAnimMngr::PlayAnim(e, 0.1, a, 1, 0x14);
    e.w32(iVar1 + 0x168, iVar2 as u32);
    clr(e, 0xffff_ffbf);
    clr(e, 0xffff_ff7f);
    APIgameobj::SetContextFlag(e, SELF, 0x60);
    e.w32(iVar1 + 0x160, addr::Poltergeist_baseST_PickUpThrowable_Execute);
    call_state(e)
}

/// 0x000140b0 anim event: holding it (0x113/0x1f, 0x115 with my id); anim done: chase the hero
pub fn Poltergeist_baseST_PickUpThrowable_Execute(e: &mut Engine) -> LResult<()> {
    let iVar2 = L(e);
    if ev(e) & 0x40 == 0 || e.r32(L(e) + 0x168) != k(e, Poltergeist_baseST_PickUpThrowable_Execute_0_1) {
        if ev(e) & 0x80 != 0 {
            clr(e, 0xffff_ff7f);
            e.w32(iVar2 + 0x160, addr::Poltergeist_baseST_TrackHero_In);
        }
    } else {
        clr(e, 0xffff_ffbf);
        let l = L(e);
        e.w32(l + 0x188, 1);
        let obj = e.r32(L(e) + 0x184);
        APIgameobj::SendMessageTo(e, obj, NULL, 0x113, 0x1f, NULL);
        let iVar2 = L(e);
        let uVar3 = APIgameobj::GetID(e, SELF, NULL);
        let obj = e.r32(iVar2 + 0x184);
        APIgameobj::SendMessageTo(e, obj, NULL, 0x115, uVar3, NULL);
    }
    Ok(())
}

/// 0x00014228 move towards the leader hero, walk anim, context 0x66
pub fn Poltergeist_baseST_TrackHero_In(e: &mut Engine) -> LResult<()> {
    let iVar1 = L(e);
    let hero = e.r32(e.r32(GDM(e)) + 8);
    Poltergeist_BaseFUNC_TrackObj(e, hero)?;
    let a = k(e, Poltergeist_baseST_TrackHero_In_0_1) as i32;
    LIBAnimMngr::PlayAnim(e, 0.1, a, 2, 0);
    APIgameobj::SetContextFlag(e, SELF, 0x66);
    e.w32(iVar1 + 0x160, addr::Poltergeist_baseST_TrackHero_Execute);
    call_state(e)
}

/// 0x000142f0 in throwing range (bit 8): attack
pub fn Poltergeist_baseST_TrackHero_Execute(e: &mut Engine) -> LResult<()> {
    let iVar1 = L(e);
    if ev(e) & 8 != 0 {
        clr(e, 0xffff_fff7);
        e.w32(iVar1 + 0x160, addr::Poltergeist_baseST_Attack_In);
    }
    Ok(())
}

/// 0x00014368 wind-up anim (L+0x168), context 0x62
pub fn Poltergeist_baseST_Attack_In(e: &mut Engine) -> LResult<()> {
    let iVar1 = L(e);
    let a = k(e, Poltergeist_baseST_Attack_In_0_1) as i32;
    let iVar2 = LIBAnimMngr::PlayAnim(e, 0.1, a, 1, 0x14);
    e.w32(iVar1 + 0x168, iVar2 as u32);
    clr(e, 0xffff_ffbf);
    clr(e, 0xffff_ff7f);
    APIgameobj::SetContextFlag(e, SELF, 0x62);
    e.w32(iVar1 + 0x160, addr::Poltergeist_baseST_Attack_Execute);
    call_state(e)
}

/// 0x00014460 wind-up event: throw (0x110 to the object, nothing held); wind-up done: follow-through
/// anim; follow-through done: AfterAttack
pub fn Poltergeist_baseST_Attack_Execute(e: &mut Engine) -> LResult<()> {
    let iVar1 = L(e);
    let cur = |e: &Engine| e.r32(L(e) + 0x168);
    if ev(e) & 0x40 == 0 {
        if ev(e) & 0x80 != 0 {
            if cur(e) == k(e, Poltergeist_baseST_Attack_Execute_0_1) {
                let a = k(e, Poltergeist_baseST_Attack_Execute_0_2) as i32;
                let iVar2 = LIBAnimMngr::PlayAnim(e, 0.1, a, 1, 0x14);
                e.w32(iVar1 + 0x168, iVar2 as u32);
                clr(e, 0xffff_ff7f);
            } else if cur(e) == k(e, Poltergeist_baseST_Attack_Execute_0_3) {
                let l = L(e);
                e.w32(l + 0x168, 0);
                e.w32(iVar1 + 0x160, addr::Poltergeist_baseST_AfterAttack_In);
                clr(e, 0xffff_ff7f);
            }
        }
    } else if cur(e) == k(e, Poltergeist_baseST_Attack_Execute_0_4) {
        clr(e, 0xffff_ffbf);
        let obj = e.r32(iVar1 + 0x184);
        APIgameobj::SendMessageTo(e, obj, NULL, 0x110, 0, NULL);
        e.w32(iVar1 + 0x188, 0);
        e.w32(iVar1 + 0x184, 0);
    }
    Ok(())
}

/// 0x00014608 rest timer L+0x208: 2.5 s, callback 0x13
pub fn Poltergeist_baseST_AfterAttack_In(e: &mut Engine) -> LResult<()> {
    let iVar1 = L(e);
    let t = e.r32(L(e) + 0x208);
    APIextra::TimerReset(e, t);
    APIextra::TimerSetFrequency(e, t, 2.5);
    APIextra::TimerRegisterNotify(e, SELF, t, 0x13);
    e.w32(iVar1 + 0x160, addr::Poltergeist_baseST_AfterAttack_Execute);
    call_state(e)
}

/// 0x000146d8 taunt (bit 0x200, msg 199): taunt anim; rest over (bit 0x100): Standing
pub fn Poltergeist_baseST_AfterAttack_Execute(e: &mut Engine) -> LResult<()> {
    let iVar1 = L(e);
    if ev(e) & 0x200 == 0 {
        if ev(e) & 0x100 != 0 {
            clr(e, 0xffff_feff);
            e.w32(iVar1 + 0x160, addr::Poltergeist_baseST_Standing_In);
        }
    } else {
        clr(e, 0xffff_fdff);
        let a = k(e, Poltergeist_baseST_AfterAttack_Execute_0_1) as i32;
        LIBAnimMngr::PlayAnim(e, 0.1, a, 1, 0);
    }
    Ok(())
}

/// 0x000147c8 hooked: scared anim (L+0x168), drop the object, context 0x66
pub fn Poltergeist_baseST_Scared_In(e: &mut Engine) -> LResult<()> {
    let iVar1 = L(e);
    clr(e, 0xffff_ff7f);
    let a = k(e, Poltergeist_baseST_Scared_In_0_1) as i32;
    let iVar2 = LIBAnimMngr::PlayAnim(e, 0.1, a, 1, 0x14);
    e.w32(iVar1 + 0x168, iVar2 as u32);
    Poltergeist_BaseFUNC_DropThrowable(e)?;
    APIgameobj::SetContextFlag(e, SELF, 0x66);
    e.w32(iVar1 + 0x160, addr::Poltergeist_baseST_Scared_Execute);
    call_state(e)
}

/// 0x000148a8 scared anim done: face the book, 3.5 m/s, struggle anim (L+0x16c = 1). Then: a free
/// throwable and the hero engageable -> go grab it; else (once) move DM: target the book, mode 0x11
/// (keep away from it).
pub fn Poltergeist_baseST_Scared_Execute(e: &mut Engine) -> LResult<()> {
    let iVar1 = L(e);
    if ev(e) & 0x80 != 0 && e.r32(L(e) + 0x168) == k(e, Poltergeist_baseST_Scared_Execute_0_1) {
        clr(e, 0xffff_ff7f);
        Poltergeist_BaseFUNC_SetTrackingSpeed(e, 0x4060_0000)?;
        let book = k(e, ID_2a00004d);
        LIBApiCFunctions::TakeRotation(e, std::f32::consts::PI, SELF, book as i32);
        let a = k(e, Poltergeist_baseST_Scared_Execute_0_2) as i32;
        LIBAnimMngr::PlayAnim(e, 0.3, a, 2, 0);
        e.w32(iVar1 + 0x16c, 1);
    }
    if e.r32(iVar1 + 0x16c) != 0 {
        let mut iVar2 = Poltergeist_BaseFUNC_ChooseThrowable(e)?;
        let mut free = false;
        if iVar2 != 0 {
            iVar2 = Poltergeist_BaseFUNC_TestAtcivationCondition(e)?;
            free = iVar2 != 0;
        }
        if !free {
            if e.r32(iVar1 + 0x170) == 0 {
                let b = e.alloca(0x80);
                let dm = e.r32(iVar1 + 4);
                APIphysic::DMGetParams(e, dm, b);
                let book = k(e, ID_2a00004d);
                for (o, v) in [(0, book), (0x14, 0x11), (8, 0), (0xc, 0), (0x10, 0)] {
                    e.w32(b + o, v);
                }
                APIphysic::DMSetParams(e, dm, b);
                e.w32(iVar1 + 0x170, 1);
            }
        } else {
            let obj = e.r32(iVar1 + 0x184);
            APIgameobj::SendMessageTo(e, obj, NULL, 0x111, 0, NULL);
            clr(e, 0xffff_fffb);
            e.w32(iVar1 + 0x160, addr::Poltergeist_baseST_GoGrabThrowable_In);
        }
    }
    Ok(())
}

/// 0x00014ad8 callback 0x15 (pre-logic notification 0x14): hooked (bit 0x400) -> Scared; hero off
/// the path / hiding -> drop and stand; range checks per sub-state (bits 4, 8, 0x20, 0x10); FB
/// effect adjust (bit 2)
pub fn Poltergeist_baseCB_PreLogic(e: &mut Engine, _: u32) -> LResult<u64> {
    let iVar2 = L(e);
    if ev(e) & 0x400 == 0 {
        if e.r32(L(e) + 0x1a4) != 0 {
            let hero = e.r32(e.r32(GDM(e)) + 8);
            let lVar6 = Poltergeist_BaseFUNC_Path_IsOnPath(e, hero)?;
            if (lVar6 == 0 || e.r32(e.r32(GDP(e)) + 0x14) & 0x0900_090c != 0)
                && state(e) != addr::Poltergeist_baseST_Standing_Execute
                && state(e) != addr::Poltergeist_baseST_Standing_In
                && e.r32(iVar2 + 0x160) != 0
            {
                Poltergeist_BaseFUNC_DropThrowable(e)?;
                e.w32(iVar2 + 0x160, addr::Poltergeist_baseST_Standing_In);
            }
        }
        let s = e.r32(iVar2 + 0x160);
        if s == addr::Poltergeist_baseST_Standing_Execute {
            let hero = e.r32(e.r32(GDM(e)) + 8);
            if LIBApiCFunctions::CheckRangeObject(e, 10.0, SELF, hero, 0) != 0 {
                let a = L(e) + 0x164;
                or32(e, a, 4);
            }
        } else if s == addr::Poltergeist_baseST_TrackHero_Execute && e.rf(iVar2 + 0x18c) != 0.0 {
            let r = e.rf(iVar2 + 0x18c);
            let hero = e.r32(e.r32(GDM(e)) + 8);
            if LIBApiCFunctions::CheckRangeObject(e, r, SELF, hero, 0) != 0 {
                let a = L(e) + 0x164;
                or32(e, a, 8);
            }
        } else if s == addr::Poltergeist_baseST_GoGrabThrowable_Execute {
            let obj = e.r32(iVar2 + 0x184);
            let bVar4 = LIBApiCFunctions::CheckRangeObject(e, 1.5, SELF, obj, 0);
            let iVar3 = L(e);
            if bVar4 != 0 {
                if e.r32(iVar2 + 0x44) == 7 {
                    let uVar5 = APIgameobj::GetID(e, SELF, NULL);
                    let obj = e.r32(iVar3 + 0x184);
                    APIgameobj::SendMessageTo(e, obj, NULL, 0x115, uVar5, NULL);
                    e.w32(iVar2 + 0x160, addr::Poltergeist_baseST_Attack_In);
                } else {
                    let a = L(e) + 0x164;
                    or32(e, a, 0x20);
                }
            }
        } else {
            let hero = e.r32(e.r32(GDM(e)) + 8);
            if LIBApiCFunctions::CheckRangeObject(e, 12.0, SELF, hero, 0) == 0 {
                let a = L(e) + 0x164;
                or32(e, a, 0x10);
            }
        }
        if ev(e) & 2 != 0 {
            Poltergeist_BaseFUNC_AdjustFBEffect(e)?;
            clr(e, 0xffff_fffd);
        }
    } else {
        clr(e, 0xffff_fbff);
        e.w32(iVar2 + 0x160, addr::Poltergeist_baseST_Scared_In);
    }
    Ok(0)
}

/// 0x00014ee0 callback 0x13: the rest timer fired (bit 0x100)
pub fn Poltergeist_baseCB_Timer(e: &mut Engine, param_1: u32) -> LResult<u64> {
    if param_1 == e.r32(L(e) + 0x208) {
        let t = e.r32(L(e) + 0x208);
        APIextra::TimerUnregisterNotify(e, SELF, t);
        let a = L(e) + 0x164;
        or32(e, a, 0x100);
    }
    Ok(0)
}

/// 0x00014f70 callback 0x11 (clipping change notification 9): L+0x204 = new clip state
/// (`param_1` points at the notification data)
pub fn Poltergeist_baseCB_ClipChange(e: &mut Engine, param_1: u32) -> LResult<u64> {
    let v = e.r32(param_1 + 4);
    let l = L(e);
    e.w32(l + 0x204, v);
    Ok(0)
}

/// 0x00014fc8 callback 0x12 (anim event notification 0xb): bit 0x40
pub fn Poltergeist_baseCB_Anim(e: &mut Engine, _: u32) -> LResult<u64> {
    let a = L(e) + 0x164;
    or32(e, a, 0x40);
    Ok(0)
}

/// 0x00015018 callback 0x14 (anim done notification 1): bit 0x80
pub fn Poltergeist_baseCB_AnimDone(e: &mut Engine, _: u32) -> LResult<u64> {
    let a = L(e) + 0x164;
    or32(e, a, 0x80);
    Ok(0)
}

/// 0x00015068 clipping distances 10/20/50/100 m (LOD sector masks) and the clip-change notification
/// (9 -> callback 0x11)
pub fn Poltergeist_BaseFUNC_ActivateFBEffect(e: &mut Engine) -> LResult<()> {
    let b = e.alloca(0x40);
    APIgameobj::GetClippingParams(e, SELF, b);
    for (o, v) in [
        (4, 0x4120_0000), (0xc, 0x1000_0007), (8, 0x1000_0007), (0x10, 0x41a0_0000), (0x18, 0x1000_0007),
        (0x14, 0x1000_0027), (0x1c, 0x4248_0000), (0x24, 0x1000_0027), (0x20, 0x1000_0027),
        (0x28, 0x42c8_0000), (0x30, 0x1000_01e7), (0x2c, 0x1000_06e7), (0x34, 0xbf80_0000),
        (0x3c, 0x1000_06e7), (0x38, 0x1000_06e7),
    ] {
        e.w32(b + o, v);
    }
    APIgameobj::SetClippingParams(e, SELF, b, 0);
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 9, 0x11);
    Ok(())
}

/// 0x000151a8 the FB effect (L+0x200) is on within 10 m of object ID_2a000037 (50 m right after a
/// clip change, L+0x204)
pub fn Poltergeist_BaseFUNC_AdjustFBEffect(e: &mut Engine) -> LResult<()> {
    let iVar2 = L(e);
    let a = e.alloca(0xc);
    let b = e.alloca(0xc);
    let o = k(e, ID_2a000037);
    APIgameobj::GetPosition(e, o, ALL, a);
    let uVar3 = APIgameobj::GetID(e, SELF, NULL);
    APIgameobj::GetPosition(e, uVar3, ALL, b);
    let fStack_50 = e.rf(a) - e.rf(b);
    let fStack_4c = e.rf(a + 4) - e.rf(b + 4);
    let fStack_48 = e.rf(a + 8) - e.rf(b + 8);
    let mut fStack_40 = APImath::sqrtf(e, fStack_50 * fStack_50 + fStack_4c * fStack_4c + fStack_48 * fStack_48);
    if fStack_40 < 0.0 {
        fStack_40 = fStack_40 * -1.0;
    }
    let fb = e.r32(iVar2 + 0x200);
    if e.r32(iVar2 + 0x204) == 0 {
        APIfbeffects::EnableFB(e, fb, (fStack_40 < 10.0) as u32);
    } else if 50.0 < fStack_40 {
        APIfbeffects::EnableFB(e, fb, 0);
    } else {
        APIfbeffects::EnableFB(e, fb, 1);
    }
    e.w32(iVar2 + 0x204, 0);
    Ok(())
}

/// 0x000153d8 pick the nearest throwable (GetID 0xff000080, type 0x19 for type-7 poltergeists, else
/// 0x14, within 150 m) into L+0x184, on the path when this poltergeist uses one. Returns 1 when found.
pub fn Poltergeist_BaseFUNC_ChooseThrowable(e: &mut Engine) -> LResult<u32> {
    let iStack_40 = L(e);
    let p = e.alloca(0x20);
    let t = if e.r32(L(e) + 0x44) == 7 { 0x19 } else { 0x14 };
    e.w32(p + 4, t);
    e.w32(p, 1);
    let id = APIgameobj::GetID(e, SELF, NULL);
    e.w32(p + 8, id);
    for (o, v) in [(0xc, 0), (0x10, 0), (0x14, 0), (0x18, 0x4316_0000), (0x1c, 0x3c23_d70a), (0, 1)] {
        e.w32(p + o, v);
    }
    let uVar1 = APIgameobj::GetID(e, 0xff00_0080, p);
    e.w32(iStack_40 + 0x184, uVar1);
    loop {
        if e.r32(p) == 0x7fff_fffe {
            return Ok(0);
        }
        if e.r32(L(e) + 0x1a4) != 0 {
            let obj = e.r32(L(e) + 0x184);
            if Poltergeist_BaseFUNC_Path_IsOnPath(e, obj)? != 0 {
                break;
            }
        }
        if e.r32(L(e) + 0x1a4) == 0 {
            return Ok(1);
        }
        let uVar1 = APIgameobj::GetID(e, 0xff00_0080, p);
        e.w32(iStack_40 + 0x184, uVar1);
    }
    Ok(1)
}

/// 0x00015598 release the object: held -> 0x114/0x1f (drop), claimed -> 0x112/0x1f (unclaim); stop
/// the rest timer
pub fn Poltergeist_BaseFUNC_DropThrowable(e: &mut Engine) -> LResult<()> {
    let iVar1 = L(e);
    let obj = e.r32(L(e) + 0x184);
    if e.r32(L(e) + 0x188) == 0 {
        if obj != 0 && obj != 0x7fff_fffe {
            APIgameobj::SendMessageTo(e, obj, NULL, 0x112, 0x1f, NULL);
        }
    } else {
        APIgameobj::SendMessageTo(e, obj, NULL, 0x114, 0x1f, NULL);
    }
    for o in [0x188, 0x184, 0x18c] {
        e.w32(iVar1 + o, 0);
    }
    let t = e.r32(L(e) + 0x208);
    APIextra::TimerUnregisterNotify(e, SELF, t);
    Ok(())
}

/// 0x000156a0 3 m/s; types 7/8 glide (anim -> L+0x38), else walk, or (scared, L+0x16c) 3.5 m/s run
pub fn Poltergeist_BaseFUNC_ChooseWalkType(e: &mut Engine) -> LResult<()> {
    let iVar1 = L(e);
    Poltergeist_BaseFUNC_SetTrackingSpeed(e, 0x4040_0000)?;
    let t = e.r32(iVar1 + 0x44);
    if t == 7 || t == 8 {
        let a = if g(e, G_Poltergeist_BaseFUNC_ChooseWalkType) == 1 {
            -1
        } else {
            k(e, Poltergeist_BaseFUNC_ChooseWalkType_0_1) as i32
        };
        let iVar2 = LIBAnimMngr::PlayAnim(e, 0.0, a, 2, 0);
        e.w32(iVar1 + 0x38, iVar2 as u32);
    } else if e.r32(iVar1 + 0x16c) == 0 {
        let a = k(e, Poltergeist_BaseFUNC_ChooseWalkType_0_2) as i32;
        LIBAnimMngr::PlayAnim(e, 0.1, a, 2, 0);
    } else if e.r32(iVar1 + 0x16c) != 0 {
        Poltergeist_BaseFUNC_SetTrackingSpeed(e, 0x4060_0000)?;
        let a = k(e, Poltergeist_BaseFUNC_ChooseWalkType_0_3) as i32;
        LIBAnimMngr::PlayAnim(e, 0.1, a, 2, 0);
    }
    Ok(())
}

/// 0x000157f0 0x116 throw range (L+0x18c = *param_3); 0xc9 restart: show and wake; 199 taunt (bit
/// 0x200); 0xd6 from the book: 100 link broken (back to Standing if scared, L+0x16c = 0), 0x65
/// hooked (sound, stop the rest timer, bit 0x400), 0x67 pulled in (drop, sound, stop, no pre-logic),
/// 0x68 captured (stop the rest timer); 0x117 tracking speed; 0x201 / 0x202 enable / disable. Then
/// Monster_Base_MessageFunction.
pub fn Poltergeist_BaseFUNC_MessageFunction(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    let iVar1 = L(e);
    let sounds = g(e, G_Poltergeist_BaseFUNC_MessageFunction) == 0;
    if param_1 == 0x116 {
        let v = e.r32(param_3);
        let l = L(e);
        e.w32(l + 0x18c, v);
    } else if param_1 < 0x117 {
        if param_1 == 0xc9 {
            if APIgameobj::IsGameObjectVisible(e, SELF) == 0 {
                APIgameobj::SetGameObjectVisible(e, SELF, 1);
                APIgameobj::Sleep(e, SELF, 0.0);
            }
        } else if param_1 < 0xca {
            if param_1 == 199 {
                let a = L(e) + 0x164;
                or32(e, a, 0x200);
            }
        } else if param_1 == 0xd6 {
            match param_2 {
                100 => {
                    if state(e) == addr::Poltergeist_baseST_Scared_Execute
                        || state(e) == addr::Poltergeist_baseST_Scared_In
                    {
                        set_state(e, addr::Poltergeist_baseST_Standing_In);
                    }
                    e.w32(iVar1 + 0x16c, 0);
                }
                0x65 => {
                    if sounds {
                        let (o, s) = (k(e, ID_2a00004b), k(e, ID_8000002a));
                        APInewSound::Play(e, o, s, 0);
                    }
                    let t = e.r32(iVar1 + 0x208);
                    APIextra::TimerUnregisterNotify(e, SELF, t);
                    let a = L(e) + 0x164;
                    or32(e, a, 0x400);
                }
                0x67 => {
                    Poltergeist_BaseFUNC_DropThrowable(e)?;
                    if sounds {
                        let (o, s) = (k(e, ID_2a00004b), k(e, ID_80000034));
                        APInewSound::Play(e, o, s, 0);
                    }
                    APIphysic::SetVelocity(e, SELF, 0.0, 0.0, 0.0);
                    let t = e.r32(iVar1 + 0x208);
                    APIextra::TimerUnregisterNotify(e, SELF, t);
                    APIgameobj::UnregisterNotify(e, SELF, 0xffff_ffff, 0x14);
                }
                0x68 => {
                    let t = e.r32(L(e) + 0x208);
                    APIextra::TimerUnregisterNotify(e, SELF, t);
                }
                _ => {}
            }
        }
    } else if param_1 == 0x201 {
        APIgameobj::EnableObject(e, SELF, 1);
    } else if param_1 < 0x202 {
        if param_1 == 0x117 {
            let v = e.r32(param_3);
            Poltergeist_BaseFUNC_SetTrackingSpeed(e, v)?;
        }
    } else if param_1 == 0x202 {
        APIgameobj::EnableObject(e, SELF, 0);
    }
    Monster_Base_MessageFunction(e, param_1, param_2, param_3)
}

/// 0x00015be0 physics: prim 800, weight 50; DMs: L+4 move (type 1, 3 m/s, mode 5, context 2), L+8
/// turn (type 4, context 4), L+0x20 type 8 (75 / 45, context 0x20), L+0x24 gravity (type 0, -20,
/// context 0x40), L+0 type 7 (0.7, 7, context 1)
pub fn Poltergeist_BaseFUNC_CreatePhysic(e: &mut Engine) -> LResult<()> {
    let puStack_30 = L(e);
    APIphysic::CreatePhysic(e, SELF, 0);
    APIphysic::SetFlags(e, SELF, 6);
    APIphysic::SetPrimaryPrim(e, SELF, 800);
    APIphysic::SetWeight(e, SELF, 50.0);
    let add = |e: &mut Engine, ty: u32, slot: u32| -> u32 {
        let dm = APIphysic::DMAdd(e, SELF, ty);
        e.w32(puStack_30 + slot, dm);
        dm
    };
    let dm = add(e, 1, 4);
    let b = e.alloca(0x80);
    APIphysic::DMGetParams(e, dm, b);
    for (o, v) in [(4, 0x4040_0000), (0, 0), (8, 0), (0xc, 0), (0x10, 0), (0x14, 5)] {
        e.w32(b + o, v);
    }
    APIphysic::DMSetParams(e, dm, b);
    APIphysic::DMSetContextMask(e, dm, 2);
    let dm = add(e, 4, 8);
    let b = e.alloca(0x80);
    APIphysic::DMGetParams(e, dm, b);
    for (o, v) in [(0, 0), (4, 0), (8, 0), (0xc, 1)] {
        e.w32(b + o, v);
    }
    APIphysic::DMSetParams(e, dm, b);
    APIphysic::DMSetContextMask(e, dm, 4);
    let dm = add(e, 8, 0x20);
    // no DMGetParams: only the first two words are set (the rest of the stack block is whatever
    // was there; the port's alloca block is zeroed)
    let b = e.alloca(0x80);
    e.w32(b + 4, 0x4234_0000);
    e.w32(b, 0x4296_0000);
    APIphysic::DMSetParams(e, dm, b);
    APIphysic::DMSetContextMask(e, dm, 0x20);
    let dm = add(e, 0, 0x24);
    let b = e.alloca(0x80);
    APIphysic::DMGetParams(e, dm, b);
    for (o, v) in [(0, 0), (4, 0xc1a0_0000), (8, 0)] {
        e.w32(b + o, v);
    }
    APIphysic::DMSetParams(e, dm, b);
    APIphysic::DMSetContextMask(e, dm, 0x40);
    let dm = add(e, 7, 0);
    let b = e.alloca(0x80);
    APIphysic::DMGetParams(e, dm, b);
    e.w32(b, 0x3f33_3333);
    e.w32(b + 4, 7);
    APIphysic::DMSetParams(e, dm, b);
    APIphysic::DMSetContextMask(e, dm, 1);
    APIphysic::PrimSetIntAttribBits(e, SELF, 800, 0, 0x88);
    APIphysic::PrimSetIntAttribBits(e, SELF, 800, 7, 2);
    APIphysic::PrimSetIntAttribBits(e, SELF, 800, 8, 0x21);
    APIphysic::SetPriority(e, SELF, 10);
    Ok(())
}

/// 0x00016000 externals 0x10/0xf (states), 0x15/0x11/0x14/0x12/0x13 (callbacks); API tables, global
/// data (game buffers 0 and 2); physics, context 0x60, clipping, notifications, the FB outline effect
/// (L+0x200); match our id in goCustomDataPoltergeist (k records); kill if captured before; rest
/// timer L+0x208; groups; sector clipping registration (L+0x20c); initial state 0x10.
pub fn poltergeist_baseConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    monster_baseConstructor(e, api, instanceCount)?;
    if instanceCount == 1 {
        let fns: [(i32, crate::logic::ExtFn); 7] = [
            (0x10, Poltergeist_baseST_Initial),
            (0xf, Poltergeist_baseST_Entry_Point),
            (0x15, Poltergeist_baseCB_PreLogic),
            (0x11, Poltergeist_baseCB_ClipChange),
            (0x14, Poltergeist_baseCB_AnimDone),
            (0x12, Poltergeist_baseCB_Anim),
            (0x13, Poltergeist_baseCB_Timer),
        ];
        for (i, f) in fns {
            APIcons::SetExternalFunction(e, i, f);
        }
        let tables: [(u32, fn(&mut Engine) -> Ptr); 7] = [
            (pAnim, |e| APIcons::GetApiAnim(e)),
            (pGameObj, |e| APIcons::GetApiGameObj(e)),
            (pExtra, |e| APIcons::GetApiExtra(e)),
            (pPhysic, |e| APIcons::GetApiPhysic(e)),
            // GetApiFBEffects is passed the string "FBEffects"
            (pFBEffects, |e| APIcons::GetApiFBEffects(e)),
            (pMath, |e| APIcons::GetApiMath(e)),
            (pPathFinding, |e| {
                let n = e.cstr("ApiPathFinding");
                APIcons::GetApiCustom(e, n)
            }),
        ];
        for (slot, get) in tables {
            let v = get(e);
            let a = G(e, slot);
            e.w32(a, v);
        }
        let v = APIcons::GetApiNewSound(e);
        let a = G(e, pSound);
        e.w32(a, v);
        let n = e.cstr("ApiSectorClipping");
        let v = APIcons::GetApiCustom(e, n);
        let a = G(e, pSectorClip);
        e.w32(a, v);
        let slot = G(e, pGlobalDataPoltergeist_Base);
        APIcons::GetGlobalData(e, slot, 0x14);
        let uVar1 = APIgameobj::GetGameBuffer(e, 0, 0x8f8, 1);
        let gp = GDP(e);
        e.w32(gp, uVar1);
        let uVar1 = APIgameobj::GetGameBuffer(e, 2, 0x158, 1);
        let gp = GDP(e);
        e.w32(gp + 4, uVar1);
    }
    let slot = G(e, pLocalDataPoltergeistII_Base);
    APIcons::GetLocalData(e, slot, 0x218);
    Poltergeist_BaseFUNC_CreatePhysic(e)?;
    APIgameobj::SetContextFlag(e, SELF, 0x60);
    LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 2, NULL, 1);
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 0xb, 0x12);
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 1, 0x14);
    let uVar1 = APIfbeffects::CreateFB(e, 2, 8);
    let l = L(e);
    e.w32(l + 0x200, uVar1);
    let b = e.alloca(0x10);
    let fb = e.r32(L(e) + 0x200);
    APIfbeffects::GetParamsFB(e, fb, 2, b);
    let id = APIgameobj::GetID(e, SELF, NULL);
    for (o, v) in [(4, id), (0, 0x10), (8, 0), (0xc, 0)] {
        e.w32(b + o, v);
    }
    APIfbeffects::SetParamsFB(e, fb, 2, b);
    APIfbeffects::EnableFB(e, fb, 1);
    let iStack_3c = APIgameobj::GetID(e, SELF, NULL);
    // records 0..=k (the loop exits when `k < uStack_40`)
    let last = k(e, poltergeist_baseConstructor_0_2);
    let mut uStack_40 = 0u32;
    while uStack_40 <= last {
        let r = G(e, goCustomDataPoltergeist) + uStack_40 * 0x4c;
        if e.r32(r) == iStack_3c {
            let l = L(e);
            for (d, s) in [(0x40, 8), (0x1a0, 4), (0x1a4, 0xc)] {
                let v = e.r32(r + s);
                e.w32(l + d, v);
            }
            LIBMiscFunc::MiscFunc_StrncpySafe(e, l + 0x1a8, r + 0x10, 0x3c);
            break;
        }
        uStack_40 += 1;
    }
    if AmIDead(e)? == 1 {
        APIgameobj::KillObject(e, SELF);
    }
    let uVar1 = APIextra::TimerCreate(e);
    let l = L(e);
    e.w32(l + 0x208, uVar1);
    let m = if e.r32(L(e) + 0x44) == 8 { 0x20_0409 } else { 0x20_0609 };
    APIgameobj::SetGroupBitMask(e, SELF, m);
    let uVar1 = APIgameobj::GetID(e, SELF, NULL);
    let l = L(e);
    e.w32(l + 0x20c, uVar1);
    e.w16(l + 0x216, 2);
    e.w16(l + 0x214, 0);
    LIBApiSectorClipping::ApiImpl_Register(e, l + 0x20c);
    APIcons::SetInitialState(e, 0x10);
    Ok(())
}

/// 0x00016680
pub fn poltergeist_baseDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    monster_baseDestructor(e)?;
    let fb = e.r32(L(e) + 0x200);
    APIfbeffects::DestroyFB(e, fb);
    let p = L(e) + 0x20c;
    LIBApiSectorClipping::ApiImpl_Unregister(e, p);
    let t = e.r32(L(e) + 0x208);
    APIextra::TimerDestroy(e, t);
    Ok(())
}

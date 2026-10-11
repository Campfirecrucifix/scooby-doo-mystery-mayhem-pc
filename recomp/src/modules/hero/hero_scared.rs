//! hero_scared.c (18 functions). Addresses are module-relative (.text at 0x10000) in scooby__402420b4.
use super::builds::*;
use super::*;
use super::skeleton::addr;
use crate::engine::*;
use crate::logic::LResult;

const SELF: u32 = 0x2a00_0000;

/// 0x00028420 (1320 bytes) `void heroInit_Scare(int param_1)`
pub fn heroInit_Scare(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    // local+0x250 = the hit flags (MSG 0x78 / CB_SCARED): 1 ethereal, 2 physical, 4 dust devil (grabbed).
    let gb = GB0(e);
    if e.r32(gb + 0x14) & 1 != 0 {
        let v = e.r32(gb + 0x14);
        e.w32(gb + 0x14, v & 0xffff_fffe);
        APIgameobj::SendMessageTo(e, k(e, heroInit_Scare_0_1), 0, 0xda, 0, 0);
    }
    let fl = e.r32(param_1 + 0x250);
    let set_gb_bit = |e: &mut Engine| {
        let a = GB0(e) + 0x14;
        let v = e.r32(a);
        e.w32(a, v | 0x200_0000);
    };
    if fl & 1 != 0 {
        debug_time(e, "heroInit_Scare ARG1_ETHEREAL", 0, 0)?;
        e.w32(param_1 + 0x220, 0x46);
        let a = heroFUNC_StartAnim(e, f32::from_bits(0x3dcccccd), k(e, heroInit_Scare_0_5) as i32, 1, 5)?;
        e.w32(param_1 + 0x218, a as i32 as u32);
        APIgameobj::SetContextFlag(e, SELF, 0x250);
        set_gb_bit(e);
    } else if fl & 2 != 0 {
        debug_time(e, "heroInit_Scare ARG1_PHYSICAL", 0, 0)?;
        e.w32(param_1 + 0x220, 0x47);
        let a = heroFUNC_StartAnim(e, f32::from_bits(0x3dcccccd), k(e, heroInit_Scare_0_4) as i32, 1, 5)?;
        e.w32(param_1 + 0x218, a as i32 as u32);
        APIgameobj::SetContextFlag(e, SELF, 0x50);
        set_gb_bit(e);
    } else if fl & 4 != 0 {
        debug_time(e, "heroInit_Scare ARG1_DUSTDEVIL", 0, 0)?;
        APIphysic::SetFlags(e, SELF, 1);
        APIphysic::SetVelocity(e, SELF, 0.0, 0.0, 0.0);
        e.w32(param_1 + 0x220, 0x48);
        let a = heroFUNC_StartAnim(e, f32::from_bits(0x3e4ccccd), k(e, heroInit_Scare_0_2) as i32, 1, 5)?;
        e.w32(param_1 + 0x218, a as i32 as u32);
        set_gb_bit(e);
        let v = e.r32(param_1 + 0x24c);
        e.w32(param_1 + 0x238, v);
        let bone = if e.r32(param_1 + 0x17c) == k(e, heroInit_Scare_0_3) {
            debug_info(e, "ST_SCARED plugg shaggy");
            0x1b
        } else {
            debug_info(e, "ST_SCARED plugg scooby");
            0x1a
        };
        APIgameobj::AddChild(e, e.r32(param_1 + 0x238), bone, SELF, 1);
        APIgameobj::SetContextFlag(e, SELF, 0);
    }
    // InputConfigSetState only runs for the leader (short-circuit), then the panel bug-proof check
    if e.r32(GB0(e) + 8) == e.r32(L(e) + 0x17c) && {
        APIinput::InputConfigSetState(e, e.r32(param_1 + 0x188) as i32, i32::MIN);
        e.r32(GB2(e)) & 0x2_0000 != 0
    } {
        let a = GB0(e) + 0x14;
        let v = e.r32(a);
        e.w32(a, v & 0xffff_f7ff);
        APIgameobj::SendMessageTo(e, 0x2a10_0006, 0, 0x32, e.r32(param_1 + 0x214), 0);
        e.w32(param_1 + 0x214, 0);
        debug_info(e, "-- MSG_PANEL_NOTIFICATION_OUT ----- BUG-PROFF PROTECTION!!!");
    }
    APIphysic::SetVelocity(e, SELF, 0.0, 0.0, 0.0);
    e.w32(param_1 + 0x158, addr::heroST_Scare_Loop);
    e.w32(param_1 + 0x15c, addr::heroExit_Scare);
    Ok(())
}

/// 0x00028948 (1296 bytes) `void heroST_Scare_Loop(int param_1)`
pub fn heroST_Scare_Loop(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    let gb = GB0(e); // iVar1, read at entry
    let leader = |e: &Engine| e.r32(GB0(e) + 8) == e.r32(L(e) + 0x17c);
    let meter = |e: &Engine| e.rf(e.r32(GD(e) + 4));
    if e.r32(param_1 + 0x168) & 1 == 0 {
        if e.r32(param_1 + 0x220) == 0x48 {
            let t = APIextra::TimerGetTime(e, 0x9000_0000);
            APIextra::TimerReset(e, e.r32(param_1 + 500));
            if e.r32(param_1 + 0x164) & 0x4_0000 != 0 {
                let v = e.r32(param_1 + 0x164);
                e.w32(param_1 + 0x164, v & 0xfffb_ffff);
                APIphysic::ResetFlags(e, SELF, 1);
                APIgameobj::LeaveParent(e, SELF, 0);
                e.w32(param_1 + 0x238, 0);
                APIgameobj::SetContextFlag(e, SELF, 0x50);
            }
            if leader(e) && e.rf(param_1 + 0x378) < t {
                let t = APIextra::TimerGetTime(e, 0x9000_0000);
                e.wf(param_1 + 0x378, t + 0.25);
                heroFUNC_CallVibration(e, f32::from_bits(0x3f28f5c3), 0.0, 0.25, 0.25)?;
            }
        }
        return Ok(());
    }
    let m = e.r32(GD(e) + 4);
    debug_time(e, "heroST_Scare_Loop event_CALLBACK_LASTFRAME", m, 2)?;
    APIphysic::ResetFlags(e, SELF, 1);
    APIgameobj::LeaveParent(e, SELF, 0);
    if e.r32(param_1 + 0x220) == 0x48 && leader(e) {
        heroFUNC_CallVibration(e, 1.0, 0.0, f32::from_bits(0x3ea8f5c3), 0.0)?;
    }
    let next = if leader(e) {
        APIphysic::DMSetActive(e, e.r32(param_1 + 0x1c0), 0);
        let a = GB0(e) + 0x14;
        let v = e.r32(a);
        e.w32(a, v & 0xffff_0fff);
        if (e.r32(gb + 0x34) & 0x101 == 0 && e.r32(GB0(e) + 4) != 0xd) || 0.0 < meter(e) {
            if meter(e) <= 0.0 {
                let a = GB0(e) + 0x14;
                let v = e.r32(a);
                e.w32(a, v | 0x400_0000);
                e.w32(param_1 + 0x154, addr::heroInit_PanicIn);
                APIgameobj::SendMessageTo(e, e.r32(param_1 + 0x180), 0, 0x8a, 0x200_0000, 0);
                return Ok(());
            }
            addr::heroInit_Stand
        } else {
            let a = GB0(e) + 0x14;
            let v = e.r32(a);
            e.w32(a, v | 0x800_0000);
            addr::heroInit_Dead
        }
    } else {
        APIphysic::PrimSetIntAttribBits(e, SELF, e.r32(param_1 + 0x18c), 2, 4);
        if (e.r32(gb + 0x34) & 0x101 == 0 || 0.0 < meter(e)) && meter(e) <= 0.0 {
            addr::heroInit_PanicIn
        } else {
            addr::heroInit_FollowStand
        }
    };
    e.w32(param_1 + 0x154, next);
    Ok(())
}

/// 0x00028e58 (176 bytes) `void heroExit_Scare(void)`
pub fn heroExit_Scare(e: &mut Engine) -> LResult<()> {
    if e.r32(GB0(e) + 8) == e.r32(L(e) + 0x17c) {
        debug_info(e, "Exit_Scare");
        let a = GB0(e) + 0x14;
        let v = e.r32(a);
        e.w32(a, v & 0xfdff_ffff);
    }
    Ok(())
}

/// 0x00028f08 (532 bytes) `void heroInit_PanicIn(int param_1)`
pub fn heroInit_PanicIn(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    let gb = GB0(e); // iStack_40
    debug_time(e, "heroInit_PanicIn", 0, 0)?;
    UpdateTimerOutOfTime(e, 1.0)?;
    APIphysic::PrimResetIntAttribBits(e, SELF, e.r32(param_1 + 0x18c), 2, 4);
    let v = e.r32(gb + 0x14);
    e.w32(gb + 0x14, v | 0x400_0000);
    let a = heroFUNC_StartAnim(e, 0.0, k(e, heroInit_PanicIn_0_1) as i32, 2, 0)?;
    e.w32(param_1 + 0x218, a as i32 as u32);
    if e.r32(GB0(e) + 8) == e.r32(L(e) + 0x17c) {
        let orp = e.alloca(0x20); // auStack_60[0] = 1, rest uninitialised
        e.w32(orp, 1);
        APIgameobj::SendMessageTo(e, 0xff00_0001, orp, 200, 0, 0);
        let cam = if g(e, G_heroInit_PanicIn) == 1 { 0x2a00_0081 } else { 0x2a00_0096 };
        APIgameobj::SendMessageTo(e, cam, 0, 0x5c, 3, 0);
    }
    heroFUNC_SetTrackID(e, 0x42c8_0000, e.r32(param_1 + 0x180) as i32, 0)?;
    APIgameobj::SetContextFlag(e, SELF, 0x2000_0850);
    APIinput::InputConfigSetState(e, e.r32(param_1 + 0x188) as i32, i32::MIN);
    APIphysic::SetVelocity(e, SELF, 0.0, 0.0, 0.0);
    e.w32(param_1 + 0x158, addr::heroST_PanicIn_Loop);
    e.w32(param_1 + 0x15c, addr::heroExit_Panic);
    Ok(())
}

/// 0x00029120 (476 bytes) `void heroST_PanicIn_Loop(int param_1)`
pub fn heroST_PanicIn_Loop(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    // Group 2 stores heroInit_PanicJump as a raw address (0x1fe00 in that build): same state.
    let near = LIBApiCFunctions::CheckRangeObject(e, 1.0, SELF, e.r32(param_1 + 0x180), 1) != 0;
    if !near {
        if e.r32(GB0(e) + 8) != e.r32(L(e) + 0x17c) {
            return Ok(());
        }
        if IsOutOfTime(e)? == 0 {
            return Ok(());
        }
        let d = if matches!(g(e, G_heroST_PanicIn_Loop), 3 | 4) { 0.35 } else { 0.3 };
        let partner = e.r32(L(e) + 0x180) as i32;
        if heroFUNC_IsAnythingBetweenUs(e, d, SELF, partner, 0, 0)? != 0 {
            let a = GB0(e) + 0x34;
            let v = e.r32(a);
            e.w32(a, v | 0x100);
            e.w32(param_1 + 0x154, addr::heroInit_Dead);
            return Ok(());
        }
    }
    if e.r32(GB0(e) + 8) != e.r32(L(e) + 0x17c) {
        debug_time(e, "heroST_PanicIn_Loop velocity reset", 0, 0)?;
        APIphysic::SetVelocity(e, SELF, 0.0, 0.0, 0.0);
    }
    e.w32(param_1 + 0x154, addr::heroInit_PanicJump);
    Ok(())
}

/// 0x00029300 (484 bytes) `void heroInit_PanicJump(int param_1)`
pub fn heroInit_PanicJump(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    if e.r32(GB0(e) + 8) == e.r32(L(e) + 0x17c) {
        let a = heroFUNC_StartAnim(e, 0.0, k(e, heroInit_PanicJump_0_1) as i32, 1, 5)?;
        e.w32(param_1 + 0x218, a as i32 as u32);
        APIphysic::ResetFlags(e, e.r32(L(e) + 0x180), 4);
        APIgameobj::AddChild(e, SELF, e.r32(param_1 + 0x184), e.r32(param_1 + 0x180), 9);
        APIgameobj::SendMessageTo(e, e.r32(param_1 + 0x180), 0, 0xd7, 1, 0);
        APIgameobj::SetContextFlag(e, SELF, 0x50);
    } else {
        let a = heroFUNC_StartAnim(e, 0.0, k(e, heroInit_PanicJump_0_2) as i32, 1, 5)?;
        e.w32(param_1 + 0x218, a as i32 as u32);
        debug_time(e, "heroInit_PanicJump velocity reset", 0, 0)?;
        APIphysic::ResetFlags(e, SELF, 4);
        APIgameobj::SetContextFlag(e, SELF, 0);
    }
    APIphysic::SetVelocity(e, SELF, 0.0, 0.0, 0.0);
    e.w32(param_1 + 0x158, addr::heroST_PanicJump_Loop);
    e.w32(param_1 + 0x15c, addr::heroExit_Panic);
    Ok(())
}

/// 0x000294e8 (124 bytes) `void heroST_PanicJump_Loop(int param_1)`
pub fn heroST_PanicJump_Loop(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    if e.r32(param_1 + 0x168) & 1 != 0 {
        let next = if e.r32(param_1 + 0x250) & 0x100 == 0 { addr::heroInit_Panic } else { addr::heroInit_Dead };
        e.w32(param_1 + 0x154, next);
    }
    Ok(())
}

/// 0x00029568 (472 bytes) `void heroInit_Panic(int param_1)`
pub fn heroInit_Panic(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    UpdateTimerOutOfTime(e, 5.0)?;
    if e.r32(GB0(e) + 8) == e.r32(L(e) + 0x17c) {
        e.w32(param_1 + 0x37c, 0);
        APIextra::TimerReset(e, e.r32(param_1 + 0x208));
        let r = LIBApiCFunctions::IntRandom(e, 1, 100);
        // spin direction: +/- 0.262 rad per step (0x3e8624dd)
        e.w32(param_1 + 0x380, if r < 0x33 { 0x3e86_24dd } else { 0xbe86_24dd });
        e.w32(param_1 + 0x2e0, 0x4020_0000);
        let a = heroFUNC_StartAnim(e, f32::from_bits(0x3dcccccd), k(e, heroInit_Panic_0_1) as i32, 2, 0)?;
        e.w32(param_1 + 0x218, a as i32 as u32);
        APIgameobj::SetContextFlag(e, SELF, 0x451);
        APIinput::InputConfigSetState(e, e.r32(param_1 + 0x188) as i32, -0x7fff_fffe);
    } else {
        let a = heroFUNC_StartAnim(e, f32::from_bits(0x3dcccccd), k(e, heroInit_Panic_0_2) as i32, 2, 0)?;
        e.w32(param_1 + 0x218, a as i32 as u32);
        APIgameobj::SetContextFlag(e, SELF, 0);
    }
    e.w32(param_1 + 0x158, addr::heroST_Panic_Loop);
    e.w32(param_1 + 0x15c, addr::heroExit_Panic);
    Ok(())
}

/// 0x00029740 (2072 bytes) `void heroST_Panic_Loop(int param_1)`
pub fn heroST_Panic_Loop(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    // Leader only: the follower rides on the leader's bone (+0x184); the leader spins at random and
    // escapes (PanicOut) when the cool meter is back or control was removed and 3 probes are free.
    if e.r32(GB0(e) + 8) != e.r32(L(e) + 0x17c) {
        return Ok(());
    }
    let shaggy_tune = matches!(g(e, G_heroST_Panic_Loop), 2 | 3);
    let st = e.alloca(0x60);
    let (d90, d80, d70, t60, d50, u40) = (st, st + 0x10, st + 0x20, st + 0x30, st + 0x40, st + 0x50);
    APIgameobj::GetPosition(e, e.r32(param_1 + 0x180), 0x7fff_ffff, d90);
    APIgameobj::GetPosition(e, SELF, e.r32(param_1 + 0x184), d80);
    let (a, b) = (e.rv3(d80), e.rv3(d90));
    let d = [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    for i in 0..3 {
        e.wf(d70 + i as u32 * 4, d[i]);
    }
    if 0.15 < d[0] * d[0] + d[1] * d[1] + d[2] * d[2] {
        let t = APIanim::GetTime(e, SELF, k(e, heroST_Panic_Loop_0_1));
        // stored as an int (the receiver, MSG 0x8a, converts it back with (float)(int))
        e.w32(t60, t as i32 as u32);
        debug_info(e, "----");
        debug_info(e, "Too Farr");
        for (slot, v) in [(heroST_Panic_Loop_0_2, d80), (heroST_Panic_Loop_0_3, d90), (heroST_Panic_Loop_0_4, d70)] {
            // the labels are .rodata strings of the module image (no literal in the decompile)
            if dprint(e) {
                let n = char_name(e);
                let m = G(e, k(e, slot));
                LIBdebug::PrintDebugInfoWithValue(e, n, m, v, 4);
            }
        }
        debug_info(e, "----");
        APIgameobj::LeaveParent(e, e.r32(param_1 + 0x180), 0);
        APIgameobj::AddChild(e, SELF, e.r32(param_1 + 0x184), e.r32(param_1 + 0x180), 9);
        APIgameobj::SendMessageTo(e, e.r32(param_1 + 0x180), 0, 0x8a, 0x400_0000, t60);
    } else {
        for i in 0..3 {
            e.w32(d50 + i * 4, 0);
        }
        APIgameobj::SetTranslation(e, e.r32(param_1 + 0x180), d50);
    }
    let out = IsOutOfTime(e)? != 0;
    if out && e.r32(param_1 + 0x164) & 0x1_0000 == 0 {
        let v = e.r32(param_1 + 0x164);
        e.w32(param_1 + 0x164, v | 0x1_0000);
        debug_info(e, "Panic_Loop - removing control");
        APIgameobj::SetContextFlag(e, SELF, 0x450);
        APIinput::InputConfigSetState(e, e.r32(param_1 + 0x188) as i32, i32::MIN);
    }
    let meter = |e: &Engine| e.rf(e.r32(GD(e) + 4));
    if 0.0 < meter(e) {
        let a = GB0(e) + 0x14;
        let v = e.r32(a);
        e.w32(a, v | 0x100_0000);
    }
    if 0.0 < meter(e) || e.r32(param_1 + 0x164) & 0x1_0000 != 0 {
        let (z50, z70, z40) = if shaggy_tune { (0.65, 0.0, 0xbf4c_cccd) } else { (0.7, 0.25, 0xbe4c_cccd) };
        e.wf(d50, -1.5);
        e.wf(d50 + 4, 0.0);
        e.wf(d50 + 8, z50);
        e.w32(d70, 0xbfc6_6666);
        e.wf(d70 + 4, 0.0);
        e.wf(d70 + 8, z70);
        e.w32(u40, 0xbfc0_0000);
        e.w32(u40 + 4, 0);
        e.w32(u40 + 8, z40);
        if heroFUNC_ValidateTarget(e, d50)? != 0
            && heroFUNC_ValidateTarget(e, d70)? != 0
            && heroFUNC_ValidateTarget(e, u40)? != 0
        {
            let a = GB0(e) + 0x14;
            let v = e.r32(a);
            e.w32(a, v | 0x100_0000);
            APIgameobj::SendMessageTo(e, e.r32(param_1 + 0x180), 0, 0x60, 0x40, 0);
            APIgameobj::SetContextFlag(e, SELF, 0x50);
            APIinput::InputConfigSetState(e, e.r32(param_1 + 0x188) as i32, i32::MIN);
            e.w32(param_1 + 0x154, addr::heroInit_PanicOut);
            return Ok(());
        }
        let t = APIextra::TimerGetTime(e, e.r32(param_1 + 0x208));
        if 10.0 < t {
            APIgameobj::SendMessageTo(e, SELF, 0, 0x68, 0x10_0000, 0);
            return Ok(());
        }
    }
    if e.rf(param_1 + 0x37c) <= 0.0 {
        if out {
            let f = LIBApiCFunctions::FloatRandom(e, 0.262, 0.785, 0.01);
            e.wf(param_1 + 0x37c, f);
            UpdateTimerOutOfTime(e, 0.33)?;
        } else if e.rf(param_1 + 0x2c8) == 0.0 && e.rf(param_1 + 0x2cc) == 0.0 && e.rf(param_1 + 0x2e0) < 2.5 {
            let f = LIBApiCFunctions::FloatRandom(e, 0.349, 3.692, 0.01);
            e.wf(param_1 + 0x37c, f);
        }
    }
    if 0.0 < e.rf(param_1 + 0x37c) {
        let v = e.rf(param_1 + 0x37c);
        e.wf(param_1 + 0x37c, v - 0.262);
        e.wf(d70, 0.0);
        e.wf(d70 + 4, 1.0);
        e.wf(d70 + 8, 0.0);
        let ang = e.rf(param_1 + 0x380);
        APIgameobj::Rotate(e, SELF, d70, ang);
    }
    Ok(())
}

/// 0x00029f60 (440 bytes) `void heroInit_PanicOut(int param_1)`
pub fn heroInit_PanicOut(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    debug_time(e, "heroInit_PanicOut", 0, 0)?;
    if e.r32(GB0(e) + 8) == e.r32(L(e) + 0x17c) {
        let a = GB0(e) + 0x14;
        let v = e.r32(a);
        e.w32(a, v | 0x100_0000);
        let cam = if g(e, G_heroInit_PanicOut) == 1 { 0x2a00_0081 } else { 0x2a00_0096 };
        APIgameobj::SendMessageTo(e, cam, 0, 0x5c, 1, 0);
        let r = heroFUNC_StartAnim(e, 0.0, k(e, heroInit_PanicOut_0_1) as i32, 1, 5)?;
        e.w32(param_1 + 0x218, r as i32 as u32);
        APIgameobj::SetContextFlag(e, SELF, 0x50);
        APIinput::InputConfigSetState(e, e.r32(param_1 + 0x188) as i32, i32::MIN);
    } else {
        let r = heroFUNC_StartAnim(e, 0.0, k(e, heroInit_PanicOut_0_2) as i32, 1, 5)?;
        e.w32(param_1 + 0x218, r as i32 as u32);
        APIgameobj::SetContextFlag(e, SELF, 0);
        APIphysic::SetFlags(e, SELF, 4);
    }
    e.w32(param_1 + 0x158, addr::heroST_PanicOut_Loop);
    e.w32(param_1 + 0x15c, addr::heroExit_Panic);
    Ok(())
}

/// 0x0002a118 (176 bytes) `void heroST_PanicOut_Loop(int param_1)`
pub fn heroST_PanicOut_Loop(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    if e.r32(param_1 + 0x168) & 1 != 0 {
        if e.r32(GB0(e) + 8) == e.r32(L(e) + 0x17c) {
            let a = GB0(e) + 0x14;
            let v = e.r32(a);
            e.w32(a, v & 0xfaff_ffff);
            e.w32(param_1 + 0x154, addr::heroInit_Stand);
        } else {
            e.w32(param_1 + 0x154, addr::heroInit_FollowStand);
        }
    }
    Ok(())
}

/// 0x0002a1c8 (772 bytes) `void heroExit_Panic(int param_1)`
pub fn heroExit_Panic(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    // Group 2 compares with heroInit_PanicJump as a raw address (0x1fe00 in that build): same state.
    debug_info(e, "Exit_Panic");
    let leader = e.r32(GB0(e) + 8) == e.r32(L(e) + 0x17c);
    if leader && e.r32(param_1 + 0x158) == addr::heroST_PanicOut_Loop {
        let a = GB0(e) + 0x14;
        let v = e.r32(a);
        e.w32(a, v & 0xfeff_ffff);
    }
    let still_panic = |e: &Engine| {
        let n = e.r32(param_1 + 0x154);
        [addr::heroInit_PanicIn, addr::heroInit_PanicJump, addr::heroInit_Panic, addr::heroInit_PanicOut, addr::heroInit_Dead]
            .contains(&n)
    };
    // the leader test is re-read between the two blocks (no intervening writes to it)
    if e.r32(GB0(e) + 8) == e.r32(L(e) + 0x17c) && !still_panic(e) {
        APIgameobj::SendMessageTo(e, k(e, ID_2a000096), 0, 0x5c, 1, 0);
    }
    if e.r32(GB0(e) + 8) != e.r32(L(e) + 0x17c) && !still_panic(e) {
        debug_info(e, "Exit_Panic - Deplug");
        APIphysic::ResetFlags(e, SELF, 1);
        APIphysic::SetFlags(e, SELF, 4);
        APIgameobj::LeaveParent(e, SELF, 0);
        APIphysic::PrimSetIntAttribBits(e, SELF, e.r32(param_1 + 0x18c), 2, 4);
        APIphysic::PrimResetIntAttribBits(e, SELF, e.r32(param_1 + 0x18c), 0, 0x80);
    }
    Ok(())
}

/// 0x0002a4d0 (900 bytes) `void heroInit_Dead(int param_1)`
pub fn heroInit_Dead(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    let set14 = |e: &mut Engine, and: u32, or: u32| {
        let a = GB0(e) + 0x14;
        let v = e.r32(a);
        e.w32(a, (v & and) | or);
    };
    if g(e, G_heroInit_Dead) == 1 && e.r32(GB0(e) + 0x14) & 2 != 0 {
        set14(e, 0xffff_fffd, 0);
        APIgameobj::SendMessageTo(e, 0x2a00_010e, 0, 0xd0, 0, 0);
    }
    if e.r32(GB0(e) + 0x14) & 1 != 0 {
        set14(e, 0xffff_fffe, 0);
        APIgameobj::SendMessageTo(e, k(e, heroInit_Dead_0_1), 0, 0xd0, 0, 0);
    }
    APIgameobj::LeaveParent(e, SELF, 1);
    APIphysic::SetFlags(e, SELF, 1);
    if e.r32(GB0(e) + 8) == e.r32(L(e) + 0x17c) {
        let gb = GB0(e);
        let m34 = e.r32(gb + 0x34);
        let anim = if m34 & 0x10 == 0 || m34 & 0x100 != 0 || e.r32(gb + 4) == 0xd {
            k(e, heroInit_Dead_0_2)
        } else {
            k(e, heroInit_Dead_0_3)
        };
        let r = heroFUNC_StartAnim(e, 0.0, anim as i32, 1, 5)?;
        e.w32(param_1 + 0x218, r as i32 as u32);
        set14(e, 0xffff_ffff, 0x800_0000);
        APIgameobj::SetContextFlag(e, SELF, 0);
        APIinput::InputConfigSetState(e, e.r32(param_1 + 0x188) as i32, i32::MIN);
        APIgameobj::SendMessageTo(e, k(e, ID_2a000096), 0, 0x5c, 0, 0);
        APIgameobj::SendMessageTo(e, k(e, ID_2a100006), 0, 0x1e, 0, 0);
        let orp = e.alloca(0x20); // auStack_50[0] = 1, rest uninitialised
        e.w32(orp, 1);
        APIgameobj::SendMessageTo(e, 0xff00_0001, orp, 0xc9, 0, 0);
        APIgameobj::SendMessageTo(e, k(e, heroInit_Dead_0_4), 0, 0x54, 1, 0);
    } else {
        let r = heroFUNC_StartAnim(e, 0.0, k(e, heroInit_Dead_0_5) as i32, 1, 5)?;
        e.w32(param_1 + 0x218, r as i32 as u32);
        APIgameobj::SetContextFlag(e, SELF, 0);
    }
    APIgraphics::DoCrappyXboxSpewHackThingy(e, e.r32(param_1 + 0x17c), 0, 1);
    APIphysic::SetVelocity(e, SELF, 0.0, 0.0, 0.0);
    e.w32(param_1 + 0x158, addr::heroST_Dead_Loop);
    e.w32(param_1 + 0x15c, addr::heroExit_Panic);
    Ok(())
}

/// 0x0002a858 (1032 bytes) `void heroST_Dead_Loop(int param_1)`
pub fn heroST_Dead_Loop(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    let gb = GB0(e); // iVar1, captured on entry
    if e.r32(param_1 + 0x168) & 1 == 0 || e.r32(GB0(e) + 8) != e.r32(L(e) + 0x17c) {
        return Ok(());
    }
    let s1 = e.alloca(8); // auStack_48 (passed to MSG_ENTER_DOOR as param_3)
    e.w32(s1, 1);
    debug_info(e, "\n\n---------------------------------------------------------\n");
    debug_info(e, "Exit ST_DEATH -> respawn");
    APInewSound::StopSound(e, k(e, heroST_Dead_Loop_0_2), 0x39);
    let other = e.r32(L(e) + 0x180);
    APIgameobj::Sleep(e, other, 0.0);
    let other = e.r32(L(e) + 0x180);
    APIphysic::ResetFlags(e, other, 1);
    let other = e.r32(L(e) + 0x180);
    APIphysic::SetFlags(e, other, 4);
    let other = e.r32(L(e) + 0x180);
    APIgameobj::SetContextFlag(e, other, 0);
    APIphysic::SetVelocity(e, SELF, 0.0, 0.0, 0.0);
    let df = g(e, G_heroST_Dead_Loop) == 2;
    let s2 = e.alloca(0x14); // DStack_40 (group 2: int S2[5], S2[0] = team mode who, S2+1 = vector)
    if e.r32(gb + 0x7c0) != 0 {
        let m = e.r32(gb + 0x7c0);
        e.w32(gb + 0x7bc, m);
        if df {
            e.w32(s2, 0xd);
            heroFUNC_MessageHandler(e, 0x7b, 0xe2, s2)?;
        } else {
            heroFUNC_MessageHandler(e, 0x7b, e.r32(gb + 0x7c0), 0)?;
        }
    }
    // keeps only bit 0x8000000 (sic)
    let a = GB0(e) + 0x14;
    let v = e.r32(a);
    e.w32(a, v & 0x800_0000);
    if e.r32(GB0(e) + 4) == 0xb && e.r32(GB0(e)) == 2 {
        let vec = if df { s2 + 4 } else { s2 };
        for i in 0..3 {
            e.w32(vec + i * 4, 0);
        }
        APIgameobj::SetTranslation(e, SELF, vec);
        APIanim::Play(e, SELF, k(e, heroST_Dead_Loop_0_4), 1, 0.0);
        APIgameobj::SendMessageTo(e, k(e, heroST_Dead_Loop_0_5), 0, 0xd2, 0x6a, 0);
        APIgameobj::SendMessageTo(e, k(e, ID_2a000136), 0, 0xd2, 0x595, 0);
        APIgameobj::SendMessageTo(e, k(e, heroST_Dead_Loop_0_6), 0, 0x47, 2, 0);
    }
    APIgameobj::SendMessageTo(e, k(e, ID_2a100006), 0, 0xcb, 0, 0);
    APIgameobj::SendMessageTo(e, k(e, heroST_Dead_Loop_0_7), 0, 0x55, 8, 0);
    APIgameobj::SendMessageTo(e, e.r32(GB0(e) + 8), 0, 0x97, k(e, ID_2a00003c), s1);
    APIgameobj::SendMessageTo(e, k(e, ID_2a000096), 0, 0x5c, 1, 0);
    e.w32(param_1 + 0x154, addr::heroInit_Busy);
    Ok(())
}

/// 0x0002ac60 (1536 bytes) `void heroFUNC_UpdateCoolMeter(uint param_1)`
pub fn heroFUNC_UpdateCoolMeter(e: &mut Engine, param_1: u32) -> LResult<()> {
    // The cool meter is the float at *(pGlobalDataHero + 4); param_1 is a single "percentage lost" bit.
    let pf = e.r32(GD(e) + 4);
    let v = e.alloca(4);
    e.w32(v, param_1);
    debug_info_value(e, "FUNC_UpdateCoolMeter -> auiPercentageLost", v, 1);
    let lost: Option<f32> = match param_1 {
        1 => Some(0.05), 2 => Some(0.1), 4 => Some(0.15), 8 => Some(0.2), 0x10 => Some(0.25),
        0x20 => Some(0.3), 0x40 => Some(0.35), 0x80 => Some(0.4), 0x100 => Some(0.45),
        0x200 => Some(0.5), 0x400 => Some(0.55), 0x800 => Some(0.6), 0x1000 => Some(0.65),
        0x2000 => Some(0.7), 0x4000 => Some(0.75), 0x8000 => Some(0.8), 0x10000 => Some(0.85),
        0x20000 => Some(0.9), 0x40000 => Some(0.95), 0x80000 | 0x100000 => Some(1.0),
        _ => None,
    };
    let cur = e.rf(pf);
    let new = match lost {
        Some(d) => cur - d,
        None => {
            debug_info_value(e, "FUNC_UpdateCoolMeter -> invalide auiPercentageLost", v, 1);
            cur - 0.1
        }
    };
    e.wf(pf, if new < 0.0 { 0.0 } else { new });
    debug_info_value(e, "FUNC_UpdateCoolMeter CoolMeter at", pf, 2);
    Ok(())
}

/// 0x0002b260 (1836 bytes) `undefined8 heroCB_SCARED(int param_1)`
pub fn heroCB_SCARED(e: &mut Engine, p: u32) -> LResult<u64> {
    // param_1: ObjCollisionNotifyStruct {+8 hitter id, +0x10 damage (cool-meter bit), +0x14 flags}.
    // Per shape group: power-up object told to hide (0xd0) and the hit-invulnerability timer frequency.
    let param_1 = p;
    let l = L(e);
    let gb = GB0(e);
    let (powerup, freq) = match g(e, G_heroCB_SCARED) {
        0 => (0x2a00_0134u32, 2.0f32),
        1 => (0x2a00_011f, 2.0),
        2 | 4 => (0x2a00_00af, 3.5),
        3 => (0x2a00_0111, 2.0),
        _ => unreachable!(),
    };
    if e.r32(gb + 0x74) != 0 {
        return Ok(0);
    }
    if e.r32(gb + 0x14) & 0x4900_094c != 0 {
        debug_info(e, "CB_SCARED -> failed to HIT");
        return Ok(0);
    }
    if e.r32(gb + 0xc) == e.r32(L(e) + 0x17c) {
        // the follower forwards the hit to the leader
        if e.r32(gb + 0x14) & 0x1_0000 != 0 {
            APIgameobj::SendMessageTo(e, e.r32(gb + 8), 0, 0x68, 0, param_1);
        }
        return Ok(0);
    }
    if e.r32(param_1 + 0x14) & 7 == 0 || e.r32(param_1 + 0x10) == 0 {
        return Ok(0);
    }
    debug_time(e, "CB_SCARED succeded", 0, 0)?;
    if e.r32(param_1 + 0x14) & 8 != 0 && APIgameobj::GetID(e, e.r32(param_1 + 8), 0) != 0x7fff_fffe {
        APIgameobj::SendMessageTo(e, e.r32(param_1 + 8), 0, 199, 0, 0);
    }
    let f = e.r32(param_1 + 0x14);
    if f & 0x20 != 0 {
        heroFUNC_CallVibration(e, f32::from_bits(0x3ea8f5c3), 0.0, f32::from_bits(0x3ea8f5c3), 0.0)?;
    } else if f & 0x40 != 0 {
        heroFUNC_CallVibration(e, f32::from_bits(0x3f28f5c3), 0.0, f32::from_bits(0x3ea8f5c3), 0.0)?;
    } else if f & 0x80 != 0 {
        heroFUNC_CallVibration(e, 1.0, 0.0, f32::from_bits(0x3ea8f5c3), 0.0)?;
    }
    if e.r32(GB0(e) + 8) == k(e, heroCB_SCARED_0_1) && e.r32(GB0(e) + 0x14) & 2 != 0 {
        debug_info(e, "Scooby is using power up - hiding it");
        if e.r32(e.r32(GD(e) + 0x10) + 4) == 0x800 {
            let a = GB0(e) + 0x14;
            let v = e.r32(a);
            e.w32(a, v & 0xffff_fffd);
            APIgameobj::SendMessageTo(e, powerup, 0, 0xd0, 0, 0);
            APIgameobj::SendMessageTo(e, 0x2a10_0006, 0, 0x28, e.r32(l + 0x17c), 0);
        }
    }
    let a = GB0(e) + 0x14;
    let v = e.r32(a);
    e.w32(a, v | 0x2000_0000);
    if e.r32(param_1 + 0x10) == 0x10_0000 {
        let v = e.r32(param_1 + 0x14);
        e.w32(param_1 + 0x14, v | 0x100);
    }
    heroFUNC_UpdateCoolMeter(e, e.r32(param_1 + 0x10))?;
    let v = e.r32(param_1 + 8);
    e.w32(l + 0x24c, v);
    let v = e.r32(param_1 + 0x14);
    e.w32(l + 0x250, v);
    if e.r32(GB0(e) + 8) == e.r32(L(e) + 0x17c) {
        // NOTE: writes the buffer read at entry (iVar4), same as GB0 unless a callee swapped it.
        e.w32(gb + 0x74, 1);
        APIextra::TimerReset(e, e.r32(l + 500));
        APIextra::TimerSetFrequency(e, e.r32(l + 500), freq);
        APIextra::TimerRegisterNotify(e, 0x2a00_0000, e.r32(l + 500), 10);
    }
    APIgameobj::SendMessageTo(e, e.r32(l + 0x180), 0, 0x78, e.r32(param_1 + 0x14), param_1 + 8);
    let m34 = e.r32(GB0(e) + 0x34);
    if m34 & 0x10 == 0 || m34 & 0x100 != 0 {
        // both branches of the original (`*pGlobalDataHero[1] <= 0.0`) set the same bit
        let v = e.r32(l + 0x170);
        e.w32(l + 0x170, v | 0x20);
    } else if e.r32(GB0(e) + 0x14) & 0x400_0000 == 0 || e.r32(GB0(e) + 4) == 0xd {
        let v = e.r32(l + 0x170);
        e.w32(l + 0x170, v | 0x20);
    } else {
        let v = e.r32(l + 0x170);
        e.w32(l + 0x170, v | 0x10);
        APIgameobj::SendMessageTo(e, e.r32(l + 0x180), 0, 0x60, 0x10, 0);
    }
    if e.r32(l + 0x170) & 0x20 != 0
        && IsInTeamMode(e)? != 0
        && LIBApiCFunctions::CheckRangeObject(e, 4.0, 0x2a00_0000, e.r32(l + 0x180), 1) != 0
        && e.r32(GB0(e) + 4) != 0xd
    {
        APIgameobj::SendMessageTo(e, e.r32(l + 0x180), 0, 0x60, 0x20, 0);
    }
    Ok(0)
}

/// 0x0002b990 (668 bytes) `undefined8 heroCB_HITTED_PARTICLES(undefined4 *param_1)`
pub fn heroCB_HITTED_PARTICLES(e: &mut Engine, param_1: u32) -> LResult<u64> {
    debug_info(e, "hero_baseCB_HITTED_PARTICLES");
    let part = e.r32(param_1) as i32;
    let st = e.alloca(0x18 + 0xc); // auStack_80 (0x18 used) + uStack_40/3c/auStack_38
    let (ctype, udtype, detect) = (st + 0x18, st + 0x1c, st + 0x20);
    APIparticle::TypeGetInt(e, part, 0x10, ctype);
    APIparticle::TypeGetInt(e, part, 0x20, udtype);
    APIparticle::TypeGetInt(e, part, 0x40, detect);
    if e.r32(detect) & 2 != 0 {
        if e.r32(ctype) == 2 {
            if e.r32(udtype) == 0 {
                e.w32(udtype, 0x10);
            }
            // the stack struct's +8 (hitter) is left uninitialised by the original
            let v = e.r32(udtype);
            e.w32(st + 0x10, v);
            e.w32(st + 0x14, 1);
            heroCB_SCARED(e, st)?;
        } else {
            let name = char_name(e);
            let f = e.cstr("--------------------------------------------------------------
");
            APIextra::null_printf(e, f, 0);
            let f = e.cstr("%s> Erreur dans les flags de la particule
");
            APIextra::null_printf(e, f, name);
            debug_info(e, "Hey, Jimbo! Ta particule est mal configuree");
            debug_info_value(e, "PL_INT_COLLISIONTYPE", ctype, 1);
            debug_info_value(e, "PL_INT_COLLISIONUDTYPE", udtype, 1);
            debug_info_value(e, "PL_INT_COLLISIONDETECTTYPE", detect, 1);
        }
    }
    Ok(0)
}

/// 0x0002bc30 (524 bytes) `void heroFUNC_CreateHittedPhysic(int param_1)`
pub fn heroFUNC_CreateHittedPhysic(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    // three kind-4 DMs (context masks 0x200/0x400/0x800) with params {0, 0, y, 1}, y = -0.75 / 8 / 10
    let p = e.alloca(0x10);
    for (slot, y, mask) in [(0x19c, 0xbf40_0000u32, 0x200), (0x1a0, 0x4100_0000, 0x400), (0x1a4, 0x4120_0000, 0x800)] {
        let h = APIphysic::DMAdd(e, SELF, 4);
        e.w32(param_1 + slot, h);
        APIphysic::DMGetParams(e, e.r32(param_1 + slot), p);
        e.w32(p + 0xc, 1);
        e.w32(p, 0);
        e.w32(p + 4, 0);
        e.w32(p + 8, y);
        APIphysic::DMSetParams(e, e.r32(param_1 + slot), p);
        APIphysic::DMSetContextMask(e, e.r32(param_1 + slot), mask);
    }
    Ok(())
}

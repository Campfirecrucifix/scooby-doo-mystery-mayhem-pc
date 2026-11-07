//! hero_power_up.c: only in 4 builds (79185eb3 / 7e698dff / ... Ep4 Scooby, c33ca2c4 Ep3 L01 Scooby);
//! the reference build has no such file. Entered from heroFUNC_CheckActionEvents (event bits 0x8000 /
//! 0x10000 of local+0x170).
//!
//! The reference build has no address for these functions, so `addr_pu` gives them their address in
//! scooby__79185eb3 (c33ca2c4 for heroExit_PowerUpInOut) + 0x100000: outside the reference module's
//! code range, so they never compare equal to a reference function. They are only stored/compared/called
//! as code pointers inside the module, like every other state.
use super::*;
use crate::engine::*;
use crate::logic::LResult;

#[allow(non_upper_case_globals)]
pub mod addr_pu {
    pub const heroInit_PowerUpIn: u32 = 0x1166c0;
    pub const heroST_PowerUpIn_Loop: u32 = 0x116c40;
    pub const heroInit_PowerUpOut: u32 = 0x116cd0;
    pub const heroST_PowerUpOut_Loop: u32 = 0x116ef8;
    pub const heroExit_PowerUpInOut: u32 = 0x115c70;
}

const SELF: u32 = 0x2a00_0000;

/// c33ca2c4 (Ep3 L01) is the second shape: the power-up there is 0x200 (object 0x2a00010e, carried
/// on bone 0x13), the 79185eb3 group's is 0x400 GS_POWER_UP_SNORKEL (swap Scooby 0x2a00001b with the
/// snorkel object 0x2a0000c0).
fn ep3(e: &Engine) -> bool {
    builds::BUILDS[e.rt.cur_build()].sha8 == "c33ca2c4"
}
/// Per-build snorkel ids (hero_power_up.c): (Scooby, snorkel object, camera, msg 0x47 In/Out P2, sound).
/// 79185eb3 = EP4/L02B, df8bf53b = EP4/L02A, 7e698dff = EP4/L01.
struct Ids { hero: u32, obj: u32, cam: u32, cam_in: u32, cam_out: u32, snd: u32 }
fn ids(e: &Engine) -> Ids {
    match builds::BUILDS[e.rt.cur_build()].sha8 {
        "df8bf53b" => Ids { hero: 0x2a00_0044, obj: 0x2a00_00e6, cam: 0x2a00_000f, cam_in: 0xc, cam_out: 0xd, snd: 0x4000_0042 },
        "7e698dff" => Ids { hero: 0x2a00_0024, obj: 0x2a00_00be, cam: 0x2a00_000e, cam_in: 0xb, cam_out: 2, snd: 0x4000_005b },
        _ => Ids { hero: 0x2a00_001b, obj: 0x2a00_00c0, cam: 0x2a00_0012, cam_in: 0xc, cam_out: 0xd, snd: 0x4000_0045 },
    }
}
fn pu_id(e: &Engine) -> u32 {
    if ep3(e) { 0x200 } else { 0x400 }
}
fn gb14(e: &mut Engine, and: u32, or: u32) {
    let a = GB0(e) + 0x14;
    let x = e.r32(a);
    e.w32(a, (x & and) | or);
}
/// `pGlobalDataHero[4]`: the power-up selection buffer (+4 = selected power-up bit).
fn gb4(e: &Engine) -> Ptr {
    e.r32(GD(e) + 0x10)
}

/// `heroValidPowerUp` (768 bytes in every build that has it)
pub fn heroValidPowerUp(e: &mut Engine) -> LResult<u32> {
    let sel = gb4(e);
    let unlocked = GB2(e);
    if e.r32(sel + 4) & 0xffff_f1ff != 0 {
        debug_info_value(e, "ValidPowerUp -> invalid bits", sel + 4, 4);
    }
    debug_info(e, "ValidPowerUp GS_POWER_UP_SNORKEL");
    let id = pu_id(e);
    e.w32(sel + 4, id);
    if e.r32(unlocked) & id == 0 {
        debug_info_value(e, "ValidPowerUp -> power up still locked", sel + 4, 1);
        return Ok(0);
    }
    let obj = if ep3(e) { 0x2a00_010e } else { ids(e).obj };
    if APIgameobj::GetID(e, obj, 0) == 0x7fff_fffe {
        return Ok(0);
    }
    debug_info(e, "ValidPowerUp -> ok!");
    Ok(1)
}

/// `heroInit_PowerUpIn`
pub fn heroInit_PowerUpIn(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    debug_time(e, "heroInit_PowerUpIn", 0, 0)?;
    if e.r32(gb4(e) + 4) != pu_id(e) {
        debug_info(e, "heroInit_PowerUpIn -> failed");
        return Ok(());
    }
    if ep3(e) {
        APIgameobj::SendMessageTo(e, 0x2a10_0006, 0, 0x28, e.r32(param_1 + 0x17c), 0);
        let a = heroFUNC_StartAnim(e, 0.0, 0x3c, 1, 5)?;
        e.w32(param_1 + 0x218, a as i32 as u32);
        APIgameobj::LeaveParent(e, 0x2a00_010e, 1);
        APIgameobj::AddChild(e, SELF, 0x13, 0x2a00_010e, 1);
        APIgameobj::SendMessageTo(e, 0x2a00_010e, 0, 0xd1, 0, 0);
        gb14(e, u32::MAX, 0x100);
        APIinput::InputConfigSetState(e, e.r32(param_1 + 0x188) as i32, i32::MIN);
        APIgameobj::SetContextFlag(e, SELF, 0x50);
        e.w32(param_1 + 0x158, addr_pu::heroST_PowerUpIn_Loop);
        e.w32(param_1 + 0x15c, addr_pu::heroExit_PowerUpInOut);
        return Ok(());
    }
    let c0 = e.alloca(0x10); // DStack_c0 = 0
    debug_info(e, "heroInit_PowerUpIn -> GS_POWER_UP_SNORKEL");
    gb14(e, u32::MAX, 0x40);
    gb14(e, u32::MAX, 0x4000_0000);
    let d = ids(e);
    let p = APIgameobj::GetPosition(e, d.hero, 0x7fff_ffff, c0);
    APIgameobj::SetTranslation(e, d.obj, p);
    APIgameobj::TakeOrientation(e, d.obj, d.hero, 0x7fff_ffff);
    if e.r32(GB0(e) + 0x34) & 0x10 != 0 {
        let a = GB0(e) + 0x34;
        let x = e.r32(a);
        e.w32(a, x | 0x300);
        // ObjectRetrieverParams {1, 0x24, SELF, 0, 0, 0, 10.0, 0.1, 0x7ffffffe}: the nearest quay
        let orp = e.alloca(0x24);
        for (i, v) in [1u32, 0x24, SELF, 0, 0, 0, 0x4120_0000, 0x3dcc_cccd, 0x7fff_fffe].into_iter().enumerate() {
            e.w32(orp + i as u32 * 4, v);
        }
        let d70 = e.alloca(0x10);
        e.wf(d70, 1.0);
        e.wf(d70 + 4, 0.0);
        e.wf(d70 + 8, 0.0);
        let quai = APIgameobj::GetID(e, 0xff00_0002, orp);
        e.w32(orp + 0x20, quai);
        e.w32(param_1 + 0x26c, quai);
        let d80 = e.alloca(0x10);
        let d60 = e.alloca(0x10);
        APIgameobj::GetPosition(e, quai, 0x7fff_ffff, d80);
        APIgameobj::GetPosition(e, e.r32(param_1 + 0x180), 0x7fff_ffff, d60);
        let dx = e.rf(d60) - e.rf(d80);
        let dz = e.rf(d60 + 8) - e.rf(d80 + 8);
        if dx * dx + dz * dz <= 0.57 {
            debug_info(e, "heroInit_PowerUpIn < DISTANCE_FROM_QUAI");
            APIgameobj::TransformVector(e, e.r32(param_1 + 0x26c), d70, d70);
            for i in 0..3 {
                let v = e.rf(d80 + i * 4) + e.rf(d70 + i * 4);
                e.wf(param_1 + 0x2d0 + i * 4, v);
            }
            APIgameobj::SendMessageTo(e, e.r32(param_1 + 0x180), 0, 0x60, 0x8_0000, 0);
            APIgameobj::SendMessageTo(e, e.r32(param_1 + 0x180), 0, 0x72, 0, param_1 + 0x2d0);
        } else {
            debug_info(e, "heroInit_PowerUpIn > DISTANCE_FROM_QUAI");
            APIgameobj::SendMessageTo(e, e.r32(param_1 + 0x180), 0, 0x6b, 0, 0);
        }
    }
    heroResetAction(e, param_1)?;
    APIgameobj::SendMessageTo(e, d.obj, 0, 0xd2, 0x6a, 0);
    APIgameobj::SendMessageTo(e, d.hero, 0, 0xd2, 0x595, 0);
    APIgameobj::SendMessageTo(e, d.cam, 0, 0x47, d.cam_in, 0);
    APInewSound::Play(e, SELF, d.snd, 0);
    e.w32(param_1 + 0x158, addr_pu::heroST_PowerUpIn_Loop);
    Ok(())
}

/// Shared end of the In/Out loops: leader -> Stand; follower -> FollowStand, except in c33ca2c4
/// where a follower with GB0+0x14 bit 1 (tome out) goes to heroInit_TomeTrack.
fn pu_done(e: &mut Engine, param_1: Ptr) {
    let n = if e.r32(GB0(e) + 8) == e.r32(L(e) + 0x17c) {
        addr::heroInit_Stand
    } else if !ep3(e) || e.r32(GB0(e) + 0x14) & 1 == 0 {
        addr::heroInit_FollowStand
    } else {
        addr::heroInit_TomeTrack
    };
    e.w32(param_1 + 0x154, n);
}

/// `heroST_PowerUpIn_Loop`
pub fn heroST_PowerUpIn_Loop(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    if e.r32(param_1 + 0x168) & 1 != 0 {
        pu_done(e, param_1);
    }
    Ok(())
}

/// `heroExit_PowerUpInOut` (c33ca2c4 only; `void(void)`, the code-pointer arg is unused)
pub fn heroExit_PowerUpInOut(e: &mut Engine, _param_1: Ptr) -> LResult<()> {
    gb14(e, 0xffff_feff, 0);
    Ok(())
}

/// `heroInit_PowerUpOut`
pub fn heroInit_PowerUpOut(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    if e.r32(gb4(e) + 4) != pu_id(e) {
        debug_info(e, "heroInit_PowerUpOut -> failed");
        return Ok(());
    }
    if ep3(e) {
        let a = heroFUNC_StartAnim(e, f32::from_bits(0x3dcccccd), 0x3c, 1, 5)?;
        e.w32(param_1 + 0x218, a as i32 as u32);
        APIgameobj::SendMessageTo(e, 0x2a00_010e, 0, 0xd0, 0, 0);
        gb14(e, u32::MAX, 0x100);
        APIinput::InputConfigSetState(e, e.r32(param_1 + 0x188) as i32, i32::MIN);
        APIgameobj::SetContextFlag(e, SELF, 0x50);
        e.w32(param_1 + 0x158, addr_pu::heroST_PowerUpOut_Loop);
        e.w32(param_1 + 0x15c, addr_pu::heroExit_PowerUpInOut);
        return Ok(());
    }
    let v = e.alloca(0x10);
    debug_info(e, "heroInit_PowerUpOut -> GS_POWER_UP_SNORKEL");
    gb14(e, 0xffff_ffbf, 0);
    let d = ids(e);
    let p = APIgameobj::GetPosition(e, d.obj, 0x7fff_ffff, v);
    APIgameobj::SetTranslation(e, d.hero, p);
    APIgameobj::TakeOrientation(e, d.hero, d.obj, 0x7fff_ffff);
    APIgameobj::SendMessageTo(e, d.hero, 0, 0xd2, 0x6a, 0);
    APIgameobj::SendMessageTo(e, d.obj, 0, 0xd2, 0x595, 0);
    APIgameobj::SendMessageTo(e, d.cam, 0, 0x47, d.cam_out, 0);
    e.w32(param_1 + 0x158, addr_pu::heroST_PowerUpOut_Loop);
    Ok(())
}

/// `heroST_PowerUpOut_Loop`
pub fn heroST_PowerUpOut_Loop(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    if e.r32(param_1 + 0x168) & 1 != 0 {
        debug_time(e, "heroST_PowerUpIn_Loop event_CALLBACK_LASTFRAME", 0, 0)?;
        APIgameobj::SendMessageTo(e, 0x2a10_0006, 0, 0x28, e.r32(param_1 + 0x17c), 0);
        pu_done(e, param_1);
    }
    Ok(())
}

/// The code pointers of this file (see `skeleton::call_code`).
pub(crate) fn call_power_up(e: &mut Engine, f: u32, a0: Ptr) -> Option<LResult<()>> {
    Some(match f {
        addr_pu::heroInit_PowerUpIn => heroInit_PowerUpIn(e, a0),
        addr_pu::heroST_PowerUpIn_Loop => heroST_PowerUpIn_Loop(e, a0),
        addr_pu::heroInit_PowerUpOut => heroInit_PowerUpOut(e, a0),
        addr_pu::heroST_PowerUpOut_Loop => heroST_PowerUpOut_Loop(e, a0),
        addr_pu::heroExit_PowerUpInOut => heroExit_PowerUpInOut(e, a0),
        _ => return None,
    })
}

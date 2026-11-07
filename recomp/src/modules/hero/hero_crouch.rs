//! hero_crouch.c (10 functions). Addresses are module-relative (.text at 0x10000) in scooby__402420b4.
//! Present in 13 builds only (crouch levels); group 1 = 5f68ee0e (release: anim ids -4, camera
//! object 0x2a000020 instead of 0x2a00008f). The 44 other builds never enter these states.
use super::builds::*;
use super::skeleton::addr;
use super::*;
use crate::engine::*;
use crate::logic::LResult;

const SELF: u32 = 0x2a00_0000;

fn gb_or(e: &mut Engine, off: u32, v: u32) {
    let a = GB0(e) + off;
    let x = e.r32(a);
    e.w32(a, x | v);
}
fn gb_and(e: &mut Engine, off: u32, v: u32) {
    let a = GB0(e) + off;
    let x = e.r32(a);
    e.w32(a, x & v);
}
/// (anim, camera object) of the build: group 1 is 5f68ee0e.
fn rel(e: &Engine, gi: usize, anim0: u32) -> (u32, u32) {
    // WP-P20 R19: the crouch anim ids / camera id are per-build literals (each scooby build's
    // hero_crouch.c: heroInit_CrouchIn StartAnim id, SendMessageTo camera; CrouchOut = In - 1, 2a9862df In + 1). A wrong id
    // makes StartAnim fail (L+0x218 = -1), no anim end sets L+0x168 bit 1 and the hero stays in
    // heroST_CrouchIn_Loop forever (EP2/L01A crawl under the beam at 257.8,282.1)
    let tab: Option<(u32, u32)> = match crate::modules::hero::builds::BUILDS[e.rt.cur_build()].sha8 {
        "11adae8e" => Some((0x27, 0x2a00_0026)),
        "242c6a48" => Some((0x2c, 0x2a00_0037)),
        "2a9862df" => Some((0x2a, 0x2a00_0027)), // Out = 0x2b here (In + 1), see below
        "402420b4" => Some((0x33, 0x2a00_008f)),
        "415322c3" => Some((0x32, 0x2a00_0025)),
        "5f68ee0e" => Some((0x2f, 0x2a00_0020)),
        "6e3372f0" => Some((0x27, 0x2a00_0004)),
        "79185eb3" => Some((0x2b, 0x2a00_0012)),
        "7e698dff" => Some((0x2f, 0x2a00_000e)),
        "83265b14" => Some((0x2b, 0x2a00_0004)),
        "96c92057" => Some((0x26, 0x2a00_0030)),
        "d8a08bbd" => Some((0x34, 0x2a00_001e)),
        "f28bcd0e" => Some((0x2a, 0x2a00_001f)),
        "f5ff9d42" => Some((0x2a, 0x2a00_0027)),
        _ => None,
    };
    if let Some((a_in, cam)) = tab {
        // anim0 = the group-0 In (0x33) / Out (0x32) id: Out = In - 1 in every build
        // CrouchOut = In - 1 in every build except 2a9862df (EP2/L01A: In 0x2a, Out 0x2b); a wrong Out anim
        // starts the stand-up from another pose and PrimMove (prim 103 pinned) snaps the hero back 0.88 m
        let out = if crate::modules::hero::builds::BUILDS[e.rt.cur_build()].sha8 == "2a9862df" { a_in + 1 } else { a_in - 1 };
        return (if anim0 == 0x33 { a_in } else { out }, cam);
    }
    if g(e, gi) == 1 { (anim0 - 4, 0x2a00_0020) } else { (anim0, 0x2a00_008f) }
}
/// Rumble every 0.25 s (local+0x378 = next time).
fn rumble(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    if e.rf(param_1 + 0x378) < APIextra::TimerGetTime(e, 0x9000_0000) {
        let t = APIextra::TimerGetTime(e, 0x9000_0000);
        e.wf(param_1 + 0x378, t + 0.25);
        heroFUNC_CallVibration(e, f32::from_bits(0x3ea8_f5c3), 0.0, 0.25, 0.25)?;
    }
    Ok(())
}

/// 0x00015d90 (276 bytes) `void heroInit_CrouchDetect(int param_1)`
pub fn heroInit_CrouchDetect(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    debug_time(e, "heroInit_CrouchDetect", 0, 0)?;
    APIinput::InputConfigSetState(e, e.r32(param_1 + 0x188) as i32, i32::MIN);
    APIgameobj::SetContextFlag(e, SELF, 0x4_0010);
    APIphysic::DMSetActive(e, e.r32(param_1 + 0x1c8), 1);
    gb_or(e, 0x14, 0x100_0000);
    APIphysic::PrimSetIntAttribBits(e, SELF, e.r32(param_1 + 0x18c), 0, 2);
    e.w32(param_1 + 0x158, addr::heroST_CrouchDetect_Loop);
    Ok(())
}

/// 0x00015ea8 (260 bytes) `void heroST_CrouchDetect_Loop(int param_1)`
pub fn heroST_CrouchDetect_Loop(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    if e.r32(param_1 + 0x164) & 0x400 != 0 {
        if e.r32(GB0(e) + 0x34) & 0x10 != 0 {
            if e.r32(GB0(e) + 0x34) & 0x100 == 0 {
                APInewSound::Play(e, SELF, 0xa000_0002, 0);
            }
            gb_or(e, 0x34, 0x1300);
            APIgameobj::SendMessageTo(e, e.r32(param_1 + 0x180), 0, 0x6b, 0, 0);
        }
        e.w32(param_1 + 0x154, addr::heroInit_CrouchIn);
    }
    Ok(())
}

/// 0x00015fb0 (528 bytes) `void heroInit_CrouchIn(int param_1)`
pub fn heroInit_CrouchIn(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    let (anim, cam) = rel(e, G_heroInit_CrouchIn, 0x33);
    debug_time(e, "heroInit_CrouchIn", 0, 0)?;
    let a = heroFUNC_StartAnim(e, 0.1, anim as i32, 1, 5)?;
    e.w32(param_1 + 0x218, a as i32 as u32);
    APIphysic::DMSetActive(e, e.r32(param_1 + 0x1c8), 0);
    gb_or(e, 0x14, 0x100_0200);
    APIphysic::PrimSetIntAttribBits(e, SELF, e.r32(param_1 + 0x18c), 0, 2);
    APIinput::InputConfigSetState(e, e.r32(param_1 + 0x188) as i32, i32::MIN);
    APIgameobj::SetContextFlag(e, SELF, 0x20_0000);
    APIgameobj::SendMessageTo(e, cam, 0, 0x79, 0, 0);
    let axis = e.alloca(0x10);
    e.wv3(axis, [0.0, 1.0, 0.0]);
    APIgameobj::SetTranslation(e, SELF, param_1 + 0x308);
    let ang = e.rf(param_1 + 0x344);
    APIgameobj::SetRotation(e, SELF, axis, ang);
    let t = APIextra::TimerGetTime(e, 0x9000_0000);
    e.wf(param_1 + 0x378, t + 0.6);
    e.w32(param_1 + 0x158, addr::heroST_CrouchIn_Loop);
    e.w32(param_1 + 0x15c, addr::heroExit_CrouchIn);
    Ok(())
}

/// 0x000161c0 (316 bytes) `void heroST_CrouchIn_Loop(int param_1)`
pub fn heroST_CrouchIn_Loop(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    if e.r32(param_1 + 0x168) & 1 == 0 {
        rumble(e, param_1)?;
    } else if e.r32(GB0(e) + 0x14) & 4 == 0 {
        e.w32(param_1 + 0x154, addr::heroInit_CrouchOut);
    } else {
        e.w32(param_1 + 0x158, addr::heroST_CrouchLoop);
    }
    Ok(())
}

/// 0x00016300 (188 bytes) `void heroST_CrouchLoop(int param_1)`
pub fn heroST_CrouchLoop(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    rumble(e, param_1)
}

/// 0x000163c0 (128 bytes) `void heroExit_CrouchIn(int param_1)`
pub fn heroExit_CrouchIn(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    gb_and(e, 0x14, 0xfeff_fdff);
    APIphysic::PrimResetIntAttribBits(e, SELF, e.r32(param_1 + 0x18c), 0, 2);
    Ok(())
}

/// 0x00016440 (336 bytes) `void heroInit_CrouchOut(int param_1)`
pub fn heroInit_CrouchOut(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    let (anim, _) = rel(e, G_heroInit_CrouchOut, 0x32);
    debug_time(e, "heroInit_CrouchOut", 0, 0)?;
    let a = heroFUNC_StartAnim(e, 0.0, anim as i32, 1, 5)?;
    e.w32(param_1 + 0x218, a as i32 as u32);
    gb_or(e, 0x14, 0x100_0200);
    APIphysic::PrimSetIntAttribBits(e, SELF, e.r32(param_1 + 0x18c), 0, 2);
    APIinput::InputConfigSetState(e, e.r32(param_1 + 0x188) as i32, i32::MIN);
    APIphysic::SetVelocity(e, SELF, 0.0, 0.0, 0.0);
    APIgameobj::SetContextFlag(e, SELF, 0x20_0000);
    e.w32(param_1 + 0x158, addr::heroST_CrouchOut_Loop);
    e.w32(param_1 + 0x15c, addr::heroExit_CrouchOut);
    Ok(())
}

/// 0x00016590 (660 bytes) `void heroST_CrouchOut_Loop(int param_1)`
/// Rumbles during the first 65% of the stand-up anim.
pub fn heroST_CrouchOut_Loop(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    let (anim, cam) = rel(e, G_heroST_CrouchOut_Loop, 0x32);
    if e.r32(param_1 + 0x168) & 1 == 0 {
        let t = APIanim::GetTime(e, SELF, anim);
        let d = APIanim::GetDuration(e, SELF, anim);
        if t / d < 0.65 {
            rumble(e, param_1)?;
        }
    } else {
        APIgameobj::SendMessageTo(e, cam, 0, 0x7a, 0, 0);
        if e.r32(GB0(e) + 0x34) & 0x10 != 0 {
            gb_and(e, 0x34, 0xffff_edff);
            APIgameobj::SendMessageTo(e, e.r32(param_1 + 0x180), 0, 0x6b, 0, 0);
        }
        APIphysic::DMSetActive(e, e.r32(param_1 + 0x1c8), 0);
        let tm = e.r32(param_1 + 0x1fc);
        APIextra::TimerReset(e, tm);
        APIextra::TimerSetFrequency(e, tm, 1.0);
        APIextra::TimerRegisterNotify(e, SELF, tm, 10);
        debug_time(e, "ST_CrouchOut_Loop - LASTFRAME", 0, 0)?;
        e.w32(param_1 + 0x154, addr::heroInit_Stand);
    }
    Ok(())
}

/// 0x00016828 (200 bytes) `void heroExit_CrouchOut(int param_1)`
pub fn heroExit_CrouchOut(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    gb_and(e, 0x14, 0xfeff_fdff);
    if e.r32(GB0(e) + 0x34) & 0x10 != 0 {
        gb_and(e, 0x34, 0xffff_edff);
    }
    APIphysic::PrimResetIntAttribBits(e, SELF, e.r32(param_1 + 0x18c), 0, 2);
    Ok(())
}

/// 0x000168f0 (668 bytes) `void heroFUNC_CreateCrouchPhysic(int param_1)`
/// Three DMs: 0x19 -> +0x1c4 (crouch zone), 0x18 -> +0x1c8 (crouch detect, inactive), 0xb -> +0x1d0.
pub fn heroFUNC_CreateCrouchPhysic(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    debug_time(e, "heroFUNC_CreateCrouchPhysic", 0, 0)?;
    dm(e, param_1, 0x19, 0x1c4, &[(8, 0x3eb3_3333), (4, 0x67), (0, 9), (0xc, 6)], 0x1_0000);
    e.w32(param_1 + 0x34c, 0x3fc0_0000);
    let input = e.r32(param_1 + 0x188);
    dm(
        e,
        param_1,
        0x18,
        0x1c8,
        &[(0, 0x4049_0fdb), (4, 0x3fc0_0000), (8, 0x67), (0xc, input), (0x10, 0x2a), (0x14, 0x4090_0000), (0x18, 0x3f00_0000), (0x20, 0x3f00_0000)],
        0x4_0000,
    );
    APIphysic::DMSetActive(e, e.r32(param_1 + 0x1c8), 0);
    dm(e, param_1, 0xb, 0x1d0, &[(0, 0x67)], 0x20_0000);
    Ok(())
}

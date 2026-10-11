//! hero_movements.c (9 functions). Addresses are module-relative (.text at 0x10000) in scooby_ghost__08a011d0.
use super::builds::*;
use super::*;
use crate::engine::*;
use crate::logic::LResult;

/// 0x0001ea90 (288 bytes) `void heroInit_Move(int param_1)`
pub fn heroInit_Move(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    APIgameobj::SetContextFlag(e, SELF, 0x1001_0052);
    APIinput::InputConfigSetState(e, e.r32(param_1 + 0x188) as i32, -0x7fff_fd11);
    let a = heroFUNC_StartBlend(e, 0.0, 1, 2, 0)?;
    e.w32(param_1 + 0x218, a as i32 as u32);
    UpdateTimerOutOfTime(e, 2.0)?;
    gb14_mask(e, 0xffff_ffcf, 0);
    gb14_mask(e, 0xffef_ff7f, 0);
    e.w32(param_1 + 0x158, addr::heroST_Move_Loop);
    Ok(())
}

/// 0x0001ebb0 (428 bytes) `void heroST_Move_Loop(int param_1)`
pub fn heroST_Move_Loop(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    let f = e.r32(param_1 + 0x16c);
    if !f & 5 == 0 {
        e.w32(param_1 + 0x154, addr::heroInit_Sneak);
    } else if f & 1 == 0 {
        if f & 4 == 0 {
            heroFUNC_UpdateBlend_Others(e, param_1, 1)?;
        } else {
            e.w32(param_1 + 0x154, addr::heroInit_Sneaking);
        }
    } else {
        let m34 = e.r32(GB0(e) + 0x34);
        // short-circuit: IsOutOfTime only runs when the team bits allow a break
        if m34 & 0x10 == 0 || m34 & 0x100 != 0 || IsOutOfTime(e)? == 0 {
            e.w32(param_1 + 0x154, addr::heroInit_Stand);
        } else {
            let partner = e.r32(param_1 + 0x180);
            let d = LIBApiCFunctions::GetDistance(e, SELF as i32, partner, 0, 1);
            if d < 3.0 {
                APIgameobj::SendMessageTo(e, e.r32(param_1 + 0x180), 0, 0x60, 0x4_0000, 0);
            }
            e.w32(param_1 + 0x154, addr::heroInit_Break);
        }
    }
    Ok(())
}

/// 0x0001ed70 (396 bytes) `void heroInit_Sneaking(int param_1)`
pub fn heroInit_Sneaking(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    let a = heroFUNC_StartAnim(e, f32::from_bits(0x3dcccccd), 0xfffff843u32 as i32, 2, 0)?;
    e.w32(param_1 + 0x218, a as i32 as u32);
    APIgameobj::SetContextFlag(e, SELF, 0x1_0054);
    APIinput::InputConfigSetState(e, e.r32(param_1 + 0x188) as i32, -0x7fff_fd11);
    UpdateTimerOutOfTime(e, -0.3)?;
    APIgameobj::SendMessageTo(e, k(e, ID_2a000096), 0, 0x5e, 1, 0);
    gb14_mask(e, u32::MAX, 0x20);
    gb14_mask(e, 0xffff_ffef, 0);
    gb14_mask(e, 0xffef_ff7f, 0);
    e.w32(param_1 + 0x158, addr::heroST_Sneaking_Loop);
    e.w32(param_1 + 0x15c, addr::heroExit_Sneak);
    Ok(())
}

/// 0x0001ef00 (1144 bytes) `void heroST_Sneaking_Loop(int param_1)`
pub fn heroST_Sneaking_Loop(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    let f = e.r32(param_1 + 0x16c);
    if !f & 5 == 0 {
        debug_time(e, "heroST_Sneaking_Loop to heroInit_Sneak", 0, 0)?;
        e.w32(param_1 + 0x154, addr::heroInit_Sneak);
    } else if f & 1 != 0 {
        end_sneaking(e, param_1)?;
        e.w32(param_1 + 0x154, addr::heroInit_Stand);
    } else if f & 4 == 0 {
        end_sneaking(e, param_1)?;
        e.w32(param_1 + 0x154, addr::heroInit_Move);
    } else if IsOutOfTime(e)? != 0 {
        let n = e.r32(param_1 + 0x268).wrapping_add(1);
        e.w32(param_1 + 0x268, n);
        UpdateTimerOutOfTime(e, 0.3)?;
        let (snd, msg) = match n {
            1 => (0x34, "Sound - 1"),
            2 | 8 => (0x35, "Sound - 2"),
            3 | 7 => (0x36, "Sound - 3"),
            4 | 6 => (0x37, "Sound - 4"),
            5 => (0x38, "Sound - 5"),
            _ => {
                e.w32(param_1 + 0x268, 1);
                (0x34, "Sound - start")
            }
        };
        e.w32(param_1 + 0x21c, snd);
        debug_info(e, msg);
        APInewSound::Play(e, k(e, ID_2a000096), e.r32(param_1 + 0x21c), 0);
    }
    Ok(())
}

/// Stop the tiptoe sound and tell the snd_obj the sneak is over (0x5e, 0).
fn end_sneaking(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    debug_time(e, "heroST_Sneaking_Loop to heroInit_Move", 0, 0)?;
    e.w32(param_1 + 0x268, 0);
    let snd_obj = k(e, ID_2a000096);
    if e.r32(param_1 + 0x21c) != u32::MAX {
        APInewSound::StopSound(e, snd_obj, e.r32(param_1 + 0x21c));
    }
    e.w32(param_1 + 0x21c, u32::MAX);
    APIgameobj::SendMessageTo(e, snd_obj, 0, 0x5e, 0, 0);
    Ok(())
}

/// 0x0001f388 (176 bytes) `void heroInit_Break(int param_1)`
pub fn heroInit_Break(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    let a = heroFUNC_StartAnim(e, 0.25, k(e, heroInit_Break_0_1) as i32, 1, 5)?; // 3 in the reference build
    e.w32(param_1 + 0x218, a as i32 as u32);
    APIgameobj::SetContextFlag(e, SELF, 0x1000_0050);
    APIinput::InputConfigSetState(e, e.r32(param_1 + 0x188) as i32, i32::MIN);
    e.w32(param_1 + 0x158, addr::heroST_Break_Loop);
    Ok(())
}

/// 0x0001f438 (172 bytes) `void heroST_Break_Loop(int param_1)`
pub fn heroST_Break_Loop(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    if e.r32(param_1 + 0x168) & 1 != 0 {
        if e.r32(GB0(e) + 8) == e.r32(L(e) + 0x17c) {
            e.w32(param_1 + 0x154, addr::heroInit_Stand);
        } else {
            APIphysic::DMSetStateBits(e, e.r32(param_1 + 0x1dc), 1);
            e.w32(param_1 + 0x154, addr::heroInit_FollowStand);
        }
    }
    Ok(())
}

/// 0x0001f4e8 (152 bytes) `bool heroIsTrackWPReached(float param_1,int param_2)`
/// The track DM's params: the waypoint position is at +8.
pub fn heroIsTrackWPReached(e: &mut Engine, param_1: f32, param_2: Ptr) -> LResult<u32> {
    let p = e.alloca(0x20); // auStack_40 [8] + auStack_38
    APIphysic::DMGetParams(e, e.r32(param_2 + 0x1d8), p);
    Ok(LIBApiCFunctions::CheckRangePoint(e, param_1, SELF, p + 8, 1) as u32)
}

/// 0x0001f588 (356 bytes) `void heroFUNC_SetTrackID(undefined4 param_1,int param_2,undefined4 *param_3)`
pub fn heroFUNC_SetTrackID(e: &mut Engine, param_1: u32, param_2: i32, param_3: Ptr) -> LResult<()> {
    debug_info(e, "FUNC_SetTrackID");
    let s = e.alloca(0x18); // iStack_40 .. uStack_2c
    APIphysic::DMGetParams(e, e.r32(L(e) + 0x1d8), s);
    e.w32(s + 4, param_1);
    if param_2 == 0 {
        e.w32(s, 0);
        for i in 0..3 {
            let v = e.r32(param_3 + i * 4);
            e.w32(s + 8 + i * 4, v);
        }
        e.w32(s + 0x14, 5);
        debug_info_value(e, "TRACK 2-", s + 8, 4);
    } else {
        for i in 0..3 {
            e.w32(s + 8 + i * 4, 0);
        }
        e.w32(s + 0x14, 1);
        e.w32(s, param_2 as u32);
    }
    APIphysic::DMSetParams(e, e.r32(L(e) + 0x1d8), s);
    Ok(())
}

/// 0x0001f6f0 (1116 bytes) `void heroFUNC_CreateMovementsPhysic(int param_1)`
pub fn heroFUNC_CreateMovementsPhysic(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    // Parameter blocks live in one frame at the original stack offsets (frame base = sp+0, size 0xa0).
    let fr = e.alloca(0xa0);
    let at = |off: u32| fr + 0xa0 - off;
    let (a0, s60, s50, s40) = (at(0xa0), at(0x60), at(0x50), at(0x40));
    let input = e.r32(param_1 + 0x188);

    // two type-6 DMs: speed 7.5, ranges from local+0x70 (radius) / 2.0
    for (slot, mask) in [(400u32, 2u32), (0x194, 4)] {
        let dm = APIphysic::DMAdd(e, SELF, 6);
        e.w32(param_1 + slot, dm);
        APIphysic::DMGetParams(e, e.r32(param_1 + slot), a0);
        e.w32(a0 + 0x14, input);
        e.w32(a0 + 0x18, 6);
        e.w32(a0, 0x40f0_0000);
        let (a, b) = if slot == 400 {
            let r = e.rf(param_1 + 0x70);
            (r * 0.9, e.rf(param_1 + 0x70) * 0.333)
        } else {
            (2.0, 2.0)
        };
        e.wf(a0 + 8, a);
        e.wf(a0 + 0xc, b);
        e.w32(a0 + 0x10, 4);
        APIphysic::DMSetParams(e, e.r32(param_1 + slot), a0);
        APIphysic::DMSetContextMask(e, e.r32(param_1 + slot), mask);
    }
    // two type-4 DMs (local+0x74)
    for (slot, mask) in [(0x198u32, 0x1000u32), (0x1a8, 0x2000)] {
        let dm = APIphysic::DMAdd(e, SELF, 4);
        e.w32(param_1 + slot, dm);
        APIphysic::DMGetParams(e, e.r32(param_1 + slot), s50);
        e.w32(s50 + 0xc, 1);
        e.w32(s50, 0);
        e.w32(s50 + 4, 0);
        let v = e.r32(param_1 + 0x74);
        e.w32(s50 + 8, v);
        APIphysic::DMSetParams(e, e.r32(param_1 + slot), s50);
        APIphysic::DMSetContextMask(e, e.r32(param_1 + slot), mask);
    }
    let dm = APIphysic::DMAdd(e, SELF, 2);
    e.w32(param_1 + 0x1ac, dm);
    APIphysic::DMGetParams(e, e.r32(param_1 + 0x1ac), s60);
    e.w32(s60, 0x40f0_0000);
    e.w32(s60 + 4, 5);
    let v = e.r32(param_1 + 0x188);
    e.w32(s60 + 8, v);
    e.w32(s60 + 0xc, 6);
    APIphysic::DMSetParams(e, e.r32(param_1 + 0x1ac), s60);
    APIphysic::DMSetContextMask(e, e.r32(param_1 + 0x1ac), 1);

    let dm = APIphysic::DMAdd(e, SELF, 1);
    e.w32(param_1 + 0x1d8, dm);
    APIphysic::DMGetParams(e, e.r32(param_1 + 0x1d8), s40);
    e.w32(s40, 0);
    e.w32(s40 + 4, 0x4120_0000);
    e.w32(s40 + 0x14, 5);
    APIphysic::DMSetParams(e, e.r32(param_1 + 0x1d8), s40);
    APIphysic::DMSetContextMask(e, e.r32(param_1 + 0x1d8), 0x2000_0000);
    Ok(())
}


// ---- helpers (from the hero port) ----

pub(crate) fn gb14_mask(e: &mut Engine, and: u32, or: u32) {
    let a = GB0(e) + 0x14;
    let x = e.r32(a);
    e.w32(a, (x & and) | or);
}

//! hero_follow.c (18 functions). Addresses are module-relative (.text at 0x10000) in scooby_ghost__08a011d0.
use super::builds::*;
use super::*;
use crate::engine::*;
use crate::logic::LResult;

/// 0x0001a5b8 (548 bytes) `void heroSet_Follower(int param_1)`
pub fn heroSet_Follower(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    let s = e.alloca(0x20); // auStack_50[8]
    APIphysic::DMGetParams(e, e.r32(param_1 + 0x1dc), s);
    if e.r32(s) != e.r32(GB0(e) + 8) {
        debug_time(e, "heroSet_Follower--> OLD Lead ID", s, 1)?;
        let p = GB0(e) + 8;
        debug_time(e, "heroSet_Follower--> NEW Lead ID", p, 1)?;
    }
    let lead = e.r32(GB0(e) + 8);
    e.w32(s, lead);
    APIphysic::DMSetParams(e, e.r32(param_1 + 0x1dc), s);
    debug_time(e, "heroSet_Follower", 0, 0)?;
    APIphysic::DMSetStateBits(e, e.r32(param_1 + 0x1dc), 1);
    APIphysic::SetPriority(e, SELF, 4);
    APIinput::InputConfigSetState(e, e.r32(param_1 + 0x188) as i32, -0x7fff_fd22);
    APIinput::InputConfigActivate(e, e.r32(param_1 + 0x188) as i32, 0);
    let m34 = e.r32(GB0(e) + 0x34);
    if m34 & 0x10 != 0 && m34 & 0x300 == 0 {
        debug_info(e, "Dm Follow active");
        APIphysic::DMSetActive(e, e.r32(param_1 + 0x1dc), 1);
    }
    Ok(())
}

/// 0x0001a7e0 (216 bytes) `void heroInit_FollowMove(int param_1)`
pub fn heroInit_FollowMove(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    APIgameobj::SetContextFlag(e, SELF, 0x5000_0050);
    let a = heroFUNC_StartBlend(e, f32::from_bits(0x3ca3d70a), 1, 2, 0)?;
    e.w32(param_1 + 0x218, a as i32 as u32);
    gb14_and(e, 0xffed_ffff);
    heroUpdateDMFollow(e, param_1, 0)?;
    e.w32(param_1 + 0x158, addr::heroST_FollowMove_Loop);
    Ok(())
}

/// 0x0001a8b8 (228 bytes) `void heroST_FollowMove_Loop(int param_1)`
pub fn heroST_FollowMove_Loop(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    let (f168, f16c) = (e.r32(param_1 + 0x168), e.r32(param_1 + 0x16c));
    if f168 & 0x10 != 0 && f16c & 4 != 0 {
        e.w32(param_1 + 0x154, addr::heroInit_FollowSneak);
    } else if f168 & 0x10 != 0 {
        e.w32(param_1 + 0x154, addr::heroInit_FollowStand);
    } else if f16c & 4 != 0 {
        e.w32(param_1 + 0x154, addr::heroInit_FollowSneaking);
    } else {
        heroFUNC_UpdateBlend_Others(e, param_1, 1)?;
    }
    Ok(())
}

/// 0x0001a9b0 (344 bytes) `void heroInit_FollowStand(int param_1)`
pub fn heroInit_FollowStand(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    if e.r32(GB0(e) + 0x34) & 0x301 == 0 {
        APIphysic::DMSetStateBits(e, e.r32(param_1 + 0x1dc), 1);
        APIextra::TimerReset(e, e.r32(param_1 + 0x208));
        heroUpdateDMFollow(e, param_1, 0)?;
        let a = heroFUNC_SelectScoobyRun2Stand(e)?;
        e.w32(param_1 + 0x218, a as i32 as u32);
        gb14_and(e, 0xffed_ffff);
        APIgameobj::SetContextFlag(e, SELF, 0x5000_0050);
        e.w32(param_1 + 0x158, addr::heroST_FollowStand_Loop);
    } else {
        debug_time(e, "heroInit_FollowStand -> still desabled", 0, 0)?;
        heroInit_Unhook(e, param_1)?;
    }
    Ok(())
}

/// 0x0001ab08 (336 bytes) `void heroST_FollowStand_Loop(int param_1)`
pub fn heroST_FollowStand_Loop(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    let (f168, f16c) = (e.r32(param_1 + 0x168), e.r32(param_1 + 0x16c));
    if f168 & 0x20 != 0 && f16c & 4 != 0 {
        e.w32(param_1 + 0x154, addr::heroInit_FollowSneaking);
    } else if f168 & 0x20 != 0 {
        e.w32(param_1 + 0x154, addr::heroInit_FollowMove);
    } else if f16c & 4 != 0 {
        e.w32(param_1 + 0x154, addr::heroInit_FollowSneak);
    } else {
        if f168 & 1 != 0 {
            let a = heroFUNC_SelectScoobyRun2Stand(e)?;
            e.w32(param_1 + 0x218, a as i32 as u32);
        }
        if e.r32(GB0(e) + 0x14) & 0x840_090c == 0 && IsOutOfTime(e)? != 0 {
            e.w32(param_1 + 0x154, addr::heroInit_FollowIdle);
        }
    }
    Ok(())
}

/// 0x0001ac68 (2356 bytes) `void heroInit_FollowIdle(int param_1)`
pub fn heroInit_FollowIdle(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    if e.r32(GB0(e) + 0x14) & 0x10_0080 == 0 {
        let fl_a = e.alloca(0x20); // aEStack_c0, +8 = uStack_b8
        let fl_b = e.alloca(0x20); // aEStack_a0, +8 = uStack_98
        APIphysic::GetFloorAttribs(e, e.r32(GB0(e) + 8), fl_a);
        APIphysic::GetFloorAttribs(e, e.r32(L(e) + 0x180), fl_b);
        UpdateTimerOutOfTime(e, 2.0)?;
        if e.r32(fl_a + 8) & 0x41 == 0 && e.r32(fl_b + 8) & 0x41 == 0 {
            let d80 = e.alloca(0x10);
            let d70 = e.alloca(0x10);
            e.wf(d80, 1.0);
            e.wf(d80 + 4, 0.0);
            e.wf(d80 + 8, -0.25);
            let lead = e.r32(GB0(e) + 8);
            APIgameobj::GetPosition(e, lead, 0x7fff_ffff, d70);
            let lead = e.r32(GB0(e) + 8);
            APIgameobj::TransformVector(e, lead, d80, d80);
            APImath::CVectorAdd(e, d70, d70, d80);
            if heroFUNC_IsAnythingBetweenUs(e, 0.35, SELF, 0, d70, 6)? == 0 {
                let s = e.alloca(0x18); // uStack_60..uStack_4c
                APIphysic::DMGetParams(e, e.r32(param_1 + 0x1d8), s);
                e.w32(s, 0);
                for i in 0..3 {
                    let v = e.r32(d70 + i * 4);
                    e.w32(param_1 + 0x2e4 + i * 4, v);
                    e.w32(s + 8 + i * 4, v);
                }
                e.w32(s + 4, 0x4148_0000);
                e.w32(s + 0x14, 5);
                APIphysic::DMSetParams(e, e.r32(param_1 + 0x1d8), s);
                let t = e.alloca(0x10); // auStack_40
                APIphysic::DMGetParams(e, e.r32(param_1 + 0x1a8), t);
                e.w32(t + 0xc, 1);
                let v = e.r32(param_1 + 0x74);
                e.w32(t + 8, v);
                APIphysic::DMSetParams(e, e.r32(param_1 + 0x1a8), t);
                gb14_or(e, 0x2_0000);
            }
        }
    }
    if e.r32(param_1 + 0x170) & 0x2_0000 == 0 {
        let gb = e.r32(GB0(e) + 0x14);
        if gb & 0x80 != 0 && gb & 0x10_0000 == 0 && e.r32(param_1 + 0x218) as i32 == -0x7bf {
            let a = heroFUNC_StartAnim(e, f32::from_bits(0x3d4ccccd), 0xfffff842u32 as i32, 2, 0)?;
            e.w32(param_1 + 0x218, a as i32 as u32);
            APIgameobj::SetContextFlag(e, SELF, 0x5000_0050);
        } else if gb & 0x10_0000 == 0 && gb & 0x2_0000 != 0 {
            let a = heroFUNC_StartAnim(e, f32::from_bits(0x3ca3d70a), 1, 2, 0)?;
            e.w32(param_1 + 0x218, a as i32 as u32);
            APIgameobj::SetContextFlag(e, SELF, 0x7000_2050);
        } else {
            if gb & 0x10_0000 == 0 {
                debug_time(e, "heroInit_FollowIdle -> Blocked", 0, 0)?;
            }
            let a = heroFUNC_SelectStand(e, param_1, e.r32(param_1 + 0x218) as i32, 0)?;
            e.w32(param_1 + 0x218, a as i32 as u32);
            let f = LIBApiCFunctions::FloatRandom(e, 8.0, 12.0, 0.1);
            UpdateTimerOutOfTime(e, f)?;
            APIgameobj::SetContextFlag(e, SELF, 0x5000_0050);
        }
    } else {
        let f = e.r32(param_1 + 0x174);
        let seek = |e: &mut Engine, off: u32| -> LResult<()> {
            let anim = e.r32(param_1 + off);
            let a = heroFUNC_SeekAnim(e, anim, 7, (param_1 + 0xa4) as i32, 1)?;
            e.w32(param_1 + 0x218, a as i32 as u32);
            Ok(())
        };
        if f & 0x100_0000 != 0 || f & 0x200_0000 != 0 {
            let (msg, off) = if f & 0x100_0000 != 0 {
                ("----follower ---- event_CTRL_IDLE_DUO_1st", 0xd4)
            } else {
                ("----follower ---- event_CTRL_IDLE_DUO_2nd", 0xe4)
            };
            debug_info(e, msg);
            seek(e, off)?;
            let d = APIanim::GetDuration(e, SELF, e.r32(param_1 + 0x218));
            UpdateTimerOutOfTime(e, d + 1.0)?;
        } else if f & 0x400_0000 != 0 {
            debug_info(e, "----follower ---- event_CTRL_IDLE_SLEEP_SOLO_IN");
            seek(e, 0xf4)?;
        } else {
            let pick = if f & 0x800_0000 != 0 {
                Some(("----follower ---- event_CTRL_IDLE_SLEEP_TEAM_IN", 0xfffff836u32, 1))
            } else if f & 0x1000_0000 != 0 {
                Some(("----follower ---- event_CTRL_IDLE_SLEEP_TEAM", 0xfffff837, 2))
            } else if f & 0x2000_0000 != 0 {
                Some(("----follower ---- event_CTRL_IDLE_SLEEP_TEAM_VAR", 0xfffff838, 1))
            } else {
                None
            };
            if let Some((msg, anim, mode)) = pick {
                debug_info(e, msg);
                let a = heroFUNC_StartAnim(e, f32::from_bits(0x3d4ccccd), anim as i32, mode, 0)?;
                e.w32(param_1 + 0x218, a as i32 as u32);
            }
        }
        APIgameobj::SetContextFlag(e, SELF, 0x5000_0050);
    }
    e.w32(param_1 + 0x158, addr::heroST_FollowIdle_Loop);
    Ok(())
}

/// 0x0001b5a0 (884 bytes) `void heroST_FollowIdle_Loop(int param_1)`
pub fn heroST_FollowIdle_Loop(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    let (f168, f16c) = (e.r32(param_1 + 0x168), e.r32(param_1 + 0x16c));
    if f168 & 0x20 != 0 && f16c & 4 != 0 {
        e.w32(param_1 + 0x154, addr::heroInit_FollowSneaking);
    } else if f168 & 0x20 != 0 {
        e.w32(param_1 + 0x154, addr::heroInit_FollowMove);
    } else if f16c & 4 != 0 {
        e.w32(param_1 + 0x154, addr::heroInit_FollowSneak);
    } else if e.r32(param_1 + 0x164) & 2 != 0 {
        e.w32(param_1 + 0x154, addr::heroInit_FollowStand);
    } else if f168 & 1 != 0 {
        e.w32(param_1 + 0x154, addr::heroInit_FollowIdle);
    } else if f168 & 8 != 0 {
        let o = APIgameobj::LaunchLatant(e, k(e, ID_2a800094));
        e.w32(param_1 + 0x264, o);
        let v = e.alloca(0x10);
        let p = APIgameobj::GetPosition(e, SELF, 0x7fff_ffff, v);
        APIgameobj::SetTranslation(e, e.r32(param_1 + 0x264), p);
        APIgameobj::TakeOrientation(e, e.r32(param_1 + 0x264), SELF, 0x7fff_ffff);
        e.w32(param_1 + 0x15c, addr::heroExit_Idle_Loop);
    } else if e.r32(GB0(e) + 0x14) & 0x40_0000 != 0 {
        e.w32(param_1 + 0x154, addr::heroInit_FollowStand);
    } else if e.r32(GB0(e) + 0x14) & 0x2_0000 != 0 && heroIsTrackWPReached(e, f32::from_bits(0x3e19999a), param_1)? != 0 {
        gb14_and(e, 0xfffd_ffff);
        gb14_or(e, 0x10_0000);
        let lead = e.r32(GB0(e) + 8);
        LIBApiCFunctions::TakeRotation(e, 0.0, SELF, lead as i32);
        APIgameobj::SetTranslation(e, SELF, param_1 + 0x2e4);
        e.w32(param_1 + 0x154, addr::heroInit_FollowIdle);
    } else if e.r32(GB0(e) + 0x14) & 0x80 == 0 && IsOutOfTime(e)? != 0 {
        e.w32(param_1 + 0x154, addr::heroInit_FollowIdle);
    }
    Ok(())
}

/// 0x0001b950 (172 bytes) `void heroInit_FollowSneaking(int param_1)`
pub fn heroInit_FollowSneaking(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    let a = heroFUNC_StartAnim(e, f32::from_bits(0x3dcccccd), 0xfffff843u32 as i32, 2, 0)?;
    e.w32(param_1 + 0x218, a as i32 as u32);
    APIgameobj::SetContextFlag(e, SELF, 0x5000_0050);
    heroUpdateDMFollow(e, param_1, 1)?;
    e.w32(param_1 + 0x158, addr::heroST_FollowSneaking_Loop);
    Ok(())
}

/// 0x0001ba00 (188 bytes) `void heroST_FollowSneaking_Loop(int param_1)`
pub fn heroST_FollowSneaking_Loop(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    let (f168, f16c) = (e.r32(param_1 + 0x168), e.r32(param_1 + 0x16c));
    if f168 & 0x10 != 0 && f16c & 4 != 0 {
        e.w32(param_1 + 0x154, addr::heroInit_FollowSneak);
    } else if f168 & 0x10 != 0 {
        e.w32(param_1 + 0x154, addr::heroInit_FollowStand);
    } else if f16c & 4 == 0 {
        e.w32(param_1 + 0x154, addr::heroInit_FollowMove);
    }
    Ok(())
}

/// 0x0001bad0 (276 bytes) `void heroUpdateDMFollow(int param_1,int param_2)`
/// Follow DM speeds/ranges: normal (param_2 == 0) or sneaking.
pub fn heroUpdateDMFollow(e: &mut Engine, param_1: Ptr, param_2: i32) -> LResult<()> {
    let s = e.alloca(0x18); // auStack_40 .. fStack_2c
    APIphysic::DMGetParams(e, e.r32(param_1 + 0x1dc), s);
    if param_2 == 0 {
        e.w32(s + 4, 0x3fe0_0000);
        e.w32(s + 8, 0x4020_0000);
        e.w32(s + 0x10, 0x3d99_999a);
        let r = e.rf(param_1 + 0x70) * 1.1;
        e.wf(s + 0x14, r);
    } else {
        e.w32(s + 4, 0x3fa0_0000);
        e.w32(s + 8, 0x3fe0_0000);
        e.w32(s + 0x10, 0x3d4c_cccd);
        e.wf(s + 0x14, 2.0);
    }
    e.w32(s + 0xc, 0x3e80_0000);
    APIphysic::DMSetParams(e, e.r32(param_1 + 0x1dc), s);
    Ok(())
}

/// 0x0001bbe8 (168 bytes) `void heroInit_FollowSneak(int param_1)`
pub fn heroInit_FollowSneak(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    let a = heroFUNC_StartBlend(e, 0.25, 3, 2, 0)?;
    e.w32(param_1 + 0x218, a as i32 as u32);
    APIgameobj::SetContextFlag(e, SELF, 0x5000_0050);
    heroUpdateDMFollow(e, param_1, 1)?;
    e.w32(param_1 + 0x158, addr::heroST_FollowSneak_Loop);
    Ok(())
}

/// 0x0001bc90 (188 bytes) `void heroST_FollowSneak_Loop(int param_1)`
pub fn heroST_FollowSneak_Loop(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    let (f168, f16c) = (e.r32(param_1 + 0x168), e.r32(param_1 + 0x16c));
    if f168 & 0x20 != 0 && f16c & 4 != 0 {
        e.w32(param_1 + 0x154, addr::heroInit_FollowSneaking);
    } else if f168 & 0x20 != 0 {
        e.w32(param_1 + 0x154, addr::heroInit_FollowMove);
    } else if f16c & 4 == 0 {
        e.w32(param_1 + 0x154, addr::heroInit_FollowStand);
    }
    Ok(())
}

/// 0x0001bd60 (772 bytes) `void heroInit_Unhook(int param_1)`
/// GB0+0x34 team bits: 1 = solo, 0x100 = unhooked (too far), 0x200 = follow disabled.
pub fn heroInit_Unhook(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    debug_time(e, "Init_Unhook", 0, 0)?;
    let dm = e.r32(param_1 + 0x1dc);
    let m34 = e.r32(GB0(e) + 0x34);
    if m34 & 1 == 0 {
        if m34 & 0x300 == 0 {
            debug_time(e, "Init_Unhook ON", 0, 0)?;
            APIphysic::DMSetActive(e, dm, 1);
            return heroInit_FollowStand(e, param_1);
        }
        if e.r32(GB0(e) + 0x34) & 0x100 != 0 {
            debug_time(e, "Init_Unhook OFF", 0, 0)?;
            APIphysic::DMSetActive(e, dm, 0);
            APIphysic::DMSetStateBits(e, dm, 1);
        }
        if e.r32(GB0(e) + 0x34) & 0x200 != 0 {
            debug_time(e, "Init_Unhook DESABLED", 0, 0)?;
            APIphysic::DMSetActive(e, dm, 0);
            APIphysic::DMSetStateBits(e, dm, 1);
        }
    } else {
        debug_time(e, "Init_Unhook SOLO MODE", 0, 0)?;
        APIphysic::DMSetActive(e, dm, 0);
        APIphysic::DMSetStateBits(e, dm, 1);
    }
    debug_time(e, "heroInit_Unhook", 0, 0)?;
    let a = heroFUNC_StartAnim(e, f32::from_bits(0x3e19999a), 0x2 as i32, 2, 0)?;
    e.w32(param_1 + 0x218, a as i32 as u32);
    APIgameobj::SetContextFlag(e, SELF, 0x5000_0050);
    APIinput::InputConfigSetState(e, e.r32(param_1 + 0x188) as i32, -0x7fff_fd22);
    UpdateTimerOutOfTime(e, 1.0)?;
    e.w32(param_1 + 0x158, addr::heroST_Unhook_Loop);
    Ok(())
}

/// 0x0001c068 (528 bytes) `void heroST_Unhook_Loop(int param_1)`
pub fn heroST_Unhook_Loop(e: &mut Engine, param_1: i32) -> LResult<()> {
    let param_1 = param_1 as u32;
    if e.r32(GB0(e) + 0x34) & 0x10 != 0 && IsOutOfTime(e)? != 0 {
        debug_info(e, "ST_Unhook_Loop");
        UpdateTimerOutOfTime(e, 0.75)?;
        let m34 = e.r32(GB0(e) + 0x34);
        if m34 & 0x100 != 0 && m34 & 0x200 == 0 {
            let lead = e.r32(GB0(e) + 8);
            let d = LIBApiCFunctions::GetDistance(e, SELF as i32, lead, 0, 1);
            if d <= 7.5 {
                let lead = e.r32(GB0(e) + 8);
                if heroFUNC_IsAnythingBetweenUs(e, 0.15, SELF, lead as i32, 0, 0)? == 0 {
                    debug_time(e, "ST_Unhook_Loop Reconnecting", 0, 0)?;
                    let a = GB0(e) + 0x34;
                    let x = e.r32(a);
                    e.w32(a, x & 0xffff_feff);
                    APInewSound::Play(e, k(e, heroST_Unhook_Loop_0_1), k(e, ID_a0000003), 0);
                    heroInit_Unhook(e, param_1)?;
                }
            }
        }
    }
    Ok(())
}

/// 0x0001c280 (520 bytes) `void heroInit_BumpByHero(int param_1)`
pub fn heroInit_BumpByHero(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    let s = e.alloca(0x10); // auStack_80
    APIphysic::DMGetParams(e, e.r32(param_1 + 0x1a8), s);
    e.w32(s + 0xc, 1);
    let v = e.r32(param_1 + 0x70);
    e.w32(s + 8, v);
    APIphysic::DMSetParams(e, e.r32(param_1 + 0x1a8), s);
    debug_time(e, "heroInit_BumpByHero", 0, 0)?;
    let a = heroFUNC_StartAnim(e, 0.0, 1, 1, 5)?;
    e.w32(param_1 + 0x218, a as i32 as u32);
    gb14_or(e, 0x4_0000);
    APIgameobj::SetContextFlag(e, SELF, 0x7000_1050);
    let fa = e.alloca(0x20); // aEStack_70, +8 = uStack_68
    let fb = e.alloca(0x20); // aEStack_50, +8 = uStack_48
    APIphysic::GetFloorAttribs(e, e.r32(GB0(e) + 8), fa);
    APIphysic::GetFloorAttribs(e, e.r32(L(e) + 0x180), fb);
    if e.r32(fa + 8) & 0x40 != 0 || e.r32(fb + 8) & 0x40 != 0 {
        debug_info(e, "OK - On floor");
        APIphysic::PrimSetIntAttribBits(e, SELF, e.r32(param_1 + 0x18c), 5, 0x200);
    }
    e.w32(param_1 + 0x158, addr::heroST_BumpByHero_Loop);
    e.w32(param_1 + 0x15c, addr::heroExit_BumpByHero);
    Ok(())
}

/// 0x0001c488 (80 bytes) `void heroST_BumpByHero_Loop(int param_1)`
pub fn heroST_BumpByHero_Loop(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    if e.r32(param_1 + 0x168) & 1 != 0 {
        e.w32(param_1 + 0x154, addr::heroInit_FollowStand);
    }
    Ok(())
}

/// 0x0001c4d8 (128 bytes) `void heroExit_BumpByHero(int param_1)`
pub fn heroExit_BumpByHero(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    gb14_and(e, 0xfffb_ffff);
    APIphysic::PrimResetIntAttribBits(e, SELF, e.r32(param_1 + 0x18c), 5, 0x200);
    Ok(())
}

/// 0x0001c558 (324 bytes) `void heroFUNC_CreateFollowPhysic(int param_1)`
/// Type-0x14 follow DM on the partner (local+0x180); sets team bit 0x10.
pub fn heroFUNC_CreateFollowPhysic(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    let dm = APIphysic::DMAdd(e, SELF, 0x14);
    e.w32(param_1 + 0x1dc, dm);
    let s = e.alloca(0x20); // uStack_40 .. uStack_24
    APIphysic::DMGetParams(e, e.r32(param_1 + 0x1dc), s);
    let p = e.r32(param_1 + 0x180);
    e.w32(s, p);
    e.w32(s + 0x1c, 1);
    e.w32(s + 4, 0x3fe0_0000);
    e.w32(s + 8, 0x4020_0000);
    e.w32(s + 0xc, 0x3e80_0000);
    e.w32(s + 0x10, 0x3d99_999a);
    let r = e.rf(param_1 + 0x70) * 1.1;
    e.wf(s + 0x14, r);
    e.w32(s + 0x18, 0x3f40_0000);
    APIphysic::DMSetParams(e, e.r32(param_1 + 0x1dc), s);
    APIphysic::DMSetContextMask(e, e.r32(param_1 + 0x1dc), 0x4000_0000);
    let a = GB0(e) + 0x34;
    let x = e.r32(a);
    e.w32(a, x | 0x10);
    Ok(())
}

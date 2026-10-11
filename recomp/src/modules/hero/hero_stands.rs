//! hero_stands.c (20 functions). Addresses are module-relative (.text at 0x10000) in scooby__402420b4.
//!
//! Per-build anim ids come from builds.rs slots. Build shaggy__c7b040f3 writes several of them as
//! negative literals (`== -0x12`); its slots hold the magnitude, so those sites go through [`kneg`].
use super::builds::*;
use super::skeleton::addr;
use super::*;
use crate::engine::*;
use crate::logic::LResult;

const SELF: u32 = 0x2a00_0000;

/// A slot the c7b040f3 build writes as `-literal` (its anim table uses negative ids).
pub(crate) fn kneg(e: &Engine, i: usize) -> u32 {
    let v = k(e, i);
    if builds::BUILDS[e.rt.cur_build()].sha8 == "c7b040f3" {
        v.wrapping_neg()
    } else {
        v
    }
}
fn or_gb14(e: &mut Engine, v: u32) {
    let a = GB0(e) + 0x14;
    let x = e.r32(a);
    e.w32(a, x | v);
}
fn and_gb14(e: &mut Engine, v: u32) {
    let a = GB0(e) + 0x14;
    let x = e.r32(a);
    e.w32(a, x & v);
}

/// 0x0002caf8 (452 bytes) `long heroFUNC_SelectStand(int param_1,int param_2,int param_3)`
/// The three stand triples (anim ids written to param_1+0xa4/0xb4/0xc4, picked by the fear meter)
/// are per-build slots; same shape in all groups (group 2's `== -0x12` via [`kneg`]).
pub fn heroFUNC_SelectStand(e: &mut Engine, param_1: Ptr, param_2: i32, param_3: i32) -> LResult<i64> {
    let mut frq: u32 = 3;
    if param_2 as u32 == kneg(e, heroFUNC_SelectStand_0_1) {
        let m = e.rf(e.r32(GD(e) + 4));
        let s = if m < 0.25 {
            [heroFUNC_SelectStand_0_2, heroFUNC_SelectStand_0_3, heroFUNC_SelectStand_0_4]
        } else if m < 0.75 {
            [heroFUNC_SelectStand_0_5, heroFUNC_SelectStand_0_6, heroFUNC_SelectStand_0_7]
        } else {
            [heroFUNC_SelectStand_0_8, heroFUNC_SelectStand_0_9, heroFUNC_SelectStand_0_10]
        };
        for (i, o) in [0xa4, 0xb4, 0xc4].into_iter().enumerate() {
            let v = k(e, s[i]);
            e.w32(param_1 + o, v);
        }
        if e.r32(GB0(e) + 0x14) & 0x10_0000 != 0 && param_3 != 0 {
            frq = 5;
        }
        let r = LIBAnimMngr::SelectAnimFrq(e, param_1 + 600, frq, (param_1 + 0xa4) as i32, 1);
        Ok(r as i64)
    } else {
        heroFUNC_StartBlend(e, f32::from_bits(0x3e19999a), 2, 2, 0)
    }
}

/// 0x0002ccc0 (524 bytes) `undefined8 heroFUNC_SelectSleep(int param_1,uint param_2)`
/// The original is a compare tree over 4 distinct case values; the cases are tested in sequence
/// here (same result: the values are distinct and each sits on the branch the tree tests it on).
/// Group 2 (2b32b17e) has the 0x22/0x21 case (slots _2_) instead of the reference's third one.
pub fn heroFUNC_SelectSleep(e: &mut Engine, param_1: i32, param_2: u32) -> LResult<u64> {
    debug_info(e, "FUNC_SelectSleep");
    let gi = g(e, G_heroFUNC_SelectSleep);
    let kk = |i| k(e, i);
    let mut cases = vec![
        (kk(heroFUNC_SelectSleep_0_1), kk(heroFUNC_SelectSleep_0_2), kk(heroFUNC_SelectSleep_0_3), kk(heroFUNC_SelectSleep_0_4)),
        (kk(heroFUNC_SelectSleep_0_6), kk(heroFUNC_SelectSleep_0_7), 2, 0),
    ];
    if gi == 2 {
        cases.push((kk(heroFUNC_SelectSleep_0_12), kk(heroFUNC_SelectSleep_0_13), kk(heroFUNC_SelectSleep_0_14), kk(heroFUNC_SelectSleep_0_15)));
        cases.push((kk(heroFUNC_SelectSleep_2_1), kk(heroFUNC_SelectSleep_2_2), 2, 0));
    } else {
        cases.push((kk(heroFUNC_SelectSleep_0_8), kk(heroFUNC_SelectSleep_0_9), kk(heroFUNC_SelectSleep_0_10), kk(heroFUNC_SelectSleep_0_11)));
        cases.push((kk(heroFUNC_SelectSleep_0_12), kk(heroFUNC_SelectSleep_0_13), kk(heroFUNC_SelectSleep_0_14), kk(heroFUNC_SelectSleep_0_15)));
    }
    for (v, anim, a, b) in cases {
        if param_2 == v {
            let r = heroFUNC_StartAnim(e, f32::from_bits(0x3d4ccccd), anim as i32, a as i32, b as i32)?;
            return Ok(r as u64);
        }
    }
    let s = if e.r32(GB0(e) + 0x14) & 0x10_0000 == 0 {
        k(e, heroFUNC_SelectSleep_0_16)
    } else {
        k(e, heroFUNC_SelectSleep_0_17)
    };
    heroFUNC_SeekAnim(e, s, 7, param_1 + 0xa4, 1)
}

/// 0x0002ced0 (380 bytes) `undefined8 heroFUNC_SeekAnim(uint param_1,uint param_2,int param_3,undefined4 param_4)`
/// Table of 16-byte records {anim, time(f32), -, flags}. Not found: index param_2 (one past the
/// end) is read anyway, like the original.
pub fn heroFUNC_SeekAnim(e: &mut Engine, param_1: u32, param_2: u32, param_3: i32, param_4: u32) -> LResult<u64> {
    let args = e.alloca(0x10); // uStack_50.. spilled params, DebugTime reads the first
    e.w32(args, param_1);
    e.w32(args + 4, param_2);
    e.w32(args + 8, param_3 as u32);
    e.w32(args + 12, param_4);
    debug_time(e, "SeekAnim", args, 1)?;
    let tab = param_3 as u32;
    let mut i = 0u32;
    while i < param_2 && param_1 != e.r32(i * 0x10 + tab) {
        i += 1;
    }
    let rec = i.wrapping_mul(0x10).wrapping_add(tab);
    debug_info_value(e, "FUNC_SeekAnim - No", rec, 1);
    let t = e.rf(rec + 4);
    let anim = e.r32(rec);
    let fl = e.r32(rec + 8);
    let r = heroFUNC_StartAnim(e, t, anim as i32, param_4 as i32, fl as i32)?;
    Ok(r as u64)
}

/// 0x0002d050 (272 bytes) `void heroInit_Stand(int param_1)`
pub fn heroInit_Stand(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    APIextra::TimerReset(e, e.r32(param_1 + 0x208));
    let a = heroFUNC_SelectScoobyRun2Stand(e, param_1)?;
    e.w32(param_1 + 0x218, a as i32 as u32);
    or_gb14(e, 0x10);
    and_gb14(e, 0xffff_ffdf);
    APIgameobj::SetContextFlag(e, SELF, 0x1001_0050);
    APIinput::InputConfigSetState(e, e.r32(param_1 + 0x188) as i32, -0x7fff_fd22);
    e.w32(param_1 + 0x158, addr::heroST_Stand_Loop);
    Ok(())
}

/// Scooby builds go to heroInit_Sneak_In, Shaggy builds (group 1, no Sneak_In) to heroInit_Sneak.
fn sneak_entry(e: &Engine, gi: usize) -> u32 {
    if g(e, gi) == 0 { addr::heroInit_Sneak_In } else { addr::heroInit_Sneak }
}

/// 0x0002d160 (312 bytes) `void heroST_Stand_Loop(int param_1)`
pub fn heroST_Stand_Loop(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    let f16c = e.r32(param_1 + 0x16c);
    if !f16c & 6 == 0 {
        e.w32(param_1 + 0x154, addr::heroInit_Sneaking);
    } else if f16c & 2 != 0 {
        e.w32(param_1 + 0x154, addr::heroInit_Move);
    } else if f16c & 4 != 0 {
        let s = sneak_entry(e, G_heroST_Stand_Loop);
        e.w32(param_1 + 0x154, s);
    } else if e.r32(param_1 + 0x168) & 1 != 0 {
        e.w32(param_1 + 0x154, addr::heroInit_Stand);
    } else if e.r32(GB0(e) + 0x14) & 0x40_0000 == 0 && IsOutOfTime(e)? != 0 {
        e.w32(param_1 + 0x154, addr::heroInit_Idle);
    }
    Ok(())
}

/// 0x0002d2b0 (1720 bytes) `void heroInit_Idle(int param_1)`
pub fn heroInit_Idle(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    if e.r32(GB0(e) + 0x14) & 0x80 == 0 {
        let early = e.r32(GB0(e) + 0x14) & 2 != 0
            || APIextra::TimerGetTime(e, e.r32(param_1 + 0x208)) <= 45.0;
        if early {
            let a = heroFUNC_SelectStand(e, param_1, e.r32(param_1 + 0x218) as i32, 1)?;
            e.w32(param_1 + 0x218, a as i32 as u32);
            let f = LIBApiCFunctions::FloatRandom(e, 8.0, 12.0, 0.1);
            UpdateTimerOutOfTime(e, f)?;
        } else {
            or_gb14(e, 0x80);
            let a = heroFUNC_SelectSleep(e, param_1 as i32, e.r32(param_1 + 0x218))?;
            e.w32(param_1 + 0x218, a as i32 as u32);
            e.w32(param_1 + 0x15c, addr::heroExit_Idle_Loop);
        }
    } else {
        let a = heroFUNC_SelectSleep(e, param_1 as i32, e.r32(param_1 + 0x218))?;
        e.w32(param_1 + 0x218, a as i32 as u32);
        e.w32(param_1 + 0x15c, addr::heroExit_Idle_Loop);
        if e.r32(param_1 + 0x218) == kneg(e, heroInit_Idle_0_1) {
            let f = LIBApiCFunctions::FloatRandom(e, 20.0, 30.0, 0.1);
            UpdateTimerOutOfTime(e, f)?;
        }
    }
    if e.r32(GB0(e) + 0x34) & 0x101 == 0 {
        let cur = e.r32(param_1 + 0x218);
        let ev: Option<(&str, u32)> = if cur == e.r32(param_1 + 0xd4) {
            Some(("----event_CTRL_IDLE_DUO_1st", 0x100_0000))
        } else if cur == e.r32(param_1 + 0xe4) {
            Some(("----event_CTRL_IDLE_DUO_2nd", 0x200_0000))
        } else if cur == e.r32(param_1 + 0xf4) {
            Some(("----event_CTRL_IDLE_SLEEP_SOLO_IN", 0x400_0000))
        } else if cur == kneg(e, heroInit_Idle_0_2) {
            Some(("----event_CTRL_IDLE_SLEEP_TEAM_IN", 0x800_0000))
        } else if cur == kneg(e, heroInit_Idle_0_3) {
            Some(("----event_CTRL_IDLE_SLEEP_TEAM", 0x1000_0000))
        } else if cur == kneg(e, heroInit_Idle_0_4) {
            Some(("----event_CTRL_IDLE_SLEEP_TEAM_VAR", 0x2000_0000))
        } else {
            None
        };
        if let Some((msg, bit)) = ev {
            debug_info(e, msg);
            let p = e.r32(param_1 + 0x180);
            APIgameobj::SendMessageTo(e, p, 0, 0x60, 0x2_0000, 0);
            APIgameobj::SendMessageTo(e, p, 0, 0x62, bit, 0);
        }
    }
    // release builds have no print at all (debug_info_value checks the build flavour)
    if e.r32(param_1 + 0x218) != kneg(e, heroInit_Idle_0_5) {
        debug_info_value(e, "Init_Idle - stand No", param_1 + 0x218, 1);
    }
    APIgameobj::SetContextFlag(e, SELF, 0x1000_0050);
    APIinput::InputConfigSetState(e, e.r32(param_1 + 0x188) as i32, -0x7fff_fd22);
    e.w32(param_1 + 0x158, addr::heroST_Idle_Loop);
    Ok(())
}

/// 0x0002d968 (684 bytes) `void heroST_Idle_Loop(int param_1)`
pub fn heroST_Idle_Loop(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    let f16c = e.r32(param_1 + 0x16c);
    let f168 = e.r32(param_1 + 0x168);
    if !f16c & 6 == 0 {
        e.w32(param_1 + 0x154, addr::heroInit_Sneaking);
    } else if f16c & 2 != 0 {
        APIgameobj::SendMessageTo(e, e.r32(param_1 + 0x180), 0, 0x62, 0x4000_0000, 0);
        e.w32(param_1 + 0x154, addr::heroInit_Move);
    } else if f16c & 4 != 0 {
        let s = sneak_entry(e, G_heroST_Idle_Loop);
        e.w32(param_1 + 0x154, s);
    } else if f168 & 1 != 0 {
        e.w32(param_1 + 0x154, addr::heroInit_Idle);
    } else if f168 & 8 != 0 {
        // the "wake up" latent object, placed on the hero
        let o = APIgameobj::LaunchLatant(e, k(e, ID_2a800094));
        e.w32(param_1 + 0x264, o);
        let v = e.alloca(0x10);
        let p = APIgameobj::GetPosition(e, SELF, 0x7fff_ffff, v);
        APIgameobj::SetTranslation(e, e.r32(param_1 + 0x264), p);
        APIgameobj::TakeOrientation(e, e.r32(param_1 + 0x264), SELF, 0x7fff_ffff);
        e.w32(param_1 + 0x15c, addr::heroExit_Idle_Loop);
    } else if e.r32(GB0(e) + 0x14) & 0x40_0000 != 0 {
        e.w32(param_1 + 0x154, addr::heroInit_Stand);
    } else if e.r32(GB0(e) + 0x14) & 0x80 == 0 {
        if IsOutOfTime(e)? != 0 {
            e.w32(param_1 + 0x154, addr::heroInit_Idle);
        }
    } else if e.r32(param_1 + 0x218) == kneg(e, heroST_Idle_Loop_0_1) && IsOutOfTime(e)? != 0 {
        e.w32(param_1 + 0x154, addr::heroInit_Idle);
    }
    Ok(())
}

/// 0x0002a158 (220 bytes) `void heroExit_Idle_Loop(int param_1)` (same code in all builds)
pub fn heroExit_Idle_Loop(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    let o = e.r32(param_1 + 0x264);
    if o != 0 && APIgameobj::GetID(e, o, 0) != 0x7fff_fffe {
        APIgameobj::SendMessageTo(e, e.r32(param_1 + 0x264), 0, 0xd4, 0, 0);
    }
    if e.r32(param_1 + 0x154) != addr::heroInit_Idle {
        and_gb14(e, 0xffff_ff7f);
    }
    Ok(())
}

/// 0x0002dd20 (280 bytes) `void heroInit_Sneak_In(int param_1)` (Scooby builds only)
pub fn heroInit_Sneak_In(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    let a = heroFUNC_StartAnim(e, f32::from_bits(0x3dcccccd), k(e, heroInit_Sneak_In_0_1) as i32, 1, 5)?;
    e.w32(param_1 + 0x218, a as i32 as u32);
    or_gb14(e, 0x30);
    APIgameobj::SetContextFlag(e, SELF, 0x1001_0050);
    APIinput::InputConfigSetState(e, e.r32(param_1 + 0x188) as i32, -0x7fff_fd22);
    APIgameobj::SendMessageTo(e, k(e, ID_2a000096), 0, 0x5e, 1, 0);
    e.w32(param_1 + 0x158, addr::heroST_Sneak_In_Loop);
    e.w32(param_1 + 0x15c, addr::heroExit_Sneak);
    Ok(())
}

/// Stop the sneak loop sound (local+0x21c) on the partner and tell it the sneak ended.
fn stop_sneak(e: &mut Engine, param_1: Ptr) {
    e.w32(param_1 + 0x268, 0);
    let partner = k(e, ID_2a000096);
    if e.r32(param_1 + 0x21c) != u32::MAX {
        APInewSound::StopSound(e, partner, e.r32(param_1 + 0x21c));
    }
    e.w32(param_1 + 0x21c, u32::MAX);
    APIgameobj::SendMessageTo(e, partner, 0, 0x5e, 0, 0);
}

/// 0x0002de38 (328 bytes) `void heroST_Sneak_In_Loop(int param_1)`
pub fn heroST_Sneak_In_Loop(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    let f = e.r32(param_1 + 0x16c);
    if !f & 6 == 0 {
        e.w32(param_1 + 0x154, addr::heroInit_Sneaking);
    } else if f & 4 == 0 && f & 2 != 0 {
        stop_sneak(e, param_1);
        e.w32(param_1 + 0x154, addr::heroInit_Move);
    } else if e.r32(param_1 + 0x168) & 1 != 0 {
        e.w32(param_1 + 0x154, addr::heroInit_Sneak);
    }
    Ok(())
}

/// 0x0002df90 (200 bytes) `void heroInit_Sneak_Out(int param_1)`
pub fn heroInit_Sneak_Out(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    let a = heroFUNC_StartAnim(e, f32::from_bits(0x3dcccccd), k(e, heroInit_Sneak_Out_0_1) as i32, 1, 5)?;
    e.w32(param_1 + 0x218, a as i32 as u32);
    APIgameobj::SetContextFlag(e, SELF, 0x1001_0050);
    APIinput::InputConfigSetState(e, e.r32(param_1 + 0x188) as i32, -0x7fff_fd22);
    e.w32(param_1 + 0x158, addr::heroST_Sneak_Out_Loop);
    e.w32(param_1 + 0x15c, addr::heroExit_Sneak);
    Ok(())
}

/// 0x0002e058 (144 bytes) `void heroST_Sneak_Out_Loop(int param_1)`
pub fn heroST_Sneak_Out_Loop(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    let f = e.r32(param_1 + 0x16c);
    if f & 4 == 0 && f & 2 != 0 {
        e.w32(param_1 + 0x154, addr::heroInit_Move);
    } else if e.r32(param_1 + 0x168) & 1 != 0 {
        e.w32(param_1 + 0x154, addr::heroInit_Stand);
    }
    Ok(())
}

/// 0x0002e0f0 (276 bytes) `void heroInit_Sneak(int param_1)`
pub fn heroInit_Sneak(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    let a = heroFUNC_StartBlend(e, 0.25, 3, 2, 0)?;
    e.w32(param_1 + 0x218, a as i32 as u32);
    or_gb14(e, 0x30);
    APIgameobj::SetContextFlag(e, SELF, 0x1001_0050);
    APIinput::InputConfigSetState(e, e.r32(param_1 + 0x188) as i32, -0x7fff_fd22);
    APIgameobj::SendMessageTo(e, k(e, ID_2a000096), 0, 0x5e, 1, 0);
    e.w32(param_1 + 0x158, addr::heroST_Sneak_Loop);
    e.w32(param_1 + 0x15c, addr::heroExit_Sneak);
    Ok(())
}

/// 0x0002e208 (264 bytes) `void heroST_Sneak_Loop(int param_1)`
/// Leaving the sneak: Scooby builds go through heroInit_Sneak_Out, Shaggy builds straight to Stand.
pub fn heroST_Sneak_Loop(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    let f = e.r32(param_1 + 0x16c);
    if !f & 6 == 0 {
        e.w32(param_1 + 0x154, addr::heroInit_Sneaking);
    } else if f & 4 == 0 {
        stop_sneak(e, param_1);
        let n = if g(e, G_heroST_Sneak_Loop) == 0 { addr::heroInit_Sneak_Out } else { addr::heroInit_Stand };
        e.w32(param_1 + 0x154, n);
    }
    Ok(())
}

/// 0x0002a830 (136 bytes) `void heroExit_Sneak(int param_1)` (same code in all builds)
pub fn heroExit_Sneak(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    let n = e.r32(param_1 + 0x154);
    if n != addr::heroInit_Sneak && n != addr::heroInit_Sneaking {
        APIgameobj::SendMessageTo(e, k(e, ID_2a000096), 0, 0x5e, 0, 0);
    }
    Ok(())
}

/// 0x0002e3a0 (440 bytes) `void heroInit_Busy(int param_1)`
pub fn heroInit_Busy(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    let f = e.r32(param_1 + 0x164);
    if f & 0x2_0000 != 0 {
        UpdateTimerOutOfTime(e, 0.25)?;
    } else if f & 0x8_0000 != 0 {
        UpdateTimerOutOfTime(e, 999.9)?;
    } else {
        UpdateTimerOutOfTime(e, 999.9)?;
        let a = heroFUNC_StartBlend(e, 0.0, 2, 2, 0)?;
        e.w32(param_1 + 0x218, a as i32 as u32);
    }
    APIphysic::SetVelocity(e, SELF, 0.0, 0.0, 0.0);
    APIgameobj::SetContextFlag(e, SELF, 0);
    APIinput::InputConfigSetState(e, e.r32(param_1 + 0x188) as i32, i32::MIN);
    if e.r32(GB0(e) + 8) == e.r32(L(e) + 0x17c) {
        or_gb14(e, 0x100_0000);
    }
    e.w32(param_1 + 0x158, addr::heroST_Busy_Loop);
    e.w32(param_1 + 0x15c, addr::heroExit_Busy);
    Ok(())
}

/// 0x0002e558 (180 bytes) `void heroST_Busy_Loop(int param_1)`
pub fn heroST_Busy_Loop(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    APIphysic::SetVelocity(e, SELF, 0.0, 0.0, 0.0);
    if IsOutOfTime(e)? != 0 {
        UpdateTimerOutOfTime(e, 999.9)?;
        let a = heroFUNC_StartBlend(e, 0.0, 2, 2, 0)?;
        e.w32(param_1 + 0x218, a as i32 as u32);
    }
    Ok(())
}

/// 0x0002ab28 (116 bytes) `void heroExit_Busy(void)` (same code in all builds)
pub fn heroExit_Busy(e: &mut Engine) -> LResult<()> {
    if e.r32(GB0(e) + 8) == e.r32(L(e) + 0x17c) {
        and_gb14(e, 0xfeff_ffff);
    }
    Ok(())
}

/// 0x0002e688 (200 bytes) `void heroSet_Leader(int param_1)`
pub fn heroSet_Leader(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    debug_time(e, "heroSet_Leader", 0, 0)?;
    let orp = e.alloca(0x20); // auStack_50[8], only [0] = 1 set
    e.w32(orp, 1);
    APIgameobj::SendMessageTo(e, 0xff00_0001, orp, 0xcd, e.r32(param_1 + 0x17c), 0);
    APIinput::InputConfigActivate(e, e.r32(param_1 + 0x188) as i32, 1);
    APIphysic::SetPriority(e, SELF, 4);
    Ok(())
}

/// 0x0002e750 (608 bytes) `long heroFUNC_SelectScoobyRun2Stand(int param_1)`
/// Shaggy builds (group 5) only do the blend branch (no IntRandom / floor read).
/// The "no special stand after these anims" test differs per group (see below).
pub fn heroFUNC_SelectScoobyRun2Stand(e: &mut Engine, param_1: Ptr) -> LResult<i64> {
    let gi = g(e, G_heroFUNC_SelectScoobyRun2Stand);
    let mut special = false;
    if gi != 5 {
        let v = e.r32(param_1 + 0x218);
        let r = LIBApiCFunctions::IntRandom(e, 0, 10);
        let attribs = e.alloca(0x20); // EnvPrimAttribs aEStack_60; uStack_58 = +8
        APIphysic::GetFloorAttribs(e, SELF, attribs);
        let kk = |i| k(e, i);
        let skip = match gi {
            0 => {
                if v < kk(heroFUNC_SelectScoobyRun2Stand_0_1) {
                    kk(heroFUNC_SelectScoobyRun2Stand_0_2) < v
                        || v == kk(heroFUNC_SelectScoobyRun2Stand_0_3)
                        || v == kk(heroFUNC_SelectScoobyRun2Stand_0_4)
                } else {
                    v == kk(heroFUNC_SelectScoobyRun2Stand_0_5)
                }
            }
            1 => {
                v == kk(heroFUNC_SelectScoobyRun2Stand_1_1)
                    || if v < kk(heroFUNC_SelectScoobyRun2Stand_0_1) {
                        v == kk(heroFUNC_SelectScoobyRun2Stand_0_3) || v == kk(heroFUNC_SelectScoobyRun2Stand_0_4)
                    } else {
                        v < kk(heroFUNC_SelectScoobyRun2Stand_0_5) && kk(heroFUNC_SelectScoobyRun2Stand_1_2) < v
                    }
            }
            2 => [
                heroFUNC_SelectScoobyRun2Stand_0_2,
                heroFUNC_SelectScoobyRun2Stand_0_3,
                heroFUNC_SelectScoobyRun2Stand_0_4,
                heroFUNC_SelectScoobyRun2Stand_2_1,
                heroFUNC_SelectScoobyRun2Stand_2_2,
            ]
            .iter()
            .any(|&i| v == kk(i)),
            // single-build groups (5257b080, aae74b1e): their literals hard-coded
            3 => v == 0x15 || if v < 0x16 { v < 6 && 3 < v } else { v == 0x20 || v == 0x40 },
            4 => v == 0xc || if v < 0xd { v < 3 && v != 0 } else { v == 0x17 || v == 0x35 },
            _ => unported_shape(e, "heroFUNC_SelectScoobyRun2Stand", G_heroFUNC_SelectScoobyRun2Stand),
        };
        if !skip && r < 8 && e.r32(attribs + 8) & 1 == 0 {
            special = true;
        }
        if special {
            let anim = if r < 4 && e.r32(GB0(e) + 0x14) & 2 == 0 {
                let a = k(e, heroFUNC_SelectScoobyRun2Stand_0_6);
                APIanim::SetTime(e, SELF, k(e, heroFUNC_SelectScoobyRun2Stand_0_7), 0.2);
                a
            } else {
                k(e, heroFUNC_SelectScoobyRun2Stand_0_8)
            };
            UpdateTimerOutOfTime(e, 999.9)?;
            let a = heroFUNC_StartAnim(e, f32::from_bits(0x3dcccccd), anim as i32, 1, 5)?;
            e.w32(param_1 + 0x218, a as i32 as u32);
            return Ok(anim as i32 as i64);
        }
    }
    let f = LIBApiCFunctions::FloatRandom(e, 8.0, 12.0, 0.1);
    UpdateTimerOutOfTime(e, f)?;
    heroFUNC_StartBlend(e, f32::from_bits(0x3e2b020c), 2, 2, 0)
}

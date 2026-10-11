//! hero_stands.c (16 functions). Addresses are module-relative (.text at 0x10000) in scooby_ghost__08a011d0.
use super::builds::*;
use super::*;
use crate::engine::*;
use crate::logic::LResult;

/// 0x000263d0 (488 bytes) `long heroFUNC_SelectStand(int param_1,int param_2,int param_3)`
/// The three stand triples (anim ids written to param_1+0xa4/0xb4/0xc4, picked by the fear meter)
/// are the same in every build.
pub fn heroFUNC_SelectStand(e: &mut Engine, param_1: Ptr, param_2: i32, param_3: i32) -> LResult<i64> {
    let mut frq: u32 = 3;
    if param_2 == 2 {
        let m = e.rf(e.r32(GD(e) + 4));
        let s: [u32; 3] = if m < 0.25 {
            [0xfffff83d, 0xfffff83e, 0xfffff83f]
        } else if e.rf(e.r32(GD(e) + 4)) < 0.75 {
            [0xfffff839, 0xfffff83a, 0xfffff83b]
        } else {
            [0xfffff831, 0xfffff832, 0xfffff833]
        };
        for (i, o) in [0xa4, 0xb4, 0xc4].into_iter().enumerate() {
            e.w32(param_1 + o, s[i]);
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

/// 0x000265b8 (556 bytes) `undefined8 heroFUNC_SelectSleep(int param_1,uint param_2)`
pub fn heroFUNC_SelectSleep(e: &mut Engine, param_1: i32, param_2: u32) -> LResult<u64> {
    // compare tree over 4 distinct values, tested in sequence here (same result)
    debug_info(e, "FUNC_SelectSleep");
    for (v, anim, a, b) in [
        (0xfffff837u32, 0xfffff838u32, 1, 5),
        (0xfffff836, 0xfffff837, 2, 0),
        (0xfffff838, 0xfffff837, 2, 0),
        (0xfffff841, 0xfffff842, 2, 0),
    ] {
        if param_2 == v {
            let r = heroFUNC_StartAnim(e, f32::from_bits(0x3d4ccccd), anim as i32, a, b)?;
            return Ok(r as u64);
        }
    }
    let s: u32 = if e.r32(GB0(e) + 0x14) & 0x10_0000 == 0 { 0xfffff841 } else { 0xfffff836 };
    heroFUNC_SeekAnim(e, s, 7, param_1 + 0xa4, 1)
}

/// 0x000267e8 (380 bytes) `undefined8 heroFUNC_SeekAnim(uint param_1,uint param_2,int param_3,undefined4 param_4)`
pub fn heroFUNC_SeekAnim(e: &mut Engine, param_1: u32, param_2: u32, param_3: i32, param_4: u32) -> LResult<u64> {
    // table of 16-byte records {anim, time(f32), flags, -}; not found: index param_2 is read anyway
    let args = e.alloca(0x10); // uStack_50.. spilled params, DebugTime reads the first
    e.w32(args, param_1);
    e.w32(args + 4, param_2);
    e.w32(args + 8, param_3 as u32);
    e.w32(args + 12, param_4);
    debug_time(e, "SeekAnim", args, 1)?;
    let tab = param_3 as u32;
    let mut i = 0u32;
    while i < param_2 && param_1 != e.r32(i.wrapping_mul(0x10).wrapping_add(tab)) {
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

/// 0x00026968 (272 bytes) `void heroInit_Stand(int param_1)`
pub fn heroInit_Stand(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    APIextra::TimerReset(e, e.r32(param_1 + 0x208));
    let a = heroFUNC_SelectScoobyRun2Stand(e)?;
    e.w32(param_1 + 0x218, a as i32 as u32);
    or_gb14(e, 0x10);
    and_gb14(e, 0xffff_ffdf);
    APIgameobj::SetContextFlag(e, SELF, 0x1001_0050);
    APIinput::InputConfigSetState(e, e.r32(param_1 + 0x188) as i32, -0x7fff_fd22);
    e.w32(param_1 + 0x158, addr::heroST_Stand_Loop);
    Ok(())
}

/// 0x00026a78 (312 bytes) `void heroST_Stand_Loop(int param_1)`
pub fn heroST_Stand_Loop(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    let f16c = e.r32(param_1 + 0x16c);
    if !f16c & 6 == 0 {
        e.w32(param_1 + 0x154, addr::heroInit_Sneaking);
    } else if f16c & 2 != 0 {
        e.w32(param_1 + 0x154, addr::heroInit_Move);
    } else if f16c & 4 != 0 {
        e.w32(param_1 + 0x154, addr::heroInit_Sneak);
    } else if e.r32(param_1 + 0x168) & 1 != 0 {
        e.w32(param_1 + 0x154, addr::heroInit_Stand);
    } else if e.r32(GB0(e) + 0x14) & 0x40_0000 == 0 && IsOutOfTime(e)? != 0 {
        e.w32(param_1 + 0x154, addr::heroInit_Idle);
    }
    Ok(())
}

/// 0x00026bc8 (1736 bytes) `void heroInit_Idle(int param_1)`
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
        if e.r32(param_1 + 0x218) == (-0x7c9i32) as u32 {
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
        } else if cur == (-0x7cai32) as u32 {
            Some(("----event_CTRL_IDLE_SLEEP_TEAM_IN", 0x800_0000))
        } else if cur == (-0x7c9i32) as u32 {
            Some(("----event_CTRL_IDLE_SLEEP_TEAM", 0x1000_0000))
        } else if cur == (-0x7c8i32) as u32 {
            Some(("----event_CTRL_IDLE_SLEEP_TEAM_VAR", 0x2000_0000))
        } else {
            None
        };
        if let Some((msg, bit)) = ev {
            debug_info(e, msg);
            APIgameobj::SendMessageTo(e, e.r32(param_1 + 0x180), 0, 0x60, 0x2_0000, 0);
            APIgameobj::SendMessageTo(e, e.r32(param_1 + 0x180), 0, 0x62, bit, 0);
        }
    }
    if e.r32(param_1 + 0x218) != 2 {
        debug_info_value(e, "Init_Idle - stand No", param_1 + 0x218, 1);
    }
    APIgameobj::SetContextFlag(e, SELF, 0x1000_0050);
    APIinput::InputConfigSetState(e, e.r32(param_1 + 0x188) as i32, -0x7fff_fd22);
    e.w32(param_1 + 0x158, addr::heroST_Idle_Loop);
    Ok(())
}

/// 0x00027290 (688 bytes) `void heroST_Idle_Loop(int param_1)`
pub fn heroST_Idle_Loop(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    let f16c = e.r32(param_1 + 0x16c);
    if !f16c & 6 == 0 {
        e.w32(param_1 + 0x154, addr::heroInit_Sneaking);
    } else if f16c & 2 != 0 {
        APIgameobj::SendMessageTo(e, e.r32(param_1 + 0x180), 0, 0x62, 0x4000_0000, 0);
        e.w32(param_1 + 0x154, addr::heroInit_Move);
    } else if f16c & 4 != 0 {
        e.w32(param_1 + 0x154, addr::heroInit_Sneak);
    } else if e.r32(param_1 + 0x168) & 1 != 0 {
        e.w32(param_1 + 0x154, addr::heroInit_Idle);
    } else if e.r32(param_1 + 0x168) & 8 != 0 {
        // the "wake up" latent object, placed on the hero
        let o = APIgameobj::LaunchLatant(e, k(e, ID_2a800094));
        e.w32(param_1 + 0x264, o);
        let v = e.alloca(0x10);
        let p = APIgameobj::GetPosition(e, SELF, ALL, v);
        APIgameobj::SetTranslation(e, e.r32(param_1 + 0x264), p);
        APIgameobj::TakeOrientation(e, e.r32(param_1 + 0x264), SELF, ALL);
        e.w32(param_1 + 0x15c, addr::heroExit_Idle_Loop);
    } else if e.r32(GB0(e) + 0x14) & 0x40_0000 != 0 {
        e.w32(param_1 + 0x154, addr::heroInit_Stand);
    } else if e.r32(GB0(e) + 0x14) & 0x80 == 0 {
        if IsOutOfTime(e)? != 0 {
            e.w32(param_1 + 0x154, addr::heroInit_Idle);
        }
    } else if e.r32(param_1 + 0x218) == (-0x7c9i32) as u32 && IsOutOfTime(e)? != 0 {
        e.w32(param_1 + 0x154, addr::heroInit_Idle);
    }
    Ok(())
}

/// 0x00027568 (220 bytes) `void heroExit_Idle_Loop(int param_1)`
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

/// 0x00027648 (276 bytes) `void heroInit_Sneak(int param_1)`
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

/// 0x00027760 (264 bytes) `void heroST_Sneak_Loop(int param_1)`
pub fn heroST_Sneak_Loop(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    let f = e.r32(param_1 + 0x16c);
    if !f & 6 == 0 {
        e.w32(param_1 + 0x154, addr::heroInit_Sneaking);
    } else if f & 4 == 0 {
        e.w32(param_1 + 0x268, 0);
        if e.r32(param_1 + 0x21c) != u32::MAX {
            APInewSound::StopSound(e, k(e, ID_2a000096), e.r32(param_1 + 0x21c));
        }
        e.w32(param_1 + 0x21c, u32::MAX);
        APIgameobj::SendMessageTo(e, k(e, ID_2a000096), 0, 0x5e, 0, 0);
        e.w32(param_1 + 0x154, addr::heroInit_Stand);
    }
    Ok(())
}

/// 0x00027870 (136 bytes) `void heroExit_Sneak(int param_1)`
pub fn heroExit_Sneak(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    let n = e.r32(param_1 + 0x154);
    if n != addr::heroInit_Sneak && n != addr::heroInit_Sneaking {
        APIgameobj::SendMessageTo(e, k(e, ID_2a000096), 0, 0x5e, 0, 0);
    }
    Ok(())
}

/// 0x000278f8 (440 bytes) `void heroInit_Busy(int param_1)`
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

/// 0x00027ab0 (180 bytes) `void heroST_Busy_Loop(int param_1)`
pub fn heroST_Busy_Loop(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    APIphysic::SetVelocity(e, SELF, 0.0, 0.0, 0.0);
    if IsOutOfTime(e)? != 0 {
        UpdateTimerOutOfTime(e, 999.9)?;
        let a = heroFUNC_StartBlend(e, 0.0, 2, 2, 0)?;
        e.w32(param_1 + 0x218, a as i32 as u32);
    }
    Ok(())
}

/// 0x00027b68 (116 bytes) `void heroExit_Busy(void)`
pub fn heroExit_Busy(e: &mut Engine) -> LResult<()> {
    if e.r32(GB0(e) + 8) == e.r32(L(e) + 0x17c) {
        and_gb14(e, 0xfeff_ffff);
    }
    Ok(())
}

/// 0x00027be0 (200 bytes) `void heroSet_Leader(int param_1)`
pub fn heroSet_Leader(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    debug_time(e, "heroSet_Leader", 0, 0)?;
    let orp = e.alloca(0x20); // auStack_50[8], only [0] = 1 set
    e.w32(orp, 1);
    APIgameobj::SendMessageTo(e, 0xff00_0001, orp, 0xcd, e.r32(param_1 + 0x17c), 0);
    APIinput::InputConfigActivate(e, e.r32(param_1 + 0x188) as i32, 1);
    APIphysic::SetPriority(e, SELF, 4);
    Ok(())
}

/// 0x00027ca8 (184 bytes) `long heroFUNC_SelectScoobyRun2Stand(void)`
pub fn heroFUNC_SelectScoobyRun2Stand(e: &mut Engine) -> LResult<i64> {
    let f = LIBApiCFunctions::FloatRandom(e, 8.0, 12.0, 0.1);
    UpdateTimerOutOfTime(e, f)?;
    heroFUNC_StartBlend(e, f32::from_bits(0x3e2b020c), 2, 2, 0)
}


// ---- helpers (from the hero port) ----

pub(crate) fn and_gb14(e: &mut Engine, v: u32) {
    let a = GB0(e) + 0x14;
    let x = e.r32(a);
    e.w32(a, x & v);
}

pub(crate) fn or_gb14(e: &mut Engine, v: u32) {
    let a = GB0(e) + 0x14;
    let x = e.r32(a);
    e.w32(a, x | v);
}

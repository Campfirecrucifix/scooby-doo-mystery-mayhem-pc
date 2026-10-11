//! panel_base.c (75 functions). Addresses are module-relative (.text at 0x10000) in panel__014a381f.
use super::builds::*;
use super::*;
use crate::engine::*;
use crate::logic::LResult;

/// 0x0001c440 (1276 bytes) `undefined4 panel_baseFUNC_Init(void)`
/// Shape group 1 (the 6 shifted-layout builds) creates its timers in another order into other fields.
pub fn panel_baseFUNC_Init(e: &mut Engine) -> LResult<u32> {
    LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 2, 0, 0);
    let l = LB(e);
    e.wf(l, 0.0);
    e.wf(l + 4, 0.0);
    e.wf(l + 8, 1.0);
    let f = GBP(e, 3);
    let v = e.r32(f) & 0xfffd_ffff;
    e.w32(f, v);
    let lang = LIBMiscHUD::miscHUDFUNC_GetLanguage(e, e.r32(GBP(e, 2) + 0x24));
    e.wf(lb(e, 0x46c), lang as f32); // stored as a float, as the original does
    e.w32(lb(e, 0x418), 0xffff_ffff);
    e.w32(lb(e, 0x41c), 1);
    e.w32(lb(e, 0x88), 0);
    e.w32(lb(e, 0x460), 1);
    e.w32(lb(e, 0x45c), 1);
    e.w32(lb(e, 0x30), 1);
    e.w32(lb(e, 0x3c), 1);
    let tod = lit(e, "TOD_Normal_Meter");
    e.w32(lb(e, 0x58), tod);
    e.w32(lb(e, 0x54), tod);
    e.w32(lb(e, 0x474), 3);
    e.w32(lb(e, 0x44c), 99);
    e.w32(lb(e, 0x494), 99);
    e.w32(lb(e, 0x2c), 0);
    e.w32(lb(e, 0x450), 1);
    e.w32(lb(e, 0x484), 0);
    let v = e.r32(lb(e, 0x468)) | 4;
    e.w32(lb(e, 0x468), v);
    e.wf(lb(e, 0x94), -0.25);
    e.w32(lb(e, 0x4b8), 406);
    panel_baseFUNC_CreateInput_Base(e)?;
    e.w32(lb(e, 0x48), 0);
    APIinput::InputEnableVibration(e, e.r32(GBP(e, 0) + 0x4c) as i32, e.r32(GBP(e, 2) + 0x10) as i32);
    APIgameobj::ResetLogicFlag(e, SELF, 0xe3a);
    // (raw build offsets, frequency-3 timer marked)
    let timers: [(u32, bool); 7] = match g(e, G_panel_baseFUNC_Init) {
        // Ghidra's pLocalData_BasePanel[n].x/y/z is a 12-byte vector: [0x54].z, [0x54].y, [0x62].y, [0x62].z,
        // [0x62].x, [0x55].x, [99].x
        0 => [(0x3f8, false), (0x3f4, false), (0x49c, false), (0x4a0, false), (0x498, false), (0x3fc, true), (0x4a4, false)],
        1 => [(0x3fc, false), (0x3f8, false), (0x4a0, false), (0x4a4, false), (0x49c, false), (0x400, true), (0x4a8, false)],
        _ => unported_shape(e, "panel_baseFUNC_Init", G_panel_baseFUNC_Init),
    };
    for (o, freq) in timers {
        let t = APIextra::TimerCreate(e);
        e.w32(LB(e) + o, t);
        if freq {
            APIextra::TimerSetFrequency(e, t, 3.0);
        }
    }
    let m = APImath::CMatCreateIdentity(e);
    e.w32(lb(e, 0x4c8), m);
    let c = e.rf(GBP(e, 1));
    e.wf(lb(e, 0x74), c);
    let a = -((1.0 - e.rf(lb(e, 0x74))) * 1.8 * 100.0) * 0.017453292;
    APIgameobj::SetRotation(e, k(e, panel_baseFUNC_Init_0_8), LB(e), a);
    panel_baseFUNC_TexFaderControl(e)?;
    if e.r32(GBP(e, 3)) & 1 != 0 {
        APIgameobj::Rotate(e, k(e, panel_baseFUNC_Init_0_9), LB(e), 0.0);
        panel_baseFUNC_ToDMeterChangeControl(e, 3)?;
    }
    APIinput::InputConfigActivate(e, 7, 1);
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 0x10, 0x1d);
    Ok(APIinput::InputErrorRegisterNotify(e, 0, SELF, 0x1f))
}

/// 0x0001c940 (844 bytes) `undefined8 panel_baseST_FIRSTMOVE(void)`
pub fn panel_baseST_FIRSTMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    panel_baseFUNC_CreateHudWins(e, img(e, 0x10000) as i32, lb(e, 0xfc) as i32, 0xbe)?;
    // the shifted-layout builds have one more window in the second table (it fills LB+0xfc)
    let n2 = if shifted(e) { 0x15 } else { 0x14 };
    panel_baseFUNC_CreateHudWins(e, img(e, 0x14740) as i32, (LB(e) + 0xac) as i32, n2)?;
    let o = GBP(e, 2);
    APIhud::WindowSetDims(e, e.r32(lb(e, 0x188)), e.rf(o + 0x30), e.rf(o + 0x34));
    APIhud::WindowSetDims(e, e.r32(lb(e, 400)), e.rf(o + 0x28), e.rf(o + 0x2c));
    APIhud::WindowSetDims(e, e.r32(lb(e, 0x18c)), e.rf(o + 0x38), e.rf(o + 0x3c));
    if e.r32(GBP(e, 0) + 0x38) != 0 {
        // DAT_00011230: first text-argument byte of window record 48 of the first table
        let b = e.r8(img(e, 0x11230)) as u32;
        APIhud::TextWindowSetText(e, e.r32(lb(e, 0x1bc)), 0x90, &[b, 0]);
    }
    panel_baseFUNC_SetCurrentEpisode(e)?;
    panel_baseFUNC_InitKeysStack(e)?;
    let w = APIhud::WindowAdd(e, 1, e.r32(lb(e, 0xfc)), 0x3080c, 1);
    e.w32(lb(e, 0x484), w);
    APIhud::WindowSetDims(e, w, 0.5, 0.5);
    APIhud::WindowSetPos(e, w, 0.0, 0.0);
    let txt = if e.r32(GBP(e, 2) + 0x20) == 0 { 0x55 } else { 0x56 };
    APIhud::TextWindowSetText(e, w, txt, &[1]);
    APIhud::TextWindowSetColor(e, w, 6);
    APIhud::WindowSetBackGround(e, w, 1, 0.7, 0.6);
    APIhud::WindowSetFlags(e, w, 0x100);
    Ok(0)
}

/// 0x0001cc90 (108 bytes) `undefined8 panel_baseST_IN_GAME_SET(void)`
pub fn panel_baseST_IN_GAME_SET(e: &mut Engine, _: u32) -> LResult<u64> {
    panel_baseFUNC_PauseOff(e)?;
    APIgameobj::SetNextState(e, SELF, 2);
    Ok(0)
}

/// 0x0001cd00 (1492 bytes) `undefined8 panel_baseST_IN_GAME(void)`
/// Shape groups: 0 reference; 1 (6 shifted boss-meter builds) also sets flag 0x200 when the object
/// notification starts and drives the boss meter (flags 0x100/0x200/0x400/0x800); 2 (bdb8bc5e) does not
/// test game +0x38 before choosing the notification. Group 1 stores the reset fields in another order
/// (same values). The axis vector (DAT_00019c68 etc.) is (0, 0, 1) in every build.
pub fn panel_baseST_IN_GAME(e: &mut Engine, _: u32) -> LResult<u64> {
    let grp = g(e, G_panel_baseST_IN_GAME);
    if grp > 2 {
        unported_shape(e, "panel_baseST_IN_GAME", G_panel_baseST_IN_GAME);
    }
    let axis = e.alloca(0x10);
    e.wf(axis, 0.0);
    e.wf(axis + 4, 0.0);
    e.wf(axis + 8, 1.0);
    let la = e.alloca(0x10);
    e.w32(lb(e, 0x4ac), 0);
    APIgameobj::GetLookAt(e, k(e, ID_2a0000ba), 0x7fff_ffff, la);
    APIgameobj::InvertTransformVector(e, k(e, ID_2a00001e), la, la);
    let ang = APImath::atan2f(e, e.rf(la + 8), e.rf(la));
    APIgameobj::SetRotation(e, k(e, panel_baseST_IN_GAME_0_2), axis, ang);
    let gf = e.r32(GBP(e, 0) + 0x14);
    if e.r32(LB(e) + 0x20) == 0 && (grp == 2 || e.r32(GBP(e, 0) + 0x38) == 0) {
        let cur = e.r32(lb(e, 0x474));
        let v = if cur != 3 && gf & 1 != 0 {
            Some(3)
        } else if cur != 1 && gf & 2 != 0 && gf & 1 == 0 {
            Some(1)
        } else if cur != 1 && gf & 0x40 != 0 {
            Some(4)
        } else if cur != 2 && gf & 0x43 == 0 && e.r32(LB(e) + 0x14) == 0 {
            Some(2)
        } else {
            None
        };
        if let Some(v) = v {
            e.w32(lb(e, 0x478), v);
        }
    }
    if e.rf(GBP(e, 1)) != e.rf(LB(e) + 0x74) && e.r32(LB(e) + 0x1c) == 0 {
        panel_baseFUNC_UpdateCoolMeter(e)?;
    }
    if e.r32(lb(e, 0x478)) == 0 {
        if e.r32(lb(e, 0x47c)) == 0 || e.r32(lb(e, 0x474)) == 0 {
            e.w32(LB(e) + 0x20, 0);
        } else {
            APIhud::WindowResetFlags(e, e.r32(LB(e) + 0xb0), 0x100);
            APIhud::WindowResetFlags(e, e.r32(LB(e) + 0xe0), 0x100);
            let (p1, p2) = (img(e, 0x147b8), img(e, 0x14c38));
            APIhud::WindowSetPos(e, e.r32(LB(e) + 0xb0), e.rf(p1), e.rf(p1 + 4));
            APIhud::WindowSetPos(e, e.r32(LB(e) + 0xe0), e.rf(p2), e.rf(p2 + 4));
            for (o, v) in [(0x474, 2), (0x478, 0), (0x454, 0), (0x47c, 0), (0x4b0, 0)] {
                e.w32(lb(e, o), v);
            }
            e.w32(LB(e) + 0x20, 0);
        }
    } else {
        panel_baseFUNC_UpdateMeters(e)?;
    }
    let f = |e: &Engine| e.r32(lb(e, 0x468));
    if f(e) & 1 == 0 || f(e) & 2 != 0 || f(e) & 8 != 0 {
        if f(e) & 2 != 0 && f(e) & 8 == 0 {
            panel_baseFUNC_InOut_ObjNotification(e, e.r32(lb(e, 0x48c)), 1)?;
        }
    } else {
        let mut v = f(e) & 0xffff_fffb;
        if grp == 1 {
            v |= 0x200;
        }
        e.w32(lb(e, 0x468), v);
        panel_baseFUNC_InOut_ObjNotification(e, e.r32(lb(e, 0x48c)), 0)?;
    }
    if f(e) & 0x10 == 0 || f(e) & 0x20 != 0 {
        if f(e) & 0x20 != 0 {
            panel_baseFUNC_InOut_ActNotification(e, e.r32(lb(e, 0x488)), 1)?;
        }
    } else {
        let v = f(e) & 0xffff_ffbf;
        e.w32(lb(e, 0x468), v);
        panel_baseFUNC_InOut_ActNotification(e, e.r32(lb(e, 0x488)), 0)?;
    }
    if grp == 1 {
        if f(e) & 0x100 == 0 || f(e) & 0x600 != 0 {
            if f(e) & 0x100 == 0 || f(e) & 0x200 != 0 {
                panel_baseFUNC_MoveBossMeter(e, 1)?;
            }
        } else {
            panel_baseFUNC_MoveBossMeter(e, 0)?;
        }
        if f(e) & 0x800 != 0 {
            panel_baseFUNC_UpdateBossMeter(e)?;
        }
    }
    Ok(0)
}

/// 0x0001d2d8 (84 bytes) `undefined8 panel_baseST_PAUSE_SET(void)`
pub fn panel_baseST_PAUSE_SET(e: &mut Engine, _: u32) -> LResult<u64> {
    panel_baseFUNC_PauseOn(e, 0)?;
    Ok(0)
}

/// 0x0001d330 (120 bytes) `undefined8 panel_baseST_PAUSE(void)`
pub fn panel_baseST_PAUSE(e: &mut Engine, _: u32) -> LResult<u64> {
    if e.r32(LB(e) + k(e, panel_baseST_PAUSE_0_1)) & 0x20 != 0 {
        let n = e.r32(LB(e) + k(e, panel_baseST_PAUSE_0_2));
        panel_baseFUNC_InOut_ActNotification(e, n, 1)?;
    }
    Ok(0)
}

/// 0x0001d3a8 (428 bytes) `undefined8 panel_baseST_SLIDE_IN(void)`
pub fn panel_baseST_SLIDE_IN(e: &mut Engine, _: u32) -> LResult<u64> {
    let t = APIextra::TimerGetTime(e, e.r32(lb(e, 0x3f4))) - e.rf(LB(e) + 0x7c);
    panel_baseFUNC_ComputeParabolicPosFromTime(e, 0.36, t)?;
    panel_baseFUNC_ComputeParabolicPosFromTime(e, 0.36, t)?;
    if 0.36 <= t {
        if e.r32(LB(e) + 0xc) == 0 {
            APIinput::InputConfigSetState(e, 7, 0x100);
        } else {
            APIinput::InputConfigSetState(e, 8, 0x192);
            APIgameobj::SetNextState(e, SELF, 4);
        }
    }
    if e.r32(LB(e) + 0x40) != 0 {
        panel_baseFUNC_PauseOff(e)?;
    }
    Ok(0)
}

/// 0x0001d558 (400 bytes) `undefined8 panel_baseST_SLIDE_OUT(void)`
pub fn panel_baseST_SLIDE_OUT(e: &mut Engine, _: u32) -> LResult<u64> {
    let t = APIextra::TimerGetTime(e, e.r32(lb(e, 0x3f4))) - e.rf(LB(e) + 0x7c);
    panel_baseFUNC_ComputeParabolicPosFromTime(e, 0.36, 0.36 - t)?;
    panel_baseFUNC_ComputeParabolicPosFromTime(e, 0.36, 0.36 - t)?;
    if 0.36 <= t && e.r32(LB(e) + 0xc) == 0 {
        APIinput::InputConfigSetState(e, 7, 0x200_0110);
        APIgameobj::SetNextState(e, SELF, 2);
    }
    Ok(0)
}

/// 0x0001d6e8 (384 bytes) `undefined8 panel_baseCB_TIMER_SAFEMODE(void)`
pub fn panel_baseCB_TIMER_SAFEMODE(e: &mut Engine, _: u32) -> LResult<u64> {
    if e.r32(LB(e) + k(e, panel_baseCB_TIMER_SAFEMODE_0_1)) == 2 {
        APIgraphics::DoStupidXboxShadowHack(e, 0);
        e.w32(GBP(e, 0) + 0x44, 0x22b);
        let s = lit(e, "INTRO");
        APIgame::LoadLevel(e, s, s, 0);
    } else {
        panel_baseFUNC_DrawNewMenu(e, 7, 9, 0xc)?;
        for s in [
            panel_baseCB_TIMER_SAFEMODE_0_2,
            panel_baseCB_TIMER_SAFEMODE_0_3,
            panel_baseCB_TIMER_SAFEMODE_0_4,
            panel_baseCB_TIMER_SAFEMODE_0_5,
        ] {
            let w = e.r32(LB(e) + k(e, s));
            APIhud::WindowResetFlags(e, w, 0x100);
        }
        APIinput::InputConfigSetState(e, 8, 0x192);
    }
    Ok(0)
}

/// 0x0001d868 (92 bytes) `undefined8 panel_baseCB_TIMER_CONTROLLER(void)`
pub fn panel_baseCB_TIMER_CONTROLLER(e: &mut Engine, _: u32) -> LResult<u64> {
    let w = e.r32(LB(e) + k(e, panel_baseCB_TIMER_CONTROLLER_0_1));
    APIhud::WindowSetFlags(e, w, 0x100);
    Ok(0)
}

/// 0x0001d8c8 (272 bytes) `undefined8 panel_baseCB_PAUSE_MENU_OPEN(void)`
pub fn panel_baseCB_PAUSE_MENU_OPEN(e: &mut Engine, _: u32) -> LResult<u64> {
    let l = LB(e);
    match g(e, G_panel_baseCB_PAUSE_MENU_OPEN) {
        0 => {
            if e.r32(l + 0x40) == 0
                && e.r32(l + 0x1c) == 0
                && 0.25 < APIextra::TimerGetTime(e, e.r32(l + k(e, panel_baseCB_PAUSE_MENU_OPEN_0_1)))
                && e.r32(l + k(e, panel_baseCB_PAUSE_MENU_OPEN_0_2)) == 0
            {
                e.w32(l + 0x2c, 1);
                APIextra::TimerReset(e, e.r32(l + k(e, panel_baseCB_PAUSE_MENU_OPEN_0_3)));
                APIgameobj::LeaveParent(e, SELF, 0);
                APIgameobj::SetNextState(e, SELF, 3);
            }
        }
        1 => {
            // ToD-boss levels 10..=12 (unsigned compare) take the other branch
            let lvl = e.r32(GBP(e, 0) + 0x3c);
            if lvl < 10 || 0xc < lvl {
                if e.r32(l + 0x40) == 0
                    && e.r32(l + 0x1c) == 0
                    && 0.25 < APIextra::TimerGetTime(e, e.r32(lb(e, 0x4a0)))
                    && e.r32(lb(e, 0x4a8)) == 0
                {
                    e.w32(l + 0x2c, 1);
                    APIextra::TimerReset(e, e.r32(lb(e, 0x4a0)));
                    APIgameobj::LeaveParent(e, SELF, 0);
                    APIgameobj::SetNextState(e, SELF, 3);
                }
            } else if (e.r32(lb(e, 0x424)) & 0xc == 0 || e.r32(l + 0x14) != 0)
                && e.r32(lb(e, 0x4a8)) == 0
                && e.r32(l + 0x40) == 0
            {
                e.w32(l + 0x2c, 1);
                e.w32(lb(e, 0x4a8), 1);
                APIgameobj::SendMessageTo(e, k(e, panel_baseCB_PAUSE_MENU_OPEN_1_1), 0, 1, 0, 0);
            }
        }
        _ => unported_shape(e, "panel_baseCB_PAUSE_MENU_OPEN", G_panel_baseCB_PAUSE_MENU_OPEN),
    }
    Ok(0)
}

/// 0x0001d9d8 (340 bytes) `undefined8 panel_baseCB_PAUSE_MENU_CLOSE(void)`
pub fn panel_baseCB_PAUSE_MENU_CLOSE(e: &mut Engine, _: u32) -> LResult<u64> {
    let l = LB(e);
    if e.r32(l + 0x18) == 1 {
        e.w32(l + 0x18, 0);
        APIinput::InputConfigSetState(e, 8, 0x196);
        let w = e.r32(l + k(e, panel_baseCB_PAUSE_MENU_CLOSE_0_1));
        APIhud::WindowSetFlags(e, w, 0x100);
    } else if e.r32(l + k(e, panel_baseCB_PAUSE_MENU_CLOSE_0_2)) == 7
        && 0.25 < APIextra::TimerGetTime(e, e.r32(l + k(e, panel_baseCB_PAUSE_MENU_CLOSE_0_3)))
    {
        APIextra::TimerReset(e, e.r32(l + k(e, panel_baseCB_PAUSE_MENU_CLOSE_0_4)));
        APIinput::InputConfigSetState(e, 8, 0);
        panel_baseFUNC_PlayPauseOUTAnims(e)?;
        e.w32(l + 0x2c, 0);
    }
    Ok(0)
}

/// 0x0001db30 (508 bytes) `undefined8 panel_baseCB_MENU_STICK_UPDOWN(int param_1)`
/// `a0` = stick input record (`param_1`, the y axis float at +4).
pub fn panel_baseCB_MENU_STICK_UPDOWN(e: &mut Engine, a0: u32) -> LResult<u64> {
    let l = LB(e);
    if e.r32(l + k(e, panel_baseCB_MENU_STICK_UPDOWN_0_1)) == 0x45 {
        panel_baseFUNC_EpFilesUPDOWN(e, a0)?;
    } else {
        APInewSound::Play(e, SELF, 10, 0);
        let y = e.rf(a0 + 4);
        if y < -0.15 {
            // ToD-boss menus skip entry 11 (10 -> 12 / 12 -> 10)
            if e.r32(GBP(e, 0) + 0x38) == 0 || e.r32(l + 0xa8) != 10 {
                let v = e.r32(l + 0xa8).wrapping_add(1);
                e.w32(l + 0xa8, v);
            } else {
                let v = e.r32(l + 0xa8).wrapping_add(2);
                e.w32(l + 0xa8, v);
            }
        } else if 0.15 < y {
            if e.r32(GBP(e, 0) + 0x38) == 0 || e.r32(l + 0xa8) != 0xc {
                let v = e.r32(l + 0xa8).wrapping_sub(1);
                e.w32(l + 0xa8, v);
            } else {
                let v = e.r32(l + 0xa8).wrapping_sub(2);
                e.w32(l + 0xa8, v);
            }
        }
        let a = e.r32(l + k(e, panel_baseCB_MENU_STICK_UPDOWN_0_2)) as i32;
        let b = e.r32(l + k(e, panel_baseCB_MENU_STICK_UPDOWN_0_3)) as i32;
        panel_baseFUNC_MenuRefresh(e, a, b)?;
    }
    Ok(0)
}

/// 0x0001dd30 (1216 bytes) `undefined8 panel_baseCB_MENU_STICK_LEFTRIGHT(float *param_1)`
/// `a0` = stick input record (`float *param_1`, x axis at +0).
/// Quirks kept: the debug level-select pages test `-0.15 < x` (not `0.15 < x`) to go right, so a centred
/// stick also pages right; pages 0x8d.. fall through into the 0x94 page code when x == -0.15 (no-op).
pub fn panel_baseCB_MENU_STICK_LEFTRIGHT(e: &mut Engine, a0: u32) -> LResult<u64> {
    fn page94(e: &mut Engine, x: f32) -> LResult<()> {
        if x < -0.15 {
            panel_baseFUNC_DrawNewMenu(e, 0x8d, 0x8e, 0x93)?;
        } else if -0.15 < x {
            panel_baseFUNC_DrawNewMenu(e, 0x9c, 0x9d, 0xa4)?;
        } else {
            page9c(e, x)?;
        }
        Ok(())
    }
    fn page9c(e: &mut Engine, x: f32) -> LResult<()> {
        if x < -0.15 {
            panel_baseFUNC_DrawNewMenu(e, 0x94, 0x95, 0x9b)?;
        }
        Ok(())
    }
    let l = LB(e);
    let kk = |e: &Engine, s: usize| l + k(e, s);
    match e.r32(kk(e, panel_baseCB_MENU_STICK_LEFTRIGHT_0_1)) {
        0xf | 0x30 => {
            APInewSound::Play(e, SELF, 10, 0);
            let x = e.rf(a0);
            if x < -0.15 {
                let v = e.r32(l + 0xa8).wrapping_sub(1);
                e.w32(l + 0xa8, v);
            } else if 0.15 < x {
                let v = e.r32(l + 0xa8).wrapping_add(1);
                e.w32(l + 0xa8, v);
            }
            let a = e.r32(kk(e, panel_baseCB_MENU_STICK_LEFTRIGHT_0_2)) as i32;
            let b = e.r32(kk(e, panel_baseCB_MENU_STICK_LEFTRIGHT_0_3)) as i32;
            panel_baseFUNC_MenuRefresh(e, a, b)?;
        }
        0x45 => {
            APInewSound::Play(e, SELF, 10, 0);
            let x = e.rf(a0);
            if 0.15 < x {
                if e.r32(kk(e, panel_baseCB_MENU_STICK_LEFTRIGHT_0_4)) == 3 {
                    e.w32(kk(e, panel_baseCB_MENU_STICK_LEFTRIGHT_0_5), 1);
                } else {
                    let v = e.r32(kk(e, panel_baseCB_MENU_STICK_LEFTRIGHT_0_7)).wrapping_add(1);
                    e.w32(kk(e, panel_baseCB_MENU_STICK_LEFTRIGHT_0_6), v);
                }
            } else if x < -0.15 {
                if e.r32(kk(e, panel_baseCB_MENU_STICK_LEFTRIGHT_0_8)) == 1 {
                    e.w32(kk(e, panel_baseCB_MENU_STICK_LEFTRIGHT_0_9), 3);
                } else {
                    let v = e.r32(kk(e, panel_baseCB_MENU_STICK_LEFTRIGHT_0_11)).wrapping_sub(1);
                    e.w32(kk(e, panel_baseCB_MENU_STICK_LEFTRIGHT_0_10), v);
                }
            }
            panel_baseFUNC_EpFilesLR(e)?;
        }
        0x79 | 0x7b..=0x7f => {
            if 0.15 < e.rf(a0) {
                panel_baseFUNC_DrawNewMenu(e, 0x80, 0x81, 0x8c)?;
            }
        }
        0x80 | 0x82..=0x8c => {
            let x = e.rf(a0);
            if x < -0.15 {
                panel_baseFUNC_DrawNewMenu(e, 0x79, 0x7a, 0x7f)?;
            } else if -0.15 < x {
                panel_baseFUNC_DrawNewMenu(e, 0x8d, 0x8e, 0x93)?;
            }
        }
        0x8d | 0x8f..=0x92 => {
            let x = e.rf(a0);
            if x < -0.15 {
                panel_baseFUNC_DrawNewMenu(e, 0x80, 0x81, 0x8c)?;
                return Ok(0);
            }
            if -0.15 < x {
                panel_baseFUNC_DrawNewMenu(e, 0x94, 0x95, 0x9b)?;
                return Ok(0);
            }
            // C falls through into the 0x94 page case
            let x = e.rf(a0);
            page94(e, x)?;
        }
        0x94 | 0x96..=0x9b => {
            let x = e.rf(a0);
            page94(e, x)?;
        }
        0x9c | 0x9e..=0xa4 => {
            let x = e.rf(a0);
            page9c(e, x)?;
        }
        _ => {}
    }
    Ok(0)
}

/// 0x0001e1f0 (2156 bytes) `undefined8 panel_baseCB_MENU_CONFIRM(void)`
/// Debug level-select pages (0x79..0xa4) load levels directly; quirk kept: page 0x8d lists 0x93
/// (EP3\L03) as a sub-case but 0x93 is never a selectable entry of that page's switch range.
pub fn panel_baseCB_MENU_CONFIRM(e: &mut Engine, _: u32) -> LResult<u64> {
    fn load(e: &mut Engine, path: &str, name: &str) {
        let (a, b) = (lit(e, path), lit(e, name));
        APIgame::LoadLevel(e, a, b, 0);
    }
    let l = LB(e);
    let sel = e.r32(l + 0xa8);
    match e.r32(l + k(e, panel_baseCB_MENU_CONFIRM_0_1)) {
        7 => {
            panel_baseFUNC_MainMenuConfirm(e)?;
        }
        0xf | 0x30 => {
            APInewSound::Play(e, SELF, 0xd, 0);
            panel_baseFUNC_YesNoConfirm(e)?;
        }
        0x12 => {
            APInewSound::Play(e, SELF, 0xd, 0);
            panel_baseFUNC_RerunMenuConfirm(e)?;
        }
        0x45 => {
            panel_baseFUNC_EpFilesConfirm(e)?;
        }
        0x79 | 0x7b..=0x7f => match sel {
            0x7b => load(e, "EP1\\L01", "EP1L01"),
            0x7c => load(e, "EP1\\L02", "EP1L02"),
            0x7d => load(e, "EP1\\L03A", "EP1L03A"),
            0x7e => load(e, "EP1\\L03B", "EP1L03B"),
            0x7f => load(e, "EP1\\L04", "EP1L04"),
            _ => {}
        },
        0x80 | 0x82..=0x8c => match sel {
            0x82 => load(e, "EP2\\L01A", "EP2L01A"),
            0x83 => load(e, "EP2\\L01B", "EP2L01B"),
            0x84 => load(e, "EP2\\L01C", "EP2L01C"),
            0x85 => load(e, "EP2\\L02A", "EP2L02A"),
            0x86 => load(e, "EP2\\L02B", "EP2L02B"),
            0x87 => load(e, "EP2\\L02C", "EP2L02C"),
            0x88 => load(e, "EP2\\L04", "EP2L04"),
            0x89 => load(e, "EP2\\L03A", "EP2L03A"),
            0x8a => load(e, "EP2\\L03B", "EP2L03B"),
            0x8b => load(e, "EP2\\L03C", "EP2L03C"),
            0x8c => load(e, "EP2\\L03D", "EP2L03D"),
            _ => {}
        },
        0x8d | 0x8f..=0x92 => match sel {
            0x8f => load(e, "EP3\\L01", "EP3L01"),
            0x90 => load(e, "EP3\\L02", "EP3L02"),
            0x91 => load(e, "EP3\\L04", "EP3L04"),
            0x92 => load(e, "EP3\\L05", "EP3L05"),
            0x93 => load(e, "EP3\\L03", "EP3L03"),
            _ => {}
        },
        0x94 | 0x96..=0x9b => match sel {
            0x96 => load(e, "EP4\\L01", "EP4L01"),
            0x97 => load(e, "EP4\\L02A", "EP4L02A"),
            0x98 => load(e, "EP4\\L02B", "EP4L02B"),
            0x99 => load(e, "EP4\\L03A", "EP4L03A"),
            0x9a => load(e, "EP4\\L03B", "EP4L03B"),
            0x9b => load(e, "EP4\\L04", "EP4L04"),
            _ => {}
        },
        0x9c | 0x9e..=0xa4 => match sel {
            0x9e => load(e, "EP6\\L01", "EP6L01"),
            0x9f => load(e, "EP6\\L02", "EP6L02"),
            0xa0 => load(e, "EP6\\L03", "EP6L03"),
            0xa1 => load(e, "EP6\\L04A", "EP6L04A"),
            0xa2 => load(e, "EP6\\L04B", "EP6L04B"),
            0xa3 => load(e, "EP6\\L05", "EP6L05"),
            0xa4 => load(e, "EP6\\L05B", "EP6L05B"),
            _ => {}
        },
        _ => {}
    }
    Ok(0)
}

/// 0x0001ea60 (1412 bytes) `undefined8 panel_baseCB_MENU_CANCEL(void)`
/// Quirk kept: the episode-files case (0x45/0x46, not on the objectives tab and not opened by L1) falls
/// through into the default case, so the pause menu is drawn twice.
pub fn panel_baseCB_MENU_CANCEL(e: &mut Engine, _: u32) -> LResult<u64> {
    let l = LB(e);
    let kk = |e: &Engine, s: usize| l + k(e, s);
    let w = |e: &Engine, s: usize| e.r32(l + k(e, s));
    if e.r32(l + 0x2c) != 0 {
        APInewSound::Play(e, SELF, 0xe, 0);
        let st = e.r32(kk(e, panel_baseCB_MENU_CANCEL_0_1));
        let mut dflt = false;
        match st {
            7 => {
                APIinput::InputConfigSetState(e, 8, 0);
                panel_baseFUNC_PlayPauseOUTAnims(e)?;
                e.w32(l + 0x2c, 0);
            }
            0x33 | 0x36 | 0x3a | 0x3e | 0x42 => {
                panel_baseFUNC_DrawNewMenu(e, 0x45, 0x46, 0x47)?;
            }
            0x45 | 0x46 => {
                if e.r32(kk(e, panel_baseCB_MENU_CANCEL_0_2)) == 4 || e.r32(kk(e, panel_baseCB_MENU_CANCEL_0_3)) == 1 {
                    panel_baseFUNC_EpFilesCancel(e)?;
                    return Ok(0);
                }
                let h = e.r32(LP(e) + k(e, panel_baseCB_MENU_CANCEL_0_4));
                APIhud::WindowSetFlags(e, h, 0x100);
                APIhud::WindowSetFlags(e, w(e, panel_baseCB_MENU_CANCEL_0_5), 0x100);
                panel_baseFUNC_DrawNewMenu(e, 7, 9, 0xc)?;
                APIhud::WindowResetFlags(e, w(e, panel_baseCB_MENU_CANCEL_0_6), 0x100);
                APIhud::WindowResetFlags(e, w(e, panel_baseCB_MENU_CANCEL_0_7), 0x100);
                APIhud::WindowResetFlags(e, w(e, panel_baseCB_MENU_CANCEL_0_8), 0x100);
                APIhud::WindowResetFlags(e, w(e, panel_baseCB_MENU_CANCEL_0_9), 0x100);
                APIhud::WindowSetFlags(e, w(e, panel_baseCB_MENU_CANCEL_0_10), 0x100);
                APIhud::WindowSetFlags(e, w(e, panel_baseCB_MENU_CANCEL_0_11), 0x100);
                APIinput::InputConfigActivate(e, 10, 0);
                APIinput::InputConfigActivate(e, 8, 1);
                APIinput::InputConfigSetState(e, 8, 0x192);
                for s in [ID_8000003f, ID_80000040, ID_80000041, ID_80000042, ID_80000043] {
                    APInewSound::StopSound(e, SELF, k(e, s));
                }
                APIanim::StopAnim(e, SELF, k(e, panel_baseCB_MENU_CANCEL_0_12));
                APIanim::Play(e, SELF, k(e, panel_baseCB_MENU_CANCEL_0_13), 2, 0.0);
                if e.r32(GBP(e, 0) + 0x38) == 0 {
                    let v = e.r32(kk(e, panel_baseCB_MENU_CANCEL_0_15)) & 0xffff_fffb;
                    e.w32(kk(e, panel_baseCB_MENU_CANCEL_0_14), v);
                    let v = e.r32(kk(e, panel_baseCB_MENU_CANCEL_0_17)) | 4;
                    e.w32(kk(e, panel_baseCB_MENU_CANCEL_0_16), v);
                    let v = e.r32(kk(e, panel_baseCB_MENU_CANCEL_0_19)) & 0xffff_fff7;
                    e.w32(kk(e, panel_baseCB_MENU_CANCEL_0_18), v);
                    let v = e.r32(kk(e, panel_baseCB_MENU_CANCEL_0_21)) | 8;
                    e.w32(kk(e, panel_baseCB_MENU_CANCEL_0_20), v);
                }
                panel_baseFUNC_PlayPauseINAnims(e)?;
                APIhud::WindowResetFlags(e, w(e, panel_baseCB_MENU_CANCEL_0_22), 0x100);
                APIhud::WindowResetFlags(e, w(e, panel_baseCB_MENU_CANCEL_0_23), 0x100);
                dflt = true; // fall through
            }
            _ => dflt = true,
        }
        if dflt {
            if e.r32(GBP(e, 0) + 0x8c) == 0 {
                panel_baseFUNC_DrawNewMenu(e, 7, 9, 0xc)?;
                APIinput::InputConfigSetState(e, 8, 0x192);
                APIhud::WindowSetFlags(e, w(e, panel_baseCB_MENU_CANCEL_0_24), 0x100);
            } else {
                panel_baseFUNC_DrawNewMenu(e, 7, 9, 0xd)?;
                APIinput::InputConfigSetState(e, 8, 0x192);
                APIhud::WindowResetFlags(e, w(e, panel_baseCB_MENU_CANCEL_0_25), 0x100);
            }
        }
    }
    Ok(0)
}

/// 0x0001efe8 (180 bytes) `undefined4 panel_baseCB_MENU_IN_LF(void)`
pub fn panel_baseCB_MENU_IN_LF(e: &mut Engine, _: u32) -> LResult<u64> {
    APIanim::StopAnim(e, SELF, k(e, panel_baseCB_MENU_IN_LF_0_1));
    APIanim::Play(e, SELF, k(e, panel_baseCB_MENU_IN_LF_0_2), 2, 0.0);
    APIextra::TimerReset(e, e.r32(LB(e) + k(e, panel_baseCB_MENU_IN_LF_0_3)));
    APIinput::InputConfigSetState(e, 8, 0x192);
    Ok(1)
}

/// 0x0001f0a0 (116 bytes) `undefined4 panel_baseCB_MENU_OUT_LF(void)`
pub fn panel_baseCB_MENU_OUT_LF(e: &mut Engine, _: u32) -> LResult<u64> {
    match g(e, G_panel_baseCB_MENU_OUT_LF) {
        0 => {
            if e.r32(LB(e) + k(e, panel_baseCB_MENU_OUT_LF_0_1)) == 0 {
                APIgameobj::SetNextState(e, SELF, 1);
            }
        }
        1 => {
            if e.r32(lb(e, 0x4a8)) == 0 {
                panel_baseFUNC_PauseOff(e)?;
            }
        }
        _ => unported_shape(e, "panel_baseCB_MENU_OUT_LF", G_panel_baseCB_MENU_OUT_LF),
    }
    Ok(1)
}

/// 0x0001f118 (3164 bytes) `undefined8 panel_baseCB_INGAME_LTRIGGER1(void)`
/// L1 toggles the episode files. Shape groups: 0 reference; 1 (3ce3c13c) = 0 + SendMessageTo(slot 1_1, 0, 0,
/// 0, 0) after entering; 2 (cb9dbf66) = 1 with object 0x2a000125 and an extra guard (not during the ToD-boss
/// levels 10..=12 while LB+0xc == 1). Other group differences (anim / sound / object ids, LP offsets) are
/// per-build slots. Quirk kept: LB+0x450 is set to 1 just before the tab test, so tabs 2/3 are dead code.
pub fn panel_baseCB_INGAME_LTRIGGER1(e: &mut Engine, _: u32) -> LResult<u64> {
    let grp = g(e, G_panel_baseCB_INGAME_LTRIGGER1);
    if grp > 2 {
        unported_shape(e, "panel_baseCB_INGAME_LTRIGGER1", G_panel_baseCB_INGAME_LTRIGGER1);
    }
    let l = LB(e);
    let kk = |e: &Engine, s: usize| l + k(e, s);
    let w = |e: &Engine, s: usize| e.r32(l + k(e, s));
    let wp = |e: &Engine, s: usize| e.r32(LP(e) + k(e, s));
    if e.r32(GBP(e, 0) + 0x38) != 0 || e.r32(l + 0x40) == 1 {
        return Ok(0);
    }
    let t = APIextra::TimerGetTime(e, w(e, panel_baseCB_INGAME_LTRIGGER1_0_1));
    let pass = if grp == 2 {
        let lvl = e.r32(GBP(e, 0) + 0x3c);
        (0.25 <= t || e.r32(l + 0x4c) != 0) && (lvl < 10 || 0xc < lvl || e.r32(l + 0xc) != 1)
    } else {
        0.25 <= t || e.r32(kk(e, panel_baseCB_INGAME_LTRIGGER1_0_2)) != k(e, panel_baseCB_INGAME_LTRIGGER1_0_3)
    };
    if !pass {
        return Ok(0);
    }
    APIextra::TimerReset(e, w(e, panel_baseCB_INGAME_LTRIGGER1_0_4));
    if e.r32(kk(e, panel_baseCB_INGAME_LTRIGGER1_0_5)) == 0 {
        let orp = e.alloca(4);
        e.w32(orp, 1);
        APIgameobj::SendMessageTo(e, 0xff00_0001, orp, 0x12, 0, 0);
        let pad = e.r32(GBP(e, 0) + 0x4c) as i32;
        APIinput::InputSetVibration(e, pad, 0, 0.0, 0.0, 0.0, 0.0);
        let pad = e.r32(GBP(e, 0) + 0x4c) as i32;
        APIinput::InputSetVibration(e, pad, 1, 0.0, 0.0, 0.0, 0.0);
        LIBApiText::DisableWindow(e);
        APIinput::InputConfigActivate(e, 7, 0);
        APIinput::InputConfigActivate(e, 8, 1);
        // slot 0_6 is misaligned in groups 1/2 (their C has the same 0x2000196)
        let st = if grp == 0 { k(e, panel_baseCB_INGAME_LTRIGGER1_0_6) } else { 0x200_0196 };
        APIinput::InputConfigSetState(e, 8, st as i32);
        match grp {
            1 => {
                APIgameobj::SendMessageTo(e, k(e, panel_baseCB_INGAME_LTRIGGER1_1_1), 0, 0, 0, 0);
            }
            2 => {
                APIgameobj::SendMessageTo(e, 0x2a00_0125, 0, 0, 0, 0);
            }
            _ => {}
        }
        e.w32(kk(e, panel_baseCB_INGAME_LTRIGGER1_0_7), 7);
        e.w32(kk(e, panel_baseCB_INGAME_LTRIGGER1_0_8), 1);
        e.w32(kk(e, panel_baseCB_INGAME_LTRIGGER1_0_9), 1);
        APIhud::WindowResetFlags(e, w(e, panel_baseCB_INGAME_LTRIGGER1_0_10), 0x100);
        for o in [0xd0, 0xd4, 0xd8, 0xb0, 0xe0, 0xe4, 0xe8] {
            APIhud::WindowSetFlags(e, e.r32(l + o), 0x100);
        }
        APIhud::WindowResetFlags(e, w(e, panel_baseCB_INGAME_LTRIGGER1_0_11), 0x100);
        APIhud::WindowResetFlags(e, w(e, panel_baseCB_INGAME_LTRIGGER1_0_12), 0x100);
        if e.r32(GBP(e, 3)) & 0x20000 != 0 {
            APIhud::WindowSetFlags(e, e.r32(l + 0xf4), 0x100);
            APIhud::WindowSetFlags(e, e.r32(l + 0xf8), 0x100);
        }
        APIgame::Pause(e, 1);
        e.w32(l + 0x2c, 1);
        APIanim::Play(e, SELF, k(e, panel_baseCB_INGAME_LTRIGGER1_0_13), 2, 0.0);
        APIanim::Play(e, SELF, k(e, panel_baseCB_INGAME_LTRIGGER1_0_14), 2, 0.0);
        if e.r32(kk(e, panel_baseCB_INGAME_LTRIGGER1_0_15)) == 1 || e.r32(kk(e, panel_baseCB_INGAME_LTRIGGER1_0_16)) == 0 {
            APIanim::Play(e, SELF, k(e, panel_baseCB_INGAME_LTRIGGER1_0_17), 1, 0.0);
            APIanim::Play(e, SELF, k(e, panel_baseCB_INGAME_LTRIGGER1_0_18), 1, 0.0);
            APIanim::Play(e, SELF, k(e, panel_baseCB_INGAME_LTRIGGER1_0_19), 1, 0.0);
            APIhud::WindowResetFlags(e, w(e, panel_baseCB_INGAME_LTRIGGER1_0_20), 0x100);
        } else if e.r32(kk(e, panel_baseCB_INGAME_LTRIGGER1_0_21)) == 2 {
            APIanim::Play(e, SELF, k(e, panel_baseCB_INGAME_LTRIGGER1_0_22), 1, 0.0);
            APIanim::Play(e, SELF, k(e, panel_baseCB_INGAME_LTRIGGER1_0_23), 1, 0.0);
            APIanim::Play(e, SELF, k(e, panel_baseCB_INGAME_LTRIGGER1_0_24), 1, 0.0);
            if e.r32(GBP(e, 3) + 0xc) & 0x3e0 == 0 {
                e.w32(kk(e, panel_baseCB_INGAME_LTRIGGER1_0_25), 99);
            } else {
                // the C passes a 4th, duplicate colour argument; the API takes three
                APIhud::TextWindowSetText(e, w(e, panel_baseCB_INGAME_LTRIGGER1_0_26), 0x138, &[6, 6]);
                // _DAT_0001230c: text id of a pause window spec record (same address in all 37 builds)
                e.w32(img(e, 0x1230c), 0x138);
                APIhud::WindowResetFlags(e, w(e, panel_baseCB_INGAME_LTRIGGER1_0_27), 0x100);
                APIhud::WindowResetFlags(e, w(e, panel_baseCB_INGAME_LTRIGGER1_0_28), 0x100);
                APIhud::WindowResetFlags(e, w(e, panel_baseCB_INGAME_LTRIGGER1_0_29), 0x100);
                e.w32(kk(e, panel_baseCB_INGAME_LTRIGGER1_0_30), 1);
            }
            panel_baseFUNC_EpFilesClueTexFade(e)?;
            panel_baseFUNC_EpFilesClueUpdate(e)?;
        } else if e.r32(kk(e, panel_baseCB_INGAME_LTRIGGER1_0_31)) == 3 {
            APIanim::Play(e, SELF, k(e, panel_baseCB_INGAME_LTRIGGER1_0_32), 1, 0.0);
            APIanim::Play(e, SELF, k(e, panel_baseCB_INGAME_LTRIGGER1_0_33), 1, 0.0);
            APIanim::Play(e, SELF, k(e, panel_baseCB_INGAME_LTRIGGER1_0_34), 1, 0.0);
            APIhud::WindowResetFlags(e, wp(e, panel_baseCB_INGAME_LTRIGGER1_0_35), 0x100);
            panel_baseFUNC_EpFilesMonstersUpdate(e)?;
        }
        APIhud::WindowSetFlags(e, w(e, panel_baseCB_INGAME_LTRIGGER1_0_36), 0x100);
        APIhud::WindowSetFlags(e, w(e, panel_baseCB_INGAME_LTRIGGER1_0_37), 0x100);
        APIhud::WindowSetFlags(e, w(e, panel_baseCB_INGAME_LTRIGGER1_0_38), 0x100);
        e.w32(kk(e, panel_baseCB_INGAME_LTRIGGER1_0_39), 0);
        APInewSound::PauseSoundEvents(e, 0x47);
        APIgameobj::SendMessageTo(e, k(e, ID_2a000030), 0, 0x5c, 5, 0);
        panel_baseFUNC_DrawNewMenu(e, 0x45, 0x46, 0x5a)?;
        panel_baseFUNC_EpFilesObjUpdate(e)?;
        if e.r32(l + 0x14) == 1 {
            APIextra::TimerUnregisterNotify(e, SELF, w(e, panel_baseCB_INGAME_LTRIGGER1_0_40));
        }
        APIgameobj::SetNextState(e, SELF, 4);
    } else {
        e.w32(kk(e, panel_baseCB_INGAME_LTRIGGER1_0_41), 0);
        APIhud::WindowSetFlags(e, w(e, panel_baseCB_INGAME_LTRIGGER1_0_42), 0x100);
        APIhud::WindowSetFlags(e, wp(e, panel_baseCB_INGAME_LTRIGGER1_0_43), 0x100);
        for s in [
            panel_baseCB_INGAME_LTRIGGER1_0_44,
            panel_baseCB_INGAME_LTRIGGER1_0_45,
            panel_baseCB_INGAME_LTRIGGER1_0_46,
            panel_baseCB_INGAME_LTRIGGER1_0_47,
            panel_baseCB_INGAME_LTRIGGER1_0_48,
            panel_baseCB_INGAME_LTRIGGER1_0_49,
            panel_baseCB_INGAME_LTRIGGER1_0_50,
        ] {
            APIhud::WindowSetFlags(e, w(e, s), 0x100);
        }
        if e.r32(GBP(e, 3)) & 0x20000 != 0 {
            APIhud::WindowResetFlags(e, e.r32(l + 0xf4), 0x100);
            APIhud::WindowResetFlags(e, e.r32(l + 0xf8), 0x100);
        }
        for s in [ID_8000003f, ID_80000040, ID_80000041, ID_80000042, ID_80000043] {
            APInewSound::StopSound(e, SELF, k(e, s));
        }
        panel_baseFUNC_PauseOff(e)?;
        e.w32(l + 0x2c, 0);
        let m = w(e, panel_baseCB_INGAME_LTRIGGER1_0_51) as i32;
        panel_baseFUNC_ShowInGame(e, m)?;
        if e.r32(l + 0x14) == 1 {
            APIhud::WindowResetFlags(e, wp(e, panel_baseCB_INGAME_LTRIGGER1_0_52), 0x100);
            APIhud::WindowResetFlags(e, wp(e, panel_baseCB_INGAME_LTRIGGER1_0_53), 0x100);
            for i in 0..5u32 {
                if e.r32(GBP(e, 1) + 0x54 + i * 4) == 0 {
                    let h = e.r32(LP(e) + k(e, panel_baseCB_INGAME_LTRIGGER1_0_54) + (i + 1) * 4);
                    APIhud::WindowSetFlags(e, h, 0x100);
                }
            }
            APIextra::TimerRegisterNotify(e, SELF, w(e, panel_baseCB_INGAME_LTRIGGER1_0_55), 0x16);
        }
        if e.r32(l + 0x4c) == 0 {
            APIgameobj::SetNextState(e, SELF, 1);
        }
    }
    e.w32(l + 0x4c, 0);
    Ok(0)
}

/// 0x0001fd78 (788 bytes) `undefined8 panel_baseCB_TRIANGLE(void)`
/// Shape group 1 (2f3cf860) lacks the "alternative meters -> LB+0x478 = 3" block.
pub fn panel_baseCB_TRIANGLE(e: &mut Engine, _: u32) -> LResult<u64> {
    let grp = g(e, G_panel_baseCB_TRIANGLE);
    if grp > 1 {
        unported_shape(e, "panel_baseCB_TRIANGLE", G_panel_baseCB_TRIANGLE);
    }
    let l = LB(e);
    let kk = |e: &Engine, s: usize| l + k(e, s);
    let w = |e: &Engine, s: usize| e.r32(l + k(e, s));
    if e.r32(kk(e, panel_baseCB_TRIANGLE_0_1)) & 0xc == 0
        && e.r32(GBP(e, 0) + 0x14) & 1 == 0
        && e.r32(kk(e, panel_baseCB_TRIANGLE_0_2)) == 0
        && e.r32(l + 0x20) != 1
        && e.r32(l + 0x44) != 1
        && e.r32(l + 0x2c) != 1
    {
        if grp == 0
            && e.r32(kk(e, panel_baseCB_TRIANGLE_0_3)) != 3
            && e.r32(GBP(e, 3)) & 1 != 0
            && e.r32(GBP(e, 0) + 0x14) & 2 == 0
        {
            e.w32(kk(e, panel_baseCB_TRIANGLE_0_4), 3);
        }
        e.w32(l + 0x14, 1);
        panel_baseFUNC_InitSlideIns(e)?;
        APIextra::TimerReset(e, w(e, panel_baseCB_TRIANGLE_0_5));
        APIextra::TimerReset(e, w(e, panel_baseCB_TRIANGLE_0_6));
        APIextra::TimerRegisterNotify(e, SELF, w(e, panel_baseCB_TRIANGLE_0_7), 0x16);
        if e.r32(GBP(e, 0) + 0x38) == 0 {
            let h = e.r32(LP(e) + k(e, panel_baseCB_TRIANGLE_0_8));
            APIhud::WindowResetFlags(e, h, 0x100);
            for i in 0..5u32 {
                if e.r32(GBP(e, 1) + 0x54 + i * 4) == 0 {
                    let h = e.r32(LP(e) + k(e, panel_baseCB_TRIANGLE_0_9) + (i + 1) * 4);
                    APIhud::WindowSetFlags(e, h, 0x100);
                }
            }
            let h = e.r32(LP(e) + k(e, panel_baseCB_TRIANGLE_0_10));
            APIhud::WindowResetFlags(e, h, 0x100);
            let v = e.r32(kk(e, panel_baseCB_TRIANGLE_0_12)) | 0xc;
            e.w32(kk(e, panel_baseCB_TRIANGLE_0_11), v);
            panel_baseFUNC_PlayPauseINAnims(e)?;
        }
    }
    Ok(0)
}

/// 0x00020098 (224 bytes) `undefined8 panel_baseCB_NOTIFY_TEXFADER(void)`
pub fn panel_baseCB_NOTIFY_TEXFADER(e: &mut Engine, _: u32) -> LResult<u64> {
    let l = LB(e);
    if e.r32(l + 0x3c) == 0 && e.r32(l + 0x44) == 0 {
        e.w32(l + 0x3c, 1);
        let v = e.r32(l + 0x58);
        e.w32(l + 0x54, v);
    }
    if e.r32(l + 0x30) == 0 {
        e.w32(l + 0x30, 1);
        let v = e.r32(lb(e, 0x460));
        e.w32(lb(e, 0x45c), v);
    }
    if e.r32(l + 0x38) == 1 {
        e.w32(l + 0x38, 0);
    }
    Ok(0)
}

/// 0x00020178 (188 bytes) `undefined4 panel_baseCB_TIMER_SLIDEINS_DISPLAY(void)`
pub fn panel_baseCB_TIMER_SLIDEINS_DISPLAY(e: &mut Engine, _: u32) -> LResult<u64> {
    APIgameobj::UnregisterNotify(e, SELF, 0xffff_ffff, 1);
    panel_baseFUNC_InitSlideIns(e)?;
    panel_baseFUNC_PlayPauseOUTAnims(e)?;
    APIinput::InputConfigSetState(e, 7, 0);
    APIgameobj::SetNextState(e, SELF, 10);
    Ok(1)
}

/// 0x00020238 (48 bytes) `undefined8 panel_baseCB_TIMER_BUTTON_FLASH(void)`
pub fn panel_baseCB_TIMER_BUTTON_FLASH(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

/// 0x00020268 (764 bytes) `undefined8 panel_baseCB_NOINPUT(int param_1)`
pub fn panel_baseCB_NOINPUT(e: &mut Engine, a0: u32) -> LResult<u64> {
    let l = LB(e);
    let kk = |e: &Engine, s: usize| l + k(e, s);
    if e.r32(l + 0x48) != 1
        && ((e.r32(l + 0x40) == 0 && e.r32(l + 0x1c) == 0) || e.r32(l + 0x2c) != 0)
        && a0 == e.r32(GBP(e, 0) + 0x4c)
    {
        let st = APIgameobj::GetCurrentStateID(e, SELF);
        // short-circuit: TimerGetTime is only called when the state test passes
        if st != 4
            && st != 3
            && st != 0
            && st != 9
            && st != 10
            && 1.0 < APIextra::TimerGetTime(e, e.r32(kk(e, panel_baseCB_NOINPUT_0_1)))
        {
            if e.r32(kk(e, panel_baseCB_NOINPUT_0_2)) == 0 {
                let m = lit(e, "NOCONTROLLER WINDOW AVAILABLE!!!\n\n");
                APIextra::null_printf(e, m, 0);
            } else {
                let w = e.r32(kk(e, panel_baseCB_NOINPUT_0_3));
                APIhud::WindowResetFlags(e, w, 0x100);
            }
            let m = lit(e, "NO INPUT \n\n ");
            APIextra::null_printf(e, m, 0);
            APIgameobj::LeaveParent(e, SELF, 0);
            if e.r32(kk(e, panel_baseCB_NOINPUT_0_4)) == 0 && st != 5 && st != 6 {
                APIgameobj::SetNextState(e, SELF, 3);
            }
        }
        // quirk: runs on every call that passes the outer test, not only after the 1 s timeout
        if e.r32(kk(e, panel_baseCB_NOINPUT_0_5)) != 0 {
            let w = e.r32(kk(e, panel_baseCB_NOINPUT_0_6));
            APIhud::WindowResetFlags(e, w, 0x100);
            APIextra::TimerReset(e, e.r32(kk(e, panel_baseCB_NOINPUT_0_7)));
            APIextra::TimerRegisterNotify(e, SELF, e.r32(kk(e, panel_baseCB_NOINPUT_0_8)), 0x1c);
            APIextra::TimerSetFrequency(e, e.r32(kk(e, panel_baseCB_NOINPUT_0_9)), 0.1);
        }
    }
    Ok(0)
}

/// 0x00020568 (164 bytes) `undefined4 panel_baseCB_ANIM_LF(void)`
pub fn panel_baseCB_ANIM_LF(e: &mut Engine, _: u32) -> LResult<u64> {
    APIgameobj::UnregisterNotify(e, SELF, 0xffff_ffff, 1);
    let v = e.r32(LB(e) + k(e, panel_baseCB_ANIM_LF_0_2)) & 0xffff_fff3;
    e.w32(LB(e) + k(e, panel_baseCB_ANIM_LF_0_1), v);
    APIanim::Play(e, SELF, k(e, panel_baseCB_ANIM_LF_0_3), 2, 0.0);
    Ok(1)
}

/// 0x00020610 (7492 bytes) `undefined4 panel_baseFUNC_MessageHandler(uint param_1,uint param_2,float *param_3)`
/// Shape group 0 (18 builds) ported; other groups panic. Ghidra shows `in_f0` (the stale f0) for
/// TimerGetTime / GetFrameDeltaTime results: here they are those calls' return values. In case 0x41 the
/// time sent to object 0x2a0000bb is likewise TimerGetTime(0x90000000) + game+0x68 (Ghidra attributed the
/// f0 value to PauseOn's return). DAT_0001a608 = -0.0 (courage meter full).
pub fn panel_baseFUNC_MessageHandler(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u32> {
    // shape groups: 0 reference; 1 (7 builds) / 2 (2 builds): other episode-files object, and a message 0x165
    // (to k 1_1 / 0x2a000010) when the slide-in is cut by 0x52; group 2 also skips the pause slide-in for
    // game +0x3c in 10..=12 (straight to state 4); 4 (3 builds): group 2's pause skip without the 0x165;
    // 5 (695761b7): the 0x30 tome tick is suppressed while LP+0x5b0 is set (cleared by 0xcb).
    // 3 (6 boss-meter builds, shifted layout handled by lb()): reference + a tail call
    // panel_baseFUNC_MessageHandlerBossMeter(param_1, param_2) on fall-through (early returns skip it).
    let grp = g(e, G_panel_baseFUNC_MessageHandler);
    // WP-Q3: the episode-files / save panel object is per build (the reference's 0x2a0000bb slot of the
    // build's id table: EP4L02A 0x2a0000cf, 05789c5e 0x2a000107, ...); the former per-shape-group constant
    // sent 0x3d to the wrong object in most builds (EP4L02A: enmyspot 0x2a0000ca) -> the 0x41 checkpoint
    // save prompt never opened and the world stayed paused.
    if !matches!(grp, 0..=5) {
        unported_shape(e, "panel_baseFUNC_MessageHandler", G_panel_baseFUNC_MessageHandler);
    }
    let ep_obj = k(e, ID_2a0000bb);
    let w = |e: &Engine, o: u32| e.r32(lb(e, o));
    let set = |e: &mut Engine, o: u32| {
        let h = w(e, o);
        APIhud::WindowSetFlags(e, h, 0x100);
    };
    let reset = |e: &mut Engine, o: u32| {
        let h = w(e, o);
        APIhud::WindowResetFlags(e, h, 0x100);
    };
    let lpw = |e: &Engine, s: usize| e.r32(LP(e) + k(e, s));
    let fl = |e: &Engine| lb(e, 0x468);
    let or = |e: &mut Engine, a: Ptr, m: u32| {
        let v = e.r32(a) | m;
        e.w32(a, v);
    };
    let and = |e: &mut Engine, a: Ptr, m: u32| {
        let v = e.r32(a) & m;
        e.w32(a, v);
    };
    // the "open pause menu" sequence shared by 0x41 / 0x42 / 0x43
    let pause_menu = |e: &mut Engine, pk: usize, arg: u32, data: Ptr| -> LResult<()> {
        panel_baseFUNC_PauseOn(e, k(e, pk) as i32)?;
        for o in [0x118, 0x114, 0x104, 0x108] {
            let h = e.r32(lb(e, o));
            APIhud::WindowSetFlags(e, h, 0x100);
        }
        APIgameobj::SendMessageTo(e, ep_obj, 0, 0x3d, arg, data);
        APIinput::InputConfigActivate(e, 8, 0);
        APIgameobj::SetNextState(e, SELF, 4);
        Ok(())
    };
    // 0x2a/0x2b: episode-file object added
    let ep_add = |e: &mut Engine, kind: u32, snd: u32, which: u32| -> LResult<u32> {
        if panel_baseFUNC_EpFilesObjAddSub(e, kind as i32, param_2)? as i32 != 1 {
            return Ok(0);
        }
        let f = if param_3 != 0 { e.rf(param_3) } else { 0.0 };
        APInewSound::Play(e, SELF, snd, 0);
        if f == 0.0 {
            if e.r32(lb(e, 0x468)) & 4 == 0 {
                e.wf(LB(e) + 0x90, 0.25);
            } else {
                e.w32(lb(e, 0x48c), which);
                let a = lb(e, 0x468);
                let v = e.r32(a) | 1;
                e.w32(a, v);
            }
        }
        Ok(1)
    };
    match param_1 {
        0x30 => {
            let go = grp != 5 || e.r32(LP(e) + 0x5b0) == 0;
            let t = if go { APIextra::TimerGetTime(e, 0x9000_0000) } else { 0.0 };
            if go && e.rf(LB(e) + 0x8c) < t {
                e.wf(LB(e) + 0x8c, t);
                let dt = APIgame::GetFrameDeltaTime(e);
                let a = (180.0 / e.rf(param_3)) * dt;
                e.wf(LB(e) + 0x80, a);
                APIgameobj::Rotate(e, k(e, panel_baseFUNC_MessageHandler_0_1), LB(e), a * 0.017453292);
            }
        }
        0x25 => {
            APIinput::InputContextSetState(e, 7, 0x34, 0x1c, if param_2 == 0 { 0 } else { 0x100 });
        }
        0x1d => {
            if e.r32(GBP(e, 0) + 0x14) & 0x40 == 0 {
                panel_baseFUNC_UpdateTomeMeter(e)?;
            }
        }
        10 => {
            panel_baseFUNC_SetItemWinPos(e, param_2 as i32, 0)?;
        }
        0 => e.w32(lb(e, 0x4a8), 0),
        0xb => {
            panel_baseFUNC_SetItemWinPos(e, param_2 as i32, 1)?;
        }
        0x15 => e.w32(LB(e) + 0x24, 1),
        0x1f => panel_baseFUNC_ShowInGame(e, param_2 as i32)?,
        0x1e => panel_baseFUNC_HideInGame(e)?,
        0x23 => {
            APIinput::InputContextSetState(e, 7, 0x3c, 0x24, if param_2 == 0 { 0 } else { 0x10 });
        }
        0x24 => {
            APIinput::InputContextSetState(e, 7, 0x41, 0x31, if param_2 == 0 { 0 } else { 0x100 });
        }
        0x2b => return ep_add(e, 1, 0x3d, 2),
        0x2a => return ep_add(e, 0, 0x3c, 1),
        0x27 => {
            if e.r32(GBP(e, 0) + 0x14) & 1 == 0 {
                panel_baseFUNC_ToDMeterChangeControl(e, 1)?;
            }
        }
        0x26 => {
            // item counters of the hero buffer, selected by the integer at *param_3
            let o = match e.r32(param_3) {
                1 => Some(0x14),
                2 => Some(0x1c),
                3 => Some(0x24),
                4 => Some(0x2c),
                5 => Some(0x44),
                6 | 7 | 8 => Some(0x34),
                9 => Some(0x3c),
                0xc => Some(0x4c),
                _ => None,
            };
            if let Some(o) = o {
                let a = GBP(e, 1) + o;
                let v = e.r32(a);
                if param_2 == 0 {
                    e.w32(a, v.wrapping_add(1));
                } else if v != 0 {
                    e.w32(a, v.wrapping_sub(1));
                }
            }
        }
        0x29 => {
            if param_2 == 1 {
                e.w32(LB(e) + 0xc, 1);
                APIextra::TimerUnregisterNotify(e, SELF, w(e, 0x3fc));
                or(e, lb(e, 0x428), 0xc);
                let (a, b) = (lpw(e, panel_baseFUNC_MessageHandler_0_4), lpw(e, panel_baseFUNC_MessageHandler_0_5));
                APIhud::WindowSetFlags(e, a, 0x100);
                APIhud::WindowSetFlags(e, b, 0x100);
                let gm = e.r32(GBP(e, 0) + 0x3c);
                if (grp == 2 || grp == 4) && 9 < gm && gm < 0xd {
                    APIhud::WindowResetFlags(e, a, 0x100);
                    APIhud::WindowResetFlags(e, b, 0x100);
                    APIgameobj::SetNextState(e, SELF, 4);
                }
                panel_baseFUNC_HideInGame(e)?;
                panel_baseFUNC_InitSlideIns(e)?;
                panel_baseFUNC_PlayPauseINAnims(e)?;
                reset(e, 0xfc);
                set(e, 0x118);
                set(e, 0x114);
                reset(e, 0x100);
                APIanim::Play(e, SELF, k(e, panel_baseFUNC_MessageHandler_0_6), 2, 0.0);
                APIanim::Play(e, SELF, k(e, panel_baseFUNC_MessageHandler_0_7), 2, 0.0);
            } else {
                panel_baseFUNC_ShowInGame(e, w(e, 0x41c) as i32)?;
                set(e, 0xfc);
                reset(e, 0x118);
                reset(e, 0x114);
                let (a, b) = (lpw(e, panel_baseFUNC_MessageHandler_0_10), lpw(e, panel_baseFUNC_MessageHandler_0_11));
                APIhud::WindowResetFlags(e, a, 0x100);
                APIhud::WindowResetFlags(e, b, 0x100);
                panel_baseFUNC_PauseOff(e)?;
                e.w32(LB(e) + 0x2c, 0);
                APIgameobj::SetNextState(e, SELF, 2);
            }
        }
        0x2d => {
            panel_baseFUNC_EpFilesObjAddSub(e, 3, param_2)?;
        }
        0x2c => {
            panel_baseFUNC_EpFilesObjAddSub(e, 2, param_2)?;
        }
        0x2f => {
            e.w32(LB(e) + 0x80, 0);
            e.w32(LB(e) + 0x14, 0);
            e.w32(lb(e, 0x41c), 1);
        }
        0x4e => {
            set(e, 0xac);
            APIinput::InputConfigActivate(e, 7, 0);
        }
        0x41 => {
            panel_baseFUNC_PauseOn(e, k(e, panel_baseFUNC_MessageHandler_0_13) as i32)?;
            for o in [0x118, 0x114, 0x104, 0x108] {
                set(e, o);
            }
            let t = APIextra::TimerGetTime(e, 0x9000_0000);
            let m = e.alloca(8);
            e.w32(m, param_2);
            let d = e.rf(GBP(e, 0) + 0x68);
            e.wf(m + 4, t + d);
            APIgameobj::SendMessageTo(e, ep_obj, 0, 0x3d, 1, m);
            APIinput::InputConfigActivate(e, 8, 0);
            APIgameobj::SetNextState(e, SELF, 4);
        }
        0x32 => {
            e.w32(lb(e, 0x488), param_2);
            and(e, fl(e), 0xffff_ffef);
            or(e, fl(e), 0x20);
        }
        0x31 => {
            or(e, GBP(e, 3), 0x20000);
            e.w32(lb(e, 0x488), param_2);
            or(e, fl(e), 0x10);
        }
        0x36 => panel_baseFUNC_EpFilesInitCurrentClueTex(e)?,
        0x3e => {
            APIinput::InputErrorRegisterNotify(e, e.r32(GBP(e, 0) + 0x4c) as i32, SELF, 0x1f);
            if param_2 == 1 || param_2 == 2 {
                if e.r32(LB(e) + 0x14) != 0 {
                    panel_baseCB_TIMER_SLIDEINS_DISPLAY(e, 0)?;
                }
                panel_baseFUNC_PauseOff(e)?;
                e.w32(LB(e) + 0x2c, 0);
                APIgameobj::SetNextState(e, SELF, 2);
            } else {
                for o in [0x114, 0x108, 0x110, 0x104, 0x10c] {
                    reset(e, o);
                }
                panel_baseFUNC_DrawNewMenu(e, 7, 9, 0xc)?;
                APIinput::InputConfigActivate(e, 8, 1);
            }
        }
        0x43 => {
            if e.r32(GBP(e, 0) + 0x8c) != 0 {
                set(e, 0x118);
                set(e, 0x114);
                pause_menu(e, panel_baseFUNC_MessageHandler_0_18, 9, 0)?;
            }
        }
        0x42 => {
            set(e, 0x118);
            set(e, 0x114);
            let m = e.alloca(4);
            e.w32(m, param_2);
            pause_menu(e, panel_baseFUNC_MessageHandler_0_21, 2, m)?;
        }
        0x4c => {
            e.w32(LB(e) + 0x1c, 1);
            set(e, 0xac);
            if e.r32(LB(e) + 0x14) != 0 {
                panel_baseCB_TIMER_SLIDEINS_DISPLAY(e, 0)?;
            }
            if e.r32(fl(e)) & 7 != 0 {
                or(e, fl(e), 8);
                set(e, if w(e, 0x48c) == 1 { 0xec } else { 0xf0 });
            }
            APIinput::InputConfigActivate(e, 7, 0);
        }
        0x4d => {
            e.w32(LB(e) + 0x40, 1);
            APIinput::InputConfigActivate(e, 7, 0);
            if w(e, 0xac) != 0 {
                set(e, 0xac);
            }
            if e.r32(LB(e) + 0x2c) != 0 && w(e, 0x470) != 2 {
                panel_baseFUNC_PauseOff(e)?;
                APIgameobj::SetNextState(e, SELF, 2);
            }
        }
        0xd1 => {
            let n = w(e, 0x480);
            let mut i = 0u32;
            while i <= n {
                e.w8(GBP(e, 0) + i * 8 + 0x744, (i as u8).wrapping_add(1));
                i += 1;
            }
        }
        0x52 => {
            if param_2 != 1 {
                e.w32(LB(e) + 0x2c, 0);
                if APIgameobj::GetCurrentStateID(e, SELF) == 9 {
                    panel_baseCB_TIMER_SLIDEINS_DISPLAY(e, 0)?;
                    panel_baseFUNC_PauseOff(e)?;
                    APIgameobj::SetNextState(e, SELF, 2);
                    match grp {
                        1 => {
                            let o = k(e, panel_baseFUNC_MessageHandler_1_1);
                            APIgameobj::SendMessageTo(e, o, 0, 0x165, 0, 0);
                        }
                        2 => {
                            APIgameobj::SendMessageTo(e, 0x2a00_0010, 0, 0x165, 0, 0);
                        }
                        _ => {}
                    }
                }
                return Ok(0);
            }
            if e.r32(LB(e) + 0x34) == 1 {
                let m = lit(e, "BACK FROM FADE\n");
                APIextra::null_printf(e, m, 0);
                e.w32(LB(e) + 0x34, 0);
                e.w32(LB(e) + 0x30, 1);
                panel_baseFUNC_TexFaderControl(e)?;
            }
            if e.r32(LB(e) + 0x1c) == 1 {
                e.w32(LB(e) + 0x40, 0);
                return Ok(0);
            }
            APIinput::InputConfigActivate(e, 7, 1);
            reset(e, 0xac);
            e.w32(LB(e) + 0x40, 0);
            if e.r32(LB(e) + 0x44) != 0 {
                return Ok(0);
            }
            if w(e, 0x474) == 2 {
                for o in [0xb0, 0xdc, 0xe0] {
                    reset(e, o);
                }
            } else {
                reset(e, 0xe4);
                reset(e, 0xe8);
            }
        }
        0x51 => {
            e.w32(LB(e) + 0x1c, 0);
            reset(e, 0xac);
            if e.r32(fl(e)) & 7 != 0 {
                and(e, fl(e), 0xffff_fff7);
                reset(e, if w(e, 0x48c) == 1 { 0xec } else { 0xf0 });
            }
            APIinput::InputConfigActivate(e, 7, 1);
        }
        0xc9 => {
            panel_baseFUNC_EpFilesInitCurrentClueTex(e)?;
            APIgameobj::EnableObject(e, ep_obj, 0);
        }
        0xcb => {
            if grp == 5 {
                e.w32(LP(e) + 0x5b0, 0);
            }
            APIgameobj::SetRotation(e, k(e, panel_baseFUNC_MessageHandler_0_25), LB(e), -0.0);
            e.wf(GBP(e, 1), 1.0);
            e.wf(LB(e) + 0x74, 1.0);
            e.w32(LB(e) + 0x34, 1);
            APIgameobj::EnableObject(e, ep_obj, 1);
        }
        0x1f5 => {
            if param_2 == 1 && w(e, 0x4a8) == 1 {
                e.w32(LB(e) + 0x4c, 1);
                panel_baseCB_INGAME_LTRIGGER1(e, 0)?;
            }
        }
        0x210 => e.w32(LB(e) + 0x48, (param_2 == 1) as u32),
        _ => {}
    }
    if grp == 3 {
        return Ok(panel_baseFUNC_MessageHandlerBossMeter(e, param_1, param_2)? as u32);
    }
    Ok(0)
}

/// 0x00022370 (84 bytes) `void panel_baseFUNC_InitSlideIns(void)`
pub fn panel_baseFUNC_InitSlideIns(e: &mut Engine) -> LResult<()> {
    let t = APIextra::TimerGetTime(e, e.r32(lb(e, 0x3f4)));
    e.wf(LB(e) + 0x7c, t);
    Ok(())
}

/// 0x000223c8 (1724 bytes) `undefined4 panel_baseFUNC_PauseOn(int param_1)`
pub fn panel_baseFUNC_PauseOn(e: &mut Engine, param_1: i32) -> LResult<u32> {
    let l = LB(e);
    let w = |e: &Engine, s: usize| e.r32(l + k(e, s));
    let wp = |e: &Engine, s: usize| e.r32(LP(e) + k(e, s));
    let orp = e.alloca(4);
    e.w32(orp, 1);
    APIgameobj::SendMessageTo(e, 0xff00_0001, orp, 0x12, 0, 0);
    e.w32(l + 0x2c, 1);
    let pad = e.r32(GBP(e, 0) + 0x4c) as i32;
    APIinput::InputSetVibration(e, pad, 0, 0.0, 0.0, 0.0, 0.0);
    APIinput::InputSetVibration(e, pad, 1, 0.0, 0.0, 0.0, 0.0);
    LIBApiText::DisableWindow(e);
    APIextra::TimerUnregisterNotify(e, SELF, w(e, panel_baseFUNC_PauseOn_0_1));
    APIinput::InputConfigActivate(e, 10, 0);
    APIinput::InputConfigActivate(e, 7, 0);
    if param_1 == 0 {
        APIinput::InputConfigActivate(e, 8, 1);
        APIinput::InputConfigSetState(e, 8, 0);
        if e.r32(GBP(e, 0) + 0x38) == 0 {
            let v = w(e, panel_baseFUNC_PauseOn_0_3) | 0xc;
            e.w32(l + k(e, panel_baseFUNC_PauseOn_0_2), v);
            APIhud::WindowResetFlags(e, wp(e, panel_baseFUNC_PauseOn_0_4), 0x100);
            APIhud::WindowResetFlags(e, wp(e, panel_baseFUNC_PauseOn_0_5), 0x100);
        }
    }
    // 0x108 / 0x104 are reset twice, as in the original
    for s in [
        panel_baseFUNC_PauseOn_0_6,
        panel_baseFUNC_PauseOn_0_7,
        panel_baseFUNC_PauseOn_0_8,
        panel_baseFUNC_PauseOn_0_9,
        panel_baseFUNC_PauseOn_0_10,
        panel_baseFUNC_PauseOn_0_11,
        panel_baseFUNC_PauseOn_0_12,
        panel_baseFUNC_PauseOn_0_13,
        panel_baseFUNC_PauseOn_0_14,
    ] {
        APIhud::WindowResetFlags(e, w(e, s), 0x100);
    }
    for o in [0xb0, 0xd0, 0xd4, 0xd8, 0xe0, 0xe4, 0xe8] {
        APIhud::WindowSetFlags(e, e.r32(l + o), 0x100);
    }
    if e.r32(GBP(e, 3)) & 0x20000 != 0 {
        APIhud::WindowSetFlags(e, e.r32(l + 0xf4), 0x100);
        APIhud::WindowSetFlags(e, e.r32(l + 0xf8), 0x100);
    }
    APInewSound::Play(e, SELF, 0xc, 0);
    if param_1 == 0 {
        if e.r32(GBP(e, 0) + 0x8c) == 0 {
            panel_baseFUNC_DrawNewMenu(e, 7, 9, 0xc)?;
            APIhud::WindowSetFlags(e, w(e, panel_baseFUNC_PauseOn_0_15), 0x100);
            APIhud::WindowSetFlags(e, w(e, panel_baseFUNC_PauseOn_0_16), 0x100);
        } else {
            panel_baseFUNC_DrawNewMenu(e, 7, 9, 0xd)?;
            APIhud::WindowSetFlags(e, w(e, panel_baseFUNC_PauseOn_0_17), 0x100);
            APIhud::WindowResetFlags(e, w(e, panel_baseFUNC_PauseOn_0_18), 0x100);
        }
    } else {
        APIhud::WindowSetFlags(e, w(e, panel_baseFUNC_PauseOn_0_19), 0x100);
        APIhud::WindowSetFlags(e, wp(e, panel_baseFUNC_PauseOn_0_20), 0x100);
    }
    e.w32(l + 0xc, 1);
    APInewSound::PauseSoundEvents(e, 0x47);
    APIgameobj::SendMessageTo(e, 0x2a00_0030, 0, 0x5c, 5, 0);
    APIinput::InputConfigSetState(e, 7, 0);
    APIextra::TimerReset(e, w(e, panel_baseFUNC_PauseOn_0_21));
    panel_baseFUNC_InitSlideIns(e)?;
    panel_baseFUNC_PlayPauseINAnims(e)?;
    APIgameobj::SetNextState(e, SELF, 9);
    Ok(APIgame::Pause(e, 1))
}

/// 0x00022a88 (1040 bytes) `undefined4 panel_baseFUNC_PauseOff(void)`
/// Shape groups: 0 reference (30 builds); 1 (2f3cf860): with game flags 0x43 shows the normal meters
/// (0xb0/0xe0) instead of swapping to the alternative pair; 2 (6 builds): the meters are only restored when
/// game +8 is k 2_1 (0x2a000006), otherwise 0xb0/0xe0 hidden and 0xe8 shown (twice, as the original;
/// null handles skipped). The pause-end message 0x5c goes to 0x2a000030 / 0x2a000041 / 0x2a000008.
pub fn panel_baseFUNC_PauseOff(e: &mut Engine) -> LResult<u32> {
    let grp = g(e, G_panel_baseFUNC_PauseOff);
    let notify = match grp {
        0 => 0x2a00_0030,
        1 => 0x2a00_0041,
        2 => 0x2a00_0008,
        _ => unported_shape(e, "panel_baseFUNC_PauseOff", G_panel_baseFUNC_PauseOff),
    };
    let hidden = e.r32(LB(e) + 0x44);
    let w = |e: &Engine, o: u32| e.r32(lb(e, o));
    e.w32(lb(e, 0x418), 0xffff_ffff);
    APIinput::InputConfigActivate(e, 8, 0);
    APIinput::InputConfigSetState(e, 8, 0);
    APIinput::InputConfigActivate(e, 7, 1);
    APIinput::InputConfigSetState(e, 7, 0x200_0110);
    let orp = e.alloca(4);
    e.w32(orp, 1);
    APIgameobj::SendMessageTo(e, 0xff00_0001, orp, 0x13, 0, 0);
    LIBApiText::EnableWindow(e);
    for o in [0xfc, 0x100, 0x104, 0x108] {
        APIhud::WindowSetFlags(e, w(e, o), 0x100);
    }
    let kw = e.r32(LP(e) + k(e, panel_baseFUNC_InitKeysStack_0_11));
    APIhud::WindowSetFlags(e, kw, 0x100);
    if hidden == 0 {
        for o in [0xd0, 0xd4, 0xd8] {
            APIhud::WindowResetFlags(e, w(e, o), 0x100);
        }
        if e.r32(GBP(e, 0) + 0x14) & 0x43 == 0 {
            if grp != 2 || e.r32(GBP(e, 0) + 8) == k(e, panel_baseFUNC_PauseOff_2_1) {
                panel_baseFUNC_ShowInGame(e, w(e, 0x41c) as i32)?;
            } else {
                for (o, set) in [(0xb0, true), (0xe0, true), (0xe8, false), (0xe8, false)] {
                    let h = w(e, o);
                    if h != 0 {
                        if set {
                            APIhud::WindowSetFlags(e, h, 0x100);
                        } else {
                            APIhud::WindowResetFlags(e, h, 0x100);
                        }
                    }
                }
            }
        } else if grp == 1 {
            APIhud::WindowResetFlags(e, w(e, 0xb0), 0x100);
            APIhud::WindowResetFlags(e, w(e, 0xe0), 0x100);
        } else {
            APIhud::WindowSetFlags(e, w(e, 0xb0), 0x100);
            APIhud::WindowSetFlags(e, w(e, 0xe0), 0x100);
            APIhud::WindowResetFlags(e, w(e, 0xe4), 0x100);
            APIhud::WindowResetFlags(e, w(e, 0xe8), 0x100);
        }
    }
    if e.r32(GBP(e, 3)) & 0x20000 != 0 {
        APIhud::WindowResetFlags(e, w(e, 0xf4), 0x100);
        APIhud::WindowResetFlags(e, w(e, 0xf8), 0x100);
    }
    e.w32(LB(e) + 0xc, 0);
    APInewSound::ResumeSoundEvents(e, 0x47);
    APIgameobj::SendMessageTo(e, notify, 0, 0x5c, 6, 0);
    APIextra::TimerReset(e, w(e, 0x4a0));
    Ok(APIgame::Pause(e, 0))
}

/// 0x00022e98 (528 bytes) `void panel_baseFUNC_DrawNewMenu(undefined4 param_1,undefined4 param_2,undefined4 param_3)`
pub fn panel_baseFUNC_DrawNewMenu(e: &mut Engine, param_1: u32, param_2: u32, param_3: u32) -> LResult<()> {
    let l = LB(e);
    let kk = |e: &Engine, s: usize| l + k(e, s);
    if e.r32(kk(e, panel_baseFUNC_DrawNewMenu_0_1)) as i32 != -1 {
        let i = e.r32(kk(e, panel_baseFUNC_DrawNewMenu_0_3));
        let w = e.r32(kk(e, panel_baseFUNC_DrawNewMenu_0_2).wrapping_add(i.wrapping_mul(4)));
        APIhud::WindowSetFlags(e, w, 0x100);
    }
    e.w32(kk(e, panel_baseFUNC_DrawNewMenu_0_4), param_1);
    e.w32(l + 0xa8, param_2);
    e.w32(kk(e, panel_baseFUNC_DrawNewMenu_0_5), param_2);
    e.w32(kk(e, panel_baseFUNC_DrawNewMenu_0_6), param_3);
    let i = e.r32(kk(e, panel_baseFUNC_DrawNewMenu_0_8));
    let w = e.r32(kk(e, panel_baseFUNC_DrawNewMenu_0_7).wrapping_add(i.wrapping_mul(4)));
    APIhud::WindowResetFlags(e, w, 0x100);
    let p = LP(e);
    if e.r32(kk(e, panel_baseFUNC_DrawNewMenu_0_9)) == 7 && e.r32(GBP(e, 0) + 0x38) == 0 {
        APIhud::WindowResetFlags(e, e.r32(p + k(e, panel_baseFUNC_DrawNewMenu_0_10)), 0x100);
        APIhud::WindowResetFlags(e, e.r32(p + k(e, panel_baseFUNC_DrawNewMenu_0_11)), 0x100);
    } else {
        APIhud::WindowSetFlags(e, e.r32(p + k(e, panel_baseFUNC_DrawNewMenu_0_12)), 0x100);
        APIhud::WindowSetFlags(e, e.r32(p + k(e, panel_baseFUNC_DrawNewMenu_0_13)), 0x100);
    }
    let a = e.r32(kk(e, panel_baseFUNC_DrawNewMenu_0_14)) as i32;
    let b = e.r32(kk(e, panel_baseFUNC_DrawNewMenu_0_15)) as i32;
    panel_baseFUNC_MenuRefresh(e, a, b)?;
    Ok(())
}

/// 0x000230a8 (668 bytes) `undefined4 panel_baseFUNC_MenuRefresh(int param_1,int param_2)`
pub fn panel_baseFUNC_MenuRefresh(e: &mut Engine, param_1: i32, param_2: i32) -> LResult<u32> {
    let l = LB(e);
    if param_1 > e.r32(l + 0xa8) as i32 {
        e.w32(l + 0xa8, param_2 as u32);
    }
    if param_2 < e.r32(l + 0xa8) as i32 {
        e.w32(l + 0xa8, param_1 as u32);
    }
    // quirk: 0x46c holds the language as a float (panel_baseFUNC_Init) but is read here as an int
    let lang = e.r32(l + k(e, panel_baseFUNC_MenuRefresh_0_2)) as i32;
    let sel = e.r32(l + 0xa8) as i32;
    let p = G(e, S_goPauseSelectorInfo).wrapping_add(lang.wrapping_mul(8).wrapping_add(sel.wrapping_mul(0x18)) as u32);
    let (x, y) = (e.rf(p), e.rf(p.wrapping_add(4)));
    APIhud::WindowSetPos(e, e.r32(l + k(e, panel_baseFUNC_MenuRefresh_0_1)), x, y);
    let mut i = param_1;
    while i <= param_2 {
        // DAT_0001002c: text id field of the pause window specs table (stride 0x60)
        let txt = e.r32(img(e, (0x1002c + i * 0x60) as u32));
        if txt != 0 {
            // the C passes a 4th, duplicate colour argument; the API takes three
            if i == e.r32(l + 0xa8) as i32 {
                let w = e.r32(l + k(e, panel_baseFUNC_MenuRefresh_0_4) + (i * 4) as u32);
                APIhud::TextWindowSetText(e, w, txt, &[6, 6]);
            } else {
                let w = e.r32(l + k(e, panel_baseFUNC_MenuRefresh_0_5) + (i * 4) as u32);
                APIhud::TextWindowSetText(e, w, txt, &[7, 7]);
            }
        }
        i += 1;
    }
    let mut r = 0;
    if e.r32(GBP(e, 0) + 0x38) != 0 {
        let w = e.r32(l + k(e, panel_baseFUNC_MenuRefresh_0_6));
        let txt = e.r32(img(e, 0x1044c));
        r = APIhud::TextWindowSetText(e, w, txt, &[8, 8]);
    }
    Ok(r)
}

/// 0x00023348 (2032 bytes) `int panel_baseFUNC_MainMenuConfirm(void)`
/// Pause main menu (entries 9..=0xe). The int result is the last call's v0 in the C (callers ignore it);
/// for case 9 (TimerGetTime returns in f0, v0 undefined) the selection-9 value is kept.
/// Quirk kept: LB+0x450 is set to 1 just before the tab test, so tabs 2/3 are dead code.
pub fn panel_baseFUNC_MainMenuConfirm(e: &mut Engine) -> LResult<i32> {
    let l = LB(e);
    let kk = |e: &Engine, s: usize| l + k(e, s);
    let w = |e: &Engine, s: usize| e.r32(l + k(e, s));
    let wp = |e: &Engine, s: usize| e.r32(LP(e) + k(e, s));
    let sel = e.r32(l + 0xa8) as i32;
    let mut r = sel.wrapping_sub(9);
    match sel {
        9 => {
            if 0.25 < APIextra::TimerGetTime(e, w(e, panel_baseFUNC_MainMenuConfirm_0_1)) {
                APInewSound::Play(e, SELF, 0xd, 0);
                APIinput::InputConfigSetState(e, 8, 0);
                APIextra::TimerReset(e, w(e, panel_baseFUNC_MainMenuConfirm_0_2));
                panel_baseFUNC_PlayPauseOUTAnims(e)?;
                r = LB(e) as i32;
                e.w32(LB(e) + 0x2c, 0);
            }
        }
        10 => {
            APInewSound::Play(e, SELF, 0xd, 0);
            panel_baseFUNC_DrawNewMenu(e, 0x14, 0x16, 0x19)?;
            panel_baseFUNC_OptionsMenuUpdate(e)?;
            APIinput::InputConfigActivate(e, 8, 0);
            APIinput::InputConfigActivate(e, 10, 1);
            APIinput::InputConfigSetState(e, 10, 0x196);
            r = APIgameobj::SetNextState(e, SELF, 5) as i32;
        }
        0xb => {
            APInewSound::Play(e, SELF, 0xd, 0);
            e.w32(kk(e, panel_baseFUNC_MainMenuConfirm_0_3), 1);
            APIanim::StopAnim(e, SELF, k(e, panel_baseFUNC_MainMenuConfirm_0_4));
            APIanim::Play(e, SELF, k(e, panel_baseFUNC_MainMenuConfirm_0_5), 1, 0.0);
            if e.r32(kk(e, panel_baseFUNC_MainMenuConfirm_0_6)) == 1 || e.r32(kk(e, panel_baseFUNC_MainMenuConfirm_0_7)) == 0 {
                APIanim::Play(e, SELF, k(e, panel_baseFUNC_MainMenuConfirm_0_8), 1, 0.0);
                APIanim::Play(e, SELF, k(e, panel_baseFUNC_MainMenuConfirm_0_9), 1, 0.0);
                APIanim::Play(e, SELF, k(e, panel_baseFUNC_MainMenuConfirm_0_10), 1, 0.0);
                APIhud::WindowSetFlags(e, w(e, panel_baseFUNC_MainMenuConfirm_0_11), 0x100);
                APIhud::WindowSetFlags(e, w(e, panel_baseFUNC_MainMenuConfirm_0_12), 0x100);
                APIhud::WindowSetFlags(e, w(e, panel_baseFUNC_MainMenuConfirm_0_13), 0x100);
                APIhud::WindowResetFlags(e, w(e, panel_baseFUNC_MainMenuConfirm_0_14), 0x100);
            } else if e.r32(kk(e, panel_baseFUNC_MainMenuConfirm_0_15)) == 2 {
                APIanim::Play(e, SELF, k(e, panel_baseFUNC_MainMenuConfirm_0_16), 1, 0.0);
                APIanim::Play(e, SELF, k(e, panel_baseFUNC_MainMenuConfirm_0_17), 1, 0.0);
                APIanim::Play(e, SELF, k(e, panel_baseFUNC_MainMenuConfirm_0_18), 1, 0.0);
                if e.r32(GBP(e, 3) + 0xc) & 0x3e0 == 0 {
                    e.w32(kk(e, panel_baseFUNC_MainMenuConfirm_0_19), 99);
                } else {
                    // the C passes a 4th, duplicate colour argument; the API takes three
                    APIhud::TextWindowSetText(e, w(e, panel_baseFUNC_MainMenuConfirm_0_20), 0x138, &[6, 6]);
                    // _DAT_0001230c: text id of a pause window spec record (same address in all 37 builds)
                    e.w32(img(e, 0x1230c), 0x138);
                    APIhud::WindowResetFlags(e, w(e, panel_baseFUNC_MainMenuConfirm_0_21), 0x100);
                    APIhud::WindowResetFlags(e, w(e, panel_baseFUNC_MainMenuConfirm_0_22), 0x100);
                    APIhud::WindowResetFlags(e, w(e, panel_baseFUNC_MainMenuConfirm_0_23), 0x100);
                    e.w32(kk(e, panel_baseFUNC_MainMenuConfirm_0_24), 1);
                }
                panel_baseFUNC_EpFilesClueTexFade(e)?;
                panel_baseFUNC_EpFilesClueUpdate(e)?;
                // fake stick record: y = -0.16 (word +0 left uninitialised in the C)
                let stick = e.alloca(8);
                e.w32(stick, 0);
                e.w32(stick + 4, 0xbe23_d70a);
                panel_baseFUNC_EpFilesUPDOWN(e, stick)?;
            } else if e.r32(kk(e, panel_baseFUNC_MainMenuConfirm_0_25)) == 3 {
                APIhud::WindowResetFlags(e, wp(e, panel_baseFUNC_MainMenuConfirm_0_26), 0x100);
                APIanim::Play(e, SELF, k(e, panel_baseFUNC_MainMenuConfirm_0_27), 1, 0.0);
                APIanim::Play(e, SELF, k(e, panel_baseFUNC_MainMenuConfirm_0_28), 1, 0.0);
                APIanim::Play(e, SELF, k(e, panel_baseFUNC_MainMenuConfirm_0_29), 1, 0.0);
                APIhud::WindowSetFlags(e, w(e, panel_baseFUNC_MainMenuConfirm_0_30), 0x100);
                APIhud::WindowSetFlags(e, w(e, panel_baseFUNC_MainMenuConfirm_0_31), 0x100);
                APIhud::WindowSetFlags(e, w(e, panel_baseFUNC_MainMenuConfirm_0_32), 0x100);
                panel_baseFUNC_EpFilesMonstersUpdate(e)?;
            }
            APIhud::WindowSetFlags(e, wp(e, panel_baseFUNC_MainMenuConfirm_0_33), 0x100);
            APIhud::WindowSetFlags(e, wp(e, panel_baseFUNC_MainMenuConfirm_0_34), 0x100);
            e.w32(kk(e, panel_baseFUNC_MainMenuConfirm_0_35), 0);
            APIinput::InputConfigSetState(e, 8, 0x196);
            panel_baseFUNC_DrawNewMenu(e, 0x45, 0x46, 0x5a)?;
            r = panel_baseFUNC_EpFilesObjUpdate(e)?;
        }
        0xc => {
            APInewSound::Play(e, SELF, 0xd, 0);
            panel_baseFUNC_DrawNewMenu(e, 0x30, 0x31, 0x32)?;
            r = APIinput::InputConfigSetState(e, 8, 0x194) as i32;
        }
        0xd => {
            panel_baseFUNC_DrawNewMenu(e, 0x79, 0x7a, 0x7f)?;
            r = APIinput::InputConfigSetState(e, 8, 0x196) as i32;
        }
        0xe => {
            panel_baseFUNC_DrawNewMenu(e, 0x29, 0x2a, 0x2f)?;
            r = APIinput::InputConfigSetState(e, 10, 0x196) as i32;
        }
        _ => {}
    }
    Ok(r)
}

/// 0x00023b38 (412 bytes) `void panel_baseFUNC_RerunMenuConfirm(void)`
/// Groups 0 and 1 (4ec0e7c2, shifted layout) differ only in per-build slots / object ids / static address.
pub fn panel_baseFUNC_RerunMenuConfirm(e: &mut Engine) -> LResult<()> {
    if g(e, G_panel_baseFUNC_RerunMenuConfirm) > 1 {
        unported_shape(e, "panel_baseFUNC_RerunMenuConfirm", G_panel_baseFUNC_RerunMenuConfirm);
    }
    if e.r32(LB(e) + 0xa8) == 0x13 {
        panel_baseFUNC_PauseOff(e)?;
        let l = LB(e);
        let w = |e: &Engine, s: usize| e.r32(l + k(e, s));
        e.w32(l + k(e, panel_baseFUNC_RerunMenuConfirm_0_1), 1);
        APIextra::TimerReset(e, w(e, panel_baseFUNC_RerunMenuConfirm_0_2));
        APIextra::TimerSetFrequency(e, w(e, panel_baseFUNC_RerunMenuConfirm_0_3), 1.0);
        APIextra::TimerRegisterNotify(e, SELF, w(e, panel_baseFUNC_RerunMenuConfirm_0_4), 0x1b);
        // moCameraParams_138 = function static moCameraParams + 0x38
        let p = G(e, S_moCameraParams) + 0x38;
        e.w32(p, 0x62);
        e.w32(p + 4, 0);
        e.w32(p + 8, 1);
        e.w32(p + 0xc, 0);
        APIinput::InputConfigActivate(e, 7, 0);
        APIinput::InputConfigActivate(e, 8, 0);
        APIgameobj::SendMessageTo(e, k(e, ID_2a00001e), 0, 0x54, 0, p);
        panel_baseFUNC_MessageHandler(e, 0x1e, 0, 0)?;
    }
    Ok(())
}

/// 0x00023cd8 (308 bytes) `void panel_baseFUNC_EpFilesInitCurrentClueTex(void)`
/// Current clue texture index (+0x494) = first unlocked clue (GB3+0xc bits 0x20..0x200 -> 1..5), else 99.
pub fn panel_baseFUNC_EpFilesInitCurrentClueTex(e: &mut Engine) -> LResult<()> {
    let f = e.r32(GBP(e, 3) + 0xc);
    let v = if f & 0x20 != 0 {
        1
    } else if f & 0x40 != 0 {
        2
    } else if f & 0x80 != 0 {
        3
    } else if f & 0x100 != 0 {
        4
    } else if f & 0x200 != 0 {
        5
    } else {
        99
    };
    e.w32(lb(e, 0x494), v);
    Ok(())
}

/// 0x00023e10 (288 bytes) `void panel_baseFUNC_InitLevelBuffer(void)`
pub fn panel_baseFUNC_InitLevelBuffer(e: &mut Engine) -> LResult<()> {
    for o in [0x24, 0x3c, 0x40, 0x28, 0x20, 0x30, 0x18, 0x1c, 0x44, 0x48, 0x34, 0x38, 0x2c, 0x14, 0x4c, 0x50] {
        let b = GBP(e, 1);
        e.w32(b + o, 0);
    }
    Ok(())
}

/// 0x00023f30 (568 bytes) `void panel_baseFUNC_YesNoConfirm(void)`
/// Groups 0 and 1 (4ec0e7c2, shifted layout) differ only in per-build slots / object ids / static address.
/// Note the order differs from RerunMenuConfirm: the camera message is sent before the input deactivation.
pub fn panel_baseFUNC_YesNoConfirm(e: &mut Engine) -> LResult<()> {
    if g(e, G_panel_baseFUNC_YesNoConfirm) > 1 {
        unported_shape(e, "panel_baseFUNC_YesNoConfirm", G_panel_baseFUNC_YesNoConfirm);
    }
    APIinput::InputConfigSetState(e, 8, 0x192);
    let l = LB(e);
    let sel = e.r32(l + 0xa8) as i32;
    if sel == 0x31 {
        panel_baseFUNC_DrawNewMenu(e, 7, 9, 0xc)?;
        return Ok(());
    }
    if sel < 0x32 {
        if sel == 0x11 {
            panel_baseFUNC_DrawNewMenu(e, 0x12, 0x13, 0x13)?;
            return Ok(());
        }
    } else if sel == 0x32 {
        let w = |e: &Engine, s: usize| e.r32(l + k(e, s));
        e.w32(l + k(e, panel_baseFUNC_YesNoConfirm_0_1), 2);
        APIextra::TimerReset(e, w(e, panel_baseFUNC_YesNoConfirm_0_2));
        APIextra::TimerSetFrequency(e, w(e, panel_baseFUNC_YesNoConfirm_0_3), 1.0);
        APIextra::TimerRegisterNotify(e, SELF, w(e, panel_baseFUNC_YesNoConfirm_0_4), 0x1b);
        // moCameraParams_148 = function static moCameraParams_1 + 0x38
        let p = G(e, S_moCameraParams_1) + 0x38;
        e.w32(p, 0x62);
        e.w32(p + 4, 0);
        e.w32(p + 8, 1);
        e.w32(p + 0xc, 0);
        APIgameobj::SendMessageTo(e, k(e, ID_2a00001e), 0, 0x54, 0, p);
        APIinput::InputConfigActivate(e, 7, 0);
        APIinput::InputConfigActivate(e, 8, 0);
        panel_baseFUNC_MessageHandler(e, 0x1e, 0, 0)?;
        return Ok(());
    }
    panel_baseFUNC_DrawNewMenu(e, 7, 9, 0xc)?;
    Ok(())
}

/// 0x00024168 (1008 bytes) `void panel_baseFUNC_PlayPauseINAnims(void)`
/// Anim ids are per-build literals (k slots). 0x2a100014 = the ingredient/sandwich object.
pub fn panel_baseFUNC_PlayPauseINAnims(e: &mut Engine) -> LResult<()> {
    let kk = |e: &Engine, s: usize| k(e, s);
    if e.r32(LB(e) + 0xc) != 0 {
        APIanim::StopAnim(e, SELF, kk(e, panel_baseFUNC_PlayPauseINAnims_0_1));
        APIanim::Play(e, SELF, kk(e, panel_baseFUNC_PlayPauseINAnims_0_2), 1, 0.0);
        APIanim::Play(e, SELF, kk(e, panel_baseFUNC_PlayPauseINAnims_0_3), 2, 0.0);
        APIanim::Play(e, SELF, kk(e, panel_baseFUNC_PlayPauseINAnims_0_4), 2, 0.0);
        APIgameobj::RegisterNotify(e, SELF, kk(e, panel_baseFUNC_PlayPauseINAnims_0_5), 1, 0x18);
    }
    let (f428, f424) = (lb(e, 0x428), lb(e, 0x424));
    if e.r32(f428) & 8 != 0 && e.r32(f424) & 8 == 0 {
        panel_baseFUNC_InitIngredientWin(e)?;
        APIanim::Play(e, SELF, kk(e, panel_baseFUNC_PlayPauseINAnims_0_8), 1, 0.0);
        APIanim::Play(e, SELF, kk(e, panel_baseFUNC_PlayPauseINAnims_0_9), 1, 0.0);
        if e.r32(LB(e) + 0xc) == 0 {
            APIgameobj::SendMessageTo(e, 0x2a10_0014, 0, 0xe2, 0, 0);
        } else {
            for s in [
                panel_baseFUNC_PlayPauseINAnims_0_10,
                panel_baseFUNC_PlayPauseINAnims_0_11,
                panel_baseFUNC_PlayPauseINAnims_0_12,
                panel_baseFUNC_PlayPauseINAnims_0_13,
                panel_baseFUNC_PlayPauseINAnims_0_14,
            ] {
                APIanim::Play(e, 0x2a10_0014, kk(e, s), 1, 0.0);
            }
        }
        let v = e.r32(f428) & 0xffff_fff7;
        e.w32(f428, v);
        let v = e.r32(f424) | 8;
        e.w32(f424, v);
    }
    if e.r32(f428) & 4 != 0 && e.r32(f424) & 4 == 0 {
        let lp_off = kk(e, panel_baseFUNC_PlayPauseINAnims_0_21);
        for i in 0..5u32 {
            if e.r32(GBP(e, 1) + 0x54 + i * 4) != 0 {
                let h = e.r32(LP(e) + lp_off + (i + 1) * 4);
                APIhud::WindowResetFlags(e, h, 0x100);
            }
        }
        let v = e.r32(f428) & 0xffff_fffb;
        e.w32(f428, v);
        let v = e.r32(f424) | 4;
        e.w32(f424, v);
        e.w32(LB(e) + 0x14, 1);
    }
    Ok(())
}

/// 0x00024558 (592 bytes) `void panel_baseFUNC_PlayPauseOUTAnims(void)`
pub fn panel_baseFUNC_PlayPauseOUTAnims(e: &mut Engine) -> LResult<()> {
    let kk = |e: &Engine, s: usize| k(e, s);
    if e.r32(LB(e) + 0xc) != 0 {
        APIanim::StopAnim(e, SELF, kk(e, panel_baseFUNC_PlayPauseOUTAnims_0_1));
        APIanim::StopAnim(e, SELF, kk(e, panel_baseFUNC_PlayPauseOUTAnims_0_2));
        APIanim::Play(e, SELF, kk(e, panel_baseFUNC_PlayPauseOUTAnims_0_3), 1, 0.0);
        APIgameobj::RegisterNotify(e, SELF, kk(e, panel_baseFUNC_PlayPauseOUTAnims_0_4), 1, 0x19);
        APInewSound::Play(e, SELF, 0xb, 0);
    }
    let f424 = lb(e, 0x424);
    if e.r32(f424) & 8 != 0 {
        APIanim::Play(e, SELF, kk(e, panel_baseFUNC_PlayPauseOUTAnims_0_6), 1, 0.0);
        APIgameobj::SendMessageTo(e, 0x2a10_0014, 0, 0xe1, 0, 0);
        APIanim::Play(e, SELF, kk(e, panel_baseFUNC_PlayPauseOUTAnims_0_7), 1, 0.0);
        APIgameobj::RegisterNotify(e, SELF, kk(e, panel_baseFUNC_PlayPauseOUTAnims_0_8), 1, 0x1e);
    }
    if e.r32(f424) & 4 != 0 {
        let lp_off = kk(e, panel_baseFUNC_PlayPauseOUTAnims_0_10);
        for i in 0..5u32 {
            if e.r32(GBP(e, 1) + 0x54 + i * 4) != 0 {
                let h = e.r32(LP(e) + lp_off + (i + 1) * 4);
                APIhud::WindowSetFlags(e, h, 0x100);
            }
        }
    }
    e.w32(LB(e) + 0x14, 0);
    Ok(())
}

/// 0x000247a8 (2836 bytes) `void panel_baseFUNC_CreateInput_Base(void)`
/// Configs 7 (in game: pause 0x34, triangle 0x3c, L1 0x41) and 8 (menus: + sticks 0x3f/0x40, confirm 0x39);
/// the pad is game buffer 0 +0x4c (set to 0x10 here).
pub fn panel_baseFUNC_CreateInput_Base(e: &mut Engine) -> LResult<()> {
    APIinput::InputConfigAdd(e, 7);
    e.w32(GBP(e, 0) + 0x4c, 0x10);
    let msg = lit(e, "Controller Used in Base Panel: %d\n");
    let pr = |e: &mut Engine| {
        let p = e.r32(GBP(e, 0) + 0x4c);
        APIextra::null_printf(e, msg, p);
    };
    let pad = |e: &Engine| e.r32(GBP(e, 0) + 0x4c) as i32;
    // (config, action, [(device, pad (-1 = game pad), a, b, c, d)], (context cb, ctx id, ctx flags), state)
    type Act = (i32, i32, &'static [(i32, i32, i32, i32, i32, i32)], (i32, i32, u32), u32);
    let act = |e: &mut Engine, (c, a, devs, (cb, id, fl), st): Act| {
        APIinput::InputActionAdd(e, c, a, 0);
        for &(d, p, x, y, z, w) in devs {
            let p = if p < 0 { pad(e) } else { p };
            APIinput::InputDeviceAdd(e, c, a, d, p, x, y, z, w);
        }
        APIinput::InputContextAdd(e, SELF, cb as u32, c, a, id, fl as i32);
        APIinput::InputContextSetState(e, c, a, id, st);
    };
    const PAUSE: &[(i32, i32, i32, i32, i32, i32)] = &[(0x32, -1, 0, 2, 0, 0xb), (0x33, 1, 1, 2, 0, 0x9c)];
    const TRI: &[(i32, i32, i32, i32, i32, i32)] = &[(0x40, -1, 0, 2, 0, 3), (0x40, 1, 1, 2, 0, 0xe)];
    const L1: &[(i32, i32, i32, i32, i32, i32)] = &[(0x43, -1, 0, 2, 0, 4), (0x43, 1, 1, 2, 0, 0x15)];
    pr(e);
    act(e, (7, 0x34, PAUSE, (0xb, 0x1c, 8), 0x100));
    pr(e);
    act(e, (7, 0x3c, TRI, (0x1a, 0x24, 8), 0x10));
    pr(e);
    pr(e);
    act(e, (7, 0x41, L1, (0x20, 0x31, 8), 0x200_0000));
    APIinput::InputConfigAdd(e, 8);
    act(e, (8, 0x34, PAUSE, (0xc, 0x1c, 8), 0x100));
    act(e, (8, 0x3f, &[(4, -1, 0, 1, 0, 0x8001), (0xd, -1, 1, 1, -100, 0xd), (0xe, -1, 1, 1, 100, 0xc), (0xd, 1, 2, 1, -100, 0xd0), (0xe, 1, 2, 1, 100, 200)], (0xd, 2, 0x30_0000), 2));
    act(e, (8, 0x40, &[(3, -1, 0, 0, 0, 0x8000), (0xb, -1, 1, 0, -100, 0xe), (0xc, -1, 1, 0, 100, 0xf), (0xb, 1, 2, 0, -100, 0xcb), (0xc, 1, 2, 0, 100, 0xcd)], (0xe, 0x2b, 0xc0_0000), 4));
    act(e, (8, 0x39, &[(0x3d, -1, 0, 2, 0, 0), (0x3d, 1, 1, 2, 0, 0x39)], (0xf, 0x1e, 8), 0x80));
    act(e, (8, 0x3c, TRI, (0x10, 0x24, 8), 0x10));
    act(e, (8, 0x41, L1, (0x20, 0x31, 8), 0x200_0000));
    APIinput::InputConfigActivate(e, 8, 0);
    APIinput::InputConfigSetState(e, 8, 0);
    APIinput::InputConfigActivate(e, 7, 0);
    APIinput::InputConfigSetState(e, 7, 0x200_0110);
    panel_baseFUNC_CreateInput_Options(e)
}

/// 0x000252c0 (2928 bytes) `undefined4 panel_baseFUNC_CreateHudWins(int param_1,int param_2,uint param_3)`
/// Creates `param_3` windows from the 0x60-byte records at `param_1` (see PORT_NOTES Step 5m) into the
/// handle array `param_2`, then hides those flagged +0x24. Note: TextWindowSetText is variadic in the original
/// (0..3 extra byte args, 0xff = absent): all are forwarded (WP-P4).
pub fn panel_baseFUNC_CreateHudWins(e: &mut Engine, param_1: i32, param_2: i32, param_3: u32) -> LResult<u32> {
    let (t, h) = (param_1 as u32, param_2 as u32);
    for i in 0..param_3 {
        let r = t + i * 0x60;
        let hp = h + i * 4;
        e.w32(hp, 0x3b9a_c9f7);
        let parent = match e.r32(r + 8) {
            2000 => e.r32(LB(e) + k(e, panel_baseFUNC_CreateHudWins_0_1)),
            1000 => e.r32(LB(e) + 0xac),
            0xffff_ffff => 0xffff_ffff,
            p => e.r32(h + p.wrapping_mul(4)),
        };
        let w = APIhud::WindowAdd(e, e.r32(r + 4), parent, e.r32(r + 0xc), e.r32(r + 0x20));
        e.w32(hp, w);
        APIhud::WindowSetDims(e, w, e.rf(r + 0x10), e.rf(r + 0x14));
        APIhud::WindowSetPos(e, w, e.rf(r + 0x18), e.rf(r + 0x1c));
        APIhud::WindowSetName(e, w, e.r32(r));
        match e.r32(r + 4) {
            0 => {
                if e.r32(r + 0xc) & 0x800 != 0 {
                    let bg = if (0x23..=0x25).contains(&i) { 2 } else { 5 };
                    APIhud::WindowSetBackGround(e, w, bg, e.rf(r + 0x10), e.rf(r + 0x14));
                }
            }
            1 => {
                APIhud::TextWindowSetFont(e, w, e.r32(r + 0x28));
                let txt = e.r32(r + 0x2c);
                let (b0, b1, b2) = (e.r8(r + 0x30), e.r8(r + 0x31), e.r8(r + 0x32));
                // WP-P4: 0..3 extra byte args exactly as the original (0xff = absent)
                if b2 != 0xff {
                    APIhud::TextWindowSetText(e, w, txt, &[b0 as u32, b1 as u32, b2 as u32]);
                } else if b1 != 0xff {
                    APIhud::TextWindowSetText(e, w, txt, &[b0 as u32, b1 as u32]);
                } else if b0 != 0xff {
                    APIhud::TextWindowSetText(e, w, txt, &[b0 as u32]);
                } else {
                    APIhud::TextWindowSetColor(e, w, 6);
                    APIhud::TextWindowSetText(e, w, txt, &[]);
                }
            }
            2 => {
                APIhud::Window2DSetTexture(e, w, r + 0x33);
                APIhud::Window2DSetTiling(e, w, e.rf(r + 0x48), e.rf(r + 0x4c));
                APIhud::Window2DSetColor(e, w, e.r32(r + 0x50));
            }
            3 => {
                let obj = e.r32(r + 0x54);
                APIhud::Window3DSetObj(e, w, obj, e.r32(r + 0x58));
                let anim = e.r32(r + 0x5c);
                if anim != 0 {
                    APIanim::Play(e, obj, anim, 1, 0.0);
                }
                if obj != SELF {
                    APIgameobj::SendMessageTo(e, obj, 0, 0x1a, w, 0);
                }
            }
            _ => {}
        }
    }
    for i in 0..param_3 {
        if e.r32(t + i * 0x60 + 0x24) != 0 {
            APIhud::WindowSetFlags(e, e.r32(h + i * 4), 0x100);
        }
    }
    let p = e.alloca(0x10);
    let w = e.r32(LB(e) + 0xf4);
    Ok(APIhud::WindowGetPos(e, w, p, LB(e) + 0x9c))
}

/// 0x00025e30 (236 bytes) `float panel_baseFUNC_ComputeParabolicPosFromTime(float param_1,float param_2)`
pub fn panel_baseFUNC_ComputeParabolicPosFromTime(e: &mut Engine, param_1: f32, param_2: f32) -> LResult<f32> {
    Ok(if param_1 < param_2 {
        0.0
    } else if param_2 < 0.0 {
        1.0
    } else {
        (-2.0 / param_1) * param_2 + 1.0 + (-(-2.0 / param_1) / param_1) * param_2 * param_2 * 0.5
    })
}

/// 0x00025f20 (540 bytes) `void panel_baseFUNC_InitKeysStack(void)`
/// Item -> 3D object id per build from keys.rs (each build's own switch / if-chain).
pub fn panel_baseFUNC_InitKeysStack(e: &mut Engine) -> LResult<()> {
    let (base, map) = keys::KEYS[e.rt.cur_build()];
    let lp_off = k(e, panel_baseFUNC_InitKeysStack_0_11);
    let mut obj = 0u32;
    for i in 0..5u32 {
        let v = e.r32(GBP(e, 1) + 0x54 + i * 4);
        if let Some(&(_, o)) = map.iter().find(|&&(x, _)| x == v) {
            obj = o;
        }
        let r = img(e, base) + (i + 1) * 0x60;
        e.w32(r + 8, 0);
        e.w32(r + 4, obj);
        if obj != 0 {
            APIhud::Window3DSetObj(e, e.r32(LP(e) + lp_off + (i + 1) * 4), e.r32(r), obj);
        }
    }
    Ok(())
}

/// 0x00026140 (916 bytes) `uint panel_baseFUNC_SetItemWinPos(int param_1,int param_2)`
/// Assumes each build's switch here equals its InitKeysStack switch (keys.rs); values outside it print
/// "invalid _iTypeTraitement" and return, as the original's default case.
pub fn panel_baseFUNC_SetItemWinPos(e: &mut Engine, param_1: i32, param_2: i32) -> LResult<u32> {
    let items = GBP(e, 1) + 0x54;
    if param_2 == 0 {
        let (base, map) = keys::KEYS[e.rt.cur_build()];
        let Some(&(_, obj)) = map.iter().find(|&&(x, _)| x == param_1 as u32) else {
            let m = lit(e, "invalid _iTypeTraitement\n");
            return Ok(APIextra::null_printf(e, m, 0));
        };
        for i in 0..5 {
            let v = e.r32(items + i * 4);
            if v == param_1 as u32 {
                return Ok(v);
            }
        }
        let lp_off = k(e, panel_baseFUNC_InitKeysStack_0_11);
        for i in 0..5u32 {
            if e.r32(items + i * 4) == 0 {
                e.w32(items + i * 4, param_1 as u32);
                let r = img(e, base) + (i + 1) * 0x60;
                e.w32(r + 8, 0);
                e.w32(r + 4, obj);
                APIhud::Window3DSetObj(e, e.r32(LP(e) + lp_off + (i + 1) * 4), e.r32(r), obj);
                break;
            }
        }
        Ok(APIhud::WindowSetFlags(e, e.r32(LP(e) + lp_off), 0x100))
    } else {
        for i in 0..5u32 {
            if e.r32(items + i * 4) == param_1 as u32 {
                panel_baseFUNC_ResetHandleTable(e, param_1 as i64 as u64, i)?;
                return Ok(0); // (the original returns ResetHandleTable's undefined v0)
            }
        }
        Ok(5)
    }
}

/// 0x000264d8 (752 bytes) `void panel_baseFUNC_ResetHandleTable(undefined8 param_1,uint param_2)`
/// Removes key item `param_2` from the 5-slot keys stack (hero buffer +0x54), shifting the following ones
/// and their 3D windows down (records of the build's keys window table, keys.rs base), then hides the stack.
pub fn panel_baseFUNC_ResetHandleTable(e: &mut Engine, param_1: u64, param_2: u32) -> LResult<()> {
    let items = GBP(e, 1) + 0x54;
    let base = img(e, keys::KEYS[e.rt.cur_build()].0);
    let lp_off = k(e, panel_baseFUNC_InitKeysStack_0_11);
    if e.r32(items + (param_2 + 1) * 4) == 0 {
        e.w32(items + param_2 * 4, 0);
    } else {
        let mut i = param_2;
        while i < 4 {
            let v = e.r32(items + (i + 1) * 4);
            e.w32(items + i * 4, v);
            let (r, r2) = (base + (i + 1) * 0x60, base + (i + 2) * 0x60);
            let a = e.r32(r2 + 4);
            e.w32(r + 4, a);
            let b = e.r32(r2 + 8);
            e.w32(r + 8, b);
            let h = e.r32(LP(e) + lp_off + (i + 1) * 4);
            APIhud::Window3DSetObj(e, h, e.r32(r), e.r32(r + 4));
            e.w32(items + (i + 1) * 4, 0);
            i += 1;
        }
    }
    for i in 0..5 {
        let h = e.r32(LP(e) + lp_off + i * 4);
        APIhud::WindowSetFlags(e, h, 0x100);
    }
    Ok(())
}

/// 0x000267c8 (448 bytes) `void panel_baseFUNC_UpdateTomeMeter(void)`
/// Rotates the Tome of Doom meter needle (window k slot, 0xf00062a in the ref build) by the change of the
/// book angle GB1+4 (degrees, 0..180) since the last update (local+0x88 = pLocalData_BasePanel[0xb].y; DTVector3d is 12 bytes).
pub fn panel_baseFUNC_UpdateTomeMeter(e: &mut Engine) -> LResult<()> {
    tome_meter_step(e, panel_baseFUNC_UpdateTomeMeter_0_1, panel_baseFUNC_UpdateTomeMeter_0_2);
    Ok(())
}
/// Shared body of UpdateTomeMeter / ToDMeterChangeControl(3) (the original inlines it twice).
fn tome_meter_step(e: &mut Engine, k_up: usize, k_down: usize) {
    let lb = LB(e);
    let cur = e.rf(GBP(e, 1) + 4);
    let last = e.rf(lb + 0x88);
    if last <= cur || cur < 0.0 {
        if last < cur && cur <= 180.0 {
            e.wf(lb + 0x88, cur);
            APIgameobj::Rotate(e, k(e, k_up), lb, -(cur - last) * 0.017453292);
        }
    } else {
        e.wf(lb + 0x88, cur);
        APIgameobj::Rotate(e, k(e, k_down), lb, (last - cur) * 0.017453292);
    }
}

/// 0x00027060 (136 bytes) `void panel_baseFUNC_UpdateMeters(void)`
/// local+0x474 = meters shown (1..4), +0x478 = requested (offsets shift by 4 in 6 builds).
pub fn panel_baseFUNC_UpdateMeters(e: &mut Engine) -> LResult<()> {
    let lb = LB(e);
    if e.r32(lb + k(e, panel_baseFUNC_UpdateMeters_0_1)) == e.r32(lb + k(e, panel_baseFUNC_UpdateMeters_0_2)) {
        e.w32(lb + k(e, panel_baseFUNC_UpdateMeters_0_3), 0);
    } else {
        let v = e.r32(lb + k(e, panel_baseFUNC_UpdateMeters_0_4));
        panel_baseFUNC_MoveMetersOut(e, v)?;
    }
    Ok(())
}

/// 0x000270e8 (624 bytes) `void panel_baseFUNC_UpdateCoolMeter(void)`
/// Courage meter: local+0x74 ([9].z) follows *GB1 (0..1) by steps of 0.01 per call, rotating its needle 1.8 deg
/// per step; out-of-range targets are clamped back into GB1.
pub fn panel_baseFUNC_UpdateCoolMeter(e: &mut Engine) -> LResult<()> {
    let lb = LB(e);
    let gb1 = GBP(e, 1);
    let shown = e.rf(lb + 0x74);
    let d = e.rf(gb1) - shown;
    if (d < 0.0 && 0.0 < shown) || (0.0 < d && shown < 1.0) {
        let step: f32 = if d <= -0.001 {
            -0.01
        } else if d < 0.001 {
            let v = e.rf(gb1);
            e.wf(lb + 0x74, v);
            return Ok(());
        } else {
            0.01
        };
        e.wf(lb + 0x74, shown + step);
        APIgameobj::Rotate(e, k(e, panel_baseFUNC_UpdateCoolMeter_0_1), lb, step * 1.8 * 100.0 * 0.017453292);
        if 0.0 <= e.rf(gb1) {
            panel_baseFUNC_TexFaderControl(e)?;
        }
    } else if 0.0 < d {
        e.w32(gb1, 0x3f80_0000);
    } else if d < 0.0 {
        e.w32(gb1, 0);
    }
    Ok(())
}

/// `_sTexFaderParams` as the original builds it on the stack: +0 mode, +4 speed, +8 from texture,
/// +0x14 to texture, +0x20..0x2c colour (1,0,0,1), +0x30 (ToD fader only).
pub(crate) fn tex_fader_params(e: &mut Engine, speed: u32, from: Ptr, to: Ptr) -> Ptr {
    let p = e.alloca(0x34);
    for i in 0..0x34 / 4 {
        e.w32(p + i * 4, 0);
    }
    e.w32(p, 3);
    e.w32(p + 4, speed);
    e.w32(p + 8, from);
    e.w32(p + 0x14, to);
    e.w32(p + 0x20, 0x3f80_0000);
    e.w32(p + 0x2c, 0x3f80_0000);
    p
}

/// 0x00027358 (1052 bytes) `void panel_baseFUNC_TexFaderControl(void)`
/// Scooby/Shaggy portrait: local+0x74 = displayed courage; when +0x30 (fader ready) is set, fades from the
/// current face (+0x45c: 1 normal, 2 nervous, 3 scared) to the one of the courage band (<0.33 scared,
/// <0.66 nervous, else normal) and records it in +0x460. The original compares the two literal pointers
/// to skip a no-op fade; the "from" texture is the band's own one only when +0x45c is out of range.
/// Note: the uninitialised stack words of the params (+0xc..+0x10, +0x18..+0x1c, +0x30) are zeroed here.
pub fn panel_baseFUNC_TexFaderControl(e: &mut Engine) -> LResult<()> {
    const N: &str = "Scooby_Shaggy_Normal";
    const V: &str = "Scooby_Shaggy_Nervous";
    const S: &str = "Scooby_Shaggy_Scared";
    let lb = LB(e);
    let c = e.rf(lb + 0x74);
    // (band face id, band texture, [slot of "!= id" test, "== a" test, "== b" test, +0x460 store, window],
    //  (a, texture a), (b, texture b))
    let (id, to, ks, (a, ta), (b, tb)) = if c < 0.33 {
        (3, S, [panel_baseFUNC_TexFaderControl_0_1, panel_baseFUNC_TexFaderControl_0_2, panel_baseFUNC_TexFaderControl_0_3, panel_baseFUNC_TexFaderControl_0_4, panel_baseFUNC_TexFaderControl_0_5], (1, N), (2, V))
    } else if c < 0.66 {
        (2, V, [panel_baseFUNC_TexFaderControl_0_6, panel_baseFUNC_TexFaderControl_0_7, panel_baseFUNC_TexFaderControl_0_8, panel_baseFUNC_TexFaderControl_0_9, panel_baseFUNC_TexFaderControl_0_10], (1, N), (3, S))
    } else if 0.66 <= c {
        (1, N, [panel_baseFUNC_TexFaderControl_0_11, panel_baseFUNC_TexFaderControl_0_12, panel_baseFUNC_TexFaderControl_0_13, panel_baseFUNC_TexFaderControl_0_14, panel_baseFUNC_TexFaderControl_0_15], (2, V), (3, S))
    } else {
        return Ok(()); // NaN
    };
    if e.r32(lb + 0x30) != 1 || e.r32(lb + k(e, ks[0])) == id {
        return Ok(());
    }
    let face = e.r32(lb + k(e, ks[1]));
    let from = if face == a {
        ta
    } else if e.r32(lb + k(e, ks[2])) == b {
        tb
    } else {
        to
    };
    e.w32(lb + k(e, ks[3]), id);
    e.w32(lb + 0x30, 0);
    if from != to {
        let (f, t) = (lit(e, from), lit(e, to));
        let p = tex_fader_params(e, 0x3e99_999a, f, t);
        APIgraphics::TexFaderPlay(e, k(e, ks[4]), p);
    }
    Ok(())
}

/// 0x00027778 (412 bytes) `void panel_baseFUNC_ToDFaderControl(uint param_1)`
/// Tome meter face: 3 = "TOD_Normal_Meter", 1 = the level's texture (+0x5c), 4 = "drama_masks"; fades from
/// the current one (+0x54) unless the fader is busy (+0x1c == 1). +0x3c (fader ready) is cleared.
pub fn panel_baseFUNC_ToDFaderControl(e: &mut Engine, param_1: u32) -> LResult<()> {
    let lb = LB(e);
    if e.r32(lb + 0x1c) == 1 {
        return Ok(());
    }
    if param_1 == 3 {
        let s = lit(e, "TOD_Normal_Meter");
        e.w32(lb + 0x58, s);
    } else if param_1 < 4 {
        if param_1 == 1 {
            let v = e.r32(lb + 0x5c);
            e.w32(lb + 0x58, v);
        }
    } else if param_1 == 4 {
        let s = lit(e, "drama_masks");
        e.w32(lb + 0x58, s);
    }
    if e.r32(lb + 0x58) != e.r32(lb + 0x54) {
        let (f, t) = (e.r32(lb + 0x54), e.r32(lb + 0x58));
        let p = tex_fader_params(e, 0x3a83_126f, f, t);
        e.w32(lb + 0x3c, 0);
        APIgraphics::TexFaderPlay(e, k(e, panel_baseFUNC_ToDFaderControl_0_1), p);
    }
    Ok(())
}

/// 0x00027918 (1096 bytes) `void panel_baseFUNC_ToDMeterChangeControl(uint param_1)`
/// Resets the tome needle, then redraws it for the new meter mode: 3 = book angle (GB1+4, degrees),
/// 1 = level gauge (*GB4, 0..1 -> 180 deg), 4 = flipped (pi).
pub fn panel_baseFUNC_ToDMeterChangeControl(e: &mut Engine, param_1: u32) -> LResult<()> {
    let lb = LB(e);
    e.wf(lb + 0x88, 0.0);
    e.wf(lb + 0x70, 0.0);
    APIgameobj::SetRotation(e, k(e, panel_baseFUNC_ToDMeterChangeControl_0_1), lb, 0.0);
    if param_1 == 3 {
        tome_meter_step(e, panel_baseFUNC_ToDMeterChangeControl_0_2, panel_baseFUNC_ToDMeterChangeControl_0_3);
    } else if param_1 < 4 {
        if param_1 == 1 {
            let gb4 = GBP(e, 4);
            let cur = e.rf(gb4);
            let last = e.rf(lb + 0x70);
            if last <= cur || last < 0.0 {
                if last < cur && last <= 1.0 {
                    e.wf(lb + 0x70, cur);
                    APIgameobj::Rotate(e, k(e, panel_baseFUNC_ToDMeterChangeControl_0_4), lb, -((cur - last) * 180.0) * 0.017453292);
                }
            } else {
                e.wf(lb + 0x70, cur);
                APIgameobj::Rotate(e, k(e, panel_baseFUNC_ToDMeterChangeControl_0_5), lb, (last - cur) * 180.0 * 0.017453292);
            }
        }
    } else if param_1 == 4 {
        APIgameobj::Rotate(e, k(e, panel_baseFUNC_ToDMeterChangeControl_0_6), lb, 3.1415927);
    }
    Ok(())
}

/// 0x00026988 (312 bytes) `void panel_baseFUNC_HideInGame(void)`
pub fn panel_baseFUNC_HideInGame(e: &mut Engine) -> LResult<()> {
    e.w32(LB(e) + 0x44, 1);
    for o in [0xd0, 0xd4, 0xd8, 0xb0, 0xe0, 0xe4, 0xe8] {
        APIhud::WindowSetFlags(e, e.r32(LB(e) + o), 0x100);
    }
    Ok(())
}

/// 0x00026ac0 (1440 bytes) `void panel_baseFUNC_ShowInGame(int param_1)`
/// Window pairs: (0xb0, 0xe0) normal meters, (0xe4, 0xe8) alternative. Shape group 1 (6 builds) never
/// shows the normal pair (the two "show 0xb0/0xe0" paths are absent, state +0x478 not set to 2 there).
pub fn panel_baseFUNC_ShowInGame(e: &mut Engine, param_1: i32) -> LResult<()> {
    let grp = g(e, G_panel_baseFUNC_ShowInGame);
    if grp > 1 {
        unported_shape(e, "panel_baseFUNC_ShowInGame", G_panel_baseFUNC_ShowInGame);
    }
    let w = |e: &Engine, o: u32| e.r32(lb(e, o));
    let show = |e: &mut Engine, a: u32, b: u32| {
        let (x, y) = (w(e, a), w(e, b));
        APIhud::WindowResetFlags(e, x, 0x100);
        APIhud::WindowResetFlags(e, y, 0x100);
    };
    let hide = |e: &mut Engine, a: u32, b: u32| {
        let (x, y) = (w(e, a), w(e, b));
        APIhud::WindowSetFlags(e, x, 0x100);
        APIhud::WindowSetFlags(e, y, 0x100);
    };
    e.w32(LB(e) + 0x44, 0);
    e.w32(lb(e, 0x41c), param_1 as u32);
    for o in [0xd0, 0xd4, 0xd8] {
        APIhud::WindowResetFlags(e, w(e, o), 0x100);
    }
    if e.r32(LB(e) + 0x14) != 0 {
        panel_baseCB_TIMER_SLIDEINS_DISPLAY(e, 0)?;
    }
    let gf = e.r32(GBP(e, 0) + 0x14);
    if param_1 == 1 || param_1 == 2 {
        if gf & 2 == 0 {
            if gf & 1 == 0 {
                if grp == 0 {
                    show(e, 0xb0, 0xe0);
                    e.w32(lb(e, 0x478), 2);
                }
            } else {
                e.w32(lb(e, 0x478), 3);
                show(e, 0xe4, 0xe8);
            }
        } else {
            e.w32(lb(e, 0x478), 1);
            show(e, 0xe4, 0xe8);
        }
    } else if param_1 == 3 {
        e.w32(lb(e, 0x478), 4);
        show(e, 0xe4, 0xe8);
    } else if gf & 0x40 == 0 {
        if gf & 1 == 0 {
            if gf & 2 == 0 {
                if e.r32(lb(e, 0x478)) == 3 {
                    show(e, 0xe4, 0xe8);
                } else if grp == 0 {
                    show(e, 0xb0, 0xe0);
                }
            } else {
                show(e, 0xe4, 0xe8);
                hide(e, 0xb0, 0xe0);
            }
        } else {
            hide(e, 0xb0, 0xe0);
            show(e, 0xe4, 0xe8);
        }
    } else {
        show(e, 0xe4, 0xe8);
        hide(e, 0xb0, 0xe0);
    }
    Ok(())
}

/// 0x00027d60 (2956 bytes) `void panel_baseFUNC_MoveMetersOut(uint param_1)`
/// Swaps the meter pairs: slides the visible pair right 0.05/frame (to 1.23) for 10 frames, then the new
/// pair back left (to 0.78). LB+0x4b0 phase (0 out, 1 in), +0x47c frame count, +0x478 target layout
/// (2 = normal meters 0xb0/0xdc/0xe0, else alternative 0xe4/0xe8), +0x474 current, +0x454 last ToD mode.
/// Shape group 1 (6 builds) is group 0 on the shifted local layout (lo()) with its own window literals.
pub fn panel_baseFUNC_MoveMetersOut(e: &mut Engine, param_1: u32) -> LResult<()> {
    if g(e, G_panel_baseFUNC_MoveMetersOut) > 1 {
        unported_shape(e, "panel_baseFUNC_MoveMetersOut", G_panel_baseFUNC_MoveMetersOut);
    }
    let w = |e: &Engine, o: u32| e.r32(lb(e, o));
    let p = e.alloca(8);
    let step = |e: &mut Engine, read: u32, wins: [u32; 2], out: bool| {
        let r = w(e, read);
        APIhud::WindowGetPos(e, r, p, p + 4);
        let x = e.rf(p);
        let x = if out { if 1.23 < x { 1.23 } else { x + 0.05 } } else if x < 0.78 { 0.78 } else { x - 0.05 };
        e.wf(p, x);
        for o in wins {
            let h = w(e, o);
            APIhud::WindowSetPos(e, h, x, e.rf(p + 4));
        }
    };
    let flags = |e: &mut Engine, os: &[u32], set: bool| {
        for &o in os {
            let h = w(e, o);
            if set {
                APIhud::WindowSetFlags(e, h, 0x100);
            } else {
                APIhud::WindowResetFlags(e, h, 0x100);
            }
        }
    };
    let notify = |e: &mut Engine, msg: u32| {
        let r = e.alloca(8);
        e.w32(r, 1);
        e.w32(r + 4, 0x26);
        APIgameobj::SendMessageTo(e, 0xff00_0008, r, msg, 0, 0);
    };
    let cnt = lb(e, 0x47c);
    let add = |e: &mut Engine, d: i32| {
        let v = (e.r32(cnt) as i32 + d) as u32;
        e.w32(cnt, v);
    };
    let phase = w(e, 0x4b0);
    if param_1 == 2 {
        if phase == 0 {
            e.w32(LB(e) + 0x20, 1);
            add(e, 1);
            step(e, 0xb0, [0xb0, 0xe0], true);
            if 9 < e.r32(cnt) {
                e.w32(cnt, 10);
                e.w32(lb(e, 0x4b0), 1);
                let x = e.rf(p);
                for o in [0xe4, 0xe8] {
                    let h = w(e, o);
                    APIhud::WindowSetPos(e, h, x, e.rf(p + 4));
                }
                flags(e, &[0xe4, 0xe8], false);
                notify(e, 0xdf);
                if w(e, 0x478) != w(e, 0x454) {
                    let m = w(e, 0x478);
                    panel_baseFUNC_ToDFaderControl(e, m)?;
                    panel_baseFUNC_ToDMeterChangeControl(e, m)?;
                    e.w32(lb(e, 0x454), m);
                }
            }
        } else if phase == 1 {
            add(e, -1);
            step(e, 0xe4, [0xe4, 0xe8], false);
            if e.r32(cnt) == 0 {
                e.w32(cnt, 0);
                e.w32(lb(e, 0x4b0), 0);
                flags(e, &[0xb0, 0xe0], true);
                let v = w(e, 0x478);
                e.w32(lb(e, 0x474), v);
                e.w32(LB(e) + 0x20, 0);
                e.w32(lb(e, 0x478), 0);
            }
        }
        return Ok(());
    }
    if !(1..=4).contains(&param_1) || param_1 == 2 {
        return Ok(());
    }
    if phase == 0 {
        e.w32(LB(e) + 0x20, 1);
        add(e, 1);
        step(e, 0xe4, [0xe4, 0xe8], true);
        if e.r32(cnt) == 10 {
            if w(e, 0x478) == 2 {
                flags(e, &[0xb0, 0xdc, 0xe0], false);
                let x = e.rf(p);
                for o in [0xb0, 0xe0] {
                    let h = w(e, o);
                    APIhud::WindowSetPos(e, h, x, e.rf(p + 4));
                }
                notify(e, 0xde);
                if param_1 == 4 {
                    let win = k(e, panel_baseFUNC_MoveMetersOut_0_12);
                    APIgameobj::SetRotation(e, win, LB(e), 0.0);
                    let win = k(e, panel_baseFUNC_MoveMetersOut_0_13);
                    APIgameobj::Rotate(e, win, LB(e), 3.1415927);
                }
            } else {
                let m = w(e, 0x478);
                panel_baseFUNC_ToDFaderControl(e, m)?;
                panel_baseFUNC_ToDMeterChangeControl(e, m)?;
            }
            e.w32(lb(e, 0x4b0), 1);
            e.w32(lb(e, 0x454), param_1);
        }
    } else if phase == 1 {
        add(e, -1);
        if w(e, 0x478) == 2 {
            step(e, 0xb0, [0xb0, 0xe0], false);
        } else {
            step(e, 0xe4, [0xe4, 0xe8], false);
        }
        if e.r32(cnt) == 0 {
            e.w32(lb(e, 0x4b0), 0);
            if w(e, 0x478) == 2 {
                flags(e, &[0xb0, 0xe0], false);
                flags(e, &[0xe4, 0xe8], true);
            } else {
                flags(e, &[0xb0, 0xe0], true);
                flags(e, &[0xe4, 0xe8], false);
            }
            let v = w(e, 0x478);
            e.w32(lb(e, 0x474), v);
            e.w32(lb(e, 0x454), 0);
            e.w32(lb(e, 0x478), 0);
            e.w32(LB(e) + 0x20, 0);
        }
    }
    Ok(())
}

/// 0x000288f0 (936 bytes) `int panel_baseFUNC_EpFilesObjUpdate(void)`
/// Quirk: the return value is 5, or (count == 5 and slot 5 empty) the address GB0 + 5*8, or the
/// WindowResetFlags result.
pub fn panel_baseFUNC_EpFilesObjUpdate(e: &mut Engine) -> LResult<i32> {
    let w = |e: &Engine, i: i32| e.r32(lb(e, (0xfc + i * 4) as u32));
    e.w32(lb(e, 0x490), 0);
    APIhud::TextWindowSetText(e, e.r32(lb(e, 0x218)), 0x47, &[6, 6]);
    let mut i: i32 = 4;
    while -1 < i {
        APIhud::WindowResetFlags(e, w(e, i + 0x56), 0x100);
        APIhud::WindowResetFlags(e, w(e, i + 0x4c), 0x100);
        APIhud::WindowResetFlags(e, w(e, i + 0x51), 0x100);
        let b = e.r8(GBP(e, 0).wrapping_add((i * 8) as u32) + 0x744);
        if b == 0 {
            APIhud::WindowSetFlags(e, w(e, i + 0x56), 0x100);
            APIhud::WindowSetFlags(e, w(e, i + 0x4c), 0x100);
            APIhud::WindowSetFlags(e, w(e, i + 0x51), 0x100);
        } else {
            let c = e.r32(lb(e, 0x490)).wrapping_add(1);
            e.w32(lb(e, 0x490), c);
            panel_baseFUNC_EpFilesObjSetText(e, b as i32, i as u32)?;
            if e.r32(GBP(e, 0) + 0x748 + (i * 8) as u32) == 0 {
                APIhud::WindowSetFlags(e, w(e, i + 0x51), 0x100);
            }
        }
        i -= 1;
    }
    let mut r: i32 = 5;
    if e.r32(lb(e, 0x490)) == 5 {
        r = GBP(e, 0).wrapping_add(e.r32(lb(e, 0x490)).wrapping_mul(8)) as i32;
        if e.r8((r as u32) + 0x744) != 0 {
            APIhud::WindowSetFlags(e, e.r32(lb(e, 0x2c8)), 0x100);
            r = APIhud::WindowResetFlags(e, e.r32(lb(e, 0x2cc)), 0x100) as i32;
        }
    }
    Ok(r)
}

/// 0x00028c98 (6592 bytes) `void panel_baseFUNC_EpFilesUPDOWN(int param_1)`
/// Stick Y (param_1+4) navigation in the episode files: objectives list (+0x450 == 1, scroll +0x490) or
/// clues (+0x450 == 2, selector +0xa8 in 0x5d..0x61, skipping locked clues GB3+0xc bits 0x20..0x200).
/// Quirks kept: moving down onto 0x61 and moving up onto 0x60..0x5d don't move the selector window;
/// the wrap-around fallbacks of the moving-up pass don't reset its hidden flag; the objectives "up"
/// check is `< 6` on the count (not 1).
pub fn panel_baseFUNC_EpFilesUPDOWN(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    fn setpos(e: &mut Engine) {
        let lang = e.r32(lb(e, 0x46c)) as i32;
        let sel = e.r32(lb(e, 0xa8)) as i32;
        let p = G(e, S_goPauseSelectorInfo).wrapping_add(lang.wrapping_mul(8).wrapping_add(sel.wrapping_mul(0x18)) as u32);
        let (x, y) = (e.rf(p), e.rf(p.wrapping_add(4)));
        APIhud::WindowSetPos(e, e.r32(lb(e, 0x114)), x, y);
    }
    fn reset_sel(e: &mut Engine) {
        APIhud::WindowResetFlags(e, e.r32(lb(e, 0x114)), 0x100);
    }
    fn snd(e: &mut Engine) {
        APInewSound::Play(e, 0x2a000000, 10, 0);
    }
    fn flags(e: &Engine) -> u32 {
        e.r32(GBP(e, 3) + 0xc)
    }
    fn sel(e: &Engine) -> i32 {
        e.r32(lb(e, 0xa8)) as i32
    }
    fn add_sel(e: &mut Engine, d: i32) {
        let v = sel(e).wrapping_add(d);
        e.w32(lb(e, 0xa8), v as u32);
    }
    fn obj(e: &Engine, i: i32) -> u8 {
        e.r8(GBP(e, 0).wrapping_add(i.wrapping_mul(8) as u32) + 0x744)
    }
    /// cur: wanted +0x494 value; plays the sound only if it changes.
    fn fallback(e: &mut Engine, cur: u32, s: Option<u32>, reset: bool) {
        if e.r32(lb(e, 0x494)) != cur {
            snd(e);
        }
        e.w32(lb(e, 0x494), cur);
        if let Some(s) = s {
            e.w32(lb(e, 0xa8), s);
            if reset {
                reset_sel(e);
            }
            setpos(e);
        }
    }
    if e.r32(lb(e, 0x28)) != 1 {
        let y = e.rf(param_1 + 4) as f64;
        if y < -0.15 {
            add_sel(e, 1);
            let m = e.r32(lb(e, 0x450));
            if m == 1 {
                if e.r32(lb(e, 0x480)) <= e.r32(lb(e, 0x490)) {
                    return Ok(());
                }
                if obj(e, e.r32(lb(e, 0x490)) as i32) == 0 {
                    return Ok(());
                }
                snd(e);
                let c = e.r32(lb(e, 0x490)).wrapping_add(1);
                e.w32(lb(e, 0x490), c);
            } else if m == 2 {
                if 0x61 < sel(e) {
                    e.w32(lb(e, 0xa8), 0x5d);
                }
                for (s, bit, v) in [(0x5d, 0x20, 1), (0x5e, 0x40, 2), (0x5f, 0x80, 3), (0x60, 0x100, 4)] {
                    if sel(e) == s {
                        if flags(e) & bit == 0 {
                            add_sel(e, 1);
                        } else {
                            snd(e);
                            e.w32(lb(e, 0x494), v);
                            reset_sel(e);
                            setpos(e);
                        }
                    }
                }
                if sel(e) == 0x61 {
                    let f = flags(e);
                    if f & 0x200 == 0 {
                        if f & 0x20 == 0 {
                            if f & 0x40 == 0 {
                                if f & 0x80 == 0 {
                                    if f & 0x100 != 0 {
                                        fallback(e, 4, Some(0x60), true);
                                    }
                                } else {
                                    fallback(e, 3, Some(0x5f), true);
                                }
                            } else {
                                fallback(e, 2, Some(0x5e), true);
                            }
                        } else {
                            fallback(e, 1, Some(0x5d), true);
                        }
                    } else {
                        // quirk: no selector move here
                        fallback(e, 5, None, false);
                    }
                }
            }
        } else if 0.15 < y {
            add_sel(e, -1);
            let m = e.r32(lb(e, 0x450));
            if m == 1 {
                if e.r32(lb(e, 0x490)) < 6 {
                    return Ok(());
                }
                if obj(e, (e.r32(lb(e, 0x490)) as i32).wrapping_sub(1)) == 0 {
                    return Ok(());
                }
                snd(e);
                let c = e.r32(lb(e, 0x490)).wrapping_sub(1);
                e.w32(lb(e, 0x490), c);
            } else if m == 2 {
                if sel(e) < 0x5d {
                    e.w32(lb(e, 0xa8), 0x61);
                }
                // quirk: no selector move in these
                for (s, bit, v) in [(0x61, 0x200, 5), (0x60, 0x100, 4), (0x5f, 0x80, 3), (0x5e, 0x40, 2)] {
                    if sel(e) == s {
                        if flags(e) & bit == 0 {
                            add_sel(e, -1);
                        } else {
                            snd(e);
                            e.w32(lb(e, 0x494), v);
                        }
                    }
                }
                if sel(e) == 0x5d {
                    let f = flags(e);
                    if f & 0x20 == 0 {
                        if f & 0x200 == 0 {
                            if f & 0x100 == 0 {
                                if f & 0x80 == 0 {
                                    if f & 0x40 != 0 {
                                        fallback(e, 2, Some(0x5e), false);
                                    }
                                } else {
                                    fallback(e, 3, Some(0x5f), false);
                                }
                            } else {
                                fallback(e, 4, Some(0x60), false);
                            }
                        } else {
                            fallback(e, 5, Some(0x61), false);
                        }
                    } else {
                        fallback(e, 1, None, false);
                    }
                }
            }
        }
        let u = e.r32(lb(e, 0x450));
        if u == 2 {
            panel_baseFUNC_EpFilesClueUpdate(e)?;
            panel_baseFUNC_EpFilesClueTexFade(e)?;
        } else if u < 3 && u == 1 {
            let n = e.r32(lb(e, 0x490));
            if obj(e, n as i32) == 0 {
                APIhud::WindowResetFlags(e, e.r32(lb(e, 0x2c8)), 0x100);
                APIhud::WindowSetFlags(e, e.r32(lb(e, 0x2cc)), 0x100);
            } else if n < 6 || obj(e, n as i32) == 0 {
                if n == 5 && obj(e, n as i32) != 0 {
                    APIhud::WindowSetFlags(e, e.r32(lb(e, 0x2c8)), 0x100);
                    APIhud::WindowResetFlags(e, e.r32(lb(e, 0x2cc)), 0x100);
                } else {
                    APIhud::WindowSetFlags(e, e.r32(lb(e, 0x2c8)), 0x100);
                    APIhud::WindowSetFlags(e, e.r32(lb(e, 0x2cc)), 0x100);
                }
            } else {
                APIhud::WindowResetFlags(e, e.r32(lb(e, 0x2c8)), 0x100);
                APIhud::WindowResetFlags(e, e.r32(lb(e, 0x2cc)), 0x100);
            }
            let w = |e: &Engine, i: i32| e.r32(lb(e, (0xfc + i * 4) as u32));
            let mut i: i32 = 4;
            while -1 < i {
                let n = e.r32(lb(e, 0x490)) as i32;
                if obj(e, n.wrapping_sub(i)) == 0 && (n as u32) < 6 {
                    APIhud::WindowSetFlags(e, w(e, i + 0x56), 0x100);
                    APIhud::WindowSetFlags(e, w(e, i + 0x4c), 0x100);
                    APIhud::WindowSetFlags(e, w(e, i + 0x51), 0x100);
                } else {
                    let d = e.r32(GBP(e, 0).wrapping_add(n.wrapping_sub(1).wrapping_sub(i).wrapping_mul(8) as u32) + 0x748);
                    if d == 0 {
                        APIhud::WindowSetFlags(e, w(e, 0x55 - i), 0x100);
                    } else if d == 1 {
                        APIhud::WindowResetFlags(e, w(e, 0x55 - i), 0x100);
                    }
                }
                let n = e.r32(lb(e, 0x490)) as i32;
                let b = obj(e, n.wrapping_sub(1).wrapping_sub(i));
                panel_baseFUNC_EpFilesObjSetText(e, b as i32, (4 - i) as u32)?;
                i -= 1;
            }
        }
    }
    Ok(())
}

/// 0x0002a658 (1056 bytes) `int panel_baseFUNC_EpFilesObjSetText(int param_1,int param_2)`
/// Text 0x13d + (param_1 - 1) (param_1 1..12) on objective line param_2; returns the call result, or
/// param_1 - 1 when out of range.
pub fn panel_baseFUNC_EpFilesObjSetText(e: &mut Engine, param_1: i32, param_2: u32) -> LResult<i32> {
    let v = param_1.wrapping_sub(1);
    if (0..=0xb).contains(&v) {
        let w = e.r32(lb(e, 0xfc + param_2.wrapping_add(0x56).wrapping_mul(4)));
        return Ok(APIhud::TextWindowSetText(e, w, 0x13d + v as u32, &[7, 7]) as i32);
    }
    Ok(v)
}

/// 0x0002b1c0 (228 bytes) `void panel_baseFUNC_EpFilesShowHideSelect(uint param_1)`
pub fn panel_baseFUNC_EpFilesShowHideSelect(e: &mut Engine, param_1: u32) -> LResult<()> {
    if e.r32(GBP(e, 1) + 0x10) & param_1 == 0 {
        APIhud::WindowSetFlags(e, e.r32(lb(e, 0x104)), 0x100);
        APIhud::WindowSetFlags(e, e.r32(lb(e, 0x10c)), 0x100);
    } else {
        APIhud::WindowResetFlags(e, e.r32(lb(e, 0x104)), 0x100);
        APIhud::WindowResetFlags(e, e.r32(lb(e, 0x10c)), 0x100);
    }
    Ok(())
}

/// 0x0002b2a8 (2380 bytes) `void panel_baseFUNC_EpFilesClueUpdate(void)`
/// Clue list texts (windows +0x270..+0x280, selector values 0x5d..0x61). Each clue's text id is also
/// written to the text-id field of its pause window spec (DAT_0001230c.. = 0x1002c + sel*0x60, like
/// MenuRefresh). Quirk: only clue 1 repositions the selector window (+0x114), whatever is selected.
pub fn panel_baseFUNC_EpFilesClueUpdate(e: &mut Engine) -> LResult<()> {
    for off in [0x26c, 0x270, 0x274, 0x278, 0x27c, 0x280] {
        APIhud::WindowResetFlags(e, e.r32(lb(e, off)), 0x100);
    }
    for off in [0x284, 0x288, 0x28c, 0x290, 0x294] {
        APIhud::WindowSetFlags(e, e.r32(lb(e, off)), 0x100);
    }
    panel_baseFUNC_MenuRefresh(e, 0x5d, 0x61)?;
    for (n, (bit, win, locked, txt)) in [
        (0x20u32, 0x270u32, 0xf6u32, 0x138u32),
        (0x40, 0x274, 0xf7, 0x139),
        (0x80, 0x278, 0xf8, 0x13a),
        (0x100, 0x27c, 0xf9, 0x13b),
        (0x200, 0x280, 0xfa, 0x13c),
    ]
    .into_iter()
    .enumerate()
    {
        let s = 0x5d + n as u32;
        let spec = img(e, 0x1002c + s * 0x60);
        if e.r32(GBP(e, 3) + 0xc) & bit == 0 {
            APIhud::TextWindowSetText(e, e.r32(lb(e, win)), locked, &[8, 8]);
            e.w32(spec, locked);
        } else {
            if e.r32(lb(e, 0xa8)) == s {
                panel_baseFUNC_EpFilesShowHideSelect(e, bit)?;
                APIhud::TextWindowSetText(e, e.r32(lb(e, win)), txt, &[6, 6]);
            } else if e.r32(GBP(e, 1) + 0x10) & bit == 0 {
                APIhud::TextWindowSetText(e, e.r32(lb(e, win)), txt, &[8, 8]);
            } else {
                APIhud::TextWindowSetText(e, e.r32(lb(e, win)), txt, &[7, 7]);
            }
            e.w32(spec, txt);
            if n == 0 {
                let lang = e.r32(lb(e, 0x46c)) as i32;
                let sel = e.r32(lb(e, 0xa8)) as i32;
                let p = G(e, S_goPauseSelectorInfo).wrapping_add(lang.wrapping_mul(8).wrapping_add(sel.wrapping_mul(0x18)) as u32);
                let (x, y) = (e.rf(p), e.rf(p.wrapping_add(4)));
                APIhud::WindowSetPos(e, e.r32(lb(e, 0x114)), x, y);
            }
        }
    }
    Ok(())
}

/// 0x0002bbf8 (200 bytes) `void panel_baseFUNC_GetDestClueTexture(int param_1,uint param_2,undefined4 *param_3)`
pub fn panel_baseFUNC_GetDestClueTexture(e: &mut Engine, param_1: i32, param_2: u32, param_3: Ptr) -> LResult<()> {
    let list = G(e, S_myEpFileTexturesList);
    if e.r32(GBP(e, 3) + 0xc) & param_2 == 0 {
        e.w32(lb(e, 0x494), 99);
        e.w32(lb(e, 0x44c), 99);
        let t = e.r32(list);
        e.w32(param_3, t);
    } else {
        e.w32(lb(e, 0x494), param_1 as u32);
        e.w32(lb(e, 0x44c), param_1 as u32);
        let t = e.r32(list.wrapping_add((param_1 as u32).wrapping_mul(4)));
        e.w32(param_3, t);
    }
    Ok(())
}

/// 0x0002bcc0 (580 bytes) `void panel_baseFUNC_EpFilesClueTexFade(void)`
/// Groups 0 and 1 differ only by the shifted local layout (lb) and the fader window id (k).
/// texClueParams (static _sTexFaderParams): +0 type, +4 speed, +8 from tex, +0x14 to tex, +0x20..+0x2c color.
pub fn panel_baseFUNC_EpFilesClueTexFade(e: &mut Engine) -> LResult<()> {
    match g(e, G_panel_baseFUNC_EpFilesClueTexFade) {
        0 | 1 => {}
        _ => unported_shape(e, "panel_baseFUNC_EpFilesClueTexFade", G_panel_baseFUNC_EpFilesClueTexFade),
    }
    if e.r32(lb(e, 0x38)) != 1 && e.r32(lb(e, 0x44c)) != e.r32(lb(e, 0x494)) {
        let p = G(e, S_texClueParams);
        e.w32(p + 0x2c, 0x3f80_0000);
        e.w32(p + 0x28, 0);
        e.w32(p + 0x24, 0);
        e.w32(p + 0x20, 0x3f80_0000);
        e.w32(p + 4, 0x3a83_126f);
        e.w32(p, 3);
        e.w32(lb(e, 0x38), 1);
        if e.r32(lb(e, 0x44c)) == 99 {
            e.w32(lb(e, 0x44c), 0);
        }
        let list = G(e, S_myEpFileTexturesList);
        let t = e.r32(list.wrapping_add(e.r32(lb(e, 0x44c)).wrapping_mul(4)));
        e.w32(p + 8, t);
        match e.r32(lb(e, 0xa8)) {
            0x5d => panel_baseFUNC_GetDestClueTexture(e, 1, 0x20, p + 0x14)?,
            0x5e => panel_baseFUNC_GetDestClueTexture(e, 2, 0x40, p + 0x14)?,
            0x5f => panel_baseFUNC_GetDestClueTexture(e, 3, 0x80, p + 0x14)?,
            0x60 => panel_baseFUNC_GetDestClueTexture(e, 4, 0x100, p + 0x14)?,
            0x61 => panel_baseFUNC_GetDestClueTexture(e, 5, 0x200, p + 0x14)?,
            _ => {}
        }
        APIgraphics::TexFaderPlay(e, k(e, panel_baseFUNC_EpFilesClueTexFade_0_6), p);
    }
    Ok(())
}

/// 0x0002bf08 (2684 bytes) `void panel_baseFUNC_EpFilesLR(void)`
/// Episode files: L/R switches between the pages (1 objects, 2 clues, 3 monsters).
pub fn panel_baseFUNC_EpFilesLR(e: &mut Engine) -> LResult<()> {
    let l = LB(e);
    let wb = |e: &Engine, s: usize| e.r32(LB(e) + k(e, s));
    let wp = |e: &Engine, s: usize| e.r32(LP(e) + k(e, s));
    let stop_sounds = |e: &mut Engine| {
        for s in [ID_8000003f, ID_80000040, ID_80000041, ID_80000042, ID_80000043] {
            APInewSound::StopSound(e, SELF, k(e, s));
        }
    };
    e.w32(l + 0x28, 1);
    let u = e.r32(l + k(e, panel_baseFUNC_EpFilesLR_0_1));
    if u == 2 {
        for s in [panel_baseFUNC_EpFilesLR_0_2, panel_baseFUNC_EpFilesLR_0_3, panel_baseFUNC_EpFilesLR_0_4] {
            APIanim::Play(e, SELF, k(e, s), 1, 0.0);
        }
        APIhud::WindowSetFlags(e, wb(e, panel_baseFUNC_EpFilesLR_0_5), 0x100);
        APIhud::WindowSetFlags(e, wb(e, panel_baseFUNC_EpFilesLR_0_6), 0x100);
        APIhud::WindowSetFlags(e, wb(e, panel_baseFUNC_EpFilesLR_0_7), 0x100);
        APIhud::WindowResetFlags(e, wb(e, panel_baseFUNC_EpFilesLR_0_8), 0x100);
        APIhud::WindowSetFlags(e, wp(e, panel_baseFUNC_EpFilesLR_0_9), 0x100);
        APIhud::WindowResetFlags(e, wb(e, panel_baseFUNC_EpFilesLR_0_10), 0x100);
        APIhud::WindowResetFlags(e, wb(e, panel_baseFUNC_EpFilesLR_0_11), 0x100);
        APIhud::WindowResetFlags(e, wb(e, panel_baseFUNC_EpFilesLR_0_12), 0x100);
        // first clue owned (flags 0x20..0x200) gets the selector; none: hide the selector windows
        let f = e.r32(GBP(e, 3) + 0xc);
        let (sel, n, s) = if f & 0x20 != 0 {
            (0x5d, 1, panel_baseFUNC_EpFilesLR_0_21)
        } else if f & 0x40 != 0 {
            (0x5e, 2, panel_baseFUNC_EpFilesLR_0_20)
        } else if f & 0x80 != 0 {
            (0x5f, 3, panel_baseFUNC_EpFilesLR_0_19)
        } else if f & 0x100 != 0 {
            (0x60, 4, panel_baseFUNC_EpFilesLR_0_18)
        } else if f & 0x200 != 0 {
            (0x61, 5, panel_baseFUNC_EpFilesLR_0_17)
        } else {
            APIhud::WindowSetFlags(e, wb(e, panel_baseFUNC_EpFilesLR_0_13), 0x100);
            APIhud::WindowSetFlags(e, wb(e, panel_baseFUNC_EpFilesLR_0_14), 0x100);
            APIhud::WindowSetFlags(e, wb(e, panel_baseFUNC_EpFilesLR_0_15), 0x100);
            (0x62, 99, panel_baseFUNC_EpFilesLR_0_16)
        };
        e.w32(l + 0xa8, sel);
        e.w32(l + k(e, s), n);
        let lang = e.r32(l + k(e, panel_baseFUNC_EpFilesLR_0_23)) as i32;
        let sel = e.r32(l + 0xa8) as i32;
        let p = G(e, S_goPauseSelectorInfo).wrapping_add(lang.wrapping_mul(8).wrapping_add(sel.wrapping_mul(0x18)) as u32);
        let (x, y) = (e.rf(p), e.rf(p + 4));
        APIhud::WindowSetPos(e, wb(e, panel_baseFUNC_EpFilesLR_0_22), x, y);
        panel_baseFUNC_EpFilesClueUpdate(e)?;
        panel_baseFUNC_EpFilesClueTexFade(e)?;
    } else if u < 3 {
        if u == 1 {
            for s in [panel_baseFUNC_EpFilesLR_0_25, panel_baseFUNC_EpFilesLR_0_26, panel_baseFUNC_EpFilesLR_0_27] {
                APIanim::Play(e, SELF, k(e, s), 1, 0.0);
            }
            APIhud::WindowSetFlags(e, wb(e, panel_baseFUNC_EpFilesLR_0_28), 0x100);
            APIhud::WindowSetFlags(e, wb(e, panel_baseFUNC_EpFilesLR_0_29), 0x100);
            APIhud::WindowSetFlags(e, wb(e, panel_baseFUNC_EpFilesLR_0_30), 0x100);
            APIhud::WindowResetFlags(e, wb(e, panel_baseFUNC_EpFilesLR_0_31), 0x100);
            APIhud::WindowSetFlags(e, wb(e, panel_baseFUNC_EpFilesLR_0_32), 0x100);
            APIhud::WindowSetFlags(e, wp(e, panel_baseFUNC_EpFilesLR_0_33), 0x100);
            stop_sounds(e);
            panel_baseFUNC_EpFilesObjUpdate(e)?;
        }
    } else if u == 3 {
        for s in [panel_baseFUNC_EpFilesLR_0_34, panel_baseFUNC_EpFilesLR_0_35, panel_baseFUNC_EpFilesLR_0_36] {
            APIanim::Play(e, SELF, k(e, s), 1, 0.0);
        }
        for s in panel_baseFUNC_EpFilesLR_0_37..=panel_baseFUNC_EpFilesLR_0_43 {
            APIhud::WindowSetFlags(e, wb(e, s), 0x100);
        }
        stop_sounds(e);
        for s in panel_baseFUNC_EpFilesLR_0_44..=panel_baseFUNC_EpFilesLR_0_54 {
            APIhud::WindowResetFlags(e, wp(e, s), 0x100);
        }
        panel_baseFUNC_EpFilesMonstersUpdate(e)?;
    }
    e.w32(l + 0x28, 0);
    Ok(())
}

/// 0x0002c988 (2336 bytes) `int panel_baseFUNC_EpFilesConfirm(void)`
/// Clues page: confirm on the selected clue (0x5d..0x61) opens its description if the clue is both
/// found this episode (GBP[3]+0xc) and in the global flags (GBP[1]+0x10). Returns the last
/// InputConfigSetState result, or the page number when not on the clues page.
pub fn panel_baseFUNC_EpFilesConfirm(e: &mut Engine) -> LResult<i32> {
    let l = LB(e);
    let mut r = e.r32(l + k(e, panel_baseFUNC_EpFilesConfirm_0_1)) as i32;
    if r == 2 {
        let mut ok = false;
        for s in [ID_8000003f, ID_80000040, ID_80000041, ID_80000042, ID_80000043] {
            APInewSound::StopSound(e, SELF, k(e, s));
        }
        let sel = e.r32(l + 0xa8);
        if (0x5d..=0x61).contains(&sel) {
            let i = sel - 0x5d;
            let bit = 0x20u32 << i;
            if e.r32(GBP(e, 3) + 0xc) & bit != 0 && e.r32(GBP(e, 1) + 0x10) & bit != 0 {
                let snd = [ID_8000003f, ID_80000040, ID_80000041, ID_80000042, ID_80000043][i as usize];
                APInewSound::Play(e, SELF, k(e, snd), 0);
                let b = panel_baseFUNC_EpFilesConfirm_0_2 + 6 * i as usize;
                e.w32(l + k(e, b), 4);
                let w = |e: &Engine, s: usize| e.r32(LB(e) + k(e, s));
                APIhud::WindowResetFlags(e, w(e, b + 1), 0x100);
                APIhud::WindowResetFlags(e, w(e, b + 2), 0x100);
                APIhud::TextWindowSetText(e, w(e, b + 3), 0x149 + i, &[7]);
                APIhud::TextWindowSetText(e, w(e, b + 4), 0x138 + i, &[7]);
                APIhud::WindowSetFlags(e, w(e, b + 5), 0x100);
                ok = true;
            }
        }
        if ok {
            APInewSound::Play(e, SELF, 0xd, 0);
            for s in panel_baseFUNC_EpFilesConfirm_0_32..=panel_baseFUNC_EpFilesConfirm_0_38 {
                APIhud::WindowSetFlags(e, e.r32(l + k(e, s)), 0x100);
            }
            r = APIinput::InputConfigSetState(e, 8, 0x90) as i32;
        } else if e.r32(l + k(e, panel_baseFUNC_EpFilesConfirm_0_39)) == 0 {
            r = APIinput::InputConfigSetState(e, 8, 0x196) as i32;
        } else {
            r = APIinput::InputConfigSetState(e, 8, 0x2000196) as i32;
        }
    }
    Ok(r)
}

/// 0x0002d2a8 (1084 bytes) `void panel_baseFUNC_EpFilesCancel(void)`
pub fn panel_baseFUNC_EpFilesCancel(e: &mut Engine) -> LResult<()> {
    for s in [ID_8000003f, ID_80000040, ID_80000041, ID_80000042, ID_80000043] {
        APInewSound::StopSound(e, SELF, k(e, s));
    }
    let l = LB(e);
    if e.r32(l + k(e, panel_baseFUNC_EpFilesCancel_0_1)) == 4 {
        e.w32(l + k(e, panel_baseFUNC_EpFilesCancel_0_2), 2);
        if e.r32(l + k(e, panel_baseFUNC_EpFilesCancel_0_3)) == 0 {
            APIinput::InputConfigSetState(e, 8, 0x196);
        } else {
            APIinput::InputConfigSetState(e, 8, 0x2000196);
        }
        for s in panel_baseFUNC_EpFilesCancel_0_4..=panel_baseFUNC_EpFilesCancel_0_11 {
            APIhud::WindowResetFlags(e, e.r32(l + k(e, s)), 0x100);
        }
        for s in panel_baseFUNC_EpFilesCancel_0_12..=panel_baseFUNC_EpFilesCancel_0_23 {
            APIhud::WindowSetFlags(e, e.r32(l + k(e, s)), 0x100);
        }
    } else {
        panel_baseCB_INGAME_LTRIGGER1(e, 0)?;
    }
    Ok(())
}

/// 0x0002d6e8 (1212 bytes) `undefined4 panel_baseFUNC_EpFilesMonstersUpdate(void)`
/// Monsters page. All 4 shape groups share one structure (only the number of monsters / flag bits /
/// literals differ): if the current episode (GBP[0]+0x18) is this build's episode, set the counters
/// (TextWindowSetText(win, txt, GBP[1][off])) and per monster flag bit (GBP[3]) play the "unknown" anims
/// and hide the windows, or play the "known" anims; then recolor 5 windows. Group 0 uses the k slots;
/// groups 1-3 have no slots, so their literals are a per-build table (extracted from the C of each build).
pub fn panel_baseFUNC_EpFilesMonstersUpdate(e: &mut Engine) -> LResult<u32> {
    type Blk = (u32, u32, &'static [u32], &'static [u32], &'static [u32]);
    type Row = (&'static str, u32, &'static [(u32, u32, u32)], &'static [Blk], &'static [u32]);
    const TAB: &[Row] = &[
        ("056f9194", 3, &[(0x59c, 0x4b, 0x14), (0x5a4, 0x4c, 0x1c), (0x5ac, 0x4d, 0x3c), (0x5b4, 0x4e, 0x34)], &[(0x2, 0x2a100019, &[0x30, 0x31], &[0x59c, 0x5a4], &[0x2f, 0x32]), (0x20, 0x2a100019, &[0x34], &[0x5ac], &[0x33]), (0x8, 0x2a100019, &[0x35], &[0x5b4], &[0x36])], &[0x2b4, 0x2b8, 0x2bc, 0x2c0, 0x2c4]),
        ("05789c5e", 1, &[(0x57c, 0x4b, 0x14), (0x584, 0x4c, 0x1c), (0x58c, 0x4d, 0x24), (0x594, 0x4e, 0x2c)], &[(0x2, 0x2a100014, &[0x2d, 0x2f, 0x31], &[0x57c, 0x584, 0x58c], &[0x2c, 0x2e, 0x30]), (0x4, 0x2a100014, &[0x33], &[0x594], &[0x32])], &[0x2b4, 0x2b8, 0x2bc, 0x2c0, 0x2c4]),
        ("05807ca8", 5, &[(0x57c, 0x4b, 0x1c), (0x584, 0x4c, 0x4c)], &[(0x2, 0x2a100019, &[0x2a], &[0x57c], &[0x29]), (0x40, 0x2a100019, &[0x2c], &[0x584], &[0x2b])], &[0x2b4, 0x2b8, 0x2bc, 0x2c0, 0x2c4]),
        ("0aaa7387", 5, &[(0x57c, 0x4b, 0x1c), (0x584, 0x4c, 0x4c)], &[(0x2, 0x2a0000c5, &[0x2a], &[0x57c], &[0x29]), (0x40, 0x2a0000c5, &[0x2c], &[0x584], &[0x2b])], &[0x2b4, 0x2b8, 0x2bc, 0x2c0, 0x2c4]),
        ("1a5c1041", 3, &[(0x59c, 0x4b, 0x14), (0x5a4, 0x4c, 0x1c), (0x5ac, 0x4d, 0x3c), (0x5b4, 0x4e, 0x34)], &[(0x2, 0x2a100019, &[0x30, 0x32], &[0x59c, 0x5a4], &[0x2f, 0x31]), (0x20, 0x2a100019, &[0x34], &[0x5ac], &[0x33]), (0x8, 0x2a100019, &[0x36], &[0x5b4], &[0x35])], &[0x2b4, 0x2b8, 0x2bc, 0x2c0, 0x2c4]),
        ("2f3cf860", 5, &[(0x5c4, 0x4b, 0x1c), (0x5cc, 0x4c, 0x4c)], &[(0x2, 0x2a100019, &[0x33], &[0x5c4], &[0x32]), (0x40, 0x2a100019, &[0x35], &[0x5cc], &[0x34])], &[0x2b4, 0x2b8, 0x2bc, 0x2c0, 0x2c4]),
        ("3c6f977c", 4, &[(0x584, 0x4b, 0x14), (0x58c, 0x4c, 0x1c), (0x594, 0x4d, 0x24), (0x59c, 0x4e, 0x4c)], &[(0x2, 0x2a100019, &[0x30, 0x32, 0x34], &[0x584, 0x58c, 0x594], &[0x2f, 0x31, 0x33]), (0x40, 0x2a100019, &[0x36], &[0x59c], &[0x35])], &[0x2b4, 0x2b8, 0x2bc, 0x2c0, 0x2c4]),
        ("3ce3c13c", 3, &[(0x5d4, 0x4b, 0x14), (0x5dc, 0x4c, 0x1c), (0x5e4, 0x4d, 0x3c), (0x5ec, 0x4e, 0x34)], &[(0x2, 0x2a000026, &[0x38, 0x3a], &[0x5d4, 0x5dc], &[0x37, 0x39]), (0x20, 0x2a000026, &[0x3c], &[0x5e4], &[0x3b]), (0x8, 0x2a000026, &[0x3e], &[0x5ec], &[0x3d])], &[0x2b4, 0x2b8, 0x2bc, 0x2c0, 0x2c4]),
        ("4ec0e7c2", 3, &[(0x5a0, 0x4b, 0x14), (0x5a8, 0x4c, 0x1c), (0x5b0, 0x4d, 0x3c), (0x5b8, 0x4e, 0x34)], &[(0x2, 0x2a100019, &[0x30, 0x32], &[0x5a0, 0x5a8], &[0x2f, 0x31]), (0x20, 0x2a100019, &[0x34], &[0x5b0], &[0x33]), (0x8, 0x2a100019, &[0x36], &[0x5b8], &[0x35])], &[0x2b8, 0x2bc, 0x2c0, 0x2c4, 0x2c8]),
        ("695761b7", 1, &[(0x57c, 0x4b, 0x14), (0x584, 0x4c, 0x1c), (0x58c, 0x4d, 0x24), (0x594, 0x4e, 0x2c)], &[(0x2, 0x2a100018, &[0x2d, 0x2f, 0x31], &[0x57c, 0x584, 0x58c], &[0x2c, 0x2e, 0x30]), (0x4, 0x2a100018, &[0x33], &[0x594], &[0x32])], &[0x2b4, 0x2b8, 0x2bc, 0x2c0, 0x2c4]),
        ("8863bd88", 5, &[(0x580, 0x4b, 0x1c), (0x588, 0x4c, 0x4c)], &[(0x2, 0x2a100018, &[0x2a], &[0x580], &[0x29]), (0x40, 0x2a100018, &[0x2c], &[0x588], &[0x2b])], &[0x2b8, 0x2bc, 0x2c0, 0x2c4, 0x2c8]),
        ("8bf6b396", 5, &[(0x57c, 0x4b, 0x1c), (0x584, 0x4c, 0x4c)], &[(0x2, 0x2a100019, &[0x2a], &[0x57c], &[0x29]), (0x40, 0x2a100019, &[0x2c], &[0x584], &[0x2b])], &[0x2b4, 0x2b8, 0x2bc, 0x2c0, 0x2c4]),
        ("977db83d", 4, &[(0x588, 0x4b, 0x14), (0x590, 0x4c, 0x1c), (0x598, 0x4d, 0x24), (0x5a0, 0x4e, 0x4c)], &[(0x2, 0x2a000094, &[0x30, 0x32, 0x34], &[0x588, 0x590, 0x598], &[0x2f, 0x31, 0x33]), (0x40, 0x2a000094, &[0x36], &[0x5a0], &[0x35])], &[0x2b8, 0x2bc, 0x2c0, 0x2c4, 0x2c8]),
        ("99c250a4", 4, &[(0x5bc, 0x4b, 0x14), (0x5c4, 0x4c, 0x1c), (0x5cc, 0x4d, 0x24), (0x5d4, 0x4e, 0x4c)], &[(0x2, 0x2a0000ce, &[0x30, 0x32, 0x34], &[0x5bc, 0x5c4, 0x5cc], &[0x2f, 0x31, 0x33]), (0x40, 0x2a0000ce, &[0x36], &[0x5d4], &[0x35])], &[0x2b4, 0x2b8, 0x2bc, 0x2c0, 0x2c4]),
        ("a8417f38", 3, &[(0x5d4, 0x4b, 0x14), (0x5dc, 0x4c, 0x1c), (0x5e4, 0x4d, 0x3c), (0x5ec, 0x4e, 0x34)], &[(0x2, 0x2a000028, &[0x38, 0x3a], &[0x5d4, 0x5dc], &[0x37, 0x39]), (0x20, 0x2a000028, &[0x3c], &[0x5e4], &[0x3b]), (0x8, 0x2a000028, &[0x3e], &[0x5ec], &[0x3d])], &[0x2b4, 0x2b8, 0x2bc, 0x2c0, 0x2c4]),
        ("af7a3894", 4, &[(0x5bc, 0x4b, 0x14), (0x5c4, 0x4c, 0x1c), (0x5cc, 0x4d, 0x24), (0x5d4, 0x4e, 0x4c)], &[(0x2, 0x2a0000c6, &[0x30, 0x32, 0x34], &[0x5bc, 0x5c4, 0x5cc], &[0x2f, 0x31, 0x33]), (0x40, 0x2a0000c6, &[0x36], &[0x5d4], &[0x35])], &[0x2b4, 0x2b8, 0x2bc, 0x2c0, 0x2c4]),
        ("bcb77619", 4, &[(0x5bc, 0x4b, 0x14), (0x5c4, 0x4c, 0x1c), (0x5cc, 0x4d, 0x24), (0x5d4, 0x4e, 0x4c)], &[(0x2, 0x2a0000ce, &[0x33, 0x35, 0x37], &[0x5bc, 0x5c4, 0x5cc], &[0x32, 0x34, 0x36]), (0x40, 0x2a0000ce, &[0x39], &[0x5d4], &[0x38])], &[0x2b4, 0x2b8, 0x2bc, 0x2c0, 0x2c4]),
        ("cb9dbf66", 1, &[(0x57c, 0x4b, 0x14), (0x584, 0x4c, 0x1c), (0x58c, 0x4d, 0x24), (0x594, 0x4e, 0x2c)], &[(0x2, 0x2a10000a, &[0x2d, 0x2f, 0x30], &[0x57c, 0x584, 0x58c], &[0x2c, 0x2e, 0x31]), (0x4, 0x2a10000a, &[0x33], &[0x594], &[0x32])], &[0x2b4, 0x2b8, 0x2bc, 0x2c0, 0x2c4]),
        ("cd8e4e92", 1, &[(0x580, 0x4b, 0x14), (0x588, 0x4c, 0x1c), (0x590, 0x4d, 0x24), (0x598, 0x4e, 0x2c)], &[(0x2, 0x2a10000b, &[0x2d, 0x2f, 0x30], &[0x580, 0x588, 0x590], &[0x2c, 0x2e, 0x31]), (0x4, 0x2a10000b, &[0x33], &[0x598], &[0x32])], &[0x2b8, 0x2bc, 0x2c0, 0x2c4, 0x2c8]),
        ("ceb7d472", 4, &[(0x5bc, 0x4b, 0x14), (0x5c4, 0x4c, 0x1c), (0x5cc, 0x4d, 0x24), (0x5d4, 0x4e, 0x4c)], &[(0x2, 0x2a0000c6, &[0x30, 0x32, 0x34], &[0x5bc, 0x5c4, 0x5cc], &[0x2f, 0x31, 0x33]), (0x40, 0x2a0000c6, &[0x36], &[0x5d4], &[0x35])], &[0x2b4, 0x2b8, 0x2bc, 0x2c0, 0x2c4]),
        ("cecbddfd", 4, &[(0x584, 0x4b, 0x14), (0x58c, 0x4c, 0x1c), (0x594, 0x4d, 0x24), (0x59c, 0x4e, 0x4c)], &[(0x2, 0x2a100019, &[0x2d, 0x2f, 0x31], &[0x584, 0x58c, 0x594], &[0x2c, 0x2e, 0x30]), (0x40, 0x2a100019, &[0x32], &[0x59c], &[0x33])], &[0x2b4, 0x2b8, 0x2bc, 0x2c0, 0x2c4]),
        ("dbdc8239", 1, &[(0x57c, 0x4b, 0x14), (0x584, 0x4c, 0x1c), (0x58c, 0x4d, 0x24), (0x594, 0x4e, 0x2c)], &[(0x2, 0x2a0000f0, &[0x2d, 0x2f, 0x30], &[0x57c, 0x584, 0x58c], &[0x2c, 0x2e, 0x31]), (0x4, 0x2a0000f0, &[0x33], &[0x594], &[0x32])], &[0x2b4, 0x2b8, 0x2bc, 0x2c0, 0x2c4]),
        ("e9a8aceb", 5, &[(0x57c, 0x4b, 0x1c), (0x584, 0x4c, 0x4c)], &[(0x2, 0x2a100019, &[0x2a], &[0x57c], &[0x29]), (0x40, 0x2a100019, &[0x2c], &[0x584], &[0x2b])], &[0x2b4, 0x2b8, 0x2bc, 0x2c0, 0x2c4]),
        ("edce70f5", 3, &[(0x59c, 0x4b, 0x14), (0x5a4, 0x4c, 0x1c), (0x5ac, 0x4d, 0x3c), (0x5b4, 0x4e, 0x34)], &[(0x2, 0x2a100018, &[0x30, 0x32], &[0x59c, 0x5a4], &[0x2f, 0x31]), (0x20, 0x2a100018, &[0x34], &[0x5ac], &[0x33]), (0x8, 0x2a100018, &[0x36], &[0x5b4], &[0x35])], &[0x2b4, 0x2b8, 0x2bc, 0x2c0, 0x2c4]),
        ("f56b7427", 4, &[(0x584, 0x4b, 0x14), (0x58c, 0x4c, 0x1c), (0x594, 0x4d, 0x24), (0x59c, 0x4e, 0x4c)], &[(0x2, 0x2a100019, &[0x30, 0x32, 0x34], &[0x584, 0x58c, 0x594], &[0x2f, 0x31, 0x33]), (0x40, 0x2a100019, &[0x35], &[0x59c], &[0x36])], &[0x2b4, 0x2b8, 0x2bc, 0x2c0, 0x2c4]),
        ("fc726958", 5, &[(0x57c, 0x4b, 0x1c), (0x584, 0x4c, 0x4c)], &[(0x2, 0x2a10001a, &[0x2a], &[0x57c], &[0x29]), (0x40, 0x2a10001a, &[0x2c], &[0x584], &[0x2b])], &[0x2b4, 0x2b8, 0x2bc, 0x2c0, 0x2c4]),
        ("fe754b69", 1, &[(0x5a4, 0x4b, 0x14), (0x5ac, 0x4c, 0x1c), (0x5b4, 0x4d, 0x24), (0x5bc, 0x4e, 0x2c)], &[(0x2, 0x2a0000be, &[0x31, 0x33, 0x35], &[0x5a4, 0x5ac, 0x5b4], &[0x32, 0x34, 0x36]), (0x4, 0x2a0000be, &[0x37], &[0x5bc], &[0x38])], &[0x2b4, 0x2b8, 0x2bc, 0x2c0, 0x2c4]),
        ("fe95add8", 5, &[(0x580, 0x4b, 0x1c), (0x588, 0x4c, 0x4c)], &[(0x2, 0x2a100018, &[0x2a], &[0x580], &[0x29]), (0x40, 0x2a100018, &[0x2c], &[0x588], &[0x2b])], &[0x2b8, 0x2bc, 0x2c0, 0x2c4, 0x2c8]),
    ];
    fn run(e: &mut Engine, ep: u32, texts: &[(u32, u32, u32)], blocks: &[(u32, u32, Vec<u32>, Vec<u32>, Vec<u32>)], cols: &[u32]) -> u32 {
        if e.r32(GBP(e, 0) + 0x18) == ep {
            for &(w, t, o) in texts {
                let v = e.r32(GBP(e, 1) + o);
                APIhud::TextWindowSetText(e, e.r32(LP(e) + w), t, &[v]);
            }
            for (bit, obj, a0, ws, a1) in blocks {
                if e.r32(GBP(e, 3)) & bit == 0 {
                    for &a in a0 {
                        APIanim::Play(e, *obj, a, 1, 0.0);
                    }
                    for &w in ws {
                        APIhud::WindowSetFlags(e, e.r32(LP(e) + w), 0x100);
                    }
                } else {
                    for &a in a1 {
                        APIanim::Play(e, *obj, a, 1, 0.0);
                    }
                }
            }
        }
        let mut r = 0;
        for &c in cols {
            r = APIhud::TextWindowSetColor(e, e.r32(LB(e) + c), 5);
        }
        r
    }
    if g(e, G_panel_baseFUNC_EpFilesMonstersUpdate) == 0 {
        let v = |e: &Engine, r: std::ops::RangeInclusive<usize>| r.map(|i| k(e, i)).collect::<Vec<u32>>();
        let q = |e: &Engine, i: usize| k(e, i);
        let obj = k(e, ID_2a100014);
        let texts = [
            (q(e, panel_baseFUNC_EpFilesMonstersUpdate_0_2), 0x4b, q(e, panel_baseFUNC_EpFilesMonstersUpdate_0_3)),
            (q(e, panel_baseFUNC_EpFilesMonstersUpdate_0_4), q(e, panel_baseFUNC_EpFilesMonstersUpdate_0_5), q(e, panel_baseFUNC_EpFilesMonstersUpdate_0_6)),
            (q(e, panel_baseFUNC_EpFilesMonstersUpdate_0_7), q(e, panel_baseFUNC_EpFilesMonstersUpdate_0_8), q(e, panel_baseFUNC_EpFilesMonstersUpdate_0_9)),
            (q(e, panel_baseFUNC_EpFilesMonstersUpdate_0_10), q(e, panel_baseFUNC_EpFilesMonstersUpdate_0_11), q(e, panel_baseFUNC_EpFilesMonstersUpdate_0_12)),
            (q(e, panel_baseFUNC_EpFilesMonstersUpdate_0_13), q(e, panel_baseFUNC_EpFilesMonstersUpdate_0_14), q(e, panel_baseFUNC_EpFilesMonstersUpdate_0_15)),
        ];
        let blocks = vec![
            (2, obj, v(e, panel_baseFUNC_EpFilesMonstersUpdate_0_16..=panel_baseFUNC_EpFilesMonstersUpdate_0_18), v(e, panel_baseFUNC_EpFilesMonstersUpdate_0_19..=panel_baseFUNC_EpFilesMonstersUpdate_0_21), v(e, panel_baseFUNC_EpFilesMonstersUpdate_0_22..=panel_baseFUNC_EpFilesMonstersUpdate_0_24)),
            (q(e, panel_baseFUNC_EpFilesMonstersUpdate_0_25), obj, v(e, panel_baseFUNC_EpFilesMonstersUpdate_0_26..=panel_baseFUNC_EpFilesMonstersUpdate_0_26), v(e, panel_baseFUNC_EpFilesMonstersUpdate_0_27..=panel_baseFUNC_EpFilesMonstersUpdate_0_27), v(e, panel_baseFUNC_EpFilesMonstersUpdate_0_28..=panel_baseFUNC_EpFilesMonstersUpdate_0_28)),
            (q(e, panel_baseFUNC_EpFilesMonstersUpdate_0_29), obj, v(e, panel_baseFUNC_EpFilesMonstersUpdate_0_30..=panel_baseFUNC_EpFilesMonstersUpdate_0_30), v(e, panel_baseFUNC_EpFilesMonstersUpdate_0_31..=panel_baseFUNC_EpFilesMonstersUpdate_0_31), v(e, panel_baseFUNC_EpFilesMonstersUpdate_0_32..=panel_baseFUNC_EpFilesMonstersUpdate_0_32)),
        ];
        let cols = v(e, panel_baseFUNC_EpFilesMonstersUpdate_0_33..=panel_baseFUNC_EpFilesMonstersUpdate_0_37);
        let ep = q(e, panel_baseFUNC_EpFilesMonstersUpdate_0_1);
        return Ok(run(e, ep, &texts, &blocks, &cols));
    }
    let sha = builds::BUILDS[e.rt.cur_build()].sha8;
    let Some(&(_, ep, texts, bl, cols)) = TAB.iter().find(|r| r.0 == sha) else {
        unported_shape(e, "panel_baseFUNC_EpFilesMonstersUpdate", G_panel_baseFUNC_EpFilesMonstersUpdate)
    };
    let blocks: Vec<_> = bl.iter().map(|&(b, o, a0, w, a1)| (b, o, a0.to_vec(), w.to_vec(), a1.to_vec())).collect();
    Ok(run(e, ep, texts, &blocks, cols))
}

/// 0x0002dba8 (1320 bytes) `uint panel_baseFUNC_InOut_ObjNotification(uint param_1,int param_2)`
/// Slides object-notification window 0x10 (param 1) or 0x11 (param 2) in from the right, holds it until
/// 3.5 s, then slides it out (bits of LB+0x468: 1 sliding in, 2 sliding out, 4 done).
pub fn panel_baseFUNC_InOut_ObjNotification(e: &mut Engine, param_1: u32, param_2: i32) -> LResult<u32> {
    if !(param_1 < 3 && param_1 != 0) {
        return Ok(if param_1 < 3 { (param_1 == 0) as u32 } else { param_1 });
    }
    let w = e.r32(LB(e) + 0xac + if param_1 == 1 { 0x10 } else { 0x11 } * 4);
    let p = e.alloca(8);
    let t = LB(e) + 0x90;
    let fl = lb(e, 0x468);
    let dt = APIgame::GetFrameDeltaTime(e);
    if param_2 == 0 {
        e.wf(t, e.rf(t) + dt);
        if e.rf(t) < 0.25 {
            APIhud::WindowResetFlags(e, w, 0x100);
            APIhud::WindowGetPos(e, w, p, p + 4);
            Ok(APIhud::WindowSetPos(e, w, e.rf(p) - 0.02, e.rf(p + 4)))
        } else if 3.5 < e.rf(t) {
            let v = (e.r32(fl) | 2) & 0xffff_fffe;
            e.w32(fl, v);
            Ok(LB(e))
        } else {
            APIhud::WindowGetPos(e, w, p, p + 4);
            Ok(APIhud::WindowSetPos(e, w, 0.77, e.rf(p + 4)))
        }
    } else {
        e.wf(t, e.rf(t) - dt);
        let v = e.r32(fl) & 0xffff_fdff;
        e.w32(fl, v);
        if 3.0 < e.rf(t) {
            APIhud::WindowGetPos(e, w, p, p + 4);
            Ok(APIhud::WindowSetPos(e, w, e.rf(p) + 0.02, e.rf(p + 4)))
        } else {
            e.wf(t, 0.0);
            APIhud::WindowGetPos(e, w, p, p + 4);
            APIhud::WindowSetPos(e, w, 1.0, e.rf(p + 4));
            APIhud::WindowSetFlags(e, w, 0x100);
            let v = (e.r32(fl) & 0xffff_fffd) | 4;
            e.w32(fl, v);
            Ok(LB(e))
        }
    }
}

/// 0x0002e0d0 (3608 bytes) `uint panel_baseFUNC_InOut_ActNotification(uint param_1,int param_2)`
/// Action prompt (button icon window LB+0xf4, text LB+0xf8). Notification -> (button anim, text id,
/// flash window k slot). Ghidra shows the frame delta as the stale f-register; the original adds/subtracts
/// GetFrameDeltaTime's result. Group 1 (05807ca8): see the 0x10 guard below.
pub fn panel_baseFUNC_InOut_ActNotification(e: &mut Engine, param_1: u32, param_2: i32) -> LResult<u32> {
    let grp = g(e, G_panel_baseFUNC_InOut_ActNotification);
    if grp > 1 {
        unported_shape(e, "panel_baseFUNC_InOut_ActNotification", G_panel_baseFUNC_InOut_ActNotification);
    }
    let (bw, tw) = (e.r32(LB(e) + 0xf4), e.r32(LB(e) + 0xf8));
    let p = e.alloca(8);
    let t = LB(e) + 0x94;
    if param_2 == 0 {
        if !(param_1 < 0x19 && param_1 >= 3) {
            return Ok(if param_1 < 0x19 { (param_1 < 3) as u32 } else { param_1 });
        }
        use super::builds::*;
        let ks = |e: &Engine, s: usize| k(e, s);
        let (anim, txt, flash) = match param_1 {
            3 | 0x12 => (ks(e, panel_baseFUNC_InOut_ActNotification_0_1), 0xdf, None),
            4 => (ks(e, panel_baseFUNC_InOut_ActNotification_0_2), 0xe1, None),
            5 => (ks(e, panel_baseFUNC_InOut_ActNotification_0_3), 0xe0, None),
            6 => (ks(e, panel_baseFUNC_InOut_ActNotification_0_4), 0xe2, None),
            7 => (ks(e, panel_baseFUNC_InOut_ActNotification_0_5), 0xe3, None),
            8 => (ks(e, panel_baseFUNC_InOut_ActNotification_0_6), 0xe4, None),
            9 => (ks(e, panel_baseFUNC_InOut_ActNotification_0_7), 0xe5, None),
            10 => (ks(e, panel_baseFUNC_InOut_ActNotification_0_8), 0xe6, None),
            0xb => (ks(e, panel_baseFUNC_InOut_ActNotification_0_9), 0xe7, None),
            0xc => (ks(e, panel_baseFUNC_InOut_ActNotification_0_10), 0xe8, None),
            0xd => (ks(e, panel_baseFUNC_InOut_ActNotification_0_11), 0xea, None),
            0xe => (ks(e, panel_baseFUNC_InOut_ActNotification_0_12), 0xe9, Some(panel_baseFUNC_InOut_ActNotification_0_13)),
            0xf => (ks(e, panel_baseFUNC_InOut_ActNotification_0_14), 0xe9, Some(panel_baseFUNC_InOut_ActNotification_0_15)),
            0x10 => (ks(e, panel_baseFUNC_InOut_ActNotification_0_16), 0xe9, Some(panel_baseFUNC_InOut_ActNotification_0_17)),
            0x11 => (ks(e, panel_baseFUNC_InOut_ActNotification_0_18), 0xeb, None),
            0x13 => (ks(e, panel_baseFUNC_InOut_ActNotification_0_19), 0xec, None),
            0x14 => (ks(e, panel_baseFUNC_InOut_ActNotification_0_20), 0xed, None),
            0x15 => (ks(e, panel_baseFUNC_InOut_ActNotification_0_21), 0xee, None),
            0x16 => (ks(e, panel_baseFUNC_InOut_ActNotification_0_22), 0xef, None),
            0x17 => (ks(e, panel_baseFUNC_InOut_ActNotification_0_23), 0xf0, None),
            _ => (ks(e, panel_baseFUNC_InOut_ActNotification_0_24), 0xf1, None), // 0x18
        };
        APIhud::Window3DSetObj(e, bw, SELF, anim);
        APIhud::TextWindowSetText(e, tw, txt, &[7, 7]);
        if let Some(fk) = flash {
            panel_baseFUNC_FlashButton_Start(e, bw, k(e, fk) as i32)?;
            panel_baseFUNC_FlashButton_Update(e)?;
        }
        if e.rf(t) < 0.1 {
            let dt = APIgame::GetFrameDeltaTime(e);
            e.wf(t, e.rf(t) + dt);
            let mut r = 0;
            for w in [bw, tw] {
                APIhud::WindowResetFlags(e, w, 0x100);
                r = APIhud::WindowGetPos(e, w, p, p + 4);
                if e.rf(p) < 0.05 {
                    r = APIhud::WindowSetPos(e, w, e.rf(p) + 0.03, e.rf(p + 4));
                }
            }
            Ok(r)
        } else {
            APIhud::WindowGetPos(e, tw, p, p + 4);
            APIhud::WindowSetPos(e, tw, 0.05, e.rf(p + 4));
            let l = LB(e);
            Ok(APIhud::WindowSetPos(e, bw, e.rf(l + 0xa0) + 0.05, e.rf(l + 0x9c) + e.rf(l + 0xa4)))
        }
    } else {
        panel_baseFUNC_FlashButton_Stop(e)?;
        if 0.01 <= e.rf(t) {
            let dt = APIgame::GetFrameDeltaTime(e);
            e.wf(t, e.rf(t) - dt);
            let mut r = 0;
            for w in [bw, tw] {
                APIhud::WindowResetFlags(e, w, 0x100);
                APIhud::WindowGetPos(e, w, p, p + 4);
                r = APIhud::WindowSetPos(e, w, e.rf(p) - 0.03, e.rf(p + 4));
            }
            Ok(r)
        } else {
            for w in [bw, tw] {
                APIhud::WindowGetPos(e, w, p, p + 4);
                APIhud::WindowSetPos(e, w, -0.25, e.rf(p + 4));
                APIhud::WindowSetFlags(e, w, 0x100);
            }
            e.wf(t, -0.25);
            // group 1 (05807ca8) keeps the prompt flag while an object notification is pending (bit 0x10)
            if grp == 0 || e.r32(lb(e, 0x468)) & 0x10 == 0 {
                let f = GBP(e, 3);
                let v = e.r32(f) & 0xfffd_ffff;
                e.w32(f, v);
            }
            let fl = lb(e, 0x468);
            let v = (e.r32(fl) | 0x40) & 0xffff_ffdf;
            e.w32(fl, v);
            Ok(LB(e))
        }
    }
}

/// 0x0002eee8 (344 bytes) `void panel_baseFUNC_FlashButton_Start(undefined4 param_1,int param_2)`
pub fn panel_baseFUNC_FlashButton_Start(e: &mut Engine, param_1: u32, param_2: i32) -> LResult<()> {
    if e.r32(lb(e, 0x42c)) != 0 {
        if e.r32(lb(e, 0x430)) == param_2 as u32 {
            return Ok(());
        }
        panel_baseFUNC_FlashButton_Stop(e)?;
    }
    e.w32(lb(e, 0x42c), param_1);
    e.w32(lb(e, 0x430), param_2 as u32);
    let m = APIgameobj::GetLMat(e, param_2 as u32) as u32;
    APImath::CMatCopy(e, m, e.r32(lb(e, 0x4c8)));
    let t = e.r32(lb(e, 0x4a4));
    APIextra::TimerSetFrequency(e, t, 0.35);
    APIextra::TimerReset(e, t);
    APIextra::TimerRegisterNotify(e, SELF, t, 0x17);
    Ok(())
}

/// 0x0002f040 (348 bytes) `void panel_baseFUNC_FlashButton_Stop(void)`
pub fn panel_baseFUNC_FlashButton_Stop(e: &mut Engine) -> LResult<()> {
    let w = e.r32(lb(e, 0x42c));
    if w != 0 {
        let p = e.alloca(8);
        APIhud::WindowGetPos(e, w, p, p + 4);
        APIhud::WindowSetPos(e, w, e.rf(p), e.rf(LB(e) + 0x9c));
        APIextra::TimerUnregisterNotify(e, SELF, e.r32(lb(e, 0x4a4)));
        let m = APImath::CMatGetWorkMatrix(e);
        APImath::CMatCopy(e, e.r32(lb(e, 0x4c8)), m);
        APIgameobj::SetLMat(e, e.r32(lb(e, 0x430)), m);
        e.w32(lb(e, 0x42c), 0);
        e.w32(lb(e, 0x430), 0);
        e.w32(LB(e) + 0xa0, 0);
        e.w32(LB(e) + 0xa4, 0);
    }
    Ok(())
}

/// 0x0002f1a0 (592 bytes) `int panel_baseFUNC_FlashButton_Update(void)`
/// Ghidra lost the TimerGetTime result (f0): the pulse phase is that timer's time, as the 0.35 s
/// frequency set in FlashButton_Start implies; LB+0x94 (the prompt slide timer) only gates the pulse.
pub fn panel_baseFUNC_FlashButton_Update(e: &mut Engine) -> LResult<i32> {
    if e.rf(LB(e) + 0x94) < 0.1 {
        e.w32(LB(e) + 0xa0, 0);
        e.w32(LB(e) + 0xa4, 0);
        return Ok(LB(e) as i32);
    }
    let m = APImath::CMatGetWorkMatrix(e);
    let t = APIextra::TimerGetTime(e, e.r32(lb(e, 0x4a4)));
    let f = t / 0.35 + t / 0.35;
    APImath::CMatCopy(e, e.r32(lb(e, 0x4c8)), m);
    let s = if f < 1.0 { f } else { 2.0 - f };
    e.wf(LB(e) + 0xa0, s * 0.0);
    e.wf(LB(e) + 0xa4, s * 0.01);
    APImath::CMatUniformScale(e, m, 1.0 - s * 0.15);
    Ok(APIgameobj::SetLMat(e, e.r32(lb(e, 0x430)), m) as i32)
}

/// 0x0002f3f0 (356 bytes) `void panel_baseFUNC_SetCurrentEpisode(void)`
pub fn panel_baseFUNC_SetCurrentEpisode(e: &mut Engine) -> LResult<()> {
    let (s, n) = match e.r32(GBP(e, 0) + 0x18) {
        1 => ("flashlight", 0xb),
        2 => ("megaphone", 10),
        3 => ("lantern", 0xc),
        4 => ("snorkel", 9),
        5 => ("fear_gun", 7),
        _ => ("drama_masks", 0x11),
    };
    let p = lit(e, s);
    e.w32(LB(e) + 0x5c, p);
    e.w32(lb(e, 0x480), n);
    Ok(())
}

/// 0x0002f558 (1056 bytes) `void panel_baseFUNC_InitIngredientWin(void)`
/// Greys out (0.2, 0.2, 0.2, 0.75) the 5 sandwich-ingredient 3D windows (records 2..6 of the window
/// table: +0x54 object, +0x58 sub id) whose bit in flags-buffer +0xc is clear; lit ones are white.
pub fn panel_baseFUNC_InitIngredientWin(e: &mut Engine) -> LResult<()> {
    let (a, b) = (k(e, panel_baseFUNC_InitIngredientWin_0_1), k(e, panel_baseFUNC_InitIngredientWin_0_2));
    let mut objs = [(0u32, 0u32); 5];
    for i in 0..5u32 {
        let r = (i + 2) * 0x60;
        objs[i as usize] = (e.r32(img(e, a + r)), e.r32(img(e, b + r)));
    }
    for (i, (o, s)) in objs.into_iter().enumerate() {
        if e.r32(GBP(e, 3) + 0xc) & (1 << i) == 0 {
            APIgraphics::SetGameObjBlendColor(e, o, s, 0.2, 0.2, 0.2, 0.75);
        } else {
            APIgraphics::SetGameObjBlendColor(e, o, s, 1.0, 1.0, 1.0, 1.0);
        }
    }
    Ok(())
}

/// 0x0002f978 (508 bytes) `void panel_baseFUNC_GetDisplayInfo(undefined4 param_1,int param_2)`
/// Per-build switch evaluated into local::DISPLAY_INFO (writes param_2+4 / +8; unknown ids untouched).
pub fn panel_baseFUNC_GetDisplayInfo(e: &mut Engine, param_1: u32, param_2: Ptr) -> LResult<()> {
    if let Some(&(_, a, b)) = super::local::DISPLAY_INFO[e.rt.cur_build()].iter().find(|r| r.0 == param_1) {
        e.w32(param_2 + 4, a);
        e.w32(param_2 + 8, b);
    }
    Ok(())
}

/// 0x0002fb78 (1692 bytes) `void panel_baseConstructor(APIcons *api,int instanceCount)`
pub fn panel_baseConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    if instanceCount == 1 {
        let fns: [(i32, crate::logic::ExtFn); 31] = [
            (0, panel_baseST_FIRSTMOVE),
            (1, panel_baseST_IN_GAME_SET),
            (2, panel_baseST_IN_GAME),
            (3, panel_baseST_PAUSE_SET),
            (4, panel_baseST_PAUSE),
            (5, panel_baseST_OPTIONS_SET),
            (6, panel_baseST_OPTIONS),
            (9, panel_baseST_SLIDE_IN),
            (10, panel_baseST_SLIDE_OUT),
            (0xb, panel_baseCB_PAUSE_MENU_OPEN),
            (0x1a, panel_baseCB_TRIANGLE),
            (0xc, panel_baseCB_PAUSE_MENU_CLOSE),
            (0xd, panel_baseCB_MENU_STICK_UPDOWN),
            (0xe, panel_baseCB_MENU_STICK_LEFTRIGHT),
            (0xf, panel_baseCB_MENU_CONFIRM),
            (0x10, panel_baseCB_MENU_CANCEL),
            (0x20, panel_baseCB_INGAME_LTRIGGER1),
            (0x11, panel_baseCB_OPTIONS_STICK_UPDOWN),
            (0x12, panel_baseCB_OPTIONS_STICK_LEFTRIGHT),
            (0x13, panel_baseCB_OPTIONS_CONFIRM),
            (0x14, panel_baseCB_OPTIONS_CANCEL),
            (0x15, panel_baseCB_TIMER_OPTIONS_VIBRATION),
            (0x16, panel_baseCB_TIMER_SLIDEINS_DISPLAY),
            (0x17, panel_baseCB_TIMER_BUTTON_FLASH),
            (0x18, panel_baseCB_MENU_IN_LF),
            (0x19, panel_baseCB_MENU_OUT_LF),
            (0x1b, panel_baseCB_TIMER_SAFEMODE),
            (0x1c, panel_baseCB_TIMER_CONTROLLER),
            (0x1d, panel_baseCB_NOTIFY_TEXFADER),
            (0x1f, panel_baseCB_NOINPUT),
            (0x1e, panel_baseCB_ANIM_LF),
        ];
        for (i, f) in fns {
            APIcons::SetExternalFunction(e, i, f);
        }
        let tables: [(usize, fn(&mut Engine) -> Ptr); 12] = [
            (S_pAnim, |e| APIcons::GetApiAnim(e)),
            (S_pGameObj, |e| APIcons::GetApiGameObj(e)),
            (S_pExtra, |e| APIcons::GetApiExtra(e)),
            (S_pHud, |e| APIcons::GetApiHUD(e)),
            (S_pInput, |e| APIcons::GetApiInput(e)),
            (S_pGame, |e| APIcons::GetApiGame(e)),
            (S_pSound, |e| APIcons::GetApiNewSound(e)),
            (S_pLight, |e| APIcons::GetApiLight(e)),
            (S_pMath, |e| APIcons::GetApiMath(e)),
            (S_pGraphic, |e| APIcons::GetApiGraphics(e)),
            (S_pStdlib, |e| APIcons::GetApiStdlib(e)),
            (S_pLoadSave, |e| APIcons::GetApiLoadSave(e)),
        ];
        for (slot, get) in tables {
            let v = get(e);
            let a = G(e, slot);
            e.w32(a, v);
        }
        for (slot, name) in [(S_pText, "ApiText"), (S_pMisc, "MiscFunc"), (S_pMiscHUD, "MiscHUD"), (S_pDebug, "debug")] {
            let n = lit(e, name);
            let v = APIcons::GetApiCustom(e, n);
            let a = G(e, slot);
            e.w32(a, v);
        }
        let slot = G(e, S_pGlobalData_BasePanel);
        APIcons::GetGlobalData(e, slot, 0x18);
        for (i, buf, size) in [(0, 0, 0x8f8), (3, 2, 0x158), (4, 4, 8), (2, 3, 0x40), (1, 1, 0x68), (5, 5, 0x4c)] {
            let v = APIgameobj::GetGameBuffer(e, buf, size, 1);
            let gd = e.r32(G(e, S_pGlobalData_BasePanel));
            e.w32(gd + i * 4, v);
        }
    }
    let slot = G(e, S_pLocalData_BasePanel);
    APIcons::GetLocalData(e, slot, k(e, panel_baseConstructor_0_1) as i32);
    panel_baseFUNC_Init(e)?;
    Ok(())
}

/// 0x00030218 (716 bytes) `void panel_baseDestructor(int instanceCount)`
pub fn panel_baseDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    if e.r32(LB(e) + 0x24) == 1 {
        let t = APIextra::TimerGetTime(e, 0x9000_0000);
        let a = GBP(e, 0) + 0x68;
        e.wf(a, e.rf(a) + t);
    }
    for o in [0x3f8, 0x3f4, 0x3fc, 0x49c, 0x4a0, 0x498, 0x4a4] {
        APIextra::TimerDestroy(e, e.r32(lb(e, o)));
    }
    APImath::CMatDestroy(e, e.r32(lb(e, 0x4c8)));
    let (t1, n1) = (lb(e, 0xfc), 0xbe);
    let n2 = if shifted(e) { 0x15 } else { 0x14 };
    for (base, n) in [(t1, n1), (LB(e) + 0xac, n2)] {
        for i in 0..n {
            let w = e.r32(base + i * 4);
            if w != 0x3b9a_c9f7 {
                APIhud::WindowDelete(e, w);
            }
        }
    }
    Ok(())
}

// Boss meter (panel_bossmeter.c, only in the 6 shifted-layout boss-fight builds). Offsets below are RAW
// offsets of those builds: LB+0xfc is the inserted word (the boss meter window), +0x46c the in-game flags
// (0x100 shown, 0x400 fully in, 0x800 filling), +0x98 slide distance, +0x4c0 target hits, +0x4c4 displayed
// hits, +0x4c8 anim time of meter anim 99.

/// Seconds of anim 99 per hit (x 1/30): 12.5 frames (group 0), 10 (group 1), 20 (group 2).
fn boss_frames_per_hit(e: &Engine) -> f32 {
    match g(e, G_panel_baseFUNC_MessageHandlerBossMeter) {
        0 => 12.5,
        1 => 10.0,
        2 => 20.0,
        _ => unported_shape(e, "panel_baseFUNC_MessageHandlerBossMeter", G_panel_baseFUNC_MessageHandlerBossMeter),
    }
}

pub fn panel_baseFUNC_ShowBossMeter(e: &mut Engine, param_1: i32) -> LResult<()> {
    let lb = LB(e);
    if param_1 == 0 {
        let v = e.r32(lb + 0x46c) & 0xffff_feff;
        e.w32(lb + 0x46c, v);
        APIhud::WindowSetFlags(e, e.r32(lb + 0xfc), 0x100);
    } else {
        let v = e.r32(lb + 0x46c) | 0x100;
        e.w32(lb + 0x46c, v);
        e.w32(lb + 0x98, 0);
        APIhud::WindowResetFlags(e, e.r32(lb + 0xfc), 0x100);
        APIanim::Play(e, SELF, 99, 1, 0.0);
        APIanim::SetSpeed(e, SELF, 99, 0.0);
        APIanim::SetTime(e, SELF, 99, e.rf(lb + 0x4c8));
    }
    Ok(())
}

pub fn panel_baseFUNC_MoveBossMeter(e: &mut Engine, param_1: u32) -> LResult<()> {
    let lb = LB(e);
    let w = e.r32(lb + 0xfc);
    let p = e.alloca(8);
    let d = e.rf(lb + 0x98);
    if param_1 == 0 {
        if d < 0.28 {
            e.wf(lb + 0x98, d + 0.02);
            APIhud::WindowGetPos(e, w, p, p + 4);
            APIhud::WindowSetPos(e, w, e.rf(p), e.rf(p + 4) - 0.02);
        } else {
            let v = e.r32(lb + 0x46c) | 0x400;
            e.w32(lb + 0x46c, v);
            APIhud::WindowGetPos(e, w, p, p + 4);
            APIhud::WindowSetPos(e, w, e.rf(p), -0.05);
        }
    } else if 0.0 < d {
        e.wf(lb + 0x98, d - 0.02);
        APIhud::WindowGetPos(e, w, p, p + 4);
        APIhud::WindowSetPos(e, w, e.rf(p), e.rf(p + 4) + 0.02);
    } else {
        let v = e.r32(lb + 0x46c) & 0xffff_fbff;
        e.w32(lb + 0x46c, v);
        e.w32(lb + 0x98, 0);
        APIhud::WindowGetPos(e, w, p, p + 4);
        APIhud::WindowSetPos(e, w, e.rf(p), 0.25);
    }
    Ok(())
}

/// Advances anim 99 by one frame per call until it reaches the target hit count.
pub fn panel_baseFUNC_UpdateBossMeter(e: &mut Engine) -> LResult<()> {
    let lb = LB(e);
    let target = e.r32(lb + 0x4c0);
    if target == e.r32(lb + 0x4c4) {
        let v = e.r32(lb + 0x46c) & 0xffff_f7ff;
        e.w32(lb + 0x46c, v);
    } else {
        let hits = target as f32; // unsigned conversion (Ghidra's >>1 | &1 doubling idiom)
        let t = e.rf(lb + 0x4c8) + 0.033333335;
        e.wf(lb + 0x4c8, t);
        APIanim::SetTime(e, SELF, 99, t);
        if (hits * boss_frames_per_hit(e)) / 30.0 <= t {
            e.w32(lb + 0x4c4, target);
        }
    }
    Ok(())
}

/// 0x39 show (param_2 ignored), 0x3a one more hit, 0x3b reset, 0x3c set to param_2 hits instantly.
pub fn panel_baseFUNC_MessageHandlerBossMeter(e: &mut Engine, param_1: u32, param_2: u32) -> LResult<u64> {
    let lb = LB(e);
    match param_1 {
        0x39 => panel_baseFUNC_ShowBossMeter(e, 1)?,
        0x3a => {
            let v = e.r32(lb + 0x46c) | 0x800;
            e.w32(lb + 0x46c, v);
            let n = e.r32(lb + 0x4c0).wrapping_add(1);
            e.w32(lb + 0x4c0, n);
        }
        0x3b => {
            e.w32(lb + 0x4c0, 0);
            e.w32(lb + 0x4c4, 0);
            e.w32(lb + 0x4c8, 0);
            APIanim::SetTime(e, SELF, 99, e.rf(lb + 0x4c8));
        }
        0x3c => {
            let t = (param_2 as f32 * boss_frames_per_hit(e)) / 30.0;
            e.wf(lb + 0x4c8, t);
            APIanim::SetTime(e, SELF, 99, t);
            e.w32(lb + 0x4c0, param_2);
            e.w32(lb + 0x4c4, param_2);
        }
        _ => {}
    }
    Ok(0)
}

/// 0x0002aa78 (1816 bytes) `undefined4 panel_baseFUNC_EpFilesObjAddSub(int param_1,uint param_2)`
/// Episode-file objective list: (id byte, done word) pairs at game buffer +0x744, count LB+0x480.
/// param_1: 0 add (pushed at the front), 1 mark done, 2 remove, 3 clear done flags. The remove loop reads
/// one slot past the count, as the original does.
pub fn panel_baseFUNC_EpFilesObjAddSub(e: &mut Engine, param_1: i32, param_2: u32) -> LResult<u32> {
    let cnt = |e: &Engine| e.r32(lb(e, 0x480));
    let id = |e: &Engine, i: u32| GBP(e, 0) + i * 8 + 0x744;
    let done = |e: &Engine, i: u32| GBP(e, 0) + 0x748 + i * 8;
    let pr = |e: &mut Engine, s: &str, a: u32| {
        let m = lit(e, s);
        APIextra::null_printf(e, m, a);
    };
    Ok(match param_1 {
        0 => {
            if cnt(e) == 0 {
                pr(e, "*SENDING ADD OBJECTIVE TOO SOON!!!\n", 0);
                0
            } else if cnt(e) < param_2 {
                pr(e, "OBJECTIFS INEXISTANT \n\n\n\n", 0);
                0
            } else {
                let mut i = 0;
                while i <= cnt(e) {
                    let b = e.r8(id(e, i)) as u32;
                    if b == param_2 && b != 0 {
                        pr(e, "*OBJECTIFS DEJA EXISTANT* : %u \n", b);
                        return Ok(0);
                    }
                    i += 1;
                }
                pr(e, "*OBJECTIVE TO ADD* : %u \n", param_2);
                let c = e.r8(id(e, 0)) as u32;
                pr(e, "*CURRENT OBJECTIVE* : %u \n", c);
                pr(e, "*PLACER A  \n", 0);
                let mut i = cnt(e);
                while i != 0 {
                    let b = e.r8(id(e, i - 1));
                    e.w8(id(e, i), b);
                    let d = e.r32(done(e, i - 1));
                    e.w32(done(e, i), d);
                    i -= 1;
                }
                e.w8(id(e, 0), param_2 as u8);
                e.w32(done(e, 0), 0);
                1
            }
        }
        1 => {
            let mut i = 0;
            while i <= cnt(e) {
                if e.r8(id(e, i)) as u32 == param_2 && e.r32(done(e, i)) == 0 {
                    e.w32(done(e, i), 1);
                    return Ok(1);
                }
                i += 1;
            }
            0
        }
        2 => {
            if cnt(e) < param_2 {
                pr(e, "OBJECTIFS INEXISTANT \n\n\n\n", 0);
                0
            } else {
                let mut i = 0;
                while i <= cnt(e) {
                    if e.r8(id(e, i)) as u32 == param_2 {
                        e.w32(done(e, i), 0);
                        e.w8(id(e, i), 0);
                        break;
                    }
                    i += 1;
                }
                while i <= cnt(e) {
                    let b = e.r8(id(e, i + 1));
                    e.w8(id(e, i), b);
                    let d = e.r32(done(e, i + 1));
                    e.w32(done(e, i), d);
                    i += 1;
                }
                1
            }
        }
        3 => {
            let mut i = 0;
            while i < cnt(e) {
                if e.r8(id(e, i)) as u32 == param_2 {
                    e.w32(done(e, i), 0);
                }
                i += 1;
            }
            1
        }
        _ => 0,
    })
}

//! panel_base_options.c (13 functions). Addresses are module-relative (.text at 0x10000) in panel__014a381f.
use super::builds::*;
use super::*;
use crate::engine::*;
use crate::logic::LResult;

/// 0x0001ac08 (568 bytes) `undefined8 panel_baseST_OPTIONS_SET(void)`
/// Saves the current options (GBP[2]) into the backup buffer (GBP[5]) and resizes the volume bars.
/// The 2-argument `TextWindowSetText` calls pass a stale a2 in the original; 0 is passed here.
pub fn panel_baseST_OPTIONS_SET(e: &mut Engine, _: u32) -> LResult<u64> {
    let p1 = GBP(e, 2);
    let p2 = GBP(e, 5);
    let l = LB(e);
    for s in [panel_baseST_OPTIONS_SET_0_1, panel_baseST_OPTIONS_SET_0_2, panel_baseST_OPTIONS_SET_0_3, panel_baseST_OPTIONS_SET_0_4] {
        APIhud::WindowResetFlags(e, e.r32(l + k(e, s)), 0x100);
    }
    APIhud::TextWindowSetText(e, e.r32(l + k(e, panel_baseST_OPTIONS_SET_0_5)), 7, &[0]);
    for (d, s) in [(0, 4), (1, 5), (4, 0), (6, 2), (5, 1), (8, 0xc), (7, 10), (9, 0xe)] {
        e.w32(p2 + d * 4, e.r32(p1 + s * 4));
    }
    for (s, a, b) in [(panel_baseST_OPTIONS_SET_0_6, 0xc, 0xd), (panel_baseST_OPTIONS_SET_0_7, 0xe, 0xf), (panel_baseST_OPTIONS_SET_0_8, 10, 0xb)] {
        APIhud::WindowSetDims(e, e.r32(l + k(e, s)), e.rf(p1 + a * 4), e.rf(p1 + b * 4));
    }
    OPTIONS_OPEN.store(true, std::sync::atomic::Ordering::Relaxed);
    APIgameobj::SetNextState(e, SELF, 6);
    Ok(0)
}

/// Port-side (P22): true while the Game Options page is shown (set on OPTIONS_SET and on the return
/// from the Sound page, cleared on the Sound page and on the return to the pause state); read by the host for its extra (non-PS2) option row.
pub static OPTIONS_OPEN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Port-side (P34): menu sound requested by the host's PC Settings page (the panel's own sound
/// slots: 10 move, 0xd confirm, 0xe back), played from the panel object in its options state.
pub static PC_PAGE_SOUND: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// Port-side (P34): selected row of the host's PC Settings page (-1 = page closed). The panel's
/// selection flower follows it like MenuRefresh moves it for the original rows: PC row i sits at
/// the goPauseSelectorInfo spot of Options row 0x16 + i (rows past the 3rd continue with the last
/// step); the host draws PC row i on the original row i's y.
pub static PC_PAGE_ROW: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(-1);
static PC_FLOWER_AT: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(-1);
/// Port-side (P34 R15): row count of the open PC Settings tab (see [`pc_row_step`])
pub static PC_PAGE_ROWS: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(0);

/// Port-side (P34 R15): the step factor for PC rows past the 3rd; a 7-row tab squeezes its rows 3..6
/// into the 6-row span so the last row clears the panel's bottom-left flower (host text and flower agree)
pub fn pc_row_step(rows: i32) -> f32 {
    if rows > 6 { 0.75 } else { 1.0 }
}

fn pc_page_flower(e: &mut Engine) {
    use std::sync::atomic::Ordering::Relaxed;
    let row = PC_PAGE_ROW.load(Relaxed);
    let n = PC_PAGE_ROWS.load(Relaxed);
    if PC_FLOWER_AT.swap(row | n << 16, Relaxed) == row | n << 16 {
        return;
    }
    let l = LB(e);
    let lang = e.r32(l + k(e, panel_baseFUNC_MenuRefresh_0_2)) as i32;
    let spot = |e: &Engine, sel: i32| {
        let p = G(e, S_goPauseSelectorInfo).wrapping_add(lang.wrapping_mul(8).wrapping_add(sel.wrapping_mul(0x18)) as u32);
        (e.rf(p), e.rf(p.wrapping_add(4)))
    };
    let (x, y) = if row < 0 {
        // page closed: back on the current original row (as MenuRefresh left it)
        spot(e, e.r32(l + 0xa8) as i32)
    } else {
        // rows 0..2 = the original Options row spots 0x16..0x18 (not evenly spaced); further rows
        // continue with the last step (spot 0x19 = APPLY CHANGES puts the flower further left)
        let r = row.min(2);
        let ((x, _), (_, y)) = (spot(e, 0x16), spot(e, 0x16 + r)); // one flower column (the gap between label and value)
        let (_, yp) = spot(e, 0x16 + r - 1);
        (x, y + (row - r) as f32 * (y - yp) * pc_row_step(n))
    };
    APIhud::WindowSetPos(e, e.r32(l + k(e, panel_baseFUNC_MenuRefresh_0_1)), x, y);
}

/// 0x0001ae40 (48 bytes) `undefined8 panel_baseST_OPTIONS(void)`
pub fn panel_baseST_OPTIONS(e: &mut Engine, _: u32) -> LResult<u64> {
    let snd = PC_PAGE_SOUND.swap(0, std::sync::atomic::Ordering::Relaxed);
    if snd != 0 {
        APInewSound::Play(e, SELF, snd, 0);
    }
    pc_page_flower(e);
    super::MENU_ROW_AT.store(LB(e) + 0xa8, std::sync::atomic::Ordering::Relaxed);
    Ok(0)
}

/// 0x0001ae70 (184 bytes) `undefined8 panel_baseCB_OPTIONS_STICK_UPDOWN(int param_1)`
pub fn panel_baseCB_OPTIONS_STICK_UPDOWN(e: &mut Engine, param_1: u32) -> LResult<u64> {
    APInewSound::Play(e, SELF, 10, 0);
    let l = LB(e);
    if e.r32(l + k(e, panel_baseCB_OPTIONS_STICK_UPDOWN_0_1)) == 0x14 || e.r32(l + k(e, panel_baseCB_OPTIONS_STICK_UPDOWN_0_2)) == 0x1c {
        panel_baseFUNC_OptionsMenuStickUpDown(e, param_1)?;
    }
    Ok(0)
}

/// 0x0001af28 (152 bytes) `undefined8 panel_baseCB_OPTIONS_STICK_LEFTRIGHT(float *param_1)`
pub fn panel_baseCB_OPTIONS_STICK_LEFTRIGHT(e: &mut Engine, param_1: u32) -> LResult<u64> {
    let l = LB(e);
    if e.r32(l + k(e, panel_baseCB_OPTIONS_STICK_LEFTRIGHT_0_1)) == 0x14 || e.r32(l + k(e, panel_baseCB_OPTIONS_STICK_LEFTRIGHT_0_2)) == 0x1c {
        panel_baseFUNC_OptionsMenuStickLeftRight(e, param_1)?;
    }
    Ok(0)
}

/// 0x0001afc0 (172 bytes) `undefined8 panel_baseCB_OPTIONS_CONFIRM(void)`
pub fn panel_baseCB_OPTIONS_CONFIRM(e: &mut Engine, _: u32) -> LResult<u64> {
    APInewSound::Play(e, SELF, 0xd, 0);
    let l = LB(e);
    if e.r32(l + k(e, panel_baseCB_OPTIONS_CONFIRM_0_1)) == 0x14 || e.r32(l + k(e, panel_baseCB_OPTIONS_CONFIRM_0_2)) == 0x1c {
        panel_baseFUNC_OptionsMenuConfirm(e)?;
    }
    Ok(0)
}

/// 0x0001b070 (948 bytes) `undefined8 panel_baseCB_OPTIONS_CANCEL(void)`
/// Restores the options from the backup buffer. The miscHUD Set*Volume calls take the volume as a float
/// in $f12 (Ghidra shows `extraout_a0_lo`): music = p1[0], FX = p1[2], movies = p1[1] (from the disasm);
/// their bits are passed as the u32 argument. 2-arg TextWindowSetText: stale a2 -> 0.
pub fn panel_baseCB_OPTIONS_CANCEL(e: &mut Engine, _: u32) -> LResult<u64> {
    let p1 = GBP(e, 2);
    let p2 = GBP(e, 5);
    let l = LB(e);
    if e.r32(l + k(e, panel_baseCB_OPTIONS_CANCEL_0_1)) == 0x14 {
        e.w32(p1 + 0x10, e.r32(p2));
        e.w32(p1 + 0x14, e.r32(p2 + 4));
        APIinput::InputEnableVibration(e, e.r32(GBP(e, 0) + 0x4c) as i32, e.r32(p1 + 0x10) as i32);
        panel_baseFUNC_DrawNewMenu(e, 7, 9, 0xc)?;
        APIhud::WindowResetFlags(e, e.r32(l + k(e, panel_baseCB_OPTIONS_CANCEL_0_2)), 0x100);
        APIhud::TextWindowSetText(e, e.r32(l + k(e, panel_baseCB_OPTIONS_CANCEL_0_3)), 8, &[0]);
        APIinput::InputConfigActivate(e, 10, 0);
        APIinput::InputConfigActivate(e, 8, 1);
        APIinput::InputConfigSetState(e, 8, 0x192);
        OPTIONS_OPEN.store(false, std::sync::atomic::Ordering::Relaxed);
    } else if e.r32(l + k(e, panel_baseCB_OPTIONS_CANCEL_0_4)) == 0x1c {
        for (d, s) in [(0, 4), (2, 6), (1, 5), (0xe, 9), (0xc, 8), (10, 7)] {
            e.w32(p1 + d * 4, e.r32(p2 + s * 4));
        }
        for (s, a, b) in [(panel_baseCB_OPTIONS_CANCEL_0_5, 0xc, 0xd), (panel_baseCB_OPTIONS_CANCEL_0_6, 0xe, 0xf), (panel_baseCB_OPTIONS_CANCEL_0_7, 10, 0xb)] {
            APIhud::WindowSetDims(e, e.r32(l + k(e, s)), e.rf(p1 + a * 4), e.rf(p1 + b * 4));
        }
        LIBMiscHUD::miscHUDFUNC_SetMusicVolume(e, e.r32(p1));
        LIBMiscHUD::miscHUDFUNC_SetFXVolume(e, e.r32(p1 + 8));
        LIBMiscHUD::miscHUDFUNC_SetMoviesVolume(e, e.r32(p1 + 4));
        APInewSound::StopSound(e, SELF, k(e, ID_80000012));
        APIhud::TextWindowSetText(e, e.r32(l + k(e, panel_baseCB_OPTIONS_CANCEL_0_8)), 6, &[0]);
        panel_baseFUNC_DrawNewMenu(e, 0x14, 0x16, 0x19)?;
        panel_baseFUNC_OptionsMenuUpdate(e)?;
        APIhud::WindowResetFlags(e, e.r32(l + k(e, panel_baseCB_OPTIONS_CANCEL_0_9)), 0x100);
        // port-side (P32 R55): Triangle on the Sound page returns to the Game Options page (not the pause menu)
        OPTIONS_OPEN.store(true, std::sync::atomic::Ordering::Relaxed);
    }
    APInewSound::Play(e, SELF, 0xe, 0);
    APIgameobj::SetNextState(e, SELF, 4);
    Ok(0)
}

/// 0x0001b428 (48 bytes) `undefined4 panel_baseCB_TIMER_OPTIONS_VIBRATION(void)`
pub fn panel_baseCB_TIMER_OPTIONS_VIBRATION(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(1)
}

/// 0x0001b458 (1364 bytes) `void panel_baseFUNC_CreateInput_Options(void)`
/// Config 10 (options menu): sticks 0x3f/0x40, confirm 0x39, triangle 0x3c; same device rows as config 8.
pub fn panel_baseFUNC_CreateInput_Options(e: &mut Engine) -> LResult<()> {
    APIinput::InputConfigAdd(e, 10);
    let pad = |e: &Engine| e.r32(GBP(e, 0) + 0x4c) as i32;
    type Dev = (i32, i32, i32, i32, i32, i32);
    let rows: [(i32, &[Dev], (i32, i32, i32), u32); 4] = [
        (0x3f, &[(4, -1, 0, 1, 0, 0x8001), (0xd, -1, 1, 1, -100, 0xd), (0xe, -1, 1, 1, 100, 0xc), (0xd, 1, 2, 1, -100, 0xd0), (0xe, 1, 2, 1, 100, 200)], (0x11, 2, 0x30_0000), 2),
        (0x40, &[(3, -1, 0, 0, 0, 0x8000), (0xb, -1, 1, 0, -100, 0xe), (0xc, -1, 1, 0, 100, 0xf), (0xb, 1, 2, 0, -100, 0xcb), (0xc, 1, 2, 0, 100, 0xcd)], (0x12, 0x2b, 0xc0_0000), 4),
        (0x39, &[(0x3d, -1, 0, 2, 0, 0), (0x3d, 1, 1, 2, 0, 0x39)], (0x13, 0x1e, 8), 0x80),
        (0x3c, &[(0x40, -1, 0, 2, 0, 3), (0x40, 1, 1, 2, 0, 0xe)], (0x14, 0x24, 8), 0x10),
    ];
    for (a, devs, (cb, id, fl), st) in rows {
        APIinput::InputActionAdd(e, 10, a, 0);
        for &(d, p, x, y, z, w) in devs {
            let p = if p < 0 { pad(e) } else { p };
            APIinput::InputDeviceAdd(e, 10, a, d, p, x, y, z, w);
        }
        APIinput::InputContextAdd(e, SELF, cb as u32, 10, a, id, fl);
        APIinput::InputContextSetState(e, 10, a, id, st);
    }
    APIinput::InputConfigActivate(e, 10, 0);
    Ok(())
}

/// 0x0001b9b0 (336 bytes) `void panel_baseFUNC_OptionsMenuStickUpDown(int param_1)`
pub fn panel_baseFUNC_OptionsMenuStickUpDown(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    let l = LB(e);
    let y = e.rf(param_1 + 4);
    if y < -0.33f32 {
        e.w32(l + 0xa8, e.r32(l + 0xa8).wrapping_add(1));
    } else if 0.33f32 < y {
        e.w32(l + 0xa8, e.r32(l + 0xa8).wrapping_sub(1));
    }
    let a = e.r32(l + k(e, panel_baseFUNC_OptionsMenuStickUpDown_0_1)) as i32;
    let b = e.r32(l + k(e, panel_baseFUNC_OptionsMenuStickUpDown_0_2)) as i32;
    panel_baseFUNC_MenuRefresh(e, a, b)?;
    // 0x20 = movies volume bar: preview sound loops while selected
    if e.r32(l + 0xa8) == 0x20 {
        APInewSound::Play(e, SELF, k(e, ID_80000012), 0);
    } else {
        APInewSound::StopSound(e, SELF, k(e, ID_80000012));
    }
    Ok(())
}

/// 0x0001bb00 (948 bytes) `void panel_baseFUNC_OptionsMenuStickLeftRight(float *param_1)`
/// Set*Volume take the volume as a float in $f12 (bits passed as the u32 argument): music = o[0],
/// FX = o[2], movies = o[1] (o = GBP[2]). SetOptionVibration's 3rd arg is the pInput table pointer.
pub fn panel_baseFUNC_OptionsMenuStickLeftRight(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    let x = e.rf(param_1);
    if x < -0.33f32 || 0.33f32 < x {
        let l = LB(e);
        let o = GBP(e, 2);
        match e.r32(l + 0xa8) {
            0x16 => {
                APInewSound::Play(e, SELF, 10, 0);
                LIBMiscHUD::miscHUDFUNC_SetOptionCamera(e, o as i32, k(e, ID_2a00001e));
                panel_baseFUNC_OptionsMenuUpdate(e)?;
            }
            0x17 => {
                APInewSound::Play(e, SELF, 10, 0);
                let inp = e.r32(G(e, S_pInput));
                LIBMiscHUD::miscHUDFUNC_SetOptionVibration(e, GBP(e, 0) as i32, o as i32, inp as i32);
                panel_baseFUNC_OptionsMenuUpdate(e)?;
            }
            0x1e => {
                let w = e.r32(l + k(e, panel_baseFUNC_OptionsMenuStickLeftRight_0_1));
                if LIBMiscHUD::miscHUDFUNC_VolumeBarUpdate(e, x, 1.0, o, w, o + 0x30, o + 0x34) != 0 {
                    APInewSound::Play(e, SELF, 10, 0);
                    let o = GBP(e, 2);
                    LIBMiscHUD::miscHUDFUNC_SetMusicVolume(e, e.r32(o));
                }
            }
            0x1f => {
                let w = e.r32(l + k(e, panel_baseFUNC_OptionsMenuStickLeftRight_0_2));
                if LIBMiscHUD::miscHUDFUNC_VolumeBarUpdate(e, x, 1.0, o + 8, w, o + 0x38, o + 0x3c) != 0 {
                    APInewSound::Play(e, SELF, 10, 0);
                    let o = GBP(e, 2);
                    LIBMiscHUD::miscHUDFUNC_SetFXVolume(e, e.r32(o + 8));
                }
            }
            0x20 => {
                let w = e.r32(l + k(e, panel_baseFUNC_OptionsMenuStickLeftRight_0_3));
                if LIBMiscHUD::miscHUDFUNC_VolumeBarUpdate(e, x, 0.85, o + 4, w, o + 0x28, o + 0x2c) != 0 {
                    APInewSound::Play(e, SELF, 10, 0);
                    let o = GBP(e, 2);
                    LIBMiscHUD::miscHUDFUNC_SetMoviesVolume(e, e.r32(o + 4));
                }
            }
            _ => {}
        }
        panel_baseFUNC_OptionsMenuUpdate(e)?;
    }
    Ok(())
}

/// 0x0001beb8 (264 bytes) `undefined4 panel_baseFUNC_OptionsMenuUpdate(void)`
pub fn panel_baseFUNC_OptionsMenuUpdate(e: &mut Engine) -> LResult<u32> {
    let l = LB(e);
    let o = GBP(e, 2);
    if e.r32(o + 0x10) == 0 {
        APIhud::TextWindowSetText(e, e.r32(l + k(e, panel_baseFUNC_OptionsMenuUpdate_0_1)), 0x3a, &[9]);
    } else {
        APIhud::TextWindowSetText(e, e.r32(l + k(e, panel_baseFUNC_OptionsMenuUpdate_0_2)), 0x39, &[9]);
    }
    let r = if e.r32(o + 0x14) == 0 {
        APIhud::TextWindowSetText(e, e.r32(l + k(e, panel_baseFUNC_OptionsMenuUpdate_0_3)), 0x3d, &[9])
    } else {
        APIhud::TextWindowSetText(e, e.r32(l + k(e, panel_baseFUNC_OptionsMenuUpdate_0_4)), 0x3e, &[9])
    };
    Ok(r)
}

/// 0x0001bfc0 (908 bytes) `undefined ** panel_baseFUNC_OptionsMenuConfirm(void)`
/// All 16 shape groups differ only in per-build literals (k slots / object ids / jump table address),
/// except group 9 (695761b7) which also does `SetNextState(SELF, 4)` after "Back" (case 0x19).
/// The return value is whatever was last in v0 (garbage the caller ignores): the last call's result,
/// or `sel - 0x16` when the selection is outside the switch.
pub fn panel_baseFUNC_OptionsMenuConfirm(e: &mut Engine) -> LResult<Ptr> {
    let p1 = GBP(e, 2);
    let i2 = GBP(e, 5);
    let l = LB(e);
    let sel = e.r32(l + 0xa8);
    let mut r = sel.wrapping_sub(0x16);
    match sel {
        0x16 => {
            LIBMiscHUD::miscHUDFUNC_SetOptionCamera(e, p1 as i32, k(e, ID_2a00001e));
            r = panel_baseFUNC_OptionsMenuUpdate(e)?;
        }
        0x17 => {
            let inp = e.r32(G(e, S_pInput));
            LIBMiscHUD::miscHUDFUNC_SetOptionVibration(e, GBP(e, 0) as i32, p1 as i32, inp as i32);
            r = panel_baseFUNC_OptionsMenuUpdate(e)?;
        }
        0x18 => {
            OPTIONS_OPEN.store(false, std::sync::atomic::Ordering::Relaxed); // port-side: Sound page
            panel_baseFUNC_DrawNewMenu(e, 0x1c, 0x1e, 0x20)?;
            // 2-arg TextWindowSetText: stale a2 -> 0
            r = APIhud::TextWindowSetText(e, e.r32(l + k(e, panel_baseFUNC_OptionsMenuConfirm_0_1)), 5, &[0]);
        }
        0x19 => {
            // port-side (P32 R54): Back always returns to the pause main menu; only group 9 also leaves
            // state 6, so clear the host's options-row flag here for every build
            OPTIONS_OPEN.store(false, std::sync::atomic::Ordering::Relaxed);
            panel_baseFUNC_DrawNewMenu(e, 7, 9, 0xc)?;
            APIhud::WindowResetFlags(e, e.r32(l + k(e, panel_baseFUNC_OptionsMenuConfirm_0_2)), 0x100);
            APIhud::TextWindowSetText(e, e.r32(l + k(e, panel_baseFUNC_OptionsMenuConfirm_0_3)), 8, &[0]);
            APIinput::InputConfigActivate(e, 10, 0);
            APIinput::InputConfigActivate(e, 8, 1);
            r = APIinput::InputConfigSetState(e, k(e, panel_baseFUNC_OptionsMenuConfirm_0_4) as i32, k(e, panel_baseFUNC_OptionsMenuConfirm_0_5) as i32);
            if g(e, G_panel_baseFUNC_OptionsMenuConfirm) == 9 {
                r = APIgameobj::SetNextState(e, SELF, 4);
            }
        }
        0x1e | 0x1f | 0x20 => {
            for (s, a, b) in [(panel_baseFUNC_OptionsMenuConfirm_0_6, 0xc, 0xd), (panel_baseFUNC_OptionsMenuConfirm_0_7, 0xe, 0xf), (panel_baseFUNC_OptionsMenuConfirm_0_8, 10, 0xb)] {
                APIhud::WindowGetDims(e, e.r32(l + k(e, s)), p1 + a * 4, p1 + b * 4);
            }
            for (d, s) in [(0x10, 0), (0x18, 2), (0x14, 1), (0x20, 0xc), (0x1c, 10), (0x24, 0xe)] {
                e.w32(i2 + d, e.r32(p1 + s * 4));
            }
            APInewSound::StopSound(e, SELF, k(e, ID_80000012));
            OPTIONS_OPEN.store(true, std::sync::atomic::Ordering::Relaxed); // port-side: back on Game Options
            panel_baseFUNC_DrawNewMenu(e, 0x14, 0x16, 0x19)?;
            APIhud::TextWindowSetText(e, e.r32(l + k(e, panel_baseFUNC_OptionsMenuConfirm_0_9)), 6, &[0]);
            r = panel_baseFUNC_OptionsMenuUpdate(e)?;
        }
        _ => {}
    }
    Ok(r)
}

/// 0x0001c350 (236 bytes) `void panel_baseFUNC_VolumeMenuStickUpDown(int param_1)`
pub fn panel_baseFUNC_VolumeMenuStickUpDown(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    let l = LB(e);
    let y = e.rf(param_1 + 4);
    if y < -0.33f32 {
        e.w32(l + 0xa8, e.r32(l + 0xa8).wrapping_add(1));
    } else if 0.33f32 < y {
        e.w32(l + 0xa8, e.r32(l + 0xa8).wrapping_sub(1));
    }
    let a = e.r32(l + k(e, panel_baseFUNC_VolumeMenuStickUpDown_0_1)) as i32;
    let b = e.r32(l + k(e, panel_baseFUNC_VolumeMenuStickUpDown_0_2)) as i32;
    panel_baseFUNC_MenuRefresh(e, a, b)?;
    Ok(())
}

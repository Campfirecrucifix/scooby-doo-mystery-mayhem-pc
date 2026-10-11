//! panel.c of the 6 Frontier Land / EP4 levels whose HUD replaces the tome-meter slide-in with a level
//! meter: the gold-nugget meter (EP3 L09 3ce3c13c, EP3 L03 a8417f38) and the gas meter (EP4 L03/L03A/
//! L09A/L09B: 99c250a4 af7a3894 ceb7d472 bcb77619). They register their own in-game state
//! (panel_LocalST_IN_GAME, state 0x22; the gas levels also replace base state 2 with it) and use
//! message/notify id 0x23 for the collision-notification timer instead of 0x22.
//! Offsets into pLocalData_BasePanel are raw (these builds use the reference layout).
use super::local::LOCAL;
use super::panel::panelCB_TIMER_COLLNOTIFY;
use super::panel_base::*;
use super::*;
use crate::engine::*;
use crate::logic::LResult;

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Kind {
    /// gold nuggets: needle object, fader object, EP3 L09 variant (3ce3c13c)
    Gold { needle: u32, fader: u32, l09: bool },
    /// gas: needle object, fader object
    Gas { needle: u32, fader: u32 },
}

pub(crate) fn kind(e: &Engine) -> Option<Kind> {
    Some(match LOCAL[e.rt.cur_build()].sha8 {
        "3ce3c13c" => Kind::Gold { needle: 0xf00_0551, fader: 0xf00_0556, l09: true },
        "a8417f38" => Kind::Gold { needle: 0xf00_054c, fader: 0xf00_0551, l09: false },
        "99c250a4" => Kind::Gas { needle: 0xf00_04a1, fader: 0xf00_04a6 },
        "af7a3894" => Kind::Gas { needle: 0xf00_046d, fader: 0xf00_0472 },
        "bcb77619" => Kind::Gas { needle: 0xf00_04a6, fader: 0xf00_04ab },
        "ceb7d472" => Kind::Gas { needle: 0xf00_046b, fader: 0xf00_0470 },
        _ => return None,
    })
}

fn set(e: &mut Engine, o: u32) {
    let h = e.r32(LB(e) + o);
    APIhud::WindowSetFlags(e, h, 0x100);
}
fn reset(e: &mut Engine, o: u32) {
    let h = e.r32(LB(e) + o);
    APIhud::WindowResetFlags(e, h, 0x100);
}
/// `**(float **)(pGlobalData_BasePanel + 4)`: the courage ("cool") meter value
fn cool(e: &Engine) -> Ptr {
    GBP(e, 1)
}

/// panelST_FIRSTMOVE_LOCAL
pub(crate) fn first_move(e: &mut Engine, k: Kind) -> LResult<u64> {
    let l = &LOCAL[e.rt.cur_build()];
    match k {
        Kind::Gold { needle, fader, l09 } => {
            if l09 {
                e.wf(cool(e), 1.0);
            }
            panel_baseST_FIRSTMOVE(e, 0)?;
            for &(t, o, n) in l.wins {
                panel_baseFUNC_CreateHudWins(e, img(e, t) as i32, (LP(e) + o) as i32, n)?;
            }
            if l09 {
                APIgameobj::Rotate(e, needle, LB(e), 0.0);
            } else {
                panel_baseFUNC_InitIngredientWin(e)?;
                APIgameobj::SetRotation(e, needle, LB(e), 0.0);
            }
            let wp = G(e, S_aMyWinParams);
            LIBApiText::InitStruct(e, wp);
            e.wf(wp, 3.5);
            e.w32(LP(e) + 0x600, 0);
            e.w32(LP(e) + 0x5fc, 0);
            let (f, t) = (lit(e, "TOD_Normal_Meter"), lit(e, "gold_nuggets"));
            let p = tex_fader_params(e, 0x3dcc_cccd, f, t);
            APIgraphics::TexFaderPlay(e, fader, p);
            if l09 {
                APIgameobj::SendMessageTo(e, 0x2a10_001f, 0, 0x23, 0, 0);
            }
        }
        Kind::Gas { needle, fader } => {
            let (f, t) = (lit(e, "TOD_Normal_Meter"), lit(e, "gasoline_can"));
            let p = tex_fader_params(e, 0x3dcc_cccd, f, t);
            APIgraphics::TexFaderPlay(e, fader, p);
            LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 2, 0, 1);
            panel_baseST_FIRSTMOVE(e, 0)?;
            for &(t, o, n) in l.wins {
                panel_baseFUNC_CreateHudWins(e, img(e, t) as i32, (LP(e) + o) as i32, n)?;
            }
            panel_baseFUNC_InitIngredientWin(e)?;
            APIgameobj::SetRotation(e, needle, LB(e), 3.1415927);
            e.w32(LP(e) + 0x5ec, 0);
            e.wf(LP(e) + 0x5e8, 180.0);
            e.w32(LP(e) + 0x5f0, 1);
            let t = APIextra::TimerCreate(e);
            e.w32(LP(e) + 0x5e0, t);
            APIinput::InputConfigSetState(e, 7, 0x200_0110);
        }
    }
    APIgameobj::SetNextState(e, SELF, 0x22);
    Ok(0)
}

/// panel_LocalST_IN_GAME (state 0x22, and state 2 in the gas levels)
pub fn panel_LocalST_IN_GAME(e: &mut Engine, _: u32) -> LResult<u64> {
    let k = kind(e).expect("panel_LocalST_IN_GAME: not a gold/gas meter build");
    let f = |e: &Engine| e.r32(LB(e) + 0x468);
    let not = |e: &mut Engine| -> LResult<()> {
        e.w32(LB(e) + 0x468, f(e) & 0xffff_fffb);
        panel_baseFUNC_InOut_ObjNotification(e, e.r32(LB(e) + 0x48c), 0)?;
        Ok(())
    };
    match k {
        Kind::Gold { l09, .. } => {
            if !l09 {
                let gp = e.r32(e.r32(G(e, S_pGlobalData_Panel)));
                let m = e.r32(gp + 0x3c);
                if 9 < m && m < 0xd {
                    APIinput::InputConfigActivate(e, 7, 1);
                }
            }
            if f(e) & 1 == 0 || f(e) & 2 != 0 {
                if f(e) & 2 != 0 {
                    panel_baseFUNC_InOut_ObjNotification(e, e.r32(LB(e) + 0x48c), 1)?;
                }
            } else {
                not(e)?;
            }
            if e.rf(cool(e)) != e.rf(LB(e) + 0x74) {
                panel_baseFUNC_UpdateCoolMeter(e)?;
            }
        }
        Kind::Gas { .. } => {
            if e.r32(LP(e) + 0x5ec) != 0 {
                e.w32(LP(e) + 0x5ec, 0);
                if 0.0 < e.rf(LP(e) + 0x5e8) {
                    panel_LocalFUNC_UpdateGazMeter(e)?;
                }
            }
            if e.rf(cool(e)) != e.rf(LB(e) + 0x74) {
                panel_baseFUNC_UpdateCoolMeter(e)?;
                if e.rf(cool(e)) <= 0.0 {
                    APIgameobj::SendMessageTo(e, 0x2a00_000d, 0, 0x17d, 0x80, 0);
                }
            }
            if f(e) & 1 == 0 || f(e) & 2 != 0 || f(e) & 8 != 0 {
                if f(e) & 2 != 0 && f(e) & 8 == 0 {
                    panel_baseFUNC_InOut_ObjNotification(e, e.r32(LB(e) + 0x48c), 1)?;
                }
            } else {
                not(e)?;
            }
            if f(e) & 0x10 == 0 || f(e) & 0x20 != 0 {
                if f(e) & 0x20 != 0 {
                    panel_baseFUNC_InOut_ActNotification(e, e.r32(LB(e) + 0x488), 1)?;
                }
            } else {
                e.w32(LB(e) + 0x468, f(e) & 0xffff_ffbf);
                panel_baseFUNC_InOut_ActNotification(e, e.r32(LB(e) + 0x488), 0)?;
            }
        }
    }
    Ok(0)
}

/// panel_LocalFUNC_UpdateGoldNuggetMeter(float angle)
pub fn panel_LocalFUNC_UpdateGoldNuggetMeter(e: &mut Engine, param_1: f32) -> LResult<u64> {
    let Some(Kind::Gold { needle, .. }) = kind(e) else { panic!("not a gold meter build") };
    APIgameobj::SetRotation(e, needle, LB(e), -param_1 * 0.017453292);
    let v = e.rf(LP(e) + 0x5fc) + e.rf(LP(e) + 0x5f8);
    e.wf(LP(e) + 0x5fc, v);
    e.w32(LP(e) + 0x600, 0);
    Ok(0)
}

/// panel_LocalFUNC_UpdateGazMeter: turns the needle by the pending delta LP+0x5e4 (degrees), the level
/// LP+0x5e8 stays within 180.
pub fn panel_LocalFUNC_UpdateGazMeter(e: &mut Engine) -> LResult<u64> {
    let Some(Kind::Gas { needle, .. }) = kind(e) else { panic!("not a gas meter build") };
    let (d, lv) = (LP(e) + 0x5e4, LP(e) + 0x5e8);
    if e.rf(d) <= 0.0 || 180.0 < e.rf(lv) {
        if e.rf(d) < 0.0 {
            APIgameobj::Rotate(e, needle, LB(e), -e.rf(d) * 0.017453292);
            e.wf(lv, e.rf(lv) + e.rf(d));
        }
    } else {
        if 180.0 < e.rf(lv) + e.rf(d) {
            e.wf(d, 180.0 - e.rf(lv));
        }
        APIgameobj::Rotate(e, needle, LB(e), -e.rf(d) * 0.017453292);
        e.wf(lv, e.rf(lv) + e.rf(d));
        e.w32(d, 0);
    }
    Ok(0)
}

/// panel_LocalFUNC_RefullGazMeter
pub fn panel_LocalFUNC_RefullGazMeter(e: &mut Engine) -> LResult<()> {
    e.wf(LP(e) + 0x5e4, 180.0);
    panel_LocalFUNC_UpdateGazMeter(e)?;
    Ok(())
}

/// panelFUNC_RegisterTimer (notify 0x23, no TimerPause unlike the reference)
pub(crate) fn register_timer(e: &mut Engine) -> LResult<()> {
    let t = e.r32(LP(e) + LOCAL[e.rt.cur_build()].cn_timer);
    APIextra::TimerReset(e, t);
    APIextra::TimerRegisterNotify(e, SELF, t, 0x23);
    APIextra::TimerSetFrequency(e, t, 2.0);
    Ok(())
}

/// The collision-notification part of message 0x33 (also message 2 in the gas levels, a fall-through of
/// the original's compare chain).
fn coll_33(e: &mut Engine, param_2: u32) -> LResult<()> {
    let l = &LOCAL[e.rt.cur_build()];
    let lp = LP(e);
    if e.r32(lp + l.cn_cur) != 0x7fff_fffe {
        panelCB_TIMER_COLLNOTIFY(e, 0)?;
    }
    e.w32(lp + l.cn_i0, 0x7fff_fffe);
    e.w32(lp + l.cn_i1, 0x7fff_fffe);
    panel_baseFUNC_GetDisplayInfo(e, param_2, lp + l.cn_cur)?;
    if e.r32(lp + l.cn_i0) != 0x7fff_fffe {
        e.w32(lp + l.cn_cur, param_2);
        for o in [l.cn_i0, l.cn_i1] {
            let i = e.r32(lp + o);
            let w = e.r32(lp + l.cn_wins + i.wrapping_mul(4));
            APIhud::WindowResetFlags(e, w, 0x100);
        }
        APIinput::InputConfigActivate(e, 7, 0);
        register_timer(e)?;
    }
    Ok(())
}

/// e4/e8 shown, b0/e0 hidden (the level meter replaces the tome/courage meters)
fn meters_back(e: &mut Engine) {
    reset(e, 0xe4);
    reset(e, 0xe8);
    set(e, 0xb0);
    set(e, 0xe0);
}

/// panelFUNC_Message. Always returns 0 (the base handler's result is dropped).
pub(crate) fn message(e: &mut Engine, k: Kind, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    let base = match k {
        Kind::Gold { l09: true, .. } => match param_1 {
            0x4d => {
                e.w32(LB(e) + 0x40, 1);
                set(e, 0xe4);
                set(e, 0xe8);
                false
            }
            0x33 => {
                coll_33(e, param_2)?;
                true
            }
            0x13 => {
                meters_back(e);
                APIgameobj::SetNextState(e, SELF, 0x22);
                APIinput::InputConfigSetState(e, 7, 0x200_0110);
                false
            }
            0x4c | 0x4e => {
                set(e, 0xac);
                false
            }
            0x51 => {
                reset(e, 0xac);
                meters_back(e);
                false
            }
            0x52 => {
                if param_2 == 1 {
                    e.w32(LB(e) + 0x40, 0);
                    meters_back(e);
                }
                false
            }
            0x183 => {
                panel_LocalFUNC_UpdateGoldNuggetMeter(e, param_2 as f32 * 0.8780488)?;
                true
            }
            _ => true,
        },
        Kind::Gold { l09: false, .. } => match param_1 {
            0x4c | 0x4e => {
                set(e, 0xac);
                false
            }
            0x1e | 0x1f => false,
            0x13 => {
                meters_back(e);
                APIgameobj::SetNextState(e, SELF, 0x22);
                APIinput::InputConfigSetState(e, 7, 0x200_0110);
                false
            }
            0x33 => {
                coll_33(e, param_2)?;
                true
            }
            0x51 => {
                reset(e, 0xac);
                meters_back(e);
                false
            }
            0x4d => {
                e.w32(LB(e) + 0x40, 1);
                set(e, 0xac);
                set(e, 0xe4);
                set(e, 0xe8);
                false
            }
            0x52 => {
                if param_2 == 1 {
                    e.w32(LB(e) + 0x40, 0);
                    meters_back(e);
                }
                false
            }
            0x183 => {
                panel_LocalFUNC_UpdateGoldNuggetMeter(e, param_2 as f32 * 0.8780488)?;
                true
            }
            _ => true,
        },
        Kind::Gas { .. } => match param_1 {
            0x4d => {
                e.w32(LB(e) + 0x40, 1);
                set(e, 0xe4);
                set(e, 0xe8);
                false
            }
            0x1e | 0x1f => false,
            2 | 0x33 => {
                coll_33(e, param_2)?;
                true
            }
            0x13 => {
                // unlike the gold levels this one falls through to the base handler
                meters_back(e);
                APIgameobj::SetNextState(e, SELF, 0x22);
                APIinput::InputConfigSetState(e, 7, 0x200_0110);
                true
            }
            0x4c | 0x4e => {
                set(e, 0xac);
                false
            }
            0x52 => {
                if param_2 == 1 {
                    e.w32(LB(e) + 0x40, 0);
                    meters_back(e);
                }
                false
            }
            0x51 => {
                reset(e, 0xac);
                meters_back(e);
                false
            }
            0x17b => {
                e.wf(LP(e) + 0x5e4, 50.0);
                panel_LocalFUNC_UpdateGazMeter(e)?;
                true
            }
            0x17a => {
                let v = e.r32(param_3);
                e.w32(LP(e) + 0x5e4, v);
                e.w32(LP(e) + 0x5ec, 1);
                true
            }
            0x17c => {
                panel_LocalFUNC_RefullGazMeter(e)?;
                true
            }
            _ => true,
        },
    };
    if base {
        panel_baseFUNC_MessageHandler(e, param_1, param_2, param_3)?;
    }
    Ok(0)
}

/// panelConstructor
pub(crate) fn constructor(e: &mut Engine, k: Kind, api: Ptr, instanceCount: i32) -> LResult<()> {
    let _ = api;
    panel_baseConstructor(e, api, instanceCount)?;
    if instanceCount == 1 {
        APIcons::SetExternalFunction(e, 0x21, super::panel::panelST_FIRSTMOVE_LOCAL);
        if matches!(k, Kind::Gas { .. }) {
            APIcons::SetExternalFunction(e, 2, panel_LocalST_IN_GAME);
        }
        APIcons::SetExternalFunction(e, 0x22, panel_LocalST_IN_GAME);
        APIcons::SetExternalFunction(e, 0x23, panelCB_TIMER_COLLNOTIFY);
        let slot = G(e, S_pGlobalData_Panel);
        APIcons::GetGlobalData(e, slot, 0x18);
        for (i, (buf, size)) in [(0, 0x8f8), (1, 0x68), (3, 0x40)].into_iter().enumerate() {
            let v = APIgameobj::GetGameBuffer(e, buf, size, 1);
            let gd = e.r32(G(e, S_pGlobalData_Panel));
            e.w32(gd + i as u32 * 4, v);
        }
    }
    APIcons::SetMessageFunction(e, super::panel::panelFUNC_Message);
    let slot = G(e, S_pLocalData_Panel);
    APIcons::GetLocalData(e, slot, LOCAL[e.rt.cur_build()].local_size as i32);
    if matches!(k, Kind::Gold { .. }) {
        let t = APIextra::TimerCreate(e);
        e.w32(LP(e) + 0x604, t);
    }
    // EFClippingParams: only the first 4 words are rewritten
    let b = e.alloca(0x28);
    APIgameobj::GetClippingParams(e, SELF, b);
    e.w32(b + 4, 0xbf80_0000);
    e.w32(b + 0xc, 0);
    e.w32(b + 8, 0);
    e.w32(b, 0);
    APIgameobj::SetClippingParams(e, SELF, b, 0);
    APIcons::SetInitialState(e, 0x21);
    Ok(())
}

/// panelDestructor. Gold: base, timer, windows. Gas: base only (its FIRSTMOVE timer is never destroyed and
/// its windows are not deleted, as in the original).
pub(crate) fn destructor(e: &mut Engine, k: Kind, instanceCount: i32) -> LResult<()> {
    panel_baseDestructor(e, instanceCount)?;
    if let Kind::Gold { .. } = k {
        APIextra::TimerDestroy(e, e.r32(LP(e) + 0x604));
        for &(o, n) in LOCAL[e.rt.cur_build()].dtor {
            for i in 0..n {
                let w = e.r32(LP(e) + o + i * 4);
                if w != 0x3b9a_c9f7 {
                    APIhud::WindowDelete(e, w);
                }
            }
        }
    }
    Ok(())
}

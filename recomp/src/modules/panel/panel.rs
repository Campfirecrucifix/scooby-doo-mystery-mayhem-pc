//! panel.c (6 functions). Addresses are module-relative (.text at 0x10000) in panel__014a381f.
//! This half of the module is level-specific: per-build data comes from local.rs (gen_panel_local.py).
//! Builds whose body differs beyond offsets/counts (gas meter, gold nuggets, level messages) panic with
//! "unported shape" (std_* = false in local.rs).
use super::local::{Local, LOCAL};
use super::*;
use crate::engine::*;
use crate::logic::LResult;

fn loc(e: &Engine) -> &'static Local {
    &LOCAL[e.rt.cur_build()]
}
#[cold]
fn unported_local(e: &Engine, f: &str) -> ! {
    panic!("{f}: level-specific body of build {} is not ported yet", loc(e).sha8)
}
/// The two collision-notification windows `pLocalData_Panel + cn_wins + idx*4`.
fn cn_windows(e: &Engine) -> [u32; 2] {
    let (l, lp) = (loc(e), LP(e));
    [l.cn_i0, l.cn_i1].map(|o| {
        let i = e.r32(lp + o);
        e.r32(lp + l.cn_wins + i.wrapping_mul(4))
    })
}

/// 0x000304e8 (288 bytes) `undefined8 panelST_FIRSTMOVE_LOCAL(void)`
pub fn panelST_FIRSTMOVE_LOCAL(e: &mut Engine, _: u32) -> LResult<u64> {
    if let Some(k) = super::panel_frontier::kind(e) {
        return super::panel_frontier::first_move(e, k);
    }
    let l = loc(e);
    // timer levels (see panelFUNC_Message) also clear their flag word at local_size-4 after the windows
    let flag = matches!(l.sha8, "05789c5e" | "695761b7" | "cb9dbf66" | "dbdc8239" | "fe754b69" | "cd8e4e92");
    if !l.std_first && !flag {
        unported_local(e, "panelST_FIRSTMOVE_LOCAL");
    }
    if l.clip {
        LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 2, 0, 1);
    }
    panel_baseST_FIRSTMOVE(e, 0)?;
    for &(t, o, n) in l.wins {
        panel_baseFUNC_CreateHudWins(e, img(e, t) as i32, (LP(e) + o) as i32, n)?;
    }
    if flag {
        e.w32(LP(e) + l.local_size - 4, 0);
    }
    APIgameobj::SetNextState(e, SELF, l.next_state);
    Ok(0)
}

/// 0x00030608 (304 bytes) `bool panelCB_TIMER_COLLNOTIFY(void)`
pub fn panelCB_TIMER_COLLNOTIFY(e: &mut Engine, _: u32) -> LResult<u64> {
    let l = loc(e);
    for w in cn_windows(e) {
        APIhud::WindowSetFlags(e, w, 0x100);
    }
    e.w32(LP(e) + l.cn_cur, 0x7fff_fffe);
    APIextra::TimerReset(e, e.r32(LP(e) + l.cn_timer));
    let b = e.r32(LP(e) + 0x1c) == 0;
    if b {
        APIinput::InputConfigActivate(e, 7, 1);
    }
    Ok(b as u64)
}

/// 0x00030740 (1020 bytes) `undefined4 panelFUNC_Message(uint param_1,uint param_2,float *param_3)`
pub fn panelFUNC_Message(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    if let Some(k) = super::panel_frontier::kind(e) {
        return super::panel_frontier::message(e, k, param_1, param_2, param_3);
    }
    let l = loc(e);
    // Timer levels (05789c5e 695761b7 cb9dbf66 dbdc8239 fe754b69): the reference body plus messages
    // 0x11b/0x11c (start/stop the 0.2 s notify timer) and 0x183 (flag at local_size-4). 695761b7 keeps
    // the notification windows as they are on pause/unpause.
    let timer_lvl = matches!(l.sha8, "05789c5e" | "695761b7" | "cb9dbf66" | "dbdc8239" | "fe754b69");
    if !l.std_msg && !timer_lvl {
        return panelFUNC_Message_Boss(e, param_1, param_2, param_3);
    }
    let wins = l.sha8 != "695761b7";
    let cur = |e: &Engine| e.r32(LP(e) + l.cn_cur);
    match param_1 {
        0x11b if timer_lvl => {
            let t = e.r32(LP(e) + l.cn_timer);
            APIextra::TimerReset(e, t);
            APIextra::TimerRegisterNotify(e, SELF, t, 0x22);
            APIextra::TimerSetFrequency(e, t, 0.2);
        }
        0x11c if timer_lvl => {
            let t = e.r32(LP(e) + l.cn_timer);
            APIextra::TimerReset(e, t);
            APIextra::TimerSetFrequency(e, t, 20.0);
            APIextra::TimerUnregisterNotify(e, SELF, t);
        }
        0x183 if timer_lvl => {
            e.w32(LP(e) + l.local_size - 4, (param_2 != 0) as u32);
        }
        0x12 | 0x4c | 0x4d => {
            if cur(e) != 0x7fff_fffe && cur(e) != 0 {
                APIextra::TimerPause(e, e.r32(LP(e) + l.cn_timer), 1);
                for w in cn_windows(e).into_iter().filter(|_| wins) {
                    APIhud::WindowSetFlags(e, w, 0x100);
                }
            }
        }
        0x13 | 0x51 | 0x52 => {
            if cur(e) != 0x7fff_fffe && cur(e) != 0 {
                APIextra::TimerPause(e, e.r32(LP(e) + l.cn_timer), 0);
                for w in cn_windows(e).into_iter().filter(|_| wins) {
                    APIhud::WindowResetFlags(e, w, 0x100);
                }
            }
        }
        0x1e => {
            if cur(e) != 0x7fff_fffe {
                panelCB_TIMER_COLLNOTIFY(e, 0)?;
            }
        }
        0x33 => {
            if cur(e) != 0x7fff_fffe {
                panelCB_TIMER_COLLNOTIFY(e, 0)?;
            }
            e.w32(LP(e) + l.cn_i0, 0x7fff_fffe);
            e.w32(LP(e) + l.cn_i1, 0x7fff_fffe);
            // GetDisplayInfo writes +4/+8 of its record: cn_cur is followed by cn_i0, cn_i1
            panel_baseFUNC_GetDisplayInfo(e, param_2, LP(e) + l.cn_cur)?;
            if e.r32(LP(e) + l.cn_i0) != 0x7fff_fffe {
                e.w32(LP(e) + l.cn_cur, param_2);
                for w in cn_windows(e) {
                    APIhud::WindowResetFlags(e, w, 0x100);
                }
                APIinput::InputConfigActivate(e, 7, 0);
                panelFUNC_RegisterTimer(e)?;
            }
        }
        _ => {}
    }
    Ok(panel_baseFUNC_MessageHandler(e, param_1, param_2, param_3)? as u64)
}

/// panelFUNC_Message of the 6 boss-fight levels (boss meter builds). No pause handling of the collectible
/// notification (0x12/0x13/...); 0x52 shows the boss meter, most variants first fade its portrait from
/// "dustdevil" to the boss texture once (static bInitBossFightMeter). Variants per build:
/// 4ec0e7c2 no fader; 613811a2/977db83d fader; 8863bd88/fe95add8 fader whose boss depends on an object
/// being present (that also preloads 4 hits); cd8e4e92 fader + 0x11b/0x11c timer and 0x183 flag messages.
fn panelFUNC_Message_Boss(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    let l = loc(e);
    let sha = l.sha8;
    if !["4ec0e7c2", "613811a2", "977db83d", "cd8e4e92", "8863bd88", "fe95add8"].contains(&sha) {
        unported_local(e, "panelFUNC_Message");
    }
    let cur = |e: &Engine| e.r32(LP(e) + l.cn_cur);
    let fader = |e: &mut Engine, obj: u32, to: &str| {
        let flag = G(e, S_bInitBossFightMeter);
        if e.r32(flag) == 0 {
            e.w32(flag, 1);
            let (f, t) = (lit(e, "dustdevil"), lit(e, to));
            let p = super::panel_base::tex_fader_params(e, 0x3a83_126f, f, t);
            e.w32(p, 2);
            APIgraphics::TexFaderPlay(e, obj, p);
        }
    };
    match param_1 {
        0x1e => {
            if cur(e) != 0x7fff_fffe {
                panelCB_TIMER_COLLNOTIFY(e, 0)?;
            }
        }
        0x33 => {
            if cur(e) != 0x7fff_fffe {
                panelCB_TIMER_COLLNOTIFY(e, 0)?;
            }
            e.w32(LP(e) + l.cn_i0, 0x7fff_fffe);
            e.w32(LP(e) + l.cn_i1, 0x7fff_fffe);
            panel_baseFUNC_GetDisplayInfo(e, param_2, LP(e) + l.cn_cur)?;
            if e.r32(LP(e) + l.cn_i0) != 0x7fff_fffe {
                e.w32(LP(e) + l.cn_cur, param_2);
                for w in cn_windows(e) {
                    APIhud::WindowResetFlags(e, w, 0x100);
                }
                APIinput::InputConfigActivate(e, 7, 0);
                panelFUNC_RegisterTimer(e)?;
            }
        }
        0x52 => match sha {
            "4ec0e7c2" => {
                if param_2 == 1 {
                    panel_baseFUNC_MessageHandlerBossMeter(e, 0x39, 1)?;
                }
            }
            "613811a2" | "977db83d" | "cd8e4e92" => {
                let (obj, to) = match sha {
                    "613811a2" => (0xf00_04c5, "redknight"),
                    "977db83d" => (0xf00_0371, "zombieboss"),
                    _ => (0xf00_0406, "fakeghost"),
                };
                fader(e, obj, to);
                if param_2 == 1 {
                    panel_baseFUNC_MessageHandlerBossMeter(e, 0x39, 1)?;
                }
            }
            "8863bd88" | "fe95add8" => {
                if param_2 == 1 {
                    let (who, obj) = if sha == "8863bd88" { (0x2a00_0012, 0xf00_060d) } else { (0x2a00_00c6, 0xf00_04ee) };
                    panel_baseFUNC_MessageHandlerBossMeter(e, 0x39, 1)?;
                    if APIgameobj::GetID(e, who, 0) != 0x7fff_fffe {
                        panel_baseFUNC_MessageHandlerBossMeter(e, 0x3c, 4)?;
                    }
                    if e.r32(G(e, S_bInitBossFightMeter)) == 0 {
                        let to = if APIgameobj::GetID(e, who, 0) == 0x7fff_fffe { "fireghost" } else { "ghost_squad" };
                        fader(e, obj, to);
                    }
                }
            }
            _ => unported_local(e, "panelFUNC_Message"),
        },
        0x11b if sha == "cd8e4e92" => {
            let t = e.r32(LP(e) + l.cn_timer);
            APIextra::TimerReset(e, t);
            APIextra::TimerRegisterNotify(e, SELF, t, 0x22);
            APIextra::TimerSetFrequency(e, t, 0.2);
        }
        0x11c if sha == "cd8e4e92" => {
            let t = e.r32(LP(e) + l.cn_timer);
            APIextra::TimerReset(e, t);
            APIextra::TimerSetFrequency(e, t, 20.0);
            APIextra::TimerUnregisterNotify(e, SELF, t);
        }
        0x183 if sha == "cd8e4e92" => {
            e.w32(LP(e) + 0x5b4, (param_2 != 0) as u32);
        }
        _ => {}
    }
    Ok(panel_baseFUNC_MessageHandler(e, param_1, param_2, param_3)? as u64)
}

/// 0x00030b40 (188 bytes) `void panelFUNC_RegisterTimer(void)`
pub fn panelFUNC_RegisterTimer(e: &mut Engine) -> LResult<()> {
    if super::panel_frontier::kind(e).is_some() {
        return super::panel_frontier::register_timer(e);
    }
    let l = loc(e);
    if !l.std_regtimer {
        unported_local(e, "panelFUNC_RegisterTimer");
    }
    let t = e.r32(LP(e) + l.cn_timer);
    APIextra::TimerReset(e, t);
    APIextra::TimerRegisterNotify(e, SELF, t, 0x22);
    APIextra::TimerSetFrequency(e, t, 2.0);
    APIextra::TimerPause(e, t, 0);
    Ok(())
}

/// 0x00030c00 (416 bytes) `void panelConstructor(APIcons *api,int instanceCount)`
pub fn panelConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    if let Some(k) = super::panel_frontier::kind(e) {
        return super::panel_frontier::constructor(e, k, api, instanceCount);
    }
    let l = loc(e);
    if !l.std_ctor {
        unported_local(e, "panelConstructor");
    }
    panel_baseConstructor(e, api, instanceCount)?;
    if instanceCount == 1 {
        APIcons::SetExternalFunction(e, 0x21, panelST_FIRSTMOVE_LOCAL);
        APIcons::SetExternalFunction(e, 0x22, panelCB_TIMER_COLLNOTIFY);
        let slot = G(e, S_pGlobalData_Panel);
        APIcons::GetGlobalData(e, slot, 0x18);
        for (i, (buf, size)) in [(0, 0x8f8), (1, 0x68), (3, 0x40)].into_iter().enumerate() {
            let v = APIgameobj::GetGameBuffer(e, buf, size, 1);
            let gd = e.r32(G(e, S_pGlobalData_Panel));
            e.w32(gd + i as u32 * 4, v);
        }
    }
    APIcons::SetMessageFunction(e, panelFUNC_Message);
    let slot = G(e, S_pLocalData_Panel);
    APIcons::GetLocalData(e, slot, l.local_size as i32);
    let t = APIextra::TimerCreate(e);
    e.w32(LP(e) + l.cn_timer, t);
    APIcons::SetInitialState(e, 0x21);
    Ok(())
}

/// 0x00030da0 (752 bytes) `void panelDestructor(int instanceCount)`
pub fn panelDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    if let Some(k) = super::panel_frontier::kind(e) {
        return super::panel_frontier::destructor(e, k, instanceCount);
    }
    let l = loc(e);
    if !l.std_dtor {
        unported_local(e, "panelDestructor");
    }
    panel_baseDestructor(e, instanceCount)?;
    for &(o, n) in l.dtor {
        for i in 0..n {
            let w = e.r32(LP(e) + o + i * 4);
            if w != 0x3b9a_c9f7 {
                APIhud::WindowDelete(e, w);
            }
        }
    }
    APIextra::TimerDestroy(e, e.r32(LP(e) + l.cn_timer));
    Ok(())
}

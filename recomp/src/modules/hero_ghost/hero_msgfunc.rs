//! hero_msgfunc.c (1 function). Two shape groups: group 1 lacks the 0x99..0x9d messages and
//! counts a shorter door table; object / sound ids come from the build literal slots.
use super::builds::*;
use super::*;
use crate::engine::*;
use crate::logic::LResult;

const SELF: u32 = 0x2a00_0000;
const NULL: Ptr = 0;
const NOT_FOUND: u32 = 0x7fff_fffe;

/// Scooby (prim 100) / Shaggy (prim 0x6e) object ids of the build.
fn scooby(e: &Engine) -> u32 {
    k(e, heroFUNC_MessageHandler_0_68)
}
fn shaggy(e: &Engine) -> u32 {
    k(e, heroFUNC_MessageHandler_0_69)
}
fn or_w(e: &mut Engine, a: Ptr, v: u32) {
    let x = e.r32(a);
    e.w32(a, x | v);
}
fn and_w(e: &mut Engine, a: Ptr, v: u32) {
    let x = e.r32(a);
    e.w32(a, x & v);
}
fn is_leader(e: &Engine) -> bool {
    e.r32(GB0(e) + 8) == e.r32(L(e) + 0x17c)
}
/// `if (GetID(hero) != NOT_FOUND) PrimSetIntAttribBits(hero, prim, 2, PrimGetIntAttrib(hero, prim, 2) | 4)`
fn hero_prim_bit4(e: &mut Engine, hero: u32, prim: u32, check: bool) {
    if check && APIgameobj::GetID(e, hero, NULL) == NOT_FOUND {
        return;
    }
    let v = APIphysic::PrimGetIntAttrib(e, hero, prim, 2);
    APIphysic::PrimSetIntAttribBits(e, hero, prim, 2, v | 4);
}
/// `heroFUNC_DebugTime("<msg>", &uStack_10c, 1)`: the value is param_2, spilled to the stack.
fn debug_time_p2(e: &mut Engine, msg: &str, param_2: u32) -> LResult<()> {
    let p = e.alloca(4);
    e.w32(p, param_2);
    debug_time(e, msg, p, 1)?;
    Ok(())
}

/// 0x0001fb50 (9764 bytes) `int heroFUNC_MessageHandler(uint param_1,uint param_2,int *param_3)`
pub fn heroFUNC_MessageHandler(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<i32> {
    let l = L(e);
    // group 1 has no 0x99..0x9d (button / lever / item) messages
    let g0 = g(e, G_heroFUNC_MessageHandler) == 0;
    match param_1 {
        0x12 => {
            if is_leader(e) {
                APIinput::InputConfigActivate(e, e.r32(l + 0x188) as i32, 0);
            }
            e.w32(l + 700, 1);
        }
        0x13 => {
            e.w32(l + 700, 0);
            if is_leader(e) {
                APIinput::InputConfigActivate(e, e.r32(l + 0x188) as i32, 1);
            }
        }
        0x4c => {
            debug_time(e, "MSG_GOING_TO_CUT", NULL, 0)?;
            if e.r32(GB0(e) + 0x14) & 0x800_0008 == 0 {
                or_w(e, l + 0x174, 4);
            } else {
                debug_info(e, "--------------------------------------------------------------");
                debug_info(e, "Going To cut??? Already in CutScene....");
                debug_info(e, "--------------------------------------------------------------");
            }
        }
        0x4f => {
            debug_time(e, "MSG_FADED", NULL, 0)?;
            or_w(e, l + 0x170, 8);
            APIextra::TimerReset(e, e.r32(l + 0x20c));
            if is_leader(e) {
                if e.r32(GB0(e) + 0x14) & 0x800_0000 != 0 {
                    let orp = e.alloca(8); // {1, 6}: ObjectRetrieverParams
                    e.w32(orp, 1);
                    e.w32(orp + 4, 6);
                    APIgameobj::SendMessageTo(e, 0xff00_0008, orp, 0xc1, 0, NULL);
                    APIgameobj::SendMessageTo(e, k(e, ID_2a00008f), NULL, 0xc1, 0, NULL);
                    let room = if e.r32(l + 0x78) != 0 {
                        e.r32(l + 0x78)
                    } else if e.r32(l + 0x260) != 0 {
                        e.r32(l + 0x260)
                    } else {
                        k(e, ID_2a00003c)
                    };
                    e.w32(l + 0x234, room);
                }
                heroFUNC_NextMove(e, l)?;
            }
        }
        0x51 => {
            debug_time(e, "MSG_BACK_FROM_CUT", NULL, 0)?;
            or_w(e, l + 0x174, 8);
            APIextra::TimerReset(e, e.r32(l + 0x20c));
            and_w(e, GB0(e) + 0x14, 0xf9df_ffff);
        }
        0x60 | 0x61 | 0x62 | 99 => {
            let (msg, off) = match param_1 {
                0x60 => ("MSG_EVENTACTION_SET", 0x170),
                0x61 => ("MSG_EVENTCB_SET", 0x168),
                0x62 => ("MSG_EVENTCTRL_SET", 0x174),
                _ => ("MSG_EVENTFLAG_SET", 0x164),
            };
            debug_time_p2(e, msg, param_2)?;
            or_w(e, l + off, param_2);
        }
        100 => or_w(e, l + 0x16c, param_2),
        0x66 => {
            debug_info(e, "MSG_RESETPOS");
            if is_leader(e) {
                let v = e.alloca(0x10);
                let p = APIgameobj::GetPosition(e, e.r32(l + 0x234), 0x7fff_ffff, v);
                APIgameobj::SetTranslation(e, SELF, p);
                APIgameobj::TakeOrientation(e, SELF, e.r32(l + 0x234), 0x7fff_ffff);
                debug_info_value(e, "MSG_RESETPOS", v, 4);
                if e.r32(GB0(e) + 0x34) & 0x10 != 0 {
                    APIgameobj::LeaveParent(e, e.r32(l + 0x180), 0);
                    heroFUNC_SetNewPos(e, e.r32(l + 0x180), e.r32(l + 0x234))?;
                }
            }
        }
        0x67 => {
            if is_leader(e) {
                APIgameobj::LeaveParent(e, SELF, 0);
                APIphysic::ResetFlags(e, SELF, 1);
            } else if e.r32(l + 0x17c) == e.r32(GB0(e) + 0xc) {
                APIgameobj::LeaveParent(e, SELF, 0);
                APIphysic::ResetFlags(e, SELF, 1);
                APIphysic::PrimSetIntAttribBits(e, SELF, e.r32(l + 0x18c), 2, 4);
            }
            hero_prim_bit4(e, scooby(e), 100, true);
            hero_prim_bit4(e, shaggy(e), 0x6e, true);
        }
        0x68 => {
            debug_info(e, "MSG_HIT_HERO");
            if e.r32(GB0(e) + 0x74) != 1 {
                if param_3 == NULL && param_2 == 0 {
                    let name = char_name(e);
                    for (s, a) in [
                        ("--------------------------------------------------------------\n", 0),
                        ("%s> aucune structure ObjCollisionNotifyStruct passer\n", name),
                        ("%s> aucune damage passer\n", name),
                        ("HINT!!!!!!\n", 0),
                        ("HINT> verifie ce que tu passe dans le MSG_HIT_HERO...\n", 0),
                    ] {
                        let f = e.cstr(s);
                        APIextra::null_printf(e, f, a);
                    }
                } else if param_3 == NULL {
                    // a stack ObjCollisionNotifyStruct: +8 = 0, +0x10 = damage, +0x14 = 0x41
                    let s = e.alloca(0x18);
                    e.w32(s + 0x10, param_2);
                    e.w32(s + 0x14, 0x41);
                    e.w32(s + 8, 0);
                    heroCB_SCARED(e, s)?;
                } else {
                    heroCB_SCARED(e, param_3)?;
                }
            }
        }
        0x6b => {
            debug_time(e, "MSG_TURN_HOOK_MODE", NULL, 0)?;
            or_w(e, l + 0x170, 2);
        }
        0x6c => {
            let p = e.alloca(0x10);
            APIphysic::DMGetParams(e, e.r32(l + 0x1b0), p);
            e.w32(p, if param_2 == 0 { 50.0f32 } else { 0.0f32 }.to_bits());
            APIphysic::DMSetParams(e, e.r32(l + 0x1b0), p);
        }
        0x6e => return Ok(e.r32(l + 0x164) as i32),
        0x6f => {
            if param_2 == 1 {
                or_w(e, GB0(e) + 0x14, 0x100_0000);
            } else {
                and_w(e, GB0(e) + 0x14, 0xfeff_ffff);
            }
        }
        0x72 => {
            if param_3 != NULL {
                debug_info(e, "MSG_UPDATE_WP - done");
                for i in 0..3 {
                    let v = e.r32(param_3 + i * 4);
                    e.w32(l + 0x2e4 + i * 4, v);
                }
                let p = e.alloca(0x18);
                APIphysic::DMGetParams(e, e.r32(l + 0x1d8), p);
                e.w32(p, 0);
                for i in 0..3 {
                    let v = e.r32(l + 0x2e4 + i * 4);
                    e.w32(p + 8 + i * 4, v);
                }
                e.w32(p + 0x14, 5);
                e.w32(p + 4, 0x4020_0000);
                APIphysic::DMSetParams(e, e.r32(l + 0x1d8), p);
            }
        }
        0x78 => {
            e.w32(l + 0x250, param_2);
            let v = e.r32(param_3);
            e.w32(l + 0x24c, v);
        }
        0x7b => return msg_teammode_set(e, l, param_1, param_2, param_3),
        0x7c => {
            debug_time(e, "MSG_LEADER_SET", NULL, 0)?;
            or_w(e, l + 0x174, 1);
            and_w(e, l + 0x174, 0xffff_fffd);
        }
        0x7d => {
            debug_time(e, "MSG_FOLLOW_SET", NULL, 0)?;
            or_w(e, l + 0x174, 2);
            and_w(e, l + 0x174, 0xffff_fffe);
        }
        0x7f => {
            debug_info(e, "MSG_LEADER_SWITCH");
            heroFUNC_SetLeader(e)?;
        }
        0x86 | 0x88 => {
            // 0x86 falls through into 0x88
            if param_1 == 0x86 && param_2 == 1 {
                if e.r32(l + 0x17c) == scooby(e) {
                    APInewSound::Play(e, SELF, k(e, ID_80000043), 0);
                }
                if e.r32(l + 0x17c) == shaggy(e) {
                    APInewSound::Play(e, SELF, k(e, ID_80000044), 0);
                }
            }
            e.w32(l + 0x2c0, 1);
        }
        0x89 => e.w32(l + 0x2c4, (param_2 != 0) as u32),
        0x8a => {
            if param_2 == 0x200_0000 {
                let st = e.r32(l + 0x158);
                if st != addr::heroST_Scare_Loop && st != addr::heroST_PanicIn_Loop {
                    or_w(e, l + 0x168, 1);
                    heroST_Scare_Loop(e, l)?;
                    debug_info(e, "MSG_SLAVE_SYNC_SCAREMODE -> CHS_SCARED");
                }
            } else if param_2 == 0x400_0000 {
                let t = e.alloca(4);
                let f = e.r32(param_3) as i32 as f32;
                e.w32(t, f.to_bits());
                APIphysic::ResetFlags(e, SELF, 4);
                APIgameobj::SetContextFlag(e, SELF, 0);
                APIphysic::SetVelocity(e, SELF, 0.0, 0.0, 0.0);
                e.w32(l + 0x154, 0);
                e.w32(l + 0x158, addr::heroST_Panic_Loop);
                e.w32(l + 0x15c, addr::heroExit_Panic);
                let a = heroFUNC_StartAnim(e, 0.0, 0xfffff87a_u32 as i32, 2, 0)?;
                e.w32(l + 0x218, a as i32 as u32);
                APIanim::SetTime(e, SELF, 0xfffff87a, f);
                debug_info_value(e, "MSG_SLAVE_SYNC_SCAREMODE -> CHS_PANIC -> fixe at", t, 2);
            }
        }
        0x8b => {
            let prim = e.r32(l + 0x18c);
            return Ok(APIphysic::PrimGetIntAttrib(e, SELF, prim, 5) as i32);
        }
        0x8c => {
            APIphysic::DMSetActive(e, e.r32(l + 0x1dc), 1);
            APIphysic::DMSetStateBits(e, e.r32(l + 0x1dc), 1);
        }
        0x8d => {
            let orp = e.alloca(8); // {1, own id}
            let id = APIgameobj::GetID(e, SELF, NULL);
            e.w32(orp + 4, id);
            e.w32(orp, 1);
            let r = APIgameobj::GetID(e, 0xff00_0020, orp);
            return Ok(if r != NOT_FOUND { r as i32 } else { 0 });
        }
        0x8e => {
            heroFUNC_MessageHandler(e, 0xd2, 0x268, NULL)?;
        }
        0x8f => {
            heroFUNC_MessageHandler(e, 0xd2, 0x494, NULL)?;
        }
        0x92 => {
            debug_info(e, "MSG_TOME_OUT");
            if e.r32(GB0(e) + 0x14) & 1 != 0 {
                or_w(e, l + 0x170, 0x2000);
                APIgameobj::SendMessageTo(e, e.r32(l + 0x180), NULL, 0x60, 0x2000, NULL);
            }
            let p = e.alloca(0x10);
            APIphysic::DMGetParams(e, e.r32(l + 0x1b0), p);
            e.w32(p, 50.0f32.to_bits());
            APIphysic::DMSetParams(e, e.r32(l + 0x1b0), p);
            hero_prim_bit4(e, scooby(e), 100, true);
            hero_prim_bit4(e, shaggy(e), 0x6e, true);
        }
        0x93 => {
            let v = if param_2 == 0 { 0x4000 } else { 0x2000 };
            APIgameobj::SendMessageTo(e, e.r32(l + 0x180), NULL, 0x60, v, NULL);
            if e.r32(l + 0x180) == scooby(e) {
                hero_prim_bit4(e, scooby(e), 100, false);
            } else {
                hero_prim_bit4(e, shaggy(e), 0x6e, false);
            }
        }
        0x97 => msg_enter_door(e, l, param_2)?,
        0x99 if g0 => {
            debug_time(e, "MSG_PUSH_BUTTON", NULL, 0)?;
            e.w32(l + 0x23c, param_3);
            e.w32(l + 0x240, 0);
            or_w(e, l + 0x170, 0x1000);
            or_w(e, GB0(e) + 0x14, 0x4000_0000);
        }
        0x9a if g0 => {
            debug_time(e, "MSG_PULL_LEVER", NULL, 0)?;
            or_w(e, GB0(e) + 0x14, 0x100);
            e.w32(l + 0x23c, param_3);
            e.w32(l + 0x240, if param_2 == 1 { 2 } else { 3 });
            or_w(e, l + 0x170, 0x1000);
            or_w(e, GB0(e) + 0x14, 0x4000_0000);
        }
        0x9b if g0 => {
            debug_time(e, "MSG_PLACE_ITEM", NULL, 0)?;
            let v = e.r32(param_3);
            e.w32(l + 0x23c, v);
            e.w32(l + 0x240, 5);
            or_w(e, l + 0x170, 0x1000);
            or_w(e, GB0(e) + 0x14, 0x4000_0000);
        }
        0x9c if g0 => {
            let leader = e.r32(GB0(e) + 8);
            if leader <= k(e, heroFUNC_MessageHandler_0_27) && leader >= k(e, heroFUNC_MessageHandler_0_28) {
                debug_time(e, "MSG_USE_ITEM", NULL, 0)?;
                or_w(e, GB0(e) + 0x14, 0x4000_0000);
                heroFUNC_UseItem(e, l, param_2 as i32, param_3)?;
            }
        }
        0x9d if g0 => {
            debug_time(e, "MSG_PUSH_HEAVYOBJ", NULL, 0)?;
            let (a, d) = (e.r32(param_3), e.r32(param_3 + 0xc));
            e.w32(l + 8, a);
            e.w32(l + 0x14, d);
            e.w32(l + 0x23c, a);
            e.w32(l + 0x240, 1);
            or_w(e, l + 0x170, 0x1000);
            or_w(e, GB0(e) + 0x14, 0x4000_0000);
            e.w32(l + 0xc, 2);
            e.w32(l + 0x10, 3);
        }
        0xa3 => {
            debug_info(e, "MSG_CHECKPOINT");
            let v = e.r32(param_3);
            e.w32(l + 0x78, v);
            let v = e.r32(l + 0x224);
            e.w32(l + 0x7c, v);
        }
        0xc4 => {
            let v = e.r32(l + 0x234).wrapping_add(1);
            e.w32(l + 0x234, v);
            heroFUNC_NextMove(e, l)?;
        }
        0xd2 => msg_d2(e, l, param_2)?,
        0xd7 => {
            let prim = e.r32(l + 0x18c);
            APIphysic::PrimSetIntAttribBits(e, SELF, prim, 0, 0x80);
        }
        0x165 => {
            heroFUNC_MessageHandler(e, 0xd2, 0x60, NULL)?;
        }
        0x166 => {
            heroFUNC_MessageHandler(e, 0xd2, 0x90, NULL)?;
        }
        0x1d3 => or_w(e, l + 0x170, 0x100_0000),
        0x1d4 => or_w(e, l + 0x170, 0x200_0000),
        0x213 => return Ok((e.r32(l + 0x16c) & 0x80 != 0) as i32),
        _ => {}
    }
    Ok(0)
}

/// case 0x7b MSG_TEAMMODE_SET: redirect to the right hero, then the team mode bits of GB0+0x34,
/// message 0xd2 (flags) to the other hero and 0x47 (who) to the camera.
fn msg_teammode_set(e: &mut Engine, l: Ptr, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<i32> {
    let mut who: u32 = 0;
    let mut flags: u32 = 0;
    debug_info(e, "MSG_TEAMMODE_SET");
    if param_2 & 2 != 0 && e.r32(l + 0x17c) != scooby(e) {
        debug_info(e, "MSG_TEAMMODE_SET -> Redirection a Scoobydoo");
        APIgameobj::SendMessageTo(e, scooby(e), NULL, param_1, param_2, param_3);
        return Ok(0);
    }
    if param_2 & 4 != 0 && e.r32(l + 0x17c) != shaggy(e) {
        debug_info(e, "MSG_TEAMMODE_SET -> Redirection a Shaggy");
        APIgameobj::SendMessageTo(e, shaggy(e), NULL, param_1, param_2, param_3);
        return Ok(0);
    }
    if param_3 != NULL {
        who = e.r32(param_3);
    }
    if param_2 & 4 != 0 {
        debug_info(e, "_MSG_TEAMMODE_SET --> _ID_SHAGGY");
        if who == 0 {
            who = 1;
        }
    } else if param_2 & 2 != 0 {
        debug_info(e, "_MSG_TEAMMODE_SET --> _ID_SCOOBYDOO");
        if who == 0 {
            who = 2;
        }
    } else if param_2 & 8 != 0 {
        debug_info(e, "_MSG_TEAMMODE_SET --> _ID_LEADER");
        if who == 0 {
            who = e.r32(l + 0x178);
        }
    }
    let m34 = GB0(e) + 0x34;
    if param_2 & 0x10 != 0 {
        debug_info(e, "_MSG_TEAMMODE_SET --> _ID_BOTH");
        and_w(e, m34, 0xffff_f8fe);
        or_w(e, m34, 0x10);
        flags |= 0x66;
        who = e.r32(l + 0x178);
    } else if param_2 & 0x20 != 0 {
        debug_info(e, "_MSG_TEAMMODE_SET --> _TEAMMODE_ALONE");
        and_w(e, m34, 0xffff_f8ef);
        or_w(e, m34, 1);
        flags |= 4;
        APIgameobj::SendMessageTo(e, e.r32(l + 0x180), NULL, 0x6b, 0, NULL);
    } else {
        debug_info(e, "_MSG_TEAMMODE_SET --> _TEAMMODE_TOGETHER");
        and_w(e, m34, 0xffff_fdff);
    }
    flags |= if param_2 & 0x40 == 0 { 0x42 } else { 0x81 };
    flags |= if param_2 & 0x80 == 0 { 0x20 } else { 0x10 };
    APIgameobj::SendMessageTo(e, e.r32(l + 0x180), NULL, 0xd2, flags, NULL);
    APIgameobj::SendMessageTo(e, k(e, ID_2a00008f), NULL, 0x47, who, NULL);
    Ok(0)
}

/// case 0x97 MSG_ENTER_DOOR: the room to go to (local+0x234) = GB0+0x48 if it is in the build's
/// door table (goNextRoomInfo, 0x10 bytes per entry), else param_2.
fn msg_enter_door(e: &mut Engine, l: Ptr, param_2: u32) -> LResult<()> {
    debug_time_p2(e, "MSG_ENTER_DOOR", param_2)?;
    APIphysic::SetVelocity(e, SELF, 0.0, 0.0, 0.0);
    if e.r32(GB0(e) + 0x48) == 0 {
        e.w32(l + 0x234, param_2);
    } else {
        let n = if g(e, G_heroFUNC_MessageHandler) == 0 {
            k(e, heroFUNC_MessageHandler_0_42)
        } else {
            k(e, heroFUNC_MessageHandler_1_15)
        };
        let mut found = false;
        let tab = G(e, goNextRoomInfo);
        for i in 0..n {
            let want = e.r32(GB0(e) + 0x48);
            if e.r32(tab + i * 0x10) == want {
                e.w32(l + 0x234, want);
                found = true;
                break;
            }
        }
        if !found {
            e.w32(l + 0x234, param_2);
        }
    }
    or_w(e, l + 0x170, 4);
    Ok(())
}

/// case 0xd2: enable / visibility / input / physics / next-state flags in param_2.
fn msg_d2(e: &mut Engine, l: Ptr, param_2: u32) -> LResult<()> {
    if param_2 & 0x40 != 0 {
        APIgameobj::EnableObject(e, SELF, 1);
        for o in [0x174, 0x170, 0x168, 0x164] {
            e.w32(l + o, 0);
        }
    } else if param_2 & 0x80 != 0 {
        APIgameobj::EnableObject(e, SELF, 0);
    }
    if param_2 & 1 != 0 {
        APIgameobj::SetGameObjectVisible(e, e.r32(l + 0x17c), 0);
        APIgameobj::ResetLogicFlag(e, SELF, 0x800);
    } else if param_2 & 2 != 0 {
        APIgameobj::SetGameObjectVisible(e, e.r32(l + 0x17c), 1);
        APIgameobj::SetLogicFlag(e, SELF, 0x800);
        e.w32(l + 0x170, 0);
    }
    if param_2 & 4 != 0 {
        APIinput::InputConfigActivate(e, e.r32(l + 0x188) as i32, 0);
    } else if param_2 & 8 != 0 && is_leader(e) {
        APIinput::InputConfigActivate(e, e.r32(l + 0x188) as i32, 1);
    }
    if param_2 & 0x10 != 0 {
        APIphysic::SetFlags(e, e.r32(l + 0x17c), 1);
    } else if param_2 & 0x20 != 0 {
        APIphysic::ResetFlags(e, e.r32(l + 0x17c), 1);
    }
    if param_2 & 0x100 != 0 {
        e.w32(l + 0x154, addr::heroInit_Busy);
    } else if param_2 & 0x200 != 0 {
        let st = if is_leader(e) { addr::heroInit_Stand } else { addr::heroInit_FollowStand };
        e.w32(l + 0x154, st);
    }
    if param_2 & 0x400 != 0 {
        let a = heroFUNC_StartAnim(e, 0.15, 2, 2, 0)?;
        e.w32(l + 0x218, a as i32 as u32);
    }
    Ok(())
}

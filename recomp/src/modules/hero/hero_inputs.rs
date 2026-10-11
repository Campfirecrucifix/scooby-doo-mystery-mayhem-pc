//! hero_inputs.c (7 functions). Addresses are module-relative (.text at 0x10000) in scooby__402420b4.
use super::builds::*;
use super::*;
use crate::engine::*;
use crate::logic::LResult;

/// 0x000228f0 (80 bytes) `undefined8 heroCB_INPUT_Idle(void)`
pub fn heroCB_INPUT_Idle(e: &mut Engine, _: u32) -> LResult<u64> {
    let l = L(e);
    or_w(e, l + 0x16c, 1);
    Ok(0)
}

/// 0x00022940 (168 bytes) `undefined8 heroCB_INPUT_Stick(undefined4 *param_1)`
pub fn heroCB_INPUT_Stick(e: &mut Engine, param_1: u32) -> LResult<u64> {
    let l = L(e);
    or_w(e, l + 0x16c, 2);
    let x = e.rf(param_1);
    let v = APImath::myfabs(e, x);
    e.wf(l + 0x2c8, v);
    let y = e.rf(param_1 + 4);
    let v = APImath::myfabs(e, y);
    e.wf(l + 0x2cc, v);
    Ok(0)
}

/// 0x000229e8 (304 bytes) `undefined8 heroCB_INPUT_Switch_Leader(void)`
pub fn heroCB_INPUT_Switch_Leader(e: &mut Engine, _: u32) -> LResult<u64> {
    let l = L(e);
    let t = APIextra::TimerGetTime(e, e.r32(l + 0x20c));
    if 0.75 <= t
        && APIphysic::GetPhysicState(e, 0x2a00_0000) & 1 != 0
        && APIphysic::GetPhysicState(e, e.r32(l + 0x180)) & 1 != 0
        && e.r32(GB0(e) + 0x14) & 0x4000_0004 == 0
    {
        or_w(e, GB0(e) + 0x14, 0x4000_0000);
        or_w(e, l + 0x16c, 0x1000_0000);
    }
    Ok(0)
}

/// 0x00022b18 (132 bytes) `undefined8 heroCB_INPUT_Sneak(void)`
pub fn heroCB_INPUT_Sneak(e: &mut Engine, _: u32) -> LResult<u64> {
    let l = L(e);
    or_w(e, l + 0x16c, 4);
    APIgameobj::SendMessageTo(e, e.r32(l + 0x180), 0, 100, 4, 0);
    Ok(0)
}

/// 0x00022ba0 (416 bytes) `undefined8 heroCB_INPUT_Tome(void)`
///
/// The "open the book" button: after 0.75 s on the hero's input timer (local+0x20c), when this hero
/// is one of the two heroes, the book object exists and the game does not block the tome (game buffer
/// 0 +0x14 bits 0x40000004), marks the input consumed (0x40000000) and requests the tome (local+0x16c
/// bit 8). Group 1 = release builds (no debug strings).
pub fn heroCB_INPUT_Tome(e: &mut Engine, _: u32) -> LResult<u64> {
    let l = L(e);
    let t = APIextra::TimerGetTime(e, e.r32(l + 0x20c));
    if 0.75 <= t {
        let me = e.r32(l + 0x17c);
        if (me == k(e, super::builds::heroCB_INPUT_Tome_0_1) || me == k(e, super::builds::heroCB_INPUT_Tome_0_2))
            && APIgameobj::GetID(e, k(e, super::builds::heroCB_INPUT_Tome_0_3), 0) != 0x7fff_fffe
            && e.r32(GB0(e) + 0x14) & 0x4000_0004 == 0
        {
            debug_time(e, "CB_INPUT_Tome", 0, 0)?;
            let v = e.r32(GB0(e) + 0x14) | 0x4000_0000;
            e.w32(GB0(e) + 0x14, v);
            let v = e.r32(l + 0x16c) | 8;
            e.w32(l + 0x16c, v);
        } else {
            debug_time(e, "CB_INPUT_Tome wrong uiMyID", l + 0x17c, 1)?;
        }
    }
    Ok(0)
}

/// 0x00022d40 (248 bytes) `undefined8 heroCB_INPUT_DetectEdge(void)`
///
/// Group 1: empty. Groups 2/3 additionally handle the "talk to partner" floor zone (floor type 4):
/// shows a hint text when nothing is displayed, else (dialogue running, GB2 bits 0x400|0x20000 and
/// local+0x214 == 0x11) triggers the partner action; the per-group hero id / distance / text id differ.
pub fn heroCB_INPUT_DetectEdge(e: &mut Engine, _: u32) -> LResult<u64> {
    let grp = g(e, G_heroCB_INPUT_DetectEdge);
    if grp == 1 {
        return Ok(0);
    }
    let l = L(e);
    let t = APIextra::TimerGetTime(e, e.r32(l + 0x20c));
    if grp == 0 {
        if 0.75 <= t && e.r32(GB0(e) + 0x14) & 0x4000_0000 == 0 && e.r32(GB0(e) + 0x14) & 0x5000 != 0 {
            or_w(e, GB0(e) + 0x14, 0x4000_0000);
            or_w(e, l + 0x16c, 0x10);
        }
        return Ok(0);
    }
    if 0.75 > t {
        return Ok(0);
    }
    if e.r32(GB0(e) + 0x14) & 0x4000_0000 == 0 && e.r32(GB0(e) + 0x14) & 0x5000 != 0 {
        or_w(e, GB0(e) + 0x14, 0x4000_0000);
        or_w(e, l + 0x16c, 0x10);
    }
    let t = APIextra::TimerGetTime(e, e.r32(l + 0x20c));
    let l = L(e);
    let (id_k, id2_k, txt_k, dist) = if grp == 2 {
        (heroCB_INPUT_DetectEdge_2_1, heroCB_INPUT_DetectEdge_2_2, heroCB_INPUT_DetectEdge_2_3, 0.3)
    } else {
        (heroCB_INPUT_DetectEdge_3_1, heroCB_INPUT_DetectEdge_3_2, heroCB_INPUT_DetectEdge_3_3, 0.35)
    };
    let leader = e.r32(GB0(e) + 8);
    if 0.75 <= t && IsCharacterOnThisFloorType(e, -0.75, 0.2, leader, 0, 4)? != 0 {
        let gb2 = e.r32(GB2(e));
        if gb2 & 0x400 != 0 || LIBApiText::IsTextDisplayed(e) != 0 {
            let gb2 = e.r32(GB2(e));
            if gb2 & 0x400 != 0 && gb2 & 0x20000 != 0 && e.r32(l + 0x214) == 0x11 {
                let ok = e.r32(GB0(e) + 8) != k(e, id_k) || {
                    let p = e.r32(L(e) + 0x180);
                    LIBApiCFunctions::CheckRangeObject(e, 2.75, 0x2a00_0000, p, 1) != 0 && {
                        let p = e.r32(L(e) + 0x180);
                        heroFUNC_IsAnythingBetweenUs(e, dist, 0x2a00_0000, p as i32, 0, 0)? == 0
                    }
                };
                if ok {
                    if e.r32(GB0(e) + 8) == k(e, id2_k) {
                        APIgameobj::SendMessageTo(e, e.r32(l + 0x180), 0, 100, 0x80, 0);
                    } else {
                        or_w(e, l + 0x16c, 0x80);
                    }
                    APIgameobj::SendMessageTo(e, 0x2a10_0006, 0, 0x32, e.r32(l + 0x214), 0);
                    e.w32(l + 0x214, 0);
                }
            }
        } else {
            let s1 = e.alloca(0x40);
            LIBApiText::InitStruct(e, s1);
            e.w32(s1, 0x4000_0000);
            LIBApiText::ShowText(e, k(e, txt_k), s1);
        }
    }
    Ok(0)
}

/// 0x00022e38 (48 bytes) `undefined8 heroCB_INPUT_PowerUp_Action(void)`
pub fn heroCB_INPUT_PowerUp_Action(_e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

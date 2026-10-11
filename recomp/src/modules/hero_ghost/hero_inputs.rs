//! hero_inputs.c (7 functions). Addresses are module-relative (.text at 0x10000) in scooby_ghost__08a011d0.
use super::builds::*;
use super::*;
use crate::engine::*;
use crate::logic::LResult;

/// 0x0001c6a0 (80 bytes) `undefined8 heroCB_INPUT_Idle(void)`
pub fn heroCB_INPUT_Idle(e: &mut Engine, _: u32) -> LResult<u64> {
    let l = L(e);
    or_w(e, l + 0x16c, 1);
    Ok(0)
}

/// 0x0001c6f0 (168 bytes) `undefined8 heroCB_INPUT_Stick(undefined4 *param_1)`
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

/// 0x0001c798 (304 bytes) `undefined8 heroCB_INPUT_Switch_Leader(void)`
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

/// 0x0001c8c8 (48 bytes) `undefined8 heroCB_INPUT_Sneak(void)`
pub fn heroCB_INPUT_Sneak(_e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

/// 0x0001c8f8 (416 bytes) `undefined8 heroCB_INPUT_Tome(void)`
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
        if (me == k(e, heroCB_INPUT_Tome_0_1) || me == k(e, heroCB_INPUT_Tome_0_2))
            && APIgameobj::GetID(e, k(e, ID_2a000093), 0) != 0x7fff_fffe
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

/// 0x0001ca98 (48 bytes) `undefined8 heroCB_INPUT_DetectEdge(void)`
pub fn heroCB_INPUT_DetectEdge(_e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

/// 0x0001cac8 (48 bytes) `undefined8 heroCB_INPUT_PowerUp_Action(void)`
pub fn heroCB_INPUT_PowerUp_Action(_e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

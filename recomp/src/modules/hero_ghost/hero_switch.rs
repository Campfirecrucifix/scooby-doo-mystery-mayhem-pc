//! hero_switch.c (4 functions). Addresses are module-relative (.text at 0x10000) in scooby_ghost__08a011d0.
use super::builds::*;
use super::*;
use crate::engine::*;
use crate::logic::LResult;

/// 0x00027d60 (508 bytes) `void heroInit_Switch(int param_1)`
pub fn heroInit_Switch(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    let a = heroFUNC_StartAnim(e, 0.1, 0x1 as i32, 2, 0)?;
    e.w32(param_1 + 0x218, a as i32 as u32);
    if e.r32(GB0(e) + 8) == e.r32(L(e) + 0x17c) {
        let g = GB0(e) + 0x14;
        let v = e.r32(g);
        e.w32(g, v | 0x110);
    }
    APIinput::InputConfigSetState(e, e.r32(param_1 + 0x188) as i32, i32::MIN);
    APIphysic::PrimResetIntAttribBits(e, SELF, e.r32(param_1 + 0x18c), 2, 4);
    if LIBApiCFunctions::CheckRangeObject(e, 1.75, SELF, e.r32(param_1 + 0x180), 1) != 0 {
        let partner = e.r32(L(e) + 0x180);
        LIBApiCFunctions::TrackID(e, 0.0, SELF as i32, partner);
    }
    let p = e.alloca(0x18); // uStack_50..uStack_3c
    APIphysic::DMGetParams(e, e.r32(param_1 + 0x1d8), p);
    e.w32(p, e.r32(param_1 + 0x180));
    e.w32(p + 8, 0);
    e.w32(p + 0xc, 0);
    e.w32(p + 0x10, 0);
    e.w32(p + 4, 0x4148_0000); // 12.5
    e.w32(p + 0x14, 5);
    APIphysic::DMSetParams(e, e.r32(param_1 + 0x1d8), p);
    APIgameobj::SetContextFlag(e, SELF, 0x2001_1050);
    e.w32(param_1 + 0x158, addr::heroST_Switch_Loop);
    e.w32(param_1 + 0x15c, addr::heroExit_Switch);
    Ok(())
}

/// 0x00027f60 (328 bytes) `void heroST_Switch_Loop(int param_1)`
pub fn heroST_Switch_Loop(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    if e.r32(param_1 + 0x168) & 1 == 0 {
        if e.r32(param_1 + 0x164) & 1 == 0
            && LIBApiCFunctions::CheckRangeObject(e, 1.5, SELF, e.r32(param_1 + 0x180), 1) != 0
        {
            let v = e.r32(param_1 + 0x164);
            e.w32(param_1 + 0x164, v | 1);
            let a = heroFUNC_StartAnim(e, 0.1, 0xfffff840u32 as i32, 1, 5)?;
            e.w32(param_1 + 0x218, a as i32 as u32);
            APIgameobj::SetContextFlag(e, SELF, 0x80_0050);
        }
    } else if e.r32(GB0(e) + 8) == e.r32(L(e) + 0x17c) {
        e.w32(param_1 + 0x154, addr::heroInit_Stand);
    } else {
        e.w32(param_1 + 0x154, addr::heroInit_FollowStand);
    }
    Ok(())
}

/// 0x000280a8 (128 bytes) `void heroExit_Switch(int param_1)`
pub fn heroExit_Switch(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    APIphysic::PrimSetIntAttribBits(e, SELF, e.r32(param_1 + 0x18c), 2, 4);
    let g = GB0(e) + 0x14;
    let v = e.r32(g);
    e.w32(g, v & 0xffff_feff);
    Ok(())
}

/// 0x00028128 (220 bytes) `void heroFUNC_SetLeader(void)`
pub fn heroFUNC_SetLeader(e: &mut Engine) -> LResult<()> {
    let l = L(e);
    APIphysic::SetPriority(e, e.r32(l + 0x17c), 4);
    APIphysic::SetPriority(e, e.r32(l + 0x180), 4);
    let me = e.r32(l + 0x178);
    APIgameobj::SendMessageTo(e, k(e, ID_2a00008f), 0, 0x47, me, 0);
    APIgameobj::SendMessageTo(e, k(e, ID_2a100006), 0, 0x47, me, 0);
    Ok(())
}

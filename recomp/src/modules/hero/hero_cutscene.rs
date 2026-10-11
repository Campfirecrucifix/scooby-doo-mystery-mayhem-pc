//! hero_cutscene.c (4 functions). Addresses are module-relative (.text at 0x10000) in scooby__402420b4.
//! Identical in every build apart from the heroFUNC_DebugTime label (debug builds only, see `debug_time`).
//! The hero's EFClippingParams are saved at local+0x114 for the cutscene and restored at its end.
use super::skeleton::addr;
use super::*;
use crate::engine::*;
use crate::logic::LResult;

const SELF: u32 = 0x2a00_0000;

fn leader(e: &Engine) -> bool {
    e.r32(GB0(e) + 8) == e.r32(L(e) + 0x17c)
}

/// 0x00017668 (424 bytes) `void heroInit_CutScene(int param_1)`
pub fn heroInit_CutScene(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    super::hero_misc::debug_time(e, "heroInit_CutScene", 0, 0)?;
    APIgameobj::LeaveParent(e, SELF, 0);
    let g = GB0(e) + 0x14;
    let v = (e.r32(g) & 0xf9df_ffff) | 8;
    e.w32(g, v);
    APIgameobj::GetClippingParams(e, SELF, param_1 + 0x114);
    LIBMiscFunc::MiscFunc_InitForCutscene(e, SELF);
    APIphysic::SetFlags(e, SELF, 1);
    APIinput::InputConfigSetState(e, e.r32(param_1 + 0x188) as i32, i32::MIN);
    // the looping walk sounds 0x29 / 0x2a are cut
    let s = e.r32(param_1 + 0x21c);
    if s == 0x2a || s == 0x29 {
        APInewSound::StopSound(e, SELF, s);
        e.w32(param_1 + 0x21c, 0xffff_ffff);
    }
    e.w32(param_1 + 0x158, addr::heroST_CutScene_Loop);
    Ok(())
}

/// 0x00017810 (36 bytes) `void heroST_CutScene_Loop(void)`: empty.
pub fn heroST_CutScene_Loop(_e: &mut Engine) -> LResult<()> {
    Ok(())
}

/// 0x00017838 (148 bytes) `void heroST_CutScene_Out(int param_1)`
pub fn heroST_CutScene_Out(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    APIphysic::ResetFlags(e, SELF, 1);
    let n = if leader(e) { addr::heroInit_Stand } else { addr::heroInit_FollowStand };
    e.w32(param_1 + 0x154, n);
    Ok(())
}

/// 0x000178d0 (416 bytes) `void heroInit_CutSceneOut(int param_1)`. Calls the next Init directly
/// (local+0x2c4 != 0: busy).
pub fn heroInit_CutSceneOut(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    super::hero_misc::debug_time(e, "heroInit_CutSceneOut", 0, 0)?;
    APIphysic::ResetFlags(e, SELF, 1);
    LIBMiscFunc::MiscFunc_EndCutscene(e, SELF, param_1 + 0x114);
    let busy = e.r32(param_1 + 0x2c4) != 0;
    if leader(e) {
        let g = GB0(e) + 0x14;
        let v = e.r32(g) & 0xffff_fff7 & 0xffff_fffb;
        e.w32(g, v);
        if busy { heroInit_Busy(e, param_1) } else { heroInit_Stand(e, param_1) }
    } else if busy {
        heroInit_Busy(e, param_1)
    } else {
        heroInit_FollowStand(e, param_1)
    }
}

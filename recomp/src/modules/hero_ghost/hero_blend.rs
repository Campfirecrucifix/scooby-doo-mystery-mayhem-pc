//! hero_blend.c (3 functions). Addresses are module-relative (.text at 0x10000) in scooby_ghost__08a011d0.
//!
//! EABI note: the float argument travels in $f12, independently of the int arguments in $a0..
use super::builds::*;
use super::*;
use crate::engine::*;
use crate::logic::LResult;

/// 0x00014070 (1036 bytes) `uint heroFUNC_UpdateBlend_Others(int param_1,int param_2)`
/// Walk/run blend: below a third of the max speed only the anim speed scales (blend weight 0),
/// above it the weight rises to 1 at 95 % (speed `w * 0.0 + 1.0`). Return value unused by callers.
pub fn heroFUNC_UpdateBlend_Others(e: &mut Engine, param_1: Ptr, param_2: i32) -> LResult<u32> {
    if e.r32(param_1 + 0x2b8) != 0 {
        // disasm order: a = GetTime(anim 1), b = GetTime(anim 2); Ghidra's in_f0 shows them swapped
        let a = APIanim::GetTime(e, SELF, 1);
        let b = APIanim::GetTime(e, SELF, k(e, heroFUNC_UpdateBlend_Others_0_1));
        if a != b {
            // float varargs are not forwarded (null_printf discards its output)
            let f = e.cstr("0.anim1 %6.4f   ->  anim2 %6.4f  =  %6.4f\n");
            APIextra::null_printf(e, f, 0);
            APIanim::SetTime(e, SELF, 1, b);
            let v = (e.r32(param_1 + 0x2b8) == 0) as u32;
            e.w32(param_1 + 0x2b8, v);
        }
    }
    if param_2 == 1 {
        let third = e.rf(param_1 + 0x70) * 0.333;
        let mut x = e.rf(param_1 + 0x2e0);
        if x < e.rf(param_1 + 0x70) * 0.6 && e.r32(param_1 + 0x158) == addr::heroST_Move_Loop {
            UpdateTimerOutOfTime(e, 2.0)?;
        }
        let (slot, speed) = if x < third {
            e.w32(param_1 + 0x2ac, 0);
            if x < third * 0.5 {
                x = third * 0.5;
            }
            (heroFUNC_UpdateBlend_Others_0_2, x / third)
        } else {
            let w = (x - third) / (e.rf(param_1 + 0x70) * 0.95 - third);
            e.wf(param_1 + 0x2ac, w);
            if 1.0 < e.rf(param_1 + 0x2ac) {
                e.w32(param_1 + 0x2ac, 0x3f80_0000);
            } else if e.rf(param_1 + 0x2ac) < 0.0 {
                e.w32(param_1 + 0x2ac, 0);
            }
            (heroFUNC_UpdateBlend_Others_0_3, e.rf(param_1 + 0x2ac) * 0.0 + 1.0)
        };
        APIanim::SetSpeed(e, SELF, k(e, slot), speed);
        APIanim::SetSpeed(e, SELF, 1, speed);
        return Ok(APIanim::ChangeBlendParams(e, SELF, param_1 + 0x2a4));
    }
    if param_2 != 0 && param_2 != 3 {
        let v = e.alloca(4);
        e.w32(v, param_2 as u32);
        debug_info_value(e, "FUNC_UpdateBlend_Others -> invalid eBlendAction", v, 1);
        return Ok(0);
    }
    Ok(param_2 as u32)
}

/// 0x00014480 (152 bytes) `long heroFUNC_StartAnim(float param_1,int param_2,int param_3,int param_4)`
pub fn heroFUNC_StartAnim(e: &mut Engine, param_1: f32, param_2: i32, param_3: i32, param_4: i32) -> LResult<i64> {
    heroFUNC_ValideCurrentSound(e)?;
    let r = LIBAnimMngr::PlayAnim(e, param_1, param_2, param_3, param_4);
    Ok(r as i64)
}

/// 0x00014518 (656 bytes) `long heroFUNC_StartBlend(undefined4 param_1,int param_2,undefined4 param_3,undefined4 param_4)`
/// `param_1` is the float StartAnim time ($f12). Actions: 1 run blend tree, 2 anim 2, 3 anim
/// 0xfffff844; others are reported invalid. Stores the action in local+0x230.
pub fn heroFUNC_StartBlend(e: &mut Engine, param_1: f32, param_2: i32, param_3: u32, param_4: u32) -> LResult<i64> {
    let l = L(e);
    e.w32(l + 0x230, param_2 as u32);
    match param_2 {
        2 => heroFUNC_StartAnim(e, param_1, 2, param_3 as i32, param_4 as i32),
        3 => heroFUNC_StartAnim(e, param_1, 0xfffff844u32 as i32, param_3 as i32, param_4 as i32),
        1 => {
            e.w32(l + 0x2a4, 0xbb00_0002);
            e.w32(l + 0x2a8, 0x3e19_999a);
            e.w32(l + 0x2b0, 0);
            e.w32(l + 0x2ac, 0x3dcc_cccd);
            e.w32(l + 0x2b4, 0x3f00_0000);
            APIanim::BeginTree(e, SELF);
            let r = heroFUNC_StartAnim(e, f32::from_bits(0x3e09_374c), 1, 2, 0)? as i32;
            APIanim::PlayBlendConnect(e, SELF, k(e, heroFUNC_StartBlend_0_1), 1, 10, l + 0x2a4);
            let fStack_38 = 0.075f32;
            APIanim::SetTime(e, SELF, 1, 0.075);
            APIanim::SetTime(e, SELF, k(e, heroFUNC_StartBlend_0_2), fStack_38);
            e.w32(l + 0x2b8, 1);
            APIanim::EndTree(e, SELF);
            Ok(r as i64)
        }
        _ => {
            let v = e.alloca(4);
            e.w32(v, param_2 as u32);
            debug_info_value(e, "heroFUNC_InitBlend -> invalid eBlendAction", v, 1);
            Ok(e.r32(v) as i32 as i64)
        }
    }
}

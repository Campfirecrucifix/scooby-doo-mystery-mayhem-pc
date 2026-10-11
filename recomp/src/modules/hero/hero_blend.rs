//! hero_blend.c (3 functions + heroFUNC_UpdateBlend_Others, which only the Shaggy builds have).
//! Addresses are module-relative (.text at 0x10000) in scooby__402420b4.
//!
//! Shape groups (builds.rs): the Scooby builds blend walk/run with `heroFUNC_UpdateBlend_Scooby`, the
//! Shaggy builds with `heroFUNC_UpdateBlend_Others`. StartBlend has 4 shapes: Scooby debug (0),
//! Scooby release (1), Shaggy debug (2), Shaggy release (3). Per-build anim ids come from the slots.
//!
//! EABI note: the float argument travels in $f12, independently of the int arguments in $a0..
//! (`heroFUNC_StartBlend(0x3dcccccd, action, 2, 0)` = time 0.1, action, flags 2, notify 0).
use super::builds::*;
use super::*;
use crate::engine::*;
use crate::logic::LResult;

/// `EFBlendParams` of the blend-tree calls: { id, f, f, f, f } (5 words, built on the stack).
fn blend_params(e: &mut Engine, id: u32) -> Ptr {
    let p = e.alloca(0x14);
    e.w32(p, id);
    e.w32(p + 4, 0);
    e.w32(p + 0xc, 0);
    // fStack_58 = 1.0 - **(float **)(pGlobalDataHero + 4)
    let f = 1.0f32 - e.rf(e.r32(GD(e) + 4));
    e.wf(p + 8, f);
    e.w32(p + 0x10, 0x3f00_0000);
    p
}

/// The walk/run prologue shared by UpdateBlend_Scooby and the Scooby StartBlend shapes: action 1 (run
/// blend) is demoted to 0 (walk) below half the max speed, 0 is promoted to 1 at or above it.
fn scooby_action(e: &Engine, l: Ptr, param_2: i32) -> i32 {
    if param_2 != 1 || e.rf(l + 0x70) * 0.5 <= e.rf(l + 0x2e0) {
        if param_2 == 0 && e.rf(l + 0x70) * 0.5 <= e.rf(l + 0x2e0) {
            1
        } else {
            param_2
        }
    } else {
        0
    }
}

/// Case 1 of every StartBlend shape: the run blend tree between two anims, both clocks at 0.075.
fn start_run_blend(e: &mut Engine, l: Ptr) -> LResult<i32> {
    e.w32(l + 0x2a4, 0xbb00_0002);
    e.w32(l + 0x2a8, 0x3e19_999a);
    e.w32(l + 0x2b0, 0);
    e.w32(l + 0x2ac, 0x3dcc_cccd);
    e.w32(l + 0x2b4, 0x3f00_0000);
    APIanim::BeginTree(e, SELF);
    let r = heroFUNC_StartAnim(e, f32::from_bits(0x3e09_374c), k(e, heroFUNC_StartBlend_0_1) as i32, 2, 0)? as i32;
    APIanim::PlayBlendConnect(e, SELF, k(e, heroFUNC_StartBlend_0_2), k(e, heroFUNC_StartBlend_0_3), 10, l + 0x2a4);
    let fStack_40 = 0.075f32;
    APIanim::SetTime(e, SELF, k(e, heroFUNC_StartBlend_0_4), 0.075);
    APIanim::SetTime(e, SELF, k(e, heroFUNC_StartBlend_0_5), fStack_40);
    e.w32(l + 0x2b8, 1);
    APIanim::EndTree(e, SELF);
    Ok(r)
}

/// A blend tree: `params` id, StartAnim(time, anim, flags, notify) then PlayBlendConnect(blend, anim, n).
fn start_tree(e: &mut Engine, id: u32, time: f32, anim: u32, flags: u32, notify: u32, blend: u32, anim2: u32, n: u32) -> LResult<i32> {
    let params = blend_params(e, id);
    APIanim::BeginTree(e, SELF);
    let r = heroFUNC_StartAnim(e, time, anim as i32, flags as i32, notify as i32)? as i32;
    APIanim::PlayBlendConnect(e, SELF, blend, anim2, n, params);
    APIanim::EndTree(e, SELF);
    Ok(r)
}

/// 0x000187d8 (1008 bytes) `long heroFUNC_UpdateBlend_Scooby(int param_1,int param_2)`
///
/// Every frame while walking/running: keeps the blend action in sync with the speed, re-syncs the two
/// blended anim clocks, and sets the blend weight from the speed. The return value is not used by any
/// caller (on the PS2 it is whatever was in $v0); the port returns the same values where the C has
/// them and 0 after `UpdateTimerOutOfTime` (void).
pub fn heroFUNC_UpdateBlend_Scooby(e: &mut Engine, param_1: Ptr, param_2: i32) -> LResult<i64> {
    let gi = G_heroFUNC_UpdateBlend_Scooby;
    if g(e, gi) > 1 {
        unported_shape(e, "heroFUNC_UpdateBlend_Scooby", gi);
    }
    let iStack_4c = scooby_action(e, param_1, param_2);
    if e.r32(param_1 + 0x230) as i32 != iStack_4c {
        // 0x3dcccccd = 0.1
        let r = heroFUNC_StartBlend(e, 0.1, iStack_4c, 2, 0)?;
        e.w32(param_1 + 0x218, r as u32);
        return Ok(r);
    }
    if e.r32(param_1 + 0x2b8) != 0 {
        // disasm: a = GetTime(anim1), b = GetTime(anim2); Ghidra's in_f0 shows them swapped
        let a = APIanim::GetTime(e, SELF, k(e, heroFUNC_UpdateBlend_Scooby_0_1));
        let b = APIanim::GetTime(e, SELF, k(e, heroFUNC_UpdateBlend_Scooby_0_2));
        if a != b {
            // "-1.anim1 %6.4f   ->  anim2 %6.4f  =  %6.4f\n" (a, b, a - b): float varargs are not
            // forwarded (null_printf discards its output)
            let f = e.cstr("-1.anim1 %6.4f   ->  anim2 %6.4f  =  %6.4f\n");
            APIextra::null_printf(e, f, 0);
            APIanim::SetTime(e, SELF, k(e, heroFUNC_UpdateBlend_Scooby_0_3), b);
            let v = (e.r32(param_1 + 0x2b8) == 0) as u32;
            e.w32(param_1 + 0x2b8, v);
        }
    }
    let r = if iStack_4c == 1 {
        let fVar3 = e.rf(param_1 + 0x2e0);
        let fVar4 = e.rf(param_1 + 0x70) * 0.5;
        if fVar3 < e.rf(param_1 + 0x70) * 0.6 && e.r32(param_1 + 0x158) == addr::heroST_Move_Loop {
            UpdateTimerOutOfTime(e, 2.0)?;
        }
        let w = (fVar3 - fVar4) / (e.rf(param_1 + 0x70) * 0.95 - fVar4);
        e.wf(param_1 + 0x2ac, w);
        if 1.0 < e.rf(param_1 + 0x2ac) {
            e.w32(param_1 + 0x2ac, 0x3f80_0000);
        } else if e.rf(param_1 + 0x2ac) < 0.0 {
            e.w32(param_1 + 0x2ac, 0);
        }
        APIanim::SetSpeed(e, SELF, k(e, heroFUNC_UpdateBlend_Scooby_0_4), 1.0);
        APIanim::SetSpeed(e, SELF, k(e, heroFUNC_UpdateBlend_Scooby_0_5), 1.0);
        APIanim::ChangeBlendParams(e, SELF, param_1 + 0x2a4) as i32 as i64
    } else if iStack_4c == 0 {
        // lVar2 = (address of heroST_Move_Loop): a leftover register value, unused
        let mut r = addr::heroST_Move_Loop as i64;
        if e.r32(param_1 + 0x158) == addr::heroST_Move_Loop {
            UpdateTimerOutOfTime(e, 2.0)?;
            r = 0;
        }
        r
    } else if iStack_4c == 3 {
        3
    } else {
        debug_time(e, "Failed to update blend", 0, 0)? as i64
    };
    Ok(r)
}

/// 0x00018bc8 (152 bytes) `long heroFUNC_StartAnim(float param_1,int param_2,int param_3,int param_4)`
///
/// Same code in every build: select the anim's looping sound, then play the anim through the
/// AnimMngr library.
pub fn heroFUNC_StartAnim(e: &mut Engine, param_1: f32, param_2: i32, param_3: i32, param_4: i32) -> LResult<i64> {
    let l = L(e);
    heroFUNC_ValideCurrentSound(e, l, param_2 as u32)?;
    let r = LIBAnimMngr::PlayAnim(e, param_1, param_2, param_3, param_4);
    Ok(r as i64)
}

/// 0x00018c60 (1280 bytes) `long heroFUNC_StartBlend(undefined4 param_1,int param_2,uint param_3,undefined4 param_4)`
///
/// `param_1` is the float StartAnim time ($f12). Actions: 0 walk (plain anim; Shaggy builds have no
/// action 0 and report it as invalid), 1 run blend, 2 slow-walk blend (Scooby: a plain anim on
/// floors whose attribute bit 0 is set), 3 the second blend tree. Stores the action in local+0x230.
pub fn heroFUNC_StartBlend(e: &mut Engine, param_1: f32, param_2: i32, param_3: u32, param_4: u32) -> LResult<i64> {
    let gi = G_heroFUNC_StartBlend;
    let l = L(e);
    match g(e, gi) {
        // Scooby (0 = debug, 1 = release)
        grp @ (0 | 1) => {
            let uStack_a0 = scooby_action(e, l, param_2);
            e.w32(l + 0x230, uStack_a0 as u32);
            if uStack_a0 == 1 {
                return Ok(start_run_blend(e, l)? as i64);
            }
            if uStack_a0 < 2 {
                if uStack_a0 == 0 {
                    return heroFUNC_StartAnim(e, param_1, k(e, heroFUNC_StartBlend_0_6) as i32, param_3 as i32, param_4 as i32);
                }
            } else if uStack_a0 == 2 {
                // EnvPrimAttribs aEStack_80[8] (0x20 bytes up to uStack_60); flags word at +8
                let attribs = e.alloca(0x20);
                APIphysic::GetFloorAttribs(e, SELF, attribs);
                let set = e.r32(attribs + 8) & 1 != 0;
                let (plain, tree, blend, anim2) = if grp == 0 {
                    (heroFUNC_StartBlend_0_7, heroFUNC_StartBlend_0_8, heroFUNC_StartBlend_0_9, heroFUNC_StartBlend_0_10)
                } else {
                    (heroFUNC_StartBlend_1_3, heroFUNC_StartBlend_0_12, heroFUNC_StartBlend_1_1, heroFUNC_StartBlend_1_2)
                };
                if set {
                    return heroFUNC_StartAnim(e, param_1, k(e, plain) as i32, param_3 as i32, param_4 as i32);
                }
                let (a, b, c) = (k(e, tree), k(e, blend), k(e, anim2));
                return Ok(start_tree(e, 0xbb00_0003, param_1, a, param_3, param_4, b, c, param_3)? as i64);
            } else if (grp == 0 && uStack_a0 as u32 == k(e, heroFUNC_StartBlend_0_11)) || (grp == 1 && uStack_a0 == 3) {
                let anim = if grp == 0 { k(e, heroFUNC_StartBlend_0_12) } else { k(e, heroFUNC_StartBlend_1_4) };
                let (b, c) = (k(e, heroFUNC_StartBlend_0_13), k(e, heroFUNC_StartBlend_0_14));
                return Ok(start_tree(e, 0xbb00_0004, param_1, anim, param_3, param_4, b, c, 2)? as i64);
            }
            if grp == 0 {
                let v = e.alloca(4);
                e.w32(v, uStack_a0 as u32);
                debug_info_value(e, "heroFUNC_InitBlend -> invalid eBlendAction", v, 1);
            }
            Ok(uStack_a0 as i64)
        }
        // Shaggy (2 = debug, 3 = release): no speed prologue
        grp @ (2 | 3) => {
            e.w32(l + 0x230, param_2 as u32);
            if param_2 == 2 {
                let (a, b, c) = if grp == 2 {
                    (heroFUNC_StartBlend_2_1, heroFUNC_StartBlend_2_2, heroFUNC_StartBlend_2_3)
                } else {
                    (heroFUNC_StartBlend_3_1, heroFUNC_StartBlend_3_2, heroFUNC_StartBlend_3_3)
                };
                let (a, b, c) = (k(e, a), k(e, b), k(e, c));
                return Ok(start_tree(e, 0xbb00_0003, param_1, a, param_3, param_4, b, c, param_3)? as i64);
            }
            if param_2 < 3 {
                if param_2 == 1 {
                    return Ok(start_run_blend(e, l)? as i64);
                }
            } else if param_2 as u32 == k(e, heroFUNC_StartBlend_0_11) {
                let (a, b, c) = (k(e, heroFUNC_StartBlend_0_12), k(e, heroFUNC_StartBlend_0_13), k(e, heroFUNC_StartBlend_0_14));
                return Ok(start_tree(e, 0xbb00_0004, param_1, a, param_3, param_4, b, c, 2)? as i64);
            }
            // release (3): falls out returning param_2 with no print
            if grp == 2 {
                let v = e.alloca(4);
                e.w32(v, param_2 as u32);
                debug_info_value(e, "heroFUNC_InitBlend -> invalid eBlendAction", v, 1);
            }
            Ok(param_2 as i64)
        }
        _ => unported_shape(e, "heroFUNC_StartBlend", gi),
    }
}

/// `heroFUNC_UpdateBlend_Others` (Shaggy builds only; 0x000153e8 in shaggy__0117f115)
/// `long heroFUNC_UpdateBlend_Others(int param_1,int param_2)`
///
/// Shaggy's walk/run blend: below a third of the max speed only the anim speed scales (blend weight
/// 0), above it the weight rises to 1 at 95 % and the speed from 2/3 to 1.25. Return value unused by
/// the callers.
pub fn heroFUNC_UpdateBlend_Others(e: &mut Engine, param_1: Ptr, param_2: i32) -> LResult<i64> {
    let gi = G_heroFUNC_UpdateBlend_Others;
    if g(e, gi) > 1 {
        unported_shape(e, "heroFUNC_UpdateBlend_Others", gi);
    }
    if e.r32(param_1 + 0x2b8) != 0 {
        // disasm (shaggy__0117f115 0x15414..): a = GetTime(anim1) @fp+8, b = GetTime(anim2) @fp+0xc
        let a = APIanim::GetTime(e, SELF, k(e, heroFUNC_UpdateBlend_Others_0_1));
        let b = APIanim::GetTime(e, SELF, k(e, heroFUNC_UpdateBlend_Others_0_2));
        if a != b {
            // "0.anim1 %6.4f   ->  anim2 %6.4f  =  %6.4f\n" (a, b, a - b): float varargs not forwarded
            let f = e.cstr("0.anim1 %6.4f   ->  anim2 %6.4f  =  %6.4f\n");
            APIextra::null_printf(e, f, 0);
            APIanim::SetTime(e, SELF, k(e, heroFUNC_UpdateBlend_Others_0_3), b);
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
        let (s1, s2, speed) = if x < third {
            e.w32(param_1 + 0x2ac, 0);
            if x < third * 0.5 {
                x = third * 0.5;
            }
            (heroFUNC_UpdateBlend_Others_0_4, heroFUNC_UpdateBlend_Others_0_5, (x / third) * 0.6666667)
        } else {
            let w = (x - third) / (e.rf(param_1 + 0x70) * 0.95 - third);
            e.wf(param_1 + 0x2ac, w);
            if 1.0 < e.rf(param_1 + 0x2ac) {
                e.w32(param_1 + 0x2ac, 0x3f80_0000);
            } else if e.rf(param_1 + 0x2ac) < 0.0 {
                e.w32(param_1 + 0x2ac, 0);
            }
            let s = e.rf(param_1 + 0x2ac) * 0.5833333 + 0.6666667;
            (heroFUNC_UpdateBlend_Others_0_6, heroFUNC_UpdateBlend_Others_0_7, s)
        };
        APIanim::SetSpeed(e, SELF, k(e, s1), speed);
        APIanim::SetSpeed(e, SELF, k(e, s2), speed);
        return Ok(APIanim::ChangeBlendParams(e, SELF, param_1 + 0x2a4) as i32 as i64);
    }
    if param_2 != 0 && param_2 != 3 && g(e, gi) == 0 {
        let v = e.alloca(4);
        e.w32(v, param_2 as u32);
        debug_info_value(e, "FUNC_UpdateBlend_Others -> invalid eBlendAction", v, 1);
    }
    Ok(param_2 as i64)
}

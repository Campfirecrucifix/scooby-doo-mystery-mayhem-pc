//! hero_sounds.c (3 functions). Addresses are module-relative (.text at 0x10000) in scooby_ghost__08a011d0.
use super::builds::*;
use super::*;
use crate::engine::*;
use crate::logic::LResult;

/// 0x00025c68 (272 bytes) `undefined4 heroFUNC_GetSurface(void)`
/// Floor material (EnvPrimAttribs halfword at +0x16) -> surface index into the per-surface sound
/// records (0x28 bytes each). Same in all builds.
pub fn heroFUNC_GetSurface(e: &mut Engine) -> LResult<u32> {
    let a = e.alloca(0x40); // aEStack_40 [22] + uStack_2a
    APIphysic::GetFloorAttribs(e, 0x2a00_0000, a);
    Ok(match e.r16(a + 0x16) {
        1 => 10,
        3 => 6,
        5 | 6 => 0xd,
        7 => 3,
        9 => 7,
        0xb => 0xb,
        0xd => 0xc,
        0xf => 9,
        0x10 => 2,
        0x13 => 5,
        0x14 => 4,
        _ => 0,
    })
}

/// 0x00025d78 (1580 bytes) `void heroFUNC_PlaySound(int *param_1,undefined4 param_2,uint param_3)`
/// `param_1` = local data: [0] surface sound table (0x28-byte records), [1] generic sound table.
/// The two shape groups only swap the switch-VO debug strings (group 0: slot 1 = Shaggy).
pub fn heroFUNC_PlaySound(e: &mut Engine, param_1: Ptr, param_2: u32, param_3: u32) -> LResult<()> {
    let surf = heroFUNC_GetSurface(e)?;
    let gsnd = |e: &Engine, o: u32| e.r32(e.r32(param_1 + 4) + o) as i32;
    let sur = |e: &Engine, o: u32| e.r32(surf * 0x28 + e.r32(param_1) + o) as i32;
    match param_2 {
        0xfffff840 => {
            if param_3 == 1 {
                let s = gsnd(e, 0x34);
                LIBSndMngr::PlaySound(e, s);
                if e.r32(GB0(e) + 8) != e.r32(L(e) + 0x17c) {
                    let id = e.r32(param_1 + 0x17c);
                    let (n1, n2) = if g(e, G_heroFUNC_PlaySound) == 0 {
                        ("SON VO_SHAGGY_SWITCH", "SON VO_SCOOBY_SWITCH")
                    } else {
                        ("SON VO_SCOOBY_SWITCH", "SON VO_SHAGGY_SWITCH")
                    };
                    if id == k(e, heroFUNC_PlaySound_0_1) {
                        debug_info(e, n1);
                        LIBSndMngr::PlaySound(e, k(e, ID_7fffffcb).wrapping_neg() as i32);
                    } else if id == k(e, heroFUNC_PlaySound_0_2) {
                        debug_info(e, n2);
                        LIBSndMngr::PlaySound(e, k(e, ID_7fffffcd).wrapping_neg() as i32);
                    }
                }
            } else if param_3 == 0 {
                let s = sur(e, 0x1c);
                LIBSndMngr::PlaySound(e, s);
            } else if param_3 == 2 {
                let s = sur(e, 0x24);
                LIBSndMngr::PlaySound(e, s);
            }
        }
        0xfffff858 => {
            if param_3 == 0 {
                let s = gsnd(e, 0x5c);
                LIBSndMngr::PlaySound(e, s);
            }
        }
        0xfffff859 => {
            if param_3 < 4 && param_3 != 0 {
                let s = gsnd(e, 0xa4);
                LIBSndMngr::PlaySound(e, s);
            }
        }
        0xfffff85a => match param_3 {
            1 | 3 | 5 => {
                let s = gsnd(e, 0x7c);
                LIBSndMngr::PlaySound(e, s);
            }
            2 | 4 | 6 => {
                let s = gsnd(e, 0x84);
                LIBSndMngr::PlaySound(e, s);
            }
            _ => {}
        },
        0xfffff864 => {
            if param_3 == 0 {
                let s = sur(e, 0x24);
                LIBSndMngr::PlaySound(e, s);
            }
        }
        0xfffff867 => {
            if param_3 == 0 {
                let s = gsnd(e, 0x44);
                LIBSndMngr::PlaySound(e, s);
            } else if param_3 == 99 {
                let s = gsnd(e, 0x74);
                LIBSndMngr::PlaySound(e, s);
            }
        }
        0xfffff869 | 0xfffff87b => {
            let s = sur(e, 0x24);
            LIBSndMngr::PlaySound(e, s);
        }
        0xfffff874 => {
            if param_3 == 0 {
                debug_info(e, "CB ANIM START DUCK");
            } else if param_3 == 1 {
                debug_info(e, "CB ANIM STOP DUCK");
                let s = e.r32(e.r32(param_1 + 4) + 0xbc);
                APInewSound::StopSound(e, SELF, s);
            }
        }
        0xfffff879 => {
            if param_3 == 0 {
                let s = gsnd(e, 0x44);
                LIBSndMngr::PlaySound(e, s);
            }
        }
        _ => {}
    }
    Ok(())
}

/// 0x000263a8 (40 bytes) `void heroFUNC_ValideCurrentSound(void)`
pub fn heroFUNC_ValideCurrentSound(_e: &mut Engine) -> LResult<()> {
    Ok(())
}

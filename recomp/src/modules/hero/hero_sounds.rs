//! hero_sounds.c (3 functions). Addresses are module-relative (.text at 0x10000) in scooby__402420b4.
use super::*;
use crate::engine::*;
use crate::logic::LResult;

/// 0x0002be40 (272 bytes) `undefined4 heroFUNC_GetSurface(void)`
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

/// 0x0002bf50 (2296 bytes) `void heroFUNC_PlaySound(int *param_1,uint param_2,uint param_3)`
/// Anim-event sounds: per-build code generated into play_sound.rs (tools/rustport/gen_hero_playsound.py).
pub fn heroFUNC_PlaySound(e: &mut Engine, param_1: Ptr, param_2: u32, param_3: u32) -> LResult<()> {
    let sha = builds::BUILDS[e.rt.cur_build()].sha8;
    let f = super::play_sound::PLAY_SOUND
        .iter()
        .find(|b| b.0 == sha)
        .unwrap_or_else(|| panic!("heroFUNC_PlaySound: no code for build {sha}"))
        .1;
    f(e, param_1, param_2, param_3)
}

/// 0x0002c848 (688 bytes) `void heroFUNC_ValideCurrentSound(int param_1,undefined4 param_2)`
///
/// Picks the looping sound that belongs to animation `param_2` (the `switch` differs per build:
/// [`tables::VALIDE_CURRENT_SOUND`]) into local+0x21c and stops the previous one if it changed.
/// The chain after StopSound only prints (debug builds).
pub fn heroFUNC_ValideCurrentSound(e: &mut Engine, param_1: Ptr, param_2: u32) -> LResult<()> {
    use tables::SndAct;
    let t = &tables::VALIDE_CURRENT_SOUND[e.rt.cur_build()];
    // uint auStack_28[2]: address-taken (PrintDebugInfoWithValue)
    let st = e.alloca(8);
    e.w32(st, e.r32(param_1 + 0x21c));
    let v = match t.cases.iter().find(|c| c.0 == param_2 as i32) {
        Some(&(_, SndAct::Field(off))) => e.r32(e.r32(param_1 + 4) + off),
        Some(&(_, SndAct::Const(c))) => c,
        None => 0xffff_ffff,
    };
    e.w32(param_1 + 0x21c, v);
    let old = e.r32(st);
    if old != 0xffff_ffff && old != e.r32(param_1 + 0x21c) {
        APInewSound::StopSound(e, SELF, old);
        if t.debug {
            debug_info_value(e, "Stopping sound", st, 1);
            if old == t.snore {
                debug_info(e, "STOP snore sound");
            } else if old == t.runaway {
                debug_info(e, "STOP runaway sound");
            } else if old == e.r32(e.r32(param_1 + 4) + 0xbc) {
                debug_info(e, "STOP DUCK");
            } else if old == e.r32(e.r32(param_1 + 4) + 0x9c) {
                debug_info(e, "STOP TEETHCHATTERING");
            }
        }
    }
    Ok(())
}

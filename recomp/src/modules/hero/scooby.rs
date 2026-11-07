//! scooby.c / shaggy.c (5 functions in the reference build). Addresses are module-relative (.text at
//! 0x10000) in scooby__402420b4. The Shaggy builds' `shaggyFUNC_*` / `shaggyConstructor` are the same
//! functions under the Scooby names here.
//!
//! This file is the per-level part of the hero: every level pack compiles its own version (43 distinct
//! bodies over the 57 builds). Ported here: every build whose file has only these 5 functions (40
//! builds; their level code is a few collision-attribute bits and message filters, dispatched by build
//! below). NOT PORTED YET (level gameplay, not book code; they panic loudly in the constructor):
//! the 16 builds with extra level states (doors, pipes, carrousel, sawing, aqueduct, go-to-point,
//! locked door) and 0117f115 (Shaggy race, hero_race.c).
use super::*;
use crate::engine::*;
use crate::logic::LResult;

const SELF: u32 = 0x2a00_0000;
const NULL: Ptr = 0;

fn sha(e: &Engine) -> &'static str {
    builds::BUILDS[e.rt.cur_build()].sha8
}

/// Level code: hero_level.rs (15 builds) and hero_race.rs (the 2 race builds).
fn is_race(e: &Engine) -> bool {
    super::hero_race::RACE_BUILDS.contains(&sha(e))
}

fn is_level(e: &Engine) -> bool {
    super::hero_level::LEVEL_BUILDS.contains(&sha(e))
}

/// 0x0002ee58 (32 bytes) `void heroFUNC_CheckActionEventsLocal(void)`
/// Empty in every ported build (0117f115 starts its race states here).
pub fn heroFUNC_CheckActionEventsLocal(e: &mut Engine) -> LResult<()> {
    if is_race(e) {
        return super::hero_race::check_action_events_local(e);
    }
    if is_level(e) {
        return super::hero_level::check_action_events_local(e);
    }
    Ok(())
}

/// 0x0002ee78 (72 bytes) `void scoobyFUNC_Create_Local(void)`
/// local+0 / +4 = the character's surface / generic sound tables. Level builds then set collision
/// attribute bits (attribute 5) on the hero's primary prim (local+0x18c); 79185eb3 / df8bf53b first
/// redo heroFUNC_Create_Hero's stores with level ids (local+0x178 = 0xd).
pub fn scoobyFUNC_Create_Local(e: &mut Engine) -> LResult<()> {
    let s = sha(e);
    let over = match s {
        "79185eb3" => Some((0x2a00_001b, 0x2a00_0021)),
        "df8bf53b" => Some((0x2a00_0044, 0x2a00_004a)),
        _ => None,
    };
    if let Some((a, b)) = over {
        debug_info(e, "scoobyFUNC_Create_Local");
        let l = L(e);
        for (o, v) in [(0x178, 0xd), (0x17c, a), (0x180, b), (0x184, 0x250_0000), (0x188, 1), (0x18c, 100)] {
            e.w32(l + o, v);
        }
    }
    let l = L(e);
    let (snd, gen_) = (G(e, goSurfaceSoundScooby), G(e, goGenericSoundScooby));
    e.w32(l, snd);
    e.w32(l + 4, gen_);
    let bits: &[u32] = match s {
        "97105e68" | "a37a6374" | "b1427d0c" | "d05de456" | "f26306af" | "35390b5f" | "2ac8998c" => &[0x400_0000],
        "914156a5" | "6e3372f0" => &[0x10_0000],
        "4fb89b85" | "74367063" | "11df665e" => &[0x80_0000],
        "c2244c8b" => &[0x10_0000, 0x80_0000],
        "2a9862df" | "f3014777" => &[0x400_0000, 0x800_0000, 0x1000_0000, 0x2000_0000],
        _ => super::hero_level::create_local_bits(e),
    };
    for &b in bits {
        let prim = e.r32(L(e) + 0x18c);
        APIphysic::PrimSetIntAttribBits(e, SELF, prim, 5, b);
    }
    Ok(())
}

fn prim_bits(e: &mut Engine, set: bool, attr: u32, bits: u32) {
    let prim = e.r32(L(e) + 0x18c);
    if std::env::var_os("GAME_TOWERLOG").is_some() { eprintln!("PRIMBITS hero {:#x} set {set} attr {attr} bits {bits:#x}", APIgameobj::GetID(e, SELF, 0)); }
    if set {
        APIphysic::PrimSetIntAttribBits(e, SELF, prim, attr, bits);
    } else {
        APIphysic::PrimResetIntAttribBits(e, SELF, prim, attr, bits);
    }
}

/// 0x0002eec0 (112 bytes) `int scoobyFUNC_MessageHandler(undefined4 param_1,uint param_2,int *param_3)`
/// The level filter (by build), then heroFUNC_MessageHandler (except 415322c3's 0x152, answered 0).
pub fn scoobyFUNC_MessageHandler(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    if is_level(e) {
        if let Some(r) = super::hero_level::message(e, param_1, param_2, param_3)? {
            return Ok(r);
        }
        return Ok(heroFUNC_MessageHandler(e, param_1, param_2, param_3)? as u32 as u64);
    }
    match sha(e) {
        // collision bit 0x4000000 on/off (0xc9 / 0xe5)
        "2b9d74e4" | "1730c849" => match param_1 {
            0xc9 => prim_bits(e, true, 5, 0x400_0000),
            0xe5 => prim_bits(e, false, 5, 0x400_0000),
            _ => {}
        },
        // 0x4f (hero dead, printed) / 0xc9: bit on; 0xe5: off; 0x1d1: attribute 0 bit 2 off;
        // 0x78: message 199 to 0x2a00000d
        "625480d1" | "61655a9e" => match param_1 {
            0x4f | 0xc9 => {
                debug_info(e, "MSG_HERO_IS_DEAD");
                prim_bits(e, true, 5, 0x400_0000);
            }
            0xe5 => prim_bits(e, false, 5, 0x400_0000),
            0x1d1 => prim_bits(e, false, 0, 2),
            0x78 => {
                APIgameobj::SendMessageTo(e, 0x2a00_000d, NULL, 199, 0, NULL);
            }
            _ => {}
        },
        "79185eb3" => {
            if param_1 == 0x1d1 {
                prim_bits(e, false, 0, 2);
            }
        }
        // 0x87: physics priority 100 / 4; 0x1d1 (f5ff9d42): bit 0x20000000 of prim 0x67
        "f5ff9d42" | "35390b5f" => {
            if param_1 == 0x87 {
                APIphysic::SetPriority(e, SELF, if param_2 == 1 { 100 } else { 4 });
            } else if param_1 == 0x1d1 && sha(e) == "f5ff9d42" {
                if param_2 == 0 {
                    APIphysic::PrimResetIntAttribBits(e, SELF, 0x67, 5, 0x2000_0000);
                } else {
                    APIphysic::PrimSetIntAttribBits(e, SELF, 0x67, 5, 0x2000_0000);
                }
            }
        }
        "415322c3" => {
            if param_1 == 0x152 {
                if e.r32(GB2(e) + 8) & 0x400_0000 != 0 {
                    prim_bits(e, true, 5, 0x200_0000);
                }
                return Ok(0);
            }
        }
        "11ffbd5f" => {
            if param_1 == 0x1fb {
                let l = L(e);
                e.w32(l + 0x78, param_2);
                e.w32(l + 0x7c, 0x2a00_003b);
            } else if param_1 == 0x1fc {
                APIphysic::PrimResetIntAttribBits(e, SELF, 0x70, 5, 0x800_0000);
            }
        }
        // 0x127..0x135: the four collision bits; 0x133: forced action (local+0x23c..0x248, event
        // 0x1000, GB0+0x14 bit 0x40000000); 0x14c: all four off; 0x1d1 (2a9862df only): attr 0 bit 2 off
        "2a9862df" | "f3014777" => match param_1 {
            0x127 => prim_bits(e, false, 5, 0x400_0000),
            0x128 => prim_bits(e, true, 5, 0x400_0000),
            299 => prim_bits(e, true, 5, 0x1000_0000),
            300 => prim_bits(e, false, 5, 0x1000_0000),
            0x12f => prim_bits(e, true, 5, 0x800_0000),
            0x130 => prim_bits(e, false, 5, 0x800_0000),
            0x133 => {
                let l = L(e);
                e.w32(l + 0x23c, param_2);
                e.w32(l + 0x240, 7);
                e.w32(l + 0x244, 2);
                e.w32(l + 0x248, 1);
                or_w(e, l + 0x170, 0x1000);
                let b = GB0(e) + 0x14;
                or_w(e, b, 0x4000_0000);
            }
            0x134 => prim_bits(e, false, 5, 0x2000_0000),
            0x135 => prim_bits(e, true, 5, 0x2000_0000),
            0x14c => {
                for b in [0x400_0000, 0x800_0000, 0x1000_0000, 0x2000_0000] {
                    prim_bits(e, false, 5, b);
                }
            }
            0x1d1 if sha(e) == "2a9862df" => prim_bits(e, false, 0, 2),
            _ => {}
        },
        "c2244c8b" => match param_1 {
            0x137 => prim_bits(e, param_2 != 1, 5, 0x20_0000),
            0x4f => {
                let b5 = e.r32(GD(e) + 0x14);
                if e.r32(b5 + 8) == 1 {
                    e.w32(b5 + 8, 0);
                    return Ok(0);
                }
            }
            // pushed by a fan: a small nudge of the leader (Scooby, or 0x2a0000e5 when GB0+0x14 bit 0
            // is set); the vectors are the build's doubles DAT_000138c0 / DAT_000138d0 (x, y) + z
            0x13a => {
                let v = e.alloca(0x10);
                let (x, z) = if param_2 == 0 { (0x3d69_78d5, 0x3be5_6042) } else { (0xbd97_8d50, 0xbc03_126f) };
                e.w32(v, x);
                e.w32(v + 4, 0);
                e.w32(v + 8, z);
                let who = if e.r32(GB0(e) + 0x14) & 1 == 0 { SELF } else { 0x2a00_00e5 };
                APIgameobj::Translate(e, who, v);
            }
            _ => {}
        },
        _ if is_race(e) => super::hero_race::message(e, param_1, param_2)?,
        _ => {}
    }
    Ok(heroFUNC_MessageHandler(e, param_1, param_2, param_3)? as u32 as u64)
}

/// The size of the level local data (`GetLocalData(&pLocalDataHeroLevel, n)`), if the build has one.
fn level_data_size(s: &str) -> Option<i32> {
    match s {
        "128635c3" | "2b32b17e" | "402420b4" | "52d75aa5" | "52d91ddd" | "97105e68" | "a37a6374" | "aae74b1e"
        | "b1427d0c" | "bc7f3c1a" | "c55e6477" | "d05de456" | "da5e187b" | "e92e5027" | "ee9dba7d" | "f26306af"
        | "f28bcd0e" | "f5ff9d42" | "35390b5f" => None,
        _ => Some(0x38c),
    }
}

/// 0x0002ef30 (140 bytes) `void scoobyConstructor(APIcons *api,int instanceCount)`
pub fn scoobyConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    heroFUNC_Constructor(e, api, instanceCount)?;
    if is_race(e) {
        use super::hero_race as r;
        r::hero_localFUNC_Constructor(e, api, instanceCount)?;
        let slot = G(e, pLocalDataHeroLevel);
        APIcons::GetLocalData(e, slot, 0x3c0);
        APIcons::SetMessageFunction(e, scoobyFUNC_MessageHandler);
        r::race_create_local(e)?;
        let ll = e.r32(G(e, pLocalDataHeroLevel));
        r::hero_localFUNC_Create_Input(e, ll)?;
        r::hero_localFUNC_Init_Blend(e, ll)?;
        APIcons::SetInitialState(e, 0);
        return Ok(());
    }
    if is_level(e) {
        use super::hero_level as lv;
        lv::ctor_before_local(e);
        let slot = G(e, pLocalDataHeroLevel);
        let n = lv::level_data_size(e);
        APIcons::GetLocalData(e, slot, n);
        lv::ctor_after_local(e);
        APIcons::SetMessageFunction(e, scoobyFUNC_MessageHandler);
        scoobyFUNC_Create_Local(e)?;
        lv::ctor_after_create(e);
        APIcons::SetInitialState(e, 0);
        return Ok(());
    }
    if let Some(n) = level_data_size(sha(e)) {
        let slot = G(e, pLocalDataHeroLevel);
        APIcons::GetLocalData(e, slot, n);
    }
    APIcons::SetMessageFunction(e, scoobyFUNC_MessageHandler);
    scoobyFUNC_Create_Local(e)?;
    if sha(e) == "415322c3" {
        prim_bits(e, false, 2, 2);
    }
    APIcons::SetInitialState(e, 0);
    Ok(())
}

/// 0x0002efc0 (72 bytes) `void scoobyDestructor(int instanceCount)`
pub fn scoobyDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    if is_level(e) {
        super::hero_level::dtor(e);
    }
    heroFUNC_Destructor(e, instanceCount)
}

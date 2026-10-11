//! AnimMngr custom library (`Game Logic Modules/Globals/Lib/animmngr__8774cb6a/AnimMngr.c`, 9 functions).
//! Both builds (5a4f23b6, 8774cb6a) are the same code. It registers the 6-slot table "AnimMngr".
//!
//! Anim table entries used by PlayAnimRandom/SelectAnim*: 0x10 bytes = {+0 anim id, +4 speed (f32),
//! +8 notify id, +0xc frequency class}.
//!
//! Non-obvious, deliberate: `pLocalDataAnimMngr` is the lib module's own instance data (0x10 bytes,
//! from its constructor), not the caller's. Every module that plays a random anim shares the
//! "last random index" at +4, as in the original.
#![allow(non_snake_case, unused_variables)]
use super::{lib_data, set_global};
use crate::engine::core::apicons as cons;
use crate::engine::*;
use crate::logic::{LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "animmngr",
    data_size: 0x20,
    ctor: animmngrConstructor,
    dtor: animmngrDestructor,
    register: Some(animmngrRegister),
    init: &[],
    build_init: None,
};

// module globals (_globals.h)
const pExtra: u32 = 0x10000;
const pGameObj: u32 = 0x10004;
const pRegLib: u32 = 0x10008;
const pAnim: u32 = 0x1000c;
const pStdlib: u32 = 0x10010;
const pDebug: u32 = 0x10014;
const pAnimMngr: u32 = 0x10018;
const pLocalDataAnimMngr: u32 = 0x1001c;

/// the value of pLocalDataAnimMngr (the lib object's local data; NULL until that object is constructed)
fn local(e: &mut Engine) -> Ptr {
    let a = lib_data(e, &MODULE, pLocalDataAnimMngr);
    e.r32(a)
}

/// 0x00010d48 play anim `param_2` on the running object at speed `param_1` (0 = default, flag 4 otherwise);
/// `param_3` 1 = flag 1, 2 = flag 2; `param_4` != 0 registers notify `param_4` on the anim.
/// Returns the anim id, or -1 if it is < 1.
pub fn PlayAnim(e: &mut Engine, param_1: f32, param_2: i32, param_3: i32, param_4: i32) -> i32 {
    let mut param_2 = param_2;
    // the original checks its pAnim table pointer, which is never NULL once the lib is registered
    if param_2 < 1 {
        param_2 = -1;
    } else {
        let mut uStack_30 = 0u32;
        if param_4 != 0 {
            APIgameobj::RegisterNotify(e, SELF, param_2 as u32, 1, param_4 as u32);
        }
        if param_1 != 0.0 {
            uStack_30 = 4;
        }
        if param_3 == 1 {
            uStack_30 |= 1;
        } else if param_3 == 2 {
            uStack_30 |= 2;
        }
        APIanim::Play(e, SELF, param_2 as u32, uStack_30, param_1);
    }
    param_2
}

/// 0x00010ec0 only registers the notify: the blend itself was compiled out of the shipped game
pub fn PlayAnimBlend(e: &mut Engine, param_1: i32, param_2: i32, param_3: u64, param_4: i32) -> i32 {
    let mut param_1 = param_1;
    if param_1 < 1 || param_2 < 1 {
        param_1 = -1;
    } else if param_4 != 0 {
        APIgameobj::RegisterNotify(e, SELF, param_1 as u32, 1, param_4 as u32);
    }
    param_1
}

/// 0x00010ff0 play a random entry of the `param_1` entries at `param_2`, never the same index twice in
/// a row (shared lib state). `rand / (0x7ffe / n)` integer division: n > 0x7ffe divides by zero and
/// traps, as the original does (trap 7).
pub fn PlayAnimRandom(e: &mut Engine, param_1: u32, param_2: Ptr, param_3: i32) -> i32 {
    if param_1 == 0 {
        return -1;
    }
    let uVar1 = APIstdlib::MyRand(e);
    let mut iStack_34 = (uVar1 as i32) / (0x7ffe / (param_1 as i32));
    let l = local(e);
    if 1 < param_1 && e.ri32(l + 4) == iStack_34 {
        if iStack_34 == 0 {
            iStack_34 = param_1 as i32 - 1;
        } else {
            iStack_34 -= 1;
        }
    }
    e.wi32(l + 4, iStack_34);
    let ent = (iStack_34 as u32).wrapping_mul(0x10).wrapping_add(param_2);
    if e.ri32(ent) == -1 {
        -1
    } else {
        let (spd, id, nt) = (e.rf(ent + 4), e.ri32(ent), e.ri32(ent + 8));
        PlayAnim(e, spd, id, param_3, nt)
    }
}

/// 0x000111a0 like PlayAnim but through APIanim::PlayBlend with blend params `param_5`
pub fn PlayBlend(e: &mut Engine, param_1: f32, param_2: i32, param_3: i32, param_4: i32, param_5: Ptr) -> i32 {
    let mut param_2 = param_2;
    if param_2 < 1 {
        param_2 = -1;
    } else {
        let mut uStack_2c = 0u32;
        if param_4 != 0 {
            APIgameobj::RegisterNotify(e, SELF, param_2 as u32, 1, param_4 as u32);
        }
        if param_1 != 0.0 {
            uStack_2c = 4;
        }
        if param_3 == 1 {
            uStack_2c |= 1;
        } else if param_3 == 2 {
            uStack_2c |= 2;
        }
        APIanim::PlayBlend(e, SELF, param_2 as u32, uStack_2c, param_1, param_5);
    }
    param_2
}

/// 0x000112e8 idle-anim picker: loop counter `*param_1` (0..7) chooses frequency class 0, 1 (loops 3 and 6)
/// or 2 (loop 7, counter reset); a random entry of that class among the `param_2` entries at `param_3`
/// is played. No entry of the class -> recurse with the next loop count.
/// The candidate buffer holds 10 entries (40 words), as on the original stack.
pub fn SelectAnimFrq(e: &mut Engine, param_1: Ptr, param_2: u32, param_3: i32, param_4: i32) -> i32 {
    if param_2 == 0 {
        return -1;
    }
    let auStack_e0 = e.alloca(0xa0);
    let mut uStack_ec = 0u32;
    let mut iStack_e8 = 0i32;
    let mut uStack_40 = e.r32(param_1);
    e.w32(param_1, uStack_40.wrapping_add(1));
    let fmt = e.cstr("api animmngr> SelectAnimFrq -> loop #%u\n");
    APIextra::null_printf(e, fmt, uStack_40);
    if uStack_40 == 6 || uStack_40 == 3 {
        iStack_e8 = 1;
    } else if uStack_40 == 7 {
        iStack_e8 = 2;
        e.w32(param_1, 0);
        uStack_40 = 0;
    }
    let base = param_3 as u32;
    for uStack_f0 in 0..param_2 {
        let ent = uStack_f0 * 0x10 + base;
        if e.ri32(ent + 0xc) == iStack_e8 {
            let d = auStack_e0 + uStack_ec * 0x10;
            for k in 0..3 {
                let v = e.r32(ent + k * 4);
                e.w32(d + k * 4, v);
            }
            uStack_ec += 1;
        }
    }
    if uStack_ec == 0 {
        SelectAnimFrq(e, param_1, param_2, param_3, param_4)
    } else {
        PlayAnimRandom(e, uStack_ec, auStack_e0, param_4)
    }
}

/// 0x00011568 random entry not played yet in this round (bit set `*param_1`; all played -> new round)
pub fn SelectAnimOnce(e: &mut Engine, param_1: Ptr, param_2: u32, param_3: i32, param_4: i32) -> i32 {
    if param_2 == 0 {
        return -1;
    }
    let mut uStack_3c;
    loop {
        let uVar2 = APIstdlib::MyRand(e);
        uStack_3c = ((uVar2 as i32) % (param_2 as i32)) as u32;
        if (1u32 << (uStack_3c & 0x1f)) & e.r32(param_1) == 0 {
            let mut uStack_34 = 0;
            let v = e.r32(param_1) | 1 << (uStack_3c & 0x1f);
            e.w32(param_1, v);
            for uStack_38 in 0..param_2 {
                if (1u32 << (uStack_38 & 0x1f)) & e.r32(param_1) != 0 {
                    uStack_34 += 1;
                }
            }
            if uStack_34 == param_2 {
                e.w32(param_1, 0);
            }
            break;
        }
    }
    let ent = uStack_3c.wrapping_mul(0x10).wrapping_add(param_3 as u32);
    let (spd, id, nt) = (e.rf(ent + 4), e.ri32(ent), e.ri32(ent + 8));
    PlayAnim(e, spd, id, param_4, nt)
}

/// 0x00011758 store the API tables, then RegisterLib("AnimMngr", 0x18) for the 6-slot table
pub fn animmngrRegister(e: &mut Engine, api: Ptr) {
    let v = cons::GetApiExtra(e);
    set_global(e, &MODULE, pExtra, v);
    let v = cons::GetApiGameObj(e);
    set_global(e, &MODULE, pGameObj, v);
    let v = cons::GetApiAnim(e);
    set_global(e, &MODULE, pAnim, v);
    let v = cons::GetApiStdlib(e);
    set_global(e, &MODULE, pStdlib, v);
    let n = e.cstr("debug");
    let v = cons::GetApiCustom(e, n);
    set_global(e, &MODULE, pDebug, v);
    let n = e.cstr("AnimMngr");
    let v = cons::GetApiCustom(e, n);
    set_global(e, &MODULE, pAnimMngr, v);
    let v = cons::GetApiRegLib(e);
    set_global(e, &MODULE, pRegLib, v);
    let v = APIRegLib::RegisterLib(e, n, 0x18);
    set_global(e, &MODULE, pAnimMngr, v);
}

/// 0x000118e8 GetLocalData(&pLocalDataAnimMngr, 0x10)
pub fn animmngrConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    let slot = lib_data(e, &MODULE, pLocalDataAnimMngr);
    APIcons::GetLocalData(e, slot, 0x10);
    Ok(())
}

/// 0x00011948 UnRegisterLib("AnimMngr")
pub fn animmngrDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    let n = e.cstr("AnimMngr");
    APIRegLib::UnRegisterLib(e, n);
    Ok(())
}

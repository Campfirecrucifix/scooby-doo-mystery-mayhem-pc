//! FireGhost.c (3 functions per build): the per-level message handler and constructor.
//! 6f63c7c0 = EP6L04A, 818e89e1 = EP6L04B, 3a7b55ab = EP6L05.
//!
//! Message 0x98 (the leader hero entered a sector: game buffer 0 +0x24 = sector number) turns the
//! fire particles on for the fire ghosts of that sector and off for the others (EP6L05: always
//! on). Everything is then passed on to fireGhostBaseFUNC_MessageHandler.
use super::*;
use crate::engine::*;
use crate::logic::LResult;

/// EP6L04A: sector -> fire ghosts burning in it
const SECTORS_6F63C7C0: [(u32, &[u32]); 6] = [
    (1, &[0x2a00_0004, 0x2a00_000a]),
    (2, &[0x2a00_0005]),
    (3, &[0x2a00_0006, 0x2a00_0008]),
    (4, &[0x2a00_0007]),
    (5, &[0x2a00_000b]),
    (6, &[0x2a00_0009]),
];
/// EP6L04B
const SECTORS_818E89E1: [(u32, &[u32]); 4] = [
    (7, &[0x2a00_0149]),
    (1, &[0x2a00_0147, 0x2a00_0148]),
    (8, &[0x2a00_014a]),
    (10, &[0x2a00_014b]),
];

/// 0x00018a38 (6f63c7c0) / 0x00018a08 (818e89e1) / 0x00018940 (3a7b55ab)
pub fn fireGhostFUNC_MessageHandler(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    let iVar1 = APIgameobj::GetID(e, SELF, NULL);
    if param_1 == 0x98 {
        let table: &[(u32, &[u32])] = match build_key(e) {
            "6f63c7c0" => &SECTORS_6F63C7C0,
            "818e89e1" => &SECTORS_818E89E1,
            // 3a7b55ab: no sector test
            _ => {
                fireGhostBaseFUNC_CreateFireParticle(e)?;
                return pass_on(e, param_1, param_2, param_3);
            }
        };
        let sector = e.r32(e.r32(GDF(e) + 4) + 0x24);
        match table.iter().find(|(s, _)| *s == sector) {
            Some((_, ids)) if ids.contains(&iVar1) => fireGhostBaseFUNC_CreateFireParticle(e)?,
            _ => fireGhostBaseFUNC_DestroyFireParticle(e)?,
        }
    }
    pass_on(e, param_1, param_2, param_3)
}

fn pass_on(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    // `(long)param_2`: sign-extended, the base handler only compares it as a u32
    fireGhostBaseFUNC_MessageHandler(e, param_1, param_2, param_3)?;
    Ok(0)
}

/// 0x00018d08 (6f63c7c0) / 0x00018c58 (818e89e1) / 0x00018a00 (3a7b55ab): the base, the per-level
/// ranges (EP6L04A: attack 3 m, sight 2.667 m, group mask 0x400; EP6L04B: 4.5 m / 4 m) or type 0x12
/// (EP6L05: wait for a spawn position), our message handler, then Monster_Base_Func_Initialise.
pub fn fireghostConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    fireghost_baseConstructor(e, api, instanceCount)?;
    let l = L(e);
    match build_key(e) {
        "6f63c7c0" => {
            e.w32(l + 0x1e0, 0x4040_0000);
            for o in [0x1dc, 0x1d8, 0x1d4] {
                e.w32(l + o, 0x402a_aaab);
            }
            APIgameobj::SetGroupBitMask(e, SELF, 0x400);
        }
        "818e89e1" => {
            e.w32(l + 0x1e0, 0x4090_0000);
            for o in [0x1dc, 0x1d8, 0x1d4] {
                e.w32(l + o, 0x4080_0000);
            }
        }
        _ => e.w32(l + 0x44, 0x12),
    }
    APIcons::SetMessageFunction(e, fireGhostFUNC_MessageHandler);
    Monster_Base_Func_Initialise(e)
}

/// 0x00018df8 / 0x00018d20 / 0x00018a88
pub fn fireghostDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    fireghost_baseDestructor(e, instanceCount)
}

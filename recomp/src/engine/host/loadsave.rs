//! APILoadSave::EnableReset (EFApi_LoadSave.cpp 0x0021b6e0). The rest of APILoadSave is ported in
//! crate::engine::core::apiloadsave (WP-M1: EFLoadSaveMgr + file-backed memory card).
#![allow(non_snake_case)]

use crate::engine::Engine;

/// EnableReset 0x0021b6e0: gui32EnableReset = a0 (read by the reset handler only, which the port
/// does not have)
pub fn EnableReset(e: &mut Engine, a0: u32) -> u32 {
    e.host.APILoadSave_EnableReset(&mut e.mem, a0)
}

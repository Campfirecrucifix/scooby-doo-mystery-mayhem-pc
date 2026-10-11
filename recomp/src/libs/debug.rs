//! debug custom library (`Game Logic Modules/Globals/Lib/debug__*/debug.c`, builds eb95cfb4 / fc09b1a7,
//! identical code). Registers the 2-slot table "debug" (8 bytes): PrintDebugInfo,
//! PrintDebugInfoWithValue. Both only forward to `pExtra->null_printf`, which discards its output, so
//! the formatted varargs are not rebuilt here (only the format string + first argument are forwarded).
//!
//! Module globals: pExtra 0x10000, pGameObj 0x10004, pRegLib 0x10008, pDebug 0x1000c,
//! pLocalDataDebug 0x10010 (4 bytes, unused).
#![allow(non_snake_case, non_upper_case_globals, unused_variables)]
use super::{lib_data, set_global};
use crate::engine::core::apicons as cons;
use crate::engine::*;
use crate::logic::{LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "debug",
    data_size: 0x20,
    ctor: debugConstructor,
    dtor: debugDestructor,
    register: Some(debugRegister),
    init: &[],
    build_init: None,
};

const pExtra: u32 = 0x10000;
const pGameObj: u32 = 0x10004;
const pRegLib: u32 = 0x10008;
const pDebug: u32 = 0x1000c;
const pLocalDataDebug: u32 = 0x10010;

fn has_extra(e: &Engine) -> bool {
    e.r32(lib_data(e, &MODULE, pExtra)) != 0
}
fn printf(e: &mut Engine, fmt: &str, a: u32) {
    let f = e.cstr(fmt);
    APIextra::null_printf(e, f, a);
}

/// 0x00010dd0
pub fn PrintDebugInfo(e: &mut Engine, param_1: u32, param_2: u32) -> u32 {
    if has_extra(e) {
        printf(e, "%s> %s\n", param_1);
    }
    0
}

/// 0x00010e40 (kind 1 %u, 2 %f, 3 %s, 4 vector, 5 %u + 32-bit 'I'/'O' bit dump)
pub fn PrintDebugInfoWithValue(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr, param_4: u32) -> u32 {
    if has_extra(e) {
        match param_4 {
            1 => printf(e, "%s> %s\t-> %u\n", param_1),
            2 => printf(e, "%s> %s\t-> %f\n", param_1),
            3 => printf(e, "%s> %s\t-> %s\n", param_1),
            4 => printf(e, "%s> %s\t-> x: %8.6f  y: %8.6f  z: %8.6f\n", param_1),
            5 => {
                let v = e.r32(param_3);
                printf(e, "%s> %s\t-> %u\n", param_1);
                printf(e, "      3         2         1\n", 0);
                printf(e, "     10987654321098765432109876543210\n", 0);
                printf(e, "     - --------- --------- --------- \n", 0);
                // 32 chars, bit 31 first ('I' set, 'O' clear)
                let p = e.alloca(0x21);
                for i in 0..32u32 {
                    e.w8(p + i, if v & (1 << (31 - i)) != 0 { 0x49 } else { 0x4f });
                }
                e.w8(p + 32, 0);
                let f = e.cstr("     %s\n");
                APIextra::null_printf(e, f, p);
            }
            _ => {}
        }
    }
    0
}

/// 0x000110f8
pub fn debugRegister(e: &mut Engine, api: Ptr) {
    let v = cons::GetApiExtra(e);
    set_global(e, &MODULE, pExtra, v);
    let v = cons::GetApiGameObj(e);
    set_global(e, &MODULE, pGameObj, v);
    let n = e.cstr("debug");
    let v = cons::GetApiCustom(e, n);
    set_global(e, &MODULE, pDebug, v);
    let v = cons::GetApiRegLib(e);
    set_global(e, &MODULE, pRegLib, v);
    let v = APIRegLib::RegisterLib(e, n, 8);
    set_global(e, &MODULE, pDebug, v);
    // the 2 slot writes: LIBdebug dispatches straight to the functions above
}

/// 0x000111e8
pub fn debugConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    let p = lib_data(e, &MODULE, pLocalDataDebug);
    cons::GetLocalData(e, p, 4);
    Ok(())
}

/// 0x00011248
pub fn debugDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    let n = e.cstr("debug");
    APIRegLib::UnRegisterLib(e, n);
    Ok(())
}

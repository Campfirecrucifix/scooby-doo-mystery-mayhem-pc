//! SndMngr custom library (`Game Logic Modules/Globals/Lib/sndmngr__*/SndMngr.c`, builds 35c466fc /
//! fa3adad0, identical code). Registers the 1-slot table "SndMngr": PlaySound plays a sound on the
//! level object 0x2a000000 and remembers the last id in the library's local data.
//!
//! Module globals: pExtra 0x10000, pGameObj 0x10004, pRegLib 0x10008, pSound 0x1000c, pStdlib 0x10010,
//! pSndMngr 0x10014, pLocalDataSndMngr 0x10018 (-> 0xa4-byte local data, +0 = last sound).
#![allow(non_snake_case, non_upper_case_globals, unused_variables)]
use super::{lib_data, set_global};
use crate::engine::core::apicons as cons;
use crate::engine::*;
use crate::logic::{LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "sndmngr",
    data_size: 0x20,
    ctor: sndmngrConstructor,
    dtor: sndmngrDestructor,
    register: Some(sndmngrRegister),
    init: &[],
    build_init: None,
};

const pExtra: u32 = 0x10000;
const pGameObj: u32 = 0x10004;
const pRegLib: u32 = 0x10008;
const pSound: u32 = 0x1000c;
const pStdlib: u32 = 0x10010;
const pSndMngr: u32 = 0x10014;
const pLocalDataSndMngr: u32 = 0x10018;

/// 0x00010cd8
pub fn PlaySound(e: &mut Engine, param_1: i32) -> i32 {
    if param_1 == -1 {
        return -1;
    }
    APInewSound::Play(e, 0x2a000000, param_1 as u32, 0);
    let l = e.r32(lib_data(e, &MODULE, pLocalDataSndMngr));
    e.wi32(l, param_1);
    param_1
}

/// 0x00010d68
pub fn sndmngrRegister(e: &mut Engine, api: Ptr) {
    let v = cons::GetApiExtra(e);
    set_global(e, &MODULE, pExtra, v);
    let v = cons::GetApiGameObj(e);
    set_global(e, &MODULE, pGameObj, v);
    let v = cons::GetApiNewSound(e);
    set_global(e, &MODULE, pSound, v);
    let v = cons::GetApiStdlib(e);
    set_global(e, &MODULE, pStdlib, v);
    let n = e.cstr("SndMngr");
    let v = cons::GetApiCustom(e, n);
    set_global(e, &MODULE, pSndMngr, v);
    let v = cons::GetApiRegLib(e);
    set_global(e, &MODULE, pRegLib, v);
    let v = APIRegLib::RegisterLib(e, n, 4);
    set_global(e, &MODULE, pSndMngr, v);
    // the slot write: LIBSndMngr dispatches straight to PlaySound
}

/// 0x00010e70
pub fn sndmngrConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    let p = lib_data(e, &MODULE, pLocalDataSndMngr);
    cons::GetLocalData(e, p, 0xa4);
    Ok(())
}

/// 0x00010ed0
pub fn sndmngrDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    let n = e.cstr("SndMngr");
    APIRegLib::UnRegisterLib(e, n);
    Ok(())
}

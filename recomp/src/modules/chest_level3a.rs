//! `chest_level3a`: EP1L03A costume chest. Same chest_base.c as `chest_level1` (shared code there,
//! per-level object/sound ids selected by module name).
//! Port of `Game Logic Modules/Ep1_Library/Objects/CHEST/LEVEL_3A/chest_level3a__4a24dfa3/`.
#![allow(non_snake_case)]

use super::chest_level1 as base;
use crate::engine::*;
use crate::logic::{LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "chest_level3a",
    data_size: 0x2220,
    ctor: ctor_dispatch,
    dtor: dtor_dispatch,
    register: None,
    init: &[],
    build_init: Some(build_init_dispatch),
};

mod xb;

/// number of hand-ported builds (indices 0..N_OLD); generated builds (`xb`, c2rs) follow
pub(crate) const N_OLD: usize = 1;

fn ctor_dispatch(e: &mut Engine, api: Ptr, n: i32) -> LResult<()> {
    if e.rt.cur_build() < N_OLD { chest_level3aConstructor(e, api, n) } else { xb::ctor(e, api, n) }
}

fn dtor_dispatch(e: &mut Engine, n: i32) -> LResult<()> {
    if e.rt.cur_build() < N_OLD { chest_level3aDestructor(e, n) } else { xb::dtor(e, n) }
}

fn build_init_dispatch(m: &mut Mem, data: Ptr, b: usize) {
    if b < N_OLD {
    } else {
        xb::build_init(m, data, b - N_OLD);
    }
}

pub fn build_index(key: &str) -> Option<usize> {
    if let Some(i) = build_index_old(key) {
        return Some(i);
    }
    xb::build_index(key).map(|i| i + N_OLD)
}

fn build_index_old(key: &str) -> Option<usize> {
    (key == "4a24dfa3").then_some(0)
}

pub fn chest_level3a_FUNC_MessageFunction(e: &mut Engine, m: u32, p: u32, d: Ptr) -> LResult<u64> {
    base::ChestBaseCB_MessageFunction(e, m, p, d)
}

pub fn chest_level3aConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    base::chest_baseConstructor(e, api, instanceCount);
    APIcons::SetMessageFunction(e, chest_level3a_FUNC_MessageFunction);
    base::ChestBaseFUNC_CreateGameObject(e);
    APIcons::SetInitialState(e, 0);
    Ok(())
}

pub fn chest_level3aDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    base::chest_baseDestructor(e, instanceCount);
    Ok(())
}

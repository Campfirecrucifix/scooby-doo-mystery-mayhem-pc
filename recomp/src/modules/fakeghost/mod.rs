//! `fakeghost` (EP1, 2 builds with unrelated code): fbe91ac1 (L04) and 4da56182 (L09, [`l09`]).
#![allow(non_snake_case, non_upper_case_globals, unused_variables, dead_code)]

mod l04;
mod l09;

use super::modkit::{custom, ext, gset, G};
use crate::engine::*;
use crate::logic::{LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "fakeghost",
    data_size: 0x78,
    ctor: fakeghostConstructor,
    dtor: fakeghostDestructor,
    register: None,
    init: &[],
    build_init: None,
};

const BUILDS: [&str; 2] = ["fbe91ac1", "4da56182"];
pub fn build_index(key: &str) -> Option<usize> {
    BUILDS.iter().position(|b| *b == key)
}

pub fn fakeghostConstructor(e: &mut Engine, api: Ptr, n: i32) -> LResult<()> {
    if e.rt.cur_build() == 0 { l04::ctor(e, api, n) } else { l09::ctor(e, api, n) }
}
pub fn fakeghostDestructor(e: &mut Engine, n: i32) -> LResult<()> {
    if e.rt.cur_build() == 0 { l04::dtor(e, n) } else { l09::dtor(e, n) }
}

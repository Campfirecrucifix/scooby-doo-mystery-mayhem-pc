//! `hs_door_l03`: EP1L03A door hot spots (11 doors). Same hs_door_base.c as hs_door_l01 (shared
//! code there, variant-selected by module name); local part only forwards collision ext fn 4.
//! Port of `Game Logic Modules/Ep1_Library/Objects/HOTSPOTS/LEVEL_03A/hs_door_l03__a80dc326/`.
#![allow(non_snake_case, non_upper_case_globals, unused_variables)]

use super::hs_door_l01 as base;
use crate::engine::*;
use crate::logic::{LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "hs_door_l03",
    // globals 0x10000..0x158a8 (+ unused gpoLanguagesNamesAndISOCodes)
    data_size: 0x58a8,
    ctor: hs_door_l03Constructor,
    dtor: hs_door_l03Destructor,
    register: None,
    init: &[],
    build_init: Some(build_init),
};

pub fn build_index(key: &str) -> Option<usize> {
    (key == "a80dc326").then_some(0)
}

fn build_init(m: &mut Mem, data: Ptr, _: usize) {
    base::build_init_tab(m, data, WORDS, STR_RELOCS);
}

pub fn fcback_COLLISION_LOCAL(e: &mut Engine, p: u32) -> LResult<u64> {
    base::hs_door_baseCB_CollisionDetection(e, p)
}

pub fn hs_door_l03Constructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    base::hs_door_baseConstructor(e, api, instanceCount);
    if instanceCount == 1 {
        APIcons::SetExternalFunction(e, 4, fcback_COLLISION_LOCAL);
    }
    APIcons::SetInitialState(e, 0);
    Ok(())
}

pub fn hs_door_l03Destructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    base::hs_door_baseDestructor(e);
    Ok(())
}

const WORDS: &[(u32, u32)] = &[(0x10000, 0x2a000016), (0x10004, 0x2a000007), (0x10008, 0x40000002), (0x10804, 0x2a000017), (0x10808, 0x2a000006), (0x1080c, 0x40000002), (0x11008, 0x2a000018), (0x1100c, 0x2a000009), (0x11010, 0x40000002), (0x1180c, 0x2a000019), (0x11810, 0x2a000008), (0x11814, 0x40000002), (0x12010, 0x2a00001a), (0x12014, 0x2a000010), (0x12018, 0x40000002), (0x12814, 0x2a000020), (0x12818, 0x2a00000e), (0x1281c, 0x40000002), (0x13018, 0x2a00001b), (0x1301c, 0x2a000013), (0x13020, 0x40000002), (0x1381c, 0x2a00001d), (0x13820, 0x2a00000a), (0x13824, 0x4000000a), (0x13828, 0x4000), (0x1382c, 0x4), (0x14020, 0x2a00001e), (0x14024, 0x2a000012), (0x14028, 0x4000000a), (0x14824, 0x2a00001f), (0x14828, 0x2a000011), (0x1482c, 0x4000000a), (0x15028, 0x2a000015), (0x15034, 0x2000), (0x15038, 0x8)];
const STR_RELOCS: &[(u32, &str)] = &[(0x1001c, "NULL"), (0x10020, "NULL"), (0x10028, ""), (0x10820, "NULL"), (0x10824, "NULL"), (0x1082c, ""), (0x11024, "NULL"), (0x11028, "NULL"), (0x11030, ""), (0x11828, "NULL"), (0x1182c, "NULL"), (0x11834, ""), (0x1202c, "NULL"), (0x12030, "NULL"), (0x12038, ""), (0x12830, "NULL"), (0x12834, "NULL"), (0x1283c, ""), (0x13034, "NULL"), (0x13038, "NULL"), (0x13040, ""), (0x13838, "NULL"), (0x1383c, "NULL"), (0x13844, ""), (0x1403c, "NULL"), (0x14040, "NULL"), (0x14048, ""), (0x14840, "NULL"), (0x14844, "NULL"), (0x1484c, ""), (0x15044, "EP1\\L03B"), (0x15048, "EP1L03B"), (0x15050, "")];

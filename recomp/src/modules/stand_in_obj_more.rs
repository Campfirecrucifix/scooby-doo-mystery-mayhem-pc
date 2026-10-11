//! `stand_in_obj_animated`, `stand_in_obj_cutscene`, `stand_in_obj_physic`: static stand-in props
//! (variants of `stand_in_obj`): first frame rewrites the clipping params (distance -1.0, two flag
//! words), optionally plays anim 1 / creates physics, then sleeps forever. The constructor calls
//! MiscFunc slot 1 ChangeClipping(SELF, 2, NULL, 1). Ports of `Globals/Objects/Stand_In_Obj_*` (3 fns each).
#![allow(non_snake_case, non_upper_case_globals, unused_variables)]

use super::modkit::*;
use crate::engine::*;
use crate::logic::{LResult, ModuleDef};

macro_rules! def {
    ($id:ident, $name:literal, $ctor:ident) => {
        pub static $id: ModuleDef = ModuleDef {
            name: $name,
            data_size: 0x10,
            ctor: $ctor,
            dtor: dtor,
            register: None,
            init: &[],
            build_init: None,
        };
    };
}
def!(MODULE, "stand_in_obj_animated", stand_in_obj_animatedConstructor);
def!(MODULE_CUTSCENE, "stand_in_obj_cutscene", stand_in_obj_cutsceneConstructor);
def!(MODULE_PHYSIC, "stand_in_obj_physic", stand_in_obj_physicConstructor);

/// GetClippingParams; +4 = -1.0, +0xc = c, +8 = d; SetClippingParams
fn clip(e: &mut Engine, d: u32, c: u32) {
    let p = e.alloca(0x44);
    APIgameobj::GetClippingParams(e, SELF, p);
    e.w32(p + 4, 0xbf80_0000);
    e.w32(p + 0xc, c);
    e.w32(p + 8, d);
    APIgameobj::SetClippingParams(e, SELF, p, 0);
}

/// stand_in_obj_animated null_main
pub fn animated_main(e: &mut Engine, _: u32) -> LResult<u64> {
    APIanim::Play(e, SELF, 1, 2, 0.0);
    clip(e, 0x6a7, 0x680);
    APIgameobj::Sleep(e, SELF, -1.0);
    Ok(0)
}
/// stand_in_obj_cutscene null_main
pub fn cutscene_main(e: &mut Engine, _: u32) -> LResult<u64> {
    clip(e, 0x698, 0x698);
    APIgameobj::Sleep(e, SELF, -1.0);
    Ok(0)
}
/// stand_in_obj_physic null_main
pub fn physic_main(e: &mut Engine, _: u32) -> LResult<u64> {
    clip(e, 0x63f, 0x638);
    APIphysic::CreatePhysic(e, SELF, 0);
    APIgameobj::Sleep(e, SELF, -1.0);
    Ok(0)
}

// globals: animated {pGameObj 0, pAnim 4, pMiscFunc 8}; cutscene {pMiscFunc 0, pGameObj 4};
// physic {pMiscFunc 0, pPhysic 4, pGameObj 8}
pub fn stand_in_obj_animatedConstructor(e: &mut Engine, api: Ptr, n: i32) -> LResult<()> {
    if n == 1 {
        ext(e, &[(0, animated_main)]);
        let v = APIcons::GetApiAnim(e); gset(e, 0x10004, v);
        let v = APIcons::GetApiGameObj(e); gset(e, 0x10000, v);
        custom(e, 0x10008, "MiscFunc");
    }
    finish(e)
}
pub fn stand_in_obj_cutsceneConstructor(e: &mut Engine, api: Ptr, n: i32) -> LResult<()> {
    if n == 1 {
        ext(e, &[(0, cutscene_main)]);
        let v = APIcons::GetApiGameObj(e); gset(e, 0x10004, v);
        custom(e, 0x10000, "MiscFunc");
    }
    finish(e)
}
pub fn stand_in_obj_physicConstructor(e: &mut Engine, api: Ptr, n: i32) -> LResult<()> {
    if n == 1 {
        ext(e, &[(0, physic_main)]);
        let v = APIcons::GetApiGameObj(e); gset(e, 0x10008, v);
        custom(e, 0x10000, "MiscFunc");
        let v = APIcons::GetApiPhysic(e); gset(e, 0x10004, v);
    }
    finish(e)
}
fn finish(e: &mut Engine) -> LResult<()> {
    LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 2, NULL, 1);
    APIcons::SetInitialState(e, 0);
    Ok(())
}
pub fn dtor(e: &mut Engine, n: i32) -> LResult<()> {
    Ok(())
}

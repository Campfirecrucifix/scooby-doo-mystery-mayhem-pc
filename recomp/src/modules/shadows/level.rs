//! The per-level source files of each shadows build (`ShadowsEpisodeOne.c` + `ShadowsLevel<N>.c`):
//! the module entry points, the level's message function and init. The port has one entry pair
//! ([`shadows_levelConstructor`] / [`shadows_levelDestructor`]) that dispatches on the loaded build.
//! Addresses are module-relative (.text at 0x10000) of the build named.
use super::*;
use crate::engine::*;
use crate::logic::{LResult, MsgFn};

/// The module entry point (`<level>Constructor` of the loaded build).
pub fn shadows_levelConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    match build_key(e) {
        "shadowslevelone" => shadowsleveloneConstructor(e, api, instanceCount),
        "shadowsleveltwo" => shadowsleveltwoConstructor(e, api, instanceCount),
        "shadowslevelthreeold" => shadowslevelthreeoldConstructor(e, api, instanceCount),
        "shadowslevelthree" => shadowslevelthreeConstructor(e, api, instanceCount),
        k => panic!("shadows: no level constructor for build {k}"),
    }
}

/// The module exit point (`<level>Destructor` of the loaded build).
pub fn shadows_levelDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    match build_key(e) {
        "shadowslevelone" => shadowsleveloneDestructor(e, instanceCount),
        "shadowsleveltwo" => shadowsleveltwoDestructor(e, instanceCount),
        "shadowslevelthreeold" => shadowslevelthreeoldDestructor(e, instanceCount),
        "shadowslevelthree" => shadowslevelthreeDestructor(e, instanceCount),
        k => panic!("shadows: no level destructor for build {k}"),
    }
}

// ---- ShadowsEpisodeOne.c ----

/// 0x00017170 (shadowslevelone)
pub fn shadowsepisodeoneConstructor(e: &mut Engine, api: Ptr, param_2: i32) -> LResult<()> {
    shadows_baseConstructor(e, api, param_2)
}

/// 0x000171c0 (shadowslevelone)
pub fn shadowsepisodeoneDestructor(e: &mut Engine) -> LResult<()> {
    shadows_baseDestructor(e)
}

// ---- level files ----

/// `episode constructor, message function, level init, Monster_Base_Func_Initialise,
/// Shadow_Base_Func_Initialisation, initial state 0xe` (every level constructor)
fn level_ctor(e: &mut Engine, api: Ptr, instanceCount: i32, msg: MsgFn, init: fn(&mut Engine) -> LResult<()>) -> LResult<()> {
    shadowsepisodeoneConstructor(e, api, instanceCount)?;
    APIcons::SetMessageFunction(e, msg);
    init(e)?;
    Monster_Base_Func_Initialise(e)?;
    Shadow_Base_Func_Initialisation(e)?;
    APIcons::SetInitialState(e, 0xe);
    Ok(())
}

/// `L+0x44 = 4` (monster type), `SetPriority(SELF, 10)`
fn init_type_4(e: &mut Engine) {
    let l = LM(e);
    e.w32(l + 0x44, 4);
}

/// 0x00017218 (shadowslevelone) L+0x34 (state to resume after a pause) = 9999 (none); the shadow
/// may not cross floors with attribute 8 until message 199
pub fn ShadowsLevelOne_Func_Initialisation(e: &mut Engine) -> LResult<()> {
    init_type_4(e);
    let l = L(e);
    e.w32(l + 0x34, 9999);
    APIphysic::PrimSetIntAttribBits(e, SELF, 0x28b, 8, 8);
    APIphysic::SetPriority(e, SELF, 10);
    Ok(())
}

/// 0x000172a0 (shadowslevelone) 0x57 pause (physics flag 1, state saved in L+0x34, state 5), 0x58
/// resume, 199 (tell 0x2a000020 with 0x104, floor attribute 8 allowed)
pub fn ShadowsLevelOne_MessageFunction(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    if param_1 == 0x58 {
        APIphysic::ResetFlags(e, SELF, 1);
        let s = e.r32(L(e) + 0x34);
        if s != 9999 {
            APIgameobj::SetNextState(e, SELF, s);
        }
    } else if param_1 < 0x59 {
        if param_1 == 0x57 {
            APIphysic::SetFlags(e, SELF, 1);
            if APIgameobj::GetCurrentStateID(e, SELF) != 5 {
                let s = APIgameobj::GetCurrentStateID(e, SELF);
                let l = L(e);
                e.w32(l + 0x34, s);
            }
            APIgameobj::SetNextState(e, SELF, 5);
        }
    } else if param_1 == 199 {
        APIgameobj::SendMessageTo(e, 0x2a00_0020, NULL, 0x104, 0, NULL);
        APIphysic::PrimResetIntAttribBits(e, SELF, 0x28b, 8, 8);
    }
    Shadows_BaseFUNC_MessageHandler(e, param_1, param_2, param_3)
}

/// 0x00017498 (shadowslevelone)
pub fn shadowsleveloneConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    level_ctor(e, api, instanceCount, ShadowsLevelOne_MessageFunction, ShadowsLevelOne_Func_Initialisation)
}
/// 0x00017558 (shadowslevelone)
pub fn shadowsleveloneDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    shadowsepisodeoneDestructor(e)
}

macro_rules! simple_level {
    ($($init:ident $init_a:literal, $msg:ident $msg_a:literal, $ctor:ident $ctor_a:literal, $dtor:ident $dtor_a:literal, $b:literal;)*) => {$(
        #[doc = concat!($init_a, " (", $b, ")")]
        pub fn $init(e: &mut Engine) -> LResult<()> {
            init_type_4(e);
            APIphysic::SetPriority(e, SELF, 10);
            Ok(())
        }
        #[doc = concat!($msg_a, " (", $b, ")")]
        pub fn $msg(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
            Shadows_BaseFUNC_MessageHandler(e, param_1, param_2, param_3)
        }
        #[doc = concat!($ctor_a, " (", $b, ")")]
        pub fn $ctor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
            level_ctor(e, api, instanceCount, $msg, $init)
        }
        #[doc = concat!($dtor_a, " (", $b, ")")]
        pub fn $dtor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
            shadowsepisodeoneDestructor(e)
        }
    )*};
}
simple_level! {
    ShadowsLevelTwo_Func_Initialisation "0x00017288", ShadowsLevelTwo_MessageFunction "0x000172e0",
        shadowsleveltwoConstructor "0x00017358", shadowsleveltwoDestructor "0x00017418", "shadowsleveltwo";
    ShadowsLevelThreeOld_Func_Initialisation "0x000172d8", ShadowsLevelThreeOld_MessageFunction "0x00017330",
        shadowslevelthreeoldConstructor "0x000173a8", shadowslevelthreeoldDestructor "0x00017468", "shadowslevelthreeold";
    ShadowsLevelThree_Func_Initialisation "0x000172b0", ShadowsLevelThree_MessageFunction "0x00017308",
        shadowslevelthreeConstructor "0x00017380", shadowslevelthreeDestructor "0x00017440", "shadowslevelthree";
}

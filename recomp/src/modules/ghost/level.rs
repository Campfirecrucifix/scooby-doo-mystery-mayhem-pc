//! The per-level source files of each ghost build (`<Size>Ghost<Episode>.c` + `<Size>GhostLevel<N>.c`,
//! `GhostBossLevel5.c`): the module entry points and the level's message function, init and extra
//! states / callbacks. The port has one entry pair ([`ghost_levelConstructor`] /
//! [`ghost_levelDestructor`]) that dispatches on the loaded build.
//!
//! Functions whose name is used by several builds with different code carry the build key as a
//! suffix (`_757255de`, `_medghostlevelthreeold`, ...); functions with the same code in several builds
//! are written once. Addresses are module-relative (.text at 0x10000) of the build named.
//!
//! The episode files (`smallghostepisodeoneConstructor`, ...) only forward to
//! `ghost_baseConstructor` / `ghost_baseDestructor` in every build.
use super::*;
use crate::engine::*;
use crate::logic::{CtorFn, ExtFn, LResult, MsgFn};

#[inline]
fn lv(e: &Engine, off: u32) -> u32 {
    e.r32(L(e) + off)
}
#[inline]
fn set_lv(e: &mut Engine, off: u32, v: u32) {
    let l = L(e);
    e.w32(l + off, v);
}
/// `TimerPause` of the four AI timers (+0x160, +0x164, +0x168, +0x170) [and the looking-around one]
fn pause_timers(e: &mut Engine, on: u32, with_878: bool) {
    for o in [0x160, 0x164, 0x168, 0x170] {
        let t = lv(e, o);
        APIextra::TimerPause(e, t, on);
    }
    if with_878 {
        let t = lv(e, 0x878);
        APIextra::TimerPause(e, t, on);
    }
}
/// `*(EFClippingParams **)(pLocalDataGhost_Base + 0x85c)`: never set by the ghost code, so NULL;
/// the original saves and restores the clipping params through PS2 address 0 (see
/// [`Engine::zero_page`]).
fn clip_save(e: &Engine) -> Ptr {
    match lv(e, 0x85c) {
        NULL => e.zero_page,
        p => p,
    }
}
/// `if (L+0x34 == 0x13) L+0x19c |= 0x800;` (cut scene ended during an attack: the attack is over)
fn end_attack_if_attacking(e: &mut Engine) {
    if lv(e, 0x34) == 0x13 {
        let a = L(e) + 0x19c;
        or32(e, a, 0x800);
    }
}
/// `L+0x34 = GetCurrentStateID(SELF)` unless it is 5 (two calls, as in the original)
fn save_state_unless_5(e: &mut Engine) {
    if APIgameobj::GetCurrentStateID(e, SELF) != 5 {
        let s = APIgameobj::GetCurrentStateID(e, SELF);
        set_lv(e, 0x34, s);
    }
}
/// cut scene start for the ghost: clipping saved, cut-scene mode, state 5
fn cut_scene_in(e: &mut Engine) {
    let c = clip_save(e);
    APIgameobj::GetClippingParams(e, SELF, c);
    let id = APIgameobj::GetID(e, SELF, NULL);
    LIBMiscFunc::MiscFunc_InitForCutscene(e, id);
    APIgameobj::SetNextState(e, SELF, 5);
}
/// cut scene end: clipping restored
fn cut_scene_out(e: &mut Engine) {
    let id = APIgameobj::GetID(e, SELF, NULL);
    let c = clip_save(e);
    LIBMiscFunc::MiscFunc_EndCutscene(e, id, c);
}
/// the text box parameters (`undefined4 auStack[16]`) with the display time in word 0
fn show_text(e: &mut Engine, text: u32, time: u32) {
    let b = e.alloca(0x40);
    LIBApiText::InitStruct(e, b);
    e.w32(b, time);
    LIBApiText::ShowText(e, text, b);
}
fn translate_home(e: &mut Engine) {
    let p = L(e) + 0xf8;
    APIgameobj::SetTranslation(e, SELF, p);
}

/// The body shared by the level constructors: episode constructor, [extra external functions],
/// message function, level init, the monster and ghost inits, the glow, [`after`], initial state 0xe.
fn level_ctor(
    e: &mut Engine,
    api: Ptr,
    instanceCount: i32,
    episode: CtorFn,
    ext: &[(i32, ExtFn)],
    msg: MsgFn,
    init: fn(&mut Engine) -> LResult<()>,
    after: Option<fn(&mut Engine) -> LResult<()>>,
) -> LResult<()> {
    episode(e, api, instanceCount)?;
    if instanceCount == 1 {
        for &(i, f) in ext {
            APIcons::SetExternalFunction(e, i, f);
        }
    }
    APIcons::SetMessageFunction(e, msg);
    init(e)?;
    Monster_Base_Func_Initialise(e)?;
    Ghost_Base_Func_Initialisation(e)?;
    CreateGlow(e)?;
    if let Some(f) = after {
        f(e)?;
    }
    APIcons::SetInitialState(e, 0xe);
    Ok(())
}

/// `pText = (*api->GetApiCustom)("ApiText");`
fn get_api_text(e: &mut Engine) {
    let n = e.cstr("ApiText");
    let v = APIcons::GetApiCustom(e, n);
    let a = G(e, pText);
    e.w32(a, v);
}

// ---- entry points ----

/// The module entry point (`<level>Constructor` of the loaded build).
pub fn ghost_levelConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    let n = instanceCount;
    match build_key(e) {
        "757255de" => smallghostleveloneConstructor_757255de(e, api, n),
        "bigghostlevelfour" => bigghostlevelfourConstructor(e, api, n),
        "01950f3a" => bigghostlevelnineConstructor_01950f3a(e, api, n),
        "1a23fd5a" => medghostleveloneConstructor_1a23fd5a(e, api, n),
        "medghostleveltwo" => medghostleveltwoConstructor(e, api, n),
        "medghostlevelthreeold" => medghostlevelthreeoldConstructor(e, api, n),
        "medghostlevelthree" => medghostlevelthreeConstructor(e, api, n),
        "medghostlevelfour" => medghostlevelfourConstructor(e, api, n),
        "de27b0f0" => medghostlevelnineConstructor_de27b0f0(e, api, n),
        "19a6f2fc" => smallghostleveltwoConstructor_19a6f2fc(e, api, n),
        "smallghostlevelthreeold" => smallghostlevelthreeoldConstructor(e, api, n),
        "smallghostlevelthree" => smallghostlevelthreeConstructor(e, api, n),
        "4324ff1c" => smallghostlevelfourConstructor_4324ff1c(e, api, n),
        "d0275f8b" => smallghostlevelnineConstructor_d0275f8b(e, api, n),
        "bigghostlevelonea" => bigghostleveloneaConstructor(e, api, n),
        "74e95992" => bigghostlevelnineConstructor_74e95992(e, api, n),
        "bigghostlevelonec" => bigghostlevelonecConstructor(e, api, n),
        "7f4484ea" => medghostlevelnineConstructor_7f4484ea(e, api, n),
        "medghostlevelonea" => medghostleveloneaConstructor(e, api, n),
        "9a257725" => smallghostlevelnineConstructor_9a257725(e, api, n),
        "smallghostlevelonea" => smallghostleveloneaConstructor(e, api, n),
        "f2015fc2" => bigghostleveloneConstructor_f2015fc2(e, api, n),
        "69625cbf" => smallghostleveltwoConstructor_69625cbf(e, api, n),
        "2e46e079" => smallghostlevelfourConstructor_2e46e079(e, api, n),
        "97a3a638" => bigghostleveloneConstructor_97a3a638(e, api, n),
        "92ff0346" => medghostleveloneConstructor_92ff0346(e, api, n),
        "056ca924" => smallghostleveloneConstructor_056ca924(e, api, n),
        "ghostbosslevel5" => ghostbosslevel5Constructor(e, api, n),
        "medghostlevel4a" => medghostlevel4aConstructor(e, api, n),
        "medghostlevel4b" => medghostlevel4bConstructor(e, api, n),
        "medghostlevel5" => medghostlevel5Constructor(e, api, n),
        k => panic!("ghost: no level constructor for build {k}"),
    }
}

/// The module exit point (`<level>Destructor` of the loaded build).
pub fn ghost_levelDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    let n = instanceCount;
    match build_key(e) {
        "757255de" => smallghostleveloneDestructor_757255de(e, n),
        "bigghostlevelfour" => bigghostlevelfourDestructor(e, n),
        "01950f3a" => bigghostlevelnineDestructor_01950f3a(e, n),
        "1a23fd5a" => medghostleveloneDestructor_1a23fd5a(e, n),
        "medghostleveltwo" => medghostleveltwoDestructor(e, n),
        "medghostlevelthreeold" => medghostlevelthreeoldDestructor(e, n),
        "medghostlevelthree" => medghostlevelthreeDestructor(e, n),
        "medghostlevelfour" => medghostlevelfourDestructor(e, n),
        "de27b0f0" => medghostlevelnineDestructor_de27b0f0(e, n),
        "19a6f2fc" => smallghostleveltwoDestructor_19a6f2fc(e, n),
        "smallghostlevelthreeold" => smallghostlevelthreeoldDestructor(e, n),
        "smallghostlevelthree" => smallghostlevelthreeDestructor(e, n),
        "4324ff1c" => smallghostlevelfourDestructor_4324ff1c(e, n),
        "d0275f8b" => smallghostlevelnineDestructor_d0275f8b(e, n),
        "bigghostlevelonea" => bigghostleveloneaDestructor(e, n),
        "74e95992" => bigghostlevelnineDestructor_74e95992(e, n),
        "bigghostlevelonec" => bigghostlevelonecDestructor(e, n),
        "7f4484ea" => medghostlevelnineDestructor_7f4484ea(e, n),
        "medghostlevelonea" => medghostleveloneaDestructor(e, n),
        "9a257725" => smallghostlevelnineDestructor_9a257725(e, n),
        "smallghostlevelonea" => smallghostleveloneaDestructor(e, n),
        "f2015fc2" => bigghostleveloneDestructor_f2015fc2(e, n),
        "69625cbf" => smallghostleveltwoDestructor_69625cbf(e, n),
        "2e46e079" => smallghostlevelfourDestructor_2e46e079(e, n),
        "97a3a638" => bigghostleveloneDestructor_97a3a638(e, n),
        "92ff0346" => medghostleveloneDestructor_92ff0346(e, n),
        "056ca924" => smallghostleveloneDestructor_056ca924(e, n),
        "ghostbosslevel5" => ghostbosslevel5Destructor(e, n),
        "medghostlevel4a" => medghostlevel4aDestructor(e, n),
        "medghostlevel4b" => medghostlevel4bDestructor(e, n),
        "medghostlevel5" => medghostlevel5Destructor(e, n),
        k => panic!("ghost: no level destructor for build {k}"),
    }
}

// ---- episode files: forwarders ----

macro_rules! episode {
    ($($c:ident $d:ident;)*) => {$(
        pub fn $c(e: &mut Engine, api: Ptr, param_2: i32) -> LResult<()> {
            ghost_baseConstructor(e, api, param_2)
        }
        pub fn $d(e: &mut Engine) -> LResult<()> {
            ghost_baseDestructor(e)
        }
    )*};
}
episode! {
    smallghostepisodeoneConstructor smallghostepisodeoneDestructor;
    medghostepisodeoneConstructor medghostepisodeoneDestructor;
    bigghostepisodeoneConstructor bigghostepisodeoneDestructor;
    smallghostepisodetwoConstructor smallghostepisodetwoDestructor;
    medghostepisodetwoConstructor medghostepisodetwoDestructor;
    bigghostepisodetwoConstructor bigghostepisodetwoDestructor;
    smallghostepisodethreeConstructor smallghostepisodethreeDestructor;
    bigghostepisodethreeConstructor bigghostepisodethreeDestructor;
    smallghostepisodefourConstructor smallghostepisodefourDestructor;
    medghostepisodefourConstructor medghostepisodefourDestructor;
    bigghostepisodefourConstructor bigghostepisodefourDestructor;
    medghostepisodesixConstructor medghostepisodesixDestructor;
}

// level destructors: `<level>Destructor(int) { <episode>Destructor(); }` in every build
macro_rules! level_dtor {
    ($($d:ident => $ep:ident;)*) => {$(
        pub fn $d(e: &mut Engine, instanceCount: i32) -> LResult<()> {
            $ep(e)
        }
    )*};
}
level_dtor! {
    smallghostleveloneDestructor_757255de => smallghostepisodeoneDestructor;
    bigghostlevelfourDestructor => bigghostepisodeoneDestructor;
    bigghostlevelnineDestructor_01950f3a => bigghostepisodeoneDestructor;
    medghostleveloneDestructor_1a23fd5a => medghostepisodeoneDestructor;
    medghostleveltwoDestructor => medghostepisodeoneDestructor;
    medghostlevelthreeoldDestructor => medghostepisodeoneDestructor;
    medghostlevelthreeDestructor => medghostepisodeoneDestructor;
    medghostlevelfourDestructor => medghostepisodeoneDestructor;
    medghostlevelnineDestructor_de27b0f0 => medghostepisodeoneDestructor;
    smallghostleveltwoDestructor_19a6f2fc => smallghostepisodeoneDestructor;
    smallghostlevelthreeoldDestructor => smallghostepisodeoneDestructor;
    smallghostlevelthreeDestructor => smallghostepisodeoneDestructor;
    smallghostlevelfourDestructor_4324ff1c => smallghostepisodeoneDestructor;
    smallghostlevelnineDestructor_d0275f8b => smallghostepisodeoneDestructor;
    bigghostleveloneaDestructor => bigghostepisodetwoDestructor;
    bigghostlevelnineDestructor_74e95992 => bigghostepisodetwoDestructor;
    bigghostlevelonecDestructor => bigghostepisodetwoDestructor;
    medghostlevelnineDestructor_7f4484ea => medghostepisodetwoDestructor;
    medghostleveloneaDestructor => medghostepisodetwoDestructor;
    smallghostlevelnineDestructor_9a257725 => smallghostepisodetwoDestructor;
    smallghostleveloneaDestructor => smallghostepisodetwoDestructor;
    bigghostleveloneDestructor_f2015fc2 => bigghostepisodethreeDestructor;
    smallghostleveltwoDestructor_69625cbf => smallghostepisodethreeDestructor;
    smallghostlevelfourDestructor_2e46e079 => smallghostepisodethreeDestructor;
    bigghostleveloneDestructor_97a3a638 => bigghostepisodefourDestructor;
    medghostleveloneDestructor_92ff0346 => medghostepisodefourDestructor;
    smallghostleveloneDestructor_056ca924 => smallghostepisodefourDestructor;
    medghostlevel4aDestructor => medghostepisodesixDestructor;
    medghostlevel4bDestructor => medghostepisodesixDestructor;
    medghostlevel5Destructor => medghostepisodesixDestructor;
}

/// 0x0001e780 (ghostbosslevel5): no episode file
pub fn ghostbosslevel5Destructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    ghost_baseDestructor(e)
}

// ---- level inits: the monster type (L+0x44, see Monster_Base_Func_Initialise / the glow) ----

macro_rules! level_init {
    ($($f:ident = $t:expr, $prim:expr;)*) => {$(
        /// `L+0x44 = type` [+ `PrimSetIntAttribBits(SELF, 600, 8, 8)`]
        pub fn $f(e: &mut Engine) -> LResult<()> {
            set_lv(e, 0x44, $t);
            if $prim {
                APIphysic::PrimSetIntAttribBits(e, SELF, 600, 8, 8);
            }
            Ok(())
        }
    )*};
}
level_init! {
    BigGhostLevelFour_Func_Initialisation = 3, false;
    BigGhostLevelNine_Func_Initialisation = 3, false;
    MedGhostLevelTwo_Func_Initialisation = 2, false;
    MedGhostLevelThree_Func_Initialisation = 2, false;
    MedGhostLevelFour_Func_Initialisation = 2, false;
    MedGhostLevelNine_Func_Initialisation = 2, false;
    SmallGhostLevelTwo_Func_Initialisation_19a6f2fc = 1, false;
    SmallGhostLevelThreeOld_Func_Initialisation = 1, false;
    SmallGhostLevelThree_Func_Initialisation = 1, false;
    SmallGhostLevelFour_Func_Initialisation_4324ff1c = 1, false;
    SmallGhostLevelNine_Func_Initialisation = 1, false;
    BigGhostLevelOneA_Func_Initialisation = 3, false;
    BigGhostLevelOneC_Func_Initialisation = 3, false;
    BigGhostLevelOne_Func_Initialisation_97a3a638 = 3, false;
    MedGhostLevelOneA_Func_Initialisation = 2, true;
    SmallGhostLevelOneA_Func_Initialisation = 1, true;
    BigGhostLevelOne_Func_Initialisation_f2015fc2 = 2, true;
    SmallGhostLevelTwo_Func_Initialisation_69625cbf = 1, true;
    SmallGhostLevelFour_Func_Initialisation_2e46e079 = 1, true;
    MedGhostLevelOne_Func_Initialisation_92ff0346 = 2, true;
    SmallGhostLevelOne_Func_Initialisation_056ca924 = 1, true;
    GhostBossLevelFive_Func_Initialisation = 0x13, true;
    MedGhostLevelFive_Func_Initialisation = 2, true;
}

/// 0x0001e5c0 (757255de) small ghost; L+0x34 (state to resume after a pause) = 9999 (none)
pub fn SmallGhostLevelOne_Func_Initialisation_757255de(e: &mut Engine) -> LResult<()> {
    set_lv(e, 0x44, 1);
    set_lv(e, 0x34, 9999);
    APIphysic::PrimSetIntAttribBits(e, SELF, 600, 8, 8);
    Ok(())
}

/// 0x0001e5d8 (1a23fd5a)
pub fn MedGhostLevelOne_Func_Initialisation_1a23fd5a(e: &mut Engine) -> LResult<()> {
    set_lv(e, 0x44, 2);
    set_lv(e, 0x34, 9999);
    APIphysic::PrimSetIntAttribBits(e, SELF, 600, 8, 8);
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 3, 0xc);
    Ok(())
}

/// 0x0001e6f0 (medghostlevelthreeold)
pub fn MedGhostLevelThreeOld_Func_Initialisation(e: &mut Engine) -> LResult<()> {
    set_lv(e, 0x44, 2);
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 0x16, 0x1f);
    Ok(())
}

// ---- message functions ----

/// `Ghost_BaseFUNC_MessageHandler(param_1, param_2, param_3)` only: MedGhostLevelTwo,
/// MedGhostLevelThree, SmallGhostLevelTwo (19a6f2fc), SmallGhostLevelThree, BigGhostLevelOneA,
/// BigGhostLevelOneC, BigGhostLevelOne (f2015fc2, 97a3a638)
fn forward_msg(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    Ghost_BaseFUNC_MessageHandler(e, param_1, param_2, param_3)
}
pub fn MedGhostLevelTwo_MessageFunction(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    forward_msg(e, param_1, param_2, param_3)
}
pub fn MedGhostLevelThree_MessageFunction(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    forward_msg(e, param_1, param_2, param_3)
}
pub fn SmallGhostLevelTwo_MessageFunction_19a6f2fc(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    forward_msg(e, param_1, param_2, param_3)
}
pub fn SmallGhostLevelThree_MessageFunction(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    forward_msg(e, param_1, param_2, param_3)
}
pub fn BigGhostLevelOneA_MessageFunction(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    forward_msg(e, param_1, param_2, param_3)
}
pub fn BigGhostLevelOneC_MessageFunction(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    forward_msg(e, param_1, param_2, param_3)
}
pub fn BigGhostLevelOne_MessageFunction(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    forward_msg(e, param_1, param_2, param_3)
}

/// message 199 (the ghost may now cross floors with attribute 8): `PrimResetIntAttribBits(SELF,
/// 600, 8, 8)`, then the ghost handler. MedGhostLevelOneA, SmallGhostLevelOneA, SmallGhostLevelTwo
/// (69625cbf), SmallGhostLevelFour (2e46e079), MedGhostLevelOne (92ff0346), SmallGhostLevelOne
/// (056ca924), MedGhostLevelFive (medghostlevel4a/4b)
fn prim_reset_msg(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    if param_1 == 199 {
        APIphysic::PrimResetIntAttribBits(e, SELF, 600, 8, 8);
    }
    Ghost_BaseFUNC_MessageHandler(e, param_1, param_2, param_3)
}
pub fn MedGhostLevelOneA_MessageFunction(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    prim_reset_msg(e, param_1, param_2, param_3)
}
pub fn SmallGhostLevelOneA_MessageFunction(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    prim_reset_msg(e, param_1, param_2, param_3)
}
pub fn SmallGhostLevelTwo_MessageFunction_69625cbf(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    prim_reset_msg(e, param_1, param_2, param_3)
}
pub fn SmallGhostLevelFour_MessageFunction_2e46e079(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    prim_reset_msg(e, param_1, param_2, param_3)
}
pub fn MedGhostLevelOne_MessageFunction_92ff0346(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    prim_reset_msg(e, param_1, param_2, param_3)
}
pub fn SmallGhostLevelOne_MessageFunction_056ca924(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    prim_reset_msg(e, param_1, param_2, param_3)
}
/// (medghostlevel4a, medghostlevel4b)
pub fn MedGhostLevelFive_MessageFunction(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    prim_reset_msg(e, param_1, param_2, param_3)
}

/// 0x57 pause (timers paused, physics flag 1, state saved in L+0x34, state 5), 0x58 resume, 199
/// (tell 0x2a000020 with 0x104, floor attribute 8 allowed). SmallGhostLevelOne (757255de),
/// MedGhostLevelOne (1a23fd5a).
fn pause_msg(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    if param_1 == 0x58 {
        pause_timers(e, 0, true);
        APIphysic::ResetFlags(e, SELF, 1);
        end_attack_if_attacking(e);
        if lv(e, 0x34) != 9999 {
            let s = lv(e, 0x34);
            APIgameobj::SetNextState(e, SELF, s);
        }
    } else if param_1 < 0x59 {
        if param_1 == 0x57 {
            pause_timers(e, 1, true);
            APIphysic::SetFlags(e, SELF, 1);
            save_state_unless_5(e);
            APIgameobj::SetNextState(e, SELF, 5);
        }
    } else if param_1 == 199 {
        APIgameobj::SendMessageTo(e, 0x2a00_0020, NULL, 0x104, 0, NULL);
        APIphysic::PrimResetIntAttribBits(e, SELF, 600, 8, 8);
    }
    Ghost_BaseFUNC_MessageHandler(e, param_1, param_2, param_3)
}
/// 0x0001e630 (757255de)
pub fn SmallGhostLevelOne_MessageFunction_757255de(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    pause_msg(e, param_1, param_2, param_3)
}
/// 0x0001e8c8 (1a23fd5a)
pub fn MedGhostLevelOne_MessageFunction_1a23fd5a(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    pause_msg(e, param_1, param_2, param_3)
}

/// captured (0xd6/0x68): tell `to` (0x11d with our path A) [; 0xde: new path A]
fn capture_report_msg(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr, to: u32, with_de: bool) -> LResult<u64> {
    if param_1 == 0xd6 {
        if param_2 == 0x68 {
            let p = lv(e, 0x82c);
            APIgameobj::SendMessageTo(e, to, NULL, 0x11d, p, NULL);
        }
    } else if with_de && param_1 == 0xde {
        set_lv(e, 0x82c, param_2);
        let p = lv(e, 0x82c);
        CheckNearestPointOnPath(e, p)?;
    }
    Ghost_BaseFUNC_MessageHandler(e, param_1, param_2, param_3)
}
/// 0x0001e6c0 (bigghostlevelfour)
pub fn BigGhostLevelFour_MessageFunction(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    capture_report_msg(e, param_1, param_2, param_3, 0x2a00_0032, false)
}
/// 0x0001e8a0 (medghostlevelfour)
pub fn MedGhostLevelFour_MessageFunction(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    capture_report_msg(e, param_1, param_2, param_3, 0x2a00_0032, true)
}
/// 0x0001e6c8 (4324ff1c)
pub fn SmallGhostLevelFour_MessageFunction_4324ff1c(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    capture_report_msg(e, param_1, param_2, param_3, 0x2a00_0032, true)
}

/// The level-nine ghosts (sent by the level, state 0x20 = asleep):
/// 0xde new path A; 0xd6/0x68 captured -> 0x11d with path A to `to` (ep2: the glow is gone, L+0x874
/// = 0); 0x1d2 param 0 = park at y -100 and sleep, else = appear at waypoint 0 of path `param_2`
/// and start (ep2: recreate the glow, the path is both paths and the new home).
fn level_nine_msg(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr, to: u32, with_de: bool, ep2: bool) -> LResult<u64> {
    if with_de && param_1 == 0xde {
        set_lv(e, 0x82c, param_2);
        let p = lv(e, 0x82c);
        CheckNearestPointOnPath(e, p)?;
    } else if param_1 == 0xd6 {
        if param_2 == 0x68 {
            if ep2 {
                set_lv(e, 0x874, 0);
            }
            let p = lv(e, 0x82c);
            APIgameobj::SendMessageTo(e, to, NULL, 0x11d, p, NULL);
        }
    } else if param_1 == 0x1d2 {
        let d = e.alloca(0x10);
        e.wf(d, 0.0);
        e.wf(d + 4, -100.0);
        e.wf(d + 8, 0.0);
        if param_2 == 0 {
            APIgameobj::SetTranslation(e, SELF, d);
            APIgameobj::SetNextState(e, SELF, 0x20);
        } else {
            if ep2 && lv(e, 0x874) == 0 {
                CreateGlow(e)?;
                set_lv(e, 0x874, 1);
            }
            APIextra::WaypointGetPos(e, param_2, 0, d);
            APIgameobj::SetTranslation(e, SELF, d);
            if ep2 {
                for i in 0..3 {
                    let v = e.r32(d + i * 4);
                    set_lv(e, 0xf8 + i * 4, v);
                }
            }
            set_lv(e, 0x82c, param_2);
            if ep2 {
                set_lv(e, 0x828, param_2);
            }
            InitPath(e, param_2)?;
            let p = lv(e, 0x82c);
            CheckNearestPointOnPath(e, p)?;
            APIgameobj::SetContextFlag(e, SELF, 7);
            APIgameobj::SetNextState(e, SELF, 0xe);
        }
    }
    Ghost_BaseFUNC_MessageHandler(e, param_1, param_2, param_3)
}
/// 0x0001e6f0 (01950f3a)
pub fn BigGhostLevelNine_MessageFunction_01950f3a(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    level_nine_msg(e, param_1, param_2, param_3, 0x2a00_0033, false, false)
}
/// 0x0001e788 (74e95992)
pub fn BigGhostLevelNine_MessageFunction_74e95992(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    level_nine_msg(e, param_1, param_2, param_3, 0x2a00_000d, false, true)
}
/// 0x0001e6f8 (de27b0f0)
pub fn MedGhostLevelNine_MessageFunction_de27b0f0(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    level_nine_msg(e, param_1, param_2, param_3, 0x2a00_0033, true, false)
}
/// 0x0001e788 (7f4484ea)
pub fn MedGhostLevelNine_MessageFunction_7f4484ea(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    level_nine_msg(e, param_1, param_2, param_3, 0x2a00_000d, true, true)
}
/// 0x0001e6f8 (d0275f8b)
pub fn SmallGhostLevelNine_MessageFunction_d0275f8b(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    level_nine_msg(e, param_1, param_2, param_3, 0x2a00_0033, true, false)
}
/// 0x0001e788 (9a257725)
pub fn SmallGhostLevelNine_MessageFunction_9a257725(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    level_nine_msg(e, param_1, param_2, param_3, 0x2a00_000d, true, true)
}

/// state 0x20 of the level-nine ghosts (asleep): nothing.
/// BigGhostLevelNineST_SLEEP / MedGhostLevelNineST_SLEEP / SmallGhostLevelNineST_SLEEP
pub fn BigGhostLevelNineST_SLEEP(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}
pub fn MedGhostLevelNineST_SLEEP(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}
pub fn SmallGhostLevelNineST_SLEEP(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

/// The first time a ghost is hooked (0xd6/0x65 with L+0x854 == 0): text 0x15b for 5 s and message
/// 0xdf to every object of class 8 (`{1, 1}` retriever), so the other ghosts do not show it again;
/// 0xc9 re-arms it. MedGhostLevelThreeOld / SmallGhostLevelThreeOld (same code).
fn three_old_msg(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    if param_1 == 0xd6 {
        if param_2 == 0x65 && lv(e, 0x854) == 0 {
            set_lv(e, 0x854, 1);
            show_text(e, 0x15b, 0x40a0_0000);
            let r = e.alloca(8);
            e.w32(r, 1);
            e.w32(r + 4, 1);
            APIgameobj::SendMessageTo(e, 0xff00_0008, r, 0xdf, 0, NULL);
        }
    } else if param_1 < 0xd7 {
        if param_1 == 0xc9 {
            set_lv(e, 0x854, 0);
        }
    } else if param_1 == 0xdf {
        set_lv(e, 0x854, 1);
    }
    Ghost_BaseFUNC_MessageHandler(e, param_1, param_2, param_3)
}
/// 0x0001eb18 (medghostlevelthreeold)
pub fn MedGhostLevelThreeOld_MessageFunction(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    three_old_msg(e, param_1, param_2, param_3)
}
/// 0x0001eb10 (smallghostlevelthreeold)
pub fn SmallGhostLevelThreeOld_MessageFunction(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    three_old_msg(e, param_1, param_2, param_3)
}

/// 0x0001e4b8 (ghostbosslevel5) boss 0x2a000012 captured: tell 0x2a00005a (0x180) and the level
/// (0x3a); the ghost handler does not see the capture.
pub fn GhostBossLevelFive_MessageFunction(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    if param_1 == 199 {
        APIphysic::PrimResetIntAttribBits(e, SELF, 600, 8, 8);
    } else if param_1 == 0xd6 && param_2 == 0x68 {
        if APIgameobj::GetID(e, SELF, NULL) == 0x2a00_0012 {
            APIgameobj::SendMessageTo(e, 0x2a00_005a, NULL, 0x180, 0, NULL);
            APIgameobj::SendMessageTo(e, 0x2a10_0006, NULL, 0x3a, 0, NULL);
        }
        return Ok(0);
    }
    Ghost_BaseFUNC_MessageHandler(e, param_1, param_2, param_3)
}

/// 0x0001e678 (medghostlevel5) 0xc9 home = origin; 0x1a9 and 0xd6/0x68 (captured) are passed on
/// to the partner ghost-holder object (0x2a0000c3 -> 0x2a0000be, 0x2a0000c4 -> 0x2a0000bf); a
/// captured ghost goes back to the origin and reinitialises instead of being captured. NOTE: a
/// 0x1a9 for another id falls into the `param_2 == 0x68` test, as in the original.
pub fn MedGhostLevelFive_MessageFunction_medghostlevel5(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    if param_1 == 0xc9 {
        set_lv(e, 0xf8, 0);
        set_lv(e, 0xfc, 0);
        set_lv(e, 0x100, 0);
    } else if param_1 < 0xca {
        if param_1 == 199 {
            APIphysic::PrimResetIntAttribBits(e, SELF, 600, 8, 8);
        }
    } else {
        if param_1 != 0xd6 {
            if param_1 != 0x1a9 {
                return Ghost_BaseFUNC_MessageHandler(e, param_1, param_2, param_3);
            }
            let iVar1 = APIgameobj::GetID(e, SELF, NULL);
            if iVar1 == 0x2a00_00c3 {
                APIgameobj::SendMessageTo(e, 0x2a00_00be, NULL, 0x1a9, 0, NULL);
                return Ok(0);
            }
            if iVar1 == 0x2a00_00c4 {
                APIgameobj::SendMessageTo(e, 0x2a00_00bf, NULL, 0x1a9, 0, NULL);
                return Ok(0);
            }
        }
        if param_2 == 0x68 {
            let d = e.alloca(0x10);
            e.wf(d, 0.0);
            e.wf(d + 4, 0.0);
            e.wf(d + 8, 0.0);
            if APIgameobj::GetID(e, SELF, NULL) == 0x2a00_00c3 {
                APIgameobj::SendMessageTo(e, 0x2a00_00be, NULL, 0x68, 0, NULL);
            } else if APIgameobj::GetID(e, SELF, NULL) == 0x2a00_00c4 {
                APIgameobj::SendMessageTo(e, 0x2a00_00bf, NULL, 0x68, 0, NULL);
            }
            APIgameobj::SetTranslation(e, SELF, d);
            Ghost_Base_Func_Initialisation(e)?;
            return Ok(0);
        }
    }
    Ghost_BaseFUNC_MessageHandler(e, param_1, param_2, param_3)
}

// ---- cut-scene callbacks (external function 0x1f, notify 0x16 {type, start}) ----

/// 0x0001e670 (1a23fd5a) ghost 0x2a000006: frozen at home during the cut scene; at the end, text
/// 0x14e for 4 s and message 0x2a (1) to the level
pub fn Ghost_BaseCB_CUT_SCENE_1a23fd5a(e: &mut Engine, param_1: u32) -> LResult<u64> {
    let iVar2 = APIgameobj::GetID(e, SELF, NULL);
    if e.r32(param_1 + 4) == 1 {
        if iVar2 == 0x2a00_0006 {
            translate_home(e);
            let s = APIgameobj::GetCurrentStateID(e, SELF);
            set_lv(e, 0x34, s);
            cut_scene_in(e);
        }
    } else if e.r32(param_1 + 4) == 0 && iVar2 == 0x2a00_0006 {
        show_text(e, 0x14e, 0x4080_0000);
        APIgameobj::SendMessageTo(e, 0x2a10_0006, NULL, 0x2a, 1, NULL);
        cut_scene_out(e);
        let s = lv(e, 0x34);
        APIgameobj::SetNextState(e, SELF, s);
    }
    Ok(0)
}

/// 0x0001e750 (medghostlevelthreeold) cut scene type 1: ghost 0x2a00002c pauses at home, the others
/// are disabled
pub fn Ghost_BaseCB_CUT_SCENE_medghostlevelthreeold(e: &mut Engine, param_1: u32) -> LResult<u64> {
    let iVar2 = APIgameobj::GetID(e, SELF, NULL);
    let (ty, start) = (e.r32(param_1), e.r32(param_1 + 4));
    if start == 1 && ty == 1 {
        if iVar2 == 0x2a00_002c {
            pause_timers(e, 1, false);
            translate_home(e);
            save_state_unless_5(e);
            cut_scene_in(e);
        } else {
            APIgameobj::EnableObject(e, SELF, 0);
        }
    } else if start == 0 && ty == 1 {
        if iVar2 == 0x2a00_002c {
            pause_timers(e, 0, false);
            cut_scene_out(e);
            end_attack_if_attacking(e);
            let s = lv(e, 0x34);
            APIgameobj::SetNextState(e, SELF, s);
        } else {
            APIgameobj::EnableObject(e, SELF, 1);
        }
    }
    Ok(0)
}

/// 0x0001e720 (smallghostlevelthreeold) as the medium ghost's, for ghost 0x2a000029, which is also
/// put back home at the end
pub fn Ghost_BaseCB_CUT_SCENE_smallghostlevelthreeold(e: &mut Engine, param_1: u32) -> LResult<u64> {
    let iVar2 = APIgameobj::GetID(e, SELF, NULL);
    let (ty, start) = (e.r32(param_1), e.r32(param_1 + 4));
    if start == 1 && ty == 1 {
        if iVar2 == 0x2a00_0029 {
            pause_timers(e, 1, false);
            translate_home(e);
            save_state_unless_5(e);
            cut_scene_in(e);
        } else {
            APIgameobj::EnableObject(e, SELF, 0);
        }
    } else if start == 0 && ty == 1 {
        if iVar2 == 0x2a00_0029 {
            pause_timers(e, 0, false);
            translate_home(e);
            cut_scene_out(e);
            end_attack_if_attacking(e);
            let s = lv(e, 0x34);
            APIgameobj::SetNextState(e, SELF, s);
        } else {
            APIgameobj::EnableObject(e, SELF, 1);
        }
    }
    Ok(0)
}

/// 0x0001e6c8 (medghostlevelfour) ghost 0x2a00001e is made visible for the cut scene
pub fn Ghost_BaseCB_CUT_SCENE_medghostlevelfour(e: &mut Engine, param_1: u32) -> LResult<u64> {
    let iVar2 = APIgameobj::GetID(e, SELF, NULL);
    if e.r32(param_1 + 4) == 1 {
        if iVar2 == 0x2a00_001e {
            APIgameobj::SetGameObjectVisible(e, SELF, 1);
            let s = APIgameobj::GetCurrentStateID(e, SELF);
            set_lv(e, 0x34, s);
            cut_scene_in(e);
        }
    } else if e.r32(param_1 + 4) == 0 && iVar2 == 0x2a00_001e {
        cut_scene_out(e);
        let s = lv(e, 0x34);
        APIgameobj::SetNextState(e, SELF, s);
    }
    Ok(0)
}

/// 0x0001e780 (97a3a638) cut scene type 0: ghost 0x2a00018e pauses at home, the others are
/// disabled
pub fn Ghost_BaseCB_CUT_SCENE_97a3a638(e: &mut Engine, param_1: u32) -> LResult<u64> {
    let iVar2 = APIgameobj::GetID(e, SELF, NULL);
    let (ty, start) = (e.r32(param_1), e.r32(param_1 + 4));
    if start == 1 && ty == 0 {
        if iVar2 == 0x2a00_018e {
            translate_home(e);
            pause_timers(e, 1, false);
            translate_home(e);
            save_state_unless_5(e);
            cut_scene_in(e);
        } else {
            APIgameobj::EnableObject(e, SELF, 0);
        }
    } else if start == 0 && ty == 0 {
        if iVar2 == 0x2a00_018e {
            pause_timers(e, 0, false);
            cut_scene_out(e);
            end_attack_if_attacking(e);
            let s = lv(e, 0x34);
            APIgameobj::SetNextState(e, SELF, s);
        } else {
            APIgameobj::EnableObject(e, SELF, 1);
        }
    }
    Ok(0)
}

/// cut scene start / end for every ghost of the module (92ff0346, 056ca924)
fn cut_scene_all(e: &mut Engine, start: bool) {
    if start {
        pause_timers(e, 1, false);
        translate_home(e);
        save_state_unless_5(e);
        cut_scene_in(e);
    } else {
        pause_timers(e, 0, false);
        cut_scene_out(e);
        end_attack_if_attacking(e);
        let s = lv(e, 0x34);
        APIgameobj::SetNextState(e, SELF, s);
    }
}

/// 0x0001e7f0 (92ff0346) cut scene type 0, every ghost
pub fn Ghost_BaseCB_CUT_SCENE_92ff0346(e: &mut Engine, param_1: u32) -> LResult<u64> {
    APIgameobj::GetID(e, SELF, NULL);
    let (ty, start) = (e.r32(param_1), e.r32(param_1 + 4));
    if start == 1 && ty == 0 {
        cut_scene_all(e, true);
    } else if start == 0 && ty == 0 {
        cut_scene_all(e, false);
    }
    Ok(0)
}

/// 0x0001e7f0 (056ca924) any cut scene, every ghost
pub fn Ghost_BaseCB_CUT_SCENE_056ca924(e: &mut Engine, param_1: u32) -> LResult<u64> {
    APIgameobj::GetID(e, SELF, NULL);
    if e.r32(param_1 + 4) == 1 {
        cut_scene_all(e, true);
    } else if e.r32(param_1 + 4) == 0 {
        cut_scene_all(e, false);
    }
    Ok(0)
}

// ---- level constructors ----

/// 0x0001e9c0 (757255de)
pub fn smallghostleveloneConstructor_757255de(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    let (m, i) = (SmallGhostLevelOne_MessageFunction_757255de as MsgFn, SmallGhostLevelOne_Func_Initialisation_757255de);
    level_ctor(e, api, instanceCount, smallghostepisodeoneConstructor, &[], m, i, None)
}
/// 0x0001e7b0 (bigghostlevelfour)
pub fn bigghostlevelfourConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    let (m, i) = (BigGhostLevelFour_MessageFunction as MsgFn, BigGhostLevelFour_Func_Initialisation);
    level_ctor(e, api, instanceCount, bigghostepisodeoneConstructor, &[], m, i, None)
}
/// 0x0001e910 (01950f3a)
pub fn bigghostlevelnineConstructor_01950f3a(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    let (m, i) = (BigGhostLevelNine_MessageFunction_01950f3a as MsgFn, BigGhostLevelNine_Func_Initialisation);
    level_ctor(e, api, instanceCount, bigghostepisodeoneConstructor, &[(0x20, BigGhostLevelNineST_SLEEP)], m, i, None)
}
/// 0x0001ec58 (1a23fd5a) ghost 0x2a000006 listens to the cut scenes of 0x2a00013b
pub fn medghostleveloneConstructor_1a23fd5a(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    medghostepisodeoneConstructor(e, api, instanceCount)?;
    if instanceCount == 1 {
        get_api_text(e);
        APIcons::SetExternalFunction(e, 0x1f, Ghost_BaseCB_CUT_SCENE_1a23fd5a);
    }
    APIcons::SetMessageFunction(e, MedGhostLevelOne_MessageFunction_1a23fd5a);
    MedGhostLevelOne_Func_Initialisation_1a23fd5a(e)?;
    Monster_Base_Func_Initialise(e)?;
    Ghost_Base_Func_Initialisation(e)?;
    CreateGlow(e)?;
    if APIgameobj::GetID(e, SELF, NULL) == 0x2a00_0006 {
        APIgameobj::RegisterNotify(e, SELF, 0x2a00_013b, 0x16, 0x1f);
    }
    APIcons::SetInitialState(e, 0xe);
    Ok(())
}
/// 0x0001e6d0 (medghostleveltwo)
pub fn medghostleveltwoConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    let (m, i) = (MedGhostLevelTwo_MessageFunction as MsgFn, MedGhostLevelTwo_Func_Initialisation);
    level_ctor(e, api, instanceCount, medghostepisodeoneConstructor, &[], m, i, None)
}
/// `RegisterNotify(SELF, 0xffffffff, 0x16, 0x1f)`: every cut scene
fn notify_cut_scenes(e: &mut Engine) -> LResult<()> {
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 0x16, 0x1f);
    Ok(())
}
/// 0x0001ecd8 (medghostlevelthreeold)
pub fn medghostlevelthreeoldConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    medghostepisodeoneConstructor(e, api, instanceCount)?;
    if instanceCount == 1 {
        APIcons::SetExternalFunction(e, 0x1f, Ghost_BaseCB_CUT_SCENE_medghostlevelthreeold);
        get_api_text(e);
    }
    APIcons::SetMessageFunction(e, MedGhostLevelThreeOld_MessageFunction);
    MedGhostLevelThreeOld_Func_Initialisation(e)?;
    Monster_Base_Func_Initialise(e)?;
    Ghost_Base_Func_Initialisation(e)?;
    CreateGlow(e)?;
    notify_cut_scenes(e)?;
    APIcons::SetInitialState(e, 0xe);
    Ok(())
}
/// 0x0001e7c8 (medghostlevelthree)
pub fn medghostlevelthreeConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    let (m, i) = (MedGhostLevelThree_MessageFunction as MsgFn, MedGhostLevelThree_Func_Initialisation);
    level_ctor(e, api, instanceCount, medghostepisodeoneConstructor, &[], m, i, None)
}
/// 0x0001e9d0 (medghostlevelfour) ghost 0x2a00001e listens to the cut scenes of object 0 (NOTE: as
/// in the original; RegisterNotify's object filter 0)
pub fn medghostlevelfourConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    let (m, i) = (MedGhostLevelFour_MessageFunction as MsgFn, MedGhostLevelFour_Func_Initialisation);
    let after: fn(&mut Engine) -> LResult<()> = |e| {
        if APIgameobj::GetID(e, SELF, NULL) == 0x2a00_001e {
            APIgameobj::RegisterNotify(e, SELF, 0, 0x16, 0x1f);
        }
        Ok(())
    };
    let ext: [(i32, ExtFn); 1] = [(0x1f, Ghost_BaseCB_CUT_SCENE_medghostlevelfour)];
    level_ctor(e, api, instanceCount, medghostepisodeoneConstructor, &ext, m, i, Some(after))
}
/// 0x0001e970 (de27b0f0)
pub fn medghostlevelnineConstructor_de27b0f0(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    let (m, i) = (MedGhostLevelNine_MessageFunction_de27b0f0 as MsgFn, MedGhostLevelNine_Func_Initialisation);
    level_ctor(e, api, instanceCount, medghostepisodeoneConstructor, &[(0x20, MedGhostLevelNineST_SLEEP)], m, i, None)
}
/// 0x0001e6d0 (19a6f2fc)
pub fn smallghostleveltwoConstructor_19a6f2fc(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    let (m, i) = (SmallGhostLevelTwo_MessageFunction_19a6f2fc as MsgFn, SmallGhostLevelTwo_Func_Initialisation_19a6f2fc);
    level_ctor(e, api, instanceCount, smallghostepisodeoneConstructor, &[], m, i, None)
}
/// 0x0001ecd0 (smallghostlevelthreeold)
pub fn smallghostlevelthreeoldConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    smallghostepisodeoneConstructor(e, api, instanceCount)?;
    if instanceCount == 1 {
        APIcons::SetExternalFunction(e, 0x1f, Ghost_BaseCB_CUT_SCENE_smallghostlevelthreeold);
        get_api_text(e);
    }
    APIcons::SetMessageFunction(e, SmallGhostLevelThreeOld_MessageFunction);
    SmallGhostLevelThreeOld_Func_Initialisation(e)?;
    Monster_Base_Func_Initialise(e)?;
    Ghost_Base_Func_Initialisation(e)?;
    CreateGlow(e)?;
    notify_cut_scenes(e)?;
    APIcons::SetInitialState(e, 0xe);
    Ok(())
}
/// 0x0001e7c8 (smallghostlevelthree)
pub fn smallghostlevelthreeConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    let (m, i) = (SmallGhostLevelThree_MessageFunction as MsgFn, SmallGhostLevelThree_Func_Initialisation);
    level_ctor(e, api, instanceCount, smallghostepisodeoneConstructor, &[], m, i, None)
}
/// 0x0001e7f8 (4324ff1c)
pub fn smallghostlevelfourConstructor_4324ff1c(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    let (m, i) = (SmallGhostLevelFour_MessageFunction_4324ff1c as MsgFn, SmallGhostLevelFour_Func_Initialisation_4324ff1c);
    level_ctor(e, api, instanceCount, smallghostepisodeoneConstructor, &[], m, i, None)
}
/// 0x0001e970 (d0275f8b)
pub fn smallghostlevelnineConstructor_d0275f8b(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    let (m, i) = (SmallGhostLevelNine_MessageFunction_d0275f8b as MsgFn, SmallGhostLevelNine_Func_Initialisation);
    level_ctor(e, api, instanceCount, smallghostepisodeoneConstructor, &[(0x20, SmallGhostLevelNineST_SLEEP)], m, i, None)
}
/// 0x0001e7e8 (bigghostlevelonea)
pub fn bigghostleveloneaConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    let (m, i) = (BigGhostLevelOneA_MessageFunction as MsgFn, BigGhostLevelOneA_Func_Initialisation);
    level_ctor(e, api, instanceCount, bigghostepisodetwoConstructor, &[], m, i, None)
}
/// `L+0x874 = 1` (the glow exists; the ep2 level-nine ghosts)
fn glow_created(e: &mut Engine) -> LResult<()> {
    set_lv(e, 0x874, 1);
    Ok(())
}
/// 0x0001ea28 (74e95992)
pub fn bigghostlevelnineConstructor_74e95992(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    let (m, i) = (BigGhostLevelNine_MessageFunction_74e95992 as MsgFn, BigGhostLevelNine_Func_Initialisation);
    level_ctor(e, api, instanceCount, bigghostepisodetwoConstructor, &[(0x20, BigGhostLevelNineST_SLEEP)], m, i, Some(glow_created))
}
/// 0x0001e748 (bigghostlevelonec)
pub fn bigghostlevelonecConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    let (m, i) = (BigGhostLevelOneC_MessageFunction as MsgFn, BigGhostLevelOneC_Func_Initialisation);
    level_ctor(e, api, instanceCount, bigghostepisodetwoConstructor, &[], m, i, None)
}
/// 0x0001ea80 (7f4484ea)
pub fn medghostlevelnineConstructor_7f4484ea(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    let (m, i) = (MedGhostLevelNine_MessageFunction_7f4484ea as MsgFn, MedGhostLevelNine_Func_Initialisation);
    level_ctor(e, api, instanceCount, medghostepisodetwoConstructor, &[(0x20, MedGhostLevelNineST_SLEEP)], m, i, Some(glow_created))
}
/// 0x0001e840 (medghostlevelonea)
pub fn medghostleveloneaConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    let (m, i) = (MedGhostLevelOneA_MessageFunction as MsgFn, MedGhostLevelOneA_Func_Initialisation);
    level_ctor(e, api, instanceCount, medghostepisodetwoConstructor, &[], m, i, None)
}
/// 0x0001ea80 (9a257725)
pub fn smallghostlevelnineConstructor_9a257725(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    let (m, i) = (SmallGhostLevelNine_MessageFunction_9a257725 as MsgFn, SmallGhostLevelNine_Func_Initialisation);
    level_ctor(e, api, instanceCount, smallghostepisodetwoConstructor, &[(0x20, SmallGhostLevelNineST_SLEEP)], m, i, Some(glow_created))
}
/// 0x0001e840 (smallghostlevelonea)
pub fn smallghostleveloneaConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    let (m, i) = (SmallGhostLevelOneA_MessageFunction as MsgFn, SmallGhostLevelOneA_Func_Initialisation);
    level_ctor(e, api, instanceCount, smallghostepisodetwoConstructor, &[], m, i, None)
}
/// 0x0001e6e0 (f2015fc2)
pub fn bigghostleveloneConstructor_f2015fc2(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    let (m, i) = (BigGhostLevelOne_MessageFunction as MsgFn, BigGhostLevelOne_Func_Initialisation_f2015fc2);
    level_ctor(e, api, instanceCount, bigghostepisodethreeConstructor, &[], m, i, None)
}
/// 0x0001e7c8 (69625cbf)
pub fn smallghostleveltwoConstructor_69625cbf(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    let (m, i) = (SmallGhostLevelTwo_MessageFunction_69625cbf as MsgFn, SmallGhostLevelTwo_Func_Initialisation_69625cbf);
    level_ctor(e, api, instanceCount, smallghostepisodethreeConstructor, &[], m, i, None)
}
/// 0x0001e788 (2e46e079)
pub fn smallghostlevelfourConstructor_2e46e079(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    let (m, i) = (SmallGhostLevelFour_MessageFunction_2e46e079 as MsgFn, SmallGhostLevelFour_Func_Initialisation_2e46e079);
    level_ctor(e, api, instanceCount, smallghostepisodethreeConstructor, &[], m, i, None)
}
/// 0x0001eb68 (97a3a638)
pub fn bigghostleveloneConstructor_97a3a638(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    let (m, i) = (BigGhostLevelOne_MessageFunction as MsgFn, BigGhostLevelOne_Func_Initialisation_97a3a638);
    let ext: [(i32, ExtFn); 1] = [(0x1f, Ghost_BaseCB_CUT_SCENE_97a3a638)];
    level_ctor(e, api, instanceCount, bigghostepisodefourConstructor, &ext, m, i, Some(notify_cut_scenes))
}
/// 0x0001eb40 (92ff0346)
pub fn medghostleveloneConstructor_92ff0346(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    let (m, i) = (MedGhostLevelOne_MessageFunction_92ff0346 as MsgFn, MedGhostLevelOne_Func_Initialisation_92ff0346);
    let ext: [(i32, ExtFn); 1] = [(0x1f, Ghost_BaseCB_CUT_SCENE_92ff0346)];
    level_ctor(e, api, instanceCount, medghostepisodefourConstructor, &ext, m, i, Some(notify_cut_scenes))
}
/// 0x0001eb20 (056ca924)
pub fn smallghostleveloneConstructor_056ca924(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    let (m, i) = (SmallGhostLevelOne_MessageFunction_056ca924 as MsgFn, SmallGhostLevelOne_Func_Initialisation_056ca924);
    let ext: [(i32, ExtFn); 1] = [(0x1f, Ghost_BaseCB_CUT_SCENE_056ca924)];
    level_ctor(e, api, instanceCount, smallghostepisodefourConstructor, &ext, m, i, Some(notify_cut_scenes))
}
/// 0x0001e628 (ghostbosslevel5) no episode file; the boss: book-holder range 4, sight / hearing 120,
/// always clipped in (MiscFunc_ChangeClipping 2)
pub fn ghostbosslevel5Constructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    let (m, i) = (GhostBossLevelFive_MessageFunction as MsgFn, GhostBossLevelFive_Func_Initialisation);
    let after: fn(&mut Engine) -> LResult<()> = |e| {
        set_lv(e, 0x850, 1);
        set_lv(e, 0x70, 0x4080_0000);
        set_lv(e, 0x74, 0x42f0_0000);
        set_lv(e, 0x78, 0x42f0_0000);
        let v = lv(e, 0x74);
        set_lv(e, 0xec, v);
        LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 2, NULL, 1);
        Ok(())
    };
    level_ctor(e, api, instanceCount, ghost_baseConstructor, &[], m, i, Some(after))
}
/// 0x0001e858 (medghostlevel4a)
pub fn medghostlevel4aConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    let (m, i) = (MedGhostLevelFive_MessageFunction as MsgFn, MedGhostLevelFive_Func_Initialisation);
    level_ctor(e, api, instanceCount, medghostepisodesixConstructor, &[], m, i, None)
}
/// 0x0001e840 (medghostlevel4b)
pub fn medghostlevel4bConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    let (m, i) = (MedGhostLevelFive_MessageFunction as MsgFn, MedGhostLevelFive_Func_Initialisation);
    level_ctor(e, api, instanceCount, medghostepisodesixConstructor, &[], m, i, None)
}
/// 0x0001e970 (medghostlevel5)
pub fn medghostlevel5Constructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    let (m, i) = (MedGhostLevelFive_MessageFunction_medghostlevel5 as MsgFn, MedGhostLevelFive_Func_Initialisation);
    level_ctor(e, api, instanceCount, medghostepisodesixConstructor, &[], m, i, None)
}

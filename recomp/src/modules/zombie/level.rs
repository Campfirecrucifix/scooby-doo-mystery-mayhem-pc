//! The per-level source files of each zombie build (`ZombieEpisodeFour.c` / `ZombieEpisodeSix.c` +
//! `ZombieLevel<N>.c`): the module entry points, the level's message function and init. The port has
//! one entry pair ([`zombie_levelConstructor`] / [`zombie_levelDestructor`]) that dispatches on the
//! loaded build. Two builds both define `zombielevelfourb*` / `ZombieLevelFourB_*` (Ep4 5e7d92cd and Ep6
//! 35a2d522): those carry a `_<key>` suffix. Addresses are module-relative (.text at 0x10000) of the
//! build named.
use super::builds::*;
use super::*;
use crate::engine::*;
use crate::logic::{LResult, MsgFn};

/// The module entry point (`<level>Constructor` of the loaded build).
pub fn zombie_levelConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    match build_key(e) {
        "zombieleveltwoa" => zombieleveltwoaConstructor(e, api, instanceCount),
        "5e7d92cd" => zombielevelfourbConstructor_5e7d92cd(e, api, instanceCount),
        "zombielevelfourc" => zombielevelfourcConstructor(e, api, instanceCount),
        "zombielevelfourd" => zombielevelfourdConstructor(e, api, instanceCount),
        "zombieleveltwo" => zombieleveltwoConstructor(e, api, instanceCount),
        "zombielevelfoura_b" => zombielevelfoura_bConstructor(e, api, instanceCount),
        "zombielevelfoura_c" => zombielevelfoura_cConstructor(e, api, instanceCount),
        "zombielevelfoura_d" => zombielevelfoura_dConstructor(e, api, instanceCount),
        "35a2d522" => zombielevelfourbConstructor_35a2d522(e, api, instanceCount),
        "zombielevelfourb_c" => zombielevelfourb_cConstructor(e, api, instanceCount),
        "zombielevelfourb_d" => zombielevelfourb_dConstructor(e, api, instanceCount),
        k => panic!("zombie: no level constructor for build {k}"),
    }
}

/// The module exit point (`<level>Destructor` of the loaded build). Every level destructor is its
/// episode destructor, i.e. zombie_baseDestructor.
pub fn zombie_levelDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    match build_key(e) {
        "zombieleveltwoa" => zombieleveltwoaDestructor(e, instanceCount),
        "5e7d92cd" => zombielevelfourbDestructor_5e7d92cd(e, instanceCount),
        "zombielevelfourc" => zombielevelfourcDestructor(e, instanceCount),
        "zombielevelfourd" => zombielevelfourdDestructor(e, instanceCount),
        "zombieleveltwo" => zombieleveltwoDestructor(e, instanceCount),
        "zombielevelfoura_b" => zombielevelfoura_bDestructor(e, instanceCount),
        "zombielevelfoura_c" => zombielevelfoura_cDestructor(e, instanceCount),
        "zombielevelfoura_d" => zombielevelfoura_dDestructor(e, instanceCount),
        "35a2d522" => zombielevelfourbDestructor_35a2d522(e, instanceCount),
        "zombielevelfourb_c" => zombielevelfourb_cDestructor(e, instanceCount),
        "zombielevelfourb_d" => zombielevelfourb_dDestructor(e, instanceCount),
        k => panic!("zombie: no level destructor for build {k}"),
    }
}

// ---- ZombieEpisodeFour.c / ZombieEpisodeSix.c ----

/// 0x00017928 (zombieleveltwoa)
pub fn zombieepisodefourConstructor(e: &mut Engine, api: Ptr, param_2: i32) -> LResult<()> {
    zombie_baseConstructor(e, api, param_2)?;
    Ok(())
}

/// 0x00017978 (zombieleveltwoa)
pub fn zombieepisodefourDestructor(e: &mut Engine, param_1: i32) -> LResult<()> {
    zombie_baseDestructor(e, param_1)
}

/// 0x000179c0 (zombieleveltwo)
pub fn zombieepisodesixConstructor(e: &mut Engine, api: Ptr, param_2: i32) -> LResult<()> {
    zombie_baseConstructor(e, api, param_2)?;
    Ok(())
}

/// 0x00017a10 (zombieleveltwo)
pub fn zombieepisodesixDestructor(e: &mut Engine, param_1: i32) -> LResult<()> {
    zombie_baseDestructor(e, param_1)
}

// ---- shared level-file shapes ----

/// `L+0x44 = 0xc` (monster type): every `ZombieLevel*_Func_Initialisation`
fn init_type_c(e: &mut Engine) -> LResult<()> {
    let l = L(e);
    e.w32(l + 0x44, 0xc);
    Ok(())
}

/// `msg; init; Monster_Base_Func_Initialise; zombie_base_Func_Initialisation` (the tail of every
/// level constructor)
fn level_tail(e: &mut Engine, msg: MsgFn, init: fn(&mut Engine) -> LResult<()>) -> LResult<()> {
    APIcons::SetMessageFunction(e, msg);
    init(e)?;
    Monster_Base_Func_Initialise(e)?;
    zombie_base_Func_Initialisation(e)
}

/// `PrimSetIntAttribBits(SELF, 0xbe, 5, 0x1000)` and the wall-ray DM's mask (+0x20) = 0x1008
fn ray_mask_1008(e: &mut Engine) {
    APIphysic::PrimSetIntAttribBits(e, SELF, 0xbe, 5, 0x1000);
    let dm = e.r32(L(e) + 0x18);
    let b = e.alloca(0x80);
    APIphysic::DMGetParams(e, dm, b);
    e.w32(b + 0x20, 0x1008);
    let dm = e.r32(L(e) + 0x18);
    APIphysic::DMSetParams(e, dm, b);
}

/// `GetID(0x2a0001ac) == 0x7ffffffe` (that object does not exist): behaviour 2 (wander)
fn wander_if_no_1ac(e: &mut Engine) {
    let iVar1 = APIgameobj::GetID(e, 0x2a00_01ac, NULL);
    if iVar1 == 0x7fff_fffe {
        e.w32(L(e) + 0x194, 2);
    }
}

/// `Play(SELF, anim, 6, blend); L+0x38 = anim`
fn play(e: &mut Engine, anim: u32, blend: f32) {
    let l = L(e);
    APIanim::Play(e, SELF, anim, 6, blend);
    e.w32(l + 0x38, anim);
}

/// `DMGetParams(L+4); {target, 5, 0, 0, 0}; DMSetParams` (move DM towards `target`)
fn move_to(e: &mut Engine, target: u32) {
    let l = L(e);
    let dm = e.r32(l + 4);
    let b = e.alloca(0x80);
    APIphysic::DMGetParams(e, dm, b);
    for (o, v) in [(0, target), (0x14, 5), (8, 0), (0xc, 0), (0x10, 0)] {
        e.w32(b + o, v);
    }
    let dm = e.r32(l + 4);
    APIphysic::DMSetParams(e, dm, b);
}

// ---- ZombieLevelTwoA.c (zombieleveltwoa, Ep4 L02A) ----

/// 0x000179d0
pub fn ZombieLevelTwoA_Func_Initialisation(e: &mut Engine) -> LResult<()> {
    init_type_c(e)
}

/// 0x00017a00
pub fn ZombieLevelTwoA_MessageFunction(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    zombie_baseMessageFonction(e, param_1, param_2, param_3)
}

/// 0x00017a68 callback 0x27 (cutscene `{type, on}`): type 0 on -> hide and freeze, off -> show
pub fn ZombieLevelTwoA_CB_CUTSCENE(e: &mut Engine, param_1: u32) -> LResult<u64> {
    if e.r32(param_1) == 0 {
        if e.r32(param_1 + 4) == 0 {
            APIphysic::ResetFlags(e, SELF, 1);
            APIgameobj::SetGameObjectVisible(e, SELF, 1);
        } else {
            APIphysic::SetFlags(e, SELF, 1);
            APIgameobj::SetGameObjectVisible(e, SELF, 0);
        }
    }
    Ok(0)
}

/// 0x00017b50
pub fn zombieleveltwoaConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    zombieepisodefourConstructor(e, api, instanceCount)?;
    if instanceCount == 1 {
        APIcons::SetExternalFunction(e, 0x27, ZombieLevelTwoA_CB_CUTSCENE);
    }
    level_tail(e, ZombieLevelTwoA_MessageFunction, ZombieLevelTwoA_Func_Initialisation)?;
    ray_mask_1008(e);
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 0x16, 0x27);
    Ok(())
}

/// 0x00017cb0
pub fn zombieleveltwoaDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    zombieepisodefourDestructor(e, instanceCount)
}

// ---- ZombieLevelFourB.c / FourC.c / FourD.c (Ep4 L04: 5e7d92cd, zombielevelfourc, zombielevelfourd) ----
// The three files are one source: they differ only in the 0xda anim (3 in fourc), the order of
// Sleep / SetGameObjectVisible on 0x1fd, and fourd's zombie_localST_RETURN telling 0x2a0000c3 (0xc1).

/// ST_CAPTURED of the three level-4 builds: unless game buffer 1 +0x30 is set, tell the manager
/// 0x2a00000d (0x14e, my id), restore the saved clipping and go to the black hole (5)
fn ep4_level4_captured(e: &mut Engine) -> LResult<u64> {
    if e.r32(e.r32(GDM(e)) + 0x30) == 0 {
        let uVar2 = APIgameobj::GetID(e, SELF, NULL);
        APIgameobj::SendMessageTo(e, 0x2a00_000d, NULL, 0x14e, uVar2, NULL);
        let p = LM(e) + 0x120;
        APIgameobj::SetClippingParams(e, SELF, p, 0);
        APIgameobj::SetNextState(e, SELF, 5);
    }
    Ok(0)
}

/// 0x00017a70 (5e7d92cd)
pub fn ZombieLevelFourB_ST_CAPTURED(e: &mut Engine, _: u32) -> LResult<u64> {
    ep4_level4_captured(e)
}

/// 0x00017a70 (zombielevelfourc)
pub fn ZombieLevelFourC_ST_CAPTURED(e: &mut Engine, _: u32) -> LResult<u64> {
    ep4_level4_captured(e)
}

/// (zombielevelfourd)
pub fn ZombieLevelFourD_ST_CAPTURED(e: &mut Engine, _: u32) -> LResult<u64> {
    ep4_level4_captured(e)
}

/// 0x00017b60 state 0x32 (knocked back, timer L+0x164): under 2 s and the hero within 1 m -> chase
/// (0x15); after 2.2 s look for the hero and, within 3.5 m of the post 0x2a0000c3, take it (anim 5,
/// state 0x20); between, walk to the post. Group 1 (fourd) also sends 0xc1 to the post.
pub fn zombie_localST_RETURN(e: &mut Engine, _: u32) -> LResult<u64> {
    let t = e.r32(L(e) + 0x164);
    let fVar2 = APIextra::TimerGetTime(e, t);
    if fVar2 < 2.0 {
        let h = e.r32(e.r32(GDM(e)) + 8);
        if LIBApiCFunctions::CheckRangeObject(e, 1.0, SELF, h, 0) != 0 {
            play(e, 1, 0.2);
            move_to(e, h);
            APIgameobj::SetContextFlag(e, SELF, 0x66);
            APIgameobj::SetNextState(e, SELF, 0x15);
        }
    } else {
        let t = e.r32(L(e) + 0x164);
        let fVar2 = APIextra::TimerGetTime(e, t);
        if 2.2 < fVar2 {
            CheckHero(e)?;
            if LIBApiCFunctions::CheckRangeObject(e, 3.5, SELF, 0x2a00_00c3, 0) != 0 {
                move_to(e, 0x2a00_00c3);
                play(e, 5, 0.4);
                APIgameobj::SetContextFlag(e, SELF, 0x62);
                if g(e, G_zombie_localST_RETURN) == 1 {
                    APIgameobj::SendMessageTo(e, 0x2a00_00c3, NULL, 0xc1, 0, NULL);
                }
                APIgameobj::SetNextState(e, SELF, 0x20);
            }
        } else {
            play(e, 1, 0.2);
            move_to(e, 0x2a00_00c3);
            APIgameobj::SetContextFlag(e, SELF, 0x66);
        }
    }
    Ok(0)
}

/// The level-4 message function: 0xd6/0x68 captured (hide, freeze, save the group mask, 0xeb to the
/// manager 0x2a000090, save the clipping, state 0xb), 0xc1/0xc9 restart at the start point (0x26 to
/// the HUD 0x2a100006), 0xda knocked back (from chase / end attack / waiting: anim, state 0x32),
/// 0x1fd respawn at 0x2a0000bb walking to 0x2a0000c3 (state 0x21); then zombie_baseMessageFonction.
fn ep4_level4_message(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr, knock_anim: u32, sleep_last: bool) -> LResult<u64> {
    if param_1 == 0xd6 {
        if param_2 == 0x68 {
            APIphysic::SetFlags(e, SELF, 1);
            APIgameobj::SetGameObjectVisible(e, SELF, 0);
            let uVar1 = APIgameobj::GetGroupBitMask(e, SELF);
            e.w32(LM(e) + 0x4c, uVar1);
            APIgameobj::SetGroupBitMask(e, SELF, 0);
            APIgameobj::SendMessageTo(e, 0x2a00_0090, NULL, 0xeb, 1, NULL);
            APIgameobj::SetContextFlag(e, SELF, 0x60);
            let p = LM(e) + 0x120;
            APIgameobj::GetClippingParams(e, SELF, p);
            LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 2, NULL, 0);
            APIgameobj::SetNextState(e, SELF, 0xb);
            return Ok(0);
        }
    } else if param_1 < 0xd7 {
        if param_1 == 0xc1 || param_1 == 0xc9 {
            let p = LM(e) + 0xf8;
            APIgameobj::SetTranslation(e, SELF, p);
            APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 3, 0xc);
            let p = LM(e) + 0x44;
            APIgameobj::SendMessageTo(e, 0x2a10_0006, NULL, 0x26, 1, p);
            let m = e.r32(LM(e) + 0x4c);
            APIgameobj::SetGroupBitMask(e, SELF, m);
            APIgameobj::SendMessageTo(e, 0x2a00_0090, NULL, 0xeb, 1, NULL);
            let id = e.r32(LM(e) + 0x48);
            APIgameobj::SetGroupID(e, SELF, id);
            APIgameobj::SetGameObjectVisible(e, SELF, 1);
            APIgameobj::Sleep(e, SELF, 0.0);
            SetClipping(e)?;
            APIphysic::SetFlags(e, SELF, 6);
            APIphysic::ResetFlags(e, SELF, 1);
            APIgameobj::SetNextState(e, SELF, 0);
            return Ok(0);
        }
    } else if param_1 == 0xda {
        let s = APIgameobj::GetCurrentStateID(e, SELF);
        if s == 0x15 || APIgameobj::GetCurrentStateID(e, SELF) == 0x16 || APIgameobj::GetCurrentStateID(e, SELF) == 0x12 {
            let t = e.r32(L(e) + 0x164);
            APIextra::TimerReset(e, t);
            play(e, knock_anim, 0.4);
            APIgameobj::SetContextFlag(e, SELF, 0x60);
            APIgameobj::SetNextState(e, SELF, 0x32);
        }
    } else if param_1 == 0x1fd {
        let DStack_70 = e.alloca(0xc);
        APIgameobj::GetPosition(e, 0x2a00_00bb, ALL, DStack_70);
        APIgameobj::SetTranslation(e, SELF, DStack_70);
        move_to(e, 0x2a00_00c3);
        APIphysic::SetFlags(e, SELF, 6);
        APIphysic::ResetFlags(e, SELF, 1);
        play(e, 1, 0.2);
        APIgameobj::SetContextFlag(e, SELF, 0x66);
        let t = e.r32(L(e) + 0x16c);
        APIextra::TimerReset(e, t);
        let v = e.r32(L(e) + 0xb0);
        e.w32(L(e) + 0x5c, v);
        let m = e.r32(LM(e) + 0x4c);
        APIgameobj::SetGroupBitMask(e, SELF, m);
        APIgameobj::SendMessageTo(e, 0x2a00_0090, NULL, 0xeb, 1, NULL);
        if sleep_last {
            APIgameobj::SetGameObjectVisible(e, SELF, 1);
            APIgameobj::Sleep(e, SELF, 0.0);
        } else {
            APIgameobj::Sleep(e, SELF, 0.0);
            APIgameobj::SetGameObjectVisible(e, SELF, 1);
        }
        APIgameobj::SetNextState(e, SELF, 0x21);
    }
    zombie_baseMessageFonction(e, param_1, param_2, param_3)
}

/// level-4 constructor: episode four, states 0x32 / 0xb (first instance), tail
fn ep4_level4_ctor(e: &mut Engine, api: Ptr, instanceCount: i32, captured: crate::logic::ExtFn, msg: MsgFn, init: fn(&mut Engine) -> LResult<()>) -> LResult<()> {
    zombieepisodefourConstructor(e, api, instanceCount)?;
    if instanceCount == 1 {
        APIcons::SetExternalFunction(e, 0x32, zombie_localST_RETURN);
        APIcons::SetExternalFunction(e, 0xb, captured);
    }
    level_tail(e, msg, init)
}

/// 0x00017f20 (5e7d92cd)
pub fn ZombieLevelFourB_Func_Initialisation_5e7d92cd(e: &mut Engine) -> LResult<()> {
    init_type_c(e)
}
/// 0x00017f50 (5e7d92cd)
pub fn ZombieLevelFourB_MessageFunction_5e7d92cd(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    ep4_level4_message(e, param_1, param_2, param_3, 4, false)
}
/// 0x00018668 (5e7d92cd)
pub fn zombielevelfourbConstructor_5e7d92cd(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    ep4_level4_ctor(e, api, instanceCount, ZombieLevelFourB_ST_CAPTURED, ZombieLevelFourB_MessageFunction_5e7d92cd, ZombieLevelFourB_Func_Initialisation_5e7d92cd)
}
/// 0x00018748 (5e7d92cd)
pub fn zombielevelfourbDestructor_5e7d92cd(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    zombieepisodefourDestructor(e, instanceCount)
}

/// (zombielevelfourc)
pub fn ZombieLevelFourC_Func_Initialisation(e: &mut Engine) -> LResult<()> {
    init_type_c(e)
}
/// (zombielevelfourc) knock-back anim 3
pub fn ZombieLevelFourC_MessageFunction(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    ep4_level4_message(e, param_1, param_2, param_3, 3, true)
}
/// (zombielevelfourc)
pub fn zombielevelfourcConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    ep4_level4_ctor(e, api, instanceCount, ZombieLevelFourC_ST_CAPTURED, ZombieLevelFourC_MessageFunction, ZombieLevelFourC_Func_Initialisation)
}
/// (zombielevelfourc)
pub fn zombielevelfourcDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    zombieepisodefourDestructor(e, instanceCount)
}

/// (zombielevelfourd)
pub fn ZombieLevelFourD_Func_Initialisation(e: &mut Engine) -> LResult<()> {
    init_type_c(e)
}
/// (zombielevelfourd)
pub fn ZombieLevelFourD_MessageFunction(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    ep4_level4_message(e, param_1, param_2, param_3, 4, true)
}
/// (zombielevelfourd)
pub fn zombielevelfourdConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    ep4_level4_ctor(e, api, instanceCount, ZombieLevelFourD_ST_CAPTURED, ZombieLevelFourD_MessageFunction, ZombieLevelFourD_Func_Initialisation)
}
/// (zombielevelfourd)
pub fn zombielevelfourdDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    zombieepisodefourDestructor(e, instanceCount)
}

// ---- ZombieLevelTwo.c (zombieleveltwo, Ep6 L02) ----

/// 0x00017a68 state 0x11: no object 0x2a0001ac -> wander; then the base WAITING_SET
pub fn zombie_leveltwoST_WAITING_SET(e: &mut Engine, param_1: u32) -> LResult<u64> {
    wander_if_no_1ac(e);
    zombie_baseST_WAITING_SET(e, param_1)
}

/// 0x00017b00
pub fn ZombieLevelTwo_Func_Initialisation(e: &mut Engine) -> LResult<()> {
    init_type_c(e)
}

/// 0x00017b30 0xde (cutscene over): walk anim, physics back on, clipping, wake up; 0xc9 / 0xc1: no
/// object 0x2a0001ac -> wander; then zombie_baseMessageFonction
pub fn ZombieLevelTwo_MessageFunction(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    if param_1 == 0xc9 || param_1 == 0xc1 {
        wander_if_no_1ac(e);
    } else if param_1 == 0xde {
        APIphysic::SetFlags(e, SELF, 6);
        APIphysic::ResetFlags(e, SELF, 1);
        play(e, 1, 0.4);
        APIgameobj::SetContextFlag(e, SELF, 0x6c);
        SetClipping(e)?;
        APIgameobj::Sleep(e, SELF, 0.0);
    }
    zombie_baseMessageFonction(e, param_1, param_2, param_3)
}

/// 0x00017ce8
pub fn zombieleveltwoConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    zombieepisodesixConstructor(e, api, instanceCount)?;
    if instanceCount == 1 {
        APIcons::SetExternalFunction(e, 0x11, zombie_leveltwoST_WAITING_SET);
    }
    level_tail(e, ZombieLevelTwo_MessageFunction, ZombieLevelTwo_Func_Initialisation)
}

/// 0x00017db0
pub fn zombieleveltwoDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    zombieepisodesixDestructor(e, instanceCount)
}

// ---- Ep6 L04A (ZombieLevelFourA_B/C/D.c) and L04B (ZombieLevelFourB/B_C/B_D.c) ----
// Each is `init type 0xc; message = zombie_baseMessageFonction; episode six constructor`; the L04B
// files also set the wall-ray mask 0x1008 like zombieleveltwoa.

macro_rules! ep6_level {
    ($init:ident, $msg:ident, $ctor:ident, $dtor:ident, $ray:expr, $b:literal) => {
        #[doc = concat!("(", $b, ")")]
        pub fn $init(e: &mut Engine) -> LResult<()> {
            init_type_c(e)
        }
        #[doc = concat!("(", $b, ")")]
        pub fn $msg(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
            zombie_baseMessageFonction(e, param_1, param_2, param_3)
        }
        #[doc = concat!("(", $b, ")")]
        pub fn $ctor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
            zombieepisodesixConstructor(e, api, instanceCount)?;
            level_tail(e, $msg, $init)?;
            if $ray {
                ray_mask_1008(e);
            }
            Ok(())
        }
        #[doc = concat!("(", $b, ")")]
        pub fn $dtor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
            zombieepisodesixDestructor(e, instanceCount)
        }
    };
}

ep6_level!(ZombieLevelFourA_B_Func_Initialisation, ZombieLevelFourA_B_MessageFunction, zombielevelfoura_bConstructor, zombielevelfoura_bDestructor, false, "zombielevelfoura_b");
ep6_level!(ZombieLevelFourA_C_Func_Initialisation, ZombieLevelFourA_C_MessageFunction, zombielevelfoura_cConstructor, zombielevelfoura_cDestructor, false, "zombielevelfoura_c");
ep6_level!(ZombieLevelFourA_D_Func_Initialisation, ZombieLevelFourA_D_MessageFunction, zombielevelfoura_dConstructor, zombielevelfoura_dDestructor, false, "zombielevelfoura_d");
ep6_level!(ZombieLevelFourB_Func_Initialisation_35a2d522, ZombieLevelFourB_MessageFunction_35a2d522, zombielevelfourbConstructor_35a2d522, zombielevelfourbDestructor_35a2d522, true, "35a2d522");
ep6_level!(ZombieLevelFourB_C_Func_Initialisation, ZombieLevelFourB_C_MessageFunction, zombielevelfourb_cConstructor, zombielevelfourb_cDestructor, true, "zombielevelfourb_c");
ep6_level!(ZombieLevelFourB_D_Func_Initialisation, ZombieLevelFourB_D_MessageFunction, zombielevelfourb_dConstructor, zombielevelfourb_dDestructor, true, "zombielevelfourb_d");

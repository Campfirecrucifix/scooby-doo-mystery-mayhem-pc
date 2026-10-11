//! hero_tome.c (9 functions): the heroes' side of the Tome of Doom. Addresses are module-relative
//! (.text at 0x10000) in scooby__402420b4.
//!
//! State flow (states are code pointers in local+0x154 = next init, local+0x158 = loop):
//! `heroInit_TomeTrack` (walk to the book's bone 9/10) -> `heroST_TomeTrack_Loop` -> in range:
//! message 0x76 to the book, `heroInit_TomeIn` (open anim) -> `heroST_TomeIn_Loop` -> anim end:
//! `heroInit_Tome` (holding loop) -> `heroST_Tome_Loop` -> ... `heroInit_TomeOut` (close anim) ->
//! `heroST_TomeOut_Loop` -> anim end: stand (leader) / follow-stand (follower).
//!
//! Per-build differences are only literals (object ids, anim ids; builds.rs slots), the debug prints
//! (debug builds only, character name per build) and Shaggy's blend function; the c7b040f3 build of
//! heroST_Tome_Loop uses negative anim ids (hard-coded below).
use super::builds::*;
use super::*;
use crate::engine::*;
use crate::logic::LResult;

/// `*(int *)(*pGlobalDataHero + 8) == *(int *)(pLocalDataHero + 0x17c)`: this hero is the leader.
fn is_leader(e: &Engine) -> bool {
    e.r32(GB0(e) + 8) == e.r32(L(e) + 0x17c)
}

/// 0x00017a88 (476 bytes) `void heroInit_TomeIn(int param_1)`
///
/// Opening the book: leader/follower open anim (time 0.1, flags 1, notify 5), clears attribute bit 4
/// of both heroes' prims (if present), context flag 0x50, input state 0x80000000.
pub fn heroInit_TomeIn(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    debug_time(e, "heroInit_TomeIn", 0, 0)?;
    let anim = if is_leader(e) { k(e, heroInit_TomeIn_0_1) } else { k(e, heroInit_TomeIn_0_2) };
    let r = heroFUNC_StartAnim(e, 0.1, anim as i32, 1, 5)?;
    e.w32(param_1 + 0x218, r as u32);
    if APIgameobj::GetID(e, k(e, heroInit_TomeIn_0_3), 0) != 0x7fff_fffe {
        APIphysic::PrimResetIntAttribBits(e, k(e, heroInit_TomeIn_0_4), 100, 2, 4);
    }
    if APIgameobj::GetID(e, k(e, heroInit_TomeIn_0_5), 0) != 0x7fff_fffe {
        APIphysic::PrimResetIntAttribBits(e, k(e, heroInit_TomeIn_0_6), 0x6e, 2, 4);
    }
    APIgameobj::SetContextFlag(e, SELF, 0x50);
    APIinput::InputConfigSetState(e, e.r32(param_1 + 0x188) as i32, i32::MIN);
    e.w32(param_1 + 0x158, addr::heroST_TomeIn_Loop);
    Ok(())
}

/// 0x00017c68 (80 bytes) `void heroST_TomeIn_Loop(int param_1)` (same in every build)
pub fn heroST_TomeIn_Loop(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    // local+0x168 bit 0: the current anim ended
    if e.r32(param_1 + 0x168) & 1 != 0 {
        e.w32(param_1 + 0x154, addr::heroInit_Tome);
    }
    Ok(())
}

/// 0x00017cb8 (180 bytes) `void heroInit_Tome(int param_1)`: the book-holding loop anim.
pub fn heroInit_Tome(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    let r = heroFUNC_StartAnim(e, 0.1, k(e, heroInit_Tome_0_1) as i32, 2, 0)?;
    e.w32(param_1 + 0x218, r as u32);
    APIgameobj::SetContextFlag(e, SELF, 0x50);
    APIinput::InputConfigSetState(e, e.r32(param_1 + 0x188) as i32, -0x7fff_fff8);
    e.w32(param_1 + 0x158, addr::heroST_Tome_Loop);
    Ok(())
}

/// 0x00017d70 (420 bytes) `void heroST_Tome_Loop(int param_1)`
///
/// local+0x164 bit 0x10 / 0x20 (set by the message handler) switch between the two holding anims;
/// the follower asks the leader (message 0x6e) and ORs the answer into the flags.
pub fn heroST_Tome_Loop(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    let gi = G_heroST_Tome_Loop;
    // (anim compared for 0x10, anim compared for 0x20, anim started for 0x20, anim started for 0x10)
    let (c10, c20, a20, a10) = match g(e, gi) {
        0 => (
            k(e, heroST_Tome_Loop_0_1),
            k(e, heroST_Tome_Loop_0_2),
            k(e, heroST_Tome_Loop_0_3),
            k(e, heroST_Tome_Loop_0_4),
        ),
        // c7b040f3: the build's anim table has no such anims (negative ids, PlayAnim returns -1)
        1 => ((-0x23i32) as u32, (-0x33i32) as u32, 0xffff_ffcd, 0xffff_ffdd),
        _ => unported_shape(e, "heroST_Tome_Loop", gi),
    };
    let f = e.r32(param_1 + 0x164);
    if f & 0x10 == 0 || e.r32(param_1 + 0x218) == c10 {
        if f & 0x20 != 0 && e.r32(param_1 + 0x218) != c20 {
            e.w32(param_1 + 0x164, f & 0xffff_ffef);
            let r = heroFUNC_StartAnim(e, 0.1, a20 as i32, 2, 0)?;
            e.w32(param_1 + 0x218, r as u32);
        }
    } else {
        e.w32(param_1 + 0x164, f & 0xffff_ffdf);
        let r = heroFUNC_StartAnim(e, 0.1, a10 as i32, 2, 0)?;
        e.w32(param_1 + 0x218, r as u32);
    }
    if !is_leader(e) && e.r32(param_1 + 0x164) & 0x30 == 0 {
        let leader = e.r32(GB0(e) + 8);
        let r = APIgameobj::SendMessageTo(e, leader, 0, 0x6e, 0, 0);
        let v = e.r32(param_1 + 0x164) | r;
        e.w32(param_1 + 0x164, v);
    }
    Ok(())
}

/// 0x00017f18 (636 bytes) `void heroInit_TomeOut(int param_1)`
///
/// Closing the book: close anim, physics flag 1 off, input state 0x80000000, sets attribute bit 4 of
/// both heroes' prims again, DM params[0] of local+0x1b0 = 50.0.
pub fn heroInit_TomeOut(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    let anim = if is_leader(e) { k(e, heroInit_TomeOut_0_1) } else { k(e, heroInit_TomeOut_0_2) };
    let r = heroFUNC_StartAnim(e, 0.1, anim as i32, 1, 5)?;
    e.w32(param_1 + 0x218, r as u32);
    APIgameobj::SetContextFlag(e, SELF, 0x50);
    APIphysic::ResetFlags(e, SELF, 1);
    APIinput::InputConfigSetState(e, e.r32(param_1 + 0x188) as i32, i32::MIN);
    if APIgameobj::GetID(e, k(e, heroInit_TomeOut_0_3), 0) != 0x7fff_fffe {
        let v = APIphysic::PrimGetIntAttrib(e, k(e, heroInit_TomeOut_0_4), 100, 2);
        APIphysic::PrimSetIntAttribBits(e, k(e, heroInit_TomeOut_0_5), 100, 2, v | 4);
    }
    if APIgameobj::GetID(e, k(e, heroInit_TomeOut_0_6), 0) != 0x7fff_fffe {
        let v = APIphysic::PrimGetIntAttrib(e, k(e, heroInit_TomeOut_0_7), 0x6e, 2);
        APIphysic::PrimSetIntAttribBits(e, k(e, heroInit_TomeOut_0_8), 0x6e, 2, v | 4);
    }
    let p = e.alloca(0x10); // undefined4 auStack_40[4]
    APIphysic::DMGetParams(e, e.r32(param_1 + 0x1b0), p);
    e.w32(p, 0x4248_0000);
    APIphysic::DMSetParams(e, e.r32(param_1 + 0x1b0), p);
    e.w32(param_1 + 0x158, addr::heroST_TomeOut_Loop);
    Ok(())
}

/// 0x00018198 (140 bytes) `void heroST_TomeOut_Loop(int param_1)` (same in every build)
pub fn heroST_TomeOut_Loop(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    if e.r32(param_1 + 0x168) & 1 != 0 {
        let next = if is_leader(e) { addr::heroInit_Stand } else { addr::heroInit_FollowStand };
        e.w32(param_1 + 0x154, next);
    }
    Ok(())
}

/// Book bone the hero walks to: 10 when the leader is the first hero id, else 9.
fn track_bone(e: &mut Engine, leader_id_slot: usize) -> u32 {
    if e.r32(GB0(e) + 8) == k(e, leader_id_slot) {
        10
    } else {
        9
    }
}

/// Points the hero's move-to DM (local+0x1d8) at `pos`: { 0, 12.5, x, y, z, 5 }.
fn set_track_target(e: &mut Engine, param_1: Ptr, pos: Ptr) {
    let p = e.alloca(0x20);
    APIphysic::DMGetParams(e, e.r32(param_1 + 0x1d8), p);
    e.w32(p, 0);
    let (x, y, z) = (e.r32(pos), e.r32(pos + 4), e.r32(pos + 8));
    e.w32(p + 8, x);
    e.w32(p + 0xc, y);
    e.w32(p + 0x10, z);
    e.w32(p + 4, 0x4148_0000);
    e.w32(p + 0x14, 5);
    APIphysic::DMSetParams(e, e.r32(param_1 + 0x1d8), p);
}

/// 0x00018228 (708 bytes) `void heroInit_TomeTrack(int param_1)`
///
/// Walk to the open book: target its bone 9/10, speed DM (local+0x1a8) = max speed (local+0x70),
/// run blend, 0.05 s grace before IsOutOfTime.
pub fn heroInit_TomeTrack(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    debug_time(e, "heroInit_TomeTrack", 0, 0)?;
    let pos = e.alloca(0x10); // DTVector3d DStack_50
    let bone = track_bone(e, heroInit_TomeTrack_0_1);
    let book = if bone == 10 { k(e, heroInit_TomeTrack_0_2) } else { k(e, heroInit_TomeTrack_0_3) };
    APIgameobj::GetPosition(e, book, bone, pos);
    set_track_target(e, param_1, pos);
    let p = e.alloca(0x10); // undefined1 auStack_60[8]; uStack_58; uStack_54
    APIphysic::DMGetParams(e, e.r32(param_1 + 0x1a8), p);
    e.w32(p + 0xc, 1);
    let v = e.r32(param_1 + 0x70);
    e.w32(p + 8, v);
    APIphysic::DMSetParams(e, e.r32(param_1 + 0x1a8), p);
    APIphysic::PrimResetIntAttribBits(e, SELF, e.r32(param_1 + 0x18c), 2, 4);
    // 0x3ca3d70a = 0.02
    let r = heroFUNC_StartBlend(e, 0.02, 1, 2, 0)?;
    e.w32(param_1 + 0x218, r as u32);
    APIgameobj::SetContextFlag(e, SELF, 0x3000_2050);
    UpdateTimerOutOfTime(e, 0.05)?;
    debug_info_value(e, "Init_TomeTrack - time", param_1 + 0x2dc, 2);
    e.w32(param_1 + 0x158, addr::heroST_TomeTrack_Loop);
    let p = e.alloca(0x10); // undefined4 auStack_40[4]
    APIphysic::DMGetParams(e, e.r32(param_1 + 0x1b0), p);
    e.w32(p, 0x4248_0000);
    APIphysic::DMSetParams(e, e.r32(param_1 + 0x1b0), p);
    Ok(())
}

/// 0x000184f0 (476 bytes) `void heroST_TomeTrack_Loop(int param_1)`
///
/// Re-targets the book bone every frame; once the grace time is over and the hero is within 0.6 of
/// it, tells the book (message 0x76) and goes to heroInit_TomeIn. Then updates the walk/run blend
/// (Scooby / Shaggy function per build).
pub fn heroST_TomeTrack_Loop(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    let gi = G_heroST_TomeTrack_Loop;
    let grp = g(e, gi);
    if grp > 3 {
        unported_shape(e, "heroST_TomeTrack_Loop", gi);
    }
    let bone = track_bone(e, heroST_TomeTrack_Loop_0_1);
    let pos = e.alloca(0x10); // DTVector3d DStack_50
    APIgameobj::GetPosition(e, k(e, heroST_TomeTrack_Loop_0_2), bone, pos);
    set_track_target(e, param_1, pos);
    if IsOutOfTime(e)? != 0 && LIBApiCFunctions::CheckRangePoint(e, 0.6, SELF, pos, 1) != 0 {
        debug_info(e, "\"ST_TOME -> IN RANGE\"");
        APIgameobj::SendMessageTo(e, k(e, heroST_TomeTrack_Loop_0_3), 0, 0x76, 0, 0);
        e.w32(param_1 + 0x154, addr::heroInit_TomeIn);
    }
    if grp < 2 {
        heroFUNC_UpdateBlend_Scooby(e, param_1, 1)?;
    } else {
        heroFUNC_UpdateBlend_Others(e, param_1, 1)?;
    }
    Ok(())
}

/// 0x000186d0 (260 bytes) `undefined4 heroValidOpenTome(void)`
///
/// 1 when the book may be opened: game-buffer-2 flag bit 0 set, the "no tome" flag (game buffer 0
/// +0x14 bit 0x100) clear, and the book object answers message 0xc6 with 0. The floor attributes
/// are fetched but not used.
pub fn heroValidOpenTome(e: &mut Engine) -> LResult<u32> {
    debug_info(e, "heroValidOpenTome");
    let attribs = e.alloca(0x40); // EnvPrimAttribs aEStack_40[32]
    APIphysic::GetFloorAttribs(e, SELF, attribs);
    if e.r32(GB2(e)) & 1 == 0 || e.r32(GB0(e) + 0x14) & 0x100 != 0 {
        return Ok(0);
    }
    if APIgameobj::SendMessageTo(e, k(e, heroValidOpenTome_0_1), 0, 0xc6, 0, 0) != 0 {
        return Ok(0);
    }
    Ok(1)
}

//! The level-specific part of scooby.c / shaggy.c in the 15 non-race level builds (doors, rotating doors,
//! go-to-point, sawing, aqueduct, pushing pipe, carrousel / track bumps, locked door). These are level
//! gameplay, not book code; scooby.rs dispatches here by build.
//!
//! Code pointers: the reference build has none of these states, so `addr_lv` gives each a code address
//! outside the reference module's range (0x200000 + the function's address in the first build that has it;
//! BumpByCarrousel / Bump use 0x2f0000.. since only their relative order matters). As with `addr_pu`, they are
//! only ever stored into local+0x154/0x158/0x15c, compared and called through `call_code`.
//!
//! The two race builds (c33ca2c4, 0117f115: hero_race.c) are NOT here yet.
use super::skeleton::addr;
use super::*;
use crate::engine::*;
use crate::logic::LResult;

const SELF: u32 = 0x2a00_0000;
const NULL: Ptr = 0;

#[allow(non_upper_case_globals)]
pub mod addr_lv {
    pub const heroInit_OpenDoor: u32 = 0x22abd8; // 5257b080
    pub const heroST_OpenDoor_Loop: u32 = 0x22ac90;
    pub const heroInit_RotatingDoor: u32 = 0x22b420; // 5f68ee0e
    pub const heroST_RotatingDoor_Loop: u32 = 0x22b598;
    pub const heroInit_GoToPoint: u32 = 0x22dd80; // 242c6a48
    pub const heroInit_GoToPoint_Loop: u32 = 0x22df40;
    pub const heroInit_Sawing: u32 = 0x22dda8; // 11adae8e
    pub const heroST_Sawing_Loop: u32 = 0x22dee0;
    pub const heroInit_PushingPipe: u32 = 0x22f8b0; // 7e698dff
    pub const heroST_PushingPipe_Loop: u32 = 0x22fa88;
    pub const heroExit_PushingPipe: u32 = 0x22fbf0;
    pub const heroInit_BumpByCarrousel: u32 = 0x2f0000;
    pub const heroST_BumpByCarrousel_Loop: u32 = 0x2f0010;
    pub const heroInit_Bump: u32 = 0x2f0020;
    pub const heroST_Bump_Loop: u32 = 0x2f0030;
}

/// The code pointers of this file (see `skeleton::call_code`).
pub(crate) fn call_level(e: &mut Engine, f: u32, a0: Ptr) -> Option<LResult<()>> {
    Some(match f {
        addr_lv::heroInit_OpenDoor => heroInit_OpenDoor(e, a0),
        addr_lv::heroST_OpenDoor_Loop => heroST_OpenDoor_Loop(e, a0),
        addr_lv::heroInit_RotatingDoor => heroInit_RotatingDoor(e, a0),
        addr_lv::heroST_RotatingDoor_Loop => heroST_RotatingDoor_Loop(e, a0),
        addr_lv::heroInit_GoToPoint => heroInit_GoToPoint(e, a0),
        addr_lv::heroInit_GoToPoint_Loop => heroInit_GoToPoint_Loop(e, a0),
        addr_lv::heroInit_Sawing => heroInit_Sawing(e, a0),
        addr_lv::heroST_Sawing_Loop => heroST_Sawing_Loop(e, a0),
        addr_lv::heroInit_PushingPipe => heroInit_PushingPipe(e, a0),
        addr_lv::heroST_PushingPipe_Loop => heroST_PushingPipe_Loop(e, a0),
        addr_lv::heroExit_PushingPipe => heroExit_PushingPipe(e),
        addr_lv::heroInit_BumpByCarrousel => heroInit_BumpByCarrousel(e, a0),
        addr_lv::heroST_BumpByCarrousel_Loop => heroST_BumpByCarrousel_Loop(e, a0),
        addr_lv::heroInit_Bump => heroInit_Bump(e, a0),
        addr_lv::heroST_Bump_Loop => heroST_Bump_Loop(e, a0),
        _ => return None,
    })
}

/// The builds of this file.
pub(crate) const LEVEL_BUILDS: [&str; 15] = [
    "5257b080", "5f68ee0e", "53df9ee8", "6dafd227", "ffd170d9", "242c6a48", "11adae8e", "562b9dae", "7e698dff",
    "160eeb3e", "9e7ce829", "cbbf384c", "d8a08bbd", "dca9fe48", "26992d64",
];

fn sha(e: &Engine) -> &'static str {
    builds::BUILDS[e.rt.cur_build()].sha8
}
/// `pLocalDataHeroLevel`
fn LL(e: &Engine) -> Ptr {
    e.r32(G(e, pLocalDataHeroLevel))
}
/// `*(int *)(*pGlobalDataHero + 8) == *(int *)(pLocalDataHero + 0x17c)`: this hero is the leader
fn leader(e: &Engine) -> bool {
    e.r32(GB0(e) + 8) == e.r32(L(e) + 0x17c)
}
fn or_w(e: &mut Engine, a: Ptr, v: u32) {
    let x = e.r32(a) | v;
    e.w32(a, x);
}
fn and_w(e: &mut Engine, a: Ptr, v: u32) {
    let x = e.r32(a) & v;
    e.w32(a, x);
}
fn prim(e: &mut Engine, set: bool, obj: u32, p: u32, bits: u32) {
    if set {
        APIphysic::PrimSetIntAttribBits(e, obj, p, 5, bits);
    } else {
        APIphysic::PrimResetIntAttribBits(e, obj, p, 5, bits);
    }
}
fn hero_prim(e: &Engine) -> u32 {
    e.r32(L(e) + 0x18c)
}
fn stand_or_follow(e: &mut Engine, param_1: Ptr) {
    let n = if leader(e) { addr::heroInit_Stand } else { addr::heroInit_FollowStand };
    e.w32(param_1 + 0x154, n);
}

// ---- heroFUNC_CheckActionEventsLocal ----

/// `heroFUNC_CheckActionEventsLocal` of the level builds (local+0x170 bits 0x10000000.. set by the
/// level messages below).
pub(crate) fn check_action_events_local(e: &mut Engine) -> LResult<()> {
    let l = L(e);
    let ev = e.r32(l + 0x170);
    // the leader tells its partner to follow (message 0x60) before the push states
    let door_push = |e: &mut Engine, init: u32| {
        if leader(e) {
            let p = e.r32(L(e) + 0x180);
            APIgameobj::SendMessageTo(e, p, NULL, 0x60, 0x1000_0000, NULL);
        }
        e.w32(l + 0x154, init);
    };
    match sha(e) {
        "5257b080" if ev & 0x1000_0000 != 0 => e.w32(l + 0x154, addr_lv::heroInit_OpenDoor),
        "5f68ee0e" | "53df9ee8" if ev & 0x1000_0000 != 0 => door_push(e, addr_lv::heroInit_RotatingDoor),
        "7e698dff" | "160eeb3e" if ev & 0x1000_0000 != 0 => door_push(e, addr_lv::heroInit_PushingPipe),
        "242c6a48" if ev & 0x1000_0000 != 0 => e.w32(l + 0x154, addr_lv::heroInit_GoToPoint),
        "11adae8e" | "562b9dae" if ev & 0x1000_0000 != 0 => e.w32(l + 0x154, addr_lv::heroInit_Sawing),
        "9e7ce829" | "cbbf384c" => {
            if ev & 0x2000_0000 != 0 {
                e.w32(l + 0x154, addr_lv::heroInit_BumpByCarrousel);
            } else if ev & 0x4000_0000 != 0 {
                if leader(e) {
                    and_w(e, GB0(e) + 0x14, 0xfeff_ffff);
                    e.w32(l + 0x154, addr::heroInit_Stand);
                } else {
                    e.w32(l + 0x154, addr::heroInit_FollowStand);
                }
            }
        }
        "6dafd227" | "ffd170d9" => {
            if ev & 0x1000_0000 != 0 {
                door_push(e, addr_lv::heroInit_RotatingDoor);
            } else if ev & 0x2000_0000 != 0 {
                e.w32(l + 0x154, addr_lv::heroInit_Bump);
            } else if ev & 0x4000_0000 != 0 {
                and_w(e, GB0(e) + 0x14, 0xffff_feff);
                stand_or_follow(e, l);
            }
        }
        _ => {}
    }
    Ok(())
}

// ---- level states ----

/// 5257b080: plays anim 0x3c (debug label "heroInit_TomeIn" as in the original).
pub fn heroInit_OpenDoor(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    super::hero_misc::debug_time(e, "heroInit_TomeIn", 0, 0)?;
    let r = LIBAnimMngr::PlayAnim(e, 0.1, 0x3c, 1, 5);
    e.w32(param_1 + 0x218, r as u32);
    APIgameobj::SetContextFlag(e, SELF, 0x50);
    e.w32(param_1 + 0x158, addr_lv::heroST_OpenDoor_Loop);
    Ok(())
}

/// At the anim end: the leader stands and opens the door (message 300 to 0x2a00011c), the partner follows.
pub fn heroST_OpenDoor_Loop(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    if e.r32(param_1 + 0x168) & 1 != 0 {
        if leader(e) {
            e.w32(param_1 + 0x154, addr::heroInit_Stand);
            APIgameobj::SendMessageTo(e, 0x2a00_011c, NULL, 300, 0, NULL);
        } else {
            e.w32(param_1 + 0x154, addr::heroInit_FollowStand);
        }
    }
    Ok(())
}

pub fn heroInit_RotatingDoor(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    if leader(e) {
        let r = if e.r32(LL(e) + 0x388) != 0 { 3.1415927 } else { 0.0 };
        let door = e.r32(param_1 + 0x23c) as i32;
        LIBApiCFunctions::TakeCoorNRotation(e, r, SELF, door, 4, NULL);
        let p = e.r32(param_1 + 0x180);
        LIBApiCFunctions::TakeCoorNRotation(e, r, p, door, 5, NULL);
    }
    or_w(e, GB0(e) + 0x14, 0x100);
    APIgameobj::SetContextFlag(e, SELF, 0x80_0000);
    let anim = match sha(e) {
        "5f68ee0e" => 0x39,
        "53df9ee8" => 0x36,
        "6dafd227" => 0x33,
        _ => 0x31, // ffd170d9
    };
    let r = heroFUNC_StartAnim(e, 0.0, anim, 1, 5)?;
    e.w32(param_1 + 0x218, r as u32);
    APIinput::InputConfigSetState(e, e.r32(param_1 + 0x188) as i32, i32::MIN);
    e.w32(param_1 + 0x158, addr_lv::heroST_RotatingDoor_Loop);
    Ok(())
}

pub fn heroST_RotatingDoor_Loop(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    if e.r32(param_1 + 0x168) & 1 != 0 {
        and_w(e, GB0(e) + 0x14, 0xffff_feff);
        stand_or_follow(e, param_1);
    }
    Ok(())
}

/// 242c6a48: walk (DM 0x1d8 target = position of 0x2a000113, speed 12.5) then busy.
pub fn heroInit_GoToPoint(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    // frame: DStack_70 +0, auStack_60 +0x10, uStack_50 +0x20
    let f = e.alloca(0x40);
    APIgameobj::GetPosition(e, 0x2a00_0113, 0x7fff_ffff, f);
    let a = f + 0x10;
    APIphysic::DMGetParams(e, e.r32(param_1 + 0x1a8), a);
    e.w32(a + 0xc, 1);
    e.w32(a + 8, e.r32(param_1 + 0x70));
    APIphysic::DMSetParams(e, e.r32(param_1 + 0x1a8), a);
    let b = f + 0x20;
    APIphysic::DMGetParams(e, e.r32(param_1 + 0x1d8), b);
    e.w32(b, 0);
    for i in 0..3 {
        let v = e.r32(f + i * 4);
        e.w32(b + 8 + i * 4, v);
    }
    e.w32(b + 4, 0x4148_0000);
    e.w32(b + 0x14, 5);
    APIphysic::DMSetParams(e, e.r32(param_1 + 0x1d8), b);
    let r = heroFUNC_StartAnim(e, 0.0, 0x17, 2, 0)?;
    e.w32(param_1 + 0x218, r as u32);
    or_w(e, GB0(e) + 0x14, 0x100);
    APIgameobj::SetContextFlag(e, SELF, 0x3000_1050);
    APIinput::InputConfigSetState(e, e.r32(param_1 + 0x188) as i32, i32::MIN);
    e.w32(param_1 + 0x158, addr_lv::heroInit_GoToPoint_Loop);
    Ok(())
}

pub fn heroInit_GoToPoint_Loop(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    if LIBApiCFunctions::CheckRangeObject(e, 0.5, SELF, 0x2a00_0113, 0) != 0 {
        e.w32(param_1 + 0x154, addr::heroInit_Busy);
    }
    Ok(())
}

/// 11adae8e / 562b9dae: snap to the saw object (local+0x23c) and play the sawing anim.
pub fn heroInit_Sawing(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    let v = e.alloca(0x10);
    let o = e.r32(param_1 + 0x23c);
    let p = APIgameobj::GetPosition(e, o, 1, v);
    APIgameobj::SetTranslation(e, SELF, p);
    let o = e.r32(param_1 + 0x23c);
    APIgameobj::TakeOrientation(e, SELF, o, 1);
    let anim = if sha(e) == "11adae8e" { 0x3a } else { 0x33 };
    let r = heroFUNC_StartAnim(e, 0.0, anim, 1, 5)?;
    e.w32(param_1 + 0x218, r as u32);
    or_w(e, GB0(e) + 0x14, 0x100);
    APIgameobj::SetContextFlag(e, SELF, 0x50);
    APIinput::InputConfigSetState(e, e.r32(param_1 + 0x188) as i32, i32::MIN);
    e.w32(param_1 + 0x158, addr_lv::heroST_Sawing_Loop);
    Ok(())
}

/// Anim flags: 0x100 attach the saw (0x2a000145) to bone 0x1001000, 0x200 drop it; end: message 0xe5 to
/// the sawn object, stand.
pub fn heroST_Sawing_Loop(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    let f = e.r32(param_1 + 0x168);
    if f & 1 == 0 {
        if f & 0x100 == 0 {
            if f & 0x200 != 0 {
                APIgameobj::LeaveParent(e, 0x2a00_0145, 1);
                APIgameobj::SetGameObjectVisible(e, 0x2a00_0145, 0);
            }
        } else {
            APIgameobj::AddChild(e, SELF, 0x100_1000, 0x2a00_0145, 1);
            APIgameobj::SetGameObjectVisible(e, 0x2a00_0145, 1);
        }
    } else {
        let o = e.r32(param_1 + 0x23c);
        APIgameobj::SendMessageTo(e, o, NULL, 0xe5, 0, NULL);
        and_w(e, GB0(e) + 0x14, 0xffff_feff);
        e.w32(param_1 + 0x154, addr::heroInit_Stand);
    }
    Ok(())
}

/// 7e698dff / 160eeb3e. Push direction (level local+0x388): 1 = pi, 2 = -pi/2 (the build's 0xbfc90fdb),
/// otherwise 0.
pub fn heroInit_PushingPipe(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    if leader(e) {
        let r = match e.r32(LL(e) + 0x388) {
            1 => f32::from_bits(0x4049_0fdb),
            2 => f32::from_bits(0xbfc9_0fdb),
            _ => 0.0,
        };
        let pipe = e.r32(param_1 + 0x23c) as i32;
        LIBApiCFunctions::TakeCoorNRotation(e, r, SELF, pipe, 2, NULL);
        let p = e.r32(param_1 + 0x180);
        LIBApiCFunctions::TakeCoorNRotation(e, r, p, pipe, 3, NULL);
        or_w(e, GB0(e) + 0x14, 0x100);
        e.w32(param_1 + 0x15c, addr_lv::heroExit_PushingPipe);
    }
    APIgameobj::SetContextFlag(e, SELF, 0x80_0000);
    let anim = if sha(e) == "7e698dff" { 0x38 } else { 0x34 };
    let r = heroFUNC_StartAnim(e, 0.0, anim, 1, 5)?;
    e.w32(param_1 + 0x218, r as u32);
    APIinput::InputConfigSetState(e, e.r32(param_1 + 0x188) as i32, i32::MIN);
    e.w32(param_1 + 0x158, addr_lv::heroST_PushingPipe_Loop);
    Ok(())
}

/// While pushing the leader's pad vibrates every 0.25 s (local+0x378 = next time).
pub fn heroST_PushingPipe_Loop(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    if e.r32(param_1 + 0x168) & 1 == 0 {
        if leader(e) && e.rf(param_1 + 0x378) < APIextra::TimerGetTime(e, 0x9000_0000) {
            let t = APIextra::TimerGetTime(e, 0x9000_0000);
            e.wf(param_1 + 0x378, t + 0.25);
            heroFUNC_CallVibration(e, f32::from_bits(0x3ea8_f5c3), 0.0, 0.25, 0.25)?;
        }
    } else {
        APIextra::TimerReset(e, e.r32(param_1 + 0x20c));
        stand_or_follow(e, param_1);
    }
    Ok(())
}

pub fn heroExit_PushingPipe(e: &mut Engine) -> LResult<()> {
    if leader(e) {
        and_w(e, GB0(e) + 0x14, 0xffff_feff);
    }
    Ok(())
}

/// The DM setup shared by heroInit_BumpByCarrousel / heroInit_Bump: DM 0x1d8 goes to the bump waypoint
/// (local+0x2e4..0x2ec) at 12.5, DM 0x1a8 speed = local+0x74.
fn bump_dm(e: &mut Engine, param_1: Ptr) {
    // frame: uStack_60 +0, auStack_40 +0x20
    let f = e.alloca(0x40);
    APIphysic::DMGetParams(e, e.r32(param_1 + 0x1d8), f);
    e.w32(f, 0);
    for i in 0..3 {
        let v = e.r32(param_1 + 0x2e4 + i * 4);
        e.w32(f + 8 + i * 4, v);
    }
    e.w32(f + 4, 0x4148_0000);
    e.w32(f + 0x14, 5);
    APIphysic::DMSetParams(e, e.r32(param_1 + 0x1d8), f);
    let a = f + 0x20;
    APIphysic::DMGetParams(e, e.r32(param_1 + 0x1a8), a);
    e.w32(a + 0xc, 1);
    e.w32(a + 8, e.r32(param_1 + 0x74));
    APIphysic::DMSetParams(e, e.r32(param_1 + 0x1a8), a);
    debug_info_value(e, "heroInit_Bump -> wp\n", f + 8, 4);
}

pub fn heroInit_BumpByCarrousel(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    bump_dm(e, param_1);
    if leader(e) {
        heroFUNC_CallVibration(e, f32::from_bits(0x3f28_f5c3), 0.0, f32::from_bits(0x3ea8_f5c3), 0.0)?;
        or_w(e, GB0(e) + 0x14, 0x100_0000);
    }
    let anim = if sha(e) == "9e7ce829" { 0x29 } else { 0x2d };
    let r = LIBAnimMngr::PlayAnim(e, 0.1, anim, 1, 5);
    e.w32(param_1 + 0x218, r as u32);
    APIinput::InputConfigSetState(e, e.r32(L(e) + 0x188) as i32, i32::MIN);
    APIgameobj::SetContextFlag(e, SELF, 0x3000_2050);
    e.w32(param_1 + 0x158, addr_lv::heroST_BumpByCarrousel_Loop);
    Ok(())
}

pub fn heroST_BumpByCarrousel_Loop(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    if e.r32(param_1 + 0x168) & 1 == 0 {
        if heroIsTrackWPReached(e, 0.5, param_1)? != 0 {
            e.w32(param_1 + 0x154, addr::heroInit_Busy);
        }
    } else {
        e.w32(param_1 + 0x154, addr::heroInit_Busy);
    }
    Ok(())
}

pub fn heroInit_Bump(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    or_w(e, GB0(e) + 0x14, 0x100);
    bump_dm(e, param_1);
    if leader(e) {
        heroFUNC_CallVibration(e, f32::from_bits(0x3f28_f5c3), 0.0, f32::from_bits(0x3ea8_f5c3), 0.0)?;
    }
    let anim = if sha(e) == "6dafd227" { 0x3a } else { 0x3b };
    let r = LIBAnimMngr::PlayAnim(e, 0.1, anim, 1, 5);
    e.w32(param_1 + 0x218, r as u32);
    APIinput::InputConfigSetState(e, e.r32(L(e) + 0x188) as i32, i32::MIN);
    APIgameobj::SetContextFlag(e, SELF, 0x3000_2050);
    e.w32(param_1 + 0x158, addr_lv::heroST_Bump_Loop);
    Ok(())
}

pub fn heroST_Bump_Loop(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    if e.r32(param_1 + 0x168) & 1 == 0 {
        if heroIsTrackWPReached(e, 0.5, param_1)? != 0 {
            and_w(e, GB0(e) + 0x14, 0xffff_feff);
            e.w32(param_1 + 0x154, addr::heroInit_Busy);
        }
    } else {
        and_w(e, GB0(e) + 0x14, 0xffff_feff);
        e.w32(param_1 + 0x154, addr::heroInit_Busy);
    }
    Ok(())
}

// ---- aqueduct / locked door ----

/// (wall object, wall prim, partner/hero object list) of the aqueduct builds' Set/ResetAqueduc: the
/// valve 0x2a0000ec toggles collision bit 0x200000 on the wall 0x2a00002d, both heroes and the partner's
/// prim.
fn aqueduc(e: &mut Engine, set: bool, param_1: u32) {
    let hp = hero_prim(e);
    match sha(e) {
        "d8a08bbd" | "dca9fe48" => {
            if param_1 == 0x2a00_00ec {
                let (partner, pp, other) =
                    if sha(e) == "d8a08bbd" { (0x2a00_002b, 0x6e, 0x2a00_00e7) } else { (0x2a00_002c, 100, 0x2a00_00e8) };
                prim(e, set, 0x2a00_002d, 0x84, 0x20_0000);
                prim(e, set, SELF, hp, 0x20_0000);
                prim(e, set, partner, pp, 0x20_0000);
                prim(e, set, other, hp, 0x20_0000);
            }
        }
        _ => {
            // 26992d64: three valves, one bit each, on 0x2a000034 / self / 0x2a0000f2
            let bit = match param_1 {
                0x2a00_00f6 => 0x40_0000,
                0x2a00_00f7 => 0x10_0000,
                0x2a00_00f8 => 0x20_0000,
                _ => return,
            };
            prim(e, set, 0x2a00_0034, 0x84, bit);
            prim(e, set, SELF, hp, bit);
            prim(e, set, 0x2a00_00f2, hp, bit);
        }
    }
}
/// `scooby_FUNC_SetAqueduc` / `shaggy_FUNC_SetAqueduc`: clears the collision bits (the water is gone)
pub fn scooby_FUNC_SetAqueduc(e: &mut Engine, param_1: u32) {
    aqueduc(e, false, param_1);
}
/// `scooby_FUNC_ResetAqueduc` / `shaggy_FUNC_ResetAqueduc`
pub fn scooby_FUNC_ResetAqueduc(e: &mut Engine, param_1: u32) {
    aqueduc(e, true, param_1);
}
pub fn shaggy_FUNC_SetLockedDoor(e: &mut Engine) {
    APIgameobj::SetGroupID(e, SELF, 6);
}
pub fn shaggy_FUNC_ResetLockedDoor(e: &mut Engine) {
    APIgameobj::SetGroupID(e, SELF, 0);
}

/// `scoobyST_FIRSTMOVE` / `shaggyST_FIRSTMOVE` of the aqueduct builds: restore the collision bits of the
/// valves not yet opened (GB2+8 bits), then heroST_FIRSTMOVE.
pub fn scoobyST_FIRSTMOVE(e: &mut Engine, a: u32) -> LResult<u64> {
    let st = e.r32(GB2(e) + 8);
    let hp = hero_prim(e);
    match sha(e) {
        "d8a08bbd" | "dca9fe48" => {
            if st & 0x2_0000 == 0 {
                let other = if sha(e) == "d8a08bbd" { 0x2a00_00e7 } else { 0x2a00_00e8 };
                prim(e, true, 0x2a00_002d, 0x84, 0x20_0000);
                prim(e, true, SELF, hp, 0x20_0000);
                prim(e, true, other, hp, 0x20_0000);
            }
        }
        _ => {
            for (flag, bit) in [(0x8000, 0x40_0000), (0x1_0000, 0x10_0000), (0x2_0000, 0x20_0000)] {
                if st & flag == 0 {
                    prim(e, true, 0x2a00_0034, 0x84, bit);
                    prim(e, true, SELF, hp, bit);
                }
            }
            let ll = LL(e);
            if e.r32(GB2(e) + 0xc) & 0x1_0000 == 0 {
                e.w32(ll + 0x390, 0);
                shaggy_FUNC_ResetLockedDoor(e);
            } else {
                e.w32(ll + 0x390, 1);
                shaggy_FUNC_SetLockedDoor(e);
            }
        }
    }
    heroST_FIRSTMOVE(e, a)
}

// ---- callbacks ----

/// 160eeb3e `heroCB_ANIM_LOCAL` (ext 6): pipe anim 0x37 / 0x3b start -> messages to the pipe / water.
pub fn heroCB_ANIM_LOCAL(e: &mut Engine, param_1: u32) -> LResult<u64> {
    if e.r32(param_1 + 4) == 1 {
        match e.r32(param_1) {
            0x37 => {
                APIgameobj::SendMessageTo(e, 0x2a00_0028, NULL, 0x98, 2, NULL);
                APIgameobj::SendMessageTo(e, 0x2a00_000e, NULL, 0xdf, 0, NULL);
            }
            0x3b => {
                APIgameobj::SendMessageTo(e, 0x2a00_0028, NULL, 0x98, 3, NULL);
            }
            _ => {}
        }
    }
    heroCB_ANIM(e, param_1)
}

/// 160eeb3e `heroCB_CUTSCENE_LOCAL` (ext 0xb). Falls through to heroCB_CUTSCENE (hero_cutscene.rs, not
/// ported: the cutscene functions were skipped on request).
pub fn heroCB_CUTSCENE_LOCAL(e: &mut Engine, param_1: u32) -> LResult<u64> {
    let (a, b) = (e.r32(param_1 + 4), e.r32(param_1));
    if a == 0 && b == 4 {
        APIgameobj::SendMessageTo(e, 0x2a00_0028, NULL, 0x98, 3, NULL);
        APIgameobj::SendMessageTo(e, 0x2a00_000e, NULL, 0xde, 0, NULL);
        APIgameobj::SendMessageTo(e, 0x2a00_00a5, NULL, 0xeb, 1, NULL);
    }
    if a == 0 && b == 2 {
        APIgameobj::SendMessageTo(e, 0x2a00_0028, NULL, 0x98, 2, NULL);
        APIgameobj::SendMessageTo(e, 0x2a00_000e, NULL, 0xdf, 0, NULL);
        APIgameobj::SendMessageTo(e, 0x2a00_00a5, NULL, 0xeb, 1, NULL);
        APIgameobj::SendMessageTo(e, 0x2a10_0006, NULL, 0x2a, 4, NULL);
    }
    heroCB_CUTSCENE(e, param_1)
}

/// 26992d64 `shaggyCB_ANIM_LF_LOCAL` (ext 5): anim 0x1a ("Invalid command") ends -> mark the level done
/// (level local+0x38c), lock the inputs, the camera cut 0x62 (the module's static camera params, shared
/// with heroFUNC_NextMove), then heroCB_LASTFRAME.
pub fn shaggyCB_ANIM_LF_LOCAL(e: &mut Engine, param_1: u32) -> LResult<u64> {
    if param_1 == 0x1a {
        debug_info(e, "Invalid command - please correct");
        let ll = LL(e);
        e.w32(ll + 0x38c, 1);
        and_w(e, GB2(e) + 4, 0x7f_1fff);
        APIgameobj::SendMessageTo(e, 0x2a10_0006, NULL, 0xb, 5, NULL);
        APIgameobj::SendMessageTo(e, 0x2a10_0006, NULL, 0xb, 10, NULL);
        // WP-R9: moCameraParams_508 is this function's own 0x4c static (C 0x104b0); DAT_000104ec/f0/f4
        // are [0xf] = 0, [0x10] = 1 (fade: FadeInOut(1,0) -> camera timer -> broadcast 0x4f -> the
        // LoadLevel EP2/L03A in the message handler), [0x11] = 0. The port wrote them as [1..3]: no fade.
        let c = G(e, moCameraParams_LF);
        e.w32(c, 0x62);
        e.w32(c + 0x3c, 0);
        e.w32(c + 0x40, 1);
        e.w32(c + 0x44, 0);
        APIgameobj::SendMessageTo(e, 0x2a00_0025, NULL, 0x54, 0, c);
        APIgameobj::SendMessageTo(e, 0x2a10_0006, NULL, 0x1e, 0, NULL);
    }
    heroCB_LASTFRAME(e, param_1)
}

/// 562b9dae `shaggyLocalCB_Sound` (ext 0xd, the level timer): once the sound (level local+0x390) is
/// over, message 0xdf to 0x2a00011d. Returns whether it ended.
pub fn shaggyLocalCB_Sound(e: &mut Engine, _: u32) -> LResult<u64> {
    let s = e.r32(LL(e) + 0x390);
    let playing = APInewSound::IsPlaying(e, SELF, s);
    if playing == 0 {
        APIgameobj::SendMessageTo(e, 0x2a00_011d, NULL, 0xdf, 0, NULL);
    }
    Ok((playing == 0) as u64)
}

// ---- message handler ----

/// Set the push message's common part: GB0+0x14 bit 0x40000000, local+0x23c = object, event bit.
fn push(e: &mut Engine, obj: u32, ev: u32) {
    or_w(e, GB0(e) + 0x14, 0x4000_0000);
    let l = L(e);
    e.w32(l + 0x23c, obj);
    or_w(e, l + 0x170, ev);
}

/// MSG_TRACK_BUMP (0x74 in 9e7ce829/cbbf384c, 0x73 in 6dafd227/ffd170d9): Some(0) = handled.
fn track_bump(e: &mut Engine, param_3: Ptr, tome_obj: u32, set_40: bool) -> Option<u64> {
    if param_3 == 0 {
        debug_info(e, "MSG_TRACK_BUMP -> ERREUR pUserData est null");
        return None;
    }
    if e.r32(GB0(e) + 0x14) & 1 != 0 {
        debug_info(e, "MSG_TRACK_BUMP -> _CHS_TOME_IS_OUT");
        and_w(e, GB0(e) + 0x14, 0xffff_fffe);
        APIgameobj::SendMessageTo(e, tome_obj, NULL, 0xda, 0, NULL);
    }
    debug_info(e, "MSG_TRACK_BUMP");
    if set_40 {
        or_w(e, GB0(e) + 0x14, 0x4000_0000);
    }
    let l = L(e);
    for i in 0..3 {
        let v = e.r32(param_3 + i * 4);
        e.w32(l + 0x2e4 + i * 4, v);
    }
    or_w(e, l + 0x170, 0x2000_0000);
    Some(0)
}

/// The level part of `scoobyFUNC_MessageHandler`: `Some(r)` = answered without heroFUNC_MessageHandler.
pub(crate) fn message(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<Option<u64>> {
    let s = sha(e);
    match s {
        "5257b080" => {
            if param_1 == 300 {
                or_w(e, L(e) + 0x170, 0x1000_0000);
            }
        }
        "5f68ee0e" | "53df9ee8" | "6dafd227" | "ffd170d9" => match param_1 {
            0x97 if s == "5f68ee0e" || s == "53df9ee8" => {
                if param_2 == 0x2a00_0066 {
                    APIgameobj::SendMessageTo(e, 0x2a00_0087, NULL, 0xe6, 0, NULL);
                }
            }
            0xee => {
                if s != "5f68ee0e" {
                    debug_info(e, "MSG_ROTATINGDOOR_PUSH");
                }
                or_w(e, GB0(e) + 0x14, 0x4000_0000);
                let l = L(e);
                e.w32(l + 0x23c, param_2);
                let v = e.r32(param_3);
                e.w32(LL(e) + 0x388, v);
                or_w(e, l + 0x170, 0x1000_0000);
                return Ok(Some(0));
            }
            0x75 if s == "6dafd227" || s == "ffd170d9" => {
                debug_info(e, "MSG_TRACK_DONE");
                or_w(e, L(e) + 0x170, 0x4000_0000);
                return Ok(Some(0));
            }
            0x73 if s == "6dafd227" || s == "ffd170d9" => {
                if let Some(r) = track_bump(e, param_3, 0x2a00_002d, s == "ffd170d9") {
                    return Ok(Some(r));
                }
            }
            _ => {}
        },
        "9e7ce829" | "cbbf384c" => match param_1 {
            0x74 => {
                if let Some(r) = track_bump(e, param_3, 0x2a00_000b, false) {
                    return Ok(Some(r));
                }
            }
            0x75 => {
                debug_info(e, "MSG_TRACK_DONE");
                or_w(e, L(e) + 0x170, 0x4000_0000);
                return Ok(Some(0));
            }
            _ => {}
        },
        "242c6a48" => match param_1 {
            0x97 | 0x1d1 => {
                let p = hero_prim(e);
                APIphysic::PrimResetIntAttribBits(e, SELF, p, 0, 2);
            }
            0xde => {
                and_w(e, GB0(e) + 0x14, 0xffff_feff);
                e.w32(L(e) + 0x154, addr::heroInit_Stand);
            }
            _ => {}
        },
        "7e698dff" | "160eeb3e" => {
            if param_1 == 0xa0 {
                debug_info(e, "MSG_PUSH_PIPE");
                or_w(e, GB0(e) + 0x14, 0x4000_0000);
                let l = L(e);
                e.w32(l + 0x23c, param_2);
                let v = e.r32(param_3);
                e.w32(LL(e) + 0x388, v);
                or_w(e, l + 0x170, 0x1000_0000);
                return Ok(Some(0));
            }
        }
        "11adae8e" | "562b9dae" => {
            let hp = hero_prim(e);
            match param_1 {
                // MSG_PUSH_PIPE starts the sawing here
                0xa0 => {
                    debug_info(e, "MSG_PUSH_PIPE");
                    push(e, param_2, 0x1000_0000);
                    return Ok(Some(0));
                }
                0x153 => {
                    prim(e, false, SELF, hp, 0x400_0000);
                    if s == "562b9dae" {
                        prim(e, false, 0x2a00_0041, 0x84, 0x400_0000);
                    }
                    return Ok(Some(0));
                }
                0x154 => {
                    prim(e, true, SELF, hp, 0x400_0000);
                    return Ok(Some(0));
                }
                0x9d => {
                    APIgameobj::SetGameObjectVisible(e, 0x2a00_011e, 0);
                }
                0x98 if s == "11adae8e" => {
                    if param_2 == 4 {
                        if e.r32(GB2(e) + 8) & 0x80_0000 == 0 {
                            prim(e, false, SELF, 0x67, 0x2000_0000);
                        }
                    } else {
                        prim(e, true, SELF, 0x67, 0x2000_0000);
                    }
                }
                0x162 if s == "11adae8e" => prim(e, true, SELF, 0x67, 0x2000_0000),
                0x163 if s == "11adae8e" => prim(e, false, SELF, 0x67, 0x2000_0000),
                0x9c if s == "562b9dae" => {
                    if param_2 == 8 {
                        or_w(e, GB2(e) + 8, 0x40_0000);
                    }
                }
                // the saw sound: level local+0x390 = sound, the level timer (local+0x38c) polls it
                // through shaggyLocalCB_Sound every 0.5 s
                0xde if s == "562b9dae" => {
                    let ll = LL(e);
                    e.w32(ll + 0x390, param_2);
                    APInewSound::Play(e, 0x2a00_003f, param_2, 0);
                    let t = e.r32(ll + 0x38c);
                    APIextra::TimerRegisterNotify(e, SELF, t, 0xd);
                    APIextra::TimerSetFrequency(e, t, 0.5);
                    APIextra::TimerReset(e, t);
                }
                _ => {}
            }
        }
        "d8a08bbd" | "dca9fe48" => match param_1 {
            0x1fe => scooby_FUNC_SetAqueduc(e, param_2),
            0x9d => {
                let v = e.r32(param_3);
                let other = if s == "d8a08bbd" { 0x2a00_00e7 } else { 0x2a00_00e8 };
                APIgameobj::SendMessageTo(e, other, NULL, 0x1fe, v, NULL);
                scooby_FUNC_SetAqueduc(e, v);
            }
            0x1ff => scooby_FUNC_ResetAqueduc(e, param_2),
            _ => {}
        },
        "26992d64" => {
            let hp = hero_prim(e);
            let ll = LL(e);
            match param_1 {
                0xd4 => {
                    if e.r32(GB2(e) + 0xc) & 0x1_0000 != 0 {
                        e.w32(ll + 0x390, 1);
                    }
                }
                0x97 => {
                    if param_2 == 0x2a00_0120 {
                        shaggy_FUNC_SetLockedDoor(e);
                        or_w(e, GB2(e) + 0xc, 0x1_0000);
                    }
                }
                // hero dead after the level end (level local+0x38c): straight to EP2 L03A, after the
                // movie \MOVIES\EP02\EP02G (the module's DAT_00013b58: only words +0 / +4 are non-zero)
                0x4f if e.r32(ll + 0x38c) != 0 => {
                    let p = e.alloca(0x7dc);
                    for o in (0..0x7dc).step_by(4) {
                        e.w32(p + o, 0);
                    }
                    e.w32(p, 2);
                    let m = e.cstr("\\MOVIES\\EP02\\EP02G");
                    e.w32(p + 4, m);
                    or_w(e, GB2(e) + 0x28, 0x80);
                    let (a, b) = (e.cstr("EP2\\L03A"), e.cstr("EP2L03A"));
                    APIgame::LoadLevel(e, a, b, p);
                }
                0x9d => {
                    let v = e.r32(param_3);
                    APIgameobj::SendMessageTo(e, 0x2a00_00f2, NULL, 0x1fe, v, NULL);
                    scooby_FUNC_SetAqueduc(e, v);
                }
                0xc9 if e.r32(ll + 0x390) == 0 => {
                    and_w(e, GB2(e) + 0xc, 0xfffe_ffff);
                    shaggy_FUNC_ResetLockedDoor(e);
                }
                0x152 => {
                    prim(e, false, SELF, hp, 0x400_0000);
                    prim(e, true, SELF, hp, 0x200_0000);
                    APIgameobj::SendMessageTo(e, 0x2a00_00f2, NULL, 0x152, 0, NULL);
                    return Ok(Some(0));
                }
                0x151 => {
                    prim(e, true, SELF, hp, 0x400_0000);
                    prim(e, false, SELF, hp, 0x200_0000);
                    APIgameobj::SendMessageTo(e, 0x2a00_00f2, NULL, 0x151, 0, NULL);
                    return Ok(Some(0));
                }
                0x1fe => scooby_FUNC_SetAqueduc(e, param_2),
                0x1ff => scooby_FUNC_ResetAqueduc(e, param_2),
                _ => {}
            }
        }
        _ => unreachable!(),
    }
    Ok(None)
}

// ---- constructor / destructor parts ----

/// Extra collision bits set by Create_Local (after the sound tables).
pub(crate) fn create_local_bits(e: &Engine) -> &'static [u32] {
    match sha(e) {
        "11adae8e" | "562b9dae" | "160eeb3e" => &[0x400_0000],
        _ => &[],
    }
}

/// Level local-data size (`GetLocalData(&pLocalDataHeroLevel, n)`).
pub(crate) fn level_data_size(e: &Engine) -> i32 {
    match sha(e) {
        "562b9dae" | "26992d64" => 0x394,
        _ => 0x38c,
    }
}

/// The constructor's level part, in the original's order around the common steps (`heroFUNC_Constructor`,
/// GetLocalData, SetMessageFunction, Create_Local, SetInitialState are done by scooby.rs).
/// `before_local`: between heroFUNC_Constructor and GetLocalData. `after`: after Create_Local.
pub(crate) fn ctor_before_local(e: &mut Engine) {
    match sha(e) {
        "160eeb3e" => {
            APIcons::SetExternalFunction(e, 6, heroCB_ANIM_LOCAL);
            APIcons::SetExternalFunction(e, 0xb, heroCB_CUTSCENE_LOCAL);
        }
        "562b9dae" => {
            APIcons::SetExternalFunction(e, 0xd, shaggyLocalCB_Sound);
        }
        "26992d64" => {
            let g = APIcons::GetApiGame(e);
            e.w32(G(e, pGame), g);
            APIcons::SetExternalFunction(e, 5, shaggyCB_ANIM_LF_LOCAL);
            APIcons::SetExternalFunction(e, 0, scoobyST_FIRSTMOVE);
        }
        _ => {}
    }
}
/// Between GetLocalData and SetMessageFunction.
pub(crate) fn ctor_after_local(e: &mut Engine) {
    if matches!(sha(e), "d8a08bbd" | "dca9fe48") {
        APIcons::SetExternalFunction(e, 0, scoobyST_FIRSTMOVE);
    }
}
/// After Create_Local, before SetInitialState.
pub(crate) fn ctor_after_create(e: &mut Engine) {
    match sha(e) {
        "d8a08bbd" | "dca9fe48" => {
            let hp = hero_prim(e);
            prim(e, true, SELF, hp, 0x200_0000);
        }
        "160eeb3e" => {
            APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 0xb, 6);
            APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 0x16, 0xb);
        }
        "562b9dae" => {
            let t = APIextra::TimerCreate(e);
            e.w32(LL(e) + 0x38c, t);
        }
        _ => {}
    }
}
/// Before heroFUNC_Destructor.
pub(crate) fn dtor(e: &mut Engine) {
    if sha(e) == "562b9dae" {
        let t = e.r32(LL(e) + 0x38c);
        APIextra::TimerDestroy(e, t);
    }
}

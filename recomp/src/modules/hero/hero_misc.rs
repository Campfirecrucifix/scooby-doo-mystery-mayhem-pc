//! hero_misc.c (13 functions). Addresses are module-relative (.text at 0x10000) in scooby__402420b4.
use super::builds::*;
use super::*;
use crate::engine::*;
use crate::logic::LResult;

/// 0x00022e68 (348 bytes) `bool heroFUNC_ValidateTarget(undefined8 *param_1)`
pub fn heroFUNC_ValidateTarget(e: &mut Engine, param_1: Ptr) -> LResult<u32> {
    // same code in every build: a ray from 0.2 above the hero along the hero-local vector *param_1
    let d0 = e.alloca(0x10);
    let c0 = e.alloca(0x10);
    let v = e.rv3(param_1);
    e.wv3(d0, v);
    APIgameobj::GetPosition(e, SELF, ALL, c0);
    APIgameobj::TransformVector(e, SELF, d0, d0);
    let p = e.rv3(c0);
    let dir = e.rv3(d0);
    let id = e.r32(L(e) + 0x17c);
    let r = ray(e, [p[0], p[1] + 0.2, p[2]], dir, [0xffff_ffff, 0, 0xffff_ffff, 0, id]);
    Ok((r == 0) as u32)
}

/// 0x00022fd0 (508 bytes) `void heroFUNC_SetNewPos(uint param_1,uint param_2)`
pub fn heroFUNC_SetNewPos(e: &mut Engine, param_1: u32, param_2: u32) -> LResult<()> {
    // groups 2 / 4 (625480d1, 61655a9e) warp the slave at the leader's height (0.0 instead of 0.5)
    let up = match g(e, G_heroFUNC_SetNewPos) {
        2 | 4 => 0.0,
        _ => 0.5,
    };
    let d40 = e.alloca(0x10);
    let d30 = e.alloca(0x10);
    e.wv3(d40, [-1.0, up, 0.0]);
    debug_info(e, "FUNC_SetNewPos Slave is warped");
    APIphysic::SetVelocity(e, param_1, 0.0, 0.0, 0.0);
    APIgameobj::TransformVector(e, param_2, d40, d40);
    APIgameobj::GetPosition(e, param_2, ALL, d30);
    let (o, p) = (e.rv3(d40), e.rv3(d30));
    // the height offset is the literal, not the transformed vector's y
    e.wv3(d30, [p[0] + o[0], p[1] + up, p[2] + o[2]]);
    APIgameobj::SetTranslation(e, param_1, d30);
    APIgameobj::TakeOrientation(e, param_1, param_2, ALL);
    debug_info_value(e, "warpto", d30, 4);
    APIgameobj::SendMessageTo(e, param_1, NULL, 0x7e, 0, NULL);
    APIgameobj::SendMessageTo(e, param_1, NULL, 0x94, 0x2f, NULL);
    Ok(())
}

/// 0x000231d0 (1104 bytes) `undefined4 heroFUNC_IsAnythingBetweenUs(float param_1,uint param_2,int param_3,undefined4 *param_4,int param_5)`
pub fn heroFUNC_IsAnythingBetweenUs(e: &mut Engine, param_1: f32, param_2: u32, param_3: i32, param_4: Ptr, param_5: i32) -> LResult<u32> {
    // same logic in every build (release builds lack the print). Up to 4 rays along (target - param_2),
    // started from points around param_2 offset in the leader's frame; param_1 == 0: one ray only.
    let d90 = e.alloca(0x10);
    let d80 = e.alloca(0x10);
    let d70 = e.alloca(0x10);
    let d50 = e.alloca(0x10);
    let d40 = e.alloca(0x10);
    let d30 = e.alloca(0x10);
    let mut f58 = param_1;
    APIgameobj::GetPosition(e, param_2, ALL, d90);
    if param_3 == 0 {
        let v = e.rv3(param_4);
        e.wv3(d80, v);
    } else {
        APIgameobj::GetPosition(e, param_3 as u32, ALL, d80);
    }
    APImath::CVectorSub(e, d70, d80, d90);
    let kind = if param_5 == 0 { 2 } else { param_5 as u32 };
    let dir = e.rv3(d70);
    let prim = e.r32(L(e) + 0x18c);
    let attr = APIphysic::PrimGetIntAttrib(e, SELF, prim, 5);
    let id = APIgameobj::GetID(e, param_2, NULL);
    if attr & 0xffff_fffc != 0 && dprint(e) {
        let v = e.alloca(0x10);
        e.w32(v, attr);
        e.w32(v + 4, 0);
        e.w32(v + 8, id);
        debug_info_value(e, "heroFUNC_IsAnythingBetweenUs", v, 1);
    }
    if param_1 != 0.0 {
        f58 = APImath::powf(e, param_1 * param_1 * 0.5, 0.5);
    }
    for i in 1..=4 {
        let (x, z) = match i {
            2 => (f58 * -1.0, f58 * -1.0),
            1 => (f58, f58 * -1.0),
            3 => (0.0, param_1),
            _ => (0.0, 0.0),
        };
        e.wv3(d40, [x, 0.0, z]);
        let leader = e.r32(GB0(e) + 8);
        APIgameobj::TransformVector(e, leader, d40, d50);
        APImath::CVectorAdd(e, d30, d90, d50);
        let p = e.rv3(d30);
        let r = ray(e, [p[0], p[1] + 0.2, p[2]], dir, [kind, 0, attr, 0, id]);
        if r != 0 {
            return Ok(1);
        }
        if param_1 == 0.0 {
            return Ok(0);
        }
    }
    Ok(0)
}

/// 0x00023630 (1656 bytes) `void heroFUNC_NextMove(int param_1)`
/// Warp through a door: finds local+0x234 in goNextRoomInfo (16-byte records: door, room, camera
/// point, no-camera flag), announces the room (0x98 to all), hides both heroes (+ one per-build object),
/// warps the hero (or, during a switch, re-places the partner), and places the camera.
/// Per-build values: [`super::nextmove_tab`]. `moCameraParams` is the function's static struct
/// (0x104f8 in the reference build; same layout everywhere, used only here).
pub fn heroFUNC_NextMove(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    let sha = builds::BUILDS[e.rt.cur_build()].sha8;
    let p = &super::nextmove_tab::NEXTMOVE_PARAMS.iter().find(|(s, _)| *s == sha).expect("hero build without NEXTMOVE_PARAMS").1;
    let mut cam_pt: u32 = 0; // uStack_5c
    let mut no_cam: u32 = 0; // iStack_58
    if p.debug { debug_info(e, "FUNC_NextMove"); }
    let door = e.r32(param_1 + 0x234);
    if door == 0 {
        if p.debug { debug_info(e, "ASSERT (\"NextMove\") -> Erreur logique \"pLocalData->uiCurrentTrack != 0\""); }
    }
    let end = match p.end_door {
        Some(rd) if e.r32(GB0(e) + 0x88) != 0 && e.r32(param_1 + 0x2c0) == 0 => Some(rd),
        _ => None,
    };
    if let Some((room, d)) = end {
        if p.debug { debug_info(e, "***************************************"); }
        if p.debug { debug_info_value(e, "heroFUNC_NextMove -> set bLoadAtTheEndOfTheLevel to NULL", GB0(e) + 0x88, 1); }
        if p.debug { debug_info(e, "***************************************"); }
        e.w32(GB0(e) + 0x88, 0);
        e.w32(GB0(e) + 0x24, room);
        cam_pt = d;
        no_cam = 0;
    } else {
        let tab = G(e, goNextRoomInfo);
        for i in 0..p.doors {
            let r = tab + i * 0x10;
            if e.r32(r) == door {
                e.w32(GB0(e) + 0x24, e.r32(r + 4));
                cam_pt = e.r32(r + 8);
                no_cam = e.r32(r + 0xc);
                break;
            }
        }
    }
    let orp = e.alloca(0x20);
    e.w32(orp, 1);
    let room = e.r32(GB0(e) + 0x24);
    APIgameobj::SendMessageTo(e, 0xff00_0001, orp, 0x98, room, 0);
    APIgraphics::DoCrappyXboxSpewHackThingy(e, e.r32(GB0(e) + 8), 0, 0);
    APIgraphics::DoCrappyXboxSpewHackThingy(e, e.r32(GB0(e) + 0xc), 0, 0);
    if APIgameobj::GetID(e, p.spew, 0) != 0x7fff_fffe {
        APIgraphics::DoCrappyXboxSpewHackThingy(e, p.spew, 0, 0);
    }
    let pos = e.alloca(0x10); // DStack_70
    if e.r32(GB0(e) + 0x14) & 0x800_0000 == 0 {
        let door = e.r32(param_1 + 0x234);
        APIgameobj::GetPosition(e, door, ALL, pos);
        APIgameobj::SetTranslation(e, SELF, pos);
        APIgameobj::TakeOrientation(e, SELF, door, ALL);
        APIphysic::SetVelocity(e, SELF, 0.0, 0.0, 0.0);
        if p.debug { debug_info_value(e, "warpto", pos, 4); }
        let g = GB0(e);
        if e.r32(g + 0x34) & 0x10 != 0
            && e.r32(g + 0x14) & 0x400_0000 == 0
            && (!p.g100 || e.r32(g + 0x34) & 0x100 == 0)
            && p.no_setnewpos.map_or(true, |x| e.r32(param_1 + 0x234) != x)
        {
            heroFUNC_SetNewPos(e, e.r32(param_1 + 0x180), e.r32(param_1 + 0x234))?;
        }
    } else {
        let a = GB0(e) + 0x14;
        let v = e.r32(a);
        e.w32(a, v & 0xf7ff_ffff);
        heroFUNC_MessageHandler(e, 0x66, 0, 0)?;
        if let Some(t) = p.ack21 {
            APIgameobj::SendMessageTo(e, t, 0, 0x21, 0, 0);
        }
        let g = GB0(e);
        if e.r32(g + 0x7c0) != 0
            && e.r32(g + 0x7c4) != 0
            && APIgameobj::GetID(e, e.r32(g + 0x7c4), 0) != 0x7fff_fffe
            && e.r32(g + 0x7c0) & 0x20 != 0
        {
            let p50 = e.alloca(0x10);
            let mark = e.r32(g + 0x7c4);
            APIgameobj::GetPosition(e, mark, ALL, p50);
            APIgameobj::SetTranslation(e, e.r32(g + 0xc), p50);
            APIgameobj::TakeOrientation(e, e.r32(GB0(e) + 0xc), e.r32(GB0(e) + 0x7c4), ALL);
        }
    }
    APIgameobj::GetPosition(e, cam_pt, ALL, pos);
    APIgameobj::SetTranslation(e, p.cam, pos);
    e.w32(GB0(e) + 100, 0);
    if no_cam == 0 {
        let c = G(e, 0x104f8);
        e.w32(c, 0x4d);
        e.w32(c + 4, cam_pt);
        e.w32(c + 0x10, 0);
        e.w32(c + 0x14, 0x3f80_0000);
        e.w32(c + 0x18, 0x4170_0000);
        APIgameobj::SendMessageTo(e, p.cam, 0, 0x54, 4, c);
    }
    let a = GB0(e) + 0x14;
    let v = e.r32(a);
    e.w32(a, v & 0xf9df_ffff);
    Ok(())
}

/// 0x00023db0 (112 bytes) `void heroFUNC_CallVibration(float param_1,float param_2,float param_3,float param_4)`
pub fn heroFUNC_CallVibration(e: &mut Engine, param_1: f32, param_2: f32, param_3: f32, param_4: f32) -> LResult<()> {
    let pad = e.r32(GB0(e) + 0x4c) as i32;
    APIinput::InputSetVibration(e, pad, 0, param_1, param_2, param_3, param_4);
    Ok(())
}

/// 0x00023e20 (444 bytes) `undefined4 IsCharacterOnThisFloorType(float param_1,float param_2,uint param_3,int param_4,int param_5)`
pub fn IsCharacterOnThisFloorType(e: &mut Engine, param_1: f32, param_2: f32, param_3: u32, param_4: i32, param_5: i32) -> LResult<u32> {
    // same in every build: a ray from param_2 above param_3 straight down (param_1 long; |param_1| <=
    // 0.01 means 1 m) against the environment types param_4 / param_5; hit (2) -> on that floor
    if param_4 == 0 && param_5 == 0 {
        debug_info(e, "IsCharacterOnThisFloorType -> aui32EnvDetectType == NULL && aui32EnvUDDetectType == NULL");
        return Ok(0);
    }
    let b0 = e.alloca(0x10);
    APIgameobj::GetPosition(e, param_3, ALL, b0);
    let mut len = param_1;
    if param_1 <= 0.01 && -0.01 <= param_1 {
        len = -1.0;
    }
    let p = e.rv3(b0);
    let r = ray(e, [p[0], p[1] + param_2, p[2]], [0.0, len, 0.0], [0, 0, param_4 as u32, param_5 as u32, param_3]);
    Ok((r == 2) as u32)
}

/// 0x00023fe8 (752 bytes) `undefined4 IsSlaveInTheWay(int param_1)`
pub fn IsSlaveInTheWay(e: &mut Engine, param_1: Ptr) -> LResult<u32> {
    // same in every build: 3 rays 1.25 m forward from 0.2 above the hero (centre, 0.35 right, 0.35 left)
    let d60 = e.alloca(0x10);
    let d50 = e.alloca(0x10);
    let d30 = e.alloca(0x10);
    APIgameobj::GetPosition(e, SELF, ALL, d60);
    APIgameobj::GetLookAt(e, SELF, ALL, d50);
    let l = e.rv3(d50);
    let dir = [l[0] * 1.25, l[1] * 1.25, l[2] * 1.25];
    let mut p = e.rv3(d60);
    p[1] += 0.2;
    e.wv3(d60, p);
    let id = e.r32(param_1 + 0x17c);
    for i in 1..=3u32 {
        let x = match i {
            2 => 0.35,
            3 => -0.35,
            _ => 0.0,
        };
        e.wv3(d30, [x, 0.0, 0.0]);
        APIgameobj::TransformVector(e, SELF, d30, d30);
        APImath::CVectorAdd(e, d30, d30, d60);
        let s = e.rv3(d30);
        let r = ray(e, s, dir, [4, 0, 0, 0, id]);
        if r == 1 || (r != 0 && r != 2) {
            debug_info(e, "-----------------IsSlaveInTheWay-----------------");
            return Ok(1);
        }
    }
    Ok(0)
}

/// 0x000242d8 (1204 bytes) `undefined4 IsWaypointClear(uint param_1)`
pub fn IsWaypointClear(e: &mut Engine, param_1: u32) -> LResult<u32> {
    // same in every build. Note: the scan loop only ever runs its first step (`if (1 < c) return 0`
    // with c starting at 1): one ray from the waypoint towards (side * right - forward) * 0.4.
    let left = LIBApiCFunctions::IsObjectInMyOrientation(e, 3.1415927, 0.0, SELF, param_1, ALL) != 0;
    let side: f32 = if left {
        debug_info(e, "IsWaypointClear - left side");
        2.0
    } else {
        debug_info(e, "IsWaypointClear - right side");
        -2.0
    };
    let d80 = e.alloca(0x10);
    let d70 = e.alloca(0x10);
    let d60 = e.alloca(0x10);
    let d50 = e.alloca(0x10);
    APIgameobj::GetPosition(e, param_1, ALL, d80);
    APIgameobj::GetLookAt(e, SELF, ALL, d70);
    APIgameobj::GetLookRight(e, SELF, ALL, d60);
    let mut r6 = e.rv3(d60);
    r6 = [r6[0] * side, r6[1] * side + 0.2, r6[2] * side];
    APIgameobj::GetPosition(e, SELF, ALL, d50); // unused (+0.2 on y)
    debug_info(e, "-----------------IsWaypointClear-----------------");
    let (p, f) = (e.rv3(d80), e.rv3(d70));
    let c = 1;
    let o = match c {
        2 => r6,
        1 => [r6[0] - f[0], r6[1] - f[1], r6[2] - f[2]],
        3 => [r6[0] + f[0], r6[1] + f[1], r6[2] + f[2]],
        _ => [0.0; 3],
    };
    let dir = [o[0] * 0.4, o[1] * 0.4, o[2] * 0.4];
    let l = L(e);
    e.wv3(l + 0x2d0, [p[0] + dir[0], p[1] + dir[1], p[2] + dir[2]]);
    if ray(e, p, dir, [4, 0, 3, 0, param_1]) != 0 {
        return Ok(0);
    }
    debug_info(e, "... end scanning Clear");
    Ok(1)
}

/// 0x00024798 (112 bytes) `undefined4 IsInTeamMode(void)`
pub fn IsInTeamMode(e: &mut Engine) -> LResult<u32> {
    let f = e.r32(GB0(e) + 0x34);
    Ok((f & 0x10 != 0 && f & 0x100 == 0) as u32)
}

/// 0x00024808 (96 bytes) `void UpdateTimerOutOfTime(float param_1)`
pub fn UpdateTimerOutOfTime(e: &mut Engine, param_1: f32) -> LResult<()> {
    // same code in every build
    let t = APIextra::TimerGetTime(e, e.r32(L(e) + 0x204));
    e.wf(L(e) + 0x2dc, t + param_1);
    Ok(())
}

/// 0x00024868 (120 bytes) `bool IsOutOfTime(void)`
pub fn IsOutOfTime(e: &mut Engine) -> LResult<u32> {
    // same code in every build
    let t = APIextra::TimerGetTime(e, e.r32(L(e) + 0x204));
    Ok((e.rf(L(e) + 0x2dc) < t) as u32)
}

/// 0x000248e8 (752 bytes) `undefined4 heroFUNC_DebugTime(undefined4 param_1,uint *param_2,undefined4 param_3)`
///
/// Release builds (tables::DEBUG_BUILD false, shape group 1) compile it as `void (void) {}`.
/// Debug builds read the game clock and hand the line to `pExtra->null_printf` (the engine's
/// discarding printf). The port's `null_printf` slot takes the format and its first argument only;
/// the remaining varargs are not forwarded (they have no effect in the engine).
pub fn heroFUNC_DebugTime(e: &mut Engine, param_1: u32, param_2: Ptr, param_3: u32) -> LResult<u32> {
    if !debug_build(e) {
        return Ok(0);
    }
    APIextra::TimerGetTime(e, 0x9000_0000);
    let name = char_name(e);
    let r = match param_3 {
        1 => {
            let f = e.cstr("%s> %s\t-> %u\n");
            APIextra::null_printf(e, f, name)
        }
        2 => {
            let f = e.cstr("%s> %s\t-> %f\n");
            APIextra::null_printf(e, f, name)
        }
        3 => {
            let f = e.cstr("%s> %s\t-> %s\n");
            APIextra::null_printf(e, f, name)
        }
        4 => {
            let f = e.cstr("%s> %s\t-> x: %8.6f  y: %8.6f  z: %8.6f\n");
            APIextra::null_printf(e, f, name)
        }
        5 => {
            let v = e.r32(param_2);
            let f = e.cstr("%s> %s\t-> %u\n");
            APIextra::null_printf(e, f, name);
            let f = e.cstr("      3         2         1\n");
            APIextra::null_printf(e, f, 0);
            let f = e.cstr("     10987654321098765432109876543210\n");
            APIextra::null_printf(e, f, 0);
            let f = e.cstr("     - --------- --------- --------- \n");
            APIextra::null_printf(e, f, 0);
            // char auStack_50[33]: bit 31-i of the value as 'I' (set) / 'O' (clear), NUL at [32]
            let buf = e.alloca(0x24);
            for i in (0..=0x1f_i32).rev() {
                let set = (1u32 << ((0x1f - i) as u32 & 0x1f)) & v != 0;
                e.w8(buf + i as u32, if set { 0x49 } else { 0x4f });
            }
            e.w8(buf + 0x20, 0);
            let f = e.cstr("     %s\n");
            APIextra::null_printf(e, f, buf)
        }
        _ => {
            let f = e.cstr("%s> %s\n");
            APIextra::null_printf(e, f, name)
        }
    };
    Ok(r)
}

/// Rust-side convenience for the `heroFUNC_DebugTime("<literal>", ptr, kind)` call sites: interns the
/// literal only in debug builds (release builds pass nothing).
pub(crate) fn debug_time(e: &mut Engine, msg: &str, p2: Ptr, p3: u32) -> LResult<u32> {
    if debug_build(e) {
        let m = e.cstr(msg);
        heroFUNC_DebugTime(e, m, p2, p3)
    } else {
        heroFUNC_DebugTime(e, 0, 0, 0)
    }
}

/// 0x00024bd8 (552 bytes) `int heroFUNC_MultiBits(undefined4 param_1,uint param_2)`
pub fn heroFUNC_MultiBits(e: &mut Engine, param_1: u32, param_2: u32) -> LResult<i32> {
    // same in every build (character name per build): debug dump when more than one bit is set
    let buf = e.alloca(0x24);
    let mut n = 0i32;
    for i in (0..=0x1f_i32).rev() {
        if (1u32 << ((0x1f - i) as u32 & 0x1f)) & param_2 == 0 {
            e.w8(buf + i as u32, 0x4f);
        } else {
            n += 1;
            e.w8(buf + i as u32, 0x49);
        }
    }
    if 1 < n {
        APIextra::TimerGetTime(e, 0x9000_0000);
        let name = char_name(e);
        // float / second varargs are not forwarded (see heroFUNC_DebugTime)
        for (f, a) in [
            ("===============%s===============\n", name),
            (" %f\n", 0),
            ("  ERROR -> Too many bits....\n", 0),
            (" %s\t-> %u\n", param_1),
            ("      3         2         1\n", 0),
            ("     10987654321098765432109876543210\n", 0),
            ("     - --------- --------- --------- \n", 0),
        ] {
            let f = e.cstr(f);
            APIextra::null_printf(e, f, a);
        }
        e.w8(buf + 0x20, 0);
        let f = e.cstr("     %s\n");
        APIextra::null_printf(e, f, buf);
        let f = e.cstr("===============%s===============\n");
        n = APIextra::null_printf(e, f, name) as i32;
    }
    Ok(n)
}

/// `pPhysic->CheckRayIntersection` with a RayIntersectIn built on the stack: start (+0), direction
/// (+0xc), then the five words +0x18..+0x28 (types / ignore masks / ignored object, as each caller
/// fills them). The 64-byte RayIntersectOut is discarded by every hero caller.
pub(crate) fn ray(e: &mut Engine, start: [f32; 3], dir: [f32; 3], w: [u32; 5]) -> u32 {
    let r = e.alloca(0x30);
    let out = e.alloca(0x40);
    e.wv3(r, start);
    e.wv3(r + 0xc, dir);
    for (i, v) in w.iter().enumerate() {
        e.w32(r + 0x18 + 4 * i as u32, *v);
    }
    APIphysic::CheckRayIntersection(e, r, out)
}

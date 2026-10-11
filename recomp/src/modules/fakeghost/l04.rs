//! `fakeghost` build fbe91ac1 (EP1L04): Peabody in a ghost costume wandering the "Path_FakeGhost" path
//! graph, choosing branches at random, pushing the rotating doors in his way, dropping wisps when
//! surprised. Port of `Ep1_Library/Characters/FakeGhost/Level04/FakeGhost.c` (40 functions).
//!
//! The function-pointer sub-state at L+0 is stored as an index into [`STATES`] (0 = BLACKHOLE_LOOP,
//! which is also what a zeroed local means). Local layout: +4 events, +8 conditions to check, +0xc spawn
//! pos, +0x18 door, +0x1c door-wait mode, +0x20..+0x2c timers, +0x30 own id, +0x34/+0x38 current/previous
//! node, +0x3c door flag, +0x40..+0x54 DMs, +0x58 wisp pos, +0x8c..+0xab front/left/right {kind, node,
//! door}, +0xb0 target node pos, +0xbc ray in, +0xe8/+0xec footstep type/emitter, +0xf0 path, +0x100 nodes.
use super::*;

pub const pExtra: u32 = 0x1002c;
pub const pGameObj: u32 = 0x10030;
pub const pPhysic: u32 = 0x10034;
pub const pParticle: u32 = 0x10038;
pub const pMath: u32 = 0x1003c;
pub const pStdlib: u32 = 0x10040;
pub const pLight: u32 = 0x10044;
pub const pSound: u32 = 0x10048;
pub const pGraphic: u32 = 0x1004c;
pub const pDebug: u32 = 0x10050;
pub const pAnimMngr: u32 = 0x10054;
pub const pFunction: u32 = 0x10058;
pub const pMiscFunc: u32 = 0x1005c;
pub const pInput: u32 = 0x10060;
pub const pPathFinding: u32 = 0x10064;
pub const pAnim: u32 = 0x10068;
pub const pLocalDataFakeGhost: u32 = 0x1006c;
pub const pGlobalDataFakeGhost: u32 = 0x10070;

/// mRotatingDoors (module .text+0, 11 ids)
const DOORS: [u32; 11] = [
    0x2a00000d, 0x2a00000e, 0x2a00000f, 0x2a000010, 0x2a000011, 0x2a000012, 0x2a000013, 0x2a000014,
    0x2a000015, 0x2a000016, 0x2a000017,
];

fn L(e: &Engine) -> Ptr {
    e.r32(G(e, pLocalDataFakeGhost))
}
fn lr(e: &Engine, o: u32) -> u32 {
    e.r32(L(e) + o)
}
fn lw(e: &mut Engine, o: u32, v: u32) {
    let a = L(e) + o;
    e.w32(a, v);
}
fn ev(e: &Engine) -> u32 {
    lr(e, 4)
}
fn set_ev(e: &mut Engine, bits: u32) {
    let v = ev(e) | bits;
    lw(e, 4, v);
}
fn clr_ev(e: &mut Engine, bits: u32) {
    let v = ev(e) & !bits;
    lw(e, 4, v);
}
fn gb(e: &Engine) -> Ptr {
    e.r32(e.r32(G(e, pGlobalDataFakeGhost)))
}
fn leader(e: &Engine) -> u32 {
    e.r32(gb(e) + 8)
}
fn go(e: &mut Engine, s: u32) {
    lw(e, 0, s);
}
fn path(e: &Engine) -> u32 {
    L(e) + 0xf0
}
fn get_node(e: &mut Engine, n: u32, out: Ptr) {
    let p = path(e);
    LIBApiPathFinding::GetNode(e, n as i32, out, p);
}
fn neighbors(e: &mut Engine, n: u32) -> i32 {
    let (l, p) = (L(e), path(e));
    LIBApiPathFinding::GetNeighborNodes(e, n as i32, (l + 0x100) as i32, p)
}

const BLACKHOLE: u32 = 0;
const STAND_SET: u32 = 1;
const STAND_LOOP: u32 = 2;
const WALK_SET: u32 = 3;
const WALK_LOOP: u32 = 4;
const SURPRISED_SET: u32 = 5;
const SURPRISED_LOOP: u32 = 6;
const PUSH_DOOR_SET: u32 = 7;
const PUSH_LOOP: u32 = 8;
const GOTO_SET: u32 = 9;
const GOTO_LOOP: u32 = 10;
const WAIT_SET: u32 = 11;
const WAIT_LOOP: u32 = 12;
const BUMP_SET: u32 = 13;
const BUMP_LOOP: u32 = 14;
static STATES: [fn(&mut Engine) -> LResult<()>; 15] = [
    |_| Ok(()),
    ST_STAND_SET,
    ST_STAND_LOOP,
    ST_WALK_SET,
    ST_WALK_LOOP,
    ST_SURPRISED_SET,
    ST_SURPRISED_LOOP,
    ST_PUSH_DOOR_SET,
    ST_PUSH_LOOP,
    ST_GOTO_SET,
    ST_GOTO_LOOP,
    ST_WAIT_DOOR_ACTION_SET,
    ST_WAIT_DOOR_ACTION_LOOP,
    ST_BUMP_SET,
    ST_BUMP_LOOP,
];

pub fn FakeGhostST_FIRSTMOVE_SET(e: &mut Engine, _: u32) -> LResult<u64> {
    let s = e.cstr("Path_FakeGhost");
    let p = path(e);
    LIBApiPathFinding::SetPath(e, s as i32, p as i32);
    go(e, BLACKHOLE);
    APIgameobj::SetGameObjectVisible(e, SELF, 0);
    APIphysic::SetFlags(e, SELF, 1);
    APIgameobj::SetNextState(e, SELF, 1);
    Ok(0)
}
pub fn FakeGhostST_ENTRY_POINT(e: &mut Engine, _: u32) -> LResult<u64> {
    GameEventSignaler(e);
    let s = lr(e, 0) as usize;
    STATES[s](e)?;
    Ok(0)
}
fn ST_STAND_SET(e: &mut Engine) -> LResult<()> {
    lw(e, 4, 0);
    LIBAnimMngr::PlayAnim(e, 0.2, 3, 2, 0);
    APIgameobj::SetContextFlag(e, SELF, 0x60);
    lw(e, 8, 0x10);
    go(e, STAND_LOOP);
    Ok(())
}
fn ST_STAND_LOOP(e: &mut Engine) -> LResult<()> {
    if ev(e) & 0x400 != 0 {
        go(e, BUMP_SET);
    } else if ev(e) & 0x2000 != 0 {
        clr_ev(e, 0x2000);
        go(e, SURPRISED_SET);
    }
    Ok(())
}
fn ST_WALK_SET(e: &mut Engine) -> LResult<()> {
    lw(e, 4, 0);
    lw(e, 0x3c, 0);
    let t = lr(e, 0x28);
    APIextra::TimerReset(e, t);
    LIBAnimMngr::PlayAnim(e, 0.2, 7, 2, 0);
    TrackPos(e, 8.0);
    SetSpeed(e, 5.0);
    let t = lr(e, 0x2c);
    APIextra::TimerReset(e, t);
    lw(e, 8, 0x3a);
    APIgameobj::SetContextFlag(e, SELF, 0x66);
    go(e, WALK_LOOP);
    Ok(())
}
fn ST_WALK_LOOP(e: &mut Engine) -> LResult<()> {
    let f = ev(e);
    if f & 0x400 != 0 {
        go(e, BUMP_SET);
    } else if f & 2 != 0 {
        clr_ev(e, 2);
        go(e, SURPRISED_SET);
    } else if f & 0x4000 != 0 {
        clr_ev(e, 0x4000);
        go(e, STAND_SET);
    } else if f & 0x80 != 0 {
        clr_ev(e, 0x80);
        let t = lr(e, 0x2c);
        APIextra::TimerReset(e, t);
        if GetNextNode(e) == 0 {
            TrackPos(e, 8.0);
        } else {
            go(e, GOTO_SET);
        }
    } else if f & 0x8000 != 0 {
        clr_ev(e, 0x8000);
        GetFirstNode(e);
        go(e, WALK_SET);
    }
    Ok(())
}
fn ST_SURPRISED_SET(e: &mut Engine) -> LResult<()> {
    lw(e, 4, 0);
    APIgameobj::SetContextFlag(e, SELF, 0x60);
    LIBAnimMngr::PlayAnim(e, 0.2, 1, 1, 3);
    DropWisp(e);
    APInewSound::Play(e, SELF, 0xa000_0001, 0);
    lw(e, 8, 0);
    go(e, SURPRISED_LOOP);
    Ok(())
}
fn ST_SURPRISED_LOOP(e: &mut Engine) -> LResult<()> {
    if ev(e) & 0x400 != 0 {
        go(e, BUMP_SET);
    } else if ev(e) & 0x100 != 0 {
        clr_ev(e, 0x100);
        for o in [0x20, 0x24] {
            let t = lr(e, o);
            APIextra::TimerReset(e, t);
        }
        GetFirstNode(e);
        go(e, WALK_SET);
    }
    Ok(())
}
fn ST_PUSH_DOOR_SET(e: &mut Engine) -> LResult<()> {
    lw(e, 4, 0);
    let (door, id) = (lr(e, 0x18), lr(e, 0x30));
    APIgameobj::SendMessageTo(e, door, NULL, 0xf0, id, NULL);
    APIgameobj::SendMessageTo(e, door, NULL, 0xef, id, NULL);
    LIBAnimMngr::PlayAnim(e, 0.2, 5, 1, 3);
    lw(e, 8, 0);
    go(e, PUSH_LOOP);
    Ok(())
}
fn ST_PUSH_LOOP(e: &mut Engine) -> LResult<()> {
    if ev(e) & 0x40 != 0 {
        clr_ev(e, 0x40);
        lw(e, 0x1c, 0);
        go(e, WAIT_SET);
    }
    Ok(())
}
fn ST_GOTO_SET(e: &mut Engine) -> LResult<()> {
    let (n, l) = (lr(e, 0x34), L(e));
    get_node(e, n, l + 0xb0);
    TrackPos(e, 15.0);
    APIgameobj::SetContextFlag(e, SELF, 0x66);
    let t = lr(e, 0x2c);
    APIextra::TimerReset(e, t);
    lw(e, 8, 0x20);
    go(e, GOTO_LOOP);
    Ok(())
}
fn ST_GOTO_LOOP(e: &mut Engine) -> LResult<()> {
    let f = ev(e);
    if f & 0x400 != 0 {
        go(e, BUMP_SET);
    } else if f & 4 != 0 {
        clr_ev(e, 4);
        lw(e, 0x1c, 1);
        go(e, WAIT_SET);
    } else if f & 0x8000 != 0 {
        clr_ev(e, 0x8000);
        GetFirstNode(e);
        go(e, WALK_SET);
    }
    Ok(())
}
fn ST_WAIT_DOOR_ACTION_SET(e: &mut Engine) -> LResult<()> {
    LIBAnimMngr::PlayAnim(e, 0.2, 3, 2, 0);
    if lr(e, 0x1c) == 1 {
        APIgameobj::SetContextFlag(e, SELF, 0x62);
        let (door, id) = (lr(e, 0x18), lr(e, 0x30));
        APIgameobj::SendMessageTo(e, door, NULL, 0xf1, id, NULL);
    } else {
        APIgameobj::SetContextFlag(e, SELF, 0x60);
    }
    lw(e, 8, 0);
    go(e, WAIT_LOOP);
    Ok(())
}
fn ST_WAIT_DOOR_ACTION_LOOP(e: &mut Engine) -> LResult<()> {
    if ev(e) & 0x10 != 0 {
        clr_ev(e, 0x10);
        go(e, PUSH_DOOR_SET);
    }
    if ev(e) & 0x20 != 0 {
        clr_ev(e, 0x20);
        GetFirstNode(e);
        go(e, WALK_SET);
    } else if ev(e) & 8 != 0 {
        clr_ev(e, 8);
        if lr(e, 0x3c) == 0 {
            go(e, WALK_SET);
        }
    }
    Ok(())
}
fn ST_BUMP_SET(e: &mut Engine) -> LResult<()> {
    lw(e, 4, 0);
    APIgameobj::SetContextFlag(e, SELF, 0x60);
    LIBAnimMngr::PlayAnim(e, 0.2, 6, 1, 3);
    lw(e, 8, 0);
    go(e, BUMP_LOOP);
    Ok(())
}
fn ST_BUMP_LOOP(e: &mut Engine) -> LResult<()> {
    if ev(e) & 0x1000 != 0 && ev(e) & 0x800 != 0 {
        clr_ev(e, 0x1000);
        GetFirstNode(e);
        go(e, WALK_SET);
    }
    Ok(())
}

pub fn FakeGhostFUNC_MessageFonction(e: &mut Engine, m: u32, p: u32, _: Ptr) -> LResult<u64> {
    match m {
        0xf2 => set_ev(e, if p == 1 { 0x10 } else { 0x20 }),
        0xee => set_ev(e, 8),
        0xc9 | 0xc1 => {
            let em = lr(e, 0xec) as i32;
            APIparticle::EmitterReset(e, em);
            APIgameobj::SetGameObjectVisible(e, SELF, 0);
            APIgameobj::SetContextFlag(e, SELF, 0x60);
            APIphysic::SetFlags(e, SELF, 1);
            APIgameobj::SetGroupBitMask(e, SELF, 0);
            APIgameobj::SendMessageTo(e, 0x2a0000b6, NULL, 0xeb, 2, NULL);
            let l = L(e);
            APIgameobj::SetTranslation(e, SELF, l + 0xc);
            go(e, BLACKHOLE);
        }
        0xf6 => set_ev(e, 0x800),
        0xf5 => set_ev(e, 0x400),
        0x11b => {
            APIgameobj::SetGameObjectVisible(e, SELF, 1);
            APIphysic::ResetFlags(e, SELF, 1);
            APIgameobj::SetGroupBitMask(e, SELF, 0x100001);
            APIgameobj::SendMessageTo(e, 0x2a0000b6, NULL, 0xeb, 2, NULL);
            GetFirstNode(e);
            go(e, STAND_SET);
        }
        _ => {}
    }
    Ok(0)
}
pub fn FakeGhostCB_COLLIDE_OBJ(e: &mut Engine, p: u32) -> LResult<u64> {
    if matches!(e.r32(p + 0xc), 200 | 0xc9 | 0xca | 0xcb) {
        set_ev(e, 4);
    }
    Ok(0)
}
pub fn FakeGhostCB_ANIM_LF(e: &mut Engine, p: u32) -> LResult<u64> {
    match p {
        1 => set_ev(e, 0x100),
        5 => set_ev(e, 0x40),
        6 => set_ev(e, 0x1000),
        _ => {}
    }
    Ok(0)
}
pub fn FakeGhostCB_CUT_SCENE(e: &mut Engine, p: u32) -> LResult<u64> {
    if e.r32(p + 4) == 1 {
        let id = lr(e, 0x30);
        LIBMiscFunc::MiscFunc_InitForCutscene(e, id);
        APIgameobj::SetNextState(e, SELF, 2);
    }
    Ok(0)
}
pub fn FakeGhostCB_ANIM(e: &mut Engine, p: u32) -> LResult<u64> {
    if e.r32(p + 4) != 2 {
        return Ok(0);
    }
    let j = match e.r32(p + 8) {
        1 | 3 => 0xd01000,
        0 | 2 => 0xd00000,
        _ => 0x7fff_ffff,
    };
    let (dm, em) = (lr(e, 0x54), lr(e, 0xec));
    LIBMiscFunc::MiscFunc_LaunchFootStep(e, dm, j, em);
    Ok(0)
}
pub fn FakeGhostCB_TIMER(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

fn TrackPos(e: &mut Engine, dist: f32) {
    let p = e.alloca(0x18);
    let (dm, l) = (lr(e, 0x40), L(e));
    APIphysic::DMGetParams(e, dm, p);
    let vals = [0, dist.to_bits(), e.r32(l + 0xb0), 0x3f00_0000, e.r32(l + 0xb8), 5];
    for (i, v) in vals.into_iter().enumerate() {
        e.w32(p + i as u32 * 4, v);
    }
    APIphysic::DMSetParams(e, dm, p);
}
fn SetSpeed(e: &mut Engine, s: f32) {
    let p = e.alloca(0x30);
    let dm = lr(e, 0x44);
    APIphysic::DMGetParams(e, dm, p);
    e.wf(p + 8, s);
    APIphysic::DMSetParams(e, dm, p);
}
fn DropWisp(e: &mut Engine) {
    let r = (APIstdlib::MyRand(e) as i32) % 100;
    let pad = e.r32(gb(e) + 0x4c) as i32;
    APIinput::InputSetVibration(e, pad, 0, 0.33, 0.0, 0.33, 0.0);
    let l = L(e);
    APIgameobj::GetPosition(e, SELF, 0x7fff_ffff, l + 0x58);
    let y = e.rf(l + 0x5c);
    e.wf(l + 0x80, y + 0.75);
    e.wf(l + 0x5c, y + 1.0);
    let n: [u32; 3] = match r {
        0..=19 => [2, 0, 0],
        20..=34 => [1, 1, 0],
        35..=49 => [3, 0, 0],
        50..=64 => [1, 2, 0],
        65..=79 => [0, 1, 1],
        80..=89 => [0, 2, 1],
        90..=99 => [0, 1, 2],
        _ => [0, 0, 0],
    };
    for (kind, cnt) in n.into_iter().enumerate() {
        for _ in 0..cnt {
            let o = APIgameobj::LaunchLatant(e, 0x2a800084);
            APIgameobj::SendMessageTo(e, o, NULL, 0x126, kind as u32, l + 0x58);
        }
    }
}
fn InitParticles_FootSteps(e: &mut Engine) {
    let t = APIparticle::TypeCreate(e);
    lw(e, 0xe8, t);
    let em = APIparticle::EmitterCreate(e, t as i32);
    lw(e, 0xec, em);
    let (t, em) = (t as i32, em as i32);
    let s = e.cstr("FootStep Peabody");
    APIparticle::TypeSetName(e, t, s);
    let s = e.cstr("footprintbrun_invers");
    APIparticle::TypeSetTexture(e, t, s);
    APIparticle::TypeSetInter(e, t, 0x10, 1.0, 1.0, 1.0, 1.0);
    APIparticle::TypeSetFloat(e, t, 2, 15.0);
    APIparticle::TypeSetFloat(e, t, 8, 0.0);
    APIparticle::TypeSetInter(e, t, 0x20, 0.0, 0.0, 0.0, 0.0);
    for k in [1, 2, 4, 8] {
        APIparticle::TypeSetInter(e, t, k, 1.0, 1.0, 0.0, 10.0);
    }
    APIparticle::TypeSetInt(e, t, 4, 0x168);
    APIparticle::TypeSetInt(e, t, 8, 1);
    APIparticle::TypeSetInt(e, t, 1, 0x201);
    let f = e.alloca(4);
    APIparticle::EmitterGetFlags(e, em, f);
    let v = e.r32(f) & 0xffff_ffbf;
    APIparticle::EmitterSetFlags(e, em, v);
    APIparticle::EmitterSetJoint(e, em, SELF, 0x7fff_ffff);
}
fn GetFirstNode(e: &mut Engine) {
    let l = L(e);
    let p = path(e);
    if LIBApiPathFinding::IsOnPath_GetNode(e, SELF, l + 0x34, p as i32) == 0 {
        let v = e.alloca(0x30);
        let (me, node, dir) = (v, v + 0x10, v + 0x20);
        let mut best = 10000.0f32;
        let mut pick = 0;
        APIgameobj::GetPosition(e, SELF, 0x7fff_ffff, me);
        let n = LIBApiPathFinding::GetNeighborNodesRange(e, 5.0, me, (l + 0x100) as i32, p as i32);
        for i in 0..n.max(0) as u32 {
            let id = e.r32(l + 0x100 + i * 4);
            get_node(e, id, node);
            if RayIntersect(e, me, node) {
                let d = APImath::CVectorGetDirectionAndDistanceTo(e, dir, me, node);
                if d < best {
                    best = d;
                    pick = id;
                }
            }
        }
        lw(e, 0x34, pick);
    }
    let n = lr(e, 0x34);
    get_node(e, n, l + 0xb0);
    let v = if neighbors(e, n) < 1 { n } else { lr(e, 0x100) };
    lw(e, 0x38, v);
}
/// returns 1 when the chosen branch goes through a door (null_printf traces dropped)
fn GetNextNode(e: &mut Engine) -> u32 {
    let r = LIBApiCFunctions::FloatRandom(e, 0.0, 100.0, 0.05);
    let mut ret = 0;
    GetDirectionInfo(e);
    let mut free = vec![];
    for i in 0..3 {
        if lr(e, 0x8c + i * 0xc) == 0 {
            free.push(lr(e, 0x90 + i * 0xc));
        }
    }
    let next = match free.len() {
        1 => free[0],
        2 => free[(r > 50.0) as usize],
        3 => {
            if r <= 50.0 {
                lr(e, 0x90)
            } else if r <= 75.0 {
                lr(e, 0xa8)
            } else {
                lr(e, 0x9c)
            }
        }
        _ => {
            let mut nx = 0xffff_ffff;
            for i in 0..3 {
                if lr(e, 0x8c + i * 0xc) == 2 {
                    let d = lr(e, 0x94 + i * 0xc);
                    lw(e, 0x18, d);
                    nx = lr(e, 0x90 + i * 0xc);
                    ret = 1;
                    break;
                }
            }
            if nx == 0xffff_ffff {
                lr(e, 0x38)
            } else {
                nx
            }
        }
    };
    let cur = lr(e, 0x34);
    lw(e, 0x38, cur);
    lw(e, 0x34, next);
    let l = L(e);
    get_node(e, next, l + 0xb0);
    ret
}
/// front/left/right slots: kind 1 = none, 0 = free, 2 = through a door
fn GetDirectionInfo(e: &mut Engine) {
    let l = L(e);
    let v = e.alloca(0x20);
    let (here, there) = (v, v + 0x10);
    let cur = lr(e, 0x34);
    get_node(e, cur, here);
    let n = neighbors(e, cur);
    for o in [0x8c, 0x98, 0xa4] {
        lw(e, o, 1);
    }
    if n == 1 {
        return;
    }
    for i in 0..n.max(0) as u32 {
        let nb = lr(e, 0x100 + i * 4);
        if nb == lr(e, 0x38) {
            continue;
        }
        let (cur, prev) = (lr(e, 0x34), lr(e, 0x38));
        let k = GetAngleType(e, cur, prev, nb);
        get_node(e, nb, there);
        let d = DoorIntersect(e, here, there, l + 0x94 + k * 0xc);
        lw(e, 0x8c + k * 0xc, if d == 0 { 0 } else { 2 });
        lw(e, 0x90 + k * 0xc, nb);
    }
}
/// clear horizontal line of sight a -> b at height 0.5 (ray params at L+0xbc, flags at L+0xd4)
fn RayIntersect(e: &mut Engine, a: Ptr, b: Ptr) -> bool {
    let l = L(e);
    let (ax, az) = (e.rf(a), e.rf(a + 8));
    let (dx, dz) = (e.rf(b) - ax, e.rf(b + 8) - az);
    for (o, v) in [(0xbc, ax), (0xc0, 0.5), (0xc4, az), (200, dx), (0xcc, 0.0), (0xd0, dz)] {
        e.wf(l + o, v);
    }
    let out = e.alloca(0x100);
    APIphysic::CheckRayIntersection(e, l + 0xbc, out) == 0
}
/// 0 straight on, 1 / 2 by the sign of the xz cross product (|c| <= 3 counts as straight)
fn GetAngleType(e: &mut Engine, a: u32, b: u32, c: u32) -> u32 {
    let v = e.alloca(0x30);
    for (i, n) in [a, b, c].into_iter().enumerate() {
        get_node(e, n, v + i as u32 * 0x10);
    }
    let x = |i: u32| e.rf(v + i * 0x10);
    let z = |i: u32| e.rf(v + i * 0x10 + 8);
    let f = (x(2) - x(0)) * (z(1) - z(0)) - (x(1) - x(0)) * (z(2) - z(0));
    if (-3.0..=3.0).contains(&f) {
        0
    } else if f > 0.0 {
        2
    } else {
        1
    }
}
fn DoorIntersect(e: &mut Engine, a: Ptr, b: Ptr, out: Ptr) -> u32 {
    let v = e.alloca(0x20);
    for d in DOORS {
        APIgameobj::GetPosition(e, d, 2, v);
        APIgameobj::GetPosition(e, d, 3, v + 0x10);
        if LineIntersect(e, a, b, v, v + 0x10) {
            e.w32(out, d);
            return 1;
        }
    }
    e.w32(out, 0);
    0
}
/// segment a-b against door segment c-d in the xz plane. The perpendiculars' y components are
/// uninitialised stack in Ghidra; taken as 0 (2D test).
fn LineIntersect(e: &mut Engine, a: Ptr, b: Ptr, c: Ptr, d: Ptr) -> bool {
    let r = |p: Ptr| [e.rf(p), e.rf(p + 4), e.rf(p + 8)];
    let (a, b, c, d) = (r(a), r(b), r(c), r(d));
    let dot = |u: [f32; 3], v: [f32; 3]| u[0] * v[0] + u[1] * v[1] + u[2] * v[2];
    let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let cd = [d[0] - c[0], d[1] - c[1], d[2] - c[2]];
    let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    let n_cd = [-cd[2], 0.0, cd[0]];
    let n_ab = [-ab[2], 0.0, ab[0]];
    let den = dot(n_cd, ab);
    let t = dot(n_cd, ac) / den;
    if !(0.0..=1.0).contains(&t) {
        return false;
    }
    let u = dot(n_ab, ac) / den;
    (0.0..=1.0).contains(&u)
}
fn Create_Physic(e: &mut Engine) {
    let l = L(e);
    APIphysic::CreatePhysic(e, SELF, 0);
    APIphysic::SetFlags(e, SELF, 6);
    APIphysic::SetPrimaryPrim(e, SELF, 900);
    APIphysic::SetWeight(e, SELF, 0.75);
    APIphysic::SetPriority(e, SELF, 10);
    APIphysic::PrimSetIntAttribBits(e, SELF, 900, 5, 0x200000);
    let ld = leader(e);
    let p = e.alloca(0x48);
    let dms: [(u32, u32, u32, Vec<(u32, u32)>); 5] = [
        (1, 0x40, 2, vec![(0, ld), (4, 0x4040_0000), (8, 0), (0xc, 0), (0x10, 0), (0x14, 5)]),
        (4, 0x44, 4, vec![(0, 0), (4, 0), (8, 0x4000_0000), (0xc, 1)]),
        (7, 0x48, 1, vec![(0, 0x41a0_0000), (4, 7)]),
        (0, 0x4c, 0x40, vec![(0, 0), (4, 0xc1a0_0000), (8, 0)]),
        (8, 0x50, 0x20, vec![(0, 0x4296_0000), (4, 0x4234_0000)]),
    ];
    for (kind, slot, mask, set) in dms {
        let h = APIphysic::DMAdd(e, SELF, kind);
        lw(e, slot, h);
        APIphysic::DMGetParams(e, h, p);
        for (o, v) in set {
            e.w32(p + o, v);
        }
        APIphysic::DMSetParams(e, h, p);
        APIphysic::DMSetContextMask(e, h, mask);
    }
    let h = APIphysic::DMAdd(e, SELF, 0x1d);
    lw(e, 0x54, h);
    APIphysic::DMSetActive(e, h, 0);
    APIphysic::DMSetContextMask(e, h, 1);
    let id = lr(e, 0x30);
    for (o, v) in [(0xd4, 0x100_0000), (0xd8, 0), (0xdc, 0x200008), (0xe0, 0), (0xe4, id)] {
        lw(e, o, v);
    }
}
fn GameEventSignaler(e: &mut Engine) {
    let c = lr(e, 8);
    if c == 0 {
        return;
    }
    let ld = leader(e);
    if c & 2 != 0 && e.r32(gb(e) + 0x14) & 0x800090c == 0 {
        let t = lr(e, 0x24);
        if APIextra::TimerGetTime(e, t) >= 5.0
            && (LIBApiCFunctions::CheckRangeObject(e, 1.0, SELF, ld, 0) != 0
                || (LIBApiCFunctions::CheckRangeObject(e, 3.5, SELF, ld, 0) != 0
                    && LIBApiCFunctions::CheckPositionOrientation(e, SELF, ld, 0xc003) != 0))
        {
            set_ev(e, 2);
        }
    }
    let l = L(e);
    if c & 8 != 0 && LIBApiCFunctions::CheckRangePoint(e, 0.75, SELF, l + 0xb0, 0) != 0 {
        set_ev(e, 0x80);
    }
    if c & 4 != 0 {
        let t = lr(e, 0x20);
        if APIextra::TimerGetTime(e, t) >= 15.0 {
            set_ev(e, 0x200);
        }
    }
    if c & 0x10 != 0 {
        let v = e.alloca(0x20);
        APIgameobj::GetPosition(e, SELF, 0x7fff_ffff, v);
        APIgameobj::GetPosition(e, ld, 0x7fff_ffff, v + 0x10);
        if RayIntersect(e, v, v + 0x10) {
            set_ev(e, 0x2000);
        } else {
            let t = lr(e, 0x28);
            if APIextra::TimerGetTime(e, t) > 5.0 {
                set_ev(e, 0x4000);
            }
        }
    }
    if c & 0x20 != 0 {
        let t = lr(e, 0x2c);
        if APIextra::TimerGetTime(e, t) >= 2.0 {
            set_ev(e, 0x8000);
        }
    }
}

pub fn ctor(e: &mut Engine, api: Ptr, n: i32) -> LResult<()> {
    if n == 1 {
        ext(e, &[
            (0, FakeGhostST_FIRSTMOVE_SET),
            (1, FakeGhostST_ENTRY_POINT),
            (4, FakeGhostCB_ANIM),
            (3, FakeGhostCB_ANIM_LF),
            (6, FakeGhostCB_TIMER),
            (5, FakeGhostCB_CUT_SCENE),
            (7, FakeGhostCB_COLLIDE_OBJ),
        ]);
        let v = APIcons::GetApiAnim(e); gset(e, pAnim, v);
        let v = APIcons::GetApiGameObj(e); gset(e, pGameObj, v);
        let v = APIcons::GetApiExtra(e); gset(e, pExtra, v);
        let v = APIcons::GetApiPhysic(e); gset(e, pPhysic, v);
        let v = APIcons::GetApiStdlib(e); gset(e, pStdlib, v);
        let v = APIcons::GetApiParticle(e); gset(e, pParticle, v);
        let v = APIcons::GetApiMath(e); gset(e, pMath, v);
        let v = APIcons::GetApiLight(e); gset(e, pLight, v);
        let v = APIcons::GetApiGraphics(e); gset(e, pGraphic, v);
        let v = APIcons::GetApiNewSound(e); gset(e, pSound, v);
        custom(e, pDebug, "debug");
        custom(e, pAnimMngr, "AnimMngr");
        custom(e, pFunction, "ApiCFunctions");
        custom(e, pMiscFunc, "MiscFunc");
        let v = APIcons::GetApiInput(e); gset(e, pInput, v);
        custom(e, pPathFinding, "ApiPathFinding");
        let s = G(e, pGlobalDataFakeGhost);
        APIcons::GetGlobalData(e, s, 4);
        let b = APIgameobj::GetGameBuffer(e, 0, 0x8f8, 1);
        let g = e.r32(G(e, pGlobalDataFakeGhost));
        e.w32(g, b);
    }
    let s = G(e, pLocalDataFakeGhost);
    APIcons::GetLocalData(e, s, 300);
    let id = APIgameobj::GetID(e, SELF, NULL);
    lw(e, 0x30, id);
    APIcons::SetMessageFunction(e, FakeGhostFUNC_MessageFonction);
    // srand seed: the f0 at the cvt is TimerGetTime's result (see other ports)
    let t = APIextra::TimerGetTime(e, 0x9000_0000);
    APIstdlib::srand(e, t as i32 as u32);
    LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 2, NULL, 0);
    Create_Physic(e);
    APIgameobj::RegisterNotify(e, SELF, 0, 0x16, 5);
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 3, 7);
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 0xb, 4);
    for o in [0x20, 0x24, 0x28, 0x2c] {
        let t = APIextra::TimerCreate(e);
        lw(e, o, t);
    }
    InitParticles_FootSteps(e);
    let l = L(e);
    APIgameobj::GetPosition(e, SELF, 0x7fff_ffff, l + 0xc);
    lw(e, 0xfc, 1);
    let id = APIgameobj::GetID(e, SELF, NULL);
    lw(e, 0xf0, id);
    APIcons::SetInitialState(e, 0);
    Ok(())
}
pub fn dtor(e: &mut Engine, _: i32) -> LResult<()> {
    for o in [0x20, 0x24, 0x28, 0x2c] {
        let t = lr(e, o);
        APIextra::TimerDestroy(e, t);
    }
    APIgameobj::SetGroupID(e, SELF, 0);
    Ok(())
}

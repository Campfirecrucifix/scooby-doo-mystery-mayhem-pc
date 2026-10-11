//! `automaticdoor`: EP6 (Sherman Tech) sliding doors. Open when the hero touches them (unless locked by a
//! key bit / disabled by a level bit), close after the open timer runs out; AI handshake with a partner
//! door (0x1a0/0x1a3/0x1a4/0x1a5, 299/300) for enemies walking through. Port of
//! `Game Logic Modules/Ep6_ShermanTech/Objects/AUTOMATICDOOR/LEVEL*/automaticdoor__*/`
//! (automaticdoor_base.c + automaticdoor.c).
//!
//! Builds (table-driven, see [`VARIANT`]):
//! - 7a09c5c9 (L01): oldest base (no debug prints); local ext 2 `LocalCB_NOTIFY_COLLIDE_OBJ` (sets the
//!   lock bit before the base test) and a fixed SetLocalClipping.
//! - 134ae423 (L02): anims shifted by 2, `ExceptionLevelL02_R_D_DEPT` in the collide test, extended
//!   DoorLocked, radar message in Enable, ctor extras.
//! - 88fadccc (L04A), 4c5da91d (L04B), 7eb8d4e1 (L09): base with debug prints; SetLocalClipping widens
//!   the clipping of door 0x2a00016a (L04A only, empty elsewhere).
//!
//! The builds' module globals sit right after mInitautomaticDoors (n x 0x14), so their addresses move.
//!
//! pLocalDataautomaticdoor_base (0x88): +0 state fn (code token), +4 flags (2 enabled, 4 hero in, 0x20
//! reserve, 0x80 AI done, 0x100 open anim end, 0x200 close anim end, 0x400 second door, 0x800 blocked),
//! +8 DM handle, +0xc current anim, +0x10 partner door, +0x14 lock mask, +0x18 locked text, +0x1c AI
//! param, +0x20 timer, +0x24 disable bit, +0x2c/+0x30 times, +0x34 second door, +0x38 AI busy,
//! +0x3c/+0x40 disabled / lock sound played, +0x44, +0x48 disabled at start, +0x4c collided this
//! frame, +0x50 open countdown, +0x54 cutscene clipping ptr, +0x58/+0x64 own pos 5/6, +0x70/+0x7c
//! partner pos 5/6.
#![allow(non_snake_case, non_upper_case_globals, unused_variables)]

use crate::engine::*;
use crate::logic::{CodeFn, ExtFn, LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "automaticdoor",
    // largest image: 7eb8d4e1, globals up to 0x101ec + gpoLanguagesNamesAndISOCodes at 0x101f0
    data_size: 0x200,
    ctor: automaticdoorConstructor,
    dtor: automaticdoorDestructor,
    register: None,
    init: &[],
    build_init: Some(build_init),
};

const KEYS: [&str; 5] = ["7a09c5c9", "134ae423", "88fadccc", "4c5da91d", "7eb8d4e1"];

pub fn build_index(key: &str) -> Option<usize> {
    KEYS.iter().position(|k| *k == key)
}

#[derive(Clone, Copy, PartialEq)]
enum V {
    /// L01
    Old,
    /// L02
    L02,
    /// L04A / L04B / L09
    New,
}
const VARIANT: [V; 5] = [V::Old, V::L02, V::New, V::New, V::New];
/// SetLocalClipping door id (New/L02 builds; 0 = empty function)
const LOCAL_CLIP_ID: [u32; 5] = [0, 0x2a000147, 0x2a00016a, 0, 0];

/// mInitautomaticDoors (0x10000): {door id, partner, lock mask, locked text, +0x44}
const DOORS: [&[[u32; 5]]; 5] = [
    &[
        [0x2a000056, 0x2a000042, 0, 0, 0], [0x2a000042, 0x2a000056, 0, 0, 0],
        [0x2a000051, 0x2a000044, 0, 0, 0], [0x2a000044, 0x2a000051, 0, 0, 0],
        [0x2a00004c, 0x2a000050, 0, 0, 0], [0x2a000050, 0x2a00004c, 0, 0, 0],
        [0x2a000041, 0x2a000052, 0, 0, 0], [0x2a000052, 0x2a000041, 0, 0, 0],
        [0x2a000053, 0x2a00004e, 0, 0, 0], [0x2a00004e, 0x2a000053, 0, 0, 0],
        [0x2a00004f, 0x2a00004d, 0, 0, 0], [0x2a00004d, 0x2a00004f, 0, 0, 0],
        [0x2a00004c, 0x2a000050, 0, 0, 0], [0x2a000050, 0x2a00004c, 0, 0, 0],
        [0x2a000059, 0x2a000055, 0, 0, 0], [0x2a000055, 0x2a000059, 0, 0, 0],
        [0x2a00004b, 0x2a000049, 0, 0, 0], [0x2a000049, 0x2a00004b, 0, 0, 0],
        [0x2a000043, 0, 0, 0, 0],
    ],
    &[
        [0x2a000146, 0, 0x80000000, 0, 1], [0x2a000145, 0, 0x800, 0x15c, 1],
        [0x2a000147, 0, 0x40000000, 0x159, 0], [0x2a00015b, 0, 0x02000000, 0x157, 1],
        [0x2a000180, 0, 0x04000000, 0x159, 0], [0x2a000119, 0, 0x40000000, 0x159, 0],
        [0x2a000125, 0, 0, 0, 1], [0x2a00015a, 0, 0, 0, 1], [0x2a00017d, 0, 0, 0, 1],
        [0x2a00017e, 0, 0x40000000, 0x159, 1], [0x2a00017f, 0, 0, 0, 1],
    ],
    &[
        [0x2a000106, 0, 0x40000000, 0x159, 0], [0x2a00016a, 0, 0x40000000, 0x159, 0],
        [0x2a000168, 0, 0, 0, 1], [0x2a000137, 0, 0, 0, 1], [0x2a000182, 0, 0, 0, 1],
        [0x2a000136, 0, 0, 0, 1], [0x2a000103, 0, 0, 0, 1], [0x2a000104, 0, 0x40000000, 0x159, 1],
        [0x2a000105, 0, 0, 0, 1], [0x2a00018b, 0, 0x00010000, 0x159, 0],
    ],
    &[
        [0x2a00005a, 0, 0x40000000, 0x156, 0], [0x2a000043, 0, 0x40000000, 0x156, 0],
        [0x2a000044, 0, 0x40000000, 0x156, 0], [0x2a000053, 0, 0x40000000, 0x156, 0],
        [0x2a000054, 0, 0x40000000, 0x156, 0],
    ],
    &[
        [0x2a000017, 0, 0x40000000, 0x182, 1], [0x2a000018, 0, 0x40000000, 0x182, 1],
        [0x2a000015, 0, 0x40000000, 0x182, 1], [0x2a000016, 0, 0, 0, 1],
        [0x2a000014, 0, 0x40000000, 0x182, 1], [0x2a000021, 0, 0x40000000, 0x182, 1],
        [0x2a000022, 0, 0, 0, 1], [0x2a000013, 0, 0, 0, 1],
        [0x2a000012, 0, 0x40000000, 0x182, 1], [0x2a000023, 0, 0x40000000, 0x182, 1],
        [0x2a000024, 0, 0x40000000, 0x182, 1], [0x2a000010, 0, 0, 0, 1],
        [0x2a000011, 0, 0x40000000, 0x182, 1], [0x2a00000f, 0, 0x40000000, 0x182, 1],
        [0x2a000019, 0, 0x40000000, 0x182, 1], [0x2a00001a, 0, 0x40000000, 0x182, 1],
        [0x2a00001b, 0, 0x40000000, 0x182, 1], [0x2a00001c, 0, 0x40000000, 0x182, 1],
        [0x2a00001d, 0, 0x40000000, 0x182, 1], [0x2a00001e, 0, 0x40000000, 0x182, 1],
        [0x2a00001f, 0, 0x40000000, 0x182, 1], [0, 0, 0, 0, 0],
    ],
];

fn build_init(m: &mut Mem, data: Ptr, b: usize) {
    for (i, row) in DOORS[b].iter().enumerate() {
        for (j, &v) in row.iter().enumerate() {
            m.w32(data + 0x14 * i as u32 + 4 * j as u32, v);
        }
    }
}

// module globals, as offsets after mInitautomaticDoors
const pExtra: u32 = 0;
const pGameObj: u32 = 4;
const pPhysic: u32 = 8;
const pAnim: u32 = 0xc;
const pGame: u32 = 0x10;
const pMath: u32 = 0x14;
const pSound: u32 = 0x18;
const pDebug: u32 = 0x1c;
const pAnimMngr: u32 = 0x20;
const pMiscFunc: u32 = 0x24;
const pText: u32 = 0x28;
const pLocalDataautomaticdoor_base: u32 = 0x2c;
const pGlobalDataautomaticdoor_base: u32 = 0x30;

fn b(e: &Engine) -> usize {
    e.rt.cur_build()
}
fn var(e: &Engine) -> V {
    VARIANT[b(e)]
}
/// address of a module global (0x10000 + mInitautomaticDoors size + off)
fn G(e: &Engine, off: u32) -> Ptr {
    e.rt.cur_data() + DOORS[b(e)].len() as u32 * 0x14 + off
}
fn L(e: &Engine) -> Ptr {
    e.r32(G(e, pLocalDataautomaticdoor_base))
}
/// pGlobalDataautomaticdoor_base[i] (0: game buffer 0 (0x8f8), 1: game buffer 2 (0x158))
fn gbuf(e: &Engine, i: u32) -> Ptr {
    let g = e.r32(G(e, pGlobalDataautomaticdoor_base));
    e.r32(g + 4 * i)
}
fn hero(e: &Engine, o: u32) -> u32 {
    let b0 = gbuf(e, 0);
    e.r32(b0 + o)
}
fn flags(e: &Engine) -> u32 {
    e.r32(L(e) + 4)
}
fn set_flags(e: &mut Engine, v: u32) {
    let l = L(e);
    e.w32(l + 4, v);
}
/// anims: (closed/closing, idle at start, opening); L02 has them shifted by 2
fn anims(e: &Engine) -> (i32, i32, i32) {
    if var(e) == V::L02 { (5, 3, 4) } else { (3, 1, 2) }
}
fn debug(e: &mut Engine, msg: &str) {
    if e.r32(G(e, pDebug)) != 0 {
        let (a, m) = (e.cstr("automaticdoor_base"), e.cstr(msg));
        LIBdebug::PrintDebugInfo(e, a, m);
    }
}

type StateFn = fn(&mut Engine, u32) -> u32;
/// `*param_1 = f; (*param_1)(param_1)`: the state function lives in local+0 as a code token
fn goto(e: &mut Engine, p: Ptr, f: StateFn) {
    set_state(e, p, f);
    run_state(e, p);
}
fn set_state(e: &mut Engine, p: Ptr, f: StateFn) {
    let t = e.code_ptr(CodeFn::Word(f));
    e.w32(p, t);
}
fn run_state(e: &mut Engine, p: Ptr) {
    let t = e.r32(p);
    if let CodeFn::Word(f) = e.code_fn(t) {
        f(e, p);
    }
}

/// automaticdoor_baseST_FIRSTMOVE @0x00010e90 (L01 address)
pub fn automaticdoor_baseST_FIRSTMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    let (close, idle, _) = anims(e);
    let l = L(e);
    e.w32(l + 0xc, close as u32);
    LIBAnimMngr::PlayAnim(e, 0.0, idle, 1, 0);
    let t = APIanim::GetTime(e, SELF, close as u32);
    APIanim::SetTime(e, SELF, close as u32, t);
    automaticdoor_baseFUNC_SetInPrim(e);
    e.w32(l + 0x38, 0);
    let t = APIextra::TimerGetTime(e, 0x9000_0000);
    e.wf(l + 0x30, t + 5.0);
    e.w32(l + 4, 0);
    let f = e.r32(l + 4) | 2;
    e.w32(l + 4, f);
    let m = e.r32(l + 0x24);
    let b1 = gbuf(e, 1);
    if m == 0 || e.r32(b1 + 8) & m == 0 {
        e.w32(l + 0x48, 0);
        automaticdoor_baseFUNC_Enable(e);
    } else {
        e.w32(l + 0x48, 1);
        automaticdoor_baseFUNC_Disable(e);
    }
    SetClipping(e);
    SetLocalClipping(e);
    set_state(e, l, automaticdoor_baseState_Close_In);
    APIgameobj::SetNextState(e, SELF, 1);
    Ok(0)
}

/// automaticdoor_baseST_ENTRY_POINT @0x00011078
pub fn automaticdoor_baseST_ENTRY_POINT(e: &mut Engine, _: u32) -> LResult<u64> {
    let l = L(e);
    run_state(e, l);
    Ok(0)
}

/// automaticdoor_baseState_Close_In @0x000110d0
fn automaticdoor_baseState_Close_In(e: &mut Engine, p: u32) -> u32 {
    let (close, _, _) = anims(e);
    e.w32(p + 0x34, 0);
    if e.ri32(p + 0xc) != close {
        let a = LIBAnimMngr::PlayAnim(e, 0.0, close, 1, 4);
        e.w32(p + 0xc, a as u32);
    }
    goto(e, p, automaticdoor_baseState_Close_Execute);
    0
}

/// automaticdoor_baseState_Close_Execute @0x00011168
fn automaticdoor_baseState_Close_Execute(e: &mut Engine, p: u32) -> u32 {
    let f = flags(e);
    if f & 0x400 == 0 {
        if f & 0x20 == 0 {
            if !f & 6 == 0 {
                if var(e) != V::Old {
                    debug(e, "State_Close_Execute -> Opening");
                }
                goto(e, p, automaticdoor_baseState_Open_In);
            }
        } else {
            set_flags(e, f & 0xffff_ffdf);
            automaticdoor_baseFUNC_ReserveDoor(e);
            if e.r32(p + 0x34) == 0 {
                goto(e, p, automaticdoor_baseState_AIOpen_In);
            } else {
                goto(e, p, automaticdoor_baseState_AIWait_SecondDoor);
            }
        }
    } else {
        set_flags(e, f & 0xffff_fbff);
        goto(e, p, automaticdoor_baseState_AIOpen_In);
    }
    let f = flags(e);
    if f & 0x200 != 0 {
        set_flags(e, f & 0xffff_fdff);
        APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 3, 2);
    }
    0
}

/// automaticdoor_baseState_Open_In @0x00011348
fn automaticdoor_baseState_Open_In(e: &mut Engine, p: u32) -> u32 {
    let (_, _, open) = anims(e);
    if e.ri32(p + 0xc) != open {
        let a = LIBAnimMngr::PlayAnim(e, 0.0, open, 1, 0);
        e.w32(p + 0xc, a as u32);
    }
    automaticdoor_baseFUNC_SetOutPrim(e);
    let t = APIextra::TimerGetTime(e, 0x9000_0000);
    e.wf(p + 0x2c, t + 3.0);
    goto(e, p, automaticdoor_baseState_Open_Execute);
    0
}

/// automaticdoor_baseState_Open_Execute @0x00011420
fn automaticdoor_baseState_Open_Execute(e: &mut Engine, p: u32) -> u32 {
    if flags(e) & 4 == 0 {
        let dt = APIgame::GetFrameDeltaTime(e);
        let v = e.rf(p + 0x50) - dt;
        e.wf(p + 0x50, v);
        if v <= 0.0 {
            APIgameobj::UnregisterNotify(e, SELF, 0xffff_ffff, 3);
            goto(e, p, automaticdoor_baseState_Close_In);
        }
    }
    0
}

/// automaticdoor_baseState_AIClose_In @0x000114f0
fn automaticdoor_baseState_AIClose_In(e: &mut Engine, p: u32) -> u32 {
    let (close, _, _) = anims(e);
    let a = LIBAnimMngr::PlayAnim(e, 0.0, close, 1, 4);
    e.w32(p + 0xc, a as u32);
    goto(e, p, automaticdoor_baseState_AIClose_Execute);
    0
}

/// automaticdoor_baseState_AIClose_Execute @0x00011570
fn automaticdoor_baseState_AIClose_Execute(e: &mut Engine, p: u32) -> u32 {
    if flags(e) & 0x200 != 0 {
        let l = L(e);
        if e.r32(p + 0x34) == 0 {
            let (a, b4) = (e.r32(p + 0x1c), e.r32(p + 0x10));
            APIgameobj::SendMessageTo(e, a, NULL, 299, b4, l + 0x70);
            APIgameobj::SendMessageTo(e, b4, NULL, 0x1a5, 0, NULL);
        }
        let t = e.r32(l + 0x20);
        APIextra::TimerRegisterNotify(e, SELF, t, 5);
        APIextra::TimerReset(e, t);
        goto(e, p, automaticdoor_baseState_Close_In);
    }
    0
}

/// automaticdoor_baseState_AIOpen_In @0x00011698
fn automaticdoor_baseState_AIOpen_In(e: &mut Engine, p: u32) -> u32 {
    let (_, _, open) = anims(e);
    let a = LIBAnimMngr::PlayAnim(e, 0.0, open, 1, 4);
    e.w32(p + 0xc, a as u32);
    goto(e, p, automaticdoor_baseState_AIOpen_Execute);
    0
}

/// automaticdoor_baseState_AIOpen_Execute @0x00011718
fn automaticdoor_baseState_AIOpen_Execute(e: &mut Engine, p: u32) -> u32 {
    if flags(e) & 0x100 != 0 {
        let id = APIgameobj::GetID(e, SELF, NULL);
        let f = flags(e);
        set_flags(e, f & 0xffff_feff);
        let ai = e.r32(p + 0x1c);
        if ai != 0 {
            let l = L(e);
            APIgameobj::SendMessageTo(e, ai, NULL, 300, id, l + 0x58);
        }
        goto(e, p, automaticdoor_baseState_AIWait_Action);
    }
    0
}

/// automaticdoor_baseState_AIWait_Action @0x00011810
fn automaticdoor_baseState_AIWait_Action(e: &mut Engine, p: u32) -> u32 {
    let f = flags(e);
    if f & 0x80 != 0 {
        set_flags(e, f & 0xffff_ff7f);
        goto(e, p, automaticdoor_baseState_AIClose_In);
    }
    0
}

/// automaticdoor_baseState_AIWait_SecondDoor @0x000118a0
fn automaticdoor_baseState_AIWait_SecondDoor(e: &mut Engine, p: u32) -> u32 {
    let f = flags(e);
    if f & 0x400 != 0 {
        set_flags(e, f & 0xffff_fbff);
        goto(e, p, automaticdoor_baseState_AIOpen_In);
    }
    0
}

/// write the clipping params at c (EFClippingParams words 0..9)
fn clip_words(e: &mut Engine, c: Ptr, w: [u32; 10]) {
    for (i, v) in w.into_iter().enumerate() {
        e.w32(c + 4 * i as u32, v);
    }
}

/// SetClipping @0x00011930
fn SetClipping(e: &mut Engine) {
    let c = e.alloca(0x40);
    APIgameobj::GetClippingParams(e, SELF, c);
    clip_words(e, c, [0x3f80_0000, 0x4170_0000, 7, 0, 0x4248_0000, 0xa7, 0x40, 0xbf80_0000, 0x6bf, 0x6bf]);
    APIgameobj::SetClippingParams(e, SELF, c, 1);
}

/// SetLocalClipping (automaticdoor.c; L01 @0x00013138: fixed params; L02 @0x000135d8 / L04A @0x000130c8:
/// one door gets every clipping level ORed with 7 + prim bit; L04B / L09: empty)
fn SetLocalClipping(e: &mut Engine) {
    let c = e.alloca(0x40);
    if var(e) == V::Old {
        APIgameobj::GetClippingParams(e, SELF, c);
        clip_words(e, c, [0x3f80_0000, 0x4170_0000, 7, 0, 0x4282_0000, 0x9f, 0, 0xbf80_0000, 0x9f, 0x9f]);
        APIgameobj::SetClippingParams(e, SELF, c, 1);
        return;
    }
    let want = LOCAL_CLIP_ID[b(e)];
    if want == 0 {
        return;
    }
    if APIgameobj::GetID(e, SELF, NULL) == want {
        APIgameobj::GetClippingParams(e, SELF, c);
        // auStack_68 = params + 8
        for i in 0..5u32 {
            for k in [1, 0] {
                let a = c + 8 + 4 * (i * 3 + k);
                let v = e.r32(a) | 7;
                e.w32(a, v);
            }
        }
        APIgameobj::SetClippingParams(e, SELF, c, 1);
        APIphysic::PrimSetIntAttribBits(e, SELF, 0x3ad, 0, 0x10);
    }
}

/// ExceptionLevelL02_R_D_DEPT (L02 only): 1 = do not open
fn ExceptionLevelL02_R_D_DEPT(e: &mut Engine, p: u32) -> u32 {
    let l = L(e);
    let id = APIgameobj::GetID(e, SELF, NULL);
    if id == 0x2a000145 {
        let u = e.r32(p + 8);
        if 0x2a000040 < u {
            if u < 0x2a000043 {
                let b1 = gbuf(e, 1);
                if e.r32(b1 + 0xc) & e.r32(l + 0x14) == 0 {
                    debug(e, "Locked");
                } else {
                    debug(e, "Opened");
                    automaticdoor_baseFUNC_Disable(e);
                    let v = e.r32(b1 + 8) | e.r32(l + 0x24);
                    e.w32(b1 + 8, v);
                    APIgameobj::SendMessageTo(e, 0x2a100006, NULL, 0xb, 0xd, NULL);
                }
            } else if u == 0x2a00012d && e.r32(l + 0x3c) == 0 {
                return 1;
            }
        }
    } else if APIgameobj::GetID(e, SELF, NULL) == 0x2a00015a {
        if flags(e) & 0x800 != 0 {
            APInewSound::Play(e, SELF, 0x40000001, 0);
            return 1;
        }
        APIgameobj::SendMessageTo(e, 0x2a000046, NULL, 0xde, 5, NULL);
    }
    let u = e.r32(p + 8);
    (u == 0x2a000153 || u == 0x2a00017c) as u32
}

/// automaticdoor_baseCB_NOTIFY_COLLIDE_OBJ @0x000119f0 (param: collision info, +8 other id, +0xc prim)
pub fn automaticdoor_baseCB_NOTIFY_COLLIDE_OBJ(e: &mut Engine, p: u32) -> LResult<u64> {
    let l = L(e);
    let other = e.r32(p + 8);
    let prim = e.r32(p + 0xc);
    let b0 = gbuf(e, 0);
    let ok = e.r32(l + 0x38) == 0
        && (other != e.r32(b0 + 8) || e.r32(b0 + 0x14) & 0x0e20_090c == 0)
        && prim != 0x3e3
        && prim != 0xbe
        && prim != 0xbf
        && prim != 0x3ed
        && automaticdoor_baseFUNC_DoorLocked(e, other) == 0
        && (var(e) != V::L02 || ExceptionLevelL02_R_D_DEPT(e, p) == 0)
        && {
            e.w32(l + 0x4c, 1);
            flags(e) & 4 == 0
        };
    if ok {
        let f = flags(e) | 4;
        set_flags(e, f);
        APIgameobj::SetContextFlag(e, SELF, 2);
        if var(e) != V::Old {
            debug(e, "CB_COLLIDE_OBJ -> valid");
        }
    }
    Ok(0)
}

/// automaticdoor_baseCB_NOTIFY_COLLIDE_OBJ_OUT @0x00011b80
pub fn automaticdoor_baseCB_NOTIFY_COLLIDE_OBJ_OUT(e: &mut Engine, _: u32) -> LResult<u64> {
    let l = L(e);
    if e.r32(l + 0x4c) == 0 {
        if var(e) != V::Old {
            debug(e, "CB_COLLIDE_OBJ_OUT");
        }
        let f = e.r32(l + 4) & 0xffff_fffb;
        e.w32(l + 4, f);
        if e.r32(l + 0x38) != 0 {
            e.w32(l + 0x38, 0);
            automaticdoor_baseFUNC_SetInPrim(e);
            goto(e, l, automaticdoor_baseState_Close_In);
        }
        APIgameobj::SetContextFlag(e, SELF, 0);
        e.w32(l + 0x50, 0x3f80_0000);
    }
    e.w32(l + 0x4c, 0);
    Ok(0)
}

/// automaticdoor_baseCB_ANIM_LF @0x00011c98 (param: anim that finished)
pub fn automaticdoor_baseCB_ANIM_LF(e: &mut Engine, p: u32) -> LResult<u64> {
    let (close, _, open) = anims(e);
    let a = p as i32;
    if a == open {
        let f = flags(e) | 0x100;
        set_flags(e, f);
    } else if a == close {
        let f = flags(e) | 0x200;
        set_flags(e, f);
    }
    Ok(1)
}

/// automaticdoor_baseCB_TIMER @0x00011d38
pub fn automaticdoor_baseCB_TIMER(e: &mut Engine, _: u32) -> LResult<u64> {
    let l = L(e);
    let t = e.r32(l + 0x20);
    APIextra::TimerUnregisterNotify(e, SELF, t);
    e.w32(l + 0x38, 0);
    automaticdoor_baseFUNC_SetInPrim(e);
    Ok(1)
}

/// automaticdoor_baseCB_CUTSCENE @0x00011db8 (param: {cutscene, 1 start / 0 end})
pub fn automaticdoor_baseCB_CUTSCENE(e: &mut Engine, p: u32) -> LResult<u64> {
    let cs = e.r32(p);
    if APIgameobj::IsInCutScene(e, SELF, cs) != 0 {
        let l = L(e);
        let phase = e.r32(p + 4);
        if phase == 1 {
            // quirk: local+0x54 is never set by this module (0 unless the engine filled it)
            let c = e.r32(l + 0x54);
            APIgameobj::GetClippingParams(e, SELF, c);
            let id = APIgameobj::GetID(e, SELF, NULL);
            LIBMiscFunc::MiscFunc_InitForCutscene(e, id);
            APIgameobj::UnregisterNotify(e, SELF, 0xffff_ffff, 3);
            APIgameobj::UnregisterNotify(e, SELF, 0xffff_ffff, 6);
        } else if phase == 0 {
            let id = APIgameobj::GetID(e, SELF, NULL);
            let c = e.r32(l + 0x54);
            LIBMiscFunc::MiscFunc_EndCutscene(e, id, c);
            APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 3, 2);
            automaticdoor_baseFUNC_MessageHandler(e, 0x1e4, 0, NULL)?;
        }
    }
    Ok(0)
}

/// automaticdoor_baseFUNC_MessageHandler @0x00011f88
pub fn automaticdoor_baseFUNC_MessageHandler(e: &mut Engine, m: u32, p2: u32, p3: Ptr) -> LResult<u64> {
    let l = L(e);
    match m {
        0x1a0 => {
            let id = APIgameobj::GetID(e, SELF, NULL);
            APIgameobj::UnregisterNotify(e, SELF, 0xffff_ffff, 6);
            APIgameobj::UnregisterNotify(e, SELF, 0xffff_ffff, 3);
            e.w32(l + 0x38, 1);
            e.w32(l + 0x34, 0);
            e.w32(l + 0x1c, p2);
            APIgameobj::SendMessageTo(e, p2, NULL, 0x19d, id, l + 0x58);
            set_state(e, l, automaticdoor_baseState_AIWait_SecondDoor);
        }
        0xd4 => {
            let m9 = e.r32(l + 0x24);
            let b1 = gbuf(e, 1);
            if m9 != 0 && e.r32(b1 + 8) & m9 != 0 {
                e.w32(l + 0x48, 1);
            }
        }
        0xc9 => {
            let f = e.r32(l + 4) & 0xffff_f7ff;
            e.w32(l + 4, f);
            if e.r32(l + 0x48) == 0 {
                automaticdoor_baseFUNC_Enable(e);
            }
        }
        0xe7 => {
            let f = e.r32(l + 4) | 0x80;
            e.w32(l + 4, f);
        }
        0x19c => {
            let f = e.r32(l + 4);
            let f = if p2 == 0 { f & 0xffff_f7ff } else { f | 0x800 };
            e.w32(l + 4, f);
        }
        0x1a4 => {
            e.w32(l + 0x38, 0);
            automaticdoor_baseFUNC_SetInPrim(e);
        }
        0x1a3 => {
            APIgameobj::UnregisterNotify(e, SELF, 0xffff_ffff, 6);
            APIgameobj::UnregisterNotify(e, SELF, 0xffff_ffff, 3);
            e.w32(l + 0x38, 1);
            e.w32(l + 0x34, p2);
            let v = e.r32(p3);
            e.w32(l + 0x1c, v);
            let f = e.r32(l + 4) | 0x20;
            e.w32(l + 4, f);
        }
        0x1a5 => {
            let f = e.r32(l + 4) | 0x400;
            e.w32(l + 4, f);
        }
        0x1e4 => {
            let (close, idle, _) = anims(e);
            e.w32(l + 4, 0);
            let f = e.r32(l + 4) | 2;
            e.w32(l + 4, f);
            e.w32(l + 0xc, close as u32);
            LIBAnimMngr::PlayAnim(e, 0.0, idle, 1, 0);
            e.w32(l + 0x38, 0);
            automaticdoor_baseFUNC_SetInPrim(e);
            APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 3, 2);
            e.w32(l + 0x4c, 0);
            goto(e, l, automaticdoor_baseState_Close_In);
        }
        _ => {}
    }
    Ok(0)
}

/// automaticdoor_baseFUNC_Create_Physic @0x00012428
fn automaticdoor_baseFUNC_Create_Physic(e: &mut Engine, l: Ptr) {
    APIphysic::CreatePhysic(e, SELF, 0);
    APIphysic::SetFlags(e, SELF, 2);
    APIphysic::SetWeight(e, SELF, 1.0);
    APIphysic::SetPriority(e, SELF, 100);
    APIphysic::PrimSetIntAttribBits(e, SELF, 0x3ac, 0, 4);
    APIphysic::PrimSetIntAttribBits(e, SELF, 0x3ad, 1, 0x8000_0000);
    let h = APIphysic::DMAdd(e, SELF, 0x25);
    e.w32(l + 8, h);
    let p = e.alloca(0x30);
    APIphysic::DMGetParams(e, h, p);
    e.w32(p + 4, 3);
    APIphysic::DMSetParams(e, h, p);
    APIphysic::DMSetContextMask(e, h, 2);
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 3, 2);
}

/// automaticdoor_baseFUNC_SetInPrim @0x000125c8 (empty)
fn automaticdoor_baseFUNC_SetInPrim(_e: &mut Engine) {}
/// automaticdoor_baseFUNC_SetOutPrim @0x000125e8 (empty)
fn automaticdoor_baseFUNC_SetOutPrim(_e: &mut Engine) {}

/// automaticdoor_baseFUNC_ReserveDoor @0x00012608
fn automaticdoor_baseFUNC_ReserveDoor(e: &mut Engine) {
    APIgameobj::UnregisterNotify(e, SELF, 0xffff_ffff, 6);
    APIgameobj::UnregisterNotify(e, SELF, 0xffff_ffff, 3);
    let l = L(e);
    if e.r32(l + 0x34) == 0 {
        APIgameobj::GetID(e, SELF, NULL);
        let partner = e.r32(l + 0x10);
        APIgameobj::SendMessageTo(e, partner, NULL, 0x1a3, 1, l + 0x1c);
    }
}

/// automaticdoor_baseFUNC_DoorLocked @0x000126e8 (L02 @0x00012a68 has the extended unlock test)
fn automaticdoor_baseFUNC_DoorLocked(e: &mut Engine, p: u32) -> u32 {
    let l = L(e);
    let mask = e.r32(l + 0x14);
    let b1 = gbuf(e, 1);
    let keys = e.r32(b1 + 0xc);
    let (h8, hc) = (hero(e, 8), hero(e, 0xc));
    let unlocked = if var(e) == V::L02 {
        let lvl = e.r32(b1 + 8) & 0x20000;
        let c = lvl == 0 || p == h8 || p == hc;
        let d = e.r32(l + 0x40) != 0;
        let ee = lvl != 0 || (p != h8 && p != hc);
        mask == 0 || (keys & mask != 0 && (c || d)) || (c && ee)
    } else {
        mask == 0 || keys & mask != 0
    };
    if unlocked {
        if mask != 0 && (p == h8 || p == hc) && keys & mask != 0 && e.r32(l + 0x40) == 0 {
            e.w32(l + 0x40, 1);
            APInewSound::Play(e, SELF, 0x4000_0000, 0);
        }
        0
    } else {
        let shown = LIBApiText::IsTextDisplayed(e) != 0;
        if !shown && e.r32(l + 0x18) != 0 {
            let s = e.alloca(0x40);
            LIBApiText::InitStruct(e, s);
            e.w32(s, 0x4060_0000);
            let t = e.r32(l + 0x18);
            LIBApiText::ShowText(e, t, s);
        }
        APInewSound::Play(e, SELF, 0x4000_0001, 0);
        1
    }
}

/// automaticdoor_baseFUNC_Enable @0x000128d8 (L02: + radar message for door 0x2a000145)
fn automaticdoor_baseFUNC_Enable(e: &mut Engine) {
    APIgameobj::SetGroupID(e, SELF, 0);
    let l = L(e);
    e.w32(l + 0x3c, 0);
    e.w32(l + 0x40, 0);
    if var(e) == V::L02
        && APIgameobj::GetID(e, 0x2a0000c5, NULL) == 0x7fff_fffe
        && APIgameobj::GetID(e, SELF, NULL) == 0x2a000145
    {
        APIgameobj::SendMessageTo(e, 0x2a100006, NULL, 10, 0xd, NULL);
    }
}

/// automaticdoor_baseFUNC_Disable @0x00012938
fn automaticdoor_baseFUNC_Disable(e: &mut Engine) {
    APIgameobj::SetGroupID(e, SELF, 6);
    let l = L(e);
    e.w32(l + 0x3c, 1);
    e.w32(l + 0x40, 1);
}

/// automaticdoor_baseConstructor @0x000129a0
fn automaticdoor_baseConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    if instanceCount == 1 {
        let fns: [(i32, ExtFn); 7] = [
            (0, automaticdoor_baseST_FIRSTMOVE),
            (1, automaticdoor_baseST_ENTRY_POINT),
            (2, automaticdoor_baseCB_NOTIFY_COLLIDE_OBJ),
            (3, automaticdoor_baseCB_NOTIFY_COLLIDE_OBJ_OUT),
            (4, automaticdoor_baseCB_ANIM_LF),
            (5, automaticdoor_baseCB_TIMER),
            (6, automaticdoor_baseCB_CUTSCENE),
        ];
        for (i, f) in fns {
            APIcons::SetExternalFunction(e, i, f);
        }
        for (a, v) in [
            (pAnim, APIcons::GetApiAnim(e)),
            (pGameObj, APIcons::GetApiGameObj(e)),
            (pExtra, APIcons::GetApiExtra(e)),
            (pPhysic, APIcons::GetApiPhysic(e)),
            (pGame, APIcons::GetApiGame(e)),
            (pSound, APIcons::GetApiNewSound(e)),
            (pMath, APIcons::GetApiMath(e)),
        ] {
            let a = G(e, a);
            e.w32(a, v);
        }
        for (a, n) in [(pDebug, "debug"), (pAnimMngr, "AnimMngr"), (pMiscFunc, "MiscFunc"), (pText, "ApiText")] {
            let s = e.cstr(n);
            let v = APIcons::GetApiCustom(e, s);
            let a = G(e, a);
            e.w32(a, v);
        }
        let slot = G(e, pGlobalDataautomaticdoor_base);
        APIcons::GetGlobalData(e, slot, 8);
        let b0 = APIgameobj::GetGameBuffer(e, 0, 0x8f8, 1);
        let gd = e.r32(slot);
        e.w32(gd, b0);
        let b2 = APIgameobj::GetGameBuffer(e, 2, 0x158, 1);
        let gd = e.r32(slot);
        e.w32(gd + 4, b2);
    }
    APIcons::SetMessageFunction(e, automaticdoor_baseFUNC_MessageHandler);
    let slot = G(e, pLocalDataautomaticdoor_base);
    APIcons::GetLocalData(e, slot, 0x88);
    LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 2, NULL, 1);
    let l = L(e);
    automaticdoor_baseFUNC_Create_Physic(e, l);
    for o in [0x14, 0x10, 0x18, 0x40, 0x44, 0x24] {
        e.w32(l + o, 0);
    }
    let id = APIgameobj::GetID(e, SELF, NULL);
    let data = e.rt.cur_data();
    let n = DOORS[b(e)].len() as u32;
    for i in 0..n {
        let row = data + i * 0x14;
        // no break: the last matching row wins
        if id == e.r32(row) {
            for (lo, ro) in [(0x10, 4), (0x14, 8), (0x18, 0xc), (0x44, 0x10)] {
                let v = e.r32(row + ro);
                e.w32(l + lo, v);
            }
        }
    }
    APIgameobj::GetPosition(e, SELF, 5, l + 0x58);
    APIgameobj::GetPosition(e, SELF, 6, l + 100);
    let partner = e.r32(l + 0x10);
    if partner != 0 {
        APIgameobj::GetPosition(e, partner, 5, l + 0x70);
        let partner = e.r32(l + 0x10);
        APIgameobj::GetPosition(e, partner, 6, l + 0x7c);
    }
    APIgameobj::SetGroupBitMask(e, SELF, 0x0400_0001);
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 0x16, 6);
    let t = APIextra::TimerCreate(e);
    e.w32(l + 0x20, t);
    APIextra::TimerSetFrequency(e, t, 1.5);
    LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 2, NULL, 0);
    APIcons::SetInitialState(e, 0);
    Ok(())
}

/// automaticdoor_baseDestructor @0x00012fb0
fn automaticdoor_baseDestructor(e: &mut Engine) {
    let l = L(e);
    let t = e.r32(l + 0x20);
    APIextra::TimerUnregisterNotify(e, SELF, t);
}

/// automaticdoor_LocalCB_NOTIFY_COLLIDE_OBJ @0x00013010 (L01 only, ext 2)
pub fn automaticdoor_LocalCB_NOTIFY_COLLIDE_OBJ(e: &mut Engine, p: u32) -> LResult<u64> {
    let l = L(e);
    let b1 = gbuf(e, 1);
    let v = e.r32(b1 + 0xc) | e.r32(l + 0x14);
    e.w32(b1 + 0xc, v);
    if e.r32(p + 8) == hero(e, 8) {
        automaticdoorFUNC_GuardInCafeteria(e);
    }
    automaticdoor_baseCB_NOTIFY_COLLIDE_OBJ(e, p)
}

/// automaticdoorFUNC_GuardInCafeteria @0x000131f8 (L01, empty)
fn automaticdoorFUNC_GuardInCafeteria(_e: &mut Engine) {}

/// automaticdoorFUNC_MessageHandler @0x000130d0 (L01 address)
pub fn automaticdoorFUNC_MessageHandler(e: &mut Engine, m: u32, p2: u32, p3: Ptr) -> LResult<u64> {
    automaticdoor_baseFUNC_MessageHandler(e, m, p2, p3)?;
    Ok(0)
}

/// automaticdoorConstructor @0x00013218 (L01 address)
pub fn automaticdoorConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    automaticdoor_baseConstructor(e, api, instanceCount)?;
    match var(e) {
        V::Old => {
            if instanceCount == 1 {
                APIcons::SetExternalFunction(e, 2, automaticdoor_LocalCB_NOTIFY_COLLIDE_OBJ);
            }
            APIcons::SetMessageFunction(e, automaticdoorFUNC_MessageHandler);
        }
        V::L02 => {
            APIcons::SetMessageFunction(e, automaticdoorFUNC_MessageHandler);
            APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 0x16, 6);
            if APIgameobj::GetID(e, SELF, NULL) == 0x2a000145 {
                let l = L(e);
                e.w32(l + 0x24, 0x0800_0000);
            }
        }
        V::New => {
            APIcons::SetMessageFunction(e, automaticdoorFUNC_MessageHandler);
        }
    }
    APIcons::SetInitialState(e, 0);
    Ok(())
}

/// automaticdoorDestructor @0x000132c0 (L01 address)
pub fn automaticdoorDestructor(e: &mut Engine, _: i32) -> LResult<()> {
    automaticdoor_baseDestructor(e);
    Ok(())
}

//! Port of wisp_base.c (Source/Globals/Collectables/Wisp/wisp_base.c) + initwisp.c: the wisp object.
//! Reference build l01_wisps; per-build literals through `k(e, NAME)` (see builds.rs).
//!
//! Local data (`pLocalDataWisp_base`, 0xc0 bytes): +0x00 state function; +0x04 flags (1 touched/picked,
//! 2 released by the book, 0x10 hooked by the book, 0x100000 respawn, 0x200000 already collected,
//! 0x1000000 / 0x2000000 cut scene start/end, 0x10000000 active); +0x08 map word 1; +0x0c "collected"
//! bit in game buffer 2 +0x14; +0x20 size (1 small, 0/2 medium, 3 large); +0x24..+0x58 launch
//! parameters (message 0x126/0xf7 payload: +0x24 start position, +0x30/+0x3c impulse min/max, +0x4c
//! ground height); +0x58 drive-module 4 parameters (+0x60 speed); +0x68 home position; +0x78/+0x7c
//! halo/spirit emitters; +0x80/+0x84/+0x88/+0x8c drive modules 1 (follow target), 4, 7, 0 (gravity);
//! +0x90 motion trail (-2 none); +0x94 own id; +0x98 target id; +0x9c speed; +0xa0 time from which it
//! can be picked; +0xa4 pick time; +0xa8 carried by an object; +0xac launched (spawned by a message);
//! +0xb0 given (message 0x124); +0xb4 book may suck it; +0xb8 sucked by the book; +0xbc instance count.
//!
//! Global data (`pGlobalDataWisp_base`, shared by all wisps of the module): +0 game buffer 0, +4 game
//! buffer 2 (+0 bit 0 "wisps active", +0x14 collected bits), +8 game buffer 1 (+4 book power), +0xc
//! wisp count, +0x10 launched-wisp count, +0x14..+0x28 the six particle types, +0x2c the TmpWispMap
//! copy (N records of 0x24), then per build: the launched-wisp ids, the GENERIC_FX object id and the
//! GENERIC_FX descriptor (two entries of 0xac bytes, each with a particle-settings function pointer).
use super::builds::*;
use super::*;
use crate::engine::*;
use crate::logic::{CodeFn, LResult};

// the state functions stored at local data +0 (module-relative addresses in l01_wisps)
const ADDR_Inactive_In: u32 = 0x000137c0;
const ADDR_Inactive_Execute: u32 = 0x00013888;
const ADDR_Active_In: u32 = 0x000138d0;
const ADDR_Active_Execute: u32 = 0x000139b8;
const ADDR_Launched_In: u32 = 0x00013a58;
const ADDR_Launched_Execute: u32 = 0x00013c98;
const ADDR_Hooked_In: u32 = 0x00013db0;
const ADDR_Hooked_Execute: u32 = 0x00013e40;
const ADDR_Released_In: u32 = 0x00013fb0;
const ADDR_Released_Exectute: u32 = 0x00014000;
const ADDR_Return_In: u32 = 0x00014108;
const ADDR_Return_Execute: u32 = 0x000141d0;
const ADDR_Picked_In: u32 = 0x00014658;
const ADDR_Picked_Execute: u32 = 0x00014720;
const ADDR_Dead_In: u32 = 0x000149b0;
const ADDR_Dead_Execute: u32 = 0x00014b40;
const ADDR_Removed_In: u32 = 0x00014c90;
const ADDR_Removed_Execute: u32 = 0x00014ce0;
const ADDR_CutScene_In: u32 = 0x00014d60;
const ADDR_CutScene_Execute: u32 = 0x00014de8;

/// `(*(code *)*param_1)(param_1)`. The local data starts as Inactive_In (constructor), so the pointer
/// is never NULL in the shipped flow; anything else panics (the original would jump to garbage).
fn call_state(e: &mut Engine, p: Ptr) {
    match e.r32(p) {
        ADDR_Inactive_In => Wisp_baseState_Inactive_In(e, p),
        ADDR_Inactive_Execute => Wisp_baseState_Inactive_Execute(e, p),
        ADDR_Active_In => Wisp_baseState_Active_In(e, p),
        ADDR_Active_Execute => Wisp_baseState_Active_Execute(e, p),
        ADDR_Launched_In => Wisp_baseState_Launched_In(e, p),
        ADDR_Launched_Execute => Wisp_baseState_Launched_Execute(e, p),
        ADDR_Hooked_In => Wisp_baseState_Hooked_In(e, p),
        ADDR_Hooked_Execute => Wisp_baseState_Hooked_Execute(e, p),
        ADDR_Released_In => Wisp_baseState_Released_In(e, p),
        ADDR_Released_Exectute => Wisp_baseState_Released_Exectute(e, p),
        ADDR_Return_In => Wisp_baseState_Return_In(e, p),
        ADDR_Return_Execute => Wisp_baseState_Return_Execute(e, p),
        ADDR_Picked_In => Wisp_baseState_Picked_In(e, p),
        ADDR_Picked_Execute => Wisp_baseState_Picked_Execute(e, p),
        ADDR_Dead_In => Wisp_baseState_Dead_In(e, p),
        ADDR_Dead_Execute => Wisp_baseState_Dead_Execute(e, p),
        ADDR_Removed_In => Wisp_baseState_Removed_In(e, p),
        ADDR_Removed_Execute => Wisp_baseState_Removed_Execute(e, p),
        ADDR_CutScene_In => Wisp_baseState_CutScene_In(e, p),
        ADDR_CutScene_Execute => Wisp_baseState_CutScene_Execute(e, p),
        a => panic!("wisps: call through bad state pointer {a:#x}"),
    }
}

const ALL: u32 = 0x7fff_ffff;
/// timer 0x90000000: the game clock the wisps compare against
const GAME_TIMER: u32 = 0x9000_0000;

// ------------------------------------------------------------------------------------------------
// initwisp.c

/// 0x00017758 copy the level's wisp table (TmpWispMap) into the global data; remember where the
/// collected bits live (game buffer 2 +0x14). Builds without wisps (N = 0) have no loop.
pub fn InitWispData(e: &mut Engine) {
    let gd = GD(e);
    if g(e, G_InitWispData) == 0 {
        let map = G(e, TmpWispMap);
        for iStack_20 in 0..k(e, InitWispData_0_1) {
            let (d, s) = (gd + iStack_20 * 0x24, map + iStack_20 * 0x24);
            let v = e.r32(s + 0x18);
            e.w32(d + 0x44, v);
            let v = e.r32(s);
            e.w32(d + 0x2c, v);
            let v = e.r32(s + 4);
            e.w32(d + 0x30, v);
            let v = e.r32(s + 8);
            e.w32(d + 0x34, v);
            let v = e.r32(s + 0xc);
            e.w32(d + 0x38, v);
            let v = e.r32(s + 0x1c);
            e.w32(d + 0x48, v);
            let v = e.r32(s + 0x10);
            e.w32(d + 0x3c, v);
            let v = e.r32(s + 0x14);
            e.w32(d + 0x40, v);
        }
    }
    let v = e.r32(gd + 4) + 0x14;
    let a = uiToStatusPointerMapWisp(e);
    e.w32(a, v);
}

// ------------------------------------------------------------------------------------------------
// states

/// 0x000134b8 state 0: set up the wisp, then run the entry point in the same frame
pub fn Wisp_baseST_FIRSTMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    let uStack_50 = e.alloca(8);
    let puVar1 = L(e);
    let a = k(e, Wisp_baseST_FIRSTMOVE_0_1) as i32;
    LIBAnimMngr::PlayAnim(e, 0.0, a, 2, 0);
    APIgameobj::SetNextState(e, SELF, 1);
    if e.r32(puVar1 + 0xa8) == 0 && e.r32(puVar1 + 0xac) == 0 {
        Wisp_BaseFUNC_Init_LocalSetting(e, puVar1);
    }
    Wisp_baseFUNC_Create_GameObject(e);
    Wisp_baseFUNC_Create_Physic(e, puVar1);
    Wisp_baseFUNC_Create_Particles(e, puVar1)?;
    Wisp_baseFUNC_Create_MotionTrail(e, puVar1);
    Wisp_baseFUNC_ClippingSet(e, 2);
    Wisp_baseFUNC_AllFXControl(e, 0);
    if e.r32(puVar1 + 0xa8) == 0 {
        APIgameobj::GetPosition(e, SELF, ALL, puVar1 + 0x68);
    } else {
        // retriever 0xff000020 with this object: the object carrying the wisp
        e.w32(uStack_50, 1);
        let id = APIgameobj::GetID(e, SELF, NULL);
        e.w32(uStack_50 + 4, id);
        let uVar2 = APIgameobj::GetID(e, 0xff00_0020, uStack_50);
        e.w32(puVar1 + 0x98, uVar2);
    }
    if e.r32(puVar1 + 0xa8) == 0 && e.r32(puVar1 + 0xac) == 0 {
        let gb2 = e.r32(GD(e) + 4);
        if e.r32(gb2) & 1 != 0 {
            let f = e.r32(puVar1 + 4);
            e.w32(puVar1 + 4, f | 0x1000_0000);
        }
        Wisp_baseFUNC_AmIDead(e, puVar1);
    }
    if e.r32(puVar1 + 0xac) != 0 {
        Wisp_baseFUNC_ClippingSet(e, 1);
        let f = e.r32(puVar1 + 4);
        e.w32(puVar1 + 4, f | 0x1000_0000);
        e.w32(puVar1, ADDR_Launched_In);
    }
    Wisp_baseST_ENTRY_POINT(e, 0)?;
    Ok(0)
}

/// 0x00013750 state 1: cut-scene / deactivation events, then the current state function
pub fn Wisp_baseST_ENTRY_POINT(e: &mut Engine, _: u32) -> LResult<u64> {
    Wisp_baseFUNC_CheckEvents(e, 0x300_0000);
    let l = L(e);
    call_state(e, l);
    Ok(0)
}

/// 0x000137c0 hidden, no collision notify
pub fn Wisp_baseState_Inactive_In(e: &mut Engine, param_1: Ptr) {
    APIgameobj::UnregisterNotify(e, SELF, 0xffff_ffff, 3);
    APIgameobj::SetGroupBitMask(e, SELF, 1);
    APIgameobj::SetGameObjectVisible(e, SELF, 0);
    Wisp_baseFUNC_AllFXControl(e, 0);
    e.w32(param_1, ADDR_Inactive_Execute);
    call_state(e, param_1);
}

/// 0x00013888 wait for activation (the original takes no argument)
pub fn Wisp_baseState_Inactive_Execute(e: &mut Engine, _param_1: Ptr) {
    Wisp_baseFUNC_CheckEvents(e, 0x1000_0000);
}

/// 0x000138d0 visible, glowing, collision notify 3 -> callback 2
pub fn Wisp_baseState_Active_In(e: &mut Engine, param_1: Ptr) {
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 3, 2);
    APIgameobj::SetGroupBitMask(e, SELF, 0x801);
    APIgameobj::SetGameObjectVisible(e, SELF, 1);
    Wisp_baseFUNC_AllFXControl(e, 1);
    APIgameobj::SetContextFlag(e, SELF, 0);
    e.w32(param_1, ADDR_Active_Execute);
    call_state(e, param_1);
}

/// 0x000139b8 bob in place until picked / hooked / deactivated
pub fn Wisp_baseState_Active_Execute(e: &mut Engine, param_1: Ptr) {
    wdbg(e, param_1, "active");
    if e.r32(param_1 + 4) & 0x1000_0000 == 0 {
        e.w32(param_1, ADDR_Inactive_In);
    } else if Wisp_baseFUNC_CheckEvents(e, 0x13) == 0 {
        Wisp_baseFUNC_RunBob(e);
    }
}

/// 0x00013a58 spawned wisp: appear at the launch position, random spin, random impulse
pub fn Wisp_baseState_Launched_In(e: &mut Engine, param_1: Ptr) {
    let DStack_60 = e.alloca(16);
    let DStack_50 = e.alloca(16);
    Wisp_baseFUNC_AllFXControl(e, 1);
    APIgameobj::SetGameObjectVisible(e, SELF, 1);
    APIgameobj::SetGroupID(e, SELF, 0);
    APIgameobj::SetContextFlag(e, SELF, 0xc);
    APIgameobj::SetTranslation(e, SELF, param_1 + 0x24);
    e.wv3(DStack_50, [0.0, 1.0, 0.0]);
    let iVar2 = LIBApiCFunctions::IntRandom(e, 1, 0x168);
    APIgameobj::Rotate(e, SELF, DStack_50, iVar2 as f32 * 0.017453292);
    for (c, lo, hi) in [(0u32, 0x30u32, 0x3cu32), (4, 0x34, 0x40), (8, 0x38, 0x44)] {
        let (a, b) = (e.rf(param_1 + lo), e.rf(param_1 + hi));
        let v = LIBApiCFunctions::FloatRandom(e, a, b, 0.1);
        e.wf(DStack_60 + c, v);
    }
    APIgameobj::TransformVector(e, SELF, DStack_60, DStack_60);
    APIphysic::Impulse(e, SELF, DStack_60);
    e.w32(param_1, ADDR_Launched_Execute);
    call_state(e, param_1);
}

/// 0x00013c98 fall until the launch's ground height, then become a normal active wisp there
pub fn Wisp_baseState_Launched_Execute(e: &mut Engine, param_1: Ptr) {
    APIgameobj::GetPosition(e, SELF, ALL, param_1 + 0x68);
    wdbg(e, param_1, "launched");
    if e.rf(param_1 + 0x6c) < e.rf(param_1 + 0x4c) {
        APIphysic::SetVelocity(e, SELF, 0.0, 0.0, 0.0);
        let v = e.r32(param_1 + 0x4c);
        e.w32(param_1 + 0x6c, v);
        APIgameobj::SetTranslation(e, SELF, param_1 + 0x68);
        APIgameobj::SetGroupID(e, SELF, 0x1a);
        Wisp_baseFUNC_ClippingSet(e, 2);
        e.w32(param_1, ADDR_Active_In);
    }
}

/// 0x00013db0 sucked by the book: follow the book (drive module 1 target)
pub fn Wisp_baseState_Hooked_In(e: &mut Engine, param_1: Ptr) {
    let book = k(e, ID_2a000025);
    Wisp_baseFUNC_SetTargetID(e, param_1, book);
    APIgameobj::SetContextFlag(e, SELF, 7);
    e.w32(param_1, ADDR_Hooked_Execute);
    call_state(e, param_1);
}

/// 0x00013e40 picked when within 1 unit of the book (if pickable and the power is not full)
pub fn Wisp_baseState_Hooked_Execute(e: &mut Engine, param_1: Ptr) {
    wdbg(e, param_1, "hooked");
    let DStack_50 = e.alloca(16);
    let DStack_40 = e.alloca(16);
    let fVar1 = APIextra::TimerGetTime(e, GAME_TIMER);
    if e.rf(param_1 + 0xa0) <= fVar1 && e.rf(power(e)) < 180.0 {
        APIgameobj::GetPosition(e, SELF, ALL, DStack_40);
        let book = k(e, ID_2a000025);
        APIgameobj::GetPosition(e, book, ALL, DStack_50);
        let a = e.rv3(DStack_40);
        let b = e.rv3(DStack_50);
        let d = [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
        e.wv3(DStack_40, d);
        if d[0] * d[0] + d[1] * d[1] + d[2] * d[2] < 1.0 {
            let f = e.r32(param_1 + 4);
            e.w32(param_1 + 4, f | 1);
        }
    }
    Wisp_baseFUNC_CheckEvents(e, 3);
}

/// 0x00013fb0 let go by the book
pub fn Wisp_baseState_Released_In(e: &mut Engine, param_1: Ptr) {
    e.w32(param_1, ADDR_Released_Exectute);
    call_state(e, param_1);
}

/// 0x00014000 slow down (speed -0.4 per frame); at 1.0 stop and return home
pub fn Wisp_baseState_Released_Exectute(e: &mut Engine, param_1: Ptr) {
    wdbg(e, param_1, "released");
    if Wisp_baseFUNC_CheckEvents(e, 0x11) == 0 {
        let v = e.rf(param_1 + 0x9c) + -0.4;
        e.wf(param_1 + 0x9c, v);
        if e.rf(param_1 + 0x9c) <= 1.0 {
            e.w32(param_1 + 0x9c, 0x3f80_0000);
            APIphysic::SetVelocity(e, SELF, 0.0, 0.0, 0.0);
            e.w32(param_1 + 0xb8, 0);
            e.w32(param_1, ADDR_Return_In);
        }
        let dm = e.r32(param_1 + 0x84);
        APIphysic::DMSetParams(e, dm, param_1 + 0x58);
    }
}

/// 0x00014108 go back home, or to the carrier
pub fn Wisp_baseState_Return_In(e: &mut Engine, param_1: Ptr) {
    if e.r32(param_1 + 0xa8) == 0 {
        APIgameobj::SetContextFlag(e, SELF, 0);
    } else {
        let t = e.r32(param_1 + 0x98);
        Wisp_baseFUNC_SetTargetID(e, param_1, t);
        APIgameobj::SetContextFlag(e, SELF, 7);
    }
    e.w32(param_1, ADDR_Return_Execute);
    call_state(e, param_1);
}

/// 0x000141d0 home: fly back (speed 5, easing x0.8 inside 5 units), active again within 0.2.
/// Carried: given back to the carrier (message 0x120) within 0.9 units.
pub fn Wisp_baseState_Return_Execute(e: &mut Engine, param_1: Ptr) {
    wdbg(e, param_1, "return");
    let DStack_90 = e.alloca(16);
    let DStack_80 = e.alloca(16);
    let DStack_70 = e.alloca(16);
    let DStack_50 = e.alloca(16);
    if Wisp_baseFUNC_CheckEvents(e, 0x11) != 0 {
        return;
    }
    if e.r32(param_1 + 0xa8) == 0 {
        APIgameobj::GetPosition(e, SELF, ALL, DStack_70);
        let p = e.rv3(DStack_70);
        let fStack_60 = e.rf(param_1 + 0x68) - p[0];
        let fStack_5c = e.rf(param_1 + 0x6c) - p[1];
        let fStack_58 = e.rf(param_1 + 0x70) - p[2];
        let sq = fStack_60 * fStack_60 + fStack_5c * fStack_5c + fStack_58 * fStack_58;
        let fVar2 = APImath::sqrtf(e, sq);
        if fVar2 < 0.2 {
            e.w32(param_1, ADDR_Active_In);
            let mt = e.r32(param_1 + 0x90);
            APIgraphics::MTStopDraw(e, mt);
        } else {
            let fVar2 = APImath::sqrtf(e, sq);
            if fVar2 < 5.0 {
                let v = e.rf(param_1 + 0x9c) * 0.8;
                e.wf(param_1 + 0x9c, v);
                if e.rf(param_1 + 0x9c) < 1.0 {
                    e.w32(param_1 + 0x9c, 0x3f80_0000);
                }
            } else {
                e.w32(param_1 + 0x9c, 0x40a0_0000);
            }
            let mut fStack_40 = APImath::sqrtf(e, sq);
            fStack_40 = 1.0 / fStack_40;
            let s = e.rf(param_1 + 0x9c) / 50.0;
            e.wv3(DStack_50, [fStack_60 * fStack_40 * s, fStack_5c * fStack_40 * s, fStack_58 * fStack_40 * s]);
            APIgameobj::Translate(e, SELF, DStack_50);
        }
    } else {
        APIgameobj::GetPosition(e, SELF, ALL, DStack_90);
        let t = e.r32(param_1 + 0x98);
        APIgameobj::GetPosition(e, t, ALL, DStack_80);
        let a = e.rv3(DStack_90);
        let b = e.rv3(DStack_80);
        let d = [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
        e.wv3(DStack_70, d);
        let fVar2 = APImath::sqrtf(e, d[0] * d[0] + d[1] * d[1] + d[2] * d[2]);
        if fVar2 < 0.9 {
            let me = e.r32(param_1 + 0x94);
            APIgameobj::SendMessageTo(e, t, NULL, 0x120, me, NULL);
            let mt = e.r32(param_1 + 0x90);
            APIgraphics::MTStopDraw(e, mt);
            e.w32(param_1, ADDR_Dead_In);
        }
    }
}

/// 0x00014658 picked: sparkles now (touch) or 0.2 s later (sucked by the book)
pub fn Wisp_baseState_Picked_In(e: &mut Engine, param_1: Ptr) {
    let fVar1 = APIextra::TimerGetTime(e, GAME_TIMER);
    if e.r32(param_1 + 0xb8) == 0 {
        e.wf(param_1 + 0xa4, fVar1);
        Wisp_baseFUNC_LaunchSparkles(e, param_1);
    } else {
        e.wf(param_1 + 0xa4, fVar1 + 0.2);
    }
    e.w32(param_1, ADDR_Picked_Execute);
    call_state(e, param_1);
}

/// 0x00014720 refill the book: +18 small, +36 medium, +54 large, capped at 180; tell the carrier, the
/// book (0x122 with the size when sucked), play sound 0x17 on the book, message 0x1d to 0x2a100006
pub fn Wisp_baseState_Picked_Execute(e: &mut Engine, param_1: Ptr) {
    wdbg(e, param_1, "picked");
    let fVar2 = APIextra::TimerGetTime(e, GAME_TIMER);
    if fVar2 <= e.rf(param_1 + 0xa4) {
        return;
    }
    let pw = power(e);
    let add = match e.ri32(param_1 + 0x20) {
        1 => 18.0,
        0 | 2 => 36.0,
        3 => 54.0,
        _ => 18.0,
    };
    let v = e.rf(pw) + add;
    e.wf(pw, v);
    let pw = power(e);
    if 180.0 <= e.rf(pw) {
        e.w32(pw, 0x4334_0000);
    }
    if e.r32(param_1 + 0xa8) != 0 || e.r32(param_1 + 0xb0) != 0 {
        let (t, me) = (e.r32(param_1 + 0x98), e.r32(param_1 + 0x94));
        APIgameobj::SendMessageTo(e, t, NULL, 0x120, me, NULL);
    }
    let book = k(e, ID_2a000025);
    if e.r32(param_1 + 0xb8) != 0 {
        let size = e.r32(param_1 + 0x20);
        APIgameobj::SendMessageTo(e, book, NULL, 0x122, size, NULL);
    }
    APInewSound::Play(e, book, 0x17, 0);
    let hud = k(e, ID_2a100006);
    APIgameobj::SendMessageTo(e, hud, NULL, 0x1d, 0, NULL);
    Wisp_baseFUNC_SetBit(e, param_1);
    e.w32(param_1, ADDR_Dead_In);
}

/// 0x000149b0 hide; level wisps wait (Dead_Execute), spawned/carried ones are removed
pub fn Wisp_baseState_Dead_In(e: &mut Engine, param_1: Ptr) {
    APIgameobj::SetGroupBitMask(e, SELF, 1);
    APIgameobj::SetContextFlag(e, SELF, 0);
    Wisp_baseFUNC_ClippingSet(e, 3);
    Wisp_baseFUNC_AllFXControl(e, 0);
    APIgameobj::UnregisterNotify(e, SELF, 0xffff_ffff, 3);
    APIgameobj::SetGameObjectVisible(e, SELF, 0);
    APIphysic::SetVelocity(e, SELF, 0.0, 0.0, 0.0);
    if e.r32(param_1 + 0xa8) == 0 && e.r32(param_1 + 0xac) == 0 {
        APIgameobj::SetGroupID(e, SELF, 6);
        e.w32(param_1, ADDR_Dead_Execute);
    } else {
        e.w32(param_1, ADDR_Removed_In);
    }
    call_state(e, param_1);
}

/// 0x00014b40 respawn (flag 0x100000, message 0xc1) or remove (flag 0x200000, already collected)
pub fn Wisp_baseState_Dead_Execute(e: &mut Engine, param_1: Ptr) {
    wdbg(e, param_1, "dead");
    let f = e.r32(param_1 + 4);
    if f & 0x10_0000 == 0 {
        if f & 0x20_0000 != 0 {
            e.w32(param_1 + 4, f & 0xffdf_ffff);
            APIgameobj::SetGroupID(e, SELF, 0);
            e.w32(param_1, ADDR_Removed_In);
            call_state(e, param_1);
        }
    } else {
        e.w32(param_1 + 4, f & 0xffef_ffff);
        Wisp_baseFUNC_ResetBit(e, param_1);
        Wisp_baseFUNC_ClippingSet(e, 2);
        APIgameobj::SetGroupID(e, SELF, 0x1a);
        e.w32(param_1, ADDR_Active_In);
        call_state(e, param_1);
    }
}

/// 0x00014c90
pub fn Wisp_baseState_Removed_In(e: &mut Engine, param_1: Ptr) {
    e.w32(param_1, ADDR_Removed_Execute);
    call_state(e, param_1);
}

/// 0x00014ce0 kill the object unless it is the module's last wisp
pub fn Wisp_baseState_Removed_Execute(e: &mut Engine, param_1: Ptr) {
    if 1 < e.r32(GD(e) + 0xc) {
        let me = e.ri32(param_1 + 0x94);
        Lat_WispFUNC_RemoveWisp(e, param_1, me);
        APIgameobj::KillObject(e, SELF);
    }
}

/// 0x00014d60
pub fn Wisp_baseState_CutScene_In(e: &mut Engine, param_1: Ptr) {
    Wisp_baseFUNC_AllFXControl(e, 1);
    APIgameobj::SetGameObjectVisible(e, SELF, 1);
    e.w32(param_1, ADDR_CutScene_Execute);
    call_state(e, param_1);
}

/// 0x00014de8 (the original takes no argument)
pub fn Wisp_baseState_CutScene_Execute(e: &mut Engine, _param_1: Ptr) {}

// ------------------------------------------------------------------------------------------------
// functions

/// 0x00014e10 switch state on the first pending event allowed by `param_1` (in this priority order):
/// 0x1000000 cut scene, 0x2000000 inactive, 1 picked, 2 released, 0x10000000 active, 0x10 hooked.
/// Returns 1 when the state changed.
pub fn Wisp_baseFUNC_CheckEvents(e: &mut Engine, param_1: u32) -> u32 {
    let puVar1 = L(e);
    let f = e.r32(puVar1 + 4);
    let (clear, state) = if param_1 & 0x100_0000 != 0 && f & 0x100_0000 != 0 {
        (0xfeff_ffff, ADDR_CutScene_In)
    } else if param_1 & 0x200_0000 != 0 && f & 0x200_0000 != 0 {
        (0xfdff_ffff, ADDR_Inactive_In)
    } else if param_1 & 1 != 0 && f & 1 != 0 {
        (0xffff_fffe, ADDR_Picked_In)
    } else if param_1 & 2 != 0 && f & 2 != 0 {
        (0xffff_fffd, ADDR_Released_In)
    } else if param_1 & 0x1000_0000 != 0 && f & 0x1000_0000 != 0 {
        (0xffff_ffff, ADDR_Active_In)
    } else if param_1 & 0x10 != 0 && f & 0x10 != 0 {
        (0xffff_ffff, ADDR_Hooked_In)
    } else {
        return 0;
    };
    e.w32(puVar1 + 4, f & clear);
    e.w32(puVar1, state);
    1
}

/// 0x00015080 start flying to the book. Whatever `param_2` computes, the speed ends up 5.0 (the
/// original overwrites it unconditionally).
pub fn Wisp_baseFUNC_SetMovingSpeed(e: &mut Engine, param_1: Ptr, param_2: i32) {
    if param_2 == 1 {
        let v = e.rf(param_1 + 0x9c) + 0.4;
        e.wf(param_1 + 0x9c, v);
        if 5.0 < e.rf(param_1 + 0x9c) {
            e.w32(param_1 + 0x9c, 0x40a0_0000);
        }
    } else if param_2 == 2 {
        let v = e.rf(param_1 + 0x9c) + 1.2;
        e.wf(param_1 + 0x9c, v);
        if 6.0 < e.rf(param_1 + 0x9c) {
            e.w32(param_1 + 0x9c, 0x40c0_0000);
        }
    }
    e.w32(param_1 + 0x9c, 0x40a0_0000);
    let v = e.r32(param_1 + 0x9c);
    e.w32(param_1 + 0x60, v);
    let dm = e.r32(param_1 + 0x84);
    APIphysic::DMSetParams(e, dm, param_1 + 0x58);
    let mt = e.r32(param_1 + 0x90);
    APIgraphics::MTStart(e, mt);
}

/// 0x000151e0 collect sparkles: pick the spark/halo settings by size, place both FX entries at the
/// wisp and play the GENERIC_FX object
pub fn Wisp_baseFUNC_LaunchSparkles(e: &mut Engine, param_1: Ptr) {
    let iVar2 = GD(e);
    let (spark, halo, s_slot, h_slot): (crate::logic::ParticleSetFn, crate::logic::ParticleSetFn, usize, usize) =
        match e.ri32(param_1 + 0x20) {
            1 => (
                SetParticleWispTouchCollect_spark_small,
                SetParticleWispTouchCollect_halo_small,
                Wisp_baseFUNC_LaunchSparkles_0_5,
                Wisp_baseFUNC_LaunchSparkles_0_6,
            ),
            0 | 2 => (
                SetParticleWispTouchCollect_spark_medium,
                SetParticleWispTouchCollect_halo_medium,
                Wisp_baseFUNC_LaunchSparkles_0_1,
                Wisp_baseFUNC_LaunchSparkles_0_2,
            ),
            3 => (
                SetParticleWispTouchCollect_spark_large,
                SetParticleWispTouchCollect_halo_large,
                Wisp_baseFUNC_LaunchSparkles_0_3,
                Wisp_baseFUNC_LaunchSparkles_0_4,
            ),
            _ => (
                SetParticleWispTouchCollect_spark_small,
                SetParticleWispTouchCollect_halo_small,
                Wisp_baseFUNC_LaunchSparkles_0_5,
                Wisp_baseFUNC_LaunchSparkles_0_6,
            ),
        };
    let t = e.code_ptr(CodeFn::Particle(spark));
    let a = iVar2 + k(e, s_slot);
    e.w32(a, t);
    let t = e.code_ptr(CodeFn::Particle(halo));
    let a = iVar2 + k(e, h_slot);
    e.w32(a, t);
    let a = iVar2 + k(e, Wisp_baseFUNC_LaunchSparkles_0_7);
    APIgameobj::GetPosition(e, SELF, ALL, a);
    let a = iVar2 + k(e, Wisp_baseFUNC_LaunchSparkles_0_8);
    APIgameobj::GetPosition(e, SELF, ALL, a);
    for i in Wisp_baseFUNC_LaunchSparkles_0_9..=Wisp_baseFUNC_LaunchSparkles_0_20 {
        let a = iVar2 + k(e, i);
        e.w32(a, 0);
    }
    let fx = e.ri32(iVar2 + k(e, Wisp_baseFUNC_LaunchSparkles_0_21));
    LIBMiscFunc::MiscFunc_PlayGenericFX(e, fx);
}

/// 0x000153a8 callback 2 (collision notify 3, data +8 = the other object): touched by the current
/// hero or the book -> picked, if pickable and the power is not full. Returns 1 when picked.
pub fn Wisp_baseCB_NOTIFY_COLLIDE_OBJ(e: &mut Engine, param_1: u32) -> LResult<u64> {
    let iVar1 = L(e);
    let other = e.r32(param_1 + 8);
    let hero = e.r32(e.r32(GD(e)) + 8);
    if other == hero || other == k(e, ID_2a000025) {
        let fVar3 = APIextra::TimerGetTime(e, GAME_TIMER);
        if fVar3 < e.rf(iVar1 + 0xa0) {
            Ok(0)
        } else if e.rf(power(e)) < 180.0 {
            let f = e.r32(iVar1 + 4);
            e.w32(iVar1 + 4, f | 1);
            Ok(1)
        } else {
            Ok(0)
        }
    } else {
        Ok(0)
    }
}

/// 0x000154b8 messages:
/// - 0x11f: carried (+0xa8). 0xd4: removed (flag 0x200000). 0x98: activate if the wisps are on.
/// - 0xc1: respawn if the wisps are on. 0x1ef: activate. 0x165/0x166: the book may / may not suck it.
/// - 0x11e: the book's suction: sub 0 = released, 1..2 = start hooking (tells the other wisps of
///   group mask 0x800 with 0x121 and speeds up).
/// - 0x121 (from another wisp being sucked): not pickable for 1 s.
/// - 0x124: given by an object (target 0x2a000028 per level), sub 2/3 = size medium/large, else small.
/// - 0xf7 / 0x126: spawned: copy the launch parameters (if any), register in the launched list.
pub fn Wisp_baseFUNC_MyMessageFunction(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    let DStack_60 = e.alloca(16);
    let uStack_50 = e.alloca(8);
    let iVar2 = L(e);
    let gb2_on = |e: &Engine| e.r32(e.r32(GD(e) + 4)) & 1 != 0;
    match param_1 {
        0x11f => {
            e.w32(iVar2 + 0xa8, 1);
            return Ok(0);
        }
        0xd4 => {
            let f = e.r32(iVar2 + 4);
            e.w32(iVar2 + 4, f | 0x20_0000);
            return Ok(0);
        }
        0x98 => {
            if gb2_on(e) {
                Wisp_baseFUNC_MyMessageFunction(e, 0x1ef, 0, NULL)?;
            }
            return Ok(0);
        }
        0xc1 => {
            if gb2_on(e) {
                let f = e.r32(iVar2 + 4);
                e.w32(iVar2 + 4, f | 0x10_0000);
            }
            return Ok(0);
        }
        0x11e => {
            if param_2 == 0 {
                if e.r32(iVar2 + 0xb8) == 0 {
                    return Ok(0);
                }
                APIgameobj::GetPosition(e, SELF, ALL, DStack_60);
                let f = e.r32(iVar2 + 4) & 0xffff_ffef;
                e.w32(iVar2 + 4, f | 2);
                return Ok(0);
            }
            if 2 < param_2 || e.r32(iVar2 + 0xb8) != 0 {
                return Ok(0);
            }
            let fVar6 = APIextra::TimerGetTime(e, GAME_TIMER);
            if fVar6 <= e.rf(iVar2 + 0xa0) {
                return Ok(0);
            }
            // (the original tests +0xa8 twice)
            if e.r32(iVar2 + 0xa8) != 0 && (e.r32(iVar2 + 0xa8) == 0 || e.r32(iVar2 + 0xb4) == 0) {
                return Ok(0);
            }
            e.w32(iVar2 + 0xb8, 1);
            let f = e.r32(iVar2 + 4) & 0xffff_fffd;
            e.w32(iVar2 + 4, f | 0x10);
            e.w32(uStack_50, 1);
            e.w32(uStack_50 + 4, 0x800);
            let me = e.r32(iVar2 + 0x94);
            APIgameobj::SendMessageTo(e, 0xff00_0010, uStack_50, 0x121, me, NULL);
            if e.r32(iVar2 + 0xa8) != 0 {
                APIgameobj::LeaveParent(e, SELF, 0);
                Wisp_baseFUNC_SetMovingSpeed(e, iVar2, 2);
                return Ok(0);
            }
            Wisp_baseFUNC_SetMovingSpeed(e, iVar2, 1);
            return Ok(0);
        }
        0x166 => {
            if e.r32(iVar2 + 0xb8) != 0 {
                return Ok(0);
            }
            e.w32(iVar2 + 0xb4, 0);
            let f = e.r32(iVar2 + 4);
            e.w32(iVar2 + 4, f & 0xefff_ffff);
            return Ok(0);
        }
        0x165 => {
            e.w32(iVar2 + 0xb4, 1);
            Wisp_baseFUNC_MyMessageFunction(e, 0x1ef, 0, NULL)?;
            return Ok(0);
        }
        0x1ef => {
            let f = e.r32(iVar2 + 4);
            e.w32(iVar2 + 4, f | 0x1000_0000);
            return Ok(0);
        }
        0x124 => {
            e.w32(iVar2 + 0xb0, 1);
            let t = k(e, ID_2a000028);
            e.w32(iVar2 + 0x98, t);
            let size = match param_2 {
                2 => 2,
                3 => 3,
                _ => 1,
            };
            e.w32(iVar2 + 0x20, size);
            return Ok(0);
        }
        0x121 => {
            if param_2 == e.r32(iVar2 + 0x94) || e.r32(iVar2 + 0xb8) != 0 {
                return Ok(0);
            }
            let fVar6 = APIextra::TimerGetTime(e, GAME_TIMER);
            e.wf(iVar2 + 0xa0, fVar6 + 1.0);
            return Ok(0);
        }
        0xf7 | 0x126 => {}
        _ => return Ok(0),
    }
    // 0xf7 / 0x126
    if param_3 != NULL {
        for i in 0..13u32 {
            let v = e.r32(param_3 + i * 4);
            e.w32(iVar2 + 0x24 + i * 4, v);
        }
    }
    e.w32(iVar2 + 0xac, 1);
    let gd = GD(e);
    let n = e.r32(gd + 0x10) + 1;
    e.w32(gd + 0x10, n);
    Lat_WispFUNC_AddWisp(e, iVar2, n, param_2 as i32);
    Ok(0)
}

/// 0x00015b08 clipping: 1 = never clipped (spawned/idle), 2 = active ranges, 3 = dead ranges
pub fn Wisp_baseFUNC_ClippingSet(e: &mut Engine, param_1: u32) {
    let p = e.alloca(0x28);
    APIgameobj::GetClippingParams(e, SELF, p);
    e.w32(p, 0x3dcc_cccd);
    if param_1 == 2 {
        e.w32(p + 0x04, 0x41f0_0000);
        e.w32(p + 0x10, 0x4248_0000);
        e.w32(p + 0x1c, 0xbf80_0000);
        e.w32(p + 0x0c, 0);
        e.w32(p + 0x08, 0x227);
        e.w32(p + 0x18, 0);
        e.w32(p + 0x14, 0x6a7);
        e.w32(p + 0x24, 0x6a7);
        e.w32(p + 0x20, 0x6a7);
    } else if param_1 != 3 {
        e.w32(p + 0x04, 0xbf80_0000);
        e.w32(p + 0x0c, 0);
        e.w32(p + 0x08, 0);
    } else {
        e.w32(p + 0x04, 0xbf80_0000);
        e.w32(p + 0x0c, 0x227);
        e.w32(p + 0x08, 0x227);
    }
    APIgameobj::SetClippingParams(e, SELF, p, 0);
}

/// 0x00015c60 start/stop the halo (not while carried) and spirit emitters
pub fn Wisp_baseFUNC_AllFXControl(e: &mut Engine, param_1: i32) {
    let iVar1 = L(e);
    if param_1 == 0 {
        let a = e.ri32(iVar1 + 0x78);
        APIparticle::EmitterStop(e, a);
        let a = e.ri32(iVar1 + 0x7c);
        APIparticle::EmitterStop(e, a);
    } else {
        if e.r32(iVar1 + 0xa8) == 0 {
            let a = e.ri32(iVar1 + 0x78);
            APIparticle::EmitterStart(e, a, 1);
        }
        let a = e.ri32(iVar1 + 0x7c);
        APIparticle::EmitterStart(e, a, 1);
    }
}

/// 0x00015d38 target of drive module 1
pub fn Wisp_baseFUNC_SetTargetID(e: &mut Engine, param_1: Ptr, param_2: u32) {
    let auStack_40 = e.alloca(0x20);
    let dm = e.r32(param_1 + 0x80);
    APIphysic::DMGetParams(e, dm, auStack_40);
    e.w32(auStack_40, param_2);
    APIphysic::DMSetParams(e, dm, auStack_40);
}

/// 0x00015db0 bob: home + 0.1 * sin(3 t) in y (not while carried)
pub fn Wisp_baseFUNC_RunBob(e: &mut Engine) {
    let DStack_30 = e.alloca(16);
    let l = L(e);
    if e.r32(l + 0xa8) == 0 {
        let fVar1 = APIextra::TimerGetTime(e, GAME_TIMER);
        let fVar1 = LIBMiscFunc::MiscFunc_Sin(e, fVar1 * 3.0);
        let l = L(e);
        let x = e.rf(l + 0x68);
        let z = e.rf(l + 0x70);
        let y = e.rf(l + 0x6c) + fVar1 * 0.1;
        e.wv3(DStack_30, [x, y, z]);
        APIgameobj::SetTranslation(e, SELF, DStack_30);
    }
}

/// 0x00015eb0 register a launched wisp as number `param_2`; past the level's limit the oldest one
/// is removed and killed. `param_3` is the size (1 medium, 2 large, else small).
pub fn Lat_WispFUNC_AddWisp(e: &mut Engine, param_1: Ptr, param_2: u32, param_3: i32) {
    let gd = GD(e);
    let me = e.r32(param_1 + 0x94);
    let a = gd + k(e, Lat_WispFUNC_AddWisp_0_1) + param_2.wrapping_sub(1).wrapping_mul(4);
    e.w32(a, me);
    if k(e, Lat_WispFUNC_AddWisp_0_2) < param_2 {
        let uVar1 = e.r32(GD(e) + k(e, Lat_WispFUNC_AddWisp_0_3));
        Lat_WispFUNC_RemoveWisp(e, param_1, uVar1 as i32);
        APIgameobj::KillObject(e, uVar1);
    }
    let size = match param_3 {
        1 => 2,
        2 => 3,
        _ => 1,
    };
    e.w32(param_1 + 0x20, size);
}

/// 0x00015fd8 unregister launched wisp `param_2` (if `param_1` is a launched wisp): shift the list
pub fn Lat_WispFUNC_RemoveWisp(e: &mut Engine, param_1: Ptr, param_2: i32) {
    let iVar1 = GD(e);
    if e.r32(param_1 + 0xac) == 0 {
        return;
    }
    let n = e.r32(iVar1 + 0x10).wrapping_sub(1);
    e.w32(iVar1 + 0x10, n);
    let mut iStack_24: i32 = 0;
    let found = loop {
        if (k(e, Lat_WispFUNC_RemoveWisp_0_1) as i32) < iStack_24 {
            break false;
        }
        if e.ri32(iVar1 + k(e, Lat_WispFUNC_RemoveWisp_0_2) + iStack_24 as u32 * 4) == param_2 {
            break true;
        }
        iStack_24 += 1;
    };
    if found {
        let mut iStack_20 = iStack_24;
        while iStack_20 < k(e, Lat_WispFUNC_RemoveWisp_0_3) as i32 {
            let v = e.r32(iVar1 + k(e, Lat_WispFUNC_RemoveWisp_0_5) + (iStack_20 as u32 + 1) * 4);
            e.w32(iVar1 + k(e, Lat_WispFUNC_RemoveWisp_0_4) + iStack_20 as u32 * 4, v);
            iStack_20 += 1;
        }
        let a = iVar1 + k(e, Lat_WispFUNC_RemoveWisp_0_6);
        e.w32(a, 0);
    }
    // code_r0x00016110: `for (iStack_24 = 0; iStack_24 < 7; iStack_24++) {}` (empty loop, no effect)
}

/// 0x00016158 already collected (its bit is set in game buffer 2 +0x14): go to Dead with "remove"
pub fn Wisp_baseFUNC_AmIDead(e: &mut Engine, param_1: Ptr) {
    let bit = e.r32(param_1 + 0xc);
    if bit != 0 && e.r32(e.r32(GD(e) + 4) + 0x14) & bit != 0 {
        let f = e.r32(param_1 + 4);
        e.w32(param_1 + 4, f | 0x20_0000);
        e.w32(param_1, ADDR_Dead_In);
    }
}

/// 0x000161d8 mark collected
pub fn Wisp_baseFUNC_SetBit(e: &mut Engine, param_1: Ptr) {
    let bit = e.r32(param_1 + 0xc);
    if bit != 0 {
        let a = e.r32(GD(e) + 4) + 0x14;
        let v = e.r32(a) | bit;
        e.w32(a, v);
    }
}

/// 0x00016238 unmark collected
pub fn Wisp_baseFUNC_ResetBit(e: &mut Engine, param_1: Ptr) {
    let a = e.r32(GD(e) + 4) + 0x14;
    let v = e.r32(a) & !e.r32(param_1 + 0xc);
    e.w32(a, v);
}

/// 0x00016290 the six shared particle types and the GENERIC_FX descriptor (in the global data)
pub fn Wisp_baseFUNC_Init_Particle(e: &mut Engine, param_1: Ptr) {
    for (off, tex, name) in [
        (0x14u32, "halo", "Wisp_small_halo"),
        (0x20, "spirit", "Wisp_small_spirit"),
        (0x18, "halo", "Wisp_medium_halo"),
        (0x24, "spirit", "Wisp_medium_spirit"),
        (0x1c, "halo", "Wisp_large_halo"),
        (0x28, "spirit", "Wisp_large_spirit"),
    ] {
        let uVar1 = APIparticle::TypeCreate(e);
        e.w32(param_1 + off, uVar1);
        let t = e.ri32(param_1 + off);
        let s = e.cstr(tex);
        APIparticle::TypeSetTexture(e, t, s);
        let s = e.cstr(name);
        APIparticle::TypeSetName(e, t, s);
    }
    let a = param_1 + k(e, Wisp_baseFUNC_Init_Particle_0_1);
    e.w32(a, 0x4040_0000);
    let f = e.code_ptr(CodeFn::Particle(SetParticleWispTouchCollect_spark_small));
    let a = param_1 + k(e, Wisp_baseFUNC_Init_Particle_0_2);
    e.w32(a, f);
    let (d, s) = (param_1 + k(e, Wisp_baseFUNC_Init_Particle_0_3), e.cstr("wisp_collect_spark"));
    LIBMiscFunc::MiscFunc_StrncpySafe(e, d, s, 0x40);
    let (d, s) = (param_1 + k(e, Wisp_baseFUNC_Init_Particle_0_4), e.cstr("spark"));
    LIBMiscFunc::MiscFunc_StrncpySafe(e, d, s, 0x40);
    let a = param_1 + k(e, Wisp_baseFUNC_Init_Particle_0_5);
    e.w32(a, 0);
    let f = e.code_ptr(CodeFn::Particle(SetParticleWispTouchCollect_halo_small));
    let a = param_1 + k(e, Wisp_baseFUNC_Init_Particle_0_6);
    e.w32(a, f);
    let (d, s) = (param_1 + k(e, Wisp_baseFUNC_Init_Particle_0_7), e.cstr("wisp_collect_halo"));
    LIBMiscFunc::MiscFunc_StrncpySafe(e, d, s, 0x40);
    let (d, s) = (param_1 + k(e, Wisp_baseFUNC_Init_Particle_0_8), e.cstr("halo"));
    LIBMiscFunc::MiscFunc_StrncpySafe(e, d, s, 0x40);
    let a = param_1 + k(e, Wisp_baseFUNC_Init_Particle_0_9);
    e.w32(a, 0);
}

/// 0x00016610 look up this wisp (+0x94) in the level table: size, collected bit, map word 1.
/// Builds with a single wisp test only record 0 (shape group 1); not found = left at 0.
pub fn Wisp_BaseFUNC_Init_LocalSetting(e: &mut Engine, param_1: Ptr) {
    let iVar1 = L(e);
    let puVar2 = iVar1 + 8;
    let gd = GD(e);
    let lim = if g(e, G_Wisp_BaseFUNC_Init_LocalSetting) == 1 { 0 } else { k(e, Wisp_BaseFUNC_Init_LocalSetting_0_1) };
    let stride = if g(e, G_Wisp_BaseFUNC_Init_LocalSetting) == 1 { 0x24 } else { k(e, Wisp_BaseFUNC_Init_LocalSetting_0_2) };
    let mut uStack_18: u32 = 0;
    loop {
        if lim < uStack_18 {
            return;
        }
        if e.r32(gd + 0x2c + uStack_18 * stride) == e.r32(param_1 + 0x94) {
            break;
        }
        uStack_18 += 1;
    }
    let r = gd + uStack_18 * stride;
    let v = e.r32(r + 0x48);
    e.w32(iVar1 + 0x20, v);
    let v = e.r32(r + 0x34);
    e.w32(iVar1 + 0xc, v);
    let v = e.r32(r + 0x30);
    e.w32(puVar2, v);
}

/// 0x00016738
pub fn Wisp_baseFUNC_Create_GameObject(e: &mut Engine) {
    APIgameobj::SetGroupID(e, SELF, 0x1a);
}

/// 0x00016780 physics and four drive modules: 1 follow the book (speed 12.5), 4 speed (L+0x58), 7
/// (0.7), 0 gravity (0, -5, 0)
pub fn Wisp_baseFUNC_Create_Physic(e: &mut Engine, param_1: Ptr) {
    let uStack_70 = e.alloca(0x20);
    let uStack_60 = e.alloca(0x20);
    let uStack_40 = e.alloca(0x20);
    APIphysic::CreatePhysic(e, SELF, 0);
    APIphysic::SetFlags(e, SELF, 2);
    let uVar1 = APIphysic::DMAdd(e, SELF, 1);
    e.w32(param_1 + 0x80, uVar1);
    APIphysic::DMGetParams(e, uVar1, uStack_60);
    let book = k(e, ID_2a000025);
    for (i, v) in [book, 0x4148_0000, 0, 0, 0, 6].into_iter().enumerate() {
        e.w32(uStack_60 + i as u32 * 4, v);
    }
    let dm = e.r32(param_1 + 0x80);
    APIphysic::DMSetParams(e, dm, uStack_60);
    APIphysic::DMSetContextMask(e, dm, 2);
    let uVar1 = APIphysic::DMAdd(e, SELF, 4);
    e.w32(param_1 + 0x84, uVar1);
    APIphysic::DMGetParams(e, uVar1, param_1 + 0x58);
    e.w32(param_1 + 0x58, 0);
    e.w32(param_1 + 0x5c, 0);
    e.w32(param_1 + 0x9c, 0x3f80_0000);
    e.w32(param_1 + 0x60, 0x3f80_0000);
    e.w32(param_1 + 0x64, 1);
    let dm = e.r32(param_1 + 0x84);
    APIphysic::DMSetParams(e, dm, param_1 + 0x58);
    APIphysic::DMSetContextMask(e, dm, 1);
    let uVar1 = APIphysic::DMAdd(e, SELF, 7);
    e.w32(param_1 + 0x88, uVar1);
    APIphysic::DMGetParams(e, uVar1, uStack_70);
    e.w32(uStack_70, 0x3f33_3333);
    e.w32(uStack_70 + 4, 7);
    let dm = e.r32(param_1 + 0x88);
    APIphysic::DMSetParams(e, dm, uStack_70);
    APIphysic::DMSetContextMask(e, dm, 4);
    let uVar1 = APIphysic::DMAdd(e, SELF, 0);
    e.w32(param_1 + 0x8c, uVar1);
    APIphysic::DMGetParams(e, uVar1, uStack_40);
    e.w32(uStack_40, 0);
    e.w32(uStack_40 + 4, 0xc0a0_0000);
    e.w32(uStack_40 + 8, 0);
    let dm = e.r32(param_1 + 0x8c);
    APIphysic::DMSetParams(e, dm, uStack_40);
    APIphysic::DMSetContextMask(e, dm, 8);
}

/// 0x00016a88 configure the shared halo/spirit types for this wisp's size and create its emitters
pub fn Wisp_baseFUNC_Create_Particles(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    let pp = e.r32(G(e, pParticle));
    let gd = GD(e);
    let (halo_off, spirit_off) = match e.ri32(param_1 + 0x20) {
        1 => (0x14, 0x20),
        0 | 2 => (0x18, 0x24),
        3 => (0x1c, 0x28),
        _ => (0x14, 0x20),
    };
    let (ht, st) = (e.r32(gd + halo_off), e.r32(gd + spirit_off));
    match halo_off {
        0x18 => {
            SetParticleHaloWispMedium(e, pp, ht)?;
            SetParticleSpiritMedium(e, pp, st)?;
        }
        0x1c => {
            SetParticleHaloWispBig(e, pp, ht)?;
            SetParticleSpiritBig(e, pp, st)?;
        }
        _ => {
            SetParticleHaloWisp(e, pp, ht)?;
            SetParticleSpiritSmall(e, pp, st)?;
        }
    }
    for (slot, toff) in [(0x78u32, halo_off), (0x7c, spirit_off)] {
        let t = e.ri32(GD(e) + toff);
        let uVar2 = APIparticle::EmitterCreate(e, t);
        e.w32(param_1 + slot, uVar2);
        let em = e.ri32(param_1 + slot);
        APIparticle::EmitterSetJoint(e, em, SELF, ALL);
        let pp = e.ri32(G(e, pParticle));
        let em = e.r32(param_1 + slot);
        LIBMiscFunc::MiscFunc_EmitterUpdateFlags(e, pp, em, 2, 0);
    }
    Ok(())
}

/// 0x00016ed8 two-point motion trail (joints 6 and 7), coloured by size
pub fn Wisp_baseFUNC_Create_MotionTrail(e: &mut Engine, param_1: Ptr) {
    if e.ri32(param_1 + 0x90) == -2 {
        let uVar2 = APIgraphics::MTCreateSystem(e, 2, 5, 0);
        e.w32(param_1 + 0x90, uVar2);
        APIgraphics::MTSetFrequency(e, uVar2, 0.6);
        APIgraphics::MTSetJoint(e, uVar2, 0, SELF, 6);
    }
    let mt = e.r32(param_1 + 0x90);
    let (r, g_, b) = match e.ri32(param_1 + 0x20) {
        1 => (0.7, 0.5, 0.0),
        0 | 2 => (0.1, 0.45, 0.1),
        3 => (0.4, 0.0, 0.4),
        _ => (0.7, 0.5, 0.0),
    };
    APIgraphics::MTSetRGB(e, mt, 0, r, g_, b);
    APIgraphics::MTSetLifeSpan(e, mt, 0, 0.5);
    APIgraphics::MTSetJoint(e, mt, 1, SELF, 7);
    APIgraphics::MTSetRGB(e, mt, 1, 0.0, 0.0, 0.0);
    APIgraphics::MTSetLifeSpan(e, mt, 1, 0.5);
}

/// 0x00017148 shared constructor of every level's wisps module
pub fn wisp_baseConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    if instanceCount == 1 {
        APIcons::SetExternalFunction(e, 0, Wisp_baseST_FIRSTMOVE);
        APIcons::SetExternalFunction(e, 1, Wisp_baseST_ENTRY_POINT);
        APIcons::SetExternalFunction(e, 2, Wisp_baseCB_NOTIFY_COLLIDE_OBJ);
        let tables: [(u32, fn(&mut Engine) -> Ptr); 10] = [
            (pAnim, |e| APIcons::GetApiAnim(e)),
            (pExtra, |e| APIcons::GetApiExtra(e)),
            (pGameObj, |e| APIcons::GetApiGameObj(e)),
            (pGraphic, |e| APIcons::GetApiGraphics(e)),
            (pLight, |e| APIcons::GetApiLight(e)),
            (pMath, |e| APIcons::GetApiMath(e)),
            (pParticle, |e| APIcons::GetApiParticle(e)),
            (pPhysic, |e| APIcons::GetApiPhysic(e)),
            (pSound, |e| APIcons::GetApiNewSound(e)),
            (pStdlib, |e| APIcons::GetApiStdlib(e)),
        ];
        for (slot, get) in tables {
            let v = get(e);
            let a = G(e, slot);
            e.w32(a, v);
        }
        for (slot, name) in [(pDebug, "debug"), (pAnimMngr, "AnimMngr"), (pFunctions, "ApiCFunctions"), (pMiscFunc, "MiscFunc")] {
            let n = e.cstr(name);
            let v = APIcons::GetApiCustom(e, n);
            let a = G(e, slot);
            e.w32(a, v);
        }
        let slot = pGlobalDataWisp_base(e);
        let size = k(e, wisp_baseConstructor_0_1) as i32;
        APIcons::GetGlobalData(e, slot, size);
        let gd = GD(e);
        let uVar1 = APIgameobj::GetGameBuffer(e, 0, 0x8f8, 1);
        e.w32(gd, uVar1);
        let uVar1 = APIgameobj::GetGameBuffer(e, 1, 0x68, 1);
        e.w32(gd + 8, uVar1);
        let uVar1 = APIgameobj::GetGameBuffer(e, 2, 0x158, 1);
        e.w32(gd + 4, uVar1);
        e.w32(gd + 0xc, 1);
        Wisp_baseFUNC_Init_Particle(e, gd);
        InitWispData(e);
        let fx = k(e, ID_2a80016a);
        let desc = GD(e) + k(e, wisp_baseConstructor_0_2) * 4;
        let uVar1 = LIBMiscFunc::MiscFunc_CreateGenericFX(e, fx, desc, 2);
        let a = GD(e) + k(e, wisp_baseConstructor_0_3) * 4;
        e.w32(a, uVar1);
    } else {
        let gd = GD(e);
        let n = e.r32(gd + 0xc) + 1;
        e.w32(gd + 0xc, n);
    }
    APIcons::SetMessageFunction(e, Wisp_baseFUNC_MyMessageFunction);
    let slot = pLocalDataWisp_base(e);
    APIcons::GetLocalData(e, slot, 0xc0);
    let l = L(e);
    e.w32(l + 0x90, 0xffff_fffe);
    e.wi32(l + 0xbc, instanceCount);
    let uVar1 = APIgameobj::GetID(e, SELF, NULL);
    e.w32(l + 0x94, uVar1);
    Wisp_baseFUNC_ClippingSet(e, 1);
    let l = L(e);
    e.w32(l, ADDR_Inactive_In);
    APIcons::SetInitialState(e, 0);
    Ok(())
}

/// 0x00017550 shared destructor: emitters, motion trail; the last wisp frees the shared types and
/// the GENERIC_FX object
pub fn wisp_baseDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    let gd = GD(e);
    let n = e.r32(gd + 0xc).wrapping_sub(1);
    e.w32(gd + 0xc, n);
    Wisp_baseFUNC_AllFXControl(e, 0);
    let a = e.ri32(L(e) + 0x78);
    APIparticle::EmitterDestroy(e, a);
    let a = e.ri32(L(e) + 0x7c);
    APIparticle::EmitterDestroy(e, a);
    if e.ri32(L(e) + 0x90) != -2 {
        let mt = e.r32(L(e) + 0x90);
        APIgraphics::MTStopDraw(e, mt);
        APIgraphics::MTDestroySystem(e, mt);
        let l = L(e);
        e.w32(l + 0x90, 0xffff_fffe);
    }
    if instanceCount == 0 {
        for off in [0x14u32, 0x20, 0x18, 0x24, 0x1c, 0x28] {
            let t = e.ri32(GD(e) + off);
            APIparticle::TypeDestroy(e, t);
        }
        let fx = e.ri32(GD(e) + k(e, wisp_baseDestructor_0_1));
        LIBMiscFunc::MiscFunc_DestroyGenericFX(e, fx);
    }
    Ok(())
}

// debug: GAME_WISPDBG=<wisp uid hex> logs that wisp's state/pos per frame
fn wdbg(e: &mut Engine, param_1: Ptr, st: &str) {
    if let Some(v) = std::env::var_os("GAME_WISPDBG") {
        let id = e.r32(param_1 + 0x94);
        if u32::from_str_radix(v.to_str().unwrap_or(""), 16).is_ok_and(|w| w == id) {
            let t = e.alloca(16);
            APIgameobj::GetPosition(e, SELF, ALL, t);
            eprintln!("wispdbg {st} {id:#x} pos {:?} home {:?} spd {} flags {:#x} a8 {:#x}", e.mem.rv3(t), e.mem.rv3(param_1 + 0x68), e.rf(param_1 + 0x9c), e.r32(param_1 + 4), e.r32(param_1 + 0xa8));
        }
    }
}

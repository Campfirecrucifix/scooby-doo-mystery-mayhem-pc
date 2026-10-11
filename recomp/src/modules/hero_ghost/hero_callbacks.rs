//! hero_callbacks.c (8 functions). Addresses are module-relative (.text at 0x10000) in scooby_ghost__08a011d0.
use super::builds::*;
use super::*;
use crate::engine::*;
use crate::logic::LResult;

/// Fills the edge quad from the collision edge `q` (a=q+0, b=q+0xc) and the offset vector at local+`off`.
fn edge_quad(e: &mut Engine, l: Ptr, q: Ptr, off: u32) {
    for i in 0..3 {
        let b = e.r32(q + 0xc + i * 4);
        e.w32(l + 0x314 + i * 4, b);
    }
    for i in 0..3 {
        let a = e.r32(q + i * 4);
        e.w32(l + 800 + i * 4, a);
    }
    for i in 0..3 {
        let v = e.rf(q + 0xc + i * 4) + e.rf(l + off + i * 4);
        e.wf(l + 0x32c + i * 4, v);
    }
    for i in 0..3 {
        let v = e.rf(q + i * 4) + e.rf(l + off + i * 4);
        e.wf(l + 0x338 + i * 4, v);
    }
}

/// 0x000147a8 (432 bytes) `undefined8 heroCB_STATECHANGE(uint *param_1)`
pub fn heroCB_STATECHANGE(e: &mut Engine, param_1: u32) -> LResult<u64> {
    let (s0, s1) = (e.r32(param_1), e.r32(param_1 + 4));
    if s0 & 1 == 0 || s1 & 1 == 0 {
        if s0 & 2 != 0 && s1 & 2 != 0 {
            debug_info(e, "FALLinggggggggggggggggggggg\n\n\n");
            APIphysic::ResetStateChangeNotifyBits(e, SELF, 2);
            APIphysic::SetStateChangeNotifyBits(e, SELF, 1);
        }
    } else {
        APIphysic::ResetStateChangeNotifyBits(e, SELF, 1);
        debug_info(e, "OUCH from fallinggggggggggggggggggggg\n\n\n");
        if is_leader(e) {
            heroFUNC_CallVibration(e, 0.66, 0.0, 0.2, 0.0)?;
        }
    }
    Ok(0)
}

/// 0x00014958 (1604 bytes) `undefined8 heroCB_DMSTATECHANGE(int *param_1)`
pub fn heroCB_DMSTATECHANGE(e: &mut Engine, param_1: u32) -> LResult<u64> {
    let l = L(e);
    let h = e.r32(param_1);
    let action = if h == e.r32(l + 0x1c0) {
        debug_info(e, "CB_DMSTATECHANGE -> ACTION_EDGE_UP_TEAM_IN");
        0x19
    } else if h == e.r32(l + 0x1c8) {
        debug_info(e, "CB_DMSTATECHANGE -> ACTION_CROUCH");
        0x1b
    } else if h == e.r32(l + 0x1dc) {
        0x2e
    } else if h == e.r32(l + 0x1e4) {
        debug_info(e, "CB_DMSTATECHANGE -> ACTION_SNORKEL_IN");
        0x1e
    } else if h == e.r32(l + 0x1e8) {
        debug_info(e, "CB_DMSTATECHANGE -> ACTION_SNORKEL_OUT");
        0x1f
    } else {
        debug_info(e, "CB_DMSTATECHANGE -> ACTION_UNKNOWN == BUG!!!");
        return Ok(0);
    };
    let (new, changed) = (e.r32(param_1 + 8), e.r32(param_1 + 0xc));
    match action {
        0x1b => {
            if changed & 1 != 0 && new & 1 != 0 {
                debug_time(e, "heroCB_DMSTATECHANGE->ACTION_CROUCH detected", 0, 0)?;
                or_w(e, l + 0x170, 0x400);
            }
            if changed & 2 != 0 && new & 2 != 0 {
                debug_time(e, "heroCB_DMSTATECHANGE->ACTION_CROUCH go", 0, 0)?;
                or_w(e, l + 0x164, 0x400);
            }
        }
        0x19 => {
            if changed & 1 == 0 || new & 1 == 0 {
                if changed & 2 == 0 || new & 2 == 0 {
                    debug_info_value(e, "ACTION_EDGE_UP_TEAM_IN -> discard -> ui32NewState", param_1 + 8, 1);
                } else {
                    debug_time(e, "heroCB_DMSTATECHANGE->ACTION_EDGE_UP_TEAM_IN hanged", 0, 0)?;
                    let b = GB0(e) + 0x14;
                    or_w(e, b, 0x800);
                    let prim = e.r32(l + 0x18c);
                    APIphysic::PrimResetIntAttribBits(e, SELF, prim, 2, 4);
                    or_w(e, l + 0x170, 0x100);
                }
            } else {
                debug_time(e, "heroCB_DMSTATECHANGE->ACTION_EDGE_UP_TEAM_IN detected", 0, 0)?;
                let b = GB0(e) + 0x14;
                or_w(e, b, 0x800);
                or_w(e, l + 0x170, 0x80);
            }
        }
        0x2e => {
            if changed & 1 == 0 {
                if new & 2 == 0 {
                    debug_info(e, "ACTION_FOLLOW --> NOTHING");
                    debug_info_value(e, "pChangeState->ui32NewState", param_1 + 8, 1);
                    debug_info_value(e, "pChangeState->ui32ChangedState", param_1 + 0xc, 1);
                } else {
                    debug_info(e, "CB_DMSTATECHANGE -> _PS_DMSTATE_FOLLOW_BLOCKED");
                    or_w(e, l + 0x170, 0x10_0000);
                }
            } else if new & 1 == 0 {
                or_w(e, l + 0x168, 0x20);
            } else {
                or_w(e, l + 0x168, 0x10);
            }
        }
        // 0x1e / 0x1f (snorkel): nothing
        _ => {}
    }
    Ok(0)
}

/// 0x00014fa0 (1780 bytes) `undefined8 heroCB_EDGEDETECT(int *param_1)`
pub fn heroCB_EDGEDETECT(e: &mut Engine, param_1: u32) -> LResult<u64> {
    let l = L(e);
    debug_info_value(e, "Edge Detect", param_1, 1);
    let id = e.r32(param_1);
    let on = e.r32(param_1 + 4) != 0;
    let q = e.r32(param_1 + 8);
    let b14 = GB0(e) + 0x14;
    if id == 1 {
        if !on {
            and_w(e, b14, 0xffff_efff);
        } else {
            let v = e.alloca(0x10);
            let c = e.alloca(0x10);
            for i in 0..3 {
                let d = e.rf(q + i * 4) - e.rf(q + 0xc + i * 4);
                e.wf(v + i * 4, d);
            }
            APImath::CVectorNormalize(e, v, v);
            APImath::CVectorCross(e, c, v, q + 0x18);
            for i in 0..3 {
                let x = e.r32(c + i * 4);
                e.w32(l + 0x2f0 + i * 4, x);
            }
            for i in 0..3 {
                let x = e.rf(c + i * 4) * e.rf(l + 0x348);
                e.wf(l + 0x2f0 + i * 4, x);
            }
            or_w(e, b14, 0x1000);
            edge_quad(e, l, q, 0x2f0);
            heroFUNC_SetEdgeLimits(e, l)?;
        }
    } else if id == 2 {
        if !on {
            and_w(e, b14, 0xffff_bfff);
        } else {
            let mid: [f32; 3] = std::array::from_fn(|i| (e.rf(q + i as u32 * 4) + e.rf(q + 0xc + i as u32 * 4)) * 0.5);
            for i in 0..3 {
                let v = mid[i as usize] + e.rf(q + 0x18 + i * 4);
                e.wf(l + 0x308 + i * 4, v);
            }
            let (nx, nz) = (e.rf(q + 0x18), e.rf(q + 0x20));
            let a = APImath::atan2f(e, -nx, -nz);
            e.wf(l + 0x344, a);
            for i in 0..3 {
                let v = e.rf(q + 0x18 + i * 4) * e.rf(l + 0x34c);
                e.wf(l + 0x2fc + i * 4, v);
            }
            or_w(e, b14, 0x4000);
            edge_quad(e, l, q, 0x2fc);
            heroFUNC_SetEdgeLimits(e, l)?;
        }
    }
    Ok(0)
}

/// 0x00015698 (1944 bytes) `void heroFUNC_SetEdgeLimits(int param_1)`
pub fn heroFUNC_SetEdgeLimits(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    let p = param_1;
    debug_info_value(e, "SideA", p + 0x314, 4);
    debug_info_value(e, "SideB", p + 800, 4);
    debug_info_value(e, "SideC", p + 0x32c, 4);
    debug_info_value(e, "SideD", p + 0x338, 4);
    let f = |e: &Engine, o: u32| e.rf(p + o);
    if f(e, 800) == f(e, 0x314) || f(e, 0x32c) == f(e, 0x314) {
        let mut i24: u32 = 0;
        e.w32(p + 0x368, 1);
        let b = f(e, 0x32c) == f(e, 0x314);
        if b {
            i24 = 2;
        }
        let mut u28 = b as u32;
        if f(e, 800) == f(e, 0x314) {
            u28 = 2;
            i24 = 1;
        }
        let ox = 0x314 + u28 * 0xc;
        if f(e, 0x314) < f(e, ox) {
            let v = e.r32(p + ox);
            e.w32(p + 0x360, v);
            let v = e.r32(p + 0x314);
            e.w32(p + 0x358, v);
        } else {
            let v = e.r32(p + 0x314);
            e.w32(p + 0x360, v);
            let v = e.r32(p + ox);
            e.w32(p + 0x358, v);
        }
        let oz = 0x31c + i24 * 0xc;
        if f(e, 0x31c) < f(e, oz) {
            let v = e.r32(p + oz);
            e.w32(p + 0x364, v);
            let v = e.r32(p + 0x31c);
            e.w32(p + 0x35c, v);
        } else {
            let v = e.r32(p + 0x31c);
            e.w32(p + 0x364, v);
            let v = e.r32(p + oz);
            e.w32(p + 0x35c, v);
        }
        debug_info_value(e, "Max1", p + 0x360, 2);
        debug_info_value(e, "Min1", p + 0x358, 2);
        debug_info_value(e, "Max2", p + 0x364, 2);
        debug_info_value(e, "Min2", p + 0x35c, 2);
    } else {
        e.w32(p + 0x368, 0);
        let s1 = (f(e, 0x328) - f(e, 0x31c)) / (f(e, 800) - f(e, 0x314));
        e.wf(p + 0x350, s1);
        let v = f(e, 0x31c) - f(e, 0x350) * f(e, 0x314);
        e.wf(p + 0x360, v);
        let v = f(e, 0x334) - f(e, 0x350) * f(e, 0x32c);
        e.wf(p + 0x358, v);
        if f(e, 0x360) < f(e, 0x358) {
            let t = e.r32(p + 0x360);
            let u = e.r32(p + 0x358);
            e.w32(p + 0x360, u);
            e.w32(p + 0x358, t);
        }
        let s2 = (f(e, 0x334) - f(e, 0x31c)) / (f(e, 0x32c) - f(e, 0x314));
        e.wf(p + 0x354, s2);
        let v = f(e, 0x328) - f(e, 0x354) * f(e, 800);
        e.wf(p + 0x364, v);
        let v = f(e, 0x31c) - f(e, 0x354) * f(e, 0x314);
        e.wf(p + 0x35c, v);
        if f(e, 0x364) < f(e, 0x35c) {
            let t = e.r32(p + 0x364);
            let u = e.r32(p + 0x35c);
            e.w32(p + 0x364, u);
            e.w32(p + 0x35c, t);
        }
        debug_info_value(e, "Pente1", p + 0x350, 2);
        debug_info_value(e, "Max1", p + 0x360, 2);
        debug_info_value(e, "Min1", p + 0x358, 2);
        debug_info_value(e, "Pente2", p + 0x354, 2);
        debug_info_value(e, "Max2", p + 0x364, 2);
        debug_info_value(e, "Min2", p + 0x35c, 2);
    }
    Ok(())
}

/// 0x00015e30 (244 bytes) `undefined4 heroCB_TIMER(int param_1)`
pub fn heroCB_TIMER(e: &mut Engine, param_1: u32) -> LResult<u64> {
    let l = L(e);
    if param_1 == e.r32(l + 500) {
        let b = GB0(e);
        e.w32(b + 0x74, 0);
    } else if param_1 == e.r32(l + 0x1f8) {
        or_w(e, l + 0x168, 2);
        return Ok(0);
    } else if param_1 == e.r32(l + 0x1fc) {
        let dm = e.r32(l + 0x1c4);
        APIphysic::DMSetActive(e, dm, 1);
    }
    Ok(1)
}

/// 0x00015f30 (668 bytes) `undefined8 heroCB_ANIM(uint *param_1)`
pub fn heroCB_ANIM(e: &mut Engine, param_1: u32) -> LResult<u64> {
    let l = L(e);
    let ty = e.r32(param_1 + 4);
    if ty == 1 {
        let ev = e.r32(param_1);
        if ev == 0xffff_f840 {
            if e.r32(l + 0x5f * 4) == k(e, heroCB_ANIM_0_1) {
                if e.r32(param_1 + 8) == 0 {
                    heroFUNC_CallVibration(e, 0.33, 0.0, 0.2, 0.0)?;
                } else {
                    debug_info(e, "CB_TYPE_ACTION --> NO_ANI_SWITCH");
                    or_w(e, l + 0x5a * 4, 0x1000_0000);
                }
            }
        } else if ev < 0xffff_f841 {
            if ev == 0xffff_f834 {
                or_w(e, l + 0x5a * 4, 8);
            }
        } else if ev == 0xffff_f860 {
            if e.r32(GB0(e) + 8) == e.r32(l + 0x5f * 4) {
                let to = e.r32(l + 0x8f * 4);
                APIgameobj::SendMessageTo(e, to, 0, 0xee, 0, 0);
            }
        } else if ev == 0xffff_f873 {
            or_w(e, l + 0x59 * 4, 0x40000);
        }
    } else if ty == 0 {
        let (id, v) = (e.r32(param_1), e.r32(param_1 + 8));
        heroFUNC_PlaySound(e, l, id, v)?;
    } else if ty == 2 {
        heroFUNC_LaunchParticles(e)?;
    }
    Ok(0)
}

/// 0x000161d0 (92 bytes) `undefined4 heroCB_LASTFRAME(undefined4 param_1)`
/// local+0x254 = the anim that ended; local+0x168 bit 0 = "anim ended" (polled by the state loops).
pub fn heroCB_LASTFRAME(e: &mut Engine, param_1: u32) -> LResult<u64> {
    let l = L(e);
    e.w32(l + 0x254, param_1);
    or_w(e, l + 0x168, 1);
    Ok(1)
}

/// 0x00016230 (404 bytes) `undefined8 heroCB_CUTSCENE(int param_1)`
/// Start: event 0x4000000, clear local+0x174 bit 2; the leader in power-up mode (GB0+0x14 bit 0) ends it
/// (message 0xda to the per-level object). End: event 0x8000000.
pub fn heroCB_CUTSCENE(e: &mut Engine, param_1: u32) -> LResult<u64> {
    let l = L(e);
    if e.r32(param_1 + 4) == 1 {
        debug_time(e, "heroCB_CUTSCENE->event_ACTION_CUTSCENE_START", 0, 0)?;
        or_w(e, l + 0x170, 0x400_0000);
        let v = e.r32(l + 0x174) & 0xffff_fffb;
        e.w32(l + 0x174, v);
        let b14 = GB0(e) + 0x14;
        if e.r32(b14) & 1 != 0 && is_leader(e) {
            and_w(e, b14, 0xffff_fffe);
            APIgameobj::SendMessageTo(e, k(e, ID_2a000093), 0, 0xda, 1, 0);
        }
    } else {
        debug_time(e, "heroCB_CUTSCENE->event_ACTION_CUTSCENE_END", 0, 0)?;
        or_w(e, l + 0x170, 0x800_0000);
    }
    Ok(0)
}


// ---- helpers (from the hero port) ----

pub(crate) fn or_w(e: &mut Engine, a: Ptr, v: u32) {
    let x = e.r32(a);
    e.w32(a, x | v);
}

pub(crate) fn and_w(e: &mut Engine, a: Ptr, v: u32) {
    let x = e.r32(a);
    e.w32(a, x & v);
}

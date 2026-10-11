//! PoltergeistII_Base_Utils.c (13 functions): DM helpers (tracking speed / target), the path
//! finding wrappers (only used when the record's path flag L+0x1a4 is set), and the three
//! Monster_Base external states every poltergeist build overrides. Addresses are module-relative
//! (.text at 0x10000) in poltergeist__0bb93729. All builds share one shape.
use super::*;
use crate::engine::*;
use crate::logic::LResult;

/// 0x00012f28 speed (+8) of the turn DM (L+8); `param_1` is the raw float bits
pub fn Poltergeist_BaseFUNC_SetTrackingSpeed(e: &mut Engine, param_1: u32) -> LResult<()> {
    let b = e.alloca(0x80);
    let dm = e.r32(L(e) + 8);
    APIphysic::DMGetParams(e, dm, b);
    e.w32(b + 8, param_1);
    APIphysic::DMSetParams(e, dm, b);
    Ok(())
}

/// 0x00012fa8 move DM (L+4) towards point `*param_1`, mode 5. As decompiled, the target's y is
/// replaced by 0.5 (only x (+0) and z (+8) are read from `param_1`).
pub fn Poltergeist_BaseFUNC_TrackPos(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    let x = e.r32(param_1);
    let uVar1 = e.r32(param_1 + 8);
    let iStack_40 = L(e);
    let b = e.alloca(0x80);
    let dm = e.r32(iStack_40 + 4);
    APIphysic::DMGetParams(e, dm, b);
    for (o, v) in [(0, 0), (0x14, 5), (8, x), (0xc, 0x3f00_0000), (0x10, uVar1)] {
        e.w32(b + o, v);
    }
    APIphysic::DMSetParams(e, dm, b);
    Ok(())
}

/// 0x00013078 move DM (L+4) towards object `param_1`, mode 5
pub fn Poltergeist_BaseFUNC_TrackObj(e: &mut Engine, param_1: u32) -> LResult<()> {
    let iStack_30 = L(e);
    let b = e.alloca(0x80);
    let dm = e.r32(iStack_30 + 4);
    APIphysic::DMGetParams(e, dm, b);
    for (o, v) in [(0x14, 5), (8, 0), (0xc, 0), (0x10, 0), (0, param_1)] {
        e.w32(b + o, v);
    }
    APIphysic::DMSetParams(e, dm, b);
    Ok(())
}

/// 0x00013110 (sic) the hero may be engaged: not hiding / in a cutscene (game buffer 0 +0x14 &
/// 0x4900094c) and, when this poltergeist uses a path, standing on it
pub fn Poltergeist_BaseFUNC_TestAtcivationCondition(e: &mut Engine) -> LResult<u32> {
    let flags = |e: &Engine| e.r32(e.r32(GDP(e)) + 0x14);
    if e.r32(L(e) + 0x1a4) == 0 && flags(e) & 0x4900_094c == 0 {
        return Ok(1);
    }
    let hero = e.r32(e.r32(GDM(e)) + 8);
    let lVar2 = Poltergeist_BaseFUNC_Path_IsOnPath(e, hero)?;
    if lVar2 == 0 || flags(e) & 0x4900_094c != 0 {
        Ok(0)
    } else {
        Ok(1)
    }
}

/// 0x000131e8 find a path from here to `*param_1` (node L+500, path L+0x1e4) and head for its first
/// node. Returns 0 without a path flag, else NextPathNode's result.
pub fn Poltergeist_BaseFUNC_Path_SetPathFinding(e: &mut Engine, param_1: Ptr) -> LResult<i32> {
    let uStack_70 = e.alloca(0xc);
    for o in [0, 4, 8] {
        let v = e.r32(param_1 + o);
        e.w32(uStack_70 + o, v);
    }
    let mut iStack_50 = 2;
    if e.r32(L(e) + 0x1a4) == 0 {
        iStack_50 = 0;
    } else {
        let DStack_60 = e.alloca(0xc);
        APIgameobj::GetPosition(e, SELF, ALL, DStack_60);
        let l = L(e);
        LIBApiPathFinding::FindPath(e, DStack_60, uStack_70, l + 500, (l + 0x1e4) as i32);
        let l = L(e);
        iStack_50 = LIBApiPathFinding::NextPathNode(e, l + 500, (l + 0x1e4) as i32) as i32;
        if iStack_50 != 0 {
            let p = L(e) + 500;
            Poltergeist_BaseFUNC_TrackPos(e, p)?;
        }
    }
    Ok(iStack_50)
}

/// 0x00013340 within 1.5 m of the current node: next node. Returns 0 without a path flag, -1 while
/// walking to the node, else NextPathNode's result (2: the path is done).
pub fn Poltergeist_BaseFUNC_Path_UpdatePathFinding(e: &mut Engine) -> LResult<i32> {
    let mut iStack_40 = -1;
    if e.r32(L(e) + 0x1a4) == 0 {
        iStack_40 = 0;
    } else {
        let p = L(e) + 500;
        let bVar1 = LIBApiCFunctions::CheckRangePoint(e, 1.5, SELF, p, 0);
        if bVar1 != 0 {
            let l = L(e);
            iStack_40 = LIBApiPathFinding::NextPathNode(e, l + 500, (l + 0x1e4) as i32) as i32;
            let p = L(e) + 500;
            Poltergeist_BaseFUNC_TrackPos(e, p)?;
        }
    }
    Ok(iStack_40)
}

/// 0x00013460
pub fn Poltergeist_BaseFUNC_Path_NeedPathFinding(e: &mut Engine, param_1: u32) -> LResult<u32> {
    if e.r32(L(e) + 0x1a4) == 0 {
        Ok(0)
    } else {
        let p = L(e) + 0x1e4;
        Ok(LIBApiPathFinding::NeedPathFinding(e, param_1, p as i32) as u32)
    }
}

/// 0x000134e8
pub fn Poltergeist_BaseFUNC_Path_IsOnPath(e: &mut Engine, param_1: u32) -> LResult<u32> {
    if e.r32(L(e) + 0x1a4) == 0 {
        Ok(0)
    } else {
        let p = L(e) + 0x1e4;
        Ok(LIBApiPathFinding::IsOnPath(e, param_1, p as i32) as u32)
    }
}

/// 0x00013570 external 0xc
pub fn Monster_BaseCB_NOTIFY_COLLIDE_OBJ(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

/// 0x000135a0 external 8
pub fn Monster_BaseST_IN_CUT_LOOP(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

/// 0x00013668 external state 0: clear the sub-state, notifications (0x14 pre-logic -> 0x15, 0xb anim
/// event -> 0x12, 1 anim done -> 0x14), restart at Poltergeist_baseST_Initial (0x10)
pub fn Monster_BaseST_REINITIALISE(e: &mut Engine, _: u32) -> LResult<u64> {
    let l = L(e);
    e.w32(l + 0x160, 0);
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 0x14, 0x15);
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 0xb, 0x12);
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 1, 0x14);
    APIgameobj::SetNextState(e, SELF, 0x10);
    Ok(0)
}

//! Monster_Base.c (24 functions): the code every monster module shares (capture by the book, speed
//! profiles, line-of-sight setup, release after a capture). Addresses are module-relative (.text at
//! 0x10000) in poltergeist__0bb93729. The poltergeist copy has neither IsTomeOpened nor
//! CheckIfInSector; context flag 0x20 when pulled in (0x67). Monster_BaseST_REINITIALISE,
//! Monster_BaseST_IN_CUT_LOOP and Monster_BaseCB_NOTIFY_COLLIDE_OBJ live in PoltergeistII_base.c.
use super::builds::*;
use super::*;
use crate::engine::*;
use crate::logic::LResult;

/// 0x00011080 copy this monster type's record of TmpMonsterMap (matched on L+0x44) into the local
/// data, pick the capture weight class (L+0x58) from the type, create the sucking DM.
pub fn Monster_Base_Func_Initialise(e: &mut Engine) -> LResult<()> {
    let iVar1 = LM(e);
    let puVar2 = LM(e) + 0x80;
    let p = LM(e) + 0xf8;
    APIgameobj::GetPosition(e, SELF, ALL, p);
    // shape group 1 (b8041da2, one record): the loop is a single pass over record 0
    let n = if g(e, G_Monster_Base_Func_Initialise) == 1 { 1 } else { k(e, Monster_Base_Func_Initialise_0_1) };
    let map = G(e, TmpMonsterMap);
    let mut uStack_3c = 0u32;
    while uStack_3c < n {
        let r = map + uStack_3c * 0x3c;
        if e.r32(r) == e.r32(LM(e) + 0x44) {
            for (dst, src) in [
                (0xa8, 0x2c), (0xac, 0x30), (0xb0, 0x34), (0xb4, 0x38), (0xa4, 0x28), (0x90, 0x14),
                (0xa0, 0x24), (0x8c, 0x10), (0x9c, 0x20), (0x88, 0xc), (0x94, 0x18),
            ] {
                let v = e.r32(r + src);
                e.w32(iVar1 + dst, v);
            }
            let v = e.r32(r + 4);
            e.w32(puVar2, v);
            let v = e.r32(r + 0x1c);
            e.w32(iVar1 + 0x98, v);
            let v = e.r32(r + 8);
            e.w32(iVar1 + 0x84, v);
            let class = match e.r32(r) {
                1 => Some(1),
                2 | 4 | 6 | 7 | 8 | 9 | 0xc => Some(2),
                3 | 5 | 10 => Some(3),
                0xd..=0x12 => Some(99),
                0x13 => Some(6),
                _ => None,
            };
            if let Some(c) = class {
                e.w32(LM(e) + 0x58, c);
            }
        }
        uStack_3c += 1;
    }
    e.w32(LM(e) + 0x78, 0x4100_0000);
    e.w32(LM(e) + 0xec, 0x4100_0000);
    Monster_BaseFUNC_Create_Physic(e)
}

/// 0x00011500
pub fn Monster_BaseST_BASICMOVE_SET(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}
/// 0x00011530
pub fn Monster_BaseST_BASICMOVE_LOOP(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}
/// 0x00011560
pub fn Monster_BaseST_SCARED_SET(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}
/// 0x00011590
pub fn Monster_BaseST_SCARED_LOOP(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}
/// 0x000115c0
pub fn Monster_BaseST_BEINGSUCKED_SET(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}
/// 0x000115f0
pub fn Monster_BaseST_BEINGSUCKED_LOOP(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}
/// 0x00011620
pub fn Monster_BaseST_BLACK_HOLE_LOOP(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

/// 0x00011650 state 0xb (captured): unless a monster is already held (game buffer 0 +0x30), tell the
/// capture manager (msg 0x14e with our id), hide (clipping 4) and sleep forever.
pub fn Monster_BaseST_CAPTURED(e: &mut Engine, _: u32) -> LResult<u64> {
    if e.r32(e.r32(GDM(e)) + 0x30) == 0 {
        let uVar2 = APIgameobj::GetID(e, SELF, NULL);
        let to = k(e, ID_2a000050);
        APIgameobj::SendMessageTo(e, to, NULL, 0x14e, uVar2, NULL);
        LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 4, NULL, 0);
        APIgameobj::Sleep(e, SELF, -1.0);
    }
    Ok(0)
}

/// 0x00011738 timer L+0x11c: back to the saved anim / context / state.
pub fn Monster_BaseCB_NOTIFY_TIMER(e: &mut Engine, param_1: u32) -> LResult<u64> {
    if param_1 == e.r32(LM(e) + 0x11c) {
        let a = e.r32(LM(e) + 0x38) as i32;
        LIBAnimMngr::PlayAnim(e, 0.0, a, 2, 0);
        let f = e.r32(LM(e) + 0x54);
        APIgameobj::SetContextFlag(e, SELF, f);
        let s = e.r32(LM(e) + 0x34);
        APIgameobj::SetNextState(e, SELF, s);
    }
    Ok(0)
}

/// 0x00011808 the leader hero is within L+0x78 and not hiding/sneaking (game flags): remember his
/// position as the noise position (L+0x110).
pub fn DoIEarSomeNoise(e: &mut Engine) -> LResult<u32> {
    let r = e.rf(LM(e) + 0x78);
    let hero = e.r32(e.r32(GDM(e)) + 8);
    let bVar1 = LIBApiCFunctions::CheckRangeObject(e, r, SELF, hero, 0);
    let flags = |e: &Engine| e.r32(e.r32(GDM(e)) + 0x14);
    if bVar1 != 0 && flags(e) & 0x30 == 0 && flags(e) & 0x800_090c == 0 && flags(e) & 0x40 == 0 {
        let hero = e.r32(e.r32(GDM(e)) + 8);
        let p = LM(e) + 0x110;
        APIgameobj::GetPosition(e, hero, ALL, p);
        Ok(1)
    } else {
        Ok(0)
    }
}

/// 0x00011920 point the move DM (L+4) at the noise position, walk speed
pub fn SetPosOfNoise(e: &mut Engine) -> LResult<()> {
    let b = e.alloca(0x80);
    let dm = e.r32(LM(e) + 4);
    APIphysic::DMGetParams(e, dm, b);
    e.w32(b, 0);
    let l = LM(e);
    for (o, s) in [(8, 0x110), (0xc, 0x114), (0x10, 0x118), (4, 0x84)] {
        let v = e.r32(l + s);
        e.w32(b + o, v);
    }
    e.w32(b + 0x14, 5);
    APIphysic::DMSetParams(e, dm, b);
    Ok(())
}

/// 0x000119e0
pub fn SetClipping(e: &mut Engine) -> LResult<()> {
    let b = e.alloca(0x40);
    APIgameobj::GetClippingParams(e, SELF, b);
    for (o, v) in [
        (0, 0x3dcc_cccd), (4, 0x41c8_0000), (0x10, 0x4248_0000), (0x1c, 0xbf80_0000), (0xc, 0x1000_0000),
        (8, 0x1000_0047), (0x18, 0x40), (0x14, 0x6a7), (0x24, 0x6a7), (0x20, 0x6a7),
    ] {
        e.w32(b + o, v);
    }
    APIgameobj::SetClippingParams(e, SELF, b, 0);
    Ok(())
}

/// 0x00011ab0 clipping while held in the book
pub fn SetClippingCaptured(e: &mut Engine) -> LResult<()> {
    let b = e.alloca(0x40);
    APIgameobj::GetClippingParams(e, SELF, b);
    for (o, v) in [(0, 0x3dcc_cccd), (4, 0xbf80_0000), (0xc, 0x2bf), (8, 0x2bf)] {
        e.w32(b + o, v);
    }
    APIgameobj::SetClippingParams(e, SELF, b, 0);
    Ok(())
}

/// move DM (L+4) speed (+4) [and mode 5 (+0x14)], turn DM (L+8) speed (+8)
fn set_speed_params(e: &mut Engine, move_speed: u32, set_mode: bool, turn_speed: u32) {
    let b1 = e.alloca(0x80);
    let dm = e.r32(LM(e) + 4);
    APIphysic::DMGetParams(e, dm, b1);
    let v = e.r32(LM(e) + move_speed);
    e.w32(b1 + 4, v);
    if set_mode {
        e.w32(b1 + 0x14, 5);
    }
    APIphysic::DMSetParams(e, dm, b1);
    let b2 = e.alloca(0x80);
    let dm = e.r32(LM(e) + 8);
    APIphysic::DMGetParams(e, dm, b2);
    let v = e.r32(LM(e) + turn_speed);
    e.w32(b2 + 8, v);
    APIphysic::DMSetParams(e, dm, b2);
}

/// 0x00011b40
pub fn SetSneakSpeedParams(e: &mut Engine) -> LResult<()> {
    set_speed_params(e, 0x80, true, 0x94);
    Ok(())
}
/// 0x00011c20 (does not set the DM mode)
pub fn SetWalkSpeedParams(e: &mut Engine) -> LResult<()> {
    set_speed_params(e, 0x84, false, 0x98);
    Ok(())
}
/// 0x00011cf8
pub fn SetAttackSpeedParams(e: &mut Engine) -> LResult<()> {
    set_speed_params(e, 0x90, true, 0xa4);
    Ok(())
}
/// 0x00011dd8 speed when fleeing the open book / being pulled
pub fn SetRunFromTomeSpeedParams(e: &mut Engine) -> LResult<()> {
    set_speed_params(e, 0x8c, true, 0xa0);
    Ok(())
}

/// 0x00011eb8 the InLineOfSight parameter block at L+0xb8
pub fn InitInLineOfSight(e: &mut Engine) -> LResult<()> {
    let l = LM(e);
    let hero = e.r32(e.r32(GDM(e)) + 8);
    for (o, v) in [
        (0xb8, SELF), (0xbc, hero), (0xc0, 0xe007), (0xd8, 0x6e), (0xdc, 100), (0xe0, 0x84), (0xc4, 4),
        (200, 0), (0xcc, 2), (0xd0, 0), (0xd4, 0), (0xe4, 0), (0xe8, 0x3f00_0000), (0xec, 0x4100_0000),
        (0xf0, 0xbf86_0a92), (0xf4, 0x3f86_0a92),
    ] {
        e.w32(l + o, v);
    }
    Ok(())
}

/// 0x00011ff0 already captured in a previous visit (game buffer 2 +4 & L+0x40): kill self
pub fn AmIDead(e: &mut Engine) -> LResult<u32> {
    let m = e.r32(e.r32(GDM(e) + 4) + 4);
    if m == 0 || m & e.r32(LM(e) + 0x40) == 0 {
        Ok(0)
    } else {
        APIgameobj::KillObject(e, SELF);
        Ok(1)
    }
}

/// 0x00012088 DM type 4 at L+0xc (the book's pull), inactive until message 0xd6/0x6a
pub fn Monster_BaseFUNC_Create_Physic(e: &mut Engine) -> LResult<()> {
    let uVar1 = APIphysic::DMAdd(e, SELF, 4);
    e.w32(LM(e) + 0xc, uVar1);
    let b = e.alloca(0x80);
    let dm = e.r32(LM(e) + 0xc);
    APIphysic::DMGetParams(e, dm, b);
    for (o, v) in [(0, 0), (4, 0), (8, 0x3f80_0000), (0xc, 1)] {
        e.w32(b + o, v);
    }
    APIphysic::DMSetParams(e, dm, b);
    APIphysic::DMSetContextMask(e, dm, 0x80);
    APIphysic::DMSetActive(e, dm, 0);
    Ok(())
}

/// hero ids accepted by message 0xcd (re-target the move DM to the new leader): shape group 0 checks
/// two ids, group 1 (1c9d41ec, 7d96cf48, b8041da2) four
fn retarget_ids(e: &Engine) -> Vec<u32> {
    if g(e, G_Monster_Base_MessageFunction) == 0 {
        vec![k(e, ID_2a00004b), k(e, ID_2a00004c)]
    } else {
        vec![k(e, ID_2a00004b), k(e, Monster_Base_MessageFunction_1_1), k(e, Monster_Base_MessageFunction_1_2), k(e, ID_2a00004c)]
    }
}

/// 0x00012188 monster messages. 0xd6 = from the book: 100 link broken, 0x65 hooked (launch the
/// beam latent and pass it our capture parameters), 0x67 pulled in, 0x68 captured, 0x6a pull vector.
/// 0xcd leader changed. 0xc9/0xc1 (re)start: reset, or come back out of the book if held.
/// The builds differ only in literals (anims of 0x67, object ids) and the length of the 0xcd id list.
pub fn Monster_Base_MessageFunction(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    if param_1 != 0xc9 {
        if 0xc9 < param_1 {
            if param_1 == 0xd4 {
                APIgameobj::SetGroupID(e, SELF, 0);
                APIgameobj::KillObject(e, SELF);
                return Ok(0);
            }
            if 0xd4 < param_1 {
                if param_1 != 0xd6 {
                    return Ok(0);
                }
                match param_2 {
                    100 => {
                        let v = e.r32(param_3);
                        e.w32(LM(e) + 0x5c, v);
                        let to = e.r32(LM(e) + 0x3c);
                        APIgameobj::SendMessageTo(e, to, NULL, 0xd6, 100, NULL);
                    }
                    0x65 => {
                        let uStack_80 = e.r32(param_3);
                        let id = k(e, ID_2a8000cc);
                        let uVar2 = APIgameobj::LaunchLatant(e, id);
                        e.w32(LM(e) + 0x3c, uVar2);
                        let child = e.r32(LM(e) + 0x3c);
                        APIgameobj::AddChild(e, SELF, ALL, child, 1);
                        let to = e.r32(LM(e) + 0x3c);
                        APIgameobj::SendMessageTo(e, to, NULL, 0xd6, 0, NULL);
                        let b = e.alloca(0x1c);
                        let l = LM(e);
                        for (o, s) in [(0, 0xb0), (8, 0xb4), (4, 0x5c), (0xc, 0x68), (0x10, 0x6c), (0x14, 0x3c), (0x18, 0x58)] {
                            let v = e.r32(l + s);
                            e.w32(b + o, v);
                        }
                        let to = e.r32(LM(e) + 0x3c);
                        APIgameobj::SendMessageTo(e, to, NULL, 0xec, 0, b);
                        APIgameobj::SendMessageTo(e, uStack_80, NULL, 0xd6, 0x66, b);
                    }
                    0x67 => {
                        APIgameobj::SetContextFlag(e, SELF, 0x20);
                        let book = k(e, ID_2a00004d);
                        LIBApiCFunctions::TrackID(e, 3.1415927, SELF as i32, book);
                        APIanim::BeginTree(e, SELF);
                        let a = k(e, Monster_Base_MessageFunction_0_1);
                        APIanim::Play(e, SELF, a, 1, 0.0);
                        let b = e.alloca(0x14);
                        let v = e.r32(param_3);
                        for (o, w) in [(0, 0xbb00_0001), (4, 0), (0xc, 0), (8, v), (0x10, 0x3f00_0000)] {
                            e.w32(b + o, w);
                        }
                        let (a1, a2) = (k(e, Monster_Base_MessageFunction_0_2), k(e, Monster_Base_MessageFunction_0_3));
                        APIanim::PlayBlendConnect(e, SELF, a1, a2, 1, b);
                        APIanim::EndTree(e, SELF);
                        APIgameobj::SetNextState(e, SELF, 5);
                    }
                    0x68 => {
                        let uStack_80 = APIgameobj::GetID(e, SELF, NULL);
                        let to = e.r32(LM(e) + 0x3c);
                        APIgameobj::SendMessageTo(e, to, NULL, 0xd6, 100, NULL);
                        APIgameobj::SetContextFlag(e, SELF, 0x60);
                        let p = LM(e) + 0xf8;
                        APIgameobj::SetTranslation(e, SELF, p);
                        let gb2 = e.r32(GDM(e) + 4);
                        let m = e.r32(LM(e) + 0x40);
                        or32(e, gb2 + 4, m);
                        let to = k(e, Monster_Base_MessageFunction_0_4);
                        let p = LM(e) + 0x44;
                        APIgameobj::SendMessageTo(e, to, NULL, 0x26, 0, p);
                        APIgameobj::UnregisterNotify(e, SELF, 0xffff_ffff, 0xc);
                        let uVar2 = APIgameobj::GetGroupBitMask(e, SELF);
                        e.w32(LM(e) + 0x4c, uVar2);
                        APIgameobj::SetGroupBitMask(e, SELF, 0);
                        let uVar2 = APIgameobj::GetGroupID(e, SELF);
                        e.w32(LM(e) + 0x48, uVar2);
                        APIgameobj::SetGroupID(e, SELF, 6);
                        let to = k(e, ID_2a0000d3);
                        APIgameobj::SendMessageTo(e, to, NULL, 0xea, uStack_80, NULL);
                        let p = LM(e) + 0x120;
                        APIgameobj::GetClippingParams(e, SELF, p);
                        SetClippingCaptured(e)?;
                        APIgameobj::SetNextState(e, SELF, 0xb);
                    }
                    0x6a => {
                        let b = e.alloca(0x80);
                        let dm = e.r32(LM(e) + 0xc);
                        APIphysic::DMGetParams(e, dm, b);
                        for o in [8, 4, 0] {
                            let v = e.r32(param_3 + o);
                            e.w32(b + o, v);
                        }
                        APIphysic::DMSetParams(e, dm, b);
                        APIphysic::DMSetActive(e, dm, 0xffff_ffff);
                        let bVar1 = APIphysic::DMIsActive(e, dm);
                        if bVar1 == 0 {
                            for m in [
                                "Monster -> ERREUR ERREUR ERREUR ERREUR ERREUR\n",
                                "Monster -> DM Move Sucking Effect Is Inactive\n",
                                "Monster -> ERREUR ERREUR ERREUR ERREUR ERREUR\n",
                            ] {
                                let s = e.cstr(m);
                                APIextra::null_printf(e, s, 0);
                            }
                        }
                    }
                    _ => {}
                }
                return Ok(0);
            }
            if param_1 != 0xcd {
                return Ok(0);
            }
            InitInLineOfSight(e)?;
            let b = e.alloca(0x80);
            let dm = e.r32(LM(e) + 4);
            APIphysic::DMGetParams(e, dm, b);
            if !retarget_ids(e).contains(&e.r32(b)) {
                return Ok(0);
            }
            let hero = e.r32(e.r32(GDM(e)) + 8);
            e.w32(b, hero);
            APIphysic::DMSetParams(e, dm, b);
            return Ok(0);
        }
        if param_1 == 0x51 || param_1 < 0x52 || param_1 != 0xc1 {
            return Ok(0);
        }
    }
    let held = |e: &Engine| e.r32(e.r32(GDM(e) + 4) + 4) & e.r32(LM(e) + 0x40);
    if held(e) == 0 {
        APIgameobj::SetNextState(e, SELF, 0);
    } else if held(e) != 0 {
        let p = LM(e) + 0xf8;
        APIgameobj::SetTranslation(e, SELF, p);
        APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 3, 0xc);
        let gb2 = e.r32(GDM(e) + 4);
        let m = e.r32(gb2 + 4) & !e.r32(LM(e) + 0x40);
        e.w32(gb2 + 4, m);
        let to = k(e, Monster_Base_MessageFunction_0_5);
        let p = LM(e) + 0x44;
        APIgameobj::SendMessageTo(e, to, NULL, 0x26, 1, p);
        let v = e.r32(LM(e) + 0x4c);
        APIgameobj::SetGroupBitMask(e, SELF, v);
        let to = k(e, ID_2a0000d3);
        APIgameobj::SendMessageTo(e, to, NULL, 0xeb, 1, NULL);
        let v = e.r32(LM(e) + 0x48);
        APIgameobj::SetGroupID(e, SELF, v);
        let p = LM(e) + 0x120;
        APIgameobj::SetClippingParams(e, SELF, p, 0);
        APIgameobj::Sleep(e, SELF, 0.0);
        APIgameobj::SetNextState(e, SELF, 0);
    }
    Ok(0)
}

/// 0x00012bc8
pub fn monster_baseConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    if instanceCount == 1 {
        let fns: [(u32, crate::logic::ExtFn); 12] = [
            (1, Monster_BaseST_BASICMOVE_SET),
            (2, Monster_BaseST_BASICMOVE_LOOP),
            (3, Monster_BaseST_SCARED_SET),
            (4, Monster_BaseST_SCARED_LOOP),
            (5, Monster_BaseST_BLACK_HOLE_LOOP),
            (6, Monster_BaseST_BEINGSUCKED_SET),
            (7, Monster_BaseST_BEINGSUCKED_LOOP),
            (8, Monster_BaseST_IN_CUT_LOOP),
            (0, Monster_BaseST_REINITIALISE),
            (0xb, Monster_BaseST_CAPTURED),
            (0xc, Monster_BaseCB_NOTIFY_COLLIDE_OBJ),
            (0xd, Monster_BaseCB_NOTIFY_TIMER),
        ];
        for (i, f) in fns {
            APIcons::SetExternalFunction(e, i as i32, f);
        }
        let tables: [(u32, fn(&mut Engine) -> Ptr); 4] = [
            (pAnim, |e| APIcons::GetApiAnim(e)),
            (pGameObj, |e| APIcons::GetApiGameObj(e)),
            (pExtra, |e| APIcons::GetApiExtra(e)),
            (pPhysic, |e| APIcons::GetApiPhysic(e)),
        ];
        for (slot, get) in tables {
            let v = get(e);
            let a = G(e, slot);
            e.w32(a, v);
        }
        for (slot, name) in [(pAnimMngr, "AnimMngr"), (pFunction, "ApiCFunctions"), (pMiscFunc, "MiscFunc"), (pDebug, "debug")] {
            let n = e.cstr(name);
            let v = APIcons::GetApiCustom(e, n);
            let a = G(e, slot);
            e.w32(a, v);
        }
        let slot = G(e, pGlobalDataMonster_Base);
        APIcons::GetGlobalData(e, slot, 0xc);
        let uVar1 = APIgameobj::GetGameBuffer(e, 0, 0x8f8, 1);
        let gd = GDM(e);
        e.w32(gd, uVar1);
        let uVar1 = APIgameobj::GetGameBuffer(e, 1, 0x68, 1);
        let gd = GDM(e);
        e.w32(gd + 8, uVar1);
    }
    APIcons::SetMessageFunction(e, Monster_Base_MessageFunction);
    let slot = G(e, pLocalDataMonster_Base);
    APIcons::GetLocalData(e, slot, 0x160);
    SetClipping(e)
}

/// 0x00012ef8
pub fn monster_baseDestructor(e: &mut Engine) -> LResult<()> {
    Ok(())
}

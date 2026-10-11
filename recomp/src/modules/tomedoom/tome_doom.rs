//! TomeDoom.c (50 functions). Addresses are module-relative (.text at 0x10000) in the reference build.
//!
//! Float arithmetic: f32 with IEEE round-to-nearest. The PS2 FPU rounds toward zero and has no
//! denormals/infinities, so results can differ from the console in the last bit (no logic depends on it).
use super::builds::*;
use super::*;
use crate::engine::*;
use crate::logic::LResult;

/// `0x7fffffff` "no bone / whole object" argument of GetPosition/TakeOrientation/GetBoneLMat
const ALL: u32 = 0x7fff_ffff;

/// 0x00013178 state 0: first frame of the book
pub fn TomeDoomST_FIRSTMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    let iVar1 = L(e);
    e.w32(L(e) + 0x68, 0);
    LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 2, NULL, 0);
    e.w32(iVar1 + 0x1c8, 1);
    if e.r32(e.r32(GD(e) + 8)) & 1 != 0 {
        let v = e.r32(e.r32(GD(e) + 4) + 4);
        e.w32(L(e) + 0x250, v);
        APIgameobj::SetGroupID(e, SELF, 6);
        APIgameobj::SetGameObjectVisible(e, SELF, 0);
        APIgameobj::EnableObject(e, SELF, 0);
    }
    APIgameobj::SetNextState(e, SELF, 1);
    Ok(0)
}

/// 0x000132a0 state 1: book put away
pub fn TomeDoomST_BASICMOVE(e: &mut Engine, _: u32) -> LResult<u64> {
    APIgameobj::EnableObject(e, SELF, 0);
    e.w32(L(e) + 0x68, 1);
    Ok(0)
}

/// 0x00013308 state 2: the book is being opened (first frame only: reset, play the opening anim,
/// put the book in front of the hero holding it)
pub fn TomeDoomST_ISOPENING(e: &mut Engine, _: u32) -> LResult<u64> {
    let auStack_90 = e.alloca(0x30); // _ShadowParams
    let DStack_60 = e.alloca(0x10);
    let DStack_50 = e.alloca(0x10);
    let uStack_40 = e.alloca(0x40); // DM params (Ghidra shows the first two words)
    let iVar1 = L(e);
    if e.ri32(L(e) + 0x68) != 2 {
        for v in [cvVelocity, cvWallNormal, cvOldSlaveVelo, cvOldMasterVelo, cvMove] {
            let a = G(e, v);
            e.wv3(a, [0.0; 3]);
        }
        e.w32(G(e, bCollision), 0);
        let uVar2 = APIgameobj::GetCurrentStateID(e, SELF);
        e.w32(iVar1 + 0x68, uVar2);
        APIgameobj::SetGameObjectVisible(e, SELF, 1);
        APIinput::InputConfigActivate(e, 3, 1);
        e.w32(iVar1 + 100, 0x1000);
        let s = e.ri32(iVar1 + 100);
        APIinput::InputConfigSetState(e, 3, s);
        APIgameobj::GetShadowParams(e, SELF, auStack_90);
        let f = e.r32(auStack_90) & 0xffff_fffb;
        e.w32(auStack_90, f);
        APIgameobj::SetShadowParams(e, SELF, auStack_90);
        if e.ri32(iVar1 + 0x60) == 0 {
            APIgameobj::SetContextFlag(e, SELF, 0x103);
        } else {
            let dm = e.r32(iVar1 + 0xc);
            APIphysic::DMGetParams(e, dm, uStack_40);
            let v = e.r32(iVar1 + 0x60);
            e.w32(uStack_40, v);
            e.w32(uStack_40 + 4, 0x40a0_0000); // 5.0f
            let dm = e.r32(iVar1 + 0xc);
            APIphysic::DMSetParams(e, dm, uStack_40);
            let dm = e.r32(iVar1 + 0xc);
            APIphysic::DMSetActive(e, dm, 1);
            APIgameobj::SetContextFlag(e, SELF, 0x103);
        }
        let a = e.ri32(iVar1 + 0x230);
        let iVar3 = LIBAnimMngr::PlayAnim(e, 0.0, a, 1, 6);
        e.wi32(iVar1 + 0x5c, iVar3);
        let hero = e.r32(iVar1 + 0x1b8);
        APIgameobj::TakeOrientation(e, SELF, hero, ALL);
        APIgameobj::GetPosition(e, hero, ALL, DStack_60);
        let off = [e.rf(iVar1 + 0x240), e.rf(iVar1 + 0x244), e.rf(iVar1 + 0x248)];
        e.wv3(DStack_50, off);
        let hero = e.r32(iVar1 + 0x1b8);
        APIgameobj::TransformVector(e, hero, DStack_50, DStack_50);
        let (a, b) = (e.rv3(DStack_50), e.rv3(DStack_60));
        e.wv3(DStack_50, [a[0] + b[0], a[1] + b[1], a[2] + b[2]]);
        APIgameobj::SetTranslation(e, SELF, DStack_50);
        APIphysic::ResetFlags(e, SELF, 1);
        APIgameobj::SetContextFlag(e, SELF, 0);
        let hero = e.r32(iVar1 + 0x1b8);
        APIphysic::SetVelocity(e, hero, 0.0, 0.0, 0.0);
        APIphysic::SetVelocity(e, SELF, 0.0, 0.0, 0.0);
        if 0.0 < e.rf(e.r32(GD(e) + 4) + 4) {
            APInewSound::Play(e, SELF, 0x2e, 0);
        }
    }
    Ok(0)
}

/// 0x00013760 state 5: the book is being closed (first frame only)
pub fn TomeDoomST_ISCLOSING(e: &mut Engine, _: u32) -> LResult<u64> {
    let uStack_70 = e.alloca(0x10); // CColor
    let auStack_60 = e.alloca(0x30); // _ShadowParams
    let iVar1 = L(e);
    if e.ri32(L(e) + 0x68) != 5 {
        let uVar2 = APIgameobj::GetCurrentStateID(e, SELF);
        e.w32(iVar1 + 0x68, uVar2);
        e.w32(iVar1 + 0x1dc, 0);
        e.w32(GB0(e) + 0x30, 0);
        e.w32(iVar1 + 0x60, 0);
        TomeDoomFUNC_CancelScanForWisps(e)?;
        e.w32(iVar1 + 0x15c, 0);
        let a = e.ri32(iVar1 + 0x234);
        let iVar3 = LIBAnimMngr::PlayAnim(e, 0.0, a, 1, 6);
        e.wi32(iVar1 + 0x5c, iVar3);
        if e.ri32(iVar1 + 0x2c) != 0 {
            for i in 0..4 {
                e.w32(uStack_70 + i * 4, 0);
            }
            let l = e.r32(iVar1 + 0x2c);
            APIlight::SetFlag(e, l, 3);
            let l = e.r32(iVar1 + 0x2c);
            APIlight::SetColor(e, l, uStack_70);
            APIgameobj::GetShadowParams(e, SELF, auStack_60);
            let f = e.r32(auStack_60) | 4;
            e.w32(auStack_60, f);
            APIgameobj::SetShadowParams(e, SELF, auStack_60);
        }
        APInewSound::StopSound(e, SELF, 0xf);
        let o = k(e, ID_2a000096);
        APIgameobj::SendMessageTo(e, o, NULL, 0x5c, 1, NULL);
        e.w32(iVar1 + 0x1b4, 0);
        APIgameobj::SetContextFlag(e, SELF, 0);
        let dm = e.r32(iVar1 + 0x18);
        APIphysic::DMResetStateBits(e, dm, 1);
        APIinput::InputConfigActivate(e, 3, 0);
        APIphysic::SetVelocity(e, SELF, 0.0, 0.0, 0.0);
        APIphysic::SetFlags(e, SELF, 1);
        APIgameobj::RemoveAllChildren(e, SELF, 0);
        APInewSound::StopSound(e, SELF, 0x2e);
        TomeDoomFUNC_Remove_NTAction(e, iVar1)?;
        TomeDoomFUNC_FixHerosMatrice(e)?;
    }
    Ok(0)
}

/// 0x00013a40 "fix so we don't fall into the wall": moves the hero(es) holding the book along with the
/// book's hook joints, honouring wall collisions; tells the heroes whether they are being dragged.
pub fn FixPourNePasTomberDansLeMur(e: &mut Engine) -> LResult<u32> {
    let DStack_b0 = e.alloca(0x10);
    let DStack_a0 = e.alloca(0x10);
    let DStack_90 = e.alloca(0x10);
    let DStack_80 = e.alloca(0x10);
    let DStack_50 = e.alloca(0x10);
    let DStack_40 = e.alloca(0x10);
    // Ghidra: `in_f0` = the return value of GetFrameDeltaTime
    let in_f0 = APIgame::GetFrameDeltaTime(e);
    let mut fStack_58: f32 = 0.0;
    let fStack_60 = in_f0;
    let hero = e.r32(L(e) + 0x1b8);
    APIgameobj::GetPosition(e, hero, ALL, DStack_90);
    e.wv3(DStack_a0, [0.0; 3]);
    APIphysic::SetVelocity(e, SELF, 0.0, 0.0, 0.0);
    let j = e.r32(G(e, ui32JointHookMasterID));
    let pDVar1 = APIgameobj::GetBoneWMat(e, SELF, j);
    APImath::CMatGetPos(e, DStack_b0, pDVar1);
    let mut f = e.rv3(G(e, cvVelocity)); // fStack_70/6c/68
    let uVar2 = APIgameobj::GetContextFlag(e, SELF);
    if uVar2 & 0x80 != 0 {
        let mv = e.rv3(G(e, cvMove));
        let fVar4 = mv[0] * 0.6;
        let fVar5 = mv[1] * 0.6;
        let fVar6 = mv[2] * 0.6;
        let a = G(e, cvMove);
        e.wv3(a, [mv[0] - fVar4, mv[1] - fVar5, mv[2] - fVar6]);
        let jd = e.rv3(G(e, cvJoyDep));
        let d50 = [jd[0] * fStack_60, jd[1] * fStack_60, jd[2] * fStack_60];
        f = [f[0] + fVar4 + d50[0], f[1] + fVar5 + d50[1], f[2] + fVar6 + d50[2]];
        let a = G(e, cvJoyDep);
        e.wv3(a, [0.0; 3]);
    }
    if e.r32(G(e, bCollision)) != 0 {
        let wn = e.rv3(G(e, cvWallNormal));
        if wn[0] * f[0] + wn[1] * f[1] + wn[2] * f[2] < 0.0 {
            f = [0.0; 3];
        }
        e.w32(G(e, bCollision), 0);
    }
    let b0 = e.rv3(DStack_b0);
    e.wv3(DStack_b0, [b0[0] + f[0], b0[1] + f[1], b0[2] + f[2]]);
    let a = G(e, cvVelocity);
    e.wv3(a, [0.0; 3]);
    if e.ri32(L(e) + 0x1c0) != 0 {
        let other = e.r32(GB0(e) + 0xc);
        APIgameobj::GetPosition(e, other, ALL, DStack_80);
        let j = e.r32(G(e, ui32JointHookSlaveID));
        let pDVar1 = APIgameobj::GetBoneWMat(e, SELF, j);
        APImath::CMatGetPos(e, DStack_a0, pDVar1);
        let mut a0 = e.rv3(DStack_a0);
        a0 = [a0[0] + f[0], a0[1] + f[1], a0[2] + f[2]];
        let old = e.rv3(G(e, cvOldSlaveVelo));
        let d80 = e.rv3(DStack_80);
        if old[0] != 0.0 || old[1] != 0.0 || old[2] != 0.0 {
            a0[0] = a0[0] + (d80[0] - a0[0]) * 0.4;
            a0[1] = a0[1] + 0.0;
            a0[2] = a0[2] + (d80[2] - a0[2]) * 0.4;
        }
        let a = G(e, cvOldSlaveVelo);
        e.wv3(a, a0);
        a0 = [a0[0] - d80[0], a0[1] - d80[1], a0[2] - d80[2]];
        fStack_58 = a0[0] * a0[0] + a0[1] * a0[1] + a0[2] * a0[2];
        a0 = [a0[0] * (1.0 / fStack_60), a0[1] * (1.0 / fStack_60), a0[2] * (1.0 / fStack_60)];
        e.wv3(DStack_a0, a0);
        let other = e.r32(GB0(e) + 0xc);
        APIphysic::GetVelocity(e, other, DStack_50);
        if e.rf(DStack_50 + 4) < -5.8 {
            let h = e.r32(GB0(e) + 8);
            APIgameobj::SendMessageTo(e, h, NULL, 0x92, 0, NULL);
        }
        let other = e.r32(GB0(e) + 0xc);
        let vy = e.rf(DStack_50 + 4);
        APIphysic::SetVelocity(e, other, a0[0], vy, a0[2]);
        let other = e.r32(GB0(e) + 0xc);
        APIgameobj::TakeOrientation(e, other, SELF, ALL);
    }
    let (b0, d90) = (e.rv3(DStack_b0), e.rv3(DStack_90));
    let mut b0 = [b0[0] - d90[0], b0[1] - d90[1], b0[2] - d90[2]];
    let fStack_5c = b0[0] * b0[0] + b0[1] * b0[1] + b0[2] * b0[2];
    b0 = [b0[0] * (1.0 / fStack_60), b0[1] * (1.0 / fStack_60), b0[2] * (1.0 / fStack_60)];
    e.wv3(DStack_b0, b0);
    let hero = e.r32(L(e) + 0x1b8);
    APIphysic::GetVelocity(e, hero, DStack_40);
    if e.rf(DStack_40 + 4) < -5.8 {
        let h = e.r32(GB0(e) + 8);
        APIgameobj::SendMessageTo(e, h, NULL, 0x92, 0, NULL);
    }
    let hero = e.r32(L(e) + 0x1b8);
    let vy = e.rf(DStack_40 + 4);
    APIphysic::SetVelocity(e, hero, b0[0], vy, b0[2]);
    e.wf(DStack_a0 + 4, 0.0);
    e.wf(DStack_b0 + 4, 0.0);
    let dragged = (0.001 < fStack_5c || 0.001 < fStack_58) as u32;
    let hero = e.r32(L(e) + 0x1b8);
    APIgameobj::SendMessageTo(e, hero, NULL, 0x6c, dragged, NULL);
    if e.ri32(L(e) + 0x1c0) != 0 {
        let other = e.r32(GB0(e) + 0xc);
        APIgameobj::SendMessageTo(e, other, NULL, 0x6c, dragged, NULL);
    }
    let hero = e.r32(L(e) + 0x1b8);
    Ok(APIgameobj::TakeOrientation(e, hero, SELF, ALL))
}

/// 0x00014410 state 3: the book is open (0x53 = sucking a monster in / button mashing,
/// 0x54 = scanning for a monster to capture)
pub fn TomeDoomST_OPEN(e: &mut Engine, _: u32) -> LResult<u64> {
    let iVar1 = L(e);
    if e.ri32(L(e) + 0x58) == 0x53 {
        TomeDoomFUNC_Update_NTAction(e, L(e))?;
        if e.ri32(iVar1 + 0x1e0) != 0 {
            let uVar3 = e.r32(iVar1 + 0x1e0) & 0xe;
            let t = e.r32(iVar1 + 0x4c);
            APIextra::TimerReset(e, t);
            let t = e.r32(iVar1 + 0x48);
            if uVar3 != e.r32(iVar1 + 0x1e8) && 1.5 < APIextra::TimerGetTime(e, t) {
                let m = e.r32(iVar1 + 0x60);
                let iVar2 = APIgameobj::GetID(e, m, NULL);
                if iVar2 != 0x7fff_fffe && 0.0 < e.rf(iVar1 + 0x70) {
                    let m = e.r32(iVar1 + 0x60);
                    APIgameobj::SendMessageTo(e, m, NULL, 0xd6, 100, iVar1 + 0xcc);
                }
                e.w32(GB0(e) + 0x30, 0);
                e.w32(iVar1 + 0x60, 0);
                TomeDoomFUNC_ShaggyTomeOut(e)?;
                APIgameobj::SetNextState(e, SELF, 5);
                return Ok(0);
            }
            if uVar3 == e.r32(iVar1 + 0x1e8) && TomeDoomFUNC_DrainMonsterLife(e, iVar1)? != 0 {
                return Ok(0);
            }
        }
        let t40 = e.r32(iVar1 + 0x40);
        let t4c = e.r32(iVar1 + 0x4c);
        let give_up = 2.0 < APIextra::TimerGetTime(e, t40)
            || 3.0 < APIextra::TimerGetTime(e, t4c)
            || {
                let m = e.r32(iVar1 + 0x60);
                APIgameobj::GetID(e, m, NULL) == 0x7fff_fffe
            };
        if give_up {
            let dm = e.r32(iVar1 + 0xc);
            APIphysic::DMSetActive(e, dm, 0);
            let m = e.r32(iVar1 + 0x60);
            let iVar2 = APIgameobj::GetID(e, m, NULL);
            if iVar2 != 0x7fff_fffe && 0.0 < e.rf(iVar1 + 0x70) {
                let m = e.r32(iVar1 + 0x60);
                APIgameobj::SendMessageTo(e, m, NULL, 0xd6, 100, iVar1 + 0xcc);
            }
            e.w32(GB0(e) + 0x30, 0);
            e.w32(iVar1 + 0x60, 0);
            TomeDoomFUNC_ShaggyTomeOut(e)?;
            APIgameobj::SetNextState(e, SELF, 5);
        } else {
            let t = e.r32(iVar1 + 0x38);
            if 0.2 < APIextra::TimerGetTime(e, t) {
                let t = e.r32(iVar1 + 0x38);
                APIextra::TimerReset(e, t);
                TomeDoomFUNC_ContactWisps(e)?;
            }
            TomeDoomFUNC_UpdatePosition(e)?;
            TomeDoomFUNC_UpdateRayIntersect(e)?;
            TomeDoomFUNC_UpdateOrientationLink(e)?;
            TomeDoomFUNC_Update_Sound_Pitch(e, iVar1, 0xf)?;
            if e.ri32(iVar1 + 0x1b4) == 0 {
                let h = e.r32(GB0(e) + 8);
                if APIgameobj::SendMessageTo(e, h, NULL, 0x6d, 1, NULL) != 0 {
                    e.w32(iVar1 + 0x1b4, 1);
                }
            }
            TomeDoomFUNC_PullingEffect(e)?;
            if e.ri32(iVar1 + 0x154) == 0 {
                let t = e.r32(iVar1 + 0x40);
                APIextra::TimerReset(e, t);
            }
        }
    } else if e.ri32(L(e) + 0x58) == 0x54 {
        let t = e.r32(L(e) + 0x38);
        if 0.2 < APIextra::TimerGetTime(e, t) {
            TomeDoomFUNC_ContactWisps(e)?;
        }
        if 0.0 < e.rf(e.r32(GD(e) + 4) + 4) && {
            let t = e.r32(iVar1 + 0x38);
            0.2 < APIextra::TimerGetTime(e, t)
        } {
            let t = e.r32(iVar1 + 0x38);
            APIextra::TimerReset(e, t);
            let iVar2 = TomeDoomFUNC_ScanForMonster(e, 0, 0xe007)?;
            e.w32(GB0(e) + 0x30, iVar2);
            e.w32(iVar1 + 0x60, iVar2);
            if e.ri32(iVar1 + 0x60) != 0 {
                TomeDoomFUNC_SET_CAPTURE(e, iVar1)?;
            }
        }
    }
    FixPourNePasTomberDansLeMur(e)?;
    Ok(0)
}

/// 0x00014a18 state 4: final sucking of the captured monster into the book
pub fn TomeDoomST_FINALSUCKING(e: &mut Engine, _: u32) -> LResult<u64> {
    let DStack_50 = e.alloca(0x10);
    let afStack_40 = e.alloca(0x10);
    let iVar1 = L(e);
    if e.ri32(L(e) + 0x68) != 4 {
        let uVar2 = APIgameobj::GetCurrentStateID(e, SELF);
        e.w32(iVar1 + 0x68, uVar2);
        APImath::CVectorScale(e, DStack_50, iVar1 + 0x11c, 1.0);
        e.wf(DStack_50 + 4, 0.0);
        let fVar3 = APImath::CVectorLength(e, iVar1 + 0x11c);
        let mut f = (fVar3 - 0.75) / 4.85;
        if 1.0 < f {
            f = 1.0;
        } else if f < 0.0 {
            f = 0.0;
        }
        e.wf(afStack_40, f);
        let fVar3 = TomeDoomFUNC_UpdateBlendFactor(e)?;
        e.wf(iVar1 + 0xec, fVar3);
        let o = e.r32(iVar1 + 0x54);
        APIgameobj::SendMessageTo(e, o, NULL, 0xd6, 0x67, iVar1 + 0xe4);
        let m = e.r32(iVar1 + 0x60);
        APIgameobj::SendMessageTo(e, m, NULL, 0xd6, 0x6c, iVar1 + 0x54);
        let m = e.r32(iVar1 + 0x60);
        APIgameobj::SendMessageTo(e, m, NULL, 0xd6, 0x67, afStack_40);
        let dm = e.r32(iVar1 + 0x18);
        APIphysic::DMResetStateChangeNotifyBits(e, dm, 1);
        e.w32(iVar1 + 100, 0x1000);
        let s = e.ri32(iVar1 + 100);
        APIinput::InputConfigSetState(e, 3, s);
        APIgameobj::SetContextFlag(e, SELF, 0x103);
        APIphysic::SetVelocity(e, SELF, 0.0, 0.0, 0.0);
        let hero = e.r32(L(e) + 0x1b8);
        APIgameobj::SendMessageTo(e, hero, NULL, 0x6c, 0, NULL);
        let other = e.r32(GB0(e) + 0xc);
        APIgameobj::SendMessageTo(e, other, NULL, 0x6c, 0, NULL);
        let hero = e.r32(L(e) + 0x1b8);
        APIphysic::SetVelocity(e, hero, 0.0, 0.0, 0.0);
        let other = e.r32(GB0(e) + 0xc);
        APIphysic::SetVelocity(e, other, 0.0, 0.0, 0.0);
    }
    let fVar3 = APIextra::TimerGetTime(e, 0x9000_0000);
    if e.rf(iVar1 + 0x24c) < fVar3 {
        let fVar3 = APIextra::TimerGetTime(e, 0x9000_0000);
        e.wf(iVar1 + 0x24c, fVar3 + 0.25);
        TomeDoomFUNC_CallVibration(e, 1.0, 0.0, 0.25, 0.25)?;
    }
    Ok(0)
}

/// 0x00014de0 start capturing the monster `L+0x60`: hands out of the book (latent 0x2a800095), sounds,
/// light, button-mash prompt, dynamic link (DM) between the book and the monster
pub fn TomeDoomFUNC_SET_CAPTURE(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    let uStack_60 = e.alloca(0x20); // DM params (frame room between uStack_60 and auStack_40)
    let auStack_40 = e.alloca(0x10);
    let id = APIgameobj::GetID(e, SELF, NULL);
    e.w32(auStack_40, id);
    let t = e.r32(param_1 + 0x44);
    APIextra::TimerSetFrequency(e, t, 0.051);
    let t = e.r32(param_1 + 0x44);
    APIextra::TimerRegisterNotify(e, SELF, t, 0x13);
    // shape group 1 (build 4a201eb9) has no light
    if g(e, G_TomeDoomFUNC_SET_CAPTURE) == 0 {
        TomeDoomFUNC_Create_Light(e, param_1)?;
    }
    e.w32(param_1 + 0x1cc, 0);
    let latent = k(e, ID_2a800095);
    let uVar1 = APIgameobj::LaunchLatant(e, latent);
    e.w32(param_1 + 0x54, uVar1);
    let c = e.r32(param_1 + 0x54);
    APIgameobj::AddChild(e, SELF, ALL, c, 1);
    APInewSound::Play(e, SELF, 0x14, 0);
    let o = k(e, ID_2a000096);
    APIgameobj::SendMessageTo(e, o, NULL, 0x5c, 2, NULL);
    APInewSound::Play(e, SELF, 0xf, 0);
    let m = e.r32(param_1 + 0x60);
    APIgameobj::SendMessageTo(e, m, NULL, 0xd6, 0x65, auStack_40);
    e.w32(param_1 + 0x1f8, 0);
    TomeDoomFUNC_GenerateButton(e, param_1)?;
    TomeDoomFUNC_Update_NTAction(e, param_1)?;
    let dm = e.r32(param_1 + 0xc);
    APIphysic::DMGetParams(e, dm, uStack_60);
    e.w32(uStack_60 + 4, 0x4120_0000); // 10.0f
    let v = e.r32(param_1 + 0x60);
    e.w32(uStack_60, v);
    let dm = e.r32(param_1 + 0xc);
    APIphysic::DMSetParams(e, dm, uStack_60);
    let dm = e.r32(param_1 + 0xc);
    APIphysic::DMSetActive(e, dm, 1);
    let dm = e.r32(param_1 + 0x18);
    APIphysic::DMSetStateChangeNotifyBits(e, dm, 1);
    let v = e.r32(param_1 + 0x60);
    e.w32(param_1 + 0xa0, v);
    for off in [0x38, 0x40, 0x4c] {
        let t = e.r32(param_1 + off);
        APIextra::TimerReset(e, t);
    }
    e.w32(param_1 + 0x154, 0);
    e.w32(param_1 + 0x158, 1);
    TomeDoomFUNC_CallVibration(e, 0.33, 0.0, 0.33, 0.0)?;
    e.w32(param_1 + 0x58, 0x53);
    e.w32(param_1 + 100, 0x1152);
    let s = e.ri32(param_1 + 100);
    APIinput::InputConfigSetState(e, 3, s);
    APIgameobj::SetContextFlag(e, SELF, 0x18b);
    APIgameobj::SetNextState(e, SELF, 3);
    Ok(())
}

/// 0x00015168 callback 6: an animation of the book finished (last frame). Opening finished -> start
/// capturing the targeted monster or scanning (0x54); closing finished -> put the book away.
/// The per-level animation slots make the compiler emit three branch shapes (G_TomeDoomCB_ANIM_LF).
pub fn TomeDoomCB_ANIM_LF(e: &mut Engine, _: u32) -> LResult<u64> {
    let iVar3 = L(e);
    e.w32(L(e) + 0x68, 0xffff_ffff);
    let uVar1 = e.r32(iVar3 + 0x5c);
    // Some(true): the closing animation finished; Some(false): the opening one; None: other -> return 1
    let closed = match g(e, G_TomeDoomCB_ANIM_LF) {
        0 => {
            if uVar1 == k(e, CB_ANIM_LF_0_1) {
                Some(true)
            } else if uVar1 < k(e, CB_ANIM_LF_0_2) {
                if uVar1 != 1 { None } else { Some(false) }
            } else if uVar1 != k(e, CB_ANIM_LF_0_3) {
                if uVar1 != k(e, CB_ANIM_LF_0_4) { None } else { Some(true) }
            } else {
                Some(false)
            }
        }
        1 => {
            if uVar1 == k(e, CB_ANIM_LF_0_1) {
                Some(true)
            } else if uVar1 < k(e, CB_ANIM_LF_0_2) {
                if uVar1 != 1 { None } else { Some(false) }
            } else if uVar1 == k(e, CB_ANIM_LF_0_3) {
                Some(true)
            } else if uVar1 != k(e, CB_ANIM_LF_0_4) {
                None
            } else {
                Some(false)
            }
        }
        _ => {
            // build 36e3927c (its 5 and 7 are literals of that build)
            if uVar1 == k(e, CB_ANIM_LF_0_1) {
                Some(false)
            } else if uVar1 < 5 {
                if uVar1 != 1 { None } else { Some(true) }
            } else if uVar1 != 5 {
                if uVar1 != 7 { None } else { Some(false) }
            } else {
                Some(true)
            }
        }
    };
    match closed {
        None => return Ok(1),
        Some(true) => {
            let t = e.r32(iVar3 + 0x44);
            APIextra::TimerUnregisterNotify(e, SELF, t);
            if e.ri32(iVar3 + 0x2c) != 0 {
                let l = e.r32(iVar3 + 0x2c);
                APIlight::DestroyLight(e, l);
            }
            e.w32(iVar3 + 0x2c, 0);
            e.w32(iVar3 + 0x70, 0);
            APIgameobj::SetGameObjectVisible(e, SELF, 0);
            APIgameobj::EnableObject(e, SELF, 0);
            APInewSound::StopSound(e, SELF, 0xf);
            APIgameobj::SetNextState(e, SELF, 1);
        }
        Some(false) => {
            let mut bVar2 = false;
            if e.ri32(iVar3 + 0x1c0) == 0 {
                let a = e.ri32(iVar3 + 0x238);
                let iVar5 = LIBAnimMngr::PlayAnim(e, 0.0, a, 2, 0);
                e.wi32(iVar3 + 0x5c, iVar5);
            } else {
                let a = k(e, CB_ANIM_LF_0_5) as i32;
                let iVar5 = LIBAnimMngr::PlayAnim(e, 0.0, a, 2, 0);
                e.wi32(iVar3 + 0x5c, iVar5);
            }
            if e.ri32(iVar3 + 0x60) != 0 {
                TomeDoomFUNC_UpdatePosition(e)?;
                if TomeDoomFUNC_IsGhostInRange(e)? {
                    bVar2 = true;
                    TomeDoomFUNC_SET_CAPTURE(e, iVar3)?;
                }
            }
            if !bVar2 {
                e.w32(iVar3 + 0x58, 0x54);
                e.w32(iVar3 + 100, 0x115c);
                let s = e.ri32(iVar3 + 100);
                APIinput::InputConfigSetState(e, 3, s);
                let t = e.r32(iVar3 + 0x38);
                APIextra::TimerReset(e, t);
                APIgameobj::SetContextFlag(e, SELF, 0x113);
                APIgameobj::SetNextState(e, SELF, 3);
            }
        }
    }
    Ok(1)
}

/// 0x00015458 callback 0xb: left stick while sucking (0x53): drag direction relative to the camera
/// (`param_1` -> float[2] stick x, y)
pub fn TomeDoomCB_INPUT_STICK(e: &mut Engine, param_1: Ptr) -> LResult<u64> {
    let DStack_50 = e.alloca(0x10);
    let DStack_40 = e.alloca(0x10);
    let iVar1 = L(e);
    if e.ri32(L(e) + 0x58) == 0x53 {
        let (p0, p1) = (e.rf(param_1), e.rf(param_1 + 4));
        e.wf(L(e) + 0x13c, p0);
        e.wf(iVar1 + 0x144, p1);
        e.w32(iVar1 + 0x140, 0);
        let cam = k(e, ID_2a00008f);
        APIgameobj::GetLookAt(e, cam, ALL, DStack_50);
        APIgameobj::GetLookRight(e, cam, ALL, DStack_40);
        let mut d50 = e.rv3(DStack_50);
        d50[1] = 0.0;
        let mut fStack_2c = APImath::sqrtf(e, d50[0] * d50[0] + 0.0 + d50[2] * d50[2]);
        fStack_2c = 1.0 / fStack_2c;
        d50 = [d50[0] * fStack_2c * p1, d50[1] * fStack_2c * p1, d50[2] * fStack_2c * p1];
        let d40 = e.rv3(DStack_40);
        let d40 = [d40[0] * -p0, d40[1] * -p0, d40[2] * -p0];
        let j = [d50[0] + d40[0], d50[1] + d40[1], d50[2] + d40[2]];
        let fStack_2c = APImath::sqrtf(e, j[0] * j[0] + j[1] * j[1] + j[2] * j[2]);
        let fStack_2c = 3.25 / fStack_2c;
        let a = G(e, cvJoyDep);
        e.wv3(a, [j[0] * fStack_2c, j[1] * fStack_2c, j[2] * fStack_2c]);
        APIgameobj::SetContextFlag(e, SELF, 0x1cf);
    }
    Ok(0)
}

/// 0x000157e8 callback 0xc: no input
pub fn TomeDoomCB_INPUT_IDLE(e: &mut Engine, _: u32) -> LResult<u64> {
    let iVar1 = L(e);
    let v = e.r32(L(e) + 100) & 0xffff_fffe;
    e.w32(L(e) + 100, v);
    let s = e.ri32(iVar1 + 100);
    APIinput::InputConfigSetState(e, 3, s);
    if e.ri32(iVar1 + 0x1b4) != 0 {
        let h = e.r32(GB0(e) + 8);
        if APIgameobj::SendMessageTo(e, h, NULL, 0x6d, 0, NULL) != 0 {
            e.w32(iVar1 + 0x1b4, 0);
        }
    }
    Ok(0)
}

/// 0x000158c0 rotate the book toward the stick direction (camera relative); 1 if it rotated
pub fn JoyRotate(e: &mut Engine, param_1: f32, param_2: f32) -> LResult<u32> {
    let DStack_80 = e.alloca(0x10);
    let DStack_70 = e.alloca(0x10);
    let DStack_60 = e.alloca(0x10);
    if 0.01 < param_1 * param_1 + param_2 * param_2 {
        let pDStack_3c = APIgameobj::GetBoneLMat(e, SELF, ALL) as Ptr;
        let cam = k(e, ID_2a00008f);
        APIgameobj::GetLookAt(e, cam, ALL, DStack_80);
        APIgameobj::GetLookRight(e, cam, ALL, DStack_70);
        let d80 = e.rv3(DStack_80);
        let d80 = [d80[0] * param_2, d80[1] * param_2, d80[2] * param_2];
        let d70 = e.rv3(DStack_70);
        let d70 = [d70[0] * -param_1, d70[1] * -param_1, d70[2] * -param_1];
        let mut f = [d80[0] + d70[0], d80[1] + d70[1], d80[2] + d70[2]]; // fStack_50/4c/48
        APImath::CMatGetUp(e, DStack_60, pDStack_3c);
        let d60 = e.rv3(DStack_60);
        let mut fStack_40 = f[0] * d60[0] + f[1] * d60[1] + f[2] * d60[2];
        f = [f[0] - d60[0] * fStack_40, f[1] - d60[1] * fStack_40, f[2] - d60[2] * fStack_40];
        let mut fStack_38 = APImath::sqrtf(e, f[0] * f[0] + f[1] * f[1] + f[2] * f[2]);
        fStack_38 = 1.0 / fStack_38;
        f = [f[0] * fStack_38, f[1] * fStack_38, f[2] * fStack_38];
        APImath::CMatGetAt(e, DStack_80, pDStack_3c);
        let d80 = e.rv3(DStack_80);
        fStack_40 = f[0] * d80[0] + f[1] * d80[1] + f[2] * d80[2];
        if fStack_40 < 0.999 {
            if fStack_40 < -0.999 {
                APImath::CMatGetUp(e, DStack_60, pDStack_3c);
            } else {
                let mut d60 = [
                    d80[1] * f[2] - d80[2] * f[1],
                    d80[2] * f[0] - d80[0] * f[2],
                    d80[0] * f[1] - d80[1] * f[0],
                ];
                let mut fStack_38 = APImath::sqrtf(e, d60[0] * d60[0] + d60[1] * d60[1] + d60[2] * d60[2]);
                fStack_38 = 1.0 / fStack_38;
                d60 = [d60[0] * fStack_38, d60[1] * fStack_38, d60[2] * fStack_38];
                e.wv3(DStack_60, d60);
            }
            if 1.0 < fStack_40 {
                fStack_40 = 1.0;
            } else if fStack_40 < -1.0 {
                fStack_40 = -1.0;
            }
            let fVar1 = APImath::acosf(e, fStack_40);
            // Ghidra printed `acos * acos * 5.5`; the code (disasm.py) is acos * (dt * 5.5)
            let dt = APIgame::GetFrameDeltaTime(e);
            let angle = fVar1 * (dt * 5.5);
            APImath::CMatRotate(e, pDStack_3c, DStack_60, angle);
            APIgameobj::SetBoneLMat(e, SELF, ALL, pDStack_3c);
            return Ok(1);
        }
    }
    Ok(0)
}

/// 0x00015e00 callback 0xf: right stick, rotate the book; the hook joint's displacement goes into cvVelocity
pub fn TomeDoomCB_INPUT_ROTATE(e: &mut Engine, param_1: Ptr) -> LResult<u64> {
    let DStack_40 = e.alloca(0x10);
    let j = e.r32(G(e, ui32JointHookMasterID));
    let pDVar1 = APIgameobj::GetBoneWMat(e, SELF, j);
    let cv = G(e, cvVelocity);
    APImath::CMatGetPos(e, cv, pDVar1);
    let (p0, p1) = (e.rf(param_1), e.rf(param_1 + 4));
    let iVar2 = JoyRotate(e, p0, p1)?;
    let j = e.r32(G(e, ui32JointHookMasterID));
    let pDVar1 = APIgameobj::GetBoneWMat(e, SELF, j);
    APImath::CMatGetPos(e, DStack_40, pDVar1);
    let (d, v) = (e.rv3(DStack_40), e.rv3(cv));
    e.wv3(cv, [d[0] - v[0], d[1] - v[1], d[2] - v[2]]);
    if iVar2 == 0 || e.ri32(L(e) + 0x1b4) != 0 {
        if iVar2 == 0 && e.ri32(L(e) + 0x1b4) != 0 {
            let h = e.r32(GB0(e) + 8);
            if APIgameobj::SendMessageTo(e, h, NULL, 0x6d, 0, NULL) != 0 {
                e.w32(L(e) + 0x1b4, 0);
            }
        }
    } else {
        let h = e.r32(GB0(e) + 8);
        if APIgameobj::SendMessageTo(e, h, NULL, 0x6d, 1, NULL) != 0 {
            e.w32(L(e) + 0x1b4, 1);
        }
    }
    let v = e.r32(L(e) + 100) | 1;
    e.w32(L(e) + 100, v);
    let s = e.ri32(L(e) + 100);
    APIinput::InputConfigSetState(e, 3, s);
    Ok(0)
}

/// 0x00016058 callback 0x10: X pressed (button mash input, bit 0x10 = a button was pressed)
pub fn TomeDoomCB_INPUT_X(e: &mut Engine, _: u32) -> LResult<u64> {
    let v = e.r32(L(e) + 0x1e0) | 0x12;
    e.w32(L(e) + 0x1e0, v);
    Ok(0)
}

/// 0x000160a8 callback 0x11: circle pressed
pub fn TomeDoomCB_INPUT_CIRCLE(e: &mut Engine, _: u32) -> LResult<u64> {
    let v = e.r32(L(e) + 0x1e0) | 0x14;
    e.w32(L(e) + 0x1e0, v);
    Ok(0)
}

/// 0x000160f8 callback 0x12: triangle pressed
pub fn TomeDoomCB_INPUT_TRIANGLE(e: &mut Engine, _: u32) -> LResult<u64> {
    let v = e.r32(L(e) + 0x1e0) | 0x18;
    e.w32(L(e) + 0x1e0, v);
    Ok(0)
}

/// 0x00016148 the right button was mashed (or 0.1 s passed without a press): drain the monster's life
/// (`L+0x70`) and the book's power (game buffer 1 +4). Monster dead -> final sucking (state 4);
/// book empty -> monster escapes, book closes (state 5). Both end with ChangeState (longjmp).
pub fn TomeDoomFUNC_DrainMonsterLife(e: &mut Engine, param_1: Ptr) -> LResult<u32> {
    let fStack_3c: f32;
    if e.r32(param_1 + 0x1e0) & 0x10 == 0 {
        let t = e.r32(param_1 + 0x34);
        let fVar2 = APIextra::TimerGetTime(e, t);
        if fVar2 <= 0.1 {
            return Ok(0);
        }
        fStack_3c = 1.05;
        APIextra::TimerReset(e, t);
    } else {
        fStack_3c = 2.5;
        let t = e.r32(param_1 + 0x34);
        APIextra::TimerReset(e, t);
    }
    let fVar2 = e.rf(param_1 + 0x70) - fStack_3c;
    e.wf(param_1 + 0x70, fVar2);
    e.wf(param_1 + 0xcc, fVar2);
    let power = e.r32(GD(e) + 4) + 4;
    let p = e.rf(power) - fStack_3c * 0.9;
    e.wf(power, p);
    if e.rf(param_1 + 0x70) <= 0.0 {
        let p = e.rf(power) + (0.0 - e.rf(param_1 + 0x70) * 0.9);
        e.wf(power, p);
        if e.rf(power) < 0.0 {
            e.w32(power, 0);
        }
        e.w32(param_1 + 0x1dc, 1);
        let cfg = e.ri32(GB0(e) + 0x10);
        APIinput::InputConfigSetState(e, cfg, 0);
        e.w32(param_1 + 0xcc, 0);
        e.w32(param_1 + 0x70, 0xbf80_0000); // -1.0f
        let o = k(e, ID_2a100006);
        APIgameobj::SendMessageTo(e, o, NULL, 0x1d, 0, NULL);
        let o = e.r32(param_1 + 0xdc);
        APIgameobj::SendMessageTo(e, o, NULL, 0xec, 0, param_1 + 200);
        let o = k(e, FUNC_DrainMonsterLife_0_1);
        APIgameobj::SendMessageTo(e, o, NULL, 0x6f, 1, NULL);
        APInewSound::Play(e, SELF, 0x42, 0);
        APIgameobj::ChangeState(e, 4)?;
        Ok(0xffff_ffff)
    } else {
        let o = k(e, ID_2a100006);
        APIgameobj::SendMessageTo(e, o, NULL, 0x1d, 0, NULL);
        let o = e.r32(param_1 + 0xdc);
        APIgameobj::SendMessageTo(e, o, NULL, 0xec, 0, param_1 + 200);
        if 0.0 < e.rf(power) {
            TomeDoomFUNC_SelectButton(e, param_1)?;
            Ok(0)
        } else {
            let hero = e.r32(GB0(e) + 8);
            let flags = e.r32(GB0(e) + 0x34);
            if hero == k(e, FUNC_DrainMonsterLife_0_2)
                || (hero == k(e, FUNC_DrainMonsterLife_0_3) && flags & 0x10 != 0 && flags & 0x100 == 0)
            {
                let (o, s) = (k(e, FUNC_DrainMonsterLife_0_4), k(e, FUNC_DrainMonsterLife_0_5));
                APInewSound::Play(e, o, s, 0);
            }
            let o = k(e, ID_2a000096);
            APIgameobj::SendMessageTo(e, o, NULL, 0x5c, 1, NULL);
            e.w32(power, 0);
            let m = e.r32(param_1 + 0x60);
            APIgameobj::SendMessageTo(e, m, NULL, 0xd6, 100, param_1 + 0xcc);
            e.w32(GB0(e) + 0x30, 0);
            e.w32(param_1 + 0x60, 0);
            TomeDoomFUNC_ShaggyTomeOut(e)?;
            APIgameobj::ChangeState(e, 5)?;
            Ok(0xffff_ffff)
        }
    }
}

/// 0x00016648 pick which of the 3 generated buttons must be mashed, from the monster's remaining
/// life fraction and its button pattern (`L+0xe0`: 2, 3 or 6 phases)
pub fn TomeDoomFUNC_SelectButton(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    let n = e.ri32(param_1 + 0xe0);
    if n == 2 || n == 3 || n == 6 {
        let fVar2 = e.rf(param_1 + 0x70) / e.rf(param_1 + 200);
        let mut uStack_38: u32 = 1;
        let uVar1 = e.r32(param_1 + 0xe0);
        if uVar1 == 3 {
            uStack_38 = if fVar2 < 0.3333 {
                3
            } else if fVar2 < 0.666 {
                2
            } else {
                1
            };
        } else if uVar1 < 4 {
            if uVar1 == 2 {
                uStack_38 = if fVar2 < 0.5 { 2 } else { 1 };
            }
        } else if uVar1 == 6 {
            uStack_38 = if fVar2 < 0.1666 {
                3
            } else if fVar2 < 0.3333 {
                2
            } else if fVar2 < 0.5 {
                1
            } else if fVar2 < 0.666 {
                3
            } else if fVar2 < 0.8333 {
                2
            } else {
                1
            };
        }
        if e.r32(param_1 + 0x1e4) != uStack_38 {
            if uStack_38 == 2 {
                let v = e.r32(param_1 + 0x1f0);
                e.w32(param_1 + 0x1e8, v);
            } else if uStack_38 < 3 {
                if uStack_38 == 1 {
                    let v = e.r32(param_1 + 0x1ec);
                    e.w32(param_1 + 0x1e8, v);
                }
            } else if uStack_38 == 3 {
                let v = e.r32(param_1 + 500);
                e.w32(param_1 + 0x1e8, v);
            }
            let t = e.r32(param_1 + 0x48);
            APIextra::TimerReset(e, t);
            e.w32(param_1 + 0x1e4, uStack_38);
            TomeDoomFUNC_CallVibration(e, 0.66, 0.0, 0.33, 0.0)?;
        }
    }
    Ok(())
}

/// 0x000169b0 shuffle the 3 mash buttons (input bits 2 = X, 4 = circle, 8 = triangle) into slots 1..3
pub fn TomeDoomFUNC_GenerateButton(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    let auStack_50: [u32; 3] = [2, 4, 8];
    let iStack_40 = LIBApiCFunctions::IntRandom(e, 0, 2);
    let mut iStack_3c = LIBApiCFunctions::IntRandom(e, 0, 2);
    while iStack_3c == iStack_40 {
        iStack_3c = LIBApiCFunctions::IntRandom(e, 0, 2);
    }
    e.w32(param_1 + 0x1ec, auStack_50[iStack_40 as usize]);
    e.w32(param_1 + 0x1f0, auStack_50[iStack_3c as usize]);
    e.w32(param_1 + 500, auStack_50[(3 - (iStack_40 + iStack_3c)) as usize]);
    e.w32(param_1 + 0x1e4, 1);
    let v = e.r32(param_1 + 0x1ec);
    e.w32(param_1 + 0x1e8, v);
    let t = e.r32(param_1 + 0x48);
    APIextra::TimerReset(e, t);
    TomeDoomFUNC_SelectButton(e, param_1)?;
    Ok(())
}

/// 0x00016b48 which hero holds the book (L+0x1b8, the other one L+0x1bc) and the matching anims
/// (L+0x230 open, +0x234 close, +0x238 idle open), hook joints and hand offset (L+0x240 = -hook pos)
pub fn TomeDoomFUNC_SetMasterParams(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    let cur = e.r32(GB0(e) + 8);
    if cur == k(e, FUNC_SetMasterParams_0_1) || cur == k(e, FUNC_SetMasterParams_0_2) {
        e.w32(param_1 + 0x1b8, cur);
        let other = e.r32(GB0(e) + 0xc);
        e.w32(param_1 + 0x1bc, other);
    } else {
        let v = k(e, FUNC_SetMasterParams_0_3);
        e.w32(param_1 + 0x1b8, v);
    }
    let hook = G(e, pcmTomeHook_105);
    if e.r32(param_1 + 0x1b8) == k(e, FUNC_SetMasterParams_0_4) {
        for (off, i) in [(0x230, FUNC_SetMasterParams_0_5), (0x234, FUNC_SetMasterParams_0_6),
                         (0x238, FUNC_SetMasterParams_0_7), (0x23c, FUNC_SetMasterParams_0_8)] {
            let v = k(e, i);
            e.w32(param_1 + off, v);
        }
        let b = k(e, FUNC_SetMasterParams_0_9);
        let m = APIgameobj::GetBoneLMat(e, SELF, b) as u32;
        e.w32(hook, m);
        let (a, b) = (k(e, FUNC_SetMasterParams_0_10), k(e, FUNC_SetMasterParams_0_11));
        e.w32(G(e, ui32JointHookMasterID), a);
        e.w32(G(e, ui32JointHookSlaveID), b);
    } else if e.r32(param_1 + 0x1b8) == k(e, FUNC_SetMasterParams_0_12) {
        for (off, i) in [(0x230, FUNC_SetMasterParams_0_13), (0x234, FUNC_SetMasterParams_0_14),
                         (0x238, FUNC_SetMasterParams_0_15), (0x23c, FUNC_SetMasterParams_0_16)] {
            let v = k(e, i);
            e.w32(param_1 + off, v);
        }
        let b = k(e, FUNC_SetMasterParams_0_17);
        let m = APIgameobj::GetBoneLMat(e, SELF, b) as u32;
        e.w32(hook, m);
        let (a, b) = (k(e, FUNC_SetMasterParams_0_18), k(e, FUNC_SetMasterParams_0_19));
        e.w32(G(e, ui32JointHookMasterID), a);
        e.w32(G(e, ui32JointHookSlaveID), b);
    }
    let m = e.r32(hook);
    APImath::CMatGetPos(e, param_1 + 0x240, m);
    let v = e.rv3(param_1 + 0x240);
    e.wv3(param_1 + 0x240, [-v[0], -v[1], -v[2]]);
    Ok(())
}

/// 0x00016d80 callback 7: state change of a dynamic link (`param_1` -> {dm id, ?, state bits});
/// for the book <-> monster link (L+0x18) bit 0 = the monster is being pulled
pub fn TomeDoomCB_DMSTATECHANGE(e: &mut Engine, param_1: Ptr) -> LResult<u64> {
    if e.r32(param_1) == e.r32(L(e) + 0x18) {
        let v = (e.r32(param_1 + 8) & 1 != 0) as u32;
        e.w32(L(e) + 0x154, v);
    }
    Ok(0)
}

/// 0x00016e10 callback 0x17 (physics): move the book so its hook joint(s) stay on the hero(es)'
/// hands; with two heroes, cast rays hero -> hero to detect a wall between them (cvWallNormal, bCollision)
pub fn TomeDoomCB_PHYSIC(e: &mut Engine, _: u32) -> LResult<u64> {
    let DStack_150 = e.alloca(0x10);
    let DStack_140 = e.alloca(0x10);
    let DStack_130 = e.alloca(0x10);
    let DStack_110 = e.alloca(0x10);
    let DStack_100 = e.alloca(0x10);
    let ray_in = e.alloca(0x30); // RayIntersectIn at fStack_e0
    let aRStack_b0 = e.alloca(0x40);
    let aRStack_70 = e.alloca(0x40);
    let hero = e.r32(L(e) + 0x1b8);
    APIgameobj::GetPosition(e, hero, ALL, DStack_150);
    let j = e.r32(G(e, ui32JointHookMasterID));
    let pDStack_120 = APIgameobj::GetBoneWMat(e, SELF, j);
    APImath::CMatGetPos(e, DStack_140, pDStack_120);
    let (d150, d140) = (e.rv3(DStack_150), e.rv3(DStack_140));
    let mut d130 = [d150[0] - d140[0], d150[1] - d140[1], d150[2] - d140[2]];
    if e.ri32(L(e) + 0x1c0) != 0 {
        let other = e.r32(GB0(e) + 0xc);
        APIgameobj::GetPosition(e, other, ALL, DStack_100);
        let j = e.r32(G(e, ui32JointHookSlaveID));
        let pDStack_120 = APIgameobj::GetBoneWMat(e, SELF, j);
        APImath::CMatGetPos(e, DStack_110, pDStack_120);
        let (d110, d100) = (e.rv3(DStack_110), e.rv3(DStack_100));
        let f = [d110[0] - d100[0], d110[1] - d100[1], d110[2] - d100[2]];
        d130 = [(d130[0] - f[0]) * 0.5, (d130[1] - f[1]) * 0.5, (d130[2] - f[2]) * 0.5];
        for (off, v) in [(0x28, 0), (0x18, 0), (0x20, 9), (0x1c, 0), (0x24, 0)] {
            e.w32(ray_in + off, v);
        }
        let dir = [d110[0] - d150[0], d110[1] - d150[1], d110[2] - d150[2]];
        e.wv3(ray_in + 0xc, dir);
        e.wf(ray_in + 8, d150[2]);
        e.wf(ray_in, d150[0]);
        e.wf(ray_in + 4, d150[1] + 0.5);
        let iVar1 = APIphysic::CheckRayIntersection(e, ray_in, aRStack_b0);
        if iVar1 == 2 {
            e.wv3(ray_in + 0xc, [-dir[0], -dir[1], -dir[2]]);
            e.wf(ray_in + 8, d100[2]);
            e.wf(ray_in, d100[0]);
            e.wf(ray_in + 4, d100[1] + 0.5);
            let iVar1 = APIphysic::CheckRayIntersection(e, ray_in, aRStack_70);
            if iVar1 == 2 {
                let (n1, n2) = (e.rv3(aRStack_70 + 0xc), e.rv3(aRStack_b0 + 0xc));
                let mut wn = [n1[0] + n2[0], n1[1] + n2[1], n1[2] + n2[2]];
                let wa = G(e, cvWallNormal);
                if 0.001 < wn[0] * wn[0] + wn[1] * wn[1] + wn[2] * wn[2] {
                    let mut fStack_30 = APImath::sqrtf(e, wn[0] * wn[0] + wn[1] * wn[1] + wn[2] * wn[2]);
                    fStack_30 = 1.0 / fStack_30;
                    wn = [wn[0] * fStack_30, wn[1] * fStack_30, wn[2] * fStack_30];
                    e.wv3(wa, wn);
                } else {
                    e.wv3(wa, wn);
                    APIgameobj::GetLookAt(e, SELF, ALL, wa);
                }
                e.w32(G(e, bCollision), 1);
            }
        }
    }
    e.wv3(DStack_130, d130);
    APIgameobj::Translate(e, SELF, DStack_130);
    Ok(0)
}

/// 0x000172f0 the hero picks up the book (first time): hide it, mark it owned (buffer 2 flags 1|2),
/// fill the book's power to 90, optional tutorial text, tell the HUD
pub fn TomeDoomFUNC_GrabBook(e: &mut Engine, param_1: u32) -> LResult<()> {
    let auStack_80 = e.alloca(0x30);
    let retr = e.alloca(0x10); // ObjectRetrieverParams {uStack_50, uStack_4c}
    APIgameobj::GetShadowParams(e, SELF, auStack_80);
    let f = e.r32(auStack_80) | 4;
    e.w32(auStack_80, f);
    APIgameobj::SetShadowParams(e, SELF, auStack_80);
    for bit in [1, 2] {
        let b2 = e.r32(GD(e) + 8);
        let v = e.r32(b2) | bit;
        e.w32(b2, v);
    }
    APIphysic::SetFlags(e, SELF, 1);
    APIgameobj::SetGameObjectVisible(e, SELF, 0);
    APIgameobj::EnableObject(e, SELF, 0);
    APIgameobj::SetGroupID(e, SELF, 6);
    let iVar1 = e.r32(GD(e) + 4);
    e.w32(L(e) + 0x250, 0x42b4_0000); // 90.0f
    e.w32(iVar1 + 4, 0x42b4_0000);
    let o = k(e, ID_2a100006);
    APIgameobj::SendMessageTo(e, o, NULL, 0x22, 0, NULL);
    if param_1 != 0 {
        TomeDoomFUNC_ShowText(e, 0x4060_0000, 0x53)?; // 3.5f
    }
    e.w32(retr, 1);
    e.w32(retr + 4, 0x1a);
    APIgameobj::SendMessageTo(e, 0xff00_0008, retr, 0x1ef, 0, NULL);
    Ok(())
}

/// 0x000174e0 callback 0x13: timer notification (`param_1` = timer id); the light timer (L+0x44)
/// animates the book's glow
pub fn TomeDoomCB_TIMER(e: &mut Engine, param_1: u32) -> LResult<u64> {
    let bVar1 = param_1 != e.r32(L(e) + 0x44);
    if !bVar1 {
        let l = L(e);
        TomeDoomFUNC_UpdateLight(e, l)?;
    }
    Ok(bVar1 as u64)
}

/// 0x00017560 callback 0x14: animation event (`param_1` -> {anim, event}); event 0 = anim started:
/// closing anims play sound 0x11, opening anims sound 0x12 (numbers per level build)
pub fn TomeDoomCB_ANIM(e: &mut Engine, param_1: Ptr) -> LResult<u64> {
    if e.r32(param_1 + 4) != 0 {
        return Ok(0);
    }
    let uVar1 = e.r32(param_1);
    // true: LAB_00017610 (sound K2), false: sound K6
    let first = if uVar1 == k(e, CB_ANIM_0_1) {
        true
    } else if uVar1 < k(e, CB_ANIM_0_3) {
        if uVar1 != 1 {
            return Ok(0);
        }
        false
    } else if g(e, G_TomeDoomCB_ANIM) == 0 {
        if uVar1 != k(e, CB_ANIM_0_4) {
            if uVar1 != k(e, CB_ANIM_0_5) {
                return Ok(0);
            }
            true
        } else {
            false
        }
    } else {
        // shape group 1 (0a8f4fed ...): `if (uVar1 == K4) goto LAB; if (uVar1 != K5) return 0;`
        if uVar1 == k(e, CB_ANIM_0_4) {
            true
        } else if uVar1 != k(e, CB_ANIM_0_5) {
            return Ok(0);
        } else {
            false
        }
    };
    let s = if first { k(e, CB_ANIM_0_2) } else { k(e, CB_ANIM_0_6) };
    APInewSound::Play(e, SELF, s, 0);
    Ok(0)
}

/// 0x00017668 callback 0x15: after all logic ran this frame, clear the buttons pressed this frame
pub fn TomeDoomCB_POSTLOGIC(e: &mut Engine, _: u32) -> LResult<u64> {
    e.w32(L(e) + 0x1e0, 0);
    Ok(0)
}

/// 0x000176b0 callback 0x16: cutscene start/end (`param_1` -> {?, 1 = shown})
pub fn TomeDoomCB_CUTSCENE(e: &mut Engine, param_1: Ptr) -> LResult<u64> {
    if e.ri32(param_1 + 4) == 1 {
        APIgameobj::SetGameObjectVisible(e, SELF, 1);
        APIgameobj::EnableObject(e, SELF, 1);
        APIphysic::SetFlags(e, SELF, 1);
        APIgameobj::Sleep(e, SELF, -1.0);
    } else {
        APIgameobj::SetGameObjectVisible(e, SELF, 0);
        APIgameobj::EnableObject(e, SELF, 0);
        APIphysic::SetFlags(e, SELF, 1);
        APIgameobj::Sleep(e, SELF, 0.0);
    }
    Ok(0)
}

/// 0x000177f0 tell the current hero the book is put away (message 0x92)
pub fn TomeDoomFUNC_ShaggyTomeOut(e: &mut Engine) -> LResult<()> {
    let h = e.r32(GB0(e) + 8);
    APIgameobj::SendMessageTo(e, h, NULL, 0x92, 0, NULL);
    Ok(())
}

/// 0x00017850 straighten both heroes (look-at kept, up = +Y)
pub fn TomeDoomFUNC_FixHerosMatrice(e: &mut Engine) -> LResult<()> {
    let DStack_70 = e.alloca(0x10);
    let DStack_60 = e.alloca(0x10);
    let DStack_40 = e.alloca(0x10);
    let DStack_30 = e.alloca(0x10);
    for (ids, at, up) in [
        ([FUNC_FixHerosMatrice_0_1, FUNC_FixHerosMatrice_0_2, FUNC_FixHerosMatrice_0_3], DStack_60, DStack_70),
        ([FUNC_FixHerosMatrice_0_4, FUNC_FixHerosMatrice_0_5, FUNC_FixHerosMatrice_0_6], DStack_30, DStack_40),
    ] {
        let o = k(e, ids[0]);
        let iVar1 = APIgameobj::GetID(e, o, NULL);
        if iVar1 != 0x7fff_fffe {
            let o = k(e, ids[1]);
            let pDStack_50 = APIgameobj::GetBoneLMat(e, o, ALL) as Ptr;
            APImath::CMatGetAt(e, at, pDStack_50);
            e.wv3(up, [0.0, 1.0, 0.0]);
            APImath::CMatLookAtAlignUp(e, pDStack_50, at, up);
            let o = k(e, ids[2]);
            APIgameobj::SetBoneLMat(e, o, ALL, pDStack_50);
        }
    }
    Ok(())
}

/// 0x00017a30 pad vibration of the current player's pad (buffer 0 +0x4c)
pub fn TomeDoomFUNC_CallVibration(e: &mut Engine, param_1: f32, param_2: f32, param_3: f32, param_4: f32) -> LResult<()> {
    let pad = e.ri32(GB0(e) + 0x4c);
    APIinput::InputSetVibration(e, pad, 0, param_1, param_2, param_3, param_4);
    Ok(())
}

/// 0x00017aa0 show text `param_2` for `param_1` (float bits) seconds unless a text is shown
pub fn TomeDoomFUNC_ShowText(e: &mut Engine, param_1: u32, param_2: u32) -> LResult<()> {
    let auStack_70 = e.alloca(0x40);
    let bVar1 = LIBApiText::IsTextDisplayed(e);
    if bVar1 == 0 {
        LIBApiText::InitStruct(e, auStack_70);
        e.w32(auStack_70, param_1);
        LIBApiText::ShowText(e, param_2, auStack_70);
    }
    Ok(())
}

/// 0x00017b38 the 5 glow colours of the book light (CColor RGBA at L+0x160 + i*0x10: grey 1.0 .. 0.2)
pub fn TomeDoomFUNC_InitColorLight(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    for uStack_1c in 0u32..5 {
        e.w32(param_1 + 0x16c + uStack_1c * 0x10, 0);
        let fVar1 = (5 - uStack_1c) as f32 * 0.2;
        e.wf(param_1 + 0x168 + uStack_1c * 0x10, fVar1);
        e.wf(param_1 + 0x164 + uStack_1c * 0x10, fVar1);
        e.wf(param_1 + 0x160 + uStack_1c * 0x10, fVar1);
    }
    Ok(())
}

/// 0x00017c40 flicker the book light while sucking: brighter colours (0..2) when the monster comes
/// closer, dimmer (2..4) when it gets away, colour 4 when it is out of range (5.6)
pub fn TomeDoomFUNC_UpdateLight(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    let DStack_60 = e.alloca(0x10);
    let DStack_50 = e.alloca(0x10);
    let DStack_40 = e.alloca(0x10);
    if e.ri32(param_1 + 0x2c) != 0 {
        let m = e.r32(param_1 + 0x60);
        APIgameobj::GetPosition(e, m, ALL, DStack_60);
        APIgameobj::GetPosition(e, SELF, ALL, DStack_50);
        APImath::CVectorSub(e, DStack_40, DStack_60, DStack_50);
        let fStack_30 = APImath::CVectorLength(e, DStack_40);
        let l = e.r32(param_1 + 0x2c);
        if fStack_30 < 5.6 {
            let iStack_2c = LIBApiCFunctions::IntRandom(e, 0, 2);
            if fStack_30 < e.rf(param_1 + 0x1b0) {
                let c = (iStack_2c * 0x10) as u32 + 0x160 + param_1;
                APIlight::SetColor(e, l, c);
            } else {
                let c = (iStack_2c * 0x10) as u32 + 0x180 + param_1;
                APIlight::SetColor(e, l, c);
            }
            e.wf(param_1 + 0x1b0, fStack_30);
        } else {
            APIlight::SetColor(e, l, param_1 + 0x1a0);
        }
    }
    Ok(())
}

/// 0x00017e18 create the book's light (child of the book, radius 3.75)
pub fn TomeDoomFUNC_Create_Light(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    let DStack_30 = e.alloca(0x10);
    if e.ri32(param_1 + 0x2c) == 0 {
        e.wv3(DStack_30, [0.0, 0.5, 0.3]);
        let uVar1 = APIlight::CreateLight(e, 0x80);
        e.w32(param_1 + 0x2c, uVar1);
        let l = e.r32(param_1 + 0x2c);
        APIlight::SetColor(e, l, param_1 + 0x1a0);
        APIlight::SetFlag(e, l, 3);
        APIlight::SetRadius(e, l, 3.75);
        APIgameobj::AddChild(e, SELF, ALL, l, 1);
        APIgameobj::SetTranslation(e, l, DStack_30);
    }
    Ok(())
}

/// 0x00017f48 pitch of the sucking sound rises as the monster's life drops
pub fn TomeDoomFUNC_Update_Sound_Pitch(e: &mut Engine, param_1: Ptr, param_2: u32) -> LResult<()> {
    let p = 12.0 - (e.rf(param_1 + 0x70) / e.rf(param_1 + 200)) * 11.9;
    APInewSound::SetPitch(e, SELF, param_2, p);
    Ok(())
}

/// 0x00017fd8 nearest capturable monster (group mask 0x200) within 5.7 of the current hero that is in
/// front of the book's aim object (ID_2a000093) according to `param_2`; 0 if none.
/// Walks the group-mask retriever (0xff000010) over the monster groups set in buffer 2's flags.
pub fn TomeDoomFUNC_ScanForMonster(e: &mut Engine, param_1: u64, param_2: u32) -> LResult<u32> {
    let retr = e.alloca(0x20); // ObjectRetrieverParams {uStack_60 cursor, uStack_5c mask}
    let mut uStack_40: u32 = 0x7fff_fffe;
    let mut uStack_3c: u32 = 0;
    let mut fStack_38: f32 = 5.7;
    let b2 = e.r32(GD(e) + 8);
    let uStack_5c = e.r32(b2) & 0x7e;
    e.w32(retr, 1);
    // (unreachable in the original: `if (mask & 0xffffff81) mask &= 0x7e;`)
    e.w32(retr + 4, uStack_5c);
    uStack_40 = APIgameobj::GetID(e, 0xff00_0010, retr);
    while uStack_40 != 0x7fff_fffe {
        let uStack_30 = APIgameobj::GetGroupBitMask(e, uStack_40);
        if uStack_30 & 0x200 != 0 {
            let hero = e.ri32(GB0(e) + 8);
            let fStack_2c = LIBApiCFunctions::GetDistance(e, hero, uStack_40, 1, 1);
            if fStack_2c < fStack_38 {
                let aim = k(e, ID_2a000093);
                let iVar1 = LIBApiCFunctions::CheckPositionOrientation(e, aim, uStack_40, param_2);
                if iVar1 != 0 {
                    fStack_38 = fStack_2c;
                    uStack_3c = uStack_40;
                }
            }
        }
        uStack_40 = APIgameobj::GetID(e, 0xff00_0010, retr);
    }
    Ok(uStack_3c)
}

/// 0x000181e0 tell every wisp (group mask 0x800) within 5.6 of the book (in-range retriever
/// 0xff000100) whether it may come to the book: msg 0x11e, sub = (book power < 180)
pub fn TomeDoomFUNC_ContactWisps(e: &mut Engine) -> LResult<()> {
    let uStack_50 = e.alloca(0x20); // ObjectRetrieverParams
    let b1 = e.r32(GD(e) + 4);
    let uStack_2c = (e.rf(b1 + 4) < 180.0) as u32;
    e.w32(uStack_50, 1);
    e.w32(uStack_50 + 4, 0x800);
    let id = APIgameobj::GetID(e, SELF, NULL);
    e.w32(uStack_50 + 8, id);
    e.w32(uStack_50 + 0x14, 0);
    e.w32(uStack_50 + 0x10, 0);
    e.w32(uStack_50 + 0xc, 0);
    e.w32(uStack_50 + 0x18, 0x40b3_3333); // 5.6f
    e.w32(uStack_50 + 0x1c, 0x3dcc_cccd); // 0.1f
    let mut uStack_30 = APIgameobj::GetID(e, 0xff00_0100, uStack_50);
    while uStack_30 != 0x7fff_fffe {
        APIgameobj::SendMessageTo(e, uStack_30, NULL, 0x11e, uStack_2c, NULL);
        uStack_30 = APIgameobj::GetID(e, 0xff00_0100, uStack_50);
    }
    Ok(())
}

/// 0x00018338 no monster (group mask 0x7e) left in the level (nearest retriever 0xff000004,
/// range 9999.9) -> msg 0xe8 to ID_2a000096
pub fn TomeDoomFUNC_CheckAnyMonsterLeft(e: &mut Engine) -> LResult<()> {
    let uStack_50 = e.alloca(0x20); // ObjectRetrieverParams
    e.w32(uStack_50, 1);
    e.w32(uStack_50 + 4, 0x7e);
    e.w32(uStack_50 + 8, SELF);
    e.w32(uStack_50 + 0x14, 0);
    e.w32(uStack_50 + 0x10, 0);
    e.w32(uStack_50 + 0xc, 0);
    e.w32(uStack_50 + 0x18, 0x461c_3f9a); // 9999.9f
    e.w32(uStack_50 + 0x1c, 0x3dcc_cccd); // 0.1f
    let iStack_30 = APIgameobj::GetID(e, 0xff00_0004, uStack_50);
    if iStack_30 == 0x7fff_fffe {
        let o = k(e, ID_2a000096);
        APIgameobj::SendMessageTo(e, o, NULL, 0xe8, 0, NULL);
    }
    Ok(())
}

/// 0x00018418 msg 0x11e sub 0 (stop coming) to every wisp (group mask retriever 0xff000010, mask 0x800)
pub fn TomeDoomFUNC_CancelScanForWisps(e: &mut Engine) -> LResult<()> {
    let uStack_40 = e.alloca(0x10); // ObjectRetrieverParams (only [0], [1] initialised)
    e.w32(uStack_40, 1);
    e.w32(uStack_40 + 4, 0x800);
    APIgameobj::SendMessageTo(e, 0xff00_0010, uStack_40, 0x11e, 0, NULL);
    Ok(())
}

/// 0x00018480 (no callers) launch latent ID_2a800139 at the aim object and attach `param_1` to it
pub fn TomeDoomFUNC_LaunchItem(e: &mut Engine, param_1: u32) -> LResult<()> {
    let DStack_40 = e.alloca(0x10);
    let lat = k(e, ID_2a800139);
    let uStack_30 = APIgameobj::LaunchLatant(e, lat);
    let aim = k(e, ID_2a000093);
    APIgameobj::GetPosition(e, aim, ALL, DStack_40);
    APIgameobj::SetTranslation(e, uStack_30, DStack_40);
    APIgameobj::TakeOrientation(e, uStack_30, aim, ALL);
    APIphysic::SetFlags(e, param_1, 1);
    APIgameobj::AddChild(e, uStack_30, ALL, param_1, 1);
    Ok(())
}

/// 0x00018588 show the button-mash prompt (msg 0x31 to the HUD ID_2a100006) for the button at L+0x1e8
/// (2 -> icon 0xe, 4 -> 0x10, 8 -> 0xf, else 0xe) when it changed or prompts were hidden (buffer 2 flag 0x20000)
pub fn TomeDoomFUNC_Update_NTAction(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    if e.r32(param_1 + 0x1e8) != e.r32(param_1 + 0x1f8) || {
        let b2 = e.r32(GD(e) + 8);
        e.r32(b2) & 0x20000 == 0
    } {
        let v = e.r32(param_1 + 0x1e8);
        e.w32(param_1 + 0x1f8, v);
        e.w32(param_1 + 0x1fc, 0xe);
        let uVar1 = e.r32(param_1 + 0x1f8);
        if uVar1 == 4 {
            e.w32(param_1 + 0x1fc, 0x10);
        } else if uVar1 < 5 {
            if uVar1 == 2 {
                e.w32(param_1 + 0x1fc, 0xe);
            }
        } else if uVar1 == 8 {
            e.w32(param_1 + 0x1fc, 0xf);
        }
        let o = k(e, ID_2a100006);
        let s = e.r32(param_1 + 0x1fc);
        APIgameobj::SendMessageTo(e, o, NULL, 0x31, s, NULL);
        APInewSound::Play(e, SELF, 0x41, 0);
    }
    Ok(())
}

/// 0x000186e8 hide the button-mash prompt (msg 0x32 to the HUD)
pub fn TomeDoomFUNC_Remove_NTAction(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    if e.ri32(param_1 + 0x1fc) != 0 {
        let o = k(e, ID_2a100006);
        let s = e.r32(param_1 + 0x1fc);
        APIgameobj::SendMessageTo(e, o, NULL, 0x32, s, NULL);
        e.w32(param_1 + 0x1fc, 0);
    }
    Ok(())
}

/// 0x00018760 the book's message handler (registered with SetMessageFunction in the constructor).
/// `iVar1` is pLocalDataTome on entry; where the original re-reads the global (`L(e)`), so does the port
/// (they only differ if a nested call made another tomedoom instance current).
pub fn TomeDoomFUNC_MessageHandler(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    let auStack_60 = e.alloca(0x30); // _ShadowParams (12 words)
    let iVar1 = L(e);
    if param_1 == 0xd1 {
        // take the book out: collision bits, enable, scan for the nearest monster (if power left)
        APIphysic::PrimResetIntAttribBits(e, SELF, 0x84, 5, 0xffff_ffff);
        if param_2 == 0 {
            let v = e.r32(iVar1 + 0x6c);
            APIphysic::PrimSetIntAttribBits(e, SELF, 0x84, 5, v);
        } else {
            let v = param_2 | e.r32(iVar1 + 0x6c);
            APIphysic::PrimSetIntAttribBits(e, SELF, 0x84, 5, v);
        }
        APIgameobj::EnableObject(e, SELF, 1);
        APIphysic::SetVelocity(e, SELF, 0.0, 0.0, 0.0);
        e.w32(iVar1 + 0x1c0, 0);
        TomeDoomFUNC_SetMasterParams(e, iVar1)?;
        let b1 = e.r32(GD(e) + 4);
        if e.rf(b1 + 4) <= 0.0 {
            let b0 = GB0(e);
            e.w32(b0 + 0x30, 0);
            e.w32(iVar1 + 0x60, 0);
        } else {
            let iVar4 = TomeDoomFUNC_ScanForMonster(e, 0, 0xffff)?;
            let b0 = GB0(e);
            e.w32(b0 + 0x30, iVar4);
            e.w32(iVar1 + 0x60, iVar4);
        }
        let l = L(e);
        e.w32(l + 0x1b4, 0);
        APIgameobj::SetNextState(e, SELF, 2);
    } else if param_1 < 0xd2 {
        if param_1 == 0x70 {
            TomeDoomFUNC_GrabBook(e, 0)?;
        } else if param_1 < 0x71 {
            if param_1 == 0x12 {
                // pause
                let bVar2 = APIinput::ConfigIsActive(e, 3);
                e.wi32(iVar1 + 0x1c4, bVar2 as i32);
                APIinput::InputConfigActivate(e, 3, 0);
                APIgameobj::Sleep(e, SELF, -1.0);
            } else if param_1 == 0x13 {
                // unpause
                APIgameobj::Sleep(e, SELF, 0.0);
                let a = e.ri32(iVar1 + 0x1c4);
                APIinput::InputConfigActivate(e, 3, a);
            }
        } else if param_1 == 0xc1 {
            // restore the book power saved by 0xd4
            let l = L(e);
            let v = e.r32(l + 0x250);
            let b1 = e.r32(GD(e) + 4);
            e.w32(b1 + 4, v);
            let o = k(e, ID_2a100006);
            APIgameobj::SendMessageTo(e, o, NULL, 0x1d, 0, NULL);
        } else if param_1 < 0xc2 {
            if param_1 == 0x76 && {
                let l = L(e);
                e.w32(l + 0x1c0, 1);
                e.r32(iVar1 + 0x5c) == e.r32(iVar1 + 0x238)
            } {
                let a = k(e, FUNC_MessageHandler_0_1) as i32;
                let iVar4 = LIBAnimMngr::PlayAnim(e, 0.0, a, 2, 0);
                e.wi32(iVar1 + 0x5c, iVar4);
            }
        } else if param_1 == 0xd0 {
            // put the book away (normal close)
            let m = e.r32(L(e) + 0x60);
            let iVar4 = APIgameobj::GetID(e, m, NULL);
            if iVar4 != 0x7fff_fffe && 0.0 < e.rf(iVar1 + 0x70) {
                let m = e.r32(iVar1 + 0x60);
                APIgameobj::SendMessageTo(e, m, NULL, 0xd6, 100, iVar1 + 0xcc);
            }
            if e.ri32(iVar1 + 0x54) != 0 {
                let h = e.r32(iVar1 + 0x54);
                APIgameobj::SendMessageTo(e, h, NULL, 0xdf, 0, NULL);
                e.w32(iVar1 + 0x54, 0);
            }
            if e.ri32(iVar1 + 0x58) == 0x53 {
                APInewSound::Play(e, SELF, 0x13, 0);
            }
            APInewSound::StopSound(e, SELF, 0xf);
            APInewSound::StopSound(e, SELF, 0x2e);
            let h = e.r32(L(e) + 0x1b8);
            APIgameobj::SendMessageTo(e, h, NULL, 0x6c, 0, NULL);
            if e.ri32(L(e) + 0x1c0) != 0 {
                let h = e.r32(GB0(e) + 0xc);
                APIgameobj::SendMessageTo(e, h, NULL, 0x6c, 0, NULL);
            }
            TomeDoomFUNC_Remove_NTAction(e, iVar1)?;
            APIgameobj::SetNextState(e, SELF, 5);
        }
    } else if param_1 == 0xe5 {
        let b2 = e.r32(GD(e) + 8);
        let v = e.r32(b2) | 0x7e;
        e.w32(b2, v);
        TomeDoomFUNC_GrabBook(e, 1)?;
    } else if param_1 < 0xe6 {
        if param_1 == 0xd6 {
            if param_2 == 0x68 {
                // the monster is captured
                let o = k(e, FUNC_MessageHandler_0_2);
                APIgameobj::SendMessageTo(e, o, NULL, 0x6f, 0, NULL);
                let m = e.r32(iVar1 + 0x60);
                APIgameobj::SendMessageTo(e, m, NULL, 0xd6, 0x68, NULL);
                let o = k(e, ID_2a000096);
                APIgameobj::SendMessageTo(e, o, NULL, 0xd6, 0x68, NULL);
                APInewSound::StopSound(e, SELF, 0xf);
                TomeDoomFUNC_CheckAnyMonsterLeft(e)?;
                e.w32(iVar1 + 0x54, 0);
                TomeDoomFUNC_ShaggyTomeOut(e)?;
                APIgameobj::SetContextFlag(e, SELF, 0);
                APIgameobj::SetNextState(e, SELF, 5);
            } else if param_2 < 0x69 {
                if param_2 == 0x66 {
                    // the monster's capture parameters (7 words) -> L+200..; life L+0x70 = L+0xcc
                    let l = L(e);
                    let v = e.r32(param_3);
                    e.w32(l + 200, v);
                    for off in (4..0x1c).step_by(4) {
                        let v = e.r32(param_3 + off);
                        e.w32(iVar1 + 200 + off, v);
                    }
                    let v = e.r32(iVar1 + 0xcc);
                    e.w32(iVar1 + 0x70, v);
                    let d0 = e.rf(iVar1 + 0xd0);
                    e.wf(iVar1 + 0x134, (d0 / (d0 + 1.0)) * 3.0);
                    let v = 3.0 - e.rf(iVar1 + 0x134);
                    e.wf(iVar1 + 0x138, v);
                    let o = e.r32(iVar1 + 0xdc);
                    APIgameobj::SendMessageTo(e, o, NULL, 0xec, 0, iVar1 + 200);
                }
            } else if param_2 == 0x69 {
                APInewSound::Play(e, SELF, 0x10, 0);
                let m = e.r32(iVar1 + 0x60);
                APIgameobj::SendMessageTo(e, m, NULL, 0xd6, 0x69, param_3);
            } else if param_2 == 0x6a {
                // cvMove = *(vec3*)param_3
                let a = G(e, cvMove);
                for off in [0, 4, 8] {
                    let v = e.r32(param_3 + off);
                    e.w32(a + off, v);
                }
            }
        } else if param_1 < 0xd7 {
            if param_1 == 0xd4 {
                // save the book power (restored by 0xc1)
                let b1 = e.r32(GD(e) + 4);
                let v = e.r32(b1 + 4);
                let l = L(e);
                e.w32(l + 0x250, v);
            }
        } else if param_1 == 0xda {
            // drop the book immediately (hero hit, cutscene, ...)
            let m = e.r32(L(e) + 0x60);
            let iVar4 = APIgameobj::GetID(e, m, NULL);
            if iVar4 != 0x7fff_fffe && 0.0 < e.rf(iVar1 + 0x70) {
                let m = e.r32(iVar1 + 0x60);
                APIgameobj::SendMessageTo(e, m, NULL, 0xd6, 100, iVar1 + 0xcc);
            }
            let b0 = GB0(e);
            e.w32(b0 + 0x30, 0);
            e.w32(iVar1 + 0x60, 0);
            if e.ri32(iVar1 + 0x54) != 0 {
                let h = e.r32(iVar1 + 0x54);
                APIgameobj::SendMessageTo(e, h, NULL, 0xdf, 0, NULL);
                e.w32(iVar1 + 0x54, 0);
            }
            APInewSound::StopSound(e, SELF, 0xf);
            APInewSound::StopSound(e, SELF, 0x2e);
            e.w32(iVar1 + 0x15c, 0);
            APIgameobj::GetShadowParams(e, SELF, auStack_60);
            let v = e.r32(auStack_60) | 4;
            e.w32(auStack_60, v);
            APIgameobj::SetShadowParams(e, SELF, auStack_60);
            let o = k(e, ID_2a000096);
            APIgameobj::SendMessageTo(e, o, NULL, 0x5c, 1, NULL);
            APIgameobj::SetContextFlag(e, SELF, 0);
            let dm = e.r32(iVar1 + 0x18);
            APIphysic::DMResetStateBits(e, dm, 1);
            APIinput::InputConfigActivate(e, 3, 0);
            APIphysic::SetVelocity(e, SELF, 0.0, 0.0, 0.0);
            if param_2 != 1 {
                let cam = k(e, ID_2a00008f);
                APIgameobj::SendMessageTo(e, cam, NULL, 0x55, 0, NULL);
            }
            let t = e.r32(iVar1 + 0x44);
            APIextra::TimerUnregisterNotify(e, SELF, t);
            if e.ri32(iVar1 + 0x2c) != 0 {
                let l = e.r32(iVar1 + 0x2c);
                APIlight::DestroyLight(e, l);
                e.w32(iVar1 + 0x2c, 0);
            }
            e.w32(iVar1 + 0x70, 0);
            APIgameobj::SetGameObjectVisible(e, SELF, 0);
            APIgameobj::EnableObject(e, SELF, 0);
            APIphysic::SetFlags(e, SELF, 1);
            APIgameobj::RemoveAllChildren(e, SELF, 0);
            let h = e.r32(L(e) + 0x1b8);
            APIgameobj::SendMessageTo(e, h, NULL, 0x6c, 0, NULL);
            if e.ri32(L(e) + 0x1c0) != 0 {
                let h = e.r32(GB0(e) + 0xc);
                APIgameobj::SendMessageTo(e, h, NULL, 0x6c, 0, NULL);
            }
            TomeDoomFUNC_Remove_NTAction(e, iVar1)?;
            TomeDoomFUNC_FixHerosMatrice(e)?;
            APIgameobj::SetNextState(e, SELF, 1);
        }
    } else if param_1 == 0x125 {
        // report the monster's remaining life in percent of 180 (x200/180) to `param_2`
        let l = L(e);
        let v = (e.rf(l + 0x70) * 200.0) / 180.0;
        e.wf(l + 0x74, v);
        if param_2 != 0 {
            APIgameobj::SendMessageTo(e, param_2, NULL, 0x125, 0, iVar1 + 0x74);
        }
    } else if param_1 < 0x126 && param_1 == 0x122 {
        // sucking particles for the monster type `param_2`
        if param_2 == 1 {
            let a = e.ri32(L(e) + 0x218);
            APIparticle::EmitterStart(e, a, 0);
            let a = e.ri32(iVar1 + 0x224);
            APIparticle::EmitterStart(e, a, 0);
        } else if param_2 == 0 || param_2 == 2 {
            let a = e.ri32(L(e) + 0x21c);
            APIparticle::EmitterStart(e, a, 0);
            let a = e.ri32(iVar1 + 0x228);
            APIparticle::EmitterStart(e, a, 0);
        } else if param_2 == 3 {
            let a = e.ri32(L(e) + 0x220);
            APIparticle::EmitterStart(e, a, 0);
            let a = e.ri32(iVar1 + 0x22c);
            APIparticle::EmitterStart(e, a, 0);
        }
        let cVar3 = APInewSound::IsPlaying(e, SELF, 0x2e);
        if cVar3 == 0 {
            APInewSound::Play(e, SELF, 0x2e, 0);
        }
    }
    Ok(0)
}

/// 0x000196c8 the book's 7 timers (L+0x34..0x4c), shadow, logic flag 0x800, input notifies 0xb..0x14
pub fn TomeDoomFUNC_Create_GameObject(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    // _ShadowParams: words +8, +0xc, +0x10, +0x18 are never written, so the original passes stack
    // garbage there; the port passes whatever the scratch stack holds
    let uStack_60 = e.alloca(0x30);
    for off in (0x34..=0x4c).step_by(4) {
        let uVar1 = APIextra::TimerCreate(e);
        e.w32(param_1 + off, uVar1);
    }
    e.w32(uStack_60, 2);
    e.w32(uStack_60 + 4, 0);
    e.w32(uStack_60 + 0x14, 0x80);
    e.w32(uStack_60 + 0x1c, 0x3dcc_cccd); // 0.1f
    e.w32(uStack_60 + 0x20, 0x4170_0000); // 15.0f
    e.w32(uStack_60 + 0x24, 0x80);
    e.w32(uStack_60 + 0x28, 0x3f40_0000); // 0.75f
    APIgameobj::SetShadowParams(e, SELF, uStack_60);
    APIgameobj::SetLogicFlag(e, SELF, 0x800);
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 0xb, 0x14);
    // shape group 1 (build 2fb4d6b9) also gets the cutscene notify
    if g(e, G_TomeDoomFUNC_Create_GameObject) == 1 {
        APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 0x16, 0x16);
    }
    e.w32(param_1 + 0x2c, 0);
    e.w32(param_1 + 0x154, 0);
    Ok(())
}

/// 0x00019878 physics: primary prim 0x84, DM 0x25 (context mask 1) and the hook DM 1 at L+0xc
/// (2.5, context mask 8) that later links the book to the monster being sucked in
pub fn TomeDoomFUNC_Create_Physic(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    let uStack_60 = e.alloca(0x20); // DM params (frame room up to auStack_40)
    let auStack_40 = e.alloca(0x20); // DM params (frame room up to uStack_30)
    APIphysic::CreatePhysic(e, SELF, 0);
    APIphysic::SetFlags(e, SELF, 0xb);
    APIphysic::SetPrimaryPrim(e, SELF, 0x84);
    APIphysic::PrimSetIntAttribBits(e, SELF, 0x84, 1, 0x80000);
    let uVar1 = APIphysic::PrimGetIntAttrib(e, SELF, 0x84, 5);
    e.w32(param_1 + 0x6c, uVar1);
    let uStack_30 = APIphysic::DMAdd(e, SELF, 0x25);
    APIphysic::DMGetParams(e, uStack_30, auStack_40);
    e.w32(auStack_40 + 4, 0x17);
    APIphysic::DMSetParams(e, uStack_30, auStack_40);
    APIphysic::DMSetContextMask(e, uStack_30, 1);
    let uVar1 = APIphysic::DMAdd(e, SELF, 1);
    e.w32(param_1 + 0xc, uVar1);
    let dm = e.r32(param_1 + 0xc);
    APIphysic::DMGetParams(e, dm, uStack_60);
    e.w32(uStack_60, 0);
    e.w32(uStack_60 + 4, 0x4020_0000); // 2.5f
    e.w32(uStack_60 + 0x14, 1);
    let dm = e.r32(param_1 + 0xc);
    APIphysic::DMSetParams(e, dm, uStack_60);
    let dm = e.r32(param_1 + 0xc);
    APIphysic::DMSetContextMask(e, dm, 8);
    Ok(())
}

/// marker in the InputDeviceAdd rows below: the pad index from game buffer 0 (+0x4c)
const PAD: i32 = -1;

/// 0x00019a68 input config 3 of the book: action 6 (sticks while sucking), 0x39/0x3b/0x3c (X, circle,
/// triangle for the button mash), 0x27 (sticks while scanning). Activated later.
/// Rows: InputDeviceAdd(3, action, r[0], r[1], r[2], r[3], r[4], r[5]), same call order as the original.
pub fn TomeDoomFUNC_Create_Input(e: &mut Engine) -> LResult<()> {
    let iVar1 = e.ri32(GB0(e) + 0x4c);
    let dev = |e: &mut Engine, act: i32, rows: &[[i32; 6]]| {
        for r in rows {
            let pad = if r[1] == PAD { iVar1 } else { r[1] };
            APIinput::InputDeviceAdd(e, 3, act, r[0], pad, r[2], r[3], r[4], r[5]);
        }
    };
    APIinput::InputConfigAdd(e, 3);
    APIinput::InputActionAdd(e, 3, 6, 0);
    dev(e, 6, &[
        [3, PAD, 0, 0, 0, 0x8000],
        [4, PAD, 0, 1, 0, 0x8001],
        [0xb, 1, 1, 0, -100, 0xcb],
        [0xb, PAD, 1, 0, -100, 0xe],
        [0xc, 1, 1, 0, 100, 0xcd],
        [0xc, PAD, 1, 0, 100, 0xf],
        [0xd, 1, 1, 1, -100, 0xd0],
        [0xd, PAD, 1, 1, -100, 0xd],
        [0xe, 1, 1, 1, 100, 200],
        [0xe, PAD, 1, 1, 100, 0xc],
    ]);
    APIinput::InputContextAdd(e, SELF, 0xb, 3, 6, 2, 0x20000);
    APIinput::InputContextSetState(e, 3, 6, 2, 2);
    APIinput::InputContextAdd(e, SELF, 0xc, 3, 6, 1, 0x10000);
    APIinput::InputContextSetState(e, 3, 6, 1, 1);
    APIinput::InputActionAdd(e, 3, 0x39, 0);
    dev(e, 0x39, &[[0x26, PAD, 0, 2, 0, 0]]);
    APIinput::InputContextAdd(e, SELF, 0x10, 3, 0x39, 0x12, 8);
    APIinput::InputContextSetState(e, 3, 0x39, 0x12, 0x10);
    APIinput::InputActionAdd(e, 3, 0x3b, 0);
    dev(e, 0x3b, &[[0x26, PAD, 0, 2, 0, 1]]);
    APIinput::InputContextAdd(e, SELF, 0x11, 3, 0x3b, 0x12, 8);
    APIinput::InputContextSetState(e, 3, 0x3b, 0x12, 0x40);
    APIinput::InputActionAdd(e, 3, 0x3c, 0);
    dev(e, 0x3c, &[[0x26, PAD, 0, 2, 0, 3]]);
    APIinput::InputContextAdd(e, SELF, 0x12, 3, 0x3c, 0x12, 8);
    APIinput::InputContextSetState(e, 3, 0x3c, 0x12, 0x100);
    APIinput::InputActionAdd(e, 3, 0x27, 0);
    dev(e, 0x27, &[
        [3, PAD, 0, 0, 0, 0x8000],
        [0xb, 1, 1, 0, -100, 0xcb],
        [0xb, PAD, 1, 0, -100, 0xe],
        [0xc, 1, 1, 0, 100, 0xcd],
        [0xc, PAD, 1, 0, 100, 0xf],
        [4, PAD, 0, 1, 0, 0x8001],
        [0xd, 1, 1, 1, -100, 0xd0],
        [0xd, PAD, 1, 1, -100, 0xd],
        [0xe, 1, 1, 1, 100, 200],
        [0xe, PAD, 1, 1, 100, 0xc],
    ]);
    APIinput::InputContextAdd(e, SELF, 0xf, 3, 0x27, 2, 0x20000);
    APIinput::InputContextSetState(e, 3, 0x27, 2, 4);
    APIinput::InputConfigActivate(e, 3, 0);
    Ok(())
}

/// 0x0001a210 the wisp-collect particle types (sparks, halos; small/medium/large) and their emitters
/// on the book (L+0x200.. types, L+0x218.. emitters). Same call order as the original: the 6
/// create+configure pairs first, then the 6 emitters.
pub fn TomeDoomFUNC_Create_Particles(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    let setters: [fn(&mut Engine, u32, u32) -> LResult<()>; 6] = [
        SetParticleWispCollect_spark_small,
        SetParticleWispCollect_spark_medium,
        SetParticleWispCollect_spark_large,
        SetParticleWispCollect_halo_small,
        SetParticleWispCollect_halo_medium,
        SetParticleWispCollect_halo_large,
    ];
    let names = [
        ("spark", "wispcollect_spark_small"),
        ("spark", "wispcollect_spark_medium"),
        ("spark", "wispcollect_spark_large"),
        ("halo", "wispcollect_halo_small"),
        ("halo", "wispcollect_halo_medium"),
        ("halo", "wispcollect_halo_large"),
    ];
    for i in 0..6u32 {
        let (tex, name) = names[i as usize];
        let (tex, name) = (e.cstr(tex), e.cstr(name));
        let uVar1 = TomeDoomFUNC_Create_TypesParticles(e, param_1 as u64, tex, name)?;
        e.wi32(param_1 + 0x200 + i * 4, uVar1);
        let p = e.r32(G(e, pParticle));
        let t = e.r32(param_1 + 0x200 + i * 4);
        setters[i as usize](e, p, t)?;
    }
    for i in 0..6u32 {
        let t = e.ri32(param_1 + 0x200 + i * 4);
        let uVar1 = APIparticle::EmitterCreate(e, t);
        e.w32(param_1 + 0x218 + i * 4, uVar1);
        let em = e.ri32(param_1 + 0x218 + i * 4);
        APIparticle::EmitterSetJoint(e, em, SELF, ALL);
    }
    Ok(())
}

/// 0x0001a5d8 new particle type with texture `param_2` and name `param_3` (`param_1` unused)
pub fn TomeDoomFUNC_Create_TypesParticles(e: &mut Engine, param_1: u64, param_2: Ptr, param_3: Ptr) -> LResult<i32> {
    let iVar1 = APIparticle::TypeCreate(e) as i32;
    APIparticle::TypeSetTexture(e, iVar1, param_2);
    APIparticle::TypeSetName(e, iVar1, param_3);
    Ok(iVar1)
}

/// 0x0001a678 module constructor, called for every new book instance (`instanceCount` = instances of
/// this module code incl. this one). The first instance registers the state/callback table, fetches the
/// API tables and the 3 shared game buffers; every instance gets its 0x254 bytes of local data.
pub fn tomedoomConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    if instanceCount == 1 {
        let fns: [(i32, crate::logic::ExtFn); 19] = [
            (0, TomeDoomST_FIRSTMOVE),
            (1, TomeDoomST_BASICMOVE),
            (2, TomeDoomST_ISOPENING),
            (3, TomeDoomST_OPEN),
            (4, TomeDoomST_FINALSUCKING),
            (5, TomeDoomST_ISCLOSING),
            (6, TomeDoomCB_ANIM_LF),
            (7, TomeDoomCB_DMSTATECHANGE),
            (0xb, TomeDoomCB_INPUT_STICK),
            (0xc, TomeDoomCB_INPUT_IDLE),
            (0xf, TomeDoomCB_INPUT_ROTATE),
            (0x10, TomeDoomCB_INPUT_X),
            (0x11, TomeDoomCB_INPUT_CIRCLE),
            (0x12, TomeDoomCB_INPUT_TRIANGLE),
            (0x13, TomeDoomCB_TIMER),
            (0x14, TomeDoomCB_ANIM),
            (0x15, TomeDoomCB_POSTLOGIC),
            (0x16, TomeDoomCB_CUTSCENE),
            (0x17, TomeDoomCB_PHYSIC),
        ];
        for (i, f) in fns {
            APIcons::SetExternalFunction(e, i, f);
        }
        // the API table pointers are stored like in the original (parity only: calls go through the traits)
        let tables: [(u32, fn(&mut Engine) -> Ptr); 10] = [
            (pAnim, |e| APIcons::GetApiAnim(e)),
            (pGameObj, |e| APIcons::GetApiGameObj(e)),
            (pInput, |e| APIcons::GetApiInput(e)),
            (pMath, |e| APIcons::GetApiMath(e)),
            (pParticle, |e| APIcons::GetApiParticle(e)),
            (pPhysic, |e| APIcons::GetApiPhysic(e)),
            (pExtra, |e| APIcons::GetApiExtra(e)),
            (pLight, |e| APIcons::GetApiLight(e)),
            (pSound, |e| APIcons::GetApiNewSound(e)),
            (pGraphics, |e| APIcons::GetApiGraphics(e)),
        ];
        for (slot, get) in tables {
            let v = get(e);
            let a = G(e, slot);
            e.w32(a, v);
        }
        for (slot, name) in [
            (pAnimMngr, "AnimMngr"),
            (pDebug, "debug"),
            (pFunctions, "ApiCFunctions"),
            (pText, "ApiText"),
            (pMiscFunc, "MiscFunc"),
        ] {
            let n = e.cstr(name);
            let v = APIcons::GetApiCustom(e, n);
            let a = G(e, slot);
            e.w32(a, v);
        }
        let v = APIcons::GetApiGame(e);
        let a = G(e, pGame);
        e.w32(a, v);
        let v = APIcons::GetApiStdlib(e);
        let a = G(e, pStdlib);
        e.w32(a, v);
        let slot = G(e, pGlobalDataTome);
        APIcons::GetGlobalData(e, slot, 0xc);
        let uVar1 = APIgameobj::GetGameBuffer(e, 0, 0x8f8, 1);
        let gd = GD(e);
        e.w32(gd, uVar1);
        let uVar1 = APIgameobj::GetGameBuffer(e, 1, 0x68, 1);
        let gd = GD(e);
        e.w32(gd + 4, uVar1);
        let uVar1 = APIgameobj::GetGameBuffer(e, 2, 0x158, 1);
        let gd = GD(e);
        e.w32(gd + 8, uVar1);
    }
    APIcons::SetMessageFunction(e, TomeDoomFUNC_MessageHandler);
    let slot = G(e, pLocalDataTome);
    APIcons::GetLocalData(e, slot, 0x254);
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 0x15, 0x15);
    LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 2, NULL, 1);
    TomeDoomFUNC_Create_Input(e)?;
    let l = L(e);
    TomeDoomFUNC_Create_Physic(e, l)?;
    let l = L(e);
    TomeDoomFUNC_Create_GameObject(e, l)?;
    let l = L(e);
    TomeDoomFUNC_Create_Particles(e, l)?;
    let l = L(e);
    TomeDoomFUNC_InitColorLight(e, l)?;
    APIcons::SetInitialState(e, 0);
    Ok(())
}

/// 0x0001ac38 module destructor (per instance): timers, light, emitters, particle types
pub fn tomedoomDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    for off in (0x34..=0x4c).step_by(4) {
        let t = e.r32(L(e) + off);
        APIextra::TimerDestroy(e, t);
    }
    if e.ri32(L(e) + 0x2c) != 0 {
        let l = e.r32(L(e) + 0x2c);
        APIlight::DestroyLight(e, l);
    }
    for iStack_2c in 0..6u32 {
        let em = e.ri32(L(e) + 0x218 + iStack_2c * 4);
        APIparticle::EmitterDestroy(e, em);
    }
    for iStack_2c in 0..6u32 {
        let t = e.ri32(L(e) + 0x200 + iStack_2c * 4);
        APIparticle::TypeDestroy(e, t);
    }
    Ok(())
}

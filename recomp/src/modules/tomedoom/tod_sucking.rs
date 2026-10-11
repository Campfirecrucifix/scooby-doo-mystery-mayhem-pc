//! Tod_Sucking.c (9 functions): the book <-> monster tug of war while sucking. The monster is pulled
//! toward the book, the hero's stick input pulls the other way, the hook link (L+0x54) is tilted
//! toward the monster.
//!
//! Local data used here (pLocalDataTome): +0x104 book pos, +0x110 monster pos, +0x11c book->monster
//! vector, +0x128 its length, +0xec blend factor, +0x12c max pull seen, +0x13c/+0x144 stick x/y,
//! +0x1c8 link mode bits, +0x1cc link angle, +0x1d0/+0x1d4/+0x1d8 link wobble.
use super::builds::*;
use super::*;
use crate::engine::*;
use crate::logic::LResult;

const ALL: u32 = 0x7fff_ffff;

/// 0x00012150 book and monster positions, the vector between them and its length.
/// Per-build shapes: group 1 (17ce1a2c) raises monster 0x2a0000bc's position by 0.4; group 2
/// (5e71b643) takes monsters with group bit 0x20 at bone 6 instead of bone 1.
pub fn TomeDoomFUNC_UpdatePosition(e: &mut Engine) -> LResult<()> {
    let l = L(e);
    APIgameobj::GetPosition(e, SELF, ALL, l + 0x104);
    let m = e.r32(L(e) + 0x60);
    let l = L(e);
    APIgameobj::GetPosition(e, m, 1, l + 0x110);
    match g(e, G_TomeDoomFUNC_UpdatePosition) {
        1 => {
            if e.ri32(L(e) + 0x60) == 0x2a00_00bc {
                let l = L(e);
                let v = e.rf(l + 0x114) + 0.4;
                e.wf(l + 0x114, v);
            }
        }
        2 => {
            let m = e.r32(L(e) + 0x60);
            let uVar1 = APIgameobj::GetGroupBitMask(e, m);
            if uVar1 & 0x20 != 0 {
                let m = e.r32(L(e) + 0x60);
                let l = L(e);
                APIgameobj::GetPosition(e, m, 6, l + 0x110);
            }
        }
        _ => {}
    }
    let l = L(e);
    APImath::CVectorSub(e, l + 0x11c, l + 0x110, l + 0x104);
    let l = L(e);
    let fVar1 = APImath::CVectorLength(e, l + 0x11c);
    let l = L(e);
    e.wf(l + 0x128, fVar1);
    Ok(())
}

/// 0x00012248 empty in the shipped game
pub fn TomeDoomFUNC_UpdateRayIntersect(e: &mut Engine) -> LResult<()> {
    Ok(())
}

/// 0x00012268 tilt the hook link (L+0x54) around X: mode bit 1 follows the monster's height,
/// bit 2 wobbles between -2 and 2, neither -> undo the tilt. Then sends the blend factor (msg 0xd5).
pub fn TomeDoomFUNC_UpdateOrientationLink(e: &mut Engine) -> LResult<()> {
    let DStack_70 = e.alloca(0x10);
    let DStack_60 = e.alloca(0x10);
    let l = L(e);
    APImath::CVectorScale(e, DStack_70, l + 0x11c, 1.0);
    e.wv3(DStack_60, [1.0, 0.0, 0.0]);
    let l = L(e);
    if e.r32(l + 0x1c8) & 1 == 0 {
        if e.r32(l + 0x1c8) & 2 == 0 {
            if e.rf(l + 0x1cc) != 0.0 {
                let v = e.rf(l + 0x1cc) * -1.0;
                e.wf(l + 0x1cc, v);
                let h = e.r32(L(e) + 0x54);
                let a = e.rf(L(e) + 0x1cc);
                APIgameobj::Rotate(e, h, DStack_60, a);
                let l = L(e);
                e.w32(l + 0x1cc, 0);
            }
        } else {
            if e.ri32(l + 0x1d8) == 0 {
                let v = e.rf(l + 0x1d0) - 0.05;
                e.wf(l + 0x1d0, v);
            } else {
                let v = e.rf(l + 0x1d0) + 0.05;
                e.wf(l + 0x1d0, v);
            }
            let fStack_44 = e.rf(l + 0x1d0);
            let fStack_48 = e.rf(l + 0x1d4);
            let mut fStack_50 = fStack_44 / fStack_48;
            fStack_50 = APImath::atanf(e, fStack_50);
            let fStack_4c = fStack_50 * 57.29578; // degrees, unused
            let l = L(e);
            let fStack_40 = fStack_50 - e.rf(l + 0x1cc);
            e.wf(l + 0x1cc, fStack_50);
            let h = e.r32(l + 0x54);
            APIgameobj::Rotate(e, h, DStack_60, fStack_40);
            let l = L(e);
            if 2.0 < e.rf(l + 0x1d0) {
                e.w32(l + 0x1d8, 0);
            } else if e.rf(l + 0x1d0) < -2.0 {
                e.w32(l + 0x1d8, 1);
            }
        }
    } else {
        let fStack_44 = e.rf(DStack_70 + 4);
        e.wf(DStack_70 + 4, 0.0);
        let fStack_48 = APImath::CVectorLength(e, DStack_70);
        let mut fStack_50 = fStack_44 / fStack_48;
        fStack_50 = APImath::atanf(e, fStack_50);
        let fStack_4c = fStack_50 * 57.29578; // degrees, unused
        let l = L(e);
        let fStack_40 = fStack_50 - e.rf(l + 0x1cc);
        e.wf(l + 0x1cc, fStack_50);
        let h = e.r32(l + 0x54);
        APIgameobj::Rotate(e, h, DStack_60, fStack_40 * -1.0);
    }
    let fVar1 = TomeDoomFUNC_UpdateBlendFactor(e)?;
    let l = L(e);
    e.wf(l + 0xec, fVar1);
    let h = e.r32(L(e) + 0x54);
    let l = L(e);
    APIgameobj::SendMessageTo(e, h, NULL, 0xd5, 0, l + 0xe4);
    Ok(())
}

/// 0x00012658 (distance - 0.75) / 7.25 clamped to 0..1
pub fn TomeDoomFUNC_UpdateBlendFactor(e: &mut Engine) -> LResult<f32> {
    let mut uStack_20 = (e.rf(L(e) + 0x128) - 0.75) / 7.25;
    if 1.0 < uStack_20 {
        uStack_20 = 1.0;
    } else if uStack_20 < 0.0 {
        uStack_20 = 0.0;
    }
    Ok(uStack_20)
}

/// 0x00012700 the monster is close enough to be captured (<= 5.6)
pub fn TomeDoomFUNC_IsGhostInRange(e: &mut Engine) -> LResult<bool> {
    Ok(e.rf(L(e) + 0x128) <= 5.6)
}

/// `1.0 / GetFrameDeltaTime()` as the original computes it (dt fetched again for every component)
fn inv_dt(e: &mut Engine) -> f32 {
    let dt = APIgame::GetFrameDeltaTime(e);
    1.0 / dt
}

/// 0x00012768 decide who wins the pull this frame. `param_1` book velocity, `param_2` monster velocity,
/// `param_3` correction toward the 5.6 ring (each a vec3 pointer). Returns what the chosen message returned.
/// Ghidra lost the GetFrameDeltaTime results here; the disassembly shows `v.c *= 1.0 / dt` per component.
pub fn TomeDoomFUNC_CalcNewDMs(e: &mut Engine, param_1: Ptr, param_2: Ptr, param_3: Ptr) -> LResult<i64> {
    let DStack_120 = e.alloca(0x10);
    let DStack_110 = e.alloca(0x10);
    let DStack_100 = e.alloca(0x10);
    let DStack_e0 = e.alloca(0x10);
    let DStack_d0 = e.alloca(0x10);
    let v = e.rv3(param_1);
    e.wv3(DStack_120, v);
    let v = e.rv3(param_2);
    e.wv3(DStack_110, v);
    let v = e.rv3(param_3);
    e.wv3(DStack_100, v);
    let fStack_f0 = APImath::CVectorLength(e, DStack_100);
    let l = L(e);
    if e.rf(l + 300) < fStack_f0 {
        e.wf(l + 300, fStack_f0);
    }
    let fStack_ec = APImath::CVectorLength(e, DStack_110);
    let fStack_e8 = APImath::CVectorLength(e, DStack_120);
    let uVar1 = APIstdlib::MyRand(e);
    if (uVar1 as i32) < 3000 {
        let d = e.rv3(DStack_100);
        e.wv3(DStack_100, [d[0] + d[0], d[1] + d[1], d[2] + d[2]]);
    }
    let lVar3: i64;
    // monster pushed back toward the ring: one frame of the opposite velocity, in its local space
    let pull_back = |e: &mut Engine, dst: Ptr| -> i64 {
        let d = e.rv3(DStack_100);
        e.wv3(dst, [-d[0], -d[1], -d[2]]);
        let x = e.rf(dst) * inv_dt(e);
        e.wf(dst, x);
        let y = e.rf(dst + 4) * inv_dt(e);
        e.wf(dst + 4, y);
        let z = e.rf(dst + 8) * inv_dt(e);
        e.wf(dst + 8, z);
        let m = e.r32(L(e) + 0x60);
        APIgameobj::InvertTransformVector(e, m, dst, dst);
        let m = e.r32(L(e) + 0x60);
        APIgameobj::SendMessageTo(e, m, NULL, 0xd6, 0x6a, dst) as i32 as i64
    };
    if 0.5 < e.rf(L(e) + 0x128) - 5.6 {
        let d = e.rv3(DStack_100);
        e.wv3(DStack_100, [d[0] * 0.5, d[1] * 0.5, d[2] * 0.5]);
        TomeDoomFUNC_MessageHandler(e, 0xd6, 0x6a, DStack_100)?;
        lVar3 = pull_back(e, DStack_e0);
    } else if fStack_e8 < fStack_ec {
        lVar3 = TomeDoomFUNC_MessageHandler(e, 0xd6, 0x6a, DStack_100)? as i64;
    } else {
        lVar3 = pull_back(e, DStack_d0);
    }
    Ok(lVar3)
}

/// 0x00012c28 book velocity (stick), monster velocity, and the pull back toward the 5.6 ring
pub fn TomeDoomFUNC_PullingEffect(e: &mut Engine) -> LResult<()> {
    let auStack_70 = e.alloca(0x10);
    let auStack_60 = e.alloca(0x10);
    let fStack_50 = e.alloca(0x10);
    TomeDoomFUNC_GetTomeVelocity(e, auStack_70 as u64)?;
    let m = e.r32(L(e) + 0x60);
    TomeDoomFUNC_GetObjectVelocity(e, auStack_60 as u64, m)?;
    let l = L(e);
    if 5.6 < e.rf(l + 0x128) {
        let v = e.rv3(l + 0x11c);
        let mut fStack_40 = APImath::sqrtf(e, v[0] * v[0] + v[1] * v[1] + v[2] * v[2]);
        let l = L(e);
        fStack_40 = (e.rf(l + 0x128) - 5.6) / fStack_40;
        let v = e.rv3(l + 0x11c);
        e.wv3(fStack_50, [v[0] * fStack_40, v[1] * fStack_40, v[2] * fStack_40]);
    } else {
        e.wv3(fStack_50, [0.0, 0.0, 0.0]);
    }
    TomeDoomFUNC_CalcNewDMs(e, auStack_70, auStack_60, fStack_50)?;
    Ok(())
}

/// 0x00012dd8 the stick direction (cvJoyDep) minus its component along the book's right axis,
/// written to `param_1` (zero when the stick is idle). Returns `param_1`.
/// Ghidra showed the y component as 0.0; the disassembly copies all of cvJoyDep and uses its y.
pub fn TomeDoomFUNC_GetTomeVelocity(e: &mut Engine, param_1: u64) -> LResult<u64> {
    let auStack_70 = e.alloca(0x10); // zero vector (fp+0)
    let uStack_60 = e.alloca(0x10); // copy of cvJoyDep (fp+0x10)
    let aDStack_50 = e.alloca(0x20); // look-right (fp+0x20)
    let puVar2 = param_1 as Ptr;
    e.wv3(auStack_70, [0.0, 0.0, 0.0]);
    let l = L(e);
    if e.rf(l + 0x13c) == 0.0 && e.rf(L(e) + 0x144) == 0.0 {
        let z = e.rv3(auStack_70);
        e.wv3(puVar2, z);
    } else {
        let j = G(e, cvJoyDep);
        let v = e.rv3(j);
        e.wv3(uStack_60, v);
        if e.rf(uStack_60) == 0.0 && e.rf(uStack_60 + 8) == 0.0 {
            let z = e.rv3(auStack_70);
            e.wv3(puVar2, z);
        } else {
            APIgameobj::GetLookRight(e, SELF, ALL, aDStack_50);
            let r = e.rv3(aDStack_50);
            let v = e.rv3(uStack_60);
            let fVar3 = v[0] * r[0] + v[1] * r[1] + v[2] * r[2];
            e.wv3(uStack_60, [v[0] - r[0] * fVar3, v[1] - r[1] * fVar3, v[2] - r[2] * fVar3]);
            let v = e.rv3(uStack_60);
            e.wv3(puVar2, v);
        }
    }
    Ok(param_1)
}

/// 0x00012fc0 the monster's escape velocity: its look-at scaled by its pull strength (L+0xd0 * 3.25)
/// and remaining life (0.3..1.0), minus the component along the book's right axis. Returns `param_1`.
pub fn TomeDoomFUNC_GetObjectVelocity(e: &mut Engine, param_1: u64, param_2: u32) -> LResult<u64> {
    let DStack_70 = e.alloca(0x10);
    let aDStack_50 = e.alloca(0x20);
    let l = L(e);
    let fStack_60 = (e.rf(l + 0x70) / e.rf(l + 200)) * 0.7 + 0.3;
    APIgameobj::GetLookAt(e, param_2, ALL, DStack_70);
    let s = e.rf(L(e) + 0xd0) * 3.25 * fStack_60;
    APImath::CVectorScale(e, DStack_70, DStack_70, s);
    APIgameobj::GetLookRight(e, SELF, ALL, aDStack_50);
    let (d, r) = (e.rv3(DStack_70), e.rv3(aDStack_50));
    let fVar1 = d[0] * r[0] + d[1] * r[1] + d[2] * r[2];
    let p = param_1 as Ptr;
    e.wv3(p, [d[0] - r[0] * fVar1, d[1] - r[1] * fVar1, d[2] - r[2] * fVar1]);
    Ok(param_1)
}

//! CameraFree.c (follow camera, attractor) part 1: helpers, attractor, orientation (WP-L1).
#![allow(non_snake_case, unused_variables, unused_mut)]
use super::*;

pub(super) type V3 = [f32; 3];
pub(super) fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
pub(super) fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
pub(super) fn scl(a: V3, s: f32) -> V3 {
    [a[0] * s, a[1] * s, a[2] * s]
}
pub(super) fn dot(a: V3, b: V3) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
pub(super) fn cross(a: V3, b: V3) -> V3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
/// `v * (1/sqrtf(|v|^2))` through APImath::sqrtf
pub(super) fn normz(e: &mut Engine, v: V3) -> V3 {
    let s = APImath::sqrtf(e, dot(v, v));
    scl(v, 1.0 / s)
}
pub(super) fn gf(e: &Engine, a: u32) -> f32 {
    e.rf(G(e, a))
}
pub(super) fn sf(e: &mut Engine, a: u32, v: f32) {
    let p = G(e, a);
    e.wf(p, v);
}
pub(super) fn gu(e: &Engine, a: u32) -> u32 {
    e.r32(G(e, a))
}
pub(super) fn su(e: &mut Engine, a: u32, v: u32) {
    let p = G(e, a);
    e.w32(p, v);
}
pub(super) fn gv(e: &Engine, a: u32) -> V3 {
    e.rv3(G(e, a))
}
pub(super) fn sv(e: &mut Engine, a: u32, v: V3) {
    let p = G(e, a);
    e.wv3(p, v);
}
/// API out-parameter helpers (DTVector3d on the module stack)
pub(super) fn pos(e: &mut Engine, id: u32, bone: u32) -> V3 {
    let t = e.alloca(0x10);
    APIgameobj::GetPosition(e, id, bone, t);
    e.rv3(t)
}
pub(super) fn lookup(e: &mut Engine, id: u32, bone: u32) -> V3 {
    let t = e.alloca(0x10);
    APIgameobj::GetLookUp(e, id, bone, t);
    e.rv3(t)
}
pub(super) fn lookat(e: &mut Engine, id: u32, bone: u32) -> V3 {
    let t = e.alloca(0x10);
    APIgameobj::GetLookAt(e, id, bone, t);
    e.rv3(t)
}
pub(super) fn mget(e: &mut Engine, f: fn(&mut Engine, Ptr, Ptr), m: Ptr) -> V3 {
    let t = e.alloca(0x10);
    f(e, t, m);
    e.rv3(t)
}
pub(super) fn rot(e: &mut Engine, m: Ptr, axis: V3, a: f32) {
    let t = e.alloca(0x10);
    e.wv3(t, axis);
    APImath::CMatRotate(e, m, t, a);
}
pub(super) fn xform_obj(e: &mut Engine, id: u32, v: V3) -> V3 {
    let t = e.alloca(0x10);
    e.wv3(t, v);
    APIgameobj::TransformVector(e, id, t, t);
    e.rv3(t)
}
pub(super) fn set_trans(e: &mut Engine, id: u32, v: V3) {
    let t = e.alloca(0x10);
    e.wv3(t, v);
    APIgameobj::SetTranslation(e, id, t);
}
/// oRayIntersectIn {start(+0), dir(+0xc)} -> CheckRayIntersection(&In, &Out)
pub(super) fn ray(e: &mut Engine, start: V3, dir: V3) -> u32 {
    let ri = G(e, oRayIntersectIn);
    e.wv3(ri, start);
    e.wv3(ri + 0xc, dir);
    let ro = G(e, oRayIntersectOut);
    APIphysic::CheckRayIntersection(e, ri, ro)
}
/// oRayIntersectOut: hit point (+0), normal (+0xc)
pub(super) fn ray_hit(e: &Engine) -> (V3, V3) {
    let ro = G(e, oRayIntersectOut);
    (e.rv3(ro), e.rv3(ro + 0xc))
}
/// oAttractorParam fields (0x10288): pos +0, +0xc (bc), +0x10 smooth in (c0), +0x14 (c4), +0x18 (c8),
/// +0x1c (cc), +0x20 (d0), +0x24 (d4), flags +0x28 (d8), target +0x2c (dc), target pos +0x30 (e0)
pub(super) const ATT: u32 = 0x10288;

/// 0x00011270: piecewise-linear lookup in fCameraCurve (6 rows of {hauteur, distance})
pub fn GetItem(e: &Engine, param_1: f32, param_2: u32, param_3: u32) -> f32 {
    let c = |i: u32| e.rf(G(e, fCameraCurve) + i);
    let mut f = 0.0;
    let mut i = 0u32;
    loop {
        if 5 < i {
            break;
        }
        if param_1 < c(param_2 * 4 + i * 8) {
            if i == 0 {
                return c(param_3 * 4);
            }
            f = c(param_3 * 4 + (i - 1) * 8)
                + (c(param_3 * 4 + i * 8) - c(param_3 * 4 + (i - 1) * 8))
                    * ((param_1 - c(param_2 * 4 + (i - 1) * 8)) / (c(param_2 * 4 + i * 8) - c(param_2 * 4 + (i - 1) * 8)));
            break;
        }
        i += 1;
    }
    // LAB_0001175c
    if i == 6 {
        f = c(param_3 * 4 + 0x28);
    }
    f
}
/// 0x000114f8 GetVDistance
pub fn GetVDistance(e: &Engine, p: f32) -> f32 {
    GetItem(e, p, 1, 0)
}
/// 0x00011558 GetVHauteur
pub fn GetVHauteur(e: &Engine, p: f32) -> f32 {
    GetItem(e, p, 0, 1)
}

/// 0x000115b8 CanIMoveCam
pub fn CanIMoveCam(e: &Engine) -> bool {
    e.r32(LB(e) + 0x4c) & 0x1000_0000 == 0 && e.r32(GB0(e) + 0x14) & 0xa00 == 0
}

/// 0x00011620: three 1.5 m rays (at/right/left of the camera, from param_3) smoothed into cvCamWallCheck
pub fn CheckWall(e: &mut Engine, param_3: V3) {
    let mut d = [scl(gv(e, 0x101b0), 1.5), scl(gv(e, 0x101c0), 1.5), scl(gv(e, 0x101d0), 1.5)];
    for v in d.iter_mut() {
        if ray(e, param_3, *v) != 0 {
            *v = sub(ray_hit(e).0, param_3);
        }
    }
    let w = gv(e, cvCamWallCheck);
    let mut n = [0.0; 3];
    for k in 0..3 {
        n[k] = w[k] - (w[k] - (d[0][k] + d[1][k] + d[2][k]) / 3.0) * 0.1;
    }
    sv(e, cvCamWallCheck, n);
}

/// 0x00011a28 CamSetCameaModeTour
pub fn CamSetCameaModeTour(e: &mut Engine, p: V3) {
    sv(e, cvCameraTourCenter, p);
    let f = gu(e, CameraFlags) | 0x800;
    su(e, CameraFlags, f);
    su(e, fCameraTourTimeIn, 0);
}
/// 0x00011a90 CamResetCameaModeTour
pub fn CamResetCameaModeTour(e: &mut Engine) {
    let f = gu(e, CameraFlags) & 0xffff_f7ff;
    su(e, CameraFlags, f);
}
/// 0x00011ad0 ChangeAttractorTarget
pub fn ChangeAttractorTarget(e: &mut Engine, p: u32) {
    su(e, ATT + 0x2c, p);
    let f = gu(e, ATT + 0x28) | 0x10;
    su(e, ATT + 0x28, f);
}
/// 0x00011b18 ResetAttractorTarget
pub fn ResetAttractorTarget(e: &mut Engine) {
    let f = gu(e, ATT + 0x28) & 0xffff_ffef;
    su(e, ATT + 0x28, f);
}

/// 0x00011b58
pub fn SetAttractorParam(e: &mut Engine, param_1: f32, param_2: f32, param_3: f32, param_4: u32, param_5: i32, param_6: u32, param_7: Ptr) -> u32 {
    if param_7 == NULL {
        if param_5 == 0 {
            return 0;
        }
        if APIgameobj::GetID(e, param_5 as u32, NULL) == 0x7fff_fffe {
            return 0;
        }
        let a = G(e, ATT);
        APIgameobj::GetPosition(e, param_5 as u32, 0x7fff_ffff, a);
    } else {
        let v = e.rv3(param_7);
        sv(e, ATT, v);
    }
    let s30 = if param_4 & 8 != 0 { 0.0 } else { param_1 };
    if param_4 & 0x10 != 0 && APIgameobj::GetID(e, param_6, NULL) != 0x7fff_fffe {
        su(e, ui32ActualTargetID, param_6);
    }
    sf(e, ATT + 0x10, s30);
    sf(e, ATT + 0xc, param_3);
    sf(e, ATT + 0x14, s30);
    sf(e, ATT + 0x18, s30);
    sf(e, ATT + 0x1c, param_2);
    sf(e, ATT + 0x20, param_2);
    sf(e, ATT + 0x24, param_2);
    su(e, ATT + 0x28, param_4);
    su(e, ATT + 0x2c, param_6);
    let t = gu(e, ui32ActualTargetID);
    let p = G(e, ATT + 0x30);
    APIgameobj::GetPosition(e, t, 0x7fff_ffff, p);
    su(e, ui32Substate, 2);
    1
}
/// 0x00011d50 ReleaseAttractor
pub fn ReleaseAttractor(e: &mut Engine) {
    su(e, ui32Substate, 4);
}
/// 0x00011d80 IsAttractorReleased
pub fn IsAttractorReleased(e: &Engine) -> bool {
    gu(e, ui32Substate) != 3
}
/// 0x00011dc0 IsAttractorSetted
pub fn IsAttractorSetted(e: &Engine) -> bool {
    gu(e, ui32Substate) == 2
}
/// 0x00011e00 IsFollowMode
pub fn IsFollowMode(e: &Engine) -> bool {
    gu(e, ui32Substate) == 1
}
/// 0x00011e40 IsFlagsAreSetted
pub fn IsFlagsAreSetted(e: &Engine, p: u32) -> u32 {
    gu(e, ATT + 0x28) & p
}

/// 0x00011e80: turn the camera bone matrix to look at the target (+ look-up * 1.2 + cvTrackEnAvant)
pub fn ReplaceOrientation(e: &mut Engine) {
    let m = APIgameobj::GetBoneLMat(e, SELF_CAM, 0x7fff_ffff) as u32;
    let t = gu(e, ui32ActualTargetID);
    let mut p90 = pos(e, t, 0x7fff_ffff);
    let up = lookup(e, t, 0x7fff_ffff);
    let p80 = mget(e, APImath::CMatGetPos, m);
    p90 = add(add(p90, gv(e, cvTrackEnAvant)), scl(up, 1.2));
    let mut d = sub(p90, p80);
    if dot(d, d) == 0.0 {
        return;
    }
    let at = mget(e, APImath::CMatGetAt, m);
    d = normz(e, d);
    let c = cross(at, d);
    let mut a0 = dot(at, d);
    let l = APImath::sqrtf(e, dot(c, c));
    if 1e-6 < l {
        let _ = mget(e, APImath::CMatGetRight, m); // DStack_90 overwritten, unused
        a0 = a0.clamp(-1.0, 1.0);
        let ang = APImath::acosf(e, a0);
        let ax = normz(e, c);
        rot(e, m, ax, ang);
        APIgameobj::SetBoneLMat(e, SELF_CAM, 0x7fff_ffff, m);
        let at = mget(e, APImath::CMatGetAt, m);
        let y = [0.0, 1.0, 0.0];
        let u = normz(e, sub(y, scl(at, dot(y, at))));
        let r = cross(u, at);
        let t = e.alloca(0x10);
        e.wv3(t, u);
        APImath::CMatSetUp(e, m, t);
        e.wv3(t, r);
        APImath::CMatSetRight(e, m, t);
        APIgameobj::SetBoneLMat(e, SELF_CAM, 0x7fff_ffff, m);
    }
}

fn dead(v: f32) -> f32 {
    // |v| < 0.2 -> 0, else (v -+ 0.2) * 1.25 (fabs via myfabs is |v|)
    if v.abs() < 0.2 {
        0.0
    } else if 0.0 < v {
        (v - 0.2) * 1.25
    } else {
        (v + 0.2) * 1.25
    }
}

/// 0x00012540: right stick (action 0x38) rotates `param_1` (CameraFlags & 0x200)
pub fn RotateCameraFixWithJoy(e: &mut Engine, param_1: Ptr) -> bool {
    if gu(e, CameraFlags) & 0x200 == 0 || !CanIMoveCam(e) {
        sf(e, 0x101e0, 0.0);
        sf(e, 0x101dc, 0.0);
        return false;
    }
    let x = -APIinput::InputActionGetAnalogX(e, 4, 0x38);
    let y = APIinput::InputActionGetAnalogY(e, 4, 0x38);
    let (s5c, s58) = (dead(x), dead(y));
    let mut sy = gf(e, 0x101e0);
    let mut sx = gf(e, 0x101dc);
    sy -= (sy - s58) * 0.3;
    sx -= (sx - s5c) * 0.3;
    if sy.abs() < 0.001 {
        sy = 0.0;
    }
    if sx.abs() < 0.001 {
        sx = 0.0;
    }
    sf(e, 0x101e0, sy);
    sf(e, 0x101dc, sx);
    if sx == 0.0 && sy == 0.0 {
        return false;
    }
    let w = APImath::CMatGetWorkMatrix(e);
    APImath::CMatSetIdentity(e, w);
    if sx != 0.0 {
        rot(e, w, [0.0, 1.0, 0.0], sx);
    }
    if sy != 0.0 {
        let r = mget(e, APImath::CMatGetRight, w);
        rot(e, w, r, sy);
    }
    APImath::CMatTransformVector(e, param_1, param_1, w);
    true
}

fn approach(e: &mut Engine, at: V3, dir: V3, dt: f32) -> V3 {
    normz(e, add(at, scl(dir, dt * 17.0)))
}

/// 0x00012978: follow orientation; param_2 = camera translation delta of this frame
pub fn ReplaceOrientationFollow(e: &mut Engine, param_1: f32, param_2: V3) {
    let m = APIgameobj::GetBoneLMat(e, SELF_CAM, 0x7fff_ffff) as u32;
    let t = gu(e, ui32ActualTargetID);
    let j = gu(e, ui32FollowJoint);
    let mut p = pos(e, t, j);
    let up = lookup(e, t, j);
    let o = e.r32(GB0(e) + 0x30);
    if o != 0 {
        let a = pos(e, o, 0x7fff_ffff);
        p = scl(add(p, a), 0.5);
    }
    let cp = mget(e, APImath::CMatGetPos, m);
    p = add(add(p, gv(e, cvTrackEnAvant)), scl(up, gf(e, fTargetCameraFollowWork)));
    let w = gv(e, cvCamWallCheck);
    let mut d = add(add(sub(p, add(cp, param_2)), w), w);
    if 0.001 < dot(d, d) {
        d = normz(e, d);
        let mut at = mget(e, APImath::CMatGetAt, m);
        let ss = gu(e, ui32Substate);
        let fl = gu(e, ATT + 0x28);
        let cf = gu(e, CameraFlags);
        if (ss == 1 && fl & 0x100 != 0 && 0.0 < gf(e, ATT + 0x20)) || (ss == 3 && fl & 0x80 != 0 && 0.0 < gf(e, ATT + 0x14)) {
            let s8c;
            if ss == 1 {
                s8c = gf(e, ATT + 0x20);
                let v = s8c - param_1;
                sf(e, ATT + 0x20, v);
                if v <= 0.0 {
                    su(e, ATT + 0x28, gu(e, ATT + 0x28) & 0xffff_feff);
                }
            } else {
                s8c = gf(e, ATT + 0x14);
                let v = s8c - param_1;
                sf(e, ATT + 0x14, v);
                if v <= 0.0 {
                    su(e, ATT + 0x28, gu(e, ATT + 0x28) & 0xffff_ff7f);
                }
            }
            let c = cross(at, d);
            if 0.001 < dot(c, c) {
                let c = normz(e, c);
                let a = dot(at, d).clamp(-1.0, 1.0);
                let a = APImath::acosf(e, a);
                let step = (a / s8c) * param_1;
                let r = if step <= a { step } else { a };
                rot(e, m, c, r);
                at = mget(e, APImath::CMatGetAt, m);
            } else {
                at = approach(e, at, d, param_1);
            }
            su(e, CameraFlags, gu(e, CameraFlags) & 0xffff_ffbb);
        } else if cf & 0x44 == 0 {
            at = approach(e, at, d, param_1);
        } else {
            at = d;
        }
        let f0 = e.alloca(0x10);
        e.wv3(f0, d);
        if RotateCameraFixWithJoy(e, f0) {
            at = e.rv3(f0);
        }
        let y = [0.0, 1.0, 0.0];
        if 0.9999 < (at[0] * 0.0 + at[1] * 1.0 + at[2] * 0.0).abs() {
            at = lookup(e, SELF_CAM, 0x7fff_ffff);
        }
        let (pa, pu) = (e.alloca(0x10), e.alloca(0x10));
        e.wv3(pa, at);
        e.wv3(pu, y);
        APImath::CMatLookAtAlignUp(e, m, pa, pu);
        let at = mget(e, APImath::CMatGetAt, m);
        if at[1] < -0.53 {
            let ay = at[1].max(-1.0);
            let a = APImath::asinf(e, -0.53);
            let b = APImath::asinf(e, ay);
            let r = mget(e, APImath::CMatGetRight, m);
            rot(e, m, r, -(a - b));
        }
    }
    APIgameobj::SetBoneLMat(e, SELF_CAM, 0x7fff_ffff, m);
}

/// 0x00013590 UpdateLastFollow
pub fn UpdateLastFollow(e: &mut Engine, param_1: f32, p: V3) {
    let l = gv(e, cvLastFollowY);
    let x = l[0] - (l[0] - p[0]) * param_1 * 6.0;
    let z = l[2] - (l[2] - p[2]) * param_1 * 6.0;
    let y = p[1] + gf(e, fHauteurCameraFollow);
    sv(e, cvLastFollowY, [x, y, z]);
}
/// 0x00013650 SetLastFollow
pub fn SetLastFollow(e: &mut Engine, p: V3) {
    let y = p[1] + gf(e, fHauteurCameraFollow);
    sv(e, cvLastFollowY, [p[0], y, p[2]]);
}

/// 0x000136b0: camera 3.6 m behind the target (ray-clipped), looking at it
pub fn ResetCamera(e: &mut Engine) -> u32 {
    let t = gu(e, ui32ActualTargetID);
    let p = pos(e, t, 0x7fff_ffff);
    SetLastFollow(e, p);
    su(e, fDistanceCameraFollowCollision, 0);
    su(e, fDistanceCameraFollowTarget, 0x4066_6666);
    sf(e, fDistanceCameraFollow, 3.6);
    let h = GetVHauteur(e, 3.6);
    sf(e, fHauteurCameraFollow, h);
    su(e, fTargetCameraFollowWork, 0x3f99_999a);
    su(e, fTargetCameraFollow, 0x3f99_999a);
    sf(e, fHauteurCameraFollowTarget, h);
    let mut c = add(xform_obj(e, t, [0.0, h, -3.6]), p);
    let s = [p[0], p[1] + 1.4, p[2]];
    if ray(e, s, sub(c, s)) != 0 {
        let (hp, n) = ray_hit(e);
        c = add(hp, scl(n, 0.4));
    }
    for a in [cvCamImpulse, cvCamImpulseWall, cvTrackEnAvant] {
        sv(e, a, [0.0; 3]);
    }
    set_trans(e, SELF_CAM, c);
    ReplaceOrientation(e);
    1
}

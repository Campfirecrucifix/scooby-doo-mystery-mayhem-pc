//! CameraFree.c part 2 (0x000142c8..0x00017ed7): joystick orbit, occlusion, distance, back turn,
//! cameraST_CAMERA_FOLLOW, Camera_BaseCB_PHYSIC / COLLIDE (WP-L1).
#![allow(non_snake_case, unused_variables, unused_mut, unused_assignments)]
use super::free::*;
use super::*;

// function statics (l03 module symtab, e.g. fSmootAngle.72 at 0x1e4)
pub const cvOldVec_71: u32 = 0x10390; // DAT_00010398 = z (from the code)
pub const fSmootAngle_72: u32 = 0x101e4;
pub const oOriginalHeroPos_86: u32 = 0x103a0; // y/z = DAT_000103a4/a8 (from the code)
pub const fTimeAfterHook_79: u32 = 0x101e8;
pub const cvCamOldPos_87: u32 = 0x103b0;
pub const uiNbFrames_88: u32 = 0x101ec;
pub const fAttractorSmootBase_89: u32 = 0x103bc;
pub const fAttractorSmootIn_90: u32 = 0x103c0;
pub const fAttractorSmootOut_91: u32 = 0x103c4;
pub const fAttractorSpeed_92: u32 = 0x103c8;

/// 0x000142c8 RotateCameraAroundTargetWithJoy; param_2 (target pos) unused; param_3 = frame move
pub fn RotateCameraAroundTargetWithJoy(e: &mut Engine, param_1: f32, param_3: &mut V3) {
    if !CanIMoveCam(e) {
        return;
    }
    let x = APIinput::InputActionGetAnalogX(e, 4, 0x38);
    let s = gf(e, fSmootAngle_72);
    let f3 = s - (s - x) * 0.35;
    sf(e, fSmootAngle_72, f3);
    if 0.015 < APImath::myfabs(e, f3) {
        let p = pos(e, SELF_CAM, 0x7fff_ffff);
        let (lf, w) = (gv(e, cvLastFollowY), gv(e, cvCamWallCheck));
        let mut a = p[0] - (lf[0] + w[0] + w[0]);
        let mut c = p[2] - (lf[2] + w[2] + w[2]);
        let mut b = 0.0f32;
        if a * a + 0.0 + c * c < 4.0 {
            if (a.to_bits() | c.to_bits()) & 0x7fff_ffff == 0 {
                a = 2.0;
                b = 0.0;
                c = 0.0;
            } else {
                let k = 2.0 / APImath::sqrtf(e, a * a + 0.0 + c * c);
                a *= k;
                b *= k;
                c *= k;
            }
        }
        let s70 = f3 * 2.5 * param_1;
        let ac = APImath::acosf(e, s70);
        let s6c = APImath::sinf(e, ac);
        let nx = a * s6c - -c * s70;
        let nz = -a * s70 + c * s6c;
        let (f3, f4, f5) = (nx - a, 0.0 - b, nz - c);
        let o = gv(e, cvOldVec_71);
        let mut l = APImath::sqrtf(e, dot(o, o));
        let v = e.alloca(0x10);
        APIphysic::GetVelocity(e, SELF_CAM, v);
        let v = e.rv3(v);
        l -= APImath::sqrtf(e, dot(v, v)) * param_1;
        let d = gf(e, fDistanceCameraFollow);
        if 0.0 < l && l < d {
            sf(e, fDistanceCameraFollow, d - l);
        }
        *param_3 = add(*param_3, [f3, f4, f5]);
        sv(e, cvOldVec_71, [f3, f4, f5]);
        su(e, CameraFlags, gu(e, CameraFlags) | 0x40);
        let h = gf(e, cvCamImpulse + 4);
        sf(e, cvCamImpulse + 4, h * 0.5);
    } else {
        su(e, CameraFlags, gu(e, CameraFlags) & 0xffff_ffbf);
        sv(e, cvOldVec_71, [0.0; 3]);
    }
}

/// 0x00014860. p3 = frame move, p4 = cam->target-head vector (ray dir), p5 = cam pos (ray start),
/// p6 = cam->last-follow vector, p7 = target head pos
pub fn IsSomethingBetwinCamAndTarget(e: &mut Engine, param_1: f32, param_2: f32, param_3: &mut V3, param_4: V3, param_5: V3, param_6: V3, param_7: V3) {
    let ro = G(e, oRayIntersectOut);
    let h1 = ray(e, param_5, param_4);
    // aRStack_d0: first hit kept on the stack (pt +0, normal +0xc)
    let (p1, n1) = ray_hit(e);
    let h2 = ray(e, param_7, scl(param_4, -1.0));
    let (p2, n2) = ray_hit(e);
    let cf = gu(e, CameraFlags) & 0xffff_fff7;
    su(e, CameraFlags, cf);
    let zero_imp = |e: &mut Engine, param_3: &mut V3| {
        sf(e, cvCamImpulse, 0.0);
        sf(e, cvCamImpulse + 4, 0.0);
        sf(e, cvCamImpulse + 8, 0.0);
        *param_3 = [0.0; 3];
    };
    if h1 == 0 || h2 == 0 || !CanIMoveCam(e) {
        if h2 == 0 || !CanIMoveCam(e) {
            if param_2 != 0.0 || !CanIMoveCam(e) {
                let s40 = if e.r32(GB0(e) + 0x14) & 0xa00 == 0 { gf(e, fSpeedFactor) } else { 2.0 };
                let mut v = gf(e, cvCamImpulse + 4) * (0.99 - s40 * 0.03);
                if APImath::myfabs(e, v) < 0.001 {
                    v = 0.0;
                }
                sf(e, cvCamImpulse + 4, v);
                let c = gf(e, fDistanceCameraFollowCollision) * (0.99 - s40 * 0.03);
                sf(e, fDistanceCameraFollowCollision, c);
                su(e, CameraFlags, gu(e, CameraFlags) & 0xffff_fffe);
            }
        } else {
            set_trans(e, SELF_CAM, add(scl(n2, 0.2), p2));
            zero_imp(e, param_3);
        }
    } else {
        let s = add(n1, n2);
        if dot(s, s) < 0.0001 || gu(e, CameraFlags) & 0x40 != 0 {
            let np = add(scl(n2, 0.2), p2);
            let w = gv(e, cvCamWallCheck);
            let d = sub(add(param_7, w), np);
            su(e, CameraFlags, gu(e, CameraFlags) | 8);
            let l = APImath::sqrtf(e, d[0] * d[0] + d[2] * d[2]);
            sf(e, fDistanceCameraFollow, l);
            set_trans(e, SELF_CAM, np);
            zero_imp(e, param_3);
        } else {
            su(e, CameraFlags, gu(e, CameraFlags) | 8);
            let _ = normz(e, param_6); // fStack_80.. overwritten below
            let t = gu(e, ui32ActualTargetID);
            let j = gu(e, ui32FollowJoint);
            let tp = pos(e, t, j);
            let c = cross(param_6, s);
            let c2 = cross(c, param_6);
            let k = 1.0 / APImath::sqrtf(e, dot(c2, c2));
            // fStack_cc (hit1.y) += n1.y
            let cc = p1[1] + n1[1];
            if (0.0 <= n1[1] && param_5[1] < cc) || (n1[1] < 0.0 && cc < param_5[1]) {
                sf(e, cvCamImpulse + 4, (cc - tp[1]) - gf(e, fHauteurCameraFollowTarget));
            }
            if 0.8 < c2[1] * k || c2[1] * k < -0.8 {
                let d = gf(e, fDistanceCameraFollow);
                sf(e, fDistanceCameraFollowCollision, gf(e, fDistanceCameraFollowTarget) - d * 0.85);
                sf(e, fDistanceCameraFollow, d * 0.85);
            } else {
                let i = gf(e, cvCamImpulse) + c2[0] * k * 25.0 * param_1;
                sf(e, cvCamImpulse, i);
                let i = gf(e, cvCamImpulse + 8) + c2[2] * k * 25.0 * param_1;
                sf(e, cvCamImpulse + 8, i);
            }
        }
    }
    let _ = ro;
}

/// 0x00015228
pub fn ChangeCameraDistanceFromTargetWithJoy(e: &mut Engine, param_1: f32, param_2: f32) {
    let mut y = APIinput::InputActionGetAnalogY(e, 4, 0x38);
    if !CanIMoveCam(e) {
        return;
    }
    let gb3 = e.r32(GDB(e) + 8);
    if e.r32(gb3 + 0x14) != 0 {
        y = -y;
    }
    su(e, CameraFlags, gu(e, CameraFlags) & 0xffff_fffb);
    let far = gf(e, fCameraCurve + 0x28);
    if e.r32(GB0(e) + 0x30) == 0 {
        if 0.5 < APImath::myfabs(e, y) {
            let mut t = gf(e, fDistanceCameraFollowTarget) + y * param_1 * 1.5;
            if t < 2.5 {
                t = 2.5;
            } else if far < t {
                t = far;
            }
            sf(e, fDistanceCameraFollowTarget, t);
            su(e, CameraFlags, gu(e, CameraFlags) | 4);
        } else if (param_2 != 0.0 || 0.0 < gf(e, fTimeAfterHook_79)) && gu(e, CameraFlags) & 0x548 == 0 {
            let mut t = gf(e, fDistanceCameraFollowTarget);
            if t < 3.6 {
                t += 0.1;
            }
            if 3.6999998 < t {
                t -= 0.1;
            }
            sf(e, fDistanceCameraFollowTarget, t);
        }
    } else {
        sf(e, fDistanceCameraFollowTarget, far);
        sf(e, fTimeAfterHook_79, 2.0);
    }
    let h = gf(e, fTimeAfterHook_79);
    if 0.0 < h {
        sf(e, fTimeAfterHook_79, h - param_1);
    }
}

/// 0x000154f0. p1 = horizontal target speed, p2 = dt, p4 = cam look-at, p5 = target look-up,
/// p6 = target look-at, p7 = cam->target head vector
pub fn TurnCameraOnBackOfTarget(e: &mut Engine, param_1: f32, param_2: f32, param_3: &mut V3, param_4: V3, param_5: V3, param_6: V3, param_7: V3) {
    let gb0 = GB0(e);
    let cf = gu(e, CameraFlags);
    if !(cf & 0x41 == 0 && e.r32(gb0 + 0x30) == 0 && e.r32(gb0 + 0x14) & 1 == 0) {
        return;
    }
    let d = dot(param_4, param_5);
    let v80 = normz(e, sub(param_4, scl(param_5, d)));
    let u70 = if cf & 0x800 == 0 {
        param_6
    } else {
        let t = gu(e, ui32ActualTargetID);
        let j = gu(e, ui32FollowJoint);
        let p = pos(e, t, j);
        let c = gv(e, cvCameraTourCenter);
        let (x, z) = (c[0] - p[0], c[2] - p[2]);
        // quirk: the y lane (uStack_70._4_4_) keeps the stale upper word = 0
        let k = 1.0 / APImath::sqrtf(e, x * x + 0.0 + z * z);
        [x * k, 0.0, z * k]
    };
    let a = dot(v80, u70).clamp(-1.0, 1.0);
    if a < 0.95 || cf & 0x800 != 0 || e.r32(gb0 + 0x14) & 0xa00 != 0 {
        let ang = APImath::acosf(e, a);
        let sp = if cf & 0x800 == 0 || e.r32(gb0 + 0x30) != 0 {
            if cf & 0x801 == 0 && e.r32(gb0 + 0x30) == 0 && e.r32(gb0 + 0x14) & 0xa00 == 0 {
                param_1 * 0.15
            } else {
                2.8
            }
        } else if gf(e, fCameraTourTimeIn) < 1.0 {
            let t = gf(e, fCameraTourTimeIn) + param_2;
            sf(e, fCameraTourTimeIn, t);
            t * 4.2
        } else {
            4.2
        };
        let mut ax = cross(u70, v80);
        if (ax[0].to_bits() | ax[1].to_bits() | ax[2].to_bits()) & 0x7fff_ffff == 0 {
            ax = [0.0, 1.0, 0.0];
        } else {
            ax = normz(e, ax);
        }
        let w = APImath::CMatGetWorkMatrix(e);
        APImath::CMatSetIdentity(e, w);
        rot(e, w, ax, ang * sp * param_2);
        let (o, i) = (e.alloca(0x10), e.alloca(0x10));
        e.wv3(i, param_7);
        APImath::CMatTransformVector(e, o, i, w);
        let r = e.rv3(o);
        *param_3 = add(*param_3, sub(r, param_7));
    }
}

/// 0x00015c70 state 3 (CAMERA_FOLLOW) of the follow camera; Substate 0 init, 1 follow,
/// 2/3 attractor, 4 attractor release.
pub fn cameraST_CAMERA_FOLLOW(e: &mut Engine) -> LResult<u64> {
    let dt = APIgame::GetFrameDeltaTime(e);
    let mut ss = gu(e, ui32Substate);
    let mut fallthrough = false;
    if ss == 0 {
        let t = e.r32(GB0(e) + 8);
        su(e, ui32ActualTargetID, t);
        let ri = G(e, oRayIntersectIn);
        e.w32(ri + 0x28, 0x2a00_004a);
        e.w32(ri + 0x18, 0x8000_0000);
        e.w32(ri + 0x20, 4);
        e.w32(ri + 0x1c, 0);
        e.w32(ri + 0x24, 0);
        ResetCamera(e);
        let p = G(e, cvCamOldPos_87);
        APIgameobj::GetPosition(e, SELF_CAM, 0x7fff_ffff, p);
        ss += 1;
        su(e, ui32Substate, ss);
        fallthrough = true;
    }
    if ss == 1 {
        follow(e, dt)?;
    } else if ss == 2 || ss == 3 {
        if ss == 2 {
            attract_start(e);
        }
        attract_run(e, dt)?;
    } else if ss == 4 {
        release(e, dt)?;
    }
    let o = if e.r32(GB0(e) + 0x30) == 0 { 0.75 } else { 0.95 };
    sf(e, fOuvertureCaptureTarget, o);
    let a = gf(e, fOuvertureCaptureActuel);
    let a = a - (a - o) * 0.1;
    sf(e, fOuvertureCaptureActuel, a);
    let c = e.r32(LB(e) + 200);
    APIcamera::SetViewWindow(e, c, a, a * 0.75);
    Ok(0)
}

fn follow(e: &mut Engine, dt: f32) -> LResult<()> {
    let gb0 = GB0(e);
    let t = e.r32(gb0 + 8);
    su(e, ui32ActualTargetID, t);
    let c = APIgameobj::GetContextFlag(e, SELF_CAM) as u32;
    APIgameobj::SetContextFlag(e, SELF_CAM, c | 8);
    if gu(e, CameraFlags) & 0x100 != 0 {
        let d = e.r32(LB(e) + 0x16c);
        let p = G(e, oEnvCollParams);
        APIphysic::DMDoAction(e, d, 1, p);
    }
    let mut mv: V3 = [0.0; 3]; // fStack_140
    let p190 = pos(e, SELF_CAM, 0x7fff_ffff);
    let a180 = lookat(e, SELF_CAM, 0x7fff_ffff);
    let j = gu(e, ui32FollowJoint);
    let mut p1e0 = pos(e, t, j);
    let u170 = lookup(e, t, 0x7fff_ffff);
    let a160 = lookat(e, t, 0x7fff_ffff);
    let vb = e.alloca(0x10);
    APIphysic::GetVelocity(e, t, vb);
    let mut v150 = e.rv3(vb);
    let s130 = APImath::sqrtf(e, dot(v150, v150));
    v150[1] = 0.0;
    let s12c = APImath::sqrtf(e, v150[0] * v150[0] + 0.0 + v150[2] * v150[2]);
    sf(e, fSpeedFactor, s130.min(5.4) / 5.4);
    su(e, CameraFlags, gu(e, CameraFlags) & 0xffff_fdff);
    let w = gv(e, cvCamWallCheck);
    let s1b0 = [p1e0[0] + w[0], p1e0[1] + w[1] + 1.4, p1e0[2] + w[2]];
    UpdateLastFollow(e, dt, p1e0);
    sf(e, fHauteurCameraFollow, gf(e, fHauteurCameraFollowTarget));
    let lf = gv(e, cvLastFollowY);
    p1e0[1] += gf(e, fTargetCameraFollowWork);
    let s1a0 = sub(s1b0, p190);
    let s1c0 = sub(lf, p190);
    let p120 = pos(e, t, j);
    let mut d120 = [(lf[0] + w[0] * 1.5) - p190[0], p120[1] - p190[1], (lf[2] + w[2] * 1.5) - p190[2]];
    let mut d100 = [d120[0], 0.0, d120[2]];
    let s110 = APImath::sqrtf(e, d120[0] * d120[0] + 0.0 + d120[2] * d120[2]);
    let cf = gu(e, CameraFlags);
    let att_out = gf(e, ATT + 0x1c);
    if cf & 0x508 == 0 || gf(e, fDistanceCameraFollowTarget) <= s110 {
        if cf & 0x500 == 0 || 0.0 < att_out || e.r32(gb0 + 0x14) & 0xa00 != 0 {
            let k = if cf & 4 != 0 { 10.0 } else { 2.0 };
            let d = gf(e, fDistanceCameraFollow);
            let tg = gf(e, fDistanceCameraFollowTarget) - gf(e, fDistanceCameraFollowCollision);
            sf(e, fDistanceCameraFollow, d - (d - tg) * k * dt);
        }
    } else {
        let d = gf(e, fDistanceCameraFollow);
        sf(e, fDistanceCameraFollow, d - (d - s110) * dt * 3.0);
    }
    let h = GetVHauteur(e, gf(e, fDistanceCameraFollow));
    let ht = gf(e, fHauteurCameraFollowTarget);
    let ht = ht - ((ht - gf(e, cvCamImpulse + 4)) - h) * 0.4;
    sf(e, fHauteurCameraFollowTarget, ht);
    if (d100[0].to_bits() | d100[1].to_bits() | d100[2].to_bits()) & 0x7fff_ffff != 0 {
        let k = gf(e, fDistanceCameraFollow) / APImath::sqrtf(e, dot(d100, d100));
        d100 = [d100[0] * k, 0.0, d100[2] * k];
    }
    d100[1] = -ht;
    d120 = sub(d120, d100);
    let mut ao = gf(e, ATT + 0x1c);
    if 0.0 < ao {
        let ec = APImath::sqrtf(e, dot(d120, d120));
        let e0 = scl(d120, 1.0 / ec);
        let f0 = (ec / ao) * dt + dot(e0, v150) * dt;
        if f0 < ec {
            let k = f0 / APImath::sqrtf(e, dot(d120, d120));
            d120 = scl(d120, k);
        } else {
            ao = 0.0;
            sf(e, ATT + 0x1c, 0.0);
        }
        let k = if ao < 0.5 { ao / 0.5 } else { 1.0 };
        sf(e, ATT + 0x1c, ao - dt * k);
    } else {
        d120 = scl(d120, 0.4);
    }
    if 4.0 < APImath::sqrtf(e, dot(d120, d120)) {
        let k = 4.0 / APImath::sqrtf(e, dot(d120, d120));
        d120 = scl(d120, k);
    }
    mv = add(mv, d120);
    CheckWall(e, p1e0);
    RotateCameraAroundTargetWithJoy(e, dt, &mut mv);
    IsSomethingBetwinCamAndTarget(e, dt, s130, &mut mv, s1a0, p190, s1c0, s1b0);
    ChangeCameraDistanceFromTargetWithJoy(e, dt, s130);
    TurnCameraOnBackOfTarget(e, s12c, dt, &mut mv, a180, u170, a160, s1a0);
    ReplaceOrientationFollow(e, dt, mv);
    let (i, iw) = (gv(e, cvCamImpulse), gv(e, cvCamImpulseWall));
    mv = [mv[0] * (1.0 / dt) + i[0] + iw[0], mv[1] * (1.0 / dt) + iw[1], mv[2] * (1.0 / dt) + i[2] + iw[2]];
    let (ix, iz) = (i[0] * 0.9, i[2] * 0.9);
    if ix * ix + iz * iz < 0.001 {
        sf(e, cvCamImpulse, 0.0);
        sf(e, cvCamImpulse + 8, 0.0);
    } else {
        sf(e, cvCamImpulse, ix);
        sf(e, cvCamImpulse + 8, iz);
    }
    let dy = mv[1] * dt;
    let a = (p190[1] - (p1e0[1] - gf(e, fTargetCameraFollowWork))) + dy;
    if gf(e, fCameraCurve + 0x2c) < a && 0.0 < mv[1] {
        mv[1] = 0.0;
    }
    APIphysic::SetVelocity(e, SELF_CAM, mv[0], mv[1], mv[2]);
    let w = gf(e, fTargetCameraFollowWork);
    sf(e, fTargetCameraFollowWork, w - (w - gf(e, fTargetCameraFollow)) * 0.1);
    su(e, CameraFlags, gu(e, CameraFlags) & 0xffff_fbff);
    Ok(())
}

fn att_target(e: &mut Engine) {
    let t = if gu(e, ATT + 0x28) & 0x10 == 0 { e.r32(GB0(e) + 8) } else { gu(e, ATT + 0x2c) };
    su(e, ui32ActualTargetID, t);
}

/// clamp `p` toward the attractor within its radius around cvLastFollowY (DStack_100)
fn att_goal(e: &mut Engine, from: V3) -> V3 {
    let a = gv(e, ATT);
    let r = gf(e, ATT + 0xc);
    let d = sub(a, from);
    let l2 = dot(d, d);
    if r * r < l2 {
        let s = APImath::sqrtf(e, l2);
        let _ = APImath::sqrtf(e, l2);
        let _ = APImath::sqrtf(e, l2);
        add(from, scl(d, r / s))
    } else {
        a
    }
}

fn attract_start(e: &mut Engine) {
    att_target(e);
    let p120 = pos(e, SELF_CAM, 0x7fff_ffff);
    let a = gv(e, ATT);
    let d = sub(a, p120);
    let l2 = dot(d, d);
    if 0.001 < l2 {
        let lf = gv(e, cvLastFollowY);
        let r = gf(e, ATT + 0xc);
        let g = if r * r < l2 {
            let s = APImath::sqrtf(e, l2);
            let _ = APImath::sqrtf(e, l2);
            let _ = APImath::sqrtf(e, l2);
            // quirk: offset from cvLastFollowY along the camera->attractor vector
            add(lf, scl(d, r / s))
        } else {
            a
        };
        let dd = sub(g, p120);
        let l = APImath::sqrtf(e, dot(dd, dd));
        sf(e, fAttractorSpeed_92, l / gf(e, ATT + 0x10));
    } else {
        sf(e, fAttractorSpeed_92, 0.0);
    }
    let fl = gu(e, ATT + 0x28);
    if fl & 8 != 0 {
        if fl & 0x40 != 0 {
            su(e, CameraFlags, gu(e, CameraFlags) | 0x40);
        }
        let t = gu(e, ui32ActualTargetID);
        let p = pos(e, t, 0x7fff_ffff);
        SetLastFollow(e, p);
    }
    let t = gu(e, ui32ActualTargetID);
    let o = G(e, oOriginalHeroPos_86);
    APIgameobj::GetPosition(e, t, 0x7fff_ffff, o);
    sf(e, fAttractorSmootBase_89, gf(e, ATT + 0x10) * 0.5);
    su(e, CameraFlags, (gu(e, CameraFlags) & 0xffff_ff7f) | 0x200);
    su(e, uiNbFrames_88, 0);
    sv(e, cvCamWallCheck, [0.0; 3]);
    sf(e, cvCamImpulse, 0.0);
    sf(e, cvCamImpulse + 4, 0.0);
    sf(e, cvCamImpulse + 8, 0.0);
    su(e, ui32Substate, 3);
}

fn attract_run(e: &mut Engine, dt: f32) -> LResult<()> {
    let r0 = 1.0f32; // afStack_d0[0]
    let fl = gu(e, ATT + 0x28);
    if fl & 4 != 0 {
        APIphysic::SetFlags(e, SELF_CAM, 1);
    }
    att_target(e);
    let t = gu(e, ui32ActualTargetID);
    let c0 = pos(e, t, 0x7fff_ffff);
    let u = lookup(e, t, 0x7fff_ffff);
    UpdateLastFollow(e, dt, c0);
    if gu(e, CameraFlags) & 0x80 != 0 && fl & 2 != 0 {
        su(e, CameraFlags, gu(e, CameraFlags) & 0xffff_ff7f);
        su(e, ui32Substate, 4);
        return cameraST_CAMERA_FOLLOW(e).map(|_| ());
    }
    let n = gu(e, uiNbFrames_88) + 1;
    su(e, uiNbFrames_88, n);
    if fl & 1 != 0 && 5 < n {
        let b = sub(pos(e, t, 0x7fff_ffff), gv(e, oOriginalHeroPos_86));
        if 0.01 < b[0] * b[0] + b[2] * b[2] {
            su(e, ui32Substate, 4);
            return cameraST_CAMERA_FOLLOW(e).map(|_| ());
        }
    }
    let lf = gv(e, cvLastFollowY);
    let mut g = att_goal(e, lf);
    let si = gf(e, ATT + 0x10);
    if 0.0 < si {
        let p90 = pos(e, SELF_CAM, 0x7fff_ffff);
        let d = sub(g, p90);
        let ec = APImath::sqrtf(e, dot(d, d));
        if ec <= 0.001 || si <= 0.01 {
            sf(e, ATT + 0x10, 0.0);
        } else {
            let base = gf(e, fAttractorSmootBase_89);
            let mut k12c = 1.0;
            let mut f0 = (si - base) / base;
            if base < si {
                k12c = 1.0 - f0;
            }
            f0 *= f0;
            let s110 = (si * f0 + si) * 0.5;
            let l = APImath::sqrtf(e, dot(d, d));
            let k = ((ec / s110) * dt * k12c) / l;
            g = add(p90, scl(d, k));
            sf(e, ATT + 0x10, si - dt);
        }
    }
    if fl & 8 == 0 {
        let lf = gv(e, cvLastFollowY);
        let r = sub(g, lf);
        let ec = dot(u, r);
        let a = sub(r, scl(u, ec));
        if APImath::sqrtf(e, dot(a, a)) < r0 + r0 {
            let k = (r0 + r0) / APImath::sqrtf(e, dot(a, a));
            g = add(g, sub(scl(a, k), a));
        }
    }
    if fl & 4 == 0 {
        let p70 = pos(e, SELF_CAM, 0x7fff_ffff);
        let d = sub(g, p70);
        ReplaceOrientationFollow(e, dt, d);
        let v = scl(d, 1.0 / dt);
        APIphysic::SetVelocity(e, SELF_CAM, v[0], v[1], v[2]);
    } else {
        set_trans(e, SELF_CAM, g);
        ReplaceOrientationFollow(e, dt, [0.0; 3]);
    }
    let p60 = pos(e, t, 0x7fff_ffff);
    UpdateLastFollow(e, dt, p60);
    let tr = gv(e, cvTrackEnAvant);
    sv(e, cvTrackEnAvant, scl(tr, 0.9));
    if fl & 0x20 != 0 {
        su(e, ui32Substate, 4);
    }
    Ok(())
}

fn release(e: &mut Engine, dt: f32) -> LResult<()> {
    let ao = gf(e, ATT + 0x1c);
    let out = (ao * 0.25).min(1.0);
    sf(e, fAttractorSmootOut_91, out);
    sf(e, fAttractorSmootIn_90, ao - out);
    let t = gu(e, ui32ActualTargetID);
    let j = gu(e, ui32FollowJoint);
    let p = pos(e, t, j);
    SetLastFollow(e, p);
    UpdateLastFollow(e, dt, p);
    sf(e, fDistanceCameraFollowCollision, 0.0);
    APIphysic::ResetFlags(e, SELF_CAM, 1);
    APIphysic::SetVelocity(e, SELF_CAM, 0.0, 0.0, 0.0);
    su(e, CameraFlags, gu(e, CameraFlags) & 0xffff_f8bf);
    su(e, ui32Substate, 1);
    let c = e.r32(LB(e) + 200);
    let o = G(e, fOuvertureCaptureActuel);
    let tmp = e.alloca(0x10);
    APIcamera::GetViewWindow(e, c, o, tmp);
    cameraST_CAMERA_FOLLOW(e)?;
    Ok(())
}

/// 0x00017d68 notify 6 (DM state change): EnvColl contact bit -> CameraFlags 0x100
pub fn Camera_BaseCB_PHYSIC(e: &mut Engine, param_1: Ptr) -> LResult<u64> {
    let d = e.r32(LB(e) + 0x16c);
    if e.r32(param_1) == d {
        let b = e.r32(param_1 + 8);
        if b & 1 == 0 {
            if b & 2 != 0 {
                APIphysic::DMResetStateBits(e, d, 2);
                APIphysic::DMSetStateChangeNotifyBits(e, d, 1);
                su(e, CameraFlags, gu(e, CameraFlags) & 0xffff_feff);
            }
        } else {
            APIphysic::DMSetStateChangeNotifyBits(e, d, 2);
            su(e, CameraFlags, gu(e, CameraFlags) | 0x100);
        }
    }
    Ok(0)
}

/// 0x00017e90 notify 3 (collision)
pub fn Camera_BaseCB_COLLIDE(e: &mut Engine, _: u32) -> LResult<u64> {
    su(e, CameraFlags, gu(e, CameraFlags) | 0x400);
    Ok(0)
}

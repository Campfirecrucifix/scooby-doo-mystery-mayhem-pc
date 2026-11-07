//! EFDMCameraFlex (Classes/EFDM/EFDMCameraFlex.cpp, DMAdd kind 0x12, type 0x50000012 = PRE|POST,
//! 300 bytes), 1:1. The flexible game camera: places the owner (the camera object) relative to a
//! target object in one of 8 modes (+0xa4: 0 spline-animated, 1 target-relative offset, 2 distance
//! with collision, 3 world offset, 4 fixed, 5 fixed with look matrix, 6 orbit (yaw/pitch), 7 logic
//! callback), moves it with a speed/acceleration limit (MoveToTarget) and slerps its orientation
//! toward the look-at point (LookAtHero). Optional orbit / first-person input.
//!
//! Replacements: the shared scratch RwMatrix `*(0x003439d0)` -> local RwMatrix; RwFrameGetLTM /
//! frame modelling matrix of the owner -> `ctx.phys.frame`; the target EFGameObject pointer (+0x34)
//! -> uid + `GetObjectFrame`; the spline pointer (+0x44) -> index into `ctx.world.Splines()`;
//! obj+0x60 animation data -> `CameraAnimChannels` (DTFCurve::Evaluate here);
//! RwFrameUpdateObjects -> no-op.
#![allow(non_snake_case)]

use crate::engine::camera::DTMatrix;
use crate::engine::physic::collision_map::RayIntersectIn;
use crate::engine::physic::dm::follow::{MatToQuat, Normalise, QuatToMat};
use crate::engine::physic::mgr::{EFRayIntersectIn, EFRayResult};
use crate::engine::physic::module::*;
use crate::engine::physic::rwm::*;
use crate::engine::spline::{EFQuadraticSpline, EFSplineCursor};

const PI: f32 = 3.1415927;
const TWO_PI: f32 = 6.2831855;
const R2D: f32 = 57.29578;

/// `0x3bffffff < (bits & 0x7f800000)`: |x| >= 2^-7 (the input dead zone test).
fn Big(x: f32) -> bool { (x.to_bits() & 0x7f80_0000) > 0x3bff_ffff }
/// `(bits & 0x7f800000) < 0x38000000`: |x| < 2^-15
fn Tiny15(x: f32) -> bool { (x.to_bits() & 0x7f80_0000) < 0x3800_0000 }

/// acos with the decomp's `<= -1 -> pi`, `>= 1 -> 0` clamping.
fn AcosClamp(c: f32) -> f32 {
    if c <= -1.0 { PI } else if 1.0 <= c { 0.0 } else { c.acos() }
}

/// The repeated inline "at + reference up -> orthonormal basis" (right = cross(upref, at),
/// up = cross(at, right), both normalised; `at` already set).
fn BuildFromAt(m: &mut RwMatrix, upref: V3) {
    m.right = RwV3dNormalize(cross(upref, m.at));
    m.up = RwV3dNormalize(cross(m.at, m.right));
}

/// DTQuaternion::Slerp (not in this file; standard shortest-arc slerp with a lerp fallback).
fn Slerp(a: &[f32; 4], b: &[f32; 4], t: f32) -> [f32; 4] {
    let mut d = a[0] * b[0] + a[1] * b[1] + a[2] * b[2] + a[3] * b[3];
    let mut b = *b;
    if d < 0.0 {
        d = -d;
        b = b.map(|x| -x);
    }
    let (k0, k1) = if 1.0 - d > 1e-6 {
        let th = d.acos();
        let s = th.sin();
        (((1.0 - t) * th).sin() / s, (t * th).sin() / s)
    } else {
        (1.0 - t, t)
    };
    std::array::from_fn(|i| a[i] * k0 + b[i] * k1)
}

pub struct EFDMCameraFlex {
    pub base: EFPhysicModule,
    /// +0x30 target object id (param 0)
    pub target_id: u32,
    /// +0x34 target game object (GetGameObject(+0x30) at SetParams)
    pub target: Option<u32>,
    /// +0x38 animation object id (param 1; its obj+0x60 holds the camera animations)
    pub anim_id: u32,
    /// +0x3c (-1, unused)
    pub unk_3c: i32,
    /// +0x40 spline cursor flags (bit1 = tangent +0x60 cached; recomputed on demand here)
    pub cursor_flags: u32,
    /// +0x44 current spline (index into _gpoSplineMgr's table)
    pub spline: Option<usize>,
    /// +0x48 global factor / +0x4c segment / +0x50 t
    pub cursor: EFSplineCursor,
    /// +0x60 cursor tangent cache
    pub tangent: V3,
    /// +0x6c yaw (mode 6) / screen x framing (mode 0/5) (param +0x48)
    pub yaw: f32,
    /// +0x70 pitch (mode 6) / screen y framing (param +0x4c)
    pub pitch: f32,
    /// +0x74 (param +0x50; only written by the animation)
    pub unk_74: f32,
    /// +0x78 position time (param +0x28)
    pub pos_time: f32,
    /// +0x7c height time (param +0x2c)
    pub height_time: f32,
    /// +0x80 look time (param +0x30)
    pub look_time: f32,
    /// +0x84 yaw time (param +0x34)
    pub yaw_time: f32,
    /// +0x88 pitch time (param +0x38)
    pub pitch_time: f32,
    /// +0x8c camera offset / position (param +0x10)
    pub offset: V3,
    /// +0x98 look-at offset / point (param +0x1c)
    pub look: V3,
    /// +0xa4 mode (param +8, u8)
    pub mode: u8,
    /// +0xa8 spline id (-1)
    pub spline_id: i32,
    /// +0xac animation data holder (obj+0x60 of +0x38) -> the object uid
    pub anim_obj: Option<u32>,
    /// +0xb0 "no spline / animation stopped" flag (u8), DM state bit0 mirror
    pub stopped: u8,
    /// +0xb4 spline search sphere radius (param +0xc)
    pub search_radius: f32,
    /// +0xb8 use the +0xbc look matrix (u8)
    pub use_look_mat: u8,
    /// +0xbc look matrix (RwMatrixCreate)
    pub look_mat: RwMatrix,
    /// +0xc0 current speed
    pub speed: f32,
    /// +0xc4 max acceleration (param +0x3c)
    pub max_accel: f32,
    /// +0xc8 predicted camera position
    pub cam_pos: V3,
    /// +0xd4 / +0xd8 input action (InputActionGet(+0xd8, +0xd4)) (params +0x54/+0x58)
    pub input_b: u32,
    pub input_a: u32,
    /// +0xdc / +0xe0 first person yaw / pitch speed (params +0x5c/+0x60)
    pub fpv_yaw_speed: f32,
    pub fpv_pitch_speed: f32,
    /// +0xe4 first person max pitch (param +0x64, 1.4835)
    pub fpv_max_pitch: f32,
    /// +0xe8 max backwards spline speed (param +0x40)
    pub back_max: f32,
    /// +0xec spline backwards speed
    pub back_speed: f32,
    /// +0xf0 backwards acceleration / damping (param +0x44)
    pub back_accel: f32,
    /// +0xf4 horizontal distance (InitDM)
    pub distance: f32,
    /// +0xf8 height difference camera - look (InitDM)
    pub height: f32,
    /// +0xfc look max angle (param +0x6c)
    pub look_max_angle: f32,
    /// +0x100 animation channel mask
    pub anim_mask: u32,
    /// +0x104 orbit speed (param +0x68)
    pub orbit_speed: f32,
    /// +0x108 logic callback (mode 7) (param +0x74)
    pub callback: u32,
    /// +0x10c distance collision state (u8: 0 free, 1 moving, 2 blocked)
    pub col_state: u8,
    /// +0x110 last free camera position
    pub last_cam: V3,
    /// +0x11c last target position
    pub last_target: V3,
    /// +0x128 flags (param +0x70): 1 spline-relative offset, 2 orbit, 4 limit look, 8 absolute
    /// look point, 0x10 no collision
    pub flags: u32,
}

impl Default for EFDMCameraFlex {
    fn default() -> Self { Self::new() }
}

impl EFDMCameraFlex {
    /// 0x00224510
    pub fn new() -> Self {
        let mut s = EFDMCameraFlex {
            base: EFPhysicModule { type_: 0x5000_0012, flags: 1, ..Default::default() },
            target_id: 0,
            target: None,
            anim_id: 0,
            unk_3c: 0,
            cursor_flags: 0,
            spline: None,
            cursor: EFSplineCursor { global: 0.0, seg: 0xffff_ffff, t: 0.0 },
            tangent: [0.0; 3],
            yaw: 0.0,
            pitch: 0.0,
            unk_74: 0.0,
            pos_time: 0.0,
            height_time: 0.0,
            look_time: 0.0,
            yaw_time: 0.0,
            pitch_time: 0.0,
            offset: [0.0; 3],
            look: [0.0; 3],
            mode: 0,
            spline_id: 0,
            anim_obj: None,
            stopped: 0,
            search_radius: 0.0,
            use_look_mat: 0,
            look_mat: RwMatrix::identity(),
            speed: 0.0,
            max_accel: 0.0,
            cam_pos: [0.0; 3],
            input_b: 0,
            input_a: 0,
            fpv_yaw_speed: 0.0,
            fpv_pitch_speed: 0.0,
            fpv_max_pitch: 0.0,
            back_max: 0.0,
            back_speed: 0.0,
            back_accel: 0.0,
            distance: 0.0,
            height: 0.0,
            look_max_angle: 0.0,
            anim_mask: 0,
            orbit_speed: 0.0,
            callback: 0,
            col_state: 0,
            last_cam: [0.0; 3],
            last_target: [0.0; 3],
            flags: 0,
        };
        s.InitValues();
        s
    }

    /// 0x00224770
    pub fn InitValues(&mut self) {
        self.spline_id = -1;
        self.unk_3c = -1;
        self.anim_obj = None;
        self.stopped = 1;
        self.speed = 0.0;
        self.distance = 3.0;
        self.search_radius = 3.0;
        self.target_id = 0;
        self.target = None;
        self.anim_id = 0;
        self.mode = 1;
        self.offset = [0.0, 0.0, -2.0];
        self.look = [0.0; 3];
        self.pos_time = 0.5;
        self.height_time = 0.5;
        self.look_time = 0.5;
        self.yaw_time = 0.1;
        self.pitch_time = 0.1;
        self.yaw = 1.0;
        self.pitch = 1.0;
        self.unk_74 = 1.0;
        self.max_accel = 25.0;
        self.input_b = 0;
        self.input_a = 0;
        self.back_max = 0.0;
        self.back_accel = 1.0;
        self.back_speed = 0.0;
        self.anim_mask = 0;
        self.look_max_angle = 0.7;
        self.fpv_yaw_speed = 0.7;
        self.fpv_pitch_speed = 0.7;
        self.callback = 0;
        self.col_state = 1;
        self.last_cam = [0.0; 3];
        self.last_target = [0.0; 3];
        self.orbit_speed = 1.0;
        self.flags = 0;
        self.fpv_max_pitch = f32::from_bits(0x3fbd_e44d);
    }

    /// 0x002249b0 (vtbl+0x28)
    pub fn InitDM(&mut self) {
        self.distance = (self.offset[2] * self.offset[2] + self.offset[0] * self.offset[0]).sqrt();
        self.height = self.offset[1] - self.look[1];
    }

    fn sp<'w>(&self, ctx: &'w DMCtx) -> Option<&'w EFQuadraticSpline> {
        self.spline.and_then(|i| ctx.world.Splines().get(i))
    }

    /// 0x002253f0. Returns true when the input rotated the view.
    pub fn FirstPersonView(&mut self, ctx: &mut DMCtx, input: Option<[f32; 2]>, tm: &RwMatrix, dt: f32) -> bool {
        let mut res = false;
        let Some(inp) = input else { return false };
        if Big(inp[0]) || Big(inp[1]) {
            let ltm = ctx.phys.frame;
            let mut m = RwMatrix::identity();
            let mut p = tm.TransformPoint(self.look);
            if self.mode == 0 || self.mode == 5 {
                p = self.GetNewLookAtPos(&ltm, p);
            }
            let mut d = sub(p, ltm.pos);
            let mut c = cross(d, [0.0, 1.0, 0.0]);
            let a = self.fpv_yaw_speed * inp[0];
            let ra = a != 0.0;
            if ra {
                m.Rotate(-a * R2D, [0.0, 1.0, 0.0], rwCOMBINEPOSTCONCAT);
                m.pos = [0.0; 3]; // the saved/restored translation (always 0 here)
            }
            let at = ltm.at;
            let mut b = self.fpv_pitch_speed * inp[1];
            let h = RwV3dNormalize([at[0], 0.0, at[2]]);
            let ang = AcosClamp(h[2] * at[2] + h[0] * at[0] + h[1] * at[1]);
            // computed in double in the original
            if (ang as f64 + (b as f64).abs()) > self.fpv_max_pitch as f64 {
                let sg: f64 = if b.to_bits() & 0x8000_0000 == 0 { 1.0 } else { -1.0 };
                b -= (sg * ((ang as f64 + (b as f64).abs()) - self.fpv_max_pitch as f64)) as f32;
            }
            let rb = b != 0.0;
            if rb {
                c = RwV3dNormalize(c);
                m.Rotate(-b * R2D, c, rwCOMBINEPOSTCONCAT);
                m.pos = [0.0; 3];
            }
            res = rb || ra;
            d = m.TransformVector(d);
            let saved = self.mode;
            self.mode = 1;
            self.LookAtHero(ctx, add(d, ltm.pos), dt);
            self.mode = saved;
        }
        res
    }

    /// 0x002259c0 (Ghidra shows `this` as param_1). Returns 1 when the input moved.
    pub fn OrbitCam(&mut self, input: Option<[f32; 2]>, tm: &RwMatrix, dt: f32) -> bool {
        let Some(inp) = input else { return false };
        let (x, y) = (inp[0], inp[1]);
        if !(Big(x) || Big(y)) {
            return false;
        }
        if self.mode == 6 {
            self.yaw += self.orbit_speed * x * dt;
            self.pitch += self.orbit_speed * y * dt;
            let v = self.yaw;
            if TWO_PI <= v {
                self.yaw = v - TWO_PI;
            } else if v < 0.0 {
                // quirk: 2pi - v instead of v + 2pi
                self.yaw = TWO_PI - v;
            }
            if 3.0415928 < self.pitch {
                self.pitch = f32::from_bits(0x4042_a975);
            } else if self.pitch < 0.0 {
                self.pitch = 0.0;
            }
        } else {
            let center = if self.mode == 4 { tm.TransformPoint(self.look) } else { self.look };
            let mut m = RwMatrix::identity();
            let mut a = self.orbit_speed * x * dt;
            if self.mode == 2 {
                let f = Self::DeltaForce(self.pos_time, dt);
                if 1e-6 < f {
                    a *= 1.0 / f;
                }
            }
            m.Rotate(a * R2D, [0.0, 1.0, 0.0], rwCOMBINEPOSTCONCAT);
            m.pos = [0.0; 3];
            let v = m.TransformVector(sub(self.offset, center));
            self.offset = add(center, v);
        }
        true
    }

    /// 0x00225d20
    pub fn UpdateCameraAnimated(&mut self, ctx: &mut DMCtx, tgt: &mut V3, dt: f32) {
        let Some(tm) = self.target.and_then(|u| ctx.world.GetObjectFrame(u)) else { return };
        if self.spline.is_none() || self.stopped != 0 {
            self.spline_id = self.FindSpline(ctx, tm.pos);
        }
        if self.stopped == 0 && -1 < self.spline_id {
            if let Some(sp) = self.sp(ctx) {
                self.cursor.SetToNearest(sp, tm.pos);
            }
            if 1.0 <= self.cursor.global || self.cursor.global <= 0.0 {
                self.stopped = 1;
            }
        }
        let mut moved = false;
        if let Some(sp) = self.sp(ctx) {
            // +0x40 bit1 tangent cache: recomputed (same value at the current cursor)
            self.tangent = sp.GetTangent(self.cursor.seg as i32, self.cursor.t);
            self.cursor_flags |= 2;
            if tm.at[2] * self.tangent[2] + tm.at[0] * self.tangent[0] + tm.at[1] * self.tangent[1] < 0.0 {
                self.back_speed += self.back_accel * -dt;
                let mut v = self.back_speed;
                if v <= -self.back_max {
                    v = -self.back_max;
                }
                self.back_speed = if 0.0 < v { 0.0 } else { v };
                self.cursor.MoveDistance(sp, self.back_speed);
                moved = true;
            }
        }
        if !moved && Big(self.back_speed) {
            self.back_speed /= dt * self.back_accel + 1.0;
            // quirk: MoveDistance(NULL spline) uses the cursor's spline (crash if none)
            if let Some(sp) = self.sp(ctx) {
                self.cursor.MoveDistance(sp, self.back_speed);
            }
        }
        self.UpdateAnimation(ctx, tgt);
    }

    fn Ray(&self, ctx: &DMCtx, origin: V3, dir: V3) -> Option<EFRayResult> {
        // RayIntersectIn +0x18 = 2, +0x20 = 0x404, +0x28 = owner obj+8.
        // quirk: +0x1c and +0x24 are never initialised on the stack (0 here).
        let rin = EFRayIntersectIn {
            map: RayIntersectIn { origin, dir, mask1: 0x404, mask2: 0 },
            obj_mask1: 2,
            obj_mask2: 0,
            exclude: ctx.phys.owner,
        };
        ctx.world.RayIntersectResult(&rin)
    }

    /// 0x00225f50
    pub fn UpdateCameraDistance(&mut self, ctx: &mut DMCtx, tm: &RwMatrix, scale_: f32, tgt: &mut V3) {
        let tp = tm.pos;
        let mut off = sub(*tgt, tp);
        let mut side = [0.0f32; 3];
        let mut dir = [0.0f32; 3];
        let mut hit: Option<EFRayResult> = None;
        if self.flags & 0x10 == 0 {
            let f = ctx.phys.frame.right;
            let k = dot(tm.at, f) / dot(f, f);
            side = scale(f, k);
            if 0.0 < dot(side, side) {
                side = RwV3dNormalize(side);
            }
            side = scale(side, scale_);
            let origin = add(tp, side);
            dir = sub(*tgt, origin);
            hit = self.Ray(ctx, origin, dir);
        } else {
            self.col_state = 1;
        }
        // state machine (labels of the original)
        let mut st = self.col_state;
        'end: {
            if st == 0 {
                if hit.is_none() {
                    let d = sub(self.last_target, tp);
                    if dot(d, d) <= DT_EPSILON_FLOAT {
                        *tgt = self.last_cam;
                        break 'end;
                    }
                }
                self.col_state = 1;
                st = 1;
            } else if st != 1 && st != 2 {
                break 'end;
            }
            if st == 1 {
                match hit {
                    None => {
                        off[1] = 0.0;
                        if 0.0 < off[2] * off[2] + off[0] * off[0] + 0.0 {
                            off = RwV3dNormalize(off);
                        }
                        off = scale(off, self.distance);
                        *tgt = add(tp, off);
                        tgt[1] = self.offset[1] + tm.pos[1];
                        self.last_cam = *tgt;
                        break 'end;
                    }
                    Some(h) => {
                        if self.distance * 4.0 * self.distance < off[2] * off[2] + off[0] * off[0] {
                            *tgt = h.pos;
                            // frame modelling pos = 0, RwFrameTranslate(hit, POSTCONCAT)
                            ctx.phys.frame.pos = [0.0; 3];
                            ctx.phys.frame.Translate(h.pos, rwCOMBINEPOSTCONCAT);
                            break 'end;
                        }
                        self.col_state = 2;
                    }
                }
            }
            // state 2
            match hit {
                None => self.col_state = 0,
                Some(h) => {
                    let mut n = h.normal;
                    // second ray back from the camera point
                    let h2 = self.Ray(ctx, *tgt, scale(dir, -1.0));
                    let n2 = h2.map(|r| r.normal).unwrap_or([0.0; 3]);
                    n = add(n, n2);
                    let l = dot(n, n);
                    if DT_EPSILON_FLOAT < l {
                        // quirk: divides by the squared length (not a normalisation)
                        n = scale(n, 1.0 / l);
                    } else {
                        n = RwV3dNormalize(side);
                    }
                    off[0] += n[0];
                    off[2] += n[2];
                    off[1] = 0.0;
                    off = RwV3dNormalize(off);
                    off = scale(off, self.distance);
                    *tgt = add(tp, off);
                    tgt[1] = self.offset[1] + tm.pos[1];
                    self.last_cam = *tgt;
                }
            }
        }
        self.last_target = tp;
        self.offset = sub(*tgt, tp);
    }

    /// 0x00226910
    pub fn UpdateCameraRelDistance(&mut self, ctx: &mut DMCtx, tm: &RwMatrix, lookp: V3, tgt: &mut V3, dt: f32) {
        let down: V3 = [0.0, -1.0, 0.0];
        // horizontal projection of the target's at
        let mut a = RwV3dNormalize(sub(tm.at, scale(down, dot(tm.at, down))));
        let cp = ctx.phys.frame.pos;
        let mut e = sub(cp, lookp);
        let _ = RwV3dLength(e);
        let mut f = sub(e, scale(down, dot(e, down)));
        f = RwV3dNormalize(f);
        e = RwV3dNormalize(e);
        let mut yaw = AcosClamp(a[2] * f[2] + a[0] * f[0] + a[1] * f[1]);
        let mut side = false;
        // computed in double (0.02)
        if ((yaw - PI) as f64).abs() < 0.02 || (yaw as f64).abs() < 0.02 {
            // quirk: the right-vector projection is not normalised
            a = sub(tm.right, scale(down, dot(tm.right, down)));
            yaw = AcosClamp(a[2] * f[2] + a[0] * f[0] + a[1] * f[1]);
            side = true;
        }
        let c = cross(a, f);
        if c[1].to_bits() & 0x8000_0000 != 0 {
            yaw = TWO_PI - yaw;
        }
        let dd = e[2] * 0.0 + e[0] * 0.0 + e[1] * -1.0;
        let pitch = if dd <= -1.0 { PI } else if dd < 1.0 { dd.acos() } else { 0.0 };
        let fy = Self::DeltaForce(self.yaw_time, dt);
        let fp = Self::DeltaForce(self.pitch_time, dt);
        let mut d: V3;
        if 0.0 < fy {
            let mut want = self.yaw;
            if side {
                want -= 1.5707964;
            }
            if PI < yaw - want {
                yaw += fy * ((want + TWO_PI) - yaw);
            } else if PI < want - yaw {
                yaw += fy * (want - (yaw + TWO_PI));
            } else {
                yaw += fy * (want - yaw);
            }
            d = [a[0] * yaw.cos() + a[2] * yaw.sin(), 0.0, -a[0] * yaw.sin() + a[2] * yaw.cos()];
        } else {
            d = f;
        }
        let mut m = RwMatrix::identity();
        let axis = [d[1] * 0.0 - d[2] * 1.0, d[2] * 0.0 - d[0] * 0.0, d[0] * 1.0 - d[1] * 0.0];
        m.Rotate(((pitch + fp * (self.pitch - pitch)) - 1.5707964) * R2D, axis, rwCOMBINEPOSTCONCAT);
        m.pos = [0.0; 3];
        d = m.TransformVector(d);
        d = scale(d, self.distance);
        *tgt = add(d, lookp);
        self.offset = d;
    }

    /// 0x002273a0: returns the velocity toward `tgt`; updates +0xc0 speed and +0xc8.
    pub fn MoveToTarget(&mut self, cur: V3, tgt: V3, dt: f32) -> V3 {
        let f = Self::DeltaForce(self.pos_time, dt);
        let fh = Self::DeltaForce(self.height_time, dt);
        // (the DTVector3d::operator* operands are lost in the decomp: tgt*f + cur*(1-f))
        let mut n = add(scale(cur, 1.0 - f), scale(tgt, f));
        n[1] = fh * tgt[1] + (1.0 - fh) * cur[1];
        let mut d = sub(n, cur);
        let l = RwV3dLength(d);
        let acc = (l / dt - self.speed) / dt;
        let a = if acc <= self.max_accel || self.pos_time <= 0.05 { acc } else { self.max_accel };
        let spd = self.speed + a * dt;
        if 0.0001 < l {
            d = scale(d, spd / l);
        }
        self.cam_pos = add(cur, scale(d, dt));
        self.speed = spd;
        d
    }

    /// 0x00227650: slerp the owner frame rotation toward `lookp` from +0xc8.
    pub fn LookAtHero(&mut self, ctx: &mut DMCtx, lookp: V3, dt: f32) {
        let fm = ctx.phys.frame;
        let q0 = MatToQuat(&fm);
        let mut l = lookp;
        let q1;
        if self.mode == 0 || self.mode == 5 {
            if self.yaw == 0.0 && self.pitch == 0.0 {
                q1 = MatToQuat(&self.look_mat);
            } else {
                l = self.GetNewLookAtPos(&fm, l);
                q1 = Self::GetLookAt(self.cam_pos, l);
            }
        } else {
            if self.mode == 2 || self.flags & 4 != 0 {
                let mut c = sub(l, self.cam_pos);
                let mut b = [c[0], -self.height, c[2]];
                let _ = RwV3dLength(b);
                c = RwV3dNormalize(c);
                b = RwV3dNormalize(b);
                let axis = cross(b, c);
                let ang = AcosClamp(c[2] * b[2] + c[0] * b[0] + c[1] * b[1]);
                if self.look_max_angle <= ang {
                    let mut m = RwMatrix::identity();
                    m.Rotate((ang - self.look_max_angle) * R2D, axis, rwCOMBINEPOSTCONCAT);
                    m.pos = [0.0; 3];
                    let mut v = sub(l, self.cam_pos);
                    v[1] = -self.height;
                    v = m.TransformVector(v);
                    l[1] = v[1] + self.cam_pos[1];
                } else {
                    l[1] = self.cam_pos[1] - self.height;
                }
            }
            q1 = Self::GetLookAt(self.cam_pos, l);
        }
        let f = Self::DeltaForce(self.look_time, dt);
        let mut q = Slerp(&q0, &q1, f);
        Normalise(&mut q);
        let m = QuatToMat(&q);
        let upref = if Tiny15(m.at[0]) && Tiny15(m.at[2]) { [0.0, 0.0, 1.0] } else { [0.0, 1.0, 0.0] };
        let fr = &mut ctx.phys.frame;
        fr.at = m.at;
        BuildFromAt(fr, upref);
        // RwFrameUpdateObjects: no-op
    }

    /// 0x00227cc0: `p` in the look space (+0xbc if +0xb8 else the owner frame), x/y scaled by
    /// +0x6c/+0x70, z = |z|, back to world. (The point argument is lost in the decomp; the callers
    /// pass their look-at point.)
    pub fn GetNewLookAtPos(&self, frame: &RwMatrix, p: V3) -> V3 {
        let m = if self.use_look_mat == 0 { *frame } else { self.look_mat };
        let inv = m.Invert();
        let l = inv.TransformPoint(p);
        let s = [l[0] * self.yaw, l[1] * self.pitch, (l[2] as f64).abs() as f32];
        m.TransformPoint(s)
    }

    /// 0x00227e10: quaternion of the look-at matrix from `from` toward `to`.
    pub fn GetLookAt(from: V3, to: V3) -> [f32; 4] {
        let d0 = sub(to, from);
        let mut d = RwV3dNormalize(d0);
        if RwV3dLength(d0) == 0.0 {
            // quirk: only z is set (x/y stay 0)
            d[2] = 1.0;
        }
        let upref = if Tiny15(d[0]) && Tiny15(d[2]) { [0.0, 0.0, 1.0] } else { [0.0, 1.0, 0.0] };
        let mut m = RwMatrix { at: d, ..RwMatrix::identity() };
        BuildFromAt(&mut m, upref);
        MatToQuat(&m)
    }

    /// 0x002280b0
    pub fn DeltaForce(t: f32, dt: f32) -> f32 {
        let mut f = 1.0;
        if 1e-6 <= t {
            f = (1.0 / t) * dt;
            if 1.0 < f {
                f = 1.0;
            } else if f <= 1e-6 {
                f = 0.0;
            }
        }
        f
    }

    fn SetSpline(&mut self, i: usize, id: i32) {
        self.spline = Some(i);
        self.cursor = EFSplineCursor { global: 0.0, seg: 0xffff_ffff, t: 0.0 };
        self.cursor_flags = 0;
        self.spline_id = id;
    }

    /// 0x00228120: nearest camera spline (type bit 2) whose sphere test hits `p`.
    pub fn FindSpline(&mut self, ctx: &DMCtx, p: V3) -> i32 {
        let splines = ctx.world.Splines();
        let mut best: i32 = -1;
        let mut bestd = f32::MAX; // DT_MAX_FLOAT
        for (i, sp) in splines.iter().enumerate() {
            if sp.ty & 4 != 0 {
                let mut g = 0.0f32;
                if sp.CheckSphereIntersection(p, self.search_radius, &mut g) {
                    let x = sp.AdjustGlobalFactor(g) * sp.nseg() as f32;
                    let seg = if x < 2.1474836e9 { x as u32 } else { ((x - 2.1474836e9) as i32 as u32) | 0x8000_0000 };
                    let q = sub(sp.GetPoint(seg as i32, x - seg as f32), p);
                    let d = dot(q, q);
                    if d < bestd {
                        bestd = d;
                        best = i as i32;
                    }
                }
            }
        }
        if best < 0 {
            self.stopped = 1;
        } else {
            let sp = &splines[best as usize];
            self.SetSpline(best as usize, sp.id as i32);
            self.cursor.SetToNearest(sp, p);
            self.stopped = 0;
        }
        self.spline_id
    }

    /// 0x002283d0: evaluate the camera animation channels of the current spline at the cursor.
    pub fn UpdateAnimation(&mut self, ctx: &mut DMCtx, tgt: &mut V3) -> i32 {
        let Some(obj) = self.anim_obj else { return -1 };
        let Some(sp) = self.sp(ctx) else { return -1 };
        let mut rot = [0.0f32; 3];
        let mut height_set = false;
        self.use_look_mat = 0;
        let Some(chans) = ctx.world.CameraAnimChannels(obj, self.spline_id) else { return -1 };
        for (ch, curve) in chans {
            // key i = first float of segment i (stride 3 floats for type 4, else 5)
            let key = |i: u32| -> f32 {
                // quirk: NULL deref out of range in the C
                match &curve.data {
                    crate::engine::anim::res::DTFCurveData::Comp(v) => v.get(i as usize).map_or(0.0, |x| x.0),
                    crate::engine::anim::res::DTFCurveData::Cubic(v) => v.get(i as usize).map_or(0.0, |x| x[0]),
                    crate::engine::anim::res::DTFCurveData::Const(_) => 0.0,
                }
            };
            let mut t;
            if sp.nseg() == curve.nb_segment as u32 {
                let seg = self.cursor.seg;
                let k1 = key(seg);
                let k0 = if 0 < seg as i32 { key(seg - 1) } else { 0.0 };
                t = k0 + (sp.GetSegmentLengthAt(seg, self.cursor.t) / sp.GetSegmentLength(seg)) * (k1 - k0);
            } else {
                t = curve.start_time + self.cursor.GetLengthFactor(sp) * (curve.end_time - curve.start_time);
            }
            // type 1 = constant (+0xc), 4 = DTFloatComp cubic, else cubic (DTFCurve::Evaluate)
            t = curve.Evaluate(t);
            let v = t;
            match ch & 0x00ff_ffff {
                3 if self.anim_mask & 0x100 == 0 => tgt[0] = v,
                4 if self.anim_mask & 0x100 == 0 => tgt[1] = v,
                5 if self.anim_mask & 0x100 == 0 => tgt[2] = v,
                6 if self.anim_mask & 0x200 == 0 => {
                    self.use_look_mat = 1;
                    rot[0] = -v;
                }
                7 if self.anim_mask & 0x200 == 0 => {
                    rot[1] = v + PI;
                    self.use_look_mat = 1;
                }
                8 if self.anim_mask & 0x200 == 0 => {
                    self.use_look_mat = 1;
                    rot[2] = v;
                }
                0xc if self.anim_mask & 8 == 0 => self.yaw = v,
                0xd if self.anim_mask & 0x10 == 0 => self.pitch = v,
                0xe if self.anim_mask & 0x20 == 0 => self.unk_74 = v,
                0xf if self.anim_mask & 0x40 == 0 => self.back_max = v,
                0x10 if self.anim_mask & 1 == 0 => self.pos_time = v,
                0x11 if self.anim_mask & 4 == 0 => self.look_time = v,
                // RwCameraSetFarClipPlane (value argument lost in the decomp: the channel value)
                0x13 => ctx.world.CameraSetFarClipPlane(v),
                0x14 if self.anim_mask & 0x80 == 0 => {
                    ctx.world.CameraSetFOV(((12.44712 / v) as f64).atan() as f32 * 2.0);
                }
                0x15 if self.anim_mask & 2 == 0 => {
                    self.height_time = v;
                    height_set = true;
                }
                _ => {}
            }
        }
        if self.flags & 1 != 0 {
            self.tangent = sp.GetTangent(self.cursor.seg as i32, self.cursor.t);
            self.cursor_flags |= 2;
            let mut m = RwMatrix { at: RwV3dNormalize(self.tangent), ..RwMatrix::identity() };
            BuildFromAt(&mut m, [0.0, 1.0, 0.0]);
            let o = m.TransformVector(self.offset);
            *tgt = add(*tgt, o);
        }
        if self.use_look_mat != 0 {
            let mut d = DTMatrix { right: self.look_mat.right, up: self.look_mat.up, at: self.look_mat.at, pos: self.look_mat.pos };
            d.SetToRotation(rot);
            self.look_mat = RwMatrix { right: d.right, up: d.up, at: d.at, pos: *tgt };
        }
        if !height_set {
            self.height_time = self.pos_time;
        }
        0
    }
}

impl EFDM for EFDMCameraFlex {
    fn base(&self) -> &EFPhysicModule { &self.base }
    fn base_mut(&mut self) -> &mut EFPhysicModule { &mut self.base }
    fn ParamWords(&self) -> usize { 30 }

    /// 0x00224860
    fn SetParams(&mut self, ctx: &mut DMCtx, p: Option<&[u32]>) {
        match p {
            None => self.InitValues(),
            Some(p) => {
                self.target_id = p[0];
                self.target = ctx.world.GetObjectFrame(p[0]).map(|_| p[0]);
                self.anim_id = p[1];
                self.mode = p[2] as u8;
                self.offset = wv(p, 4);
                self.look = wv(p, 7);
                self.pos_time = wf(p, 10);
                self.height_time = wf(p, 11);
                self.look_time = wf(p, 12);
                self.yaw_time = wf(p, 13);
                self.pitch_time = wf(p, 14);
                self.pitch = wf(p, 19);
                self.yaw = wf(p, 18);
                self.unk_74 = wf(p, 20);
                self.max_accel = wf(p, 15);
                self.input_b = p[21];
                self.input_a = p[22];
                self.fpv_yaw_speed = wf(p, 23);
                self.fpv_pitch_speed = wf(p, 24);
                self.back_max = wf(p, 16);
                self.back_accel = wf(p, 17);
                self.search_radius = wf(p, 3);
                self.look_max_angle = wf(p, 27);
                self.flags = p[28];
                self.orbit_speed = wf(p, 26);
                self.fpv_max_pitch = wf(p, 25);
                self.callback = p[29];
            }
        }
        self.InitDM();
    }

    /// 0x002249e0
    fn GetParams(&self, _ctx: &DMCtx, p: &mut [u32]) {
        p[0] = self.target_id;
        p[2] = self.mode as u32;
        put_v(p, 4, self.offset);
        put_v(p, 7, self.look);
        p[10] = fw(self.pos_time);
        p[11] = fw(self.height_time);
        p[12] = fw(self.look_time);
        p[13] = fw(self.yaw_time);
        p[14] = fw(self.pitch_time);
        p[19] = fw(self.pitch);
        p[18] = fw(self.yaw);
        p[20] = fw(self.unk_74);
        p[1] = self.anim_id;
        p[15] = fw(self.max_accel);
        p[21] = self.input_b;
        p[22] = self.input_a;
        p[23] = fw(self.fpv_yaw_speed);
        p[24] = fw(self.fpv_pitch_speed);
        p[25] = fw(self.fpv_max_pitch);
        p[16] = fw(self.back_max);
        p[17] = fw(self.back_accel);
        p[3] = fw(self.search_radius);
        p[27] = fw(self.look_max_angle);
        p[26] = fw(self.orbit_speed);
        p[28] = self.flags;
        p[29] = self.callback;
    }

    /// 0x00224ae0
    fn PreColUpdate(&mut self, ctx: &mut DMCtx, dp: &mut EFDynamicPhysic, dt: f32) -> bool {
        let Some(tid) = self.target else { return true };
        let Some(tm) = ctx.world.GetObjectFrame(tid) else { return true };
        let ltm = ctx.phys.frame;
        let input = ctx.world.InputActionGet(self.input_a, self.input_b);
        if self.mode == 0 {
            // quirk: obj+0x60 is never NULL (even for a missing object)
            self.anim_obj = Some(self.anim_id);
        }
        let cur = ltm.pos;
        if self.mode == 2 {
            self.offset[0] = cur[0] - tm.pos[0];
            self.offset[2] = cur[2] - tm.pos[2];
        }
        let mut tgt = cur;
        let mut lookp = [0.0f32; 3];
        if self.flags & 2 != 0 {
            self.OrbitCam(input, &tm, dt);
        }
        match self.mode {
            0 => self.UpdateCameraAnimated(ctx, &mut tgt, dt),
            1 => tgt = tm.TransformPoint(self.offset),
            2 => {
                let s = ctx.world.CameraFlexTargetScale(tid).unwrap_or(1.0);
                tgt = add(tm.pos, self.offset);
                self.UpdateCameraDistance(ctx, &tm, s, &mut tgt);
            }
            3 => tgt = add(tm.pos, self.offset),
            4 => tgt = self.offset,
            5 => {
                tgt = self.offset;
                self.use_look_mat = 1;
                let m = &mut self.look_mat;
                m.at = sub(self.look, self.offset);
                let upref = if m.at[2] == 0.0 && m.at[0] == 0.0 { [0.0, 0.0, 1.0] } else { [0.0, 1.0, 0.0] };
                m.at = RwV3dNormalize(m.at);
                BuildFromAt(m, upref);
                m.pos = self.offset;
            }
            6 => {
                lookp = if self.flags & 8 != 0 { self.look } else { tm.pos };
                self.UpdateCameraRelDistance(ctx, &tm, lookp, &mut tgt, dt);
            }
            7 => {
                tgt = ctx.phys.frame.pos;
                lookp = self.look;
                if self.flags & 8 == 0 {
                    lookp = add(lookp, tm.pos);
                }
                ctx.world.CallCB(ctx.phys.owner, self.callback);
            }
            _ => {}
        }
        let v = self.MoveToTarget(cur, tgt, dt);
        ctx.phys.velocity = add(dp.disp, v);
        ctx.phys.falling_timer = ctx.phys.falling_delay;
        let mut fpv = false;
        if self.flags & 2 == 0 {
            fpv = self.FirstPersonView(ctx, input, &tm, dt);
        }
        if !fpv {
            match self.mode {
                0..=4 | 6 => lookp = tm.TransformPoint(self.look),
                5 => lookp = tm.pos,
                _ => {}
            }
            self.LookAtHero(ctx, lookp, dt);
        }
        if self.stopped == 0 {
            self.base.ResetStateBits(1, ctx);
        } else {
            self.base.SetStateBits(1, ctx);
        }
        true
    }

    /// 0x00229030
    fn PostColUpdate(&mut self, ctx: &mut DMCtx, _stage: Option<&EFCollisionStage>, dt: f32) {
        let Some(tm) = self.target.and_then(|u| ctx.world.GetObjectFrame(u)) else { return };
        let d = sub(self.cam_pos, ctx.phys.frame.pos);
        if 0.0001 < dot(d, d) {
            self.cam_pos = ctx.phys.frame.pos;
            let l = if self.mode == 5 { tm.pos } else { tm.TransformPoint(self.look) };
            self.LookAtHero(ctx, l, dt);
        }
    }

    /// 0x00228f80
    fn DoAction(&mut self, ctx: &mut DMCtx, action: u32, p: &mut [u32]) {
        if action == 1 {
            self.stopped = 1;
        } else if action == 2 {
            let id = p[0];
            let found = ctx.world.Splines().iter().position(|s| s.ty & 4 != 0 && id == s.id as u32);
            if let Some(i) = found {
                self.SetSpline(i, id as i32);
                self.stopped = 0;
            }
        }
    }
}

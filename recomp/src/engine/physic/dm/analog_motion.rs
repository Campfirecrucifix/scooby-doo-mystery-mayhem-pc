//! 6 EFDMAnalogMotion (Classes/EFDM/EFDMAnalogMotion.cpp, 0xa0 bytes, type 0x50000006), 1:1:
//! camera-relative analog stick motion along the owner's up (mode 2) or at (mode 1) plane, with a
//! turn towards the stick direction (DoRotation) and wall sliding from the PostColUpdate contacts.
//! Seams: the input pointer (+0x64 = InputActionGet result) is re-read each frame through
//! `PhysicWorld::InputActionGet`; the EFCollisionStage contact list (+4/+8) comes from
//! `PhysicWorld::StageContacts`; EFLogicInstance::CallCB from `PhysicWorld::CallCB`.
#![allow(non_snake_case)]

use crate::engine::physic::module::*;
use crate::engine::physic::rwm::*;

/// `(bits & 0x7f800000) < lim` tiny tests used in the class
#[inline]
fn tiny(x: f32, lim: u32) -> bool { (x.to_bits() & 0x7f80_0000) < lim }
#[inline]
fn v_tiny(v: V3) -> bool { tiny(v[0], 0x3c80_0000) && tiny(v[1], 0x3c80_0000) && tiny(v[2], 0x3c80_0000) }

pub struct EFDMAnalogMotion {
    pub base: EFPhysicModule,
    /// +0x30 rotation speed (max), +0x34 rotation speed at zero input (flag 8)
    pub rot_speed: f32,
    pub rot_speed_min: f32,
    /// +0x38 max speed, +0x3c min speed
    pub speed_max: f32,
    pub speed_min: f32,
    /// +0x40 rotation pivot (local; zero = rotate about the frame origin)
    pub pivot: V3,
    /// +0x4c projection mode (2 = plane of up, 1 = plane of at)
    pub mode: u32,
    /// +0x50 flags: 2 velocity = displacement, 4 scale by facing, 8 input-scaled turn speed,
    /// 0x10 keep turning without input, 0x20 speed via logic callback
    pub flags: u32,
    /// +0x54 logic callback
    pub cb: u32,
    /// +0x58 input length threshold for the flag 0x10 turn
    pub threshold: f32,
    /// +0x5c/+0x60 input action
    pub action: [u32; 2],
    /// +0x64 InputActionGet() != NULL
    pub input: bool,
    /// +0x68 angle to the stick direction (cos while projecting, then acos)
    pub angle: f32,
    /// +0x6c/+0x70 last non-zero input
    pub last_input: [f32; 2],
    /// +0x74 motion (world)
    pub motion: V3,
    /// +0x80 rotation axis
    pub axis: V3,
    /// +0x8c touching a wall (friction 0 contacts only)
    pub wall: bool,
    /// +0x90 averaged wall normal (y = 0)
    pub wall_normal: V3,
    /// +0x9c logic instance of the owner (obj+0x10) — the owner uid here
    pub logic: u32,
}

impl EFDMAnalogMotion {
    pub fn new() -> Self {
        let mut s = EFDMAnalogMotion {
            base: EFPhysicModule { type_: 0x5000_0006, flags: 1, ..Default::default() },
            rot_speed: 0.0,
            rot_speed_min: 0.0,
            speed_max: 0.0,
            speed_min: 0.0,
            pivot: [0.0; 3],
            mode: 0,
            flags: 0,
            cb: 0,
            threshold: 0.0,
            action: [0; 2],
            input: false,
            angle: 0.0,
            last_input: [0.0; 2],
            motion: [0.0; 3],
            axis: [0.0; 3],
            wall: false,
            wall_normal: [0.0; 3],
            logic: 0,
        };
        s.SetDefaultParams();
        s
    }

    pub fn SetDefaultParams(&mut self) {
        self.rot_speed = 1.0;
        self.rot_speed_min = 1.0;
        self.speed_max = 1.0;
        self.speed_min = 0.0;
        self.flags = 0;
        self.action = [0, 0];
        self.mode = 2;
        self.pivot = [0.0; 3];
        self.motion = [0.0; 3];
        self.wall = false;
        self.cb = 0;
        self.threshold = 0.25;
    }

    pub fn GetInputLength(x: f32, y: f32) -> f32 {
        let l = (y * y + x * x).sqrt();
        if l <= 1.0 { l } else { 1.0 }
    }

    pub fn ProjectInputs(&mut self, ctx: &mut DMCtx, x: f32, y: f32) {
        let cam = ctx.world.GetCameraLTM();
        let obj = ctx.phys.frame;
        let len = Self::GetInputLength(x, y);
        let mut v = [0.0f32; 3];
        if self.mode == 2 {
            let d = dot(cam.at, obj.up).abs();
            v[1] = y * d;
            v[2] = y * (1.0 - d);
            v[0] = -x;
        } else if self.mode == 1 {
            let d = dot(cam.at, obj.up).abs();
            v[2] = y * d;
            v[1] = y * (1.0 - d);
            v[0] = -x;
        }
        let w = cam.TransformVector(v);
        let mut fac = 1.0f32;
        let a;
        if self.mode == 2 || self.mode == 1 {
            let (n, f_ref, def) = if self.mode == 2 { (obj.up, obj.at, obj.up) } else { (obj.at, obj.up, obj.at) };
            let c = cross(w, n);
            let f = RwV3dNormalize(cross(n, c));
            self.angle = dot(f_ref, f);
            self.axis = cross(f_ref, f);
            if v_tiny(self.axis) {
                self.axis = def;
            } else {
                self.axis = RwV3dNormalize(self.axis);
            }
            if self.flags & 4 != 0 {
                fac = (self.angle + 1.0) / 2.0;
            }
            let mut s = len * fac;
            if self.flags & 0x20 == 0 {
                s = len * fac * (self.speed_max - self.speed_min) + self.speed_min;
            } else {
                ctx.world.CallCB(self.logic, self.cb);
            }
            self.motion = if self.mode == 2 { [0.0, 0.0, s] } else { [0.0, s, 0.0] };
            a = self.angle;
        } else {
            a = self.angle;
        }
        self.angle = if 1.0 < a {
            0.0
        } else if a < -1.0 {
            f32::from_bits(0x4049_0fdb)
        } else {
            (a as f64).acos() as f32
        };
        self.motion = ctx.phys.frame.TransformVector(self.motion);
    }

    pub fn DoRotation(&mut self, ctx: &mut DMCtx, len: f32, dt: f32) -> f32 {
        let sp = if self.flags & 8 == 0 { self.rot_speed } else { self.rot_speed_min + len * (self.rot_speed - self.rot_speed_min) };
        let a = self.angle * sp * dt;
        if 0.001 < a {
            let f = &mut ctx.phys.frame;
            if v_tiny(self.pivot) {
                let pos = f.pos;
                f.Rotate(a * RAD2DEG, self.axis, rwCOMBINEPOSTCONCAT);
                f.pos = pos;
            } else {
                // quirk: postconcat also rotates the position about the world origin; only the
                // pivot's rotation delta is subtracted afterwards (as in the C++).
                let before = f.TransformVector(self.pivot);
                f.Rotate(a * RAD2DEG, self.axis, rwCOMBINEPOSTCONCAT);
                let after = f.TransformVector(self.pivot);
                f.pos = sub(f.pos, sub(after, before));
            }
        }
        a
    }
}

impl EFDM for EFDMAnalogMotion {
    fn base(&self) -> &EFPhysicModule { &self.base }
    fn base_mut(&mut self) -> &mut EFPhysicModule { &mut self.base }
    fn ParamWords(&self) -> usize { 13 }
    fn SetParams(&mut self, ctx: &mut DMCtx, p: Option<&[u32]>) {
        let Some(p) = p else { return self.SetDefaultParams() };
        self.rot_speed = wf(p, 0);
        self.rot_speed_min = wf(p, 1);
        self.speed_max = wf(p, 2);
        self.speed_min = wf(p, 3);
        self.flags = p[4];
        self.action = [p[5], p[6]];
        self.mode = p[7];
        self.pivot = wv(p, 8);
        self.cb = p[11];
        self.threshold = wf(p, 12);
        self.input = ctx.world.InputActionGet(self.action[0], self.action[1]).is_some();
    }
    fn GetParams(&self, _c: &DMCtx, p: &mut [u32]) {
        p[0] = fw(self.rot_speed);
        p[1] = fw(self.rot_speed_min);
        p[2] = fw(self.speed_max);
        p[3] = fw(self.speed_min);
        p[4] = self.flags;
        p[5] = self.action[0];
        p[6] = self.action[1];
        p[7] = self.mode;
        put_v(p, 8, self.pivot);
        p[11] = self.cb;
        p[12] = fw(self.threshold);
    }
    fn PreColUpdate(&mut self, ctx: &mut DMCtx, dp: &mut EFDynamicPhysic, dt: f32) -> bool {
        if !self.input {
            return true;
        }
        self.logic = ctx.phys.owner;
        let [x, y] = ctx.world.InputActionGet(self.action[0], self.action[1]).unwrap_or([0.0; 2]);
        if tiny(x, 0x3c00_0000) && tiny(y, 0x3c00_0000) {
            if self.flags & 2 != 0 {
                ctx.phys.velocity = dp.disp;
                ctx.phys.falling_timer = ctx.phys.falling_delay;
            }
            if self.flags & 0x10 != 0 {
                let l = Self::GetInputLength(self.last_input[0], self.last_input[1]);
                if self.threshold < l {
                    let r = self.DoRotation(ctx, l, dt);
                    self.angle -= r;
                }
            }
        } else {
            self.last_input = [x, y];
            self.ProjectInputs(ctx, x, y);
            let l = Self::GetInputLength(x, y);
            self.DoRotation(ctx, l, dt);
            if self.wall {
                let d = dot(self.motion, self.wall_normal);
                if d < 0.0 {
                    self.motion = sub(self.motion, scale(self.wall_normal, d));
                }
            }
            dp.disp = add(dp.disp, self.motion);
            if self.flags & 2 != 0 {
                ctx.phys.velocity = dp.disp;
                ctx.phys.falling_timer = ctx.phys.falling_delay;
            }
        }
        true
    }
    fn DoAction(&mut self, ctx: &mut DMCtx, action: u32, p: &mut [u32]) {
        if action == 1 {
            if 0x3bff_ffff < (p[0] & 0x7f80_0000) || 0x3bff_ffff < (p[1] & 0x7f80_0000) {
                self.ProjectInputs(ctx, wf(p, 0), wf(p, 1));
            }
            put_v(p, 2, self.motion);
            p[5] = fw(self.angle);
        }
    }
    fn PostColUpdate(&mut self, ctx: &mut DMCtx, _stage: Option<&EFCollisionStage>, _dt: f32) {
        let contacts = ctx.world.StageContacts(ctx.phys.owner);
        self.PostColContacts(&contacts);
    }
}

impl EFDMAnalogMotion {
    /// PostColUpdate body over the stage contacts (material friction (+0xc of contact+0), normal +0xc).
    pub fn PostColContacts(&mut self, contacts: &[(f32, V3)]) {
        self.wall_normal = [0.0; 3];
        self.wall = false;
        let mut n = 0u32;
        for &(fr, nrm) in contacts {
            if fr == 0.0 {
                self.wall = true;
                n += 1;
                self.wall_normal = add(self.wall_normal, nrm);
            }
            if 0.0 < fr {
                self.wall = false;
                return;
            }
        }
        if self.wall {
            self.wall_normal = scale(self.wall_normal, 1.0 / n as f32);
            self.wall_normal[1] = 0.0;
            if 0.0 < dot(self.wall_normal, self.wall_normal) {
                self.wall_normal = RwV3dNormalize(self.wall_normal);
            }
        }
    }
}

//! 0xe EFDMConstraintRotation (Classes/EFDM/EFDMConstraintRotation.cpp, 0x001e4ef0..), 1:1.
//! Turns the owner frame (about a local pivot) until its at / up axes match target directions.
//! The owner is a root frame (parent LTM transform of the snap branch is skipped; see module.rs).
#![allow(non_snake_case)]

use crate::engine::physic::module::*;
use crate::engine::physic::rwm::*;

/// 0x60 bytes. Params (10 words): 0 up xyz, 3 at xyz, 6 pivot xyz, 9 speed.
pub struct EFDMConstraintRotation {
    pub base: EFPhysicModule,
    /// +0x30 target up
    pub up: V3,
    /// +0x3c target at
    pub at: V3,
    /// +0x48 local pivot kept in place by the rotation
    pub pivot: V3,
    /// +0x54 up constraint set (|up| not ~0)
    pub has_up: bool,
    /// +0x55 at constraint set
    pub has_at: bool,
    /// +0x58 speed (fraction of the remaining angle per second, clamped to 1 per frame)
    pub speed: f32,
}

/// `((uint)(x - 0.0) & 0x7f800000) < 0x3e800000` (|x| < 0.25)
fn small(x: f32) -> bool { ((x - 0.0).to_bits() & 0x7f80_0000) < 0x3e80_0000 }

impl EFDMConstraintRotation {
    pub fn new() -> Self {
        EFDMConstraintRotation {
            base: EFPhysicModule { type_: 0x1000_000e, flags: 1, ..Default::default() },
            up: [0.0; 3],
            at: [0.0; 3],
            pivot: [0.0; 3],
            has_up: false,
            has_at: false,
            speed: 0.0,
        }
    }
    /// rotate the frame about `axis` (postconcat) and shift it so the pivot stays put
    fn rotate_about_pivot(&self, m: &mut RwMatrix, deg: f32, axis: V3) {
        let before = m.TransformVector(self.pivot);
        m.Rotate(deg, axis, rwCOMBINEPOSTCONCAT);
        let after = m.TransformVector(self.pivot);
        m.pos = sub(m.pos, sub(after, before));
    }
}

impl EFDM for EFDMConstraintRotation {
    fn base(&self) -> &EFPhysicModule { &self.base }
    fn base_mut(&mut self) -> &mut EFPhysicModule { &mut self.base }
    fn ParamWords(&self) -> usize { 10 }
    fn SetParams(&mut self, _c: &mut DMCtx, p: Option<&[u32]>) {
        match p {
            None => {
                self.up = [0.0; 3];
                self.at = [0.0; 3];
                self.pivot = [0.0; 3];
                self.speed = 0.0;
            }
            Some(p) => {
                self.up = wv(p, 0);
                self.at = wv(p, 3);
                self.pivot = wv(p, 6);
                self.speed = wf(p, 9);
            }
        }
        self.has_up = !(small(self.up[0]) && small(self.up[1]) && small(self.up[2]));
        if self.has_up {
            self.up = RwV3dNormalize(self.up);
        }
        self.has_at = !(small(self.at[0]) && small(self.at[1]) && small(self.at[2]));
        if self.has_at {
            self.at = RwV3dNormalize(self.at);
        }
    }
    /// quirk: the pivot (words 6..8) is not written back
    fn GetParams(&self, _c: &DMCtx, p: &mut [u32]) {
        put_v(p, 0, self.up);
        put_v(p, 3, self.at);
        p[9] = fw(self.speed);
    }
    fn PreColUpdate(&mut self, ctx: &mut DMCtx, _dp: &mut EFDynamicPhysic, dt: f32) -> bool {
        let mut f = self.speed * dt;
        if 1.0 <= f {
            f = 1.0;
        }
        let mut m = ctx.phys.frame;
        let cur_at = m.at;
        let cur_up = m.up;
        let mut done = true;
        if self.has_at {
            let d = dot(cur_at, self.at).clamp(-1.0, 1.0);
            let ang = d.acos();
            done = (ang.to_bits() & 0x7f80_0000) < 0x3e00_0000;
            let axis = cross(cur_at, self.at);
            self.rotate_about_pivot(&mut m, ang * f * RAD2DEG, axis);
        }
        if self.has_up {
            let d = dot(cur_up, self.up);
            if f != 1.0 && (d.abs() as f64) < 1.0 {
                let ang = d.acos();
                done &= (ang.to_bits() & 0x7f80_0000) < 0x3e00_0000;
                let axis = cross(cur_up, self.up);
                self.rotate_about_pivot(&mut m, ang * f * RAD2DEG, axis);
            } else {
                // snap: up = target, right = up x at, at = right x up (RwFrameUpdateObjects)
                let up = self.up;
                m.up = up;
                m.right = cross(up, m.at);
                m.at = cross(m.right, up);
                done = true;
            }
        }
        ctx.phys.frame = m;
        if done {
            self.base.SetStateBits(1, ctx);
        }
        true
    }
}

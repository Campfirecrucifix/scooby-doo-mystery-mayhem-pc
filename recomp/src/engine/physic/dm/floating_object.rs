//! 0x15 EFDMFloatingObject (Classes/EFDM/EFDMFloatingObject.cpp, 0x0023d7e0..0x0023df4b, 0x4c
//! bytes, type 0x10000015), 1:1: the owner bobs on the first EFDMWaterSimulation (kind 0xc) found
//! in the manager's physic list 0: its root bone is tilted towards the water normal and its
//! vertical velocity set to reach `water level + offset + height * scale` this frame.
//! DM state bit 1 = in the water (notify 6 on change).
//!
//! Seams:
//! - GetWaterSimModule (walks _gpoPhysicMgr+0x200c, EFPhysic::FindModule(0xc)): the modules are
//!   borrowed one at a time during the update, so the lookup is the module-local trait
//!   [`WaterSimSource`] in `source` (None = no module found). [`FindWaterSimModule`] is the 1:1
//!   list walk over an `EFPhysicMgr`; [`WaterSimSnapshot`] wraps its result.
//! - Get3dObject(owner,0) HAnim root frame (hierarchy +0x10 -> +0xc, node 0) -> `PhysicWorld::
//!   GetBone(owner, 0)` / `SetBoneModelling`. Without a hierarchy the original calls
//!   RwFrameGetLTM(NULL); here the physic frame is used as LTM and no rotation is written.
//! - DTQuaternion SetValue/SetFromAxes/Slerpf/GetValue are ported locally (DTQuaternion.cpp).
#![allow(non_snake_case)]

use std::sync::Arc;

use super::water_simulation::WaterSimRef;
use crate::engine::physic::mgr::EFPhysicMgr;
use crate::engine::physic::module::*;
use crate::engine::physic::rwm::*;

/// GetWaterSimModule() provider.
pub trait WaterSimSource: Send + Sync {
    fn GetWaterSimModule(&self) -> Option<WaterSimRef> { None }
}

/// A resolved lookup (e.g. `WaterSimSnapshot(FindWaterSimModule(&mgr))`).
pub struct WaterSimSnapshot(pub Option<WaterSimRef>);
impl WaterSimSource for WaterSimSnapshot {
    fn GetWaterSimModule(&self) -> Option<WaterSimRef> { self.0.clone() }
}

/// EFDMFloatingObject::GetWaterSimModule's walk: physic list 0 (+0x200c), first physic with a
/// kind 0xc module.
pub fn FindWaterSimModule(mgr: &EFPhysicMgr) -> Option<WaterSimRef> {
    for &id in &mgr.lists[0] {
        let h = mgr.FindModule(id, 0xc, 0);
        if h != 0 {
            let m = mgr.Module(h)?;
            let w = m.as_water_sim()?;
            return Some(w.sim.clone());
        }
    }
    None
}

/// DTQuaternion::SetFromAxes(a, b) ([x, y, z, w])
fn SetFromAxes(a: V3, b: V3) -> [f32; 4] {
    let d = dot(a, b);
    if 0.99999 < d {
        [0.0, 0.0, 0.0, 1.0]
    } else if d < -0.99999 {
        let (mut f4, mut f5, mut f2) = (a[0], 0.0f32, -a[1]);
        let mut f3 = 0.0;
        if ((f2 * f2 + f4 * f4).sqrt() as f64) < 1e-6 {
            f2 = a[0];
            f4 = 0.0;
            f5 = -a[2];
            f3 = f5 * f5;
        }
        let s = 1.0 / (f2 * f2 + f3 + f4 * f4).sqrt();
        [f5 * s, f4 * s, f2 * s, 0.0]
    } else {
        let h = ((1.0 - d) * 0.5).sqrt();
        let c = cross(a, b);
        let s = 1.0 / (c[2] * c[2] + c[0] * c[0] + c[1] * c[1]).sqrt();
        [c[0] * s * h, c[1] * s * h, c[2] * s * h, ((d + 1.0) * 0.5).sqrt()]
    }
}

/// DTQuaternion::SetValue(axis, angle, normalized=true)
fn SetValue(axis: V3, ang: f32) -> [f32; 4] {
    let (s, c) = ((ang / 2.0).sin(), (ang / 2.0).cos());
    [axis[0] * s, axis[1] * s, axis[2] * s, c]
}

/// DTQuaternion::Slerpf(a, b, t)
fn Slerpf(a: [f32; 4], b: [f32; 4], mut t: f32) -> [f32; 4] {
    let mut b = b;
    let mut d = a[3] * b[3] + a[2] * b[2] + a[0] * b[0] + a[1] * b[1];
    if d < 0.0 {
        d = -d;
        b = [-b[0], -b[1], -b[2], -b[3]];
    }
    let s0;
    // _dpfgt(1 - d, 1e-6 as double)
    if ((1.0 - d) as f64) > f64::from_bits(0x3eb0c6f7a0b5ed8d) {
        let o = d.acos();
        let so = o.sin();
        s0 = ((1.0 - t) * o).sin() / so;
        t = (t * o).sin() / so;
    } else {
        s0 = 1.0 - t;
    }
    [t * b[0] + s0 * a[0], t * b[1] + s0 * a[1], t * b[2] + s0 * a[2], t * b[3] + s0 * a[3]]
}

/// DTQuaternion::GetValue(axis, angle)
fn GetValue(q: [f32; 4]) -> (V3, f32) {
    let l = q[2] * q[2] + q[0] * q[0] + q[1] * q[1];
    let (axis, mut ang) = if !((l as f64) > 1e-6) {
        ([0.0, 0.0, 1.0], 0.0)
    } else {
        // quirk: divides by the squared length (RwFrameRotate re-normalises the axis)
        let s = 1.0 / l;
        ([q[0] * s, q[1] * s, q[2] * s], q[3].acos() * 2.0)
    };
    if 3.1415927 < ang {
        ang = 3.1415927;
    }
    (axis, ang)
}

pub struct EFDMFloatingObject {
    pub base: EFPhysicModule,
    /// +0x30 tilt scale (1.0)
    pub tilt: f32,
    /// +0x34 height offset (0)
    pub offset: f32,
    /// +0x38 wave height scale (1.0)
    pub height_scale: f32,
    /// +0x3c max height above the water level (0.1)
    pub max_above: f32,
    /// +0x40 the water simulation (NULL = None)
    pub water: Option<WaterSimRef>,
    /// +0x44 water level (grid origin y)
    pub level: f32,
    /// +0x48 look the water module up on the next update
    pub lookup: bool,
    /// GetWaterSimModule provider
    pub source: Option<Arc<dyn WaterSimSource>>,
}

impl EFDMFloatingObject {
    pub fn new() -> Self {
        let mut m = EFDMFloatingObject {
            base: EFPhysicModule { type_: 0x1000_0015, flags: 1, ..Default::default() },
            tilt: 0.0,
            offset: 0.0,
            height_scale: 0.0,
            max_above: 0.0,
            water: None,
            level: 0.0,
            lookup: false,
            source: None,
        };
        m.SetDefaultParams();
        m
    }

    /// EFDMFloatingObject::SetDefaultParams
    pub fn SetDefaultParams(&mut self) {
        self.tilt = 1.0;
        self.offset = 0.0;
        self.height_scale = 1.0;
        self.max_above = 0.1;
        self.water = None;
        self.lookup = true;
        self.level = 0.0;
    }

    /// EFDMFloatingObject::GetWaterSimModule
    pub fn GetWaterSimModule(&self) -> Option<WaterSimRef> {
        self.source.as_ref().and_then(|s| s.GetWaterSimModule())
    }
}

impl Default for EFDMFloatingObject {
    fn default() -> Self { Self::new() }
}

impl EFDM for EFDMFloatingObject {
    fn base(&self) -> &EFPhysicModule { &self.base }
    fn base_mut(&mut self) -> &mut EFPhysicModule { &mut self.base }
    fn ParamWords(&self) -> usize { 4 }

    fn SetParams(&mut self, _ctx: &mut DMCtx, p: Option<&[u32]>) {
        let Some(p) = p else {
            self.SetDefaultParams();
            return;
        };
        self.tilt = wf(p, 0);
        self.offset = wf(p, 1);
        self.height_scale = wf(p, 2);
        self.max_above = wf(p, 3);
        self.water = None;
        self.lookup = true;
    }

    fn GetParams(&self, _ctx: &DMCtx, p: &mut [u32]) {
        p[0] = fw(self.tilt);
        p[1] = fw(self.offset);
        p[2] = fw(self.height_scale);
        p[3] = fw(self.max_above);
    }

    fn PreColUpdate(&mut self, ctx: &mut DMCtx, dp: &mut EFDynamicPhysic, dt: f32) -> bool {
        let owner = ctx.phys.owner;
        let bone = ctx.world.GetBone(owner, 0);
        let ltm = bone.map_or(ctx.phys.frame, |b| b.1);
        let (up, at) = (ltm.up, ltm.at);
        if self.lookup {
            self.water = self.GetWaterSimModule();
            let Some(w) = self.water.as_ref() else { return true };
            // the grid plane normal (normalize(cross(x axis, z axis))) is computed and unused
            self.level = w.read().unwrap().origin[1];
            self.lookup = false;
        }
        let mut normal = [0.0f32; 3];
        let mut h = 0.0f32;
        // NULL module after the lookup (SetParams cleared it without lookup) would crash: outside
        let mut r = match self.water.as_ref() {
            Some(w) => w.read().unwrap().GetWaterValueAtThisPos(dp.pos, &mut normal, &mut h),
            None => 2,
        };
        if self.level + self.max_above < dp.pos[1] {
            r = 2;
        }
        if r == 2 {
            if self.base.state & 1 != 0 {
                self.base.ResetStateBits(1, ctx);
            }
        } else {
            if self.base.state & 1 == 0 {
                self.base.SetStateBits(1, ctx);
            }
            let q0 = SetValue(at, 0.0);
            let q1 = SetFromAxes(up, normal);
            if q1[3] != 0.0 {
                let q = Slerpf(q0, q1, 1.0);
                let (axis, mut ang) = GetValue(q);
                if ang != 0.0 {
                    ang *= self.tilt;
                    if let Some((mut m, _)) = bone {
                        let pos = m.pos;
                        m.Rotate(ang * RAD2DEG, axis, rwCOMBINEPOSTCONCAT);
                        m.pos = pos;
                        ctx.world.SetBoneModelling(owner, 0, m);
                    }
                }
            }
            let p = &mut *ctx.phys;
            p.velocity[1] = ((self.height_scale * h + self.level + self.offset) - dp.pos[1]) / dt;
            p.falling_timer = p.falling_delay;
        }
        true
    }
}

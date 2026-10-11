//! 0x25 EFDMPlugin (Classes/EFDM/EFDMPlugin.cpp, 0x00267960..), 1:1.
//! Calls logic callbacks of the owner before (0x12) and after (0x13) the collision stage.
//!
//! Replacement: the C++ registers EFNotificationMgr notifications (ids 0x12 / 0x13, source = target
//! = the owner uid) whose handler is EFGameLogicMgr::ExecLogicCallback(0x001fd300): push the
//! object on _gpoLogicStack and EFLogicInstance::CallCB(obj+0x2c, cb, &dt). Here the module emits
//! `PhysicNotify { owner, code: 0x12|0x13, data: [cb, dt bits, 0] }`; the coordinator runs the
//! callback. (RegisterNotify happens on every SetParams; re-registration duplicates are not
//! modelled.)
#![allow(non_snake_case)]

use crate::engine::physic::module::*;

/// 0x40 bytes. Params (2 words): pre callback, post callback (-1 = none).
pub struct EFDMPlugin {
    pub base: EFPhysicModule,
    /// +0x30 PreColUpdate logic callback (-1)
    pub pre_cb: u32,
    /// +0x34 PostColUpdate logic callback (-1)
    pub post_cb: u32,
    /// +0x38 owner uid (instance+8), set by SetParams
    pub owner: u32,
}

impl EFDMPlugin {
    pub fn new() -> Self {
        EFDMPlugin {
            base: EFPhysicModule { type_: 0x5000_0025, flags: 1, ..Default::default() },
            pre_cb: 0xffff_ffff,
            post_cb: 0xffff_ffff,
            owner: 0,
        }
    }
}

impl EFDM for EFDMPlugin {
    fn base(&self) -> &EFPhysicModule { &self.base }
    fn base_mut(&mut self) -> &mut EFPhysicModule { &mut self.base }
    fn ParamWords(&self) -> usize { 2 }
    fn SetParams(&mut self, ctx: &mut DMCtx, p: Option<&[u32]>) {
        if let Some(p) = p {
            self.pre_cb = p[0];
            self.post_cb = p[1];
            self.owner = ctx.phys.owner;
        }
    }
    fn GetParams(&self, _c: &DMCtx, p: &mut [u32]) {
        p[0] = self.pre_cb;
        p[1] = self.post_cb;
    }
    fn PreColUpdate(&mut self, ctx: &mut DMCtx, _dp: &mut EFDynamicPhysic, dt: f32) -> bool {
        if self.pre_cb != 0xffff_ffff {
            ctx.notify.push(PhysicNotify { owner: self.owner, code: 0x12, data: [self.pre_cb, dt.to_bits(), 0, 0] });
        }
        true
    }
    fn PostColUpdate(&mut self, ctx: &mut DMCtx, _s: Option<&EFCollisionStage>, dt: f32) {
        if self.post_cb != 0xffff_ffff {
            ctx.notify.push(PhysicNotify { owner: self.owner, code: 0x13, data: [self.post_cb, dt.to_bits(), 0, 0] });
        }
    }
}

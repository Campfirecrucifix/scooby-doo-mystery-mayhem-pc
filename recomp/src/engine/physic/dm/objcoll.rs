//! 0x23 EFDMObjColl (Classes/EFDM/EFDMObjColl.cpp, 0x40 bytes), 1:1.
//! Collision notification seam: SetParams registers EFDMObjColl::CollisionCB for notification
//! 0x17 of the owner (EFNotificationMgr, _gpoGameLogicMgr+0x14). Here `registered` is set and the
//! coordinator, when the owner receives notify 0x17 {_, prim tag, other uid, other prim tag},
//! calls `CollisionCB` on every 0x23 module of the owner's physic (FindModule(0x23) loop).
#![allow(non_snake_case)]

use crate::engine::physic::module::*;

pub struct EFDMObjColl {
    pub base: EFPhysicModule,
    /// +0x30 own prim tag (-1)
    pub prim_tag: u32,
    /// +0x34 other object's prim tag (-1)
    pub other_prim_tag: u32,
    /// +0x38 other object uid
    pub other: u32,
    /// +0x3c collided this frame
    pub hit: u32,
    /// RegisterNotify(0x17, owner, CollisionCB) done (replacement for the notification manager)
    pub registered: bool,
}

impl EFDMObjColl {
    pub fn new() -> Self {
        EFDMObjColl {
            base: EFPhysicModule { type_: 0x4000_0023, flags: 1, ..Default::default() },
            prim_tag: 0xffff_ffff,
            other_prim_tag: 0xffff_ffff,
            other: 0,
            hit: 0,
            registered: false,
        }
    }
    /// EFDMObjColl::CollisionCB(instance, data): data words +4 prim tag, +8 other uid, +0xc other tag.
    pub fn CollisionCB(&mut self, data: &[u32; 4]) -> u32 {
        if data[1] == self.prim_tag && data[2] == self.other && data[3] == self.other_prim_tag {
            self.hit = 1;
        }
        0
    }
}

impl EFDM for EFDMObjColl {
    fn base(&self) -> &EFPhysicModule { &self.base }
    fn base_mut(&mut self) -> &mut EFPhysicModule { &mut self.base }
    fn ParamWords(&self) -> usize { 3 }
    fn SetParams(&mut self, _ctx: &mut DMCtx, p: Option<&[u32]>) {
        if let Some(p) = p {
            self.prim_tag = p[0];
            self.other_prim_tag = p[1];
            self.other = p[2];
            // the two PrimTagToPrimIndex calls have their results discarded
            self.registered = true;
        }
        self.hit = 0;
    }
    fn GetParams(&self, _c: &DMCtx, p: &mut [u32]) {
        p[0] = self.prim_tag;
        p[1] = self.other_prim_tag;
        p[2] = self.other;
    }
    fn PostColUpdate(&mut self, ctx: &mut DMCtx, stage: Option<&EFCollisionStage>, _dt: f32) {
        if stage.is_none() {
            return;
        }
        let b = &mut self.base;
        // quirk: state is zeroed before the bit is set, so the change mask is always the new bit
        let bit = if b.state & 1 == 0 && self.hit == 1 {
            1
        } else if b.state & 1 != 0 && self.hit == 0 {
            2
        } else {
            0
        };
        if bit != 0 {
            b.state = bit;
            if bit & b.notify_bits != 0 {
                ctx.notify.push(PhysicNotify { owner: ctx.phys.owner, code: 6, data: [b.handle, b.type_, b.state, bit] });
            }
        }
        self.hit = 0;
    }
}

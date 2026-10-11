//! 0xb EFDMPrimMove (Classes/EFDM/EFDMPrimMove.cpp, 0x001e5ac0..), 1:1.
//! Derives the physic velocity from the displacement of one collision prim between frames.
#![allow(non_snake_case)]

use crate::engine::physic::module::*;
use crate::engine::physic::rwm::*;

/// 0x44 bytes. +0x30 previous prim world pos, +0x3c prim index, +0x40 frame stamp.
pub struct EFDMPrimMove {
    pub base: EFPhysicModule,
    /// +0x30
    pub prev_pos: V3,
    /// +0x3c prim index in the collision model (+0x58 table)
    pub prim: u32,
    /// +0x40 _gpoPhysicMgr+0x2024 of the last update
    pub stamp: u32,
}

impl EFDMPrimMove {
    pub fn new() -> Self {
        EFDMPrimMove {
            base: EFPhysicModule { type_: 0x1000_000b, flags: 1, ..Default::default() },
            prev_pos: [0.0; 3],
            prim: 0,
            stamp: 0,
        }
    }
}

impl EFDM for EFDMPrimMove {
    fn base(&self) -> &EFPhysicModule { &self.base }
    fn base_mut(&mut self) -> &mut EFPhysicModule { &mut self.base }
    fn ParamWords(&self) -> usize { 1 }
    fn SetParams(&mut self, ctx: &mut DMCtx, p: Option<&[u32]>) {
        self.prim = match p {
            None => 0,
            Some(p) => ctx.phys.model.PrimTagToPrimIndex(p[0]),
        };
    }
    fn GetParams(&self, ctx: &DMCtx, p: &mut [u32]) {
        // quirk: C++ dereferences (NULL)+8 when the model has no prim table; port writes 0.
        p[0] = ctx.phys.model.prims.get(self.prim as usize).and_then(|q| q.as_ref()).map_or(0, |q| q.base.tag);
    }
    fn PreColUpdate(&mut self, ctx: &mut DMCtx, dp: &mut EFDynamicPhysic, dt: f32) -> bool {
        let (frame, paused, prev_paused) = (ctx.frame, ctx.paused_frames, ctx.prev_paused);
        let p = &mut *ctx.phys;
        let Some(Some(prim)) = p.model.prims.get_mut(self.prim as usize) else { return true };
        let prim = &mut prim.base;
        if prim.word(0xc) & 0x10 == 0 {
            // re-seed the previous pos when teleported or when a frame was skipped (unless the
            // gap is exactly the paused frame count)
            if prim.d8 & 2 != 0
                || (frame != self.stamp.wrapping_add(1)
                    && (prev_paused != 1 || frame.wrapping_sub(self.stamp.wrapping_add(1)) != paused))
            {
                self.prev_pos = prim.center;
                prim.d8 &= 0xffff_fffd;
            }
            // quirk: (prev - current) / dt, and prev_pos is NOT refreshed here
            let v = scale(sub(self.prev_pos, prim.center), 1.0 / dt);
            p.velocity = add(dp.disp, v);
            p.falling_timer = p.falling_delay;
            self.stamp = frame;
        }
        true
    }
}

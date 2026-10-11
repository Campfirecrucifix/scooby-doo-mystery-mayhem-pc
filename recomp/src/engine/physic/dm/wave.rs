//! 9 EFDMWave (Classes/EFDM/EFDMWave.cpp, 0x001e8620..), 1:1.
//! Back-and-forth motion between two points with an accelerate / cruise / decelerate profile.
#![allow(non_snake_case)]

use crate::engine::physic::module::*;
use crate::engine::physic::rwm::*;

/// 100 bytes. Param block (10 words): 0 start xyz, 3 end xyz, 6 speed, 7 accel time,
/// 8 period, 9 flags.
pub struct EFDMWave {
    pub base: EFPhysicModule,
    /// +0x30 cruise speed (1.0) — the acceleration is speed / accel time
    pub speed: f32,
    /// +0x34 start point
    pub start: V3,
    /// +0x40 end point
    pub end: V3,
    /// +0x4c acceleration time
    pub accel_time: f32,
    /// +0x50 full period (2.0)
    pub period: f32,
    /// +0x54 distance covered while accelerating
    pub accel_dist: f32,
    /// +0x58 flags: 2 = period given (else accel time), 4 = snap to start, 8 = snap to end and
    /// start half-way, 0x10 = reset the physic velocity
    pub flags: u32,
    /// +0x5c time in the period
    pub time: f32,
    /// +0x60 first update after SetParams (SetFlags skipped once)
    pub first: bool,
}

impl EFDMWave {
    pub fn new() -> Self {
        EFDMWave {
            base: EFPhysicModule { type_: 0x1000_0009, flags: 1, ..Default::default() },
            speed: 1.0,
            start: [0.0; 3],
            end: [0.0; 3],
            accel_time: 0.0,
            period: 2.0,
            accel_dist: 0.0,
            flags: 0,
            time: 0.0,
            first: true,
        }
    }

    /// EFDMWave::SetFlags(float dt): DM state bit 0x20 = in the first half of the period this
    /// frame, 0x40 = crossing/at the half-period point.
    pub fn SetFlags(&mut self, ctx: &mut DMCtx, dt: f32) {
        if self.first {
            self.first = false;
            return;
        }
        if dt < self.time || self.time + dt < 0.0 {
            if self.base.state & 0x20 != 0 {
                self.base.ResetStateBits(0x20, ctx);
            }
        } else if self.base.state & 0x20 == 0 {
            self.base.SetStateBits(0x20, ctx);
        }
        let h = self.period * 0.5;
        if h < self.time || self.time + dt < h {
            if self.base.state & 0x40 != 0 {
                self.base.ResetStateBits(0x40, ctx);
            }
        } else if self.base.state & 0x40 == 0 {
            self.base.SetStateBits(0x40, ctx);
        }
    }
}

impl EFDM for EFDMWave {
    fn base(&self) -> &EFPhysicModule { &self.base }
    fn base_mut(&mut self) -> &mut EFPhysicModule { &mut self.base }
    fn ParamWords(&self) -> usize { 10 }
    fn SetParams(&mut self, ctx: &mut DMCtx, p: Option<&[u32]>) {
        self.first = true;
        let Some(p) = p else {
            self.speed = 1.0;
            let pos = ctx.phys.frame.pos;
            self.start = pos;
            self.end = [pos[0], pos[1], pos[2] + 1.0];
            self.accel_time = 0.0;
            self.accel_dist = 0.0;
            self.period = 2.0;
            self.time = 0.0;
            self.flags = 2;
            return;
        };
        self.speed = wf(p, 6);
        self.start = wv(p, 0);
        self.end = wv(p, 3);
        self.time = 0.0;
        let d = sub(self.end, self.start);
        self.flags = p[9];
        if self.flags & 2 == 0 {
            let half = RwV3dLength(d) * 0.5;
            let ta = wf(p, 7);
            self.accel_time = ta;
            if 0.0 < ta {
                let ad = ta * (self.speed / ta) * ta * 0.5;
                self.accel_dist = ad;
                if half < ad {
                    self.accel_dist = half;
                    let t = ((half * 2.0) / (self.speed / ta)).sqrt();
                    self.accel_time = t;
                    self.period = t * 4.0;
                    self.speed = (self.accel_dist * 2.0) / self.accel_time;
                } else {
                    self.period = (self.accel_time + (half - ad) / self.speed) * 4.0;
                }
            } else {
                self.accel_dist = 0.0;
                self.period = (half * 4.0) / self.speed;
            }
        } else {
            let len = RwV3dLength(d);
            self.period = wf(p, 8);
            let ad = self.period * 0.25 * self.speed - len * 0.5;
            self.accel_dist = ad;
            if len * 0.5 < ad {
                self.accel_dist = len * 0.5;
                self.speed = (self.accel_dist * 8.0) / self.period;
                self.accel_time = self.period * 0.25;
            } else if ad < 0.0 {
                self.accel_dist = 0.0;
                self.speed = (len * 2.0) / self.period;
                self.accel_time = 0.0;
            } else {
                self.accel_time = (ad * 2.0) / self.speed;
            }
        }
        let fr = &mut ctx.phys.frame;
        if self.flags & 4 == 0 {
            if self.flags & 8 != 0 {
                self.time = self.period * 0.5;
                fr.pos = [0.0; 3];
                fr.Translate(self.end, rwCOMBINEPOSTCONCAT);
            }
        } else {
            self.time = 0.0;
            fr.pos = [0.0; 3];
            fr.Translate(self.start, rwCOMBINEPOSTCONCAT);
        }
        if self.flags & 0x10 != 0 {
            let ph = &mut *ctx.phys;
            ph.velocity = [0.0; 3];
            ph.falling_timer = ph.falling_delay;
        }
    }
    fn GetParams(&self, _c: &DMCtx, p: &mut [u32]) {
        p[6] = fw(self.speed);
        put_v(p, 0, self.start);
        put_v(p, 3, self.end);
        p[7] = fw(self.accel_time);
        p[8] = fw(self.period);
        p[9] = self.flags;
    }
    fn PreColUpdate(&mut self, ctx: &mut DMCtx, dp: &mut EFDynamicPhysic, dt: f32) -> bool {
        self.SetFlags(ctx, dt);
        let mut dir = sub(self.end, self.start);
        let mut dist = RwV3dLength(dir);
        let half = self.period * 0.5;
        self.time += dt;
        let mut t;
        loop {
            t = self.time;
            if t <= self.period {
                break;
            }
            self.time -= self.period;
        }
        if half < t {
            t -= half;
        }
        let ta = self.accel_time;
        if t < ta {
            dist = (t * self.speed * 0.5 * t) / ta;
        } else if half - ta < t {
            dist -= ((half - t) * self.speed * 0.5 * (half - t)) / ta;
        } else {
            dist = self.accel_dist + self.speed * (t - ta);
        }
        dir = RwV3dNormalize(dir);
        let target = if self.time <= half {
            add(self.start, scale(dir, dist))
        } else {
            sub(self.end, scale(dir, dist))
        };
        let delta = sub(target, dp.pos);
        dp.disp = scale(delta, 1.0 / dt);
        let ph = &mut *ctx.phys;
        dp.accel = scale(sub(dp.disp, ph.velocity), 1.0 / dt);
        ph.velocity = dp.disp;
        ph.falling_timer = ph.falling_delay;
        true
    }
}

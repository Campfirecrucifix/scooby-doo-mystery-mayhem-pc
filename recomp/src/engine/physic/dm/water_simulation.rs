//! 0xc / 0xd EFDMWaterSimulation (Classes/EFDM/EFDMWaterSimulation.cpp, 0x001e6b50..0x001e8613,
//! 0xd4 bytes, type 0x1000000c / 0x1000000d), 1:1: a 2D wave-equation height field (three
//! W*H float buffers: +0xb4 current, +0xb8 previous, +0xbc per-cell damping) laid on the owner's
//! render grid, disturbed by up to 32 tracked game objects (MakeMeInteractWithCharacter).
//! Kind 0xc drives an EFWaterGrid_BC (3d object type 0x41), kind 0xd an EFGroundFogGrid_BC (0x56).
//!
//! Seams / replacements:
//! - EFWaterGrid_BC / EFGroundFogGrid_BC (the render grids, `EFGameObject::Get3dObject(obj, 0)`)
//!   are NOT ported: they are the module-local trait [`WaterRenderGrid`] (every method defaulted),
//!   attached by the host in `grid` (None = NULL 3d object).
//! - The simulation state lives in `Arc<RwLock<WaterSimState>>` ([`WaterSimRef`]) so the
//!   EFDMFloatingObject of another physic can read it (GetWaterValueAtThisPos) while modules are
//!   borrowed one at a time by the manager.
//! - MMMalloc'd buffers -> Vec<f32>; the 0x200-byte tracked-object table (+0xac) -> [WaterTracked; 32].
//! - EFWorld::GetGameObject + RwFrameGetLTM(obj+0x14) -> `PhysicWorld::GetObjectFrame(uid)`.
//! - EFInstance::Notify(obj, 0xc, {water uid, dy, dy_prev}) -> `PhysicNotify` code 0xc on `ctx.notify`.
//! - rand() -> the shared libc sequence (`SharedRand`, newlib), field `rng`.
//! - SetParams' object id list pointer (param +0x24 for 0xc, +0x14 for 0xd): the word slice carries
//!   the pointer word (non-zero = present) and the ids themselves follow the fixed block
//!   (`p[ParamWords()..]`).
#![allow(non_snake_case)]

use std::sync::{Arc, Mutex, RwLock};

use crate::engine::audio::mgr::SharedRand;
use crate::engine::audio::sbh::Rand;
use crate::engine::physic::module::*;
use crate::engine::physic::rwm::*;

/// EFWaterGrid_BC / EFGroundFogGrid_BC as seen by the simulation (render side not ported).
pub trait WaterRenderGrid: Send + Sync {
    /// vtbl+0x08 GetType: 0x41 EFWaterGrid_BC, 0x56 EFGroundFogGrid_BC
    fn GetType(&self) -> u32 { 0 }
    /// vtbl+0x74 (sim params this+0xc4: c4, c8, cc)
    fn SetSimParams(&mut self, _p: [f32; 3]) {}
    /// vtbl+0x78 (per-frame update)
    fn Update(&mut self) {}
    /// +0x118 (water) / +0x128 (fog): non-zero = skip the simulation this frame
    fn FrameIndex(&self) -> u32 { 0 }
    /// UpdateGridInfo then +0x90..+0xb0: origin, x axis (x = cell size), z axis (z = cell size)
    fn UpdateGridInfo(&mut self) -> [f32; 9] { [0.0; 9] }
    /// +0x10c / +0x11c [FrameIndex]: the simulation time step divisor
    fn StepScale(&self, _i: u32) -> f32 { 1.0 }
    /// +0x70 / +0x74 grid size (W, H)
    fn Dims(&self) -> (i32, i32) { (0, 0) }
    /// EFGroundFogGrid_BC::SetFogParams(param block)
    fn SetFogParams(&mut self, _p: &[u32]) {}
}

/// Default grid (no 3d object behaviour).
pub struct NoRenderGrid;
impl WaterRenderGrid for NoRenderGrid {}

/// One entry of the +0xac table (0x10 bytes).
#[derive(Debug, Clone, Copy, Default)]
pub struct WaterTracked {
    /// +0 game object uid
    pub uid: u32,
    /// +4 last position
    pub pos: V3,
}

/// The simulation fields (+0x34..+0xd0) and buffers.
#[derive(Debug, Clone)]
pub struct WaterSimState {
    /// +0x30 kind (0xc / 0xd)
    pub kind: u8,
    /// +0x34 damping (0.98) — fills +0xbc
    pub damping: f32,
    /// +0x38 wave speed (0.2 / 1.0)
    pub speed: f32,
    /// +0x3c impulse scale (1.0 / 0.4)
    pub impulse: f32,
    /// +0x44
    pub p44: f32,
    /// +0x48
    pub p48: f32,
    /// +0x4c random impulse amplitude
    pub rand_amp: f32,
    /// +0x50 tracked object count
    pub count: u32,
    /// +0x54 flags (param +0x34)
    pub flags: u32,
    /// +0x58..+0x68 fog params (5.0, 1.0, 0, 10.0, 0)
    pub fog: [f32; 5],
    /// +0x78 grid origin
    pub origin: V3,
    /// +0x84 grid x axis (x = cell size)
    pub axis_x: V3,
    /// +0x90 grid z axis (z = cell size)
    pub axis_z: V3,
    /// +0x9c W
    pub w: i32,
    /// +0xa0 H
    pub h: i32,
    /// +0xa4 W*H
    pub n: i32,
    /// +0xa8 initialised
    pub init: bool,
    /// +0xac tracked objects
    pub tracked: [WaterTracked; 32],
    /// +0xb0 flag bit0
    pub b0: bool,
    /// +0xb4 current heights (NULL = empty)
    pub cur: Vec<f32>,
    /// +0xb8 previous heights
    pub prev: Vec<f32>,
    /// +0xbc per-cell damping
    pub damp: Vec<f32>,
    /// +0xc0 max height (0.2)
    pub max_h: f32,
    /// +0xc4, +0xc8, +0xcc grid sim params (0.1 / 0.2, 0.5, 1.0)
    pub grid_params: [f32; 3],
    /// +0xd0 time step scale
    pub step: f32,
}

impl WaterSimState {
    fn new(kind: u8) -> Self {
        let mut s = WaterSimState {
            kind,
            damping: 0.0,
            speed: 0.0,
            impulse: 0.0,
            p44: 0.0,
            p48: 0.0,
            rand_amp: 0.0,
            count: 0,
            flags: 0,
            fog: [0.0; 5],
            origin: [0.0; 3],
            axis_x: [0.0; 3],
            axis_z: [0.0; 3],
            w: 0,
            h: 0,
            n: 0,
            init: false,
            tracked: [WaterTracked::default(); 32],
            b0: false,
            cur: Vec::new(),
            prev: Vec::new(),
            damp: Vec::new(),
            max_h: 0.0,
            grid_params: [0.0; 3],
            step: 0.0,
        };
        s.Defaults();
        s
    }
    /// the per-kind default block (ctor / SetParams(NULL))
    fn Defaults(&mut self) {
        if self.kind == 0xc {
            self.damping = 0.98;
            self.speed = 0.2;
            self.impulse = 1.0;
            self.p44 = 1.0;
            self.p48 = 1.0;
            self.rand_amp = 0.0;
            self.count = 0;
            self.flags = 0;
            self.max_h = 0.2;
            self.b0 = false;
            self.grid_params = [0.1, 0.5, 1.0];
        } else if self.kind == 0xd {
            self.fog = [5.0, 1.0, 0.0, 10.0, 0.0];
            self.damping = 0.98;
            self.speed = 1.0;
            self.impulse = 0.4;
            self.p44 = 1.0;
            self.p48 = 1.0;
            self.rand_amp = 0.0;
            self.count = 0;
            self.flags = 0;
            self.max_h = 0.2;
            self.b0 = false;
            self.grid_params = [0.2, 0.5, 1.0];
        }
    }

    /// EFDMWaterSimulation::ConvertWorldPosToGridCoord: 0 = inside (x, z set), 2 = outside.
    pub fn ConvertWorldPosToGridCoord(&self, p: V3, x: &mut i32, z: &mut i32) -> u32 {
        let d = sub(p, self.origin);
        let ix = (d[0] / self.axis_x[0] + 0.5) as i32;
        let iz = (d[2] / self.axis_z[2] + 0.5) as i32;
        if 3 < ix {
            if self.w - 4 <= ix {
                return 2;
            }
            if 0 < iz && iz < self.h - 1 {
                *x = ix;
                *z = iz;
                return 0;
            }
        }
        2
    }

    /// EFDMWaterSimulation::ComputeNormal(x, z): cross of the two cell diagonals over +0xb4.
    pub fn ComputeNormal(&self, x: i32, z: i32) -> V3 {
        let (zm, zp) = (z - 1, z + 1);
        let at = |xx: i32, zz: i32| self.cur[(xx + zz * self.w) as usize];
        let a = [(x - 1) as f32 * self.axis_x[0], at(x - 1, zm), zm as f32 * self.axis_z[2]];
        let b = [(x + 1) as f32 * self.axis_x[0], at(x + 1, zm), zm as f32 * self.axis_z[2]];
        let c = [(x - 1) as f32 * self.axis_x[0], at(x - 1, zp), zp as f32 * self.axis_z[2]];
        let d = [(x + 1) as f32 * self.axis_x[0], at(x + 1, zp), zp as f32 * self.axis_z[2]];
        let u = sub(a, d);
        let v = sub(b, c);
        RwV3dNormalize(cross(u, v))
    }

    /// EFDMWaterSimulation::GetWaterValueAtThisPos(pos, normal, height): 0 inside, 2 outside
    /// (height 0, normal +Y).
    pub fn GetWaterValueAtThisPos(&self, p: V3, normal: &mut V3, h: &mut f32) -> u32 {
        let (mut x, mut z) = (0, 0);
        if self.ConvertWorldPosToGridCoord(p, &mut x, &mut z) == 2 {
            *h = 0.0;
            *normal = [0.0, 1.0, 0.0];
            2
        } else {
            *h = self.cur[(x + z * self.w) as usize];
            *normal = self.ComputeNormal(x, z);
            0
        }
    }

    /// EFDMWaterSimulation::ReInitWater (dims already read from the grid)
    fn ReInitWater(&mut self, dims: (i32, i32)) {
        self.w = dims.0;
        self.h = dims.1;
        self.n = self.w * self.h;
        let n = self.n.max(0) as usize;
        self.cur = vec![0.0; n];
        self.prev = vec![0.0; n];
        self.damp = vec![0.0; n];
        let mut row = self.w;
        for _z in 1..self.h - 1 {
            for x in 1..self.w - 1 {
                self.damp[(row + x) as usize] = self.damping;
            }
            row += self.w;
        }
    }

    /// The wave-equation step of PreColUpdate (writes +0xb4 from +0xb8, then swaps them).
    fn Step(&mut self, dt: f32) {
        let w = self.w as isize;
        let h = self.h as isize;
        let mut k = (self.speed * dt) / self.step;
        k = k * 0.5 * k;
        let out = &mut self.cur;
        let src = &self.prev;
        let s = |j: isize| src[j as usize];
        let mut i = w + 1;
        let mut j = w + 1;
        let mut end = w * 2 - 1;
        // quirk: the damping is read once, from cell (1,1), for the whole grid
        let d = self.damp[(w + 1) as usize];
        while i < (h - 1) * w {
            let mut f15 = s(j + w);
            let mut f14 = s(j - w);
            while i < end {
                let f11 = s(j + 1 - w);
                let f12 = s(j + w + 1);
                // quirk: f14/f15 are "running" neighbour sums (not the plain up/down cells)
                out[i as usize] = (k * d * (f15 + f12 + f14 + f11 + s(j - 1) + s(j + 1))
                    + (2.0 - k * 8.0) * d * s(j))
                    - d * out[i as usize];
                f14 = (f14 + f11) - s(j - 1 - w);
                f15 = (f15 + f12) - s(j + w - 1);
                j += 1;
                i += 1;
            }
            end += w;
            j += 2;
            i += 2;
        }
        std::mem::swap(&mut self.cur, &mut self.prev);
    }
}

pub type WaterSimRef = Arc<RwLock<WaterSimState>>;

pub struct EFDMWaterSimulation {
    pub base: EFPhysicModule,
    /// +0x34..+0xd0
    pub sim: WaterSimRef,
    /// Get3dObject(obj, 0): the render grid (None = NULL)
    pub grid: Option<Box<dyn WaterRenderGrid>>,
    /// +0x70 water grid set (InitWater, kind 0xc)
    pub has_water_grid: bool,
    /// +0x74 fog grid set (SetParams, kind 0xd)
    pub has_fog_grid: bool,
    /// libc rand()
    pub rng: SharedRand,
}

impl EFDMWaterSimulation {
    pub fn new(kind: u32) -> Self {
        let k = kind as u8;
        EFDMWaterSimulation {
            base: EFPhysicModule { type_: k as u32 | 0x1000_0000, flags: 1, ..Default::default() },
            sim: Arc::new(RwLock::new(WaterSimState::new(k))),
            grid: None,
            has_water_grid: false,
            has_fog_grid: false,
            rng: Arc::new(Mutex::new(Rand::default())),
        }
    }

    fn kind(&self) -> u8 { self.sim.read().unwrap().kind }

    fn GridType(&self) -> Option<u32> { self.grid.as_ref().map(|g| g.GetType()) }

    /// EFDMWaterSimulation::InitWater
    pub fn InitWater(&mut self) {
        let mut s = self.sim.write().unwrap();
        if s.kind == 0xc {
            self.has_water_grid = self.grid.is_some();
            // NULL 3d object: the original crashes on the vtbl call; return (sim stays empty)
            let Some(g) = self.grid.as_mut() else { return };
            if g.GetType() != 0x41 {
                return;
            }
            g.SetSimParams(s.grid_params);
        } else if s.kind == 0xd {
            // quirk: +0x74 is only set by SetParams; unset = NULL vtbl call (crash) -> return
            if !self.has_fog_grid {
                return;
            }
            let Some(g) = self.grid.as_mut() else { return };
            g.SetSimParams(s.grid_params);
        }
        let dims = self.grid.as_ref().map_or((0, 0), |g| g.Dims());
        s.ReInitWater(dims);
    }

    /// EFDMWaterSimulation::MakeMeInteractWithCharacter(dt)
    pub fn MakeMeInteractWithCharacter(&mut self, ctx: &mut DMCtx, _dt: f32) {
        let mut s = self.sim.write().unwrap();
        let s = &mut *s;
        let me = ctx.phys.owner;
        let mut i = 0usize;
        while (i as u32) < s.count {
            // quirk: count > 32 (kind 0xd SetParams) reads past the 0x200 table in the original
            if i >= 32 {
                return;
            }
            let e = s.tracked[i];
            if let Some(ltm) = ctx.world.GetObjectFrame(e.uid) {
                let p = ltm.pos;
                let d = sub(p, s.origin);
                let ix = (d[0] / s.axis_x[0] + 0.5) as i32;
                let iz = (d[2] / s.axis_z[2] + 0.5) as i32;
                if 3 < ix && ix < s.w - 4 && 0 < iz && iz < s.h - 1 {
                    let mv = sub(p, e.pos);
                    let mut f12 = s.impulse * RwV3dLength(mv);
                    if f12 < 0.0001 {
                        let r = self.rng.lock().unwrap().rand() as u32;
                        f12 += (s.rand_amp * ((r & 0xff) as i32 - 0x80) as f32) / 128.0;
                    }
                    let mut f13 = s.max_h * 0.4;
                    if f12 <= f13 {
                        f13 = f12;
                    }
                    if mv[1] < 0.0 {
                        f13 = -f13;
                    }
                    let w = s.w;
                    let c = (ix + iz * w) as isize;
                    let buf = &mut s.prev;
                    let lvl = s.origin[1] + buf[c as usize];
                    let lo = if e.pos[1] <= p[1] { e.pos[1] } else { p[1] };
                    if lvl < lo {
                        // quirk: above the water the stored position is NOT updated
                        i += 1;
                        continue;
                    }
                    ctx.notify.push(PhysicNotify {
                        owner: e.uid,
                        code: 0xc,
                        data: [me, fw(p[1] - lvl), fw(e.pos[1] - lvl), 0],
                    });
                    let half = f13 * 0.5;
                    let q = f13 * 0.25;
                    let w = w as isize;
                    let mut add = |o: isize, v: f32| buf[(c + o) as usize] += v;
                    add(0, f13);
                    if 1 < ix {
                        add(-1, half);
                    }
                    if ix < s.w - 2 {
                        add(1, half);
                    }
                    if 1 < iz {
                        add(-w, half);
                    }
                    if iz < s.h - 2 {
                        add(w, half);
                    }
                    if 1 < iz && 1 < ix {
                        add(-1 - w, q);
                    }
                    if 1 < iz && ix < s.w - 2 {
                        add(1 - w, q);
                    }
                    if iz < s.h - 2 && 1 < ix {
                        add(w - 1, q);
                    }
                    if iz < s.h - 2 && ix < s.w - 2 {
                        add(w + 1, q);
                    }
                    for a in -1..2isize {
                        for b in -1..2isize {
                            let k = (c + a + b * w) as usize;
                            if s.max_h < buf[k] {
                                buf[k] = s.max_h;
                            }
                            if buf[k] < -s.max_h {
                                buf[k] = -s.max_h;
                            }
                        }
                    }
                }
                s.tracked[i].pos = p;
            }
            i += 1;
        }
    }

    /// The object-id list of SetParams into +0xac/+0x50 (`n` raw count word, ids follow the block).
    fn SetTracked(s: &mut WaterSimState, ctx: &DMCtx, n: u32, ids: &[u32]) {
        s.count = 0;
        let cnt = if n < 0x21 { n } else { 32 };
        for k in 0..cnt as usize {
            let uid = ids.get(k).copied().unwrap_or(0);
            if let Some(ltm) = ctx.world.GetObjectFrame(uid) {
                s.tracked[s.count as usize] = WaterTracked { uid, pos: ltm.pos };
                s.count += 1;
            }
        }
    }
}

impl EFDM for EFDMWaterSimulation {
    fn base(&self) -> &EFPhysicModule { &self.base }
    fn base_mut(&mut self) -> &mut EFPhysicModule { &mut self.base }
    fn as_water_sim(&self) -> Option<&EFDMWaterSimulation> { Some(self) }
    /// 0xc: 0x38 bytes; 0xd: 0x24 bytes
    fn ParamWords(&self) -> usize { if self.kind() == 0xc { 14 } else { 9 } }

    fn SetParams(&mut self, ctx: &mut DMCtx, p: Option<&[u32]>) {
        let pw = self.ParamWords();
        let mut fog_params = None;
        {
            let mut s = self.sim.write().unwrap();
            let s = &mut *s;
            s.init = false;
            let Some(p) = p else {
                s.Defaults();
                return;
            };
            if s.kind == 0xc {
                s.damping = wf(p, 0);
                s.speed = wf(p, 1);
                s.p44 = wf(p, 3);
                s.p48 = wf(p, 4);
                s.b0 = p[13] & 1 != 0;
                if p[9] != 0 {
                    s.count = 0;
                    if p[13] & 2 != 0 {
                        Self::SetTracked(s, ctx, p[8], p.get(pw..).unwrap_or(&[]));
                    }
                }
                s.rand_amp = wf(p, 5);
                s.impulse = wf(p, 2);
                s.max_h = wf(p, 7);
                s.grid_params = [wf(p, 10), wf(p, 11), wf(p, 12)];
                s.flags = p[13];
            } else if s.kind == 0xd {
                s.Defaults();
                s.fog = [wf(p, 0), wf(p, 1), wf(p, 2), wf(p, 3), s.fog[4]];
                s.count = p[4];
                if p[5] != 0 {
                    Self::SetTracked(s, ctx, p[4], p.get(pw..).unwrap_or(&[]));
                }
                s.grid_params = [wf(p, 6), wf(p, 7), wf(p, 8)];
                fog_params = Some(p);
            }
        }
        if let Some(p) = fog_params {
            self.has_fog_grid = self.grid.is_some();
            if self.GridType() == Some(0x56) {
                self.grid.as_mut().unwrap().SetFogParams(p);
            }
        }
    }

    fn GetParams(&self, _ctx: &DMCtx, p: &mut [u32]) {
        let s = self.sim.read().unwrap();
        if s.kind == 0xc {
            p[0] = fw(s.damping);
            p[1] = fw(s.speed);
            p[3] = fw(s.p44);
            p[4] = fw(s.p48);
            p[13] = s.b0 as u32;
            if s.count != 0 {
                p[13] |= 2;
            }
            // quirk: the flags computed above are overwritten
            p[13] = s.flags;
            p[5] = fw(s.rand_amp);
            p[2] = fw(s.impulse);
            p[9] = 0;
            p[8] = s.count;
            p[7] = fw(s.max_h);
            p[10] = fw(s.grid_params[0]);
            p[11] = fw(s.grid_params[1]);
            p[12] = fw(s.grid_params[2]);
        } else if s.kind == 0xd {
            p[0] = fw(s.fog[0]);
            p[1] = fw(s.fog[1]);
            p[2] = fw(s.fog[2]);
            p[3] = fw(s.fog[3]);
            p[5] = 0;
            p[4] = s.count;
            p[6] = fw(s.grid_params[0]);
            p[7] = fw(s.grid_params[1]);
            p[8] = fw(s.grid_params[2]);
        }
    }

    fn PreColUpdate(&mut self, ctx: &mut DMCtx, _dp: &mut EFDynamicPhysic, dt: f32) -> bool {
        if !self.sim.read().unwrap().init {
            self.InitWater();
            self.sim.write().unwrap().init = true;
        }
        let kind = self.kind();
        let (want, set) = match kind {
            0xc => (0x41, self.has_water_grid),
            0xd => (0x56, self.has_fog_grid),
            _ => (0, false),
        };
        if kind == 0xc || kind == 0xd {
            if set && self.GridType() != Some(want) {
                return true;
            }
            if self.sim.read().unwrap().cur.is_empty() {
                return true;
            }
            let Some(g) = self.grid.as_mut() else { return true };
            g.Update();
            if g.FrameIndex() != 0 {
                return true;
            }
            let info = g.UpdateGridInfo();
            let step = g.StepScale(g.FrameIndex());
            let mut s = self.sim.write().unwrap();
            s.origin = [info[0], info[1], info[2]];
            s.axis_x = [info[3], info[4], info[5]];
            s.axis_z = [info[6], info[7], info[8]];
            s.step = step;
        } else if self.sim.read().unwrap().cur.is_empty() {
            return true;
        }
        self.sim.write().unwrap().Step(dt);
        self.MakeMeInteractWithCharacter(ctx, dt);
        true
    }

    fn DoAction(&mut self, _ctx: &mut DMCtx, action: u32, _p: &mut [u32]) {
        // DoAction(1): reset the height field
        let mut s = self.sim.write().unwrap();
        if action == 1 && s.init {
            for i in 0..s.n.max(0) as usize {
                s.cur[i] = 0.0;
                s.prev[i] = 0.0;
            }
        }
    }
}

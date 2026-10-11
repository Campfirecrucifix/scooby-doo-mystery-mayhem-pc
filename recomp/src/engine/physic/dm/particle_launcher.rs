//! EFDMParticleLauncher (Classes/EFDM/EFDMParticleLauncher.cpp, DMAdd kind 0x1d, type 0x5000001d
//! = PRE|POST, 0xb0 bytes) and its base EFDMRayIntersect (Classes/EFDM/EFDMRayIntersect.cpp, kind
//! 0x1b, 0xa0 bytes). The base casts a ray (local to the owner frame) each PostColUpdate; on a hit
//! the launcher launches one particle of emitter +0xac at the hit point, oriented on the hit normal.
//!
//! Seams: owner frame LTM = `PhysicWorld::GetObjectFrame(owner)`; stage env polygons =
//! `StageContactPolys` + `StageContactPolyMaterials`; object / global rays =
//! `RayIntersectResult`; particle launch = `LaunchParticle`; `rand()` = `Rand`.
#![allow(non_snake_case)]

use crate::engine::physic::collision_map::RayIntersectIn;
use crate::engine::physic::mgr::EFRayIntersectIn;
use crate::engine::physic::module::*;
use crate::engine::physic::rwm::*;

/// EFDMRayIntersect (0xa0 bytes). Param block = +0x30..+0x9c (27 words).
#[derive(Debug, Clone, Default)]
pub struct EFDMRayIntersect {
    pub base: EFPhysicModule,
    /// +0x30 ray origin (owner-local; world during PostColUpdate)
    pub origin: V3,
    /// +0x3c ray delta (owner-local; world during PostColUpdate)
    pub dir: V3,
    /// +0x48 / +0x4c object prim masks (RayIntersectIn+0x18/+0x1c)
    pub obj_mask1: u32,
    pub obj_mask2: u32,
    /// +0x50 / +0x54 map material masks (RayIntersectIn+0x20/+0x24)
    pub map_mask1: u32,
    pub map_mask2: u32,
    /// +0x58 object uid to ignore (0 = none)
    pub exclude: u32,
    /// +0x5c result: 0 none, 1 object, 2 map
    pub result: u32,
    /// +0x60 hit position (owner-local after PostColUpdate)
    pub hit_pos: V3,
    /// +0x6c hit normal (owner-local after PostColUpdate)
    pub hit_normal: V3,
    /// +0x78 object hit: owner uid; map hit: shorts material+0x14/+0x16
    pub hit_id: u32,
    /// +0x7c..+0x9c object hit: [0] = prim id; map hit: material words 0..7
    pub hit_mat: [u32; 8],
    /// +0x9c byte: ray starts outside the swept sphere -> IntersectAll instead of stage polys
    pub use_all: bool,
}

impl EFDMRayIntersect {
    /// EFDMRayIntersect(uint type, EFPhysic*) (0x001e2be0): type | 0x50000000, flags 1.
    pub fn new(type_: u32) -> Self {
        let mut s = EFDMRayIntersect { base: EFPhysicModule { type_: type_ | 0x5000_0000, flags: 1, ..Default::default() }, ..Default::default() };
        s.use_all = false;
        s.InitParams();
        s
    }

    /// 0x001e2950: zero +0x30..+0x9c (+0x9c byte untouched).
    pub fn InitParams(&mut self) {
        let keep = (self.base.clone(), self.use_all);
        *self = EFDMRayIntersect { base: keep.0, use_all: keep.1, ..Default::default() };
    }

    /// 0x001e2d10
    pub fn SetParams(&mut self, p: Option<&[u32]>) {
        let Some(p) = p else {
            self.InitParams();
            return;
        };
        self.origin = wv(p, 0);
        self.dir = wv(p, 3);
        self.obj_mask1 = p[6];
        self.obj_mask2 = p[7];
        self.map_mask1 = p[8];
        self.map_mask2 = p[9];
        self.exclude = p[10];
        self.result = p[11];
        self.hit_pos = wv(p, 12);
        self.hit_normal = wv(p, 15);
        self.hit_id = p[18];
        self.hit_mat.copy_from_slice(&p[19..27]);
    }

    /// 0x001e2e30
    pub fn GetParams(&self, p: &mut [u32]) {
        put_v(p, 0, self.origin);
        put_v(p, 3, self.dir);
        p[6] = self.obj_mask1;
        p[7] = self.obj_mask2;
        p[8] = self.map_mask1;
        p[9] = self.map_mask2;
        p[10] = self.exclude;
        p[11] = self.result;
        put_v(p, 12, self.hit_pos);
        put_v(p, 15, self.hit_normal);
        p[18] = self.hit_id;
        p[19..27].copy_from_slice(&self.hit_mat);
    }

    fn rin(&self) -> EFRayIntersectIn {
        EFRayIntersectIn {
            map: RayIntersectIn { origin: self.origin, dir: self.dir, mask1: self.map_mask1, mask2: self.map_mask2 },
            obj_mask1: self.obj_mask1,
            obj_mask2: self.obj_mask2,
            exclude: self.exclude,
        }
    }

    /// 0x001e2f30: EFPhysicMgr::RayIntersect with the whole RayIntersectIn (+0x30).
    pub fn IntersectAll(&mut self, ctx: &mut DMCtx) -> bool {
        self.result = 0;
        let out = ctx.world.RayIntersectResult(&self.rin());
        let out = out.unwrap_or_default();
        self.result = out.kind as u32;
        if self.result != 0 {
            self.hit_pos = out.pos;
            self.hit_normal = out.normal;
        }
        if self.result == 1 {
            self.hit_id = ctx.world.PhysicOwnerUid(out.physic);
            // quirk-seam: +0x7c = prim+8 (prim id); EFRayResult only has the prim index.
            self.hit_mat[0] = out.prim as u32;
        } else if self.result == 2 {
            // not 1:1: map material words (*out+0x24) are not exposed by EFRayResult; left as is.
        }
        self.result != 0
    }

    /// 0x001e30d0: ray vs the collision stage's contact polygons (stage +4, count +8).
    pub fn IntersectEnvCollision(&mut self, ctx: &mut DMCtx, t: &mut f32) {
        let polys = ctx.world.StageContactPolys(ctx.phys.owner);
        let mats = ctx.world.StageContactPolyMaterials(ctx.phys.owner);
        for (i, (n, d, _verts, edges)) in polys.iter().enumerate() {
            let mat = mats.get(i).copied().unwrap_or((0, 0));
            if mat.0 & self.map_mask1 == 0 && mat.1 & self.map_mask2 == 0 {
                continue;
            }
            let den = n[2] * self.dir[2] + n[0] * self.dir[0] + n[1] * self.dir[1];
            if den == 0.0 {
                continue;
            }
            let tt = (n[2] * self.origin[2] + n[0] * self.origin[0] + n[1] * self.origin[1] + d) / -den;
            if !(0.0 <= tt && tt < *t) {
                continue;
            }
            let p = add(self.origin, scale(self.dir, tt));
            // inside all edge planes (d + p.n <= 0)
            let inside = edges.iter().all(|e| !(e[3] + p[2] * e[2] + p[0] * e[0] + p[1] * e[1] > 0.0));
            if inside {
                // not 1:1: only material words 1/2 are known (StageContactPolyMaterials); word 5
                // (+0x78 shorts) and the others stay 0.
                let m = [0, mat.0, mat.1, 0, 0, 0, 0, 0];
                self.hit_id = m[5];
                self.hit_normal = *n;
                self.hit_mat = m;
                self.hit_pos = p;
                self.result = 2;
                *t = tt;
            }
        }
    }

    /// 0x001e3420: ray vs every physic (both mgr lists, skipping flags 0x10020 and the +0x58 uid).
    pub fn IntersectObjCollision(&mut self, ctx: &mut DMCtx, t: &mut f32) {
        // not 1:1: the per-physic EFPhysic::RayIntersect loop is answered by the host through
        // RayIntersectResult with the map masks cleared (object part only); the shared `t` is
        // emulated by comparing the hit's ray parameter.
        let mut rin = self.rin();
        rin.map.mask1 = 0;
        rin.map.mask2 = 0;
        let Some(out) = ctx.world.RayIntersectResult(&rin) else { return };
        if out.kind != 1 {
            return;
        }
        let dd = dot(self.dir, self.dir);
        let te = if dd > 0.0 { dot(sub(out.pos, self.origin), self.dir) / dd } else { 0.0 };
        if *t < te {
            return;
        }
        *t = te;
        self.hit_id = ctx.world.PhysicOwnerUid(out.physic);
        self.result = 1;
        if self.result == 1 {
            self.hit_pos = out.pos;
            self.hit_normal = out.normal;
            self.hit_mat[0] = out.prim as u32;
        }
    }

    /// 0x001e35c0
    pub fn IntersectCollision(&mut self, ctx: &mut DMCtx) -> bool {
        let mut t = 1.0f32;
        self.result = 0;
        if self.map_mask1 != 0 || self.map_mask2 != 0 {
            self.IntersectEnvCollision(ctx, &mut t);
        }
        if self.obj_mask1 != 0 || self.obj_mask2 != 0 {
            self.IntersectObjCollision(ctx, &mut t);
        }
        self.result != 0
    }

    /// 0x001e38a0 (`_sfMaxAjust_1774` = 0.0025)
    pub fn CalcRadiusFor(&self, dp: &EFDynamicPhysic, p: V3) -> f32 {
        let v = sub(p, dp.sphere);
        let l = RwV3dLength(v);
        let f2 = l - (dp.radius * dp.radius).sqrt();
        if 0.0025 < f2 {
            -l
        } else if f2 < 0.0 {
            dp.radius
        } else {
            l
        }
    }

    /// 0x001e39a0
    pub fn PreColUpdate(&mut self, dp: &mut EFDynamicPhysic) -> bool {
        // quirk: the (owner-local) ray is compared with the world-space swept sphere.
        let mut f = self.CalcRadiusFor(dp, self.origin);
        if 0.0 < f {
            f = self.CalcRadiusFor(dp, add(self.origin, self.dir));
        }
        self.use_all = f <= 0.0;
        if dp.radius < f {
            dp.radius = f;
        }
        true
    }

    /// 0x001e3640
    pub fn PostColUpdate(&mut self, ctx: &mut DMCtx, stage: Option<&EFCollisionStage>) {
        if stage.is_none() {
            return;
        }
        let (so, sd) = (self.origin, self.dir);
        let ltm = ctx.world.GetObjectFrame(ctx.phys.owner).unwrap_or_default();
        self.origin = ltm.TransformPoint(self.origin);
        self.dir = ltm.TransformVector(self.dir);
        let hit = if !self.use_all { self.IntersectCollision(ctx) } else { self.IntersectAll(ctx) };
        if !hit {
            // quirk: the notify carries a 4th word (changed bits) that PhysicNotify drops.
            self.base.ResetStateBits(1, ctx);
        } else {
            self.base.SetStateBits(1, ctx);
            let inv = ltm.Invert();
            self.hit_pos = inv.TransformPoint(self.hit_pos);
            self.hit_normal = inv.TransformVector(self.hit_normal);
        }
        self.origin = so;
        self.dir = sd;
    }
}

/// EFDMParticleLauncher (0xb0 bytes). Param block = EFDMRayIntersect's 27 words + 4.
pub struct EFDMParticleLauncher {
    pub ray: EFDMRayIntersect,
    /// +0xa0 launch "up" direction (owner-local, normalized by SetParams; ctor (0, 1, 0))
    pub up: V3,
    /// +0xac emitter id (PLMgr_BC::GetEmitter)
    pub emitter: i32,
}

impl Default for EFDMParticleLauncher {
    fn default() -> Self { Self::new() }
}

impl EFDMParticleLauncher {
    /// DMAdd 0x1d: EFDMRayIntersect(0x4000001d, physic); +0xa0 = 0, +0xa4 = 1.0, +0xa8 = 0, +0xac = 0.
    pub fn new() -> Self {
        EFDMParticleLauncher { ray: EFDMRayIntersect::new(0x4000_001d), up: [0.0, 1.0, 0.0], emitter: 0 }
    }
}

impl EFDM for EFDMParticleLauncher {
    fn base(&self) -> &EFPhysicModule { &self.ray.base }
    fn base_mut(&mut self) -> &mut EFPhysicModule { &mut self.ray.base }
    fn ParamWords(&self) -> usize { 31 }

    /// inherited EFDMRayIntersect::PreColUpdate (0x001e39a0)
    fn PreColUpdate(&mut self, _ctx: &mut DMCtx, dp: &mut EFDynamicPhysic, _dt: f32) -> bool { self.ray.PreColUpdate(dp) }

    /// 0x0021cb50
    fn SetParams(&mut self, _ctx: &mut DMCtx, p: Option<&[u32]>) {
        match p {
            None => {
                self.ray.SetParams(None);
                self.emitter = 0;
                self.up = [0.0, 1.0, 0.0];
            }
            Some(p) => {
                self.ray.SetParams(Some(p));
                self.emitter = p[30] as i32;
                self.up = wv(p, 27);
                self.up = RwV3dNormalize(self.up);
            }
        }
    }

    /// 0x0021cbe0
    fn GetParams(&self, _ctx: &DMCtx, p: &mut [u32]) {
        self.ray.GetParams(p);
        p[30] = self.emitter as u32;
        put_v(p, 27, self.up);
    }

    /// 0x0021cc30
    fn PostColUpdate(&mut self, ctx: &mut DMCtx, stage: Option<&EFCollisionStage>, _dt: f32) {
        if stage.is_none() {
            return;
        }
        self.ray.PostColUpdate(ctx, stage);
        if self.ray.result == 0 {
            return;
        }
        let ltm = ctx.world.GetObjectFrame(ctx.phys.owner).unwrap_or_default();
        let (hit_pos, hit_normal, upl) = (self.ray.hit_pos, self.ray.hit_normal, self.up);
        let world = ctx.world;
        world.LaunchParticle(self.emitter, &mut |m: &mut RwMatrix| {
            let p = ltm.TransformPoint(hit_pos);
            let n = ltm.TransformVector(hit_normal);
            let d = ltm.TransformVector(upl);
            m.pos = add(p, scale(n, 0.001));
            m.at = n;
            let k = d[2] * n[2] + d[0] * n[0] + d[1] * n[1];
            let mut up = sub(d, scale(n, k));
            let l2 = up[2] * up[2] + up[0] * up[0] + up[1] * up[1];
            let mut degenerate = false;
            if 0.0001 < l2 {
                up = scale(up, 1.0 / l2.sqrt());
            } else {
                degenerate = true;
                // quirk: builds a fallback axis (unit axis of the smallest of n.x/n.y/n.z minus
                // n * that component, normalized) but never uses it; `up` stays the tiny vector.
                let mut mn = n[1];
                if n[1] >= n[0] {
                    mn = n[0];
                }
                let mut i = if n[1] < n[0] { 1 } else { 0 };
                if n[2] < mn {
                    i = 2;
                }
                let mut a = [0.0f32; 3];
                a[i] = 1.0;
                let _unused = RwV3dNormalize(sub(a, scale(n, mn)));
            }
            let right = [m.at[1] * up[2] - m.at[2] * up[1], m.at[2] * up[0] - m.at[0] * up[2], m.at[0] * up[1] - m.at[1] * up[0]];
            m.right = right;
            m.up = up;
            if degenerate {
                let axis = m.at;
                let r = world.Rand();
                let pos = m.pos;
                m.Rotate((((r & 0xff) as f32 * 2.0 * 3.1415927) / 256.0) * 57.29578, axis, rwCOMBINEPOSTCONCAT);
                m.pos = pos;
            }
        });
    }
}

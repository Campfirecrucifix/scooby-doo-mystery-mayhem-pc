//! 0x1b EFDMRayIntersect (Classes/EFDM/EFDMRayIntersect.cpp, 0xa0 bytes), 1:1 logic.
//!
//! A ray (frame-local origin +0x30 / delta +0x3c) cast every PostColUpdate, either against the
//! collision stage's env prims + the manager's physics (IntersectCollision) or, when the
//! PreColUpdate found the ray leaves the physic's swept sphere, through EFPhysicMgr::RayIntersect
//! (IntersectAll). State bit 0 = "ray hits"; the hit (frame-local) is read back by GetParams.
//!
//! Seams (module-local, see `RayIntersectHost`): the C++ reaches `_gpoPhysicMgr` (RayIntersect,
//! physic lists +0x200c/+0x2018, EFPhysic::RayIntersect, map prim materials). During module
//! updates the manager is borrowed, so the EFDM trait path uses `WorldRayHost` (PhysicWorld's
//! RayIntersectResult / PhysicOwnerUid; no object list => IntersectObjCollision finds nothing)
//! and the stage env prims from `PhysicWorld::StageContactPolys` (+ material words 1/2 from
//! StageContactPolyMaterials). A coordinator with the real data calls `PostColUpdateWith`
//! (e.g. with `MgrRayHost` over the real EFPhysicMgr + collision system and `EnvPrim`s of the
//! real EFCollisionStage).
#![allow(non_snake_case)]

use crate::engine::physic::collision_map::RayIntersectIn;
use crate::engine::physic::mgr::{CollisionSystem, EFPhysicMgr, EFRayIntersectIn, EFRayResult, PhysicRayIntersect};
use crate::engine::physic::module::*;
use crate::engine::physic::objcoll_check::EnvPrim;
use crate::engine::physic::physic::PhysId;
use crate::engine::physic::rwm::*;

/// What EFDMRayIntersect reads through `_gpoPhysicMgr`.
pub trait RayIntersectHost {
    /// EFPhysicMgr::RayIntersect(_gpoPhysicMgr, in, out) (nearest hit; kind 0 none/1 obj/2 map).
    fn MgrRayIntersect(&mut self, _rin: &EFRayIntersectIn) -> EFRayResult { EFRayResult::default() }
    /// `*(*(physic + 0xac) + 8)`: owner game-object uid of a physic.
    fn PhysicOwner(&self, _id: PhysId) -> u32 { 0 }
    /// EFRayIntersectOut+0x20 prim -> prim+8 tag (prim index `prim` of physic `id`).
    fn PrimTag(&self, _id: PhysId, _prim: usize) -> u32 { 0 }
    /// EFRayIntersectOut+0x24 map prim -> `*prim` material record (8 words).
    fn MapMaterial(&self, _map_prim: (usize, usize)) -> [u32; 8] { [0; 8] }
    /// _gpoPhysicMgr+0x200c (l=0) / +0x2018 (l=1) physic lists.
    fn PhysicList(&self, _l: usize) -> Vec<PhysId> { Vec::new() }
    /// EFPhysic+0xc flags
    fn PhysicFlags(&self, _id: PhysId) -> u32 { 0 }
    /// EFPhysic::RayIntersect(in, out, &t) != 0: Some((out+4 pos, out+0x10 normal, out+0x20 prim tag)).
    fn PhysicRayIntersect(&mut self, _id: PhysId, _rin: &EFRayIntersectIn, _t: &mut f32) -> Option<(V3, V3, u32)> { None }
}

/// The real manager as host (outside module updates / tests).
pub struct MgrRayHost<'a> {
    pub mgr: &'a mut EFPhysicMgr,
    pub cs: &'a mut dyn CollisionSystem,
}

impl RayIntersectHost for MgrRayHost<'_> {
    fn MgrRayIntersect(&mut self, rin: &EFRayIntersectIn) -> EFRayResult { self.mgr.RayIntersect(rin, self.cs) }
    fn PhysicOwner(&self, id: PhysId) -> u32 { self.mgr.Physic(id).map_or(0, |p| p.owner) }
    fn PrimTag(&self, id: PhysId, prim: usize) -> u32 {
        self.mgr.Physic(id).and_then(|p| p.model.prims.get(prim)?.as_ref().map(|q| q.base.tag)).unwrap_or(0)
    }
    fn MapMaterial(&self, (m, p): (usize, usize)) -> [u32; 8] {
        let mut r = [0u32; 8];
        if let Some(map) = self.mgr.maps.get(m) {
            let mat = map.PrimMaterial(p);
            for (k, w) in r.iter_mut().enumerate() {
                *w = map.w.get(mat + k).copied().unwrap_or(0);
            }
        }
        r
    }
    fn PhysicList(&self, l: usize) -> Vec<PhysId> { self.mgr.lists[l].clone() }
    fn PhysicFlags(&self, id: PhysId) -> u32 { self.mgr.Physic(id).map_or(0, |p| p.flags) }
    fn PhysicRayIntersect(&mut self, id: PhysId, rin: &EFRayIntersectIn, t: &mut f32) -> Option<(V3, V3, u32)> {
        let p = self.mgr.Physic(id)?;
        let mut n = [0.0; 3];
        let prim = PhysicRayIntersect(p, rin, t, &mut n, self.cs)?;
        // out+4: hit position origin + delta * t (as EFPhysicMgr::RayIntersect computes it)
        let pos = add(rin.map.origin, scale(rin.map.dir, *t));
        Some((pos, n, p.model.prims[prim].as_ref().map_or(0, |q| q.base.tag)))
    }
}

/// PhysicWorld as host (EFDM trait path).
pub struct WorldRayHost<'a>(pub &'a dyn PhysicWorld);
impl RayIntersectHost for WorldRayHost<'_> {
    fn MgrRayIntersect(&mut self, rin: &EFRayIntersectIn) -> EFRayResult { self.0.RayIntersectResult(rin).unwrap_or_default() }
    fn PhysicOwner(&self, id: PhysId) -> u32 { self.0.PhysicOwnerUid(id) }
}

/// The stage env prims via PhysicWorld (contact polys + material words 1/2; others 0).
pub fn WorldStageEnvPrims(w: &dyn PhysicWorld, owner: u32) -> Vec<EnvPrim> {
    let mats = w.StageContactPolyMaterials(owner);
    w.StageContactPolys(owner)
        .into_iter()
        .enumerate()
        .map(|(i, (normal, d, verts, edges))| {
            let (m1, m2) = mats.get(i).copied().unwrap_or((0, 0));
            EnvPrim { id: i, mat: [0, m1, m2, 0, 0, 0, 0, 0], w4: 0, nverts: verts.len() as u16, normal, d, verts, edges }
        })
        .collect()
}

pub struct EFDMRayIntersect {
    pub base: EFPhysicModule,
    /// +0x30 ray origin (frame-local; world during PostColUpdate)
    pub origin: V3,
    /// +0x3c ray delta (t in [0,1])
    pub dir: V3,
    /// +0x48 / +0x4c object prim masks
    pub obj_mask1: u32,
    pub obj_mask2: u32,
    /// +0x50 / +0x54 env material masks (material word 1 / 2)
    pub map_mask1: u32,
    pub map_mask2: u32,
    /// +0x58 object uid to ignore (0 = none)
    pub exclude: u32,
    /// +0x5c result: 0 none, 1 object, 2 env/map
    pub result: u32,
    /// +0x60 hit position (frame-local after PostColUpdate)
    pub pos: V3,
    /// +0x6c hit normal
    pub normal: V3,
    /// +0x78..+0x98 hit info: kind 1 [0] owner uid, [1] prim tag; kind 2 [0] = material u16 pair
    /// (+0x14/+0x16 = word 5), [1..9] the material record (8 words)
    pub info: [u32; 9],
    /// +0x9c PreColUpdate: ray leaves the swept sphere -> use IntersectAll
    pub use_all: bool,
}

impl Default for EFDMRayIntersect {
    fn default() -> Self { Self::new() }
}

impl EFDMRayIntersect {
    /// EFDMRayIntersect(EFPhysic*): type 0x5000001b (Pre + Post).
    pub fn new() -> Self {
        let mut s = EFDMRayIntersect { base: EFPhysicModule { type_: 0x5000_001b, flags: 1, ..Default::default() }, ..Self::zeroed() };
        s.InitParams();
        s
    }
    /// 0x001e2950: zero +0x30..+0x9b (not +0x9c)
    pub fn InitParams(&mut self) {
        let u = self.use_all;
        let b = std::mem::take(&mut self.base);
        *self = EFDMRayIntersect { base: b, use_all: u, ..Self::zeroed() };
    }
    fn zeroed() -> Self {
        EFDMRayIntersect {
            base: EFPhysicModule::default(),
            origin: [0.0; 3],
            dir: [0.0; 3],
            obj_mask1: 0,
            obj_mask2: 0,
            map_mask1: 0,
            map_mask2: 0,
            exclude: 0,
            result: 0,
            pos: [0.0; 3],
            normal: [0.0; 3],
            info: [0; 9],
            use_all: false,
        }
    }
    fn rin(&self) -> EFRayIntersectIn {
        EFRayIntersectIn {
            map: RayIntersectIn { origin: self.origin, dir: self.dir, mask1: self.map_mask1, mask2: self.map_mask2 },
            obj_mask1: self.obj_mask1,
            obj_mask2: self.obj_mask2,
            exclude: self.exclude,
        }
    }

    /// 0x001e2f30
    pub fn IntersectAll(&mut self, host: &mut dyn RayIntersectHost) -> bool {
        self.result = 0;
        let out = host.MgrRayIntersect(&self.rin());
        self.result = out.kind as u32;
        if self.result != 0 {
            self.pos = out.pos;
            self.normal = out.normal;
        }
        if self.result == 1 {
            self.info[0] = host.PhysicOwner(out.physic);
            self.info[1] = host.PrimTag(out.physic, out.prim);
        } else if self.result == 2 {
            // C++ derefs out+0x24 unconditionally (a kind-2 hit always has a prim)
            let mat = out.map_prim.map_or([0; 8], |mp| host.MapMaterial(mp));
            self.info[0] = mat[5];
            self.info[1..9].copy_from_slice(&mat);
        }
        self.result != 0
    }

    /// 0x001e30d0: nearest env polygon of the stage hit by the ray (t in [0, *t)).
    pub fn IntersectEnvCollision(&mut self, env: &[EnvPrim], t: &mut f32) {
        let (o, d) = (self.origin, self.dir);
        for e in env {
            if e.mat[1] & self.map_mask1 == 0 && e.mat[2] & self.map_mask2 == 0 {
                continue;
            }
            let n = e.normal;
            let den = n[2] * d[2] + n[0] * d[0] + n[1] * d[1];
            if den == 0.0 {
                continue;
            }
            let tt = (n[2] * o[2] + n[0] * o[0] + n[1] * o[1] + e.d) / -den;
            if !(0.0 <= tt && tt < *t) {
                continue;
            }
            let p = add(o, scale(d, tt));
            // inside all edge planes (raw u16 count; planes after `& 0x7fff` vertices)
            let mut inside = true;
            for i in 0..e.nverts as usize {
                let Some(ep) = e.edges.get(i) else { break };
                if ep[3] + p[2] * ep[2] + p[0] * ep[0] + p[1] * ep[1] > 0.0 {
                    inside = false;
                    break;
                }
            }
            if inside {
                self.info[0] = e.mat[5];
                self.normal = n;
                self.info[1..9].copy_from_slice(&e.mat);
                self.pos = p;
                self.result = 2;
                *t = tt;
            }
        }
    }

    /// 0x001e3420: every physic of the manager lists (not deleted/disabled 0x10020, not excluded).
    pub fn IntersectObjCollision(&mut self, host: &mut dyn RayIntersectHost, t: &mut f32) {
        let rin = self.rin();
        let excl = self.exclude;
        let mut out: Option<(V3, V3, u32)> = None;
        for l in 0..2 {
            for id in host.PhysicList(l) {
                if host.PhysicFlags(id) & 0x10020 != 0 {
                    continue;
                }
                let owner = host.PhysicOwner(id);
                if excl != 0 && self.exclude == owner {
                    continue;
                }
                if let Some(o) = host.PhysicRayIntersect(id, &rin, t) {
                    out = Some(o);
                    self.info[0] = owner;
                    self.result = 1;
                }
            }
        }
        if self.result == 1 {
            let (p, n, tag) = out.unwrap_or_default();
            self.pos = p;
            self.normal = n;
            self.info[1] = tag;
        }
    }

    /// 0x001e35c0
    pub fn IntersectCollision(&mut self, env: &[EnvPrim], host: &mut dyn RayIntersectHost) -> bool {
        let mut t = 1.0f32;
        self.result = 0;
        if self.map_mask1 != 0 || self.map_mask2 != 0 {
            self.IntersectEnvCollision(env, &mut t);
        }
        if self.obj_mask1 != 0 || self.obj_mask2 != 0 {
            self.IntersectObjCollision(host, &mut t);
        }
        self.result != 0
    }

    /// 0x001e3640 with explicit stage env prims / host. `stage` = EFCollisionStage* != NULL.
    pub fn PostColUpdateWith(&mut self, ctx: &mut DMCtx, stage: bool, env: &[EnvPrim], host: &mut dyn RayIntersectHost) {
        if !stage {
            return;
        }
        let (so, sd) = (self.origin, self.dir);
        let ltm = ctx.phys.frame;
        self.origin = ltm.TransformPoint(self.origin);
        self.dir = ltm.TransformVector(self.dir);
        let hit = if !self.use_all { self.IntersectCollision(env, host) } else { self.IntersectAll(host) };
        if !hit {
            self.base.ResetStateBits(1, ctx);
        } else {
            self.base.SetStateBits(1, ctx);
            let inv = ltm.Invert();
            self.pos = inv.TransformPoint(self.pos);
            self.normal = inv.TransformVector(self.normal);
        }
        self.origin = so;
        self.dir = sd;
    }

    /// 0x001e38a0. static _sfMaxAjust = 0.0025.
    pub fn CalcRadiusFor(&self, dp: &EFDynamicPhysic, p: V3) -> f32 {
        let v = sub(p, dp.sphere);
        let mut l = RwV3dLength(v);
        let f = l - (dp.radius * dp.radius).sqrt();
        if 0.0025 < f {
            l = -l;
        } else if f < 0.0 {
            l = dp.radius;
        }
        l
    }
}

impl EFDM for EFDMRayIntersect {
    fn base(&self) -> &EFPhysicModule { &self.base }
    fn base_mut(&mut self) -> &mut EFPhysicModule { &mut self.base }
    /// 0x6c bytes
    fn ParamWords(&self) -> usize { 27 }
    /// 0x001e39a0
    fn PreColUpdate(&mut self, _ctx: &mut DMCtx, dp: &mut EFDynamicPhysic, _dt: f32) -> bool {
        let mut r = self.CalcRadiusFor(dp, self.origin);
        if 0.0 < r {
            r = self.CalcRadiusFor(dp, add(self.origin, self.dir));
        }
        self.use_all = r <= 0.0;
        if dp.radius < r {
            dp.radius = r;
        }
        true
    }
    fn PostColUpdate(&mut self, ctx: &mut DMCtx, stage: Option<&EFCollisionStage>, _dt: f32) {
        // the real stage (collision system): its env prims + the manager (WP-J4)
        if let Some(st) = stage {
            if let Some(h) = st.ray {
                return self.PostColUpdateWith(ctx, true, st.env, &mut *h.borrow_mut());
            }
        }
        // seam stage: PhysicWorld contact polys
        let world = ctx.world;
        let env = if stage.is_some() { WorldStageEnvPrims(world, ctx.phys.owner) } else { Vec::new() };
        self.PostColUpdateWith(ctx, stage.is_some(), &env, &mut WorldRayHost(world));
    }
    /// 0x001e2d10
    fn SetParams(&mut self, _ctx: &mut DMCtx, p: Option<&[u32]>) {
        let Some(p) = p else { return self.InitParams() };
        self.origin = wv(p, 0);
        self.dir = wv(p, 3);
        self.obj_mask1 = p[6];
        self.obj_mask2 = p[7];
        self.map_mask1 = p[8];
        self.map_mask2 = p[9];
        self.exclude = p[10];
        self.result = p[11];
        self.pos = wv(p, 12);
        self.normal = wv(p, 15);
        self.info.copy_from_slice(&p[18..27]);
    }
    /// 0x001e2e30
    fn GetParams(&self, _ctx: &DMCtx, p: &mut [u32]) {
        put_v(p, 0, self.origin);
        put_v(p, 3, self.dir);
        p[6] = self.obj_mask1;
        p[7] = self.obj_mask2;
        p[8] = self.map_mask1;
        p[9] = self.map_mask2;
        p[10] = self.exclude;
        p[11] = self.result;
        put_v(p, 12, self.pos);
        put_v(p, 15, self.normal);
        p[18..27].copy_from_slice(&self.info);
    }
}

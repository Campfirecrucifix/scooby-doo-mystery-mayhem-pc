//! EFPhysicMgr (Classes/EF/EFPhysicMgr.cpp), 1:1 except where noted.
//!
//! Layout of the original (0x2098 bytes) and where each part lives here:
//! - +0x0000..0x2000 DM handle table: 0x400 x {u32 serial, EFPhysicModule*} -> `slots` (the module
//!   objects themselves are owned by the slot: `Box<dyn EFDM>`). +0x2000 last allocated slot,
//!   +0x2004 module count.
//! - +0x2008 EFCollisionMap list (AddMap pushes at the head) -> `maps` (index 0 = head).
//! - +0x200c / +0x2018: two physic lists (CreatePhysic(.., list)) -> `lists[0..2]` (head first);
//!   the EFPhysic objects live in `physics` (PhysId = index + 1).
//! - +0x2024 frame counter, +0x2028 paused frame count, +0x202c previous paused flag,
//!   +0x2030 paused flag (from _gpoWorldMgr+0x14).
//! - +0x2034/+0x2054/+0x2074 DTQuickPool<EFCollisionReport/EFPushReport/EFContactReport> carved
//!   out of DTWorkMemory each frame (limits 0x80, grow 200/0x32): owned by WP-E3's report.rs; the
//!   per-frame reset is the `CollisionSystem::BeginFrame/EndFrame` seam.
//! - the global query stamp `iRam00343cc0` (incremented by every RayIntersect /
//!   GetPrimitivesInSphere; collision-map primitives remember the last stamp that saw them) is the
//!   field `query_stamp`.
#![allow(non_snake_case)]

use std::collections::HashMap;

use super::collision_map::{EFCollisionMap, EFGetPrimitivesInSphereData, EFRayIntersectOut, RayIntersectIn};
use super::module::*;
use super::physic::*;
use super::rwm::*;

pub const MAX_MODULES: usize = 0x400;

/// RayIntersectIn as seen by EFPhysicMgr::RayIntersect (the map part is collision_map's struct).
#[derive(Debug, Clone, Copy, Default)]
pub struct EFRayIntersectIn {
    /// +0x00 origin, +0x0c delta, +0x20/+0x24 map material masks
    pub map: RayIntersectIn,
    /// +0x18 / +0x1c object prim masks
    pub obj_mask1: u32,
    pub obj_mask2: u32,
    /// +0x28 object uid to ignore (0 = none; 0x2a000000 = SELF, resolved by the API)
    pub exclude: u32,
}

/// Result of EFPhysicMgr::RayIntersect.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct EFRayResult {
    /// 0 none, 1 object hit, 2 map hit, 3 callback mode
    pub kind: u8,
    /// +0x04
    pub pos: V3,
    /// +0x10
    pub normal: V3,
    /// +0x1c physic hit (kind 1)
    pub physic: PhysId,
    /// +0x20 prim index of that physic (kind 1)
    pub prim: usize,
    /// +0x24 map primitive (kind 2): (map index, prim word index)
    pub map_prim: Option<(usize, usize)>,
}

/// EFUpdatePhysicStruct: +0 dt, then 2 passes x {EFDynamicPhysic* first, count}, +0x14 emitter
/// array, +0x18 emitter count (PLParticleMgr walk: unported, always empty).
#[derive(Debug, Clone, Default)]
pub struct EFUpdatePhysicStruct {
    pub dt: f32,
    pub passes: [Vec<EFDynamicPhysic>; 2],
    pub emitter_count: u32,
}

/// Seam for WP-E3's EFCollisionSystem (+ report pools) and object-prim ray tests.
pub trait CollisionSystem {
    /// start of EFPhysicMgr::Update: (re)carve the report pools
    fn BeginFrame(&mut self) {}
    /// end of Update: deallocate the pools
    fn EndFrame(&mut self) {}
    /// EFCollisionSystem ctor; Init(ups, pass, index, contacts); Update; PostUpdate; Terminate.
    /// Must end by calling `EFPhysicMgr::PostCollisionUpdate` for the solved physics (as the
    /// original EFCollisionStage does) and set `flags |= 2` on the dyn entries it consumed.
    fn Solve(&mut self, _mgr: &mut EFPhysicMgr, _ups: &mut EFUpdatePhysicStruct, _pass: usize, _index: usize, _world: &dyn PhysicWorld) {}
    /// prim vtbl+0x24 RayIntersect(in, out, t) of EFObjCollisionPrim `prim` of `phys`
    fn PrimRayIntersect(&mut self, _phys: &EFPhysic, _prim: usize, _rin: &EFRayIntersectIn, _t: &mut f32, _normal: &mut V3) -> bool { false }
    /// EFObjCollisionPrim::CheckSphereSphereCollision(ray as swept point vs primary sphere):
    /// returns (hit flags, entry t). Default: analytic segment/sphere test.
    fn CheckSphereSphere(&mut self, origin_rel: V3, delta: V3, radius: f32) -> (u32, f32) {
        segment_sphere(origin_rel, delta, radius)
    }
}

/// Stateless EFObjCollisionPrim queries (RayIntersect / CheckSphereSphereCollision) as a
/// CollisionSystem, for manager ray queries made from inside a collision update (WP-J4).
pub struct PrimQuery;
impl CollisionSystem for PrimQuery {
    fn PrimRayIntersect(&mut self, phys: &EFPhysic, prim: usize, rin: &EFRayIntersectIn, t: &mut f32, normal: &mut V3) -> bool {
        let Some(Some(p)) = phys.model.prims.get(prim) else { return false };
        let mut out = super::collision_map::EFRayIntersectOut::default();
        let r = p.RayIntersect(&rin.map, &mut out, t);
        if r != 0 {
            *normal = out.hit.normal;
        }
        r != 0
    }
    fn CheckSphereSphere(&mut self, origin_rel: V3, delta: V3, radius: f32) -> (u32, f32) {
        use super::objcoll_prim::*;
        let i = EFCheckSphereSphereCollisionIn { center: origin_rel, motion: delta, radius };
        let mut o = EFCheckSphereSphereCollisionOut::default();
        let r = EFObjCollisionPrim::CheckSphereSphereCollision(&i, &mut o);
        (r, o.t)
    }
}

/// Default for CheckSphereSphereCollision with a zero-radius moving sphere: bit0 = starts inside,
/// bit3 = enters during the move. NOT 1:1 (EFObjCollisionPrim belongs to WP-E3).
pub fn segment_sphere(o: V3, d: V3, r: f32) -> (u32, f32) {
    let c = dot(o, o) - r * r;
    if c <= 0.0 {
        return (1, 0.0);
    }
    let a = dot(d, d);
    let b = dot(o, d);
    if a <= 0.0 || b >= 0.0 {
        return (0, 0.0);
    }
    let disc = b * b - a * c;
    if disc < 0.0 {
        return (0, 0.0);
    }
    let t = (-b - disc.sqrt()) / a;
    if t <= 1.0 { (8, t) } else { (0, t) }
}

/// No collision system (WP-E3 not wired): COLLIDE physics are not solved.
pub struct NoCollision;
impl CollisionSystem for NoCollision {}

pub struct EFPhysicMgr {
    /// +0x0000 handle table {serial, module}
    pub slots: Vec<(u32, Option<Box<dyn EFDM>>)>,
    /// +0x2000
    pub last_slot: u32,
    /// +0x2004
    pub module_count: u32,
    /// +0x2008
    pub maps: Vec<EFCollisionMap>,
    pub physics: Vec<Option<EFPhysic>>,
    /// +0x200c, +0x2018
    pub lists: [Vec<PhysId>; 2],
    /// +0x2024
    pub frame: u32,
    /// +0x2028
    pub paused_frames: u32,
    /// +0x202c
    pub prev_paused: u32,
    /// +0x2030
    pub paused: u32,
    /// iRam00343cc0
    pub query_stamp: i32,
    /// EFInstance::Notify calls produced by the physics, drained by the coordinator
    pub notify: Vec<PhysicNotify>,
    /// WP-E10: EFInstance::Notify(0x17 then 3) of EFCollisionReport::CheckNotify, as
    /// (instance uid, notify id, msg) queued by EFCollisionSystem::Update; drained by the coordinator.
    pub col_notify: Vec<(u32, u32, crate::engine::physic::report::CollisionNotifyMsg)>,
    /// game object uid -> EFPhysicData+0x28 (EFWorld::GetGameObject(uid)+0x28)
    pub by_owner: HashMap<u32, PhysId>,
}

impl Default for EFPhysicMgr {
    fn default() -> Self { Self::new() }
}

impl EFPhysicMgr {
    /// EFPhysicMgr::EFPhysicMgr
    pub fn new() -> Self {
        EFPhysicMgr {
            slots: (0..MAX_MODULES).map(|_| (0, None)).collect(),
            last_slot: 0,
            module_count: 0,
            maps: Vec::new(),
            physics: Vec::new(),
            lists: [Vec::new(), Vec::new()],
            frame: 0,
            paused_frames: 0,
            prev_paused: 0,
            paused: 0,
            query_stamp: 0,
            notify: Vec::new(),
            col_notify: Vec::new(),
            by_owner: HashMap::new(),
        }
    }

    pub fn Physic(&self, id: PhysId) -> Option<&EFPhysic> {
        if id == 0 { None } else { self.physics.get(id as usize - 1)?.as_ref() }
    }
    pub fn PhysicMut(&mut self, id: PhysId) -> Option<&mut EFPhysic> {
        if id == 0 { None } else { self.physics.get_mut(id as usize - 1)?.as_mut() }
    }
    /// EFWorld::GetGameObject(uid) -> +0x28
    pub fn PhysicOf(&self, owner: u32) -> PhysId { self.by_owner.get(&owner).copied().unwrap_or(0) }

    /// EFPhysicMgr::AddMap: push at the head of the map list.
    pub fn AddMap(&mut self, map: EFCollisionMap) -> u32 {
        self.maps.insert(0, map);
        1
    }

    /// EFPhysicMgr::CreatePhysic(EFPhysicData*, list) (+ EFPhysicData::CreatePhysic's "only once")
    pub fn CreatePhysic(&mut self, owner: u32, frame: RwMatrix, list: u32) -> PhysId {
        let cur = self.PhysicOf(owner);
        if cur != 0 {
            return cur;
        }
        let p = EFPhysic::new(owner, frame);
        let id = match self.physics.iter().position(|x| x.is_none()) {
            Some(i) => {
                self.physics[i] = Some(p);
                i as u32 + 1
            }
            None => {
                self.physics.push(Some(p));
                self.physics.len() as u32
            }
        };
        self.lists[list as usize & 1].insert(0, id);
        self.by_owner.insert(owner, id);
        id
    }

    /// EFPhysicMgr::DeletePhysic: deferred (flags |= 0x21; destroyed by UpdateDeletedPhysic).
    pub fn DeletePhysic(&mut self, id: PhysId) {
        if let Some(p) = self.PhysicMut(id) {
            p.flags |= 0x21;
            p.UpdatePhysicFlags();
        }
    }

    /// EFPhysicMgr::DestroyPhysic: unlink, clear EFPhysicData+0x28, delete (RemoveAllModules).
    pub fn DestroyPhysic(&mut self, id: PhysId) {
        if self.Physic(id).is_none() {
            return;
        }
        for l in self.lists.iter_mut() {
            l.retain(|&x| x != id);
        }
        self.RemoveAllModules(id);
        let owner = self.physics[id as usize - 1].take().unwrap().owner;
        if self.by_owner.get(&owner) == Some(&id) {
            self.by_owner.remove(&owner);
        }
    }

    /// EFPhysicMgr::UpdateDeletedPhysic
    pub fn UpdateDeletedPhysic(&mut self) {
        for l in 0..2 {
            let ids: Vec<PhysId> = self.lists[l].clone();
            for id in ids {
                if self.Physic(id).map_or(false, |p| p.flags & 0x20 != 0) {
                    self.DestroyPhysic(id);
                }
            }
        }
    }

    /// EFPhysicMgr::Terminate: delete the maps and every physic.
    pub fn Terminate(&mut self) {
        self.maps.clear();
        for l in 0..2 {
            while let Some(&id) = self.lists[l].first() {
                self.DestroyPhysic(id);
            }
        }
        self.paused_frames = 0;
        self.prev_paused = 0;
        self.paused = 0;
    }

    // ---------------- handle table (EFPhysicModule ctor / dtor) ----------------

    /// The slot search of EFPhysicModule::EFPhysicModule. Returns the handle (0 when full).
    pub fn AllocHandle(&mut self, m: Box<dyn EFDM>) -> u32 {
        if self.module_count as usize == MAX_MODULES {
            return 0;
        }
        let mut i = self.last_slot as usize + 1;
        while i < MAX_MODULES && self.slots[i].1.is_some() {
            i += 1;
        }
        if i == MAX_MODULES {
            // quirk: wraps to 0 and scans up to last_slot inclusive
            i = 0;
            while i <= self.last_slot as usize && self.slots[i].1.is_some() {
                i += 1;
            }
        }
        self.last_slot = i as u32;
        let s = &mut self.slots[i];
        s.0 = s.0.wrapping_add(1);
        if s.0 == 0 {
            // quirk: the serial is a u32 but the overflow reset writes the u16 pair {1, 0}
            s.0 = 1;
        }
        self.module_count += 1;
        let h = (i as u32) << 16 | (s.0 & 0xffff);
        let mut m = m;
        m.base_mut().handle = h;
        s.1 = Some(m);
        h
    }

    /// handle -> slot index when the serial matches (the `(h & 0xffff) == table[h>>16].serial` test)
    pub fn Slot(&self, h: u32) -> Option<usize> {
        let i = (h >> 16) as usize;
        let s = self.slots.get(i)?;
        if (h & 0xffff) == s.0 && s.1.is_some() { Some(i) } else { None }
    }
    pub fn Module(&self, h: u32) -> Option<&dyn EFDM> {
        let i = self.Slot(h)?;
        self.slots[i].1.as_deref()
    }
    pub fn ModuleMut(&mut self, h: u32) -> Option<&mut Box<dyn EFDM>> {
        let i = self.Slot(h)?;
        self.slots[i].1.as_mut()
    }

    /// EFPhysicModule::~EFPhysicModule: unlink from the physic, clear the slot, count--.
    pub fn DestroyModule(&mut self, h: u32) {
        let Some(i) = self.Slot(h) else { return };
        let m = self.slots[i].1.take().unwrap();
        let (pid, ty) = (m.base().physic, m.base().type_);
        if let Some(p) = self.PhysicMut(pid) {
            UnLinkModule(p, h, ty);
        }
        self.module_count -= 1;
    }

    /// DMAdd tail: new module (already constructed for `phys`), handle, EFPhysic::LinkModule.
    pub fn AddModule(&mut self, phys: PhysId, mut m: Box<dyn EFDM>) -> u32 {
        if self.Physic(phys).is_none() {
            return 0;
        }
        m.base_mut().physic = phys;
        let ty = m.base().type_;
        let h = self.AllocHandle(m);
        if h == 0 {
            return 0;
        }
        let types: Vec<(u32, u32)> = self.ModuleTypes(phys);
        let p = self.PhysicMut(phys).unwrap();
        LinkModule(p, h, ty, &types);
        h
    }
    fn ModuleTypes(&self, phys: PhysId) -> Vec<(u32, u32)> {
        let Some(p) = self.Physic(phys) else { return Vec::new() };
        p.all_list.iter().filter_map(|&h| self.Module(h).map(|m| (h, m.base().type_))).collect()
    }
    fn TypeOf(&self, h: u32) -> u32 { self.Module(h).map_or(0, |m| m.base().type_) }

    /// EFPhysic::RemoveAllModules (deletes every module of the "all" list)
    pub fn RemoveAllModules(&mut self, phys: PhysId) {
        let Some(p) = self.Physic(phys) else { return };
        let all = p.all_list.clone();
        for h in all {
            self.DestroyModule(h);
        }
        if let Some(p) = self.PhysicMut(phys) {
            p.all_list.clear();
            p.pre_list.clear();
            p.col_list.clear();
            p.post_list.clear();
        }
    }

    /// EFPhysic::FindModule(type, after): first module of `type` (without list bits) in the
    /// type-sorted "all" list, starting after `after` (0 = from the head). Returns the handle.
    pub fn FindModule(&self, phys: PhysId, ty: u32, after: u32) -> u32 {
        let Some(p) = self.Physic(phys) else { return 0 };
        let start = if after == 0 { 0 } else {
            match p.all_list.iter().position(|&h| h == after) {
                Some(i) => i + 1,
                None => return 0,
            }
        };
        for &h in &p.all_list[start..] {
            let t = self.TypeOf(h) & !ALL_TYPES;
            if ty <= t {
                return if t == ty { h } else { 0 };
            }
        }
        0
    }

    // ---------------- frame update ----------------

    fn ctx_parts(&mut self, phys: PhysId) -> Option<(EFPhysic, u32, u32, u32)> {
        let p = self.physics.get_mut(phys as usize - 1)?.take()?;
        Some((p, self.frame, self.paused_frames, self.prev_paused))
    }
    fn put_back(&mut self, phys: PhysId, p: EFPhysic) { self.physics[phys as usize - 1] = Some(p); }

    /// EFPhysicMgr::Update(dt). `world_paused` = *(_gpoWorldMgr+0x14).
    pub fn Update(&mut self, dt: f32, world_paused: bool, world: &dyn PhysicWorld, cs: &mut dyn CollisionSystem) {
        if world_paused {
            self.paused_frames += 1;
            self.paused = 1;
        } else {
            self.paused = 0;
        }
        self.frame = self.frame.wrapping_add(1);
        let mut ups = EFUpdatePhysicStruct { dt, ..Default::default() };
        self.InitFrame(&mut ups, world);
        cs.BeginFrame();
        for pass in 0..2 {
            self.PreCollisionStage(&mut ups, pass, world);
            self.CollisionStage(&mut ups, pass, world, cs);
            // PostCollisionStage: empty
        }
        cs.EndFrame();
        if !world_paused {
            self.paused_frames = 0;
        }
        self.prev_paused = self.paused;
    }

    /// EFPhysicMgr::InitFrame (0x001dbd70). The original walks the world's game-object list
    /// (_gpoWorldMgr+0x4148, next = obj+0x168) and takes each object's physic (obj+0x28): that
    /// order is `world.world_order()`. When the host gives none, the physic lists are walked
    /// (list 0 then 1) — same set, possibly different order. Particle emitters: unported.
    pub fn InitFrame(&mut self, ups: &mut EFUpdatePhysicStruct, world: &dyn PhysicWorld) {
        ups.emitter_count = 0;
        ups.passes[0].clear();
        ups.passes[1].clear();
        let order: Vec<PhysId> = match world.world_order() {
            Some(uids) => uids.iter().map(|&u| self.PhysicOf(u)).filter(|&id| id != 0).collect(),
            None => self.lists[0].iter().chain(self.lists[1].iter()).copied().collect(),
        };
        {
            for &id in &order {
                let Some(p) = self.physics.get_mut(id as usize - 1).and_then(|p| p.as_mut()) else { continue };
                if p.flags & 0x10020 != 0 {
                    continue;
                }
                let frame = rows(&p.frame);
                // C++ order: +0x30 = EFPhysic+0x60 (local bsphere center) before PrecalcData,
                // +0x3c = EFPhysic+0x6c (bsphere radius) after it.
                let sphere = p.model.bsphere_center;
                p.model.PrecalcData(&frame);
                ups.passes[0].push(EFDynamicPhysic {
                    physic: id,
                    flags: 0,
                    prev_state: p.state,
                    sphere,
                    radius: p.model.bsphere_radius,
                    emitters: ups.emitter_count,
                    ..Default::default()
                });
            }
        }
    }

    /// EFPhysicMgr::PreCollisionStage(ups, pass)
    pub fn PreCollisionStage(&mut self, ups: &mut EFUpdatePhysicStruct, pass: usize, world: &dyn PhysicWorld) {
        let dt = ups.dt;
        for k in 0..ups.passes[pass].len() {
            let id = ups.passes[pass][k].physic;
            let dp = &mut ups.passes[pass][k];
            dp.disp = [0.0; 3];
            dp.accel = [0.0; 3];
            let Some(p) = self.PhysicMut(id) else { continue };
            p.moved = false; // instance+0xc &= ~0x20
            if p.flags & flags::DYNAMIC != 0 {
                self.PreCollisionUpdate(id, dt, dp, world);
            }
            let p = self.Physic(id).unwrap();
            dp.sphere = p.frame.TransformPoint(dp.sphere);
            let fd = dot(dp.disp, dp.disp);
            let mut fv = dot(p.velocity, p.velocity);
            if fv <= fd {
                fv = fd;
            }
            dp.radius += dt * fv.sqrt();
        }
    }

    /// EFPhysic::PreCollisionUpdate(dt, dyn)
    pub fn PreCollisionUpdate(&mut self, id: PhysId, dt: f32, dp: &mut EFDynamicPhysic, world: &dyn PhysicWorld) {
        let Some((mut p, frame, pf, pp)) = self.ctx_parts(id) else { return };
        dp.pos = p.frame.pos;
        let context = p.context;
        p.state = 0;
        let list = p.pre_list.clone();
        let mut dead = Vec::new();
        for h in list {
            // modules deleted by a previous module of this loop are skipped (they were unlinked)
            if !p.pre_list.contains(&h) {
                continue;
            }
            let Some(i) = self.Slot(h) else { continue };
            let m = self.slots[i].1.as_mut().unwrap();
            let b = m.base_mut();
            let on = (b.flags & 1 != 0 && context & b.context_mask != 0) || b.flags & 0x10 != 0;
            if !on {
                continue;
            }
            b.flags &= !0x10;
            let mut ctx = DMCtx { phys: &mut p, frame, paused_frames: pf, prev_paused: pp, world, notify: &mut self.notify };
            if !m.PreColUpdate(&mut ctx, dp, dt) {
                dead.push(h);
                let ty = m.base().type_;
                UnLinkModule(&mut p, h, ty);
            }
        }
        self.put_back(id, p);
        for h in dead {
            self.DestroyModule(h);
        }
    }

    /// EFPhysicMgr::CollisionStage(ups, pass, ...)
    pub fn CollisionStage(&mut self, ups: &mut EFUpdatePhysicStruct, pass: usize, world: &dyn PhysicWorld, cs: &mut dyn CollisionSystem) {
        let dt = ups.dt;
        let n = ups.passes[pass].len();
        for k in 0..n {
            if ups.passes[pass][k].flags & 2 != 0 {
                continue;
            }
            let id = ups.passes[pass][k].physic;
            let Some(p) = self.PhysicMut(id) else { continue };
            if p.flags & flags::COLLIDE == 0 {
                let d = scale(p.velocity, dt);
                ups.passes[pass][k].flags |= 2;
                if p.flags & flags::DYNAMIC != 0 {
                    let tiny = DTIsTiny(d[0] - 0.0) && DTIsTiny(d[1] - 0.0) && DTIsTiny(d[2] - 0.0);
                    if !tiny {
                        p.frame.Translate(d, rwCOMBINEPOSTCONCAT);
                    }
                    p.moved = true;
                }
                let mut dp = std::mem::take(&mut ups.passes[pass][k]);
                self.PostCollisionUpdate(id, dt, None, &mut dp, world, None);
                ups.passes[pass][k] = dp;
            } else {
                cs.Solve(self, ups, pass, k, world);
            }
        }
    }

    /// EFPhysic::PostCollisionUpdate(dt, stage, dyn). `validate` = EFCollisionModel::
    /// ValidatePrimaryPrimTrans(stage) on this physic (supplied by the collision system together
    /// with the stage; called before the modules and after the particles as in the C++).
    pub fn PostCollisionUpdate(&mut self, id: PhysId, dt: f32, stage: Option<&EFCollisionStage>, dp: &mut EFDynamicPhysic, world: &dyn PhysicWorld, validate: Option<&dyn Fn(&mut EFPhysic, u32)>) {
        let Some((mut p, frame, pf, pp)) = self.ctx_parts(id) else { return };
        let context = p.context;
        dp.pos = p.frame.pos;
        if let Some(v) = validate {
            v(&mut p, frame);
        }
        if p.flags & flags::DYNAMIC != 0 {
            for h in p.post_list.clone() {
                let Some(i) = self.Slot(h) else { continue };
                let m = self.slots[i].1.as_mut().unwrap();
                let b = m.base();
                let on = (b.flags & 1 != 0 && context & b.context_mask != 0) || b.flags & 0x40 != 0;
                if on {
                    // WP-J4: the module gets manager access (C++: the _gpoPhysicMgr global) through
                    // the stage. The module and the running physic are out of the manager during the
                    // call (Rust borrows); the physic is the ray's own owner, which the module rays
                    // exclude (rin.exclude), so the queries see the same objects as the C++.
                    let mut m = self.slots[i].1.take().unwrap();
                    let mut notify = std::mem::take(&mut self.notify);
                    match stage {
                        Some(st) => {
                            let mut q = PrimQuery;
                            let host = std::cell::RefCell::new(super::dm::ray_intersect::MgrRayHost { mgr: &mut *self, cs: &mut q });
                            let st2 = EFCollisionStage { dyn_: st.dyn_, env: st.env, ray: Some(&host) };
                            let mut ctx = DMCtx { phys: &mut p, frame, paused_frames: pf, prev_paused: pp, world, notify: &mut notify };
                            m.PostColUpdate(&mut ctx, Some(&st2), dt);
                        }
                        None => {
                            let mut ctx = DMCtx { phys: &mut p, frame, paused_frames: pf, prev_paused: pp, world, notify: &mut notify };
                            m.PostColUpdate(&mut ctx, None, dt);
                        }
                    }
                    notify.append(&mut self.notify);
                    self.notify = notify;
                    m.base_mut().flags &= !0x40;
                    self.slots[i].1 = Some(m);
                    continue;
                }
                m.base_mut().flags &= !0x40;
            }
        }
        // flags & 0x80000: CheckParticleCollisions (particles unported)
        if let Some(v) = validate {
            v(&mut p, frame);
        }
        if let Some(st) = p.UpdatePhysicState(dt, dp.prev_state) {
            self.notify.push(PhysicNotify { owner: p.owner, code: 5, data: [st, 0, 0, 0] });
        }
        self.put_back(id, p);
    }

    /// Run the ColUpdate modules of `id` (called by the collision system for each contact; the
    /// original iterates EFPhysic+0x14 inside EFCollisionStage). Same activity test as Post.
    pub fn ColUpdateModules(&mut self, id: PhysId, stage: &EFCollisionStage, d: &EFColDMData, dt: f32, world: &dyn PhysicWorld) {
        let Some((mut p, frame, pf, pp)) = self.ctx_parts(id) else { return };
        let context = p.context;
        for h in p.col_list.clone() {
            let Some(i) = self.Slot(h) else { continue };
            let m = self.slots[i].1.as_mut().unwrap();
            let b = m.base();
            let on = (b.flags & 1 != 0 && context & b.context_mask != 0) || b.flags & 0x20 != 0;
            if on {
                let mut ctx = DMCtx { phys: &mut p, frame, paused_frames: pf, prev_paused: pp, world, notify: &mut self.notify };
                m.ColUpdate(&mut ctx, stage, d, dt);
            }
            m.base_mut().flags &= !0x20;
        }
        self.put_back(id, p);
    }

    /// Run `f` on a module with a DMCtx (API helpers: SetParams/GetParams/DoAction/GetData).
    pub fn WithModule<R>(&mut self, h: u32, world: &dyn PhysicWorld, f: impl FnOnce(&mut dyn EFDM, &mut DMCtx) -> R) -> Option<R> {
        let i = self.Slot(h)?;
        let pid = self.slots[i].1.as_ref().unwrap().base().physic;
        let (mut p, frame, pf, pp) = self.ctx_parts(pid)?;
        let m = self.slots[i].1.as_mut().unwrap();
        let mut ctx = DMCtx { phys: &mut p, frame, paused_frames: pf, prev_paused: pp, world, notify: &mut self.notify };
        let r = f(m.as_mut(), &mut ctx);
        self.put_back(pid, p);
        Some(r)
    }

    // ---------------- queries ----------------

    /// EFPhysicMgr::GetPrimitivesInSphere: bumps the stamp, sums every map.
    pub fn GetPrimitivesInSphere(&mut self, d: &mut EFGetPrimitivesInSphereData) -> i32 {
        self.query_stamp = self.query_stamp.wrapping_add(1);
        let stamp = self.query_stamp;
        let mut n = 0;
        for m in self.maps.iter_mut() {
            n += m.GetPrimitivesInSphere(d, stamp);
        }
        n
    }

    /// EFPhysicMgr::RayIntersect(in, out): nearest hit over the maps (materials masked by
    /// +0x20/+0x24) then the physics (prims masked by +0x18/+0x1c). Nearest-hit mode only (the
    /// callback mode is used by engine-internal callers through EFCollisionMap directly).
    pub fn RayIntersect(&mut self, rin: &EFRayIntersectIn, cs: &mut dyn CollisionSystem) -> EFRayResult {
        let mut res = EFRayResult::default();
        let mut t = 1.0f32;
        self.query_stamp = self.query_stamp.wrapping_add(1);
        let stamp = self.query_stamp;
        if rin.map.mask1 != 0 || rin.map.mask2 != 0 {
            for (mi, m) in self.maps.iter_mut().enumerate() {
                let mut out = EFRayIntersectOut::default();
                let r = m.RayIntersect(&rin.map, &mut out, &mut t, stamp);
                if r != 0 {
                    res.kind = r as u8;
                    res.normal = out.hit.normal;
                    res.map_prim = out.hit.prim.map(|p| (mi, p));
                }
            }
        }
        if res.kind == 2 {
            res.pos = add(rin.map.origin, scale(rin.map.dir, t));
        }
        if rin.obj_mask1 != 0 || rin.obj_mask2 != 0 {
            for l in 0..2 {
                for &id in &self.lists[l] {
                    let Some(p) = self.Physic(id) else { continue };
                    if p.flags & 0x10020 != 0 || (rin.exclude != 0 && rin.exclude == p.owner) {
                        continue;
                    }
                    let mut normal = [0.0; 3];
                    if let Some(prim) = PhysicRayIntersect(p, rin, &mut t, &mut normal, cs) {
                        res.kind = 1;
                        res.physic = id;
                        res.prim = prim;
                        res.normal = normal;
                    }
                }
            }
        }
        if res.kind == 1 {
            res.pos = add(rin.map.origin, scale(rin.map.dir, t));
        }
        res
    }
}

/// EFPhysic::RayIntersect(in, out, t) in nearest-hit mode: masks vs +0x94/+0x98, primary sphere
/// precheck (CheckSphereSphereCollision, result & 9, entry t <= current t), then every prim whose
/// masks match. Returns the last prim that hit.
pub fn PhysicRayIntersect(p: &EFPhysic, rin: &EFRayIntersectIn, t: &mut f32, normal: &mut V3, cs: &mut dyn CollisionSystem) -> Option<usize> {
    // EFPhysic+0x94/+0x98 = model+0x40/+0x44 (masks[0], masks[1])
    if rin.obj_mask1 & p.model.masks[0] == 0 && rin.obj_mask2 & p.model.masks[1] == 0 {
        return None;
    }
    let o = sub(rin.map.origin, p.model.bsphere_world);
    let (hit, te) = cs.CheckSphereSphere(o, rin.map.dir, p.model.bsphere_radius);
    if hit & 9 == 0 || *t < te {
        return None;
    }
    let mut r = None;
    for (i, pr) in p.model.prims.iter().enumerate() {
        let Some(pr) = pr else { continue };
        if (rin.obj_mask1 & pr.base.word(0x10) != 0 || rin.obj_mask2 & pr.base.word(0x18) != 0) && cs.PrimRayIntersect(p, i, rin, t, normal) {
            r = Some(i);
        }
    }
    r
}

/// EFPhysic::LinkModule: insert `h` in each list its type selects, after every module of a
/// type <= its own (type compared without the list bits); always into the "all" list.
pub fn LinkModule(p: &mut EFPhysic, h: u32, ty: u32, types: &[(u32, u32)]) {
    let key = ty & !ALL_TYPES;
    let tof = |x: u32| types.iter().find(|e| e.0 == x).map_or(0, |e| e.1) & !ALL_TYPES;
    let ins = |l: &mut Vec<u32>| {
        let pos = l.iter().position(|&x| key < tof(x)).unwrap_or(l.len());
        l.insert(pos, h);
    };
    if ty & TYPE_PRE != 0 {
        ins(&mut p.pre_list);
    }
    if ty & TYPE_COL != 0 {
        ins(&mut p.col_list);
    }
    if ty & TYPE_POST != 0 {
        ins(&mut p.post_list);
    }
    ins(&mut p.all_list);
}

/// EFPhysic::UnLinkModule. quirk: the "all" list (+0x1c) is unlinked too (the C++ does it last).
pub fn UnLinkModule(p: &mut EFPhysic, h: u32, ty: u32) {
    if ty & TYPE_PRE != 0 {
        p.pre_list.retain(|&x| x != h);
    }
    if ty & TYPE_COL != 0 {
        p.col_list.retain(|&x| x != h);
    }
    if ty & TYPE_POST != 0 {
        p.post_list.retain(|&x| x != h);
    }
    p.all_list.retain(|&x| x != h);
}

/// RwMatrix (right/up/at/pos) -> model.rs row matrix.
pub fn rows(m: &RwMatrix) -> [[f32; 4]; 4] {
    [
        [m.right[0], m.right[1], m.right[2], 0.0],
        [m.up[0], m.up[1], m.up[2], 0.0],
        [m.at[0], m.at[1], m.at[2], 0.0],
        [m.pos[0], m.pos[1], m.pos[2], 1.0],
    ]
}

//! EFCollisionSystem (Classes/EF/EFCollisionSystem.cpp, all 14 functions) plus the EFPhysic
//! collision methods it drives (EFPhysic::CheckEnvCollisions 0x001d9090, CheckNoPushCollisions
//! 0x001d9940, CheckObjCollisions 0x001d9cb0, CollisionUpdate 0x001da8a0: ported below as EFPhysic_*) and
//! EFCollisionStage::CheckMultipleCollisions (0x001dddc0, needs EFPhysic state).
//!
//! Memory model:
//! - The C++ system lives on the stack of EFPhysicMgr::CollisionStage and carves its stage array,
//!   collidable lists and env prim lists out of a work buffer; here they are Vecs owned by the
//!   system (rebuilt by every Init).
//! - DTQuickPool<EFCollisionReport/EFPushReport/EFContactReport> -> `arena` (collision reports,
//!   lists are index Vecs, see report.rs) and per-stage Vecs (push/contact lists, head = index 0).
//! - EFPhysic+0x54 EFCollisionModel: WP-E2 only has `EFCollisionModelSeam`; the real model.rs
//!   models live in the side table `models` keyed by owner uid (EFPhysic::owner). Every prim is
//!   assumed attached to the physic's root frame (prim LTM = EFPhysic::frame, resynced by `Solve`).
//! - EFInstance::Notify messages of EFCollisionReport::CheckNotify -> `notifies` (drained by the
//!   coordinator).
//! - EFPhysic+0xac instance parent links (+0x1c) -> `PhysicWorld::GetParentObject`.
#![allow(non_snake_case)]

use super::mgr::{CollisionSystem, EFPhysicMgr, EFRayIntersectIn, EFUpdatePhysicStruct};
use super::model::{EFCollisionModel, ObjPrim};
use super::module::{EFColDMData, EFCollisionStage as DMStage, EFDynamicPhysic, PhysicWorld};
use super::objcoll_check::{EFCheckCollisionIn, EFCheckCollisionOut, EnvPrim};
use super::collision_map::{EFGetPrimitivesInSphereData, EFRayIntersectOut};
use super::objcoll_prim::{EFCheckSphereSphereCollisionIn, EFCheckSphereSphereCollisionOut, EFObjCollisionPrim};
use super::physic::{flags, EFPhysic, PhysId};
use super::report::{CollisionNotifyMsg, EFCollisionReport, ReportPrim};
use super::rwm::{rwCOMBINEPOSTCONCAT, RwMatrix, V3};
use super::stage::{EFContactReport, EFCollisionStage, EFPushReport, StagePhysic};

/// C++ dot order (z*z + x*x) + y*y
#[inline]
fn dz(a: V3, b: V3) -> f32 {
    a[2] * b[2] + a[0] * b[0] + a[1] * b[1]
}
#[inline]
fn sc(a: V3, s: f32) -> V3 {
    [a[0] * s, a[1] * s, a[2] * s]
}
#[inline]
fn sb(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
#[inline]
fn ad(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
#[inline]
fn neg(a: V3) -> V3 {
    [-a[0], -a[1], -a[2]]
}
fn rw_to_rows(m: &RwMatrix) -> [[f32; 4]; 4] {
    [
        [m.right[0], m.right[1], m.right[2], 0.0],
        [m.up[0], m.up[1], m.up[2], 0.0],
        [m.at[0], m.at[1], m.at[2], 0.0],
        [m.pos[0], m.pos[1], m.pos[2], 1.0],
    ]
}

/// Maximum stages of one system (CreateSystem: `while (count < 0x10)`).
pub const MAX_STAGES: usize = 0x10;
/// BuildCollidableLists cap: 0x100 bytes of pointers from the buffer start = 64 entries over
/// both lists (quirk: compared against the buffer start, not the list start).
pub const MAX_COLLIDABLES: usize = 0x40;
/// GetPrimitivesInSphere capacity per stage (the C++ uses what is left of the work buffer).
pub const ENV_CAPACITY: i32 = 0x400;

/// Prim-level tests (vtable +0x14 CheckEnvCollision / +0x18 CheckObjPrimCollision). Default =
/// the objcoll_check.rs dispatchers; tests/coordinator may substitute.
pub trait PrimTests {
    fn CheckEnvCollision(&self, p: &ObjPrim, i: &EFCheckCollisionIn, o: &mut EFCheckCollisionOut, e: &EnvPrim) -> u32 {
        p.CheckEnvCollision(i, o, e)
    }
    fn CheckObjPrimCollision(&self, p: &ObjPrim, i: &EFCheckCollisionIn, o: &mut EFCheckCollisionOut, other: &ObjPrim) -> u32 {
        p.CheckObjPrimCollision(i, o, other)
    }
}
pub struct ObjPrimTests;
impl PrimTests for ObjPrimTests {}

/// EFCollisionReport (0x30 bytes) as used by the system. `base` keeps the report.rs view
/// (stage index, prim, other_prim, t) so its list ops can be reused.
#[derive(Debug, Clone, Default)]
pub struct SysReport {
    /// +0x00 stage (index), +0x10 prim (index in stage col_prims), +0x24 other prim index, +0x20 t
    pub base: EFCollisionReport,
    /// +0x04 other stage (NULL for env / non-stage objects)
    pub other_stage: Option<usize>,
    /// +0x0c other EFDynamicPhysic (pass, index); None = environment
    pub other: Option<(usize, usize)>,
    /// +0x14 normal
    pub normal: V3,
    /// +0x24 env primitive (env reports)
    pub env: Option<EnvPrim>,
    /// +0x2c 1 = already overlapping (result & 2)
    pub overlap: bool,
}

/// EFCollisionData (ComputeCollision / CheckMultipleCollisions stack block handed to
/// EFPhysic::CollisionUpdate): +0 t, +4 normal, +0x10 bool add contact, +0x14 other EFPhysic,
/// +0x18 other prim, +0x1c env prim, +0x20 contact list.
#[derive(Debug, Clone, Default)]
pub struct EFCollisionData {
    pub t: f32,
    pub normal: V3,
    pub add_contact: bool,
    pub other: PhysId,
    /// (owner uid, prim index)
    pub other_prim: Option<(u32, usize)>,
    pub env: Option<EnvPrim>,
    pub contacts: Vec<EFContactReport>,
}

pub struct EFCollisionSystem {
    /// +0x00 initialized
    pub initialized: bool,
    /// +0x04 stages / +0x08 count
    pub stages: Vec<EFCollisionStage>,
    /// +0x0c pending collision reports (t-sorted, indices into `arena`)
    pub pending: Vec<usize>,
    /// +0x10 current time
    pub time: f32,
    /// +0x14 bounding sphere center, +0x20 radius
    pub center: V3,
    pub radius: f32,
    /// +0x24 OR of the stage physic flags (+0xc)
    pub phys_flags: u32,
    /// +0x28..+0x34 OR of the model masks (EFPhysic+0x94..+0xa0 = model+0x40..+0x4c)
    pub masks: [u32; 4],
    /// +0x38/+0x3c and +0x40/+0x44 collidable lists (pass, index)
    pub lists: [Vec<(usize, usize)>; 2],
    /// report pool
    pub arena: Vec<SysReport>,
    /// EFInstance::Notify(target uid, id, msg) calls of CheckNotify, in order
    pub notifies: Vec<(u32, u32, CollisionNotifyMsg)>,
    pub tests: Box<dyn PrimTests>,
    /// the game's single newlib rand() (0x00140308) used by GetEscapeNoPF; the coordinator shares
    /// the runtime's `LibcRand.0` here (default: a private generator, seed 1)
    pub rand: crate::engine::audio::mgr::SharedRand,
}

impl Default for EFCollisionSystem {
    fn default() -> Self {
        Self::new()
    }
}

/// InitColPrims adapter: E2 has no child-physic chain (EFInstance+0xac child links), so only the
/// physic's own prims are listed.
struct PrimList(u32, PhysId);
impl StagePhysic for PrimList {
    fn col_prims(&self) -> Vec<u32> {
        (0..self.0).collect()
    }
    fn pushable_children(&self) -> Vec<&dyn StagePhysic> {
        vec![]
    }
    fn id(&self) -> u32 {
        self.1
    }
}

impl EFCollisionSystem {
    /// EFCollisionSystem() (0x001de410): +0 = 0, +0xc = 0, +0x14..+0x1c = 0.
    pub fn new() -> Self {
        EFCollisionSystem {
            initialized: false,
            stages: Vec::new(),
            pending: Vec::new(),
            time: 0.0,
            center: [0.0; 3],
            radius: 0.0,
            phys_flags: 0,
            masks: [0; 4],
            lists: [Vec::new(), Vec::new()],
            arena: Vec::new(),
            notifies: Vec::new(),
            tests: Box::new(ObjPrimTests),
            rand: Default::default(),
        }
    }

    // ---------------------------------------------------------------- accessors

    fn dyn_of<'a>(ups: &'a EFUpdatePhysicStruct, d: (usize, usize)) -> &'a EFDynamicPhysic {
        &ups.passes[d.0][d.1]
    }
    fn pid(ups: &EFUpdatePhysicStruct, d: (usize, usize)) -> PhysId {
        ups.passes[d.0][d.1].physic
    }
    fn owner(mgr: &EFPhysicMgr, id: PhysId) -> u32 {
        mgr.Physic(id).map_or(0, |p| p.owner)
    }
    fn vel(mgr: &EFPhysicMgr, id: PhysId) -> V3 {
        mgr.Physic(id).map_or([0.0; 3], |p| p.velocity)
    }
    fn prim_count(&self, mgr: &EFPhysicMgr, id: PhysId) -> usize {
        Self::model(mgr, Self::owner(mgr, id)).map_or(0, |m| m.prims.len())
    }
    fn prim(mgr: &EFPhysicMgr, owner: u32, i: usize) -> Option<&ObjPrim> {
        Self::model(mgr, owner)?.prims.get(i)?.as_ref()
    }
    /// EFPhysic+0x54 of the physic owned by `owner`
    fn model(mgr: &EFPhysicMgr, owner: u32) -> Option<&EFCollisionModel> {
        mgr.Physic(mgr.PhysicOf(owner)).map(|p| &p.model)
    }
    fn model_mut(mgr: &mut EFPhysicMgr, owner: u32) -> Option<&mut EFCollisionModel> {
        let id = mgr.PhysicOf(owner);
        mgr.PhysicMut(id).map(|p| &mut p.model)
    }
    fn report_prim(p: Option<&ObjPrim>) -> Option<ReportPrim> {
        p.map(|p| ReportPrim { tag: p.base.tag, flags: p.base.word(0xc), user: [p.base.word(0x28), p.base.word(0x2c), p.base.word(0x30), p.base.word(0x34)] })
    }
    fn insert_report(&mut self, r: SysReport, list_stage: usize) -> usize {
        let me = self.arena.len();
        self.arena.push(r);
        let bases: Vec<EFCollisionReport> = self.arena.iter().map(|r| r.base).collect();
        EFCollisionReport::InsertTo(&bases, me, &mut self.stages[list_stage].reports);
        me
    }

    /// Resync EFPhysic+0x54 prims with their frames and EFCollisionModel::PrecalcData. Seam: the
    /// C++ prims read their DTFrame LTM directly; here `joint` 0xffffffff = the physic's root frame,
    /// else the bone LTM from `PhysicWorld::GetBone` (root frame when unavailable).
    pub fn SyncModels(&mut self, mgr: &mut EFPhysicMgr, world: &dyn PhysicWorld) {
        for p in mgr.physics.iter_mut().flatten() {
            let ltm = rw_to_rows(&p.frame);
            for pr in p.model.prims.iter_mut().flatten() {
                pr.base.ltm = match pr.base.joint {
                    0xffff_ffff => ltm,
                    j => world.GetBone(p.owner, j).map_or(ltm, |(_, l)| rw_to_rows(&l)),
                };
            }
            p.model.PrecalcData(&ltm);
        }
    }

    /// EFCollisionModel::Translate on EFPhysic+0x54 including its RwFrameTranslate and
    /// FlagTranslatedChild side effects.
    pub fn ModelTranslate(&mut self, mgr: &mut EFPhysicMgr, id: PhysId, v: V3) {
        let Some(p) = mgr.PhysicMut(id) else { return };
        let m = &mut p.model;
        let mv = m.Translate(v);
        m.translated = false;
        if mv {
            p.frame.Translate(v, rwCOMBINEPOSTCONCAT);
            for pr in m.prims.iter_mut().flatten() {
                for k in 0..3 {
                    pr.base.ltm[3][k] += v[k];
                }
            }
        }
        // FlagTranslatedChild: EFPhysicData+0xc |= 0x20
        p.moved = true;
    }

    /// EFCollisionModel::ValidatePrimaryPrimTrans(stage) on the stage physic's model.
    pub fn ValidatePrimaryPrimTrans(&mut self, mgr: &mut EFPhysicMgr, ups: &EFUpdatePhysicStruct, s: usize) {
        let id = Self::pid(ups, self.stages[s].dyn_ref);
        let frame = mgr.frame;
        let Some(p) = mgr.PhysicMut(id) else { return };
        ValidatePhysic(p, frame, &self.stages[s]);
    }

    // ---------------------------------------------------------------- EFCollisionSystem

    /// Init(ups, pass, index, buf, bufEnd) (0x001de480)
    pub fn Init(&mut self, mgr: &mut EFPhysicMgr, ups: &mut EFUpdatePhysicStruct, pass: usize, index: usize) {
        if self.initialized {
            self.Terminate(ups);
        }
        self.CreateSystem(mgr, ups, pass, index);
        self.BuildCollidableLists(ups);
        self.BuildParticleEmittersList(mgr, ups);
        self.BuildEnvPrimsLists(mgr, ups);
        self.initialized = true;
        for s in 0..self.stages.len() {
            let owner = Self::owner(mgr, Self::pid(ups, self.stages[s].dyn_ref));
            let has = Self::model(mgr, owner).is_some_and(|m| m.primary != -1 && m.prims.get(m.primary as usize).is_some_and(|p| p.is_some()));
            if has {
                self.ValidatePrimaryPrimTrans(mgr, ups, s);
            }
        }
    }

    /// CreateSystem(ups, pass, index, buf) (0x001de5d0): stage 0 = the dyn physic, then every
    /// later COLLIDE physic of the pass whose dyn sphere touches a stage's sphere (max 16).
    pub fn CreateSystem(&mut self, mgr: &EFPhysicMgr, ups: &mut EFUpdatePhysicStruct, pass: usize, index: usize) {
        let dt = ups.dt;
        self.stages.clear();
        self.stages.push(EFCollisionStage { dyn_ref: (pass, index), ..Default::default() });
        ups.passes[pass][index].flags |= 1;
        self.phys_flags = 0;
        self.masks = [0; 4];
        let n = ups.passes[pass].len();
        let mut s = 0;
        while s < self.stages.len() {
            let dref = self.stages[s].dyn_ref;
            let id = Self::pid(ups, dref);
            let (pflags, vel) = mgr.Physic(id).map_or((0, [0.0; 3]), |p| (p.flags, p.velocity));
            let mm = Self::model(mgr, Self::owner(mgr, id)).map_or([0; 6], |m| m.masks);
            for k in 0..4 {
                self.masks[k] |= mm[k];
            }
            self.phys_flags |= pflags;
            let dp = Self::dyn_of(ups, dref).clone();
            let st = &mut self.stages[s];
            st.reports.clear();
            st.min_t = 0.0;
            st.active = true;
            st.r2 = dp.radius * dp.radius;
            let l = dz(vel, vel).sqrt();
            st.step = if l == 0.0 { dt } else { 0.001 / l };
            st.pushes.clear();
            st.contacts.clear();
            st.push_count = 0;
            st.moved = [0.0; 3];
            st.lists = [Vec::new(), Vec::new()];
            st.InitColPrims(&PrimList(Self::model(mgr, Self::owner(mgr, id)).map_or(0, |m| m.prims.len() as u32), id));
            let mut j = index;
            while self.stages.len() < MAX_STAGES {
                j += 1;
                if j >= n {
                    break;
                }
                let dj = &ups.passes[pass][j];
                let jid = dj.physic;
                let ok = dj.flags & 3 == 0 && mgr.Physic(jid).is_some_and(|p| p.flags & flags::COLLIDE != 0) && self.prim_count(mgr, jid) != 0;
                if !ok {
                    continue;
                }
                let d = sb(dj.sphere, dp.sphere);
                let rr = dj.radius + dp.radius;
                if rr * rr < d[2] * d[2] + d[0] * d[0] + d[1] * d[1] {
                    continue;
                }
                self.stages.push(EFCollisionStage { dyn_ref: (pass, j), ..Default::default() });
                ups.passes[pass][j].flags |= 1;
            }
            s += 1;
        }
        self.ComputeBSphere(mgr, ups);
    }

    /// BuildParticleEmittersList (0x001de8f0): only when the stage flags have bit 19 (PARTICLES):
    /// the emitters (with live particles) of the types whose user ints meet the system masks and
    /// whose frame lies within system radius + type bsphere radius of the system center; every
    /// stage's dyn +0x40/+0x44 = that list.
    pub fn BuildParticleEmittersList(&mut self, mgr: &EFPhysicMgr, ups: &mut EFUpdatePhysicStruct) {
        if self.phys_flags & flags::PARTICLES == 0 {
            return;
        }
        let m = self.masks; // +0x28 = [0], +0x2c = [1], +0x30 = [2], +0x34 = [3]
        let mut list = Vec::new();
        for (i, em) in mgr.emitters.iter().enumerate() {
            let u = em.user;
            if m[2] & u[0] == 0 && m[3] & u[1] == 0 && m[0] & u[2] == 0 && m[1] & u[3] == 0 {
                continue;
            }
            if em.parts.is_empty() {
                continue;
            }
            let r = self.radius + em.type_radius;
            let d = [em.pos[0] - self.center[0], em.pos[1] - self.center[1], em.pos[2] - self.center[2]];
            if d[2] * d[2] + d[0] * d[0] + d[1] * d[1] <= r * r {
                list.push(i);
            }
        }
        for st in &self.stages {
            if let Some(dp) = ups.passes[st.dyn_ref.0].get_mut(st.dyn_ref.1) {
                dp.emitters = list.len() as u32;
                dp.emitter_list = list.clone();
            }
        }
    }

    /// BuildEnvPrimsLists(ups, buf, bufEnd) (0x001deae0): per stage
    /// EFPhysicMgr::GetPrimitivesInSphere({dyn sphere, dyn radius, model+0x50, model+0x54}) and
    /// active = 1. The prims are copied (EnvPrim::from_map) from every map.
    pub fn BuildEnvPrimsLists(&mut self, mgr: &mut EFPhysicMgr, ups: &EFUpdatePhysicStruct) {
        for s in 0..self.stages.len() {
            let dref = self.stages[s].dyn_ref;
            let dp = Self::dyn_of(ups, dref);
            let mm = Self::model(mgr, Self::owner(mgr, dp.physic)).map_or([0; 6], |m| m.masks);
            let mut d = EFGetPrimitivesInSphereData { center: dp.sphere, radius: dp.radius, mask1: mm[4], mask2: mm[5], out: Vec::new(), capacity: ENV_CAPACITY };
            // EFPhysicMgr::GetPrimitivesInSphere (one stamp, every map), keeping the map of each prim
            mgr.query_stamp = mgr.query_stamp.wrapping_add(1);
            let stamp = mgr.query_stamp;
            let mut env = Vec::new();
            for m in mgr.maps.iter_mut() {
                d.out.clear();
                m.GetPrimitivesInSphere(&mut d, stamp);
                for &p in &d.out {
                    env.push(EnvPrim::from_map(m, p));
                }
            }
            let st = &mut self.stages[s];
            st.env_prims = env.iter().map(|e| e.id).collect();
            st.env = env;
            st.active = true;
        }
    }

    /// BuildCollidableLists(ups, buf) (0x001debd0)
    pub fn BuildCollidableLists(&mut self, ups: &EFUpdatePhysicStruct) {
        self.lists = [Vec::new(), Vec::new()];
        let mut total = 0;
        'passes: for l in 0..2 {
            for (i, dp) in ups.passes[l].iter().enumerate() {
                if dp.flags & 1 != 0 {
                    continue;
                }
                let d = sb(dp.sphere, self.center);
                let r = self.radius + dp.radius;
                if d[2] * d[2] + d[0] * d[0] + d[1] * d[1] <= r * r {
                    self.lists[l].push((l, i));
                    total += 1;
                    if MAX_COLLIDABLES <= total {
                        break 'passes;
                    }
                }
            }
        }
        if self.stages.len() == 1 {
            self.stages[0].lists = self.lists.clone();
        } else {
            for s in 0..self.stages.len() {
                let me = Self::dyn_of(ups, self.stages[s].dyn_ref).clone();
                for l in 0..2 {
                    // quirk: no capacity check here (the C++ can overrun its buffer)
                    let v: Vec<(usize, usize)> = self.lists[l]
                        .iter()
                        .copied()
                        .filter(|&d| {
                            let o = Self::dyn_of(ups, d);
                            if o.flags & 1 != 0 {
                                return false;
                            }
                            let q = sb(o.sphere, me.sphere);
                            let r = me.radius + o.radius;
                            q[2] * q[2] + q[0] * q[0] + q[1] * q[1] <= r * r
                        })
                        .collect();
                    self.stages[s].lists[l] = v;
                }
            }
        }
    }

    /// ComputeBSphere() (0x001def50). quirk: the pair search adds the *squared* radii (+0x28) to
    /// the squared center distance (reads EFPhysic+0x70, the model's world center).
    pub fn ComputeBSphere(&mut self, mgr: &EFPhysicMgr, ups: &EFUpdatePhysicStruct) {
        let n = self.stages.len();
        if n == 1 {
            let dp = Self::dyn_of(ups, self.stages[0].dyn_ref);
            self.center = dp.sphere;
            self.radius = dp.radius;
            return;
        }
        let wc = |s: usize| mgr.Physic(Self::pid(ups, self.stages[s].dyn_ref)).map_or([0.0; 3], |p| p.model.bsphere_world);
        let mut best = -1.0f32;
        let (mut a, mut b) = (0, 0);
        for i in 0..n.saturating_sub(1) {
            let ci = wc(i);
            for j in i + 1..n {
                let d = sb(wc(j), ci);
                let f = self.stages[j].r2 + self.stages[i].r2 + (d[2] * d[2] + d[0] * d[0] + d[1] * d[1]);
                if !(f <= best) {
                    best = f;
                    a = i;
                    b = j;
                }
            }
        }
        self.center = sc(ad(wc(a), wc(b)), 0.5);
        self.radius = -1.0;
        for s in 0..n {
            let dp = Self::dyn_of(ups, self.stages[s].dyn_ref);
            let d = sb(dp.sphere, self.center);
            let f = dp.radius + (d[2] * d[2] + d[0] * d[0] + d[1] * d[1]).sqrt();
            if self.radius < f {
                self.radius = f;
            }
        }
    }

    /// Update(ups) (0x001df230): up to 11 sub-steps: find collisions, move every stage to the
    /// first hit time, resolve every pending hit, then the multi-contact constraints.
    pub fn Update(&mut self, mgr: &mut EFPhysicMgr, ups: &mut EFUpdatePhysicStruct, world: &dyn PhysicWorld) {
        let dt = ups.dt;
        self.time = 0.0;
        let mut it = 0u32;
        loop {
            let stop = 10 < it;
            it += 1;
            if stop {
                break;
            }
            self.CheckCollisions(mgr, ups, world);
            let mut t1 = dt;
            if let Some(&h) = self.pending.first() {
                t1 = self.arena[h].base.t;
                if dt <= t1 {
                    t1 = dt;
                }
            }
            for s in 0..self.stages.len() {
                let id = Self::pid(ups, self.stages[s].dyn_ref);
                let Some(p) = mgr.Physic(id) else { continue };
                let k = t1 - self.time;
                let mut d = sc(p.velocity, k);
                // carrier: parent instance whose physic has RELATIVE_VEL -> parent-local relative motion
                if let Some(parent) = world.GetParentObject(p.owner) {
                    if let Some(pp) = mgr.Physic(mgr.PhysicOf(parent)) {
                        if pp.flags & flags::RELATIVE_VEL != 0 {
                            let rel = sb(p.velocity, pp.velocity);
                            d = sc(rel, t1 - self.time);
                            let inv = pp.frame.Invert();
                            d = inv.TransformVector(d);
                        }
                    }
                }
                self.ModelTranslate(mgr, id, d);
                let st = &mut self.stages[s];
                st.moved = ad(st.moved, d);
                if 1e-6 < d[2] * d[2] + d[0] * d[0] + d[1] * d[1] {
                    st.contacts.clear();
                }
            }
            // the C++ also computes min(step)+t1 limits here; the values are dead
            while let Some(&h) = self.pending.first() {
                if !(self.arena[h].base.t < dt) {
                    break;
                }
                self.ComputeCollision(mgr, ups, world, h);
                self.pending.remove(0);
            }
            for s in 0..self.stages.len() {
                if self.stages[s].active {
                    self.Stage_CheckMultipleCollisions(mgr, ups, world, s, dt);
                }
            }
            self.time = t1;
            if !(0.0005 < dt - self.time) {
                break;
            }
        }
        for s in 0..self.stages.len() {
            self.ValidatePrimaryPrimTrans(mgr, ups, s);
        }
        // EFInstance::Notify(0x17 / 3) calls of CheckNotify -> manager queue for the coordinator
        mgr.col_notify.append(&mut self.notifies);
    }

    /// ComputeCollision(ups) (0x001df6d0) on the pending report `h` (the list head).
    pub fn ComputeCollision(&mut self, mgr: &mut EFPhysicMgr, ups: &mut EFUpdatePhysicStruct, world: &dyn PhysicWorld, h: usize) {
        let dt = ups.dt;
        let r = self.arena[h].clone();
        let t0 = r.base.t;
        let n = r.normal;
        let add_contact = !r.overlap;
        let sa = r.base.stage as usize;
        let da = self.stages[sa].dyn_ref;
        let ida = Self::pid(ups, da);
        let Some(pa) = mgr.Physic(ida).cloned() else { return };
        let va0 = pa.velocity; // fStack_200
        let mut vb0 = [0.0f32; 3]; // fStack_1f0
        let mut rel = pa.velocity;
        let idb = r.other.map_or(0, |d| Self::pid(ups, d));
        let pb = mgr.Physic(idb).cloned();
        if let Some(pb) = &pb {
            rel = sb(rel, pb.velocity);
            vb0 = pb.velocity;
        }
        if -1e-5 < dz(rel, n) {
            if r.other.is_some() {
                self.Report_CheckNotify(mgr, ups, &r, va0, vb0);
            }
            return;
        }
        // restitution
        let mut e = 0.0f32;
        match &pb {
            None => {
                let (reb, hard) = r.env.as_ref().map_or((0.0, 0.0), |p| (f32::from_bits(p.mat[4]), f32::from_bits(p.mat[6])));
                let s = 2.0 - (pa.hardness + hard);
                if 0.0 < s {
                    e = ((1.0 - pa.hardness) * pa.rebound) / s + ((1.0 - hard) * reb) / s;
                }
            }
            Some(pb) => {
                let s = 2.0 - (pa.hardness + pb.hardness);
                if 0.0 < s {
                    e = ((1.0 - pa.hardness) * pa.rebound) / s + ((1.0 - pb.hardness) * pb.rebound) / s;
                }
            }
        }
        let mut f15 = dz(va0, n);
        if let Some(p) = mgr.PhysicMut(ida) {
            p.velocity = ad(p.velocity, sc(n, -f15));
        }
        let mut f11 = 0.0f32;
        if pb.is_some() {
            f11 = dz(vb0, n);
            if r.other_stage.is_some() {
                if let Some(p) = mgr.PhysicMut(idb) {
                    p.velocity = ad(p.velocity, sc(n, -f11));
                }
            }
        }
        let acc_a = ups.passes[da.0][da.1].accel;
        let mut f13 = dz(acc_a, n);
        ups.passes[da.0][da.1].accel = sb(acc_a, sc(n, f13));
        let mut f16 = 0.0f32;
        if let Some(db) = r.other {
            let acc_b = ups.passes[db.0][db.1].accel;
            f16 = dz(acc_b, n);
            ups.passes[db.0][db.1].accel = sb(acc_b, sc(n, f16));
        }
        let dtl = dt - t0;
        f15 = f15 - f13 * dtl;
        f11 = f11 - f16 * dtl;
        if 0.0 < f15 - f11 {
            let k = -(f15 - f11) / (dtl * (f13 - f16));
            let f9 = f13 * k;
            let f10 = f16 * k;
            f15 = f15 + dtl * f9;
            f13 = f13 - f9;
            f11 = f11 + dtl * f10;
            f16 = f16 - f10;
        }
        let pbp = pb.as_ref().map_or(0, |p| p.priority);
        let pbw = pb.as_ref().map_or(0.0, |p| p.weight);
        let stage_b = r.other_stage.is_some();
        if !stage_b || pa.priority != pbp {
            if !(stage_b && pbp < pa.priority) {
                f13 = f16;
            }
        } else {
            f16 = (f16 * pbw + f13 * pa.weight) / (pa.weight + pbw);
            f13 = f16;
        }
        ups.passes[da.0][da.1].accel = ad(ups.passes[da.0][da.1].accel, sc(n, f13));
        if let Some(db) = r.other {
            ups.passes[db.0][db.1].accel = ad(ups.passes[db.0][db.1].accel, sc(n, f13));
        }
        f15 = f15 + f13 * dtl;
        f11 = f11 + f13 * dtl;
        let mut f12 = (e + 1.0) * (f11 - f15);
        if !stage_b || pa.priority != pbp {
            if !stage_b || pa.priority <= pbp {
                f15 = f15 + f12;
            } else {
                f11 = f11 + f12;
            }
        } else {
            let ia = 1.0 / pa.weight;
            let ib = 1.0 / pbw;
            f12 = f12 / (ia + ib);
            f15 = f15 + f12 * ia;
            f11 = f11 - f12 * ib;
        }
        if let Some(p) = mgr.PhysicMut(ida) {
            p.velocity = ad(p.velocity, sc(n, f15));
        }
        if stage_b {
            if let Some(p) = mgr.PhysicMut(idb) {
                p.velocity = ad(p.velocity, sc(n, f11));
            }
        }
        let mut data = EFCollisionData { t: t0, normal: n, add_contact, other: idb, ..Default::default() };
        match &pb {
            // quirk: +0x18 (other prim) is left uninitialised for env hits
            None => data.env = r.env.clone(),
            Some(pb) => data.other_prim = Some((pb.owner, r.base.other_prim as usize)),
        }
        self.EFPhysic_CollisionUpdate(mgr, ups, world, ida, Some(sa), &data, dt);
        if r.other.is_some() {
            data.normal = neg(data.normal);
            let cp = self.stages[sa].col_prims.get(r.base.prim as usize).copied();
            data.other_prim = cp.map(|(pi, pid)| (Self::owner(mgr, pid), pi as usize));
            data.other = ida;
            self.EFPhysic_CollisionUpdate(mgr, ups, world, idb, r.other_stage, &data, dt);
        }
        if let Some(sb_) = r.other_stage {
            self.Stage_CheckMultipleCollisions(mgr, ups, world, sa, dt);
            let mut va = Self::vel(mgr, ida);
            let mut vb = Self::vel(mgr, idb);
            let rn = r.normal;
            let fa = dz(va, rn);
            let mut fb = dz(vb, rn);
            if fa < fb {
                let d = fb - fa;
                vb = sb(vb, sc(rn, d));
                fb = fa;
            }
            self.Stage_CheckMultipleCollisions(mgr, ups, world, sb_, dt);
            let fa = dz(va, rn);
            if fa < fb {
                let d = fb - fa;
                va = ad(va, sc(rn, d));
            }
            if -0.0001 < dz(va, rn) - dz(vb, rn) {
                va = sc(va, 1.0001);
            }
            if let Some(p) = mgr.PhysicMut(ida) {
                p.velocity = va;
                p.falling_timer = p.falling_delay;
            }
            if let Some(p) = mgr.PhysicMut(idb) {
                p.velocity = vb;
                p.falling_timer = p.falling_delay;
            }
        }
        self.Report_CheckNotify(mgr, ups, &r, va0, vb0);
    }

    /// PostUpdate(ups) (0x001e04d0)
    pub fn PostUpdate(&mut self, mgr: &mut EFPhysicMgr, ups: &mut EFUpdatePhysicStruct, world: &dyn PhysicWorld) {
        let dt = ups.dt;
        self.CheckNoPushCollisions(mgr, ups);
        for s in 0..self.stages.len() {
            let dref = self.stages[s].dyn_ref;
            let id = Self::pid(ups, dref);
            // EFPhysic::PostCollisionUpdate(dt, stage, dyn): Validate, modules, particles,
            // Validate, UpdatePhysicState.
            let mut dp = std::mem::take(&mut ups.passes[dref.0][dref.1]);
            let snap = dp.clone();
            let stage = &self.stages[s];
            let v = |p: &mut EFPhysic, frame: u32| ValidatePhysic(p, frame, stage);
            mgr.PostCollisionUpdate(id, dt, Some(&DMStage { dyn_: &snap, env: &stage.env, ray: None }), &mut dp, world, Some(&v));
            ups.passes[dref.0][dref.1] = dp;
            let f = &mut ups.passes[dref.0][dref.1].flags;
            *f &= 0xffff_fffe;
            *f |= 2;
            let st = &mut self.stages[s];
            st.reports.clear();
            st.contacts.clear();
        }
    }

    /// CheckCollisions(ups) (0x001e0600)
    pub fn CheckCollisions(&mut self, mgr: &mut EFPhysicMgr, ups: &mut EFUpdatePhysicStruct, world: &dyn PhysicWorld) {
        let dt = ups.dt;
        // drop the pending reports touching an active stage
        let pending = std::mem::take(&mut self.pending);
        self.pending = pending
            .into_iter()
            .filter(|&h| {
                let r = &self.arena[h];
                !self.stages[r.base.stage as usize].active && r.other_stage.is_none_or(|o| !self.stages[o].active)
            })
            .collect();
        for st in self.stages.iter_mut() {
            if st.active {
                st.min_t = f32::from_bits(0x47c3_5000); // 100000
            }
        }
        let n = self.stages.len();
        for i in 0..n {
            if !self.stages[i].active {
                continue;
            }
            let time = self.time;
            self.EFPhysic_CheckEnvCollisions(mgr, ups, i, time, dt);
            for l in 0..2 {
                let list = self.stages[i].lists[l].clone();
                for d in list {
                    self.EFPhysic_CheckObjCollisions(mgr, ups, world, i, d, None, time, dt);
                }
            }
            for j in 0..n {
                if (i <= j || !self.stages[j].active) && j != i {
                    let dj = self.stages[j].dyn_ref;
                    self.EFPhysic_CheckObjCollisions(mgr, ups, world, i, dj, Some(j), time, dt);
                }
            }
            let chain = std::mem::take(&mut self.stages[i].reports);
            let bases: Vec<EFCollisionReport> = self.arena.iter().map(|r| r.base).collect();
            EFCollisionReport::MergeInto(&bases, &chain, &mut self.pending);
        }
        for s in 0..n {
            self.stages[s].active = false;
            if let Some(v) = self.stages[s].CheckPush() {
                let id = Self::pid(ups, self.stages[s].dyn_ref);
                self.ModelTranslate(mgr, id, v);
            }
        }
    }

    /// Terminate(ups) (0x001e0930)
    pub fn Terminate(&mut self, ups: &mut EFUpdatePhysicStruct) {
        if self.initialized {
            self.pending.clear();
            for st in &self.stages {
                let d = st.dyn_ref;
                if let Some(dp) = ups.passes[d.0].get_mut(d.1) {
                    dp.emitters = ups.emitter_count;
                    dp.emitter_list.clear();
                }
            }
            self.arena.clear();
            self.initialized = false;
        }
    }

    /// CheckNoPushCollisions() (0x001e09e0): stage vs its lists, then vs every later stage.
    pub fn CheckNoPushCollisions(&mut self, mgr: &EFPhysicMgr, ups: &EFUpdatePhysicStruct) {
        let n = self.stages.len();
        for i in 0..n {
            for l in 0..2 {
                for d in self.stages[i].lists[l].clone() {
                    self.EFPhysic_CheckNoPushCollisions(mgr, ups, i, d);
                }
            }
            for j in i + 1..n {
                let d = self.stages[j].dyn_ref;
                self.EFPhysic_CheckNoPushCollisions(mgr, ups, i, d);
            }
        }
    }

    // ---------------------------------------------------------------- EFCollisionStage

    /// EFCollisionStage::CheckMultipleCollisions(dt) (0x001dddc0)
    pub fn Stage_CheckMultipleCollisions(&mut self, mgr: &mut EFPhysicMgr, ups: &mut EFUpdatePhysicStruct, world: &dyn PhysicWorld, s: usize, dt: f32) {
        if self.stages[s].contacts.len() < 2 {
            return;
        }
        let id = Self::pid(ups, self.stages[s].dyn_ref);
        let mut v = Self::vel(mgr, id);
        {
            let m: &EFPhysicMgr = mgr;
            self.stages[s].SolveMultipleCollisions(&mut v, &|o| Self::vel(m, o));
        }
        let mut d = sb(v, Self::vel(mgr, id));
        let f = d[2] * d[2] + d[0] * d[0] + d[1] * d[1];
        if 1e-6 < f {
            d = sc(d, 1.0 / f.sqrt());
            if let Some(p) = mgr.PhysicMut(id) {
                p.velocity = v;
                p.falling_timer = p.falling_delay;
            }
            let data = EFCollisionData { t: 0.0, normal: d, contacts: self.stages[s].contacts.clone(), ..Default::default() };
            self.EFPhysic_CollisionUpdate(mgr, ups, world, id, Some(s), &data, dt);
        }
        if let Some(p) = mgr.PhysicMut(id) {
            p.velocity = v;
            p.falling_timer = p.falling_delay;
        }
    }

    // ---------------------------------------------------------------- EFPhysic (collision part)

    /// EFPhysic::CheckEnvCollisions(stage, t0, t1) (0x001d9090). Returns true when a report was added.
    pub fn EFPhysic_CheckEnvCollisions(&mut self, mgr: &EFPhysicMgr, ups: &EFUpdatePhysicStruct, s: usize, t0: f32, t1: f32) -> bool {
        let id = Self::pid(ups, self.stages[s].dyn_ref);
        let vel = Self::vel(mgr, id);
        let input = EFCheckCollisionIn { tmax: t1 - t0, motion: vel };
        let mut any = false;
        let cps = self.stages[s].col_prims.clone();
        let env = self.stages[s].env.clone();
        for (pi, &(prim, pid)) in cps.iter().enumerate() {
            let owner = Self::owner(mgr, pid);
            let Some(p) = Self::prim(mgr, owner, prim as usize).cloned() else { continue };
            let f = p.base.word(0xc);
            if f & 0x10 != 0 || f & 2 != 0 || f & 1 == 0 {
                continue;
            }
            let (m1, m2) = (p.base.word(0x20), p.base.word(0x24));
            if m1 == 0 && m2 == 0 {
                continue;
            }
            for e in &env {
                if e.mat[1] & m1 == 0 && e.mat[2] & m2 == 0 {
                    continue;
                }
                let mut o = EFCheckCollisionOut::default();
                let r = self.tests.CheckEnvCollision(&p, &input, &mut o, e);
                if r & 3 == 0 {
                    continue;
                }
                if r & 2 != 0 {
                    let st = &mut self.stages[s];
                    st.contacts.insert(0, EFContactReport { n: o.normal, other: 0, prim: None, env: Some(e.clone()) });
                    if 0.001 < o.depth {
                        st.pushes.insert(0, EFPushReport { v: o.normal, depth: o.depth, prio: 100, weight: 1.0 });
                        st.push_count += 1;
                    }
                }
                let x = dz(o.normal, vel);
                // |x| >= 2^-8 (exponent test) and negative
                if 0x3b7f_ffff < (x.to_bits() & 0x7f80_0000) && x.to_bits() & 0x8000_0000 != 0 {
                    let t = if r & 2 == 0 { o.t + t0 } else { t0 };
                    any = true;
                    let rep = SysReport {
                        base: EFCollisionReport { stage: s as u32, prim: pi as u32, other: 0, other_prim: 0, t },
                        other_stage: None,
                        other: None,
                        normal: o.normal,
                        env: Some(e.clone()),
                        overlap: r & 2 != 0,
                    };
                    let me = self.insert_report(rep, s);
                    let st = &mut self.stages[s];
                    if self.arena[me].base.t < st.min_t {
                        st.min_t = self.arena[me].base.t;
                    }
                }
            }
        }
        any
    }

    /// EFPhysic::CheckObjCollisions(stage, dyn, otherStage, t0, t1) (0x001d9cb0)
    #[allow(clippy::too_many_arguments)]
    pub fn EFPhysic_CheckObjCollisions(&mut self, mgr: &EFPhysicMgr, ups: &EFUpdatePhysicStruct, world: &dyn PhysicWorld, s: usize, d: (usize, usize), os: Option<usize>, t0: f32, t1: f32) {
        let ida = Self::pid(ups, self.stages[s].dyn_ref);
        let idb = Self::pid(ups, d);
        let (Some(pa), Some(pb)) = (mgr.Physic(ida), mgr.Physic(idb)) else { return };
        let off = if os.is_none() { sc(pb.velocity, t0) } else { [0.0; 3] };
        let input = EFCheckCollisionIn { tmax: t1 - t0, motion: sb(pa.velocity, pb.velocity) };
        if pb.flags == 0 && pa.flags & flags::RELATIVE_VEL == 0 {
            // skip parent/child pairs (EFInstance+0x1c chains)
            let mut c = Some(pb.owner);
            while let Some(u) = c {
                if u == pa.owner {
                    return;
                }
                c = world.GetParentObject(u);
            }
            let mut c = Some(pa.owner);
            while let Some(u) = c {
                if u == pb.owner {
                    return;
                }
                c = world.GetParentObject(u);
            }
        }
        let (a_owner, b_owner) = (pa.owner, pb.owner);
        let (a_prio, a_w, b_prio, b_w) = (pa.priority, pa.weight, pb.priority, pb.weight);
        let nb = self.prim_count(mgr, idb);
        let cps = self.stages[s].col_prims.clone();
        for (pi, &(prim, pid)) in cps.iter().enumerate() {
            let Some(p) = Self::prim(mgr, Self::owner(mgr, pid), prim as usize).cloned() else { continue };
            let f = p.base.word(0xc);
            if f & 0x10 != 0 || f & 2 != 0 || f & 0x80 != 0 {
                continue;
            }
            let (w10, w14, w18, w1c) = (p.base.word(0x10), p.base.word(0x14), p.base.word(0x18), p.base.word(0x1c));
            let _ = a_owner;
            for oj in 0..nb {
                let Some(q) = Self::prim(mgr, b_owner, oj) else { continue };
                let g = q.base.word(0xc);
                if g & 0x10 != 0 || g & 2 != 0 || g & 0x80 != 0 {
                    continue;
                }
                if w10 & q.base.word(0x14) == 0 && w14 & q.base.word(0x10) == 0 && w18 & q.base.word(0x1c) == 0 && w1c & q.base.word(0x18) == 0 {
                    continue;
                }
                // vtbl+0x10 Translate(+off) / test / Translate(-off) on the other prim
                // (applied to a copy: mgr is shared here; the C++ +off/-off round trip may leave
                // float drift in the stored prim, not reproduced)
                let mut q = q.clone();
                q.Translate(off);
                let mut o = EFCheckCollisionOut::default();
                let r = self.tests.CheckObjPrimCollision(&p, &input, &mut o, &q);
                if r & 3 == 0 {
                    continue;
                }
                if r & 2 != 0 {
                    self.stages[s].contacts.insert(0, EFContactReport { n: o.normal, other: idb, prim: Some((b_owner, oj)), env: None });
                    if let Some(j) = os {
                        let opid = cps[pi].1;
                        self.stages[j].contacts.insert(0, EFContactReport { n: neg(o.normal), other: ida, prim: Some((Self::owner(mgr, opid), prim as usize)), env: None });
                    }
                    let mut depth = o.depth;
                    let mut f17 = depth;
                    if 0.001 < depth {
                        if let Some(j) = os {
                            if b_prio <= a_prio {
                                if b_prio == a_prio {
                                    let sum = a_w + b_w;
                                    f17 = (depth * a_w) / sum;
                                    depth = depth * (b_w / sum);
                                } else {
                                    depth = 0.0;
                                }
                                let st = &mut self.stages[j];
                                st.pushes.insert(0, EFPushReport { v: neg(o.normal), depth: f17, prio: a_prio as i32, weight: a_w });
                                st.push_count += 1;
                            }
                        }
                        if depth != 0.0 {
                            let st = &mut self.stages[s];
                            st.pushes.insert(0, EFPushReport { v: o.normal, depth, prio: b_prio as i32, weight: b_w });
                            st.push_count += 1;
                        }
                    }
                }
                let x = dz(o.normal, input.motion);
                if 0x3b7f_ffff < (x.to_bits() & 0x7f80_0000) && x.to_bits() & 0x8000_0000 != 0 {
                    let t = if r & 2 == 0 { o.t + t0 } else { t0 };
                    let rep = SysReport {
                        base: EFCollisionReport { stage: s as u32, prim: pi as u32, other: idb, other_prim: oj as u32, t },
                        other_stage: os,
                        other: Some(d),
                        normal: o.normal,
                        env: None,
                        overlap: r & 2 != 0,
                    };
                    if let Some(j) = os {
                        if t < self.stages[j].min_t {
                            self.stages[j].min_t = t;
                        }
                    }
                    let me = self.insert_report(rep, s);
                    if self.arena[me].base.t < self.stages[s].min_t {
                        self.stages[s].min_t = self.arena[me].base.t;
                    }
                }
            }
        }
    }

    /// EFPhysic::CheckNoPushCollisions(stage, dyn) (0x001d9940): overlap test (tmax 0, no motion)
    /// of the prims where either side has flag 2 (no push) or 0x80, notify only.
    pub fn EFPhysic_CheckNoPushCollisions(&mut self, mgr: &EFPhysicMgr, ups: &EFUpdatePhysicStruct, s: usize, d: (usize, usize)) {
        let ida = Self::pid(ups, self.stages[s].dyn_ref);
        let idb = Self::pid(ups, d);
        let (Some(pa), Some(pb)) = (mgr.Physic(ida), mgr.Physic(idb)) else { return };
        let (a_owner, b_owner, va, vb) = (pa.owner, pb.owner, pa.velocity, pb.velocity);
        let input = EFCheckCollisionIn::default();
        let na = self.prim_count(mgr, ida);
        let nb = self.prim_count(mgr, idb);
        for i in 0..na {
            let Some(p) = Self::prim(mgr, a_owner, i).cloned() else { continue };
            let f = p.base.word(0xc);
            if f & 0x10 != 0 {
                continue;
            }
            let (w10, w14, w18, w1c) = (p.base.word(0x10), p.base.word(0x14), p.base.word(0x18), p.base.word(0x1c));
            for j in 0..nb {
                let Some(q) = Self::prim(mgr, b_owner, j).cloned() else { continue };
                let g = q.base.word(0xc);
                if g & 0x10 != 0 || !(f & 2 != 0 || g & 2 != 0 || f & 0x80 != 0 || g & 0x80 != 0) {
                    continue;
                }
                if w10 & q.base.word(0x14) == 0 && w14 & q.base.word(0x10) == 0 && w18 & q.base.word(0x1c) == 0 && w1c & q.base.word(0x18) == 0 {
                    continue;
                }
                let mut o = EFCheckCollisionOut::default();
                let r = self.tests.CheckObjPrimCollision(&p, &input, &mut o, &q);
                if r & 2 != 0 {
                    // quirk: report+0x10 = index in this physic's prim array, read back as an index
                    // into the stage's +0x40 list (same order when the physic has no children)
                    let rep = SysReport {
                        base: EFCollisionReport { stage: s as u32, prim: i as u32, other: idb, other_prim: j as u32, t: 0.0 },
                        other: Some(d),
                        normal: o.normal,
                        ..Default::default()
                    };
                    self.Report_CheckNotify(mgr, ups, &rep, va, vb);
                }
            }
        }
    }

    /// EFCollisionReport::CheckNotify(p0, p1) (report.rs) on a system report: resolves the two
    /// prims / instances and queues the EFInstance::Notify calls in `notifies`.
    pub fn Report_CheckNotify(&mut self, mgr: &EFPhysicMgr, ups: &EFUpdatePhysicStruct, r: &SysReport, p0: V3, p1: V3) {
        let Some(od) = r.other else { return };
        let st = &self.stages[r.base.stage as usize];
        let (ap, auid) = match st.col_prims.get(r.base.prim as usize) {
            Some(&(pi, pid)) => {
                let o = Self::owner(mgr, pid);
                // quirk: a NULL +0x40 prim is dereferenced by the C++; default here
                (Self::report_prim(Self::prim(mgr, o, pi as usize)).unwrap_or_default(), o)
            }
            None => (ReportPrim::default(), 0),
        };
        let buid = Self::owner(mgr, Self::pid(ups, od));
        let bp = Self::report_prim(Self::prim(mgr, buid, r.base.other_prim as usize));
        let out = EFCollisionReport::CheckNotify((auid, ap), Some((buid, bp)), p0, p1);
        // P23 R44 debug: GAME_RPDBG=1 prints notifies to/from the EP6/L02 arcs
        if !out.is_empty() && std::env::var_os("GAME_RPDBG").is_some() && [auid, buid].iter().any(|u| (0x2a00_0180..0x2a00_01a0).contains(u)) {
            let pa = st.col_prims.get(r.base.prim as usize).and_then(|&(pi, _)| Self::prim(mgr, auid, pi as usize)).map(|p| p.base.center);
            let pb = Self::prim(mgr, buid, r.base.other_prim as usize).map(|p| (p.base.center, p.base.kind, p.base.radius));
            eprintln!("RPDBG a {auid:#x} prim {} {pa:.2?} b {buid:#x} prim {} {pb:.2?} t {:.3}", r.base.prim, r.base.other_prim, r.base.t);
        }
        self.notifies.extend(out);
    }

    /// EFPhysic::CollisionUpdate(stage, data, dt) (0x001da8a0): ColUpdate modules, contact
    /// report, stage re-activation, floor / slope / object-floor state bits.
    #[allow(clippy::too_many_arguments)]
    pub fn EFPhysic_CollisionUpdate(&mut self, mgr: &mut EFPhysicMgr, ups: &EFUpdatePhysicStruct, world: &dyn PhysicWorld, id: PhysId, stage: Option<usize>, d: &EFCollisionData, dt: f32) {
        // EFColDMData {+0 t, +4 normal, +0x1c other, +0x20 prim, +0x24 env, +0x28 contacts}
        let other = mgr.Physic(d.other).map(|o| (o.velocity, o.flags, o.friction));
        let dm = EFColDMData {
            normal: d.normal,
            other,
            env_friction: d.env.as_ref().map(|e| f32::from_bits(e.mat[3])),
            contacts: d
                .contacts
                .iter()
                .map(|c| (c.env.as_ref().map(|e| f32::from_bits(e.mat[3])), mgr.Physic(c.other).map(|o| (o.flags, o.friction))))
                .collect(),
        };
        let dp = stage.map(|s| Self::dyn_of(ups, self.stages[s].dyn_ref).clone()).unwrap_or_default();
        mgr.ColUpdateModules(id, &DMStage::seam(&dp), &dm, dt, world);
        if let Some(s) = stage {
            let vel = Self::vel(mgr, id);
            let st = &mut self.stages[s];
            if d.add_contact {
                st.contacts.insert(0, EFContactReport { n: d.normal, other: d.other, prim: d.other_prim, env: d.env.clone() });
            }
            let l = dz(vel, vel).sqrt();
            st.step = if l == 0.0 { dt } else { 0.001 / l };
            st.active = true;
        }
        let other_flags = mgr.Physic(d.other).map(|o| o.flags);
        let other_prim = d.other_prim.and_then(|(ow, i)| Self::prim(mgr, ow, i).cloned());
        let rng = self.rand.clone();
        let Some(p) = mgr.PhysicMut(id) else { return };
        if p.floor_cos < d.normal[1] {
            if d.other == 0 || p.flags & flags::COLLIDE == 0 {
                p.state |= 1;
                p.floor = d.env.as_ref().map(|e| (e.mat, e.normal));
            } else if other_flags.unwrap_or(0) & 0x60_0000 == 0 {
                let mut f = [0.0f32; 3];
                let ny = d.normal[1];
                if 0.995 < ny {
                    // other prim vtbl+0x2c GetEscapeNoPF(instance LTM pos, &f, &velocity +0x28)
                    if let Some(op) = &other_prim {
                        let mut rand = || {
                            rng.lock().unwrap().rand() as u32
                        };
                        op.GetEscapeNoPF(p.frame.pos, &mut f, p.velocity, &mut rand);
                    }
                } else {
                    let t = sc(d.normal, ny);
                    f = super::rwm::RwV3dNormalize([t[0], t[1] - 1.0, t[2]]);
                }
                let x = dz(f, p.velocity);
                if x < 2.0 && -0.5 < x {
                    if x < 1.5 {
                        p.velocity = ad(p.velocity, sc(f, 0.5));
                    } else {
                        p.velocity = ad(p.velocity, sc(f, 2.0 - x));
                    }
                }
                p.state |= 8;
            } else {
                p.state |= 4;
            }
        }
    }
}

impl CollisionSystem for EFCollisionSystem {
    fn BeginFrame(&mut self) {
        self.arena.clear();
    }
    fn EndFrame(&mut self) {
        self.arena.clear();
    }
    /// EFPhysicMgr::CollisionStage, COLLIDE branch: Init; Update; PostUpdate; Terminate.
    fn Solve(&mut self, mgr: &mut EFPhysicMgr, ups: &mut EFUpdatePhysicStruct, pass: usize, index: usize, world: &dyn PhysicWorld) {
        self.SyncModels(mgr, world);
        self.Init(mgr, ups, pass, index);
        self.Update(mgr, ups, world);
        self.PostUpdate(mgr, ups, world);
        self.Terminate(ups);
    }
    fn PrimRayIntersect(&mut self, phys: &EFPhysic, prim: usize, rin: &EFRayIntersectIn, t: &mut f32, normal: &mut V3) -> bool {
        let Some(Some(p)) = phys.model.prims.get(prim) else { return false };
        let mut out = EFRayIntersectOut::default();
        let r = p.RayIntersect(&rin.map, &mut out, t);
        if r != 0 {
            *normal = out.hit.normal;
        }
        r != 0
    }
    fn CheckSphereSphere(&mut self, origin_rel: V3, delta: V3, radius: f32) -> (u32, f32) {
        let i = EFCheckSphereSphereCollisionIn { center: origin_rel, motion: delta, radius };
        let mut o = EFCheckSphereSphereCollisionOut::default();
        let r = EFObjCollisionPrim::CheckSphereSphereCollision(&i, &mut o);
        (r, o.t)
    }
}

/// EFCollisionModel::ValidatePrimaryPrimTrans(stage) on EFPhysic+0x54 including the
/// RwFrameTranslate / FlagTranslatedChild side effects (model.rs only reports the request).
pub fn ValidatePhysic(p: &mut EFPhysic, frame: u32, stage: &EFCollisionStage) {
    let m = &mut p.model;
    let before = m.bsphere_world;
    let moved = m.ValidatePrimaryPrimTrans(frame, |pos, d, m1, m2| stage.ValidateTranslationEnv(pos, d, m1, m2));
    m.translated = false;
    if moved {
        // recover v from model+0x1c
        let v = sb(m.bsphere_world, before);
        for pr in m.prims.iter_mut().flatten() {
            for k in 0..3 {
                pr.base.ltm[3][k] += v[k];
            }
        }
        if let Some(Some(pr)) = m.prims.get_mut(m.primary as usize) {
            pr.PrecalcPrimData();
            m.primary_pos = pr.base.center;
        }
        p.frame.Translate(v, rwCOMBINEPOSTCONCAT);
        p.moved = true;
    }
}

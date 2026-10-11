//! PLMgr_BC / PLMgr (_gpoPLParticleMgr): Classes/PL/PLMgr_BC.cpp, PLMgr.cpp.

#![allow(non_snake_case)]

use super::*;
use crate::engine::camera::EFGraphicsMgrCameras;
use crate::engine::dt::descriptors::{DTEmitterContainerDescriptor, DTParticleContainerDescriptor};
use crate::engine::dt::property::DTStreamable_LoadFromStream;
use crate::engine::dt::stream::DTBasePersistStream;

/// PLMgr_BC (DTStreamable; the instanciator of classes 0x29 PLMgrType and 0x25 PLMgrEmitter returns
/// the manager itself, PLMgr_BC::Instanciate).
#[derive(Debug, Clone)]
pub struct PLMgr_BC {
    /// +0xc number of LoadFromStream calls (1st = types, 2nd = emitters)
    pub load_count: i32,
    /// +0x18 nb types, +0x1c active particles, +0x20 nb emitters, +0x24, +0x28 launches
    pub nb_types: i32,
    pub nb_particles: i32,
    pub nb_emitters: i32,
    pub nb_24: i32,
    pub nb_launched: i32,
    /// +0x2c PLLinkList<PLType>, index 0 = head
    pub types: Vec<PLType>,
    /// +0x14 / +0x38 particle array + free list
    pub pool: PLParticlePool,
    /// +0x44 next created emitter id, +0x48 next created type id
    pub next_emitter_id: u32,
    pub next_type_id: u32,
    /// RwTexDictionaryGetCurrent() replacement: names of the textures of the current dictionary
    /// (None = no dictionary). Set by the coordinator from the level TXD before loading.
    pub textures: Option<Vec<String>>,
    /// EFNotificationMgr messages produced by UpdateClippingState (drained by the coordinator)
    pub notifications: Vec<PLNotification>,
    /// EFBillboardMgr +0 max billboards (only SetMaxNumBillboards is ported, see api.rs)
    pub max_billboards: u32,
}

impl Default for PLMgr_BC {
    fn default() -> Self {
        Self::new()
    }
}

impl PLMgr_BC {
    /// PLMgr_BC::PLMgr_BC (0x0020e3d0)
    pub fn new() -> Self {
        PLMgr_BC {
            load_count: 0,
            nb_types: 0,
            nb_particles: 0,
            nb_emitters: 0,
            nb_24: 0,
            nb_launched: 0,
            types: Vec::new(),
            pool: PLParticlePool::new(),
            next_emitter_id: 0x2e80_0001,
            next_type_id: 0x2d80_0001,
            textures: None,
            notifications: Vec::new(),
            max_billboards: 500,
        }
    }

    /// PLMgr_BC::Update (0x0020e560). `paused` = _gpoWorldMgr+0x14.
    pub fn Update(&mut self, dt: f32, paused: bool, cams: &mut EFGraphicsMgrCameras) {
        self.nb_particles = 0;
        self.nb_24 = 0;
        self.nb_launched = 0;
        for t in self.types.iter_mut() {
            PLType_Update(t, dt, paused, &mut self.pool, cams, &mut self.notifications);
            self.nb_particles += t.d.nb_active;
            self.nb_24 += t.d.nb_19c;
            self.nb_launched += t.d.nb_launched;
        }
    }

    /// PLMgr_BC::GetType(int) (0x0020e600): index in `types`
    pub fn GetType(&self, id: i32) -> Option<usize> {
        self.types.iter().position(|t| t.d.id == id as u32)
    }
    /// PLMgr_BC::GetType(const char*) (0x0020e660): strncmp(name, 0x1f)
    pub fn GetTypeByName(&self, name: &str) -> Option<usize> {
        self.types.iter().position(|t| emitter::strncmp_eq(&t.d.name, name, 0x1f))
    }
    /// PLMgr_BC::GetEmitter(int) (0x0020e6d0): (type index, emitter index)
    pub fn GetEmitter(&self, id: i32) -> Option<(usize, usize)> {
        for (ti, t) in self.types.iter().enumerate() {
            if let Some(ei) = t.emitters.iter().position(|e| e.id == id as u32) {
                return Some((ti, ei));
            }
        }
        None
    }
    /// PLMgr_BC::GetEmitter(const char*, uint objUid) (0x0020e750)
    pub fn GetEmitterByName(&self, name: &str, obj: u32) -> Option<(usize, usize)> {
        for (ti, t) in self.types.iter().enumerate() {
            if let Some(ei) = t.emitters.iter().position(|e| e.object_uid == obj && emitter::strncmp_eq(&e.name, name, 0x20)) {
                return Some((ti, ei));
            }
        }
        None
    }
    pub fn emitter_mut(&mut self, id: i32) -> Option<&mut PLEmitter> {
        let (t, e) = self.GetEmitter(id)?;
        Some(&mut self.types[t].emitters[e])
    }

    /// PLMgr_BC::ParticleTypeCreate (0x0020e800)
    pub fn ParticleTypeCreate(&mut self) -> u32 {
        let mut t = PLType::PLType();
        t.d.id = self.next_type_id;
        self.next_type_id = self.next_type_id.wrapping_add(1);
        let id = t.d.id;
        self.types.insert(0, t);
        self.nb_types += 1;
        id
    }
    /// PLMgr_BC::ParticleTypeDelete (0x0020e880): 1 = not found. ~PLType_BC deletes its emitters
    /// (quirk: their particles are released by ~PLEmitter_BC).
    pub fn ParticleTypeDelete(&mut self, id: i32) -> u32 {
        let Some(i) = self.GetType(id) else { return 1 };
        let mut t = self.types.remove(i);
        for mut e in t.emitters.drain(..) {
            e.ReleaseParticlesFromList(&mut self.pool);
            self.nb_emitters -= 1;
        }
        self.nb_types -= 1;
        0
    }
    /// PLMgr_BC::ParticleEmitterCreate (0x0020e940): 1 = unknown type. No frame is created.
    pub fn ParticleEmitterCreate(&mut self, type_id: i32, owner: Option<u32>) -> u32 {
        let Some(ti) = self.GetType(type_id) else { return 1 };
        let mut e = PLEmitter::PLEmitter();
        e.id = self.next_emitter_id;
        self.next_emitter_id = self.next_emitter_id.wrapping_add(1);
        e.owner = owner;
        let id = e.id;
        self.types[ti].emitters.insert(0, e);
        self.nb_emitters += 1;
        id
    }
    /// PLMgr_BC::ParticleEmitterDelete (0x0020e9f0): 2 = not found
    pub fn ParticleEmitterDelete(&mut self, id: i32) -> u32 {
        let Some((t, e)) = self.GetEmitter(id) else { return 2 };
        let mut e = self.types[t].emitters.remove(e);
        e.ReleaseParticlesFromList(&mut self.pool);
        self.nb_emitters -= 1;
        0
    }
    /// PLMgr_BC::ParticleEmitterSetType (0x0020ea70): 1 = no type, 2 = no emitter
    pub fn ParticleEmitterSetType(&mut self, id: i32, type_id: i32) -> u32 {
        let Some(nt) = self.GetType(type_id) else { return 1 };
        let Some((t, e)) = self.GetEmitter(id) else { return 2 };
        let em = self.types[t].emitters.remove(e);
        self.types[nt].emitters.insert(0, em);
        0
    }
    /// PLMgr_BC::Terminate (0x0020eb00)
    pub fn Terminate(&mut self) {
        let types: Vec<PLType> = self.types.drain(..).collect();
        for mut t in types {
            for mut e in t.emitters.drain(..) {
                e.ReleaseParticlesFromList(&mut self.pool);
                self.nb_emitters -= 1;
            }
            self.nb_types -= 1;
        }
        self.load_count = 0;
    }

    /// PLMgr_BC::LoadFromStream (0x0020eba0): the pack holds one 0x29 entry (types) then one 0x25
    /// entry (emitters); the call counter, not the class id, selects which is parsed. -2 = error
    /// or third call.
    pub fn LoadFromStream(&mut self, s: &mut dyn DTBasePersistStream) -> i32 {
        self.load_count += 1;
        if self.load_count == 1 {
            self.LoadTypes(s)
        } else if self.load_count == 2 {
            let mut d = DTEmitterContainerDescriptor::default();
            crate::engine::dt::property::DTObject::LoadFromStream(&mut d, s);
            self.LoadEmitters(&d, None)
        } else {
            -2
        }
    }

    /// PLMgr_BC::LoadTypes (0x0020ec40): DTParticleContainerDescriptor, PARTTYPES = NB records of
    /// 0xd8 bytes; type ids 0x2d000000 | index; AddFirst (list ends up reversed).
    pub fn LoadTypes(&mut self, s: &mut dyn DTBasePersistStream) -> i32 {
        let mut d = DTParticleContainerDescriptor::default();
        DTStreamable_LoadFromStream(&mut d, s);
        self.LoadTypesFromDescriptor(&d)
    }
    pub fn LoadTypesFromDescriptor(&mut self, d: &DTParticleContainerDescriptor) -> i32 {
        // replacement: NB == 0 would divide by zero in the original
        if d.nb <= 0 || d.partype_size as i32 / d.nb != 0xd8 {
            return -2;
        }
        let dict = self.textures.clone();
        for i in 0..d.nb as usize {
            let mut t = PLType::PLType();
            t.d.id = (i as u32 & 0xff_ffff) | 0x2d00_0000;
            let rec = d.parttypes.data.get(i * 0xd8..(i + 1) * 0xd8).unwrap_or(&[]);
            // quirk: the Initialize result is ignored
            t.d.Initialize(rec, 0xd8, dict.as_deref());
            self.types.insert(0, t);
            self.nb_types += 1;
        }
        0
    }

    /// PLMgr_BC::LoadEmitters (0x0020ed80): one emitter per DTEmitterDescriptor; with an object,
    /// the emitter frame becomes a child of the object's frame BONE_ID (missing frame -> -2).
    /// quirk: the first emitter whose type is not found stops the load (-2), later ones are lost.
    pub fn LoadEmitters(&mut self, d: &DTEmitterContainerDescriptor, obj: Option<(u32, &dyn PLWorld)>) -> i32 {
        let n = d.nb_emi.max(0) as usize;
        for ed in d.emitters.iter().take(n) {
            let mut e = PLEmitter::PLEmitter();
            e.id = (self.nb_emitters as u32 & 0xff_ffff) | 0x2e00_0000;
            let local = DTMatrix { right: ed.look_right, up: ed.look_up, at: ed.look_at, pos: ed.pos };
            let mut frame = PLFrame { attach: PLFrameAttach::Own, local, ltm: local };
            if let Some((uid, w)) = obj {
                let Some(parent) = w.GetObjectFrame(uid, ed.bone_id as u32) else { return -2 };
                frame.attach = PLFrameAttach::Child { obj: uid, bone: ed.bone_id as u32 };
                frame.ltm = mat_mul(&local, &parent);
            }
            e.frame = Some(frame);
            match e.Initialize(&ed.partemi.data, ed.partemi_size, obj.map(|o| o.0), &self.types) {
                Err(_) => return -2,
                Ok(ti) => self.types[ti].emitters.insert(0, e),
            }
            self.nb_emitters += 1;
        }
        0
    }

    /// PLMgr_BC::DestroyAttachedEmitters (0x0020efd0)
    pub fn DestroyAttachedEmitters(&mut self, obj: u32) -> i32 {
        let mut n = 0;
        for t in self.types.iter_mut() {
            let mut i = 0;
            while i < t.emitters.len() {
                if t.emitters[i].object_uid == obj {
                    let mut e = t.emitters.remove(i);
                    e.ReleaseParticlesFromList(&mut self.pool);
                    n += 1;
                    self.nb_emitters -= 1;
                } else {
                    i += 1;
                }
            }
        }
        n
    }

    /// Replacement for RwFrameGetLTM: refreshes every emitter frame LTM from the world (call
    /// before Update each frame). Frames whose parent no longer resolves keep their last LTM.
    pub fn SyncFrames(&mut self, w: &dyn PLWorld) {
        for t in self.types.iter_mut() {
            for e in t.emitters.iter_mut() {
                let Some(f) = e.frame.as_mut() else { continue };
                let ltm = match f.attach {
                    PLFrameAttach::Own => Some(f.local),
                    PLFrameAttach::Child { obj, bone } => w.GetObjectFrame(obj, bone).map(|p| mat_mul(&f.local, &p)),
                    PLFrameAttach::ObjectFrame { obj, bone } => w.GetObjectFrame(obj, bone),
                    PLFrameAttach::PosData { uid } => w.GetObjectPosDataFrame(uid),
                };
                if let Some(l) = ltm {
                    f.ltm = l;
                }
            }
        }
    }

    /// PLMgr_BC::Render (0x00274480): every emitter not (clipped && flag 0x40), in list order
    /// (quirk: the depth sort loop in the original is empty, so no sort). Camera-space
    /// transform, fog state save/restore and Begin/EndRender are backend work.
    pub fn Render(&mut self) -> PLDrawList {
        let mut dl = PLDrawList::default();
        for t in self.types.iter_mut() {
            for e in t.emitters.iter_mut() {
                if e.iflags & IF_EMITTER_CLIPPED != 0 && e.flags & 0x40 != 0 {
                    continue;
                }
                if e.frame.is_none() {
                    continue;
                }
                if let Some(b) = e.Render(&t.d, &self.pool) {
                    dl.batches.push(b);
                }
            }
        }
        dl
    }
}

/// PLType_BC::Update (0x002101f0)
pub fn PLType_Update(t: &mut PLType, dt: f32, paused: bool, pool: &mut PLParticlePool, cams: &mut EFGraphicsMgrCameras, notes: &mut Vec<PLNotification>) {
    let d = &mut t.d;
    if d.flags & 0x10 != 0 {
        if !paused {
            d.flags &= !8;
        } else {
            d.flags |= 8;
        }
    }
    d.nb_active = 0;
    d.nb_launched = 0;
    d.nb_19c = 0;
    for e in t.emitters.iter_mut() {
        if d.flags & 8 == 0 {
            e.UpdateClippingState(pool, cams, notes);
            let run = e.iflags & IF_EMITTER_CLIPPED == 0 || e.flags & 0x80 == 0;
            if run {
                d.nb_launched += e.LaunchParticle(d, pool, dt, paused) as i32;
                d.nb_active += e.UpdatePhysic(d, pool, dt);
            } else {
                d.nb_active += e.GetNbActivePart();
            }
        } else {
            d.nb_active += e.GetNbActivePart();
        }
        if e.started {
            d.nb_launched += 1;
        }
    }
}

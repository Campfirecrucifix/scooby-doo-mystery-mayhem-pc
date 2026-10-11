//! WP-J2: runtime integration part 2 — animation, physics, audio, camera and light managers
//! wired into the runtime host and the EFWorld::Update frame slots.
//!
//! Included from runtime.rs (`#[path]` submodule) so engine/mod.rs needs no new line.
//!
//! What lives here:
//! - [`Subsystems`]: the per-level EFAnimationMgr of every game object (EFGameObject +0x4c),
//!   _gpoPhysicMgr, _gpoNewAudioMgr, the EFGraphicsMgr camera slots / EFLightMgr while the logic
//!   runs (swapped in from EngineRuntime around EFGameLogicMgr::Update), and the input actions.
//! - the APIanim_* / APIphysic_* / APInewSound_* / APIcamera_* / APIlight_* host bindings
//!   (called from runtime.rs' `impl Host for RuntimeHost`).
//! - [`RuntimePhysicWorld`]: `PhysicWorld` over the runtime's object frames, input and camera 0.

#![allow(non_snake_case)]

use super::{RuntimeWorld, SharedParticles};
use crate::engine::anim::api as aapi;
use crate::engine::anim::mgr::EFAnimationMgr;
use crate::engine::anim::nodes::EFHAnimNode;
use crate::engine::anim::res::EFHAnimRes;
use crate::engine::anim::AnimInstanciator;
use crate::engine::audio::handlers::{AudioInstanciator, CreateSoundInstances, EFAudioEnvDescriptor};
use crate::engine::audio::mgr::{EFNewAudioMgr, GeometryProvider, SharedRand};
use crate::engine::audio::sound3d::{Frame, Geometry};
use crate::engine::camera::{DTMatrix, EFGraphicsMgrCameras};
use crate::engine::dt::descriptors::{DT3dObjectDescriptor, DTAnimDescriptor, EFIDescriptor};
use crate::engine::level::EFGameObject;
use crate::engine::light::EFLightMgr_BC;
use crate::engine::pack::{DTDynamicInstanciator, DTStreamable, EFRessourcesMgr};
use crate::engine::physic::api as papi;
use crate::engine::physic::mgr::EFPhysicMgr;
use crate::engine::physic::system::EFCollisionSystem;
use crate::engine::physic::module::{PhysicNotify, PhysicWorld};
use crate::engine::physic::rwm::RwMatrix;
use crate::engine::{Mem, Ptr};
use std::collections::HashMap;
use std::sync::Arc;

// ---------------------------------------------------------------------------------------------
// level instanciator

/// The pack instanciator of the runtime: WP-D resources (EFAnimRes/EFHAnimRes...) parsed by
/// AnimInstanciator, EF_AUDIOENV_DESC (0x3c) by AudioInstanciator; every other class stays the
/// RawStreamable the J1 registration code reads.
#[derive(Default)]
pub struct RuntimeInstanciator {
    pub anim: AnimInstanciator,
    pub audio: AudioInstanciator,
}

impl DTDynamicInstanciator for RuntimeInstanciator {
    fn Instanciate(&mut self, class_id: u32) -> Option<Box<dyn DTStreamable>> {
        if class_id == 0x3c {
            self.audio.Instanciate(class_id)
        } else {
            self.anim.Instanciate(class_id)
        }
    }
}

// ---------------------------------------------------------------------------------------------
// input

/// EFInputMgr::InputActionGet(config, action) analog values as seen by the physics modules
/// (EFDMAnalogMove/AnalogRotation). The Bevy input layer (or a test script) writes them.
/// `stick` = value returned for every action without an explicit entry (scripted pad).
#[derive(Debug, Clone, Default)]
pub struct RuntimeInput {
    pub actions: HashMap<(u32, u32), [f32; 2]>,
    pub stick: Option<[f32; 2]>,
}

// ---------------------------------------------------------------------------------------------
// API call log (debug / tests)

#[derive(Debug, Clone, Default)]
pub struct ApiLog {
    /// (obj, anim, flags, started)
    pub anim_play: Vec<(u32, u32, u32, bool)>,
    /// (obj, id, return code: 0 ok / 1 unknown id / 2 no handle)
    pub sound_play: Vec<(u32, u32, u32)>,
    /// (obj, list, physic id)
    pub physic_create: Vec<(u32, u32, u32)>,
    /// (obj, kind, handle)
    pub dm_add: Vec<(u32, u32, u32)>,
    /// (obj, camera id)
    pub camera_create: Vec<(u32, u32)>,
    /// anim/physics notifications delivered (msg, key)
    pub notifications: Vec<(u32, u32)>,
}

// ---------------------------------------------------------------------------------------------
// subsystems

/// One game object's animation binding: its EF3dObj instance uid (EFHAnimNode +0x2c target)
/// and the DT_ANIM_NODE_DESCR (0x4b) instances in descriptor order: (anim id, EFHAnimRes uid).
#[derive(Debug, Clone, Default)]
pub struct AnimBinding {
    pub obj3d: u32,
    pub anims: Vec<(u32, u32)>,
    /// N2: per anim, the REFPUID of the DT_ANIM_NODE_DESCR (EFGameObject::ResolveRefPUID: the
    /// 3D descriptor with that PUID is the EF3dObj the EFHAnimNode drives)
    pub refs: Vec<u32>,
}

/// K6: a latent template's per-uid data saved by [`Subsystems::CloneInstances`]
#[derive(Clone)]
pub struct LatentTpl {
    obj3d: Option<(u32, bool)>,
    obj3d_list: Option<Vec<(u32, u32)>>,
    anim_bind: Option<AnimBinding>,
}

pub struct Subsystems {
    /// EFGameObject +0x4c EFAnimationMgr, per game object uid (created at level load from the
    /// GLO's anim descriptors; EFGameObject::CreateInstances)
    pub anim: HashMap<u32, EFAnimationMgr>,
    pub anim_bind: HashMap<u32, AnimBinding>,
    /// EFRessourceFactory: HAnim resources by uid
    pub hanim: HashMap<u32, Arc<EFHAnimRes>>,
    /// _gpoPhysicMgr
    pub physic: EFPhysicMgr,
    /// J3: the EFCollisionSystem driven by EFPhysicMgr::CollisionStage (was NoCollision in J2;
    /// without it the heroes free-fall, heroDoTestForOnFloor raises event_ACTION_DEAD and the
    /// hero never reaches a state whose input context enables the Tome button)
    pub collision: EFCollisionSystem,
    /// J3: EFInstance::Notify(0x17 / 3) collision notifications drained after EFPhysicMgr::Update
    /// J3: EF3dObjRes uid -> its clump collision primitives
    pub res_collision: HashMap<u32, crate::engine::rw::clump::ApClumpCollision>,
    /// J10: per EF3dObjRes uid: (HAnim node ids, node index -> joint index of the parent frame)
    /// for EFHAnimNode::ScaleCompensate (see [`ScaleCompensateParents`])
    pub hanim_parent: HashMap<u32, Arc<(Vec<i32>, Vec<Option<u32>>)>>,
    /// J3: game object uid -> (EF3dObj resource uid, HAnim hierarchy = EF3dObj+0x2c & 4)
    pub obj3d: HashMap<u32, (u32, bool)>,
    /// J9: DT_HOTSPOT_DESCR of hot-spot game objects (EFGameObject::InitHotspot)
    pub hotspot: HashMap<u32, crate::engine::dt::descriptors::DTHotSpotDescriptor>,
    /// J8: game object uid -> EFGameObject+0x174: (descriptor index n, EF3dObj resource uid) of
    /// every 3D object descriptor (APIhud_Window3DSetObj picks +0x174[n])
    pub obj3d_list: HashMap<u32, Vec<(u32, u32)>>,
    /// N2: anim-node `obj` keys of the EF3dObjs other than `obj3d` ((game object uid, 3D res
    /// puid) -> key; key -> game object uid): objects with several animated 3D descriptors
    /// (pause-menu panel 0x2a100006: ~45 EF3dObjs, each with its own anims)
    pub anim_obj_keys: HashMap<(u32, u32), u32>,
    pub anim_obj_owner: HashMap<u32, u32>,
    /// K6: latent templates whose uid was later reused by a launched instance (see CloneInstances)
    pub latent_tpl: HashMap<u32, LatentTpl>,
    /// J3: EFAnimNode events (node obj uid, event) drained by UpdateAnims, delivered by the runtime
    pub anim_events: Vec<(u32, crate::engine::anim::nodes::AnimEvent)>,
    pub col_notify: Vec<(u32, u32, crate::engine::physic::report::CollisionNotifyMsg)>,
    /// _gpoNewAudioMgr (draws from the shared libc rand)
    pub audio: EFNewAudioMgr,
    /// (bank, sample) -> seconds (SPU playback end, replaces the SPU "key off" status)
    pub sample_len: HashMap<(u32, u32), f32>,
    /// EFGraphicsMgr camera slots / EFLightMgr while the logic runs (swapped with EngineRuntime)
    pub cameras: EFGraphicsMgrCameras,
    pub lights: EFLightMgr_BC,
    pub input: RuntimeInput,
    pub log: ApiLog,
}

impl Subsystems {
    pub fn new(rng: SharedRand) -> Self {
        Subsystems {
            anim: HashMap::new(),
            anim_bind: HashMap::new(),
            hanim: HashMap::new(),
            physic: EFPhysicMgr::new(),
            collision: EFCollisionSystem { rand: rng.clone(), ..EFCollisionSystem::new() },
            col_notify: Vec::new(),
            anim_events: Vec::new(),
            res_collision: HashMap::new(),
            hanim_parent: HashMap::new(),
            obj3d: HashMap::new(),
            hotspot: HashMap::new(),
            obj3d_list: HashMap::new(),
            anim_obj_keys: HashMap::new(),
            anim_obj_owner: HashMap::new(),
            latent_tpl: HashMap::new(),
            audio: EFNewAudioMgr::with_rng(rng),
            sample_len: HashMap::new(),
            cameras: EFGraphicsMgrCameras::default(),
            lights: EFLightMgr_BC::new(),
            input: RuntimeInput::default(),
            log: ApiLog::default(),
        }
    }

    /// Level part of the subsystems (EFRessourcesMgr::LoadLevel registrations + EFGameObject::
    /// CreateInstances for anims and sounds). `fs` reads the sound banks (SOUND_BankLoad).
    pub fn LoadLevel(&mut self, res: &mut EFRessourcesMgr, level_dir: &str, read: &mut dyn FnMut(&str) -> Option<Vec<u8>>) {
        let rng = self.audio.rng.clone();
        *self = Subsystems { input: std::mem::take(&mut self.input), ..Subsystems::new(rng) };
        for o in res.objects.iter_mut() {
            if let Some(r) = o.as_raw() {
                // EF_COLLISION_MAP -> EFPhysicMgr::AddMap
                if r.class_id == 0x24 {
                    if let Ok((m, _)) = crate::engine::physic::collision_map::EFCollisionMap::LoadFromRaw(&r.data) {
                        self.physic.AddMap(m);
                    }
                }
                // J3: EF3dObjRes clump collision primitives (ApClumpCollision plugin), used by
                // APIphysic::CreatePhysic -> EF3dObj::CreateCollisionPrimitives
                if r.class_id == 0x5b {
                    let mut g = crate::engine::gfx::EF3dObjRes::default();
                    if g.LoadPropertiesBinary(&r.data).is_ok() {
                        if let Some(c) = &g.clump {
                            // J10: EFHAnimNode::ScaleCompensate input of this EF3dObj
                            if let Some(t) = ScaleCompensateParents(c) {
                                self.hanim_parent.insert(g.res.uid, Arc::new(t));
                            }
                        }
                        if let Some(c) = g.clump.and_then(|c| c.collision) {
                            self.res_collision.insert(g.res.uid, c);
                        }
                    }
                }
                continue;
            }
            let Some(d) = o.as_dt() else { continue };
            let a = d.as_any();
            if let Some(h) = a.downcast_ref::<EFHAnimRes>() {
                self.hanim.insert(h.base.hdr.uid, Arc::new(h.clone()));
            } else if let Some(env) = a.downcast_ref::<EFAudioEnvDescriptor>() {
                env.Apply(&mut self.audio);
            }
        }
        // EF_AUDIOENV_DESC (collected by the AudioInstanciator sink at pack load; carried in the
        // "AUDIOENV" side file as 4 NUL-separated strings, see EngineRuntime::ReadLevel)
        if let Some((_, b)) = res.side_files.iter().find(|(p, _)| p == "AUDIOENV") {
            let s: Vec<String> = String::from_utf8_lossy(b).split(char::from(0)).map(str::to_string).collect();
            if s.len() == 4 {
                EFAudioEnvDescriptor { strings: [s[0].clone(), s[1].clone(), s[2].clone(), s[3].clone()], ..Default::default() }.Apply(&mut self.audio);
            }
        }
        self.audio.LoadSoundBanks(level_dir, "EN", read);
        for (b, bank) in self.audio.banks.iter().enumerate() {
            let Some(bank) = bank else { continue };
            for (i, pcm) in bank.pcm.iter().enumerate() {
                let rate = bank.sample(i as u32).map_or(0, |s| s.sampling_rate);
                if let (Some(p), true) = (pcm, rate > 0) {
                    self.sample_len.insert((b as u32, i as u32), p.samples.len() as f32 / rate as f32);
                }
            }
        }
        for go in &res.game_objects {
            self.CreateInstances(go);
        }
    }

    /// EFGameObject::CreateInstances, anim + sound part: the 3D object descriptor gives the
    /// EF3dObj, every DT_ANIM_NODE_DESCR an EFHAnimNode (EFAnimationMgr::AddAnim, anim id =
    /// descriptor instance uid, first one = mgr base), every sound descriptor an
    /// EFNewSoundMgr entry.
    pub fn CreateInstances(&mut self, go: &EFGameObject) {
        let mut b = AnimBinding::default();
        // anim id = index in OBJECT_DESCRIPTORS (the 3D descriptor is index 0, so base = 1)
        for (i, c) in go.glo.object_descriptors.objects.iter().enumerate() {
            let a = c.as_any();
            // J9: EFGameObject::InitHotspot (collision prim built at CreatePhysic, see physic_create)
            if let Some(h) = a.downcast_ref::<crate::engine::dt::descriptors::DTHotSpotDescriptor>() {
                self.hotspot.insert(go.uid, h.clone());
                continue;
            }
            if let Some(d) = a.downcast_ref::<EFIDescriptor<DT3dObjectDescriptor>>() {
                b.obj3d = go.uid;
                self.obj3d.insert(go.uid, (d.desc.puid as u32, false));
                self.obj3d_list.entry(go.uid).or_default().push((i as u32, d.desc.puid as u32));
            } else if let Some(d) = a.downcast_ref::<EFIDescriptor<DTAnimDescriptor>>() {
                b.anims.push((i as u32, d.desc.puid as u32));
                b.refs.push(d.desc.refpuid as u32);
            }
        }
        if !b.anims.is_empty() {
            // HAnim objects get the EF3dObj HAnim flag (4)
            if let Some(o) = self.obj3d.get_mut(&go.uid) {
                o.1 = true;
            }
            let base = b.anims[0].0;
            let count = b.anims.iter().map(|(id, _)| id.wrapping_sub(base) as usize + 1).max().unwrap_or(0);
            let mut m = EFAnimationMgr::new(base, count);
            m.owner_uid = go.uid;
            for (k, &(id, ruid)) in b.anims.iter().enumerate() {
                if let Some(r) = self.hanim.get(&ruid).cloned() {
                    let mut n = EFHAnimNode::new(r, id);
                    let (obj, res) = self.anim_target(go.uid, b.refs.get(k).copied());
                    n.obj = obj;
                    n.parent_joint = self.parent_joint_res(res);
                    m.AddAnim(n, id);
                }
            }
            self.anim.insert(go.uid, m);
            self.anim_bind.insert(go.uid, b);
        }
        CreateSoundInstances(&mut self.audio, go);
    }

    /// LaunchLatant's EFGameObject duplicate (EFWorld::LaunchLatant -> new EFGameObject from the
    /// latent's GLO -> CreateInstances): the anim part is rebuilt for `new_id` from the template's
    /// binding (same descriptors, fresh EFAnimationMgr/EFHAnimNodes). J4: before this, launched
    /// latents (tod_beam) had no EFAnimationMgr, so their anims never ended.
    /// deviation: the sound instances of the latent are not duplicated (the EP1L01 latents used
    /// by the capture play no sounds of their own).
    /// J10: EFHAnimNode::ScaleCompensate (0x00231690) lookup on the node's EF3dObj (`obj` = the
    /// game object owning the 3D object): channel-set bone -> node index (RpHAnimIDGetIndex for
    /// HAnim ids) -> pNodeInfo[i].pFrame -> parent RwFrame -> RpHAnimFrameGetID ->
    /// EF3dObj::GetJointIndex. None (C++ reads garbage) when the parent joint is not found.
    fn parent_joint_fn(&self, obj: u32) -> Option<crate::engine::anim::nodes::ParentJointFn> {
        self.parent_joint_res(self.obj3d.get(&obj)?.0)
    }

    fn parent_joint_res(&self, res: u32) -> Option<crate::engine::anim::nodes::ParentJointFn> {
        let t = self.hanim_parent.get(&res)?.clone();
        Some(crate::engine::anim::nodes::ParentJointFn(Arc::new(move |bone: u32| {
            let i = if bone & 0xffff_0000 != 0 { t.0.iter().position(|&id| id as u32 == bone)? } else { bone as usize };
            *t.1.get(i)?
        })))
    }

    /// N2: EFGameObject::ResolveRefPUID for an anim descriptor of `go`: the EF3dObj whose 3D
    /// descriptor PUID is the anim's REFPUID. Returns (anim-node obj key, 3D res puid). The main
    /// 3D object (`obj3d`) keeps the game object uid as key; other ones get a private key.
    fn anim_target(&mut self, go: u32, refpuid: Option<u32>) -> (u32, u32) {
        let main = self.obj3d.get(&go).map_or(0, |o| o.0);
        let Some(r) = refpuid.filter(|&r| r != main && self.obj3d_list.get(&go).is_some_and(|l| l.iter().any(|x| x.1 == r))) else { return (go, main) };
        let n = self.anim_obj_keys.len() as u32;
        let key = *self.anim_obj_keys.entry((go, r)).or_insert(0xfe00_0000 | n);
        self.anim_obj_owner.insert(key, go);
        (key, r)
    }

    /// N2: pose of the EF3dObj (3D res puid `res`) of game object `go`
    pub fn PoseOf(&self, go: u32, res: u32) -> Option<Vec<crate::engine::anim::res::DTTransfo>> {
        let m = self.anim.get(&go)?;
        match self.anim_obj_keys.get(&(go, res)) {
            Some(&k) => m.GetPose(k),
            None if self.obj3d.get(&go).is_some_and(|o| o.0 == res) => m.GetPose(self.anim_bind.get(&go)?.obj3d),
            None => None,
        }
    }

    /// K6: the original duplicates the latent's own EFGameObject (EFGameLogicMgr::LaunchLatant ->
    /// EFGameObject::Duplicate of the DTMap<uint, EFLatant> entry), so a launched instance never
    /// touches its template. Here per-object data is keyed by uid, and instance ids (counter from
    /// 0x2a800000, EFGameLogicMgr +0x264) are in the same range as the latent uids of the pack
    /// (0x2a8000xx): e.g. on EP6L02 the 0x45th launch (a 0x2a8001c6 copy) got id 0x2a800045, the
    /// tod_beam latent, so the beam later cloned that copy's 2 anims instead of its own and its
    /// opening anim never ended (zombie capture stalled in FINALSUCKING). The template's data is
    /// moved aside the first time an instance id overwrites it and is read from there.
    pub fn CloneInstances(&mut self, template: u32, new_id: u32) {
        if !self.latent_tpl.contains_key(&new_id) {
            let t = LatentTpl { obj3d: self.obj3d.get(&new_id).copied(), obj3d_list: self.obj3d_list.get(&new_id).cloned(), anim_bind: self.anim_bind.get(&new_id).cloned() };
            if t.obj3d.is_some() || t.obj3d_list.is_some() || t.anim_bind.is_some() {
                self.latent_tpl.insert(new_id, t);
            }
        }
        let tpl = self.latent_tpl.get(&template).cloned().unwrap_or_else(|| LatentTpl { obj3d: self.obj3d.get(&template).copied(), obj3d_list: self.obj3d_list.get(&template).cloned(), anim_bind: self.anim_bind.get(&template).cloned() });
        match tpl.obj3d {
            Some(o) => { self.obj3d.insert(new_id, o); }
            None => { self.obj3d.remove(&new_id); }
        }
        match tpl.obj3d_list {
            Some(l) => { self.obj3d_list.insert(new_id, l); }
            None => { self.obj3d_list.remove(&new_id); }
        }
        let Some(b) = tpl.anim_bind else {
            self.anim.remove(&new_id);
            self.anim_bind.remove(&new_id);
            return;
        };
        let obj = if b.obj3d == template { new_id } else { b.obj3d };
        let base = b.anims[0].0;
        let count = b.anims.iter().map(|(id, _)| id.wrapping_sub(base) as usize + 1).max().unwrap_or(0);
        let mut m = EFAnimationMgr::new(base, count);
        m.owner_uid = new_id;
        for (k, &(id, ruid)) in b.anims.iter().enumerate() {
            if let Some(r) = self.hanim.get(&ruid).cloned() {
                let mut n = EFHAnimNode::new(r, id);
                let (o, res) = if obj == new_id { self.anim_target(new_id, b.refs.get(k).copied()) } else { (obj, self.obj3d.get(&obj).map_or(0, |o| o.0)) };
                n.obj = o;
                n.parent_joint = self.parent_joint_res(res);
                m.AddAnim(n, id);
            }
        }
        self.anim.insert(new_id, m);
        self.anim_bind.insert(new_id, AnimBinding { obj3d: obj, anims: b.anims, refs: b.refs });
    }

    /// EFWorld::Update animation slot: EFAnimationMgr::Update of every game object (list
    /// order replaced by uid order). Returns the queued Notify(0x11, owner).
    pub fn UpdateAnims(&mut self, dt: f32, paused: bool) -> Vec<(u32, u32)> {
        let mut out = Vec::new();
        let mut keys: Vec<u32> = self.anim.keys().copied().collect();
        keys.sort_unstable();
        for k in keys {
            let m = self.anim.get_mut(&k).unwrap();
            // N2: EFWorld::Update calls EFAnimationMgr::Update for every object; the world pause
            // only reaches the pausable objects (EFWorld::Pause -> SetLogicFlags 0x8 -> +0x50 bit0)
            let _ = paused;
            m.Update(dt);
            out.append(&mut m.notifications);
            // J3: EFAnimNode::Notify (0x0022d5a0) queued by the nodes during Evaluate:
            // EFNotificationMgr::Notify(gameLogic, code, node obj uid, code 0xb ? obj uid : node user, arg, 1)
            for n in m.nodes.iter_mut().flatten() {
                let (obj, ev) = match &mut n.kind {
                    crate::engine::anim::mgr::NodeKind::HAnim(h) => (h.obj, std::mem::take(&mut h.events)),
                    crate::engine::anim::mgr::NodeKind::Texture(t) => (t.obj, std::mem::take(&mut t.events)),
                    _ => continue,
                };
                // N2: nodes driving a secondary EF3dObj notify for their game object (EFAnimNode +0x28)
                let obj = self.anim_obj_owner.get(&obj).copied().unwrap_or(obj);
                if obj != 0 {
                    self.anim_events.extend(ev.into_iter().map(|e| (obj, e)));
                }
            }
        }
        out
    }

    /// Bone positions (object space) of the animated objects: the HAnim pose translation of
    /// each joint (replaces RpHAnimHierarchy matrices: no clump hierarchy yet, see findings).
    pub fn UpdateBones(&self, world: &mut RuntimeWorld) {
        world.bones.clear();
        for (go, b) in &self.anim_bind {
            let Some(m) = self.anim.get(go) else { continue };
            if let Some(p) = m.GetPose(b.obj3d) {
                world.bones.insert(*go, p.iter().map(|t| t.translation()).collect());
            }
        }
    }

    /// EFPhysicMgr::Update (2 passes) with the object frames: physic frames are the objects'
    /// RwFrames (shared in the C++), so they are copied in before and back after.
    pub fn UpdatePhysics(&mut self, dt: f32, world: &mut RuntimeWorld, camera: RwMatrix, splines: &[crate::engine::spline::EFQuadraticSpline]) -> Vec<PhysicNotify> {
        for p in self.physic.physics.iter_mut().flatten() {
            if let Some(f) = world.frames.get(&p.owner) {
                p.frame = dt_to_rw(f);
            }
        }
        {
            let w = RuntimePhysicWorld { world: &*world, input: &self.input, camera, splines };
            self.physic.Update(dt, world.paused, &w, &mut self.collision);
        }
        self.col_notify.append(&mut self.physic.col_notify);
        for p in self.physic.physics.iter().flatten() {
            if world.frames.contains_key(&p.owner) {
                world.frames.insert(p.owner, rw_to_dt(&p.frame));
            }
        }
        std::mem::take(&mut self.physic.notify)
    }

    /// EFNewAudioMgr update (MainLoop audio slot, vtbl 0x40): 3D geometry from the object
    /// frames and camera 0.
    pub fn UpdateAudio(&mut self, dt: f32, world: &RuntimeWorld, camera: Option<DTMatrix>) {
        let geo = AudioGeo { world, camera, listener: self.audio.listener };
        let lens = &self.sample_len;
        let len_of = |v: &crate::engine::audio::mgr::Voice| lens.get(&(v.bank, v.sample)).copied();
        self.audio.UpdateWith(dt, &len_of, Some(&geo));
    }

    // ---- APIanim_* (EFApi_Anim.cpp: GetGameObject(obj)+0x4c) ----
    fn am(&mut self, obj: u32) -> Option<&mut EFAnimationMgr> {
        self.anim.get_mut(&obj)
    }
}

pub(crate) fn dt_to_rw(m: &DTMatrix) -> RwMatrix {
    RwMatrix { right: m.right, up: m.up, at: m.at, pos: m.pos }
}
fn rw_to_dt(m: &RwMatrix) -> DTMatrix {
    DTMatrix { right: m.right, up: m.up, at: m.at, pos: m.pos }
}

/// PhysicWorld over the runtime state.
pub struct RuntimePhysicWorld<'a> {
    pub world: &'a RuntimeWorld,
    pub input: &'a RuntimeInput,
    pub camera: RwMatrix,
    /// _gpoSplineMgr splines (EFDMSplineMove / RopeSlide / CameraFlex)
    pub splines: &'a [crate::engine::spline::EFQuadraticSpline],
}

impl PhysicWorld for RuntimePhysicWorld<'_> {
    fn Splines(&self) -> &[crate::engine::spline::EFQuadraticSpline] {
        self.splines
    }
    fn GetObjectFrame(&self, uid: u32) -> Option<RwMatrix> {
        self.world.frames.get(&uid).map(dt_to_rw)
    }
    fn InputActionGet(&self, a: u32, b: u32) -> Option<[f32; 2]> {
        self.input.actions.get(&(a, b)).copied().or(self.input.stick)
    }
    fn GetCameraLTM(&self) -> RwMatrix {
        self.camera
    }
}

struct AudioGeo<'a> {
    world: &'a RuntimeWorld,
    camera: Option<DTMatrix>,
    listener: u32,
}

impl GeometryProvider for AudioGeo<'_> {
    fn geometry(&self, owner: u32, joint: Option<u32>) -> Option<Geometry> {
        use crate::engine::particle::PLWorld;
        let cam = self.camera.map(|c| Frame { at: c.at, pos: c.pos });
        let listener = if self.listener != 0 { self.world.frames.get(&self.listener).map(|f| Frame { at: f.at, pos: f.pos }) } else { None };
        // A2: EFSoundEvent_BC::Initialize (0x00244100) gives 3D events their own frame, child of
        // the owner's (or the joint's) frame, translated by local (0, 0.05, 0).
        let emitter = self.world.GetObjectFrame(owner, joint.unwrap_or(0)).map(|f| [0, 1, 2].map(|i| f.pos[i] + f.up[i] * 0.05));
        Some(Geometry { listener, camera: cam, emitter })
    }
}

// ---------------------------------------------------------------------------------------------
// host bindings (RuntimeHost delegates here)

fn rf(mem: &Mem, a: Ptr) -> f32 {
    f32::from_bits(mem.r32(a))
}
fn wv3(mem: &mut Mem, a: Ptr, v: [f32; 3]) {
    for (i, x) in v.iter().enumerate() {
        mem.w32(a + 4 * i as u32, x.to_bits());
    }
}
fn read_words(mem: &Mem, p: Ptr, n: usize) -> Vec<u32> {
    (0..n).map(|i| mem.r32(p + 4 * i as u32)).collect()
}
fn write_words(mem: &mut Mem, p: Ptr, v: &[u32]) {
    for (i, w) in v.iter().enumerate() {
        mem.w32(p + 4 * i as u32, *w);
    }
}

/// Lock helper used by the RuntimeHost bindings.
pub struct J2<'a>(pub &'a SharedParticles);

macro_rules! with_s1 {
    ($s:expr, |$sub:ident, $world:ident, $s1:ident| $body:expr) => {{
        let mut g = $s.0.lock().unwrap();
        let super::ParticleShared { sub: $sub, world: $world, s1: $s1, .. } = &mut *g;
        $body
    }};
}

macro_rules! with {
    ($s:expr, |$sub:ident, $world:ident| $body:expr) => {{
        let mut g = $s.0.lock().unwrap();
        let super::ParticleShared { sub: $sub, world: $world, .. } = &mut *g;
        let _ = &$world;
        $body
    }};
}

impl J2<'_> {
    // ---- anim
    pub fn anim_play(&self, obj: u32, anim: u32, flags: u32, t: f32) -> u32 {
        with!(self, |s, _w| {
            let r = aapi::APIanim_Play(s.am(obj), anim, flags, t);
            let started = s.anim.get(&obj).map_or(false, |m| aapi::APIanim_IsAnimPlaying(Some(m), anim) != 0);
            s.log.anim_play.push((obj, anim, flags, started));
            r
        })
    }
    pub fn anim_play_blend(&self, mem: &Mem, obj: u32, anim: u32, flags: u32, t: f32, p: Ptr) -> u32 {
        with!(self, |s, _w| {
            let r = aapi::APIanim_PlayBlend(s.am(obj), mem, anim, flags, t, p);
            let started = s.anim.get(&obj).map_or(false, |m| aapi::APIanim_IsAnimPlaying(Some(m), anim) != 0);
            s.log.anim_play.push((obj, anim, flags, started));
            r
        })
    }
    pub fn anim_play_blend_connect(&self, mem: &Mem, obj: u32, anim: u32, to: u32, flags: u32, p: Ptr) -> u32 {
        with!(self, |s, _w| aapi::APIanim_PlayBlendConnect(s.am(obj), mem, anim, to, flags, p))
    }
    pub fn anim_change_blend(&self, mem: &Mem, obj: u32, p: Ptr) -> u32 {
        with!(self, |s, _w| aapi::APIanim_ChangeBlendParams(s.am(obj), mem, p))
    }
    pub fn anim_stop(&self, obj: u32, anim: u32) -> u32 {
        with!(self, |s, _w| aapi::APIanim_StopAnim(s.am(obj), anim))
    }
    pub fn anim_stop_all(&self, obj: u32) -> u32 {
        with!(self, |s, _w| aapi::APIanim_StopAllAnim(s.am(obj)))
    }
    pub fn anim_get_time(&self, obj: u32, anim: u32) -> f32 {
        with!(self, |s, _w| aapi::APIanim_GetTime(s.anim.get(&obj), anim))
    }
    pub fn anim_set_time(&self, obj: u32, anim: u32, t: f32) -> u32 {
        with!(self, |s, _w| aapi::APIanim_SetTime(s.am(obj), anim, t))
    }
    pub fn anim_get_speed(&self, obj: u32, anim: u32) -> u32 {
        with!(self, |s, _w| aapi::APIanim_GetSpeed(s.anim.get(&obj), anim))
    }
    pub fn anim_set_speed(&self, obj: u32, anim: u32, v: f32) -> u32 {
        with!(self, |s, _w| aapi::APIanim_SetSpeed(s.am(obj), anim, v))
    }
    pub fn anim_get_duration(&self, obj: u32, anim: u32) -> f32 {
        with!(self, |s, _w| aapi::APIanim_GetDuration(s.anim.get(&obj), anim))
    }
    pub fn anim_connect(&self, obj: u32, anim: u32, target: u32) -> u32 {
        with!(self, |s, w| {
            let exists = w.frames.contains_key(&target);
            aapi::APIanim_Connect(s.am(obj), anim, exists)
        })
    }
    pub fn anim_get_group(&self, obj: u32, anim: u32) -> u32 {
        with!(self, |s, _w| aapi::APIanim_GetGroup(s.anim.get(&obj), anim))
    }
    pub fn anim_begin_tree(&self, obj: u32) -> u32 {
        with!(self, |s, _w| aapi::APIanim_BeginTree(s.am(obj)))
    }
    pub fn anim_end_tree(&self, obj: u32) -> u32 {
        with!(self, |s, _w| aapi::APIanim_EndTree(s.am(obj)))
    }
    pub fn anim_is_playing(&self, obj: u32, anim: u32) -> u8 {
        with!(self, |s, _w| aapi::APIanim_IsAnimPlaying(s.anim.get(&obj), anim))
    }

    // ---- physic
    pub fn physic_create(&self, obj: u32, list: u32) -> u32 {
        with!(self, |s, w| {
            let f = w.frames.get(&obj).map(dt_to_rw);
            let id = match f {
                // EFWorld::GetGameObject(obj) == NULL -> 0
                Some(f) => {
                    let id = papi::CreatePhysic(&mut s.physic, obj, list, f);
                    // EFGameObject::Get3dObject -> EF3dObj::CreateCollisionPrimitives(physic).
                    // Deviation: HAnim node LTMs = the root LTM (no RpHAnimHierarchy at creation).
                    if let (Some(p), Some(&(res, hanim))) = (s.physic.PhysicMut(id), s.obj3d.get(&obj)) {
                        let root = crate::engine::physic::mgr::rows(&f);
                        let col = s.res_collision.get(&res);
                        crate::engine::physic::create_prims::CreateCollisionPrimitives(col, if hanim { 4 } else { 0 }, root, &|_| root, &mut p.model);
                    } else if let (Some(p), Some(h)) = (s.physic.PhysicMut(id), s.hotspot.get(&obj)) {
                        // J9: EFGameObject::InitHotspot. Deviation: the C++ creates the physic and
                        // its prim at object init; here at the logic's CreatePhysic (the hot-spot
                        // modules all call it first; the frame exists only then).
                        if p.model.prims.iter().all(|x| x.is_none()) {
                            crate::engine::physic::create_prims::InitHotspot(h, crate::engine::physic::mgr::rows(&f), &mut p.model);
                        }
                    }
                    id
                }
                None => 0,
            };
            s.log.physic_create.push((obj, list, id));
            id
        })
    }
    pub fn physic_has(&self, obj: u32) -> bool {
        with!(self, |s, _w| s.physic.PhysicOf(obj) != 0)
    }
    pub fn physic_destroy(&self, obj: u32) {
        with!(self, |s, _w| {
            let id = s.physic.PhysicOf(obj);
            if id != 0 {
                s.physic.DestroyPhysic(id);
            }
        })
    }
    pub fn physic<R>(&self, f: impl FnOnce(&mut EFPhysicMgr, &RuntimePhysicWorld) -> R) -> R {
        with_s1!(self, |s, w, s1| {
            let pw = RuntimePhysicWorld { world: &*w, input: &s.input, camera: RwMatrix::default(), splines: &s1.splines.splines };
            f(&mut s.physic, &pw)
        })
    }
    /// J3: APIphysic::CheckRayIntersection(in, out) (0x001f7fa0) against the objects and the
    /// stage EFCollisionMap(s) through the runtime EFCollisionSystem. Guest RayIntersectIn: +0 origin,
    /// +0xc delta, +0x18/+0x1c object masks, +0x20/+0x24 map masks, +0x28 exclude (SELF already
    /// resolved by the caller). Returns 0 none / 1 object / 2 map, out packed by CheckRayIntersectionOut.
    pub fn ray_intersect(&self, mem: &mut Mem, rin: Ptr, out: Ptr) -> u32 {
        let w = read_words(mem, rin, 11);
        let f = |i: usize| f32::from_bits(w[i]);
        let rin = crate::engine::physic::mgr::EFRayIntersectIn {
            map: crate::engine::physic::collision_map::RayIntersectIn { origin: [f(0), f(1), f(2)], dir: [f(3), f(4), f(5)], mask1: w[8], mask2: w[9] },
            obj_mask1: w[6],
            obj_mask2: w[7],
            exclude: w[10],
        };
        let r = with_s1!(self, |s, w, s1| {
            let pw = RuntimePhysicWorld { world: &*w, input: &s.input, camera: RwMatrix::default(), splines: &s1.splines.splines };
            papi::CheckRayIntersectionOut(&mut s.physic, rin, &pw, &mut s.collision)
        });
        match r {
            Some(o) => {
                write_words(mem, out, &o);
                if o.len() == 8 { 1 } else { 2 }
            }
            None => 0,
        }
    }
    pub fn dm_add(&self, obj: u32, kind: u32) -> u32 {
        with!(self, |s, _w| {
            let h = papi::DMAdd(&mut s.physic, obj, kind);
            s.log.dm_add.push((obj, kind, h));
            h
        })
    }
    pub fn dm_get_params(&self, mem: &mut Mem, h: u32, p: Ptr) -> u32 {
        let v = self.physic(|m, w| papi::DMGetParams(m, h, w));
        if let (Some(v), true) = (v, p != 0) {
            write_words(mem, p, &v);
        }
        0
    }
    pub fn dm_set_params(&self, mem: &Mem, h: u32, p: Ptr) -> u32 {
        self.physic(|m, w| {
            let n = papi::DMGetParams(m, h, w).map_or(0, |v| v.len());
            let words = if p != 0 { Some(read_words(mem, p, n)) } else { None };
            papi::DMSetParams(m, h, words.as_deref(), w)
        });
        0
    }
    pub fn dm_get_data(&self, mem: &mut Mem, h: u32, p: Ptr) -> u32 {
        let v = self.physic(|m, w| papi::DMGetData(m, h, w));
        if let (Some(v), true) = (v, p != 0) {
            write_words(mem, p, &v);
        }
        0
    }
    pub fn dm_do_action(&self, mem: &mut Mem, h: u32, action: u32, p: Ptr) -> u32 {
        // the action block size is module specific; 16 words cover every ported DoAction
        let mut words = if p != 0 { read_words(mem, p, 16) } else { vec![0; 16] };
        self.physic(|m, w| papi::DMDoAction(m, h, action, &mut words, w));
        if p != 0 {
            write_words(mem, p, &words);
        }
        0
    }
    pub fn get_velocity(&self, mem: &mut Mem, obj: u32, p: Ptr) -> u32 {
        match self.physic(|m, w| papi::GetVelocity(m, obj, w)) {
            Some(v) => {
                wv3(mem, p, v);
                0
            }
            None => 1,
        }
    }
    pub fn get_floor_normal(&self, mem: &mut Mem, obj: u32, p: Ptr) -> u32 {
        let (r, n) = self.physic(|m, _| papi::GetFloorNormal(m, obj));
        wv3(mem, p, n);
        r
    }

    // ---- sound
    pub fn sound<R>(&self, f: impl FnOnce(&mut EFNewAudioMgr) -> R) -> R {
        with!(self, |s, _w| f(&mut s.audio))
    }
    pub fn sound_play(&self, obj: u32, id: u32, p3: u32) -> u32 {
        with!(self, |s, _w| {
            let r = s.audio.APInewSound_Play(obj, id, p3);
            s.log.sound_play.push((obj, id, r));
            r
        })
    }

    // ---- camera / light (the slots are swapped in while the logic runs)
    pub fn cameras<R>(&self, f: impl FnOnce(&mut EFGraphicsMgrCameras, &RuntimeWorld) -> R) -> R {
        with!(self, |s, w| f(&mut s.cameras, &*w))
    }
    pub fn camera_create(&self, mem: &mut Mem, obj: u32) -> u32 {
        with!(self, |s, w| {
            let f = w.frames.get(&obj).copied();
            // L4: the logic creates the real camera on the camera object (EP1L01 l01_camera
            // Camera_BaseFUNC_Constructor): drop the J2 load-time stand-in on that object (not
            // logic-created), else it keeps rendering slot 0 with the RwCameraCreate far = 10.
            for slot in s.cameras.cameras.iter_mut() {
                if matches!(slot, Some(c) if c.frame_owner == obj && !s.log.camera_create.iter().any(|x| x.1 == c.uid)) {
                    *slot = None;
                    s.cameras.count -= 1;
                }
            }
            let id = s.cameras.APIcamera_CreateCamera(mem, obj, f);
            s.log.camera_create.push((obj, id));
            id
        })
    }
    pub fn lights<R>(&self, f: impl FnOnce(&mut EFLightMgr_BC) -> R) -> R {
        with!(self, |s, _w| f(&mut s.lights))
    }
}


/// J10: ScaleCompensate parent table of a clump's RpHAnimHierarchy (EF3dObj::InitHierarchy
/// search): (node ids, per node index: EF3dObj::GetJointIndex(RpHAnimFrameGetID(parent frame of
/// pNodeInfo[i].pFrame))). GetJointIndex: ids < 0x10000 are indices (< numNodes), others go
/// through RpHAnimIDGetIndex.
pub fn ScaleCompensateParents(c: &crate::engine::rw::clump::RpClump) -> Option<(Vec<i32>, Vec<Option<u32>>)> {
    use crate::engine::gfx::hierarchy::{EF3dObjHierarchy, FrameTree};
    let t = FrameTree::from_clump(c);
    let mut h = EF3dObjHierarchy::default();
    let mut fl = 0;
    if h.InitHierarchy(c, &t, &mut fl) == 0 {
        return None;
    }
    let h = h.hierarchy?;
    let n = h.num_nodes;
    let joint = |id: i32| -> Option<u32> {
        let id = id as u32;
        if id & 0xffff_0000 == 0 {
            (id < n).then_some(id)
        } else {
            h.node_ids.iter().position(|&x| x as u32 == id).map(|i| i as u32)
        }
    };
    let par = h
        .node_frames
        .iter()
        .map(|f| {
            let p = t.parent[(*f)?];
            if p < 0 {
                return None;
            }
            joint(t.hanim_id[p as usize])
        })
        .collect();
    Some((h.node_ids.clone(), par))
}

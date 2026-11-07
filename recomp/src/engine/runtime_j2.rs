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
    /// per anim, the descriptor's SOUND_EVENT_IDS (EFHAnimNode::Initialize sound table)
    pub sounds: Vec<Vec<i32>>,
    /// camera anim descriptors (0x53): (anim id, EFAnimRes uid)
    pub cams: Vec<(u32, u32)>,
    /// texture anim descriptors (0x54): (anim id, EFTextureAnimRes uid, REFPUID, texture name)
    pub texs: Vec<(u32, u32, u32, String)>,
}

/// EFHAnimNode::Initialize: this+0x48 = per callback index the resolved sound (0 for id -1);
/// None when the descriptor lists no sound events. Kind-0 callbacks with a sound play it
/// instead of notifying the logic (0xb). The ids are pre-resolved by [`resolve_sounds`].
fn sound_table(ids: Option<&Vec<i32>>) -> Option<Vec<u32>> {
    let ids = ids.filter(|v| !v.is_empty())?;
    Some(ids.iter().map(|&i| if i == -1 { 0 } else { i as u32 }).collect())
}

/// EFNewSoundMgr::ResolveSoundEventID = GetSound on the owner's sound list: an id is found only
/// when the object's GLO has a sound (container) descriptor with it; unresolved ids become -1 (0).
fn resolve_sounds(go: &EFGameObject, ids: &[i32]) -> Vec<i32> {
    use crate::engine::dt::descriptors::{DTSoundContainerDescriptor, DTSoundDescriptor};
    let has = |id: u32| go.glo.object_descriptors.objects.iter().any(|d| {
        let a = d.as_any();
        a.downcast_ref::<EFIDescriptor<DTSoundDescriptor>>().is_some_and(|w| w.desc.index as u32 == id)
            || a.downcast_ref::<EFIDescriptor<DTSoundContainerDescriptor>>().is_some_and(|w| w.desc.sound_event_id == id)
    });
    ids.iter().map(|&i| if i != -1 && has(i as u32) { i } else { -1 }).collect()
}

/// K6: a latent template's per-uid data saved by [`Subsystems::CloneInstances`]
#[derive(Clone)]
pub struct LatentTpl {
    obj3d: Option<(u32, bool)>,
    obj3d_flags: Option<u32>,
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
    /// EFRessourceFactory: EFAnimRes (0x49) DTAnimHdr by uid (EFCameraAnimNode resources)
    pub anim_res: HashMap<u32, Arc<crate::engine::anim::res::DTAnimHdr>>,
    /// P26 R31: EFRessourceFactory: EFTextureAnimRes (0x2c) by uid (EFTextureAnimNode resources)
    pub tex_anim_res: HashMap<u32, Arc<crate::engine::anim::res::EFTextureAnimRes>>,
    /// P26 R31: texture names of each EF3dObjRes clump's materials (EFTextureAnimNode::Connect's
    /// ForAllAtomicFindMaterialCB search, done on names; the renderer swaps by name)
    pub res_tex_names: HashMap<u32, Vec<String>>,
    /// R6 P27: EFCameraAnimNode writes of the last UpdateAnims, applied by the runtime (which
    /// owns the cameras and the camera RwFrame = the owning object's frame)
    pub camera_anim_out: Vec<crate::engine::camera::CameraAnimOut>,
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
    /// P26 R16: game object uid -> DT3dObjectDescriptor INSTANCE_FLAG (+0x20), the
    /// DTCreationParameters flags EF3dObj::Initialize turns into the transparent-pass bits
    /// (8 -> 0x10, 0x20 -> 0x100, 0x40 -> 0x200)
    pub obj3d_flags: HashMap<u32, u32>,
    /// J9: DT_HOTSPOT_DESCR of hot-spot game objects (EFGameObject::InitHotspot)
    pub hotspot: HashMap<u32, crate::engine::dt::descriptors::DTHotSpotDescriptor>,
    /// J8: game object uid -> EFGameObject+0x174: (descriptor index n, EF3dObj resource uid) of
    /// every 3D object descriptor (APIhud_Window3DSetObj picks +0x174[n])
    pub obj3d_list: HashMap<u32, Vec<(u32, u32)>>,
    /// P28 B10: 3D object descriptor UID (0xf000xxx, EF3dObj+8) -> (game object uid, res puid).
    /// EFWorld::GetObjectPosData(0xf..) returns the EF3dObj itself, so pGameObj->Rotate/SetRotation
    /// on such a uid moves that 3D object's own frame (HUD courage needle 0xf000624, tome 0xf00062a).
    pub obj3d_desc: HashMap<u32, (u32, u32)>,
    /// N2: anim-node `obj` keys of the EF3dObjs other than `obj3d` ((game object uid, 3D res
    /// puid) -> key; key -> game object uid): objects with several animated 3D descriptors
    /// (pause-menu panel 0x2a100006: ~45 EF3dObjs, each with its own anims)
    pub anim_obj_keys: HashMap<(u32, u32), u32>,
    pub anim_obj_owner: HashMap<u32, u32>,
    /// P22 R26 GAME_LOGICCLIP: objects off the EFWorld +0x414c animation list (disabled, or clip word 0x20)
    pub anim_clip_off: std::collections::HashSet<u32>,
    /// EFDMCameraFlex::UpdateAnimation camera writes from the last UpdatePhysics (FOV channel 0x14 ->
    /// EFCamera::SetFOV(GetCamera(0)), far clip channel 0x13); applied by the runtime to camera 0
    pub cam_flex_req: (Option<f32>, Option<f32>),
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
    /// P20 R121 ANIMSND switch: EFHAnimNode::Initialize sound table (kind-0 anim callbacks with a
    /// resolved sound play it instead of notifying the logic, as the C does). ON in game.exe; the
    /// headless tests keep it OFF unless P17_ANIMSND=1 (routes tuned without it; see common.md).
    /// Survives LoadLevel.
    pub anim_sound: bool,
    /// P26 R19 CAMANIM switch: EFDMCameraFlex::UpdateAnimation camera anim channels (0x002283d0). ON in
    /// game.exe and (P20 R170) in headless tests (tests/all/common.rs camanim_on; P17_CAMANIM=0 = off). Survives LoadLevel.
    pub cam_anim: bool,
    /// P28 R11: logic-side joint posing on anim start (see gfx::render::NodePoser)
    pub node_poser: crate::engine::gfx::render::NodePoser,
}

impl Subsystems {
    /// P28 R11: EFAnimationMgr::PlayHAnim poses the hierarchy at once: re-publish `obj`'s node frames
    /// from its new pose, for objects whose nodes are published (renderer / headless node set) and whose
    /// clump the publisher registered. Others are left to the publisher, as before.
    pub fn ReposeNodes(&mut self, w: &mut RuntimeWorld, obj: u32) {
        if !w.nodes.contains_key(&obj) {
            return;
        }
        let Some(&(res_uid, _)) = self.obj3d.get(&obj) else { return };
        let Some(f) = w.frames.get(&obj).copied() else { return };
        let pose = self.anim_bind.get(&obj).and_then(|b| self.anim.get(&obj).and_then(|m| m.GetPose(b.obj3d)));
        let ltm = [f.right[0], f.right[1], f.right[2], 0., f.up[0], f.up[1], f.up[2], 0., f.at[0], f.at[1], f.at[2], 0., f.pos[0], f.pos[1], f.pos[2], 1.];
        if let Some((nodes, ids)) = self.node_poser.Repose(obj, res_uid, pose.as_deref(), ltm) {
            w.nodes.insert(obj, nodes);
            w.node_ids.insert(obj, ids);
            w.node_root.insert(obj, f);
        }
    }
    pub fn new(rng: SharedRand) -> Self {
        Subsystems {
            anim: HashMap::new(),
            anim_bind: HashMap::new(),
            cam_flex_req: (None, None),
            hanim: HashMap::new(),
            anim_res: HashMap::new(),
            tex_anim_res: HashMap::new(),
            res_tex_names: HashMap::new(),
            camera_anim_out: Vec::new(),
            physic: EFPhysicMgr::new(),
            collision: EFCollisionSystem { rand: rng.clone(), ..EFCollisionSystem::new() },
            col_notify: Vec::new(),
            anim_events: Vec::new(),
            res_collision: HashMap::new(),
            hanim_parent: HashMap::new(),
            obj3d: HashMap::new(),
            obj3d_flags: HashMap::new(),
            hotspot: HashMap::new(),
            obj3d_list: HashMap::new(),
            obj3d_desc: HashMap::new(),
            anim_obj_keys: HashMap::new(),
            anim_obj_owner: HashMap::new(),
            anim_clip_off: Default::default(),
            latent_tpl: HashMap::new(),
            audio: EFNewAudioMgr::with_rng(rng),
            sample_len: HashMap::new(),
            cameras: EFGraphicsMgrCameras::default(),
            lights: EFLightMgr_BC::new(),
            input: RuntimeInput::default(),
            log: ApiLog::default(),
            anim_sound: false,
            cam_anim: false,
            node_poser: Default::default(),
        }
    }

    /// Level part of the subsystems (EFRessourcesMgr::LoadLevel registrations + EFGameObject::
    /// CreateInstances for anims and sounds). `fs` reads the sound banks (SOUND_BankLoad).
    pub fn LoadLevel(&mut self, res: &mut EFRessourcesMgr, level_dir: &str, read: &mut dyn FnMut(&str) -> Option<Vec<u8>>) {
        crate::crashlog::set_level(level_dir);
        let rng = self.audio.rng.clone();
        *self = Subsystems { input: std::mem::take(&mut self.input), anim_sound: self.anim_sound, cam_anim: self.cam_anim, ..Subsystems::new(rng) };
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
                        if let Some(c) = &g.clump {
                            let names = c.geometries.iter().flat_map(|g| g.materials.iter()).filter_map(|m| m.texture.as_ref().map(|t| t.name.clone())).collect();
                            self.res_tex_names.insert(g.res.uid, names);
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
            } else if let Some(r) = a.downcast_ref::<crate::engine::anim::res::EFTextureAnimRes>() {
                self.tex_anim_res.insert(r.hdr.uid, Arc::new(r.clone()));
            } else if let Some(r) = a.downcast_ref::<crate::engine::anim::res::EFAnimRes>() {
                if let Some(hdr) = &r.anim {
                    self.anim_res.insert(r.hdr.uid, Arc::new(hdr.clone()));
                }
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
            // WP-P21 R30: STREAM banks (VAG voice lines) have no PCM; without a length their voices never
            // completed (ObjIsSoundPlaying stuck true, e.g. EP3/L01 inv_racecontroller waits for
            // 0x8000000c to end). Length = VAG body (BE size @0xc) / 16 * 28 samples at the rate @0x10.
            if bank.sbh.is_stream() && std::env::var_os("GAME_NOSTREAMLEN").is_none() {
                for (i, s) in bank.sbh.samples.iter().enumerate() {
                    let Some(f) = read(&crate::engine::audio::paths::stream_path(&s.name)) else { continue };
                    // P26 R10: .INT interleaved stereo streams (A2M header, e.g. EP1/L01 inv_tutorial VO
                    // CS1_4_2.INT): per-channel length as spu::AdpcmStream::interleaved plays it. Without
                    // it the voice never completed, so no end-of-sound notification (B9 pickup cut-scene).
                    if crate::engine::audio::spu::is_interleaved(&f) {
                        use crate::engine::audio::spu::{INT_HEADER, INT_INTERLEAVE};
                        let rate = if s.sampling_rate != 0 { s.sampling_rate } else { u32::from_be_bytes(f[0x10..0x14].try_into().unwrap()) };
                        let per_ch = (f.len() - INT_HEADER) / (2 * INT_INTERLEAVE) * INT_INTERLEAVE;
                        if rate > 0 {
                            let len = (per_ch / 16 * 28) as f32 / rate as f32;
                            if std::env::var_os("GAME_STREAMLEN").is_some() {
                                eprintln!("STREAMLEN bank {b} {i} {} INT rate {rate} file {:#x} -> {len}s hdr {:02x?} tail {:02x?}", s.name, f.len(), &f[..0x30], &f[f.len() - 0x40..]);
                            }
                            self.sample_len.insert((b as u32, i as u32), len);
                        }
                        continue;
                    }
                    if f.len() < 0x30 || &f[0..4] != b"VAGp" { continue }
                    let be = |o: usize| u32::from_be_bytes(f[o..o + 4].try_into().unwrap());
                    let rate = if s.sampling_rate != 0 { s.sampling_rate } else { be(0x10) };
                    if rate > 0 {
                        let len = (be(0xc) / 16 * 28) as f32 / rate as f32;
                        if std::env::var_os("GAME_STREAMLEN").is_some() {
                            eprintln!("STREAMLEN bank {b} {i} {} rate {rate} hdr {} size {:#x} file {:#x} -> {len}s", s.name, be(0x10), be(0xc), f.len());
                        }
                        self.sample_len.insert((b as u32, i as u32), len);
                    }
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
        // EFCameraAnimNode descriptors (instance class 0x53): (descriptor index, resource puid)
        let mut cams: Vec<(u32, u32)> = Vec::new();
        // P26 R31: EFTextureAnimNode descriptors (instance class 0x54): (index, res puid, REFPUID, texture name)
        let mut texs: Vec<(u32, u32, u32, String)> = Vec::new();
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
                self.obj3d_flags.insert(go.uid, d.desc.instance_flag as u32);
                self.obj3d_list.entry(go.uid).or_default().push((i as u32, d.desc.puid as u32));
                if d.desc.uid >> 24 == 0xf {
                    self.obj3d_desc.insert(d.desc.uid, (go.uid, d.desc.puid as u32));
                }
            } else if let Some(d) = a.downcast_ref::<EFIDescriptor<DTAnimDescriptor>>() {
                b.anims.push((i as u32, d.desc.puid as u32));
                b.refs.push(d.desc.refpuid as u32);
                b.sounds.push(if self.anim_sound { resolve_sounds(go, &d.desc.sound_event_ids) } else { Vec::new() });
            } else if let Some(d) = a.downcast_ref::<EFIDescriptor<crate::engine::dt::descriptors::DTTextureAnimationDescriptor>>() {
                texs.push((i as u32, d.desc.puid as u32, d.desc.refpuid as u32, d.desc.texture_file_name.clone()));
            } else if let Some(d) = crate::engine::dt::descriptors::as_idesc(c.as_ref()).filter(|d| d.GetInstanceCLSID() == 0x53 && std::env::var_os("P27_NOCAMANIM").is_none()) {
                // R6 P27: EFGameObject::CreateInstances 0x001ff350 creates an EFAnimNode for the
                // instance classes 0x4a/0x53/0x54/0x51 (EFBaseIDescriptor::CreateNode: res = puid)
                cams.push((i as u32, d.DescPUID() as u32));
            }
        }
        if !b.anims.is_empty() || !cams.is_empty() || !texs.is_empty() {
            // HAnim objects get the EF3dObj HAnim flag (4)
            if !b.anims.is_empty() {
                if let Some(o) = self.obj3d.get_mut(&go.uid) {
                    o.1 = true;
                }
            }
            // +0x60 base = index of the first anim-node descriptor
            let ids = || b.anims.iter().map(|x| x.0).chain(cams.iter().map(|x| x.0)).chain(texs.iter().map(|x| x.0));
            let base = ids().min().unwrap_or(0);
            let count = ids().map(|id| id.wrapping_sub(base) as usize + 1).max().unwrap_or(0);
            let mut m = EFAnimationMgr::new(base, count);
            m.owner_uid = go.uid;
            for (k, &(id, ruid)) in b.anims.iter().enumerate() {
                if let Some(r) = self.hanim.get(&ruid).cloned() {
                    let mut n = EFHAnimNode::new(r, id);
                    // EFGameObject::Initialize passes ResolveRefPUID(REFPUID) to InitialiseNode as the
                    // creation params' 3D object (+4); EFHAnimNode::Initialize builds the sound table only
                    // when it is non-null (REFPUID among the object's descriptors, or a 0x18 static obj)
                    let resolved = b.refs.get(k).is_some_and(|&r| r >> 24 == 0x18 || self.obj3d_list.get(&go.uid).is_some_and(|l| l.iter().any(|x| x.1 == r)));
                    n.sound_table = if resolved { sound_table(b.sounds.get(k)) } else { None };
                    // debug: P17_ANIMDUMP=<hex uid> lists the anim's callbacks (time, kind, idx) + sound table
                    if std::env::var("P17_ANIMDUMP").ok().and_then(|h| u32::from_str_radix(&h, 16).ok()) == Some(go.uid) {
                        let cbs: Vec<(f32, u8, u32)> = (0..n.res.cb_count).map(|i| n.cb(i)).collect();
                        eprintln!("ANIMDUMP {:#x} anim {} res {:#x} ref {:#x?} cbs {:?} snd {:?} tbl {:?}", go.uid, id, ruid, b.refs.get(k), cbs, b.sounds.get(k), n.sound_table);
                    }
                    let (obj, res) = self.anim_target(go.uid, b.refs.get(k).copied());
                    n.obj = obj;
                    n.parent_joint = self.parent_joint_res(res);
                    m.AddAnim(n, id);
                }
            }
            self.add_cam_tex_nodes(go.uid, &mut m, &cams, &texs);
            b.cams = cams;
            b.texs = texs;
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

    /// P26 R31: the EFTextureAnimNodes in the EF3dObj +0x60 list of game object `go`'s 3D object
    /// `res` (connected 0xf nodes that were played and not stopped): (original texture, current
    /// frame texture) for EF3dObj::Render's ApplyText/RestoreText
    pub fn TexAnimsOf(&self, go: u32, res: u32) -> Vec<(String, String)> {
        use crate::engine::anim::mgr::NodeKind;
        let Some(m) = self.anim.get(&go) else { return Vec::new() };
        m.anims.iter().flatten().filter_map(|&i| match &m.node(i).kind {
            NodeKind::Texture(t) if t.target == 0xf && t.obj == res && t.played && t.state & crate::engine::anim::nodes::state::STOPPED == 0 => {
                Some((t.orig_texture.clone()?, t.current_texture()?.to_string()))
            }
            _ => None,
        }).collect()
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
            let t = LatentTpl { obj3d: self.obj3d.get(&new_id).copied(), obj3d_flags: self.obj3d_flags.get(&new_id).copied(), obj3d_list: self.obj3d_list.get(&new_id).cloned(), anim_bind: self.anim_bind.get(&new_id).cloned() };
            if t.obj3d.is_some() || t.obj3d_list.is_some() || t.anim_bind.is_some() {
                self.latent_tpl.insert(new_id, t);
            }
        }
        let tpl = self.latent_tpl.get(&template).cloned().unwrap_or_else(|| LatentTpl { obj3d: self.obj3d.get(&template).copied(), obj3d_flags: self.obj3d_flags.get(&template).copied(), obj3d_list: self.obj3d_list.get(&template).cloned(), anim_bind: self.anim_bind.get(&template).cloned() });
        match tpl.obj3d {
            Some(o) => { self.obj3d.insert(new_id, o); }
            None => { self.obj3d.remove(&new_id); }
        }
        match tpl.obj3d_flags {
            Some(f) => { self.obj3d_flags.insert(new_id, f); }
            None => { self.obj3d_flags.remove(&new_id); }
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
        // EFGameObject::CreateInstances creates every anim node kind (0x4a/0x53/0x54) per instance
        let ids = || b.anims.iter().map(|x| x.0).chain(b.cams.iter().map(|x| x.0)).chain(b.texs.iter().map(|x| x.0));
        let base = ids().min().unwrap_or(0);
        let count = ids().map(|id| id.wrapping_sub(base) as usize + 1).max().unwrap_or(0);
        let mut m = EFAnimationMgr::new(base, count);
        m.owner_uid = new_id;
        for (k, &(id, ruid)) in b.anims.iter().enumerate() {
            if let Some(r) = self.hanim.get(&ruid).cloned() {
                let mut n = EFHAnimNode::new(r, id);
                n.sound_table = sound_table(b.sounds.get(k));
                let (o, res) = if obj == new_id { self.anim_target(new_id, b.refs.get(k).copied()) } else { (obj, self.obj3d.get(&obj).map_or(0, |o| o.0)) };
                n.obj = o;
                n.parent_joint = self.parent_joint_res(res);
                m.AddAnim(n, id);
            }
        }
        self.add_cam_tex_nodes(new_id, &mut m, &b.cams, &b.texs);
        self.anim.insert(new_id, m);
        self.anim_bind.insert(new_id, AnimBinding { obj3d: obj, ..b });
    }

    /// EFGameObject::CreateInstances: the camera (0x53) and texture (0x54) anim nodes of `uid`
    fn add_cam_tex_nodes(&self, uid: u32, m: &mut EFAnimationMgr, cams: &[(u32, u32)], texs: &[(u32, u32, u32, String)]) {
        for &(id, ruid) in cams {
            if let Some(r) = self.anim_res.get(&ruid).cloned() {
                m.AddCameraAnim(crate::engine::camera::EFCameraAnimNode::new(r, id), id);
            }
        }
        for (id, ruid, refpuid, name) in texs {
            let Some(r) = self.tex_anim_res.get(ruid).cloned() else { continue };
            // EFTextureAnimNode::Initialize: +0x54 = the descriptor's texture (current TXD), then
            // Connect(ResolveRefPUID(REFPUID)) when it resolves (a 3D object of this game object
            // or a 0x18 static world instance)
            let mut n = crate::engine::anim::texture_node::EFTextureAnimNode::new(r, *id, Some(name.clone()));
            let refpuid = *refpuid;
            if refpuid >> 24 == 0x18 {
                n.Connect(0, 0x18, true);
            } else if self.obj3d_list.get(&uid).is_some_and(|l| l.iter().any(|x| x.1 == refpuid)) {
                let found = self.res_tex_names.get(&refpuid).is_some_and(|v| v.iter().any(|t| t.eq_ignore_ascii_case(name)));
                n.Connect(refpuid, 0xf, found);
            }
            m.AddTextureAnim(n, *id);
        }
    }

    /// EFWorld::Update animation slot: EFAnimationMgr::Update of every game object (list
    /// order replaced by uid order). Returns the queued Notify(0x11, owner).
    pub fn UpdateAnims(&mut self, dt: f32, paused: bool) -> Vec<(u32, u32)> {
        let mut out = Vec::new();
        let mut keys: Vec<u32> = self.anim.keys().copied().collect();
        keys.sort_unstable();
        for k in keys {
            if self.anim_clip_off.contains(&k) {
                continue;
            }
            let m = self.anim.get_mut(&k).unwrap();
            // N2: EFWorld::Update calls EFAnimationMgr::Update for every object; the world pause
            // only reaches the pausable objects (EFWorld::Pause -> SetLogicFlags 0x8 -> +0x50 bit0)
            let _ = paused;
            m.Update(dt);
            out.append(&mut m.notifications);
            // R6 P27: EFCameraAnimNode writes + Notify (node +0x28 = this game object)
            for n in m.nodes.iter_mut().flatten() {
                if let crate::engine::anim::mgr::NodeKind::Camera(c) = &mut n.kind {
                    self.camera_anim_out.extend(c.out.take());
                    if std::env::var_os("GAME_CAMANIM").is_some() && c.state & 0x800_0000 == 0 {
                        eprintln!("CAMNODE f{} obj {k:#x} cam {:x?} t {} / {} state {:#x}", crate::engine::runtime::TRACE_FRAME.load(std::sync::atomic::Ordering::Relaxed), c.camera, c.time, c.duration, c.state);
                    }
                    self.anim_events.extend(c.notifies.drain(..).map(|(code, user)| (k, crate::engine::anim::nodes::AnimEvent::Notify { code, user, arg: user })));
                }
            }
            // J3: EFAnimNode::Notify (0x0022d5a0) queued by the nodes during Evaluate:
            // EFNotificationMgr::Notify(gameLogic, code, node obj uid, code 0xb ? obj uid : node user, arg, 1)
            for n in m.nodes.iter_mut().flatten() {
                let (obj, ev) = match &mut n.kind {
                    crate::engine::anim::mgr::NodeKind::HAnim(h) => (h.obj, std::mem::take(&mut h.events)),
                    // EFTextureAnimNode::Update/Stop: EFAnimNode::Notify with +0x28 = the game object
                    // (EFGameObject::CreateInstances); t.obj is the connected 3D object
                    crate::engine::anim::mgr::NodeKind::Texture(t) => (k, std::mem::take(&mut t.events)),
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
            let w = RuntimePhysicWorld { world: &*world, input: &self.input, camera, splines, anim: if self.cam_anim { Some(&self.anim) } else { None }, cam_req: Default::default() };
            self.physic.Update(dt, world.paused, &w, &mut self.collision);
            self.cam_flex_req = w.cam_req.get();
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
    /// EFGameObject +0x60 EFAnimationMgr per object (EFDMCameraFlex::UpdateAnimation)
    pub anim: Option<&'a HashMap<u32, EFAnimationMgr>>,
    /// (FOV, far clip) written by EFDMCameraFlex::UpdateAnimation to GetCamera(0)
    pub cam_req: std::cell::Cell<(Option<f32>, Option<f32>)>,
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
    fn GetParentObject(&self, uid: u32) -> Option<u32> {
        self.world.parents.get(&uid).copied()
    }
    /// EFDMCameraFlex::UpdateAnimation (0x00228420): walk GetGameObject(uid)+0x60 (EFAnimationMgr,
    /// count +0x4c, table +4 indexed by i - first id +0) -> node +0x30 res +0x10 DTAnimHdr, first
    /// channel set whose id == spline id.
    fn CameraAnimChannels(&self, uid: u32, spline_id: i32) -> Option<Vec<(u32, crate::engine::anim::res::DTFCurve)>> {
        use crate::engine::anim::mgr::NodeKind;
        let m = self.anim?.get(&uid)?;
        for i in 0..m.anims.len() as u32 {
            // quirk: the C indexes the table with i - first id (garbage when the first id != 0)
            let Some(Some(n)) = m.anims.get(i.wrapping_sub(m.base) as usize) else { continue };
            let hdr = match &m.nodes.get(*n).and_then(|x| x.as_ref())?.kind {
                NodeKind::HAnim(h) => h.res.base.anim.as_ref(),
                NodeKind::Camera(c) => Some(&*c.anim),
                _ => None,
            };
            if let Some(set) = hdr.and_then(|h| h.sets.iter().find(|s| s.bone == spline_id as u32)) {
                return Some(set.channels.iter().map(|c| (c.id, c.curve.clone())).collect());
            }
        }
        None
    }
    fn CameraSetFarClipPlane(&self, v: f32) {
        let (f, _) = self.cam_req.get();
        self.cam_req.set((f, Some(v)));
    }
    fn CameraSetFOV(&self, fov: f32) {
        let (_, c) = self.cam_req.get();
        self.cam_req.set((Some(fov), c));
    }
    /// P20 R44: the HAnim node frames published by the renderer (RuntimeWorld::nodes, modelling
    /// matrix + LTM); `bone` with high bits = HAnim id (node_ids), else a node index. Before this
    /// the default None made SyncModels pin every bone-attached prim to the root frame (EP2/L01A
    /// castlegate 0x16a's gate prim on node 1 never followed its open anim).
    fn GetBone(&self, uid: u32, bone: u32) -> Option<(RwMatrix, RwMatrix)> {
        // debug/bisect: GAME_NO_GETBONE=1 restores the pre-R44 behaviour (no bone frames)
        static OFF: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        // P20 R46: node 0 (HAnim root) stays on the object frame (pre-R44 behaviour): the hero prims sit on
        // node 0 and following it broke EP1/L03B (ep1_chain) -- unresolved whether the C does follow it
        if bone == 0 || *OFF.get_or_init(|| std::env::var_os("GAME_NO_GETBONE").is_some()) {
            return None;
        }
        let i = if bone & 0xffff_0000 != 0 { self.world.node_ids.get(&uid)?.iter().position(|&h| h as u32 == bone)? } else { bone as usize };
        let (m, l) = self.world.nodes.get(&uid)?.get(i)?;
        let mut l = dt_to_rw(l);
        // the nodes are published by the renderer after the previous frame: move them with the object
        // (LTM = rel * root; rel = LTM * root_pub^-1) so a walking hero's prims do not lag a frame
        if let (Some(r0), Some(r1)) = (self.world.node_root.get(&uid), self.world.frames.get(&uid)) {
            l = RwMatrix::Multiply(&RwMatrix::Multiply(&l, &dt_to_rw(r0).Invert()), &dt_to_rw(r1));
        }
        Some((dt_to_rw(m), l))
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
        with!(self, |s, w| {
            if std::env::var_os("GAME_CAMANIM").is_some() && s.anim.get_mut(&obj).is_some_and(|m| m.camera_mut(anim).is_some()) {
                eprintln!("CAMPLAY f{} obj {obj:#x} anim {anim} flags {flags:#x} t {t}", crate::engine::runtime::TRACE_FRAME.load(std::sync::atomic::Ordering::Relaxed));
            }
            let r = aapi::APIanim_Play(s.am(obj), anim, flags, t);
            let started = s.anim.get(&obj).map_or(false, |m| aapi::APIanim_IsAnimPlaying(Some(m), anim) != 0);
            s.log.anim_play.push((obj, anim, flags, started));
            if std::env::var("GAME_ANIMPLAY").is_ok_and(|v| u32::from_str_radix(v.trim_start_matches("0x"), 16) == Ok(obj)) {
                eprintln!("ANIMPLAY f{} obj {obj:#x} anim {anim:#x} flags {flags:#x} t {t} r {r} started {started}", crate::engine::runtime::TRACE_FRAME.load(std::sync::atomic::Ordering::Relaxed));
            }
            if started {
                s.ReposeNodes(w, obj);
            }
            r
        })
    }
    pub fn anim_play_blend(&self, mem: &Mem, obj: u32, anim: u32, flags: u32, t: f32, p: Ptr) -> u32 {
        with!(self, |s, w| {
            let r = aapi::APIanim_PlayBlend(s.am(obj), mem, anim, flags, t, p);
            let started = s.anim.get(&obj).map_or(false, |m| aapi::APIanim_IsAnimPlaying(Some(m), anim) != 0);
            s.log.anim_play.push((obj, anim, flags, started));
            if started {
                s.ReposeNodes(w, obj);
            }
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
        with!(self, |s, _w| {
            if std::env::var_os("GAME_CAMANIM").is_some() && s.anim.get_mut(&obj).is_some_and(|m| m.camera_mut(anim).is_some()) {
                eprintln!("CAMSTOP f{} obj {obj:#x} anim {anim}", crate::engine::runtime::TRACE_FRAME.load(std::sync::atomic::Ordering::Relaxed));
            }
            aapi::APIanim_StopAnim(s.am(obj), anim)
        })
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
            // EFWorld::GetObjectPosData(target): camera instances (0x10) or object frames
            let exists = if target >> 24 == 0x10 { s.cameras.GetCamera(target).is_some() } else { w.frames.contains_key(&target) };
            // EFCameraAnimNode::Connect (vtable +0x28); the other node types' Connect is empty
            if exists {
                if let Some(c) = s.anim.get_mut(&obj).and_then(|m| m.camera_mut(anim)) {
                    c.Connect(target);
                }
                // P26 R31: EFTextureAnimNode::Connect(EFInstance*): an EF3dObj instance (0xf) is
                // searched for a material with the node's texture; a 0x18 static instance connects
                // to the world material
                let r3d = s.obj3d_desc.get(&target).map(|x| x.1);
                let names = r3d.and_then(|r| s.res_tex_names.get(&r)).cloned();
                if let Some(m) = s.anim.get_mut(&obj) {
                    if let Some(i) = m.anim_node(anim) {
                        if let crate::engine::anim::mgr::NodeKind::Texture(t) = &mut m.node_mut(i).kind {
                            let orig = t.orig_texture.clone().unwrap_or_default();
                            match (target >> 24, r3d) {
                                (0xf, Some(r)) => t.Connect(r, 0xf, names.is_some_and(|v| v.iter().any(|n| n.eq_ignore_ascii_case(&orig)))),
                                (0x18, _) => t.Connect(0, 0x18, true),
                                _ => {}
                            }
                            let o = t.obj;
                            m.node_mut(i).obj = o;
                        }
                    }
                }
            }
            aapi::APIanim_Connect(s.am(obj), anim, exists)
        })
    }
    /// EFCutScenePlayer::Play: a type-7 node (EFCameraAnimNode) gets vtable +0x28 Connect(pos data)
    pub fn anim_node_connect(&self, obj: u32, anim: u32, target: u32) {
        with!(self, |s, _w| {
            if let Some(c) = s.anim.get_mut(&obj).and_then(|m| m.camera_mut(anim)) {
                c.Connect(target);
            }
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
            let pw = RuntimePhysicWorld { world: &*w, input: &s.input, camera: RwMatrix::default(), splines: &s1.splines.splines, anim: None, cam_req: Default::default() };
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
            let pw = RuntimePhysicWorld { world: &*w, input: &s.input, camera: RwMatrix::default(), splines: &s1.splines.splines, anim: None, cam_req: Default::default() };
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
            // P26 B1: also drop the owner-0 identity stand-in (levels without a camera-anim GLO,
            // e.g. INTRO: title_camera creates the only camera in the C); it kept slot 0 and the
            // title menu rendered from the origin instead of title_camera's frame.
            for slot in s.cameras.cameras.iter_mut() {
                if matches!(slot, Some(c) if (c.frame_owner == obj || c.frame_owner == 0) && !s.log.camera_create.iter().any(|x| x.1 == c.uid)) {
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

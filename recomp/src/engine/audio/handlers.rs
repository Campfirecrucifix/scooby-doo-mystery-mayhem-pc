//! Level-load descriptor classes (DTDynamicInstanciator type-1):
//! 0x17 EFSoundEventDescriptorHandler / 0x59 EFSoundContainerDescriptorHandler: Initialize
//! (0x00241e50 / 0x00268790) = owner's EFNewSoundMgr::AddSound(descriptor +0x14) (DTSoundDescriptor
//! INDEX / DTSoundContainerDescriptor SoundEventID). The descriptor wrappers that create them are
//! SOUND_DESCR 0x07 / SOUND_CONTAINER_DESCR 0x58 (dt::descriptors).
//! 0x3c EFAudioEnvDescriptor: LoadPropertiesBinary (0x001f9db0) = 3 x u32 (discarded) + 4 x
//! LoadTextData(0x100): s0 -> gpoNewAudioMgr+0x32c, SendSpecificData(s1, s3, s2).
//! Wiring: level::CreateInstance should fall back to [`CreateAudioInstance`].
#![allow(non_snake_case)]
use crate::engine::dt::property::{prop, DTObject, DTProperty, DTVariant};
use crate::engine::dt::stream::DTBasePersistStream;
use super::mgr::EFNewAudioMgr;
use crate::engine::dt::descriptors::{DTSoundContainerDescriptor, DTSoundDescriptor, EFBaseIDescriptor, EFIDescriptor};
use crate::engine::level::EFGameObject;
use crate::engine::pack::{DTDynamicInstanciator, DTObj, DTStreamable, RawInstanciator};
use std::sync::{Arc, Mutex};

pub const SOUND_INST: u32 = 0x17;
pub const SOUND_CONTAINER_INST: u32 = 0x59;
pub const EF_AUDIOENV_DESC: u32 = 0x3c;

#[derive(Debug, Default)]
pub struct EFSoundDescriptorHandler { pub uid: u32, pub class_id: u32 }
impl EFSoundDescriptorHandler {
    /// Initialize(DTCreationParameters): `owner` = object uid (handler +0x10), `id` = descriptor +0x14.
    pub fn Initialize(&self, mgr: &mut EFNewAudioMgr, owner: u32, id: u32) -> u32 {
        mgr.object(owner).AddSound(id);
        1
    }
}
impl DTObject for EFSoundDescriptorHandler {
    fn GetClassID(&self) -> u32 { self.class_id }
    fn GetClassName(&self) -> &'static str { if self.class_id == SOUND_INST { "SOUND_INST" } else { "SOUND_CONTAINER_INST" } }
    fn Members(&mut self) -> Vec<DTProperty<'_>> { vec![prop("UID", DTVariant::Uint(&mut self.uid))] }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

/// Where a loaded EFAudioEnvDescriptor is delivered. The original writes gpoNewAudioMgr from
/// inside LoadPropertiesBinary; the manager is not reachable from the pack stream here, so the
/// descriptor pushes a copy into this sink and the level builder calls `Apply` (same order).
pub type AudioEnvSink = Arc<Mutex<Vec<EFAudioEnvDescriptor>>>;

#[derive(Debug, Default, Clone)]
pub struct EFAudioEnvDescriptor { pub uid: u32, pub ints: [i32; 3], pub strings: [String; 4], pub sink: Option<AudioEnvSink> }
impl EFAudioEnvDescriptor {
    /// LoadPropertiesBinary tail (0x001f9db0): `strcpy(mgr+0x32c, s0); SendSpecificData(s1, s3, s2)`.
    pub fn Apply(&self, mgr: &mut EFNewAudioMgr) {
        mgr.env_name = self.strings[0].clone();
        mgr.SendSpecificData(&self.strings[1], &self.strings[3], &self.strings[2]);
    }
}
impl DTObject for EFAudioEnvDescriptor {
    fn GetClassID(&self) -> u32 { EF_AUDIOENV_DESC }
    fn GetClassName(&self) -> &'static str { "EF_AUDIOENV_DESC" }
    fn Members(&mut self) -> Vec<DTProperty<'_>> { vec![prop("UID", DTVariant::Uint(&mut self.uid))] }
    fn IsLoader(&self) -> bool { true }
    fn LoadPropertiesBinary(&mut self, s: &mut dyn DTBasePersistStream) -> i32 {
        for i in 0..3 { let r = s.LoadData_int(&mut self.ints[i]); if r != 0 { return r; } }
        for i in 0..4 {
            let mut b = Vec::new();
            let r = s.LoadTextData(&mut b, 0x100);
            if r != 0 { return r; }
            let n = b.iter().position(|&c| c == 0).unwrap_or(b.len());
            self.strings[i] = String::from_utf8_lossy(&b[..n]).into_owned();
        }
        if let Some(k) = &self.sink { k.lock().unwrap().push(EFAudioEnvDescriptor { sink: None, ..self.clone() }); }
        0
    }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

/// DTDynamicInstanciator::CreateInstance for the audio classes.
pub fn CreateAudioInstance(id: u32) -> Option<Box<dyn DTObject>> {
    Some(match id {
        SOUND_INST | SOUND_CONTAINER_INST => Box::new(EFSoundDescriptorHandler { uid: 0xffff_ffff, class_id: id }),
        EF_AUDIOENV_DESC => Box::new(EFAudioEnvDescriptor { uid: 0xffff_ffff, ..Default::default() }),
        _ => return None,
    })
}

/// Level-pack instanciator: RawInstanciator (level::CreateInstance, which chains
/// [`CreateAudioInstance`]) except that EF_AUDIOENV_DESC instances get the env sink attached.
#[derive(Default)]
pub struct AudioInstanciator { pub env: AudioEnvSink }
impl DTDynamicInstanciator for AudioInstanciator {
    fn Instanciate(&mut self, class_id: u32) -> Option<Box<dyn DTStreamable>> {
        if class_id == EF_AUDIOENV_DESC {
            return Some(Box::new(DTObj(Box::new(EFAudioEnvDescriptor { uid: 0xffff_ffff, sink: Some(self.env.clone()), ..Default::default() }))));
        }
        RawInstanciator.Instanciate(class_id)
    }
}

/// EFGameObject::CreateInstances (0x001ff350) + Initialize, sound part only: every descriptor of
/// the GLO whose wrapper instanciates SOUND_INST 0x17 / SOUND_CONTAINER_INST 0x59
/// (CreateObjectInstance vtbl 0x4c, handler +0x10 = owner) gets its handler Initialize'd
/// (owner's EFNewSoundMgr::AddSound(descriptor +0x14)), in descriptor order.
/// Returns the registered (instance class, id) pairs.
pub fn CreateSoundInstances(mgr: &mut EFNewAudioMgr, go: &EFGameObject) -> Vec<(u32, u32)> {
    let mut out = Vec::new();
    for d in &go.glo.object_descriptors.objects {
        let a = d.as_any();
        let (cls, id) = if let Some(w) = a.downcast_ref::<EFIDescriptor<DTSoundDescriptor>>() {
            (w.GetInstanceCLSID(), w.desc.index as u32)
        } else if let Some(w) = a.downcast_ref::<EFIDescriptor<DTSoundContainerDescriptor>>() {
            (w.GetInstanceCLSID(), w.desc.sound_event_id)
        } else {
            continue;
        };
        let Some(h) = CreateAudioInstance(cls) else { continue };
        let Some(h) = h.as_any().downcast_ref::<EFSoundDescriptorHandler>() else { continue };
        h.Initialize(mgr, go.uid, id);
        out.push((cls, id));
    }
    out
}

//! Level objects created while a pack loads: EFGameObject (EF_GLO, class 0x20) and the engine-level
//! DTDynamicInstanciator::CreateInstance (Classes/EF/EFGameObject.cpp, DT/DTDynamicInstanciator.cpp).

#![allow(non_snake_case)]

use super::dt::descriptors::*;
use super::dt::property::*;
use super::dt::stream::*;

/// EFGameObject (class 0x20 "EF_GLO").
///
/// EFGameObject::LoadFromStream (0x001ffa90): `glo = new DTGlo(0); glo->LoadFromStream(s)`
/// (vtbl 0x24: no Pre/PostPersist on the DTGlo), this+8 = glo UID, CreateInstances(glo),
/// this+0x38 = EFGameLogicMgr::GetLogicCode(), then if (glo UID & 0x800000) the object is
/// "latent": (this, glo) is stored in EFGameLogicMgr's latent map keyed by UID; else
/// Initialize(glo) + EFWorld::AddGameObject(this) and the DTGlo is deleted.
///
/// Deferred (non-obvious): CreateInstances/Initialize/AddGameObject need the world, graphics and
/// logic managers, which do not exist while the stream is parsed here. The parsed DTGlo is kept
/// and the level builder runs those three steps per object in load order afterwards. Load order
/// and the latent split (UID & 0x800000) are preserved; see `IsLatent`.
#[derive(Default, Debug)]
pub struct EFGameObject {
    /// +0x8
    pub uid: u32,
    pub glo: DTGlo,
}

impl EFGameObject {
    pub const CLASS_ID: u32 = 0x20;
    pub const CLASS_NAME: &'static str = "EF_GLO";
    pub const LATENT_UID_BIT: u32 = 0x80_0000;

    pub fn IsLatent(&self) -> bool {
        self.glo.uid & Self::LATENT_UID_BIT != 0
    }
}

impl DTObject for EFGameObject {
    fn GetClassID(&self) -> u32 {
        Self::CLASS_ID
    }
    fn GetClassName(&self) -> &'static str {
        Self::CLASS_NAME
    }
    fn Members(&mut self) -> Vec<DTProperty<'_>> {
        vec![prop("UID", DTVariant::Uint(&mut self.uid))]
    }
    fn LoadFromStream(&mut self, s: &mut dyn DTBasePersistStream) -> i32 {
        self.glo = DTGlo::default();
        // quirk: the return value of the DTGlo load is ignored, LoadFromStream returns 0
        self.glo.LoadFromStream(s);
        self.uid = self.glo.uid;
        0
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// DTDynamicInstanciator::CreateInstance(classId) (0x00106760) for objects created inside containers.
/// One registry in the original: SOUND_INST 0x17 (Instanciate 0x00241da0), SOUND_CONTAINER_INST 0x59
/// (0x002686e0) and EF_AUDIOENV_DESC 0x3c (0x001f9ec0) come from the same table, so they are
/// chained to the audio module here.
pub fn CreateInstance(id: u32) -> Option<Box<dyn DTObject>> {
    match id {
        EFGameObject::CLASS_ID => Some(Box::new(EFGameObject::default())),
        _ => CreateDescriptorInstance(id).or_else(|| super::audio::handlers::CreateAudioInstance(id)),
    }
}

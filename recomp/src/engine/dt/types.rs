//! DT class ids (GetClassID / DTDynamicInstanciator registration table, verified against EXE.1).
//! `INSTANCIABLE` = registered with type 1 (LoadLevel only instanciates type-1 classes);
//! type-2 entries are animation nodes created by their resources, never by the pack loop.

pub const STREAMABLE: u32 = 0x01;
pub const STATIC_3D_OBJECT_DESCR: u32 = 0x02;
pub const OBJECT_3D_DESCR: u32 = 0x03;
pub const LIGHT_DESCR: u32 = 0x06;
pub const SOUND_DESCR: u32 = 0x07;
pub const MORPH_ANIM_DESCR: u32 = 0x08;
pub const TEXTURE_ANIM_DESCR: u32 = 0x0b;
pub const CAMERA_DESCR: u32 = 0x0c;
pub const OBJECT_3D_INST: u32 = 0x0f;
pub const MORPH_ANIM_INST: u32 = 0x13;
pub const SOUND_INST: u32 = 0x17;
pub const STATIC_3D_OBJECT_INST: u32 = 0x18;
pub const DT_DYNAMIC_PROPERTY_CONTAINER: u32 = 0x1f;
pub const EF_GLO: u32 = 0x20;
pub const TEXTURE_DICTIONARY: u32 = 0x22;
pub const EF_COLLISION_MAP: u32 = 0x24;
pub const PL_EMITTER_CONTAINER: u32 = 0x25;
pub const EF_COLLISION_PRIM_ANIM: u32 = 0x26;
pub const EF_LOGIC_CODE: u32 = 0x27;
pub const PL_TYPE_CONTAINER: u32 = 0x29;
pub const EI_TEXTURE_ANIM_RES: u32 = 0x2c;
pub const PL_TYPE: u32 = 0x2d;
pub const DT_WAYPOINT_GRP: u32 = 0x30;
pub const INVISIBLE_DESCR: u32 = 0x33;
pub const EI_HANIMATION_DESCR: u32 = 0x37;
pub const EF_FONTS: u32 = 0x3a;
pub const EF_TEXTS: u32 = 0x3b;
pub const EF_AUDIOENV_DESC: u32 = 0x3c;
pub const EF_FONTS_PAL: u32 = 0x3e;
pub const DT_HOTSPOT_DESCRIPTOR: u32 = 0x3f;
pub const EI_OCTREE_RES: u32 = 0x40;
pub const EF_3DOBJECT_WATER: u32 = 0x41;
pub const EF_HUD_CONFIG: u32 = 0x43;
pub const EF_CONSOLE_FILE: u32 = 0x44;
pub const EF_LIBRARY_FILE: u32 = 0x45;
pub const EF_LOADING_SCREEN: u32 = 0x46;
pub const EF_GENERIC_FILE: u32 = 0x48;
pub const EF_ANIM_RES: u32 = 0x49;
pub const EF_ANIM_NODE: u32 = 0x4a;
pub const DT_ANIM_NODE_DESCR: u32 = 0x4b;
pub const DT_LIGHT_ANIM_DESCR: u32 = 0x4d;
pub const EF_HANIM_RES: u32 = 0x4f;
pub const EF_MORPH_ANIM_NODE: u32 = 0x51;
pub const EF_CUTSCENE: u32 = 0x52;
pub const EF_CAMERA_ANIM_NODE: u32 = 0x53;
pub const EF_TEXT_ANIM_NODE: u32 = 0x54;
pub const DT_CAMERA_ANIM_DESCR: u32 = 0x55;
pub const EF_3DOBJECT_GROUNDFOG: u32 = 0x56;
pub const EF_PALETTE_DICTIONARY: u32 = 0x57;
pub const SOUND_CONTAINER_DESCR: u32 = 0x58;
pub const SOUND_CONTAINER_INST: u32 = 0x59;
pub const EF_3DOBJ_RES: u32 = 0x5b;

/// (registered name, class id, instanciator type, implementing EF/DT class) — the full
/// DTDynamicInstanciator table in registration-id order.
pub const CLASS_TABLE: &[(&str, u32, u8, &str)] = &[
    ("3D_STATIC_OBJECT_DESCR", 0x02, 1, "DT3dStaticObjectDescriptor"),
    ("3D_OBJECT_DESCR", 0x03, 1, "DT3dObjectDescriptor"),
    ("LIGHT_DESCR", 0x06, 1, "DTLightDescriptor"),
    ("SOUND_DESCR", 0x07, 1, "DTSoundDescriptor"),
    ("MORPH_ANIM_DESCR", 0x08, 1, "DTMorphAnimationDescriptor"),
    ("TEXTURE_ANIM_DESCR", 0x0b, 1, "DTTextureAnimationDescriptor"),
    ("CAMERA_DESCR", 0x0c, 1, "DTCameraDescriptor"),
    ("3D_OBJECT_INST", 0x0f, 1, "EF3dObj"),
    ("MORPH_ANIM_INST", 0x13, 1, "EFMorphAnimRes"),
    ("SOUND_INST", 0x17, 1, "EFSoundEventDescriptorHandler"),
    ("STATIC_3D_OBJECT_INST", 0x18, 1, "EFStatic3dObj"),
    ("EF_GLO", 0x20, 1, "EFGameObject"),
    ("TEXTURE_DICTIONARY", 0x22, 1, "EFTextureDict"),
    ("EF_COLLISION_MAP", 0x24, 1, "EFCollisionMap"),
    ("PL_EMITTER_CONTAINER", 0x25, 1, "PLMgrEmitter"),
    ("EF_COLLISION_PRIM_ANIM", 0x26, 1, "EFCollisionPrimAnim"),
    ("EF_LOGIC_CODE", 0x27, 1, "EFLogicCode"),
    ("PL_TYPE_CONTAINER", 0x29, 1, "PLMgrType"),
    ("EI_TEXTURE_ANIM_RES", 0x2c, 1, "EFTextureAnimRes"),
    ("PL_TYPE", 0x2d, 1, "PLType"),
    ("INVISIBLE_DESCR", 0x33, 1, "EFInvisibleIdescriptor"),
    ("EI_HANIMATION_DESCR", 0x37, 1, "EFHAnimationIDescriptor"),
    ("EF_FONTS", 0x3a, 1, "EFFont"),
    ("EF_TEXTS", 0x3b, 1, "EFText"),
    ("EF_AUDIOENV_DESC", 0x3c, 1, "EFAudioEnvDescriptor"),
    ("EF_FONTS_PAL", 0x3e, 1, "EFHudPalette"),
    ("EI_OCTREE_RES", 0x40, 1, "EFEnvMap"),
    ("EF_3DOBJECT_WATER", 0x41, 1, "EFWaterGrid"),
    ("EF_HUD_CONFIG", 0x43, 1, "EFHudWindowStreamer"),
    ("EF_CONSOLE_FILE", 0x44, 1, "EFLogicCodeConsole"),
    ("EF_LIBRARY_FILE", 0x45, 1, "EFLogicCodeLib"),
    ("EF_LOADING_SCREEN", 0x46, 1, "EFLoadingScreen"),
    ("EF_GENERIC_FILE", 0x48, 1, "EFGenericDataLoader"),
    ("EF_ANIM_RES", 0x49, 1, "EFAnimRes"),
    ("EF_ANIM_NODE", 0x4a, 2, "EFHAnimNode"),
    ("DT_ANIM_NODE_DESCR", 0x4b, 1, "DTAnimDescriptor"),
    ("DT_LIGHT_ANIM_DESCR", 0x4d, 1, "DTLightAnimDescriptor"),
    ("EF_HANIM_RES", 0x4f, 1, "EFHAnimRes"),
    ("EF_MORPH_ANIM_NODE", 0x51, 2, "EFMorphAnimNode"),
    ("EF_CUTSCENE", 0x52, 1, "EFCutScenePlayer"),
    ("EF_CAMERA_ANIM_NODE", 0x53, 2, "EFCameraAnimNode"),
    ("EF_TEXT_ANIM_NODE", 0x54, 2, "EFTextureAnimNode"),
    ("DT_CAMERA_ANIM_DESCR", 0x55, 1, "DTCameraAnimDescriptor"),
    ("EF_3DOBJECT_GROUNDFOG", 0x56, 1, "EFGroundFogGrid"),
    ("EF_PALETTE_DICTIONARY", 0x57, 1, "EFPaletteDictionnary"),
    ("SOUND_CONTAINER_DESCR", 0x58, 1, "DTSoundContainerDescriptor"),
    ("SOUND_CONTAINER_INST", 0x59, 1, "EFSoundContainerDescriptorHandler"),
    ("EF_3DOBJ_RES", 0x5b, 1, "EF3dObjRes"),
];

/// Registered name for a class id (None for unregistered ids such as 0x1f/0x30/0x3f).
pub fn class_name(id: u32) -> Option<&'static str> {
    CLASS_TABLE.iter().find(|e| e.1 == id).map(|e| e.0)
}

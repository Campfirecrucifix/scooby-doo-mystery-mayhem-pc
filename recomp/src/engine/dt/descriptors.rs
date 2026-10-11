//! DT descriptors and their EF*IDescriptor wrappers (Classes/DT/DT*Descriptor.cpp, DTGlo.cpp,
//! DTMagicData.cpp, DTWaypointGrp.cpp, DTDynamicPropertyContainer.cpp,
//! Classes/EF/EF*IDescriptor.cpp, EFBaseIDescriptor.cpp).
//!
//! Member lists are the constructors' RegisterMember calls in order (extracted by
//! tools/engineport/schemas.py into tools/engineport/schemas.json). Each struct flattens its base
//! classes' members (UID first). Field docs give the C++ offset.

#![allow(non_snake_case)]

use super::property::*;
use super::stream::*;

/// Defines a DT class: fields with defaults, and the registered member list in order.
macro_rules! dt_class {
    ($(#[$m:meta])* $name:ident, $id:expr, $cname:expr, { $($(#[$fm:meta])* $f:ident : $t:ty = $d:expr => $pname:literal $kind:ident),* $(,)? } $(, $extra:item)*) => {
        $(#[$m])*
        #[derive(Debug, Clone)]
        pub struct $name { $($(#[$fm])* pub $f: $t),* }
        impl Default for $name {
            fn default() -> Self { Self { $($f: $d),* } }
        }
        impl $name {
            pub const CLASS_ID: u32 = $id;
            pub fn members(&mut self) -> Vec<DTProperty<'_>> {
                vec![$(prop($pname, DTVariant::$kind(&mut self.$f))),*]
            }
        }
        $($extra)*
    };
}

/// Plain DTObject impl (no overridden virtuals).
macro_rules! dt_object {
    ($name:ident) => {
        impl DTObject for $name {
            fn GetClassID(&self) -> u32 { Self::CLASS_ID }
            fn GetClassName(&self) -> &'static str { Self::CLASS_NAME }
            fn Members(&mut self) -> Vec<DTProperty<'_>> { self.members() }
            fn as_any(&self) -> &dyn std::any::Any { self }
            fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
        }
    };
}

const UNSET: u32 = 0xffff_ffff;

// ---------------------------------------------------------------------------------------------
// Base classes

dt_class!(
    /// DTBaseInstanceDescriptor : DTStreamable (UID +0x8, PUID +0xc, REFPUID +0x10)
    DTBaseInstanceDescriptor, 1, "", {
    uid: u32 = UNSET => "UID" Uint,
    puid: i32 = 0 => "PUID" Int,
    refpuid: i32 = -1 => "REFPUID" Int,
});

dt_class!(
    /// DTPosObjectDescriptor : DTBaseInstanceDescriptor
    DTPosObjectDescriptor, 1, "", {
    uid: u32 = UNSET => "UID" Uint,
    puid: i32 = 0 => "PUID" Int,
    refpuid: i32 = -1 => "REFPUID" Int,
    pos: [f32; 3] = [0.0; 3] => "POS" Vec3,
    look_at: [f32; 3] = [0.0; 3] => "LOOK_AT" Vec3,
    look_up: [f32; 3] = [0.0; 3] => "LOOK_UP" Vec3,
    look_right: [f32; 3] = [0.0; 3] => "LOOK_RIGHT" Vec3,
});

/// DT3dStaticObjectDescriptor : DTBaseInstanceDescriptor (no own members)
pub type DT3dStaticObjectDescriptor = DTBaseInstanceDescriptor;
/// DTHAnimationDescriptor / DTMorphAnimationDescriptor / DTCameraAnimDescriptor /
/// DTLightAnimDescriptor : DTBaseInstanceDescriptor (no own members, no overrides)
pub type DTHAnimationDescriptor = DTBaseInstanceDescriptor;

dt_class!(
    /// DT3dObjectDescriptor : DT3dStaticObjectDescriptor (SCALE, INSTANCE_FLAG)
    DT3dObjectDescriptor, 1, "", {
    uid: u32 = UNSET => "UID" Uint,
    puid: i32 = 0 => "PUID" Int,
    refpuid: i32 = -1 => "REFPUID" Int,
    scale: [f32; 3] = [1.0; 3] => "SCALE" Vec3,
    instance_flag: i32 = 0 => "INSTANCE_FLAG" Int,
});

dt_class!(
    /// DTAnimDescriptor : DTBaseInstanceDescriptor (+0x14 SOUND_EVENT_IDS DTVector<int>)
    DTAnimDescriptor, 1, "", {
    uid: u32 = UNSET => "UID" Uint,
    puid: i32 = 0 => "PUID" Int,
    refpuid: i32 = -1 => "REFPUID" Int,
    sound_event_ids: Vec<i32> = Vec::new() => "SOUND_EVENT_IDS" VecInt,
});

dt_class!(
    /// DTLightDescriptor : DTBaseInstanceDescriptor
    DTLightDescriptor, 1, "", {
    uid: u32 = UNSET => "UID" Uint,
    puid: i32 = 0 => "PUID" Int,
    refpuid: i32 = -1 => "REFPUID" Int,
    /// +0x14
    light_type: i32 = 0 => "LIGHT_TYPE" Int,
    /// +0x18
    color: [f32; 4] = [0.0; 4] => "COLOR" Vec4,
    /// +0x28
    flags: u32 = 0 => "FLAGS" Uint,
    /// +0x2c
    radius: f32 = 0.0 => "RADIUS" Float,
    /// +0x30
    cone_angle: f32 = 0.0 => "CONE_ANGLE" Float,
    light_pos: [f32; 3] = [0.0; 3] => "LIGHT_POS" Vec3,
    light_at: [f32; 3] = [0.0; 3] => "LIGHT_AT" Vec3,
    light_up: [f32; 3] = [0.0; 3] => "LIGHT_UP" Vec3,
    light_right: [f32; 3] = [0.0; 3] => "LIGHT_RIGHT" Vec3,
});

dt_class!(
    /// DTSoundDescriptor : DTBaseInstanceDescriptor (+0x14 INDEX)
    DTSoundDescriptor, 1, "", {
    uid: u32 = UNSET => "UID" Uint,
    puid: i32 = 0 => "PUID" Int,
    refpuid: i32 = -1 => "REFPUID" Int,
    index: i32 = 0 => "INDEX" Int,
});

dt_class!(
    /// DTSoundContainerDescriptor : DTBaseInstanceDescriptor (+0x14 SoundEventID)
    DTSoundContainerDescriptor, 1, "", {
    uid: u32 = UNSET => "UID" Uint,
    puid: i32 = 0 => "PUID" Int,
    refpuid: i32 = -1 => "REFPUID" Int,
    sound_event_id: u32 = 0 => "SoundEventID" Uint,
});

dt_class!(
    /// DTTextureAnimationDescriptor : DTBaseInstanceDescriptor (+0x14 TEXTURE_FILE_NAME)
    DTTextureAnimationDescriptor, 1, "", {
    uid: u32 = UNSET => "UID" Uint,
    puid: i32 = 0 => "PUID" Int,
    refpuid: i32 = -1 => "REFPUID" Int,
    texture_file_name: String = String::new() => "TEXTURE_FILE_NAME" String,
});

dt_class!(
    /// DTLoadingScreenDescriptor : DTBaseInstanceDescriptor
    DTLoadingScreenDescriptor, 1, "", {
    uid: u32 = UNSET => "UID" Uint,
    puid: i32 = 0 => "PUID" Int,
    refpuid: i32 = -1 => "REFPUID" Int,
    loading_screen1: String = String::new() => "LOADING_SCREEN1" String,
    loading_screen2: String = String::new() => "LOADING_SCREEN2" String,
});

// ---------------------------------------------------------------------------------------------
// DTMagicData

/// DTMagicData : DTStreamable (+0xc data, +0x10 size "MAGIC_SIZE").
/// LoadFromStream is overridden: one data bloc "MAGIC_BLOCK" (text streams skip a 0x14-byte token
/// first); no UID is read.
/// Non-obvious: owners register their own "xxx_SIZE" member on this object's +0x10 (an alias).
/// The C++ allocates MAGIC_SIZE bytes and then reads the bloc's own size into it; here the buffer
/// takes the bloc's size and `size` is set to it after loading (identical for the shipped data,
/// where both sizes are equal).
#[derive(Debug, Clone)]
pub struct DTMagicData {
    pub uid: u32,
    pub size: u32,
    pub data: Vec<u8>,
}

impl Default for DTMagicData {
    fn default() -> Self {
        Self { uid: UNSET, size: 0, data: Vec::new() }
    }
}

impl DTMagicData {
    pub const CLASS_ID: u32 = 1;
    pub const CLASS_NAME: &'static str = "";
}

impl DTObject for DTMagicData {
    fn GetClassID(&self) -> u32 {
        Self::CLASS_ID
    }
    fn GetClassName(&self) -> &'static str {
        Self::CLASS_NAME
    }
    fn Members(&mut self) -> Vec<DTProperty<'_>> {
        vec![prop("UID", DTVariant::Uint(&mut self.uid)), prop("MAGIC_SIZE", DTVariant::Uint(&mut self.size))]
    }
    fn LoadFromStream(&mut self, s: &mut dyn DTBasePersistStream) -> i32 {
        if s.GetStreamType() & STREAM_FLAG_TEXT != 0 {
            let mut tok = Vec::new();
            s.LoadString(&mut tok, 0x14);
        }
        let mut p = prop("MAGIC_BLOCK", DTVariant::DataBloc(&mut self.data));
        let r = p.LoadFromStream(s);
        self.size = self.data.len() as u32;
        r
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

// ---------------------------------------------------------------------------------------------
// Particles

/// DTEmitterDescriptor : DTStreamable (size 0x60)
#[derive(Debug, Clone)]
pub struct DTEmitterDescriptor {
    pub uid: u32,
    /// +0xc
    pub bone_id: i32,
    pub pos: [f32; 3],
    pub look_at: [f32; 3],
    pub look_up: [f32; 3],
    pub look_right: [f32; 3],
    /// +0x50 "PARTEMI_SIZE" (aliases partemi.size)
    pub partemi_size: u32,
    /// +0x40
    pub partemi: DTMagicData,
}

impl Default for DTEmitterDescriptor {
    fn default() -> Self {
        Self { uid: UNSET, bone_id: 0, pos: [0.0; 3], look_at: [0.0; 3], look_up: [0.0; 3], look_right: [0.0; 3], partemi_size: 0, partemi: Default::default() }
    }
}

impl DTEmitterDescriptor {
    pub const CLASS_ID: u32 = 1;
    pub const CLASS_NAME: &'static str = "";
    pub fn members(&mut self) -> Vec<DTProperty<'_>> {
        vec![
            prop("UID", DTVariant::Uint(&mut self.uid)),
            prop("BONE_ID", DTVariant::Int(&mut self.bone_id)),
            prop("POS", DTVariant::Vec3(&mut self.pos)),
            prop("LOOK_AT", DTVariant::Vec3(&mut self.look_at)),
            prop("LOOK_UP", DTVariant::Vec3(&mut self.look_up)),
            prop("LOOK_RIGHT", DTVariant::Vec3(&mut self.look_right)),
            prop("PARTEMI_SIZE", DTVariant::Uint(&mut self.partemi_size)),
            prop("PARTEMI", DTVariant::Streamable(&mut self.partemi)),
        ]
    }
}
dt_object!(DTEmitterDescriptor);

/// DTEmitterContainerDescriptor : DTBaseInstanceDescriptor (+0x14 NB_EMI, +0x18 DTVector<DTEmitterDescriptor>)
#[derive(Debug, Clone)]
pub struct DTEmitterContainerDescriptor {
    pub uid: u32,
    pub puid: i32,
    pub refpuid: i32,
    pub nb_emi: i32,
    pub emitters: Vec<DTEmitterDescriptor>,
}

impl Default for DTEmitterContainerDescriptor {
    fn default() -> Self {
        Self { uid: UNSET, puid: 0, refpuid: -1, nb_emi: 0, emitters: Vec::new() }
    }
}

impl DTEmitterContainerDescriptor {
    pub const CLASS_ID: u32 = 1;
    pub const CLASS_NAME: &'static str = "";
}

impl DTObject for DTEmitterContainerDescriptor {
    fn GetClassID(&self) -> u32 {
        Self::CLASS_ID
    }
    fn GetClassName(&self) -> &'static str {
        Self::CLASS_NAME
    }
    fn Members(&mut self) -> Vec<DTProperty<'_>> {
        vec![
            prop("UID", DTVariant::Uint(&mut self.uid)),
            prop("PUID", DTVariant::Int(&mut self.puid)),
            prop("REFPUID", DTVariant::Int(&mut self.refpuid)),
            prop("NB_EMI", DTVariant::Int(&mut self.nb_emi)),
        ]
    }
    /// base members, then NB_EMI emitters via DTStreamable::LoadFromStream (errors summed, quirk).
    fn LoadFromStream(&mut self, s: &mut dyn DTBasePersistStream) -> i32 {
        let mut r = DTStreamable_LoadFromStream(self, s);
        if r == 0 && self.nb_emi > 0 {
            self.emitters = Vec::with_capacity(self.nb_emi as usize);
            for _ in 0..self.nb_emi {
                let mut e = DTEmitterDescriptor::default();
                r += DTStreamable_LoadFromStream(&mut e, s);
                self.emitters.push(e);
            }
        }
        r
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// DTParticleContainerDescriptor : DTBaseInstanceDescriptor (+0x14 NB, +0x28 PARTYPE_SIZE aliasing
/// +0x18 PARTTYPES DTMagicData)
#[derive(Debug, Clone)]
pub struct DTParticleContainerDescriptor {
    pub uid: u32,
    pub puid: i32,
    pub refpuid: i32,
    pub nb: i32,
    pub partype_size: u32,
    pub parttypes: DTMagicData,
}

impl Default for DTParticleContainerDescriptor {
    fn default() -> Self {
        Self { uid: UNSET, puid: 0, refpuid: -1, nb: 0, partype_size: 0, parttypes: Default::default() }
    }
}

impl DTParticleContainerDescriptor {
    pub const CLASS_ID: u32 = 1;
    pub const CLASS_NAME: &'static str = "";
    pub fn members(&mut self) -> Vec<DTProperty<'_>> {
        vec![
            prop("UID", DTVariant::Uint(&mut self.uid)),
            prop("PUID", DTVariant::Int(&mut self.puid)),
            prop("REFPUID", DTVariant::Int(&mut self.refpuid)),
            prop("NB", DTVariant::Int(&mut self.nb)),
            prop("PARTYPE_SIZE", DTVariant::Uint(&mut self.partype_size)),
            prop("PARTTYPES", DTVariant::Streamable(&mut self.parttypes)),
        ]
    }
}
dt_object!(DTParticleContainerDescriptor);

// ---------------------------------------------------------------------------------------------
// Waypoints, hotspots

dt_class!(
    /// DTWaypointGrp : DTBaseInstanceDescriptor (class 0x30, size 0x2c): +0x14 NB_WAYPOINT,
    /// +0x18 POINTS DTVector<float> (xyz triples).
    DTWaypointGrp, 0x30, "", {
    uid: u32 = UNSET => "UID" Uint,
    puid: i32 = 0 => "PUID" Int,
    refpuid: i32 = -1 => "REFPUID" Int,
    nb_waypoint: u32 = 0 => "NB_WAYPOINT" Uint,
    points: Vec<f32> = Vec::new() => "POINTS" VecFloat,
});

impl DTWaypointGrp {
    pub const CLASS_NAME: &'static str = "DT_WAYPOINT_GRP";
}

impl DTObject for DTWaypointGrp {
    fn GetClassID(&self) -> u32 {
        Self::CLASS_ID
    }
    fn GetClassName(&self) -> &'static str {
        Self::CLASS_NAME
    }
    fn Members(&mut self) -> Vec<DTProperty<'_>> {
        self.members()
    }
    /// base load, then DTWpGrpMgr::GetMgr()->AddWaypointGrp(this). The manager is the level's
    /// [`super::super::world::DTWpGrpMgr`]; the level builder registers loaded groups in pack order.
    fn LoadFromStream(&mut self, s: &mut dyn DTBasePersistStream) -> i32 {
        DTStreamable_LoadFromStream(self, s)
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

dt_class!(
    /// DTHotSpotDescriptor : DT3dObjectDescriptor (class 0x3f). After the members, the primitive's
    /// extra floats at +0x70..: PrimType 2 -> 3 floats, 1 -> 4 floats, 0 -> 1 float, else -2.
    DTHotSpotDescriptor, 0x3f, "", {
    uid: u32 = UNSET => "UID" Uint,
    puid: i32 = 0 => "PUID" Int,
    refpuid: i32 = -1 => "REFPUID" Int,
    scale: [f32; 3] = [1.0; 3] => "SCALE" Vec3,
    instance_flag: i32 = 0 => "INSTANCE_FLAG" Int,
    /// +0x24
    prim_type: u32 = 0 => "PrimType" Uint,
    primitive_tag: u32 = 0 => "PrimitiveTag" Uint,
    bone_id: u32 = 0 => "BoneID" Uint,
    translation_x: f32 = 0.0 => "TranslationX" Float,
    translation_y: f32 = 0.0 => "TranslationY" Float,
    translation_z: f32 = 0.0 => "TranslationZ" Float,
    hardness: f32 = 0.0 => "Hardness" Float,
    rebound: f32 = 0.0 => "Rebound" Float,
    detection_mode: u32 = 0 => "DetectionMode" Uint,
    type_: u32 = 0 => "Type" Uint,
    detect_type: u32 = 0 => "DetectType" Uint,
    ud_type: u32 = 0 => "UDType" Uint,
    ud_detect_type: u32 = 0 => "UDDetectType" Uint,
    env_detect_type: u32 = 0 => "EnvDetectType" Uint,
    ud_env_detect_type: u32 = 0 => "UDEnvDetectType" Uint,
    notify_type: u32 = 0 => "NotifyType" Uint,
    arg1: u32 = 0 => "Arg1" Uint,
    arg2: u32 = 0 => "Arg2" Uint,
    arg3: u32 = 0 => "Arg3" Uint,
    /// +0x70..+0x7c (not registered; read by LoadFromStream)
    prim: [f32; 4] = [0.0; 4] => "" Vec4,
});

impl DTHotSpotDescriptor {
    pub const CLASS_NAME: &'static str = "DT_HOTSPOT_DESCR";
}

impl DTObject for DTHotSpotDescriptor {
    fn GetClassID(&self) -> u32 {
        Self::CLASS_ID
    }
    fn GetClassName(&self) -> &'static str {
        Self::CLASS_NAME
    }
    fn Members(&mut self) -> Vec<DTProperty<'_>> {
        let mut m = self.members();
        m.pop(); // `prim` is not a registered member
        m
    }
    fn LoadFromStream(&mut self, s: &mut dyn DTBasePersistStream) -> i32 {
        let r = DTStreamable_LoadFromStream(self, s);
        let n = match self.prim_type {
            2 => 3,
            1 => 4,
            0 => 1,
            _ => return -2,
        };
        for i in 0..n {
            s.LoadData_float(&mut self.prim[i]);
        }
        r
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

// ---------------------------------------------------------------------------------------------
// DTGlo

dt_class!(
    /// DTGlo : DTStreamable (size 0x84). Registration order and defaults per DTGlo::DTGlo.
    DTGlo, 1, "", {
    /// +0x8
    uid: u32 = UNSET => "UID" Uint,
    /// +0xc the logic code's UID
    glo_file_id: u32 = UNSET => "GLO_FILE_ID" Uint,
    /// +0x10
    glo_enable_bit_pos: u32 = UNSET => "GLO_ENABLE_BIT_POS" Uint,
    /// +0x40
    pos: [f32; 3] = [0.0; 3] => "POS" Vec3,
    /// +0x4c
    look_at: [f32; 3] = [0.0, 0.0, 1.0] => "LOOK_AT" Vec3,
    /// +0x58
    look_up: [f32; 3] = [0.0, 1.0, 0.0] => "LOOK_UP" Vec3,
    /// +0x64
    look_right: [f32; 3] = [1.0, 0.0, 0.0] => "LOOK_RIGHT" Vec3,
    /// +0x14
    object_flags: u32 = 0 => "OBJECT_FLAGS" Uint,
    /// +0x18
    shadows_flags: u32 = 0 => "SHADOWS_FLAGS" Uint,
    /// +0x1c
    object_descriptors: DTDynamicPropertyContainer = Default::default() => "OBJECT_DESCRIPTORS" Streamable,
    /// +0x80 (aliases magic_data.size)
    magic_size: u32 = 0 => "MAGIC_SIZE" Uint,
    /// +0x70
    magic_data: DTMagicData = Default::default() => "MAGIC_DATA" Streamable,
});

impl DTGlo {
    pub const CLASS_NAME: &'static str = "";
}
dt_object!(DTGlo);

// ---------------------------------------------------------------------------------------------
// DTDynamicPropertyContainer

/// DTDynamicPropertyContainer : DTStreamable (class 0x1f, size 0x20, +0xc DTVector<DTStreamable*>).
/// Binary: `int classId` then the object's Load, repeated until classId 0 (no UID is read).
/// Unknown class -> -6.
#[derive(Default)]
pub struct DTDynamicPropertyContainer {
    pub uid: u32,
    pub objects: Vec<Box<dyn DTObject>>,
}

impl std::fmt::Debug for DTDynamicPropertyContainer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.objects.iter().map(|o| o.GetClassName())).finish()
    }
}

impl Clone for DTDynamicPropertyContainer {
    /// Not in the C++ (containers are never copied); a clone keeps no objects.
    fn clone(&self) -> Self {
        Self { uid: self.uid, objects: Vec::new() }
    }
}

impl DTDynamicPropertyContainer {
    pub const CLASS_ID: u32 = 0x1f;
    pub const CLASS_NAME: &'static str = "DT_DYN_PROP_CONTAINER";

    /// Clear: deletes every object.
    pub fn Clear(&mut self) {
        self.objects.clear();
    }
    /// AddClass (vtbl 0x4c): push, returns the index.
    pub fn AddClass(&mut self, o: Box<dyn DTObject>) -> i32 {
        self.objects.push(o);
        self.objects.len() as i32 - 1
    }
    /// GetAt(i): NULL past the end.
    pub fn GetAt(&self, i: u32) -> Option<&dyn DTObject> {
        self.objects.get(i as usize).map(|b| b.as_ref())
    }
}

impl DTObject for DTDynamicPropertyContainer {
    fn GetClassID(&self) -> u32 {
        Self::CLASS_ID
    }
    fn GetClassName(&self) -> &'static str {
        Self::CLASS_NAME
    }
    fn Members(&mut self) -> Vec<DTProperty<'_>> {
        vec![prop("UID", DTVariant::Uint(&mut self.uid))]
    }
    /// Clear(); DTStreamable::LoadFromStream (-> the LoadProperties* overrides below).
    fn LoadFromStream(&mut self, s: &mut dyn DTBasePersistStream) -> i32 {
        self.Clear();
        DTStreamable_LoadFromStream(self, s)
    }
    fn LoadPropertiesBinary(&mut self, s: &mut dyn DTBasePersistStream) -> i32 {
        loop {
            let mut id = 0i32;
            let r = s.LoadData_int(&mut id);
            if r != 0 {
                return r;
            }
            if id == 0 {
                return 0;
            }
            let Some(o) = CreateInstance(id as u32) else { return -6 };
            // AddClass before Load (binary order)
            let i = self.AddClass(o);
            let r = self.objects[i as usize].Load(s);
            if r != 0 {
                return r;
            }
        }
    }
    /// text: class-name tokens until "END"; Load then AddClass (text order).
    fn LoadPropertiesText(&mut self, tok: &mut Vec<u8>, s: &mut dyn DTBasePersistStream) -> i32 {
        loop {
            let r = s.LoadString(tok, 0x800);
            if r != 0 {
                return r;
            }
            if tok == b"END" {
                return 0;
            }
            let name = String::from_utf8_lossy(tok).into_owned();
            let Some(mut o) = super::types::CLASS_TABLE.iter().find(|e| e.0 == name).and_then(|e| CreateInstance(e.1)) else {
                return -6;
            };
            let r = o.Load(s);
            if r != 0 {
                return r;
            }
            self.AddClass(o);
        }
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

// ---------------------------------------------------------------------------------------------
// EF*IDescriptor wrappers (EFBaseIDescriptor : DTStreamable, embedded DT descriptor at +0xc).
// LoadFromStream(s) = GetDescriptor()->LoadFromStream(s). GetInstanceCLSID = the class the
// level builder instanciates from the descriptor (EFBaseIDescriptor::CreateObjectInstance /
// CreateNode).

/// EFBaseIDescriptor family: one wrapper type, the C++ subclass differs only by ids, name and the
/// embedded descriptor type.
pub struct EFIDescriptor<D> {
    /// the wrapper's own DTStreamable UID (+0x8; never read from the stream)
    pub uid: u32,
    pub class_id: u32,
    pub class_name: &'static str,
    /// GetInstanceCLSID
    pub instance_clsid: u32,
    /// GetDescriptor() (+0xc)
    pub desc: D,
}

/// Object-safe view of a wrapper, for the level builder.
pub trait EFBaseIDescriptor: DTObject {
    fn GetInstanceCLSID(&self) -> u32;
    /// GetDescriptor()'s UID (+0x8) / PUID (+0xc)
    fn DescUID(&self) -> u32;
    fn DescPUID(&self) -> i32;
}

macro_rules! ef_idesc {
    ($d:ty) => {
        impl DTObject for EFIDescriptor<$d> {
            fn GetClassID(&self) -> u32 { self.class_id }
            fn GetClassName(&self) -> &'static str { self.class_name }
            fn Members(&mut self) -> Vec<DTProperty<'_>> { vec![prop("UID", DTVariant::Uint(&mut self.uid))] }
            fn LoadFromStream(&mut self, s: &mut dyn DTBasePersistStream) -> i32 { DTObject::LoadFromStream(&mut self.desc, s) }
            fn as_any(&self) -> &dyn std::any::Any { self }
            fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
        }
        impl EFBaseIDescriptor for EFIDescriptor<$d> {
            fn GetInstanceCLSID(&self) -> u32 { self.instance_clsid }
            fn DescUID(&self) -> u32 { self.desc.uid }
            fn DescPUID(&self) -> i32 { self.desc.puid }
        }
    };
}

macro_rules! dt_object_named {
    ($($name:ident),*) => { $(
        impl $name { pub const CLASS_NAME: &'static str = ""; }
        dt_object!($name);
    )* };
}
dt_object_named!(DTBaseInstanceDescriptor, DTPosObjectDescriptor, DT3dObjectDescriptor, DTAnimDescriptor, DTLightDescriptor, DTSoundDescriptor, DTSoundContainerDescriptor, DTTextureAnimationDescriptor, DTLoadingScreenDescriptor);

ef_idesc!(DTBaseInstanceDescriptor);
ef_idesc!(DTPosObjectDescriptor);
ef_idesc!(DT3dObjectDescriptor);
ef_idesc!(DTAnimDescriptor);
ef_idesc!(DTLightDescriptor);
ef_idesc!(DTSoundDescriptor);
ef_idesc!(DTSoundContainerDescriptor);
ef_idesc!(DTTextureAnimationDescriptor);

fn wrap<D: Default>(class_id: u32, class_name: &'static str, instance_clsid: u32) -> EFIDescriptor<D> {
    EFIDescriptor { uid: UNSET, class_id, class_name, instance_clsid, desc: D::default() }
}

/// DTDynamicInstanciator::CreateInstance(classId) for the descriptor classes (type-1 registered
/// classes whose objects are plain data). Engine classes with behaviour (EFGameObject, logic code,
/// resources...) are created by [`super::super::level::CreateInstance`], which falls back to this.
pub fn CreateDescriptorInstance(id: u32) -> Option<Box<dyn DTObject>> {
    Some(match id {
        0x02 => Box::new(wrap::<DT3dStaticObjectDescriptor>(0x02, "3D_STATIC_OBJECT_DESCR", 0x18)),
        0x03 => Box::new(wrap::<DT3dObjectDescriptor>(0x03, "3D_OBJECT_DESCR", 0x0f)),
        0x06 => Box::new(wrap::<DTLightDescriptor>(0x06, "LIGHT_DESCR", 0x12)),
        0x07 => Box::new(wrap::<DTSoundDescriptor>(0x07, "SOUND_DESCR", 0x17)),
        0x08 => Box::new(wrap::<DTBaseInstanceDescriptor>(0x08, "MORPH_ANIM_DESCR", 0x51)),
        0x0b => Box::new(wrap::<DTTextureAnimationDescriptor>(0x0b, "TEXTURE_ANIM_DESCR", 0x54)),
        0x0c => Box::new(wrap::<DTPosObjectDescriptor>(0x0c, "CAMERA_DESCR", 0x10)),
        0x33 => Box::new(wrap::<DTPosObjectDescriptor>(0x33, "INVISIBLE_DESCR", 0x34)),
        0x37 => Box::new(wrap::<DTHAnimationDescriptor>(0x37, "EI_HANIMATION_DESCR", 0x39)),
        0x4b => Box::new(wrap::<DTAnimDescriptor>(0x4b, "DT_ANIM_NODE_DESCR", 0x4a)),
        0x4d => Box::new(wrap::<DTBaseInstanceDescriptor>(0x4d, "DT_LIGHT_ANIM_DESCR", 0x4e)),
        0x55 => Box::new(wrap::<DTBaseInstanceDescriptor>(0x55, "DT_CAMERA_ANIM_DESCR", 0x53)),
        0x58 => Box::new(wrap::<DTSoundContainerDescriptor>(0x58, "SOUND_CONTAINER_DESCR", 0x59)),
        0x30 => Box::new(DTWaypointGrp::default()),
        0x3f => Box::new(DTHotSpotDescriptor::default()),
        0x1f => Box::new(DTDynamicPropertyContainer::default()),
        _ => return None,
    })
}

/// Hook used by DTDynamicPropertyContainer: the engine-level factory.
pub fn CreateInstance(id: u32) -> Option<Box<dyn DTObject>> {
    crate::engine::level::CreateInstance(id)
}

/// Downcast helper.
pub fn as_idesc(o: &dyn DTObject) -> Option<&dyn EFBaseIDescriptor> {
    let a = o.as_any();
    macro_rules! try_ { ($($d:ty),*) => { $( if let Some(x) = a.downcast_ref::<EFIDescriptor<$d>>() { return Some(x); } )* }; }
    try_!(DTBaseInstanceDescriptor, DTPosObjectDescriptor, DT3dObjectDescriptor, DTAnimDescriptor, DTLightDescriptor, DTSoundDescriptor, DTSoundContainerDescriptor, DTTextureAnimationDescriptor);
    None
}

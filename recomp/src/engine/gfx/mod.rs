//! EF 3D resources, data side (Classes/EF/EF3dObjRes*, EFStatic3dObj, EFInstance, EF3dObj,
//! EFTextureDict, EFTextureAnimRes, EFPaletteDictionary). Game-specific code is 1:1 in load order
//! and field semantics; the RenderWare objects they own are the plain-data readers of `rw/`.
//! Rendering (RxPipelineExecute, render callbacks, stencil, tex fader) is a TODO hook: the
//! coordinator builds Bevy meshes from `RpClump`/`RpWorld` data.
#![allow(non_snake_case)]

pub mod hierarchy;
pub mod mgr;
pub mod render;
pub mod texfader;
pub mod sprite;
pub mod motion_trail;
pub mod projector;
pub mod shadow;
pub mod api;
#[cfg(feature = "bevy_render")]
pub mod bevy_adapter;

use crate::engine::dt::descriptors::DTEmitterContainerDescriptor;
use crate::engine::dt::property::DTStreamable_LoadFromStream;
use crate::engine::dt::stream::DTBinaryPersistStream;
use crate::engine::error::EngineError;
use crate::engine::rw::clump::{RpClump, RpLight};
use crate::engine::rw::image::{LoadTextureDictionary, RwTexDictionary};
use crate::engine::rw::stream::*;
use crate::engine::rw::world::RpWorld;

fn rd_u32(b: &[u8], o: usize) -> Result<u32, EngineError> {
    b.get(o..o + 4)
        .map(|x| u32::from_le_bytes(x.try_into().unwrap()))
        .ok_or_else(|| EngineError::Format("short resource header".into()))
}

/// EFRessource base: this+0x8 uid, this+0xc refcount.
#[derive(Debug, Clone, Default)]
pub struct EFRessource {
    /// this+0x8
    pub uid: u32,
    /// this+0xc
    pub ref_count: u32,
}

/// EF3dObjResData (EFRessource; class 0x5b via EF3dObjRes "EF_3DOBJ_RES").
#[derive(Default)]
pub struct EF3dObjResData {
    pub res: EFRessource,
    /// second header word (forced to 1 by LoadPropertiesBinary): RegisterRessource arg
    pub res_type: u32,
    /// this+0x10 RpClump*
    pub clump: Option<RpClump>,
    /// this+0x14 version read after the clump (0 when missing)
    pub version: i32,
    /// this+0x18 DTEmitterContainerDescriptor*
    pub emitters: Option<DTEmitterContainerDescriptor>,
}

pub type EF3dObjRes = EF3dObjResData;

impl EF3dObjResData {
    pub fn GetClassID(&self) -> u32 {
        0x5b
    }
    pub fn GetClassName(&self) -> &'static str {
        "EF_3DOBJ_RES"
    }
    pub fn GetClump(&self) -> Option<&RpClump> {
        self.clump.as_ref()
    }

    /// LoadPropertiesBinary over one pack entry. Returns bytes consumed.
    /// RegisterRessource(uid, 1, this) is left to the caller (EFRessourceFactory not in WP-C).
    pub fn LoadPropertiesBinary(&mut self, b: &[u8]) -> Result<usize, EngineError> {
        self.res.uid = rd_u32(b, 0)?;
        let _ = rd_u32(b, 4)?;
        self.res_type = 1; // quirk: the read value is overwritten with 1
        let n = self.Load3dObj(&b[8..])?;
        Ok(8 + n)
    }

    /// Load3dObj (RwStreamFindChunk(clump) / RpClumpStreamRead / ReadInt32 version; for
    /// version > 0x5405 an i32 marker and, if != -1, an emitter container descriptor).
    pub fn Load3dObj(&mut self, b: &[u8]) -> Result<usize, EngineError> {
        let mut s = RwStream::new(b);
        if let Some(h) = s.FindChunk(rwID_CLUMP) {
            let mut c = s.sub(h.size as usize)?;
            self.clump = Some(RpClump::StreamRead(&mut c)?);
            self.version = s.i32().unwrap_or(0);
        }
        let mut used = b.len() - s.remaining();
        if (self.version as u32 & 0xffff) > 0x5405 {
            let marker = rd_u32(b, used)? as i32;
            used += 4;
            if marker != -1 {
                let mut ps = DTBinaryPersistStream::new(b[used..].to_vec());
                let mut e = DTEmitterContainerDescriptor::default();
                DTStreamable_LoadFromStream(&mut e, &mut ps);
                used += ps.Tell();
                self.emitters = Some(e);
            }
        }
        Ok(used)
    }
}

/// A static light of the level world (EFStatic3dObj::ReadLights): EFLightMgr_BC::CreateStaticLight
/// with uid (i + 0x800000) | 0x12000000, colour/flags/radius/cone copied from the streamed light,
/// followed by the frame matrix (12 reals: right, up, at, pos).
#[derive(Debug, Clone)]
pub struct EFStaticLight {
    pub uid: u32,
    pub light: RpLight,
    pub matrix: [f32; 12],
}

/// EFStatic3dObj : EFInstance (class 0x18 "STATIC_3D_OBJECT_INST"): the level BSP world.
#[derive(Default)]
pub struct EFStatic3dObj {
    pub instance: EFInstance,
    /// this+0x8 uid ((uid & 0xffffff) | 0x18000000)
    pub uid: u32,
    /// this+0x14 RpWorld*
    pub world: Option<RpWorld>,
    pub lights: Vec<EFStaticLight>,
    /// this+0x1c trailing int (0 when missing)
    pub extra: i32,
}

impl EFStatic3dObj {
    pub fn GetClassID(&self) -> u32 {
        0x18
    }
    pub fn GetClassName(&self) -> &'static str {
        "STATIC_3D_OBJECT_INST"
    }

    /// LoadPropertiesBinary over one pack entry (EFWorld_SP::SetStatic3dObj left to the caller).
    pub fn LoadPropertiesBinary(&mut self, b: &[u8]) -> Result<usize, EngineError> {
        self.uid = (rd_u32(b, 0)? & 0xff_ffff) | 0x1800_0000;
        let _ = rd_u32(b, 4)?;
        let mut s = RwStream::new(&b[8..]);
        self.LoadStatic3dObjFromStream(&mut s)?;
        Ok(b.len() - s.remaining())
    }

    pub fn LoadStatic3dObjFromStream(&mut self, s: &mut RwStream) -> Result<(), EngineError> {
        let h = s.FindChunk(rwID_WORLD).ok_or_else(|| EngineError::Format("no world chunk".into()))?;
        let mut w = s.sub(h.size as usize)?;
        let mut world = RpWorld::StreamRead(&mut w)?;
        self.ReadLights(s)?;
        self.extra = s.i32().unwrap_or(0);
        // FixAlphaSectorCallBack: prelit alpha forced to 0xff on every sector vertex
        for sec in world.sectors.iter_mut() {
            for m in sec.meshes.iter_mut() {
                for c in m.mesh.colors.iter_mut() {
                    c[3] = 0xff;
                }
            }
        }
        // TODO(render): EFProjectorMgr_SP::SetWorldSectorRenderCallback
        self.world = Some(world);
        Ok(())
    }

    pub fn ReadLights(&mut self, s: &mut RwStream) -> Result<bool, EngineError> {
        let n = s.i32()?;
        for i in 0..n.max(0) as u32 {
            let Some(h) = s.FindChunk(rwID_LIGHT) else { return Ok(false) };
            let mut l = s.sub(h.size as usize)?;
            let light = RpLight::StreamRead(&mut l)?;
            let mut matrix = [0f32; 12];
            for m in matrix.iter_mut() {
                *m = s.f32()?;
            }
            self.lights.push(EFStaticLight { uid: (i + 0x80_0000) | 0x1200_0000, light, matrix });
        }
        Ok(true)
    }

    pub fn Terminate(&mut self) {
        self.lights.clear();
        self.world = None;
    }
}

/// EFInstance : DTStreamable (+0xc, +0x10 zeroed). All virtuals are no-ops.
#[derive(Debug, Clone, Default)]
pub struct EFInstance {
    /// this+0xc
    pub field_c: u32,
    /// this+0x10
    pub field_10: u32,
}

impl EFInstance {
    pub fn SetRessource(&mut self, _r: u32) {}
    pub fn Initialize(&mut self) -> i32 {
        1
    }
    pub fn Terminate(&mut self) {}
    pub fn ShallowCopy(&self) -> Option<EFInstance> {
        None
    }
}

/// EFTextureDict (class 0x22 "TEXTURE_DICTIONARY"): loads image chunks into the current
/// RwTexDictionary (singleton 0x1f83dd8 -> owned by the caller here).
pub struct EFTextureDict;

impl EFTextureDict {
    pub fn GetClassID() -> u32 {
        0x22
    }
    pub fn GetClassName() -> &'static str {
        "TEXTURE_DICTIONARY"
    }
    /// LoadPropertiesBinary -> LoadTextureDictionary; 0 ok, 0xfffffffe error.
    pub fn LoadPropertiesBinary(b: &[u8], dict: &mut RwTexDictionary) -> u32 {
        let mut s = RwStream::new(b);
        LoadTextureDictionary(&mut s, dict)
    }
    /// Clear: destroy the current dictionary and make a new one current.
    pub fn Clear(dict: &mut RwTexDictionary) {
        *dict = RwTexDictionary::default();
    }
}

/// EFTextureAnimRes (class 0x2c "EI_TEXTURE_ANIM_RES"): +0x10 count, +0x14 {texture*, float}[].
#[derive(Debug, Clone, Default)]
pub struct EFTextureAnimRes {
    pub res: EFRessource,
    /// (texture index in the dictionary or None when not found, frame time)
    pub frames: Vec<(Option<usize>, f32, String)>,
}

impl EFTextureAnimRes {
    pub fn GetClassID(&self) -> u32 {
        0x2c
    }
    pub fn GetClassName(&self) -> &'static str {
        "EI_TEXTURE_ANIM_RES"
    }
    /// LoadTextAnim for the binary stream: u32 count, then per frame f32 time + string
    /// (LoadTextData: u32 length-prefixed bytes in binary DT streams).
    pub fn LoadPropertiesBinary(&mut self, b: &[u8], dict: &RwTexDictionary) -> Result<usize, EngineError> {
        self.res.uid = rd_u32(b, 0)?;
        let _type = rd_u32(b, 4)?;
        let mut s = RwStream::new(&b[8..]);
        let n = s.u32()?;
        for _ in 0..n {
            let t = s.f32()?;
            let len = s.u32()? as usize;
            let name = cstr(s.Read(len)?);
            // RwTexDictionaryFindNamedTexture + refcount++
            self.frames.push((dict.FindNamedTexture(&name), t, name));
        }
        Ok(b.len() - s.remaining())
    }
}

/// EFPaletteDictionary (class 0x57 "EF_PALETTE_DICTIONARY") lookup:
/// GetPalette(name, i) = base + entry(name).offsets[i] (DTstricmp on the names).
#[derive(Debug, Clone, Default)]
pub struct EFPaletteDictionary {
    /// this+0x18 palette data
    pub data: Vec<u8>,
    /// this+0x24 entries (+0x4 name, +0xc offsets), count this+0x20
    pub entries: Vec<(String, Vec<u32>)>,
}

impl EFPaletteDictionary {
    pub fn GetClassID(&self) -> u32 {
        0x57
    }
    pub fn GetClassName(&self) -> &'static str {
        "EF_PALETTE_DICTIONARY"
    }
    /// Returns the byte offset in `data` (the original returns a pointer, 0 when not found).
    pub fn GetPalette(&self, name: &str, idx: u32) -> Option<u32> {
        let e = self.entries.iter().find(|e| e.0.eq_ignore_ascii_case(name))?;
        e.1.get(idx as usize).copied()
    }
}

/// EF3dObj (class 0xf "3D_OBJECT_INST") data side: transfo pool flags and resource link.
#[derive(Debug, Clone)]
pub struct EF3dObj {
    /// EF3dObj_BC this+0x28 resource uid
    pub ressource: Option<u32>,
    /// EF3dObj_BC this+0x2c flags (init 2; |0x10 / 0x100 / 0x200 from creation flags 8/0x20/0x40;
    /// 4 = has HAnim hierarchy)
    pub flags: u32,
    /// this+0x4c free-transfo bit mask (bit 1 set after ctor)
    pub free_transfos: u32,
    /// this+0x58 (0x20)
    pub field_58: u32,
    /// this+0x68 fading (non-zero -> EFTexFader_SP::FadeAtomic)
    pub fading: u32,
    /// joint count of the HAnim hierarchy (this+0x34 +4)
    pub num_nodes: u32,
}

impl Default for EF3dObj {
    fn default() -> Self {
        // ctor loop: for i in 1..2 { mask |= 1<<i }
        EF3dObj { ressource: None, flags: 2, free_transfos: 2, field_58: 0x20, fading: 0, num_nodes: 0 }
    }
}

impl EF3dObj {
    pub fn GetClassID(&self) -> u32 {
        0xf
    }
    pub fn GetClassName(&self) -> &'static str {
        "3D_OBJECT_INST"
    }
    pub fn SetRessource(&mut self, r: u32) {
        self.ressource = Some(r);
    }
    pub fn GetRessource(&self) -> Option<u32> {
        self.ressource
    }
    /// Creation flag mapping of Initialize (descriptor +0x20): 8 -> 0x10, else 0x20 -> 0x100,
    /// else 0x40 -> 0x200.
    pub fn ApplyCreationFlags(&mut self, f: u32) {
        if f & 8 != 0 {
            self.flags |= 0x10;
        } else if f & 0x20 != 0 {
            self.flags |= 0x100;
        } else if f & 0x40 != 0 {
            self.flags |= 0x200;
        }
    }
    /// Returns the transfo slot (1) or None (0).
    pub fn LockTransfo(&mut self) -> Option<u32> {
        let mut bit = 2;
        let mut i = 1;
        while self.free_transfos != 0 && i < 2 {
            if bit & self.free_transfos != 0 {
                self.free_transfos &= !bit;
                return Some(i);
            }
            i += 1;
            bit <<= 1;
        }
        None
    }
    pub fn ReleaseTransfo(&mut self, slot: u32) {
        if slot == 1 {
            self.free_transfos |= 1 << slot;
        }
    }
    /// GetJointIndex: needs flag 4; ids with high bits go through RpHAnimIDGetIndex.
    pub fn GetJointIndex(&self, id: u32, hanim_ids: &[i32]) -> u32 {
        if self.flags & 4 == 0 {
            return 0xffff_ffff;
        }
        if id & 0xffff_0000 == 0 {
            if id < self.num_nodes { id } else { 0xffff_ffff }
        } else {
            hanim_ids.iter().position(|&x| x as u32 == id).map(|p| p as u32).unwrap_or(0xffff_ffff)
        }
    }
    /// LightObject: sets/clears rpGEOMETRYLIGHT (0x20) on every atomic geometry of the clump.
    pub fn LightObject(clump: &mut RpClump, on: bool) {
        for g in clump.geometries.iter_mut() {
            if on {
                g.flags |= 0x20;
            } else {
                g.flags &= !0x20;
            }
        }
    }
    // TODO(render): Render / SetClippingBSphere / InitHierarchy are wired by the coordinator.
}

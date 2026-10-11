//! RpMaterial / RpMatList / RpGeometry / RpMesh (bin mesh) / RpSkin stream readers (replacement
//! for RenderWare internals, not 1:1). Output is plain data a Bevy Mesh can be built from.
#![allow(non_snake_case)]

use super::stream::*;
use crate::engine::error::EngineError;

/// RpGeometryFlag
pub const rpGEOMETRYTRISTRIP: u32 = 0x01;
pub const rpGEOMETRYPOSITIONS: u32 = 0x02;
pub const rpGEOMETRYTEXTURED: u32 = 0x04;
pub const rpGEOMETRYPRELIT: u32 = 0x08;
pub const rpGEOMETRYNORMALS: u32 = 0x10;
pub const rpGEOMETRYLIGHT: u32 = 0x20;
pub const rpGEOMETRYMODULATEMATERIALCOLOR: u32 = 0x40;
pub const rpGEOMETRYTEXTURED2: u32 = 0x80;
pub const rpGEOMETRYNATIVE: u32 = 0x0100_0000;

/// Texture reference of a material (rwID_TEXTURE): the texture itself is looked up by name in
/// the current RwTexDictionary when the material is read (RwTextureRead).
#[derive(Debug, Clone, Default)]
pub struct RpTextureRef {
    /// struct u32: filter mode (low byte), addressing U (bits 8-11), V (12-15), mipmap flag (16)
    pub filter_addressing: u32,
    pub name: String,
    pub mask: String,
    /// rwID_SKYMIPMAP plugin value (PS2 mipmap K/L), if present
    pub sky_mipmap: Option<u32>,
}

#[derive(Debug, Clone, Default)]
pub struct RpMaterial {
    pub flags: u32,
    /// RwRGBA
    pub color: [u8; 4],
    pub ambient: f32,
    pub specular: f32,
    pub diffuse: f32,
    pub texture: Option<RpTextureRef>,
    pub ext: Extension,
}

impl RpMaterial {
    /// RpMaterialStreamRead (stream at the rwID_MATERIAL payload)
    pub fn StreamRead(s: &mut RwStream) -> Result<RpMaterial, EngineError> {
        s.expect(rwID_STRUCT)?;
        let flags = s.u32()?;
        let c = s.Read(4)?;
        let color = [c[0], c[1], c[2], c[3]];
        let _unused = s.u32()?;
        let textured = s.i32()?;
        let (ambient, specular, diffuse) = (s.f32()?, s.f32()?, s.f32()?);
        let mut texture = None;
        if textured != 0 {
            let h = s.expect(rwID_TEXTURE)?;
            let mut t = s.sub(h.size as usize)?;
            t.expect(rwID_STRUCT)?;
            let filter_addressing = t.u32()?;
            let name = t.string_chunk()?;
            let mask = t.string_chunk()?;
            let ext = Extension::read(&mut t)?;
            let sky_mipmap = ext.get(rwID_SKYMIPMAP).and_then(|b| b.get(..4)).map(|b| u32::from_le_bytes(b.try_into().unwrap()));
            texture = Some(RpTextureRef { filter_addressing, name, mask, sky_mipmap });
        }
        let ext = Extension::read(s)?;
        Ok(RpMaterial { flags, color, ambient, specular, diffuse, texture, ext })
    }
}

/// _rpMaterialListStreamRead: struct {numMaterials, i32 index[numMaterials]} (-1 = a new
/// material chunk follows, else a reference to an earlier entry), then the material chunks.
pub fn ReadMaterialList(s: &mut RwStream) -> Result<Vec<RpMaterial>, EngineError> {
    s.expect(rwID_MATLIST)?;
    s.expect(rwID_STRUCT)?;
    let n = s.u32()? as usize;
    let idx: Vec<i32> = (0..n).map(|_| s.i32()).collect::<Result<_, _>>()?;
    let mut mats: Vec<RpMaterial> = Vec::with_capacity(n);
    for &i in &idx {
        if i < 0 {
            let h = s.expect(rwID_MATERIAL)?;
            let mut m = s.sub(h.size as usize)?;
            mats.push(RpMaterial::StreamRead(&mut m)?);
        } else {
            let m = mats.get(i as usize).cloned().ok_or_else(|| EngineError::Format("matlist ref".into()))?;
            mats.push(m);
        }
    }
    Ok(mats)
}

/// RpMesh: one material's index list (strip or list depending on the header flags).
#[derive(Debug, Clone, Default)]
pub struct RpMesh {
    pub material: u32,
    pub num_indices: u32,
    /// empty for native (PS2) data: the vertices are in the DMA packets instead
    pub indices: Vec<u32>,
}

/// RpMeshHeader (rwID_BINMESH plugin): flags (1 = rpMESHHEADERTRISTRIP), meshes.
#[derive(Debug, Clone, Default)]
pub struct RpMeshHeader {
    pub flags: u32,
    pub total_indices: u32,
    pub meshes: Vec<RpMesh>,
}

pub const rpMESHHEADERTRISTRIP: u32 = 1;

impl RpMeshHeader {
    /// _rpBinaryMeshStreamRead: u32 flags, numMeshes, totalIndices; per mesh u32 numIndices,
    /// matIndex, then (non-native only) numIndices u32 indices.
    pub fn read(b: &[u8], native: bool) -> Result<RpMeshHeader, EngineError> {
        let mut s = RwStream::new(b);
        let flags = s.u32()?;
        let n = s.u32()?;
        let total_indices = s.u32()?;
        let mut meshes = Vec::with_capacity(n as usize);
        for _ in 0..n {
            let num_indices = s.u32()?;
            let material = s.u32()?;
            let indices = if native { Vec::new() } else { (0..num_indices).map(|_| s.u32()).collect::<Result<_, _>>()? };
            meshes.push(RpMesh { material, num_indices, indices });
        }
        Ok(RpMeshHeader { flags, total_indices, meshes })
    }
    pub fn is_strip(&self) -> bool {
        self.flags & rpMESHHEADERTRISTRIP != 0
    }
}

/// Triangle strip -> triangle list. Degenerate triangles (two equal indices, how RW joins
/// strips) are dropped; odd triangles are flipped to keep the winding.
pub fn strip_to_list(strip: &[u32]) -> Vec<u32> {
    let mut out = Vec::new();
    for i in 2..strip.len() {
        let (a, b, c) = (strip[i - 2], strip[i - 1], strip[i]);
        if a == b || b == c || a == c {
            continue;
        }
        if i % 2 == 0 {
            out.extend_from_slice(&[a, b, c]);
        } else {
            out.extend_from_slice(&[b, a, c]);
        }
    }
    out
}

/// RpTriangle {u16 vertIndex[3], u16 matIndex}
#[derive(Debug, Clone, Copy, Default)]
pub struct RpTriangle {
    pub v: [u16; 3],
    pub mat: u16,
}

#[derive(Debug, Clone, Default)]
pub struct RpMorphTarget {
    /// RwSphere {center, radius}
    pub bsphere: [f32; 4],
    pub verts: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
}

/// RpSkin (rwID_SKINPLUGIN on a non-native geometry)
#[derive(Debug, Clone, Default)]
pub struct RpSkin {
    pub num_bones: u32,
    pub max_weights: u32,
    pub used_bones: Vec<u8>,
    /// per vertex 4 bone indices
    pub indices: Vec<[u8; 4]>,
    /// per vertex 4 weights
    pub weights: Vec<[f32; 4]>,
    /// per bone inverse bind (RwMatrix 4x4 row-major as streamed: right, up, at, pos with w)
    pub inverse_matrices: Vec<[f32; 16]>,
    /// split data (3.4): boneLimit, numMeshes, numRLE + remap/RLE tables (kept raw)
    pub bone_limit: u32,
    pub num_meshes: u32,
    pub num_rle: u32,
    pub split_tables: Vec<u8>,
}

impl RpSkin {
    /// _rpSkinGeometryStreamRead (generic, non-native layout, RW >= 3.4.0.3: no per-matrix marker)
    pub fn read(b: &[u8], num_verts: usize) -> Result<RpSkin, EngineError> {
        let mut s = RwStream::new(b);
        let num_bones = s.u8()? as u32;
        let num_used = s.u8()? as usize;
        let max_weights = s.u8()? as u32;
        s.u8()?;
        let used_bones = s.Read(num_used)?.to_vec();
        let indices = (0..num_verts).map(|_| s.Read(4).map(|c| [c[0], c[1], c[2], c[3]])).collect::<Result<_, _>>()?;
        let weights = (0..num_verts).map(|_| Ok([s.f32()?, s.f32()?, s.f32()?, s.f32()?])).collect::<Result<_, EngineError>>()?;
        let mut inverse_matrices = Vec::with_capacity(num_bones as usize);
        for _ in 0..num_bones {
            let mut m = [0f32; 16];
            for v in m.iter_mut() {
                *v = s.f32()?;
            }
            inverse_matrices.push(m);
        }
        let (mut bone_limit, mut num_meshes, mut num_rle) = (0, 0, 0);
        if s.remaining() >= 12 {
            bone_limit = s.u32()?;
            num_meshes = s.u32()?;
            num_rle = s.u32()?;
        }
        let split_tables = s.Read(s.remaining())?.to_vec();
        Ok(RpSkin { num_bones, max_weights, used_bones, indices, weights, inverse_matrices, bone_limit, num_meshes, num_rle, split_tables })
    }
}

#[derive(Debug, Clone, Default)]
pub struct RpGeometry {
    pub flags: u32,
    pub num_tex_sets: u32,
    pub num_triangles: u32,
    pub num_vertices: u32,
    pub prelit: Vec<[u8; 4]>,
    pub uvs: Vec<Vec<[f32; 2]>>,
    pub triangles: Vec<RpTriangle>,
    pub morph_targets: Vec<RpMorphTarget>,
    pub materials: Vec<RpMaterial>,
    pub mesh_header: Option<RpMeshHeader>,
    pub skin: Option<RpSkin>,
    pub ext: Extension,
}

/// One material's triangle list (indices into the geometry vertex arrays).
#[derive(Debug, Clone, Default)]
pub struct SubMesh {
    pub material: u32,
    pub indices: Vec<u32>,
}

impl RpGeometry {
    /// RpGeometryStreamRead (stream at the rwID_GEOMETRY payload)
    pub fn StreamRead(s: &mut RwStream) -> Result<RpGeometry, EngineError> {
        let h = s.expect(rwID_STRUCT)?;
        let mut st = s.sub(h.size as usize)?;
        let raw_flags = st.u32()?;
        let flags = raw_flags & 0x0000_00ff | raw_flags & rpGEOMETRYNATIVE;
        let mut num_tex_sets = (raw_flags >> 16) & 0xff;
        if num_tex_sets == 0 {
            num_tex_sets = if flags & rpGEOMETRYTEXTURED2 != 0 {
                2
            } else if flags & rpGEOMETRYTEXTURED != 0 {
                1
            } else {
                0
            };
        }
        let num_triangles = st.u32()?;
        let num_vertices = st.u32()?;
        let num_morph = st.u32()?;
        if h.version() < 0x34000 {
            st.Skip(12)?; // surface properties (pre-3.4 streams only)
        }
        let nv = num_vertices as usize;
        let mut g = RpGeometry { flags, num_tex_sets, num_triangles, num_vertices, ..Default::default() };
        if flags & rpGEOMETRYNATIVE == 0 {
            if flags & rpGEOMETRYPRELIT != 0 {
                g.prelit = (0..nv).map(|_| st.Read(4).map(|c| [c[0], c[1], c[2], c[3]])).collect::<Result<_, _>>()?;
            }
            for _ in 0..num_tex_sets {
                g.uvs.push((0..nv).map(|_| Ok([st.f32()?, st.f32()?])).collect::<Result<_, EngineError>>()?);
            }
            for _ in 0..num_triangles {
                // two packed words: (v0 << 16 | v1), (v2 << 16 | matIndex)
                let w0 = st.u32()?;
                let w1 = st.u32()?;
                g.triangles.push(RpTriangle { v: [(w0 >> 16) as u16, w0 as u16, (w1 >> 16) as u16], mat: w1 as u16 });
            }
        }
        for _ in 0..num_morph {
            let bsphere = [st.f32()?, st.f32()?, st.f32()?, st.f32()?];
            let has_verts = st.u32()?;
            let has_normals = st.u32()?;
            let mut mt = RpMorphTarget { bsphere, ..Default::default() };
            if has_verts != 0 {
                mt.verts = (0..nv).map(|_| st.v3()).collect::<Result<_, _>>()?;
            }
            if has_normals != 0 {
                mt.normals = (0..nv).map(|_| st.v3()).collect::<Result<_, _>>()?;
            }
            g.morph_targets.push(mt);
        }
        g.materials = ReadMaterialList(s)?;
        g.ext = Extension::read(s)?;
        let native = flags & rpGEOMETRYNATIVE != 0;
        if let Some(b) = g.ext.get(rwID_BINMESH) {
            g.mesh_header = Some(RpMeshHeader::read(b, native)?);
        }
        if let Some(b) = g.ext.get(rwID_SKINPLUGIN) {
            if !native {
                g.skin = Some(RpSkin::read(b, nv)?);
            }
        }
        Ok(g)
    }

    /// Per-material triangle lists: from the bin mesh (strips converted), else from the
    /// triangle array.
    pub fn submeshes(&self) -> Vec<SubMesh> {
        if let Some(mh) = &self.mesh_header {
            if mh.meshes.iter().any(|m| !m.indices.is_empty()) {
                return mh
                    .meshes
                    .iter()
                    .map(|m| SubMesh {
                        material: m.material,
                        indices: if mh.is_strip() { strip_to_list(&m.indices) } else { m.indices.clone() },
                    })
                    .collect();
            }
        }
        let mut out: Vec<SubMesh> =
            (0..self.materials.len() as u32).map(|material| SubMesh { material, indices: Vec::new() }).collect();
        for t in &self.triangles {
            if let Some(sm) = out.get_mut(t.mat as usize) {
                sm.indices.extend(t.v.iter().map(|&v| v as u32));
            }
        }
        out
    }
}

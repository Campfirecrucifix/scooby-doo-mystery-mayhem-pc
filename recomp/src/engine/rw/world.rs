//! RpWorld / RpWorldSector / plane sectors (BSP) stream reader with PS2 native sector data
//! decoded to vertex arrays (replacement for RpWorldStreamRead + the PS2 world pipeline).
#![allow(non_snake_case)]

use super::geometry::{ReadMaterialList, RpMaterial, RpMeshHeader};
use super::ps2::{read_native_blocks, NativeMesh};
use super::stream::*;
use crate::engine::error::EngineError;

/// One material's decoded vertices in a world sector.
#[derive(Debug, Clone, Default)]
pub struct SectorMesh {
    /// index into RpWorld::materials (bin-mesh matIndex)
    pub material: u32,
    pub mesh: NativeMesh,
    /// triangle list over `mesh` vertices
    pub indices: Vec<u32>,
}

/// RpWorldSector (struct: matListWindowBase, numTriangles, numVertices, bbox inf, sup,
/// collSectorPresent, unused).
#[derive(Debug, Clone, Default)]
pub struct RpWorldSector {
    pub mat_list_window_base: u32,
    pub num_triangles: u32,
    pub num_vertices: u32,
    pub bbox_inf: [f32; 3],
    pub bbox_sup: [f32; 3],
    pub mesh_header: Option<RpMeshHeader>,
    pub meshes: Vec<SectorMesh>,
    pub ext: Extension,
}

/// BSP node (RpPlaneSector: type 0/4/8 = x/y/z, value, left/right children, left/right values)
#[derive(Debug, Clone)]
pub enum RpSector {
    Plane { plane_type: u32, value: f32, left_value: f32, right_value: f32, left: Box<RpSector>, right: Box<RpSector> },
    /// index into RpWorld::sectors
    Atomic(usize),
}

#[derive(Debug, Clone)]
pub struct RpWorld {
    pub root_is_world_sector: bool,
    pub inv_world_origin: [f32; 3],
    pub num_triangles: u32,
    pub num_vertices: u32,
    pub num_plane_sectors: u32,
    pub num_world_sectors: u32,
    pub col_sector_size: u32,
    pub format: u32,
    pub bbox_sup: [f32; 3],
    pub bbox_inf: [f32; 3],
    pub materials: Vec<RpMaterial>,
    pub root: RpSector,
    /// world sectors in stream order (RpWorldForAllWorldSectors order)
    pub sectors: Vec<RpWorldSector>,
    pub ext: Extension,
}

fn read_sector(s: &mut RwStream, h: ChunkHeader, w_native: bool, sectors: &mut Vec<RpWorldSector>) -> Result<RpSector, EngineError> {
    let mut b = s.sub(h.size as usize)?;
    match h.type_id {
        rwID_ATOMICSECT => {
            b.expect(rwID_STRUCT)?;
            let mut sec = RpWorldSector {
                mat_list_window_base: b.u32()?,
                num_triangles: b.u32()?,
                num_vertices: b.u32()?,
                bbox_inf: b.v3()?,
                bbox_sup: b.v3()?,
                ..Default::default()
            };
            let _coll = b.u32()?;
            let _unused = b.u32()?;
            if !w_native {
                // ponytail: non-native (PC) sector arrays are not present in the PS2 data
                return Err(EngineError::Format("non-native world sectors are not supported".into()));
            }
            sec.ext = Extension::read(&mut b)?;
            if let Some(m) = sec.ext.get(rwID_BINMESH) {
                sec.mesh_header = Some(RpMeshHeader::read(m, true)?);
            }
            if let (Some(mh), Some(nd)) = (&sec.mesh_header, sec.ext.get(rwID_NATIVEDATA)) {
                let strip = mh.is_strip();
                let blocks = read_native_blocks(nd, mh.meshes.len())?;
                for (m, blk) in mh.meshes.iter().zip(blocks) {
                    let mesh = NativeMesh::decode(&blk, strip)?;
                    let indices = mesh.triangle_indices(strip);
                    sec.meshes.push(SectorMesh { material: m.material, mesh, indices });
                }
            }
            sectors.push(sec);
            Ok(RpSector::Atomic(sectors.len() - 1))
        }
        rwID_PLANESECT => {
            b.expect(rwID_STRUCT)?;
            let plane_type = b.u32()?;
            let value = b.f32()?;
            let _left_is_ws = b.u32()?;
            let _right_is_ws = b.u32()?;
            let left_value = b.f32()?;
            let right_value = b.f32()?;
            let lh = b.header()?;
            let left = Box::new(read_sector(&mut b, lh, w_native, sectors)?);
            let rh = b.header()?;
            let right = Box::new(read_sector(&mut b, rh, w_native, sectors)?);
            Ok(RpSector::Plane { plane_type, value, left_value, right_value, left, right })
        }
        t => Err(EngineError::Format(format!("unexpected world chunk {t:#x}"))),
    }
}

impl RpWorld {
    /// RpWorldStreamRead (stream at the rwID_WORLD payload)
    pub fn StreamRead(s: &mut RwStream) -> Result<RpWorld, EngineError> {
        let h = s.expect(rwID_STRUCT)?;
        let mut st = s.sub(h.size as usize)?;
        let root_is_world_sector = st.u32()? != 0;
        let inv_world_origin = st.v3()?;
        let num_triangles = st.u32()?;
        let num_vertices = st.u32()?;
        let num_plane_sectors = st.u32()?;
        let num_world_sectors = st.u32()?;
        let col_sector_size = st.u32()?;
        let format = st.u32()?;
        let bbox_sup = st.v3()?;
        let bbox_inf = st.v3()?;
        let materials = ReadMaterialList(s)?;
        let mut sectors = Vec::new();
        let sh = s.header()?;
        let root = read_sector(s, sh, format & super::geometry::rpGEOMETRYNATIVE != 0, &mut sectors)?;
        let ext = Extension::read(s)?;
        Ok(RpWorld {
            root_is_world_sector,
            inv_world_origin,
            num_triangles,
            num_vertices,
            num_plane_sectors,
            num_world_sectors,
            col_sector_size,
            format,
            bbox_sup,
            bbox_inf,
            materials,
            root,
            sectors,
            ext,
        })
    }
}

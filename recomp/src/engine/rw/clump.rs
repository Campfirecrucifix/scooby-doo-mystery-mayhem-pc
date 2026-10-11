//! RpClump / RwFrame list / RpAtomic / RpHAnim / RpLight stream readers + the game's clump and
//! atomic plugins (Ap plugins). Replacement for RenderWare internals, not 1:1.
#![allow(non_snake_case)]

use super::geometry::RpGeometry;
use super::stream::*;
use crate::engine::error::EngineError;

/// RpHAnimNodeInfo {nodeID, nodeIndex, flags}
#[derive(Debug, Clone, Copy, Default)]
pub struct RpHAnimNodeInfo {
    pub id: i32,
    pub index: i32,
    /// rpHANIMPOPPARENTMATRIX 1 / rpHANIMPUSHPARENTMATRIX 2
    pub flags: u32,
}

/// rwID_HANIMPLUGIN frame data: version 0x100, node id; the root frame also carries the
/// hierarchy (numNodes > 0: flags, keyFrameSize, nodes).
#[derive(Debug, Clone, Default)]
pub struct RpHAnimFrameData {
    pub id: i32,
    pub hierarchy_flags: u32,
    pub key_frame_size: u32,
    pub nodes: Vec<RpHAnimNodeInfo>,
}

impl RpHAnimFrameData {
    pub fn read(b: &[u8]) -> Result<RpHAnimFrameData, EngineError> {
        let mut s = RwStream::new(b);
        let _version = s.u32()?;
        let id = s.i32()?;
        let n = s.u32()?;
        let mut h = RpHAnimFrameData { id, ..Default::default() };
        if n > 0 {
            h.hierarchy_flags = s.u32()?;
            h.key_frame_size = s.u32()?;
            for _ in 0..n {
                h.nodes.push(RpHAnimNodeInfo { id: s.i32()?, index: s.i32()?, flags: s.u32()? });
            }
        }
        Ok(h)
    }
}

/// RwFrame as streamed (rwFrameList entry): rotation rows right/up/at, pos, parent index.
#[derive(Debug, Clone, Default)]
pub struct RwFrame {
    pub right: [f32; 3],
    pub up: [f32; 3],
    pub at: [f32; 3],
    pub pos: [f32; 3],
    /// index of the parent frame, -1 for the root
    pub parent: i32,
    pub matrix_flags: u32,
    pub name: Option<String>,
    pub hanim: Option<RpHAnimFrameData>,
    pub ext: Extension,
}

impl RwFrame {
    /// The local transform as a column-major 4x4 (Bevy/glam `Mat4::from_cols_array` order).
    pub fn local_matrix(&self) -> [f32; 16] {
        let (r, u, a, p) = (self.right, self.up, self.at, self.pos);
        [r[0], r[1], r[2], 0.0, u[0], u[1], u[2], 0.0, a[0], a[1], a[2], 0.0, p[0], p[1], p[2], 1.0]
    }
}

/// _rwFrameListStreamRead
pub fn ReadFrameList(s: &mut RwStream) -> Result<Vec<RwFrame>, EngineError> {
    s.expect(rwID_FRAMELIST)?;
    s.expect(rwID_STRUCT)?;
    let n = s.u32()? as usize;
    let mut frames = Vec::with_capacity(n);
    for _ in 0..n {
        frames.push(RwFrame {
            right: s.v3()?,
            up: s.v3()?,
            at: s.v3()?,
            pos: s.v3()?,
            parent: s.i32()?,
            matrix_flags: s.u32()?,
            ..Default::default()
        });
    }
    for f in frames.iter_mut() {
        f.ext = Extension::read(s)?;
        if let Some(b) = f.ext.get(rwID_FRAMENAME) {
            f.name = Some(String::from_utf8_lossy(b).trim_end_matches('\0').to_string());
        }
        if let Some(b) = f.ext.get(rwID_HANIMPLUGIN) {
            f.hanim = Some(RpHAnimFrameData::read(b)?);
        }
    }
    Ok(frames)
}

/// RpAtomic as streamed + the game's atomic plugin (ApAtomicData: one i32 of flags, read by
/// CustomDataAtomicRead into plugin offset +4) and rwID_RIGHTTORENDER (pipeline id, extra data).
#[derive(Debug, Clone, Default)]
pub struct RpAtomic {
    pub frame: u32,
    pub geometry: u32,
    /// rpATOMICCOLLISIONTEST 1 / rpATOMICRENDER 4
    pub flags: u32,
    /// ApGetAtomicFlags (0x1bad008 plugin)
    pub ap_flags: Option<i32>,
    pub right_to_render: Option<(u32, u32)>,
    pub ext: Extension,
}

/// One clump collision primitive (ApCollisionPrimitives, record size 0x5c).
#[derive(Debug, Clone, Default)]
pub struct ApCollisionPrimitive {
    /// +0x0 type (0..3; 4+ makes the reader fail), +0x4, +0x8
    pub head: [i32; 3],
    /// +0xc..+0x1f
    pub reals: [f32; 5],
    /// +0x20..+0x4b
    pub attribs: [i32; 11],
    /// +0x4c.. type-specific: 0 -> 1 real, 1 -> 4 reals, 2 -> 3 reals, 3 -> 2 ints (as raw bits)
    pub extra: Vec<u32>,
}

/// ApCollisionPrimitives clump plugin data (+0 total size, +4 count, +8 bsphere*, +0xc prims,
/// +0x10 polygon data size, +0x14 polygon data).
#[derive(Debug, Clone, Default)]
pub struct ApClumpCollision {
    pub total_size: i32,
    pub bsphere: Option<[f32; 4]>,
    pub primitives: Vec<ApCollisionPrimitive>,
    /// read as i32 words (RwStreamReadInt32)
    pub polygon_data: Vec<u8>,
}

impl ApClumpCollision {
    /// CollisionPrimitivesClumpRead. Returns None where the original returns NULL.
    pub fn read(b: &[u8]) -> Option<ApClumpCollision> {
        let len = b.len() as i32;
        let mut s = RwStream::new(b);
        let total_size = s.i32().ok()?;
        let count = s.i32().ok()?;
        let mut c = ApClumpCollision { total_size, ..Default::default() };
        let mut consumed = 8;
        if count != 0 {
            c.bsphere = Some([s.f32().ok()?, s.f32().ok()?, s.f32().ok()?, s.f32().ok()?]);
            consumed = 0x18;
        }
        for _ in 0..count.max(0) {
            // quirk: stops (successfully) as soon as the consumed size reaches the chunk length
            if len <= consumed {
                break;
            }
            let mut p = ApCollisionPrimitive::default();
            for v in p.head.iter_mut() {
                *v = s.i32().ok()?;
            }
            for v in p.reals.iter_mut() {
                *v = s.f32().ok()?;
            }
            for v in p.attribs.iter_mut() {
                *v = s.i32().ok()?;
            }
            let n = match p.head[0] {
                0 => 1,
                1 => 4,
                2 => 3,
                3 => 2,
                _ => return None,
            };
            for _ in 0..n {
                p.extra.push(s.u32().ok()?);
            }
            consumed += 0x4c + 4 * n as i32;
            c.primitives.push(p);
        }
        let rest = total_size - consumed;
        if rest > 0 {
            c.polygon_data = s.Read(rest as usize).ok()?.to_vec();
        }
        Some(c)
    }
}

/// RpLight as streamed (struct {radius, r, g, b, minusCosAngle, u32 typeAndFlags}).
#[derive(Debug, Clone, Default)]
pub struct RpLight {
    pub radius: f32,
    pub color: [f32; 3],
    pub minus_cos_angle: f32,
    /// +0x2 flags (rpLIGHTLIGHTATOMICS 1, rpLIGHTLIGHTWORLD 2)
    pub flags: u8,
    /// +0x1 type: 1 directional, 2 ambient, 0x80 point, 0x81 spot, 0x82 soft spot
    pub light_type: u8,
    pub ext: Extension,
}

impl RpLight {
    /// RpLightStreamRead (stream at the rwID_LIGHT payload)
    pub fn StreamRead(s: &mut RwStream) -> Result<RpLight, EngineError> {
        s.expect(rwID_STRUCT)?;
        let radius = s.f32()?;
        let color = s.v3()?;
        let minus_cos_angle = s.f32()?;
        let tf = s.u32()?;
        let ext = Extension::read(s)?;
        Ok(RpLight { radius, color, minus_cos_angle, flags: tf as u8, light_type: (tf >> 16) as u8, ext })
    }
}

#[derive(Debug, Clone, Default)]
pub struct RpClump {
    pub frames: Vec<RwFrame>,
    pub geometries: Vec<RpGeometry>,
    pub atomics: Vec<RpAtomic>,
    /// (frame index, light)
    pub lights: Vec<(i32, RpLight)>,
    pub collision: Option<ApClumpCollision>,
    pub ext: Extension,
}

impl RpClump {
    /// RpClumpStreamRead (stream at the rwID_CLUMP payload)
    pub fn StreamRead(s: &mut RwStream) -> Result<RpClump, EngineError> {
        let h = s.expect(rwID_STRUCT)?;
        let mut st = s.sub(h.size as usize)?;
        let num_atomics = st.u32()?;
        let num_lights = if st.remaining() >= 4 { st.u32()? } else { 0 };
        let _num_cameras = if st.remaining() >= 4 { st.u32()? } else { 0 };
        let mut c = RpClump { frames: ReadFrameList(s)?, ..Default::default() };
        let gh = s.expect(rwID_GEOMETRYLIST)?;
        let _ = gh;
        s.expect(rwID_STRUCT)?;
        let ng = s.u32()?;
        for _ in 0..ng {
            let h = s.expect(rwID_GEOMETRY)?;
            let mut g = s.sub(h.size as usize)?;
            c.geometries.push(RpGeometry::StreamRead(&mut g)?);
        }
        for _ in 0..num_atomics {
            let h = s.expect(rwID_ATOMIC)?;
            let mut a = s.sub(h.size as usize)?;
            a.expect(rwID_STRUCT)?;
            let mut at = RpAtomic { frame: a.u32()?, geometry: a.u32()?, flags: a.u32()?, ..Default::default() };
            let _unused = a.u32()?;
            at.ext = Extension::read(&mut a)?;
            at.ap_flags = at.ext.get(apID_ATOMIC_DATA).and_then(|b| b.get(..4)).map(|b| i32::from_le_bytes(b.try_into().unwrap()));
            at.right_to_render = at.ext.get(rwID_RIGHTTORENDER).filter(|b| b.len() >= 8).map(|b| {
                (u32::from_le_bytes(b[..4].try_into().unwrap()), u32::from_le_bytes(b[4..8].try_into().unwrap()))
            });
            c.atomics.push(at);
        }
        for _ in 0..num_lights {
            s.expect(rwID_STRUCT)?;
            let fi = s.i32()?;
            let h = s.expect(rwID_LIGHT)?;
            let mut l = s.sub(h.size as usize)?;
            c.lights.push((fi, RpLight::StreamRead(&mut l)?));
        }
        c.ext = Extension::read(s)?;
        if let Some(b) = c.ext.get(apID_CLUMP_COLLISION_PRIMITIVES) {
            c.collision = ApClumpCollision::read(b);
        }
        Ok(c)
    }

    /// Frame world transforms (column-major 4x4), composing parents (parents precede children
    /// in RW frame lists).
    pub fn frame_world_matrices(&self) -> Vec<[f32; 16]> {
        let mut out: Vec<[f32; 16]> = Vec::with_capacity(self.frames.len());
        for f in &self.frames {
            let l = f.local_matrix();
            let w = if f.parent >= 0 && (f.parent as usize) < out.len() { mat_mul(&out[f.parent as usize], &l) } else { l };
            out.push(w);
        }
        out
    }

    /// The RpHAnim hierarchy (from the first frame that carries nodes).
    pub fn hanim_hierarchy(&self) -> Option<(usize, &RpHAnimFrameData)> {
        self.frames.iter().enumerate().find_map(|(i, f)| f.hanim.as_ref().filter(|h| !h.nodes.is_empty()).map(|h| (i, h)))
    }
}

/// column-major 4x4 multiply a*b
pub fn mat_mul(a: &[f32; 16], b: &[f32; 16]) -> [f32; 16] {
    let mut r = [0f32; 16];
    for c in 0..4 {
        for row in 0..4 {
            r[c * 4 + row] = (0..4).map(|k| a[k * 4 + row] * b[c * 4 + k]).sum();
        }
    }
    r
}

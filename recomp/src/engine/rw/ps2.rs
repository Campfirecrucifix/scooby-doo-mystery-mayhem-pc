//! PS2 native instance data (rwID_NATIVEDATA, platform 4) decoder. Replacement for the RW PS2
//! pipelines (PS2All / RpPDS + VU1 microcode), not 1:1: instead of kicking the DMA chain to VIF1,
//! the chain is walked on the CPU and the VIF UNPACKs are decoded into vertex arrays.
//!
//! Layout (_rpWorldSectorNativeRead / _rpGeometryNativeRead): struct chunk {u32 platform = 4;
//! per mesh: u32 dataSize (bytes), u32 noPointers (data 128-byte aligned when != 0),
//! dataSize bytes of DMA chain}. The chain uses DMA tags cnt (1: qwc inline qwords follow),
//! ref (3: tag.addr = qword offset of the data inside the mesh block; the RW reader fixes it to
//! an absolute pointer) and ends on refe/end/ret (0/6/7). The tag's upper 64 bits are VIF codes.
//! Vertex batches: STCYCL cl=4 wl=1, then one UNPACK per attribute with addr = attribute slot
//! (|0x8000 TOPS-relative): 0 position (V3-32), 1 uv (V2-32), 2 prelit RGBA (V4-8 unsigned),
//! 3 normal (V3-8 signed), then ITOP/MSCAL. For tri-strip meshes consecutive batches repeat the
//! last two vertices of the previous batch (verified on every EP1L01 world sector:
//! sum(batch verts) - 2*(batches-1) == bin-mesh index count).
#![allow(non_snake_case)]

use crate::engine::error::EngineError;

fn fmt_err(m: &str) -> EngineError {
    EngineError::Format(format!("PS2 native data: {m}"))
}

/// One decoded VIF UNPACK.
#[derive(Debug, Clone)]
pub struct Unpack {
    /// VIF cmd (0x60 | vn<<2 | vl)
    pub cmd: u8,
    pub num: u32,
    /// imm: addr (bits 0-9), usn (bit 14), flg (bit 15)
    pub imm: u16,
    pub data: Vec<u8>,
}

impl Unpack {
    /// element i as up to 4 f32 components (integers are converted, not normalised)
    pub fn element(&self, i: usize) -> [f32; 4] {
        let vn = ((self.cmd >> 2) & 3) as usize + 1;
        let vl = self.cmd & 3;
        let usn = self.imm & 0x4000 != 0;
        let mut out = [0f32; 4];
        for c in 0..vn {
            let k = i * vn + c;
            out[c] = match vl {
                0 => f32::from_le_bytes(self.data[4 * k..4 * k + 4].try_into().unwrap()),
                1 => {
                    let v = u16::from_le_bytes([self.data[2 * k], self.data[2 * k + 1]]);
                    if usn { v as f32 } else { v as i16 as f32 }
                }
                2 => {
                    let v = self.data[k];
                    if usn { v as f32 } else { v as i8 as f32 }
                }
                _ => 0.0,
            };
        }
        out
    }
    pub fn slot(&self) -> u16 {
        self.imm & 0x3ff
    }
}

/// Parses VIF codes in `b`; UNPACK payloads are taken inline, except for the last code of a ref
/// tag whose payload is `ref_data`.
fn parse_vif(b: &[u8], mut ref_data: Option<&[u8]>, out: &mut Vec<Unpack>, batches: &mut usize) -> Result<(), EngineError> {
    let mut o = 0;
    while o + 4 <= b.len() {
        let c = u32::from_le_bytes(b[o..o + 4].try_into().unwrap());
        o += 4;
        let cmd = ((c >> 24) & 0x7f) as u8;
        let num = (c >> 16) & 0xff;
        let imm = (c & 0xffff) as u16;
        if cmd & 0x60 == 0x60 {
            let n = if num == 0 { 256 } else { num };
            let vn = ((cmd >> 2) & 3) as u32 + 1;
            let vl = (cmd & 3) as u32;
            let size = (((32 >> vl) * vn * n + 31) / 32 * 4) as usize;
            let data = if let Some(r) = ref_data.take() {
                r.get(..size).ok_or_else(|| fmt_err("ref unpack overruns"))?.to_vec()
            } else {
                let d = b.get(o..o + size).ok_or_else(|| fmt_err("unpack overruns"))?.to_vec();
                o += size;
                d
            };
            out.push(Unpack { cmd, num: n, imm, data });
            continue;
        }
        match cmd {
            0x14 | 0x15 | 0x17 => *batches += 1, // MSCAL / MSCALF / MSCNT
            0x20 => o += 4,                      // STMASK
            0x30 | 0x31 => o += 16,              // STROW / STCOL
            0x4a => o += 8 * if num == 0 { 256 } else { num as usize }, // MPG
            0x50 | 0x51 => o += 16 * if imm == 0 { 65536 } else { imm as usize }, // DIRECT/DIRECTHL
            _ => {}
        }
    }
    Ok(())
}

/// Walks one mesh's DMA chain and returns its UNPACKs in order.
pub fn walk_dma(data: &[u8]) -> Result<Vec<Unpack>, EngineError> {
    let mut ops = Vec::new();
    let mut o = 0usize;
    let mut batches = 0;
    loop {
        let q = data.get(o..o + 16).ok_or_else(|| fmt_err("chain runs off the block"))?;
        let tag = u32::from_le_bytes(q[0..4].try_into().unwrap());
        let addr = u32::from_le_bytes(q[4..8].try_into().unwrap()) as usize;
        let qwc = (tag & 0xffff) as usize;
        match (tag >> 28) & 7 {
            1 | 0 | 6 | 7 => {
                // cnt (and the terminating refe/ret/end, whose inline payload the RW chains
                // carry as with cnt)
                let end = (o + 16 + qwc * 16).min(data.len());
                parse_vif(&data[o + 8..end], None, &mut ops, &mut batches)?;
                o += 16 + qwc * 16;
                if (tag >> 28) & 7 != 1 {
                    break;
                }
            }
            3 => {
                let r = data.get(addr * 16..addr * 16 + qwc * 16).ok_or_else(|| fmt_err("bad ref"))?;
                parse_vif(&q[8..16], Some(r), &mut ops, &mut batches)?;
                o += 16;
            }
            t => return Err(fmt_err(&format!("unsupported DMA tag id {t}"))),
        }
    }
    Ok(ops)
}

/// Decoded vertices of one native mesh (already de-overlapped: one entry per bin-mesh index).
#[derive(Debug, Clone, Default)]
pub struct NativeMesh {
    pub positions: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    /// prelit RGBA as stored (u8 per channel)
    pub colors: Vec<[u8; 4]>,
    pub normals: Vec<[f32; 3]>,
    pub batches: u32,
}

impl NativeMesh {
    /// Decode a mesh's DMA block. `strip`: batches overlap by two vertices.
    pub fn decode(data: &[u8], strip: bool) -> Result<NativeMesh, EngineError> {
        let ops = walk_dma(data)?;
        let mut m = NativeMesh::default();
        // batch index of the current unpack: a slot-0 (position) unpack starts a new batch
        let mut batch = 0u32;
        for u in &ops {
            if u.slot() == 0 {
                batch += 1;
            }
            let skip = if strip && batch > 1 { 2 } else { 0 };
            let n = u.num as usize;
            match u.slot() {
                0 => {
                    for i in skip..n {
                        let e = u.element(i);
                        m.positions.push([e[0], e[1], e[2]]);
                    }
                    m.batches += 1;
                }
                1 => {
                    for i in skip..n {
                        let e = u.element(i);
                        m.uvs.push([e[0], e[1]]);
                    }
                }
                2 => {
                    for i in skip..n {
                        let e = u.element(i);
                        m.colors.push([e[0] as u8, e[1] as u8, e[2] as u8, e[3] as u8]);
                    }
                }
                3 => {
                    for i in skip..n {
                        let e = u.element(i);
                        m.normals.push([e[0] / 127.0, e[1] / 127.0, e[2] / 127.0]);
                    }
                }
                _ => {}
            }
        }
        Ok(m)
    }

    /// Triangle list over the decoded vertices (strip -> list; degenerate = repeated position).
    pub fn triangle_indices(&self, strip: bool) -> Vec<u32> {
        let n = self.positions.len() as u32;
        if !strip {
            return (0..n).collect();
        }
        let mut out = Vec::new();
        for i in 2..n {
            let (a, b, c) = (i - 2, i - 1, i);
            let p = |k: u32| self.positions[k as usize];
            if p(a) == p(b) || p(b) == p(c) || p(a) == p(c) {
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
}

/// The rwID_NATIVEDATA payload: platform + per-mesh blocks.
pub fn read_native_blocks(b: &[u8], num_meshes: usize) -> Result<Vec<Vec<u8>>, EngineError> {
    let mut s = super::stream::RwStream::new(b);
    s.expect(super::stream::rwID_STRUCT)?;
    let platform = s.u32()?;
    if platform != 4 {
        return Err(fmt_err(&format!("platform {platform} is not PS2")));
    }
    let mut out = Vec::with_capacity(num_meshes);
    for _ in 0..num_meshes {
        let size = s.u32()? as usize;
        let _no_pointers = s.u32()?;
        out.push(s.Read(size)?.to_vec());
    }
    Ok(out)
}

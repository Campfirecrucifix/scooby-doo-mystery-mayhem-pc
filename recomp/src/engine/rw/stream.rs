//! RwStream replacement (not 1:1): a little-endian cursor over an in-memory RenderWare binary
//! stream. Keeps what the EF* code observes: chunk headers {type, size, libraryID},
//! RwStreamFindChunk (skips non-matching chunks), RwStreamRead / ReadInt32 / ReadReal.
#![allow(non_snake_case)]

use crate::engine::error::EngineError;

/// RwChunkHeaderInfo as read from the stream: u32 type, u32 size (payload bytes), u32 libraryID.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChunkHeader {
    pub type_id: u32,
    pub size: u32,
    pub lib_id: u32,
}

impl ChunkHeader {
    /// RW 3.x libraryID unpacking: 0x1003ffff -> version 0x34003 (3.4.0.3), build 0xffff.
    pub fn version(&self) -> u32 {
        if self.lib_id & 0xffff_0000 != 0 {
            (((self.lib_id >> 14) & 0x3ff00) + 0x30000) | ((self.lib_id >> 16) & 0x3f)
        } else {
            self.lib_id << 8
        }
    }
}

// RenderWare chunk ids used by the game data.
pub const rwID_STRUCT: u32 = 0x01;
pub const rwID_STRING: u32 = 0x02;
pub const rwID_EXTENSION: u32 = 0x03;
pub const rwID_TEXTURE: u32 = 0x06;
pub const rwID_MATERIAL: u32 = 0x07;
pub const rwID_MATLIST: u32 = 0x08;
pub const rwID_ATOMICSECT: u32 = 0x09;
pub const rwID_PLANESECT: u32 = 0x0a;
pub const rwID_WORLD: u32 = 0x0b;
pub const rwID_FRAMELIST: u32 = 0x0e;
pub const rwID_GEOMETRY: u32 = 0x0f;
pub const rwID_CLUMP: u32 = 0x10;
pub const rwID_LIGHT: u32 = 0x12;
pub const rwID_ATOMIC: u32 = 0x14;
pub const rwID_IMAGE: u32 = 0x18;
pub const rwID_GEOMETRYLIST: u32 = 0x1a;
pub const rwID_RIGHTTORENDER: u32 = 0x1f;
pub const rwID_SKYMIPMAP: u32 = 0x110;
pub const rwID_SKINPLUGIN: u32 = 0x116;
pub const rwID_HANIMPLUGIN: u32 = 0x11e;
pub const rwID_MATERIALEFFECTS: u32 = 0x120;
pub const rwID_BINMESH: u32 = 0x50e;
pub const rwID_NATIVEDATA: u32 = 0x510;
pub const rwID_FRAMENAME: u32 = 0x253f2fe;
/// Game plugins (Free Functions/Ap plugins - *.cpp)
pub const apID_CLUMP_COLLISION_PRIMITIVES: u32 = 0x1bad003;
pub const apID_MORPH_ANIMATION: u32 = 0x1bad006;
pub const apID_ATOMIC_DATA: u32 = 0x1bad008;

/// In-memory RwStream (rwSTREAMMEMORY equivalent).
#[derive(Clone)]
pub struct RwStream<'a> {
    pub data: &'a [u8],
    pub pos: usize,
}

fn short(what: &str, pos: usize) -> EngineError {
    EngineError::Format(format!("RwStream: short read ({what}) at {pos:#x}"))
}

impl<'a> RwStream<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        RwStream { data, pos: 0 }
    }
    pub fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }
    /// RwStreamRead
    pub fn Read(&mut self, n: usize) -> Result<&'a [u8], EngineError> {
        if self.remaining() < n {
            return Err(short("bytes", self.pos));
        }
        let s = &self.data[self.pos..self.pos + n];
        self.pos += n;
        Ok(s)
    }
    pub fn Skip(&mut self, n: usize) -> Result<(), EngineError> {
        self.Read(n).map(|_| ())
    }
    pub fn u8(&mut self) -> Result<u8, EngineError> {
        Ok(self.Read(1)?[0])
    }
    pub fn u16(&mut self) -> Result<u16, EngineError> {
        Ok(u16::from_le_bytes(self.Read(2)?.try_into().unwrap()))
    }
    /// RwStreamReadInt32 (one value; the PS2 is little-endian so RwMemNative32 is a no-op)
    pub fn u32(&mut self) -> Result<u32, EngineError> {
        Ok(u32::from_le_bytes(self.Read(4)?.try_into().unwrap()))
    }
    pub fn i32(&mut self) -> Result<i32, EngineError> {
        Ok(self.u32()? as i32)
    }
    /// RwStreamReadReal (one value)
    pub fn f32(&mut self) -> Result<f32, EngineError> {
        Ok(f32::from_bits(self.u32()?))
    }
    pub fn v3(&mut self) -> Result<[f32; 3], EngineError> {
        Ok([self.f32()?, self.f32()?, self.f32()?])
    }
    pub fn header(&mut self) -> Result<ChunkHeader, EngineError> {
        Ok(ChunkHeader { type_id: self.u32()?, size: self.u32()?, lib_id: self.u32()? })
    }
    pub fn peek_header(&self) -> Option<ChunkHeader> {
        self.clone().header().ok()
    }
    /// RwStreamFindChunk: reads chunk headers, skipping the payload of non-matching chunks,
    /// until a chunk of `type_id` is found (stream left at its payload). None at end of stream.
    pub fn FindChunk(&mut self, type_id: u32) -> Option<ChunkHeader> {
        loop {
            let h = self.header().ok()?;
            if h.type_id == type_id {
                return Some(h);
            }
            self.Skip(h.size as usize).ok()?;
        }
    }
    /// FindChunk that fails with a format error.
    pub fn expect(&mut self, type_id: u32) -> Result<ChunkHeader, EngineError> {
        let at = self.pos;
        self.FindChunk(type_id).ok_or_else(|| EngineError::Format(format!("RW chunk {type_id:#x} not found at {at:#x}")))
    }
    /// Sub-stream over the next `n` bytes (advances self).
    pub fn sub(&mut self, n: usize) -> Result<RwStream<'a>, EngineError> {
        Ok(RwStream::new(self.Read(n)?))
    }
    /// Reads a rwID_STRING chunk (NUL-terminated, padded) and returns the string.
    pub fn string_chunk(&mut self) -> Result<String, EngineError> {
        let h = self.expect(rwID_STRING)?;
        let b = self.Read(h.size as usize)?;
        Ok(cstr(b))
    }
}

/// Bytes up to the first NUL as a String (lossy).
pub fn cstr(b: &[u8]) -> String {
    let e = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    String::from_utf8_lossy(&b[..e]).into_owned()
}

/// A plugin extension chunk: (plugin id, raw payload). Plugins the port parses are decoded by
/// their owners; unknown ones are kept raw (RW skips them the same way).
#[derive(Debug, Clone, Default)]
pub struct Extension {
    pub plugins: Vec<(u32, Vec<u8>)>,
}

impl Extension {
    /// _rwPluginRegistryReadDataChunks: rwID_EXTENSION header then plugin chunks.
    pub fn read(s: &mut RwStream) -> Result<Extension, EngineError> {
        let h = s.expect(rwID_EXTENSION)?;
        let mut sub = s.sub(h.size as usize)?;
        let mut e = Extension::default();
        while sub.remaining() >= 12 {
            let p = sub.header()?;
            e.plugins.push((p.type_id, sub.Read(p.size as usize)?.to_vec()));
        }
        Ok(e)
    }
    pub fn get(&self, id: u32) -> Option<&[u8]> {
        self.plugins.iter().find(|p| p.0 == id).map(|p| p.1.as_slice())
    }
}

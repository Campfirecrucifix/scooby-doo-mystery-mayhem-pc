//! RwImage / RwRaster / RwTexture / RwTexDictionary replacement (not 1:1).
//!
//! The game's TEXDIC_n.TXD side files are NOT RenderWare texture dictionaries (rwID_TEXDICTIONARY):
//! they are a plain sequence of `rwID_IMAGE` chunks (RwImageStreamRead: struct {width, height,
//! depth, stride}, height*stride pixel bytes, then (1<<depth) RGBA palette entries when depth<=8),
//! each followed by `i32 nameLen, char name[nameLen]` (EFTextureDict::LoadTextureDictionary).
//! The PS2 build converts them at load time with RwRasterSetFromImage; the port keeps the
//! observable result: a named texture with RGBA8 texels in the current dictionary.
#![allow(non_snake_case)]

use super::stream::*;
use crate::engine::error::EngineError;

/// RwImage (+0x4 width, +0x8 height, +0xc depth, +0x10 stride, +0x14 pixels, +0x18 palette)
#[derive(Debug, Clone, Default)]
pub struct RwImage {
    pub width: u32,
    pub height: u32,
    pub depth: u32,
    pub stride: u32,
    pub pixels: Vec<u8>,
    /// (1<<depth) RGBA entries when depth <= 8
    pub palette: Vec<[u8; 4]>,
}

impl RwImage {
    /// RwImageStreamRead (stream positioned at the rwID_IMAGE payload).
    pub fn StreamRead(s: &mut RwStream) -> Result<RwImage, EngineError> {
        s.expect(rwID_STRUCT)?;
        let (width, height, depth, stride) = (s.u32()?, s.u32()?, s.u32()?, s.u32()?);
        let pixels = s.Read((height * stride) as usize)?.to_vec();
        let mut palette = Vec::new();
        if depth <= 8 {
            let b = s.Read(4usize << depth)?;
            palette = b.chunks_exact(4).map(|c| [c[0], c[1], c[2], c[3]]).collect();
        }
        Ok(RwImage { width, height, depth, stride, pixels, palette })
    }

    /// Texel (x, y) as RGBA8. Depth 4/8 index the palette, 32 is RGBA, 16 is 1555 (A1B5G5R5 as
    /// the PS2 GS PSMCT16 stores it).
    pub fn texel(&self, x: u32, y: u32) -> [u8; 4] {
        let row = &self.pixels[(y * self.stride) as usize..];
        match self.depth {
            4 => {
                // replacement: RwImage 4-bit rows are one index per byte when stride >= width,
                // else two per byte, low nibble first. (No 4-bit images in the game data.)
                let i = if self.stride >= self.width {
                    row[x as usize] & 0xf
                } else {
                    let b = row[(x / 2) as usize];
                    if x & 1 == 0 { b & 0xf } else { b >> 4 }
                };
                self.palette[i as usize]
            }
            8 => self.palette[row[x as usize] as usize],
            16 => {
                let v = u16::from_le_bytes([row[2 * x as usize], row[2 * x as usize + 1]]);
                let c = |s: u32| (((v >> s) & 0x1f) as u8) << 3 | (((v >> s) & 0x1f) as u8) >> 2;
                [c(0), c(5), c(10), if v & 0x8000 != 0 { 0xff } else { 0 }]
            }
            24 => {
                let p = &row[3 * x as usize..];
                [p[0], p[1], p[2], 0xff]
            }
            _ => {
                let p = &row[4 * x as usize..];
                [p[0], p[1], p[2], p[3]]
            }
        }
    }

    pub fn to_rgba8(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity((self.width * self.height * 4) as usize);
        for y in 0..self.height {
            for x in 0..self.width {
                out.extend_from_slice(&self.texel(x, y));
            }
        }
        out
    }

    /// RwImageCreateResample replacement: nearest-neighbour to (w, h), result is a 32-bit image.
    pub fn CreateResample(&self, w: u32, h: u32) -> RwImage {
        let mut pixels = Vec::with_capacity((w * h * 4) as usize);
        for y in 0..h {
            for x in 0..w {
                pixels.extend_from_slice(&self.texel(x * self.width / w, y * self.height / h));
            }
        }
        RwImage { width: w, height: h, depth: 32, stride: w * 4, pixels, palette: Vec::new() }
    }
}

/// RwTexture (+0x0 raster, +0x10 name[32], +0x30 mask[32], +0x50 filterAddressing) with its
/// raster's texels decoded to RGBA8 (the replacement for RwRaster).
#[derive(Debug, Clone, Default)]
pub struct RwTexture {
    pub name: String,
    pub mask: String,
    pub width: u32,
    pub height: u32,
    /// RwRaster format flags passed to RwRasterCreate (0x504 for "fog"/"fog2", else the
    /// FindRasterFormat result: here 0 = default, see LoadTextureDictionary).
    pub raster_format: u32,
    /// +0x50 filterAddressing: low byte = RwTextureFilterMode (2 = rwFILTERLINEAR), then
    /// addressing U (bits 8-11) / V (bits 12-15).
    pub filter_addressing: u32,
    /// width*height RGBA8, row-major, top row first.
    pub rgba: Vec<u8>,
}

/// RwTexDictionary replacement. Textures are added at the list head (RwTexDictionaryAddTexture),
/// so a lookup finds the most recently added texture of a name.
#[derive(Debug, Clone, Default)]
pub struct RwTexDictionary {
    pub textures: Vec<RwTexture>,
}

/// rwstrcmp-ish compare of RwTexDictionaryFindNamedTexture: ASCII case-insensitive.
pub fn rw_name_eq(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
}

impl RwTexDictionary {
    pub fn AddTexture(&mut self, t: RwTexture) {
        self.textures.push(t);
    }
    /// RwTexDictionaryFindNamedTexture: index into `textures`.
    pub fn FindNamedTexture(&self, name: &str) -> Option<usize> {
        self.textures.iter().rposition(|t| rw_name_eq(&t.name, name))
    }
    pub fn Get(&self, name: &str) -> Option<&RwTexture> {
        self.FindNamedTexture(name).map(|i| &self.textures[i])
    }
}

/// Builds the RwTexture LoadTextureDictionary/LoadTexture would create from an image + name
/// (shared body of both functions).
pub fn texture_from_image(img: RwImage, name: &str, fog_special: bool) -> RwTexture {
    let (w, h) = (img.width, img.height);
    // RwImageFindRasterFormat (sky driver): keeps the image size for texture rasters (replacement:
    // assumed identity); non power-of-two sizes get a 256x256 raster (original code path).
    let pow2 = w & w.wrapping_sub(1) == 0 && h & h.wrapping_sub(1) == 0;
    let (rw, rh) = if pow2 { (w, h) } else { (0x100, 0x100) };
    let raster_format = if pow2 && fog_special && (name == "fog" || name == "fog2") { 0x504 } else { 0 };
    let mut img = if (w, h) != (rw, rh) { img.CreateResample(rw, rh) } else { img };
    // quirk: palettised images (depth < 9): every palette entry whose RGB is pure black is made
    // fully transparent before the raster is built.
    if img.depth < 9 {
        for p in img.palette.iter_mut() {
            if p[0] == 0 && p[1] == 0 && p[2] == 0 {
                p[3] = 0;
            }
        }
    }
    RwTexture {
        name: name.to_string(),
        mask: String::new(),
        width: rw,
        height: rh,
        raster_format,
        // `*(tex+0x50) = *(tex+0x50) & 0xffffff00 | 2` on RwTextureCreate's default (0x1102)
        filter_addressing: 0x1100 | 2,
        rgba: img.to_rgba8(),
    }
}

/// Reads one {rwID_IMAGE, i32 nameLen, name} record. Ok(None) at end of stream.
fn read_named_image(s: &mut RwStream) -> Result<Option<(RwImage, String)>, EngineError> {
    if s.FindChunk(rwID_IMAGE).is_none() {
        return Ok(None);
    }
    let img = RwImage::StreamRead(s)?;
    let n = s.i32()?;
    let name = cstr(s.Read(n.max(0) as usize)?);
    Ok(Some((img, name)))
}

/// EFTextureDict::LoadTextureDictionary body: every image of the stream into `dict`.
/// Returns 0, or 0xfffffffe on a malformed record (like the original).
pub fn LoadTextureDictionary(s: &mut RwStream, dict: &mut RwTexDictionary) -> u32 {
    loop {
        match read_named_image(s) {
            Ok(None) => return 0,
            Ok(Some((img, name))) => dict.AddTexture(texture_from_image(img, &name, true)),
            Err(_) => return 0xfffffffe,
        }
    }
}

/// EFTextureDict::LoadTexture body: one image. quirk: no "fog" raster-format special case here.
pub fn LoadTexture(s: &mut RwStream, dict: &mut RwTexDictionary) -> u32 {
    match read_named_image(s) {
        Ok(Some((img, name))) => {
            dict.AddTexture(texture_from_image(img, &name, false));
            0
        }
        _ => 0xfffffffe,
    }
}

//! WP-F: HUD resource files (FONT*.FT / *.PAL / TEXT.TX / LOAD.LS side files of a level).
//!
//! The C++ loads these through `DT*Descriptor::LoadFromStream` (DTProperty binary streams). The
//! binary layout below was derived from the EP1L01 EN files and the C++ readers; field offsets of
//! the C++ objects are kept in doc comments.
//! Common header of every file (DTStreamable): `u32 id (this+8), u32 0, u32 0xffffffff`.
#![allow(non_snake_case)]

use crate::engine::error::EngineError;

/// Little-endian cursor over a side file.
pub(crate) struct Rd<'a> {
    pub b: &'a [u8],
    pub p: usize,
}

impl<'a> Rd<'a> {
    pub fn new(b: &'a [u8]) -> Self {
        Rd { b, p: 0 }
    }
    pub fn bytes(&mut self, n: usize) -> Result<&'a [u8], EngineError> {
        let e = self.p.checked_add(n).filter(|&e| e <= self.b.len());
        let e = e.ok_or_else(|| EngineError::Format(format!("short HUD file (need {n} at {:#x})", self.p)))?;
        let s = &self.b[self.p..e];
        self.p = e;
        Ok(s)
    }
    pub fn u8(&mut self) -> Result<u8, EngineError> {
        Ok(self.bytes(1)?[0])
    }
    pub fn u32(&mut self) -> Result<u32, EngineError> {
        Ok(u32::from_le_bytes(self.bytes(4)?.try_into().unwrap()))
    }
    pub fn f32(&mut self) -> Result<f32, EngineError> {
        Ok(f32::from_bits(self.u32()?))
    }
    /// Length-prefixed blob (DTProperty binary payload).
    pub fn blob(&mut self) -> Result<&'a [u8], EngineError> {
        let n = self.u32()? as usize;
        self.bytes(n)
    }
    /// Length-prefixed zero-terminated string.
    pub fn string(&mut self) -> Result<String, EngineError> {
        let s = self.blob()?;
        let s = s.split(|&c| c == 0).next().unwrap_or(&[]);
        Ok(String::from_utf8_lossy(s).into_owned())
    }
    /// DTStreamable header -> id (this+8).
    pub fn header(&mut self) -> Result<u32, EngineError> {
        let id = self.u32()?;
        self.u32()?;
        self.u32()?;
        Ok(id)
    }
}

fn le32(b: &[u8], i: usize) -> u32 {
    u32::from_le_bytes(b[i * 4..i * 4 + 4].try_into().unwrap())
}

/// EFFontChar (0x2c bytes, 11 floats).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct EFFontChar {
    /// +0x00 texture u
    pub u: f32,
    /// +0x04 texture v
    pub v: f32,
    /// +0x08 u size
    pub du: f32,
    /// +0x0c v size
    pub dv: f32,
    /// +0x10 width (screen fraction)
    pub w: f32,
    /// +0x14 height (screen fraction)
    pub h: f32,
    /// +0x18 vertical offset (screen fraction)
    pub yoff: f32,
    /// +0x1c width in pixels (ComputeScreenDims)
    pub sw: f32,
    /// +0x20 height in pixels
    pub sh: f32,
    /// +0x24 vertical offset in pixels
    pub syoff: f32,
    /// +0x28 texture index (stored as a float in the file)
    pub tex: f32,
}

/// EFFont (class 0x3a, "EF_FONTS"; DTFontsDescriptor + texture pointers).
#[derive(Debug, Clone, Default)]
pub struct EFFont {
    /// this+0x08: font slot in EFHudMgr (0xffffffff = not registered).
    pub id: u32,
    /// this+0x14 number of EFFontChar
    pub nb_chars: u32,
    /// this+0x18 (unknown; 0xd/0xf in the EN fonts)
    pub unk18: u32,
    /// this+0x20 space width (screen fraction)
    pub space: f32,
    /// this+0x24 line height (screen fraction)
    pub line_height: f32,
    /// this+0x28 extra char spacing (screen fraction)
    pub spacing: f32,
    /// this+0x34 count + texture names (EFTextureDict::GetTexture -> this+0x148).
    pub textures: Vec<String>,
    /// this+0x144 "TEXT_INDEX_TABLE": text code -> char index (this+0x1c = count).
    pub text_index: Vec<u32>,
    /// this+0x40 "CHAR_TABLE"
    pub chars: Vec<EFFontChar>,
    /// this+0x44 "ASCII_TABLE": ascii (0..0x80) -> text code.
    pub ascii: Vec<u16>,
    /// this+0x150 line height in pixels
    pub screen_line_height: f32,
    /// this+0x154 space width in pixels
    pub screen_space: f32,
    /// this+0x158 char spacing in pixels
    pub screen_spacing: f32,
}

impl EFFont {
    pub fn GetClassID() -> u32 {
        0x3a
    }

    /// DTFontsDescriptor::LoadFromStream (binary). Registering in the HUD mgr (EFFont::LoadFromStream
    /// -> EFHudMgr_BC::AddFont) is done by [`super::EFHudMgr::LoadSideFiles`].
    pub fn load(b: &[u8]) -> Result<EFFont, EngineError> {
        let mut r = Rd::new(b);
        let mut f = EFFont { id: r.header()?, ..Default::default() };
        f.nb_chars = r.u32()?;
        f.unk18 = r.u32()?;
        f.space = r.f32()?;
        f.line_height = r.f32()?;
        f.spacing = r.f32()?;
        let ntex = r.u32()?;
        for _ in 0..ntex {
            f.textures.push(r.string()?);
        }
        let ntxt = r.u32()? as usize;
        let t = r.blob()?;
        if t.len() < ntxt * 4 {
            return Err(EngineError::Format("FT text index table".into()));
        }
        f.text_index = (0..ntxt).map(|i| le32(t, i)).collect();
        let c = r.blob()?;
        if c.len() < f.nb_chars as usize * 0x2c {
            return Err(EngineError::Format("FT char table".into()));
        }
        f.chars = c.chunks_exact(0x2c).take(f.nb_chars as usize).map(|e| {
            let g = |i: usize| f32::from_bits(le32(e, i));
            EFFontChar { u: g(0), v: g(1), du: g(2), dv: g(3), w: g(4), h: g(5), yoff: g(6), sw: g(7), sh: g(8), syoff: g(9), tex: g(10) }
        }).collect();
        let a = r.blob()?;
        f.ascii = vec![0; 128];
        for i in 0..128.min(a.len() / 2) {
            f.ascii[i] = u16::from_le_bytes([a[i * 2], a[i * 2 + 1]]);
        }
        Ok(f)
    }

    /// EFFont::ComputeScreenDims(sx, sy)
    pub fn ComputeScreenDims(&mut self, sx: f32, sy: f32) {
        for c in &mut self.chars {
            c.sw = c.w * sx;
            c.sh = c.h * sy;
            c.syoff = c.yoff * sy;
        }
        self.screen_line_height = self.line_height * sy;
        self.screen_space = self.space * sx;
        self.screen_spacing = self.spacing * sx;
    }

    /// chars[text_index[code]] (the C++ does no bounds check; out of range -> None here).
    pub fn Char(&self, code: u32) -> Option<&EFFontChar> {
        self.chars.get(*self.text_index.get(code as usize)? as usize)
    }
}

/// RGBA as stored in the palette (r,g,b,a bytes).
pub type RGBA = [u8; 4];

/// One EFHudPalette entry (0x2c bytes): 4 corner colours + fade state.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct EFPaletteEntry {
    /// +0x00 four corner colours (EFFontColorGroup)
    pub colors: [RGBA; 4],
    /// +0x10 fade flags: 0x80000000 fading, 0x10000 ping-pong, low bits = ColorBlend mask
    pub flags: u32,
    /// +0x14 fade time
    pub time: f32,
    /// +0x18 fade duration
    pub duration: f32,
    /// +0x1c palette index
    pub pal: u32,
    /// +0x20 dest colour index
    pub dst: u32,
    /// +0x24 colour A
    pub a: u32,
    /// +0x28 colour B
    pub b: u32,
}

/// EFHudPalette (class 0x3e, "EF_FONTS_PAL").
#[derive(Debug, Clone, Default)]
pub struct EFHudPalette {
    /// this+0x08: -1 = global palette (mgr+0xc), else mgr+0x38+id*4.
    pub id: u32,
    /// this+0x1c count, this+0x20 entries
    pub entries: Vec<EFPaletteEntry>,
}

impl EFHudPalette {
    pub fn GetClassID() -> u32 {
        0x3e
    }
    /// DTFontsPalDescriptor::LoadFromStream + EFHudPalette::Init(count, groups).
    pub fn load(b: &[u8]) -> Result<EFHudPalette, EngineError> {
        let mut r = Rd::new(b);
        let id = r.header()?;
        let n = r.u32()? as usize;
        let d = r.blob()?;
        if d.len() < n * 16 {
            return Err(EngineError::Format("PAL colour table".into()));
        }
        let mut p = EFHudPalette { id, entries: Vec::new() };
        p.Init(n as u32, d);
        Ok(p)
    }
    /// EFHudPalette::Init: copies 16 bytes/entry, clears flags (+0x10).
    pub fn Init(&mut self, n: u32, groups: &[u8]) {
        self.entries = (0..n as usize).map(|i| {
            let mut e = EFPaletteEntry::default();
            for k in 0..4 {
                e.colors[k].copy_from_slice(&groups[i * 16 + k * 4..i * 16 + k * 4 + 4]);
            }
            e
        }).collect();
    }
}

/// EFText (class 0x3b, "EF_TEXTS"; DTTextsDescriptor). One text-command blob per language.
#[derive(Debug, Clone, Default)]
pub struct EFText {
    /// this+0x14 languages, this+0x18 texts per language, this+0x1c tables
    /// (TEXTS_TABLE offsets, TEXTS data) per language.
    pub langs: Vec<(Vec<u32>, Vec<u8>)>,
}

impl EFText {
    pub fn GetClassID() -> u32 {
        0x3b
    }
    pub fn load(b: &[u8]) -> Result<EFText, EngineError> {
        let mut r = Rd::new(b);
        r.header()?;
        let nl = r.u32()?;
        let nt = r.u32()? as usize;
        let mut t = EFText::default();
        if nl == 0 || nt == 0 {
            return Ok(t);
        }
        for _ in 0..nl {
            let tab = r.blob()?;
            if tab.len() < nt * 4 {
                return Err(EngineError::Format("TX offset table".into()));
            }
            let offs = (0..nt).map(|i| le32(tab, i)).collect();
            let _size = r.u32()?; // vtbl+0x28 ReadU32 (allocation size)
            let _ = r.u8()?; // vtbl+0x0c Read u8
            t.langs.push((offs, r.blob()?.to_vec()));
        }
        Ok(t)
    }
    /// EFText::GetText(id): text-command stream of `id`.
    /// quirk: the C++ always reads language table 0 (`**(this+0x1c)`) whatever the current language.
    pub fn GetText(&self, id: u32) -> Option<&[u8]> {
        let (offs, data) = self.langs.first()?;
        let o = *offs.get(id as usize)? as usize;
        data.get(o..)
    }
}

/// EFLoadingScreen data (LOAD.LS, class 0x46): background + progress bar texture names.
#[derive(Debug, Clone, Default)]
pub struct EFLoadingScreen {
    pub background: String,
    pub progress_bar: String,
}

impl EFLoadingScreen {
    pub fn GetClassID() -> u32 {
        0x46
    }
    pub fn load(b: &[u8]) -> Result<EFLoadingScreen, EngineError> {
        let mut r = Rd::new(b);
        r.header()?;
        Ok(EFLoadingScreen { background: r.string()?, progress_bar: r.string()? })
    }
}

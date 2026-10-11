//! Game data that the port needs from the original module images, read at run time from the user's
//! disc image ("bring your own ISO"): nothing derived from the game's files is compiled into the
//! port. The logic modules (relocatable MIPS ELF objects inside the level packs, P2R entries
//! 0x27/0x45) carry their initialised globals in `.text`; the ported modules rebuild their data image
//! from those bytes instead of embedded tables.
//!
//! Modules are identified by the first 8 hex digits of the SHA-1 of the ELF image (the key the port
//! already uses for builds), so a wrong or modified disc gives a clear error instead of wrong data.
//! ELF images are registered when a pack is linked ([`register_elf`]); a module asked for before
//! its pack was loaded (tests) is found by scanning the disc's packs once.

use super::iso::IsoFs;
use super::{Mem, Ptr};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

static ISO_PATH: Mutex<Option<PathBuf>> = Mutex::new(None);
static ELFS: OnceLock<Mutex<HashMap<String, &'static [u8]>>> = OnceLock::new();
static SCANNED: Mutex<bool> = Mutex::new(false);

fn elfs() -> &'static Mutex<HashMap<String, &'static [u8]>> {
    ELFS.get_or_init(Default::default)
}

/// Remembers the disc image the engine opened (the first one wins); used for the fallback scan.
pub fn set_iso_path(p: &Path) {
    let mut g = ISO_PATH.lock().unwrap();
    if g.is_none() {
        *g = Some(p.to_path_buf());
    }
}

/// sha8 of an ELF image.
pub fn sha8(img: &[u8]) -> String {
    sha1_smol::Sha1::from(img).digest().to_string()[..8].to_string()
}

/// Registers a module ELF image (exact bytes, `sha8` = its SHA-1 prefix).
pub fn register_elf(sha8: &str, img: &[u8]) {
    let mut m = elfs().lock().unwrap();
    if !m.contains_key(sha8) {
        m.insert(sha8.to_string(), Box::leak(img.to_vec().into_boxed_slice()));
    }
}

/// Byte size of an ELF image: the furthest section / section header table end.
pub fn elf_size(b: &[u8]) -> usize {
    let r32 = |o: usize| b.get(o..o + 4).map_or(0, |s| u32::from_le_bytes(s.try_into().unwrap()) as usize);
    let r16 = |o: usize| b.get(o..o + 2).map_or(0, |s| u16::from_le_bytes(s.try_into().unwrap()) as usize);
    let (shoff, shentsize, shnum) = (r32(0x20), r16(0x2e), r16(0x30));
    let mut end = shoff + shentsize * shnum;
    for i in 0..shnum {
        let sh = shoff + i * shentsize;
        if sh + 40 > b.len() {
            break;
        }
        if r32(sh + 4) != 8 {
            end = end.max(r32(sh + 16) + r32(sh + 20));
        }
    }
    end.min(b.len())
}

/// Registers every ELF image of every pack (.P2R) on the disc.
fn scan_iso() {
    let mut done = SCANNED.lock().unwrap();
    if *done {
        return;
    }
    *done = true;
    let Some(path) = ISO_PATH.lock().unwrap().clone() else { return };
    let Ok(fs) = IsoFs::open(&path) else { return };
    let mut names: Vec<String> = fs.names().filter(|n| n.ends_with(".P2R")).map(String::from).collect();
    names.sort();
    for n in names {
        let Ok(d) = fs.read(&n) else { continue };
        let mut i = 0;
        while let Some(p) = d[i..].windows(4).position(|w| w == b"\x7fELF") {
            let at = i + p;
            let img = &d[at..at + elf_size(&d[at..])];
            if img.len() > 52 {
                register_elf(&sha8(img), img);
            }
            i = at + 4;
        }
    }
}

/// The module ELF image with this sha8. Panics with a clear message when the disc does not hold it.
pub fn module_elf(key: &str) -> &'static [u8] {
    if let Some(&b) = elfs().lock().unwrap().get(key) {
        return b;
    }
    scan_iso();
    match elfs().lock().unwrap().get(key) {
        Some(&b) => b,
        None => panic!(
            "game module {key} not found: this port needs the original 'Scooby-Doo! Mystery Mayhem' (PS2) disc image \
             ({}). Use --iso <path> or place the .ISO next to the executable.",
            ISO_PATH.lock().unwrap().as_ref().map_or("no disc image opened".into(), |p| format!("opened {}", p.display()))
        ),
    }
}

// ------------------------------------------------------------------------------------------ ELF32 LE

pub struct Section<'a> {
    pub name: &'a str,
    pub ty: u32,
    pub offset: usize,
    pub size: usize,
    pub link: usize,
    pub info: usize,
    pub data: &'a [u8],
}

pub struct Sym {
    pub value: u32,
    pub info: u8,
    pub shndx: u16,
    pub name: String,
}

pub struct Rel {
    pub offset: u32,
    pub ty: u32,
    pub sym: usize,
    pub addend: Option<i32>,
}

pub struct Elf<'a> {
    pub b: &'a [u8],
    pub secs: Vec<Section<'a>>,
}

fn rd32(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}

impl<'a> Elf<'a> {
    pub fn parse(b: &'a [u8]) -> Self {
        let r16 = |o: usize| u16::from_le_bytes(b[o..o + 2].try_into().unwrap()) as usize;
        let (shoff, shentsize, shnum, shstrndx) = (rd32(b, 0x20) as usize, r16(0x2e), r16(0x30), r16(0x32));
        let hdr = |i: usize| {
            let h = shoff + i * shentsize;
            (rd32(b, h), rd32(b, h + 4), rd32(b, h + 16) as usize, rd32(b, h + 20) as usize, rd32(b, h + 24) as usize, rd32(b, h + 28) as usize)
        };
        let (_, _, so, ss, _, _) = hdr(shstrndx);
        let strtab = &b[so..so + ss];
        let cs = |t: &'a [u8], o: usize| -> &'a str {
            let e = t[o..].iter().position(|&c| c == 0).map_or(t.len(), |p| o + p);
            std::str::from_utf8(&t[o..e]).unwrap_or("")
        };
        let secs = (0..shnum)
            .map(|i| {
                let (n, ty, offset, size, link, info) = hdr(i);
                let data = if ty == 8 { &b[0..0] } else { &b[offset..(offset + size).min(b.len())] };
                Section { name: cs(strtab, n as usize), ty, offset, size, link, info, data }
            })
            .collect();
        Self { b, secs }
    }

    pub fn index(&self, name: &str) -> usize {
        self.secs.iter().position(|s| s.name == name).unwrap_or_else(|| panic!("ELF has no {name} section"))
    }

    pub fn section(&self, name: &str) -> &Section<'a> {
        &self.secs[self.index(name)]
    }

    pub fn syms(&self, idx: usize) -> Vec<Sym> {
        let s = &self.secs[idx];
        let st = &self.secs[s.link].data;
        s.data
            .chunks_exact(16)
            .map(|e| {
                let n = rd32(e, 0) as usize;
                let end = st[n..].iter().position(|&c| c == 0).map_or(st.len(), |p| n + p);
                Sym {
                    name: String::from_utf8_lossy(&st[n..end]).into_owned(),
                    value: rd32(e, 4),
                    info: e[12],
                    shndx: u16::from_le_bytes([e[14], e[15]]),
                }
            })
            .collect()
    }

    pub fn symtab(&self) -> Vec<Sym> {
        self.syms(self.index(".symtab"))
    }

    /// Relocations of a SHT_REL (9) / SHT_RELA (4) section.
    pub fn rels(&self, idx: usize) -> Vec<Rel> {
        let s = &self.secs[idx];
        let n = if s.ty == 4 { 12 } else { 8 };
        s.data
            .chunks_exact(n)
            .map(|e| {
                let info = rd32(e, 4);
                Rel { offset: rd32(e, 0), ty: info & 0xff, sym: (info >> 8) as usize, addend: (s.ty == 4).then(|| rd32(e, 8) as i32) }
            })
            .collect()
    }
}

// ------------------------------------------------------------------------------ c2rs module images

/// Initialised data image of a c2rs-converted module build (what tools/rustport/c2rs.py `image()`
/// extracted): for every module address in `runs` (module-relative, 0x10000-based, end exclusive),
/// R_MIPS_32 relocations of `.text` against `.rodata*` become C strings (interned by the engine),
/// against `.text` data-relative pointers, other non-zero words are copied.
pub struct Image {
    pub words: Vec<(u32, u32)>,
    pub strs: Vec<(u32, String)>,
    pub rels: Vec<(u32, u32)>,
}

pub fn c2rs_image(key: &str, runs: &[(u32, u32)]) -> Image {
    let elf = Elf::parse(module_elf(key));
    let text = elf.section(".text").data;
    let tw = |o: usize| {
        let mut w = [0u8; 4];
        for (i, x) in w.iter_mut().enumerate() {
            *x = *text.get(o + i).unwrap_or(&0);
        }
        u32::from_le_bytes(w)
    };
    let inr = |a: u32| runs.iter().any(|&(x, y)| x <= a && a < y);
    let (mut strs, mut rels) = (Vec::new(), Vec::new());
    let mut relocated = std::collections::HashSet::new();
    for (si, sec) in elf.secs.iter().enumerate() {
        if !(sec.ty == 9 || sec.ty == 4) || !(sec.name == ".rel.text" || sec.name == ".rela.text") {
            continue;
        }
        let syms = elf.syms(sec.link);
        for r in elf.rels(si) {
            let o = r.offset;
            if !inr(0x10000 + o) || r.ty != 2 {
                continue;
            }
            let sy = &syms[r.sym];
            let tsec = (sy.shndx != 0 && (sy.shndx as usize) < 0xff00).then(|| &elf.secs[sy.shndx as usize]);
            let add = r.addend.map_or_else(|| tw(o as usize), |a| a as u32);
            let at = sy.value.wrapping_add(add);
            relocated.insert(0x10000 + o);
            match tsec {
                Some(t) if t.name.starts_with(".rodata") => {
                    let d = t.data;
                    let s = &d[at as usize..];
                    let s = &s[..s.iter().position(|&c| c == 0).expect("unterminated string")];
                    strs.push((0x10000 + o, s.iter().map(|&c| c as char).collect()));
                }
                Some(t) if t.name == ".text" => rels.push((0x10000 + o, 0x10000 + at)),
                _ => {}
            }
        }
    }
    let mut words = Vec::new();
    for &(x, y) in runs {
        for a in (x..y).step_by(4) {
            let w = tw((a - 0x10000) as usize);
            if w != 0 && !relocated.contains(&a) {
                words.push((a, w));
            }
        }
    }
    Image { words, strs, rels }
}

/// build_init of a c2rs module: words, then interned strings, then data-relative pointers.
pub fn apply_image(m: &mut Mem, data: Ptr, key: &str, runs: &[(u32, u32)]) {
    let img = c2rs_image(key, runs);
    for &(a, v) in &img.words {
        m.w32(data + (a - 0x10000), v);
    }
    for (a, st) in &img.strs {
        let p = m.cstr(st);
        m.w32(data + (a - 0x10000), p);
    }
    for &(a, t) in &img.rels {
        m.w32(data + (a - 0x10000), data + (t - 0x10000));
    }
}

/// C string at module address `a` (0x10000-based) of the module's .text (strings some builds keep there).
pub fn text_cstr(key: &str, a: u32) -> String {
    let elf = Elf::parse(module_elf(key));
    let s = &elf.section(".text").data[(a - 0x10000) as usize..];
    s[..s.iter().position(|&c| c == 0).expect("unterminated string")].iter().map(|&c| c as char).collect()
}

// --------------------------------------------------------------------------- per-build word arrays

/// Raw `.text` words of a module: `n` words at module address `addr` (0x10000-based). Cached.
pub fn words(key: &str, addr: u32, n: u32) -> &'static &'static [u32] {
    static EMPTY: &[u32] = &[];
    static CACHE: OnceLock<Mutex<HashMap<(String, u32, u32), &'static &'static [u32]>>> = OnceLock::new();
    if n == 0 {
        return &EMPTY;
    }
    let c = CACHE.get_or_init(Default::default);
    if let Some(&w) = c.lock().unwrap().get(&(key.to_string(), addr, n)) {
        return w;
    }
    let elf = Elf::parse(module_elf(key));
    let text = elf.section(".text").data;
    let v: Vec<u32> = (0..n as usize)
        .map(|i| {
            let o = (addr - 0x10000) as usize + 4 * i;
            u32::from_le_bytes(text[o..o + 4].try_into().unwrap())
        })
        .collect();
    let w: &'static &'static [u32] = Box::leak(Box::new(&*Box::leak(v.into_boxed_slice())));
    *c.lock().unwrap().entry((key.to_string(), addr, n)).or_insert(w)
}

/// Per-build initialised global arrays: (module sha8, module address, word count) each; `("", 0, 0)` =
/// absent from that build. Indexing yields the words (read from the disc image on first use).
pub struct Words<const N: usize>(pub [(&'static str, u32, u32); N]);

impl<const N: usize> std::ops::Index<usize> for Words<N> {
    type Output = &'static [u32];
    fn index(&self, i: usize) -> &&'static [u32] {
        let (k, a, n) = self.0[i];
        words(k, a, n)
    }
}

impl<const N: usize> Words<N> {
    pub const fn len(&self) -> usize {
        N
    }
}

/// Non-zero raw `.text` words (relocated words keep their unrelocated value) in module address `runs`.
pub fn raw_words(key: &str, runs: &[(u32, u32)]) -> Vec<(u32, u32)> {
    let elf = Elf::parse(module_elf(key));
    let text = elf.section(".text").data;
    let mut out = Vec::new();
    for &(x, y) in runs {
        for a in (x..y).step_by(4) {
            let o = (a - 0x10000) as usize;
            let w = u32::from_le_bytes(text[o..o + 4].try_into().unwrap());
            if w != 0 {
                out.push((a, w));
            }
        }
    }
    out
}

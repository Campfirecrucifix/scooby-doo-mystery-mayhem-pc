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

/// P39 R3: `panic!` for game data the disc does not hold -> the one [`missing`] error path
macro_rules! missing {
    ($($t:tt)*) => {
        missing(format!($($t)*))
    };
}

static ISO_PATH: Mutex<Option<PathBuf>> = Mutex::new(None);
static ELFS: OnceLock<Mutex<HashMap<String, &'static [u8]>>> = OnceLock::new();
static SCANNED: Mutex<bool> = Mutex::new(false);

fn elfs() -> &'static Mutex<HashMap<String, &'static [u8]>> {
    ELFS.get_or_init(Default::default)
}

/// Remembers the disc image the engine opened (the first one wins); used for the fallback scan.
pub fn set_iso_path(p: &Path) {
    // P39 R6: unit tests open scratch images in the temp dir (iso.rs / settings.rs p39_* tests); first-wins
    // would make one of those the fallback-scan disc for every later test in the process (lit_strings_present
    // read a temp `ok.iso`): scratch images never register
    #[cfg(test)]
    if p.starts_with(std::env::temp_dir()) {
        return;
    }
    let mut g = ISO_PATH.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    if g.is_none() {
        *g = Some(p.to_path_buf());
    }
}

/// P40 R6: the disc image for tests from any worktree: $SCOOBY_ISO, $GAME_ISO, the worktree root
/// (recomp/..), then the main checkout found through the worktree's `.git` file (gitdir: <main>/.git/worktrees/<n>).
pub fn find_test_iso() -> Option<PathBuf> {
    const NAME: &str = "Scooby Doo Mystery Mayhem.ISO";
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut c: Vec<PathBuf> = ["SCOOBY_ISO", "GAME_ISO"].iter().filter_map(|v| std::env::var_os(v)).map(PathBuf::from).collect();
    c.push(root.join(NAME));
    if let Some(gd) = std::fs::read_to_string(root.join(".git")).ok().and_then(|s| s.strip_prefix("gitdir:").map(|g| PathBuf::from(g.trim()))) {
        // <main>/.git/worktrees/<n> -> <main>
        if let Some(main) = gd.ancestors().nth(3) {
            c.push(main.join(NAME));
        }
    }
    c.into_iter().find(|p| p.is_file())
}

/// sha8 of an ELF image.
pub fn sha8(img: &[u8]) -> String {
    sha1_smol::Sha1::from(img).digest().to_string()[..8].to_string()
}

/// Registers a module ELF image (exact bytes, `sha8` = its SHA-1 prefix).
pub fn register_elf(sha8: &str, img: &[u8]) {
    let mut m = elfs().lock().unwrap_or_else(std::sync::PoisonError::into_inner);
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
    let mut done = SCANNED.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    if *done {
        return;
    }
    // WP-T1: only a real scan counts (a lookup before any disc was opened must not disable the scan for the
    // whole process: tests share one process and their order decides which one comes first)
    let Some(path) = ISO_PATH.lock().unwrap_or_else(std::sync::PoisonError::into_inner).clone() else { return };
    *done = true;
    register_iso_elfs(&path);
}

/// Registers every ELF image of every pack (.P2R) of the disc image at `path` (content-addressed, so a
/// second image only adds). P40 R7: tests call it with the real disc (another test may have opened a
/// temporary image first, and ISO_PATH keeps the first one).
pub fn register_iso_elfs(path: &Path) {
    let Ok(fs) = IsoFs::open(path) else { return };
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

/// The module ELF image with this sha8, None when the disc does not hold it.
pub fn module_elf_opt(key: &str) -> Option<&'static [u8]> {
    if let Some(&b) = elfs().lock().unwrap_or_else(std::sync::PoisonError::into_inner).get(key) {
        return Some(b);
    }
    scan_iso();
    elfs().lock().unwrap_or_else(std::sync::PoisonError::into_inner).get(key).copied()
}

/// The module ELF image with this sha8. Panics with a clear message when the disc does not hold it.
pub fn module_elf(key: &str) -> &'static [u8] {
    if let Some(&b) = elfs().lock().unwrap_or_else(std::sync::PoisonError::into_inner).get(key) {
        return b;
    }
    scan_iso();
    let found = elfs().lock().unwrap_or_else(std::sync::PoisonError::into_inner).get(key).copied(); // guard dropped before a panic (no poisoned ELFS)
    if let Some(b) = found {
        return b;
    }
    // WP-X4: the ISO_PATH guard must be dropped before panicking too (a temporary inside `panic!(..)` lives
    // through the unwind and poisoned ISO_PATH, so every later set_iso_path failed: one missing-ISO test took
    // down ~126 tests in the shared test binary)
    missing(format!("game module {key} not found"))
}

/// P39 R3: the game.exe hook that shows a [`missing`] message to the player (message box) before the panic
pub static FATAL_HOOK: OnceLock<fn(&str)> = OnceLock::new();

/// P39 R3: the one error path for game data this disc does not hold (a module ELF, a symbol, a function, a
/// literal / locator of a data table): an untested release gets the "disc version isn't supported" message,
/// the tested one "damaged or modified", no disc "use --iso". Shown through [`FATAL_HOOK`] (game.exe), then a
/// panic (its crash.log record by the crash-log hook; tests see the panic as before).
#[cold]
#[track_caller]
pub fn missing(what: String) -> ! {
    // WP-X4: no ISO_PATH guard may live through the panic (it would poison ISO_PATH for every later test)
    let path = ISO_PATH.lock().unwrap_or_else(std::sync::PoisonError::into_inner).clone();
    let hint = missing_hint(path.as_deref());
    let msg = format!("{what}: {hint}");
    if let Some(f) = FATAL_HOOK.get() {
        f(&msg);
    }
    panic!("{msg}")
}

/// P39 R3: why the disc lacks game data, for the player ([`missing`])
pub fn missing_hint(path: Option<&Path>) -> String {
    match path.map(|p| (p, super::settings::disc_release(p))) {
        Some((_, Some((elf, false)))) => super::settings::unsupported_disc(&elf),
        Some((p, Some((_, true)))) => format!("the disc image {} may be damaged or modified (re-dump the disc)", p.display()),
        Some((p, None)) => format!(
            "this port needs the original 'Scooby-Doo! Mystery Mayhem' (PS2) disc image (opened {}). Use --iso <path> or place the .ISO next to the executable.",
            p.display()
        ),
        None => "this port needs the original 'Scooby-Doo! Mystery Mayhem' (PS2) disc image (no disc image opened). Use --iso <path> or place the .ISO next to the executable.".into(),
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
        self.secs.iter().position(|s| s.name == name).unwrap_or_else(|| missing!("ELF has no {name} section"))
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
    if let Some(&w) = c.lock().unwrap_or_else(std::sync::PoisonError::into_inner).get(&(key.to_string(), addr, n)) {
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
    *c.lock().unwrap_or_else(std::sync::PoisonError::into_inner).entry((key.to_string(), addr, n)).or_insert(w)
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

// ------------------------------------------------------------------- per-build literals (WP-X2)

/// One literal locator: `KLoc(fn, shape, build, kind, off, aux)` — the literal sits in function
/// `fns[fn]` at instruction `off` (word index from the function start) and is decoded per `kind`
/// (see [`decode_lit`]). It applies to every build whose function has masked-code hash `shape`
/// ([`shape_hash`]) when `build` is 255, else only to build `build` (index in [`KSpec::builds`]).
pub struct KLoc(pub u16, pub u32, pub u8, pub u8, pub u16, pub u8);

/// Locators of one entry; builds no locator applies to take the value of build `home` (the entry's
/// value in the build gen_builds.py took it from) unless listed in `lits` (values that are no
/// immediate of any instruction: Ghidra-only artefacts of misaligned code shapes).
pub struct KEntry {
    pub home: u8,
    pub locs: &'static [KLoc],
    pub lits: &'static [(u8, u32)],
}

impl KEntry {
    /// An entry no port code reads (value 0).
    pub const UNUSED: KEntry = KEntry { home: 255, locs: &[], lits: &[] };
}

/// Per-build literal table of a module family (generated by tools/rustport/x2_gen.py).
pub struct KSpec {
    pub builds: &'static [&'static str],
    pub fns: &'static [&'static str],
    pub entries: &'static [KEntry],
}

/// Row `build` of a [`KSpec`]: `row[i]` is entry `i` of that build, read from its module ELF on first use.
pub struct KRow(pub &'static KSpec, pub usize);

/// Function symbols (name -> (.text offset, size)) of a module ELF. Cached.
fn funcs(key: &str) -> &'static HashMap<String, (u32, u32)> {
    static CACHE: OnceLock<Mutex<HashMap<String, &'static HashMap<String, (u32, u32)>>>> = OnceLock::new();
    let c = CACHE.get_or_init(Default::default);
    if let Some(&f) = c.lock().unwrap_or_else(std::sync::PoisonError::into_inner).get(key) {
        return f;
    }
    let elf = Elf::parse(module_elf(key));
    let st = elf.section(".symtab");
    let strs = elf.secs[st.link].data;
    let mut m = HashMap::new();
    for e in st.data.chunks_exact(16) {
        // STT_FUNC with a size; the first symbol of a name wins
        let (n, size) = (rd32(e, 0) as usize, rd32(e, 8));
        if e[12] & 0xf != 2 || size == 0 {
            continue;
        }
        let end = strs[n..].iter().position(|&c| c == 0).map_or(strs.len(), |p| n + p);
        m.entry(String::from_utf8_lossy(&strs[n..end]).into_owned()).or_insert((rd32(e, 4), size));
    }
    let f: &'static HashMap<String, (u32, u32)> = Box::leak(Box::new(m));
    *c.lock().unwrap_or_else(std::sync::PoisonError::into_inner).entry(key.to_string()).or_insert(f)
}

/// WP-X4: the initialised data object `name` (OBJECT symbol of `.text`, `st_size` bytes) of module `key`
/// as raw (unrelocated) words: (module address, words). Replaces ModuleDef::init literal tables.
pub fn sym_words(key: &str, name: &str) -> (u32, &'static [u32]) {
    let (o, size) = *data_syms(key).get(name).unwrap_or_else(|| missing!("{key}: no symbol {name}"));
    assert!(size != 0, "{key}: symbol {name} has no size");
    (0x10000 + o, words(key, 0x10000 + o, size / 4))
}

/// WP-X4: writes the named data objects of module `key` (see [`sym_words`]) into a module data image.
pub fn init_syms(m: &mut Mem, data: Ptr, key: &str, names: &[&str]) {
    for n in names {
        let (a, w) = sym_words(key, n);
        for (i, &v) in w.iter().enumerate() {
            m.w32(data + (a - 0x10000) + 4 * i as u32, v);
        }
    }
}

/// WP-X4: writes the raw `.text` words of module `key` in the module address runs `[x, y)` into a module
/// data image (objects without an ELF size, e.g. function statics).
pub fn init_runs(m: &mut Mem, data: Ptr, key: &str, runs: &[(u32, u32)]) {
    for &(x, y) in runs {
        for (i, &v) in words(key, x, (y - x) / 4).iter().enumerate() {
            m.w32(data + (x - 0x10000) + 4 * i as u32, v);
        }
    }
}

/// Defined non-function symbols of the module's `.text` (name -> (offset, size)). Cached.
/// Value (.text offset) of the NOTYPE/OBJECT `.text` symbol `name` of a module (panics if absent).
pub fn text_sym(key: &str, name: &str) -> u32 {
    data_syms(key).get(name).unwrap_or_else(|| missing!("{key}: no symbol {name}")).0
}

fn data_syms(key: &str) -> &'static HashMap<String, (u32, u32)> {
    static CACHE: OnceLock<Mutex<HashMap<String, &'static HashMap<String, (u32, u32)>>>> = OnceLock::new();
    let c = CACHE.get_or_init(Default::default);
    if let Some(&f) = c.lock().unwrap_or_else(std::sync::PoisonError::into_inner).get(key) {
        return f;
    }
    let elf = Elf::parse(module_elf(key));
    let text = elf.index(".text") as u16;
    let mut m = HashMap::new();
    for s in elf.symtab() {
        if s.shndx == text && matches!(s.info & 0xf, 0 | 1) && !s.name.is_empty() {
            m.entry(s.name).or_insert((s.value, 0));
        }
    }
    // sizes (st_size is not kept by `Sym`)
    let st = elf.section(".symtab");
    let strs = elf.secs[st.link].data;
    for e in st.data.chunks_exact(16) {
        let n = rd32(e, 0) as usize;
        let end = strs[n..].iter().position(|&c| c == 0).map_or(strs.len(), |p| n + p);
        if let Some(x) = m.get_mut(&*String::from_utf8_lossy(&strs[n..end])) {
            if x.0 == rd32(e, 4) {
                x.1 = rd32(e, 8);
            }
        }
    }
    let f: &'static HashMap<String, (u32, u32)> = Box::leak(Box::new(m));
    *c.lock().unwrap_or_else(std::sync::PoisonError::into_inner).entry(key.to_string()).or_insert(f)
}

/// Word indices (from module address `base`) of the `.text` words below `base + 4 * n` that carry an
/// R_MIPS_32 relocation (data-relative pointers of an initialised image).
pub fn rel_words(key: &str, base: u32, n: u32) -> &'static [usize] {
    let elf = Elf::parse(module_elf(key));
    let mut v = Vec::new();
    for (si, sec) in elf.secs.iter().enumerate() {
        if (sec.ty == 9 || sec.ty == 4) && (sec.name == ".rel.text" || sec.name == ".rela.text") {
            for r in elf.rels(si) {
                let a = 0x10000 + r.offset;
                if r.ty == 2 && a >= base && a < base + 4 * n {
                    v.push(((a - base) / 4) as usize);
                }
            }
        }
    }
    v.sort();
    v.dedup();
    lk(v)
}

/// A string built at run time (table rows naming their build).
pub fn lks(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}

fn text_word(text: &[u8], o: usize) -> u32 {
    let mut w = [0u8; 4];
    for (i, x) in w.iter_mut().enumerate() {
        *x = *text.get(o + i).unwrap_or(&0);
    }
    u32::from_le_bytes(w)
}

/// FNV-1a 32 of a function's instruction words with immediates / jump targets masked: builds with
/// the same hash have the same code apart from literals.
pub fn shape_hash(text: &[u8], base: u32, size: u32) -> u32 {
    let mut h = 0x811c9dc5u32;
    for i in 0..size / 4 {
        let w = text_word(text, (base + 4 * i) as usize);
        let w = match w >> 26 {
            2 | 3 => w & 0xfc00_0000,
            op if op >= 4 => w & 0xffff_0000,
            _ => w,
        };
        for k in 0..4 {
            h = (h ^ ((w >> (8 * k)) & 0xff)).wrapping_mul(0x0100_0193);
        }
    }
    h
}

/// Literal of a MIPS instruction (sequence) at `base + 4*off` of `.text`. Kinds: 0 simm16, 1 zimm16,
/// 2 lui << 16, 3 lui + ori/addiu `aux` instructions later (same register), 4 that pair as a module
/// address (+0x10000), 5 the `.text` word that pair loads (lw/lwc1), 6/7 simm16 +1/-1 (`<=` compiled
/// as `<`), 8 the negated pair, 12/13 the pair +1/-1 (WP-X3). Kinds 9..11 (WP-X3, see `k_value`) name a data symbol instead of a
/// function: 9 its module address, 10 its size, 11 its end as a word count.
pub fn decode_lit(text: &[u8], base: u32, kind: u8, off: u16, aux: u8) -> Option<u32> {
    let at = |o: u32| text_word(text, (base + 4 * o) as usize);
    let itype = |i: u32| i >> 26 >= 8 && i >> 26 != 0x0f;
    let simm = |i: u32| i as u16 as i16 as i32 as u32;
    let i = at(off as u32);
    match kind {
        0 | 1 | 6 | 7 => itype(i).then(|| match kind {
            0 => simm(i),
            1 => i & 0xffff,
            6 => simm(i).wrapping_add(1),
            _ => simm(i).wrapping_sub(1),
        }),
        _ => {
            if i >> 26 != 0x0f {
                return None;
            }
            let hi = (i & 0xffff) << 16;
            if kind == 2 {
                return Some(hi);
            }
            let j = at(off as u32 + aux as u32);
            if !itype(j) || (j >> 21) & 31 != (i >> 16) & 31 {
                return None;
            }
            let pair = match j >> 26 {
                0x0d => Some(hi | (j & 0xffff)),
                0x09 => Some(hi.wrapping_add(simm(j))),
                _ => None,
            };
            let a = hi.wrapping_add(simm(j));
            match kind {
                3 => pair,
                12 => pair.map(|v| v.wrapping_add(1)),
                13 => pair.map(|v| v.wrapping_sub(1)),
                8 => pair.map(|v| v.wrapping_neg()),
                4 => (j >> 26 == 0x09).then(|| a.wrapping_add(0x10000)),
                5 => (matches!(j >> 26, 0x23 | 0x31) && (a as usize) + 4 <= text.len()).then(|| text_word(text, a as usize)),
                _ => None,
            }
        }
    }
}

/// Entry `i` of build `b`.
fn k_value(spec: &'static KSpec, b: usize, i: usize) -> u32 {
    let e = &spec.entries[i];
    if e.home == 255 {
        return 0;
    }
    let key = spec.builds[b];
    let text = Elf::parse(module_elf(key)).section(".text").data;
    let fs = funcs(key);
    for l in e.locs {
        if l.2 != 255 && l.2 as usize != b {
            continue;
        }
        let Some(&(base, size)) = fs.get(spec.fns[l.0 as usize]) else { continue };
        if l.2 == 255 && shape_hash(text, base, size) != l.1 {
            continue;
        }
        return decode_lit(text, base, l.3, l.4, l.5)
            .unwrap_or_else(|| missing!("{key}: literal {i} of {} not decodable", spec.fns[l.0 as usize]));
    }
    if let Some(&(_, v)) = e.lits.iter().find(|x| x.0 as usize == b) {
        return v;
    }
    assert!(e.home as usize != b, "{key}: no locator for literal {i}");
    KRow(spec, e.home as usize)[i]
}

impl std::ops::Index<usize> for KRow {
    type Output = u32;
    fn index(&self, i: usize) -> &u32 {
        type Row = &'static [OnceLock<u32>];
        static ROWS: OnceLock<Mutex<HashMap<(usize, usize), Row>>> = OnceLock::new();
        let key = (self.0 as *const KSpec as usize, self.1);
        let row = {
            let mut m = ROWS.get_or_init(Default::default).lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            *m.entry(key).or_insert_with(|| Box::leak((0..self.0.entries.len()).map(|_| OnceLock::new()).collect()))
        };
        row[i].get_or_init(|| k_value(self.0, self.1, i))
    }
}

// ------------------------------------------------------------------- per-build tables (WP-X3)

/// Conversion of a literal word read from a module ELF to the type of the table field it fills.
pub trait FromK {
    fn from_k(v: u32) -> Self;
}

macro_rules! from_k_int {
    ($($t:ty),*) => { $(impl FromK for $t { fn from_k(v: u32) -> Self { v as $t } })* };
}
from_k_int!(u32, i32, u16, i16, u8, i8, usize);

impl FromK for f32 {
    fn from_k(v: u32) -> Self {
        f32::from_bits(v)
    }
}

impl FromK for bool {
    fn from_k(v: u32) -> Self {
        v != 0
    }
}

/// Entry of a per-build [`Table`] (WP-X3): `L(fn, kind, off, aux)` one locator valid for every build
/// that reads the entry (no shape check), `M(locs)` locators as in [`KEntry`] (shape hash / build
/// checked, first match wins), `X(locs, lits)` plus per-build values no instruction holds.
pub enum TE {
    L(u16, u8, u16, u8),
    M(&'static [KLoc]),
    X(&'static [KLoc], &'static [(u8, u32)]),
}

/// Literal locators of a [`Table`] (generated by tools/rustport/x3_gen.py).
pub struct TSpec {
    pub builds: &'static [&'static str],
    pub fns: &'static [&'static str],
    pub entries: &'static [TE],
}

/// Build `.1` of a [`TSpec`]: `k.v(i)` = entry `i` of that build, read from its module ELF.
pub struct TRow(pub &'static TSpec, pub usize);

fn loc_value(key: &str, fns: &[&str], b: usize, l: &KLoc, check: bool) -> Option<u32> {
    if l.2 != 255 && l.2 as usize != b {
        return None;
    }
    if (9..=11).contains(&l.3) {
        // symbol locators: `fns[fn]` names a data symbol of the module
        let &(v, size) = data_syms(key).get(fns[l.0 as usize])?;
        return Some(match l.3 {
            9 => v + 0x10000,
            10 => size,
            11 => (v + size) / 4,
            _ => missing!("{key}: bad locator kind {}", l.3),
        });
    }
    let text = Elf::parse(module_elf(key)).section(".text").data;
    let &(base, size) = funcs(key).get(fns[l.0 as usize])?;
    if check && l.2 == 255 && shape_hash(text, base, size) != l.1 {
        return None;
    }
    Some(decode_lit(text, base, l.3, l.4, l.5).unwrap_or_else(|| missing!("{key}: literal of {} not decodable", fns[l.0 as usize])))
}

impl TRow {
    /// Entry `i`, converted to the field type.
    pub fn v<T: FromK>(&self, i: usize) -> T {
        let (spec, b) = (self.0, self.1);
        let key = spec.builds[b];
        let v = match &spec.entries[i] {
            &TE::L(f, kind, off, aux) => loc_value(key, spec.fns, b, &KLoc(f, 0, 255, kind, off, aux), false),
            TE::M(locs) => locs.iter().find_map(|l| loc_value(key, spec.fns, b, l, true)),
            TE::X(locs, lits) => locs
                .iter()
                .find_map(|l| loc_value(key, spec.fns, b, l, true))
                .or_else(|| lits.iter().find(|x| x.0 as usize == b).map(|x| x.1)),
        };
        T::from_k(v.unwrap_or_else(|| missing!("{key}: no locator for table entry {i}")))
    }
}

/// A slice built at run time that lives as long as the (cached) table row.
pub fn lk<T>(v: Vec<T>) -> &'static [T] {
    Box::leak(v.into_boxed_slice())
}

/// Per-build table (generated by tools/rustport/x3_gen.py): row `b` is built on first use by `f` from
/// the table's skeleton and the literals of build `b` ([`TRow`] of `spec`), read from its module ELF.
pub struct Table<T: 'static> {
    pub spec: &'static TSpec,
    pub f: fn(&TRow) -> T,
    rows: OnceLock<Box<[OnceLock<&'static T>]>>,
}

impl<T: 'static> Table<T> {
    pub const fn new(spec: &'static TSpec, f: fn(&TRow) -> T) -> Self {
        Self { spec, f, rows: OnceLock::new() }
    }
    pub fn len(&self) -> usize {
        self.spec.builds.len()
    }
    pub fn is_empty(&self) -> bool {
        self.spec.builds.is_empty()
    }
    pub fn iter(&self) -> impl Iterator<Item = &'static T> + '_ {
        (0..self.len()).map(move |b| self.row(b))
    }
    pub fn row(&self, b: usize) -> &'static T {
        let rows = self.rows.get_or_init(|| (0..self.len()).map(|_| OnceLock::new()).collect());
        rows[b].get_or_init(|| Box::leak(Box::new((self.f)(&TRow(self.spec, b)))))
    }
}

impl<T: 'static> std::ops::Index<usize> for Table<T> {
    type Output = T;
    fn index(&self, b: usize) -> &T {
        self.row(b)
    }
}

// ------------------------------------------------------------------------- constant stores (WP-X3)

/// Constant stores of a function: the instructions are scanned in address order, tracking registers
/// set by lui/ori/addiu (and mtc1 into FP registers); every `sw`/`swc1` of a known value through a
/// base other than `$sp` is `(offset, value)` of the current run, and a run ends after each branch
/// / jump delay slot (runs without stores are dropped). For functions that are nothing but a switch
/// of constant stores (ghost CreateEmitterPos), run `i` is the stores of branch `i` in source order.
pub fn store_runs(key: &str, func: &str) -> &'static [Vec<(u32, u32)>] {
    static CACHE: OnceLock<Mutex<HashMap<(String, String), &'static [Vec<(u32, u32)>]>>> = OnceLock::new();
    let c = CACHE.get_or_init(Default::default);
    if let Some(&r) = c.lock().unwrap_or_else(std::sync::PoisonError::into_inner).get(&(key.to_string(), func.to_string())) {
        return r;
    }
    let text = Elf::parse(module_elf(key)).section(".text").data;
    let &(base, size) = funcs(key).get(func).unwrap_or_else(|| missing!("{key}: no function {func}"));
    let (mut regs, mut fr) = ([None::<u32>; 32], [None::<u32>; 32]);
    let (mut runs, mut cur, mut pend) = (Vec::new(), Vec::new(), false);
    for o in (base..base + size).step_by(4) {
        let i = text_word(text, o as usize);
        let (op, rs, rt, rd) = (i >> 26, (i >> 21 & 31) as usize, (i >> 16 & 31) as usize, (i >> 11 & 31) as usize);
        let (im, si) = (i & 0xffff, i as u16 as i16 as i32 as u32);
        let end = std::mem::replace(&mut pend, false);
        regs[0] = Some(0);
        match op {
            0x0f => regs[rt] = Some(im << 16),
            0x0d => regs[rt] = regs[rs].map(|v| v | im),
            0x09 => regs[rt] = regs[rs].map(|v| v.wrapping_add(si)),
            0x2b if rs != 29 => cur.extend(regs[rt].map(|v| (si, v))),
            0x39 if rs != 29 => cur.extend(fr[rt].map(|v| (si, v))),
            0x11 if rs == 4 => fr[rd] = regs[rt],
            1 | 4..=7 | 20..=23 => pend = true,
            0 if matches!(i & 0x3f, 8 | 9) => pend = true,
            0 => regs[rd] = None,
            0x08 | 0x0a..=0x0c | 0x0e | 0x20 | 0x21 | 0x23..=0x25 => regs[rt] = None,
            _ => {}
        }
        if end && !cur.is_empty() {
            runs.push(std::mem::take(&mut cur));
        }
    }
    if !cur.is_empty() {
        runs.push(cur);
    }
    let r: &'static [Vec<(u32, u32)>] = Box::leak(runs.into_boxed_slice());
    *c.lock().unwrap_or_else(std::sync::PoisonError::into_inner).entry((key.to_string(), func.to_string())).or_insert(r)
}

/// Run `.2` of [`store_runs`]`(.0, .1)`, read from the disc on first use.
#[derive(Clone, Copy)]
pub struct StoreRun(pub &'static str, pub &'static str, pub usize);

impl std::ops::Deref for StoreRun {
    type Target = [(u32, u32)];
    fn deref(&self) -> &[(u32, u32)] {
        &store_runs(self.0, self.1)[self.2]
    }
}

impl std::fmt::Debug for StoreRun {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        (**self).fmt(f)
    }
}

// ------------------------------------------------------------------- leaf function evaluation (WP-X3)

/// Runs a leaf module function (`func` of module `key`, MIPS integer subset: no calls, no FPU) with
/// `$a0 = a0` and `$a1` pointing at an output block, and returns the words it stored at `$a1+4` /
/// `$a1+8` (None: neither written). `.text` is the memory image (relocated words keep their
/// unrelocated value, i.e. `.text` offsets, which is what the code's own address pairs produce, so
/// switch jump tables work unrelocated). Used for per-build `switch(id)` / compare-tree functions
/// whose case tables the port needs (panel_baseFUNC_GetDisplayInfo).
pub fn eval_leaf(key: &str, func: &str, a0: u32) -> Option<(Option<u32>, Option<u32>)> {
    const SP: u32 = 0x7f00_0000;
    const OUT: u32 = 0x7e00_0000;
    const RA: u32 = 0xdead_0000;
    let text = Elf::parse(module_elf(key)).section(".text").data;
    let &(base, _) = funcs(key).get(func).unwrap_or_else(|| missing!("{key}: no function {func}"));
    let mut r = [0u32; 32];
    (r[29], r[4], r[5], r[31]) = (SP, a0, OUT, RA);
    let mut mem: HashMap<u32, u32> = HashMap::new();
    let (mut pc, mut npc) = (base, base + 4);
    let s = |x: u32| x as i32;
    for _ in 0..100_000 {
        if pc == RA {
            let out = (mem.get(&(OUT + 4)).copied(), mem.get(&(OUT + 8)).copied());
            return (out != (None, None)).then_some(out);
        }
        let i = text_word(text, pc as usize);
        let (op, rs, rt, rd, sh) = (i >> 26, (i >> 21 & 31) as usize, (i >> 16 & 31) as usize, (i >> 11 & 31) as usize, i >> 6 & 31);
        let (im, si) = (i & 0xffff, i as u16 as i16 as i32 as u32);
        let tgt = npc.wrapping_add(si << 2);
        let (mut w, mut br, mut likely) = (None, None, false);
        match op {
            0 => match i & 63 {
                0x21 | 0x2d => w = Some((rd, r[rs].wrapping_add(r[rt]))),
                0x25 => w = Some((rd, r[rs] | r[rt])),
                0x23 => w = Some((rd, r[rs].wrapping_sub(r[rt]))),
                0 => w = Some((rd, r[rt] << sh)),
                2 => w = Some((rd, r[rt] >> sh)),
                0x2b => w = Some((rd, (r[rs] < r[rt]) as u32)),
                0x2a => w = Some((rd, (s(r[rs]) < s(r[rt])) as u32)),
                8 => br = Some(r[rs]),
                f => missing!("{key} {func}: unsupported SPECIAL {f:#x} at {pc:#x}"),
            },
            9 => w = Some((rt, r[rs].wrapping_add(si))),
            0x0f => w = Some((rt, im << 16)),
            0x0d => w = Some((rt, r[rs] | im)),
            0x0e => w = Some((rt, r[rs] ^ im)),
            0x0b => w = Some((rt, (r[rs] < si) as u32)),
            0x0a => w = Some((rt, (s(r[rs]) < s(si)) as u32)),
            0x2b | 0x3f => {
                mem.insert(r[rs].wrapping_add(si), r[rt]);
            }
            0x23 | 0x37 => {
                let a = r[rs].wrapping_add(si);
                w = Some((rt, mem.get(&a).copied().unwrap_or_else(|| text_word(text, a as usize))));
            }
            4 | 5 | 0x14 | 0x15 => {
                likely = op >= 0x14;
                if (r[rs] == r[rt]) == (op & 1 == 0) {
                    br = Some(tgt);
                }
            }
            6 | 7 => {
                if (op == 6) == (s(r[rs]) <= 0) {
                    br = Some(tgt);
                }
            }
            1 => {
                likely = rt & 2 != 0;
                if (rt & 1 == 0) == (s(r[rs]) < 0) {
                    br = Some(tgt);
                }
            }
            _ => missing!("{key} {func}: unsupported op {op:#x} at {pc:#x}"),
        }
        if let Some((d, v)) = w {
            if d != 0 {
                r[d] = v;
            }
        }
        if likely && br.is_none() {
            // branch-likely not taken: the delay slot is annulled
            (pc, npc) = (npc + 4, npc + 8);
        } else {
            (pc, npc) = (npc, br.unwrap_or(npc + 4));
        }
    }
    missing!("{key} {func}: no return")
}

/// `(id, out+4, out+8)` for every id a `switch(id)` / compare-tree leaf function ([`eval_leaf`])
/// writes: ids from 2 below the smallest to 0x400 above the largest object id (0x2a......) the code
/// builds with lui/ori, as such or negated (`id - base` jump tables).
pub fn eval_cases(key: &str, func: &str) -> Vec<(u32, u32, u32)> {
    let text = Elf::parse(module_elf(key)).section(".text").data;
    let &(base, size) = funcs(key).get(func).unwrap_or_else(|| missing!("{key}: no function {func}"));
    let ws: Vec<u32> = (base..base + size).step_by(4).map(|o| text_word(text, o as usize)).collect();
    let mut ids = Vec::new();
    for (j, &i) in ws.iter().enumerate() {
        if i >> 26 != 0x0f {
            continue;
        }
        for &i2 in ws.iter().skip(j + 1).take(3) {
            if i2 >> 26 == 0x0d && (i2 >> 21 & 31) == (i >> 16 & 31) {
                let v = (i & 0xffff) << 16 | (i2 & 0xffff);
                ids.extend([v, v.wrapping_neg()].into_iter().filter(|x| (0x2a00_0000..0x2b00_0000).contains(x)));
            }
        }
    }
    let (Some(&lo), Some(&hi)) = (ids.iter().min(), ids.iter().max()) else { return Vec::new() };
    (lo - 2..hi + 0x400)
        .filter_map(|id| eval_leaf(key, func, id).map(|(a, b)| (id, a.unwrap_or(0), b.unwrap_or(0))))
        .collect()
}

#[cfg(test)]
mod p39_tests {
    /// P39 R6: opening a scratch image in a unit test never becomes the process-wide fallback disc
    #[test]
    fn p39_scratch_iso_does_not_register() {
        let d = crate::engine::iso::tests::tmp("noreg");
        let p = d.join("ok.iso");
        std::fs::write(&p, crate::engine::iso::tests::build_iso(&[("SYSTEM.CNF", b"BOOT2 = cdrom0:\\PBPX_955.06;1")])).unwrap();
        crate::engine::iso::IsoFs::open(&p).unwrap();
        let g = super::ISO_PATH.lock().unwrap_or_else(std::sync::PoisonError::into_inner).clone();
        assert!(g.as_ref().is_none_or(|g| !g.starts_with(std::env::temp_dir())), "{g:?}");
        let _ = std::fs::remove_dir_all(&d);
    }
}

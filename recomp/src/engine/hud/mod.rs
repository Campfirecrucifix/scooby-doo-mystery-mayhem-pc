//! WP-F: HUD / 2D — EFHudMgr (BC+SP), HUD windows, fonts, palettes, texts, loading screen data and
//! the APIhud_* functions of EFApi_HUD.cpp.
//!
//! Replacements (not 1:1, rule 2): RenderWare immediate-mode 2D (RwIm2DRenderPrimitive /
//! RwIm2DRenderIndexedPrimitive, render states, EFCamera) -> a per-frame `Vec<HudDrawCmd>`
//! (`EFHudMgr::draw`); EFTextureDict texture pointers -> texture names; DTWorkMemory render-command
//! buffer -> a Vec per text window; MMMalloc text commands -> Vec<u8>.
#![allow(non_snake_case)]

pub mod api;
pub mod data;
pub mod window;

pub use data::{EFFont, EFFontChar, EFHudPalette, EFLoadingScreen, EFPaletteEntry, EFText, RGBA};
pub use window::{flags, EFHud2DWindow, EFHud3DWindow, EFHudTextWindow, EFHudWindow, EFHudWindowKind, HudDrawCmd, HudTex};

use crate::engine::error::EngineError;
use std::collections::HashMap;

/// Size of the window handle table (mgr+0xb4, 0x200 entries of {serial, window*}).
pub const MAX_WINDOWS: usize = 0x200;

/// Live memory the text windows read at update time (DisplayArg with flag 2 dereferences an
/// int pointer, flag 0x40 reads a u16 string). Implemented for the logic-module [`Mem`].
pub trait HudMem {
    fn r32(&self, p: u32) -> u32;
    fn r16(&self, p: u32) -> u16;
}

impl HudMem for crate::engine::Mem {
    fn r32(&self, p: u32) -> u32 {
        crate::engine::Mem::r32(self, p)
    }
    fn r16(&self, p: u32) -> u16 {
        crate::engine::Mem::r16(self, p)
    }
}

/// No memory (all reads 0).
pub struct NoMem;
impl HudMem for NoMem {
    fn r32(&self, _: u32) -> u32 {
        0
    }
    fn r16(&self, _: u32) -> u16 {
        0
    }
}

/// EFHudMgr (EFHudMgr_BC + EFHudMgr_SP; global `_gpoHudMgr`).
#[derive(Debug, Clone)]
pub struct EFHudMgr {
    /// +0x04 root window slot
    pub root: Option<u16>,
    /// +0x0c global palette (colour index with bit 31 set)
    pub global_palette: Option<EFHudPalette>,
    /// +0x10 fonts[10]
    pub fonts: Vec<Option<EFFont>>,
    /// +0x38 palettes[10]
    pub palettes: Vec<Option<EFHudPalette>>,
    /// +0xa0 EFText
    pub text: Option<EFText>,
    /// +0xa4 / +0xa8 camera view window (1.0, 0.75)
    pub view_window: [f32; 2],
    /// +0xac / +0xb0 screen size in pixels (GraphicsMgr+0x38/+0x3c)
    pub screen: [f32; 2],
    /// +0xb4 handle table: (serial, window)
    pub slots: Vec<(u32, Option<EFHudWindow>)>,
    /// +0x10b4 last allocated slot
    pub last_slot: u32,
    /// +0x10b8 live window count
    pub count: u32,
    /// +0x10bc current language
    pub language: u32,
    /// EFHudWindow_BC::msuiIdRef (static in the C++)
    pub id_ref: u32,
    /// EFLoadingScreen data of the level (LOAD.LS)
    pub loading_screen: Option<EFLoadingScreen>,
    /// Texture sizes (RwRaster width/height) the coordinator registers; used by 2D windows with
    /// flags 0x30000 (size from texture).
    pub texture_dims: HashMap<String, (u32, u32)>,
    /// EFIniFile "MaxFPS" (30 default; 60 halves 2D texture heights - quirk kept)
    pub max_fps: u32,
    /// Output of the last Update: the 2D primitives in render order.
    pub draw: Vec<HudDrawCmd>,
}

impl Default for EFHudMgr {
    fn default() -> Self {
        Self::new()
    }
}

/// EFDisplayTextStruct (layout state of ConvertTextCommandsToRenderCommands).
struct Disp {
    /// +0x00 cursor x
    x: f32,
    /// +0x04 cursor y
    y: f32,
    /// +0x08 current font slot
    font: u32,
    /// +0x0c line start, +0x10 word start, +0x14 write pos, +0x18 end of last glyph (word indices)
    line: usize,
    word: usize,
    wr: usize,
    last: usize,
    /// +0x1c x of the current word
    word_x: f32,
    /// +0x20 next vararg
    arg: usize,
}

const END: u32 = 0xffff_ffff;
const FONT: u32 = 0xffff_fffe;
const COLOR: u32 = 0xffff_fffd;

impl EFHudMgr {
    /// EFHudMgr_BC::EFHudMgr_BC
    pub fn new() -> Self {
        EFHudMgr {
            root: None,
            global_palette: None,
            fonts: vec![None; 10],
            palettes: vec![None; 10],
            text: None,
            view_window: [1.0, 0.75],
            screen: [0.0, 0.0],
            slots: vec![(0, None); MAX_WINDOWS],
            last_slot: 0,
            count: 0,
            language: 0,
            id_ref: 0,
            loading_screen: None,
            texture_dims: HashMap::new(),
            max_fps: 30,
            draw: Vec::new(),
        }
    }

    /// EFHudMgr_BC::Init: creates the root window (EFHudWindow(0,0)).
    pub fn Init(&mut self) -> u32 {
        let h = self.NewWindow(EFHudWindowKind::Base, 0, 0);
        self.root = self.slot_of(h);
        1
    }

    /// EFHudMgr_BC::Terminate (fonts, palettes, root children, text).
    pub fn Terminate(&mut self) {
        for i in 0..10 {
            self.fonts[i] = None;
            self.palettes[i] = None;
        }
        self.global_palette = None;
        if let Some(r) = self.root {
            while let Some(c) = self.slots[r as usize].1.as_ref().and_then(|w| w.child) {
                self.DeleteWindow(c);
            }
        }
        self.text = None;
    }

    /// Feeds the level side files (EFRessourcesMgr::LoadLevel `side_files`) to the HUD:
    /// *.FT -> EFFont::LoadFromStream (AddFont), *.PAL -> EFHudPalette::LoadFromStream
    /// (AddPalette), *.TX -> EFText::LoadFromStream, *.LS -> EFLoadingScreen.
    pub fn LoadSideFiles(&mut self, files: &[(String, Vec<u8>)]) -> Result<(), EngineError> {
        for (p, b) in files {
            let up = p.to_ascii_uppercase();
            if up.ends_with(".FT") {
                let f = EFFont::load(b)?;
                if f.id != 0xffff_ffff {
                    self.AddFont(f);
                }
            } else if up.ends_with(".PAL") {
                self.AddPalette(EFHudPalette::load(b)?);
            } else if up.ends_with(".TX") {
                self.text = Some(EFText::load(b)?);
            } else if up.ends_with(".LS") {
                self.loading_screen = Some(EFLoadingScreen::load(b)?);
            }
        }
        Ok(())
    }

    /// EFHudMgr_BC::AddFont
    pub fn AddFont(&mut self, mut f: EFFont) {
        if 0.0 < self.screen[0] && 0.0 < self.screen[1] {
            f.ComputeScreenDims(self.screen[0], self.screen[1]);
        }
        let i = f.id as usize;
        if i < 10 {
            self.fonts[i] = Some(f);
        }
    }

    /// EFHudMgr_BC::AddPalette (palette already Init'ed by EFHudPalette::load).
    pub fn AddPalette(&mut self, p: EFHudPalette) {
        if p.id == 0xffff_ffff {
            self.global_palette = Some(p);
        } else if (p.id as usize) < 10 {
            let i = p.id as usize;
            self.palettes[i] = Some(p);
        }
    }

    // ---------------------------------------------------------------- handle table

    fn slot_of(&self, h: u32) -> Option<u16> {
        let s = (h >> 16) as usize;
        if s < MAX_WINDOWS && self.slots[s].1.is_some() && (h & 0xffff) == self.slots[s].0 { Some(s as u16) } else { None }
    }

    /// Handle -> slot as in every APIhud function: 0xffffffff = root window.
    pub fn Resolve(&self, h: u32) -> Option<u16> {
        if h == 0xffff_ffff { self.root } else { self.slot_of(h) }
    }

    pub fn Window(&self, h: u32) -> Option<&EFHudWindow> {
        self.Resolve(h).and_then(|s| self.slots[s as usize].1.as_ref())
    }

    pub fn WindowMut(&mut self, h: u32) -> Option<&mut EFHudWindow> {
        let s = self.Resolve(h)?;
        self.slots[s as usize].1.as_mut()
    }

    fn w(&self, s: u16) -> &EFHudWindow {
        self.slots[s as usize].1.as_ref().unwrap()
    }

    fn wm(&mut self, s: u16) -> &mut EFHudWindow {
        self.slots[s as usize].1.as_mut().unwrap()
    }

    /// EFHudWindow::EFHudWindow: allocates a handle slot. Returns the handle (0 when full: the
    /// C++ then keeps an unregistered window; here it is dropped).
    fn NewWindow(&mut self, kind: EFHudWindowKind, fl: u32, prio: u32) -> u32 {
        let id = self.id_ref;
        self.id_ref = self.id_ref.wrapping_add(1);
        let mut w = EFHudWindow::new(kind, fl, prio, id);
        if self.count as usize == MAX_WINDOWS {
            return 0;
        }
        let mut s = self.last_slot as usize + 1;
        while s < MAX_WINDOWS && self.slots[s].1.is_some() {
            s += 1;
        }
        if s == MAX_WINDOWS {
            s = 0;
            while s <= self.last_slot as usize && self.slots[s].1.is_some() {
                s += 1;
            }
        }
        self.last_slot = s as u32;
        let e = &mut self.slots[s];
        e.0 = e.0.wrapping_add(1);
        if e.0 == 0 {
            // quirk: 32-bit serial wrap writes ushort[0]=1, ushort[1]=0
            e.0 = 1;
        }
        self.count += 1;
        // quirk: the handle uses only the low 16 bits of the serial, the lookup compares all 32
        w.handle = (s as u32) << 16 | (e.0 & 0xffff);
        let h = w.handle;
        e.1 = Some(w);
        h
    }

    /// EFHudWindow::AddChild (sorted by priority, descending; equal goes first).
    fn AddChild(&mut self, parent: u16, c: u16) {
        let prio = self.w(c).priority;
        let mut prev: Option<u16> = None;
        let mut it = self.w(parent).child;
        while let Some(i) = it {
            if prio >= self.w(i).priority {
                break;
            }
            prev = Some(i);
            it = self.w(i).next;
        }
        match prev {
            None => {
                let first = self.w(parent).child;
                self.wm(c).next = first;
                self.wm(parent).child = Some(c);
            }
            Some(p) => {
                let n = self.w(p).next;
                self.wm(c).next = n;
                self.wm(p).next = Some(c);
            }
        }
        self.wm(c).parent = Some(parent);
    }

    /// EFHudWindow::RemoveChild
    fn RemoveChild(&mut self, parent: u16, c: u16) {
        let n = self.w(c).next;
        if self.w(parent).child == Some(c) {
            self.wm(parent).child = n;
        } else {
            let mut it = self.w(parent).child;
            while let Some(i) = it {
                if self.w(i).next == Some(c) {
                    self.wm(i).next = n;
                    break;
                }
                it = self.w(i).next;
            }
        }
        self.wm(c).parent = None;
    }

    /// EFHudWindow::~EFHudWindow (vtbl 0x1c): unlink, free the slot, delete children.
    fn DeleteWindow(&mut self, s: u16) {
        if let Some(p) = self.w(s).parent {
            self.RemoveChild(p, s);
        }
        let h = self.w(s).handle;
        // quirk: the count is decremented even when the handle check fails
        self.count = self.count.wrapping_sub(1);
        while let Some(c) = self.w(s).child {
            let n = self.w(c).next;
            self.wm(c).parent = None;
            self.wm(s).child = n;
            self.DeleteWindow(c);
        }
        let _ = h;
        self.slots[s as usize].1 = None;
        if self.root == Some(s) {
            self.root = None;
        }
    }

    /// EFHudMgr_BC::WindowAdd(type, parent, flags, priority) -> handle (0 when full).
    pub fn WindowAdd(&mut self, kind: u8, parent: u32, fl: u32, prio: u32) -> u32 {
        if self.count as usize == MAX_WINDOWS {
            return 0;
        }
        let k = match kind {
            0 => EFHudWindowKind::Base,
            1 => EFHudWindowKind::Text(EFHudTextWindow::default()),
            2 => EFHudWindowKind::Win2D(EFHud2DWindow::default()),
            3 => EFHudWindowKind::Win3D(EFHud3DWindow::default()),
            // quirk: other types leave the window pointer uninitialised in the C++
            _ => return 0,
        };
        let h = self.NewWindow(k, fl, prio);
        let (Some(c), Some(p)) = (self.slot_of(h), self.Resolve(parent)) else { return h };
        self.AddChild(p, c);
        h
    }

    /// EFHudMgr_BC::WindowDelete (0xffffffff ignored).
    pub fn WindowDelete(&mut self, h: u32) {
        if h != 0xffff_ffff {
            if let Some(s) = self.slot_of(h) {
                self.DeleteWindow(s);
            }
        }
    }

    /// EFHudMgr_BC::FindHudWindow(id) -> handle of the window with user id `id`.
    /// quirk: the C++ discards the result (void) and the root's first-child subtree is searched
    /// only through the sibling path; here the found handle (or 0) is returned.
    pub fn FindHudWindow(&self, id: u32) -> u32 {
        fn rec(m: &EFHudMgr, s: u16, id: u32) -> Option<u16> {
            let w = m.w(s);
            if w.id == id {
                return Some(s);
            }
            w.next.and_then(|n| rec(m, n, id)).or_else(|| w.child.and_then(|c| rec(m, c, id)))
        }
        self.root.and_then(|r| rec(self, r, id)).map(|s| self.w(s).handle).unwrap_or(0)
    }

    // ---------------------------------------------------------------- palettes

    /// Colour index -> palette entry: bit 31 set = global palette (index & 0x7fffffff),
    /// else palettes[pal][index].
    pub fn Color(&self, pal: u32, idx: u32) -> Option<&EFPaletteEntry> {
        if idx & 0x8000_0000 != 0 {
            self.global_palette.as_ref()?.entries.get((idx & 0x7fff_ffff) as usize)
        } else {
            self.palettes.get(pal as usize)?.as_ref()?.entries.get(idx as usize)
        }
    }

    fn ColorMut(&mut self, pal: u32, idx: u32) -> Option<&mut EFPaletteEntry> {
        if idx & 0x8000_0000 != 0 {
            self.global_palette.as_mut()?.entries.get_mut((idx & 0x7fff_ffff) as usize)
        } else {
            self.palettes.get_mut(pal as usize)?.as_mut()?.entries.get_mut(idx as usize)
        }
    }

    /// EFHudPalette::ColorCopy(pal, dst, src, mask): mask bits 1/2/4/8 = r/g/b/a,
    /// 0x10<<k = corner k.
    pub fn ColorCopy(&mut self, pal: u32, dst: u32, src: u32, mask: u32) {
        let Some(s) = self.Color(pal, src).map(|e| e.colors) else { return };
        let Some(d) = self.ColorMut(pal, dst) else { return };
        for k in 0..4 {
            if mask & (0x10 << k) != 0 {
                for c in 0..4 {
                    if mask & (1 << c) != 0 {
                        d.colors[k][c] = s[k][c];
                    }
                }
            }
        }
    }

    /// EFHudPalette::ColorBlend(pal, dst, a, b, t, mask): dst = a + (int)(t * (b - a)).
    pub fn ColorBlend(&mut self, pal: u32, dst: u32, a: u32, b: u32, t: f32, mask: u32) {
        let Some(ca) = self.Color(pal, a).map(|e| e.colors) else { return };
        let Some(cb) = self.Color(pal, b).map(|e| e.colors) else { return };
        let Some(d) = self.ColorMut(pal, dst) else { return };
        for k in 0..4 {
            if mask & (0x10 << k) != 0 {
                for c in 0..4 {
                    if mask & (1 << c) != 0 {
                        let diff = cb[k][c] as i32 - ca[k][c] as i32;
                        d.colors[k][c] = ca[k][c].wrapping_add((t * diff as f32) as i32 as u8);
                    }
                }
            }
        }
    }

    /// EFHudPalette::ColorFadeStart(pal, dst, a, b, duration, mask)
    pub fn ColorFadeStart(&mut self, pal: u32, dst: u32, a: u32, b: u32, dur: f32, mask: u32) {
        if let Some(e) = self.ColorMut(pal, dst) {
            e.time = 0.0;
            e.duration = dur;
            e.flags = mask | 0x8000_0000;
            e.dst = dst;
            e.a = a;
            e.b = b;
            e.pal = pal;
        }
    }

    /// EFHudPalette::ColorFadeStop(this = palette `pal` or global, idx).
    /// quirk: an index with bit 31 recurses into palettes[0] (mgr+0x38), not the global palette.
    pub fn ColorFadeStop(&mut self, pal: u32, idx: u32) {
        let p = if pal == 0xffff_ffff { self.global_palette.as_mut() } else { self.palettes.get_mut(pal as usize).and_then(|p| p.as_mut()) };
        if idx & 0x8000_0000 == 0 {
            if let Some(e) = p.and_then(|p| p.entries.get_mut(idx as usize)) {
                e.flags &= 0x7fff_ffff;
            }
        } else if let Some(e) = self.palettes[0].as_mut().and_then(|p| p.entries.get_mut((idx & 0x7fff_ffff) as usize)) {
            e.flags &= 0x7fff_ffff;
        }
    }

    /// APIhud ColorIsFading(pal, idx)
    pub fn ColorIsFading(&self, pal: u32, idx: u32) -> bool {
        self.Color(pal, idx).map(|e| e.flags & 0x8000_0000 != 0).unwrap_or(false)
    }

    /// EFHudPalette::Update(dt) of one palette (None = global).
    fn PaletteUpdate(&mut self, which: Option<usize>, dt: f32) {
        let n = match which {
            None => self.global_palette.as_ref().map(|p| p.entries.len()),
            Some(i) => self.palettes[i].as_ref().map(|p| p.entries.len()),
        }
        .unwrap_or(0);
        for i in 0..n {
            fn get(m: &mut EFHudMgr, which: Option<usize>, i: usize) -> &mut EFPaletteEntry {
                match which {
                    None => &mut m.global_palette.as_mut().unwrap().entries[i],
                    Some(p) => &mut m.palettes[p].as_mut().unwrap().entries[i],
                }
            }
            let e = get(self, which, i);
            if e.flags & 0x8000_0000 == 0 {
                continue;
            }
            e.time += dt;
            let dur = e.duration;
            let mut t = if 0.001 < dur { e.time * 2.0 / dur } else { 1.0 };
            let ev = *e;
            if 1.0 <= t {
                if ev.flags & 0x10000 == 0 {
                    // quirk: the copy passes (dst, b) - the C++ ColorCopy(pal, dst, src=b, flags)
                    self.ColorCopy(ev.pal, ev.dst, ev.b, ev.flags);
                    let e = get(self, which, i);
                    e.flags &= 0x7fff_ffff;
                    continue;
                }
                if 2.0 < t {
                    let mut f = 2.0;
                    if t - 2.0 <= 2.0 {
                        f = t - 2.0;
                    }
                    get(self, which, i).time -= dur;
                    t = f;
                }
                if 1.0 < t {
                    t = 2.0 - t;
                }
            }
            self.ColorBlend(ev.pal, ev.dst, ev.a, ev.b, t, ev.flags);
        }
    }

    // ---------------------------------------------------------------- update / render

    /// EFHudMgr_SP::Update(camera, dt): screen size change -> ComputeScreenDims, palettes, window
    /// Update + Render. The draw list replaces the RenderWare 2D output.
    pub fn Update(&mut self, dt: f32, screen_w: u32, screen_h: u32, mem: &dyn HudMem) {
        if self.global_palette.is_none() {
            let mut p = EFHudPalette { id: 0xffff_ffff, entries: Vec::new() };
            p.Init(1, &[0xff; 16]);
            self.global_palette = Some(p);
        }
        if screen_w as f32 != self.screen[0] || screen_h as f32 != self.screen[1] {
            self.screen = [screen_w as f32, screen_h as f32];
            let (sx, sy) = (self.screen[0], self.screen[1]);
            for f in self.fonts.iter_mut().flatten() {
                f.ComputeScreenDims(sx, sy);
            }
        }
        for i in 0..10 {
            if self.palettes[i].is_some() {
                self.PaletteUpdate(Some(i), dt);
            }
        }
        self.PaletteUpdate(None, dt);
        self.draw.clear();
        if let Some(r) = self.root {
            self.WindowUpdate(r, dt, mem);
            self.WindowRender(r);
        }
    }

    /// EFHudWindow::ComputePos
    fn ComputePos(&mut self, s: u16) {
        let w = self.w(s);
        let ps = w.parent.or(self.root).unwrap_or(s);
        let (pr, pc) = (self.w(ps).rect, self.w(ps).clip);
        let has_parent = w.parent.is_some();
        let w = self.wm(s);
        w.rect = w.screen;
        let f = w.flags;
        if f & 3 != 0 {
            w.rect[0] = pr[0] + w.pos[0] * pr[2];
            w.rect[1] = pr[1] + w.pos[1] * pr[3];
            if f & 2 != 0 {
                w.rect[2] = w.dims[0] * pr[2];
                w.rect[3] = w.dims[1] * pr[3];
            }
        }
        if f & 4 != 0 {
            w.rect[0] += (pr[2] - w.rect[2]) * 0.5;
        }
        if f & 8 != 0 {
            w.rect[1] += (pr[3] - w.rect[3]) * 0.5;
        }
        if f & 0x10 != 0 {
            w.rect[0] += pr[2] - w.rect[2];
        }
        if f & 0x20 != 0 {
            w.rect[1] += pr[3] - w.rect[3];
        }
        w.clip = w.rect;
        if has_parent {
            let c = &mut w.clip;
            if c[0] < pc[0] {
                c[2] -= pc[0] - c[0];
                c[0] = pc[0];
            }
            if c[1] < pc[1] {
                c[3] -= pc[1] - c[1];
                c[1] = pc[1];
            }
            if pc[0] + pc[2] < c[0] + c[2] {
                c[2] = pc[0] + pc[2] - c[0];
            }
            if pc[1] + pc[3] < c[1] + c[3] {
                c[3] = pc[1] + pc[3] - c[1];
            }
        }
    }

    /// EFHudWindow::Update(dt, window, workmem)
    fn WindowUpdate(&mut self, s: u16, dt: f32, mem: &dyn HudMem) {
        if self.w(s).flags & 0x4000_0100 != 0 {
            return;
        }
        let sc = self.screen;
        let w = self.wm(s);
        w.screen = [w.pos[0] * sc[0], w.pos[1] * sc[1], w.dims[0] * sc[0], w.dims[1] * sc[1]];
        self.ComputePos(s);
        let c = self.w(s).clip;
        if 0.5 <= c[2] && 0.5 <= c[3] && self.SpecificUpdate(s, mem) & 2 != 0 {
            self.ComputePos(s);
        }
        let mut it = self.w(s).child;
        while let Some(i) = it {
            self.WindowUpdate(i, dt, mem);
            it = self.w(i).next;
        }
        let w = self.wm(s);
        if w.flags & 0x8000_0000 != 0 {
            w.timer -= dt;
        }
    }

    /// SpecificUpdate (vtbl 0x08) of the window classes.
    fn SpecificUpdate(&mut self, s: u16, mem: &dyn HudMem) -> u32 {
        match self.w(s).type_byte() {
            1 => {
                self.TextSpecificUpdate(s, mem);
                0
            }
            2 => {
                // EFHud2DWindow_SP::SpecificUpdate: flags 0x30000 -> size from the texture
                let w = self.w(s);
                if w.flags & 0x30000 != 0x30000 {
                    return 0;
                }
                let (tx, tl) = match &w.kind {
                    EFHudWindowKind::Win2D(d) => (d.texture.clone(), d.tiling),
                    _ => unreachable!(),
                };
                // quirk: the C++ dereferences the texture unconditionally (crash if unset)
                let (tw, th) = tx.and_then(|t| self.texture_dims.get(&t).copied()).unwrap_or((0, 0));
                let fps60 = self.max_fps == 60;
                let w = self.wm(s);
                w.screen[2] = tw as f32 * tl[0];
                w.screen[3] = th as f32 * tl[1];
                if fps60 {
                    w.screen[3] *= 0.5;
                }
                2
            }
            _ => 0,
        }
    }

    /// EFHudWindow::Render (camera, ...) -> appends to `self.draw`.
    fn WindowRender(&mut self, s: u16) {
        let w = self.w(s);
        if w.flags & 0x100 != 0 || !(0.5 <= w.clip[2]) || !(0.5 <= w.clip[3]) {
            return;
        }
        if w.flags & 0x800 != 0 {
            self.DrawBackGround(s);
        }
        self.SpecificRender(s);
        let mut prev: Option<u16> = None;
        let mut it = self.w(s).child;
        while let Some(c) = it {
            let next = self.w(c).next;
            self.WindowRender(c);
            if self.w(c).flags & 0x4000_0000 != 0 {
                // quirk: the deleted child is unlinked from the list by hand after its destructor
                self.DeleteWindow(c);
                it = next;
                continue;
            }
            prev = Some(c);
            it = next;
        }
        let _ = prev;
        let w = self.wm(s);
        if w.flags & 0x8000_0000 != 0 && w.timer <= 0.0 {
            if w.flags & 0x200 != 0 {
                w.flags |= 0x100;
            }
            if w.flags & 0x400 != 0 {
                w.flags |= 0x4000_0000;
            }
            w.flags &= 0x7fff_ffff;
        }
    }

    fn global_colors(&self, idx: u32) -> [RGBA; 4] {
        self.global_palette.as_ref().and_then(|p| p.entries.get(idx as usize)).map(|e| e.colors).unwrap_or([[0; 4]; 4])
    }

    /// EFHudWindow::DrawBackGround (FillPoly = untextured quads).
    fn DrawBackGround(&mut self, s: u16) {
        let w = self.w(s);
        let r = w.rect;
        // vertices: 0 (x+w,y) 1 (x,y) 2 (x+w,y+h) 3 (x,y+h)
        let inner = [[r[0] + r[2], r[1]], [r[0], r[1]], [r[0] + r[2], r[1] + r[3]], [r[0], r[1] + r[3]]];
        // quirk: the background colour indexes the global palette entries directly (no bit 31)
        let col = self.global_colors(w.bg_color);
        let prio = w.priority;
        let quad = |pos: [[f32; 2]; 4], colors: [RGBA; 4]| HudDrawCmd { tex: HudTex::None, pos, uv: [[0.0; 2]; 4], colors, priority: prio };
        if w.bg_border == [0.0, 0.0] {
            self.draw.push(quad(inner, col));
            return;
        }
        let ps = w.parent.or(self.root).unwrap_or(s);
        let pr = self.w(ps).rect;
        let (bx, by) = if w.flags & 0x1000 == 0 {
            ((w.bg_border[0] * pr[2] - r[2]) / 2.0, (w.bg_border[1] * pr[3] - r[3]) / 2.0)
        } else {
            (w.bg_border[0] * pr[2], w.bg_border[1] * pr[3])
        };
        let outer = [[inner[0][0] + bx, inner[0][1] - by], [inner[1][0] - bx, inner[1][1] - by], [inner[2][0] + bx, inner[2][1] + by], [inner[3][0] - bx, inner[3][1] + by]];
        self.draw.push(quad(inner, col));
        // uiVertexBorder edge table (not in the dump): assumed the 4 outline edges 0-1,1-3,3-2,2-0.
        for (a, b) in [(0usize, 1usize), (1, 3), (3, 2), (2, 0)] {
            self.draw.push(quad([outer[a], outer[b], inner[a], inner[b]], [[0; 4], [0; 4], col[a], col[a]]));
        }
    }

    /// SpecificRender (vtbl 0x0c) of the window classes.
    fn SpecificRender(&mut self, s: u16) {
        let w = self.w(s);
        let r = w.rect;
        let prio = w.priority;
        match &w.kind {
            EFHudWindowKind::Win2D(d) => {
                // EFHud2DWindow_SP::SpecificRender: strip (x,y) (x,y+h) (x+w,y) (x+w,y+h)
                let col = self.global_colors(d.color);
                let (mut u1, mut v1) = (d.tiling[0], d.tiling[1]);
                if w.flags & 0x10000 != 0 {
                    let (tw, th) = d.texture.as_ref().and_then(|t| self.texture_dims.get(t).copied()).unwrap_or((1, 1));
                    u1 = r[2] * d.tiling[0] / tw as f32;
                    v1 = r[3] * d.tiling[1] / th as f32;
                    if self.max_fps == 60 {
                        v1 *= 2.0;
                    }
                }
                let tex = d.texture.clone().map(HudTex::Texture).unwrap_or(HudTex::None);
                self.draw.push(HudDrawCmd {
                    tex,
                    pos: [[r[0], r[1]], [r[0], r[1] + r[3]], [r[0] + r[2], r[1]], [r[0] + r[2], r[1] + r[3]]],
                    uv: [[0.0, 0.0], [0.0, v1], [u1, 0.0], [u1, v1]],
                    colors: col,
                    priority: prio,
                });
            }
            EFHudWindowKind::Win3D(d) => {
                // EFHud3DWindow_SP::SpecificRender renders the object clump with the HUD camera:
                // unported (world/RW); exposed as a marker command.
                // quirk: the C++ dereferences +0x8c unchecked (NULL slot would crash); skipped here
                if d.obj != 0 {
                    self.draw.push(HudDrawCmd { tex: HudTex::Object3D(d.owner, d.obj), pos: [[r[0], r[1]], [r[0], r[1] + r[3]], [r[0] + r[2], r[1]], [r[0] + r[2], r[1] + r[3]]], uv: [[0.0; 2]; 4], colors: [[0xff; 4]; 4], priority: prio });
                }
            }
            EFHudWindowKind::Text(t) => {
                let cmds = t.render_cmds.clone();
                let tcolor = t.color;
                self.TextSpecificRender(&cmds, tcolor, prio);
            }
            EFHudWindowKind::Base => {}
        }
    }

    /// EFHudTextWindow_SP::SpecificRender
    fn TextSpecificRender(&mut self, cmds: &[u32], tcolor: u32, prio: u32) {
        let mut font: Option<u32> = None;
        let mut fslot = 0u32;
        let mut color = 0x8000_0000u32;
        let mut cols: Option<[RGBA; 4]> = None;
        let mut i = 0;
        while i < cmds.len() && cmds[i] != END {
            while i < cmds.len() && (cmds[i] as i32) >= 0 {
                let (code, x, y) = (cmds[i], f32::from_bits(cmds[i + 1]), f32::from_bits(cmds[i + 2]));
                i += 3;
                if !(y < 500.0 && -20.0 < y) {
                    continue;
                }
                let Some(f) = font.and_then(|f| self.fonts[f as usize].as_ref()) else { continue };
                let Some(c) = f.Char(code) else { continue };
                self.draw.push(HudDrawCmd {
                    tex: HudTex::Font { font: font.unwrap(), tex: c.tex as i32 as u32 },
                    pos: [[x, y], [x, y + c.sh], [x + c.sw, y], [x + c.sw, y + c.sh]],
                    uv: [[c.u, c.v], [c.u, c.v + c.dv], [c.u + c.du, c.v], [c.u + c.du, c.v + c.dv]],
                    colors: cols.unwrap_or([[0; 4]; 4]),
                    priority: prio,
                });
            }
            if i >= cmds.len() {
                break;
            }
            match cmds[i] {
                COLOR => {
                    color = cmds[i + 1];
                    if font.is_some() {
                        if color == 0xffff_ffff {
                            color = tcolor;
                        }
                        // quirk: non-global colours come from the palette with the font's slot
                        cols = self.Color(fslot, color).map(|e| e.colors);
                    }
                    i += 2;
                }
                FONT => {
                    fslot = cmds[i + 1];
                    font = Some(fslot).filter(|&f| (f as usize) < 10 && self.fonts[f as usize].is_some());
                    cols = self.Color(fslot, color).map(|e| e.colors);
                    i += 2;
                }
                _ => i += 1,
            }
        }
    }

    // ---------------------------------------------------------------- text windows

    /// EFHudTextWindow_BC::SpecificUpdate
    fn TextSpecificUpdate(&mut self, s: u16, mem: &dyn HudMem) {
        let w = self.w(s);
        let rs = if w.flags & 0x100000 == 0 { self.root.unwrap_or(s) } else { s };
        let rr = self.w(rs).rect;
        let rect = w.rect;
        let fl = w.flags;
        let EFHudWindowKind::Text(t) = &w.kind else { return };
        let m = t.margins;
        let area = [rect[0] + m[0] * rr[2], rect[1] + m[1] * rr[3], rect[2] - (m[0] + m[2]) * rr[2], rect[3] - (m[1] + m[3]) * rr[3]];
        let tw = t.clone();
        let cmds: Option<Vec<u8>> = if tw.text_id == 0xffff_ffff {
            tw.string_cmds.clone()
        } else {
            // quirk: no NULL check on GetText in the C++
            self.text.as_ref().and_then(|x| x.GetText(tw.text_id)).map(|b| b.to_vec())
        };
        let mut tw = tw;
        tw.area = area;
        tw.render_cmds.clear();
        if let Some(c) = cmds {
            tw.render_cmds = self.ConvertTextCommandsToRenderCommands(&tw, fl, &c, mem);
        }
        if let Some(t) = self.wm(s).text() {
            t.area = tw.area;
            t.render_cmds = tw.render_cmds;
        }
    }

    fn font_ref(&self, slot: u32) -> Option<&EFFont> {
        self.fonts.get(slot as usize)?.as_ref()
    }

    /// EFHudTextWindow_BC::ConvertTextCommandsToRenderCommands
    fn ConvertTextCommandsToRenderCommands(&self, t: &EFHudTextWindow, fl: u32, cmds: &[u8], mem: &dyn HudMem) -> Vec<u32> {
        let mut out = Vec::new();
        out.extend_from_slice(&[FONT, t.font, COLOR, t.color]);
        let mut d = Disp { x: t.area[0], y: t.area[1], font: t.font, line: 4, word: 4, wr: 4, last: 4, word_x: t.area[0], arg: 0 };
        let rd = |p: usize| -> u32 { cmds.get(p..p + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap())).unwrap_or(1) };
        let mut p = 0usize;
        loop {
            let c = rd(p);
            p += 4;
            match c {
                1 => break,
                0 => {
                    let mut s = Vec::new();
                    let mut q = p;
                    while q + 2 <= cmds.len() {
                        let u = u16::from_le_bytes([cmds[q], cmds[q + 1]]);
                        if u == 0 {
                            break;
                        }
                        s.push(u);
                        q += 2;
                    }
                    self.DisplayString(t.area, fl, &s, &mut d, &mut out);
                    p = (q + 4) & !3;
                }
                2 => {
                    let b = cmds.get(p..p + 4).unwrap_or(&[0; 4]);
                    let ty = u16::from_le_bytes([b[0], b[1]]);
                    self.DisplayArg(t, fl, ty, b[2], &mut d, mem, &mut out);
                    p += 4;
                }
                3 | 4 => {
                    let v = if c == 3 {
                        p += 4;
                        rd(p - 4)
                    } else {
                        d.arg += 1;
                        t.args.get(d.arg - 1).copied().unwrap_or(0)
                    };
                    out.truncate(d.wr);
                    out.extend_from_slice(&[COLOR, v]);
                    d.wr += 2;
                }
                5 | 6 => {
                    let mut v = if c == 5 {
                        p += 4;
                        rd(p - 4)
                    } else {
                        d.arg += 1;
                        t.args.get(d.arg - 1).copied().unwrap_or(0)
                    };
                    if v == 0xffff_ffff {
                        v = t.font;
                    }
                    out.truncate(d.wr);
                    out.extend_from_slice(&[FONT, v]);
                    d.wr += 2;
                    d.font = v;
                }
                // quirk: unknown commands are skipped one word at a time
                _ => {}
            }
            if p >= cmds.len() {
                break;
            }
        }
        if !(d.x < t.area[0] + t.area[2]) {
            self.AlignLine(t.area, fl, &mut d, &mut out);
        } else {
            d.word = d.last;
        }
        self.AlignLine(t.area, fl, &mut d, &mut out);
        self.AlignY(t.area, fl, &d, &mut out);
        out.truncate(d.wr);
        out.push(END);


        out
    }

    /// Glyph lookup: (char width px, y offset px).
    fn glyph(&self, font: u32, code: u32) -> (f32, f32) {
        self.font_ref(font).and_then(|f| f.Char(code)).map(|c| (c.sw, c.syoff)).unwrap_or((0.0, 0.0))
    }

    /// Glyph branch of DisplayString / DisplayUnicodeString.
    fn put_glyph(&self, code: u32, d: &mut Disp, out: &mut Vec<u32>) {
        let (sw, yo) = self.glyph(d.font, code);
        let sp = self.font_ref(d.font).map(|f| f.screen_spacing).unwrap_or(0.0);
        if d.word == d.wr {
            d.word_x = d.x;
        }
        out.truncate(d.wr);
        out.extend_from_slice(&[code, d.x.to_bits(), (d.y + yo).to_bits()]);
        d.x += sp + sw;
        d.wr += 3;
        d.last = d.wr;
    }

    /// Space / tab branch: returns the wrap flag.
    fn space(&self, area: [f32; 4], adv: f32, d: &mut Disp) -> bool {
        let wrap = area[0] + area[2] < d.x;
        if !wrap {
            d.word = d.last;
            d.word_x = d.x + adv;
        }
        d.x += adv;
        wrap
    }

    /// EFHudTextWindow_BC::DisplayString (u16 text codes; 0xffff newline, 0xfffe space, 0xfffd tab)
    fn DisplayString(&self, area: [f32; 4], fl: u32, s: &[u16], d: &mut Disp, out: &mut Vec<u32>) {
        for &u in s {
            let sp = self.font_ref(d.font).map(|f| f.screen_space).unwrap_or(0.0);
            let wrap = match u {
                0xffff => {
                    if d.x < area[0] + area[2] {
                        d.word = d.last;
                        d.word_x = d.x;
                    }
                    true
                }
                0xfffe => self.space(area, sp, d),
                0xfffd => self.space(area, sp * 3.0, d),
                _ => {
                    self.put_glyph(u as u32, d, out);
                    false
                }
            };
            if wrap {
                self.AlignLine(area, fl, d, out);
            }
        }
    }

    /// EFHudTextWindow_BC::DisplayUnicodeString ("\n", "\t", ' ', ascii via the font table).
    fn DisplayUnicodeString(&self, area: [f32; 4], fl: u32, s: &[u16], d: &mut Disp, out: &mut Vec<u32>) {
        let mut i = 0;
        while i < s.len() {
            let u = s[i];
            let sp = self.font_ref(d.font).map(|f| f.screen_space).unwrap_or(0.0);
            let mut wrap = false;
            if u == 0x5c {
                i += 1;
                match s.get(i) {
                    Some(0x6e) => {
                        if d.x < area[0] + area[2] {
                            d.word = d.last;
                            d.word_x = d.x;
                        }
                        wrap = true;
                    }
                    Some(0x74) => wrap = self.space(area, sp * 3.0, d),
                    _ => {}
                }
                if i >= s.len() {
                    break;
                }
            } else if u == 0x20 {
                wrap = self.space(area, sp, d);
            } else {
                let code = if u < 0x80 { self.font_ref(d.font).map(|f| f.ascii[u as usize] as u32).unwrap_or(1) } else { 1 };
                self.put_glyph(code, d, out);
            }
            if wrap {
                self.AlignLine(area, fl, d, out);
            }
            i += 1;
        }
    }

    /// EFHudTextWindow_BC::DisplayArg (type flags: 1 number, 2 deref, 4 pad to width, 8 pad with
    /// '0', 0x10 hex, 0x20 unsigned, 0x40 unicode string pointer, 0x80 text id).
    #[allow(clippy::too_many_arguments)]
    fn DisplayArg(&self, t: &EFHudTextWindow, fl: u32, ty: u16, width: u8, d: &mut Disp, mem: &dyn HudMem, out: &mut Vec<u32>) {
        let arg = t.args.get(d.arg).copied().unwrap_or(0);
        let asc = |d: &Disp, c: u8| self.font_ref(d.font).map(|f| f.ascii[c as usize]).unwrap_or(0);
        if ty & 1 != 0 {
            d.arg += 1;
            let mut buf = [0u16; 32];
            let mut v = arg;
            if ty & 2 != 0 {
                v = mem.r32(v);
            }
            let mut neg = false;
            if ty & 0x20 == 0 && (v as i32) < 0 {
                neg = true;
                v = (v as i32).wrapping_neg() as u32;
            }
            let mut idx: usize = 0x1e;
            if v == 0 {
                idx = 0x1d;
                buf[0x1e] = asc(d, b'0');
            }
            while v != 0 {
                let dig = if ty & 0x10 == 0 { b'0' + (v % 10) as u8 } else { b"0123456789ABCDEF"[(v & 0xf) as usize] };
                buf[idx] = asc(d, dig);
                idx -= 1;
                v = if ty & 0x10 == 0 { v / 10 } else { v >> 4 };
            }
            let mut u5 = idx;
            if neg {
                buf[idx] = asc(d, b'-');
                u5 = idx - 1;
            }
            let lim = 0x1e - (width as usize).min(0x1e);
            if ty & 4 != 0 && lim < u5 {
                let mut fill = 0xfffe;
                if ty & 8 != 0 {
                    if neg {
                        u5 += 1;
                    }
                    fill = asc(d, b'0');
                }
                while lim <= u5 {
                    buf[u5] = fill;
                    if u5 == 0 {
                        break;
                    }
                    u5 -= 1;
                }
                u5 += 1;
                if neg {
                    // quirk: the '-' is written on the slot just before the displayed digits
                    buf[u5] = asc(d, b'-');
                }
            }
            let s: Vec<u16> = buf[u5 + 1..0x1f].to_vec();
            self.DisplayString(t.area, fl, &s, d, out);
        } else if ty & 0x40 != 0 {
            d.arg += 1;
            let mut s = Vec::new();
            let mut p = arg;
            while s.len() < 0x400 {
                let c = mem.r16(p);
                if c == 0 {
                    break;
                }
                s.push(c);
                p += 2;
            }
            self.DisplayUnicodeString(t.area, fl, &s, d, out);
        } else if ty & 0x80 != 0 {
            d.arg += 1;
            let s: Vec<u16> = self.text.as_ref().and_then(|x| x.GetText(arg)).map(|b| {
                let b = &b[4.min(b.len())..];
                b.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).take_while(|&c| c != 0).collect()
            }).unwrap_or_default();
            self.DisplayString(t.area, fl, &s, d, out);
        }
    }


    /// EFHudTextWindow_BC::AlignLine (end of a line: horizontal alignment, then move the pending
    /// word to the next line).
    fn AlignLine(&self, area: [f32; 4], fl: u32, d: &mut Disp, out: &mut [u32]) {
        let left = area[0];
        let mut target = left;
        let lh = self.font_ref(d.font).map(|f| f.screen_line_height).unwrap_or(0.0);
        if fl & 0x50000 != 0 && d.word >= 3 {
            let lastg = d.word - 3;
            let end = f32::from_bits(out[lastg + 1]) + self.glyph(d.font, out[lastg]).0 - left;
            target = if fl & 0x10000 == 0 { left + area[2] - end } else { left + (area[2] - end) * 0.5 };
        }
        if 0.5 < target - left {
            let mut i = d.line;
            while d.word >= 3 && i <= d.word - 3 && i + 2 < out.len() {
                match out[i] {
                    COLOR | FONT => i = i.wrapping_sub(1), // C++ int index: -1, then +3
                    v if (v as i32) >= 0 => out[i + 1] = (f32::from_bits(out[i + 1]) + (target - left)).to_bits(),
                    _ => {}
                }
                i = i.wrapping_add(3);
            }
        }
        d.y += lh;
        let shift = area[0] - d.word_x;
        let mut i = d.word;
        while i < d.wr && i + 2 < out.len() + 1 {
            match out[i] {
                COLOR | FONT => i = i.wrapping_sub(1), // C++ int index: -1, then +3
                v if (v as i32) >= 0 => {
                    out[i + 1] = (f32::from_bits(out[i + 1]) + shift).to_bits();
                    out[i + 2] = (f32::from_bits(out[i + 2]) + lh).to_bits();
                }
                _ => {}
            }
            i = i.wrapping_add(3);
        }
        d.x += shift;
        d.word_x += shift;
        d.line = d.word;
        d.word = d.last;
    }

    /// EFHudTextWindow_BC::AlignY (flags 0xa0000: bottom / 0x20000 centre).
    fn AlignY(&self, area: [f32; 4], fl: u32, d: &Disp, out: &mut [u32]) {
        if fl & 0xa0000 == 0 {
            return;
        }
        let mut dy = area[3] - (d.y - area[1]);
        if fl & 0x20000 != 0 {
            dy *= 0.5;
        }
        let mut i = 0;
        while i < d.wr && i + 2 < out.len() + 1 {
            match out[i] {
                COLOR | FONT => i = i.wrapping_sub(1), // C++ int index: -1, then +3
                v if (v as i32) >= 0 => out[i + 2] = (f32::from_bits(out[i + 2]) + dy).to_bits(),
                _ => {}
            }
            i = i.wrapping_add(3);
        }
    }
}

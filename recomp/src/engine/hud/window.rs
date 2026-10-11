//! WP-F: EFHudWindow / EFHud2DWindow / EFHud3DWindow / EFHudTextWindow (BC + SP + final classes).
//!
//! Windows live in the EFHudMgr handle table (`EFHudMgr::slots`, mgr+0xb4); C++ pointers between
//! windows (parent +0x58, next sibling +0x5c, first child +0x60) become slot indices.
#![allow(non_snake_case)]

use super::data::RGBA;

/// EFHudWindow flags (+0x44).
pub mod flags {
    pub const REL_POS: u32 = 0x1;
    pub const REL_DIMS: u32 = 0x2;
    pub const CENTER_X: u32 = 0x4;
    pub const CENTER_Y: u32 = 0x8;
    pub const RIGHT: u32 = 0x10;
    pub const BOTTOM: u32 = 0x20;
    pub const HIDDEN: u32 = 0x100;
    pub const HIDE_ON_TIMER: u32 = 0x200;
    pub const DELETE_ON_TIMER: u32 = 0x400;
    pub const BACKGROUND: u32 = 0x800;
    pub const BG_BORDER_ABS: u32 = 0x1000;
    /// text: centre lines; 2D: size from texture
    pub const TEXT_CENTER: u32 = 0x10000;
    /// text: vertical centre; 2D (with 0x10000): size from texture
    pub const TEXT_VCENTER: u32 = 0x20000;
    pub const TEXT_RIGHT: u32 = 0x40000;
    pub const TEXT_BOTTOM: u32 = 0x80000;
    pub const TEXT_MARGINS_SELF: u32 = 0x100000;
    pub const DELETE: u32 = 0x4000_0000;
    pub const TIMER: u32 = 0x8000_0000;
}

/// Window type byte (+0x64), also the WindowAdd type argument.
#[derive(Debug, Clone, PartialEq)]
pub enum EFHudWindowKind {
    /// 0: EFHudWindow
    Base,
    /// 1: EFHudTextWindow
    Text(EFHudTextWindow),
    /// 2: EFHud2DWindow
    Win2D(EFHud2DWindow),
    /// 3: EFHud3DWindow
    Win3D(EFHud3DWindow),
}

/// EFHudTextWindow_BC fields (+0x8c..+0x110).
#[derive(Debug, Clone, PartialEq)]
pub struct EFHudTextWindow {
    /// +0x8c text id (0xffffffff = use the string commands +0xf4)
    pub text_id: u32,
    /// +0x90 render commands of the last update (C++: pointer into the frame work memory;
    /// here an owned buffer of u32 words: glyph = code,x(f32 bits),y(f32 bits); -3,colour;
    /// -2,font; -1 end).
    pub render_cmds: Vec<u32>,
    /// +0x94..+0xe0 the 20 vararg words
    pub args: [u32; 20],
    /// +0xe4 left, +0xe8 top, +0xec right, +0xf0 bottom margins
    pub margins: [f32; 4],
    /// +0xf4 text commands built from a string (MMMalloc'd in the C++)
    pub string_cmds: Option<Vec<u8>>,
    /// +0xf8 font slot
    pub font: u32,
    /// +0xfc x, +0x100 y, +0x104 w, +0x108 h of the text area (SpecificUpdate)
    pub area: [f32; 4],
    /// +0x10c colour index
    pub color: u32,
}

impl Default for EFHudTextWindow {
    fn default() -> Self {
        EFHudTextWindow { text_id: 0xffff_ffff, render_cmds: Vec::new(), args: [0; 20], margins: [0.0; 4], string_cmds: None, font: 0, area: [0.0; 4], color: 0 }
    }
}

/// EFHud2DWindow_BC fields.
#[derive(Debug, Clone, PartialEq)]
pub struct EFHud2DWindow {
    /// +0x8c / +0x90 tiling (uv max)
    pub tiling: [f32; 2],
    /// +0x94 colour index in the global palette
    pub color: u32,
    /// +0xa0 texture (EFTextureDict::GetTexture(name)); here the name
    pub texture: Option<String>,
}

impl Default for EFHud2DWindow {
    fn default() -> Self {
        EFHud2DWindow { tiling: [1.0, 1.0], color: 0, texture: None }
    }
}

/// EFHud3DWindow fields.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EFHud3DWindow {
    /// +0x8c object (GameObject+0x174 [n]); here the EF3dObj resource uid of descriptor n
    /// (0 = NULL slot)
    pub obj: u32,
    /// owner game object uid of `obj` (the C++ EF3dObj knows its owner via +0x10)
    pub owner: u32,
}

/// EFHudWindow (EFHudWindow_BC fields +0x04..+0x88).
#[derive(Debug, Clone, PartialEq)]
pub struct EFHudWindow {
    /// +0x04 / +0x08 position (fraction of parent or screen)
    pub pos: [f32; 2],
    /// +0x0c / +0x10 dims
    pub dims: [f32; 2],
    /// +0x14..+0x20 pos/dims in pixels (Update)
    pub screen: [f32; 4],
    /// +0x24..+0x30 computed rect (ComputePos)
    pub rect: [f32; 4],
    /// +0x34..+0x40 rect clipped to parent
    pub clip: [f32; 4],
    /// +0x44
    pub flags: u32,
    /// +0x48 user id (WindowSetID / FindHudWindow)
    pub id: u32,
    /// +0x4c background colour index (global palette)
    pub bg_color: u32,
    /// +0x50 / +0x54 background border
    pub bg_border: [f32; 2],
    /// +0x58 parent, +0x5c next sibling, +0x60 first child (slot indices)
    pub parent: Option<u16>,
    pub next: Option<u16>,
    pub child: Option<u16>,
    /// +0x68 priority (children sorted descending)
    pub priority: u32,
    /// +0x84 timer
    pub timer: f32,
    /// +0x88 handle (slot << 16 | serial), 0 when the table is full
    pub handle: u32,
    /// +0x64 type + subclass fields
    pub kind: EFHudWindowKind,
}

impl EFHudWindow {
    /// EFHudWindow_BC ctor (flags, priority); `id` = msuiIdRef++.
    pub fn new(kind: EFHudWindowKind, flags: u32, priority: u32, id: u32) -> Self {
        EFHudWindow {
            pos: [0.0; 2],
            dims: [1.0; 2],
            screen: [0.0; 4],
            rect: [0.0; 4],
            clip: [0.0; 4],
            flags,
            id,
            bg_color: 0,
            bg_border: [0.0; 2],
            parent: None,
            next: None,
            child: None,
            priority,
            timer: 0.0,
            handle: 0,
            kind,
        }
    }
    pub fn type_byte(&self) -> u8 {
        match self.kind {
            EFHudWindowKind::Base => 0,
            EFHudWindowKind::Text(_) => 1,
            EFHudWindowKind::Win2D(_) => 2,
            EFHudWindowKind::Win3D(_) => 3,
        }
    }
    pub fn text(&mut self) -> Option<&mut EFHudTextWindow> {
        if let EFHudWindowKind::Text(t) = &mut self.kind { Some(t) } else { None }
    }
    pub fn win2d(&mut self) -> Option<&mut EFHud2DWindow> {
        if let EFHudWindowKind::Win2D(t) = &mut self.kind { Some(t) } else { None }
    }
}

/// What a 2D draw command samples.
#[derive(Debug, Clone, PartialEq)]
pub enum HudTex {
    /// untextured (FillPoly)
    None,
    /// named texture of the level texture dictionary
    Texture(String),
    /// font texture `tex` of font slot `font` (EFFont::textures[tex])
    Font { font: u32, tex: u32 },
    /// EFHud3DWindow: render a game object in this rect (not drawn by the HUD itself)
    /// (owner game object uid, EF3dObj resource uid)
    Object3D(u32, u32),
}

/// One 2D primitive (replaces the RwIm2D vertex buffers): 4 vertices in pixels, as a strip
/// v0 v1 v2 v3 (RwIm2D tri-strip / indexed quad order 0,1,2 2,1,3), with uv and RGBA.
#[derive(Debug, Clone, PartialEq)]
pub struct HudDrawCmd {
    pub tex: HudTex,
    pub pos: [[f32; 2]; 4],
    pub uv: [[f32; 2]; 4],
    pub colors: [RGBA; 4],
    /// window priority (+0x68); the C++ z = cam+0x90 + priority * cam+0x8c
    pub priority: u32,
}
